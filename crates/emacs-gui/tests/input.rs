use std::sync::mpsc;
use std::time::{Duration, Instant};

use emacs_core::emacs::Startup;
use emacs_core::keyboard::{CTRL_MODIFIER, DOWN_MODIFIER, UP_MODIFIER};
use emacs_core::lisp::Lisp;
use emacs_core::termhooks::{EventKind, InputEvent};
use emacs_core::value::Value;
use emacs_gui::display::Display;
use emacs_gui::protocol::Notice;
use iced::futures::channel::mpsc as fmpsc;

fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(512 << 20)
            .spawn_scoped(scope, f)
            .expect("spawn test thread")
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

fn eval(lisp: &mut Lisp, source: &str) -> String {
    match lisp.eval_string(source) {
        Ok(value) => lisp.prin1_to_string(value),
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            format!("error: {}", lisp.prin1_to_string(condition))
        }
    }
}

fn with_frame(notices: mpsc::Receiver<Notice>) -> (Lisp, Value) {
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(&mut lisp, Display::new(fmpsc::unbounded().0, notices));
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    eval(
        &mut lisp,
        "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"i\") (width . 20) (height . 5)))))",
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    (lisp, f)
}

fn input(frame: u64, kind: EventKind, code: u32, modifiers: i64) -> Notice {
    Notice::Input(InputEvent {
        kind,
        code,
        modifiers,
        frame,
        timestamp: 77,
        ..InputEvent::default()
    })
}

fn next_event(lisp: &mut Lisp) -> String {
    let (event, _, _, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
    lisp.prin1_to_string(event)
}

#[test]
fn mouse_notices_become_mouse_events() {
    on_big_stack(|| {
        let (tx, notices) = mpsc::channel();
        let (mut lisp, f) = with_frame(notices);
        let id = lisp.frame(f).id as u64;
        let window = lisp.frame(f).selected_window;
        let mouse = |kind, modifiers, timestamp| {
            Notice::Input(InputEvent {
                kind,
                code: 0,
                modifiers,
                x: 3,
                y: 3,
                timestamp,
                frame: id,
                ..InputEvent::default()
            })
        };
        tx.send(mouse(EventKind::MouseClick, DOWN_MODIFIER, 1000))
            .unwrap();
        tx.send(Notice::Input(InputEvent {
            frame: id + 1000,
            ..match mouse(EventKind::MouseClick, DOWN_MODIFIER, 1050) {
                Notice::Input(input) => input,
                _ => unreachable!(),
            }
        }))
        .unwrap();
        tx.send(mouse(EventKind::MouseClick, UP_MODIFIER, 1100))
            .unwrap();
        tx.send(mouse(EventKind::Wheel, DOWN_MODIFIER, 2000))
            .unwrap();
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "t");
        let mut seen = Vec::new();
        while seen.len() < 3 {
            let (event, _, _, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
            let head = lisp.car_safe(event);
            if head == lisp.intern("switch-frame") {
                continue;
            }
            let position = lisp.car_safe(lisp.cdr_safe(event));
            seen.push((
                lisp.prin1_to_string(head),
                lisp.car_safe(position) == window,
            ));
        }
        assert_eq!(
            seen,
            [
                ("down-mouse-1".to_string(), true),
                ("mouse-1".to_string(), true),
                ("wheel-down".to_string(), true)
            ]
        );
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "nil");
        assert_eq!(
            eval(&mut lisp, "last-event-device"),
            "\"Virtual core pointer\""
        );
    });
}

#[test]
fn motion_notices_move_the_mouse() {
    use emacs_core::dispextern::GlyphRowArea;
    on_big_stack(|| {
        let (tx, notices) = mpsc::channel();
        let (mut lisp, f) = with_frame(notices);
        eval(
            &mut lisp,
            "(progn (select-frame f) (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"text\")) (set-buffer \"text\") (setq bidi-paragraph-direction 'left-to-right) (insert \"line 0\\nline 1\\n\") (goto-char 1))",
        );
        lisp.frame_mut(f).flags.visible = true;
        lisp.redisplay().unwrap();
        let id = lisp.frame(f).id as u64;
        let frame = lisp.prin1_to_string(f);
        let w = lisp.frame(f).root_window;
        let g = lisp.frame(f).geometry.clone();
        let (cw, lh, ib) = (g.column_width, g.line_height, g.internal_border_width);
        let x = lisp.window_box_left(w, GlyphRowArea::Text) + 2 * cw + 1;
        let y = ib + lh + 2;
        let motion = |x: i64, y: i64, timestamp: u64| Notice::Motion {
            frame: id,
            x,
            y,
            timestamp,
        };
        assert_eq!(
            eval(&mut lisp, "(mouse-pixel-position)"),
            format!("({frame} nil)")
        );
        tx.send(motion(x, y, 500)).unwrap();
        lisp.gobble_input().unwrap();
        assert!(lisp.frame(f).flags.mouse_moved);
        assert_eq!(
            eval(&mut lisp, "(mouse-pixel-position)"),
            format!("({frame} {x} . {y})")
        );
        assert!(!lisp.frame(f).flags.mouse_moved);
        let (col, row) = lisp.pixel_to_glyph_coords(f, x, y, true);
        assert_eq!(
            eval(&mut lisp, "(mouse-position)"),
            format!("({frame} {col} . {row})")
        );
        tx.send(motion(x + 1, y, 600)).unwrap();
        lisp.gobble_input().unwrap();
        assert!(!lisp.frame(f).flags.mouse_moved);
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "nil");
        eval(&mut lisp, "(setq track-mouse t)");
        tx.send(motion(x + cw, y, 700)).unwrap();
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "t");
        let (event, _, _, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
        let position = lisp.car_safe(lisp.cdr_safe(event));
        assert_eq!(lisp.car_safe(event), lisp.intern("mouse-movement"));
        assert_eq!(lisp.car_safe(position), w);
        let timestamp = lisp.car_safe(lisp.cdr_safe(lisp.cdr_safe(lisp.cdr_safe(position))));
        assert_eq!(lisp.prin1_to_string(timestamp), "700");
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "nil");
        eval(
            &mut lisp,
            "(setq track-mouse nil mouse-autoselect-window t)",
        );
        let mini = lisp.frame(f).minibuffer_window;
        let mini_y = ib + lisp.window(mini).pixel_top + 2;
        tx.send(motion(x, mini_y, 800)).unwrap();
        tx.send(motion(x + 3 * cw, mini_y, 900)).unwrap();
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "t");
        let window = lisp.prin1_to_string(mini);
        assert_eq!(next_event(&mut lisp), format!("(select-window ({window}))"));
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "nil");
    });
}

#[test]
fn notices_become_lisp_events() {
    on_big_stack(|| {
        let (tx, notices) = mpsc::channel();
        let (mut lisp, f) = with_frame(notices);
        let id = lisp.frame(f).id as u64;
        let frame = lisp.prin1_to_string(f);
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "nil");
        tx.send(input(id, EventKind::AsciiKeystroke, u32::from(b'x'), 0))
            .unwrap();
        tx.send(input(
            id + 1000,
            EventKind::AsciiKeystroke,
            u32::from(b'y'),
            0,
        ))
        .unwrap();
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "t");
        assert_eq!(next_event(&mut lisp), format!("(switch-frame {frame})"));
        assert_eq!(next_event(&mut lisp), "120");
        assert_eq!(eval(&mut lisp, "(eq last-event-frame f)"), "t");
        assert_eq!(
            eval(&mut lisp, "last-event-device"),
            "\"Virtual core keyboard\""
        );
        tx.send(input(
            id,
            EventKind::AsciiKeystroke,
            u32::from(b'a'),
            CTRL_MODIFIER,
        ))
        .unwrap();
        tx.send(input(id, EventKind::NonAsciiKeystroke, 0x1008_FF02, 0))
            .unwrap();
        tx.send(Notice::Input(InputEvent {
            kind: EventKind::MultibyteCharKeystroke,
            frame: id,
            arg: Some("\u{e9}".to_string()),
            ..InputEvent::default()
        }))
        .unwrap();
        tx.send(input(id, EventKind::DeleteWindow, 0, 0)).unwrap();
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "t");
        assert_eq!(next_event(&mut lisp), "1");
        assert_eq!(next_event(&mut lisp), "XF86MonBrightnessUp");
        assert_eq!(eval(&mut lisp, "locale-coding-system"), "nil");
        assert_eq!(next_event(&mut lisp), "195");
        assert_eq!(next_event(&mut lisp), "169");
        assert_eq!(next_event(&mut lisp), format!("(delete-frame ({frame}))"));
        assert_eq!(eval(&mut lisp, "(input-pending-p)"), "nil");
    });
}

#[test]
fn focus_notices_track_the_focus_frame() {
    on_big_stack(|| {
        let (tx, notices) = mpsc::channel();
        let (mut lisp, f) = with_frame(notices);
        let id = lisp.frame(f).id as u64;
        let frame = lisp.prin1_to_string(f);
        tx.send(input(id, EventKind::FocusIn, 0, 0)).unwrap();
        tx.send(input(id, EventKind::FocusIn, 0, 0)).unwrap();
        lisp.gobble_input().unwrap();
        assert_eq!(emacs_gui::pgtkterm::focus_frame(&lisp, f), Some(f));
        assert_eq!(next_event(&mut lisp), format!("(focus-in {frame})"));
        assert_eq!(eval(&mut lisp, "(input-pending-p t)"), "nil");
        tx.send(input(id, EventKind::FocusOut, 0, 0)).unwrap();
        lisp.gobble_input().unwrap();
        assert_eq!(emacs_gui::pgtkterm::focus_frame(&lisp, f), None);
        assert_eq!(next_event(&mut lisp), format!("(focus-out {frame})"));
    });
}

#[test]
fn waiting_for_keyboard_input_wakes_up_on_a_notice() {
    on_big_stack(|| {
        let (tx, notices) = mpsc::channel();
        let (mut lisp, f) = with_frame(notices);
        let id = lisp.frame(f).id as u64;
        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            tx.send(input(id, EventKind::AsciiKeystroke, u32::from(b'z'), 0))
                .unwrap();
            tx
        });
        let start = Instant::now();
        lisp.wait_reading_process_output(20, 0, 1, false, Value::NIL)
            .unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(150), "{elapsed:?}");
        assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
        assert!(lisp.detect_input_pending().unwrap());
        drop(sender.join().unwrap());
    });
}
