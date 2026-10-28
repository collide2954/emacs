mod common;

use common::{check, eval, on_big_stack};
use emacs_core::lisp::Lisp;

#[test]
fn minibuffer_state() {
    check(&[
        ("(active-minibuffer-window)", "nil"),
        ("(minibuffer-innermost-command-loop-p)", "nil"),
        (
            "(abort-minibuffers)",
            "error: (error \"Not in a minibuffer\")",
        ),
        (
            "(set-minibuffer-window (selected-window))",
            "error: (error \"Window is not a minibuffer window\")",
        ),
        (
            "(eq (set-minibuffer-window (minibuffer-window)) (minibuffer-window))",
            "t",
        ),
        (
            "(set-minibuffer-window 'x)",
            "error: (wrong-type-argument windowp x)",
        ),
        (
            "(save-current-buffer (set-buffer (get-buffer-create \"mb\")) (insert \"abc\") (list (minibuffer-prompt-end) (minibuffer-contents)))",
            "(1 \"abc\")",
        ),
        (
            "(save-current-buffer (set-buffer (get-buffer \" *Minibuf-0*\")) (erase-buffer) (insert (propertize \"Prompt: \" 'field t 'front-sticky t 'rear-nonsticky t) \"input\") (list (minibuffer-prompt-end) (minibuffer-contents-no-properties)))",
            "(9 \"input\")",
        ),
    ]);
}

#[test]
fn batch_reading() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        lisp.stdin = Box::new(std::io::Cursor::new(b"hello\r(1 2)\n\nfoo\n1 2\n".to_vec()));
        lisp.stdout = Box::new(std::io::sink());
        for (source, expected) in [
            ("(read-string \"Name: \")", "\"hello\""),
            ("(read-from-minibuffer \"Form: \" nil nil t)", "(1 2)"),
            ("(read-string \"Default: \" nil nil \"dflt\")", "\"dflt\""),
            ("(read-from-minibuffer \"Sym: \" nil nil t)", "foo"),
            (
                "(read-from-minibuffer \"Junk: \" nil nil t)",
                "error: (invalid-read-syntax \"Trailing garbage following expression\")",
            ),
            (
                "(read-string \"End: \")",
                "error: (end-of-file \"Error reading from stdin\")",
            ),
            (
                "(let ((inhibit-interaction t)) (read-string \"x\"))",
                "error: (inhibited-interaction)",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}
