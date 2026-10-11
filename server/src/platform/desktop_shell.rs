//! Asking the signed-in user's Explorer shell to start a process for us.

use std::ffi::c_void;

/// The `nShowCmd` Explorer passes to the new process.
#[derive(Clone, Copy)]
pub(crate) enum ShowWindow {
    Hidden = 0,
}

#[repr(C)]
struct Guid(u32, u16, u16, [u8; 8]);

/// A 24-byte OLE VARIANT holding VT_EMPTY, VT_I4 or VT_BSTR.
#[repr(C)]
struct Variant {
    vt: u16,
    reserved: [u16; 3],
    data: [u64; 2],
}

const VT_EMPTY: u16 = 0;
const VT_I4: u16 = 3;
const VT_BSTR: u16 = 8;

impl Variant {
    fn new(vt: u16, data: u64) -> Self {
        Self {
            vt,
            reserved: [0; 3],
            data: [data, 0],
        }
    }
}

const CLSID_SHELL_WINDOWS: Guid = Guid(
    0x9BA0_5972,
    0xF6A8,
    0x11CF,
    [0xA4, 0x42, 0x00, 0xA0, 0xC9, 0x0A, 0x8F, 0x39],
);
const IID_ISHELL_WINDOWS: Guid = Guid(
    0x85CB_6900,
    0x4D95,
    0x11CF,
    [0x96, 0x0C, 0x00, 0x80, 0xC7, 0xF4, 0xEE, 0x85],
);
const IID_ISERVICE_PROVIDER: Guid = Guid(
    0x6D51_40C1,
    0x7436,
    0x11CE,
    [0x80, 0x34, 0x00, 0xAA, 0x00, 0x60, 0x09, 0xFA],
);
const SID_STOP_LEVEL_BROWSER: Guid = Guid(
    0x4C96_BE40,
    0x915C,
    0x11CF,
    [0x99, 0xD3, 0x00, 0xAA, 0x00, 0x4A, 0xE8, 0x37],
);
const IID_ISHELL_BROWSER: Guid = Guid(0x0002_14E2, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
const IID_IDISPATCH: Guid = Guid(0x0002_0400, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
const IID_ISHELL_FOLDER_VIEW_DUAL: Guid = Guid(
    0xE7A1_AF80,
    0x4D96,
    0x11CF,
    [0x96, 0x0C, 0x00, 0x80, 0xC7, 0xF4, 0xEE, 0x85],
);
const IID_ISHELL_DISPATCH2: Guid = Guid(
    0xA4C6_892C,
    0x3BA9,
    0x11D2,
    [0x9D, 0xEA, 0x00, 0xC0, 0x4F, 0xB1, 0x61, 0x62],
);

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, flags: u32) -> i32;
    fn CoUninitialize();
    fn CoCreateInstance(
        clsid: *const Guid,
        outer: *mut c_void,
        context: u32,
        iid: *const Guid,
        out: *mut *mut c_void,
    ) -> i32;
}

#[link(name = "oleaut32")]
extern "system" {
    fn SysAllocString(text: *const u16) -> *mut u16;
    fn SysFreeString(text: *mut u16);
}

/// An owned COM interface pointer, released on drop.
struct Com(*mut c_void);

impl Com {
    /// The `index`th entry of the object's vtable, cast to the method's signature `F`.
    unsafe fn method<F: Copy>(&self, index: usize) -> F {
        let vtable = *(self.0 as *const *const *const c_void);
        std::mem::transmute_copy(&*vtable.add(index))
    }

    unsafe fn query(&self, iid: &Guid) -> std::io::Result<Com> {
        let query: unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32 =
            self.method(0);
        let mut out = std::ptr::null_mut();
        check(query(self.0, iid, &mut out), "QueryInterface")?;
        Ok(Com(out))
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            let release: unsafe extern "system" fn(*mut c_void) -> u32 = self.method(2);
            release(self.0);
        }
    }
}

fn check(hresult: i32, call: &str) -> std::io::Result<()> {
    if hresult < 0 {
        return Err(std::io::Error::other(format!(
            "{call} failed with HRESULT 0x{:08X}",
            hresult as u32
        )));
    }
    Ok(())
}

struct Bstr(*mut u16);

impl Bstr {
    fn new(text: &std::ffi::OsStr) -> Self {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = text.encode_wide().chain(Some(0)).collect();
        Self(unsafe { SysAllocString(wide.as_ptr()) })
    }
}

impl Drop for Bstr {
    fn drop(&mut self) {
        unsafe { SysFreeString(self.0) };
    }
}

/// Starts `file` the way the Start menu does: the running Explorer shell creates the process (IShellDispatch2::ShellExecute on the desktop's shell view), so it belongs to no job of this process's callers, inherits none of their handles, and gets the signed-in user's token and environment. Only works from a process in the same desktop session as that Explorer.
///
/// SEE-ALSO: `shell_execute_from_desktop` in tooling/xtask/src/start/windows_native.rs, the same COM walk for the dev start.
pub(crate) fn shell_execute_from_desktop(
    file: &std::path::Path,
    arguments: &str,
    directory: &std::path::Path,
    show: ShowWindow,
) -> std::io::Result<()> {
    const COINIT_APARTMENTTHREADED: u32 = 0x2;
    const CLSCTX_ALL: u32 = 0x17;
    const CSIDL_DESKTOP: u64 = 0;
    const SWC_DESKTOP: i32 = 8;
    const SWFO_NEEDDISPATCH: i32 = 1;
    const SVGIO_BACKGROUND: u32 = 0;
    unsafe {
        let initialized = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED) >= 0;
        let result = (|| {
            let mut out = std::ptr::null_mut();
            check(
                CoCreateInstance(
                    &CLSID_SHELL_WINDOWS,
                    std::ptr::null_mut(),
                    CLSCTX_ALL,
                    &IID_ISHELL_WINDOWS,
                    &mut out,
                ),
                "CoCreateInstance(ShellWindows)",
            )?;
            let shell_windows = Com(out);
            // IShellWindows::FindWindowSW
            let find_window: unsafe extern "system" fn(
                *mut c_void,
                *const Variant,
                *const Variant,
                i32,
                *mut i32,
                i32,
                *mut *mut c_void,
            ) -> i32 = shell_windows.method(15);
            let location = Variant::new(VT_I4, CSIDL_DESKTOP);
            let root = Variant::new(VT_EMPTY, 0);
            let mut hwnd = 0i32;
            let mut out = std::ptr::null_mut();
            let found = find_window(
                shell_windows.0,
                &location,
                &root,
                SWC_DESKTOP,
                &mut hwnd,
                SWFO_NEEDDISPATCH,
                &mut out,
            );
            check(found, "IShellWindows::FindWindowSW")?;
            if out.is_null() {
                return Err(std::io::Error::other(
                    "Explorer's desktop window was not found; is Explorer running as the shell?",
                ));
            }
            let desktop = Com(out);
            let services = desktop.query(&IID_ISERVICE_PROVIDER)?;
            // IServiceProvider::QueryService
            let query_service: unsafe extern "system" fn(
                *mut c_void,
                *const Guid,
                *const Guid,
                *mut *mut c_void,
            ) -> i32 = services.method(3);
            let mut out = std::ptr::null_mut();
            check(
                query_service(
                    services.0,
                    &SID_STOP_LEVEL_BROWSER,
                    &IID_ISHELL_BROWSER,
                    &mut out,
                ),
                "QueryService(IShellBrowser)",
            )?;
            let browser = Com(out);
            // IShellBrowser::QueryActiveShellView
            let active_view: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32 =
                browser.method(15);
            let mut out = std::ptr::null_mut();
            check(
                active_view(browser.0, &mut out),
                "IShellBrowser::QueryActiveShellView",
            )?;
            let view = Com(out);
            // IShellView::GetItemObject
            let item_object: unsafe extern "system" fn(
                *mut c_void,
                u32,
                *const Guid,
                *mut *mut c_void,
            ) -> i32 = view.method(15);
            let mut out = std::ptr::null_mut();
            check(
                item_object(view.0, SVGIO_BACKGROUND, &IID_IDISPATCH, &mut out),
                "IShellView::GetItemObject",
            )?;
            let background = Com(out);
            let folder_view = background.query(&IID_ISHELL_FOLDER_VIEW_DUAL)?;
            // IShellFolderViewDual::get_Application
            let application: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32 =
                folder_view.method(7);
            let mut out = std::ptr::null_mut();
            check(
                application(folder_view.0, &mut out),
                "IShellFolderViewDual::get_Application",
            )?;
            let shell = Com(out).query(&IID_ISHELL_DISPATCH2)?;
            // IShellDispatch2::ShellExecute
            let shell_execute: unsafe extern "system" fn(
                *mut c_void,
                *mut u16,
                Variant,
                Variant,
                Variant,
                Variant,
            ) -> i32 = shell.method(31);
            let file = Bstr::new(file.as_os_str());
            let arguments = Bstr::new(std::ffi::OsStr::new(arguments));
            let directory = Bstr::new(directory.as_os_str());
            check(
                shell_execute(
                    shell.0,
                    file.0,
                    Variant::new(VT_BSTR, arguments.0 as u64),
                    Variant::new(VT_BSTR, directory.0 as u64),
                    Variant::new(VT_EMPTY, 0),
                    Variant::new(VT_I4, show as u64),
                ),
                "IShellDispatch2::ShellExecute",
            )
        })();
        if initialized {
            CoUninitialize();
        }
        result
    }
}
