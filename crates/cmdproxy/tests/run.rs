#![cfg(windows)]

use std::ffi::OsStr;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::process::{Command, Output, Stdio};

const CMDPROXY: &str = env!("CARGO_BIN_EXE_cmdproxy");

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn proxy() -> Command {
    let mut command = Command::new(CMDPROXY);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

fn run(args: &[&str], configure: impl FnOnce(&mut Command)) -> Output {
    let mut command = proxy();
    command.args(args).stdin(Stdio::null());
    configure(&mut command);
    command.output().expect("cmdproxy runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn no_comspec(command: &mut Command) {
    command.env_remove("COMSPEC");
}

#[test]
fn shell_commands() {
    let out = run(&["-c", "echo hi& echo there"], |_| {});
    assert_eq!(text(&out.stdout), "hi\r\nthere\r\n");
    assert_eq!(text(&out.stderr), "");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(run(&["-c", "exit 3"], |_| {}).status.code(), Some(3));
    assert_eq!(run(&["/C", "exit 4"], |_| {}).status.code(), Some(4));
    let out = run(&["/q", "-c", "echo %CMDPROXY_TEST%"], |c| {
        c.env("CMDPROXY_TEST", "expanded");
    });
    assert_eq!(text(&out.stdout), "expanded\r\n");
}

#[test]
fn failures() {
    let out = run(&["-c"], |_| {});
    assert_eq!(text(&out.stderr), "error: expecting arg for -c\n");
    assert_eq!(out.status.code(), Some(-1));
    let out = run(&["-c", "exit 2", "extra"], |_| {});
    assert_eq!(
        text(&out.stderr),
        "warning: extra args ignored after 'exit 2'\n"
    );
    assert_eq!(out.status.code(), Some(2));
    let out = run(&["-c", "exit 2"], no_comspec);
    assert_eq!(text(&out.stderr), "error: COMSPEC is not set\n");
    assert_eq!(out.status.code(), Some(-1));
    let out = run(&["-c", "exit 2"], |c| {
        c.env("COMSPEC", "C:/no/such/shell.exe");
    });
    assert_eq!(
        text(&out.stderr),
        "error: the program C:\\no\\such\\shell.exe could not be found.\n"
    );
    assert_eq!(out.status.code(), Some(-1));
    let out = run(&["-c", "\"\" x"], |_| {});
    assert_eq!(text(&out.stderr), "error: no program name specified.\n");
    assert_eq!(out.status.code(), Some(-1));
    let long = format!("exit 0 & rem {}", "y".repeat(2000));
    let out = run(&["-c", &long, "extra"], |_| {});
    assert_eq!(out.stderr.len(), 1023);
    assert!(text(&out.stderr).starts_with("warning: extra args ignored after 'exit 0 & rem yyy"));
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn direct_programs() {
    assert_eq!(
        run(&["-c", "cmd /c exit 5"], no_comspec).status.code(),
        Some(5)
    );
    assert_eq!(
        run(&["-c", "\"cmd\" /c exit 2"], no_comspec).status.code(),
        Some(2)
    );
    let root = std::env::var("SystemRoot").expect("SystemRoot");
    let full = format!("{root}\\System32\\cmd /c exit 6");
    assert_eq!(run(&["-c", &full], no_comspec).status.code(), Some(6));
    assert_eq!(
        run(&["-c", "cmd /c exit 5 ^& rem"], no_comspec)
            .status
            .code(),
        Some(5)
    );
    let out = run(&["-c", "cmd /c exit 5 & rem"], no_comspec);
    assert_eq!(text(&out.stderr), "error: COMSPEC is not set\n");
    assert_eq!(
        run(&["-c", "cmd /c exit 5 & exit 7"], |_| {}).status.code(),
        Some(7)
    );
}

#[test]
fn callers_directory() {
    let dir = std::env::temp_dir().join(format!("cmdproxy-dir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create directory");
    for line in ["cmd /c cd", "cd"] {
        let out = run(&["-c", line], |c| {
            c.current_dir(&dir);
        });
        assert_eq!(
            text(&out.stdout).trim_end().to_lowercase(),
            dir.to_string_lossy().to_lowercase(),
            "{line}"
        );
    }
}

#[test]
fn interactive_shell() {
    let mut child = proxy()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cmdproxy starts");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"exit 6\r\n")
        .expect("write");
    let out = child.wait_with_output().expect("cmdproxy exits");
    assert_eq!(out.status.code(), Some(6));
    let mut child = proxy()
        .env_remove("COMSPEC")
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cmdproxy starts");
    let mut err = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().expect("stderr"), &mut err)
        .expect("read");
    assert_eq!(err, "error: COMSPEC is not set\n");
    assert_eq!(child.wait().expect("exit").code(), Some(-1));
}

#[test]
fn helper_for_other_programs() {
    let codepages = (emacs_w32::console_cp(), emacs_w32::console_output_cp());
    let dir = std::env::current_dir().expect("current dir");
    let proxy = OsStr::new(CMDPROXY);
    for (line, code) in [
        ("cmd /c exit 5", 5),
        ("cmdproxy -c \"exit 8\"", 8),
        ("CMDPROXY.EXE -c \"exit 10\"", 10),
    ] {
        let child = emacs_w32::create_process(Some(proxy), OsStr::new(line), dir.as_os_str())
            .expect("cmdproxy starts");
        assert_eq!(child.wait(), code, "{line}");
    }
    let line = format!("\"{CMDPROXY}\" -c \"exit 9\"");
    let child = emacs_w32::create_process(Some(proxy), OsStr::new(&line), dir.as_os_str())
        .expect("cmdproxy starts");
    let rc = child.wait();
    if codepages.0 != 0 {
        emacs_w32::set_console_cp(codepages.0);
        emacs_w32::set_console_output_cp(codepages.1);
    }
    assert_eq!(rc, 9);
}
