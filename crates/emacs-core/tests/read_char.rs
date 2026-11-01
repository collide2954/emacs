mod common;

use common::{eval, on_big_stack};
use emacs_core::keyboard::CTRL_MODIFIER;
use emacs_core::lisp::Lisp;
use emacs_core::termhooks::{EventKind, KbdEvent};

const STUBS: &str = "(progn (fset 'internal-echo-keystrokes-prefix (function (lambda () nil))) (fset 'internal-timer-start-idle (function (lambda () nil))) nil)";

fn reader() -> Lisp {
    let mut lisp = Lisp::new();
    assert_eq!(eval(&mut lisp, STUBS), "nil");
    lisp
}

fn store(lisp: &mut Lisp, code: u32, modifiers: i64) {
    let event = KbdEvent {
        kind: EventKind::AsciiKeystroke,
        code,
        modifiers,
        frame_or_window: lisp.frames.selected_frame,
        ..KbdEvent::default()
    };
    lisp.kbd_buffer_store_event(event).unwrap();
}

fn check(lisp: &mut Lisp, cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(eval(lisp, source), *expected, "evaluating {source}");
    }
}

#[test]
fn reading_unread_and_queued_events() {
    on_big_stack(|| {
        let mut lisp = reader();
        check(
            &mut lisp,
            &[
                (
                    "(progn (setq unread-command-events '(97)) (list (read-event) (this-command-keys-vector) (recent-keys) last-input-event num-input-keys))",
                    "(97 [97] [97] 97 1)",
                ),
                (
                    "(progn (setq unread-command-events '((t . 98))) (list (read-event) (this-command-keys-vector)))",
                    "(98 [97 98])",
                ),
                (
                    "(progn (setq unread-command-events '(left)) (read-char))",
                    "error: (error \"Non-character input-event\")",
                ),
                ("unread-command-events", "(left)"),
                ("(read-event)", "left"),
                (
                    "(progn (setq unread-command-events '(left 120)) (read-char-exclusive))",
                    "120",
                ),
                (
                    "(progn (put 'kp-enter 'ascii-character 13) (internal-event-symbol-parse-modifiers 'M-kp-enter) (setq unread-command-events '(M-kp-enter)) (read-char))",
                    "134217741",
                ),
                ("(read-char-exclusive nil nil 0.01)", "nil"),
                ("(read-event nil nil 0)", "nil"),
            ],
        );
        store(&mut lisp, u32::from(b'x'), 0);
        store(&mut lisp, u32::from(b'a'), CTRL_MODIFIER);
        check(
            &mut lisp,
            &[
                ("(read-char)", "120"),
                ("(read-event)", "1"),
                (
                    "(recent-keys)",
                    "[97 98 left left left 120 M-kp-enter 120 1]",
                ),
                ("(this-command-keys-vector)", "[97 98 120 1]"),
                (
                    "(progn (clear-this-command-keys t) (this-command-keys-vector))",
                    "[]",
                ),
                ("(length (recent-keys))", "9"),
                ("(progn (clear-this-command-keys) (recent-keys))", "[]"),
                ("(lossage-size)", "300"),
                (
                    "(lossage-size 99)",
                    "error: (user-error \"Value must be >= 100\")",
                ),
                (
                    "(lossage-size 'x)",
                    "error: (user-error \"Value must be a positive integer\")",
                ),
                ("(lossage-size 150)", "150"),
                ("(recursion-depth)", "0"),
            ],
        );
    });
}

#[test]
fn translation_and_special_events() {
    on_big_stack(|| {
        let mut lisp = reader();
        check(
            &mut lisp,
            &[(
                "(progn (setq keyboard-translate-table (make-char-table 'keyboard-translate-table)) (aset keyboard-translate-table ?x ?y) nil)",
                "nil",
            )],
        );
        store(&mut lisp, u32::from(b'x'), 0);
        check(
            &mut lisp,
            &[
                ("(read-char)", "121"),
                (
                    "(progn (setq keyboard-translate-table nil ran nil) (fset 'command-execute (function (lambda (cmd &rest args) (setq ran (list cmd (aref (nth 1 args) 0)))))) (define-key special-event-map [delete-frame] 'my-delete) (insert-special-event '(delete-frame 42)) nil)",
                    "nil",
                ),
            ],
        );
        store(&mut lisp, u32::from(b'z'), 0);
        check(
            &mut lisp,
            &[
                ("(read-event)", "122"),
                ("ran", "(my-delete (delete-frame (nil)))"),
                ("last-input-event", "122"),
            ],
        );
    });
}

#[test]
fn quitting_while_reading() {
    on_big_stack(|| {
        let mut lisp = reader();
        check(&mut lisp, &[("(setq inhibit-quit t)", "t")]);
        store(&mut lisp, u32::from(b'g'), CTRL_MODIFIER);
        check(
            &mut lisp,
            &[
                ("quit-flag", "t"),
                ("(list (read-event) quit-flag)", "(7 nil)"),
                ("(this-command-keys-vector)", "[7]"),
            ],
        );
        assert!(lisp.keyboard.getcjmp.is_nil());
    });
}
