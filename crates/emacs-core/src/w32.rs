use crate::lisp::Lisp;

#[cfg(windows)]
pub fn init_environment(lisp: &mut Lisp) {
    lisp.w32_init_environment();
}

#[cfg(not(windows))]
pub fn init_environment(_lisp: &mut Lisp) {}

#[cfg(windows)]
const REG_ROOT: &[u8] = b"SOFTWARE\\GNU\\Emacs";

#[cfg(windows)]
const MAX_UTF8_PATH: usize = 260 * 4;

#[cfg(windows)]
fn is_directory_sep(c: u8) -> bool {
    c == b'/' || c == b'\\'
}

#[cfg(windows)]
fn normalize_filename(name: &mut [u8], path_sep: u8) {
    let mut i = 0;
    if name.len() > 1 && name[1] == b':' && name[0].is_ascii_uppercase() {
        name[0] = name[0].to_ascii_lowercase();
        i = 2;
    }
    for c in &mut name[i..] {
        if is_directory_sep(*c) && *c != path_sep {
            *c = path_sep;
        }
    }
}

#[cfg(windows)]
fn parse_root(name: &[u8]) -> usize {
    let at = |i: usize| name.get(i).copied().unwrap_or(0);
    let mut i = 0;
    if at(0).is_ascii_alphabetic() && at(1) == b':' {
        i = 2;
        if is_directory_sep(at(i)) {
            i += 1;
        }
    } else if is_directory_sep(at(0)) && is_directory_sep(at(1)) {
        let mut slashes = 2;
        i = 2;
        while at(i) != 0 {
            if is_directory_sep(at(i)) {
                slashes -= 1;
                if slashes == 0 {
                    break;
                }
            }
            i += 1;
        }
        if is_directory_sep(at(i)) {
            i += 1;
        }
    }
    i
}

#[cfg(windows)]
fn get_long_basename(name: &[u8]) -> Option<Vec<u8>> {
    if name.iter().any(|c| b"*?|<>\"".contains(c)) {
        return None;
    }
    let name = String::from_utf8_lossy(name);
    let found = emacs_w32::find_first_file_name(&name)?;
    (!found.is_empty() && found.len() < MAX_UTF8_PATH).then(|| found.into_bytes())
}

#[cfg(windows)]
pub fn w32_get_long_filename(name: &[u8]) -> Option<Vec<u8>> {
    if name.len() >= MAX_UTF8_PATH {
        return None;
    }
    let mut full = name.to_vec();
    normalize_filename(&mut full, b'\\');
    let root = parse_root(&full);
    let mut out = full[..root].to_vec();
    let mut p = root;
    while p < full.len() {
        let end = full[p..].iter().position(|&c| c == b'\\').map(|i| p + i);
        let prefix_end = end.unwrap_or(full.len());
        let base = get_long_basename(&full[..prefix_end])?;
        out.extend_from_slice(&base);
        match end {
            Some(e) => {
                out.push(b'\\');
                if out.len() + 2 > MAX_UTF8_PATH {
                    return None;
                }
                p = e + 1;
            }
            None => p = full.len(),
        }
    }
    Some(out)
}

#[cfg(windows)]
pub const D_OK: i64 = 8;

#[cfg(windows)]
fn is_exec(name: &str) -> bool {
    name.rfind('.').is_some_and(|i| {
        let ext = &name[i..];
        [".exe", ".com", ".bat", ".cmd"]
            .iter()
            .any(|e| ext.eq_ignore_ascii_case(e))
    })
}

#[cfg(windows)]
fn access_attrs(path: &std::path::Path) -> std::io::Result<u32> {
    use std::io::{Error, ErrorKind};
    emacs_w32::file_attributes_wide(path).map_err(|w32err| match w32err {
        123 | 161 | 2 | 3 | 15 | 21 | 53 | 67 => Error::from(ErrorKind::NotFound),
        _ => Error::from(ErrorKind::PermissionDenied),
    })
}

#[cfg(windows)]
pub fn faccessat(path: &std::path::Path, mode: i64) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    use std::os::windows::fs::MetadataExt;
    let name = path.to_string_lossy();
    let mut mode = mode;
    if name.ends_with(['/', '\\']) && mode == 0 {
        mode |= D_OK;
    }
    let mut attributes = access_attrs(path)?;
    if attributes & emacs_w32::FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        attributes = match std::fs::metadata(path) {
            Ok(m) => m.file_attributes(),
            Err(_) => access_attrs(path)?,
        };
    }
    let denied = Error::from(ErrorKind::PermissionDenied);
    if mode & 1 != 0 && !(is_exec(&name) || attributes & emacs_w32::FILE_ATTRIBUTE_DIRECTORY != 0) {
        return Err(denied);
    }
    if mode & 2 != 0 && attributes & emacs_w32::FILE_ATTRIBUTE_READONLY != 0 {
        return Err(denied);
    }
    if mode & D_OK != 0 && attributes & emacs_w32::FILE_ATTRIBUTE_DIRECTORY == 0 {
        return Err(denied);
    }
    Ok(())
}

#[cfg(windows)]
fn until_nul(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    &bytes[..end]
}

#[cfg(windows)]
impl Lisp {
    fn w32_init_environment(&mut self) {
        use crate::symbols as sym;
        use emacs_w32::{REG_EXPAND_SZ, REG_SZ};
        for tmp in ["$TMPDIR", "$TEMP", "$TMP", "c:/"] {
            let dir = match tmp.strip_prefix('$') {
                Some(var) => emacs_w32::getenv(var.as_bytes()),
                None => Some(tmp.as_bytes().to_vec()),
            };
            if let Some(dir) = dir
                && emacs_w32::is_directory(&dir)
            {
                emacs_w32::putenv(b"TMPDIR", &dir);
                break;
            }
        }
        let mut env_vars: Vec<(&[u8], Option<Vec<u8>>)> = vec![
            (b"HOME", Some(b"C:/".to_vec())),
            (b"PRELOAD_WINSOCK", None),
            (b"emacs_dir", Some(b"C:/emacs".to_vec())),
            (b"EMACSLOADPATH", None),
            (b"SHELL", Some(b"cmdproxy.exe".to_vec())),
            (b"EMACSDATA", None),
            (b"EMACSPATH", None),
            (b"INFOPATH", None),
            (b"EMACSDOC", None),
            (b"TERM", Some(b"cmd".to_vec())),
            (b"LANG", None),
        ];
        let mut appdata = false;
        if !emacs_w32::file_exists(b"C:/.emacs")
            && let Some(default_home) = emacs_w32::appdata_folder()
        {
            env_vars[0].1 = Some(default_home);
            appdata = true;
        }
        if let Some(mut locale_name) = emacs_w32::abbrev_lang_name(emacs_w32::LOCALE_USER_DEFAULT) {
            if locale_name == b"ZZZ" {
                let lang_id = emacs_w32::user_default_ui_language();
                if lang_id != 0
                    && let Some(locale_name_def) = emacs_w32::abbrev_lang_name(u32::from(lang_id))
                {
                    locale_name = locale_name_def;
                }
            }
            if let Some(entry) = env_vars.iter_mut().find(|(name, _)| *name == b"LANG") {
                entry.1 = Some(locale_name);
            }
        }
        let mut modname = emacs_w32::module_file_name().expect("GetModuleFileNameA");
        let p = modname
            .iter()
            .rposition(|&b| b == b'\\')
            .expect("module file name has a directory");
        modname.truncate(p);
        if let Some(p) = modname.iter().rposition(|&b| b == b'\\') {
            let tail = &modname[p..];
            if tail.eq_ignore_ascii_case(b"\\bin") || tail.eq_ignore_ascii_case(b"\\src") {
                let within_build_tree = tail.eq_ignore_ascii_case(b"\\src");
                modname.truncate(p);
                for c in &mut modname {
                    if *c == b'\\' {
                        *c = b'/';
                    }
                }
                emacs_w32::putenv(b"emacs_dir", &modname);
                if within_build_tree {
                    let mut shell = modname.clone();
                    shell.extend_from_slice(b"/nt/cmdproxy.exe");
                    emacs_w32::putenv(b"SHELL", &shell);
                }
            }
        }
        for (name, def_value) in env_vars {
            if emacs_w32::getenv(name).is_some() {
                continue;
            }
            let (lpval, dw_type) = match emacs_w32::get_resource(REG_ROOT, name) {
                Some((value, kind)) if value.first().is_some_and(|&b| b != 0) => {
                    (Some(value), kind)
                }
                _ => {
                    let fallback = if name == b"SHELL" {
                        let path_exec = crate::emacs::path_exec();
                        path_exec
                            .as_bytes()
                            .split(|&c| c == b';')
                            .filter(|part| !part.is_empty())
                            .find_map(|part| {
                                let mut fname = part.to_vec();
                                fname.extend_from_slice(b"/cmdproxy.exe");
                                let bufc = emacs_w32::expand_environment_strings(&fname);
                                emacs_w32::file_exists(&bufc).then_some(bufc)
                            })
                            .map_or((def_value, REG_EXPAND_SZ), |found| (Some(found), REG_SZ))
                    } else {
                        (def_value, REG_EXPAND_SZ)
                    };
                    if name == b"HOME" && !appdata {
                        let kind = self.intern("initialization");
                        let text = self.string(
                            "Use of `C:\\.emacs' without defining `HOME'\n\
                             in the environment is deprecated, see `Windows HOME' in the Emacs manual.",
                        );
                        let warning = self.list(&[kind, text]);
                        let list = self.plain_value(sym::DELAYED_WARNINGS_LIST);
                        let list = self.cons(warning, list);
                        self.set_plain_value(sym::DELAYED_WARNINGS_LIST, list);
                    }
                    fallback
                }
            };
            if let Some(lpval) = lpval {
                let buf1 = if dw_type == REG_EXPAND_SZ {
                    emacs_w32::expand_environment_strings(&lpval)
                } else if dw_type == REG_SZ {
                    until_nul(&lpval).to_vec()
                } else {
                    continue;
                };
                emacs_w32::putenv(name, &buf1);
            }
        }
        let mut env = self.find_symbol_value(sym::PROCESS_ENVIRONMENT);
        while env.is_cons() {
            let entry = self.xcar(env);
            env = self.xcdr(env);
            if !entry.is_string() {
                continue;
            }
            for name in [&b"PATH="[..], &b"COMSPEC="[..]] {
                let data = &mut self.heap.string_mut(entry).data;
                if data.len() >= name.len() && data[..name.len()].eq_ignore_ascii_case(name) {
                    data[..name.len()].copy_from_slice(name);
                    break;
                }
            }
        }
    }
}
