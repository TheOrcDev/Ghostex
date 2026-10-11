//! Keeping the windows a workspace window owns on it while the user moves it.

use gpui::Window;

/// Shifts the windows `window` owns by the distance it moved since it last reported its frame.
/// Called from the workspace window's bounds observer and once when it opens, which records the
/// starting position.
///
/// CDXC:AppModal 2026-10-04 DECISION:
/// User: "I just moved the window while maximize was enabled in the gpui chat view composer and the maximize window stayed in its place even though i moved the app around ... Do same for other similar cases in the app." Every window drawn over a workspace window moves with it. macOS already does this for windows attached with `addChildWindow:`. A Windows owned window keeps its screen position when its owner moves, so every window whose owner chain reaches the moved workspace window is shifted by the same distance: GPUI owns each popup by the window that was active when it opened (the maximized composer and the chat's other dialogs and menus, the app modals and, through them, their corner X, the toasts, the Docs drawer, the floating sidebar panel, the hidden New Thread picker). The windows Ghostex Capture detaches from their owner stay where they are.
/// CDXC:AppModal 2026-10-04 WHY:
/// The shift runs from a task, outside the app update, the way GPUI's own `Window::resize` does: each shifted window's GPUI state takes its new origin from WM_MOVE without being drawn again, which a moved callback arriving during an update would refuse. The Windows move loop runs foreground tasks on its timer, so the windows follow during the drag, not only once it ends. A minimized window's parked frame is not a move, so restoring it moves nothing.
#[cfg(target_os = "windows")]
pub(super) fn follow_owner_window_frame(window: &Window, cx: &gpui::App) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::cell::RefCell;
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, IsIconic};

    thread_local! {
        /// Each workspace window's top-left corner when it last reported, in physical pixels.
        static OWNER_ORIGINS: RefCell<Vec<(isize, (i32, i32))>> = const { RefCell::new(Vec::new()) };
    }

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let owner = handle.hwnd.get();
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: `owner` is the live window this observer belongs to.
    if unsafe { IsIconic(owner as HWND) } != 0
        || unsafe { GetWindowRect(owner as HWND, &mut rect) } == 0
    {
        return;
    }
    let origin = (rect.left, rect.top);
    let previous = OWNER_ORIGINS.with(|origins| {
        let mut origins = origins.borrow_mut();
        match origins.iter_mut().find(|(hwnd, _)| *hwnd == owner) {
            Some((_, last)) => Some(std::mem::replace(last, origin)),
            None => {
                origins.push((owner, origin));
                None
            }
        }
    });
    let Some(previous) = previous.filter(|previous| *previous != origin) else {
        return;
    };
    let (dx, dy) = (origin.0 - previous.0, origin.1 - previous.1);
    cx.foreground_executor()
        .spawn(async move { shift_owned_windows(owner, dx, dy) })
        .detach();
}

#[cfg(not(target_os = "windows"))]
pub(super) fn follow_owner_window_frame(_: &Window, _: &gpui::App) {}

/// Moves every top-level window of this thread whose owner chain reaches `owner` by `(dx, dy)`
/// physical pixels, keeping its size, stacking and activation.
#[cfg(target_os = "windows")]
fn shift_owned_windows(owner: isize, dx: i32, dy: i32) {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumThreadWindows, GW_OWNER, GetWindow, GetWindowRect, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
        SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
    };

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> windows_sys::core::BOOL {
        // SAFETY: `lparam` is the `windows` vector below, alive for the whole EnumThreadWindows call.
        let windows = unsafe { &mut *(lparam as *mut Vec<HWND>) };
        windows.push(hwnd);
        1
    }
    let owner = owner as HWND;
    let owned_by_owner = |mut hwnd: HWND| {
        // Owner chains are short (a menu, its modal, the workspace window); the bound guards a cycle.
        for _ in 0..8 {
            // SAFETY: GetWindow only reads the window's owner.
            hwnd = unsafe { GetWindow(hwnd, GW_OWNER) };
            if hwnd.is_null() {
                return false;
            }
            if hwnd == owner {
                return true;
            }
        }
        false
    };
    let mut windows: Vec<HWND> = Vec::new();
    // SAFETY: `collect` only appends to the vector the pointer names.
    unsafe {
        EnumThreadWindows(
            GetCurrentThreadId(),
            Some(collect),
            &mut windows as *mut Vec<HWND> as LPARAM,
        )
    };
    for hwnd in windows {
        if hwnd == owner || !owned_by_owner(hwnd) {
            continue;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: `hwnd` was enumerated on this thread just above.
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            continue;
        }
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                rect.left + dx,
                rect.top + dy,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE,
            )
        };
    }
}
