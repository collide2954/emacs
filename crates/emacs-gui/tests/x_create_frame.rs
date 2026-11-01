use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::xfaces::DEFAULT_FACE_ID;
use emacs_gui::display::Display;
use emacs_gui::protocol::Command;
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

fn connect(lisp: &mut Lisp) {
    lisp.init_display();
    eval(
        lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    assert_eq!(
        eval(
            lisp,
            "(progn (setq x-resource-name \"emacs\") (x-open-connection nil) nil)"
        ),
        "nil"
    );
}

#[test]
fn window_fringes_scroll_bars_and_margins_on_a_pgtk_frame() {
    on_big_stack(|| {
        let mut lisp = Lisp::boot(Startup::program(false));
        emacs_gui::install(
            &mut lisp,
            Display::new(fmpsc::unbounded().0, mpsc::channel().1),
        );
        connect(&mut lisp);
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq f (x-create-frame '((name . \"w\") (width . 40) (height . 10))) r (frame-root-window f)) (window-live-p r))"
            ),
            "t"
        );
        for (source, expected) in [
            ("(set-window-fringes r 3 4 t t)", "t"),
            ("(window-fringes r)", "(3 4 t t)"),
            ("(set-window-fringes r 3 4 t t)", "nil"),
            ("(set-window-fringes r 100000)", "t"),
            ("(window-fringes r)", "(3 4 nil t)"),
            ("(set-window-fringes r 100000 nil nil)", "nil"),
            (
                "(set-window-fringes r -1)",
                "error: (args-out-of-range -1 0 2147483647)",
            ),
            ("(set-window-fringes r nil nil)", "t"),
            ("(nthcdr 2 (window-fringes r))", "(nil nil)"),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        let symbol = lisp.intern("f");
        let f = lisp.find_symbol_value(symbol);
        let geometry = &lisp.frame(f).geometry;
        let fringes = format!(
            "({} {})",
            geometry.left_fringe_width, geometry.right_fringe_width
        );
        assert!(geometry.left_fringe_width > 0);
        assert_eq!(
            eval(
                &mut lisp,
                "(list (car (window-fringes r)) (nth 1 (window-fringes r)))"
            ),
            fringes
        );
        for (source, expected) in [
            ("(set-window-scroll-bars r 12 'left nil nil t)", "t"),
            (
                "(equal (window-scroll-bars r) (list 12 (/ (+ 12 (frame-char-width f) -1) (frame-char-width f)) 'left nil 0 nil t))",
                "t",
            ),
            ("(set-window-scroll-bars r 12 'left nil nil t)", "nil"),
            (
                "(set-window-scroll-bars r 12 'bogus)",
                "error: (error \"Invalid type of vertical scroll bar\")",
            ),
            ("(set-window-scroll-bars r 0 'bogus)", "t"),
            ("(window-scroll-bars r)", "(0 0 nil nil 0 nil nil)"),
            ("(set-window-scroll-bars r 100000 'right)", "nil"),
            ("(window-scroll-bars r)", "(0 0 nil nil 0 nil nil)"),
            ("(set-window-margins r 1 2)", "t"),
            ("(window-margins r)", "(1 . 2)"),
            (
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-fringes r 2 2 nil t) (set-window-scroll-bars r 7 'right nil nil nil) (save-current-buffer (set-buffer (get-buffer-create \"fr\")) (setq left-fringe-width 5 scroll-bar-width 9 vertical-scroll-bar 'left)) (set-window-buffer r \"fr\") (list (window-fringes r) (car (window-scroll-bars r)) (nth 2 (window-scroll-bars r)) (window-margins r)))",
                "((2 2 nil t) 9 left (nil))",
            ),
            (
                "(list (set-window-fringes r 2 2 nil nil) (progn (set-window-buffer r \"fr\") (car (window-fringes r))) (nth 3 (window-fringes r)))",
                "(nil 5 nil)",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn pgtk_register_dnd_targets_sets_the_frame_drop_targets() {
    on_big_stack(|| {
        let mut lisp = Lisp::boot(Startup::program(false));
        emacs_gui::install(
            &mut lisp,
            Display::new(fmpsc::unbounded().0, mpsc::channel().1),
        );
        connect(&mut lisp);
        assert_eq!(
            eval(&mut lisp, "(pgtk-register-dnd-targets nil '(\"STRING\"))"),
            "error: (error \"Window system frame should be used\")"
        );
        eval(
            &mut lisp,
            "(setq f (x-create-frame '((name . \"d\") (width . 20) (height . 5))))",
        );
        for (source, expected) in [
            (
                "(pgtk-register-dnd-targets f '(\"text/uri-list\" \"STRING\"))",
                "nil",
            ),
            (
                "(pgtk-register-dnd-targets f 'x)",
                "error: (wrong-type-argument listp x)",
            ),
            (
                "(pgtk-register-dnd-targets f '(\"a\" . \"b\"))",
                "error: (wrong-type-argument listp \"b\")",
            ),
            (
                "(pgtk-register-dnd-targets f '(\"a\" 1))",
                "error: (wrong-type-argument stringp 1)",
            ),
            (
                "(pgtk-register-dnd-targets 'g nil)",
                "error: (wrong-type-argument frame-live-p g)",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        let symbol = lisp.intern("f");
        let f = lisp.find_symbol_value(symbol);
        assert_eq!(
            emacs_gui::pgtkterm::with_output(&lisp, f, |output| output.drag_dest_targets.clone()),
            Some(vec![b"text/uri-list".to_vec(), b"STRING".to_vec()])
        );
        eval(&mut lisp, "(pgtk-register-dnd-targets f nil)");
        assert_eq!(
            emacs_gui::pgtkterm::with_output(&lisp, f, |output| output.drag_dest_targets.clone()),
            Some(Vec::new())
        );
    });
}

#[test]
fn x_create_frame_makes_a_visible_pgtk_frame() {
    on_big_stack(|| {
        let mut lisp = Lisp::boot(Startup::program(false));
        let (commands, mut command_rx) = fmpsc::unbounded();
        let (_notice_tx, notices) = mpsc::channel();
        emacs_gui::install(&mut lisp, Display::new(commands, notices));
        connect(&mut lisp);
        let frames_before = eval(&mut lisp, "(length (frame-list))");
        assert_eq!(
            eval(
                &mut lisp,
                "(framep (setq f (x-create-frame '((name . \"test\") (width . 40) (height . 10)))))"
            ),
            "pgtk"
        );
        for (source, expected) in [
            ("(frame-live-p f)", "pgtk"),
            ("(window-system f)", "pgtk"),
            ("(and (memq f (frame-list)) t)", "t"),
            (
                "(length (frame-list))",
                &(frames_before.parse::<i64>().unwrap() + 1).to_string(),
            ),
            ("(frame-parameter f 'name)", "\"test\""),
            ("(frame-parameter f 'font-backend)", "(ftcr)"),
            ("(stringp (frame-parameter f 'font))", "t"),
            ("(frame-parameter f 'foreground-color)", "\"black\""),
            ("(frame-parameter f 'background-color)", "\"white\""),
            ("(frame-parameter f 'visibility)", "nil"),
            ("(frame-text-cols f)", "40"),
            ("(frame-text-lines f)", "10"),
            ("(frame-parameter f 'minibuffer)", "t"),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        let symbol = lisp.intern("f");
        let f = lisp.find_symbol_value(symbol);
        let face = lisp.face_from_id(f, DEFAULT_FACE_ID).unwrap();
        assert_eq!((face.foreground, face.background), (0x000000, 0xFFFFFF));
        assert_eq!(face.font, lisp.frame(f).font);
        let (width, height) = (
            lisp.frame(f).geometry.pixel_width,
            lisp.frame(f).geometry.pixel_height,
        );
        let mut opened = None;
        while let Ok(command) = command_rx.try_recv() {
            if let Command::OpenFrame {
                frame,
                title,
                width,
                height,
            } = command
            {
                opened = Some((frame, title, width, height));
            }
        }
        assert_eq!(
            opened,
            Some((
                lisp.frame(f).id as u64,
                "test".to_string(),
                width as u32,
                height as u32
            ))
        );
        assert_eq!(
            eval(&mut lisp, "(x-create-frame '((name . 5)))"),
            "error: (error \"Invalid frame name--not a string or nil\")"
        );
    });
}
