use std::sync::mpsc;

use emacs_core::dispextern::{EmacsCursorKind, GlyphRowArea, TEXT_AREA};
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

fn displayed(extra: &str) -> (Lisp, Value, Value) {
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
            &format!(
                "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"h\") (width . 30) (height . 8)))) (select-frame f) (set-window-buffer nil (get-buffer-create \"text\")) (set-buffer \"text\") (setq bidi-paragraph-direction 'left-to-right) (setq mode-line-format \"%b\") (let ((i 0)) (while (< i 20) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1) {extra} (framep f))"
            )
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

fn cursor_of(lisp: &Lisp, f: Value) -> Option<EmacsCursorKind> {
    emacs_gui::pgtkterm::with_output(lisp, f, |o| o.current_cursor)
}

fn spot(lisp: &mut Lisp, f: Value, w: Value, row: i64, col: i64) -> (i64, i64) {
    let g = lisp.frame(f).geometry.clone();
    (
        lisp.window_box_left(w, GlyphRowArea::Text) + col * g.column_width + 1,
        g.internal_border_width + row * g.line_height + 2,
    )
}

fn hl(lisp: &Lisp, f: Value) -> (Value, i32, i32, i32, i32, i32) {
    let h = lisp.mouse_hl_info(f);
    (
        h.mouse_face_window,
        h.mouse_face_beg_row,
        h.mouse_face_beg_col,
        h.mouse_face_end_row,
        h.mouse_face_end_col,
        h.mouse_face_end_x,
    )
}

#[test]
fn mouse_face_text_properties_highlight_and_set_help_echo() {
    on_big_stack(|| {
        let (mut lisp, f, w) = displayed(
            "(put-text-property 8 14 'mouse-face 'highlight) (put-text-property 8 14 'help-echo \"hello\")",
        );
        let cw = i32::try_from(lisp.frame(f).geometry.column_width).unwrap();
        let buffer = lisp.window(w).contents;
        let (x, y) = spot(&mut lisp, f, w, 1, 2);
        lisp.note_mouse_highlight(f, x, y).unwrap();
        assert_eq!(hl(&lisp, f), (w, 1, 0, 1, 6, 6 * cw));
        let help = lisp.xdisp.help_echo_string;
        assert_eq!(lisp.prin1_to_string(help), "\"hello\"");
        assert_eq!(lisp.xdisp.help_echo_window, w);
        assert_eq!(lisp.xdisp.help_echo_object, buffer);
        assert_eq!(lisp.xdisp.help_echo_pos, 10);
        assert_eq!(cursor_of(&lisp, f), Some(EmacsCursorKind::Hand));
        let mouse_face_p =
            lisp.window_matrix(w, WhichMatrix::Current).unwrap().rows[1].mouse_face_p;
        assert!(mouse_face_p);
        let (x, y) = spot(&mut lisp, f, w, 2, 2);
        lisp.note_mouse_highlight(f, x, y).unwrap();
        assert_eq!(hl(&lisp, f).0, Value::NIL);
        assert_eq!(lisp.xdisp.help_echo_string, Value::NIL);
        assert_eq!(cursor_of(&lisp, f), Some(EmacsCursorKind::Text));
        eval(&mut lisp, "(setq mouse-highlight nil)");
        let (x, y) = spot(&mut lisp, f, w, 1, 3);
        lisp.note_mouse_highlight(f, x, y).unwrap();
        assert_eq!(hl(&lisp, f).0, Value::NIL);
        let help = lisp.xdisp.help_echo_string;
        assert_eq!(lisp.prin1_to_string(help), "\"hello\"");
    });
}

#[test]
fn mouse_face_overlays_highlight_the_overlay() {
    on_big_stack(|| {
        let (mut lisp, f, w) = displayed(
            "(setq ov (make-overlay 22 28)) (overlay-put ov 'mouse-face 'highlight) (overlay-put ov 'help-echo \"ov\")",
        );
        let cw = i32::try_from(lisp.frame(f).geometry.column_width).unwrap();
        let symbol = lisp.intern("ov");
        let ov = lisp.find_symbol_value(symbol);
        let (x, y) = spot(&mut lisp, f, w, 3, 1);
        lisp.note_mouse_highlight(f, x, y).unwrap();
        assert_eq!(hl(&lisp, f), (w, 3, 0, 3, 6, 6 * cw));
        assert_eq!(lisp.mouse_hl_info(f).mouse_face_overlay, ov);
        assert_eq!(lisp.xdisp.help_echo_object, ov);
        assert_eq!(lisp.xdisp.help_echo_pos, 23);
        let (x, y) = spot(&mut lisp, f, w, 3, 4);
        lisp.note_mouse_highlight(f, x, y).unwrap();
        assert_eq!(hl(&lisp, f), (w, 3, 0, 3, 6, 6 * cw));
    });
}

#[test]
fn the_mode_line_and_fringes() {
    on_big_stack(|| {
        let (mut lisp, f, w) = displayed(
            "(setq mode-line-format (list (propertize \"%b\" 'mouse-face 'highlight 'help-echo \"buffer name\")))",
        );
        let g = lisp.frame(f).geometry.clone();
        let (mode_row, row_y, used) = {
            let m = lisp.window_matrix(w, WhichMatrix::Current).unwrap();
            let r = &m.rows[m.mode_line_row()];
            (
                m.mode_line_row() as i32,
                i64::from(r.y),
                r.area_glyphs(TEXT_AREA).len(),
            )
        };
        assert!(used >= 4);
        let x = lisp.window_left_edge_x(w) + 1;
        let y = lisp.window_top_edge_y(w) + row_y + 1;
        lisp.note_mouse_highlight(f, x, y).unwrap();
        let (window, beg_row, beg_col, end_row, end_col, _) = hl(&lisp, f);
        assert_eq!(
            (window, beg_row, beg_col, end_row, end_col),
            (w, mode_row, 0, mode_row, 4)
        );
        let help = lisp.xdisp.help_echo_string;
        assert_eq!(lisp.prin1_to_string(help), "\"buffer name\"");
        assert_eq!(cursor_of(&lisp, f), Some(EmacsCursorKind::Hand));
        let x = lisp.window_left_edge_x(w) + 20 * g.column_width;
        lisp.note_mouse_highlight(f, x, y).unwrap();
        assert_eq!(hl(&lisp, f).0, Value::NIL);
        let x = lisp.window_left_edge_x(w) + 1;
        let y = g.internal_border_width + g.line_height + 2;
        lisp.note_mouse_highlight(f, x, y).unwrap();
        assert_eq!(cursor_of(&lisp, f), Some(EmacsCursorKind::Nontext));
        assert_eq!(lisp.xdisp.help_echo_string, Value::NIL);
    });
}
