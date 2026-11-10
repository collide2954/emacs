use std::ffi::OsStr;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

use crate::{
    batch_file_p, canon_filename, get_next_token, parse_switches, run_command_dot_com,
    same_program_name, shell_command_line, truncate_message, try_dequote_cmdline,
};

const MAX_PATH: usize = 260;

const EXTS: [&str; 4] = [".bat", ".cmd", ".exe", ".com"];

static CHILD: AtomicIsize = AtomicIsize::new(0);

static INTERACTIVE: AtomicBool = AtomicBool::new(true);

fn lossy(s: &OsStr) -> String {
    s.to_string_lossy().into_owned()
}

fn warn(msg: &str) {
    let _ = std::io::stderr().write_all(truncate_message(msg).as_bytes());
}

fn fail(msg: &str) -> ! {
    warn(msg);
    std::process::exit(-1)
}

fn console_event_handler(event: u32) -> bool {
    if (event == emacs_w32::CTRL_C_EVENT || event == emacs_w32::CTRL_BREAK_EVENT)
        && !INTERACTIVE.load(Ordering::SeqCst)
    {
        let child = CHILD.load(Ordering::SeqCst);
        if child != 0 && !emacs_w32::wait_for_handle(child, 500) {
            emacs_w32::terminate_handle(child, 0);
        }
        std::process::exit(emacs_w32::STATUS_CONTROL_C_EXIT as i32);
    }
    true
}

fn search(dir: &str, file: &str, extension: Option<&str>) -> Option<String> {
    emacs_w32::search_path(
        Some(OsStr::new(dir)),
        OsStr::new(file),
        extension.map(OsStr::new),
    )
    .map(|found| lossy(&found))
}

fn search_dir(dir: &str, exec: &str) -> Option<String> {
    let pext = exec.rfind('\\').map_or(exec, |i| &exec[i..]);
    if pext.contains('.') {
        search(dir, exec, None).or_else(|| {
            EXTS.iter()
                .find_map(|ext| search(dir, &format!("{exec}{ext}"), None))
        })
    } else {
        EXTS.iter().find_map(|ext| search(dir, exec, Some(ext)))
    }
}

fn make_absolute(prog: &str) -> Option<String> {
    let b = prog.as_bytes();
    if (b.first().is_some_and(u8::is_ascii_alphabetic) && b.get(1) == Some(&b':'))
        || b.first() == Some(&b'\\')
    {
        let fname = prog.rfind('\\').unwrap_or(2);
        return search_dir(&prog[..fname], prog);
    }
    let curdir = lossy(std::env::current_dir().ok()?.as_os_str());
    if prog.contains('\\') {
        return search_dir(&curdir, prog);
    }
    let path = format!(
        "{curdir};{}",
        lossy(&std::env::var_os("PATH").unwrap_or_default())
    );
    let mut rest = path.as_str();
    while !rest.is_empty() {
        let end = rest.find(';').unwrap_or(rest.len());
        let dir = &rest[..rest.floor_char_boundary(end.min(MAX_PATH - 1))];
        if let Some(found) = search_dir(dir, prog) {
            return Some(found);
        }
        rest = rest.get(end + 1..).unwrap_or("");
    }
    None
}

fn spawn(progname: Option<&str>, cmdline: &OsStr, dir: &OsStr) -> Option<u32> {
    let progname =
        progname.filter(|p| !(cmdline.to_string_lossy().starts_with('"') && batch_file_p(p)));
    let child = emacs_w32::create_process(progname.map(OsStr::new), cmdline, dir)?;
    CHILD.store(child.handle(), Ordering::SeqCst);
    let rc = child.wait();
    CHILD.store(0, Ordering::SeqCst);
    Some(rc)
}

pub fn run() -> i32 {
    INTERACTIVE.store(true, Ordering::SeqCst);
    emacs_w32::set_console_ctrl_handler(console_event_handler);
    let dir = std::env::current_dir()
        .unwrap_or_else(|_| fail("error: GetCurrentDirectory failed\n"))
        .into_os_string();
    let modname = lossy(
        std::env::current_exe()
            .unwrap_or_else(|_| fail("error: GetModuleFileName failed\n"))
            .as_os_str(),
    );
    if let Some(i) = modname.rfind('\\') {
        let _ = std::env::set_current_dir(&modname[..i]);
    }
    let acp = emacs_w32::ansi_code_page();
    emacs_w32::set_console_cp(acp);
    emacs_w32::set_console_output_cp(acp);
    let args: Vec<String> = std::env::args_os().map(|arg| lossy(&arg)).collect();
    let argv0 = args.first().cloned().unwrap_or_default();
    let found = emacs_w32::search_path(None, OsStr::new(&argv0), Some(OsStr::new(".exe")))
        .map(|found| lossy(&found));
    if !found
        .as_ref()
        .is_some_and(|path| modname.eq_ignore_ascii_case(path))
    {
        let mut path = found.unwrap_or_else(|| argv0.replace('/', "\\"));
        let mut modname = modname.clone();
        let mut same = false;
        if let Some(short) = emacs_w32::short_path_name(OsStr::new(&modname)) {
            modname = lossy(&short);
            if let Some(short) = emacs_w32::short_path_name(OsStr::new(&path)) {
                path = lossy(&short);
                same = modname.eq_ignore_ascii_case(&path);
            }
        }
        if !same && !same_program_name(&path, &modname) {
            let command_line = emacs_w32::command_line();
            if let Some(rc) = spawn(None, &command_line, &dir) {
                return rc as i32;
            }
            fail(&format!("Could not run {}\n", lossy(&command_line)));
        }
    }
    let envsize = emacs_w32::environment_size() as i32 + 300;
    let switches = match parse_switches(&args, envsize, &mut |w| warn(w)) {
        Ok(switches) => switches,
        Err(err) => fail(&err),
    };
    INTERACTIVE.store(switches.cmdline.is_none(), Ordering::SeqCst);
    let mut need_shell = true;
    let mut progname = None;
    let mut cmdline = switches.cmdline.clone();
    if let Some(line) = switches.cmdline.as_deref() {
        let (path, _) = get_next_token(line);
        if path.is_empty() {
            fail("error: no program name specified.\n");
        }
        progname = make_absolute(&canon_filename(&path));
        let dequoted = if progname.is_some() {
            try_dequote_cmdline(line)
        } else {
            None
        };
        match dequoted {
            Some(dequoted) => {
                cmdline = Some(dequoted);
                need_shell = false;
            }
            None => progname = None,
        }
    }
    loop {
        if need_shell {
            let comspec =
                std::env::var_os("COMSPEC").unwrap_or_else(|| fail("error: COMSPEC is not set\n"));
            let comspec = canon_filename(&lossy(&comspec));
            let shell = match make_absolute(&comspec) {
                Some(shell) if shell.contains('\\') => shell,
                _ => fail(&format!(
                    "error: the program {comspec} could not be found.\n"
                )),
            };
            let dot_com = run_command_dot_com(&shell);
            let command_dir = if dot_com && cmdline.is_none() {
                let short = emacs_w32::short_path_name(OsStr::new(&shell))
                    .map(|short| lossy(&short))
                    .unwrap_or_default();
                short
                    .rfind('\\')
                    .map_or(short.clone(), |i| short[..=i].to_string())
            } else {
                String::new()
            };
            cmdline = Some(shell_command_line(
                &shell,
                &switches.pass_through_args,
                cmdline.as_deref(),
                dot_com,
                switches.envsize,
                &command_dir,
            ));
            progname = Some(shell);
        }
        let Some(prog) = progname.clone() else {
            fail("Internal error: program name not defined\n");
        };
        let line = cmdline.clone().unwrap_or_else(|| prog.clone());
        if let Some(rc) = spawn(Some(&prog), OsStr::new(&line), &dir) {
            return rc as i32;
        }
        if !need_shell {
            need_shell = true;
            continue;
        }
        fail(&format!("Could not run {prog}\n"));
    }
}
