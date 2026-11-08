use std::sync::mpsc;

use emacs_core::dispextern::{GlyphU, TEXT_AREA, TextPos};
use emacs_core::dispnew::matrix::WhichMatrix;
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

fn frame_showing(setup: &str) -> (Lisp, Value, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"m\") (width . 30) (height . 8)))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    let w = lisp.frame(f).root_window;
    assert_eq!(
        eval(
            &mut lisp,
            &format!(
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"modes\")) (set-buffer \"modes\") {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    let fr = lisp.frame_mut(f);
    fr.fonts_changed = false;
    fr.flags.visible = true;
    fr.flags.garbaged = false;
    (lisp, f, w)
}

#[test]
fn format_mode_line_without_properties() {
    on_big_stack(|| {
        let (mut lisp, _, _) = frame_showing("(insert \"abc\")");
        for (form, expected) in [
            ("(format-mode-line \"%b\" 0)", "\"modes\""),
            ("(format-mode-line '(\"a\" (\"b\" \"c\")) 0)", "\"abc\""),
            ("(format-mode-line '(6 \"x\") 0)", "\"x     \""),
            ("(format-mode-line '(-2 \"xyz\") 0)", "\"xy\""),
            (
                "(format-mode-line '(:eval (concat \"e\" \"v\")) 0)",
                "\"ev\"",
            ),
            ("(format-mode-line \"%*%+%&\" 0)", "\"***\""),
            (
                "(progn (setq nope nil) (format-mode-line '(nope \"yes\" \"no\") 0))",
                "\"no\"",
            ),
            (
                "(progn (setq yep t) (format-mode-line '(yep \"yes\" \"no\") 0))",
                "\"yes\"",
            ),
            ("(format-mode-line \"%5i|%i\" 0)", "\"    3|3\""),
            ("(format-mode-line \"[%[%]]\" 0)", "\"[]\""),
            ("(format-mode-line \"%n\" 0)", "\"\""),
            (
                "(progn (narrow-to-region 1 2) (prog1 (format-mode-line \"%n\" 0) (widen)))",
                "\" Narrow\"",
            ),
            ("(format-mode-line \"100%%\" 0)", "\"100%\""),
            ("(format-mode-line 'mode-line-format 0)", "\"%-\""),
            (
                "(format-mode-line (default-value 'mode-line-format) 0)",
                "\"--\"",
            ),
            (
                "(progn (put 'display-table 'char-table-extra-slots 6) (let ((dt (make-char-table 'display-table))) (aset dt ?a (vector 1)) (setq buffer-display-table dt)) (prog1 (format-mode-line '(-1 \"ab\") 0) (setq buffer-display-table nil)))",
                "\"a\"",
            ),
        ] {
            assert_eq!(eval(&mut lisp, form), expected, "{form}");
        }
    });
}

#[test]
fn format_mode_line_coding_systems() {
    on_big_stack(|| {
        let (mut lisp, _, _) = frame_showing("");
        for (form, expected) in [
            ("(format-mode-line \"%z\" 0)", "\"-\""),
            ("(format-mode-line \"%Z\" 0)", "\"-:\""),
            (
                "(progn (setq buffer-file-coding-system 'no-conversion) (format-mode-line \"%Z\" 0))",
                "\"=:\"",
            ),
            (
                "(format-mode-line \"%z\" 0 (frame-root-window (car (delq f (frame-list)))))",
                "\"=--\"",
            ),
        ] {
            assert_eq!(eval(&mut lisp, form), expected, "{form}");
        }
    });
}

#[test]
fn format_mode_line_with_a_face() {
    on_big_stack(|| {
        let (mut lisp, _, _) = frame_showing("");
        assert_eq!(
            eval(
                &mut lisp,
                "(get-text-property 0 'face (format-mode-line \"x\" 'mode-line-inactive))"
            ),
            "mode-line-inactive"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(format-mode-line '(:propertize \"pq\" face bold) nil)"
            ),
            "#(\"pq\" 0 2 (mode-line-elt-no 2 face bold))"
        );
    });
}

#[test]
fn display_mode_lines_builds_the_mode_line_row() {
    on_big_stack(|| {
        let (mut lisp, f, w) = frame_showing(
            "(setq mode-line-format \"%b L%l\") (insert \"a\\nb\\nc\") (goto-char 5)",
        );
        assert!(lisp.window_wants_mode_line(w));
        assert_eq!(lisp.try_window(w, TextPos::new(1, 1), 0).unwrap(), 1);
        assert_eq!(lisp.display_mode_lines(w).unwrap(), 1);
        let m = lisp.window_matrix(w, WhichMatrix::Desired).unwrap();
        let row = &m.rows[m.mode_line_row()];
        assert!(row.enabled_p && row.mode_line_p && row.full_width_p);
        let text: String = row
            .area_glyphs(TEXT_AREA)
            .iter()
            .filter_map(|g| match g.u {
                GlyphU::Ch(c) => char::from_u32(c),
                _ => None,
            })
            .collect();
        assert!(text.starts_with("modes L3"), "{text:?}");
        assert!(lisp.window(w).flags.must_be_updated_p);
        lisp.update_frame(f, false).unwrap();
        let m = lisp.window_matrix(w, WhichMatrix::Current).unwrap();
        assert!(m.rows[m.mode_line_row()].enabled_p);
    });
}
