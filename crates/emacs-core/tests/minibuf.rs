mod common;

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use common::{check, eval, on_big_stack};
use emacs_core::lisp::Lisp;

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Capture {
    fn take(&self) -> String {
        let text = String::from_utf8_lossy(&self.0.borrow()).into_owned();
        self.0.borrow_mut().clear();
        text
    }
}

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

#[test]
fn yes_or_no_p_in_batch() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let out = Capture::default();
        lisp.stdin = Box::new(std::io::Cursor::new(
            b"Yes\nno\nyes\n\nyes\nNO\nyes\n".to_vec(),
        ));
        lisp.stdout = Box::new(out.clone());
        let dialog = "(progn (fset 'x-popup-dialog (function (lambda (pos menu &optional header) (list pos menu header)))) (let ((last-input-event '(mouse-1)) (last-nonmenu-event '(mouse-1))) (yes-or-no-p \"Q\")))";
        let tty_menu = "(progn (setq from--tty-menu-p t) (prog1 (let ((last-input-event 97) (last-nonmenu-event 97)) (yes-or-no-p \"Q\")) (makunbound 'from--tty-menu-p)))";
        for (source, expected, prompt) in [
            (
                "(list (boundp 'yes-or-no-p-history) yes-or-no-p-history)",
                "(t nil)",
                "",
            ),
            ("(yes-or-no-p \"Really?\")", "t", "Really? (yes or no) "),
            ("(yes-or-no-p \"Sure \")", "nil", "Sure  (yes or no) "),
            (
                "(let ((unicode-category-table (make-char-table nil))) (aset unicode-category-table 32 23) (yes-or-no-p \"Sure \"))",
                "t",
                "Sure (yes or no) ",
            ),
            ("(yes-or-no-p \"\")", "t", "(yes or no) (yes or no) "),
            (
                "(let ((yes-or-no-prompt \"[y/n] \")) (yes-or-no-p \"Q:\"))",
                "nil",
                "Q: [y/n] ",
            ),
            (
                "(let ((use-short-answers t)) (fset 'y-or-n-p (function (lambda (p) (list 'short p)))) (yes-or-no-p \"Q? \"))",
                "(short \"Q? \")",
                "",
            ),
            (
                "(yes-or-no-p 'x)",
                "error: (wrong-type-argument stringp x)",
                "",
            ),
            (dialog, "(t (\"Q\" (\"Yes\" . t) (\"No\")) nil)", ""),
            (tty_menu, "(t (\"Q\" (\"Yes\" . t) (\"No\")) nil)", ""),
            (
                "(let ((last-input-event '(mouse-1)) (last-nonmenu-event '(mouse-1)) (use-dialog-box nil)) (yes-or-no-p \"W\"))",
                "t",
                "W (yes or no) ",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
            assert_eq!(out.take(), prompt, "prompt of {source}");
        }
    });
}
