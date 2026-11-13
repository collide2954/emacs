mod common;

use common::check;

#[test]
fn initial_terminal_and_frame() {
    check(&[
        ("(framep (selected-frame))", "t"),
        ("(frame-live-p (selected-frame))", "t"),
        ("(window-system)", "nil"),
        ("(length (frame-list))", "1"),
        ("(frame-parameter nil 'name)", "\"F1\""),
        ("(frame-parameter nil 'minibuffer)", "t"),
        ("(frame-parameter nil 'menu-bar-lines)", "1"),
        (
            "(frame-parameter nil 'background-color)",
            "\"unspecified-bg\"",
        ),
        (
            "(frame-parameter nil 'foreground-color)",
            "\"unspecified-fg\"",
        ),
        ("(frame-text-cols)", "80"),
        ("(frame-text-lines)", "25"),
        ("(terminal-name)", "\"initial_terminal\""),
        ("(terminal-live-p nil)", "t"),
        ("(frame-terminal)", "#<terminal 0 on initial_terminal>"),
        ("(frame-initial-p)", "t"),
        ("(eq terminal-frame (selected-frame))", "t"),
        ("(eq (next-frame) (selected-frame))", "t"),
        ("(frame-visible-p (selected-frame))", "t"),
    ]);
}

#[test]
fn tty_color_parameter_that_is_not_unspecified() {
    check(&[
        (
            "(progn (modify-frame-parameters nil '((background-color . \"red\"))) (frame-parameter nil 'background-color))",
            "\"red\"",
        ),
        (
            "(progn (modify-frame-parameters nil '((foreground-color . \"unspecified-bg\"))) (frame-parameter nil 'foreground-color))",
            "\"unspecified-bg\"",
        ),
    ]);
}

#[test]
fn deleting_the_sole_frame_and_terminal() {
    check(&[
        (
            "(delete-frame)",
            "error: (error \"Attempt to delete the sole visible or iconified frame\")",
        ),
        (
            "(delete-frame nil t)",
            "error: (error \"Attempt to delete the only frame\")",
        ),
        ("(delete-frame 'x)", "error: (wrong-type-argument framep x)"),
        (
            "(delete-terminal)",
            "error: (error \"Attempt to delete the sole active display terminal\")",
        ),
        ("(delete-terminal 'x)", "nil"),
        ("(frame-live-p (selected-frame))", "t"),
        ("(length (terminal-list))", "1"),
    ]);
}

#[test]
fn frame_visibility() {
    check(&[
        ("(eq (make-frame-visible) (selected-frame))", "t"),
        (
            "(progn (setq buffer-display-time nil) (make-frame-visible) (null buffer-display-time))",
            "nil",
        ),
        (
            "(make-frame-invisible)",
            "error: (error \"Attempt to make invisible the sole visible or iconified frame\")",
        ),
        ("(make-frame-invisible nil t)", "nil"),
        ("(iconify-frame)", "nil"),
        ("(raise-frame)", "nil"),
        ("(lower-frame)", "nil"),
        (
            "(make-frame-visible 'x)",
            "error: (wrong-type-argument frame-live-p x)",
        ),
    ]);
}

#[test]
fn frame_parameters_round_trip() {
    check(&[
        ("(modify-frame-parameters nil '((foo . 1)))", "nil"),
        ("(frame-parameter nil 'foo)", "1"),
        ("(cdr (assq 'foo (frame-parameters)))", "1"),
        ("(modify-frame-parameters nil '((name . \"Hi\")))", "nil"),
        ("(frame-parameter nil 'name)", "\"Hi\""),
        ("(modify-frame-parameters nil '((name)))", "nil"),
        ("(frame-parameter nil 'name)", "\"F2\""),
        (
            "(modify-frame-parameters nil '((minibuffer . nil)))",
            "error: (error \"Can\u{2019}t change the \u{2018}minibuffer\u{2019} parameter of this frame\")",
        ),
        ("(frame-parameter nil 'minibuffer)", "t"),
    ]);
}

#[test]
fn initial_windows() {
    check(&[
        ("(selected-window)", "#<window 1 on *scratch*>"),
        ("(eq (frame-root-window) (selected-window))", "t"),
        ("(window-live-p (frame-root-window))", "t"),
        ("(window-minibuffer-p (minibuffer-window))", "t"),
        (
            "(window-buffer (minibuffer-window))",
            "#<buffer  *Minibuf-0*>",
        ),
        ("(minibufferp (window-buffer (minibuffer-window)))", "t"),
        ("(window-total-height)", "24"),
        ("(window-total-width)", "80"),
        ("(window-total-height (minibuffer-window))", "1"),
        (
            "(window-next-sibling (frame-root-window))",
            "#<window 2 on  *Minibuf-0*>",
        ),
        ("(window-point)", "1"),
        ("(window-start)", "1"),
        (
            "(select-window (selected-window))",
            "#<window 1 on *scratch*>",
        ),
        ("(window-parameter nil 'x)", "nil"),
        ("(set-window-parameter nil 'x 3)", "3"),
        ("(window-parameters)", "((x . 3))"),
        ("(minibuffer-depth)", "0"),
        ("(window-dedicated-p)", "nil"),
        ("(frame-first-window)", "#<window 1 on *scratch*>"),
    ]);
}

#[test]
fn make_frame_without_minibuffer_and_minibuffer_frame() {
    common::on_big_stack(|| {
        let mut lisp = emacs_core::lisp::Lisp::new();
        let kb = lisp.current_kboard;
        let sf = lisp.frames.selected_frame;
        assert!(
            lisp.eval_string("(setq default-minibuffer-frame (selected-frame))")
                .is_ok()
        );
        let f = lisp
            .make_frame_without_minibuffer(
                emacs_core::value::Value::NIL,
                kb,
                emacs_core::value::Value::NIL,
            )
            .unwrap();
        let mini = lisp.frame(sf).minibuffer_window;
        assert_eq!(lisp.frame(f).minibuffer_window, mini);
        assert_eq!(
            lisp.get_frame_param(f, emacs_core::symbols::MINIBUFFER),
            mini
        );
        assert!(!lisp.window(lisp.frame(f).root_window).mini);
        let e = lisp.make_frame_without_minibuffer(mini, kb + 7, emacs_core::value::Value::NIL);
        assert!(e.is_err());
        let m = lisp.make_minibuffer_frame().unwrap();
        let fr = lisp.frame(m);
        assert_eq!(fr.minibuffer_window, fr.root_window);
        assert!(fr.flags.no_split && !fr.flags.wants_modeline && !fr.flags.auto_raise);
        assert_eq!(
            lisp.get_frame_param(m, emacs_core::symbols::MINIBUFFER),
            emacs_core::symbols::ONLY
        );
        let w = lisp.window(lisp.frame(m).root_window);
        assert!(w.mini && w.next.is_nil() && w.prev.is_nil());
        assert!(lisp.is_buffer(w.contents));
    });
}

#[test]
fn monitor_attribute_lists() {
    use emacs_core::dispextern::EmacsRectangle;
    use emacs_core::frame::MonitorInfo;
    use emacs_core::lisp::Lisp;
    use emacs_core::value::Value;
    common::on_big_stack(|| {
        let mut lisp = Lisp::new();
        let rect = |x, y, width, height| EmacsRectangle {
            x,
            y,
            width,
            height,
        };
        let monitor =
            |geom, work, mm_width, mm_height, name: Option<&str>, scale_factor| MonitorInfo {
                geom,
                work,
                mm_width,
                mm_height,
                name: name.map(str::to_string),
                scale_factor,
            };
        let monitors = [
            monitor(
                rect(0, 0, 1536, 864),
                rect(0, 0, 1536, 816),
                344,
                193,
                Some("A"),
                1.0,
            ),
            monitor(rect(1536, 0, 0, 0), rect(0, 0, 0, 0), 1, 1, Some("Z"), 1.0),
            monitor(
                rect(1536, 0, 800, 600),
                rect(1536, 0, 800, 560),
                200,
                150,
                None,
                2.0,
            ),
            monitor(
                rect(-800, 0, 800, 600),
                rect(-800, 0, 800, 600),
                0,
                0,
                Some("C"),
                1.5,
            ),
        ];
        let f = lisp.frames.selected_frame;
        let frame = lisp.prin1_to_string(f);
        let on_third = lisp.cons(f, Value::NIL);
        let frames = [Value::NIL, Value::NIL, on_third, Value::NIL];
        let list = lisp.make_monitor_attribute_list(&monitors, 2, &frames, Some("Gdk"));
        assert_eq!(
            lisp.prin1_to_string(list),
            format!(
                "(((geometry 1536 0 800 600) (workarea 1536 0 800 560) (mm-size 200 150) (scale-factor . 2.0) (frames {frame}) (source . \"Gdk\")) ((name . \"C\") (geometry -800 0 800 600) (workarea -800 0 800 600) (mm-size 0 0) (scale-factor . 1.5) (frames) (source . \"Gdk\")) ((name . \"A\") (geometry 0 0 1536 864) (workarea 0 0 1536 816) (mm-size 344 193) (scale-factor . 1.0) (frames) (source . \"Gdk\")))"
            )
        );
        let list = lisp.make_monitor_attribute_list(&monitors[..1], 5, &frames[..1], None);
        assert_eq!(
            lisp.prin1_to_string(list),
            "(((name . \"A\") (geometry 0 0 1536 864) (workarea 0 0 1536 816) (mm-size 344 193) (scale-factor . 1.0) (frames)))"
        );
    });
}
