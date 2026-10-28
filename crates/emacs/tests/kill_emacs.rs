use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("emacs-kill-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch directory");
    dir
}

fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn lisp_string(path: &Path) -> String {
    format!("{:?}", slashed(path))
}

fn run(dir: &Path, source: &str) -> Output {
    let script = dir.join("script.el");
    std::fs::write(&script, source).expect("write script");
    Command::new(env!("CARGO_BIN_EXE_emacs"))
        .args(["--batch", "-nl", "-l"])
        .arg(slashed(&script))
        .output()
        .expect("run emacs")
}

#[test]
fn exit_codes() {
    let dir = scratch("codes");
    for (form, code) in [
        ("(kill-emacs 7)", 7),
        ("(kill-emacs)", 0),
        ("(kill-emacs t)", 0),
        ("(kill-emacs \"stuff\")", 0),
        ("(kill-emacs #x100000003)", 3),
        ("(kill-emacs (1+ most-positive-fixnum))", 0),
        (
            "(progn (setq quit-flag 'kill-emacs) (sleep-for 0.01) (kill-emacs 9))",
            0,
        ),
        (
            "(let ((inhibit-quit t)) (setq quit-flag 'kill-emacs) (sleep-for 0.01) (kill-emacs 9))",
            9,
        ),
    ] {
        let out = run(&dir, form);
        assert_eq!(
            out.status.code(),
            Some(code),
            "{form}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.is_empty(), "{form}");
    }
    let out = run(&dir, "(kill-emacs -1)");
    assert_eq!(
        out.status.code(),
        Some(if cfg!(windows) { -1 } else { 255 })
    );
}

#[test]
fn kill_emacs_hook() {
    let dir = scratch("hook");
    let out = run(
        &dir,
        "(defalias 'boom (function (lambda () (signal 'error '(\"boom\")))))\n\
         (defalias 'report (function (lambda () (princ \"ran\"))))\n\
         (setq kill-emacs-hook '(boom report))\n\
         (kill-emacs 5)\n",
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(5), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ran");
    assert!(
        stderr.contains("Error in kill-emacs-hook (boom): (error \"boom\")\n"),
        "{stderr}"
    );
}

#[test]
fn auto_save_at_exit() {
    let dir = scratch("autosave");
    let saved = dir.join("#notes#");
    let list = dir.join("saves");
    let out = run(
        &dir,
        &format!(
            "(set-buffer (get-buffer-create \"notes\"))\n\
             (insert \"hello\")\n\
             (setq buffer-auto-save-file-name {})\n\
             (setq auto-save-list-file-name {})\n\
             (kill-emacs 0)\n",
            lisp_string(&saved),
            lisp_string(&list)
        ),
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&saved).expect("auto-save file"),
        "hello"
    );
    assert!(!list.exists());
}

#[test]
fn restart() {
    let dir = scratch("restart");
    let mark = dir.join("mark");
    let out = run(
        &dir,
        &format!(
            "(if (file-exists-p {m}) (progn (princ \"second\") (kill-emacs 4)) (write-region \"\" nil {m}) (kill-emacs 0 t))\n",
            m = lisp_string(&mark)
        ),
    );
    assert!(mark.exists());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "second",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(if cfg!(windows) { 0 } else { 4 }));
}
