use std::path::{Path, PathBuf};

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

fn f_insert_file_contents(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (filename, visit, beg, end, replace) = (args[0], args[1], args[2], args[3], args[4]);
    let filename = lisp.check_string(filename)?;
    if let Some(r) = lisp.handled(sym::INSERT_FILE_CONTENTS, args)? {
        return Ok(r);
    }
    let expanded = lisp.expand_file_name(filename, Value::NIL)?;
    lisp.stack.push(expanded);
    let path = PathBuf::from(lisp.string_to_rust(expanded));
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            lisp.stack.pop();
            if !visit.is_nil() && e.kind() == std::io::ErrorKind::NotFound {
                let b = lisp.current_buffer;
                lisp.buffer_local_set(b, sym::BUFFER_FILE_NAME, expanded);
            }
            return Err(lisp.file_error("Opening input file", &e, expanded));
        }
    };
    let start = if beg.is_nil() {
        0
    } else {
        lisp.check_fixnat(beg)? as usize
    };
    let stop = if end.is_nil() {
        bytes.len()
    } else {
        (lisp.check_fixnat(end)? as usize).min(bytes.len())
    };
    let slice = &bytes[start.min(stop)..stop];
    if !replace.is_nil() {
        let (begv, zv) = (lisp.cur().begv, lisp.cur().zv);
        lisp.del_range(begv, zv)?;
    }
    let pt = lisp.cur().pt;
    let multibyte = lisp.cur().text.multibyte;
    let (data, chars) = if multibyte {
        let normalized = normalize_eol(slice);
        crate::lread::decode_utf8_emacs(&normalized)
    } else {
        (slice.to_vec(), slice.len())
    };
    lisp.insert_internal(&data, chars, &[], false, false)?;
    lisp.goto_char_internal(pt);
    if !visit.is_nil() {
        let b = lisp.current_buffer;
        lisp.buffer_local_set(b, sym::BUFFER_FILE_NAME, expanded);
        let modiff = lisp.cur().text.modiff;
        lisp.cur_mut().text.save_modiff = modiff;
        let utf8 = lisp.intern("utf-8-unix");
        lisp.buffer_local_set(b, sym::BUFFER_FILE_CODING_SYSTEM, utf8);
        lisp.buffer_local_set(b, sym::BUFFER_UNDO_LIST, Value::NIL);
    }
    lisp.stack.pop();
    let count = Value::fixnum(chars as i64);
    Ok(lisp.list(&[expanded, count]))
}

fn normalize_eol(bytes: &[u8]) -> Vec<u8> {
    if !bytes.contains(&b'\r') {
        return bytes.to_vec();
    }
    let crlf = bytes.windows(2).any(|w| w == b"\r\n");
    if !crlf {
        return bytes.to_vec();
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
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
