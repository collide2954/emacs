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
fn remember_mouse_glyph_finds_the_glyph_under_the_mouse() {
    use emacs_core::dispextern::TEXT_AREA;
    use emacs_core::dispnew::matrix::WhichMatrix;
    on_big_stack(|| {
        let (mut lisp, f, w) = displayed();
        let g = lisp.frame(f).geometry.clone();
        let (cw, lh, ib) = (g.column_width, g.line_height, g.internal_border_width);
        let text_left = lisp.window_box_left(w, GlyphRowArea::Text);
        let rect = |lisp: &mut Lisp, x: i64, y: i64| {
            eval(lisp, &format!("(remember-mouse-glyph nil {x} {y})"))
        };
        assert_eq!(
            rect(&mut lisp, text_left + 2 * cw + 1, ib + lh + 2),
            format!("({} {} {cw} {lh})", text_left + 2 * cw, ib + lh)
        );
        assert_eq!(
            rect(&mut lisp, text_left + 20 * cw + 1, ib + lh + 2),
            format!("({} {} {cw} {lh})", text_left + 20 * cw, ib + lh)
        );
        let (row_x, row_y, row_height, first_width) = {
            let m = lisp.window_matrix(w, WhichMatrix::Current).unwrap();
            let row = &m.rows[m.mode_line_row()];
            (
                i64::from(row.x),
                i64::from(row.y),
                i64::from(row.height),
                i64::from(row.area_glyphs(TEXT_AREA)[0].pixel_width),
            )
        };
        assert_eq!(
            rect(&mut lisp, ib + 1, ib + row_y + 1),
            format!("({} {} {first_width} {row_height})", ib + row_x, ib + row_y)
        );
        let (sw, sh) = {
            let info = lisp.display_info(lisp.frame(f).terminal);
            (
                i64::from(info.smallest_char_width),
                i64::from(info.smallest_font_height),
            )
        };
        assert_eq!(
            rect(&mut lisp, -1, -1),
            format!("({} {} {sw} {sh})", -sw, -sh)
        );
        eval(&mut lisp, "(setq mouse-prefer-closest-glyph t)");
        assert_eq!(
            rect(&mut lisp, text_left + 3 * cw - 1, ib + lh + 2),
            format!(
                "({} {} {} {lh})",
                text_left + 2 * cw + cw / 2,
                ib + lh,
                cw / 2
            )
        );
        eval(
            &mut lisp,
            "(setq mouse-prefer-closest-glyph nil window-resize-pixelwise t)",
        );
        assert_eq!(rect(&mut lisp, 13, 17), "(13 17 1 1)");
        eval(
            &mut lisp,
            "(setq window-resize-pixelwise nil mouse-fine-grained-tracking t)",
        );
        assert_eq!(rect(&mut lisp, 13, 17), "(13 17 1 1)");
        assert_eq!(
            eval(&mut lisp, "(remember-mouse-glyph nil 'a 0)"),
            "error: (wrong-type-argument fixnump a)"
        );
    });
}

#[test]
fn window_cursor_info_describes_the_drawn_cursor() {
    on_big_stack(|| {
        let (mut lisp, f, w) = displayed();
        let cw = lisp.frame(f).geometry.column_width;
        let lh = lisp.frame(f).geometry.line_height;
        let win = lisp.window(w);
        assert!(win.flags.phys_cursor_on_p);
        assert_eq!((win.phys_cursor_width, win.phys_cursor_height), (cw, lh));
        let ascent = win.phys_cursor_ascent;
        assert_eq!(
            eval(&mut lisp, "(window-cursor-info)"),
            format!("[t 0 0 {cw} {lh} {ascent}]")
        );
        let mini = lisp.frame(f).minibuffer_window;
        let win = lisp.window(mini);
        assert_eq!(
            win.phys_cursor_type,
            emacs_core::dispextern::TextCursorKind::NoCursor
        );
        assert_eq!(
            eval(&mut lisp, "(window-cursor-info (minibuffer-window))"),
            format!("[t 0 0 0 {lh} 0]")
        );
        lisp.window_mut(mini).flags.phys_cursor_on_p = false;
        assert_eq!(
            eval(&mut lisp, "(window-cursor-info (minibuffer-window))"),
            "nil"
        );
    });
}

#[test]
fn window_end_and_window_lines_pixel_dimensions() {
    use emacs_core::dispnew::matrix::WhichMatrix;
    on_big_stack(|| {
        let (mut lisp, f, w) = displayed();
        let cw = lisp.frame(f).geometry.column_width;
        let lh = lisp.frame(f).geometry.line_height;
        let box_height = lisp.window_box_height(w);
        let rows = (box_height + lh - 1) / lh;
        let end = 1 + 7 * rows;
        assert_eq!(eval(&mut lisp, "(window-end)"), end.to_string());
        assert_eq!(eval(&mut lisp, "(window-end nil t)"), end.to_string());
        let (pixel_width, pixel_height) = (lisp.window(w).pixel_width, lisp.window(w).pixel_height);
        let text_bottom = lisp.window_text_bottom_y(w);
        let body_width = lisp
            .window_body_width(w, emacs_core::window::WindowBodyUnit::Pixels)
            .unwrap();
        let nrows = lisp.window_matrix(w, WhichMatrix::Current).unwrap().nrows;
        let dims = |range: std::ops::Range<i64>, max_y: i64, x: &dyn Fn(i64) -> i64| {
            let items: Vec<String> = range
                .filter(|r| (r + 1) * lh < max_y)
                .map(|r| format!("({} . {})", x(r), (r + 1) * lh))
                .collect();
            if items.is_empty() {
                "nil".to_string()
            } else {
                format!("({})", items.join(" "))
            }
        };
        assert_eq!(
            eval(&mut lisp, "(window-lines-pixel-dimensions)"),
            dims(0..rows, pixel_height, &|_| 7 * cw)
        );
        assert_eq!(
            eval(&mut lisp, "(window-lines-pixel-dimensions nil nil nil t)"),
            dims(0..rows, text_bottom, &|_| 7 * cw)
        );
        assert_eq!(
            eval(&mut lisp, "(window-lines-pixel-dimensions nil 1 2)"),
            dims(1..3, pixel_height, &|_| 7 * cw)
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(window-lines-pixel-dimensions nil nil nil nil t)"
            ),
            dims(0..rows, pixel_height, &|_| pixel_width - 7 * cw)
        );
        assert_eq!(
            eval(&mut lisp, "(window-lines-pixel-dimensions nil 0 0 t t)"),
            dims(0..1, text_bottom, &|_| body_width - 7 * cw)
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(window-lines-pixel-dimensions nil 0 0 nil nil t)"
            ),
            dims(0..1, pixel_height, &|_| pixel_width - cw)
        );
        assert_eq!(
            eval(&mut lisp, "(window-lines-pixel-dimensions nil 0 0 nil t t)"),
            dims(0..1, pixel_height, &|_| cw)
        );
        assert_eq!(
            eval(&mut lisp, "(window-lines-pixel-dimensions nil -1)"),
            format!("error: (args-out-of-range -1 0 {nrows})")
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(window-lines-pixel-dimensions (minibuffer-window))"
            ),
            "nil"
        );
        eval(&mut lisp, "(progn (goto-char 1) (insert \"ab\\n\"))");
        assert_eq!(eval(&mut lisp, "(window-lines-pixel-dimensions)"), "nil");
        assert_eq!(eval(&mut lisp, "(window-end)"), (end + 3).to_string());
        assert_eq!(eval(&mut lisp, "(window-end nil t)"), (end - 4).to_string());
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
