use std::cell::RefCell;

use emacs_core::dispextern::{
    DrawFringeBitmapParams, DrawGlyphsFace, EmacsGc, EmacsRectangle, Glyph, GlyphRowArea,
    GlyphRowRef, GlyphString, GlyphType, GlyphU, RedisplayInterface,
};
use emacs_core::font::FontMetrics;
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_core::value::Value;
use emacs_core::xdisp::draw::{
    gui_clear_end_of_line, gui_fix_overlapping_area, gui_get_glyph_overhangs, gui_insert_glyphs,
    gui_write_glyphs,
};
use emacs_core::xfaces::{
    CHILD_FRAME_BORDER_FACE_ID, FaceBoxType, FaceUnderlineType, INTERNAL_BORDER_FACE_ID,
    VERTICAL_BORDER_FACE_ID, WINDOW_DIVIDER_FACE_ID, WINDOW_DIVIDER_FIRST_PIXEL_FACE_ID,
    WINDOW_DIVIDER_LAST_PIXEL_FACE_ID,
};

use crate::pgtkterm::{frame_key, with_output};
use crate::surface::{Rect, Surface};

const HIGHLIGHT_COLOR_DARK_BOOST_LIMIT: i64 = 48000;
const CORNER_BOTTOM_RIGHT: i32 = 0;
const CORNER_BOTTOM_LEFT: i32 = 1;
const CORNER_TOP_LEFT: i32 = 2;
const CORNER_TOP_RIGHT: i32 = 3;
const CORNER_LAST: i32 = 4;
const GLYPHLESS_DISPLAY_THIN_SPACE: u8 = 0;
const GLYPHLESS_DISPLAY_ACRONYM: u8 = 2;
const GLYPHLESS_DISPLAY_HEX_CODE: u8 = 3;

pub static PGTK_REDISPLAY_INTERFACE: RedisplayInterface = RedisplayInterface {
    write_glyphs: Some(gui_write_glyphs),
    insert_glyphs: Some(gui_insert_glyphs),
    clear_end_of_line: Some(gui_clear_end_of_line),
    after_update_window_line_hook: Some(pgtk_after_update_window_line),
    flush_display: Some(pgtk_flush_display),
    get_glyph_overhangs: Some(gui_get_glyph_overhangs),
    fix_overlapping_area: Some(gui_fix_overlapping_area),
    draw_fringe_bitmap: Some(pgtk_draw_fringe_bitmap),
    define_fringe_bitmap: Some(pgtk_define_fringe_bitmap),
    destroy_fringe_bitmap: Some(pgtk_destroy_fringe_bitmap),
    compute_glyph_string_overhangs: Some(pgtk_compute_glyph_string_overhangs),
    draw_glyph_string: Some(pgtk_draw_glyph_string),
    clear_frame_area: Some(pgtk_clear_frame_area),
    clear_under_internal_border: Some(pgtk_clear_under_internal_border),
    draw_vertical_window_border: Some(pgtk_draw_vertical_window_border),
    draw_window_divider: Some(pgtk_draw_window_divider),
    ..RedisplayInterface::NONE
};

struct FringeBmp {
    width: i32,
    height: i32,
    rows: Vec<u16>,
}

thread_local! {
    static FRINGE_BMP: RefCell<Vec<Option<FringeBmp>>> = const { RefCell::new(Vec::new()) };
}

fn with_surface<T>(lisp: &Lisp, f: Value, g: impl FnOnce(&mut Surface) -> T) -> Option<T> {
    let key = frame_key(lisp, f) as u64;
    crate::with_display(|d| d.surface_mut(key).map(g)).flatten()
}

fn begin_cr_clip(lisp: &Lisp, f: Value) {
    with_surface(lisp, f, Surface::save);
}

fn end_cr_clip(lisp: &Lisp, f: Value) {
    with_surface(lisp, f, Surface::restore);
}

fn rect(r: &EmacsRectangle) -> Rect {
    Rect::new(r.x, r.y, r.width, r.height)
}

fn set_clip_rectangles(lisp: &Lisp, f: Value, rects: &[EmacsRectangle]) {
    if rects.is_empty() {
        return;
    }
    let rects: Vec<Rect> = rects.iter().map(rect).collect();
    with_surface(lisp, f, |s| s.clip_rectangles(&rects));
}

#[allow(clippy::too_many_arguments)]
pub fn pgtk_fill_rectangle(
    lisp: &Lisp,
    f: Value,
    color: u64,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    respect_alpha_background: bool,
) {
    with_surface(lisp, f, |s| {
        s.fill_rectangle(
            color as u32,
            Rect::new(x, y, width, height),
            respect_alpha_background,
        )
    });
}

#[allow(clippy::too_many_arguments)]
fn pgtk_draw_rectangle(
    lisp: &Lisp,
    f: Value,
    color: u64,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    respect_alpha_background: bool,
) {
    with_surface(lisp, f, |s| {
        s.stroke_rectangle(
            color as u32,
            Rect::new(x, y, width, height),
            respect_alpha_background,
        )
    });
}

pub fn pgtk_clear_area(lisp: &Lisp, f: Value, x: i32, y: i32, width: i32, height: i32) {
    let background = with_output(lisp, f, |o| o.background_color) as u64;
    pgtk_fill_rectangle(lisp, f, background, x, y, width, height, true);
}

fn pgtk_clear_frame_area(lisp: &mut Lisp, f: Value, x: i32, y: i32, width: i32, height: i32) {
    pgtk_clear_area(lisp, f, x, y, width, height);
}

fn face_colors(lisp: &Lisp, f: Value, face: usize) -> (u64, u64, i32) {
    let face = lisp.face_from_id(f, face).expect("realized face");
    (face.foreground, face.background, face.stipple)
}

#[allow(clippy::too_many_arguments)]
fn fill_background_by_face(
    lisp: &Lisp,
    f: Value,
    face: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let (_, background, stipple) = face_colors(lisp, f, face);
    begin_cr_clip(lisp, f);
    with_surface(lisp, f, |s| {
        s.clip_rectangles(&[Rect::new(x, y, width, height)]);
        s.fill_rectangle(background as u32, Rect::new(x, y, width, height), true);
    });
    if stipple != 0 {
        unreachable!("face stipples need image.c bitmaps");
    }
    end_cr_clip(lisp, f);
}

fn fill_background(lisp: &Lisp, s: &GlyphString, x: i32, y: i32, width: i32, height: i32) {
    fill_background_by_face(
        lisp,
        s.f,
        s.face.expect("glyph string face"),
        x,
        y,
        width,
        height,
    );
}

fn first_glyph(lisp: &Lisp, s: &GlyphString) -> Glyph {
    let r = lisp.glyph_row(s.row);
    r.glyphs[r.area[s.area] + s.first_glyph]
}

fn row_glyph(lisp: &Lisp, s: &GlyphString, i: usize) -> Glyph {
    let r = lisp.glyph_row(s.row);
    r.glyphs[r.area[s.area] + i]
}

fn face_of(lisp: &Lisp, s: &GlyphString) -> emacs_core::xfaces::Face {
    lisp.face_from_id(s.f, s.face.expect("glyph string face"))
        .expect("realized face")
        .clone()
}

fn pgtk_set_cursor_gc(lisp: &mut Lisp, s: &mut GlyphString) {
    let face = face_of(lisp, s);
    let fr = lisp.frame(s.f);
    let (frame_font, frame_bg, frame_fg) = (
        fr.font,
        fr.background_pixel as u64,
        fr.foreground_pixel as u64,
    );
    let (xgcv_fg, xgcv_bg, cursor_color, cursor_fg) = with_output(lisp, s.f, |o| {
        (
            o.cursor_xgcv_foreground as u64,
            o.cursor_xgcv_background as u64,
            o.cursor_color as u64,
            o.cursor_foreground_color as u64,
        )
    });
    if s.font == frame_font
        && face.background == frame_bg
        && face.foreground == frame_fg
        && s.cmp.is_none()
    {
        s.xgcv = EmacsGc {
            foreground: xgcv_fg,
            background: xgcv_bg,
        };
    } else {
        let mut xgcv = EmacsGc {
            background: cursor_color,
            foreground: face.background,
        };
        if xgcv.foreground == xgcv.background {
            xgcv.foreground = face.foreground;
        }
        if xgcv.foreground == xgcv.background {
            xgcv.foreground = cursor_fg;
        }
        if xgcv.foreground == xgcv.background {
            xgcv.foreground = face.foreground;
        }
        if xgcv.background == face.background && xgcv.foreground == face.foreground {
            xgcv.background = face.foreground;
            xgcv.foreground = face.background;
        }
        s.xgcv = xgcv;
    }
}

fn pgtk_set_mouse_face_gc(lisp: &mut Lisp, s: &mut GlyphString) {
    let id = s.face.expect("glyph string face");
    lisp.prepare_face_for_display(s.f, id);
    let face = face_of(lisp, s);
    s.xgcv = EmacsGc {
        foreground: face.foreground,
        background: face.background,
    };
}

fn pgtk_set_mode_line_face_gc(lisp: &Lisp, s: &mut GlyphString) {
    let face = face_of(lisp, s);
    s.xgcv = EmacsGc {
        foreground: face.foreground,
        background: face.background,
    };
}

fn pgtk_set_glyph_string_gc(lisp: &mut Lisp, s: &mut GlyphString) {
    let id = s.face.expect("glyph string face");
    lisp.prepare_face_for_display(s.f, id);
    let face = face_of(lisp, s);
    match s.hl {
        DrawGlyphsFace::NormalText => {
            s.xgcv = EmacsGc {
                foreground: face.foreground,
                background: face.background,
            };
            s.stippled_p = face.stipple != 0;
        }
        DrawGlyphsFace::InverseVideo => {
            pgtk_set_mode_line_face_gc(lisp, s);
            s.stippled_p = face.stipple != 0;
        }
        DrawGlyphsFace::Cursor => {
            pgtk_set_cursor_gc(lisp, s);
            s.stippled_p = false;
        }
        DrawGlyphsFace::MouseFace => {
            pgtk_set_mouse_face_gc(lisp, s);
            s.stippled_p = face.stipple != 0;
        }
        DrawGlyphsFace::ImageRaised | DrawGlyphsFace::ImageSunken => {
            s.xgcv = EmacsGc {
                foreground: face.foreground,
                background: face.background,
            };
            s.stippled_p = face.stipple != 0;
        }
    }
}

fn pgtk_set_glyph_string_clipping(lisp: &mut Lisp, arena: &[GlyphString], si: usize) {
    let rects = lisp.get_glyph_string_clip_rects(arena, si, 2);
    set_clip_rectangles(lisp, arena[si].f, &rects);
}

fn pgtk_set_glyph_string_clipping_exactly(
    lisp: &Lisp,
    arena: &mut [GlyphString],
    src: usize,
    dst: usize,
) {
    let r = EmacsRectangle {
        x: arena[src].x,
        y: arena[src].y,
        width: arena[src].width,
        height: arena[src].height,
    };
    arena[dst].clip[0] = r;
    arena[dst].num_clips = 1;
    set_clip_rectangles(lisp, arena[src].f, &[r]);
}

fn text_extents(lisp: &mut Lisp, font: Value, codes: &[u32]) -> FontMetrics {
    let extents = lisp
        .font_object(font)
        .driver
        .expect("font driver")
        .text_extents;
    extents(lisp, font, codes)
}

fn pgtk_compute_glyph_string_overhangs(lisp: &mut Lisp, arena: &mut [GlyphString], si: usize) {
    let first = first_glyph(lisp, &arena[si]);
    let s = &arena[si];
    if s.cmp.is_none() && matches!(first.kind, GlyphType::Char | GlyphType::Composite) {
        let metrics = if first.kind == GlyphType::Char {
            let codes = s.char2b[..s.nchars].to_vec();
            text_extents(lisp, s.font, &codes)
        } else {
            let gstring = lisp.composition_gstring_from_id(s.cmp_id as usize);
            let mut metrics = FontMetrics::default();
            let (from, to) = (s.cmp_from as usize, s.cmp_to as usize);
            lisp.composition_gstring_width(gstring, from, to, Some(&mut metrics));
            metrics
        };
        let s = &mut arena[si];
        s.right_overhang = if metrics.rbearing > metrics.width {
            metrics.rbearing - metrics.width
        } else {
            0
        };
        s.left_overhang = if metrics.lbearing < 0 {
            -metrics.lbearing
        } else {
            0
        };
    } else if let Some(cmp) = s.cmp {
        let c = &lisp.compositions.table[cmp];
        let (right, left) = (c.rbearing - c.pixel_width, -c.lbearing);
        arena[si].right_overhang = right;
        arena[si].left_overhang = left;
    }
}

fn pgtk_clear_glyph_string_rect(lisp: &Lisp, s: &GlyphString, x: i32, y: i32, w: i32, h: i32) {
    let first = first_glyph(lisp, s);
    pgtk_fill_rectangle(
        lisp,
        s.f,
        s.xgcv.background,
        x,
        y,
        w,
        h,
        first.kind != GlyphType::Stretch || s.hl != DrawGlyphsFace::Cursor,
    );
}

fn font_too_high(lisp: &Lisp, font: Value) -> bool {
    let fo = lisp.font_object(font);
    fo.pixel_size > 0 && fo.ascent + fo.descent > 3 * fo.pixel_size
}

fn pgtk_draw_glyph_string_background(lisp: &Lisp, s: &mut GlyphString, force_p: bool) {
    if s.background_filled_p {
        return;
    }
    let face = face_of(lisp, s);
    let box_line_width = face.box_horizontal_line_width.max(0);
    if s.stippled_p {
        fill_background(
            lisp,
            s,
            s.x,
            s.y + box_line_width,
            s.background_width,
            s.height - 2 * box_line_width,
        );
        s.background_filled_p = true;
    } else if lisp.font_object(s.font).height < s.height - 2 * box_line_width
        || font_too_high(lisp, s.font)
        || s.font_not_found_p
        || s.extends_to_end_of_line_p
        || force_p
    {
        pgtk_clear_glyph_string_rect(
            lisp,
            s,
            s.x,
            s.y + box_line_width,
            s.background_width,
            s.height - 2 * box_line_width,
        );
        s.background_filled_p = true;
    }
}

fn font_draw(
    lisp: &mut Lisp,
    s: &mut GlyphString,
    from: usize,
    to: usize,
    x: i32,
    y: i32,
    with_background: bool,
) {
    let draw = lisp
        .font_object(s.font)
        .driver
        .expect("font driver")
        .draw
        .expect("font driver draw");
    draw(lisp, s, from, to, x, y, with_background);
}

fn box_start_x(lisp: &Lisp, s: &GlyphString) -> i32 {
    let first = first_glyph(lisp, s);
    match s.face.and_then(|id| lisp.face_from_id(s.f, id)) {
        Some(face) if face.box_type != FaceBoxType::NoBox && first.left_box_line_p => {
            s.x + face.box_vertical_line_width.max(0)
        }
        _ => s.x,
    }
}

fn pgtk_draw_glyph_string_foreground(lisp: &mut Lisp, s: &mut GlyphString) {
    let mut x = box_start_x(lisp, s);
    if s.font_not_found_p {
        let foreground = face_of(lisp, s).foreground;
        for i in 0..s.nchars {
            let g = row_glyph(lisp, s, s.first_glyph + i);
            pgtk_draw_rectangle(
                lisp,
                s.f,
                foreground,
                x,
                s.y,
                i32::from(g.pixel_width) - 1,
                s.height - 1,
                false,
            );
            x += i32::from(g.pixel_width);
        }
    } else {
        let fo = lisp.font_object(s.font);
        let mut boff = fo.baseline_offset;
        if fo.vertical_centering {
            boff = lisp.vcenter_baseline_offset(s.font, s.f) - boff;
        }
        let y = s.ybase - boff;
        let nchars = s.nchars;
        if s.for_overlaps != 0 || (s.background_filled_p && s.hl != DrawGlyphsFace::Cursor) {
            font_draw(lisp, s, 0, nchars, x, y, false);
        } else {
            font_draw(lisp, s, 0, nchars, x, y, true);
        }
        if face_of(lisp, s).overstrike {
            font_draw(lisp, s, 0, nchars, x + 1, y, false);
        }
    }
}

fn pgtk_draw_composite_glyph_string_foreground(lisp: &mut Lisp, s: &mut GlyphString) {
    let x0 = box_start_x(lisp, s);
    let mut x = x0;
    let first = first_glyph(lisp, s);
    let overstrike = s
        .face
        .and_then(|id| lisp.face_from_id(s.f, id))
        .is_some_and(|face| face.overstrike);
    if s.font_not_found_p {
        if s.cmp_from == 0 {
            let foreground = face_of(lisp, s).foreground;
            pgtk_draw_rectangle(
                lisp,
                s.f,
                foreground,
                x,
                s.y,
                s.width - 1,
                s.height - 1,
                false,
            );
        }
    } else if first.u.val() & 1 == 0 {
        let cmp = s.cmp.expect("static composition");
        let y = s.ybase;
        let from = s.cmp_from as usize;
        for j in from..from + s.nchars {
            if lisp.composition_glyph(cmp, j) != i32::from(b'\t') {
                let offsets = &lisp.compositions.table[cmp].offsets;
                let xx = x + i32::from(offsets[j * 2]);
                let yy = y - i32::from(offsets[j * 2 + 1]);
                font_draw(lisp, s, j, j + 1, xx, yy, false);
                if overstrike {
                    font_draw(lisp, s, j, j + 1, xx + 1, yy, false);
                }
            }
        }
    } else {
        let gstring = lisp.composition_gstring_from_id(s.cmp_id as usize);
        let y = s.ybase;
        let mut width = 0;
        let mut j = s.cmp_from as usize;
        let mut i = j;
        while i < s.cmp_to as usize {
            let glyph = lisp.lgstring_glyph(gstring, i);
            if lisp.lglyph_adjustment(glyph).is_nil() {
                width += lisp.lglyph_int(glyph, emacs_core::composite::LGLYPH_IX_WIDTH) as i32;
            } else {
                if j < i {
                    font_draw(lisp, s, j, i, x, y, false);
                    if overstrike {
                        font_draw(lisp, s, j, i, x + 1, y, false);
                    }
                    x += width;
                }
                let xoff = lisp.lglyph_adjust_part(glyph, 0) as i32;
                let yoff = lisp.lglyph_adjust_part(glyph, 1) as i32;
                let wadjust = lisp.lglyph_adjust_part(glyph, 2) as i32;
                font_draw(lisp, s, i, i + 1, x + xoff, y + yoff, false);
                if overstrike {
                    font_draw(lisp, s, i, i + 1, x + xoff + 1, y + yoff, false);
                }
                x += wadjust;
                j = i + 1;
                width = 0;
            }
            i += 1;
        }
        if j < i {
            font_draw(lisp, s, j, i, x, y, false);
            if overstrike {
                font_draw(lisp, s, j, i, x + 1, y, false);
            }
        }
    }
}

fn pgtk_draw_glyphless_glyph_string_foreground(lisp: &mut Lisp, s: &mut GlyphString) {
    let mut x = box_start_x(lisp, s);
    let saved = std::mem::replace(&mut s.char2b, vec![0; 8]);
    let foreground = face_of(lisp, s).foreground;
    let table = lisp.find_symbol_value(sym::GLYPHLESS_CHAR_DISPLAY);
    for i in 0..s.nchars {
        let glyph = row_glyph(lisp, s, s.first_glyph + i);
        let GlyphU::Glyphless {
            method,
            for_no_font,
            len,
            ch,
        } = glyph.u
        else {
            unreachable!("glyphless glyph string over a non-glyphless glyph");
        };
        let len = usize::from(len);
        let mut text: Option<Vec<u8>> = None;
        if method == GLYPHLESS_DISPLAY_ACRONYM {
            if len > 0 && lisp.is_char_table(table) && !lisp.char_table(table).extras.is_empty() {
                let mut acronym = if !for_no_font {
                    lisp.char_table_ref(table, ch)
                } else {
                    lisp.char_table(table).extras[0]
                };
                if acronym.is_cons() {
                    acronym = lisp.xcar(acronym);
                }
                if acronym.is_string() {
                    text = Some(lisp.string_bytes(acronym).to_vec());
                }
            }
        } else if method == GLYPHLESS_DISPLAY_HEX_CODE {
            text = Some(if ch < 0x10000 {
                format!("{ch:04X}").into_bytes()
            } else {
                format!("{ch:06X}").into_bytes()
            });
        }
        if let Some(text) = text {
            let upper_len = len.div_ceil(2);
            for j in 0..len {
                let c = i32::from(text.get(j).copied().unwrap_or(0));
                let code = lisp.font_encode_char(s.font, c) & 0xFFFF;
                s.char2b[j] = code;
            }
            let (upper_xoff, upper_yoff, lower_xoff, lower_yoff) = match glyph.slice {
                emacs_core::dispextern::GlyphSlice::Glyphless {
                    upper_xoff,
                    upper_yoff,
                    lower_xoff,
                    lower_yoff,
                } => (
                    i32::from(upper_xoff),
                    i32::from(upper_yoff),
                    i32::from(lower_xoff),
                    i32::from(lower_yoff),
                ),
                _ => (0, 0, 0, 0),
            };
            font_draw(
                lisp,
                s,
                0,
                upper_len,
                x + upper_xoff,
                s.ybase + upper_yoff,
                false,
            );
            font_draw(
                lisp,
                s,
                upper_len,
                len,
                x + lower_xoff,
                s.ybase + lower_yoff,
                false,
            );
        }
        if method != GLYPHLESS_DISPLAY_THIN_SPACE {
            pgtk_draw_rectangle(
                lisp,
                s.f,
                foreground,
                x,
                s.ybase - i32::from(glyph.ascent),
                i32::from(glyph.pixel_width) - 1,
                i32::from(glyph.ascent) + i32::from(glyph.descent) - 1,
                false,
            );
        }
        x += i32::from(glyph.pixel_width);
    }
    s.char2b = saved;
}

fn query_color(pixel: u64) -> (i64, i64, i64) {
    (
        (((pixel >> 16) & 0xff) * 257) as i64,
        (((pixel >> 8) & 0xff) * 257) as i64,
        ((pixel & 0xff) * 257) as i64,
    )
}

fn pgtk_compute_lighter_color(pixel: u64, factor: f64, delta: i64) -> u64 {
    let (red, green, blue) = query_color(pixel);
    let scale = |v: i64| (factor * v as f64).min(65535.0) as i64;
    let (mut nr, mut ng, mut nb) = (scale(red), scale(green), scale(blue));
    let bright = (2 * red + 3 * green + blue) / 6;
    if bright < HIGHLIGHT_COLOR_DARK_BOOST_LIMIT {
        let dimness = 1.0 - bright as f64 / HIGHLIGHT_COLOR_DARK_BOOST_LIMIT as f64;
        let min_delta = (delta as f64 * dimness * factor / 2.0) as i64;
        if factor < 1.0 {
            nr = (nr - min_delta).max(0);
            ng = (ng - min_delta).max(0);
            nb = (nb - min_delta).max(0);
        } else {
            nr = (min_delta + nr).min(0xffff);
            ng = (min_delta + ng).min(0xffff);
            nb = (min_delta + nb).min(0xffff);
        }
    }
    let mut new = ((nr >> 8) << 16 | (ng >> 8) << 8 | (nb >> 8)) as u64;
    if new == pixel {
        nr = (delta + red).min(0xffff);
        ng = (delta + green).min(0xffff);
        nb = (delta + blue).min(0xffff);
        new = ((nr >> 8) << 16 | (ng >> 8) << 8 | (nb >> 8)) as u64;
    }
    new
}

#[allow(clippy::too_many_arguments)]
fn pgtk_fill_trapezoid_for_relief(
    lisp: &Lisp,
    f: Value,
    color: u64,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    top_p: bool,
) {
    let points = [
        ((if top_p { x } else { x + height }) as f32, y as f32),
        (x as f32, (y + height) as f32),
        (
            (if top_p { x + width - height } else { x + width }) as f32,
            (y + height) as f32,
        ),
        ((x + width) as f32, y as f32),
    ];
    begin_cr_clip(lisp, f);
    with_surface(lisp, f, |s| s.fill_polygon(color as u32, &points));
    end_cr_clip(lisp, f);
}

fn push_arc(builder: &mut tiny_skia::PathBuilder, xc: f64, yc: f64, r: f64, a0: f64, a1: f64) {
    let start = (xc + r * a0.cos(), yc + r * a0.sin());
    builder.line_to(start.0 as f32, start.1 as f32);
    let k = 4.0 / 3.0 * ((a1 - a0) / 4.0).tan();
    let end = (xc + r * a1.cos(), yc + r * a1.sin());
    let c1 = (start.0 - k * r * a0.sin(), start.1 + k * r * a0.cos());
    let c2 = (end.0 + k * r * a1.sin(), end.1 - k * r * a1.cos());
    builder.cubic_to(
        c1.0 as f32,
        c1.1 as f32,
        c2.0 as f32,
        c2.1 as f32,
        end.0 as f32,
        end.1 as f32,
    );
}

#[allow(clippy::too_many_arguments)]
fn pgtk_erase_corners_for_relief(
    lisp: &Lisp,
    f: Value,
    color: u64,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    radius: f64,
    margin: f64,
    corners: i32,
) {
    let mut builder = tiny_skia::PathBuilder::new();
    for i in 0..CORNER_LAST {
        if corners & (1 << i) == 0 {
            continue;
        }
        let (xm, xc) = if i == CORNER_TOP_LEFT || i == CORNER_BOTTOM_LEFT {
            let xm = f64::from(x) - margin;
            (xm, xm + radius)
        } else {
            let xm = f64::from(x + width) + margin;
            (xm, xm - radius)
        };
        let (ym, yc) = if i == CORNER_TOP_LEFT || i == CORNER_TOP_RIGHT {
            let ym = f64::from(y) - margin;
            (ym, ym + radius)
        } else {
            let ym = f64::from(y + height) + margin;
            (ym, ym - radius)
        };
        builder.move_to(xm as f32, ym as f32);
        let a0 = f64::from(i) * std::f64::consts::FRAC_PI_2;
        push_arc(
            &mut builder,
            xc,
            yc,
            radius,
            a0,
            a0 + std::f64::consts::FRAC_PI_2,
        );
        builder.close();
    }
    let path = builder.finish();
    begin_cr_clip(lisp, f);
    with_surface(lisp, f, |s| {
        s.clip_path(path.as_ref());
        s.fill_rectangle(color as u32, Rect::new(x, y, width, height), false);
    });
    end_cr_clip(lisp, f);
}

fn pgtk_setup_relief_colors(lisp: &Lisp, s: &GlyphString) {
    let face = face_of(lisp, s);
    let first = first_glyph(lisp, s);
    let color = if face.use_box_color_for_shadows_p {
        face.box_color
    } else if first.kind == GlyphType::Image {
        unreachable!("image glyphs need image.c");
    } else {
        s.xgcv.background
    };
    with_output(lisp, s.f, |o| {
        if !o.relief_background_valid_p || o.relief_background != color as i64 {
            o.relief_background_valid_p = true;
            o.relief_background = color as i64;
            o.white_relief_pixel = pgtk_compute_lighter_color(color, 1.2, 0x8000) as i64;
            o.black_relief_pixel = pgtk_compute_lighter_color(color, 0.6, 0x4000) as i64;
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn pgtk_draw_relief_rect(
    lisp: &Lisp,
    f: Value,
    left_x: i32,
    top_y: i32,
    right_x: i32,
    bottom_y: i32,
    hwidth: i32,
    vwidth: i32,
    raised_p: bool,
    top_p: bool,
    bot_p: bool,
    left_p: bool,
    right_p: bool,
    clip_rect: &EmacsRectangle,
) {
    let mut corners = 0;
    begin_cr_clip(lisp, f);
    let (white, black) = with_output(lisp, f, |o| {
        (o.white_relief_pixel as u64, o.black_relief_pixel as u64)
    });
    let (top_left_color, bottom_right_color) = if raised_p {
        (white, black)
    } else {
        (black, white)
    };
    set_clip_rectangles(lisp, f, std::slice::from_ref(clip_rect));
    if left_p {
        pgtk_fill_rectangle(
            lisp,
            f,
            top_left_color,
            left_x,
            top_y,
            vwidth,
            bottom_y + 1 - top_y,
            false,
        );
        if top_p {
            corners |= 1 << CORNER_TOP_LEFT;
        }
        if bot_p {
            corners |= 1 << CORNER_BOTTOM_LEFT;
        }
    }
    if right_p {
        pgtk_fill_rectangle(
            lisp,
            f,
            bottom_right_color,
            right_x + 1 - vwidth,
            top_y,
            vwidth,
            bottom_y + 1 - top_y,
            false,
        );
        if top_p {
            corners |= 1 << CORNER_TOP_RIGHT;
        }
        if bot_p {
            corners |= 1 << CORNER_BOTTOM_RIGHT;
        }
    }
    if top_p {
        if !right_p {
            pgtk_fill_rectangle(
                lisp,
                f,
                top_left_color,
                left_x,
                top_y,
                right_x + 1 - left_x,
                hwidth,
                false,
            );
        } else {
            pgtk_fill_trapezoid_for_relief(
                lisp,
                f,
                top_left_color,
                left_x,
                top_y,
                right_x + 1 - left_x,
                hwidth,
                true,
            );
        }
    }
    if bot_p {
        if !left_p {
            pgtk_fill_rectangle(
                lisp,
                f,
                bottom_right_color,
                left_x,
                bottom_y + 1 - hwidth,
                right_x + 1 - left_x,
                hwidth,
                false,
            );
        } else {
            pgtk_fill_trapezoid_for_relief(
                lisp,
                f,
                bottom_right_color,
                left_x,
                bottom_y + 1 - hwidth,
                right_x + 1 - left_x,
                hwidth,
                false,
            );
        }
    }
    if left_p && vwidth > 1 {
        pgtk_fill_rectangle(
            lisp,
            f,
            bottom_right_color,
            left_x,
            top_y,
            1,
            bottom_y + 1 - top_y,
            false,
        );
    }
    if top_p && hwidth > 1 {
        pgtk_fill_rectangle(
            lisp,
            f,
            bottom_right_color,
            left_x,
            top_y,
            right_x + 1 - left_x,
            1,
            false,
        );
    }
    if corners != 0 {
        let background = lisp.frame(f).background_pixel as u64;
        pgtk_erase_corners_for_relief(
            lisp,
            f,
            background,
            left_x,
            top_y,
            right_x - left_x + 1,
            bottom_y - top_y + 1,
            6.0,
            1.0,
            corners,
        );
    }
    end_cr_clip(lisp, f);
}

#[allow(clippy::too_many_arguments)]
fn pgtk_draw_box_rect(
    lisp: &Lisp,
    s: &mut GlyphString,
    left_x: i32,
    top_y: i32,
    right_x: i32,
    bottom_y: i32,
    hwidth: i32,
    vwidth: i32,
    left_p: bool,
    right_p: bool,
    clip_rect: &EmacsRectangle,
) {
    begin_cr_clip(lisp, s.f);
    let foreground_backup = s.xgcv.foreground;
    s.xgcv.foreground = face_of(lisp, s).box_color;
    set_clip_rectangles(lisp, s.f, std::slice::from_ref(clip_rect));
    let fg = s.xgcv.foreground;
    pgtk_fill_rectangle(
        lisp,
        s.f,
        fg,
        left_x,
        top_y,
        right_x - left_x + 1,
        hwidth,
        false,
    );
    if left_p {
        pgtk_fill_rectangle(
            lisp,
            s.f,
            fg,
            left_x,
            top_y,
            vwidth,
            bottom_y - top_y + 1,
            false,
        );
    }
    pgtk_fill_rectangle(
        lisp,
        s.f,
        fg,
        left_x,
        bottom_y - hwidth + 1,
        right_x - left_x + 1,
        hwidth,
        false,
    );
    if right_p {
        pgtk_fill_rectangle(
            lisp,
            s.f,
            fg,
            right_x - vwidth + 1,
            top_y,
            vwidth,
            bottom_y - top_y + 1,
            false,
        );
    }
    s.xgcv.foreground = foreground_backup;
    end_cr_clip(lisp, s.f);
}

fn pgtk_draw_glyph_string_box(lisp: &mut Lisp, arena: &mut [GlyphString], si: usize) {
    let s = &arena[si];
    let (full_width_p, pseudo) = (
        lisp.glyph_row(s.row).full_width_p,
        lisp.window(s.w).flags.pseudo_window_p,
    );
    let last_x = if full_width_p && !pseudo {
        lisp.window_right_edge_x(s.w) as i32
    } else {
        lisp.window_box_right(s.w, emacs_core::xdisp::draw::glyph_row_area(s.area)) as i32
    };
    let last_glyph = if s.cmp.is_some() {
        first_glyph(lisp, s)
    } else {
        row_glyph(lisp, s, s.first_glyph + s.nchars.saturating_sub(1))
    };
    let face = face_of(lisp, s);
    let vwidth = face.box_vertical_line_width.abs();
    let hwidth = face.box_horizontal_line_width.abs();
    let raised_p = face.box_type == FaceBoxType::RaisedBox;
    let left_x = s.x;
    let right_x = if full_width_p && s.extends_to_end_of_line_p {
        last_x - 1
    } else {
        last_x.min(s.x + s.background_width) - 1
    };
    let top_y = s.y;
    let bottom_y = top_y + s.height - 1;
    let first = first_glyph(lisp, s);
    let left_p = first.left_box_line_p
        || (s.hl == DrawGlyphsFace::MouseFace && s.prev.is_none_or(|p| arena[p].hl != s.hl));
    let right_p = last_glyph.right_box_line_p
        || (s.hl == DrawGlyphsFace::MouseFace && s.next.is_none_or(|n| arena[n].hl != s.hl));
    let clip_rect = lisp.get_glyph_string_clip_rect(arena, si);
    if face.box_type == FaceBoxType::SimpleBox {
        pgtk_draw_box_rect(
            lisp,
            &mut arena[si],
            left_x,
            top_y,
            right_x,
            bottom_y,
            hwidth,
            vwidth,
            left_p,
            right_p,
            &clip_rect,
        );
    } else {
        pgtk_setup_relief_colors(lisp, &arena[si]);
        let f = arena[si].f;
        pgtk_draw_relief_rect(
            lisp, f, left_x, top_y, right_x, bottom_y, hwidth, vwidth, raised_p, true, true,
            left_p, right_p, &clip_rect,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn pgtk_draw_horizontal_wave(
    lisp: &Lisp,
    f: Value,
    color: u64,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    wave_length: i32,
) {
    let dx = wave_length as f32;
    let mut dy = (height - 1) as f32;
    let mut y = y;
    begin_cr_clip(lisp, f);
    with_surface(lisp, f, |s| {
        s.clip_rectangles(&[Rect::new(x, y, width, height)])
    });
    let mut xoffset = if x >= 0 {
        let o = x % (wave_length * 2);
        if o == 0 { wave_length * 2 } else { o }
    } else {
        x % (wave_length * 2) + wave_length * 2
    };
    let mut n = (width + xoffset) / wave_length + 1;
    if xoffset > wave_length {
        xoffset -= wave_length;
        n -= 1;
        y += height - 1;
        dy = -dy;
    }
    let mut points = vec![((x - xoffset) as f32 + 0.5, y as f32 + 0.5)];
    let mut current = points[0];
    for _ in 0..n.max(0) {
        current = (current.0 + dx, current.1 + dy);
        points.push(current);
        dy = -dy;
    }
    with_surface(lisp, f, |s| {
        s.stroke_polyline(color as u32, &points, 1.0, None)
    });
    end_cr_clip(lisp, f);
}

fn pgtk_draw_underwave(lisp: &Lisp, s: &GlyphString, color: u64, thickness: i32) {
    let wave_height = 3 * thickness;
    let wave_length = 2 * thickness;
    pgtk_draw_horizontal_wave(
        lisp,
        s.f,
        color,
        s.x,
        s.ybase - wave_height + 3,
        s.width,
        wave_height,
        wave_length,
    );
}

fn pgtk_draw_glyph_string_bg_rect(lisp: &Lisp, s: &GlyphString, x: i32, y: i32, w: i32, h: i32) {
    if s.stippled_p {
        fill_background(lisp, s, x, y, w, h);
    } else {
        pgtk_clear_glyph_string_rect(lisp, s, x, y, w, h);
    }
}

fn pgtk_draw_stretch_glyph_string(lisp: &mut Lisp, arena: &mut [GlyphString], si: usize) {
    let s = &arena[si];
    let reversed_p = lisp.glyph_row(s.row).reversed_p;
    if s.hl == DrawGlyphsFace::Cursor && !lisp.var_true(sym::X_STRETCH_CURSOR) {
        let mut background_width = s.background_width;
        let mut x = s.x;
        if !reversed_p {
            let left_x = lisp.window_box_left_offset(s.w, GlyphRowArea::Text) as i32;
            if x < left_x {
                background_width -= left_x - x;
                x = left_x;
            }
        } else {
            let right_x = lisp.window_box_right(s.w, GlyphRowArea::Text) as i32;
            if x + background_width > right_x {
                background_width -= x - right_x;
            }
            x += background_width;
        }
        let width = (lisp.frame(s.f).geometry.column_width as i32).min(background_width);
        if reversed_p {
            x -= width;
        }
        pgtk_draw_glyph_string_bg_rect(lisp, s, x, s.y, width, s.height);
        if width < background_width {
            let y = s.y;
            let w = background_width - width;
            let h = s.height;
            if !reversed_p {
                x += width;
            } else {
                x = s.x;
            }
            let row_mouse_face_p = lisp.glyph_row(s.row).mouse_face_p;
            let color = if row_mouse_face_p && lisp.cursor_in_mouse_face_p(s.w) {
                pgtk_set_mouse_face_gc(lisp, &mut arena[si]);
                arena[si].xgcv.foreground
            } else {
                face_of(lisp, &arena[si]).background
            };
            let f = arena[si].f;
            begin_cr_clip(lisp, f);
            let r = lisp.get_glyph_string_clip_rect(arena, si);
            set_clip_rectangles(lisp, f, &[r]);
            if face_of(lisp, &arena[si]).stipple != 0 {
                fill_background(lisp, &arena[si], x, y, w, h);
            } else {
                pgtk_fill_rectangle(lisp, f, color, x, y, w, h, true);
            }
            end_cr_clip(lisp, f);
        }
    } else if !s.background_filled_p {
        let mut background_width = s.background_width;
        let mut x = s.x;
        let text_left_x = lisp.window_box_left(s.w, GlyphRowArea::Text) as i32;
        let mode_line_p = lisp.glyph_row(s.row).mode_line_p;
        if s.area == emacs_core::dispextern::TEXT_AREA && x < text_left_x && !mode_line_p {
            background_width -= text_left_x - x;
            x = text_left_x;
        }
        if background_width > 0 {
            pgtk_draw_glyph_string_bg_rect(lisp, s, x, s.y, background_width, s.height);
        }
    }
    arena[si].background_filled_p = true;
}

#[allow(clippy::too_many_arguments)]
fn pgtk_draw_dash(
    lisp: &Lisp,
    f: Value,
    s: &GlyphString,
    foreground: u64,
    width: i32,
    segment: i32,
    offset: i32,
    thickness: i32,
) {
    begin_cr_clip(lisp, s.f);
    let y_center = s.ybase as f32 + offset as f32 + thickness as f32 / 2.0;
    with_surface(lisp, f, |surface| {
        surface.stroke_line(
            foreground as u32,
            (s.x as f32, y_center),
            ((s.x + width) as f32, y_center),
            thickness as f32,
            Some((segment as f32, s.x as f32)),
        )
    });
    end_cr_clip(lisp, f);
}

#[allow(clippy::too_many_arguments)]
fn pgtk_fill_underline(
    lisp: &Lisp,
    f: Value,
    s: &GlyphString,
    foreground: u64,
    style: FaceUnderlineType,
    position: i32,
    decoration_width: i32,
    thickness: i32,
) {
    let mut segment = thickness * 3;
    match style {
        FaceUnderlineType::Single | FaceUnderlineType::DoubleLine => pgtk_fill_rectangle(
            lisp,
            f,
            foreground,
            s.x,
            s.ybase + position,
            decoration_width,
            thickness,
            false,
        ),
        FaceUnderlineType::Dots | FaceUnderlineType::Dashes => {
            if style == FaceUnderlineType::Dots {
                segment = thickness;
            }
            pgtk_draw_dash(
                lisp,
                f,
                s,
                foreground,
                decoration_width,
                segment,
                position,
                thickness,
            );
        }
        FaceUnderlineType::NoUnderline | FaceUnderlineType::Wave => {
            unreachable!("pgtk_fill_underline called for {style:?}")
        }
    }
}

fn int_var(lisp: &Lisp, symbol: Value) -> i64 {
    lisp.find_symbol_value(symbol).as_fixnum().unwrap_or(0)
}

fn bool_var(lisp: &mut Lisp, name: &str) -> bool {
    let symbol = lisp.intern(name);
    !lisp.find_symbol_value(symbol).is_nil()
}

fn draw_successor_backgrounds(lisp: &mut Lisp, arena: &mut [GlyphString], si: usize) {
    let mut width = 0;
    let mut next = arena[si].next;
    while let Some(n) = next {
        if width >= arena[si].right_overhang {
            break;
        }
        if first_glyph(lisp, &arena[n]).kind != GlyphType::Image {
            let f = arena[n].f;
            begin_cr_clip(lisp, f);
            pgtk_set_glyph_string_gc(lisp, &mut arena[n]);
            pgtk_set_glyph_string_clipping(lisp, arena, n);
            if first_glyph(lisp, &arena[n]).kind == GlyphType::Stretch {
                pgtk_draw_stretch_glyph_string(lisp, arena, n);
            } else {
                pgtk_draw_glyph_string_background(lisp, &mut arena[n], true);
            }
            arena[n].num_clips = 0;
            end_cr_clip(lisp, f);
        }
        width += arena[n].width;
        next = arena[n].next;
    }
}

fn underline_metrics(lisp: &mut Lisp, arena: &mut [GlyphString], si: usize) {
    let s = &arena[si];
    let face = face_of(lisp, s);
    if face.underline == FaceUnderlineType::NoUnderline {
        return;
    }
    let font = lisp.font_for_underline_metrics(s);
    let font_metrics = (!font.is_nil()).then(|| {
        let fo = lisp.font_object(font);
        (fo.underline_thickness, fo.underline_position, fo.descent)
    });
    let scaled_thickness = match font_metrics {
        Some((t, _, _)) if t > 0 => t,
        _ => 1,
    };
    let mut thickness = if lisp.var_true(sym::UNDERLINE_LINE_SCALING_FLAG) {
        scaled_thickness
    } else {
        1
    };
    if face.underline == FaceUnderlineType::Wave {
        let color = if face.underline_defaulted_p {
            s.xgcv.foreground
        } else {
            face.underline_color
        };
        pgtk_draw_underwave(lisp, s, color, thickness);
        return;
    }
    let prev_face = s.prev.map(|p| face_of(lisp, &arena[p]));
    let mut position;
    match prev_face {
        Some(pf)
            if pf.underline != FaceUnderlineType::Wave
                && pf.underline != FaceUnderlineType::NoUnderline
                && pf.underline_at_descent_line_p == face.underline_at_descent_line_p
                && pf.underline_pixels_above_descent_line
                    == face.underline_pixels_above_descent_line =>
        {
            let p = &arena[s.prev.expect("previous glyph string")];
            thickness = p.underline_thickness;
            position = p.underline_position;
        }
        _ => {
            if bool_var(lisp, "x-underline-at-descent-line") || face.underline_at_descent_line_p {
                position = (s.height - thickness)
                    - (s.ybase - s.y)
                    - face.underline_pixels_above_descent_line;
            } else {
                position = match font_metrics {
                    Some((_, up, _))
                        if bool_var(lisp, "x-use-underline-position-properties") && up >= 0 =>
                    {
                        up
                    }
                    Some((_, _, descent)) => (descent + 1) / 2,
                    None => int_var(lisp, sym::UNDERLINE_MINIMUM_OFFSET) as i32,
                };
            }
            if face.underline_pixels_above_descent_line == 0 {
                position = position.max(int_var(lisp, sym::UNDERLINE_MINIMUM_OFFSET) as i32);
            }
        }
    }
    let s = &mut arena[si];
    if s.y + s.height <= s.ybase + position {
        position = (s.height - 1) - (s.ybase - s.y);
    }
    if s.y + s.height < s.ybase + position + thickness {
        thickness = (s.y + s.height) - (s.ybase + position);
    }
    s.underline_thickness = thickness;
    s.underline_position = position;
    let foreground = if face.underline_defaulted_p {
        s.xgcv.foreground
    } else {
        face.underline_color
    };
    let s = &arena[si];
    pgtk_fill_underline(
        lisp,
        s.f,
        s,
        foreground,
        face.underline,
        position,
        s.width,
        thickness,
    );
    if face.underline == FaceUnderlineType::DoubleLine {
        let position = position - thickness - 1;
        pgtk_fill_underline(
            lisp,
            s.f,
            s,
            foreground,
            face.underline,
            position,
            s.width,
            thickness,
        );
    }
}

fn redraw_overhang_foreground(lisp: &mut Lisp, arena: &mut [GlyphString], si: usize, other: usize) {
    let f = arena[si].f;
    let save = arena[other].hl;
    arena[other].hl = arena[si].hl;
    pgtk_set_glyph_string_gc(lisp, &mut arena[other]);
    with_surface(lisp, f, Surface::save);
    pgtk_set_glyph_string_clipping_exactly(lisp, arena, si, other);
    if first_glyph(lisp, &arena[other]).kind == GlyphType::Char {
        pgtk_draw_glyph_string_foreground(lisp, &mut arena[other]);
    } else {
        pgtk_draw_composite_glyph_string_foreground(lisp, &mut arena[other]);
    }
    with_surface(lisp, f, Surface::restore);
    arena[other].hl = save;
    arena[other].num_clips = 0;
}

pub fn pgtk_draw_glyph_string(lisp: &mut Lisp, arena: &mut [GlyphString], si: usize) {
    let mut relief_drawn_p = false;
    if arena[si].next.is_some() && arena[si].right_overhang != 0 && arena[si].for_overlaps == 0 {
        draw_successor_backgrounds(lisp, arena, si);
    }
    pgtk_set_glyph_string_gc(lisp, &mut arena[si]);
    let f = arena[si].f;
    begin_cr_clip(lisp, f);
    let face = face_of(lisp, &arena[si]);
    let kind = first_glyph(lisp, &arena[si]).kind;
    let s = &arena[si];
    if s.for_overlaps == 0
        && face.box_type != FaceBoxType::NoBox
        && matches!(kind, GlyphType::Char | GlyphType::Composite)
    {
        pgtk_set_glyph_string_clipping(lisp, arena, si);
        pgtk_draw_glyph_string_background(lisp, &mut arena[si], true);
        pgtk_draw_glyph_string_box(lisp, arena, si);
        pgtk_set_glyph_string_clipping(lisp, arena, si);
        relief_drawn_p = true;
    } else if s.clip_head.is_none()
        && s.clip_tail.is_none()
        && ((s.prev.is_some_and(|p| arena[p].hl != s.hl) && s.left_overhang != 0)
            || (s.next.is_some_and(|n| arena[n].hl != s.hl) && s.right_overhang != 0))
    {
        pgtk_set_glyph_string_clipping_exactly(lisp, arena, si, si);
    } else {
        pgtk_set_glyph_string_clipping(lisp, arena, si);
    }
    match kind {
        GlyphType::Image => unreachable!("image glyphs need image.c"),
        GlyphType::Xwidget => unreachable!("xwidgets are not supported"),
        GlyphType::Stretch => pgtk_draw_stretch_glyph_string(lisp, arena, si),
        GlyphType::Char => {
            if arena[si].for_overlaps != 0 {
                arena[si].background_filled_p = true;
            } else {
                pgtk_draw_glyph_string_background(lisp, &mut arena[si], false);
            }
            pgtk_draw_glyph_string_foreground(lisp, &mut arena[si]);
        }
        GlyphType::Composite => {
            let static_cmp = first_glyph(lisp, &arena[si]).u.val() & 1 == 0;
            if arena[si].for_overlaps != 0 || (arena[si].cmp_from > 0 && static_cmp) {
                arena[si].background_filled_p = true;
            } else {
                pgtk_draw_glyph_string_background(lisp, &mut arena[si], true);
            }
            pgtk_draw_composite_glyph_string_foreground(lisp, &mut arena[si]);
        }
        GlyphType::Glyphless => {
            if arena[si].for_overlaps != 0 {
                arena[si].background_filled_p = true;
            } else {
                pgtk_draw_glyph_string_background(lisp, &mut arena[si], true);
            }
            pgtk_draw_glyphless_glyph_string_foreground(lisp, &mut arena[si]);
        }
    }
    if arena[si].for_overlaps == 0 {
        let font = lisp.font_for_underline_metrics(&arena[si]);
        let scaled_thickness = if !font.is_nil() && lisp.font_object(font).underline_thickness > 0 {
            lisp.font_object(font).underline_thickness
        } else {
            1
        };
        if !relief_drawn_p && face.box_type != FaceBoxType::NoBox {
            pgtk_draw_glyph_string_box(lisp, arena, si);
        }
        underline_metrics(lisp, arena, si);
        let s = &arena[si];
        if face.overline_p {
            let h = if lisp.var_true(sym::OVERLINE_LINE_SCALING_FLAG) {
                scaled_thickness
            } else {
                1
            };
            let color = if face.overline_color_defaulted_p {
                s.xgcv.foreground
            } else {
                face.overline_color
            };
            pgtk_fill_rectangle(lisp, s.f, color, s.x, s.y, s.width, h, false);
        }
        if face.strike_through_p {
            let first = first_glyph(lisp, s);
            let glyph_y = s.ybase - i32::from(first.ascent);
            let glyph_height = i32::from(first.ascent) + i32::from(first.descent);
            let h = if lisp.var_true(sym::STRIKE_THROUGH_LINE_SCALING_FLAG) {
                scaled_thickness
            } else {
                1
            };
            let dy = (glyph_height - h) / 2;
            let color = if face.strike_through_color_defaulted_p {
                s.xgcv.foreground
            } else {
                face.strike_through_color
            };
            pgtk_fill_rectangle(lisp, s.f, color, s.x, glyph_y + dy, s.width, h, false);
        }
        let mut prev = arena[si].prev;
        while let Some(p) = prev {
            let s = &arena[si];
            if arena[p].hl != s.hl && arena[p].x + arena[p].width + arena[p].right_overhang > s.x {
                redraw_overhang_foreground(lisp, arena, si, p);
            }
            prev = arena[p].prev;
        }
        let mut next = arena[si].next;
        while let Some(n) = next {
            let s = &arena[si];
            if arena[n].hl != s.hl && arena[n].x - arena[n].left_overhang < s.x + s.width {
                redraw_overhang_foreground(lisp, arena, si, n);
                arena[n].clip_head = arena[si].next;
            }
            next = arena[n].next;
        }
    }
    let row = arena[si].row;
    if !lisp.glyph_row(row).stipple_p {
        lisp.glyph_row_mut(row).stipple_p = face.stipple != 0;
    }
    end_cr_clip(lisp, f);
    arena[si].num_clips = 0;
}

fn pgtk_after_update_window_line(lisp: &mut Lisp, w: Value, desired_row: GlyphRowRef) {
    let pseudo = lisp.window(w).flags.pseudo_window_p;
    let (mode_line_p, full_width_p, visible_height, y) = {
        let r = lisp.glyph_row(desired_row);
        (r.mode_line_p, r.full_width_p, r.visible_height, r.y)
    };
    if !mode_line_p && !pseudo {
        lisp.glyph_row_mut(desired_row).redraw_fringe_bitmaps_p = true;
    }
    let f = lisp.window(w).frame;
    let width = lisp.frame(f).geometry.internal_border_width as i32;
    if lisp.windows.windows_or_buffers_changed != 0
        && full_width_p
        && width != 0
        && visible_height > 0
    {
        let y = lisp.window_to_frame_pixel_y(w, i64::from(y.max(0))) as i32;
        let frame_width = lisp.frame(f).geometry.pixel_width as i32;
        pgtk_clear_frame_area(lisp, f, 0, y, width, visible_height);
        pgtk_clear_frame_area(lisp, f, frame_width - width, y, width, visible_height);
    }
}

fn pgtk_flush_display(_lisp: &mut Lisp, _f: Value) {}

pub fn pgtk_clear_under_internal_border(lisp: &mut Lisp, f: Value) {
    let border = lisp.frame(f).geometry.internal_border_width as i32;
    if border <= 0 {
        return;
    }
    let width = lisp.frame(f).geometry.pixel_width as i32;
    let height = lisp.frame(f).geometry.pixel_height as i32;
    let margin = lisp.frame_top_margin_height(f) as i32;
    let bottom_margin = if lisp.frame(f).tool_bar_position == sym::BOTTOM {
        lisp.frame(f).geometry.tool_bar_height as i32
    } else {
        0
    };
    let base = if lisp.frame(f).parent_frame.is_nil() {
        INTERNAL_BORDER_FACE_ID
    } else {
        CHILD_FRAME_BORDER_FACE_ID
    };
    let face_id = if lisp.find_symbol_value(sym::FACE_REMAPPING_ALIST).is_nil() {
        base
    } else {
        lisp.lookup_basic_face(None, f, base).unwrap_or(base)
    };
    if lisp.face_from_id(f, face_id).is_some() {
        fill_background_by_face(lisp, f, face_id, 0, margin, width, border);
        fill_background_by_face(lisp, f, face_id, 0, 0, border, height);
        fill_background_by_face(lisp, f, face_id, width - border, 0, border, height);
        fill_background_by_face(
            lisp,
            f,
            face_id,
            0,
            height - bottom_margin - border,
            width,
            border,
        );
    } else {
        pgtk_clear_area(lisp, f, 0, 0, border, height);
        pgtk_clear_area(lisp, f, 0, margin, width, border);
        pgtk_clear_area(lisp, f, width - border, 0, border, height);
        pgtk_clear_area(lisp, f, 0, height - bottom_margin - border, width, border);
    }
}

fn pgtk_draw_vertical_window_border(lisp: &mut Lisp, w: Value, x: i32, y0: i32, y1: i32) {
    let f = lisp.window(w).frame;
    let color = match lisp.face_from_id(f, VERTICAL_BORDER_FACE_ID) {
        Some(face) => face.foreground,
        None => lisp.frame(f).foreground_pixel as u64,
    };
    pgtk_fill_rectangle(lisp, f, color, x, y0, 1, y1 - y0, false);
}

fn pgtk_draw_window_divider(lisp: &mut Lisp, w: Value, x0: i32, x1: i32, y0: i32, y1: i32) {
    let f = lisp.window(w).frame;
    let foreground = lisp.frame(f).foreground_pixel as u64;
    let color_of = |lisp: &Lisp, id| {
        lisp.face_from_id(f, id)
            .map_or(foreground, |face| face.foreground)
    };
    let color = color_of(lisp, WINDOW_DIVIDER_FACE_ID);
    let color_first = color_of(lisp, WINDOW_DIVIDER_FIRST_PIXEL_FACE_ID);
    let color_last = color_of(lisp, WINDOW_DIVIDER_LAST_PIXEL_FACE_ID);
    let respect = lisp.frame(f).flags.borders_respect_alpha_background;
    if y1 - y0 > x1 - x0 && x1 - x0 > 2 {
        pgtk_fill_rectangle(lisp, f, color_first, x0, y0, 1, y1 - y0, respect);
        pgtk_fill_rectangle(lisp, f, color, x0 + 1, y0, x1 - x0 - 2, y1 - y0, respect);
        pgtk_fill_rectangle(lisp, f, color_last, x1 - 1, y0, 1, y1 - y0, respect);
    } else if x1 - x0 > y1 - y0 && y1 - y0 > 3 {
        pgtk_fill_rectangle(lisp, f, color_first, x0, y0, x1 - x0, 1, respect);
        pgtk_fill_rectangle(lisp, f, color, x0, y0 + 1, x1 - x0, y1 - y0 - 2, respect);
        pgtk_fill_rectangle(lisp, f, color_last, x0, y1 - 1, x1 - x0, 1, respect);
    } else {
        pgtk_fill_rectangle(lisp, f, color, x0, y0, x1 - x0, y1 - y0, respect);
    }
}

fn pgtk_define_fringe_bitmap(_lisp: &mut Lisp, which: usize, bits: &[u16], h: i32, wd: i32) {
    FRINGE_BMP.with(|bmp| {
        let mut bmp = bmp.borrow_mut();
        if which >= bmp.len() {
            bmp.resize_with(which + 20, || None);
        }
        bmp[which] = Some(FringeBmp {
            width: wd,
            height: h,
            rows: bits[..h.max(0) as usize].to_vec(),
        });
    });
}

fn pgtk_destroy_fringe_bitmap(_lisp: &mut Lisp, which: usize) {
    FRINGE_BMP.with(|bmp| {
        if let Some(slot) = bmp.borrow_mut().get_mut(which) {
            *slot = None;
        }
    });
}

fn pgtk_clip_to_row(lisp: &mut Lisp, w: Value, row: GlyphRowRef, area: GlyphRowArea) {
    let (window_x, window_y, window_width, _) = lisp.window_box(w, area);
    let (row_y, visible_height) = {
        let r = lisp.glyph_row(row);
        (r.y, r.visible_height)
    };
    let y = (lisp.window_to_frame_pixel_y(w, i64::from(row_y.max(0)))).max(window_y);
    let f = lisp.window(w).frame;
    with_surface(lisp, f, |s| {
        s.clip_rectangles(&[Rect::new(
            window_x as i32,
            y as i32,
            window_width as i32,
            visible_height,
        )])
    });
}

#[allow(clippy::too_many_arguments)]
fn pgtk_cr_draw_bitmap(
    lisp: &Lisp,
    f: Value,
    gc: &EmacsGc,
    which: usize,
    src_y: i32,
    width: i32,
    height: i32,
    dest_x: i32,
    dest_y: i32,
    overlay_p: bool,
) {
    begin_cr_clip(lisp, f);
    if !overlay_p {
        with_surface(lisp, f, |s| {
            s.fill_rectangle(
                gc.background as u32,
                Rect::new(dest_x, dest_y, width, height),
                false,
            )
        });
    }
    let coverage = FRINGE_BMP.with(|bmp| {
        let bmp = bmp.borrow();
        let fb = bmp.get(which).and_then(Option::as_ref)?;
        let mut coverage = vec![0u8; (width.max(0) * height.max(0)) as usize];
        for row in 0..height {
            let src = src_y + row;
            if src < 0 || src >= fb.height {
                continue;
            }
            let bits = fb.rows[src as usize];
            for col in 0..width.min(fb.width) {
                if bits & (1 << (fb.width - 1 - col)) != 0 {
                    coverage[(row * width + col) as usize] = 255;
                }
            }
        }
        Some(coverage)
    });
    if let Some(coverage) = coverage {
        with_surface(lisp, f, |s| {
            s.draw_mask(
                Rect::new(dest_x, dest_y, width, height),
                &coverage,
                gc.foreground as u32,
            )
        });
    }
    end_cr_clip(lisp, f);
}

fn pgtk_draw_fringe_bitmap(
    lisp: &mut Lisp,
    w: Value,
    row: GlyphRowRef,
    p: &DrawFringeBitmapParams,
) {
    let f = lisp.window(w).frame;
    begin_cr_clip(lisp, f);
    pgtk_clip_to_row(lisp, w, row, GlyphRowArea::Any);
    if p.bx >= 0 && !p.overlay_p {
        fill_background_by_face(lisp, f, p.face, p.bx, p.by, p.nx, p.ny);
    }
    let which = p.which as usize;
    let max_fringe_bmp = FRINGE_BMP.with(|bmp| bmp.borrow().len());
    if p.which != 0
        && which < max_fringe_bmp
        && i64::from(p.which) < lisp.fringe.max_used_fringe_bitmap
    {
        let defined = FRINGE_BMP.with(|bmp| bmp.borrow()[which].is_some());
        if !defined {
            lisp.gui_define_fringe_bitmap(f, i64::from(p.which));
        }
        let (fg, bg, _) = face_colors(lisp, f, p.face);
        let cursor_color = with_output(lisp, f, |o| o.cursor_color as u64);
        let gc = EmacsGc {
            foreground: if p.cursor_p {
                if p.overlay_p { bg } else { cursor_color }
            } else {
                fg
            },
            background: bg,
        };
        pgtk_cr_draw_bitmap(lisp, f, &gc, which, p.dh, p.wd, p.h, p.x, p.y, p.overlay_p);
    }
    end_cr_clip(lisp, f);
}

pub fn pgtk_update_begin(lisp: &mut Lisp, f: Value) {
    pgtk_clear_under_internal_border(lisp, f);
}

pub fn pgtk_update_end(lisp: &mut Lisp, f: Value) {
    lisp.mouse_hl_info_mut(f).mouse_face_defer = false;
}
