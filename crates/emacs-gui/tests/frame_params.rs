use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::frame::ResourceType;
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
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

fn eval(lisp: &mut Lisp, source: &str) -> Value {
    match lisp.eval_string(source) {
        Ok(value) => value,
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            panic!("{source}: {}", lisp.prin1_to_string(condition))
        }
    }
}

fn show(lisp: &mut Lisp, value: Value) -> String {
    if value == sym::UNBOUND {
        return "unbound".to_string();
    }
    lisp.prin1_to_string(value)
}

fn connected() -> (Lisp, Value) {
    let mut lisp = Lisp::boot(Startup::program(false));
    let (commands, _command_rx) = fmpsc::unbounded();
    let (_notice_tx, notices) = mpsc::channel();
    emacs_gui::install(&mut lisp, Display::new(commands, notices));
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (setq x-resource-name \"emacs\") (x-open-connection nil))",
    );
    let check = lisp
        .window_system
        .expect("window system")
        .check_x_display_info;
    let dpyinfo = check(&mut lisp, Value::NIL).expect("display info");
    (lisp, dpyinfo)
}

#[test]
fn gui_display_get_arg_consumes_alist_entries() {
    on_big_stack(|| {
        let (mut lisp, dpyinfo) = connected();
        let alist = eval(
            &mut lisp,
            "(setq params (list (cons 'foo 1) (cons 'bar 2) (cons 'foo 3)))",
        );
        let foo = lisp.intern("foo");
        let v = lisp
            .gui_display_get_arg(Some(dpyinfo), alist, foo, None, None, ResourceType::Number)
            .unwrap();
        assert_eq!(show(&mut lisp, v), "1");
        let params = eval(&mut lisp, "params");
        assert_eq!(show(&mut lisp, params), "((nil . 1) (bar . 2) (nil . 3))");
        eval(&mut lisp, "(setq default-frame-alist '((baz . 7)))");
        let baz = lisp.intern("baz");
        let v = lisp
            .gui_display_get_arg(
                Some(dpyinfo),
                alist,
                baz,
                Some("baz"),
                Some("Baz"),
                ResourceType::Number,
            )
            .unwrap();
        assert_eq!(show(&mut lisp, v), "7");
        let qux = lisp.intern("qux");
        let v = lisp
            .gui_display_get_arg(
                Some(dpyinfo),
                alist,
                qux,
                Some("qux"),
                Some("Qux"),
                ResourceType::String,
            )
            .unwrap();
        assert_eq!(show(&mut lisp, v), "unbound");
        let v = lisp
            .gui_display_get_arg(
                None,
                alist,
                qux,
                Some("qux"),
                Some("Qux"),
                ResourceType::String,
            )
            .unwrap();
        assert_eq!(show(&mut lisp, v), "unbound");
    });
}

#[test]
fn resource_type_conversions() {
    assert_eq!(emacs_core::frame::c_atoi(b"  -42abc"), -42);
    assert_eq!(emacs_core::frame::c_atoi(b"+7"), 7);
    assert_eq!(emacs_core::frame::c_atoi(b"x9"), 0);
    assert_eq!(emacs_core::frame::c_atof(b" 1.5e2xyz"), 150.0);
    assert_eq!(emacs_core::frame::c_atof(b".25"), 0.25);
    assert_eq!(emacs_core::frame::c_atof(b"abc"), 0.0);
}

#[test]
fn generic_frame_parameter_handlers() {
    on_big_stack(|| {
        let (mut lisp, _dpyinfo) = connected();
        let f = lisp.frames.selected_frame;
        lisp.gui_set_autoraise(f, sym::T, Value::NIL).unwrap();
        lisp.gui_set_unsplittable(f, sym::T, Value::NIL).unwrap();
        assert!(lisp.frame(f).flags.auto_raise && lisp.frame(f).flags.no_split);
        lisp.gui_set_line_spacing(f, Value::fixnum(3), Value::NIL)
            .unwrap();
        assert_eq!(lisp.frame(f).geometry.extra_line_spacing, 3);
        let pair = eval(&mut lisp, "'(1 . 2)");
        lisp.gui_set_line_spacing(f, pair, Value::NIL).unwrap();
        assert_eq!(
            (
                lisp.frame(f).geometry.extra_line_spacing,
                lisp.frame(f).geometry.extra_line_spacing_above
            ),
            (3, 1)
        );
        let bad = lisp.intern("bad");
        let e = lisp.gui_set_line_spacing(f, bad, Value::NIL).unwrap_err();
        let c = lisp.flow_condition(&e);
        assert_eq!(
            lisp.prin1_to_string(c),
            "(error \"Invalid line-spacing\" bad)"
        );
        let half = eval(&mut lisp, "0.5");
        lisp.gui_set_alpha(f, half, Value::NIL).unwrap();
        assert_eq!(lisp.frame(f).alpha, [0.5, 0.5]);
        let e = lisp
            .gui_set_alpha(f, Value::fixnum(101), Value::NIL)
            .unwrap_err();
        let c = lisp.flow_condition(&e);
        assert_eq!(lisp.prin1_to_string(c), "(args-out-of-range 0 100)");
        let maximized = sym::MAXIMIZED;
        lisp.gui_set_fullscreen(f, maximized, Value::NIL).unwrap();
        assert_eq!(
            lisp.frame(f).want_fullscreen,
            emacs_core::frame::Fullscreen::Maximized
        );
        let gamma = eval(&mut lisp, "2.2");
        lisp.gui_set_screen_gamma(f, gamma, Value::NIL).unwrap();
        assert!((lisp.frame(f).gamma - 1.0 / (0.4545 * 2.2)).abs() < 1e-12);
        let e = lisp
            .gui_set_screen_gamma(f, Value::fixnum(-1), Value::NIL)
            .unwrap_err();
        let c = lisp.flow_condition(&e);
        assert_eq!(
            lisp.prin1_to_string(c),
            "(error \"Invalid screen-gamma\" -1)"
        );
    });
}
