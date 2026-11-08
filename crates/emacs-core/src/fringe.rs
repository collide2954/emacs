use crate::dispextern::{DrawFringeBitmapParams, GlyphRowArea, GlyphRowRef, TextCursorKind};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::xfaces::{DEFAULT_FACE_ID, FRINGE_FACE_ID};

pub const NO_FRINGE_BITMAP: i64 = 0;
pub const UNDEF_FRINGE_BITMAP: i64 = 1;
pub const MAX_STANDARD_FRINGE_BITMAPS: i64 = 25;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FringeBitmapAlign {
    Center,
    Top,
    Bottom,
}

#[derive(Clone, Debug)]
pub struct FringeBitmap {
    pub bits: Vec<u16>,
    pub height: i32,
    pub width: i32,
    pub period: i32,
    pub align: FringeBitmapAlign,
    pub dynamic: bool,
}

const QUESTION_MARK_BITS: &[u16] = &[0x3c, 0x7e, 0xc3, 0xc3, 0x0c, 0x18, 0x18, 0x00, 0x18, 0x18];
const EXCLAMATION_MARK_BITS: &[u16] = &[0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18];
const LEFT_ARROW_BITS: &[u16] = &[0x18, 0x30, 0x60, 0xfc, 0xfc, 0x60, 0x30, 0x18];
const RIGHT_ARROW_BITS: &[u16] = &[0x18, 0x0c, 0x06, 0x3f, 0x3f, 0x06, 0x0c, 0x18];
const UP_ARROW_BITS: &[u16] = &[0x18, 0x3c, 0x7e, 0xff, 0x18, 0x18, 0x18, 0x18];
const DOWN_ARROW_BITS: &[u16] = &[0x18, 0x18, 0x18, 0x18, 0xff, 0x7e, 0x3c, 0x18];
const LEFT_CURLY_ARROW_BITS: &[u16] = &[0x3c, 0x7c, 0xc0, 0xe4, 0xfc, 0x7c, 0x3c, 0x7c];
const RIGHT_CURLY_ARROW_BITS: &[u16] = &[0x3c, 0x3e, 0x03, 0x27, 0x3f, 0x3e, 0x3c, 0x3e];
const LARGE_CIRCLE_BITS: &[u16] = &[0x3c, 0x7e, 0xff, 0xff, 0xff, 0xff, 0x7e, 0x3c];
const LEFT_TRIANGLE_BITS: &[u16] = &[0x03, 0x0f, 0x1f, 0x3f, 0x3f, 0x1f, 0x0f, 0x03];
const RIGHT_TRIANGLE_BITS: &[u16] = &[0xc0, 0xf0, 0xf8, 0xfc, 0xfc, 0xf8, 0xf0, 0xc0];
const TOP_LEFT_ANGLE_BITS: &[u16] = &[0xfc, 0xfc, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0x00];
const TOP_RIGHT_ANGLE_BITS: &[u16] = &[0x3f, 0x3f, 0x03, 0x03, 0x03, 0x03, 0x03, 0x00];
const BOTTOM_LEFT_ANGLE_BITS: &[u16] = &[0x00, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xfc, 0xfc];
const BOTTOM_RIGHT_ANGLE_BITS: &[u16] = &[0x00, 0x03, 0x03, 0x03, 0x03, 0x03, 0x3f, 0x3f];
const LEFT_BRACKET_BITS: &[u16] = &[0xfc, 0xfc, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xfc, 0xfc];
const RIGHT_BRACKET_BITS: &[u16] = &[0x3f, 0x3f, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x3f, 0x3f];
const FILLED_RECTANGLE_BITS: &[u16] = &[
    0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe,
];
const HOLLOW_RECTANGLE_BITS: &[u16] = &[
    0xfe, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0xfe,
];
const HOLLOW_SQUARE_BITS: &[u16] = &[0x7e, 0x42, 0x42, 0x42, 0x42, 0x7e];
const FILLED_SQUARE_BITS: &[u16] = &[0x7e, 0x7e, 0x7e, 0x7e, 0x7e, 0x7e];
const VERTICAL_BAR_BITS: &[u16] = &[
    0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0,
];
const HORIZONTAL_BAR_BITS: &[u16] = &[0xfe, 0xfe];
const EMPTY_LINE_BITS: &[u16] = &[
    0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00,
    0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c,
    0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00,
    0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00,
    0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x3c, 0x00,
];

const STANDARD_BITMAPS: [(&[u16], i32, FringeBitmapAlign); MAX_STANDARD_FRINGE_BITMAPS as usize] = [
    (&[], 0, FringeBitmapAlign::Center),
    (QUESTION_MARK_BITS, 0, FringeBitmapAlign::Center),
    (EXCLAMATION_MARK_BITS, 0, FringeBitmapAlign::Center),
    (LEFT_ARROW_BITS, 0, FringeBitmapAlign::Center),
    (RIGHT_ARROW_BITS, 0, FringeBitmapAlign::Center),
    (UP_ARROW_BITS, 0, FringeBitmapAlign::Top),
    (DOWN_ARROW_BITS, 0, FringeBitmapAlign::Bottom),
    (LEFT_CURLY_ARROW_BITS, 0, FringeBitmapAlign::Center),
    (RIGHT_CURLY_ARROW_BITS, 0, FringeBitmapAlign::Center),
    (LARGE_CIRCLE_BITS, 0, FringeBitmapAlign::Center),
    (LEFT_TRIANGLE_BITS, 0, FringeBitmapAlign::Center),
    (RIGHT_TRIANGLE_BITS, 0, FringeBitmapAlign::Center),
    (TOP_LEFT_ANGLE_BITS, 0, FringeBitmapAlign::Top),
    (TOP_RIGHT_ANGLE_BITS, 0, FringeBitmapAlign::Top),
    (BOTTOM_LEFT_ANGLE_BITS, 0, FringeBitmapAlign::Bottom),
    (BOTTOM_RIGHT_ANGLE_BITS, 0, FringeBitmapAlign::Bottom),
    (LEFT_BRACKET_BITS, 0, FringeBitmapAlign::Center),
    (RIGHT_BRACKET_BITS, 0, FringeBitmapAlign::Center),
    (FILLED_RECTANGLE_BITS, 0, FringeBitmapAlign::Center),
    (HOLLOW_RECTANGLE_BITS, 0, FringeBitmapAlign::Center),
    (FILLED_SQUARE_BITS, 0, FringeBitmapAlign::Center),
    (HOLLOW_SQUARE_BITS, 0, FringeBitmapAlign::Center),
    (VERTICAL_BAR_BITS, 0, FringeBitmapAlign::Center),
    (HORIZONTAL_BAR_BITS, 0, FringeBitmapAlign::Bottom),
    (EMPTY_LINE_BITS, 3, FringeBitmapAlign::Top),
];

pub const HOLLOW_RECTANGLE_HEIGHT: i32 = 13;

pub fn standard_bitmap(bn: usize) -> FringeBitmap {
    let (bits, period, align) = STANDARD_BITMAPS[bn];
    FringeBitmap {
        bits: bits.to_vec(),
        height: bits.len() as i32,
        width: if bn == 0 { 0 } else { 8 },
        period,
        align,
        dynamic: false,
    }
}

pub struct Fringe {
    pub bitmaps: Vec<Option<FringeBitmap>>,
    pub faces: Vec<Value>,
    pub max_used_fringe_bitmap: i64,
}

impl Default for Fringe {
    fn default() -> Fringe {
        let max = MAX_STANDARD_FRINGE_BITMAPS as usize + 20;
        Fringe {
            bitmaps: vec![None; max],
            faces: vec![Value::NIL; max],
            max_used_fringe_bitmap: MAX_STANDARD_FRINGE_BITMAPS,
        }
    }
}

impl Fringe {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&self.faces);
    }
}

impl Lisp {
    pub fn lookup_fringe_bitmap(&self, bitmap: Value) -> i64 {
        let bitmap = self.get(bitmap, sym::FRINGE);
        let Some(bn) = bitmap.as_fixnum() else {
            return 0;
        };
        if bn > NO_FRINGE_BITMAP
            && bn < self.fringe.max_used_fringe_bitmap
            && (bn < MAX_STANDARD_FRINGE_BITMAPS
                || self
                    .fringe
                    .bitmaps
                    .get(bn as usize)
                    .is_some_and(Option::is_some))
        {
            return bn;
        }
        0
    }

    pub fn get_fringe_bitmap_data(&self, bn: i64) -> FringeBitmap {
        match self
            .fringe
            .bitmaps
            .get(bn as usize)
            .and_then(Option::as_ref)
        {
            Some(fb) => fb.clone(),
            None => standard_bitmap(if bn < MAX_STANDARD_FRINGE_BITMAPS {
                bn as usize
            } else {
                UNDEF_FRINGE_BITMAP as usize
            }),
        }
    }

    pub fn gui_define_fringe_bitmap(&mut self, f: Value, n: i64) {
        let Some(define) = self.frame_rif(f).define_fringe_bitmap else {
            return;
        };
        if n >= self.fringe.max_used_fringe_bitmap {
            return;
        }
        let fb = match self.fringe.bitmaps.get(n as usize).and_then(Option::as_ref) {
            Some(fb) => fb.clone(),
            None if n < MAX_STANDARD_FRINGE_BITMAPS => standard_bitmap(n as usize),
            None => return,
        };
        define(self, n as usize, &fb.bits, fb.height, fb.width);
    }

    pub fn gui_init_fringe(&mut self, rif: &crate::dispextern::RedisplayInterface) {
        let Some(define) = rif.define_fringe_bitmap else {
            return;
        };
        let mut bt = NO_FRINGE_BITMAP + 1;
        while bt < MAX_STANDARD_FRINGE_BITMAPS {
            if self.fringe.bitmaps[bt as usize].is_none() {
                let fb = standard_bitmap(bt as usize);
                define(self, bt as usize, &fb.bits, fb.height, fb.width);
            }
            bt += 1;
        }
        while bt < self.fringe.max_used_fringe_bitmap {
            if let Some(fb) = self.fringe.bitmaps[bt as usize].clone() {
                define(self, bt as usize, &fb.bits, fb.height, fb.width);
            }
            bt += 1;
        }
    }

    fn window_leftmost_p(&self, w: Value) -> bool {
        self.window(w).pixel_left == 0
    }

    fn draw_fringe_bitmap_1(
        &mut self,
        w: Value,
        row: GlyphRowRef,
        left_p: bool,
        overlay: i32,
        which: i64,
    ) -> LispResult<()> {
        let f = self.window(w).frame;
        let mut p = DrawFringeBitmapParams {
            overlay_p: (overlay & 1) == 1,
            cursor_p: (overlay & 2) == 2,
            ..DrawFringeBitmapParams::default()
        };
        let mut which = which;
        let mut face_id = DEFAULT_FACE_ID;
        let (row_y, row_height, visible_height) = {
            let r = self.glyph_row(row);
            (r.y, r.height, r.visible_height)
        };
        let offset = if which != NO_FRINGE_BITMAP {
            0
        } else {
            let r = self.glyph_row(row);
            if left_p {
                which = i64::from(r.left_fringe_bitmap);
                face_id = r.left_fringe_face_id as usize;
                r.left_fringe_offset
            } else {
                which = i64::from(r.right_fringe_bitmap);
                face_id = r.right_fringe_face_id as usize;
                r.right_fringe_offset
            }
        };
        if face_id == DEFAULT_FACE_ID {
            let face = self
                .fringe
                .faces
                .get(which as usize)
                .copied()
                .unwrap_or(Value::NIL);
            let id = if face.is_nil() {
                self.lookup_named_face(Some(w), f, sym::FRINGE, false)?
            } else {
                self.lookup_derived_face(Some(w), f, face, FRINGE_FACE_ID, false)?
            };
            face_id = if id < 0 { FRINGE_FACE_ID } else { id as usize };
        }
        let fb = self.get_fringe_bitmap_data(which);
        let period = fb.period;
        p.y = self.window_to_frame_pixel_y(w, i64::from(row_y)) as i32 + offset;
        p.which = which as i32;
        p.wd = fb.width;
        p.h = fb.height;
        p.dh = if period > 0 { p.y % period } else { 0 };
        p.h -= p.dh;
        match fb.align {
            FringeBitmapAlign::Center => p.y += (row_height - p.h) / 2,
            FringeBitmapAlign::Bottom => p.y += visible_height - p.h,
            FringeBitmapAlign::Top => {}
        }
        if self.face_from_id(f, face_id).is_none() {
            return Ok(());
        }
        p.face = face_id;
        self.prepare_face_for_display(f, face_id);
        p.bx = -1;
        let header_line_height =
            (self.window_tab_line_height(w) + self.window_header_line_height(w)) as i32;
        p.by = self.window_to_frame_pixel_y(w, i64::from(header_line_height.max(row_y))) as i32;
        p.ny = visible_height;
        let outside = self.window_has_fringes_outside_margins(w);
        if left_p {
            let mut wd = self.window_left_fringe_width(w) as i32;
            let x = self.window_box_left(
                w,
                if outside {
                    GlyphRowArea::LeftMargin
                } else {
                    GlyphRowArea::Text
                },
            ) as i32;
            if p.wd > wd {
                p.wd = wd;
            }
            p.x = x - p.wd - (wd - p.wd) / 2;
            if p.wd < wd || p.y > p.by || p.y + p.h < p.by + p.ny {
                let reduce = !self.window_leftmost_p(w)
                    && self.frame(f).geometry.right_divider_width == 0
                    && !self.window_has_vertical_scroll_bar(w)
                    && self.window(w).left_margin_cols == 0;
                wd -= i32::from(reduce);
                p.bx = x - wd;
                p.nx = wd;
            }
        } else {
            let x = self.window_box_right(
                w,
                if outside {
                    GlyphRowArea::RightMargin
                } else {
                    GlyphRowArea::Text
                },
            ) as i32;
            let wd = self.window_right_fringe_width(w) as i32;
            if p.wd > wd {
                p.wd = wd;
            }
            p.x = x + (wd - p.wd) / 2;
            if p.wd < wd || p.y > p.by || p.y + p.h < p.by + p.ny {
                p.bx = x;
                p.nx = wd;
            }
        }
        let left_edge = self.window_box_left_edge_x(w) as i32;
        if p.x >= left_edge && (p.x + p.wd) <= left_edge + self.window(w).pixel_width as i32 {
            let draw = self
                .frame_rif(f)
                .draw_fringe_bitmap
                .expect("redisplay interface draw_fringe_bitmap");
            draw(self, w, row, &p);
        }
        Ok(())
    }

    fn get_logical_cursor_bitmap(&mut self, w: Value, cursor: Value) -> LispResult<i64> {
        let buffer = self.window(w).contents;
        let cmap = self.buffer_local_value(sym::FRINGE_CURSOR_ALIST, buffer);
        if !cmap.is_nil() {
            let bm = crate::fns::f_assq(self, &[cursor, cmap])?;
            if bm.is_cons() {
                let bm = self.xcdr(bm);
                if bm.is_nil() {
                    return Ok(NO_FRINGE_BITMAP);
                }
                return Ok(self.lookup_fringe_bitmap(bm));
            }
        }
        let default = self.default_value_unchecked(sym::FRINGE_CURSOR_ALIST);
        if cmap == default {
            return Ok(NO_FRINGE_BITMAP);
        }
        let bm = crate::fns::f_assq(self, &[cursor, default])?;
        if !bm.is_cons() {
            return Ok(NO_FRINGE_BITMAP);
        }
        let bm = self.xcdr(bm);
        if bm.is_nil() {
            return Ok(NO_FRINGE_BITMAP);
        }
        Ok(self.lookup_fringe_bitmap(bm))
    }

    pub fn draw_fringe_bitmap(
        &mut self,
        w: Value,
        row: GlyphRowRef,
        left_p: bool,
    ) -> LispResult<()> {
        let mut overlay = 0;
        let (reversed_p, cursor_in_fringe_p, visible_height) = {
            let r = self.glyph_row(row);
            (r.reversed_p, r.cursor_in_fringe_p, r.visible_height)
        };
        if left_p == reversed_p && cursor_in_fringe_p {
            let cursor = match self.window(w).phys_cursor_type {
                TextCursorKind::HollowBox => {
                    if visible_height >= HOLLOW_RECTANGLE_HEIGHT {
                        sym::HOLLOW
                    } else {
                        sym::HOLLOW_SMALL
                    }
                }
                TextCursorKind::FilledBox => sym::BOX,
                TextCursorKind::Bar => sym::BAR,
                TextCursorKind::Hbar => sym::HBAR,
                TextCursorKind::NoCursor | TextCursorKind::Default => {
                    self.window_mut(w).flags.phys_cursor_on_p = false;
                    self.glyph_row_mut(row).cursor_in_fringe_p = false;
                    Value::NIL
                }
            };
            if !cursor.is_nil() {
                let bm = self.get_logical_cursor_bitmap(w, cursor)?;
                if bm != NO_FRINGE_BITMAP {
                    self.draw_fringe_bitmap_1(w, row, left_p, 2, bm)?;
                    overlay = if cursor == sym::BOX { 3 } else { 1 };
                }
            }
        }
        self.draw_fringe_bitmap_1(w, row, left_p, overlay, NO_FRINGE_BITMAP)?;
        let overlay_arrow_bitmap = i64::from(self.glyph_row(row).overlay_arrow_bitmap);
        if left_p && overlay_arrow_bitmap != NO_FRINGE_BITMAP {
            self.draw_fringe_bitmap_1(w, row, true, 1, overlay_arrow_bitmap)?;
        }
        Ok(())
    }

    pub fn draw_row_fringe_bitmaps(&mut self, w: Value, row: GlyphRowRef) -> LispResult<()> {
        if self.glyph_row(row).visible_height <= 0 {
            return Ok(());
        }
        if self.window_left_fringe_width(w) != 0 {
            self.draw_fringe_bitmap(w, row, true)?;
        }
        if self.window_right_fringe_width(w) != 0 {
            self.draw_fringe_bitmap(w, row, false)?;
        }
        Ok(())
    }

    pub fn draw_window_fringes(&mut self, w: Value, no_fringe_p: bool) -> LispResult<bool> {
        let mut updated_p = false;
        if self.window(w).flags.pseudo_window_p {
            return Ok(updated_p);
        }
        let yb = self.window_text_bottom_y(w) as i32;
        let nrows = self
            .window_matrix(w, WhichMatrix::Current)
            .map_or(0, |m| m.nrows.max(0) as usize);
        let window_buffer = self.window(w).contents;
        let oldbuf = self.current_buffer;
        self.set_buffer_internal(window_buffer);
        if no_fringe_p
            && (self.window_left_fringe_width(w) == 0 || self.window_right_fringe_width(w) == 0)
        {
            updated_p = true;
        }
        let mut y = self.window(w).vscroll as i32;
        let mut rn = 0;
        let mut result = Ok(());
        while y < yb && rn < nrows {
            let row = GlyphRowRef::Window {
                window: w,
                desired: false,
                vpos: rn,
            };
            let (height, redraw) = {
                let r = self.glyph_row(row);
                (r.height, r.redraw_fringe_bitmaps_p)
            };
            if redraw {
                result = self.draw_row_fringe_bitmaps(w, row);
                if result.is_err() {
                    break;
                }
                self.glyph_row_mut(row).redraw_fringe_bitmaps_p = false;
                updated_p = true;
            }
            y += height;
            rn += 1;
        }
        self.set_buffer_internal(oldbuf);
        result.map(|()| updated_p)
    }

    fn get_logical_fringe_bitmap(
        &mut self,
        w: Value,
        bitmap: Value,
        right_p: usize,
        partial_p: bool,
    ) -> LispResult<i64> {
        let ix1 = right_p;
        let ix2 = ix1 + if partial_p { 2 } else { 0 };
        let buffer = self.window(w).contents;
        let cmap = self.buffer_local_value(sym::FRINGE_INDICATOR_ALIST, buffer);
        let mut bm1 = Value::NIL;
        let mut bm2 = Value::NIL;
        let mut ln1 = 0;
        let mut ln2 = 0;
        if !cmap.is_nil() {
            bm1 = crate::fns::f_assq(self, &[bitmap, cmap])?;
            if bm1.is_cons() {
                bm1 = self.xcdr(bm1);
                if bm1.is_nil() {
                    return Ok(NO_FRINGE_BITMAP);
                }
                if bm1.is_cons() {
                    ln1 = self.list_length(bm1)?;
                    if partial_p {
                        if ln1 > ix2 {
                            let bm = crate::fns::f_nth(self, &[Value::fixnum(ix2 as i64), bm1])?;
                            if bm != sym::T {
                                return Ok(self.lookup_fringe_bitmap(bm));
                            }
                        }
                    } else if ln1 > ix1 {
                        let bm = crate::fns::f_nth(self, &[Value::fixnum(ix1 as i64), bm1])?;
                        if bm != sym::T {
                            return Ok(self.lookup_fringe_bitmap(bm));
                        }
                    }
                } else if bm1 != sym::T {
                    return Ok(self.lookup_fringe_bitmap(bm1));
                }
            }
        }
        let default = self.default_value_unchecked(sym::FRINGE_INDICATOR_ALIST);
        if cmap != default && !default.is_nil() {
            bm2 = crate::fns::f_assq(self, &[bitmap, default])?;
            if bm2.is_cons() {
                bm2 = self.xcdr(bm2);
                if !bm2.is_nil() && bm2.is_cons() {
                    ln2 = self.list_length(bm2)?;
                    if partial_p && ln2 > ix2 {
                        let bm = crate::fns::f_nth(self, &[Value::fixnum(ix2 as i64), bm2])?;
                        if bm != sym::T {
                            return Ok(self.lookup_fringe_bitmap(bm));
                        }
                    }
                }
            }
        }
        if ln1 > ix1 {
            let bm = crate::fns::f_nth(self, &[Value::fixnum(ix1 as i64), bm1])?;
            if bm != sym::T {
                return Ok(self.lookup_fringe_bitmap(bm));
            }
        }
        if ln2 > ix1 {
            let bm = crate::fns::f_nth(self, &[Value::fixnum(ix1 as i64), bm2])?;
            if bm != sym::T {
                return Ok(self.lookup_fringe_bitmap(bm));
            }
            return Ok(NO_FRINGE_BITMAP);
        }
        if bm2.is_nil() {
            return Ok(NO_FRINGE_BITMAP);
        }
        Ok(self.lookup_fringe_bitmap(bm2))
    }

    fn fringe_row(&self, w: Value, rn: usize) -> GlyphRowRef {
        let desired_enabled = self
            .window_matrix(w, WhichMatrix::Desired)
            .is_some_and(|m| m.rows[rn].enabled_p);
        GlyphRowRef::Window {
            window: w,
            desired: desired_enabled,
            vpos: rn,
        }
    }

    pub fn update_window_fringes(&mut self, w: Value, keep_current_p: bool) -> LispResult<bool> {
        if self.window(w).flags.pseudo_window_p {
            return Ok(false);
        }
        let count = self.specpdl_index();
        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
        let result = self.update_window_fringes_1(w, keep_current_p);
        self.unbind_to(count)?;
        result
    }

    fn update_window_fringes_1(&mut self, w: Value, keep_current_p: bool) -> LispResult<bool> {
        let yb = self.window_text_bottom_y(w) as i32;
        let nrows = self
            .window_matrix(w, WhichMatrix::Current)
            .map_or(0, |m| m.nrows.max(0) as usize);
        let mut redraw_p = false;
        let mut boundary_top = Value::NIL;
        let mut boundary_bot = Value::NIL;
        let mut arrow_top = Value::NIL;
        let mut arrow_bot = Value::NIL;
        let buffer = self.window(w).contents;
        let mut ind = Value::NIL;
        if !self.window(w).mini {
            ind = self.buffer_local_value(sym::INDICATE_BUFFER_BOUNDARIES, buffer);
            if !ind.is_nil() {
                if ind == sym::LEFT || ind == sym::RIGHT {
                    boundary_top = ind;
                    boundary_bot = ind;
                    arrow_top = ind;
                    arrow_bot = ind;
                } else if ind.is_cons() && self.xcar(ind).is_cons() {
                    let pos = crate::fns::f_assq(self, &[sym::T, ind])?;
                    if !pos.is_nil() {
                        let v = self.xcdr(pos);
                        boundary_top = v;
                        boundary_bot = v;
                        arrow_top = v;
                        arrow_bot = v;
                    }
                    let pos = crate::fns::f_assq(self, &[sym::TOP, ind])?;
                    if !pos.is_nil() {
                        boundary_top = self.xcdr(pos);
                    }
                    let pos = crate::fns::f_assq(self, &[sym::BOTTOM, ind])?;
                    if !pos.is_nil() {
                        boundary_bot = self.xcdr(pos);
                    }
                    let pos = crate::fns::f_assq(self, &[sym::UP, ind])?;
                    if !pos.is_nil() {
                        arrow_top = self.xcdr(pos);
                    }
                    let pos = crate::fns::f_assq(self, &[sym::DOWN, ind])?;
                    if !pos.is_nil() {
                        arrow_bot = self.xcdr(pos);
                    }
                } else {
                    boundary_top = sym::LEFT;
                    boundary_bot = sym::LEFT;
                }
            }
        }
        let mut top_ind_rn: i64 = -1;
        let mut bot_ind_rn: i64 = -1;
        let (begv, zv) = {
            let b = self.buffer(buffer);
            (b.begv as i64, b.zv as i64)
        };
        let top_lines = (self.window_tab_line_height(w) + self.window_header_line_height(w)) as i32;
        if !ind.is_nil() {
            let mut y = self.window(w).vscroll as i32;
            let mut rn = 0;
            while y < yb && rn < nrows {
                let row = self.fringe_row(w, rn);
                {
                    let r = self.glyph_row_mut(row);
                    r.indicate_bob_p = false;
                    r.indicate_top_line_p = false;
                    r.indicate_eob_p = false;
                    r.indicate_bottom_line_p = false;
                }
                let r = self.glyph_row(row).clone();
                if !r.mode_line_p {
                    if top_ind_rn < 0 && r.visible_height > 0 {
                        let at_top = r.height != r.visible_height && r.y < top_lines;
                        if r.start_charpos() <= begv && !at_top {
                            self.glyph_row_mut(row).indicate_bob_p = !boundary_top.is_nil();
                        } else {
                            self.glyph_row_mut(row).indicate_top_line_p = !arrow_top.is_nil();
                        }
                        top_ind_rn = rn as i64;
                    }
                    if bot_ind_rn < 0 {
                        if r.end_charpos() >= zv
                            && !self.matrix_row_partially_visible_at_bottom_p(w, &r)
                        {
                            self.glyph_row_mut(row).indicate_eob_p = !boundary_bot.is_nil();
                            bot_ind_rn = rn as i64;
                        } else if y + r.height >= yb {
                            self.glyph_row_mut(row).indicate_bottom_line_p = !arrow_bot.is_nil();
                            bot_ind_rn = rn as i64;
                        }
                    }
                }
                y += r.height;
                rn += 1;
            }
        }
        let mut empty_pos = self.buffer_local_value(sym::INDICATE_EMPTY_LINES, buffer);
        if !empty_pos.is_nil() && empty_pos != sym::RIGHT {
            empty_pos = if self.window_left_fringe_width(w) == 0 {
                sym::RIGHT
            } else {
                sym::LEFT
            };
        }
        let mut bitmap_cache = [-1i64; 32];
        let mut left_fringe = |lisp: &mut Lisp, cache: usize, which: Value, partial_p: bool| {
            let slot = cache * 4 + usize::from(partial_p);
            if bitmap_cache[slot] < 0 {
                bitmap_cache[slot] = lisp.get_logical_fringe_bitmap(w, which, 0, partial_p)?;
            }
            Ok::<i64, crate::error::Flow>(bitmap_cache[slot])
        };
        let mut right_cache = [-1i64; 32];
        let mut right_fringe = |lisp: &mut Lisp, cache: usize, which: Value, partial_p: bool| {
            let slot = cache * 4 + 2 + usize::from(partial_p);
            if right_cache[slot] < 0 {
                right_cache[slot] = lisp.get_logical_fringe_bitmap(w, which, 1, partial_p)?;
            }
            Ok::<i64, crate::error::Flow>(right_cache[slot])
        };
        let mut top_ind_min_y = -1;
        let mut bot_ind_max_y = -1;
        let mut top_row_ends_at_zv_p = false;
        let mut bot_row_ends_at_zv_p = false;
        if top_ind_rn >= 0 {
            let row = self.fringe_row(w, top_ind_rn as usize);
            let r = self.glyph_row(row).clone();
            top_row_ends_at_zv_p = r.ends_at_zv_p;
            let mut bn = NO_FRINGE_BITMAP;
            if r.indicate_bob_p {
                bn = if boundary_top == sym::LEFT {
                    if r.indicate_eob_p && boundary_bot == sym::LEFT {
                        left_fringe(self, 1, sym::TOP_BOTTOM, r.ends_at_zv_p)?
                    } else {
                        left_fringe(self, 2, sym::TOP, false)?
                    }
                } else if r.indicate_eob_p && boundary_bot == sym::RIGHT {
                    right_fringe(self, 1, sym::TOP_BOTTOM, r.ends_at_zv_p)?
                } else {
                    right_fringe(self, 2, sym::TOP, false)?
                };
            } else if r.indicate_top_line_p {
                bn = if arrow_top == sym::LEFT {
                    left_fringe(self, 6, sym::UP, false)?
                } else {
                    right_fringe(self, 6, sym::UP, false)?
                };
            }
            if bn != NO_FRINGE_BITMAP {
                let fb = self.get_fringe_bitmap_data(bn);
                if fb.align == FringeBitmapAlign::Top && fb.period == 0 {
                    top_ind_min_y = top_lines;
                    let mut top_ind_max_y = top_ind_min_y + fb.height;
                    if top_ind_max_y > yb {
                        top_ind_max_y = yb;
                    }
                    let mut y = r.y + r.height;
                    let mut rn = top_ind_rn as usize + 1;
                    while y < top_ind_max_y && rn < nrows {
                        if bot_ind_rn >= 0 && rn as i64 >= bot_ind_rn {
                            break;
                        }
                        let row1 = self.fringe_row(w, rn);
                        let r1 = self.glyph_row_mut(row1);
                        r1.indicate_bob_p = r.indicate_bob_p;
                        r1.indicate_top_line_p = r.indicate_top_line_p;
                        y += r1.height;
                        rn += 1;
                    }
                }
            }
        }
        if bot_ind_rn >= 0 {
            let row = self.fringe_row(w, bot_ind_rn as usize);
            let r = self.glyph_row(row).clone();
            bot_row_ends_at_zv_p = r.ends_at_zv_p;
            let mut bn = NO_FRINGE_BITMAP;
            if r.indicate_eob_p {
                bn = if boundary_bot == sym::LEFT {
                    left_fringe(self, 3, sym::BOTTOM, r.ends_at_zv_p)?
                } else {
                    right_fringe(self, 3, sym::BOTTOM, r.ends_at_zv_p)?
                };
            } else if r.indicate_bottom_line_p {
                bn = if arrow_bot == sym::LEFT {
                    left_fringe(self, 7, sym::DOWN, false)?
                } else {
                    right_fringe(self, 7, sym::DOWN, false)?
                };
            }
            if bn != NO_FRINGE_BITMAP {
                let fb = self.get_fringe_bitmap_data(bn);
                if fb.align == FringeBitmapAlign::Bottom && fb.period == 0 {
                    bot_ind_max_y = r.y + r.visible_height;
                    let mut bot_ind_min_y = bot_ind_max_y - fb.height;
                    if bot_ind_min_y < top_lines {
                        bot_ind_min_y = top_lines;
                    }
                    let mut y = r.y;
                    let mut rn = bot_ind_rn - 1;
                    while y >= bot_ind_min_y && rn >= 0 {
                        if top_ind_rn >= 0 && rn <= top_ind_rn {
                            break;
                        }
                        let row1 = self.fringe_row(w, rn as usize);
                        let r1 = self.glyph_row_mut(row1);
                        r1.indicate_eob_p = r.indicate_eob_p;
                        r1.indicate_bottom_line_p = r.indicate_bottom_line_p;
                        y -= r1.height;
                        rn -= 1;
                    }
                }
            }
        }
        let left_width = self.window_left_fringe_width(w);
        let right_width = self.window_right_fringe_width(w);
        let mut y = self.window(w).vscroll as i32;
        let mut rn = 0;
        while y < yb && rn < nrows {
            let row = self.fringe_row(w, rn);
            let cur_ref = GlyphRowRef::Window {
                window: w,
                desired: false,
                vpos: rn,
            };
            let r = self.glyph_row(row).clone();
            let mut left_face_id = DEFAULT_FACE_ID as u32;
            let mut right_face_id = DEFAULT_FACE_ID as u32;
            let mut left_offset = 0;
            let mut right_offset = 0;
            let left = if left_width == 0 {
                NO_FRINGE_BITMAP
            } else if i64::from(r.left_user_fringe_bitmap) != NO_FRINGE_BITMAP {
                left_face_id = r.left_user_fringe_face_id;
                i64::from(r.left_user_fringe_bitmap)
            } else if (!r.reversed_p && r.truncated_on_left_p)
                || (r.reversed_p && r.truncated_on_right_p)
            {
                left_fringe(self, 0, sym::TRUNCATION, false)?
            } else if r.indicate_bob_p && boundary_top == sym::LEFT {
                let bn = if r.indicate_eob_p && boundary_bot == sym::LEFT {
                    left_fringe(self, 1, sym::TOP_BOTTOM, top_row_ends_at_zv_p)?
                } else {
                    left_fringe(self, 2, sym::TOP, false)?
                };
                if top_ind_min_y >= 0 {
                    left_offset = top_ind_min_y - r.y;
                }
                bn
            } else if r.indicate_eob_p && boundary_bot == sym::LEFT {
                let bn = left_fringe(self, 3, sym::BOTTOM, bot_row_ends_at_zv_p)?;
                if bot_ind_max_y >= 0 {
                    left_offset = bot_ind_max_y - (r.y + r.visible_height);
                }
                bn
            } else if (!r.reversed_p && r.continuation_line_p()) || (r.reversed_p && r.continued_p)
            {
                left_fringe(self, 4, sym::CONTINUATION, false)?
            } else if r.indicate_empty_line_p && empty_pos == sym::LEFT {
                left_fringe(self, 5, sym::EMPTY_LINE, false)?
            } else if r.indicate_top_line_p && arrow_top == sym::LEFT {
                let bn = left_fringe(self, 6, sym::UP, false)?;
                if top_ind_min_y >= 0 {
                    left_offset = top_ind_min_y - r.y;
                }
                bn
            } else if r.indicate_bottom_line_p && arrow_bot == sym::LEFT {
                let bn = left_fringe(self, 7, sym::DOWN, false)?;
                if bot_ind_max_y >= 0 {
                    left_offset = bot_ind_max_y - (r.y + r.visible_height);
                }
                bn
            } else {
                NO_FRINGE_BITMAP
            };
            let right = if right_width == 0 {
                NO_FRINGE_BITMAP
            } else if i64::from(r.right_user_fringe_bitmap) != NO_FRINGE_BITMAP {
                right_face_id = r.right_user_fringe_face_id;
                i64::from(r.right_user_fringe_bitmap)
            } else if (!r.reversed_p && r.truncated_on_right_p)
                || (r.reversed_p && r.truncated_on_left_p)
            {
                right_fringe(self, 0, sym::TRUNCATION, false)?
            } else if r.indicate_bob_p && boundary_top == sym::RIGHT {
                let bn = if r.indicate_eob_p && boundary_bot == sym::RIGHT {
                    right_fringe(self, 1, sym::TOP_BOTTOM, top_row_ends_at_zv_p)?
                } else {
                    right_fringe(self, 2, sym::TOP, false)?
                };
                if top_ind_min_y >= 0 {
                    right_offset = top_ind_min_y - r.y;
                }
                bn
            } else if r.indicate_eob_p && boundary_bot == sym::RIGHT {
                let bn = right_fringe(self, 3, sym::BOTTOM, bot_row_ends_at_zv_p)?;
                if bot_ind_max_y >= 0 {
                    right_offset = bot_ind_max_y - (r.y + r.visible_height);
                }
                bn
            } else if (!r.reversed_p && r.continued_p) || (r.reversed_p && r.continuation_line_p())
            {
                right_fringe(self, 4, sym::CONTINUATION, false)?
            } else if r.indicate_top_line_p && arrow_top == sym::RIGHT {
                let bn = right_fringe(self, 6, sym::UP, false)?;
                if top_ind_min_y >= 0 {
                    right_offset = top_ind_min_y - r.y;
                }
                bn
            } else if r.indicate_bottom_line_p && arrow_bot == sym::RIGHT {
                let bn = right_fringe(self, 7, sym::DOWN, false)?;
                if bot_ind_max_y >= 0 {
                    right_offset = bot_ind_max_y - (r.y + r.visible_height);
                }
                bn
            } else if r.indicate_empty_line_p && empty_pos == sym::RIGHT {
                right_fringe(self, 5, sym::EMPTY_LINE, false)?
            } else {
                NO_FRINGE_BITMAP
            };
            let periodic_p = self.get_fringe_bitmap_data(left).period != 0
                || self.get_fringe_bitmap_data(right).period != 0;
            let cur = self.glyph_row(cur_ref).clone();
            if r.y != cur.y
                || r.visible_height != cur.visible_height
                || r.ends_at_zv_p != cur.ends_at_zv_p
                || left != i64::from(cur.left_fringe_bitmap)
                || right != i64::from(cur.right_fringe_bitmap)
                || left_face_id != cur.left_fringe_face_id
                || right_face_id != cur.right_fringe_face_id
                || left_offset != cur.left_fringe_offset
                || right_offset != cur.right_fringe_offset
                || periodic_p != cur.fringe_bitmap_periodic_p
                || cur.redraw_fringe_bitmaps_p
            {
                redraw_p = true;
                self.glyph_row_mut(row).redraw_fringe_bitmaps_p = true;
                if !keep_current_p {
                    let c = self.glyph_row_mut(cur_ref);
                    c.redraw_fringe_bitmaps_p = true;
                    c.left_fringe_bitmap = left as u32;
                    c.right_fringe_bitmap = right as u32;
                    c.left_fringe_face_id = left_face_id;
                    c.right_fringe_face_id = right_face_id;
                    c.left_fringe_offset = left_offset;
                    c.right_fringe_offset = right_offset;
                    c.fringe_bitmap_periodic_p = periodic_p;
                }
            }
            if r.overlay_arrow_bitmap < 0 {
                let bm = self.get_logical_fringe_bitmap(w, sym::OVERLAY_ARROW, 0, false)?;
                self.glyph_row_mut(row).overlay_arrow_bitmap = bm as i32;
            }
            let overlay_arrow_bitmap = self.glyph_row(row).overlay_arrow_bitmap;
            if overlay_arrow_bitmap != self.glyph_row(cur_ref).overlay_arrow_bitmap {
                redraw_p = true;
                self.glyph_row_mut(row).redraw_fringe_bitmaps_p = true;
                if !keep_current_p {
                    let c = self.glyph_row_mut(cur_ref);
                    c.redraw_fringe_bitmaps_p = true;
                    c.overlay_arrow_bitmap = overlay_arrow_bitmap;
                }
            }
            let rr = self.glyph_row_mut(row);
            rr.left_fringe_bitmap = left as u32;
            rr.right_fringe_bitmap = right as u32;
            rr.left_fringe_face_id = left_face_id;
            rr.right_fringe_face_id = right_face_id;
            rr.left_fringe_offset = left_offset;
            rr.right_fringe_offset = right_offset;
            rr.fringe_bitmap_periodic_p = periodic_p;
            y += rr.height;
            rn += 1;
        }
        Ok(redraw_p && !keep_current_p)
    }
}
