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

fn frame() -> (Lisp, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"t\") (width . 30) (height . 8)))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.frame_mut(f).flags.visible = true;
    (lisp, f)
}

fn show(lisp: &mut Lisp, insert_args: &str) {
    assert_eq!(
        eval(
            lisp,
            &format!(
                "(progn (set-buffer (get-buffer-create \"probe\")) (setq bidi-paragraph-direction 'left-to-right mode-line-format \"%b\" header-line-format nil truncate-lines nil) (erase-buffer) (insert {insert_args}) (goto-char 1) (set-window-buffer nil (current-buffer)) (bufferp (current-buffer)))"
            )
        ),
        "t"
    );
}

#[test]
fn window_text_pixel_size_measures_the_window_buffer() {
    on_big_stack(|| {
        let (mut lisp, f) = frame();
        let (c, l) = {
            let g = &lisp.frame(f).geometry;
            (g.column_width, g.line_height)
        };
        show(
            &mut lisp,
            "(let ((s \"\") (i 0)) (while (< i 20) (setq s (concat s (format \"line %d\\n\" i)) i (1+ i))) s)",
        );
        assert_eq!(eval(&mut lisp, "(line-pixel-height)"), l.to_string());
        let mlh: i64 = eval(&mut lisp, "(window-mode-line-height)")
            .parse()
            .unwrap();
        let cases = [
            (
                "(window-text-pixel-size)",
                format!("({} . {})", 7 * c, 20 * l),
            ),
            (
                "(window-text-pixel-size nil 1 8)",
                format!("({} . {l})", 6 * c),
            ),
            (
                "(window-text-pixel-size nil 1 7)",
                format!("({} . {l})", 6 * c),
            ),
            (
                "(window-text-pixel-size nil 3 5)",
                format!("({} . {l})", 2 * c),
            ),
            (
                "(window-text-pixel-size nil 3 10)",
                format!("({} . {})", 6 * c, 2 * l),
            ),
            ("(window-text-pixel-size nil 8 8)", "(0 . 0)".to_string()),
            (
                "(window-text-pixel-size nil 1 8 nil nil nil t)",
                format!("({} . {l})", 6 * c),
            ),
            (
                "(window-text-pixel-size nil 1 9 nil nil nil t)",
                format!("({} . {l})", 6 * c),
            ),
            (
                &format!("(window-text-pixel-size nil nil nil {})", 2 * c),
                format!("({} . {})", 2 * c, 70 * l),
            ),
            (
                "(window-text-pixel-size nil nil nil t)",
                format!("({} . {})", 7 * c, 20 * l),
            ),
            (
                &format!("(window-text-pixel-size nil nil nil nil {})", l + l / 2),
                format!("({} . {})", 6 * c, l + l / 2),
            ),
            (
                "(window-text-pixel-size nil nil nil nil nil t)",
                format!("({} . {})", 7 * c, 20 * l + mlh),
            ),
            (
                "(window-text-pixel-size nil nil nil nil nil 'mode-line)",
                format!("({} . {})", 7 * c, 20 * l + mlh),
            ),
            (
                "(window-text-pixel-size nil nil nil nil nil 'header-line)",
                format!("({} . {})", 7 * c, 20 * l),
            ),
            (
                "(window-text-pixel-size nil '(15 . 0) 30)",
                format!("({} . {})", 6 * c, 3 * l),
            ),
            (
                &format!("(window-text-pixel-size nil '(15 . {}) 60)", 2 * l),
                format!("({} {} 29)", 6 * c, 5 * l),
            ),
            (
                &format!("(window-text-pixel-size nil '(60 . {}) 80)", -2 * l),
                format!("({} {} 43)", 7 * c, 6 * l),
            ),
            ("(window-text-pixel-size nil 300)", "(0 . 0)".to_string()),
            ("(window-text-pixel-size nil 5 2)", format!("(0 . {l})")),
            (
                "(window-text-pixel-size nil '(1 . x))",
                "error: (wrong-type-argument fixnump x)".to_string(),
            ),
            (
                "(window-text-pixel-size 'x)",
                "error: (wrong-type-argument window-live-p x)".to_string(),
            ),
        ];
        for (source, expected) in &cases {
            assert_eq!(&eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn window_text_pixel_size_skips_whitespace_and_display_strings() {
    on_big_stack(|| {
        let (mut lisp, f) = frame();
        let (c, l) = {
            let g = &lisp.frame(f).geometry;
            (g.column_width, g.line_height)
        };
        show(&mut lisp, "\"  \\n  abc  \\n\\n  de \\n\\n\"");
        for (source, expected) in [
            (
                "(window-text-pixel-size nil t t)",
                format!("({} . {})", 7 * c, 4 * l),
            ),
            (
                "(window-text-pixel-size nil t nil)",
                format!("({} . {})", 7 * c, 5 * l),
            ),
            (
                "(window-text-pixel-size nil nil t)",
                format!("({} . {})", 7 * c, 4 * l),
            ),
            (
                "(window-text-pixel-size nil 4 9)",
                format!("({} . {l})", 5 * c),
            ),
            (
                "(window-text-pixel-size nil t t nil nil nil t)",
                format!("({} . {})", 7 * c, 4 * l),
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        show(
            &mut lisp,
            "\"ab\" (propertize \"cd\" 'display \"WXYZ\") \"ef\"",
        );
        for (source, expected) in [
            (
                "(window-text-pixel-size nil 1 4)",
                format!("({} . {l})", 6 * c),
            ),
            (
                "(window-text-pixel-size nil 1 3)",
                format!("({} . {l})", 2 * c),
            ),
            (
                "(window-text-pixel-size nil 1 5)",
                format!("({} . {l})", 6 * c),
            ),
            (
                "(window-text-pixel-size nil 4 7)",
                format!("({} . {l})", 2 * c),
            ),
            ("(window-text-pixel-size)", format!("({} . {l})", 8 * c)),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        show(
            &mut lisp,
            "\"abc\\n\" (propertize \"de\" 'display \"WXYZ\") \"\\nf\"",
        );
        for (source, expected) in [
            (
                "(window-text-pixel-size nil 1 6)",
                format!("({} . {})", 4 * c, 2 * l),
            ),
            (
                "(window-text-pixel-size nil 1 6 nil nil nil t)",
                format!("({} . {l})", 4 * c),
            ),
            (
                "(window-text-pixel-size nil 1 7 nil nil nil t)",
                format!("({} . {l})", 4 * c),
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn window_text_pixel_size_adds_header_and_mode_lines() {
    on_big_stack(|| {
        let (mut lisp, f) = frame();
        let (c, l) = {
            let g = &lisp.frame(f).geometry;
            (g.column_width, g.line_height)
        };
        show(&mut lisp, "\"line 0\\nline 1\\nline 2\\n\"");
        eval(&mut lisp, "(setq header-line-format \"H\")");
        let mlh: i64 = eval(&mut lisp, "(window-mode-line-height)")
            .parse()
            .unwrap();
        let hlh: i64 = eval(&mut lisp, "(window-header-line-height)")
            .parse()
            .unwrap();
        assert!(hlh > 0);
        for (source, expected) in [
            (
                "(window-text-pixel-size nil nil nil nil nil 'header-line)",
                format!("({} . {})", 6 * c, 3 * l + hlh),
            ),
            (
                "(window-text-pixel-size nil nil nil nil nil t)",
                format!("({} . {})", 6 * c, 3 * l + hlh + mlh),
            ),
            (
                "(window-text-pixel-size)",
                format!("({} . {})", 6 * c, 3 * l),
            ),
            (
                "(window-text-pixel-size nil '(8 . 0) nil)",
                format!("({} . {})", 6 * c, 2 * l),
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn buffer_text_pixel_size_measures_other_buffers() {
    on_big_stack(|| {
        let (mut lisp, f) = frame();
        let (c, l) = {
            let g = &lisp.frame(f).geometry;
            (g.column_width, g.line_height)
        };
        show(&mut lisp, "\"line 0\\nline 1\\nline 2\\n\"");
        let body: i64 = eval(&mut lisp, "(window-body-width nil t)")
            .parse()
            .unwrap();
        assert_eq!(body, 30 * c);
        let window = eval(&mut lisp, "(selected-window)");
        let temp = |contents: &str, call: &str| {
            format!(
                "(let ((b (get-buffer-create \"temp\"))) (save-current-buffer (set-buffer b) (setq truncate-lines nil) (erase-buffer) (insert {contents}) {call}))"
            )
        };
        let cases = [
            (
                temp("\"abcd\\nef\"", "(buffer-text-pixel-size)"),
                format!("({} . {})", 4 * c, 2 * l),
            ),
            (
                "(buffer-text-pixel-size \"probe\")".to_string(),
                format!("({} . {})", 6 * c, 3 * l),
            ),
            (
                format!("(buffer-text-pixel-size \"probe\" nil {})", c + c / 2),
                format!("({} . {})", c + c / 2, 18 * l),
            ),
            (
                format!("(buffer-text-pixel-size \"probe\" nil nil {})", l + l / 4),
                format!("({} . {})", 6 * c, l + l / 4),
            ),
            (
                temp(
                    "\"  \\n  abc  \\n\\n  de \\n\\n\"",
                    "(buffer-text-pixel-size nil nil t t)",
                ),
                format!("({} . 0)", 2 * c),
            ),
            (
                temp(
                    "\"  \\n  abc  \\n\\n  de \\n\\n\"",
                    "(buffer-text-pixel-size nil nil t nil)",
                ),
                format!("({} . {})", 7 * c, 5 * l),
            ),
            (
                temp(
                    "(make-string 100 ?x)",
                    "(progn (setq truncate-lines t) (buffer-text-pixel-size))",
                ),
                format!("({body} . {l})"),
            ),
            (
                temp(
                    "(make-string 100 ?x)",
                    "(progn (setq truncate-lines t) (buffer-text-pixel-size nil nil t))",
                ),
                format!("({} . {l})", 100 * c),
            ),
            (
                temp(
                    "(make-string 100 ?x)",
                    &format!(
                        "(progn (setq truncate-lines t) (buffer-text-pixel-size nil nil {}))",
                        5 * c
                    ),
                ),
                format!("({} . {l})", 5 * c),
            ),
            (
                temp("(make-string 100 ?x)", "(buffer-text-pixel-size)"),
                format!("({body} . {})", 4 * l),
            ),
            (
                temp("(make-string 100 ?x)", "(buffer-text-pixel-size nil nil t)"),
                format!("({} . {l})", 100 * c),
            ),
            (
                temp("\"\"", "(buffer-text-pixel-size)"),
                "(0 . 0)".to_string(),
            ),
            (
                temp("\"ab\"", "(buffer-text-pixel-size)"),
                format!("({} . {l})", 2 * c),
            ),
            (
                temp("\"ab\"", "(buffer-text-pixel-size nil nil nil 0)"),
                "(0 . 0)".to_string(),
            ),
            (
                "(buffer-text-pixel-size \"no such buffer\")".to_string(),
                "error: (wrong-type-argument bufferp nil)".to_string(),
            ),
        ];
        for (source, expected) in &cases {
            assert_eq!(&eval(&mut lisp, source), expected, "evaluating {source}");
        }
        assert_eq!(eval(&mut lisp, "(selected-window)"), window);
        assert_eq!(
            eval(&mut lisp, "(buffer-name (window-buffer))"),
            "\"probe\""
        );
    });
}
