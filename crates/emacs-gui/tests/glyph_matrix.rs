use std::sync::mpsc;

use emacs_core::dispextern::{LAST_AREA, LEFT_MARGIN_AREA, RIGHT_MARGIN_AREA, TEXT_AREA};
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

fn gui_frame() -> (Lisp, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"m\") (width . 20) (height . 5)))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    (lisp, f)
}

#[test]
fn producing_glyphs_into_a_desired_row() {
    use emacs_core::dispextern::{GlyphType, GlyphU, It, TextPos};
    on_big_stack(|| {
        let (mut lisp, f) = gui_frame();
        let w = lisp.frame(f).root_window;
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"g\")) (set-buffer \"g\") (insert \"ab\\tc\") (goto-char 1) (buffer-size))"
            ),
            "4"
        );
        lisp.adjust_frame_glyphs(f);
        let buffer = lisp.window(w).contents;
        let mut it = It::default();
        lisp.start_display(&mut it, w, TextPos::new(1, 1)).unwrap();
        assert!(it.glyph_row.is_some());
        for _ in 0..4 {
            assert!(lisp.get_next_display_element(&mut it).unwrap());
            lisp.produce_glyphs_macro(&mut it).unwrap();
            lisp.set_iterator_to_next(&mut it, true).unwrap();
        }
        let row = &lisp.window_matrix(w, WhichMatrix::Desired).unwrap().rows[0];
        let glyphs = row.area_glyphs(TEXT_AREA);
        assert_eq!(glyphs.len(), 4);
        let kinds: Vec<GlyphType> = glyphs.iter().map(|g| g.kind).collect();
        assert_eq!(
            kinds,
            [
                GlyphType::Char,
                GlyphType::Char,
                GlyphType::Stretch,
                GlyphType::Char
            ]
        );
        assert_eq!(glyphs[0].u, GlyphU::Ch(u32::from(b'a')));
        assert_eq!(glyphs[3].u, GlyphU::Ch(u32::from(b'c')));
        let positions: Vec<i64> = glyphs.iter().map(|g| g.charpos).collect();
        assert_eq!(positions, [1, 2, 3, 4]);
        assert!(
            glyphs
                .iter()
                .all(|g| g.object == buffer && g.pixel_width > 0)
        );
        let column = lisp.frame(f).geometry.column_width as i16;
        assert_eq!(glyphs[2].pixel_width, 6 * column);
        assert_eq!(it.current_x, i32::from(9 * column));
    });
}

#[test]
fn try_window_builds_the_desired_matrix() {
    use emacs_core::dispextern::{GlyphType, GlyphU, TextPos};
    on_big_stack(|| {
        let (mut lisp, f) = gui_frame();
        let w = lisp.frame(f).root_window;
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"t\")) (set-buffer \"t\") (insert \"ab\\ncd\") (goto-char 5) (point))"
            ),
            "5"
        );
        lisp.adjust_frame_glyphs(f);
        lisp.frame_mut(f).fonts_changed = false;
        let buffer = lisp.window(w).contents;
        assert_eq!(lisp.try_window(w, TextPos::new(1, 1), 0).unwrap(), 1);
        assert!(!lisp.frame(f).fonts_changed);
        let m = lisp.window_matrix(w, WhichMatrix::Desired).unwrap();
        let (row0, row1, row2) = (&m.rows[0], &m.rows[1], &m.rows[2]);
        assert!(row0.enabled_p && row0.displays_text_p);
        let chars = |row: &emacs_core::dispextern::GlyphRow| {
            row.area_glyphs(TEXT_AREA)
                .iter()
                .filter(|g| g.object == buffer)
                .map(|g| match g.u {
                    GlyphU::Ch(c) => char::from_u32(c).unwrap(),
                    _ => '?',
                })
                .collect::<String>()
        };
        assert_eq!(chars(row0), "ab");
        assert_eq!(chars(row1), "cd");
        let space = row0.area_glyphs(TEXT_AREA)[2];
        assert_eq!(
            (space.kind, space.object, space.charpos),
            (GlyphType::Char, Value::NIL, 0)
        );
        assert_eq!((row0.minpos.charpos, row0.maxpos.charpos), (1, 4));
        assert_eq!((row1.minpos.charpos, row1.maxpos.charpos), (4, 6));
        assert!(row1.ends_at_zv_p && !row0.ends_at_zv_p);
        assert_eq!(row0.y, 0);
        assert_eq!(row1.y, row0.height);
        assert!(row0.height > 0 && row0.hash != 0);
        assert!(row2.enabled_p && !row2.displays_text_p);
        let win = lisp.window(w);
        assert_eq!((win.cursor.vpos, win.cursor.hpos), (1, 1));
        assert_eq!(win.cursor.y, i64::from(row0.height));
        assert_eq!(win.window_end_vpos, 1);
        assert!(!win.flags.window_end_valid);
        assert_eq!(lisp.marker(win.start).charpos, 1);
    });
}

fn buffer_chars(row: &emacs_core::dispextern::GlyphRow, buffer: Value) -> String {
    use emacs_core::dispextern::GlyphU;
    row.area_glyphs(TEXT_AREA)
        .iter()
        .filter(|g| g.object == buffer)
        .map(|g| match g.u {
            GlyphU::Ch(c) => char::from_u32(c).unwrap(),
            _ => '?',
        })
        .collect()
}

fn try_window_on(lisp: &mut Lisp, f: Value, setup: &str) -> Value {
    use emacs_core::dispextern::TextPos;
    let w = lisp.frame(f).root_window;
    assert_eq!(
        eval(
            lisp,
            &format!(
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"l\")) (set-buffer \"l\") {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    lisp.frame_mut(f).fonts_changed = false;
    assert_eq!(lisp.try_window(w, TextPos::new(1, 1), 0).unwrap(), 1);
    w
}

#[test]
fn try_window_continues_long_lines() {
    on_big_stack(|| {
        let (mut lisp, f) = gui_frame();
        let w = try_window_on(
            &mut lisp,
            f,
            "(insert (make-string 25 ?a) \"\\nb\") (goto-char 23)",
        );
        let buffer = lisp.window(w).contents;
        let m = lisp.window_matrix(w, WhichMatrix::Desired).unwrap();
        let (row0, row1, row2) = (&m.rows[0], &m.rows[1], &m.rows[2]);
        assert_eq!(buffer_chars(row0, buffer), "a".repeat(20));
        assert!(row0.continued_p && !row0.continuation_line_p());
        assert_eq!((row0.minpos.charpos, row0.maxpos.charpos), (1, 21));
        assert_eq!(buffer_chars(row1, buffer), "a".repeat(5));
        assert!(!row1.continued_p && row1.continuation_line_p());
        assert_eq!((row1.minpos.charpos, row1.maxpos.charpos), (21, 27));
        assert_eq!(buffer_chars(row2, buffer), "b");
        assert!(row2.ends_at_zv_p);
        let win = lisp.window(w);
        assert_eq!((win.cursor.vpos, win.cursor.hpos), (1, 2));
        assert_eq!(win.window_end_vpos, 2);
    });
}

#[test]
fn try_window_truncates_long_lines() {
    on_big_stack(|| {
        let (mut lisp, f) = gui_frame();
        let w = try_window_on(
            &mut lisp,
            f,
            "(setq truncate-lines t) (insert (make-string 25 ?a) \"\\nb\") (goto-char 27)",
        );
        let buffer = lisp.window(w).contents;
        let m = lisp.window_matrix(w, WhichMatrix::Desired).unwrap();
        let (row0, row1) = (&m.rows[0], &m.rows[1]);
        assert_eq!(buffer_chars(row0, buffer), "a".repeat(20));
        assert!(row0.truncated_on_right_p && !row0.continued_p);
        assert_eq!(row0.end.pos.charpos, 27);
        assert_eq!(buffer_chars(row1, buffer), "b");
        assert!(row1.ends_at_zv_p && !row1.continuation_line_p());
        let win = lisp.window(w);
        assert_eq!((win.cursor.vpos, win.cursor.hpos), (1, 0));
    });
}

#[test]
fn glyph_matrices_for_window_redisplay() {
    on_big_stack(|| {
        let (mut lisp, f) = gui_frame();
        let w = lisp.frame(f).root_window;
        assert!(lisp.window(w).current_matrix.is_none());
        lisp.adjust_frame_glyphs(f);
        assert!(lisp.frame(f).flags.glyphs_initialized_p);
        let t = lisp.frame(f).terminal;
        let (ch_width, ch_height) = {
            let info = lisp.display_info(t);
            (
                info.smallest_char_width.max(1) as i64,
                info.smallest_font_height.max(1) as i64,
            )
        };
        let win = lisp.window(w);
        let width = ((win.pixel_width + ch_width - 1) / ch_width + 4) as usize;
        let box_height = lisp.window_box_height(w);
        let height = ((box_height + ch_height - 1) / ch_height + 5) as i32;
        for which in [WhichMatrix::Current, WhichMatrix::Desired] {
            let m = lisp.window_matrix(w, which).expect("matrix");
            assert_eq!(m.nrows, height);
            assert!(m.rows.len() >= height as usize);
            assert_eq!(m.matrix_w as usize, width);
            assert_eq!((m.left_margin_glyphs, m.right_margin_glyphs), (0, 0));
            for row in &m.rows {
                assert_eq!(row.glyphs.len(), width);
                assert_eq!(row.area, [0, 0, width, width]);
                assert!(!row.enabled_p);
            }
        }
        let bar = lisp.frame(f).tab_bar_window;
        assert!(lisp.is_window(bar));
        assert!(lisp.window(bar).flags.pseudo_window_p);
        assert_eq!(lisp.window(bar).total_lines, 0);
        assert_eq!(
            lisp.window_matrix(bar, WhichMatrix::Desired).unwrap().nrows,
            5
        );

        assert_eq!(
            eval(&mut lisp, "(progn (set-window-margins nil 2 3) nil)"),
            "nil"
        );
        lisp.allocate_matrices_for_window_redisplay(w);
        let total_cols = lisp.window(w).total_cols as f64;
        let reserve = |margin: f64| {
            ((width as f64 / total_cols * margin.min((total_cols as i64 / 2 - 1) as f64)) as i32)
                .max(1)
        };
        let (left, right) = (reserve(2.0), reserve(3.0));
        let m = lisp.window_matrix(w, WhichMatrix::Desired).unwrap();
        assert_eq!((m.left_margin_glyphs, m.right_margin_glyphs), (left, right));
        let text_row = &m.rows[1];
        assert_eq!(
            text_row.area,
            [0, left as usize, width - right as usize, width]
        );
        let mode_line = &m.rows[m.mode_line_row()];
        assert_eq!(mode_line.area[LEFT_MARGIN_AREA], 0);
        assert_eq!(mode_line.area[TEXT_AREA], left as usize);
        assert_eq!(
            text_row.area[LAST_AREA] - text_row.area[RIGHT_MARGIN_AREA],
            right as usize
        );

        lisp.prepare_desired_row(w, 2, false);
        assert!(lisp.window_matrix(w, WhichMatrix::Desired).unwrap().rows[2].enabled_p);
        lisp.prepare_desired_row(w, 0, true);
        let row0 = &lisp.window_matrix(w, WhichMatrix::Desired).unwrap().rows[0];
        assert_eq!(row0.area[TEXT_AREA], row0.area[LEFT_MARGIN_AREA]);
        assert_eq!(row0.area[RIGHT_MARGIN_AREA], row0.area[LAST_AREA]);

        lisp.window_matrix_mut(w, WhichMatrix::Desired)
            .unwrap()
            .rows[2]
            .y = 17;
        lisp.make_current(w, 2);
        let current = &lisp.window_matrix(w, WhichMatrix::Current).unwrap().rows[2];
        assert!(current.enabled_p);
        assert_eq!(current.y, 17);

        lisp.clear_current_matrices(f);
        assert!(!lisp.window_matrix(w, WhichMatrix::Current).unwrap().rows[2].enabled_p);
        lisp.free_glyphs(f);
        assert!(!lisp.frame(f).flags.glyphs_initialized_p);
        assert!(lisp.window(w).current_matrix.is_none());
        assert!(lisp.frame(f).tab_bar_window.is_nil());
    });
}
