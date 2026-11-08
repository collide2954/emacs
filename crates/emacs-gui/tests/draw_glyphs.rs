use std::cell::RefCell;
use std::sync::mpsc;

use emacs_core::dispextern::{
    DrawFringeBitmapParams, DrawGlyphsFace, GlyphRowArea, GlyphRowRef, GlyphString,
    RedisplayInterface, TEXT_AREA, TextPos,
};
use emacs_core::dispnew::matrix::WhichMatrix;
use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
use emacs_core::xdisp::draw::{gui_clear_end_of_line, gui_write_glyphs};
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

#[derive(Clone, Debug, PartialEq)]
enum Call {
    String {
        x: i32,
        nchars: usize,
        first_glyph: usize,
        face: Option<usize>,
        width: i32,
        background_width: i32,
        extends: bool,
    },
    Clear {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    Fringe {
        vpos: usize,
        params: DrawFringeBitmapParams,
    },
}

thread_local! {
    static CALLS: RefCell<Vec<Call>> = const { RefCell::new(Vec::new()) };
}

fn record_string(_lisp: &mut Lisp, strings: &mut [GlyphString], i: usize) {
    let s = &strings[i];
    let call = Call::String {
        x: s.x,
        nchars: s.nchars,
        first_glyph: s.first_glyph,
        face: s.face,
        width: s.width,
        background_width: s.background_width,
        extends: s.extends_to_end_of_line_p,
    };
    CALLS.with(|c| c.borrow_mut().push(call));
}

fn record_clear(_lisp: &mut Lisp, _f: Value, x: i32, y: i32, width: i32, height: i32) {
    CALLS.with(|c| {
        c.borrow_mut().push(Call::Clear {
            x,
            y,
            width,
            height,
        })
    });
}

fn record_fringe(_lisp: &mut Lisp, _w: Value, row: GlyphRowRef, p: &DrawFringeBitmapParams) {
    let GlyphRowRef::Window { vpos, .. } = row else {
        panic!("fringe drawn for a scratch row");
    };
    CALLS.with(|c| c.borrow_mut().push(Call::Fringe { vpos, params: *p }));
}

static RECORDING: RedisplayInterface = RedisplayInterface {
    write_glyphs: Some(gui_write_glyphs),
    clear_end_of_line: Some(gui_clear_end_of_line),
    draw_glyph_string: Some(record_string),
    clear_frame_area: Some(record_clear),
    draw_fringe_bitmap: Some(record_fringe),
    ..RedisplayInterface::NONE
};

fn take_calls() -> Vec<Call> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn window_showing(setup: &str) -> (Lisp, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"d\") (width . 20) (height . 6)))) (select-frame f) (framep f))"
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
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"d\")) (set-buffer \"d\") {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    lisp.frame_mut(f).fonts_changed = false;
    assert_eq!(lisp.try_window(w, TextPos::new(1, 1), 0).unwrap(), 1);
    let t = lisp.frame(f).terminal;
    lisp.terminal_mut(t).rif = Some(&RECORDING);
    take_calls();
    (lisp, w)
}

fn desired_row(w: Value, vpos: usize) -> GlyphRowRef {
    GlyphRowRef::Window {
        window: w,
        desired: true,
        vpos,
    }
}

fn glyph_widths(lisp: &Lisp, w: Value, vpos: usize) -> Vec<i32> {
    lisp.window_matrix(w, WhichMatrix::Desired).unwrap().rows[vpos]
        .area_glyphs(TEXT_AREA)
        .iter()
        .map(|g| i32::from(g.pixel_width))
        .collect()
}

#[test]
fn draw_glyphs_builds_one_string_per_face_run() {
    on_big_stack(|| {
        let (mut lisp, w) = window_showing(
            "(insert \"abcd\") (put-text-property 3 5 'face '(:foreground \"red\"))",
        );
        let widths = glyph_widths(&lisp, w, 0);
        assert_eq!(widths.len(), 5);
        let left = lisp.window_box_left(w, GlyphRowArea::Text) as i32;
        let reached = lisp
            .draw_glyphs(
                w,
                0,
                desired_row(w, 0),
                TEXT_AREA,
                0,
                5,
                DrawGlyphsFace::NormalText,
                0,
            )
            .unwrap();
        let calls = take_calls();
        assert_eq!(calls.len(), 3);
        let strings: Vec<(i32, usize, usize, i32)> = calls
            .iter()
            .map(|c| match c {
                Call::String {
                    x,
                    nchars,
                    first_glyph,
                    width,
                    ..
                } => (*x, *nchars, *first_glyph, *width),
                Call::Clear { .. } | Call::Fringe { .. } => panic!("unexpected call"),
            })
            .collect();
        let ab = widths[0] + widths[1];
        let cd = widths[2] + widths[3];
        assert_eq!(
            strings,
            [
                (left, 2, 0, ab),
                (left + ab, 2, 2, cd),
                (left + ab + cd, 1, 4, widths[4])
            ]
        );
        let faces: Vec<Option<usize>> = calls
            .iter()
            .map(|c| match c {
                Call::String { face, .. } => *face,
                Call::Clear { .. } | Call::Fringe { .. } => None,
            })
            .collect();
        assert_eq!(faces[0], faces[2]);
        assert_ne!(faces[0], faces[1]);
        assert_eq!(reached, widths.iter().sum::<i32>());
    });
}

#[test]
fn draw_glyphs_extends_a_filled_line_to_the_area_end() {
    on_big_stack(|| {
        let (mut lisp, w) = window_showing("(insert \"ab\")");
        lisp.glyph_row_mut(desired_row(w, 0)).fill_line_p = true;
        let left = lisp.window_box_left(w, GlyphRowArea::Text) as i32;
        let last_x = left + lisp.window_box_width(w, GlyphRowArea::Text) as i32;
        let widths = glyph_widths(&lisp, w, 0);
        let reached = lisp
            .draw_glyphs(
                w,
                0,
                desired_row(w, 0),
                TEXT_AREA,
                0,
                widths.len() as i64,
                DrawGlyphsFace::NormalText,
                0,
            )
            .unwrap();
        let calls = take_calls();
        let Some(Call::String {
            x,
            background_width,
            extends,
            ..
        }) = calls.last().cloned()
        else {
            panic!("no glyph strings drawn: {calls:?}");
        };
        assert!(extends);
        assert_eq!(background_width, last_x - x + 1);
        assert_eq!(reached, last_x + 1 - left);
    });
}

#[test]
fn writing_glyphs_and_clearing_to_the_end_of_line() {
    on_big_stack(|| {
        let (mut lisp, w) = window_showing("(insert \"abc\")");
        let widths = glyph_widths(&lisp, w, 0);
        let row = desired_row(w, 0);
        let (row_y, row_height) = {
            let r = lisp.glyph_row(row);
            (r.y, r.height)
        };
        {
            let win = lisp.window_mut(w);
            win.output_cursor.x = 0;
            win.output_cursor.y = i64::from(row_y);
            win.output_cursor.hpos = 0;
            win.output_cursor.vpos = 0;
        }
        gui_write_glyphs(&mut lisp, w, row, TEXT_AREA, 0, 2).unwrap();
        let cursor = lisp.window(w).output_cursor;
        assert_eq!(cursor.hpos, 2);
        assert_eq!(cursor.x, i64::from(widths[0] + widths[1]));
        assert_eq!(take_calls().len(), 1);
        gui_clear_end_of_line(&mut lisp, w, row, TEXT_AREA, -1).unwrap();
        let left = lisp.window_box_left(w, GlyphRowArea::Text) as i32;
        let width = lisp.window_box_width(w, GlyphRowArea::Text) as i32;
        let f = lisp.window(w).frame;
        let top = (lisp.frame(f).geometry.internal_border_width + lisp.window(w).pixel_top) as i32;
        let x = left + widths[0] + widths[1];
        assert_eq!(
            take_calls(),
            [Call::Clear {
                x,
                y: top + row_y,
                width: left + width - x,
                height: row_height,
            }]
        );
    });
}

#[test]
fn drawing_the_fringes_of_rows_marked_for_redraw() {
    on_big_stack(|| {
        let (mut lisp, w) = window_showing("(insert \"a\nb\")");
        let nrows = lisp.window_matrix(w, WhichMatrix::Desired).unwrap().nrows as usize;
        for vpos in 0..nrows {
            lisp.make_current(w, vpos);
        }
        let row = GlyphRowRef::Window {
            window: w,
            desired: false,
            vpos: 1,
        };
        let (row_y, row_height) = {
            let r = lisp.glyph_row_mut(row);
            r.left_fringe_bitmap = 3;
            r.redraw_fringe_bitmaps_p = true;
            (r.y, r.height)
        };
        assert!(lisp.draw_window_fringes(w, false).unwrap());
        assert!(!lisp.glyph_row(row).redraw_fringe_bitmaps_p);
        let calls = take_calls();
        assert_eq!(calls.len(), 2);
        let Call::Fringe { vpos, params: left } = calls[0] else {
            panic!("expected a fringe bitmap: {calls:?}");
        };
        let Call::Fringe { params: right, .. } = calls[1] else {
            panic!("expected a fringe bitmap: {calls:?}");
        };
        assert_eq!(vpos, 1);
        let f = lisp.window(w).frame;
        let top = (lisp.frame(f).geometry.internal_border_width + lisp.window(w).pixel_top) as i32;
        let text_left = lisp.window_box_left(w, GlyphRowArea::Text) as i32;
        assert_eq!((left.which, left.wd, left.h), (3, 8, 8));
        assert_eq!(left.x, text_left - 8);
        assert_eq!(left.y, top + row_y + (row_height - 8) / 2);
        assert_eq!((left.by, left.ny), (top + row_y, row_height));
        assert_eq!(right.which, 0);
        assert_eq!(right.wd, 0);
        let text_right = lisp.window_box_right(w, GlyphRowArea::Text) as i32;
        assert_eq!((right.bx, right.nx), (text_right, 8));
        assert!(!lisp.draw_window_fringes(w, false).unwrap());
        assert!(take_calls().is_empty());
    });
}
