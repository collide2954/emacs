mod common;

use common::{check, eval, on_big_stack};
use emacs_core::lisp::Lisp;

#[test]
fn docstring_objects() {
    check(&[
        ("(documentation-stringp \"x\")", "t"),
        ("(documentation-stringp 5)", "t"),
        ("(documentation-stringp '(\"f\" . 3))", "t"),
        ("(documentation-stringp '(3 . 3))", "nil"),
        ("(documentation-stringp nil)", "nil"),
        ("(internal-subr-documentation (symbol-function 'car))", "0"),
        ("(internal-subr-documentation 'car)", "t"),
        (
            "(progn (put 'xd-v 'variable-documentation \"Doc.\") (documentation-property 'xd-v 'variable-documentation t))",
            "\"Doc.\"",
        ),
        (
            "(progn (put 'xd-v 'xd-prop '(concat \"a\" \"b\")) (documentation-property 'xd-v 'xd-prop t))",
            "\"ab\"",
        ),
        (
            "(progn (put 'xd-f 'function-documentation \"Hi.\") (documentation 'xd-f t))",
            "\"Hi.\"",
        ),
        (
            "(documentation 'xd-none t)",
            "error: (void-function xd-none)",
        ),
    ]);
}

#[test]
fn text_quoting() {
    check(&[
        ("(text-quoting-style)", "curve"),
        (
            "(let ((text-quoting-style 'grave)) (text-quoting-style))",
            "grave",
        ),
        (
            "(let ((text-quoting-style 'straight)) (text-quoting-style))",
            "straight",
        ),
        (
            "(let ((text-quoting-style 'zork)) (text-quoting-style))",
            "curve",
        ),
        (
            "(let ((internal--text-quoting-flag nil)) (text-quoting-style))",
            "grave",
        ),
        (
            "(let ((internal--text-quoting-flag nil)) (format-message \"`x'\"))",
            "\"`x'\"",
        ),
        ("(format-message \"`x'\")", "\"‘x’\""),
    ]);
}

#[test]
fn snarf_and_fetch() {
    on_big_stack(|| {
        let dir = std::env::temp_dir().join("emacs-doc-test");
        std::fs::create_dir_all(&dir).expect("create doc dir");
        let doc = b"\x1fSfoo.c\n\x1fVxd-var\nVariable doc.\n\x1fFcar\nReturn the car.\n\x1fVxd-skip\nSKIP\n\x1fVxd-esc\nA\x01_B\x01\x01C\n";
        std::fs::write(dir.join("DOC-TEST"), doc).expect("write DOC");
        let dir_name = dir.to_string_lossy().replace('\\', "/");
        let mut lisp = Lisp::new();
        let setup = format!(
            "(progn (setq doc-directory \"{dir_name}/\") (defvar xd-var 1) (defvar xd-skip 2) (defvar xd-esc 3) (Snarf-documentation \"DOC-TEST\"))"
        );
        assert_eq!(eval(&mut lisp, &setup), "nil");
        let cases = [
            ("internal-doc-file-name", "\"DOC-TEST\""),
            (
                "(documentation-property 'xd-var 'variable-documentation t)",
                "\"Variable doc.\n\"",
            ),
            ("(get 'xd-skip 'variable-documentation)", "nil"),
            (
                "(documentation-property 'xd-esc 'variable-documentation t)",
                "\"A\u{1f}B\u{1}C\n\"",
            ),
            (
                "(> (internal-subr-documentation (symbol-function 'car)) 0)",
                "t",
            ),
            (
                "(condition-case e (Snarf-documentation \"NO-SUCH-DOC\") (file-missing 'missing))",
                "missing",
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}
