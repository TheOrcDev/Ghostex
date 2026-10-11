//! The system-wide Ghostex Capture hotkeys: Cmd+Ctrl+Shift on macOS and Alt+Ctrl+Shift on Windows
//! and Linux, plus S (open the panel), A (area), Space (current app), F (full screen) or T (prompt).
//!
//! CDXC:GhostexCapture 2026-09-30 DECISION:
//! User: "let's do alt/cmd ctrl shift s for now", and "the 3 modifiers but a or space or f or t do
//! the screenshot or writing action directly without seeing the pop up first".
//!
//! Each backend delivers presses into one channel the app drains: Carbon hotkeys on macOS (no
//! Accessibility permission needed), `RegisterHotKey` on a thread with its own message loop on
//! Windows (GPUI's loop drops thread messages), and a key grab on the X11 root window on Linux.
//! Wayland has no way for a client to grab keys.

use std::sync::Mutex;

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};

use super::model::CaptureAction;

/// The hotkeys in registration order; a backend's hotkey id is the index here.
const HOTKEY_ACTIONS: [CaptureAction; 5] = [
    CaptureAction::TogglePanel,
    CaptureAction::Area,
    CaptureAction::CurrentApp,
    CaptureAction::FullScreen,
    CaptureAction::Prompt,
];

static SENDER: Mutex<Option<UnboundedSender<CaptureAction>>> = Mutex::new(None);

fn deliver(index: usize) {
    let Some(action) = HOTKEY_ACTIONS.get(index).copied() else {
        return;
    };
    if let Ok(sender) = SENDER.lock()
        && let Some(sender) = sender.as_ref()
    {
        let _ = sender.unbounded_send(action);
    }
}

/// Registers the hotkeys and returns the stream of presses, or `None` when this platform cannot
/// register them.
pub(super) fn register() -> Option<UnboundedReceiver<CaptureAction>> {
    let (sender, receiver) = unbounded();
    if let Ok(mut slot) = SENDER.lock() {
        *slot = Some(sender);
    }
    if backend::register() {
        Some(receiver)
    } else {
        unregister();
        None
    }
}

pub(super) fn unregister() {
    backend::unregister();
    if let Ok(mut slot) = SENDER.lock() {
        *slot = None;
    }
}

#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
pub extern "C" fn GhostexGpuiCaptureHotkeyPressed(hotkey_id: u32) {
    deliver(hotkey_id as usize);
}

#[cfg(target_os = "macos")]
mod backend {
    unsafe extern "C" {
        fn GhostexGpuiCaptureRegisterHotkeys(key_codes: *const u32, count: u32) -> u32;
        fn GhostexGpuiCaptureUnregisterHotkeys();
    }

    /// Carbon virtual key codes for S, A, Space, F and T, in `HOTKEY_ACTIONS` order.
    const KEY_CODES: [u32; 5] = [1, 0, 49, 3, 17];

    pub(super) fn register() -> bool {
        unsafe { GhostexGpuiCaptureRegisterHotkeys(KEY_CODES.as_ptr(), KEY_CODES.len() as u32) > 0 }
    }

    pub(super) fn unregister() {
        unsafe { GhostexGpuiCaptureUnregisterHotkeys() };
    }
}

#[cfg(target_os = "windows")]
mod backend {
    use std::sync::atomic::{AtomicU32, Ordering};

    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, RegisterHotKey, UnregisterHotKey, VK_SPACE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetMessageW, MSG, PostThreadMessageW, WM_HOTKEY, WM_QUIT,
    };

    static THREAD_ID: AtomicU32 = AtomicU32::new(0);

    /// Virtual keys for S, A, Space, F and T, in `HOTKEY_ACTIONS` order.
    const KEYS: [u32; 5] = [
        b'S' as u32,
        b'A' as u32,
        VK_SPACE as u32,
        b'F' as u32,
        b'T' as u32,
    ];

    pub(super) fn register() -> bool {
        unregister();
        let (ready_sender, ready) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("ghostex-capture-hotkeys".into())
            .spawn(move || {
                let modifiers = MOD_ALT | MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT;
                let registered = KEYS
                    .iter()
                    .enumerate()
                    .filter(|(index, key)| unsafe {
                        RegisterHotKey(std::ptr::null_mut(), *index as i32 + 1, modifiers, **key)
                            != 0
                    })
                    .count();
                THREAD_ID.store(unsafe { GetCurrentThreadId() }, Ordering::SeqCst);
                let _ = ready_sender.send(registered > 0);
                if registered == 0 {
                    return;
                }
                let mut message: MSG = unsafe { std::mem::zeroed() };
                while unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } > 0 {
                    if message.message == WM_HOTKEY && message.wParam >= 1 {
                        super::deliver(message.wParam - 1);
                    }
                }
                for index in 0..KEYS.len() {
                    unsafe { UnregisterHotKey(std::ptr::null_mut(), index as i32 + 1) };
                }
            });
        spawned.is_ok() && ready.recv().unwrap_or(false)
    }

    pub(super) fn unregister() {
        let thread = THREAD_ID.swap(0, Ordering::SeqCst);
        if thread != 0 {
            unsafe { PostThreadMessageW(thread, WM_QUIT, 0, 0) };
        }
    }
}

#[cfg(target_os = "linux")]
mod backend {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use x11rb::connection::Connection;
    use x11rb::protocol::Event;
    use x11rb::protocol::xproto::{ConnectionExt, GrabMode, ModMask};

    static STOP: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
    use std::sync::Mutex;

    /// Keysyms for s, a, space, f and t, in `HOTKEY_ACTIONS` order.
    const KEYSYMS: [u32; 5] = [0x73, 0x61, 0x20, 0x66, 0x74];

    pub(super) fn register() -> bool {
        unregister();
        if !super::super::platform::supported() {
            return false;
        }
        let stop = Arc::new(AtomicBool::new(false));
        if let Ok(mut slot) = STOP.lock() {
            *slot = Some(stop.clone());
        }
        let (ready_sender, ready) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("ghostex-capture-hotkeys".into())
            .spawn(move || {
                let Ok((connection, screen)) = x11rb::connect(None) else {
                    let _ = ready_sender.send(false);
                    return;
                };
                let setup = connection.setup();
                let Some(root) = setup.roots.get(screen).map(|screen| screen.root) else {
                    let _ = ready_sender.send(false);
                    return;
                };
                let (min, max) = (setup.min_keycode, setup.max_keycode);
                let Some(mapping) = connection
                    .get_keyboard_mapping(min, max - min + 1)
                    .ok()
                    .and_then(|cookie| cookie.reply().ok())
                else {
                    let _ = ready_sender.send(false);
                    return;
                };
                let per = mapping.keysyms_per_keycode as usize;
                let keycode_of = |keysym: u32| {
                    mapping
                        .keysyms
                        .chunks(per.max(1))
                        .position(|syms| syms.contains(&keysym))
                        .map(|index| min + index as u8)
                };
                let keycodes: Vec<Option<u8>> =
                    KEYSYMS.iter().map(|sym| keycode_of(*sym)).collect();
                let base = ModMask::M1 | ModMask::CONTROL | ModMask::SHIFT;
                // Grab with and without Caps Lock and Num Lock, which X counts as modifiers.
                let variants = [
                    base,
                    base | ModMask::LOCK,
                    base | ModMask::M2,
                    base | ModMask::LOCK | ModMask::M2,
                ];
                for keycode in keycodes.iter().flatten() {
                    for modifiers in variants {
                        let _ = connection.grab_key(
                            false,
                            root,
                            modifiers,
                            *keycode,
                            GrabMode::ASYNC,
                            GrabMode::ASYNC,
                        );
                    }
                }
                let _ = connection.flush();
                let _ = ready_sender.send(keycodes.iter().any(Option::is_some));
                while !stop.load(Ordering::Relaxed) {
                    match connection.poll_for_event() {
                        Ok(Some(Event::KeyPress(event))) => {
                            if let Some(index) =
                                keycodes.iter().position(|code| *code == Some(event.detail))
                            {
                                super::deliver(index);
                            }
                        }
                        Ok(Some(_)) => {}
                        Ok(None) => std::thread::sleep(std::time::Duration::from_millis(40)),
                        Err(_) => break,
                    }
                }
                for keycode in keycodes.iter().flatten() {
                    for modifiers in variants {
                        let _ = connection.ungrab_key(*keycode, root, modifiers);
                    }
                }
                let _ = connection.flush();
            });
        spawned.is_ok() && ready.recv().unwrap_or(false)
    }

    pub(super) fn unregister() {
        if let Ok(mut slot) = STOP.lock()
            && let Some(stop) = slot.take()
        {
            stop.store(true, Ordering::Relaxed);
        }
    }
}
