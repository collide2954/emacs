use std::sync::mpsc;

use emacs_core::dispextern::{DrawGlyphsFace, GlyphRowArea, GlyphRowRef, TEXT_AREA, TextPos};
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

struct Drawn {
    lisp: Lisp,
    w: Value,
    frame_id: u64,
}

fn drawn(setup: &str) -> Drawn {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"p\") (width . 20) (height . 6) (foreground-color . \"black\") (background-color . \"white\")))) (select-frame f) (framep f))"
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
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"p\")) (set-buffer \"p\") {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    lisp.frame_mut(f).fonts_changed = false;
    assert_eq!(lisp.try_window(w, TextPos::new(1, 1), 0).unwrap(), 1);
    let frame_id = lisp.frame(f).id as u64;
    let used = lisp.window_matrix(w, WhichMatrix::Desired).unwrap().rows[0].used[TEXT_AREA];
    lisp.draw_glyphs(
        w,
        0,
        GlyphRowRef::Window {
            window: w,
            desired: true,
            vpos: 0,
        },
        TEXT_AREA,
        0,
        i64::from(used),
        DrawGlyphsFace::NormalText,
        0,
    )
    .unwrap();
    Drawn { lisp, w, frame_id }
}

impl Drawn {
    fn pixel(&self, x: i32, y: i32) -> [u8; 3] {
        let [r, g, b, _] = emacs_gui::with_display(|d| {
            d.surface(self.frame_id)
                .expect("frame surface")
                .pixel(x as u32, y as u32)
                .expect("pixel in bounds")
        })
        .expect("display installed");
        [r, g, b]
    }

    fn cell(&self, hpos: usize) -> (i32, i32, i32, i32) {
        let row = &self
            .lisp
            .window_matrix(self.w, WhichMatrix::Desired)
            .unwrap()
            .rows[0];
        let left = self.lisp.window_box_left(self.w, GlyphRowArea::Text) as i32;
        let glyphs = row.area_glyphs(TEXT_AREA);
        let x = left
            + glyphs[..hpos]
                .iter()
                .map(|g| i32::from(g.pixel_width))
                .sum::<i32>();
        let y = self.lisp.window_to_frame_pixel_y(self.w, i64::from(row.y)) as i32;
        (x, y, i32::from(glyphs[hpos].pixel_width), row.height)
    }

    fn baseline(&self) -> i32 {
        let row = &self
            .lisp
            .window_matrix(self.w, WhichMatrix::Desired)
            .unwrap()
            .rows[0];
        self.lisp.window_to_frame_pixel_y(self.w, i64::from(row.y)) as i32 + row.ascent
    }

    fn ink(
        &self,
        (x, y, width, height): (i32, i32, i32, i32),
        inked: impl Fn([u8; 3]) -> bool,
    ) -> usize {
        (x..x + width)
            .flat_map(|px| (y..y + height).map(move |py| (px, py)))
            .filter(|&(px, py)| inked(self.pixel(px, py)))
            .count()
    }
}

fn dark([r, g, b]: [u8; 3]) -> bool {
    r < 160 && g < 160 && b < 160
}

#[test]
fn drawing_text_paints_glyphs_in_the_face_colors() {
    on_big_stack(|| {
        let d = drawn("(insert \"Mb\")");
        let cell = d.cell(0);
        assert!(d.ink(cell, dark) > 5, "no ink in the M cell");
        let (x, y, width, height) = cell;
        assert_eq!(d.pixel(x, y), [255, 255, 255]);
        let past = d.cell(2);
        assert_eq!(d.ink((past.0 + past.2, y, width, height), dark), 0);
    });
}
#[test]
fn face_backgrounds_fill_their_cells() {
    on_big_stack(|| {
        let d = drawn(
            "(insert \"ab\") (put-text-property 2 3 'face '(:background \"red\" :foreground \"blue\"))",
        );
        let (x, y, width, height) = d.cell(1);
        assert_eq!(d.pixel(x, y), [255, 0, 0]);
        assert_eq!(d.pixel(x + width - 1, y + height - 1), [255, 0, 0]);
        assert!(d.ink((x, y, width, height), |[r, _, b]| b > r) > 3);
        let (ax, ay, _, _) = d.cell(0);
        assert_eq!(d.pixel(ax, ay), [255, 255, 255]);
    });
}

#[test]
fn boxes_and_underlines_are_drawn_around_text() {
    on_big_stack(|| {
        let d = drawn(
            "(insert \"ab\") (put-text-property 1 2 'face '(:box (:line-width 1 :color \"green\"))) (put-text-property 2 3 'face '(:underline \"blue\"))",
        );
        let (x, y, width, height) = d.cell(0);
        assert_eq!(d.pixel(x + width / 2, y), [0, 255, 0]);
        assert_eq!(d.pixel(x + width / 2, y + height - 1), [0, 255, 0]);
        let (ux, _, uwidth, _) = d.cell(1);
        let baseline = d.baseline();
        let underlined =
            (baseline..y + height).any(|py| d.pixel(ux + uwidth / 2, py) == [0, 0, 255]);
        assert!(underlined, "no underline below the baseline");
    });
}
