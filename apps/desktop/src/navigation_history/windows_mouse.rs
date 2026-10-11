use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;

use gpui::{AsyncApp, WeakEntity};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GA_ROOT, GetAncestor, HHOOK, MSG, PM_REMOVE, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_GETMESSAGE, WM_NULL, WM_XBUTTONDBLCLK, WM_XBUTTONDOWN, WM_XBUTTONUP,
    XBUTTON1, XBUTTON2,
};

use crate::GhostexGpuiApp;
use crate::app::window::GpuiAppModalHostWindow;

#[derive(Clone)]
struct NavigationTarget {
    view: NavigationView,
    async_app: AsyncApp,
}

#[derive(Clone)]
enum NavigationView {
    Main(WeakEntity<GhostexGpuiApp>),
    Modal(WeakEntity<GpuiAppModalHostWindow>),
}

#[derive(Default)]
struct MouseNavigation {
    hook: HHOOK,
    targets: HashMap<usize, NavigationTarget>,
}

thread_local! {
    static MOUSE_NAVIGATION: RefCell<MouseNavigation> = RefCell::default();
}

/// CDXC:Navigation 2026-09-28 DECISION:
/// User: enable the mouse Back/Forward buttons on Windows, using the existing Ghostex navigation history; Settings owns its separate page history.
pub(crate) fn register(
    root_view: *mut c_void,
    app: WeakEntity<GhostexGpuiApp>,
    async_app: AsyncApp,
) {
    register_target(
        root_view,
        NavigationTarget {
            view: NavigationView::Main(app),
            async_app,
        },
    );
}

pub(crate) fn register_modal(
    root_view: *mut c_void,
    modal: WeakEntity<GpuiAppModalHostWindow>,
    async_app: AsyncApp,
) {
    register_target(
        root_view,
        NavigationTarget {
            view: NavigationView::Modal(modal),
            async_app,
        },
    );
}

fn register_target(root_view: *mut c_void, target: NavigationTarget) {
    if root_view.is_null() {
        return;
    }
    MOUSE_NAVIGATION.with(|state| {
        let mut state = state.borrow_mut();
        if state.hook.is_null() {
            // CEF child HWNDs receive their own input and do not bubble GPUI
            // events. Observe only this UI thread, then match the owning main
            // window below. Each modal has its own registered target.
            state.hook = unsafe {
                SetWindowsHookExW(
                    WH_GETMESSAGE,
                    Some(mouse_message),
                    std::ptr::null_mut(),
                    GetCurrentThreadId(),
                )
            };
            if state.hook.is_null() {
                eprintln!(
                    "Could not install Ghostex mouse navigation: {}",
                    std::io::Error::last_os_error()
                );
                return;
            }
        }
        state.targets.insert(root_view as usize, target);
    });
}

pub(crate) fn unregister(root_view: *mut c_void) {
    MOUSE_NAVIGATION.with(|state| {
        let mut state = state.borrow_mut();
        state.targets.remove(&(root_view as usize));
        if state.targets.is_empty() && !state.hook.is_null() {
            unsafe { UnhookWindowsHookEx(state.hook) };
            state.hook = std::ptr::null_mut();
        }
    });
}

unsafe extern "system" fn mouse_message(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if code >= 0 && w_param == PM_REMOVE as usize && l_param != 0 {
        let message = unsafe { &mut *(l_param as *mut MSG) };
        if matches!(
            message.message,
            WM_XBUTTONDOWN | WM_XBUTTONDBLCLK | WM_XBUTTONUP
        ) {
            let button = ((message.wParam >> 16) & 0xffff) as u16;
            if button == XBUTTON1 || button == XBUTTON2 {
                let root = unsafe { GetAncestor(message.hwnd, GA_ROOT) };
                let target = MOUSE_NAVIGATION
                    .with(|state| state.borrow().targets.get(&(root as usize)).cloned());
                if let Some(target) = target {
                    let released = message.message == WM_XBUTTONUP;
                    // Consume both halves so Chromium cannot also walk its URL
                    // history. PM_REMOVE avoids navigating twice on PeekMessage.
                    message.message = WM_NULL;
                    if released {
                        navigate(target, button == XBUTTON1);
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param) }
}

fn navigate(target: NavigationTarget, back: bool) {
    let NavigationTarget {
        view,
        mut async_app,
    } = target;
    let foreground = async_app.foreground_executor().clone();
    foreground
        .spawn(async move {
            match view {
                NavigationView::Main(app) => {
                    let _ = app.update_in(&mut async_app, |this, _, cx| {
                        if this.app_modal_window.is_none() {
                            this.navigate_history_from_input(back, cx);
                        }
                    });
                }
                // The CEF modal window hosts only extension pages now; the press is
                // consumed so Chromium cannot walk the page's URL history.
                // Native Settings walks its pages from GPUI's own mouse event.
                NavigationView::Modal(_) => {}
            }
        })
        .detach();
}
