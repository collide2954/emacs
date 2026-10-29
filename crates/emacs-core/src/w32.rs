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
