use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::coding::{CodingSystem, FOR_UNIBYTE_MASK, REQUIRE_FLUSHING_MASK};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("find-file-name-handler", 2, 2, f_find_file_name_handler),
    Subr::fixed("file-name-directory", 1, 1, f_file_name_directory),
    Subr::fixed("file-name-nondirectory", 1, 1, f_file_name_nondirectory),
    Subr::fixed(
        "unhandled-file-name-directory",
        1,
        1,
        f_unhandled_file_name_directory,
    ),
    Subr::fixed("file-name-as-directory", 1, 1, f_file_name_as_directory),
    Subr::fixed("directory-name-p", 1, 1, f_directory_name_p),
    Subr::fixed("directory-file-name", 1, 1, f_directory_file_name),
    Subr::fixed("make-temp-file-internal", 4, 4, f_make_temp_file_internal),
    Subr::fixed("make-temp-name", 1, 1, f_make_temp_name),
    Subr::many("file-name-concat", 1, f_file_name_concat),
    Subr::fixed("expand-file-name", 1, 2, f_expand_file_name),
    Subr::fixed("substitute-in-file-name", 1, 1, f_substitute_in_file_name),
    Subr::fixed("copy-file", 2, 6, f_copy_file),
    Subr::fixed("make-directory-internal", 1, 1, f_make_directory_internal),
    Subr::fixed(
        "delete-directory-internal",
        1,
        1,
        f_delete_directory_internal,
    ),
    Subr::fixed("delete-file-internal", 1, 1, f_delete_file_internal),
    Subr::fixed(
        "file-name-case-insensitive-p",
        1,
        1,
        f_file_name_case_insensitive_p,
    ),
    Subr::fixed("rename-file", 2, 3, f_rename_file),
    Subr::fixed("add-name-to-file", 2, 3, f_add_name_to_file),
    Subr::fixed("make-symbolic-link", 2, 3, f_make_symbolic_link),
    Subr::fixed("file-name-absolute-p", 1, 1, f_file_name_absolute_p),
    Subr::fixed("file-exists-p", 1, 1, f_file_exists_p),
    Subr::fixed("file-executable-p", 1, 1, f_file_executable_p),
    Subr::fixed("file-readable-p", 1, 1, f_file_readable_p),
    Subr::fixed("file-writable-p", 1, 1, f_file_writable_p),
    Subr::fixed("access-file", 2, 2, f_access_file),
    Subr::fixed("file-symlink-p", 1, 1, f_file_symlink_p),
    Subr::fixed("file-directory-p", 1, 1, f_file_directory_p),
    Subr::fixed(
        "file-accessible-directory-p",
        1,
        1,
        f_file_accessible_directory_p,
    ),
    Subr::fixed("file-regular-p", 1, 1, f_file_regular_p),
    Subr::fixed("file-modes", 1, 2, f_file_modes),
    Subr::fixed("set-file-modes", 2, 3, f_set_file_modes),
    Subr::fixed("set-default-file-modes", 1, 1, f_ignore1),
    Subr::fixed("default-file-modes", 0, 0, f_default_file_modes),
    Subr::fixed("file-newer-than-file-p", 2, 2, f_file_newer_than_file_p),
    Subr::fixed("insert-file-contents", 1, 5, f_insert_file_contents),
    Subr::fixed("write-region", 3, 7, f_write_region),
    Subr::fixed("car-less-than-car", 2, 2, f_car_less_than_car),
    Subr::fixed("verify-visited-file-modtime", 0, 1, f_true),
    Subr::fixed("visited-file-modtime", 0, 0, f_zero),
    Subr::fixed("set-visited-file-modtime", 0, 1, f_ignore1),
    Subr::fixed("set-buffer-auto-saved", 0, 0, f_ignore0),
    Subr::fixed("clear-buffer-auto-save-failure", 0, 0, f_ignore0),
    Subr::fixed("recent-auto-save-p", 0, 0, f_ignore0),
    Subr::fixed("next-read-file-uses-dialog-p", 0, 0, f_ignore0),
    Subr::fixed("file-system-info", 1, 1, f_ignore1),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::FILE_NAME_HANDLER_ALIST, Value::NIL);
    lisp.defvar(sym::INHIBIT_FILE_NAME_HANDLERS, Value::NIL);
    lisp.defvar(sym::INHIBIT_FILE_NAME_OPERATION, Value::NIL);
    lisp.defvar(sym::FILE_NAME_CODING_SYSTEM, Value::NIL);
    lisp.defvar(sym::DEFAULT_FILE_NAME_CODING_SYSTEM, Value::NIL);
    lisp.defvar(sym::SET_AUTO_CODING_FUNCTION, Value::NIL);
    lisp.defvar(sym::AFTER_INSERT_FILE_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::WRITE_REGION_ANNOTATE_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::WRITE_REGION_POST_ANNOTATION_FUNCTION, Value::NIL);
    lisp.defvar(sym::WRITE_REGION_ANNOTATIONS_SO_FAR, Value::NIL);
    lisp.defvar(sym::INHIBIT_FILE_NAME_HANDLERS, Value::NIL);
    lisp.defvar(sym::AUTO_SAVE_LIST_FILE_NAME, Value::NIL);
    lisp.defvar(sym::AUTO_SAVE_VISITED_FILE_NAME, Value::NIL);
    lisp.defvar(sym::AUTO_SAVE_INCLUDE_BIG_DELETIONS, Value::NIL);
    lisp.defvar_bool(sym::WRITE_REGION_INHIBIT_FSYNC, true);
    lisp.defvar_bool(sym::DELETE_BY_MOVING_TO_TRASH, false);
    lisp.defvar(sym::AFTER_INSERT_FILE_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::FILE_NAME_HISTORY, Value::NIL);
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut dir = normalize_slashes(&cwd.to_string_lossy());
    if !dir.ends_with('/') {
        dir.push('/');
    }
    let d = lisp.string(&dir);
    lisp.set_default(sym::DEFAULT_DIRECTORY, d).ok();
    let invocation = std::env::current_exe().unwrap_or_default();
    let dir_path = invocation
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut idir = normalize_slashes(&dir_path.to_string_lossy());
    if !idir.ends_with('/') {
        idir.push('/');
    }
    let id = lisp.string(&idir);
    lisp.defvar(sym::INVOCATION_DIRECTORY, id);
    let name = invocation
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "emacs".to_string());
    let iname = lisp.string(&name);
    lisp.defvar(sym::INVOCATION_NAME, iname);
}

pub fn normalize_slashes(s: &str) -> String {
    if cfg!(windows) {
        s.replace('\\', "/")
    } else {
        s.to_string()
    }
}

fn is_sep(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

fn has_drive(s: &str) -> bool {
    cfg!(windows) && {
        let b = s.as_bytes();
        b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
    }
}

pub fn file_name_absolute_p(s: &str) -> bool {
    let first = s.chars().next();
    match first {
        Some(c) if is_sep(c) => true,
        Some('~') => {
            let user: String = s[1..].chars().take_while(|&c| !is_sep(c)).collect();
            user.is_empty() || home_of(&user).is_some()
        }
        _ => has_drive(s) && s.chars().nth(2).is_some_and(is_sep),
    }
}

fn home_dir() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/".to_string());
    normalize_slashes(&home)
}

fn home_of(user: &str) -> Option<String> {
    if user.is_empty() {
        return Some(home_dir());
    }
    let current = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    (user == current).then(home_dir)
}

pub fn file_name_directory(s: &str) -> Option<String> {
    let idx = s.rfind(is_sep);
    match idx {
        Some(i) => Some(s[..=i].to_string()),
        None if has_drive(s) => Some(s[..2].to_string()),
        None => None,
    }
}

pub fn file_name_nondirectory(s: &str) -> String {
    match s.rfind(is_sep) {
        Some(i) => s[i + 1..].to_string(),
        None if has_drive(s) => s[2..].to_string(),
        None => s.to_string(),
    }
}

pub fn file_name_as_directory(s: &str) -> String {
    if s.is_empty() {
        return "./".to_string();
    }
    if s.ends_with(is_sep) {
        return s.to_string();
    }
    format!("{s}/")
}

pub fn directory_file_name(s: &str) -> String {
    let trimmed = s.trim_end_matches(is_sep);
    if trimmed.is_empty() {
        if s.starts_with("//") && s.len() == 2 {
            return "//".to_string();
        }
        return if s.is_empty() {
            String::new()
        } else {
            "/".to_string()
        };
    }
    if has_drive(trimmed) && trimmed.len() == 2 && s.len() > 2 {
        return format!("{trimmed}/");
    }
    trimmed.to_string()
}

pub fn expand_path(name: &str, default_directory: &str) -> String {
    let mut name = normalize_slashes(name);
    if let Some(rest) = name.strip_prefix('~') {
        let user: String = rest.chars().take_while(|&c| c != '/').collect();
        if let Some(home) = home_of(&user) {
            name = format!("{}{}", home.trim_end_matches('/'), &rest[user.len()..]);
            if name.is_empty() {
                name = "/".to_string();
            }
        }
    }
    let absolute = file_name_absolute_p(&name) && !name.starts_with('~');
    let full = if absolute {
        name
    } else if cfg!(windows) && has_drive(&name) {
        let drive = &name[..2];
        let rest = &name[2..];
        let base =
            if default_directory.len() >= 2 && default_directory[..2].eq_ignore_ascii_case(drive) {
                default_directory.to_string()
            } else {
                format!("{drive}/")
            };
        format!("{}/{}", base.trim_end_matches('/'), rest)
    } else {
        let dir = normalize_slashes(default_directory);
        let dir = if file_name_absolute_p(&dir) && !dir.starts_with('~') {
            dir
        } else {
            let cwd = normalize_slashes(
                &std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy(),
            );
            if dir.is_empty() {
                cwd
            } else {
                expand_path(&dir, &format!("{cwd}/"))
            }
        };
        if dir.ends_with('/') {
            format!("{dir}{name}")
        } else {
            format!("{dir}/{name}")
        }
    };
    canonicalize_components(&full)
}

fn canonicalize_components(path: &str) -> String {
    let (prefix, rest) = if has_drive(path) {
        let mut d = path[..2].to_string();
        if let Some(c) = d.get_mut(0..1) {
            let lower = c.to_ascii_lowercase();
            d.replace_range(0..1, &lower);
        }
        (d, &path[2..])
    } else if path.starts_with("//") && !path.starts_with("///") {
        ("/".to_string(), &path[1..])
    } else {
        (String::new(), path)
    };
    let trailing = rest.ends_with('/') && rest.len() > 1;
    let mut parts: Vec<&str> = Vec::new();
    for comp in rest.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|p| *p != "..") {
                    parts.pop();
                } else if parts.is_empty() {
                    if prefix.is_empty() {
                        parts.push("..");
                    }
                } else {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    let mut out = format!("{prefix}/{}", parts.join("/"));
    if trailing && !out.ends_with('/') {
        out.push('/');
    }
    if out.starts_with("/..")
        && prefix.is_empty()
        && rest.starts_with("/..")
        && parts.first() == Some(&"..")
    {
        return out;
    }
    out
}

pub fn substitute_in_file_name(name: &str) -> String {
    let mut s = normalize_slashes(name);
    if let Some(pos) = find_embedded_absolute(&s) {
        s = s[pos..].to_string();
    }
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '$' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        if i + 1 < chars.len() && chars[i + 1] == '$' {
            out.push('$');
            i += 2;
            continue;
        }
        let (var, next) = if i + 1 < chars.len() && chars[i + 1] == '{' {
            match chars[i + 2..].iter().position(|&c| c == '}') {
                Some(end) => (
                    chars[i + 2..i + 2 + end].iter().collect::<String>(),
                    i + 3 + end,
                ),
                None => {
                    out.push('$');
                    i += 1;
                    continue;
                }
            }
        } else {
            let var: String = chars[i + 1..]
                .iter()
                .take_while(|c| c.is_ascii_alphanumeric() || **c == '_')
                .collect();
            let len = var.chars().count();
            (var, i + 1 + len)
        };
        match std::env::var(&var) {
            Ok(value) if !var.is_empty() => {
                let value = normalize_slashes(&value);
                if file_name_absolute_p(&value) {
                    out.clear();
                }
                out.push_str(&value);
            }
            _ => {
                out.extend(&chars[i..next]);
            }
        }
        i = next;
    }
    out
}

fn find_embedded_absolute(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut found = None;
    for i in 1..b.len() {
        if b[i - 1] == b'/' && (b[i] == b'/' || b[i] == b'~') && i > 1 {
            found = Some(i);
        } else if b[i - 1] == b'/' && b[i] == b'/' && i == 1 {
            continue;
        }
        if cfg!(windows)
            && i >= 2
            && b[i - 2] == b'/'
            && b[i - 1].is_ascii_alphabetic()
            && b[i] == b':'
        {
            found = Some(i - 1);
        }
    }
    found
}

impl Lisp {
    pub fn default_directory_string(&self) -> String {
        let d = self.find_symbol_value(sym::DEFAULT_DIRECTORY);
        if d.is_string() {
            self.string_to_rust(d)
        } else {
            let cwd = std::env::current_dir().unwrap_or_default();
            format!("{}/", normalize_slashes(&cwd.to_string_lossy()))
        }
    }

    pub fn expand_file_name(&mut self, name: Value, default_directory: Value) -> LispResult {
        let name = self.check_string(name)?;
        if let Some(handler) = self.find_file_name_handler(name, sym::EXPAND_FILE_NAME)? {
            return self.funcall(handler, &[sym::EXPAND_FILE_NAME, name, default_directory]);
        }
        let n = self.string_to_rust(name);
        let dir = if default_directory.is_nil() {
            self.default_directory_string()
        } else if default_directory.is_string() {
            let d = self.string_to_rust(default_directory);
            if file_name_absolute_p(&d) {
                d
            } else {
                let base = self.default_directory_string();
                expand_path(&d, &base)
            }
        } else {
            self.default_directory_string()
        };
        let expanded = expand_path(&n, &dir);
        Ok(self.string(&expanded))
    }

    pub fn find_file_name_handler(
        &mut self,
        filename: Value,
        operation: Value,
    ) -> LispResult<Option<Value>> {
        let alist = self.find_symbol_value(sym::FILE_NAME_HANDLER_ALIST);
        if !alist.is_cons() {
            return Ok(None);
        }
        let inhibited = if self.plain_value(sym::INHIBIT_FILE_NAME_OPERATION) == operation {
            self.plain_value(sym::INHIBIT_FILE_NAME_HANDLERS)
        } else {
            Value::NIL
        };
        if self.heap.symbol_ref(sym::STRING_MATCH).function.is_nil() {
            return Ok(None);
        }
        let mut best: Option<(usize, Value)> = None;
        let mut tail = alist;
        let base = self.stack.len();
        self.stack.push(alist);
        self.stack.push(filename);
        while tail.is_cons() {
            let elt = self.xcar(tail);
            tail = self.xcdr(tail);
            if !elt.is_cons() {
                continue;
            }
            let regexp = self.xcar(elt);
            let handler = self.xcdr(elt);
            if !regexp.is_string() {
                continue;
            }
            if !self.memq(handler, inhibited).is_nil() {
                continue;
            }
            let ops = self.get(handler, sym::OPERATIONS);
            if !ops.is_nil() && !self.memq(operation, ops).is_nil() || ops.is_nil() {
                let m = match self.funcall(sym::STRING_MATCH, &[regexp, filename]) {
                    Ok(m) => m,
                    Err(e) => {
                        self.stack.truncate(base);
                        return Err(e);
                    }
                };
                if let Some(pos) = m.as_fixnum() {
                    let pos = pos as usize;
                    if best.is_none_or(|(b, _)| pos > b) {
                        best = Some((pos, handler));
                    }
                }
            }
        }
        self.stack.truncate(base);
        Ok(best.map(|(_, h)| h))
    }

    fn file_arg(&mut self, filename: Value) -> LispResult<PathBuf> {
        let expanded = self.expand_file_name(filename, Value::NIL)?;
        Ok(PathBuf::from(self.string_to_rust(expanded)))
    }

    fn handled(&mut self, op: Value, args: &[Value]) -> LispResult<Option<Value>> {
        if let Some(h) = self.find_file_name_handler(args[0], op)? {
            let mut call = vec![op];
            call.extend_from_slice(args);
            return Ok(Some(self.funcall(h, &call)?));
        }
        Ok(None)
    }

    pub fn file_error(
        &mut self,
        message: &str,
        err: &std::io::Error,
        file: Value,
    ) -> crate::error::Flow {
        let symbol = match err.kind() {
            std::io::ErrorKind::NotFound => sym::FILE_MISSING,
            std::io::ErrorKind::PermissionDenied => sym::PERMISSION_DENIED,
            std::io::ErrorKind::AlreadyExists => sym::FILE_ALREADY_EXISTS,
            _ => sym::FILE_ERROR,
        };
        let msg = self.string(message);
        let text = match err.kind() {
            std::io::ErrorKind::NotFound => "No such file or directory".to_string(),
            std::io::ErrorKind::PermissionDenied => "Permission denied".to_string(),
            std::io::ErrorKind::AlreadyExists => "File exists".to_string(),
            std::io::ErrorKind::IsADirectory => "Is a directory".to_string(),
            _ => err.to_string(),
        };
        let detail = self.string(&text);
        self.signal_list(symbol, &[msg, detail, file])
    }
}

fn f_ignore0(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_ignore1(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_true(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(sym::T)
}

fn f_zero(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(0))
}

fn f_find_file_name_handler(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let filename = lisp.check_string(args[0])?;
    Ok(lisp
        .find_file_name_handler(filename, args[1])?
        .unwrap_or(Value::NIL))
}

fn string_op(lisp: &mut Lisp, v: Value, op: Value, f: fn(&str) -> Option<String>) -> LispResult {
    let s = lisp.check_string(v)?;
    if let Some(r) = lisp.handled(op, &[s])? {
        return Ok(r);
    }
    let text = lisp.string_to_rust(s);
    match f(&text) {
        Some(r) => Ok(lisp.string(&r)),
        None => Ok(Value::NIL),
    }
}

fn f_file_name_directory(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    string_op(lisp, args[0], sym::FILE_NAME_DIRECTORY, file_name_directory)
}

fn f_file_name_nondirectory(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    string_op(lisp, args[0], sym::FILE_NAME_NONDIRECTORY, |s| {
        Some(file_name_nondirectory(s))
    })
}

fn f_unhandled_file_name_directory(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    let text = lisp.string_to_rust(s);
    Ok(lisp.string(&file_name_as_directory(&text)))
}

fn f_file_name_as_directory(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    string_op(lisp, args[0], sym::FILE_NAME_AS_DIRECTORY, |s| {
        Some(file_name_as_directory(s))
    })
}

fn f_directory_name_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    let text = lisp.string_to_rust(s);
    Ok(lisp.bool(text.ends_with(is_sep)))
}

fn f_directory_file_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    string_op(lisp, args[0], sym::DIRECTORY_FILE_NAME, |s| {
        Some(directory_file_name(s))
    })
}

fn f_make_temp_file_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (prefix, dir_flag, suffix, text) = (args[0], args[1], args[2], args[3]);
    let prefix = lisp.check_string(prefix)?;
    let base = lisp.expand_file_name(prefix, Value::NIL)?;
    let base = lisp.string_to_rust(base);
    let suffix = if suffix.is_string() {
        lisp.string_to_rust(suffix)
    } else {
        String::new()
    };
    for _ in 0..100 {
        let r = lisp.next_random() % 1_000_000;
        let name = format!("{base}{r:06}{suffix}");
        let path = PathBuf::from(&name);
        let created = if dir_flag.is_nil() {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map(|_| ())
        } else {
            std::fs::create_dir(&path)
        };
        match created {
            Ok(()) => {
                if text.is_string() {
                    let bytes = lisp.string_bytes(text).to_vec();
                    let _ = std::fs::write(&path, bytes);
                }
                return Ok(lisp.string(&name));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                let v = lisp.string(&name);
                return Err(lisp.file_error("Creating file with prefix", &e, v));
            }
        }
    }
    Err(lisp.error("Cannot create temporary name"))
}

fn f_make_temp_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prefix = lisp.check_string(args[0])?;
    let p = lisp.string_to_rust(prefix);
    let r = lisp.next_random() % 1_000_000;
    Ok(lisp.string(&format!("{p}{r:06}")))
}

fn f_file_name_concat(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut parts: Vec<String> = Vec::new();
    for &a in args {
        if a.is_nil() {
            continue;
        }
        let s = lisp.check_string(a)?;
        let text = lisp.string_to_rust(s);
        if !text.is_empty() {
            parts.push(text);
        }
    }
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        out.push_str(part);
        if i + 1 < parts.len() && !part.ends_with(is_sep) {
            out.push('/');
        }
    }
    Ok(lisp.string(&out))
}

fn f_expand_file_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.expand_file_name(args[0], args[1])
}

fn f_substitute_in_file_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    if let Some(r) = lisp.handled(sym::SUBSTITUTE_IN_FILE_NAME, &[s])? {
        return Ok(r);
    }
    let text = lisp.string_to_rust(s);
    let r = substitute_in_file_name(&text);
    if r == text {
        return Ok(s);
    }
    Ok(lisp.string(&r))
}

fn f_copy_file(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (file, newname, ok_if_exists) = (args[0], args[1], args[2]);
    if let Some(r) = lisp.handled(sym::COPY_FILE, args)? {
        return Ok(r);
    }
    let from = lisp.file_arg(file)?;
    let mut to = lisp.file_arg(newname)?;
    let new_text = lisp.string_to_rust(newname);
    if (new_text.ends_with(is_sep) || to.is_dir())
        && let Some(name) = from.file_name()
    {
        to = to.join(name);
    }
    if to.exists() && ok_if_exists.is_nil() {
        let v = lisp.string(&to.to_string_lossy());
        let msg = lisp.string("File already exists");
        return Err(lisp.signal_list(sym::FILE_ALREADY_EXISTS, &[msg, v]));
    }
    if let Err(e) = std::fs::copy(&from, &to) {
        return Err(lisp.file_error("Copying file", &e, file));
    }
    Ok(Value::NIL)
}

fn f_make_directory_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if let Some(r) = lisp.handled(sym::MAKE_DIRECTORY_INTERNAL, args)? {
        return Ok(r);
    }
    let path = lisp.file_arg(args[0])?;
    if let Err(e) = std::fs::create_dir(&path) {
        return Err(lisp.file_error("Creating directory", &e, args[0]));
    }
    Ok(Value::NIL)
}

fn f_delete_directory_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let path = lisp.file_arg(args[0])?;
    if let Err(e) = std::fs::remove_dir(&path) {
        return Err(lisp.file_error("Removing directory", &e, args[0]));
    }
    Ok(Value::NIL)
}

fn f_delete_file_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let path = lisp.file_arg(args[0])?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(Value::NIL),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Value::NIL),
        Err(e) => Err(lisp.file_error("Removing old name", &e, args[0])),
    }
}

fn f_file_name_case_insensitive_p(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.bool(cfg!(windows) || cfg!(target_os = "macos")))
}

fn f_rename_file(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if let Some(r) = lisp.handled(sym::RENAME_FILE, args)? {
        return Ok(r);
    }
    let from = lisp.file_arg(args[0])?;
    let to = lisp.file_arg(args[1])?;
    if to.exists() && args[2].is_nil() {
        let v = lisp.string(&to.to_string_lossy());
        let msg = lisp.string("File already exists");
        return Err(lisp.signal_list(sym::FILE_ALREADY_EXISTS, &[msg, v]));
    }
    if let Err(e) = std::fs::rename(&from, &to) {
        return Err(lisp.file_error("Renaming", &e, args[0]));
    }
    Ok(Value::NIL)
}

fn f_add_name_to_file(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let from = lisp.file_arg(args[0])?;
    let to = lisp.file_arg(args[1])?;
    if let Err(e) = std::fs::hard_link(&from, &to) {
        return Err(lisp.file_error("Adding new name", &e, args[0]));
    }
    Ok(Value::NIL)
}

fn f_make_symbolic_link(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let target = lisp.check_string(args[0])?;
    let target = PathBuf::from(lisp.string_to_rust(target));
    let link = lisp.file_arg(args[1])?;
    #[cfg(unix)]
    let r = std::os::unix::fs::symlink(&target, &link);
    #[cfg(windows)]
    let r = if target.is_dir() {
        std::os::windows::fs::symlink_dir(&target, &link)
    } else {
        std::os::windows::fs::symlink_file(&target, &link)
    };
    if let Err(e) = r {
        return Err(lisp.file_error("Making symbolic link", &e, args[1]));
    }
    Ok(Value::NIL)
}

fn f_file_name_absolute_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    let text = lisp.string_to_rust(s);
    Ok(lisp.bool(file_name_absolute_p(&text)))
}

fn predicate(lisp: &mut Lisp, args: &[Value], op: Value, f: fn(&Path) -> bool) -> LispResult {
    let s = lisp.check_string(args[0])?;
    if let Some(r) = lisp.handled(op, &[s])? {
        return Ok(r);
    }
    let path = lisp.file_arg(s)?;
    Ok(lisp.bool(f(&path)))
}

fn f_file_exists_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    predicate(lisp, args, sym::FILE_EXISTS_P, |p| {
        p.exists() || p.symlink_metadata().is_ok()
    })
}

fn f_file_executable_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    predicate(lisp, args, sym::FILE_EXECUTABLE_P, |p| {
        if p.is_dir() {
            return true;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            p.metadata()
                .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            p.extension().is_some_and(|e| {
                matches!(
                    e.to_string_lossy().to_ascii_lowercase().as_str(),
                    "exe" | "com" | "bat" | "cmd"
                )
            })
        }
    })
}

fn f_file_readable_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    predicate(lisp, args, sym::FILE_READABLE_P, |p| {
        if p.is_dir() {
            std::fs::read_dir(p).is_ok()
        } else {
            std::fs::File::open(p).is_ok()
        }
    })
}

fn f_file_writable_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    predicate(lisp, args, sym::FILE_WRITABLE_P, |p| match p.metadata() {
        Ok(m) => !m.permissions().readonly(),
        Err(_) => p
            .parent()
            .is_none_or(|d| d.as_os_str().is_empty() || d.is_dir()),
    })
}

fn f_access_file(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let path = lisp.file_arg(args[0])?;
    if let Err(e) = std::fs::metadata(&path) {
        let msg = lisp.check_string(args[1])?;
        let m = lisp.string_to_rust(msg);
        return Err(lisp.file_error(&m, &e, args[0]));
    }
    Ok(Value::NIL)
}

fn f_file_symlink_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    if let Some(r) = lisp.handled(sym::FILE_SYMLINK_P, &[s])? {
        return Ok(r);
    }
    let path = lisp.file_arg(s)?;
    match std::fs::read_link(&path) {
        Ok(target) => {
            let t = normalize_slashes(&target.to_string_lossy());
            Ok(lisp.string(&t))
        }
        Err(_) => Ok(Value::NIL),
    }
}

fn f_file_directory_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    predicate(lisp, args, sym::FILE_DIRECTORY_P, Path::is_dir)
}

fn f_file_accessible_directory_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    predicate(lisp, args, sym::FILE_ACCESSIBLE_DIRECTORY_P, |p| {
        p.is_dir() && std::fs::read_dir(p).is_ok()
    })
}

fn f_file_regular_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    predicate(lisp, args, sym::FILE_REGULAR_P, Path::is_file)
}

fn f_file_modes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let path = lisp.file_arg(args[0])?;
    match std::fs::metadata(&path) {
        Ok(m) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                Ok(Value::fixnum((m.permissions().mode() & 0o7777) as i64))
            }
            #[cfg(not(unix))]
            {
                let mut mode = 0o444;
                if !m.permissions().readonly() {
                    mode |= 0o222;
                }
                if m.is_dir() {
                    mode |= 0o111;
                }
                Ok(Value::fixnum(mode))
            }
        }
        Err(_) => Ok(Value::NIL),
    }
}

fn f_set_file_modes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let path = lisp.file_arg(args[0])?;
    let mode = lisp.check_fixnum(args[1])?;
    if let Ok(m) = std::fs::metadata(&path) {
        let mut perms = m.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(mode as u32);
        }
        #[cfg(not(unix))]
        {
            perms.set_readonly(mode & 0o200 == 0);
        }
        if let Err(e) = std::fs::set_permissions(&path, perms) {
            return Err(lisp.file_error("Doing chmod", &e, args[0]));
        }
    }
    Ok(Value::NIL)
}

fn f_default_file_modes(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(0o755))
}

fn f_file_newer_than_file_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.file_arg(args[0])?;
    let b = lisp.file_arg(args[1])?;
    let ma = std::fs::metadata(&a).and_then(|m| m.modified());
    let mb = std::fs::metadata(&b).and_then(|m| m.modified());
    Ok(match (ma, mb) {
        (Ok(x), Ok(y)) => lisp.bool(x > y),
        (Ok(_), Err(_)) => sym::T,
        _ => Value::NIL,
    })
}

const READ_BUF_SIZE: usize = 16 * 1024;
const INSERT_READ_SIZE_MAX: usize = 1024 * 1024;
const NONEXISTENT_MODTIME_NSECS: i64 = -1;
const UNKNOWN_MODTIME_NSECS: i64 = -2;

struct InsertFile {
    filename: Value,
    orig_filename: Value,
    visit: Value,
    beg: Value,
    end: Value,
    replace: Value,
    handler: Value,
    val: Value,
    inserted: usize,
    total: i64,
    save_errno: Option<std::io::Error>,
    set_coding_system: bool,
    coding_system: Value,
    coding: CodingSystem,
    window_markers: Value,
    same_at_start: usize,
    same_at_end_charpos: usize,
    mtime: (i64, i64),
    st_size: i64,
    old_deactivate_mark: Value,
}

fn file_offset(lisp: &mut Lisp, val: Value) -> LispResult<i64> {
    if let Some(n) = val.as_fixnum() {
        if n >= 0 {
            return Ok(n);
        }
    } else if lisp.is_bignum(val) {
        if let Some(n) = lisp.bignum_ref(val).and_then(|b| i64::try_from(b).ok())
            && n >= 0
        {
            return Ok(n);
        }
    } else if val.is_float() {
        let v = lisp.heap.float_value(val);
        if (0.0..9.223_372_036_854_776e18).contains(&v) && v.fract() == 0.0 {
            return Ok(v as i64);
        }
    }
    Err(lisp.wrong_type(sym::FILE_OFFSET, val))
}

fn time_error_value(err: &std::io::Error) -> (i64, i64) {
    let ns = match err.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory => {
            NONEXISTENT_MODTIME_NSECS
        }
        _ => UNKNOWN_MODTIME_NSECS,
    };
    (0, ns)
}

fn stat_mtime(meta: &std::fs::Metadata) -> (i64, i64) {
    match meta.modified() {
        Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => (d.as_secs() as i64, d.subsec_nanos() as i64),
            Err(e) => {
                let d = e.duration();
                (-(d.as_secs() as i64), -(d.subsec_nanos() as i64))
            }
        },
        Err(_) => (0, UNKNOWN_MODTIME_NSECS),
    }
}

fn full_read(file: &mut File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut done = 0;
    while done < buf.len() {
        match file.read(&mut buf[done..]) {
            Ok(0) => break,
            Ok(n) => done += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) if done == 0 => return Err(e),
            Err(_) => break,
        }
    }
    Ok(done)
}

fn read_some(file: &mut File, buf: &mut [u8]) -> std::io::Result<usize> {
    loop {
        match file.read(buf) {
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            r => return r,
        }
    }
}

fn char_head_p(byte: u8) -> bool {
    byte & 0xC0 != 0x80
}

impl Lisp {
    fn xlseek(&mut self, file: &mut File, pos: SeekFrom, filename: Value) -> LispResult<i64> {
        match file.seek(pos) {
            Ok(p) => Ok(p as i64),
            Err(e) => Err(self.file_error("Setting file position", &e, filename)),
        }
    }

    fn fetch_byte(&self, pos: usize) -> u8 {
        let t = &self.cur().text;
        if pos > t.bytes() { 0 } else { t.byte_at(pos) }
    }

    fn set_pt_both(&mut self, charpos: usize, bytepos: usize) {
        let b = self.cur_mut();
        b.pt = charpos;
        b.pt_byte = bytepos;
    }

    fn del_range_byte(&mut self, from_byte: usize, to_byte: usize) -> LispResult<()> {
        let b = self.cur();
        let from_byte = from_byte.max(b.begv_byte);
        let to_byte = to_byte.min(b.zv_byte);
        if to_byte <= from_byte {
            return Ok(());
        }
        let from = b.text.byte_to_char(from_byte);
        let to = b.text.byte_to_char(to_byte);
        self.del_range(from, to)
    }

    fn set_multibyte_flag(&mut self, buffer: Value, flag: bool) {
        self.buffer_mut(buffer).text.multibyte = flag;
        let v = self.bool(flag);
        self.buffer_local_set(buffer, sym::ENABLE_MULTIBYTE_CHARACTERS, v);
    }

    fn fboundp(&self, symbol: Value) -> bool {
        !self.heap.symbol_ref(symbol).function.is_nil()
    }

    fn unlock_file_if_supported(&mut self, file: Value) -> LispResult<()> {
        if self.fboundp(sym::UNLOCK_FILE) {
            self.funcall(sym::UNLOCK_FILE, &[file])?;
        }
        Ok(())
    }

    fn check_inserted(&mut self, insval: Value) -> LispResult<usize> {
        let b = self.cur();
        let limit = (b.zv - b.pt) as i64;
        match insval.as_fixnum() {
            Some(n) if (0..=limit).contains(&n) => Ok(n as usize),
            _ => Err(self.wrong_type(sym::INSERTED_CHARS, insval)),
        }
    }

    fn get_window_points_and_markers(&mut self) -> LispResult {
        let pt_marker = crate::editfns::f_point_marker(self, &[])?;
        self.stack.push(pt_marker);
        let buffer = self.current_buffer;
        let windows = self.funcall(sym::GET_BUFFER_WINDOW_LIST, &[buffer, Value::NIL, sym::T])?;
        self.stack.push(windows);
        let pt = Value::fixnum(self.cur().pt as i64);
        let head = self.cons(pt_marker, pt);
        Ok(self.cons(head, windows))
    }

    fn restore_window_points(
        &mut self,
        mut window_markers: Value,
        inserted: usize,
        same_at_start: usize,
        same_at_end: usize,
    ) -> LispResult<()> {
        while window_markers.is_cons() {
            let car = self.xcar(window_markers);
            if car.is_cons() {
                let marker = self.xcar(car);
                let oldpos = self.xcdr(car);
                if let Some(old) = oldpos.as_fixnum()
                    && self.is_marker(marker)
                    && old > same_at_start as i64
                    && old <= same_at_end as i64
                {
                    let oldsize = (same_at_end - same_at_start) as f64;
                    let growth = inserted as f64 / oldsize;
                    let newpos = same_at_start as i64
                        + (growth * (old - same_at_start as i64) as f64) as i64;
                    self.set_marker(marker, Value::fixnum(newpos), Value::NIL, false)?;
                }
            }
            window_markers = self.xcdr(window_markers);
        }
        Ok(())
    }

    fn decode_coding_c_string(
        &mut self,
        coding: &mut CodingSystem,
        bytes: Vec<u8>,
        dst_object: Value,
    ) -> LispResult<()> {
        let n = bytes.len();
        coding.source = bytes;
        coding.src_chars = n;
        coding.src_bytes = n;
        self.decode_coding_object(coding, Value::NIL, (0, 0, n, n), dst_object)
    }
}

pub(crate) fn restore_point_unwind(lisp: &mut Lisp, location: Value) -> LispResult<()> {
    let pos = lisp.marker_position_checked(location)?;
    lisp.goto_char_internal(pos);
    lisp.unchain_marker(location);
    Ok(())
}

fn decide_coding_unwind(lisp: &mut Lisp, unwind_data: Value) -> LispResult<()> {
    let multibyte = lisp.xcar(unwind_data);
    let tmp = lisp.xcdr(unwind_data);
    let undo_list = lisp.xcar(tmp);
    let buffer = lisp.xcdr(tmp);
    lisp.set_buffer_internal(buffer);
    let z = lisp.cur().z();
    lisp.adjust_markers_for_delete(buffer, 1, z);
    lisp.adjust_overlays_for_delete(buffer, 1, z);
    let b = lisp.cur_mut();
    b.text.props.clear();
    b.pt = 1;
    b.pt_byte = 1;
    let bytes = b.text.slice(0, b.text.bytes());
    let z_byte = b.z_byte();
    let chars = b.text.chars;
    b.text.delete_bytes(1, z_byte, chars);
    b.zv = 1;
    b.zv_byte = 1;
    let s = lisp.unibyte_string(&bytes);
    lisp.heap.set_car(unwind_data, s);
    lisp.set_multibyte_flag(buffer, !multibyte.is_nil());
    lisp.set_undo_list(undo_list);
    Ok(())
}

fn f_insert_file_contents(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let count = lisp.specpdl.len();
    let base = lisp.stack.len();
    let r = insert_file_contents(lisp, args);
    if let Ok(v) = r {
        lisp.stack.push(v);
    }
    let u = lisp.unbind_to(count);
    lisp.stack.truncate(base);
    let v = r?;
    u?;
    Ok(v)
}

fn insert_file_contents(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let arg = |i: usize| args.get(i).copied().unwrap_or(Value::NIL);
    let (visit, beg, end, replace) = (arg(1), arg(2), arg(3), arg(4));
    let empty_undo_list_p = !visit.is_nil() && lisp.undo_list().is_nil() && lisp.cur().z() == 1;
    let old_deactivate_mark = lisp.bound_var(sym::DEACTIVATE_MARK);
    lisp.stack.push(old_deactivate_mark);
    if !lisp.cur().base_buffer.is_nil() && !visit.is_nil() {
        return Err(lisp.error("Cannot do file visiting in an indirect buffer"));
    }
    if !lisp.find_symbol_value(sym::BUFFER_READ_ONLY).is_nil() {
        let pt = lisp.cur().pt;
        lisp.barf_if_buffer_read_only(pt)?;
    }
    let filename = lisp.check_string(arg(0))?;
    let filename = lisp.expand_file_name(filename, Value::NIL)?;
    lisp.stack.push(filename);
    let handler = lisp
        .find_file_name_handler(filename, sym::INSERT_FILE_CONTENTS)?
        .unwrap_or(Value::NIL);
    let b = lisp.cur();
    let mut s = InsertFile {
        filename,
        orig_filename: filename,
        visit,
        beg,
        end,
        replace,
        handler,
        val: Value::NIL,
        inserted: 0,
        total: 0,
        save_errno: None,
        set_coding_system: false,
        coding_system: Value::NIL,
        coding: CodingSystem::unset(),
        window_markers: Value::NIL,
        same_at_start: b.begv_byte,
        same_at_end_charpos: b.zv,
        mtime: (0, 0),
        st_size: -1,
        old_deactivate_mark,
    };
    if !handler.is_nil() {
        let val = lisp.funcall(
            handler,
            &[
                sym::INSERT_FILE_CONTENTS,
                filename,
                visit,
                beg,
                end,
                replace,
            ],
        )?;
        lisp.stack.push(val);
        s.val = val;
        if val.is_cons() && lisp.xcdr(val).is_cons() {
            let n = lisp.xcar(lisp.xcdr(val));
            let b = lisp.cur();
            if let Some(n) = n.as_fixnum()
                && (0..=(b.zv - b.pt) as i64).contains(&n)
            {
                s.inserted = n as usize;
            }
        }
    } else {
        insert_file_contents_body(lisp, &mut s)?;
    }
    insert_file_contents_handled(lisp, &mut s, empty_undo_list_p)
}

fn insert_file_contents_body(lisp: &mut Lisp, s: &mut InsertFile) -> LispResult<()> {
    if !s.visit.is_nil() {
        if !s.beg.is_nil() || !s.end.is_nil() {
            return Err(lisp.error("Attempt to visit less than an entire file"));
        }
        if lisp.cur().z() > 1 && s.replace.is_nil() {
            return Err(lisp.error("Cannot do file visiting in a non-empty buffer"));
        }
    }
    let mut beg_offset = if s.beg.is_nil() {
        0
    } else {
        file_offset(lisp, s.beg)?
    };
    let mut end_offset = if s.end.is_nil() {
        i64::MAX
    } else {
        file_offset(lisp, s.end)?
    };
    end_offset = end_offset.max(beg_offset);
    let path = PathBuf::from(lisp.string_to_rust(s.filename));
    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            if s.visit.is_nil() {
                return Err(lisp.file_error("Opening input file", &e, s.orig_filename));
            }
            s.mtime = time_error_value(&e);
            s.save_errno = Some(e);
            let csr = lisp.bound_var(sym::CODING_SYSTEM_FOR_READ);
            if !csr.is_nil() {
                lisp.check_coding_system_arg(csr)?;
                lisp.set(sym::BUFFER_FILE_CODING_SYSTEM, csr)?;
            }
            return insert_file_contents_notfound(lisp, s, Vec::new());
        }
    };
    if !s.replace.is_nil() {
        s.window_markers = lisp.get_window_points_and_markers()?;
        lisp.stack.push(s.window_markers);
        let head = lisp.xcar(s.window_markers);
        let marker = lisp.xcar(head);
        lisp.record_unwind(restore_point_unwind, marker);
    }
    let meta = match file.metadata() {
        Ok(m) => m,
        Err(e) => return Err(lisp.file_error("Input file status", &e, s.orig_filename)),
    };
    if meta.is_dir() {
        let e = std::io::Error::from(std::io::ErrorKind::IsADirectory);
        return Err(lisp.file_error("Read error", &e, s.orig_filename));
    }
    let regular = meta.is_file();
    if regular {
        s.st_size = meta.len() as i64;
    }
    s.mtime = stat_mtime(&meta);
    if !regular {
        if !s.visit.is_nil() || !s.replace.is_nil() {
            if !s.visit.is_nil() || s.replace != sym::IF_REGULAR {
                let msg = lisp.string("not a regular file");
                return Err(lisp.signal_list(sym::FILE_ERROR, &[msg, s.orig_filename]));
            }
            s.replace = sym::UNBOUND;
        }
        if !s.beg.is_nil() {
            let msg = lisp.string("can use a start position only in a regular file");
            return Err(lisp.signal_list(sym::FILE_ERROR, &[msg, s.orig_filename]));
        }
    }
    let likely_end = end_offset.min(s.st_size);
    let mut curpos: i64 = -1;
    let mut read_buf = vec![0u8; READ_BUF_SIZE];
    let csr = lisp.bound_var(sym::CODING_SYSTEM_FOR_READ);
    if csr == sym::AUTO_SAVE_CODING {
        s.coding_system = lisp.coding_inherit_eol_type(sym::UTF_8_EMACS, sym::UNIX)?;
        s.coding = lisp.setup_coding_system(s.coding_system)?;
        s.set_coding_system = true;
    } else if lisp.cur().z() > 1 {
        if !csr.is_nil() {
            s.coding_system = csr;
        } else {
            let sacf = lisp.bound_var(sym::SET_AUTO_CODING_FUNCTION);
            if regular && !sacf.is_nil() {
                if !s.beg.is_nil() {
                    curpos = lisp.xlseek(
                        &mut file,
                        SeekFrom::Start(beg_offset as u64),
                        s.orig_filename,
                    )?;
                } else {
                    curpos = lisp.xlseek(&mut file, SeekFrom::Current(0), s.orig_filename)?;
                    beg_offset = curpos;
                    end_offset = end_offset.max(beg_offset);
                }
                let trial = (end_offset - beg_offset).min(4 * 1024) as usize;
                let mut nread: i64 = if trial > 0 {
                    full_read(&mut file, &mut read_buf[..trial]).map_or(-1, |n| n as i64)
                } else {
                    0
                };
                curpos += nread;
                if nread == 4 * 1024 && curpos < end_offset {
                    curpos = file.seek(SeekFrom::End(-3 * 1024)).map_or(-1, |p| p as i64);
                    if curpos < 0 {
                        curpos = beg_offset + nread;
                    } else {
                        let tailbeg = if curpos <= beg_offset + nread {
                            beg_offset + nread
                        } else {
                            curpos.min(end_offset - 3 * 1024)
                        };
                        if tailbeg != curpos {
                            curpos = lisp.xlseek(
                                &mut file,
                                SeekFrom::Start(tailbeg as u64),
                                s.orig_filename,
                            )?;
                        }
                    }
                    let trial = (end_offset - curpos).min((READ_BUF_SIZE - 1024) as i64) as usize;
                    match full_read(&mut file, &mut read_buf[1024..1024 + trial]) {
                        Ok(n) => {
                            curpos += n as i64;
                            nread = n as i64 + 1024;
                        }
                        Err(_) => nread = -1,
                    }
                }
                if nread < 0 {
                    let e = std::io::Error::last_os_error();
                    return Err(lisp.file_error("Read error", &e, s.orig_filename));
                } else if nread > 0 {
                    let prev = lisp.current_buffer;
                    lisp.record_unwind_current_buffer();
                    let name = lisp.string(" *code-converting-work*");
                    let workbuf = lisp.get_buffer_create(name, true)?;
                    lisp.stack.push(workbuf);
                    crate::overlay::f_delete_all_overlays(lisp, &[workbuf])?;
                    let dir = lisp
                        .buffer_local_get(prev, sym::DEFAULT_DIRECTORY)
                        .unwrap_or(Value::NIL);
                    lisp.buffer_local_set(workbuf, sym::DEFAULT_DIRECTORY, dir);
                    lisp.buffer_local_set(workbuf, sym::BUFFER_READ_ONLY, Value::NIL);
                    lisp.buffer_local_set(workbuf, sym::BUFFER_FILE_NAME, Value::NIL);
                    lisp.buffer_local_set(workbuf, sym::BUFFER_UNDO_LIST, sym::T);
                    lisp.set_buffer_internal(workbuf);
                    crate::buffer::f_erase_buffer(lisp, &[])?;
                    lisp.set_multibyte_flag(workbuf, false);
                    let n = nread as usize;
                    let (pt, pt_byte) = (lisp.cur().pt, lisp.cur().pt_byte);
                    lisp.insert_from_gap(pt, pt_byte, &read_buf[..n], n, false);
                    lisp.set_pt_both(1, 1);
                    s.coding_system = lisp.funcall(sacf, &[s.filename, Value::fixnum(nread)])?;
                    lisp.stack.push(s.coding_system);
                    lisp.set_buffer_internal(prev);
                    lisp.specpdl.pop();
                }
            }
            if s.coding_system.is_nil() {
                let r = crate::coding::f_find_operation_coding_system(
                    lisp,
                    &[
                        sym::INSERT_FILE_CONTENTS,
                        s.orig_filename,
                        s.visit,
                        s.beg,
                        s.end,
                        s.replace,
                    ],
                )?;
                s.coding_system = if r.is_cons() { lisp.xcar(r) } else { r };
                lisp.stack.push(s.coding_system);
            }
        }
        if s.coding_system.is_nil() {
            s.coding_system = sym::UNDECIDED;
        } else {
            lisp.check_coding_system_arg(s.coding_system)?;
        }
        if !lisp.cur().text.multibyte {
            s.coding_system = lisp.raw_text_coding_system(s.coding_system)?;
        }
        s.coding = lisp.setup_coding_system(s.coding_system)?;
        s.set_coding_system = true;
    }
    let mut same_at_start = s.same_at_start;
    let mut same_at_end = lisp.cur().zv_byte;
    let mut replace_handled = false;
    let (begv, zv) = (lisp.cur().begv, lisp.cur().zv);
    if !s.replace.is_nil()
        && s.replace != sym::UNBOUND
        && begv < zv
        && (s.coding_system.is_nil() || !s.coding.require_decoding())
    {
        let mut giveup_match_end = false;
        let begv_byte = lisp.cur().begv_byte;
        let zv_byte = lisp.cur().zv_byte;
        if beg_offset != curpos {
            if !s.beg.is_nil() || 0 <= curpos {
                curpos = lisp.xlseek(
                    &mut file,
                    SeekFrom::Start(beg_offset as u64),
                    s.orig_filename,
                )?;
            } else {
                curpos = lisp.xlseek(&mut file, SeekFrom::Current(0), s.orig_filename)?;
                beg_offset = curpos;
                end_offset = end_offset.max(beg_offset);
            }
        }
        loop {
            let trial = (end_offset - curpos).min(READ_BUF_SIZE as i64) as usize;
            let nread = if trial > 0 {
                match read_some(&mut file, &mut read_buf[..trial]) {
                    Ok(n) => n,
                    Err(e) => return Err(lisp.file_error("Read error", &e, s.orig_filename)),
                }
            } else {
                0
            };
            if nread == 0 {
                drop(file);
                lisp.del_range_byte(same_at_start, same_at_end)?;
                s.same_at_start = same_at_start;
                return Ok(());
            }
            curpos += nread as i64;
            if s.coding.require_detection() {
                s.coding_system = lisp.detect_coding_system(
                    read_buf[..nread].to_vec(),
                    nread,
                    true,
                    false,
                    s.coding_system,
                )?;
                s.coding = lisp.setup_coding_system(s.coding_system)?;
            }
            if s.coding.require_decoding() {
                giveup_match_end = true;
                break;
            }
            let mut bufpos = 0;
            let bufposlim = nread.min(same_at_end - same_at_start);
            while bufpos < bufposlim && lisp.fetch_byte(same_at_start) == read_buf[bufpos] {
                same_at_start += 1;
                bufpos += 1;
            }
            if bufpos != nread {
                break;
            }
        }
        let mut endpos = 0i64;
        if !giveup_match_end {
            endpos = end_offset;
            if endpos == i64::MAX {
                endpos = file.seek(SeekFrom::End(0)).map_or(-1, |p| p as i64);
                if endpos < 0 {
                    endpos = curpos;
                }
                let n = match full_read(&mut file, &mut read_buf) {
                    Ok(n) => n,
                    Err(e) => return Err(lisp.file_error("Read error", &e, s.orig_filename)),
                };
                endpos += n as i64;
                curpos = endpos;
                giveup_match_end = n == READ_BUF_SIZE;
                if !giveup_match_end {
                    let offset_from_beg = endpos - beg_offset;
                    if offset_from_beg < (same_at_start - begv_byte) as i64 {
                        giveup_match_end = offset_from_beg < 0;
                        if !giveup_match_end {
                            same_at_start = offset_from_beg as usize + begv_byte;
                        }
                    }
                }
            }
        }
        while !giveup_match_end {
            let same_at_start_pos = beg_offset + (same_at_start - begv_byte) as i64;
            let file_overlap_pos = endpos - (same_at_end - same_at_start) as i64;
            let poslim = same_at_start_pos.max(file_overlap_pos);
            let trial = (curpos - poslim).min(READ_BUF_SIZE as i64);
            if trial == 0 {
                break;
            }
            curpos = lisp.xlseek(&mut file, SeekFrom::Current(-trial), s.orig_filename)?;
            let trial = trial as usize;
            let nread = match full_read(&mut file, &mut read_buf[..trial]) {
                Ok(n) => n,
                Err(e) => return Err(lisp.file_error("Read error", &e, s.orig_filename)),
            };
            curpos += nread as i64;
            if nread < trial {
                giveup_match_end = true;
                break;
            }
            let mut bufpos = nread;
            while bufpos > 0 && lisp.fetch_byte(same_at_end - 1) == read_buf[bufpos - 1] {
                same_at_end -= 1;
                bufpos -= 1;
            }
            if bufpos != 0 {
                if same_at_end > same_at_start
                    && lisp.fetch_byte(same_at_end - 1) >= 0x80
                    && lisp.cur().text.multibyte
                    && (s.coding.require_decoding() || s.coding.require_detection())
                {
                    giveup_match_end = true;
                }
                break;
            }
        }
        if !giveup_match_end {
            let multibyte = lisp.cur().text.multibyte;
            if multibyte {
                while same_at_start > begv_byte && !char_head_p(lisp.fetch_byte(same_at_start)) {
                    same_at_start -= 1;
                }
                while same_at_end < zv_byte && !char_head_p(lisp.fetch_byte(same_at_end)) {
                    same_at_end += 1;
                }
            }
            s.same_at_end_charpos = lisp.cur().text.byte_to_char(same_at_end);
            beg_offset += (same_at_start - begv_byte) as i64;
            end_offset = endpos - (zv_byte - same_at_end) as i64;
            let this_count = lisp.specpdl.len();
            let b = lisp.cur();
            if !s.visit.is_nil() && b.begv == 1 && b.z() == b.zv {
                lisp.specbind(sym::BUFFER_FILE_NAME, Value::NIL)?;
            }
            lisp.del_range_byte(same_at_start, same_at_end)?;
            let temp = lisp.cur().text.byte_to_char(same_at_start);
            lisp.set_pt_both(temp, same_at_start);
            lisp.unbind_to(this_count)?;
            replace_handled = true;
        }
    }
    s.same_at_start = same_at_start;
    let (begv, zv) = (lisp.cur().begv, lisp.cur().zv);
    if !s.replace.is_nil() && s.replace != sym::UNBOUND && !replace_handled && begv < zv {
        let this_count = lisp.specpdl.len();
        let multibyte = lisp.cur().text.multibyte;
        let conversion_buffer = lisp.code_conversion_save(true, multibyte)?;
        lisp.stack.push(conversion_buffer);
        if beg_offset != curpos {
            if !s.beg.is_nil() || 0 <= curpos {
                curpos = lisp.xlseek(
                    &mut file,
                    SeekFrom::Start(beg_offset as u64),
                    s.orig_filename,
                )?;
            } else {
                curpos = lisp.xlseek(&mut file, SeekFrom::Current(0), s.orig_filename)?;
                end_offset = end_offset.max(curpos);
            }
        }
        let mut unprocessed: Vec<u8> = Vec::new();
        loop {
            let trial =
                (end_offset - curpos).min((READ_BUF_SIZE - unprocessed.len()) as i64) as usize;
            let this = if trial > 0 {
                match read_some(&mut file, &mut read_buf[..trial]) {
                    Ok(n) => n,
                    Err(e) => return Err(lisp.file_error("Read error", &e, s.orig_filename)),
                }
            } else {
                0
            };
            if this == 0 {
                break;
            }
            curpos += this as i64;
            let cb = lisp.buffer_mut(conversion_buffer);
            cb.pt = cb.z();
            cb.pt_byte = cb.z_byte();
            let mut chunk = std::mem::take(&mut unprocessed);
            chunk.extend_from_slice(&read_buf[..this]);
            lisp.decode_coding_c_string(&mut s.coding, chunk, conversion_buffer)?;
            unprocessed = std::mem::take(&mut s.coding.carryover);
        }
        drop(file);
        if !unprocessed.is_empty() {
            let cb = lisp.buffer_mut(conversion_buffer);
            cb.pt = cb.z();
            cb.pt_byte = cb.z_byte();
            s.coding.mode |= crate::coding::MODE_LAST_BLOCK;
            lisp.decode_coding_c_string(&mut s.coding, unprocessed, conversion_buffer)?;
            s.coding.mode &= !crate::coding::MODE_LAST_BLOCK;
        }
        s.coding_system = lisp.coding_id_name(s.coding.id);
        s.set_coding_system = true;
        let decoded = {
            let t = &lisp.buffer(conversion_buffer).text;
            t.slice(0, t.bytes())
        };
        let mut inserted = decoded.len();
        let begv_byte = lisp.cur().begv_byte;
        let zv_byte = lisp.cur().zv_byte;
        let mut bufpos = 0;
        let bufposlim = inserted.min(same_at_end - same_at_start);
        while bufpos < bufposlim && lisp.fetch_byte(same_at_start) == decoded[bufpos] {
            same_at_start += 1;
            bufpos += 1;
        }
        if bufpos == inserted {
            if same_at_start != same_at_end {
                let b = lisp.cur();
                if !s.visit.is_nil() && b.begv == 1 && b.z() == b.zv {
                    lisp.specbind(sym::BUFFER_FILE_NAME, Value::NIL)?;
                }
                lisp.del_range_byte(same_at_start, same_at_end)?;
            }
            s.inserted = 0;
            lisp.unbind_to(this_count)?;
            s.same_at_start = same_at_start;
            return Ok(());
        }
        let current_multibyte = lisp.cur().text.multibyte;
        if current_multibyte {
            while same_at_start > begv_byte && !char_head_p(lisp.fetch_byte(same_at_start)) {
                same_at_start -= 1;
            }
        }
        let bufposlim = ((same_at_start - begv_byte) as i64)
            .max(inserted as i64 - (same_at_end - same_at_start) as i64);
        let mut bufpos = inserted as i64;
        while bufposlim < bufpos && lisp.fetch_byte(same_at_end - 1) == decoded[bufpos as usize - 1]
        {
            same_at_end -= 1;
            bufpos -= 1;
        }
        if current_multibyte {
            while same_at_end < zv_byte && !char_head_p(lisp.fetch_byte(same_at_end)) {
                same_at_end += 1;
            }
        }
        s.same_at_end_charpos = lisp.cur().text.byte_to_char(same_at_end);
        inserted -= (zv_byte - same_at_end) + (same_at_start - begv_byte);
        let b = lisp.cur();
        if !s.visit.is_nil() && b.begv == 1 && b.z() == b.zv {
            lisp.specbind(sym::BUFFER_FILE_NAME, Value::NIL)?;
        }
        let temp = if same_at_end != same_at_start {
            lisp.del_range_byte(same_at_start, same_at_end)?;
            lisp.cur().text.byte_to_char(same_at_start)
        } else {
            s.same_at_end_charpos
        };
        lisp.set_pt_both(temp, same_at_start);
        let (start_charpos, end_charpos) = {
            let t = &lisp.buffer(conversion_buffer).text;
            (
                t.byte_to_char(same_at_start - begv_byte + 1),
                t.byte_to_char(same_at_start + inserted - begv_byte + 1),
            )
        };
        crate::editfns::f_insert_buffer_substring(
            lisp,
            &[
                conversion_buffer,
                Value::fixnum(start_charpos as i64),
                Value::fixnum(end_charpos as i64),
            ],
        )?;
        s.inserted = lisp.cur().pt - temp;
        lisp.set_pt_both(temp, same_at_start);
        lisp.unbind_to(this_count)?;
        s.same_at_start = same_at_start;
        return Ok(());
    }
    if beg_offset != curpos {
        if !s.beg.is_nil() || 0 <= curpos {
            lisp.xlseek(
                &mut file,
                SeekFrom::Start(beg_offset as u64),
                s.orig_filename,
            )?;
        } else if end_offset != i64::MAX {
            curpos = file.stream_position().map_or(-1, |p| p as i64);
            beg_offset = curpos.max(0);
            end_offset = end_offset.max(beg_offset);
        }
    }
    s.total = end_offset - beg_offset;
    let mut we_locked_file = false;
    if s.visit.is_nil() && s.total > 0 {
        let truename = lisp.find_symbol_value(sym::BUFFER_FILE_TRUENAME);
        let fname = lisp.find_symbol_value(sym::BUFFER_FILE_NAME);
        let t = &lisp.cur().text;
        if !truename.is_nil() && !fname.is_nil() && t.save_modiff >= t.modiff {
            we_locked_file = true;
        }
        let pt = lisp.cur().pt;
        lisp.prepare_to_modify_buffer(pt, pt)?;
    }
    if s.replace == sym::UNBOUND {
        let (begv, zv) = (lisp.cur().begv, lisp.cur().zv);
        lisp.del_range(begv, zv)?;
    }
    let estimate = (likely_end - beg_offset).max(0) as usize;
    let mut raw: Vec<u8> = Vec::with_capacity(estimate.min(1 << 30));
    let mut read_error = None;
    while (raw.len() as i64) < s.total {
        let bufsize = (s.total - raw.len() as i64).min(INSERT_READ_SIZE_MAX as i64) as usize;
        let old = raw.len();
        raw.resize(old + bufsize, 0);
        match read_some(&mut file, &mut raw[old..]) {
            Ok(0) => {
                raw.truncate(old);
                break;
            }
            Ok(n) => raw.truncate(old + n),
            Err(e) => {
                raw.truncate(old);
                read_error = Some(e);
                break;
            }
        }
    }
    if raw.is_empty() {
        if we_locked_file {
            let truename = lisp.find_symbol_value(sym::BUFFER_FILE_TRUENAME);
            lisp.unlock_file_if_supported(truename)?;
        }
        lisp.set_plain_value(sym::DEACTIVATE_MARK, s.old_deactivate_mark);
    } else {
        lisp.set(sym::DEACTIVATE_MARK, sym::T)?;
    }
    drop(file);
    if let Some(e) = read_error {
        return Err(lisp.file_error("Read error", &e, s.orig_filename));
    }
    s.inserted = raw.len();
    insert_file_contents_notfound(lisp, s, raw)
}

fn insert_file_contents_notfound(
    lisp: &mut Lisp,
    s: &mut InsertFile,
    mut raw: Vec<u8>,
) -> LispResult<()> {
    if s.coding_system.is_nil() {
        let csr = lisp.bound_var(sym::CODING_SYSTEM_FOR_READ);
        if !csr.is_nil() {
            s.coding_system = csr;
        } else {
            let buffer = lisp.current_buffer;
            let multibyte = lisp.bool(lisp.cur().text.multibyte);
            let undo_list = lisp.undo_list();
            let tail = lisp.cons(undo_list, buffer);
            let unwind_data = lisp.cons(multibyte, tail);
            lisp.stack.push(unwind_data);
            let count1 = lisp.specpdl.len();
            lisp.set_multibyte_flag(buffer, false);
            lisp.set_undo_list(sym::T);
            lisp.record_unwind(decide_coding_unwind, unwind_data);
            {
                let n = raw.len();
                let b = lisp.cur_mut();
                b.text.insert_bytes(1, &raw, n);
                b.zv += n;
                b.zv_byte += n;
            }
            let sacf = lisp.bound_var(sym::SET_AUTO_CODING_FUNCTION);
            if s.inserted > 0 && !sacf.is_nil() {
                let n = Value::fixnum(s.inserted as i64);
                s.coding_system = lisp.funcall(sacf, &[s.filename, n])?;
                lisp.stack.push(s.coding_system);
            }
            if s.coding_system.is_nil() {
                let r = crate::coding::f_find_operation_coding_system(
                    lisp,
                    &[
                        sym::INSERT_FILE_CONTENTS,
                        s.orig_filename,
                        s.visit,
                        s.beg,
                        s.end,
                        Value::NIL,
                    ],
                )?;
                s.coding_system = if r.is_cons() { lisp.xcar(r) } else { r };
                lisp.stack.push(s.coding_system);
            }
            lisp.unbind_to(count1)?;
            let text = lisp.xcar(unwind_data);
            raw = lisp.string_bytes(text).to_vec();
            s.inserted = raw.len();
        }
        if s.coding_system.is_nil() {
            s.coding_system = sym::UNDECIDED;
        } else {
            lisp.check_coding_system_arg(s.coding_system)?;
        }
        if !lisp.cur().text.multibyte {
            s.coding_system = lisp.raw_text_coding_system(s.coding_system)?;
        }
        s.coding = lisp.setup_coding_system(s.coding_system)?;
        s.set_coding_system = true;
    }
    if !s.visit.is_nil() && s.coding.common_flags & FOR_UNIBYTE_MASK != 0 && s.replace.is_nil() {
        if s.inserted > 0 {
            let buffer = lisp.current_buffer;
            lisp.set_multibyte_flag(buffer, false);
        } else {
            crate::buffer::f_set_buffer_multibyte(lisp, &[Value::NIL])?;
        }
    }
    s.coding.dst_multibyte = lisp.cur().text.multibyte;
    if (s.coding.require_decoding() || s.coding.require_detection())
        && (s.inserted > 0 || s.coding.common_flags & REQUIRE_FLUSHING_MASK != 0)
    {
        lisp.decode_coding_gap(&mut s.coding, raw)?;
        s.inserted = s.coding.produced_char;
        s.coding_system = lisp.coding_id_name(s.coding.id);
    } else if s.inserted > 0 {
        let (pt, pt_byte) = (lisp.cur().pt, lisp.cur().pt_byte);
        lisp.insert_from_gap(pt, pt_byte, &raw, s.inserted, false);
    }
    if s.inserted > 0 && s.total > 0 && (s.visit.is_nil() || !s.replace.is_nil()) {
        let pt = lisp.cur().pt;
        lisp.signal_after_change(pt, 0, s.inserted)?;
        let pt = lisp.cur().pt;
        lisp.update_compositions(pt, pt, crate::composite::CHECK_BORDER)?;
    }
    Ok(())
}

fn insert_file_contents_handled(
    lisp: &mut Lisp,
    s: &mut InsertFile,
    empty_undo_list_p: bool,
) -> LispResult {
    if s.inserted > 0 {
        let start = lisp.cur().text.byte_to_char(s.same_at_start);
        lisp.restore_window_points(s.window_markers, s.inserted, start, s.same_at_end_charpos)?;
    }
    if !s.visit.is_nil() {
        if empty_undo_list_p {
            lisp.set_undo_list(Value::NIL);
        }
        let buffer = lisp.current_buffer;
        if s.handler.is_nil() {
            let b = lisp.cur_mut();
            b.modtime = s.mtime;
            b.modtime_size = s.st_size;
            lisp.buffer_local_set(buffer, sym::BUFFER_FILE_NAME, s.orig_filename);
        }
        let t = &mut lisp.cur_mut().text;
        t.save_modiff = t.modiff;
        t.autosave_modiff = t.modiff;
        let size = Value::fixnum(lisp.cur().z() as i64 - 1);
        lisp.buffer_local_set(buffer, sym::BUFFER_SAVED_SIZE, size);
        if s.handler.is_nil() {
            let truename = lisp.find_symbol_value(sym::BUFFER_FILE_TRUENAME);
            if !truename.is_nil() {
                lisp.unlock_file_if_supported(truename)?;
            }
            lisp.unlock_file_if_supported(s.filename)?;
        }
    }
    if s.set_coding_system {
        lisp.set_plain_value(sym::LAST_CODING_SYSTEM_USED, s.coding_system);
    }
    if lisp.fboundp(sym::AFTER_INSERT_FILE_SET_CODING) {
        let n = Value::fixnum(s.inserted as i64);
        let insval = lisp.funcall(sym::AFTER_INSERT_FILE_SET_CODING, &[n, s.visit])?;
        if !insval.is_nil() {
            s.inserted = lisp.check_inserted(insval)?;
        }
    }
    if s.inserted > 0 && lisp.fboundp(sym::FORMAT_DECODE) {
        let count1 = lisp.specpdl.len();
        let old_inserted = s.inserted;
        lisp.specbind(sym::INHIBIT_POINT_MOTION_HOOKS, sym::T)?;
        lisp.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
        let old_undo = lisp.undo_list();
        lisp.stack.push(old_undo);
        lisp.set_undo_list(sym::T);
        if s.replace.is_nil() {
            let n = Value::fixnum(s.inserted as i64);
            let insval = lisp.funcall(sym::FORMAT_DECODE, &[Value::NIL, n, s.visit])?;
            s.inserted = lisp.check_inserted(insval)?;
        } else {
            let (opoint, opoint_byte) = (lisp.cur().pt, lisp.cur().pt_byte);
            let b = lisp.cur();
            let (oinserted, ochars_modiff) = (b.zv - b.begv, b.text.chars_modiff);
            let (begv, begv_byte) = (b.begv, b.begv_byte);
            lisp.set_pt_both(begv, begv_byte);
            let n = Value::fixnum(oinserted as i64);
            let insval = lisp.funcall(sym::FORMAT_DECODE, &[Value::NIL, n, s.visit])?;
            let n = lisp.check_inserted(insval)?;
            if ochars_modiff == lisp.cur().text.chars_modiff {
                lisp.set_pt_both(opoint, opoint_byte);
            } else {
                s.inserted = n;
            }
        }
        let mut p = lisp.bound_var(sym::AFTER_INSERT_FILE_FUNCTIONS);
        lisp.stack.push(p);
        while p.is_cons() {
            let f = lisp.xcar(p);
            if s.replace.is_nil() {
                let n = Value::fixnum(s.inserted as i64);
                let insval = lisp.funcall(f, &[n])?;
                if !insval.is_nil() {
                    s.inserted = lisp.check_inserted(insval)?;
                }
            } else {
                let (opoint, opoint_byte) = (lisp.cur().pt, lisp.cur().pt_byte);
                let b = lisp.cur();
                let (oinserted, ochars_modiff) = (b.zv - b.begv, b.text.chars_modiff);
                let (begv, begv_byte) = (b.begv, b.begv_byte);
                lisp.set_pt_both(begv, begv_byte);
                let n = Value::fixnum(oinserted as i64);
                let insval = lisp.funcall(f, &[n])?;
                if !insval.is_nil() {
                    let n = lisp.check_inserted(insval)?;
                    if ochars_modiff == lisp.cur().text.chars_modiff {
                        lisp.set_pt_both(opoint, opoint_byte);
                    } else {
                        s.inserted = n;
                    }
                }
            }
            p = lisp.xcdr(p);
        }
        if !empty_undo_list_p {
            lisp.set_undo_list(old_undo);
            if old_undo.is_cons() && s.inserted != old_inserted {
                let tem = lisp.xcar(old_undo);
                if tem.is_cons() {
                    let (a, d) = (lisp.xcar(tem), lisp.xcdr(tem));
                    let pt = lisp.cur().pt as i64;
                    if a.is_fixnum() && d.as_fixnum() == Some(pt + old_inserted as i64) {
                        lisp.heap
                            .set_cdr(tem, Value::fixnum(pt + s.inserted as i64));
                    }
                }
            }
        } else {
            lisp.set_undo_list(if old_undo == sym::T {
                sym::T
            } else {
                Value::NIL
            });
        }
        lisp.unbind_to(count1)?;
    }
    if let Some(e) = s.save_errno.take() {
        return Err(lisp.file_error("Opening input file", &e, s.orig_filename));
    }
    if s.val.is_nil() {
        let n = Value::fixnum(s.inserted as i64);
        s.val = lisp.list(&[s.orig_filename, n]);
    }
    Ok(s.val)
}

fn f_write_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (start, end, filename, append, visit) = (args[0], args[1], args[2], args[3], args[4]);
    let filename = lisp.check_string(filename)?;
    if let Some(r) = lisp.handled(sym::WRITE_REGION, args)? {
        return Ok(r);
    }
    let expanded = lisp.expand_file_name(filename, Value::NIL)?;
    lisp.stack.push(expanded);
    let path = PathBuf::from(lisp.string_to_rust(expanded));
    let data: Vec<u8> = if start.is_string() {
        lisp.string_to_rust(start).into_bytes()
    } else {
        let (s, e) = if start.is_nil() {
            (lisp.cur().begv, lisp.cur().zv)
        } else {
            lisp.validate_region(start, end)?
        };
        let text = lisp.buffer_substring_value(s, e, false);
        let multibyte = lisp.string_multibyte(text);
        if multibyte {
            lisp.string_to_rust(text).into_bytes()
        } else {
            lisp.string_bytes(text).to_vec()
        }
    };
    let result = if !append.is_nil() {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .and_then(|mut f| f.write_all(&data))
    } else {
        std::fs::write(&path, &data)
    };
    if let Err(e) = result {
        lisp.stack.pop();
        return Err(lisp.file_error("Opening output file", &e, expanded));
    }
    if visit == sym::T || visit.is_string() {
        let b = lisp.current_buffer;
        lisp.buffer_local_set(b, sym::BUFFER_FILE_NAME, expanded);
        let modiff = lisp.cur().text.modiff;
        lisp.cur_mut().text.save_modiff = modiff;
    }
    if visit.is_nil() || visit == sym::T {
        let name = lisp.string_to_rust(expanded);
        if !lisp.noninteractive() {
            let msg = lisp.string(&format!("Wrote {name}"));
            lisp.message_string(msg)?;
        }
    }
    lisp.stack.pop();
    Ok(Value::NIL)
}

fn f_car_less_than_car(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.car(args[0])?;
    let b = lisp.car(args[1])?;
    let (lt, _, _) = lisp.arithcompare(a, b)?;
    Ok(lisp.bool(lt))
}
