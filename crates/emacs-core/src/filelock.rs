use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("lock-file", 1, 1, f_lock_file),
    Subr::fixed("unlock-file", 1, 1, f_unlock_file),
    Subr::fixed("lock-buffer", 0, 1, f_lock_buffer),
    Subr::fixed("unlock-buffer", 0, 0, f_unlock_buffer),
    Subr::fixed("file-locked-p", 1, 1, f_file_locked_p),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::TEMPORARY_FILE_DIRECTORY, Value::NIL);
    lisp.defvar_bool(sym::CREATE_LOCKFILES, true);
}

const MAX_LFINFO: usize = 8 * 1024;

struct LockInfo {
    user: Vec<u8>,
    at: usize,
    dot: usize,
    colon: usize,
}

enum Owner {
    Free,
    Another(LockInfo),
    Mine,
    Error(io::Error),
}

fn get_boot_sec() -> i64 {
    crate::boot_time::get_boot_time().map_or(0, |t| t.0)
}

fn einval() -> io::Error {
    io::Error::from(io::ErrorKind::InvalidInput)
}

fn enametoolong() -> io::Error {
    io::Error::from(io::ErrorKind::InvalidFilename)
}

fn missing(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

fn integer_prefixed(s: &[u8]) -> bool {
    let i = usize::from(s.first() == Some(&b'-'));
    s.get(i).is_some_and(u8::is_ascii_digit)
}

fn strtoimax(s: &[u8]) -> (i64, usize, bool) {
    let mut i = 0;
    let negative = match s.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    let mut value: i64 = 0;
    let mut overflow = false;
    while let Some(&b) = s.get(i)
        && b.is_ascii_digit()
    {
        let d = i64::from(b - b'0');
        let next = value.checked_mul(10).and_then(|v| {
            if negative {
                v.checked_sub(d)
            } else {
                v.checked_add(d)
            }
        });
        match next {
            Some(v) if !overflow => value = v,
            _ => {
                overflow = true;
                value = if negative { i64::MIN } else { i64::MAX };
            }
        }
        i += 1;
    }
    (value, i, overflow)
}

fn within_one_second(a: i64, b: i64) -> bool {
    a.checked_sub(b).is_some_and(|d| (-1..=1).contains(&d))
}

#[cfg(unix)]
fn valid_process_id(p: i64) -> bool {
    0 < p && p <= emacs_unix::PID_MAX
}

#[cfg(windows)]
fn valid_process_id(p: i64) -> bool {
    !(p == -1 || p == 0 || p > i64::from(u32::MAX) || p < i64::from(i32::MIN))
}

#[cfg(unix)]
fn process_exists(pid: i64) -> bool {
    match emacs_unix::kill(pid, 0) {
        Ok(()) => true,
        Err(e) => e.raw_os_error() == Some(emacs_unix::EPERM),
    }
}

#[cfg(windows)]
fn process_exists(pid: i64) -> bool {
    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_INVALID_PARAMETER: u32 = 87;
    let orig_pid = pid as i32;
    let pid = orig_pid.wrapping_abs();
    if pid <= 4 {
        return true;
    }
    match emacs_w32::open_process_query(pid as u32) {
        Err(ERROR_ACCESS_DENIED) => true,
        Err(ERROR_INVALID_PARAMETER) => {
            if orig_pid == pid {
                return false;
            }
            !matches!(
                emacs_w32::open_process_query(orig_pid as u32),
                Err(ERROR_INVALID_PARAMETER)
            )
        }
        _ => true,
    }
}

#[cfg(unix)]
fn read_lock_data(path: &Path) -> io::Result<Vec<u8>> {
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::OpenOptionsExt;
    loop {
        match std::fs::read_link(path) {
            Ok(target) => {
                let mut bytes = target.into_os_string().into_vec();
                bytes.truncate(MAX_LFINFO + 1);
                return Ok(bytes);
            }
            Err(e) if e.raw_os_error() == Some(emacs_unix::EINVAL) => {
                match std::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(emacs_unix::O_NOFOLLOW)
                    .open(path)
                {
                    Ok(f) => {
                        let mut bytes = Vec::new();
                        f.take((MAX_LFINFO + 1) as u64).read_to_end(&mut bytes)?;
                        return Ok(bytes);
                    }
                    Err(e) if e.raw_os_error() == Some(emacs_unix::ELOOP) => continue,
                    Err(e) => return Err(e),
                }
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(windows)]
fn read_lock_data(path: &Path) -> io::Result<Vec<u8>> {
    match std::fs::read_link(path) {
        Ok(target) => {
            let mut bytes = target.to_string_lossy().into_owned().into_bytes();
            bytes.truncate(MAX_LFINFO + 1);
            Ok(bytes)
        }
        Err(e) if missing(&e) => Err(e),
        Err(_) => {
            let f = File::open(path)?;
            let mut bytes = Vec::new();
            f.take((MAX_LFINFO + 1) as u64).read_to_end(&mut bytes)?;
            Ok(bytes)
        }
    }
}

#[cfg(unix)]
fn rename_lock_file(old: &Path, new: &Path, force: bool) -> io::Result<()> {
    if !force {
        match emacs_unix::rename_noreplace(old, new) {
            Err(e) if e.raw_os_error() == Some(emacs_unix::ENOSYS) => {}
            r => return r,
        }
        match std::fs::hard_link(old, new) {
            Ok(()) => {
                return match std::fs::remove_file(old) {
                    Err(e) if e.raw_os_error() != Some(emacs_unix::ENOENT) => Err(e),
                    _ => Ok(()),
                };
            }
            Err(e)
                if e.raw_os_error() != Some(emacs_unix::ENOSYS)
                    && e.raw_os_error() != Some(emacs_unix::EPERM) =>
            {
                return Err(e);
            }
            Err(_) => {}
        }
        match std::fs::symlink_metadata(new) {
            Ok(_) => return Err(io::Error::from_raw_os_error(emacs_unix::EEXIST)),
            Err(e) if e.raw_os_error() == Some(emacs_unix::EOVERFLOW) => {
                return Err(io::Error::from_raw_os_error(emacs_unix::EEXIST));
            }
            Err(e) if e.raw_os_error() != Some(emacs_unix::ENOENT) => return Err(e),
            Err(_) => {}
        }
    }
    std::fs::rename(old, new)
}

#[cfg(windows)]
fn rename_lock_file(old: &Path, new: &Path, force: bool) -> io::Result<()> {
    match emacs_w32::move_file_no_replace(old, new) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists && force => {
            let mut perms = std::fs::metadata(new)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            std::fs::set_permissions(new, perms)?;
            if std::fs::metadata(new)?.is_dir() {
                std::fs::remove_dir(new)?;
            } else {
                std::fs::remove_file(new)?;
            }
            emacs_w32::move_file_no_replace(old, new)
        }
        r => r,
    }
}

#[cfg(unix)]
fn symlink_error(info: &[u8], lfname: &Path, force: bool) -> Option<io::Error> {
    use std::os::unix::ffi::OsStrExt;
    let target = std::ffi::OsStr::from_bytes(info);
    let mut err = std::os::unix::fs::symlink(target, lfname).err();
    if force
        && let Some(e) = &err
        && e.raw_os_error() == Some(emacs_unix::EEXIST)
    {
        let _ = std::fs::remove_file(lfname);
        err = std::os::unix::fs::symlink(target, lfname).err();
    }
    err
}

#[cfg(unix)]
fn use_nonce(err: &io::Error) -> bool {
    [
        emacs_unix::ENOSYS,
        emacs_unix::EPERM,
        emacs_unix::ENAMETOOLONG,
    ]
    .iter()
    .any(|&code| err.raw_os_error() == Some(code))
}

#[cfg(unix)]
fn create_nonce(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(windows)]
fn create_nonce(path: &Path) -> io::Result<File> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
}

#[cfg(unix)]
fn fchmod_lock(file: &File) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(0o644))
}

#[cfg(windows)]
fn fchmod_lock(_file: &File) -> io::Result<()> {
    Ok(())
}

const TEMPNAME_LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

impl Lisp {
    fn mkostemp(&mut self, template: &str) -> io::Result<(File, PathBuf)> {
        let prefix = &template[..template.len() - 6];
        for _ in 0..62 * 62 * 62 {
            let mut v = self.next_random();
            let mut name = prefix.to_string();
            for _ in 0..6 {
                name.push(TEMPNAME_LETTERS[(v % 62) as usize] as char);
                v /= 62;
            }
            let path = PathBuf::from(&name);
            match create_nonce(&path) {
                Ok(f) => return Ok((f, path)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::from(io::ErrorKind::AlreadyExists))
    }

    fn create_lock_file(&mut self, lfname: &str, info: &[u8], force: bool) -> io::Result<()> {
        let path = Path::new(lfname);
        #[cfg(unix)]
        let err = symlink_error(info, path, force);
        #[cfg(windows)]
        let err: Option<io::Error> = None;
        #[cfg(unix)]
        let nonce_wanted = err.as_ref().is_some_and(use_nonce);
        #[cfg(windows)]
        let nonce_wanted = true;
        if !nonce_wanted {
            return err.map_or(Ok(()), Err);
        }
        let lfdirlen = lfname.rfind('/').map_or(0, |i| i + 1);
        let template = format!("{}.#-emacsXXXXXX", &lfname[..lfdirlen]);
        let (mut file, nonce) = self.mkostemp(&template)?;
        let mut err = file.write_all(info).and_then(|()| fchmod_lock(&file)).err();
        drop(file);
        if err.is_none() {
            err = rename_lock_file(&nonce, path, force).err();
        }
        if let Some(e) = err {
            let _ = std::fs::remove_file(&nonce);
            return Err(e);
        }
        Ok(())
    }

    fn lock_file_1(&mut self, lfname: Value, force: bool) -> LispResult<io::Result<()>> {
        let boot = get_boot_sec();
        let luser_name = crate::editfns::f_user_login_name(self, &[Value::NIL])?;
        let lhost_name = crate::editfns::f_system_name(self, &[])?;
        let user_name = if luser_name.is_string() {
            self.string_bytes(luser_name).to_vec()
        } else {
            Vec::new()
        };
        let host_name = if lhost_name.is_string() {
            self.string_bytes(lhost_name).to_vec()
        } else {
            Vec::new()
        };
        let pid = std::process::id() as i64;
        let mut info = user_name;
        info.push(b'@');
        if info.len() > MAX_LFINFO {
            return Ok(Err(enametoolong()));
        }
        for &b in &host_name {
            if info.len() >= MAX_LFINFO {
                return Ok(Err(enametoolong()));
            }
            info.push(if b == b'@' { b'-' } else { b });
        }
        let suffix = if boot != 0 {
            format!(".{pid}:{boot}")
        } else {
            format!(".{pid}")
        };
        if suffix.len() >= MAX_LFINFO + 1 - info.len() {
            return Ok(Err(enametoolong()));
        }
        info.extend_from_slice(suffix.as_bytes());
        let lfname = self.string_to_rust(lfname);
        Ok(self.create_lock_file(&lfname, &info, force))
    }

    fn current_lock_owner(&mut self, lfname: Value) -> LispResult<Owner> {
        let path = PathBuf::from(self.string_to_rust(lfname));
        let user = match read_lock_data(&path) {
            Ok(user) => user,
            Err(e) if missing(&e) => return Ok(Owner::Free),
            Err(e) => return Ok(Owner::Error(e)),
        };
        let len = user.len();
        if len > 0 {
            if MAX_LFINFO < len {
                return Ok(Owner::Error(enametoolong()));
            }
            let Some(at) = user.iter().rposition(|&b| b == b'@') else {
                return Ok(Owner::Error(einval()));
            };
            let at_end = user[at..]
                .iter()
                .position(|&b| b == 0)
                .map_or(len, |p| at + p);
            let Some(dot) = user[at..at_end]
                .iter()
                .rposition(|&b| b == b'.')
                .map(|p| at + p)
            else {
                return Ok(Owner::Error(einval()));
            };
            if !integer_prefixed(&user[dot + 1..]) {
                return Ok(Owner::Error(einval()));
            }
            let (pid, pid_len, overflow) = strtoimax(&user[dot + 1..]);
            let pid = if overflow { -1 } else { pid };
            let colon = dot + 1 + pid_len;
            let boot = match user.get(colon) {
                None | Some(0) => None,
                Some(0o357) => {
                    if !(user.get(colon + 1) == Some(&0o200) && user.get(colon + 2) == Some(&0o242))
                    {
                        return Ok(Owner::Error(einval()));
                    }
                    Some(colon + 3)
                }
                Some(b':') => Some(colon + 1),
                Some(_) => return Ok(Owner::Error(einval())),
            };
            let (boot_time, lfinfo_end) = match boot {
                None => (0, colon),
                Some(boot) => {
                    if !integer_prefixed(&user[boot..]) {
                        return Ok(Owner::Error(einval()));
                    }
                    let (boot_time, n, _) = strtoimax(&user[boot..]);
                    (boot_time, boot + n)
                }
            };
            if lfinfo_end != len {
                return Ok(Owner::Error(einval()));
            }
            let linkhost = &user[at + 1..dot];
            let system_name = crate::editfns::f_system_name(self, &[])?;
            let on_current_host = if system_name.is_nil() {
                linkhost.is_empty()
            } else {
                let sysname = self.string_bytes(system_name);
                linkhost.len() == sysname.len()
                    && linkhost
                        .iter()
                        .zip(sysname)
                        .all(|(&l, &s)| l == if s == b'@' { b'-' } else { s })
            };
            if !on_current_host {
                return Ok(Owner::Another(LockInfo {
                    user,
                    at,
                    dot,
                    colon,
                }));
            }
            if pid == std::process::id() as i64 {
                return Ok(Owner::Mine);
            }
            if valid_process_id(pid)
                && process_exists(pid)
                && (boot_time == 0 || within_one_second(boot_time, get_boot_sec()))
            {
                return Ok(Owner::Another(LockInfo {
                    user,
                    at,
                    dot,
                    colon,
                }));
            }
        }
        Ok(match std::fs::remove_file(&path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Owner::Error(e),
            _ => Owner::Free,
        })
    }

    fn lock_if_free(&mut self, lfname: Value) -> LispResult<Owner> {
        loop {
            match self.lock_file_1(lfname, false)? {
                Ok(()) => return Ok(Owner::Free),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    match self.current_lock_owner(lfname)? {
                        Owner::Free => continue,
                        Owner::Mine => return Ok(Owner::Free),
                        owner => return Ok(owner),
                    }
                }
                Err(e) => return Ok(Owner::Error(e)),
            }
        }
    }

    fn make_lock_file_name(&mut self, fn_: Value) -> LispResult {
        let fn_ = self.expand_file_name(fn_, Value::NIL)?;
        self.stack.push(fn_);
        let lock_file_name = self.funcall(sym::MAKE_LOCK_FILE_NAME, &[fn_]);
        self.stack.pop();
        lock_file_name
    }

    fn lock_file(&mut self, fn_: Value) -> LispResult<()> {
        let base = self.stack.len();
        let r = self.lock_file_internal(fn_);
        self.stack.truncate(base);
        r
    }

    fn lock_file_internal(&mut self, fn_: Value) -> LispResult<()> {
        let mut lfname = Value::NIL;
        if self.var_true(sym::CREATE_LOCKFILES) {
            lfname = self.make_lock_file_name(fn_)?;
            if lfname.is_nil() {
                return Ok(());
            }
            self.stack.push(lfname);
        }
        let subject_buf = crate::buffer::f_get_truename_buffer(self, &[fn_])?;
        if !subject_buf.is_nil()
            && crate::fileio::f_verify_visited_file_modtime(self, &[subject_buf])?.is_nil()
            && !crate::fileio::f_file_exists_p(self, &[fn_])?.is_nil()
            && !(!lfname.is_nil() && matches!(self.current_lock_owner(lfname)?, Owner::Mine))
        {
            self.funcall(sym::USERLOCK__ASK_USER_ABOUT_SUPERSESSION_THREAT, &[fn_])?;
        }
        if !lfname.is_nil()
            && let Owner::Another(info) = self.lock_if_free(lfname)?
        {
            let mut attack = info.user[..info.dot].to_vec();
            attack.extend_from_slice(b" (pid ");
            attack.extend_from_slice(&info.user[info.dot + 1..info.colon]);
            attack.push(b')');
            let attack = self.make_string(&attack);
            let attack = self.funcall(sym::ASK_USER_ABOUT_LOCK, &[fn_, attack])?;
            if !attack.is_nil() {
                let _ = self.lock_file_1(lfname, true)?;
            }
        }
        Ok(())
    }

    fn unlock_file(&mut self, fn_: Value) -> LispResult {
        let lfname = self.make_lock_file_name(fn_)?;
        if lfname.is_nil() {
            return Ok(Value::NIL);
        }
        let base = self.stack.len();
        self.stack.push(lfname);
        let owner = self.current_lock_owner(lfname);
        self.stack.truncate(base);
        let err = match owner? {
            Owner::Free | Owner::Another(_) => None,
            Owner::Mine => {
                let path = PathBuf::from(self.string_to_rust(lfname));
                match std::fs::remove_file(&path) {
                    Err(e) if e.kind() != io::ErrorKind::NotFound => Some(e),
                    _ => None,
                }
            }
            Owner::Error(e) => Some(e),
        };
        if let Some(e) = err {
            return Err(self.file_error("Unlocking file", &e, fn_));
        }
        Ok(Value::NIL)
    }

    pub fn unlock_all_files(&mut self) -> LispResult<()> {
        let buffers = self.buffers.clone();
        let base = self.stack.len();
        self.stack.extend_from_slice(&buffers);
        for b in buffers {
            if !self.buffer_live_p(b) {
                continue;
            }
            let truename = self
                .buffer_local_get(b, sym::BUFFER_FILE_TRUENAME)
                .unwrap_or(Value::NIL);
            let text = &self.buffer(b).text;
            if truename.is_string() && text.save_modiff < text.modiff {
                f_unlock_file(self, &[truename])?;
            }
        }
        self.stack.truncate(base);
        Ok(())
    }

    pub(crate) fn unlock_buffer(&mut self, buffer: Value) -> LispResult<()> {
        let text = &self.buffer(buffer).text;
        let modified = text.save_modiff < text.modiff;
        let truename = self
            .buffer_local_get(buffer, sym::BUFFER_FILE_TRUENAME)
            .unwrap_or(Value::NIL);
        if modified && truename.is_string() {
            f_unlock_file(self, &[truename])?;
        }
        Ok(())
    }
}

pub(crate) fn f_lock_file(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let file = lisp.check_string(args[0])?;
    if let Some(handler) = lisp.find_file_name_handler(file, sym::LOCK_FILE)? {
        return lisp.funcall(handler, &[sym::LOCK_FILE, file]);
    }
    lisp.lock_file(file)?;
    Ok(Value::NIL)
}

pub(crate) fn f_unlock_file(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let file = lisp.check_string(args[0])?;
    if let Some(handler) = lisp.find_file_name_handler(file, sym::UNLOCK_FILE)? {
        lisp.funcall(handler, &[sym::UNLOCK_FILE, file])?;
        return Ok(Value::NIL);
    }
    let conditions = lisp.list(&[sym::FILE_ERROR]);
    lisp.stack.push(conditions);
    let r = lisp.condition_case_internal(
        conditions,
        |l| l.unlock_file(file),
        |l, err| {
            l.funcall(sym::USERLOCK__HANDLE_UNLOCK_ERROR, &[err])?;
            Ok(Value::NIL)
        },
    );
    lisp.stack.pop();
    r?;
    Ok(Value::NIL)
}

fn f_lock_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let file = if args[0].is_nil() {
        lisp.find_symbol_value(sym::BUFFER_FILE_TRUENAME)
    } else {
        lisp.check_string(args[0])?
    };
    let text = &lisp.cur().text;
    if text.save_modiff < text.modiff && !file.is_nil() {
        f_lock_file(lisp, &[file])?;
    }
    Ok(Value::NIL)
}

fn f_unlock_buffer(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let truename = lisp.find_symbol_value(sym::BUFFER_FILE_TRUENAME);
    let text = &lisp.cur().text;
    if text.save_modiff < text.modiff && truename.is_string() {
        f_unlock_file(lisp, &[truename])?;
    }
    Ok(Value::NIL)
}

fn f_file_locked_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let filename = lisp.check_string(args[0])?;
    if let Some(handler) = lisp.find_file_name_handler(filename, sym::FILE_LOCKED_P)? {
        return lisp.funcall(handler, &[sym::FILE_LOCKED_P, filename]);
    }
    let lfname = lisp.make_lock_file_name(filename)?;
    if lfname.is_nil() {
        return Ok(Value::NIL);
    }
    let base = lisp.stack.len();
    lisp.stack.push(lfname);
    let owner = lisp.current_lock_owner(lfname);
    lisp.stack.truncate(base);
    match owner? {
        Owner::Mine => Ok(sym::T),
        Owner::Another(info) => Ok(lisp.make_string(&info.user[..info.at])),
        Owner::Free => Ok(Value::NIL),
        Owner::Error(e) => Err(lisp.file_error("Testing file lock", &e, filename)),
    }
}
