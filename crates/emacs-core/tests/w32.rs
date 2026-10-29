#![cfg(windows)]

mod common;

use common::{check, eval, on_big_stack};
use emacs_core::lisp::Lisp;

#[test]
fn long_file_names() {
    let dir = std::env::temp_dir().join(format!("emacs-w32-long-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create directory");
    std::fs::write(dir.join("MixedCase.Txt"), "x").expect("create file");
    let lower = dir.to_string_lossy().replace('\\', "/").to_lowercase();
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        let found = eval(
            l,
            &format!("(w32-long-file-name \"{lower}/mixedcase.txt\")"),
        );
        assert!(found.ends_with("/MixedCase.Txt\""), "{found}");
        assert_eq!(found.to_lowercase(), format!("\"{lower}/mixedcase.txt\""));
        assert!(found.starts_with("\"c:/"), "{found}");
        let found_dir = eval(l, &format!("(w32-long-file-name \"{lower}/\")"));
        assert!(found_dir.ends_with("/\""), "{found_dir}");
        assert_eq!(found_dir.to_lowercase(), format!("\"{lower}/\""));
        for (source, expected) in [
            (
                format!("(w32-long-file-name \"{lower}/no-such-file\")"),
                "nil",
            ),
            (format!("(w32-long-file-name \"{lower}/mixed*\")"), "nil"),
            ("(w32-long-file-name \"C:/\")".to_string(), "\"c:/\""),
            (
                "(w32-long-file-name 1)".to_string(),
                "error: (wrong-type-argument stringp 1)",
            ),
        ] {
            assert_eq!(eval(l, &source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn init_environment() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        for (source, expected) in [
            ("(file-directory-p (getenv-internal \"TMPDIR\"))", "t"),
            ("(stringp (getenv-internal \"LANG\"))", "t"),
            ("(stringp (getenv-internal \"HOME\"))", "t"),
            ("(stringp (getenv-internal \"SHELL\"))", "t"),
            ("(stringp (getenv-internal \"TERM\"))", "t"),
            ("(stringp (getenv-internal \"emacs_dir\"))", "t"),
            (
                "(let ((process-environment '(\"LANG\"))) (getenv-internal \"LANG\"))",
                "nil",
            ),
            ("(getenv-internal \"LANG\" '(\"LANG=x\"))", "\"x\""),
            ("(getenv-internal \"lang\" '(\"LANG=x\"))", "\"x\""),
            ("(getenv-internal \"LANG\" '(\"LANG\"))", "t"),
            ("(getenv-internal \"LANG\" '(\"PATH=x\"))", "nil"),
            (
                "(let ((process-environment nil)) (stringp (getenv-internal \"TMPDIR\")))",
                "t",
            ),
        ] {
            assert_eq!(eval(l, source), expected, "evaluating {source}");
        }
        let lang = std::env::var("LANG").expect("LANG is set");
        assert_eq!(eval(l, "(getenv-internal \"LANG\")"), format!("{lang:?}"));
        for list in ["process-environment", "initial-environment"] {
            let source = format!(
                "(let ((case-fold-search nil) (l {list}) (seen nil)) (while l (if (string-match \"\\\\`[Pp][Aa][Tt][Hh]=\\\\|\\\\`[Cc][Oo][Mm][Ss][Pp][Ee][Cc]=\" (car l)) (setq seen (cons (substring (car l) 0 (match-end 0)) seen))) (setq l (cdr l))) (sort seen (function string-lessp)))"
            );
            assert_eq!(eval(l, &source), "(\"COMSPEC=\" \"PATH=\")", "{list}");
        }
    });
}

#[test]
fn code_pages() {
    let ansi = emacs_w32::ansi_code_page().to_string();
    let multibyte = emacs_w32::multibyte_code_page().to_string();
    let console = emacs_w32::console_cp().to_string();
    let output = emacs_w32::console_output_cp().to_string();
    check(&[
        ("w32-ansi-code-page", &ansi),
        ("w32-multibyte-code-page", &multibyte),
        ("(w32-get-console-codepage)", &console),
        ("(w32-get-console-output-codepage)", &output),
        ("(w32-set-console-codepage 0)", "nil"),
        ("(w32-set-console-output-codepage 0)", "nil"),
        (
            "(w32-set-console-codepage 'x)",
            "error: (wrong-type-argument fixnump x)",
        ),
        (
            "(w32-set-console-output-codepage 1.5)",
            "error: (wrong-type-argument fixnump 1.5)",
        ),
    ]);
}
