mod common;

use common::{check, eval, on_big_stack};
use emacs_core::error::LispResult;
use emacs_core::keyboard::{
    CTRL_MODIFIER, DOWN_MODIFIER, META_MODIFIER, SHIFT_MODIFIER, UP_MODIFIER,
};
use emacs_core::lisp::Lisp;
use emacs_core::subr::Subr;
use emacs_core::symbols as sym;
use emacs_core::termhooks::{EventKind, KbdEvent};
use emacs_core::timefns::{current_timespec, timespec_add};
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
    let (event, _, _, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
    lisp.prin1_to_string(event)
}

fn mouse(
    lisp: &mut Lisp,
    kind: EventKind,
    code: u32,
    modifiers: i64,
    (x, y): (i64, i64),
    timestamp: u64,
) -> String {
    let mut event = KbdEvent {
        kind,
        code,
        modifiers,
        x: Value::fixnum(x),
        y: Value::fixnum(y),
        timestamp,
        frame_or_window: lisp.frames.selected_frame,
        ..KbdEvent::default()
    };
    let obj = lisp.make_lispy_event(&mut event).unwrap();
    if obj.is_nil() {
        return "nil".to_string();
    }
    let items = lisp.list_to_vec(obj).unwrap();
    let mut out = format!("{} {}", lisp.prin1_to_string(items[0]), items.len());
    if let Some(count) = items.get(2).filter(|v| v.is_fixnum()) {
        out.push_str(&format!(" {}", lisp.prin1_to_string(*count)));
    }
    out
}

#[test]
fn posn_point_and_track_mouse() {
    check(&[
        ("(posn-point '(w nil nil nil nil 42))", "42"),
        ("(posn-point '(w (3 . 4)))", "3"),
        ("(posn-point '(w 7))", "7"),
        ("(posn-point '(w vertical-scroll-bar))", "nil"),
        ("(posn-point 1)", "error: (wrong-type-argument listp 1)"),
        (
            "(list (internal--track-mouse (function (lambda () track-mouse))) track-mouse)",
            "(t nil)",
        ),
        (
            "(list (condition-case nil (internal--track-mouse (function (lambda () (error \"x\")))) (error track-mouse)) track-mouse)",
            "(nil nil)",
        ),
        (
            "(let ((track-mouse 'dragging)) (list (internal--track-mouse (function (lambda () track-mouse))) track-mouse))",
            "(t dragging)",
        ),
    ]);
}

#[test]
fn mouse_clicks_and_wheels() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        let click = EventKind::MouseClick;
        for (kind, code, modifiers, at, t, expected) in [
            (click, 0, DOWN_MODIFIER, (0, 1), 1000, "down-mouse-1 2"),
            (click, 0, UP_MODIFIER, (0, 1), 1100, "mouse-1 2"),
            (
                click,
                0,
                DOWN_MODIFIER,
                (0, 1),
                1200,
                "double-down-mouse-1 3 2",
            ),
            (click, 0, UP_MODIFIER, (0, 1), 1250, "double-mouse-1 3 2"),
            (
                click,
                0,
                DOWN_MODIFIER,
                (0, 1),
                1300,
                "triple-down-mouse-1 3 3",
            ),
            (click, 0, UP_MODIFIER, (0, 1), 1350, "triple-mouse-1 3 3"),
            (click, 0, DOWN_MODIFIER, (0, 1), 5000, "down-mouse-1 2"),
            (click, 0, UP_MODIFIER, (10, 3), 5100, "drag-mouse-1 3"),
            (click, 0, UP_MODIFIER, (10, 3), 5200, "nil"),
            (
                click,
                0,
                DOWN_MODIFIER | CTRL_MODIFIER,
                (2, 2),
                5300,
                "C-down-mouse-1 2",
            ),
            (click, 9, DOWN_MODIFIER, (2, 2), 5400, "down-mouse-10 2"),
            (click, 9, UP_MODIFIER, (2, 2), 5450, "mouse-10 2"),
            (EventKind::Wheel, 0, UP_MODIFIER, (1, 1), 6000, "wheel-up 2"),
            (
                EventKind::Wheel,
                0,
                UP_MODIFIER,
                (1, 1),
                6100,
                "double-wheel-up 3 2",
            ),
            (
                EventKind::HorizWheel,
                0,
                DOWN_MODIFIER,
                (1, 1),
                6150,
                "wheel-right 2",
            ),
            (
                EventKind::HorizWheel,
                0,
                UP_MODIFIER,
                (1, 1),
                6200,
                "wheel-left 2",
            ),
            (
                EventKind::Wheel,
                0,
                DOWN_MODIFIER,
                (1, 1),
                6250,
                "wheel-down 2",
            ),
        ] {
            assert_eq!(
                mouse(l, kind, code, modifiers, at, t),
                expected,
                "{kind:?} {code} {modifiers} {at:?} {t}"
            );
        }
        for (source, expected) in [
            ("(get 'mouse-1 'event-kind)", "mouse-click"),
            ("(get 'wheel-up 'event-kind)", "mouse-click"),
            ("(get 'mouse-10 'event-kind)", "mouse-click"),
        ] {
            assert_eq!(eval(l, source), expected, "evaluating {source}");
        }
    });
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
            let (event, _, frame, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
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
            let (event, _, _, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
            assert_eq!(lisp.xcar(event), head);
            assert_eq!(lisp.xcar(lisp.xcdr(event)), f);
        }
    });
}

static SWITCH_KBOARD: Subr = Subr::fixed("test-switch-kboard", 0, 0, switch_kboard);

fn switch_kboard(lisp: &mut Lisp, _: &[Value]) -> LispResult {
    lisp.current_kboard = lisp.kboards.len() - 1;
    Ok(Value::NIL)
}

#[test]
fn event_kind_and_kboard_switched_while_waiting() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        lisp.kbd_buffer_store_event(key(&lisp, EventKind::AsciiKeystroke, 97, 0))
            .unwrap();
        let (event, _, _, kind) = lisp.kbd_buffer_get_event(None, None).unwrap();
        assert_eq!(
            (event, kind),
            (Value::fixnum(97), EventKind::AsciiKeystroke)
        );
        eval(&mut lisp, "(setq unread-command-events '(1))");
        let (event, _, _, kind) = lisp.kbd_buffer_get_event(None, None).unwrap();
        assert_eq!((event, kind), (Value::fixnum(1), EventKind::NoEvent));
        let other = lisp.allocate_kboard(Value::NIL);
        assert_ne!(lisp.current_kboard, other);
        lisp.defsubr(&SWITCH_KBOARD);
        eval(
            &mut lisp,
            "(defalias 'timer-event-handler (function (lambda (timer) (test-switch-kboard))))",
        );
        eval(
            &mut lisp,
            "(setq timer-list (list (vector nil 0 0 0 nil 'switch nil nil 0 nil)))",
        );
        let end = timespec_add(current_timespec(), (0, 200_000_000));
        let (event, kb, _, kind) = lisp.kbd_buffer_get_event(None, Some(end)).unwrap();
        assert_eq!(
            (event, kb, kind),
            (Value::fixnum(-2), other, EventKind::NoEvent)
        );
        assert_eq!(lisp.current_kboard, other);
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
