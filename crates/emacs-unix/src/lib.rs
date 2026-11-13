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

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn getrandom(buf: &mut [u8]) -> bool {
    let n = unsafe { libc::getrandom(buf.as_mut_ptr().cast(), buf.len(), 0) };
    n == buf.len() as isize
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub fn getrandom(buf: &mut [u8]) -> bool {
    use std::io::Read;
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(buf))
        .is_ok()
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Passwd {
    pub name: Vec<u8>,
    pub passwd: Vec<u8>,
    pub uid: u32,
    pub gid: u32,
    pub gecos: Vec<u8>,
    pub dir: Vec<u8>,
    pub shell: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Group {
    pub name: Vec<u8>,
    pub gid: u32,
}

pub fn getuid() -> u32 {
    unsafe { libc::getuid() }
}

pub fn geteuid() -> u32 {
    unsafe { libc::geteuid() }
}

pub fn getgid() -> u32 {
    unsafe { libc::getgid() }
}

pub fn getegid() -> u32 {
    unsafe { libc::getegid() }
}

fn c_bytes(p: *const libc::c_char) -> Vec<u8> {
    if p.is_null() {
        Vec::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(p) }.to_bytes().to_vec()
    }
}

fn passwd_with(
    lookup: impl Fn(*mut libc::passwd, *mut libc::c_char, usize, *mut *mut libc::passwd) -> libc::c_int,
) -> Option<Passwd> {
    let mut size = 1024usize;
    loop {
        let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
        let mut buf = vec![0 as libc::c_char; size];
        let mut result: *mut libc::passwd = std::ptr::null_mut();
        let rc = lookup(&mut pwd, buf.as_mut_ptr(), size, &mut result);
        if rc == libc::ERANGE {
            size *= 2;
            continue;
        }
        if rc != 0 || result.is_null() {
            return None;
        }
        return Some(Passwd {
            name: c_bytes(pwd.pw_name),
            passwd: c_bytes(pwd.pw_passwd),
            uid: pwd.pw_uid,
            gid: pwd.pw_gid,
            gecos: c_bytes(pwd.pw_gecos),
            dir: c_bytes(pwd.pw_dir),
            shell: c_bytes(pwd.pw_shell),
        });
    }
}

pub fn getpwuid(uid: u32) -> Option<Passwd> {
    passwd_with(|pwd, buf, size, result| unsafe { libc::getpwuid_r(uid, pwd, buf, size, result) })
}

pub fn getpwnam(name: &[u8]) -> Option<Passwd> {
    let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    let name = CString::new(&name[..end]).ok()?;
    passwd_with(|pwd, buf, size, result| unsafe {
        libc::getpwnam_r(name.as_ptr(), pwd, buf, size, result)
    })
}

pub fn getgrgid(gid: u32) -> Option<Group> {
    let mut size = 1024usize;
    loop {
        let mut grp: libc::group = unsafe { std::mem::zeroed() };
        let mut buf = vec![0 as libc::c_char; size];
        let mut result: *mut libc::group = std::ptr::null_mut();
        let rc = unsafe { libc::getgrgid_r(gid, &mut grp, buf.as_mut_ptr(), size, &mut result) };
        if rc == libc::ERANGE {
            size *= 2;
            continue;
        }
        if rc != 0 || result.is_null() {
            return None;
        }
        return Some(Group {
            name: c_bytes(grp.gr_name),
            gid: grp.gr_gid,
        });
    }
}

pub fn gethostname() -> Vec<u8> {
    let mut size = 256usize;
    loop {
        let mut buf = vec![0u8; size];
        unsafe { libc::gethostname(buf.as_mut_ptr().cast(), size - 1) };
        buf[size - 1] = 0;
        let len = buf.iter().position(|&b| b == 0).unwrap_or(size);
        if len < size - 1 {
            buf.truncate(len);
            return buf;
        }
        size += size / 2 + 1;
    }
}
