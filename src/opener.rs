//! Handing a link or a folder to the desktop.
//!
//! Everywhere but Windows this is the `open` crate, which picks the right
//! launcher for the desktop it finds. Windows gets `ShellExecuteW`, the
//! call Explorer itself makes when someone clicks a link. Calling it directly
//! keeps percent escapes in OAuth URLs intact and works on installations where
//! PowerShell is not available on `PATH`.

use std::ffi::OsStr;
use std::io;

/// Opens a URL or a path in whatever the desktop uses for it.
#[cfg(not(windows))]
pub fn open(target: impl AsRef<OsStr>) -> io::Result<()> {
    open::that(target.as_ref())
}

/// Opens a URL or a path in whatever the desktop uses for it.
#[cfg(windows)]
pub fn open(target: impl AsRef<OsStr>) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn wide(text: &OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    // A shell extension can be a COM client. The opener runs on worker
    // threads, so initialize the apartment before handing Explorer a target.
    // An already initialized thread simply keeps its existing apartment.
    unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };

    let file = wide(target.as_ref());
    let verb = wide(OsStr::new("open"));
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // Anything above 32 opened. At or below it the value is one of the
    // historical SE_ERR_* codes; expose the platform error to the caller.
    if result as isize > 32 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
