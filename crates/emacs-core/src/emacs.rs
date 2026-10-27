use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::LispResult;
use crate::fileio::normalize_slashes;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("invocation-name", 0, 0, f_invocation_name),
    Subr::fixed("invocation-directory", 0, 0, f_invocation_directory),
    Subr::fixed("kill-emacs", 0, 2, f_kill_emacs).interactive("P"),
    Subr::fixed("daemonp", 0, 0, f_nil),
    Subr::fixed("daemon-initialized", 0, 0, f_nil),
    Subr::many("call-process", 1, f_call_process),
    Subr::many("call-process-region", 3, f_call_process_region),
    Subr::fixed("getenv-internal", 1, 2, f_getenv_internal),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::COMMAND_LINE_ARGS, Value::NIL);
    let system_type = if cfg!(windows) {
        "windows-nt"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "freebsd") {
        "berkeley-unix"
    } else {
        "gnu/linux"
    };
    let st = lisp.intern(system_type);
    lisp.defvar(sym::SYSTEM_TYPE, st);
    let config = format!(
        "{}-{}",
        std::env::consts::ARCH,
        if cfg!(windows) {
            "w64-mingw32"
        } else if cfg!(target_os = "macos") {
            "apple-darwin"
        } else {
            "pc-linux-gnu"
        }
    );
    let sc = lisp.string(&config);
    lisp.defvar(sym::SYSTEM_CONFIGURATION, sc);
    let empty = lisp.string("");
    lisp.defvar(sym::SYSTEM_CONFIGURATION_OPTIONS, empty);
    let features = lisp.string("");
    lisp.defvar(sym::SYSTEM_CONFIGURATION_FEATURES, features);
    lisp.defvar(sym::KILL_EMACS_HOOK, Value::NIL);
    let sep = lisp.string(if cfg!(windows) { ";" } else { ":" });
    lisp.defvar(sym::PATH_SEPARATOR, sep);
    lisp.defvar(sym::INSTALLATION_DIRECTORY, Value::NIL);
    lisp.defvar(sym::SYSTEM_MESSAGES_LOCALE, Value::NIL);
    lisp.defvar(sym::SYSTEM_TIME_LOCALE, Value::NIL);
    lisp.defvar(sym::BEFORE_INIT_TIME, Value::NIL);
    lisp.defvar(sym::AFTER_INIT_TIME, Value::NIL);
    lisp.defvar_bool(sym::INHIBIT_X_RESOURCES, false);
    let copyright = lisp.string("Copyright (C) 2026 Free Software Foundation, Inc.");
    lisp.defvar(sym::EMACS_COPYRIGHT, copyright);
    let bug = lisp.string("bug-gnu-emacs@gnu.org");
    lisp.defvar(sym::REPORT_EMACS_BUG_ADDRESS, bug);
    lisp.defvar(sym::DUMP_MODE, Value::NIL);
    lisp.defvar(sym::DYNAMIC_LIBRARY_ALIST, Value::NIL);
    let shell = if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    };
    let shell = lisp.string(&normalize_slashes(&shell));
    lisp.defvar(sym::SHELL_FILE_NAME, shell);
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs: Vec<Value> = std::env::split_paths(&path_var)
        .map(|p| {
            let s = normalize_slashes(&p.to_string_lossy());
            lisp.string(&s)
        })
        .collect();
    let exec_dir = lisp.plain_value(sym::INVOCATION_DIRECTORY);
    dirs.push(exec_dir);
    let exec_path = lisp.list(&dirs);
    lisp.defvar(sym::EXEC_PATH, exec_path);
    let suffixes: Vec<Value> = if cfg!(windows) {
        [".exe", ".com", ".bat", ".cmd", ""]
            .iter()
            .map(|s| lisp.string(s))
            .collect()
    } else {
        vec![lisp.string("")]
    };
    let suffixes = lisp.list(&suffixes);
    lisp.defvar(sym::EXEC_SUFFIXES, suffixes);
    lisp.defvar(sym::EXEC_DIRECTORY, exec_dir);
    lisp.defvar(sym::DATA_DIRECTORY, Value::NIL);
    lisp.defvar(sym::DOC_DIRECTORY, Value::NIL);
    lisp.defvar(sym::CONFIGURE_INFO_DIRECTORY, Value::NIL);
    lisp.defvar(sym::SHARED_GAME_SCORE_DIRECTORY, Value::NIL);
    let env: Vec<Value> = std::env::vars_os()
        .map(|(k, v)| {
            let s = format!("{}={}", k.to_string_lossy(), v.to_string_lossy());
            lisp.string(&s)
        })
        .collect();
    let env = lisp.list(&env);
    lisp.defvar(sym::INITIAL_ENVIRONMENT, env);
    let items = lisp.list_to_vec(env).unwrap_or_default();
    let process_environment = lisp.list(&items);
    lisp.defvar(sym::PROCESS_ENVIRONMENT, process_environment);
    for (symbol, name) in [
        (sym::CTAGS_PROGRAM_NAME, "ctags"),
        (sym::ETAGS_PROGRAM_NAME, "etags"),
        (sym::HEXL_PROGRAM_NAME, "hexl"),
        (sym::EMACSCLIENT_PROGRAM_NAME, "emacsclient"),
        (sym::MOVEMAIL_PROGRAM_NAME, "movemail"),
        (sym::EBROWSE_PROGRAM_NAME, "ebrowse"),
        (sym::RCS2LOG_PROGRAM_NAME, "rcs2log"),
    ] {
        let v = lisp.string(name);
        lisp.defvar(symbol, v);
    }
    lisp.defvar(sym::COMMAND_LINE_MAX_LENGTH, Value::fixnum(32768));
}

impl Lisp {
    pub fn set_source_directory(&mut self, root: &Path) {
        let as_dir = |p: &Path| {
            let mut s = normalize_slashes(&p.to_string_lossy());
            if !s.ends_with('/') {
                s.push('/');
            }
            s
        };
        let root_s = as_dir(root);
        let etc = as_dir(&root.join("etc"));
        let lisp_dir = normalize_slashes(&root.join("lisp").to_string_lossy());
        let r = self.string(&root_s);
        self.set_plain_value(sym::SOURCE_DIRECTORY, r);
        self.set_plain_value(sym::INSTALLATION_DIRECTORY, r);
        let e = self.string(&etc);
        self.set_plain_value(sym::DATA_DIRECTORY, e);
        self.set_plain_value(sym::DOC_DIRECTORY, e);
        let l = self.string(&lisp_dir);
        let load_path = self.list(&[l]);
        self.set_plain_value(sym::LOAD_PATH, load_path);
    }

    pub fn set_command_line_args(&mut self, args: &[String]) {
        let values: Vec<Value> = args.iter().map(|a| self.string(a)).collect();
        let list = self.list(&values);
        self.set_plain_value(sym::COMMAND_LINE_ARGS, list);
    }

    pub fn getenv(&mut self, name: &str) -> Option<String> {
        let env = self.find_symbol_value(sym::PROCESS_ENVIRONMENT);
        let mut tail = env;
        while tail.is_cons() {
            let entry = self.xcar(tail);
            tail = self.xcdr(tail);
            if !entry.is_string() {
                continue;
            }
            let text = self.string_to_rust(entry);
            let matches = |key: &str| {
                if cfg!(windows) {
                    key.eq_ignore_ascii_case(name)
                } else {
                    key == name
                }
            };
            match text.split_once('=') {
                Some((key, value)) if matches(key) => return Some(value.to_string()),
                None if matches(&text) => return None,
                _ => {}
            }
        }
        None
    }
}

pub fn find_source_root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("EMACS_SOURCE_DIR") {
        return Some(PathBuf::from(dir));
    }
    let candidates = [std::env::current_exe().ok(), std::env::current_dir().ok()];
    for start in candidates.into_iter().flatten() {
        let mut dir: Option<&Path> = Some(start.as_path());
        while let Some(d) = dir {
            if d.join("lisp").join("subr.el").is_file() {
                return Some(d.to_path_buf());
            }
            dir = d.parent();
        }
    }
    None
}

fn f_nil(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_invocation_name(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.plain_value(sym::INVOCATION_NAME))
}

fn f_invocation_directory(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.plain_value(sym::INVOCATION_DIRECTORY))
}

fn f_kill_emacs(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let arg = args[0];
    let hook = lisp.find_symbol_value(sym::KILL_EMACS_HOOK);
    if !hook.is_nil() && hook != sym::UNBOUND {
        let _ = lisp.run_hook(sym::KILL_EMACS_HOOK);
    }
    let code = if let Some(n) = arg.as_fixnum() {
        n as i32
    } else if arg.is_string() {
        let text = lisp.string_to_rust(arg);
        let _ = lisp.stdout.write_all(text.as_bytes());
        0
    } else if arg.is_nil() {
        0
    } else {
        1
    };
    let _ = lisp.stdout.flush();
    let _ = lisp.stderr.flush();
    std::process::exit(code);
}

fn f_getenv_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = lisp.check_string(args[0])?;
    let name = lisp.string_to_rust(variable);
    let value = if args[1].is_cons() {
        let count = lisp.specpdl.len();
        lisp.specbind(sym::PROCESS_ENVIRONMENT, args[1])?;
        let v = lisp.getenv(&name);
        lisp.unbind_to(count)?;
        v
    } else {
        lisp.getenv(&name)
    };
    match value {
        Some(v) => Ok(lisp.string(&v)),
        None => Ok(Value::NIL),
    }
}

fn run_process(
    lisp: &mut Lisp,
    program: Value,
    input: Option<Vec<u8>>,
    destination: Value,
    display: Value,
    args: &[Value],
) -> LispResult {
    let _ = display;
    let program = lisp.check_string(program)?;
    let prog = lisp.string_to_rust(program);
    let mut cmd = std::process::Command::new(&prog);
    for &a in args {
        let a = lisp.check_string(a)?;
        cmd.arg(lisp.string_to_rust(a));
    }
    let dir = lisp.default_directory_string();
    if Path::new(&dir).is_dir() {
        cmd.current_dir(&dir);
    }
    cmd.env_clear();
    let env = lisp.find_symbol_value(sym::PROCESS_ENVIRONMENT);
    let mut tail = env;
    while tail.is_cons() {
        let entry = lisp.xcar(tail);
        tail = lisp.xcdr(tail);
        if entry.is_string() {
            let text = lisp.string_to_rust(entry);
            if let Some((k, v)) = text.split_once('=') {
                cmd.env(k, v);
            }
        }
    }
    cmd.stdin(if input.is_some() {
        std::process::Stdio::piped()
    } else {
        std::process::Stdio::null()
    });
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Err(lisp.file_error("Searching for program", &e, program));
        }
    };
    if let Some(data) = input
        && let Some(mut stdin) = child.stdin.take()
    {
        let _ = stdin.write_all(&data);
    }
    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return Err(lisp.file_error("Running program", &e, program)),
    };
    let (dest, stderr_dest) = if destination.is_cons() {
        (
            lisp.xcar(destination),
            lisp.car_safe(lisp.xcdr(destination)),
        )
    } else {
        (destination, Value::NIL)
    };
    let mut text = output.stdout.clone();
    if stderr_dest == sym::T || (stderr_dest.is_nil() && !destination.is_cons()) {
        text.extend_from_slice(&output.stderr);
    }
    if dest == sym::T || lisp.is_buffer(dest) || dest.is_string() {
        let count = lisp.specpdl.len();
        lisp.record_unwind_current_buffer();
        if lisp.is_buffer(dest) {
            lisp.set_buffer_internal(dest);
        } else if dest.is_string() {
            let b = lisp.get_buffer_create(dest, false)?;
            lisp.set_buffer_internal(b);
        }
        let normalized: Vec<u8> = if cfg!(windows) {
            String::from_utf8_lossy(&text)
                .replace("\r\n", "\n")
                .into_bytes()
        } else {
            text
        };
        let (data, chars) = crate::lread::decode_utf8_emacs(&normalized);
        lisp.insert_internal(&data, chars, &[], false, false)?;
        lisp.unbind_to(count)?;
    }
    Ok(Value::fixnum(output.status.code().unwrap_or(-1) as i64))
}

fn f_call_process(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let program = args[0];
    let infile = args.get(1).copied().unwrap_or(Value::NIL);
    let destination = args.get(2).copied().unwrap_or(Value::NIL);
    let display = args.get(3).copied().unwrap_or(Value::NIL);
    let input = if infile.is_string() {
        let path = lisp.expand_file_name(infile, Value::NIL)?;
        let p = lisp.string_to_rust(path);
        std::fs::read(p).ok()
    } else {
        None
    };
    let rest = if args.len() > 4 { &args[4..] } else { &[] };
    run_process(lisp, program, input, destination, display, rest)
}

fn f_call_process_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (start, end, program) = (args[0], args[1], args[2]);
    let delete = args.get(3).copied().unwrap_or(Value::NIL);
    let destination = args.get(4).copied().unwrap_or(Value::NIL);
    let display = args.get(5).copied().unwrap_or(Value::NIL);
    let input = if start.is_string() {
        lisp.string_to_rust(start).into_bytes()
    } else {
        let (s, e) = if start.is_nil() {
            (lisp.cur().begv, lisp.cur().zv)
        } else {
            lisp.validate_region(start, end)?
        };
        let text = lisp.buffer_substring_value(s, e, false);
        let t = lisp.string_to_rust(text).into_bytes();
        if !delete.is_nil() {
            lisp.del_range(s, e)?;
        }
        t
    };
    let rest = if args.len() > 6 { &args[6..] } else { &[] };
    run_process(lisp, program, Some(input), destination, display, rest)
}
