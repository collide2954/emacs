use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
use emacs_gui::display::Display;
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

fn displayed() -> (Lisp, Value) {
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(
        &mut lisp,
        Display::new(fmpsc::unbounded().0, mpsc::channel().1),
    );
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    assert_eq!(
        eval(
            &mut lisp,
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (defalias 'timer-event-handler (function (lambda (timer) (setq at n)))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"w\") (width . 30) (height . 8)))) (select-frame f) (setq bidi-paragraph-direction 'left-to-right) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.frame_mut(f).flags.visible = true;
    lisp.redisplay().unwrap();
    eval(
        &mut lisp,
        "(setq n 0 pre-redisplay-function (function (lambda (_) (setq n (1+ n)))))",
    );
    (lisp, f)
}

const RIPE: &str =
    "(progn (setq n 0 at nil timer-list (list (vector nil 0 0 0 nil 'tick nil nil 0 nil))) nil)";

#[test]
fn redisplay_redisplays_unless_a_keyboard_macro_is_executing() {
    on_big_stack(|| {
        let (mut lisp, _) = displayed();
        assert_eq!(
            eval(&mut lisp, "(progn (setq n 0) (list (redisplay) (> n 0)))"),
            "(t t)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq n 0) (list (redisplay 'force) (> n 0)))"
            ),
            "(t t)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq n 0) (list (let ((executing-kbd-macro \"x\")) (redisplay)) n))"
            ),
            "(nil 0)"
        );
    });
}

#[test]
fn timers_that_run_while_waiting_redisplay_only_with_do_display() {
    on_big_stack(|| {
        let (mut lisp, _) = displayed();
        eval(&mut lisp, RIPE);
        lisp.wait_reading_process_output(0, 10_000_000, 0, false, Value::NIL)
            .unwrap();
        assert_eq!(eval(&mut lisp, "(list at n)"), "(0 0)");
        eval(&mut lisp, RIPE);
        lisp.wait_reading_process_output(0, 10_000_000, 0, true, Value::NIL)
            .unwrap();
        assert_eq!(eval(&mut lisp, "(list at (> n at))"), "(0 t)");
    });
}

#[test]
fn swallowing_events_and_detecting_input_redisplay_after_timers() {
    on_big_stack(|| {
        let (mut lisp, _) = displayed();
        eval(&mut lisp, RIPE);
        lisp.swallow_events(false).unwrap();
        assert_eq!(eval(&mut lisp, "(list at n)"), "(0 0)");
        eval(&mut lisp, RIPE);
        lisp.swallow_events(true).unwrap();
        assert_eq!(eval(&mut lisp, "(list at (> n at))"), "(0 t)");
        eval(&mut lisp, RIPE);
        assert!(!lisp.detect_input_pending_run_timers(false).unwrap());
        assert_eq!(eval(&mut lisp, "(list at n)"), "(0 0)");
        eval(&mut lisp, RIPE);
        assert!(!lisp.detect_input_pending_run_timers(true).unwrap());
        assert_eq!(eval(&mut lisp, "(list at (> n at))"), "(0 t)");
    });
}

#[test]
fn sit_for_redisplays_before_waiting_with_display_option_two() {
    on_big_stack(|| {
        let (mut lisp, _) = displayed();
        eval(&mut lisp, "(setq n 0)");
        assert_eq!(
            lisp.sit_for(Value::fixnum(0), false, 1).unwrap(),
            emacs_core::symbols::T
        );
        assert_eq!(eval(&mut lisp, "n"), "0");
        assert_eq!(
            lisp.sit_for(Value::fixnum(0), false, 2).unwrap(),
            emacs_core::symbols::T
        );
        assert_eq!(eval(&mut lisp, "(> n 0)"), "t");
    });
}

#[test]
fn a_garbaged_frame_is_redisplayed_while_waiting_with_do_display() {
    on_big_stack(|| {
        let (mut lisp, _) = displayed();
        eval(&mut lisp, "(setq n 0)");
        lisp.frames.frame_garbaged = true;
        lisp.wait_reading_process_output(0, 1_000_000, 0, false, Value::NIL)
            .unwrap();
        assert_eq!(eval(&mut lisp, "n"), "0");
        lisp.frames.frame_garbaged = true;
        lisp.wait_reading_process_output(0, 1_000_000, 0, true, Value::NIL)
            .unwrap();
        assert_eq!(eval(&mut lisp, "(> n 0)"), "t");
    });
}

#[test]
fn a_delayed_size_change_is_applied_after_waiting() {
    on_big_stack(|| {
        let (mut lisp, f) = displayed();
        let (width, height) = {
            let g = &lisp.frame(f).geometry;
            (g.pixel_width, g.pixel_height)
        };
        lisp.change_frame_size(f, width + 40, height + 40, true, false)
            .unwrap();
        assert!(lisp.frames.delayed_size_change);
        lisp.wait_reading_process_output(0, 1_000_000, 0, false, Value::NIL)
            .unwrap();
        assert!(!lisp.frames.delayed_size_change);
        assert!(!lisp.frame(f).flags.new_size_p);
    });
}
