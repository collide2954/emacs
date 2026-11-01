mod common;

use common::{eval, on_big_stack};
use emacs_core::lisp::Lisp;

const STUBS: &str = "(progn (fset 'internal-echo-keystrokes-prefix (function (lambda () nil))) (fset 'internal-timer-start-idle (function (lambda () nil))) (fset 'add-to-history (function (lambda (var elt &rest _) (set var (cons elt (symbol-value var)))))) (fset 'handle-shift-selection (function (lambda () (setq log (cons 'shift log))))) (setq log nil) nil)";

fn caller() -> Lisp {
    let mut lisp = Lisp::new();
    assert_eq!(eval(&mut lisp, STUBS), "nil");
    lisp
}

fn check(lisp: &mut Lisp, cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(eval(lisp, source), *expected, "evaluating {source}");
    }
}

#[test]
fn prefix_point_and_region_codes() {
    on_big_stack(|| {
        let mut lisp = caller();
        check(
            &mut lisp,
            &[
                (
                    "(progn (fset 'f-p (function (lambda (n raw) (interactive \"p\\nP\") (list n raw)))) (list (call-interactively 'f-p) (let ((current-prefix-arg '(4))) (call-interactively 'f-p)) (let ((current-prefix-arg '-)) (call-interactively 'f-p))))",
                    "((1 nil) (4 (4)) (-1 -))",
                ),
                (
                    "(progn (fset 'f-r (function (lambda (b e) (interactive \"r\") (list b e)))) (erase-buffer) (insert \"abcdef\") (goto-char 5) (call-interactively 'f-r))",
                    "error: (error \"The mark is not set now, so there is no region\")",
                ),
                (
                    "(progn (set-marker (mark-marker) 3) (list (call-interactively 'f-r) (call-interactively (function (lambda (p m) (interactive \"d\\nm\") (list p m))))))",
                    "((3 5) (5 3))",
                ),
                (
                    "(let ((transient-mark-mode t) (mark-even-if-inactive nil)) (put 'mark-inactive 'error-conditions '(mark-inactive error)) (call-interactively 'f-r))",
                    "error: (mark-inactive)",
                ),
                (
                    "(progn (setq command-history nil) (call-interactively 'f-r t) command-history)",
                    "((f-r (region-beginning) (region-end)))",
                ),
                (
                    "(call-interactively (function (lambda (a b) (interactive \"i\\n^p\") (list a b log))))",
                    "error: (error \"Invalid control letter ‘^’ (#o136, #x005e) in interactive calling string\")",
                ),
                (
                    "(call-interactively (function (lambda (a) (interactive \"^-p\") (list a log))))",
                    "(1 (shift))",
                ),
            ],
        );
    });
}

#[test]
fn spec_errors_and_lisp_specs() {
    on_big_stack(|| {
        let mut lisp = caller();
        check(
            &mut lisp,
            &[
                (
                    "(progn (fset 'plain (function (lambda () 1))) (call-interactively 'plain))",
                    "error: (wrong-type-argument commandp plain)",
                ),
                (
                    "(list (interactive-form 'self-insert-command) (interactive-form (function (lambda () \"doc\" (interactive \"p\" text-mode) 1))) (interactive-form 'car) (interactive-form 'no-such-function))",
                    "((interactive (list (prefix-numeric-value current-prefix-arg) last-command-event)) (interactive \"p\") nil nil)",
                ),
                (
                    "(progn (erase-buffer) (let ((last-command-event 120) (current-prefix-arg 3)) (call-interactively 'self-insert-command)) (buffer-string))",
                    "\"xxx\"",
                ),
                (
                    "(call-interactively (function (lambda (x) (interactive \"q\") x)))",
                    "error: (error \"Invalid control letter ‘q’ (#o161, #x0071) in interactive calling string\")",
                ),
                (
                    "(call-interactively (function (lambda () (interactive \"+\") 1)))",
                    "error: (error \"‘+’ is not used in ‘interactive’ for ordinary commands\")",
                ),
                (
                    "(progn (fset 'f-e (function (lambda (e) (interactive \"e\") e))) (call-interactively 'f-e))",
                    "error: (error \"f-e must be bound to an event with parameters\")",
                ),
                (
                    "(progn (fset 'f-e (function (lambda (e) (interactive \"e\") e))) (call-interactively 'f-e nil [97 (mouse-1 x) 98]))",
                    "(mouse-1 x)",
                ),
                (
                    "(progn (setq buffer-read-only t) (prog1 (call-interactively (function (lambda () (interactive \"*\") 1))) (setq buffer-read-only nil)))",
                    "error: (buffer-read-only #<buffer *scratch*>)",
                ),
                (
                    "(progn (setq buffer-read-only nil) (fset 'h (function (lambda (x) (interactive (progn (setq this-command 'clobbered) (list this-command))) (list x this-command)))) (setq this-command 'outer command-history nil) (list (call-interactively 'h) command-history (call-interactively 'h t) command-history))",
                    "((clobbered outer) nil (clobbered outer) ((h 'clobbered)))",
                ),
            ],
        );
    });
}

#[test]
fn reading_characters_and_keys() {
    on_big_stack(|| {
        let mut lisp = caller();
        check(
            &mut lisp,
            &[
                (
                    "(progn (fset 'f-c (function (lambda (c) (interactive \"cChar: \") c))) (setq command-history nil unread-command-events '(120)) (list (call-interactively 'f-c) command-history))",
                    "(120 ((f-c 120)))",
                ),
                (
                    "(progn (setq unread-command-events '(left)) (call-interactively 'f-c))",
                    "error: (error \"Non-character input-event\")",
                ),
                (
                    "(progn (setq unread-command-events nil) (use-global-map (make-sparse-keymap)) (fset 'f-k (function (lambda (k v) (interactive \"kKey: \\nKKey: \") (list k v)))) (setq unread-command-events '(97 98)) (call-interactively 'f-k))",
                    "(\"a\" [98])",
                ),
            ],
        );
    });
}
