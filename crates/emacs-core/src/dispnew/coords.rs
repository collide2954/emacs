use crate::bidi::BidiDir;
use crate::dispextern::{
    DisplayElementType, DisplayPos, GlyphRowArea, GlyphType, It, LEFT_MARGIN_AREA, LineWrap,
    MOVE_TO_POS, MOVE_TO_X, MOVE_TO_Y, RIGHT_MARGIN_AREA, TEXT_AREA, TextPos, WindowPart,
};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

#[derive(Clone, Copy, Debug)]
pub struct AreaHit {
    pub string: Value,
    pub charpos: i64,
    pub object: Option<Value>,
    pub x: i64,
    pub y: i64,
    pub dx: i64,
    pub dy: i64,
    pub width: i64,
    pub height: i64,
}

#[derive(Clone, Copy, Debug)]
pub struct BufferPosn {
    pub string: Value,
    pub pos: DisplayPos,
    pub object: Option<Value>,
    pub x: i64,
    pub y: i64,
    pub dx: i64,
    pub dy: i64,
    pub width: i64,
    pub height: i64,
}

impl Lisp {
    pub fn buffer_posn_from_coords(&mut self, w: Value, x: i64, y: i64) -> LispResult<BufferPosn> {
        self.with_gc_deferred(|lisp| lisp.buffer_posn_from_coords_1(w, x, y))
    }

    fn buffer_posn_from_coords_1(&mut self, w: Value, x: i64, y: i64) -> LispResult<BufferPosn> {
        let old_current_buffer = self.current_buffer;
        let contents = self.window(w).contents;
        crate::buffer::f_set_buffer(self, &[contents])?;
        let itdata = self.bidi_shelve_cache();
        let startp = {
            let m = self.marker(self.window(w).start);
            let (charpos, bytepos) = (m.charpos as i64, m.bytepos as i64);
            let b = self.cur();
            if charpos < b.begv as i64 {
                TextPos::new(b.begv as i64, b.begv_byte as i64)
            } else if charpos > b.zv as i64 {
                TextPos::new(b.zv as i64, b.zv_byte as i64)
            } else {
                TextPos::new(charpos, bytepos)
            }
        };
        let mut it = It::default();
        self.start_display(&mut it, w, startp)?;
        let x0 = x as i32;
        self.move_it_to(&mut it, -1, 0, y as i32, -1, MOVE_TO_X | MOVE_TO_Y)?;
        let mut to_x = x0;
        if it.bidi_it.paragraph_dir == BidiDir::R2L {
            to_x = self.window_box_width(w, GlyphRowArea::Text) as i32 - to_x - 1;
        }
        to_x += it.first_visible_x;
        let pt = self.cur().pt as i64;
        if it.line_wrap == LineWrap::Truncate
            && self.find_symbol_value(sym::AUTO_HSCROLL_MODE) == sym::CURRENT_LINE
            && it.charpos() < pt
        {
            let mut it2 = it.clone();
            let it2data = self.bidi_shelve_cache();
            it2.last_visible_x = 1_000_000;
            self.move_it_in_display_line(&mut it2, pt, -1, MOVE_TO_POS)?;
            if it2.charpos() >= pt {
                let cw = self.frame(it.f).geometry.column_width as i32;
                let (hscroll, min_hscroll) = {
                    let win = self.window(w);
                    (win.hscroll as i32, win.min_hscroll as i32)
                };
                to_x += (hscroll - min_hscroll) * cw;
                it.first_visible_x += hscroll * cw;
                it.last_visible_x += hscroll * cw;
            }
            self.bidi_unshelve_cache(it2data, false);
        }
        let zv = self.cur().zv as i64;
        self.move_it_in_display_line(&mut it, zv, to_x, MOVE_TO_X)?;
        if self.var_true(sym::MOUSE_PREFER_CLOSEST_GLYPH) {
            let next_x = it.current_x + it.pixel_width;
            let before_dx = to_x - it.current_x;
            let after_dx = next_x - to_x;
            if before_dx > after_dx {
                self.move_it_in_display_line(&mut it, zv, next_x, MOVE_TO_X)?;
            }
        }
        self.bidi_unshelve_cache(itdata, false);
        crate::buffer::f_set_buffer(self, &[old_current_buffer])?;
        let dx = i64::from(to_x - it.current_x);
        let dy = y - i64::from(it.current_y);
        let string = if it.string.is_string() {
            it.string
        } else {
            contents
        };
        let mut pos = it.current;
        if it.what == DisplayElementType::Composition
            && it.cmp_it.nchars > 1
            && it.cmp_it.reversed_p
        {
            pos.pos.charpos -= it.cmp_it.nchars - 1;
            pos.pos.bytepos = if it.string.is_string() {
                self.string_char_to_byte_index(string, pos.pos.charpos as usize) as i64
            } else {
                self.buffer(contents)
                    .text
                    .char_to_byte(pos.pos.charpos as usize) as i64
            };
        }
        if it.what == DisplayElementType::Image {
            unreachable!("image glyphs need image.c");
        }
        let it_vpos = it.vpos
            + i32::from(self.window_wants_header_line(w))
            + i32::from(self.window_wants_tab_line(w));
        let (mut width, mut height) = (0, 0);
        if let Some(m) = self.window_matrix(w, WhichMatrix::Current)
            && it_vpos < m.nrows
            && m.rows[it_vpos as usize].enabled_p
        {
            let row = &m.rows[it_vpos as usize];
            if it.hpos < i32::from(row.used[TEXT_AREA]) {
                let glyph = &row.area_glyphs(TEXT_AREA)[it.hpos as usize];
                width = i64::from(glyph.pixel_width);
                height = i64::from(glyph.ascent) + i64::from(glyph.descent);
            } else {
                height = i64::from(row.height);
            }
        }
        let x1 = 0.max(it.current_x + it.pixel_width);
        if to_x > x1 {
            it.hpos += (to_x - x1) / self.window_frame_column_width(w) as i32;
        }
        Ok(BufferPosn {
            string,
            pos,
            object: None,
            x: i64::from(it.hpos),
            y: i64::from(it.vpos),
            dx,
            dy,
            width,
            height,
        })
    }

    pub fn mode_line_string(&self, w: Value, part: WindowPart, x: i64, y: i64) -> AreaHit {
        let m = self
            .window_matrix(w, WhichMatrix::Current)
            .expect("current matrix");
        let vpos = match part {
            WindowPart::ModeLine => m.mode_line_row(),
            WindowPart::TabLine => 0,
            _ => m.header_line_row(),
        };
        let row = &m.rows[vpos];
        let y0 = y - i64::from(row.y);
        let mut hit = AreaHit {
            string: Value::NIL,
            charpos: 0,
            object: None,
            x,
            y: vpos as i64 - m.first_text_row() as i64,
            dx: 0,
            dy: y0,
            width: 0,
            height: 0,
        };
        let mut x0;
        if row.mode_line_p && row.enabled_p {
            let glyphs = row.area_glyphs(TEXT_AREA);
            x0 = x;
            let mut i = 0;
            while i < glyphs.len() && x0 >= i64::from(glyphs[i].pixel_width) {
                x0 -= i64::from(glyphs[i].pixel_width);
                i += 1;
            }
            hit.x = i as i64;
            if let Some(glyph) = glyphs.get(i) {
                hit.string = glyph.object;
                hit.charpos = glyph.charpos;
                hit.width = i64::from(glyph.pixel_width);
                hit.height = i64::from(glyph.ascent) + i64::from(glyph.descent);
                if glyph.kind == GlyphType::Image {
                    unreachable!("image glyphs need image.c");
                }
            } else {
                hit.x += x0 / self.window_frame_column_width(w);
                hit.height = i64::from(row.height);
            }
        } else {
            hit.x = 0;
            x0 = 0;
        }
        hit.dx = x0;
        hit
    }

    pub fn marginal_area_string(&self, w: Value, part: WindowPart, x: i64, y: i64) -> AreaHit {
        let area = match part {
            WindowPart::LeftMargin => LEFT_MARGIN_AREA,
            WindowPart::RightMargin => RIGHT_MARGIN_AREA,
            _ => unreachable!("emacs_abort in marginal_area_string"),
        };
        let m = self
            .window_matrix(w, WhichMatrix::Current)
            .expect("current matrix");
        let wy = y;
        let mut i = 0;
        while i < m.rows.len() && m.rows[i].enabled_p && i < m.nrows as usize {
            let row = &m.rows[i];
            if wy >= i64::from(row.y) && wy < i64::from(row.y) + i64::from(row.height) {
                break;
            }
            i += 1;
        }
        let row = &m.rows[i.min(m.rows.len() - 1)];
        let mut hit = AreaHit {
            string: Value::NIL,
            charpos: 0,
            object: None,
            x,
            y: i as i64 - m.first_text_row() as i64,
            dx: 0,
            dy: y - i64::from(row.y),
            width: 0,
            height: 0,
        };
        let mut x0;
        if i < m.rows.len() && row.enabled_p {
            x0 = if area == RIGHT_MARGIN_AREA {
                (if self.window_has_fringes_outside_margins(w) {
                    self.window_left_fringe_width(w)
                } else {
                    self.window_fringes_width(w)
                }) + self.window_box_width(w, GlyphRowArea::LeftMargin)
                    + self.window_box_width(w, GlyphRowArea::Text)
            } else if self.window_has_fringes_outside_margins(w) {
                self.window_left_fringe_width(w)
            } else {
                0
            };
            let glyphs = row.area_glyphs(area);
            x0 = x - x0;
            let mut g = 0;
            while g < glyphs.len() && x0 >= i64::from(glyphs[g].pixel_width) {
                x0 -= i64::from(glyphs[g].pixel_width);
                g += 1;
            }
            hit.x = g as i64;
            if let Some(glyph) = glyphs.get(g) {
                hit.string = glyph.object;
                hit.charpos = glyph.charpos;
                hit.width = i64::from(glyph.pixel_width);
                hit.height = i64::from(glyph.ascent) + i64::from(glyph.descent);
                if glyph.kind == GlyphType::Image {
                    unreachable!("image glyphs need image.c");
                }
            } else {
                hit.x += x0 / self.window_frame_column_width(w);
                hit.height = i64::from(row.height);
            }
        } else {
            x0 = 0;
            hit.x = 0;
        }
        hit.dx = x0;
        hit
    }
}
