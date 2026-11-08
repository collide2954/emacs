use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
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

fn frame_showing(setup: &str) -> Lisp {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"p\") (width . 30) (height . 8)))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    assert_eq!(
        eval(
            &mut lisp,
            &format!(
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"lines\")) (set-buffer \"lines\") (setq bidi-paragraph-direction 'left-to-right) {setup} nil)"
            )
        ),
        "nil"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.adjust_frame_glyphs(f);
    let fr = lisp.frame_mut(f);
    fr.fonts_changed = false;
    fr.flags.visible = true;
    fr.flags.garbaged = false;
    lisp
}

const TWENTY_LINES: &str = "(let ((i 0)) (while (< i 20) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1)";

#[test]
fn positions_in_and_out_of_the_window() {
    on_big_stack(|| {
        let mut lisp = frame_showing(TWENTY_LINES);
        for (form, expected) in [
            ("(pos-visible-in-window-p 1)", "t"),
            ("(pos-visible-in-window-p (point-max))", "nil"),
            ("(pos-visible-in-window-p 1 nil t)", "(0 0)"),
            (
                "(equal (pos-visible-in-window-p 10 nil t) (list (* 2 (frame-char-width)) (frame-char-height)))",
                "t",
            ),
            (
                "(progn (set-window-start nil 8) (pos-visible-in-window-p 1))",
                "nil",
            ),
            (
                "(progn (set-window-start nil 1) (pos-visible-in-window-p))",
                "t",
            ),
        ] {
            assert_eq!(eval(&mut lisp, form), expected, "{form}");
        }
    });
}

#[test]
fn the_last_visible_line() {
    on_big_stack(|| {
        let mut lisp = frame_showing(TWENTY_LINES);
        let text_lines = eval(
            &mut lisp,
            "(/ (window-body-height nil t) (frame-char-height))",
        );
        let n: i64 = text_lines.parse().expect("integer");
        let last_line_start = 1 + 7 * (n - 1);
        assert_eq!(
            eval(
                &mut lisp,
                &format!("(pos-visible-in-window-p {last_line_start})")
            ),
            "t"
        );
        assert_eq!(
            eval(
                &mut lisp,
                &format!("(pos-visible-in-window-p {})", last_line_start + 7 * 2)
            ),
            "nil"
        );
    });
}
