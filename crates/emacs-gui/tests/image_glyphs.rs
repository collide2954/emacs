use std::sync::mpsc;

use emacs_core::dispextern::{
    EmacsCursorKind, GlyphRow, GlyphRowArea, GlyphSlice, GlyphType, GlyphU, TEXT_AREA,
    TextCursorKind,
};
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

const TALL: &str = "(setq tall (list 'image :type 'xbm :data-width 8 :data-height 40 :data (make-bool-vector 320 t) :ascent 100))";
const SMALL: &str = "(setq small (list 'image :type 'pbm :data \"P3 2 2 255 255 0 0 0 0 255 0 0 255 255 0 0\" :ascent 100))";

struct Shown {
    lisp: Lisp,
    f: Value,
    w: Value,
}

fn shown(setup: &str) -> Shown {
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
                "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"g\") (width . 30) (height . 8) (foreground-color . \"black\") (background-color . \"white\")))) (select-frame f) (set-window-buffer nil (get-buffer-create \"img\")) (set-buffer \"img\") (setq bidi-paragraph-direction 'left-to-right) (setq mode-line-format nil) {TALL} {SMALL} {setup} (framep f))"
            )
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    let w = lisp.frame(f).root_window;
    lisp.frame_mut(f).flags.visible = true;
    let terminal = lisp.frame(f).terminal;
    lisp.display_info_mut(terminal).highlight_frame = f;
    lisp.redisplay().unwrap();
    Shown { lisp, f, w }
}

impl Shown {
    fn row(&self, vpos: usize) -> &GlyphRow {
        &self
            .lisp
            .window_matrix(self.w, WhichMatrix::Current)
            .unwrap()
            .rows[vpos]
    }

    fn cw(&self) -> i32 {
        self.lisp.frame(self.f).geometry.column_width as i32
    }

    fn eval(&mut self, source: &str) -> String {
        eval(&mut self.lisp, source)
    }

    fn pixel(&self, x: i32, y: i32) -> [u8; 3] {
        let id = self.lisp.frame(self.f).id as u64;
        let [r, g, b, _] = emacs_gui::with_display(|d| {
            d.surface(id)
                .expect("frame surface")
                .pixel(x as u32, y as u32)
                .expect("pixel in bounds")
        })
        .expect("display installed");
        [r, g, b]
    }

    fn glyph_left(&self, vpos: usize, hpos: usize) -> i32 {
        let left = self.lisp.window_box_left(self.w, GlyphRowArea::Text) as i32;
        left + self.row(vpos).area_glyphs(TEXT_AREA)[..hpos]
            .iter()
            .map(|g| i32::from(g.pixel_width))
            .sum::<i32>()
    }

    fn baseline(&self, vpos: usize) -> i32 {
        let row = self.row(vpos);
        self.lisp.window_to_frame_pixel_y(self.w, i64::from(row.y)) as i32 + row.ascent
    }
}

fn rgb(pixel: u64) -> [u8; 3] {
    [(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8]
}

#[test]
fn display_properties_produce_image_glyphs() {
    on_big_stack(|| {
        let mut s =
            shown("(insert \"a\" (propertize \"b\" 'display tall) \"c\\nd\") (goto-char 1)");
        let buffer = s.lisp.window(s.w).contents;
        let cw = s.cw();
        let row = s.row(0).clone();
        let glyphs = row.area_glyphs(TEXT_AREA);
        assert_eq!(
            glyphs[..3].iter().map(|g| g.kind).collect::<Vec<_>>(),
            [GlyphType::Char, GlyphType::Image, GlyphType::Char]
        );
        let (a, img) = (glyphs[0], glyphs[1]);
        assert_eq!(
            (img.pixel_width, img.ascent, img.descent, img.charpos),
            (8, 40, 0, 2)
        );
        assert_eq!(img.object, buffer);
        assert_eq!(
            img.slice,
            GlyphSlice::Img {
                x: 0,
                y: 0,
                width: 8,
                height: 40
            }
        );
        let GlyphU::ImgId(id) = img.u else {
            panic!("image glyph without an image id");
        };
        let spec = s.lisp.image(s.f, id as usize).expect("cached image").spec;
        let symbol = s.lisp.intern("tall");
        assert_eq!(spec, s.lisp.find_symbol_value(symbol));
        assert_eq!(row.ascent, 40);
        assert_eq!(row.height, 40 + i32::from(a.descent));
        assert_eq!(
            s.eval("(window-text-pixel-size nil 1 4)"),
            format!("({} . {})", 2 * cw + 8, row.height)
        );
    });
}

#[test]
fn margins_slices_and_centered_ascents_shape_image_glyphs() {
    on_big_stack(|| {
        let s = shown(
            "(insert \"a\" (propertize \"b\" 'display (list 'image :type 'xbm :data-width 8 :data-height 40 :data (make-bool-vector 320 t) :margin '(3 . 2) :ascent 'center)) \"\\n\" (propertize \"c\" 'display (list '(slice 2 10 4 20) tall)) \"\\n\" (propertize \"d\" 'display (list '(slice 0.5 0.25 0.5 0.5) tall)))",
        );
        let row = s.row(0).clone();
        let glyphs = row.area_glyphs(TEXT_AREA);
        let (a, img) = (glyphs[0], glyphs[1]);
        let centered = (42 + i32::from(a.ascent) - i32::from(a.descent) + 1) / 2;
        assert_eq!(
            (
                img.pixel_width,
                i32::from(img.ascent),
                i32::from(img.descent)
            ),
            (14, centered, 44 - centered)
        );
        for vpos in [1, 2] {
            let g = s.row(vpos).area_glyphs(TEXT_AREA)[0];
            assert_eq!(g.kind, GlyphType::Image);
            assert_eq!((g.pixel_width, g.ascent, g.descent), (4, 20, 0));
        }
        assert_eq!(
            s.row(1).area_glyphs(TEXT_AREA)[0].slice,
            GlyphSlice::Img {
                x: 2,
                y: 10,
                width: 4,
                height: 20
            }
        );
        assert_eq!(
            s.row(2).area_glyphs(TEXT_AREA)[0].slice,
            GlyphSlice::Img {
                x: 4,
                y: 10,
                width: 4,
                height: 20
            }
        );
    });
}

#[test]
fn space_specs_and_line_prefixes_can_use_images() {
    on_big_stack(|| {
        let s = shown(
            "(insert (propertize \"x\" 'display (list 'space :width tall)) (propertize \"y\" 'display (list 'space :width 1 :height tall)) \"\\nz\") (put-text-property (point-max) (1- (point-max)) 'line-prefix small) (setq line-prefix tall)",
        );
        let cw = s.cw();
        let glyphs = s.row(0).area_glyphs(TEXT_AREA).to_vec();
        assert_eq!(glyphs[0].kind, GlyphType::Image);
        assert!(glyphs[0].avoid_cursor_p);
        assert_eq!(glyphs[0].pixel_width, 8);
        assert_eq!(
            (glyphs[1].kind, glyphs[1].pixel_width),
            (GlyphType::Stretch, 8)
        );
        assert_eq!(glyphs[2].kind, GlyphType::Stretch);
        assert_eq!(i32::from(glyphs[2].pixel_width), cw);
        assert_eq!(glyphs[2].ascent + glyphs[2].descent, 40);
        let next = s.row(1).area_glyphs(TEXT_AREA).to_vec();
        assert_eq!((next[0].kind, next[0].pixel_width), (GlyphType::Image, 2));
        assert_eq!(next[1].kind, GlyphType::Char);
    });
}

#[test]
fn image_pixels_margins_and_reliefs_are_drawn() {
    on_big_stack(|| {
        let s = shown(
            "(insert \"a\" (propertize \"b\" 'display small) \"c\\n\" \"a\" (propertize \"b\" 'display (append small '(:margin 1)) 'face '(:background \"green\")) \"c\\n\" \"a\" (propertize \"b\" 'display (append small '(:relief 1))) \"c\") (goto-char (point-max))",
        );
        let (x, base) = (s.glyph_left(0, 1), s.baseline(0));
        assert_eq!(s.pixel(x, base - 2), [255, 0, 0]);
        assert_eq!(s.pixel(x + 1, base - 2), [0, 0, 255]);
        assert_eq!(s.pixel(x, base - 1), [0, 0, 255]);
        assert_eq!(s.pixel(x + 1, base - 1), [255, 0, 0]);
        assert_eq!(s.pixel(x, base - 3), [255, 255, 255]);
        let (x, base) = (s.glyph_left(1, 1), s.baseline(1));
        assert_eq!(s.row(1).area_glyphs(TEXT_AREA)[1].pixel_width, 4);
        assert_eq!(s.pixel(x, base - 2), [0, 255, 0]);
        assert_eq!(s.pixel(x + 1, base - 2), [255, 0, 0]);
        assert_eq!(s.pixel(x + 2, base - 1), [255, 0, 0]);
        assert_eq!(s.pixel(x + 3, base), [0, 255, 0]);
        let (x, base) = (s.glyph_left(2, 1), s.baseline(2));
        let (white, black, background) = emacs_gui::pgtkterm::with_output(&s.lisp, s.f, |o| {
            (
                o.white_relief_pixel as u64,
                o.black_relief_pixel as u64,
                o.relief_background,
            )
        });
        assert_eq!(background, 0xFF0000);
        assert_eq!(s.pixel(x + 1, base - 3), rgb(white));
        assert_eq!(s.pixel(x, base - 2), rgb(white));
        assert_eq!(s.pixel(x + 1, base), rgb(black));
        assert_eq!(s.pixel(x + 3, base - 1), rgb(black));
        assert_eq!(s.pixel(x + 1, base - 2), [255, 0, 0]);
    });
}

#[test]
fn cursors_on_images_are_hollow_unless_the_image_is_masked() {
    on_big_stack(|| {
        let mut s = shown(
            "(insert \"a\" (propertize \"b\" 'display tall) (propertize \"c\" 'display (append small '(:mask heuristic))) \"d\") (goto-char 2)",
        );
        assert_eq!(
            s.lisp.window(s.w).phys_cursor_type,
            TextCursorKind::HollowBox
        );
        s.eval("(goto-char 3)");
        s.lisp.redisplay().unwrap();
        assert_eq!(
            s.lisp.window(s.w).phys_cursor_type,
            TextCursorKind::FilledBox
        );
        s.eval("(progn (setq cursor-type 'bar) (goto-char 4))");
        s.lisp.redisplay().unwrap();
        assert_eq!(s.lisp.window(s.w).phys_cursor_type, TextCursorKind::Bar);
        s.eval("(goto-char 3)");
        s.lisp.redisplay().unwrap();
        assert_eq!(
            s.lisp.window(s.w).phys_cursor_type,
            TextCursorKind::HollowBox
        );
    });
}

#[test]
fn positions_over_images_report_the_image_and_its_hot_spots() {
    on_big_stack(|| {
        let mut s = shown(
            "(setq tall (append tall '(:map (((rect . ((0 . 0) . (4 . 4))) area1 (pointer hand help-echo \"hi\")))))) (insert \"a\" (propertize \"b\" 'display tall) \"c\\nd\") (goto-char 1)",
        );
        let cw = s.cw();
        let window = s.lisp.prin1_to_string(s.w);
        let symbol = s.lisp.intern("tall");
        let tall = s.lisp.find_symbol_value(symbol);
        let spec = s.lisp.prin1_to_string(tall);
        assert_eq!(
            s.eval(&format!("(posn-at-x-y {cw} 0)")),
            format!("({window} area1 ({cw} . 0) 0 nil 2 (1 . 0) {spec} (0 . 0) (8 . 40))")
        );
        assert_eq!(
            s.eval(&format!("(posn-at-x-y {} 1)", cw + 1)),
            format!(
                "({window} area1 ({} . 1) 0 nil 2 (1 . 0) {spec} (1 . 1) (8 . 40))",
                cw + 1
            )
        );
        assert_eq!(
            s.eval(&format!("(posn-at-x-y {} 6)", cw + 6)),
            format!(
                "({window} 2 ({} . 6) 0 nil 2 (1 . 0) {spec} (6 . 6) (8 . 40))",
                cw + 6
            )
        );
        let left = s.lisp.window_box_left(s.w, GlyphRowArea::Text);
        let top = s.lisp.frame(s.f).geometry.internal_border_width;
        let (f, w) = (s.f, s.w);
        s.lisp
            .note_mouse_highlight(f, left + i64::from(cw) + 1, top + 1)
            .unwrap();
        let help = s.lisp.xdisp.help_echo_string;
        assert_eq!(s.lisp.prin1_to_string(help), "\"hi\"");
        assert_eq!(s.lisp.xdisp.help_echo_window, w);
        assert_eq!(s.lisp.xdisp.help_echo_pos, 2);
        assert_eq!(
            emacs_gui::pgtkterm::with_output(&s.lisp, f, |o| o.current_cursor),
            Some(EmacsCursorKind::Hand)
        );
    });
}

#[test]
fn columns_count_an_images_display_width() {
    on_big_stack(|| {
        let mut s = shown(
            "(insert \"a\" (propertize \"b\" 'display (list 'image :type 'xbm :data-width 24 :data-height 2 :data (make-bool-vector 48 t))) \"cd\")",
        );
        let columns = (24.0 / f64::from(s.cw()) + 0.5) as i64;
        assert_eq!(
            s.eval("(progn (goto-char 3) (current-column))"),
            (1 + columns).to_string()
        );
        assert_eq!(
            s.eval("(progn (goto-char 4) (current-column))"),
            (2 + columns).to_string()
        );
    });
}

#[test]
fn image_maps_are_looked_up_by_shape() {
    on_big_stack(|| {
        let mut s = shown("");
        let map = "'(((rect . ((0 . 0) . (10 . 10))) a) ((circle . ((20 . 20) . 5)) b) ((poly . [30 0 40 10 30 10]) c))";
        for (x, y, expected) in [
            (5, 5, "((rect (0 . 0) 10 . 10) a)"),
            (22, 23, "((circle (20 . 20) . 5) b)"),
            (25, 25, "nil"),
            (35, 8, "((poly . [30 0 40 10 30 10]) c)"),
            (35, 2, "nil"),
        ] {
            assert_eq!(
                s.eval(&format!("(lookup-image-map {map} {x} {y})")),
                expected
            );
        }
        assert_eq!(s.eval("(lookup-image-map nil 'a 0)"), "nil");
        assert_eq!(
            s.eval("(lookup-image-map '(x) 'a 0)"),
            "error: (wrong-type-argument fixnump a)"
        );
    });
}

#[test]
fn mode_line_and_margin_images_are_found_by_position() {
    on_big_stack(|| {
        let mut s = shown(
            "(setq mode-line-format (list (propertize \"m\" 'display small))) (set-window-margins nil 2) (insert (propertize \"q\" 'display (list '(margin left-margin) small)) \"text\")",
        );
        let spec = s.eval("small");
        let m = s.lisp.window_matrix(s.w, WhichMatrix::Current).unwrap();
        let ml = m.mode_line_row();
        let (ml_y, ml_ascent) = (m.rows[ml].y, m.rows[ml].ascent);
        let g = m.rows[ml].area_glyphs(TEXT_AREA)[0];
        assert_eq!((g.kind, g.pixel_width, g.ascent), (GlyphType::Image, 2, 2));
        let posn = format!("(posn-at-x-y 1 {} nil t)", ml_y + 1);
        assert_eq!(s.eval(&format!("(nth 1 {posn})")), "mode-line");
        assert_eq!(s.eval(&format!("(nth 7 {posn})")), spec);
        assert_eq!(
            s.eval(&format!("(nth 8 {posn})")),
            format!("(1 . {})", 1 - (ml_ascent - 2))
        );
        let margin_ascent = s.row(0).ascent;
        let mg = s.row(0).area_glyphs(GlyphRowArea::LeftMargin.index())[0];
        assert_eq!((mg.kind, mg.pixel_width), (GlyphType::Image, 2));
        let y = margin_ascent - 1;
        let posn = format!("(posn-at-x-y 1 {y} nil t)");
        assert_eq!(s.eval(&format!("(nth 1 {posn})")), "left-margin");
        assert_eq!(s.eval(&format!("(nth 7 {posn})")), spec);
        assert_eq!(
            s.eval(&format!("(nth 8 {posn})")),
            format!("(1 . {})", y - (margin_ascent - 2))
        );
    });
}

#[test]
fn mouse_face_redraws_images_in_the_mouse_face() {
    on_big_stack(|| {
        let mut s = shown(
            "(insert \"a\" (propertize \"b\" 'display (append small '(:margin 1)) 'mouse-face 'highlight) \"c\")",
        );
        let (x, base) = (s.glyph_left(0, 1), s.baseline(0));
        let left = s.lisp.window_box_left(s.w, GlyphRowArea::Text);
        let top = s.lisp.frame(s.f).geometry.internal_border_width;
        let f = s.f;
        s.lisp
            .note_mouse_highlight(f, left + i64::from(s.cw()) + 1, top + 1)
            .unwrap();
        let face = s.lisp.mouse_hl_info(f).mouse_face_face_id;
        let background = s.lisp.face_from_id(f, face).unwrap().background;
        assert_eq!(s.pixel(x, base - 2), rgb(background));
        assert_eq!(s.pixel(x + 1, base - 2), [255, 0, 0]);
    });
}

fn two_frame_gif() -> Vec<u8> {
    let palette = [255, 0, 0, 0, 0, 255];
    let mut out = Vec::new();
    {
        let mut e = gif::Encoder::new(&mut out, 2, 1, &palette).unwrap();
        e.set_repeat(gif::Repeat::Infinite).unwrap();
        for index in [0u8, 1] {
            let frame = gif::Frame {
                width: 2,
                height: 1,
                delay: 10,
                buffer: std::borrow::Cow::Owned(vec![index; 2]),
                ..gif::Frame::default()
            };
            e.write_frame(&frame).unwrap();
        }
    }
    out
}

#[test]
fn showing_another_frame_redraws_an_animated_image() {
    on_big_stack(|| {
        let mut s = shown(
            "(insert \"a\" (propertize \"b\" 'display (setq anim (list 'image :type 'gif :index 0 :ascent 100))) \"c\")",
        );
        let data = s.lisp.unibyte_string(&two_frame_gif());
        let symbol = s.lisp.intern("frames");
        s.lisp.set_plain_value(symbol, data);
        s.eval(
            "(progn (plist-put (cdr anim) :data frames) (force-window-update (current-buffer)))",
        );
        s.lisp.redisplay().unwrap();
        let (x, base) = (s.glyph_left(0, 1), s.baseline(0));
        assert_eq!(s.row(0).area_glyphs(TEXT_AREA)[1].kind, GlyphType::Image);
        assert_eq!(s.pixel(x, base - 1), [255, 0, 0]);
        assert_eq!(
            s.eval(
                "(let ((m (image-metadata anim))) (list (plist-get m 'count) (plist-get m 'delay)))"
            ),
            "(2 0.1)"
        );
        s.eval("(progn (plist-put (cdr anim) :index 1) (force-window-update (current-buffer)))");
        s.lisp.redisplay().unwrap();
        assert_eq!(s.pixel(x, base - 1), [0, 0, 255]);
        assert_eq!(s.pixel(x + 1, base - 1), [0, 0, 255]);
        s.eval("(progn (plist-put (cdr anim) :index 0) (force-window-update (current-buffer)))");
        s.lisp.redisplay().unwrap();
        assert_eq!(s.pixel(x, base - 1), [255, 0, 0]);
    });
}
