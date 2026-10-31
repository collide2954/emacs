#![cfg(unix)]

use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub use libc::O_NOFOLLOW;
pub use libc::{EEXIST, EINVAL, ELOOP, ENAMETOOLONG, ENOENT, ENOSYS, ENOTDIR, EOVERFLOW, EPERM};

pub const PID_MAX: i64 = libc::pid_t::MAX as i64;

pub struct Utmpx {
    pub boot_time: bool,
    pub user: Vec<u8>,
    pub line: Vec<u8>,
    pub tv_sec: i64,
    pub tv_usec: i64,
}

fn c_path(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))
}

fn result(r: libc::c_int) -> io::Result<()> {
    if r == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn kill(pid: i64, sig: i32) -> io::Result<()> {
    result(unsafe { libc::kill(pid as libc::pid_t, sig) })
}

pub fn strsignal(sig: i32) -> Option<String> {
    let name = unsafe { libc::strsignal(sig) };
    if name.is_null() {
        None
    } else {
        Some(
            unsafe { std::ffi::CStr::from_ptr(name) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

pub fn faccessat_eaccess(path: &Path, mode: i32) -> io::Result<()> {
    let path = c_path(path)?;
    result(unsafe { libc::faccessat(libc::AT_FDCWD, path.as_ptr(), mode, libc::AT_EACCESS) })
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn rename_noreplace(old: &Path, new: &Path) -> io::Result<()> {
    let old = c_path(old)?;
    let new = c_path(new)?;
    let r = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            old.as_ptr(),
            libc::AT_FDCWD,
            new.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if r == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
pub fn rename_noreplace(old: &Path, new: &Path) -> io::Result<()> {
    let old = c_path(old)?;
    let new = c_path(new)?;
    result(unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            old.as_ptr(),
            libc::AT_FDCWD,
            new.as_ptr(),
            libc::RENAME_EXCL,
        )
    })
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
pub fn rename_noreplace(old: &Path, new: &Path) -> io::Result<()> {
    c_path(old)?;
    c_path(new)?;
    Err(io::Error::from_raw_os_error(libc::ENOSYS))
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn c_chars(chars: &[libc::c_char]) -> Vec<u8> {
    chars
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect()
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
))]
pub fn utmpx_entries() -> Vec<Utmpx> {
    let mut entries = Vec::new();
    unsafe { libc::setutxent() };
    loop {
        let entry = unsafe { libc::getutxent() };
        if entry.is_null() {
            break;
        }
        let ut = unsafe { &*entry };
        entries.push(Utmpx {
            boot_time: ut.ut_type == libc::BOOT_TIME,
            user: c_chars(&ut.ut_user),
            line: c_chars(&ut.ut_line),
            tv_sec: ut.ut_tv.tv_sec as i64,
            tv_usec: ut.ut_tv.tv_usec as i64,
        });
    }
    unsafe { libc::endutxent() };
    entries
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
)))]
pub fn utmpx_entries() -> Vec<Utmpx> {
    Vec::new()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn clock_boottime() -> Option<(i64, i64)> {
    let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
    (unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) } >= 0)
        .then_some((ts.tv_sec as i64, ts.tv_nsec as i64))
}

#[cfg(target_os = "linux")]
pub fn sysinfo_uptime() -> Option<i64> {
    let mut info: libc::sysinfo = unsafe { std::mem::zeroed() };
    (unsafe { libc::sysinfo(&mut info) } >= 0).then_some(info.uptime as i64)
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub fn sysctl_boottime() -> Option<(i64, i64)> {
    let mut request = [libc::CTL_KERN, libc::KERN_BOOTTIME];
    let mut tv: libc::timeval = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::timeval>();
    let r = unsafe {
        libc::sysctl(
            request.as_mut_ptr(),
            2,
            (&mut tv as *mut libc::timeval).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (r >= 0).then_some((tv.tv_sec as i64, tv.tv_usec as i64 * 1000))
}
