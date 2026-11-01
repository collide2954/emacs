mod common;

use common::{eval, on_big_stack};
use emacs_core::lisp::Lisp;

const STUBS: &str = "(progn (fset 'internal-echo-keystrokes-prefix (function (lambda () nil))) (fset 'internal-timer-start-idle (function (lambda () nil))) (fset 'undo-auto--add-boundary (function (lambda () nil))) (fset 'command-execute (function (lambda (cmd &rest _) (funcall cmd)))) (fset 'undefined (function (lambda () (setq log (cons 'undefined log))))) (fset 'cmd-exit (function (lambda () (throw 'exit 'done)))) (setq log nil) nil)";

fn looper(bindings: &str) -> Lisp {
    let mut lisp = Lisp::new();
    assert_eq!(eval(&mut lisp, STUBS), "nil");
    let setup = format!(
        "(progn (setq g (make-sparse-keymap)) (use-global-map g) (define-key g [120] 'cmd-exit) {bindings} nil)"
    );
    assert_eq!(eval(&mut lisp, &setup), "nil");
    lisp
}

fn check(lisp: &mut Lisp, cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(eval(lisp, source), *expected, "evaluating {source}");
    }
}

#[test]
fn commands_run_until_exit() {
    on_big_stack(|| {
        let mut lisp = looper(
            "(fset 'cmd-a (function (lambda () (setq log (cons (list 'a (recursion-depth) last-command this-command last-command-event) log))))) (define-key g [97] 'cmd-a)",
        );
        check(
            &mut lisp,
            &[
                (
                    "(progn (setq unread-command-events '(97 97 98 120)) (list (recursive-edit) (recursion-depth) (nreverse log)))",
                    "(nil 0 ((a 1 nil cmd-a 97) (a 1 cmd-a cmd-a 97) undefined))",
                ),
                (
                    "(list (recent-keys t) (recent-keys) num-input-keys)",
                    "([97 (nil . cmd-a) 97 (nil . cmd-a) 98 (nil) 120 (nil . cmd-exit)] [97 97 98 120] 4)",
                ),
                (
                    "(exit-recursive-edit)",
                    "error: (user-error \"No recursive edit is in progress\")",
                ),
                (
                    "(abort-recursive-edit)",
                    "error: (user-error \"No recursive edit is in progress\")",
                ),
            ],
        );
    });
}

#[test]
fn exit_values() {
    on_big_stack(|| {
        let mut lisp = looper(
            "(fset 'cmd-t (function (lambda () (throw 'exit t)))) (define-key g [116] 'cmd-t) (fset 'cmd-s (function (lambda () (throw 'exit \"boom\")))) (define-key g [115] 'cmd-s) (fset 'cmd-f (function (lambda () (throw 'exit (function (lambda () (setq log (cons 'called log)))))))) (define-key g [102] 'cmd-f) (fset 'cmd-q (function (lambda () (abort-recursive-edit)))) (define-key g [113] 'cmd-q) (fset 'cmd-e (function (lambda () (exit-recursive-edit)))) (define-key g [101] 'cmd-e)",
        );
        check(
            &mut lisp,
            &[
                (
                    "(progn (setq unread-command-events '(116)) (recursive-edit))",
                    "error: (quit)",
                ),
                (
                    "(progn (setq unread-command-events '(115)) (recursive-edit))",
                    "error: (error \"boom\")",
                ),
                (
                    "(progn (setq unread-command-events '(102)) (list (recursive-edit) log))",
                    "(nil (called))",
                ),
                (
                    "(progn (setq unread-command-events '(113)) (recursive-edit))",
                    "error: (quit)",
                ),
                (
                    "(progn (setq unread-command-events '(101)) (list (recursive-edit) (recursion-depth)))",
                    "(nil 0)",
                ),
            ],
        );
    });
}

#[test]
fn hooks_errors_and_remapping() {
    on_big_stack(|| {
        let mut lisp = looper(
            "(fset 'cmd-b (function (lambda () (setq log (cons (list 'b this-command this-original-command (this-command-keys-vector)) log))))) (define-key g [97] 'cmd-a) (define-key g [remap cmd-a] 'cmd-b) (fset 'cmd-err (function (lambda () (car 1)))) (define-key g [101] 'cmd-err) (fset 'cmd-top (function (lambda () (top-level)))) (define-key g [116] 'cmd-top) (setq command-error-function (function (lambda (data context signal) (setq log (cons (list 'err data context) log))))) (setq pre-command-hook (list (function (lambda () (setq log (cons (list 'pre this-command) log)))))) (setq post-command-hook (list (function (lambda () (setq log (cons (list 'post this-command) log))))))",
        );
        check(
            &mut lisp,
            &[
                (
                    "(progn (setq unread-command-events '(97 101 116 120)) (list (catch 'top-level (recursive-edit)) (nreverse log) (recursion-depth) unread-command-events))",
                    "(nil ((post nil) (pre cmd-b) (b cmd-b cmd-a [97]) (post cmd-b) (pre cmd-err) (err (wrong-type-argument listp 1) \"\") (post cmd-err) (pre cmd-top)) 0 (120))",
                ),
                (
                    "(progn (setq log nil) (fset 'bad-hook (function (lambda () (car 1)))) (setq pre-command-hook (list 'bad-hook) post-command-hook nil unread-command-events '(120)) (list (recursive-edit) pre-command-hook))",
                    "(nil nil)",
                ),
            ],
        );
    });
}

#[test]
fn point_adjustment_after_commands() {
    on_big_stack(|| {
        let mut lisp = looper(
            "(fset 'cmd-go (function (lambda () (setq log (cons (point) log)) (goto-char 3)))) (define-key g [103] 'cmd-go) (fset 'cmd-stay (function (lambda () (setq log (cons (point) log)) (goto-char 3) (setq disable-point-adjustment t)))) (define-key g [104] 'cmd-stay)",
        );
        check(
            &mut lisp,
            &[
                (
                    "(progn (set-buffer (window-buffer (selected-window))) (erase-buffer) (insert \"abcdef\") (put-text-property 2 5 'invisible t) (goto-char 1) nil)",
                    "nil",
                ),
                (
                    "(progn (setq unread-command-events '(103 103 104 120)) (list (recursive-edit) (nreverse log) (point)))",
                    "(nil (1 2 6) 3)",
                ),
            ],
        );
    });
}
