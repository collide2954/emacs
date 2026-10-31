use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("emacs-top-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch directory");
    dir
}

fn stderr_text(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|line| !line.starts_with("Loading "))
        .map(|line| format!("{line}\n"))
        .collect()
}

fn run(dir: &Path, source: &str) -> Output {
    let script = dir.join("script.el");
    std::fs::write(&script, source).expect("write script");
    Command::new(env!("CARGO_BIN_EXE_emacs"))
        .args(["--batch", "-nl", "-l"])
        .arg(script.to_string_lossy().replace('\\', "/"))
        .output()
        .expect("run emacs")
}

const FOO: &str = "(defalias 'foo (function (lambda () (signal 'error '(\"Boo\")))))\n";

#[test]
fn unhandled_error_in_batch() {
    let dir = scratch("unhandled");
    let out = run(
        &dir,
        &format!(
            "(defalias 'debug-early--handler (function (lambda (err) (message \"handler %S\" err))))\n{FOO}(foo)\n"
        ),
    );
    assert_eq!(stderr_text(&out), "handler (error \"Boo\")\nBoo\n");
    assert!(out.stdout.is_empty());
    assert_eq!(
        out.status.code(),
        Some(if cfg!(windows) { -1 } else { 255 })
    );
}

#[test]
fn command_error_function() {
    let dir = scratch("cef");
    let out = run(
        &dir,
        &format!(
            "(defalias 'debug-early--handler (function (lambda (_err) nil)))\n\
             (setq kill-emacs-hook (list (function (lambda () (princ \"bye\")))))\n\
             (setq command-error-function (function (lambda (data context signal) (message \"cef %S %S %S\" data context signal))))\n\
             {FOO}(foo)\n"
        ),
    );
    let stderr = stderr_text(&out);
    assert_eq!(stderr, "cef (error \"Boo\") \"\" foo\n");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "bye");
    assert_eq!(out.status.code(), Some(0), "{stderr}");
}

#[test]
fn end_of_top_level_kills_emacs() {
    let dir = scratch("end");
    let out = run(
        &dir,
        "(setq kill-emacs-hook (list (function (lambda () (princ \"bye\")))))\n",
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "bye");
    assert_eq!(out.status.code(), Some(0));
}
