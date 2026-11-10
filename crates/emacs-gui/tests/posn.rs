use std::sync::mpsc;

use emacs_core::dispextern::GlyphRowArea;
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

fn displayed() -> (Lisp, Value, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"p\") (width . 30) (height . 8)))) (select-frame f) (set-window-buffer nil (get-buffer-create \"text\")) (set-buffer \"text\") (setq bidi-paragraph-direction 'left-to-right) (setq mode-line-format \"%b\") (let ((i 0)) (while (< i 20) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    let w = lisp.frame(f).root_window;
    lisp.frame_mut(f).flags.visible = true;
    lisp.redisplay().unwrap();
    (lisp, f, w)
}

#[test]
fn posn_at_point_describes_the_glyph_at_a_position() {
    on_big_stack(|| {
        let (mut lisp, f, w) = displayed();
        let window = lisp.prin1_to_string(w);
        let cw = lisp.frame(f).geometry.column_width;
        let lh = lisp.frame(f).geometry.line_height;
        assert_eq!(
            eval(&mut lisp, "(posn-at-point)"),
            format!("({window} 1 (0 . 0) 0 nil 1 (0 . 0) nil (0 . 0) ({cw} . {lh}))")
        );
        assert_eq!(
            eval(&mut lisp, "(posn-at-point 11)"),
            format!(
                "({window} 11 ({} . {lh}) 0 nil 11 (3 . 1) nil (0 . 0) ({cw} . {lh}))",
                3 * cw
            )
        );
        assert_eq!(eval(&mut lisp, "(posn-at-point 140)"), "nil");
        assert_eq!(
            eval(&mut lisp, "(posn-at-point 1 'x)"),
            "error: (wrong-type-argument window-live-p x)"
        );
    });
}

#[test]
fn posn_at_x_y_finds_text_and_the_mode_line() {
    on_big_stack(|| {
        let (mut lisp, f, w) = displayed();
        let window = lisp.prin1_to_string(w);
        let cw = lisp.frame(f).geometry.column_width;
        let lh = lisp.frame(f).geometry.line_height;
        assert_eq!(
            eval(
                &mut lisp,
                &format!("(posn-at-x-y {} {})", 2 * cw + 1, lh + 2)
            ),
            format!(
                "({window} 10 ({} . {}) 0 nil 10 (2 . 1) nil (1 . 2) ({cw} . {lh}))",
                2 * cw + 1,
                lh + 2
            )
        );
        let bottom = lisp.window_text_bottom_y(w);
        let ml = eval(&mut lisp, &format!("(posn-at-x-y 1 {})", bottom + 1));
        assert!(ml.starts_with(&format!("({window} mode-line (")), "{ml}");
        assert_eq!(
            eval(&mut lisp, "(posn-at-x-y -2 0)"),
            "error: (wrong-type-argument wholenump -2)"
        );
    });
}

#[test]
fn coordinates_in_window_p_and_window_at() {
    on_big_stack(|| {
        let (mut lisp, f, w) = displayed();
        let g = lisp.frame(f).geometry.clone();
        let text_left = lisp.window_box_left(w, GlyphRowArea::Text);
        let x = 2 * g.column_width + g.internal_border_width - text_left;
        let expected_x = if x % g.column_width == 0 {
            (x / g.column_width).to_string()
        } else {
            let v = lisp.float(x as f64 / g.column_width as f64);
            lisp.prin1_to_string(v)
        };
        assert_eq!(
            eval(&mut lisp, "(coordinates-in-window-p '(2 . 1) nil)"),
            format!("({expected_x} . 1)")
        );
        let mode_row = lisp.window(w).total_lines - 1;
        assert_eq!(
            eval(
                &mut lisp,
                &format!("(coordinates-in-window-p '(3 . {mode_row}) nil)")
            ),
            "mode-line"
        );
        assert_eq!(
            eval(&mut lisp, "(coordinates-in-window-p '(0 . 1) nil)"),
            "left-fringe"
        );
        assert_eq!(
            eval(&mut lisp, "(coordinates-in-window-p '(500 . 1) nil)"),
            "nil"
        );
        assert_eq!(
            eval(&mut lisp, "(eq (window-at 2 1) (selected-window))"),
            "t"
        );
        let mini = lisp.frame(f).minibuffer_window;
        let mini_row = lisp.window(mini).pixel_top / g.line_height;
        assert_eq!(
            eval(
                &mut lisp,
                &format!("(eq (window-at 2 {mini_row}) (minibuffer-window))")
            ),
            "t"
        );
        assert_eq!(eval(&mut lisp, "(window-at -1 1)"), "nil");
    });
}
