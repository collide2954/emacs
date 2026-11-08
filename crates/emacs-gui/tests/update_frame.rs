use std::sync::mpsc;

use emacs_core::dispextern::{GlyphRowArea, TEXT_AREA, TextCursorKind, TextPos};
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

struct Frame {
    lisp: Lisp,
    f: Value,
    w: Value,
}

fn frame_showing(setup: &str) -> Frame {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"u\") (width . 20) (height . 6) (foreground-color . \"black\") (background-color . \"white\")))) (select-frame f) (modify-frame-parameters f '((cursor-color . \"red\"))) (framep f))"
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
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"u\")) (set-buffer \"u\") {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    let fr = lisp.frame_mut(f);
    fr.fonts_changed = false;
    fr.flags.visible = true;
    fr.flags.garbaged = false;
    Frame { lisp, f, w }
}

impl Frame {
    fn redisplay(&mut self, start: i64) {
        let buffer = self.lisp.window(self.w).contents;
        let byte = self.lisp.buffer(buffer).text.char_to_byte(start as usize) as i64;
        emacs_core::dispnew::matrix::clear_glyph_matrix(
            self.lisp
                .window_matrix_mut(self.w, WhichMatrix::Desired)
                .unwrap(),
        );
        assert_eq!(
            self.lisp
                .try_window(self.w, TextPos::new(start, byte), 0)
                .unwrap(),
            1
        );
        self.lisp.window_mut(self.w).flags.must_be_updated_p = true;
        self.lisp.update_frame(self.f, false).unwrap();
    }

    fn pixel(&self, x: i32, y: i32) -> [u8; 3] {
        let frame_id = self.lisp.frame(self.f).id as u64;
        let [r, g, b, _] = emacs_gui::with_display(|d| {
            d.surface(frame_id)
                .expect("frame surface")
                .pixel(x as u32, y as u32)
                .expect("pixel in bounds")
        })
        .expect("display installed");
        [r, g, b]
    }

    fn row_pixels(&self, vpos: usize) -> Vec<[u8; 3]> {
        let row = &self
            .lisp
            .window_matrix(self.w, WhichMatrix::Current)
            .unwrap()
            .rows[vpos];
        let left = self.lisp.window_box_left(self.w, GlyphRowArea::Text) as i32;
        let width = self.lisp.window_box_width(self.w, GlyphRowArea::Text) as i32;
        let y = self.lisp.window_to_frame_pixel_y(self.w, i64::from(row.y)) as i32;
        (y..y + row.height)
            .flat_map(|py| (left..left + width).map(move |px| (px, py)))
            .map(|(px, py)| self.pixel(px, py))
            .collect()
    }
}

#[test]
fn update_frame_makes_the_desired_matrix_current_and_draws_the_cursor() {
    on_big_stack(|| {
        let mut fr = frame_showing("(insert \"Hello\\nworld\") (goto-char 1)");
        fr.redisplay(1);
        let current = fr.lisp.window_matrix(fr.w, WhichMatrix::Current).unwrap();
        assert!(current.rows[0].enabled_p && current.rows[1].enabled_p);
        assert_eq!(current.rows[0].used[TEXT_AREA], 6);
        assert_eq!(current.rows[1].start_charpos(), 7);
        let desired = fr.lisp.window_matrix(fr.w, WhichMatrix::Desired).unwrap();
        assert!(!desired.rows[0].enabled_p);
        assert!(!fr.lisp.window(fr.w).flags.must_be_updated_p);
        let win = fr.lisp.window(fr.w);
        assert!(win.flags.phys_cursor_on_p);
        assert_eq!((win.phys_cursor.hpos, win.phys_cursor.vpos), (0, 0));
        assert_ne!(win.phys_cursor_type, TextCursorKind::NoCursor);
        let left = fr.lisp.window_box_left(fr.w, GlyphRowArea::Text) as i32;
        let top = fr.lisp.window_to_frame_pixel_y(fr.w, 0) as i32;
        assert_eq!(fr.pixel(left, top), [255, 0, 0]);
        let dark = fr
            .row_pixels(1)
            .into_iter()
            .filter(|&[r, g, b]| r < 160 && g < 160 && b < 160)
            .count();
        assert!(dark > 10, "second line not drawn");
    });
}

#[test]
fn update_frame_scrolls_rows_that_moved() {
    on_big_stack(|| {
        let mut fr = frame_showing("(insert \"aaa\\nbbb\\nccc\\nddd\") (goto-char (point-max))");
        fr.redisplay(1);
        let before = fr.row_pixels(2);
        fr.redisplay(5);
        assert_eq!(
            fr.lisp
                .window_matrix(fr.w, WhichMatrix::Current)
                .unwrap()
                .rows[1]
                .start_charpos(),
            9
        );
        assert_eq!(fr.row_pixels(1), before);
    });
}

#[test]
fn update_frame_draws_fringe_indicators_for_continued_lines() {
    on_big_stack(|| {
        let mut fr = frame_showing(
            "(put 'right-curly-arrow 'fringe 8) (put 'left-curly-arrow 'fringe 7) (set-default 'fringe-indicator-alist '((continuation left-curly-arrow right-curly-arrow))) (insert (make-string 50 ?x)) (goto-char 1)",
        );
        fr.redisplay(1);
        let row = fr
            .lisp
            .window_matrix(fr.w, WhichMatrix::Current)
            .unwrap()
            .rows[0]
            .clone();
        assert!(row.continued_p);
        assert_eq!(row.right_fringe_bitmap, 8);
        let right = fr.lisp.window_box_right(fr.w, GlyphRowArea::Text) as i32;
        let y = fr.lisp.window_to_frame_pixel_y(fr.w, i64::from(row.y)) as i32;
        let inked = (right..right + 8)
            .flat_map(|x| (y..y + row.height).map(move |py| (x, py)))
            .filter(|&(x, py)| fr.pixel(x, py) != [255, 255, 255])
            .count();
        assert!(inked > 5, "no continuation bitmap in the right fringe");
    });
}
