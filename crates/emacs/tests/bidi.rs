use std::path::PathBuf;
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("emacs-bidi-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch directory");
    dir
}

fn run_loaded(name: &str, source: &str) -> String {
    let dir = scratch(name);
    let script = dir.join("script.el");
    std::fs::write(&script, source).expect("write script");
    let out = Command::new(env!("CARGO_BIN_EXE_emacs"))
        .args(["--batch", "-Q", "-l"])
        .arg(script.to_string_lossy().replace('\\', "/"))
        .output()
        .expect("run emacs");
    assert!(
        out.status.success(),
        "emacs failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn paragraph_direction_and_overrides() {
    let out = run_loaded(
        "direction",
        ";;; -*- coding: utf-8; lexical-binding: t -*-\n\
         (defun show (x) (princ (format \"%S\\n\" x)))\n\
         (show redisplay--inhibit-bidi)\n\
         (with-temp-buffer (show (current-bidi-paragraph-direction)))\n\
         (with-temp-buffer (insert \"hello\") (show (current-bidi-paragraph-direction)))\n\
         (with-temp-buffer (insert \"\u{5e9}\u{5dc}\u{5d5}\u{5dd}\") (show (current-bidi-paragraph-direction)))\n\
         (with-temp-buffer (insert \"123 \u{5e9}\u{5dc}\u{5d5}\u{5dd}\") (show (current-bidi-paragraph-direction)))\n\
         (with-temp-buffer (insert \"abc\\n\\n\u{5e9}\u{5dc}\u{5d5}\u{5dd}\")\n\
           (show (current-bidi-paragraph-direction))\n\
           (goto-char (point-min))\n\
           (show (current-bidi-paragraph-direction)))\n\
         (with-temp-buffer (insert \"\u{5e9}\u{5dc}\u{5d5}\u{5dd}\\n\\n123\") (show (current-bidi-paragraph-direction)))\n\
         (with-temp-buffer (setq bidi-paragraph-direction 'right-to-left) (show (current-bidi-paragraph-direction)))\n\
         (show (bidi-find-overridden-directionality 0 nil (concat \"ab\" (string #x202e) \"cd\")))\n\
         (show (bidi-find-overridden-directionality 0 nil \"abcd\"))\n\
         (with-temp-buffer (insert \"xy\" #x202e \"zw\")\n\
           (show (bidi-find-overridden-directionality (point-min) (point-max) nil)))\n",
    );
    assert_eq!(
        out,
        "nil\nleft-to-right\nleft-to-right\nright-to-left\nright-to-left\nright-to-left\nleft-to-right\nright-to-left\nright-to-left\n3\nnil\n4\n"
    );
}
