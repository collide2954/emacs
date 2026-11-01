mod common;

use common::{eval, on_big_stack};
use emacs_core::lisp::Lisp;

const STUBS: &str = "(progn (fset 'internal-echo-keystrokes-prefix (function (lambda () nil))) (fset 'internal-timer-start-idle (function (lambda () nil))) nil)";

fn reader(bindings: &str) -> Lisp {
    let mut lisp = Lisp::new();
    assert_eq!(eval(&mut lisp, STUBS), "nil");
    let setup = format!("(progn (setq g (make-sparse-keymap)) (use-global-map g) {bindings} nil)");
    assert_eq!(eval(&mut lisp, &setup), "nil");
    lisp
}

fn check(lisp: &mut Lisp, cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(eval(lisp, source), *expected, "evaluating {source}");
    }
}

#[test]
fn plain_and_prefix_sequences() {
    on_big_stack(|| {
        let mut lisp = reader("(define-key g [97] 'cmd-a) (define-key g [24 102] 'cmd-xf)");
        check(
            &mut lisp,
            &[
                (
                    "(progn (setq unread-command-events '(97)) (read-key-sequence nil))",
                    "\"a\"",
                ),
                (
                    "(progn (setq unread-command-events '(24 102)) (read-key-sequence-vector nil))",
                    "[24 102]",
                ),
                (
                    "(list (this-command-keys-vector) (this-single-command-keys) (this-single-command-raw-keys) last-nonmenu-event)",
                    "([24] [24] [24 102] 102)",
                ),
                (
                    "(progn (setq unread-command-events '(122 97)) (list (read-key-sequence-vector nil) unread-command-events))",
                    "([122] (97))",
                ),
                (
                    "(progn (setq unread-command-events '(24 122)) (read-key-sequence-vector nil))",
                    "[24 122]",
                ),
                (
                    "(progn (setq unread-command-events '((t . 97))) (read-key-sequence-vector nil t))",
                    "[97]",
                ),
                (
                    "(list (this-command-keys-vector) (this-single-command-keys) (this-single-command-raw-keys))",
                    "([24 97] [97] [97])",
                ),
                (
                    "(read-key-sequence 5)",
                    "error: (wrong-type-argument stringp 5)",
                ),
            ],
        );
    });
}

#[test]
fn upper_case_and_shift_fallbacks() {
    on_big_stack(|| {
        let mut lisp = reader(
            "(define-key g [98] 'cmd-b) (define-key g [97] 'cmd-a) (define-key g [f5] 'cmd-f5)",
        );
        check(
            &mut lisp,
            &[
                (
                    "(progn (setq this-command-keys-shift-translated nil unread-command-events '(66)) (list (read-key-sequence-vector nil) this-command-keys-shift-translated (this-single-command-raw-keys) (this-command-keys-vector)))",
                    "([98] t [66] [98])",
                ),
                (
                    "(progn (setq this-command-keys-shift-translated nil unread-command-events '(66)) (list (read-key-sequence-vector nil nil t) this-command-keys-shift-translated))",
                    "([66] nil)",
                ),
                (
                    "(progn (setq this-command-keys-shift-translated nil unread-command-events '(90)) (list (read-key-sequence-vector nil) this-command-keys-shift-translated))",
                    "([90] nil)",
                ),
                (
                    "(progn (setq this-command-keys-shift-translated nil unread-command-events '(33554529)) (list (read-key-sequence-vector nil) this-command-keys-shift-translated))",
                    "([97] t)",
                ),
                (
                    "(progn (setq this-command-keys-shift-translated nil unread-command-events '(S-f5)) (list (read-key-sequence-vector nil) this-command-keys-shift-translated))",
                    "([f5] t)",
                ),
                (
                    "(let ((translate-upper-case-key-bindings nil)) (setq unread-command-events '(66)) (read-key-sequence-vector nil))",
                    "[66]",
                ),
            ],
        );
    });
}

#[test]
fn translation_maps() {
    on_big_stack(|| {
        let mut lisp = reader(
            "(define-key g [up] 'cmd-up) (define-key g [49] 'cmd-1) (define-key g [97] 'cmd-a)",
        );
        check(
            &mut lisp,
            &[
                (
                    "(progn (define-key input-decode-map [113] [up]) (setq unread-command-events '(113)) (list (read-key-sequence-vector nil) (this-single-command-raw-keys)))",
                    "([up] [113])",
                ),
                (
                    "(progn (define-key local-function-key-map [kp-1] [49]) (setq unread-command-events '(kp-1)) (list (read-key-sequence-vector nil) (this-command-keys-vector) (this-single-command-raw-keys)))",
                    "([49] [49] [kp-1])",
                ),
                (
                    "(progn (fset 'remap-w (function (lambda (prompt) (setq seen (list prompt current-key-remap-sequence)) [97]))) (define-key key-translation-map [119] 'remap-w) (setq unread-command-events '(119)) (list (read-key-sequence-vector \"P\") seen))",
                    "([97] (\"P\" [119]))",
                ),
                (
                    "(progn (fset 'bad-remap (function (lambda (prompt) 5))) (define-key key-translation-map [118] 'bad-remap) (setq unread-command-events '(118)) (read-key-sequence-vector nil))",
                    "error: (error \"Function returns invalid key sequence\" bad-remap)",
                ),
            ],
        );
    });
}

#[test]
fn mouse_events_and_switch_frames() {
    on_big_stack(|| {
        let mut lisp = reader(
            "(define-key g [mouse-1] 'cmd-m1) (define-key g [24 102] 'cmd-xf) (define-key g [mode-line mouse-1] 'cmd-ml) (define-key g [menu-bar foo] 'cmd-mb) (setq w (selected-window) p (list w 1 '(0 . 0) 0))",
        );
        check(
            &mut lisp,
            &[
                (
                    "(progn (setq unread-command-events (list (list 'down-mouse-1 p) (list 'mouse-1 p))) (let ((v (read-key-sequence-vector nil))) (list (length v) (car (aref v 0)) (eq (nth 1 (aref v 0)) p) (length (this-single-command-raw-keys)))))",
                    "(1 mouse-1 t 2)",
                ),
                (
                    "(progn (setq unread-command-events (list (list 'drag-mouse-1 p p))) (let ((v (read-key-sequence-vector nil))) (list (length v) (car (aref v 0)) (length (aref v 0)) (eq (nth 1 (aref v 0)) p))))",
                    "(1 mouse-1 2 t)",
                ),
                (
                    "(progn (put 'mouse-1 'event-kind 'mouse-click) (setq unread-command-events (list (list 'mouse-1 (list w 'mode-line '(0 . 0) 0)))) (let ((v (read-key-sequence-vector nil))) (list (length v) (aref v 0) (car (aref v 1)))))",
                    "(2 mode-line mouse-1)",
                ),
                (
                    "(progn (setq unread-command-events (list (list 'foo (list w 'menu-bar '(0 . 0) 0)))) (let ((v (read-key-sequence-vector nil))) (list (length v) (aref v 0) (car (aref v 1)) (nth 1 (nth 1 (aref v 1))))))",
                    "(2 menu-bar foo (menu-bar))",
                ),
                (
                    "(progn (setq unread-command-events (list 24 (list 'switch-frame (selected-frame)) 102)) (list (read-key-sequence-vector nil) (this-single-command-raw-keys) (car (read-event))))",
                    "([24 102] [24 102] switch-frame)",
                ),
                (
                    "(progn (setq unread-command-events (list (list 'switch-frame (selected-frame)) 97)) (let ((v (read-key-sequence-vector nil nil nil t))) (list (length v) (car (aref v 0)) unread-command-events)))",
                    "(1 switch-frame (97))",
                ),
            ],
        );
    });
}
