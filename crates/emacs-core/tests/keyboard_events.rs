mod common;

use common::{check, eval, on_big_stack};
use emacs_core::keyboard::{CTRL_MODIFIER, META_MODIFIER, SHIFT_MODIFIER};
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_core::termhooks::{EventKind, KbdEvent};
use emacs_core::value::Value;

fn key(lisp: &Lisp, kind: EventKind, code: u32, modifiers: i64) -> KbdEvent {
    KbdEvent {
        kind,
        code,
        modifiers,
        frame_or_window: lisp.frames.selected_frame,
        ..KbdEvent::default()
    }
}

fn read(lisp: &mut Lisp) -> String {
    let (event, _, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
    lisp.prin1_to_string(event)
}

#[test]
fn keystroke_events() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let cases = [
            (EventKind::AsciiKeystroke, u32::from(b'a'), 0, "97"),
            (
                EventKind::AsciiKeystroke,
                u32::from(b'a'),
                CTRL_MODIFIER,
                "1",
            ),
            (
                EventKind::AsciiKeystroke,
                u32::from(b'a'),
                META_MODIFIER,
                "134217825",
            ),
            (
                EventKind::AsciiKeystroke,
                u32::from(b'A'),
                CTRL_MODIFIER,
                "1",
            ),
            (
                EventKind::AsciiKeystroke,
                u32::from(b'a'),
                CTRL_MODIFIER | SHIFT_MODIFIER,
                "33554433",
            ),
            (
                EventKind::AsciiKeystroke,
                u32::from(b' '),
                SHIFT_MODIFIER,
                "33554464",
            ),
            (EventKind::MultibyteCharKeystroke, 0xE9, 0, "233"),
            (EventKind::NonAsciiKeystroke, 0xFF51, 0, "left"),
            (
                EventKind::NonAsciiKeystroke,
                0xFF51,
                CTRL_MODIFIER,
                "C-left",
            ),
            (EventKind::NonAsciiKeystroke, 0xFFBE, META_MODIFIER, "M-f1"),
            (EventKind::NonAsciiKeystroke, 0xFF08, 0, "backspace"),
            (EventKind::NonAsciiKeystroke, 0xFFFF, 0, "delete"),
            (EventKind::NonAsciiKeystroke, 0xFE20, 0, "iso-lefttab"),
            (
                EventKind::NonAsciiKeystroke,
                0x1008_FF02,
                0,
                "key-269025026",
            ),
        ];
        for (kind, code, modifiers, expected) in cases {
            let event = key(&lisp, kind, code, modifiers);
            lisp.kbd_buffer_store_event(event).unwrap();
            assert_eq!(
                read(&mut lisp),
                expected,
                "{kind:?} {code:#x} {modifiers:#x}"
            );
        }
        assert_eq!(eval(&mut lisp, "(get 'C-left 'event-kind)"), "function-key");
        assert_eq!(eval(&mut lisp, "(get 'left 'event-kind)"), "function-key");
        assert_eq!(
            eval(&mut lisp, "(internal-event-symbol-parse-modifiers 'C-left)"),
            "(left control)"
        );
        let mut event = key(&lisp, EventKind::MultibyteCharKeystroke, 0, 0);
        event.arg = lisp.string("x\u{e9}");
        lisp.kbd_buffer_store_event(event).unwrap();
        assert_eq!(read(&mut lisp), "120");
        assert!(lisp.detect_input_pending().unwrap());
        assert_eq!(read(&mut lisp), "233");
        assert!(!lisp.detect_input_pending().unwrap());
    });
}

#[test]
fn frame_and_special_events() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let f = lisp.frames.selected_frame;
        for (kind, head) in [
            (EventKind::DeleteWindow, sym::DELETE_FRAME),
            (EventKind::Iconify, sym::ICONIFY_FRAME),
            (EventKind::Deiconify, sym::MAKE_FRAME_VISIBLE),
            (EventKind::MoveFrame, sym::MOVE_FRAME),
        ] {
            lisp.kbd_buffer_store_event(key(&lisp, kind, 0, 0)).unwrap();
            let (event, _, frame) = lisp.kbd_buffer_get_event(None, None).unwrap();
            assert_eq!(lisp.xcar(event), head);
            let arg = lisp.xcar(lisp.xcdr(event));
            assert_eq!((lisp.xcar(arg), lisp.xcdr(arg)), (f, Value::NIL));
            assert_eq!(frame, f);
        }
        for (kind, head) in [
            (EventKind::FocusIn, sym::FOCUS_IN),
            (EventKind::FocusOut, sym::FOCUS_OUT),
        ] {
            lisp.kbd_buffer_store_event(key(&lisp, kind, 0, 0)).unwrap();
            let (event, _, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
            assert_eq!(lisp.xcar(event), head);
            assert_eq!(lisp.xcar(lisp.xcdr(event)), f);
        }
    });
}

#[test]
fn quitting_and_throwing_on_input() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let event = key(
            &lisp,
            EventKind::AsciiKeystroke,
            u32::from(b'g'),
            CTRL_MODIFIER,
        );
        lisp.kbd_buffer_store_event(event).unwrap();
        assert_eq!(eval(&mut lisp, "quit-flag"), "t");
        lisp.set_plain_value(sym::QUIT_FLAG, Value::NIL);
        assert!(!lisp.detect_input_pending().unwrap());
        eval(&mut lisp, "(setq throw-on-input 'stop)");
        lisp.kbd_buffer_store_event(key(&lisp, EventKind::FocusIn, 0, 0))
            .unwrap();
        assert_eq!(eval(&mut lisp, "quit-flag"), "nil");
        lisp.kbd_buffer_store_event(key(&lisp, EventKind::AsciiKeystroke, 97, 0))
            .unwrap();
        assert_eq!(eval(&mut lisp, "quit-flag"), "stop");
        lisp.set_plain_value(sym::QUIT_FLAG, Value::NIL);
        lisp.set_plain_value(sym::THROW_ON_INPUT, Value::NIL);
        for _ in 0..5000 {
            lisp.kbd_buffer_store_event(key(&lisp, EventKind::AsciiKeystroke, 97, 0))
                .unwrap();
        }
        let mut n = 0;
        while lisp.detect_input_pending().unwrap() {
            read(&mut lisp);
            n += 1;
        }
        assert_eq!(n, 4095);
    });
}

#[test]
fn special_events_and_input_pending() {
    check(&[
        ("(input-pending-p)", "nil"),
        ("(insert-special-event '(focus-in))", "nil"),
        ("(input-pending-p)", "nil"),
        ("(insert-special-event '(iconify-frame))", "nil"),
        ("(input-pending-p)", "nil"),
        ("(setq input-pending-p-filter-events nil)", "nil"),
        ("(input-pending-p)", "t"),
        ("(setq input-pending-p-filter-events t)", "t"),
        ("(input-pending-p)", "nil"),
        ("(insert-special-event '(delete-frame . (1 2)))", "nil"),
        ("(input-pending-p)", "t"),
        ("(input-pending-p t)", "t"),
        (
            "(insert-special-event '(bogus))",
            "error: (error \"Invalid special event kind\" bogus)",
        ),
        (
            "(insert-special-event 'x)",
            "error: (wrong-type-argument consp x)",
        ),
        (
            "(progn (setq unread-command-events '(1)) (input-pending-p))",
            "t",
        ),
    ]);
}

#[test]
fn special_event_bindings() {
    check(&[(
        "(mapcar (function (lambda (e) (lookup-key special-event-map (vector e)))) '(delete-frame iconify-frame make-frame-visible save-session thread-event file-notify config-changed-event language-change focus-in focus-out move-frame sleep-event))",
        if cfg!(windows) {
            "(handle-delete-frame ignore ignore handle-save-session thread-handle-event file-notify-handle-event ignore ignore handle-focus-in handle-focus-out handle-move-frame ignore)"
        } else {
            "(handle-delete-frame ignore ignore handle-save-session thread-handle-event file-notify-handle-event ignore nil handle-focus-in handle-focus-out handle-move-frame ignore)"
        },
    )]);
}

#[test]
fn internal_handle_focus_in() {
    check(&[
        (
            "(internal-handle-focus-in '(focus-in nil))",
            "error: (error \"Invalid focus-in event\")",
        ),
        (
            "(internal-handle-focus-in 'focus-in)",
            "error: (error \"Invalid focus-in event\")",
        ),
        (
            "(list (internal-handle-focus-in (list 'focus-in (selected-frame))) (progn (setq unread-command-events nil) (read-event nil nil 0)))",
            "(nil nil)",
        ),
    ]);
}
