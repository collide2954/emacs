use crate::bidi::ShelvedBidiCache;
use crate::dispextern::{
    DisplayElementType, Glyph, GlyphRow, GlyphRowArea, GlyphRowRef, GlyphType, GlyphU, It,
    ItMethod, LEFT_MARGIN_AREA, LineWrap, MOVE_TO_POS, MOVE_TO_X, MOVE_TO_Y, MoveItResult,
    RIGHT_MARGIN_AREA, TEXT_AREA, TextPos,
};
use crate::dispnew::matrix::{WhichMatrix, clear_glyph_matrix, row_hash};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::window::MarginUnit;

pub const TRY_WINDOW_CHECK_MARGINS: i32 = 1;
pub const TRY_WINDOW_IGNORE_FONTS_CHANGE: i32 = 2;

#[derive(Clone, Copy)]
struct MinMax {
    min_pos: i64,
    min_bpos: i64,
    max_pos: i64,
    max_bpos: i64,
}
use crate::xfaces::{
    DEFAULT_FACE_ID, FaceBoxType, FaceUnderlineType, LFACE_EXTEND_INDEX, MARGIN_FACE_ID,
};

fn space_glyph() -> Glyph {
    Glyph {
        kind: GlyphType::Char,
        u: GlyphU::Ch(u32::from(b' ')),
        face_id: DEFAULT_FACE_ID as u32,
        charpos: -1,
        ..Glyph::default()
    }
}

fn row_text_area_empty(row: &GlyphRow) -> bool {
    let glyphs = row.area_glyphs(TEXT_AREA);
    let checked = if row.reversed_p {
        &glyphs[..glyphs.len().saturating_sub(1)]
    } else {
        glyphs
    };
    !checked.iter().any(|g| !g.object.is_nil() || g.charpos > 0)
}

impl Lisp {
    pub(super) fn it_row(&self, it: &It) -> &GlyphRow {
        self.glyph_row(it.glyph_row.expect("iterator has no glyph row"))
    }

    pub(super) fn it_row_mut(&mut self, it: &It) -> &mut GlyphRow {
        self.glyph_row_mut(it.glyph_row.expect("iterator has no glyph row"))
    }

    pub(super) fn row_vpos(it: &It) -> usize {
        match it.glyph_row {
            Some(GlyphRowRef::Window { vpos, .. }) => vpos,
            _ => 0,
        }
    }

    fn face_background(&self, f: Value, face_id: usize) -> u64 {
        self.face_from_id(f, face_id)
            .expect("face is not realized")
            .background
    }

    pub(super) fn fill_column_indicator_column(&self, it: &It, char_width: i32) -> i32 {
        if self.var_true(sym::DISPLAY_FILL_COLUMN_INDICATOR)
            && !self.window(it.window).flags.pseudo_window_p
            && it.continuation_lines_width == 0
        {
            let ch = self.find_symbol_value(sym::DISPLAY_FILL_COLUMN_INDICATOR_CHARACTER);
            if !matches!(ch.as_fixnum(), Some(c) if (0..=i64::from(crate::character::MAX_CHAR)).contains(&c))
            {
                return -1;
            }
            let column = self.find_symbol_value(sym::DISPLAY_FILL_COLUMN_INDICATOR_COLUMN);
            let col = if column == sym::T {
                self.find_symbol_value(sym::FILL_COLUMN)
            } else {
                column
            };
            if let Some(c) = col.as_fixnum()
                && (0..=i64::from(i32::MAX)).contains(&c)
                && let Some(v) = (c as i32).checked_mul(char_width)
                && let Some(v) = v.checked_add(it.lnum_pixel_width)
            {
                return v;
            }
        }
        -1
    }

    pub(crate) fn compute_line_metrics(&mut self, it: &mut It) {
        let window_p = self.frame_window_p(it.f);
        let line_height = self.frame(it.f).geometry.line_height as i32;
        if window_p {
            let min_y = (self.window_tab_line_height(it.window)
                + self.window_header_line_height(it.window)) as i32;
            let max_y = self.window_box_height_no_mode_line(it.window) as i32;
            let first_text_row = self
                .window_matrix(it.window, WhichMatrix::Desired)
                .map(|m| m.first_text_row());
            let vpos = Self::row_vpos(it);
            if self.it_row(it).height == 0 {
                if it.max_ascent + it.max_descent == 0 {
                    it.max_descent = line_height;
                    it.max_phys_descent = line_height;
                }
                let r = self.it_row_mut(it);
                r.ascent = it.max_ascent;
                r.height = it.max_ascent + it.max_descent;
                r.phys_ascent = it.max_phys_ascent;
                r.phys_height = it.max_phys_ascent + it.max_phys_descent;
                r.extra_line_spacing = it.max_extra_line_spacing;
            }
            let r = self.it_row_mut(it);
            r.pixel_width = r.x
                + r.area_glyphs(TEXT_AREA)
                    .iter()
                    .map(|g| i32::from(g.pixel_width))
                    .sum::<i32>();
            let overlaps_succ = r.phys_height - r.phys_ascent > r.height - r.ascent;
            let overlaps_pred = r.phys_ascent > r.ascent;
            r.overlapping_p = overlaps_succ || overlaps_pred;
            if first_text_row == Some(vpos) && r.phys_ascent > r.ascent {
                r.height += r.phys_ascent - r.ascent;
                r.ascent = r.phys_ascent;
            }
            r.visible_height = r.height;
            if r.y < min_y {
                r.visible_height -= min_y - r.y;
            }
            if r.y + r.height > max_y {
                r.visible_height -= r.y + r.height - max_y;
            }
        } else {
            let (continuation, truncation) = (
                i32::from(it.continuation_pixel_width),
                i32::from(it.truncation_pixel_width),
            );
            let r = self.it_row_mut(it);
            r.pixel_width = i32::from(r.used[TEXT_AREA]);
            if r.continued_p {
                r.pixel_width -= continuation;
            } else if r.truncated_on_right_p {
                r.pixel_width -= truncation;
            }
            r.ascent = 0;
            r.phys_ascent = 0;
            r.height = 1;
            r.phys_height = 1;
            r.visible_height = 1;
            r.extra_line_spacing = 0;
        }
        let r = self.it_row_mut(it);
        r.hash = row_hash(r);
        it.max_ascent = 0;
        it.max_descent = 0;
        it.max_phys_ascent = 0;
        it.max_phys_descent = 0;
    }

    fn append_space_for_newline(&mut self, it: &mut It, default_face_p: bool) -> LispResult<bool> {
        let (n, room) = {
            let r = self.it_row(it);
            let n = r.used[TEXT_AREA] as usize;
            (n, r.area[TEXT_AREA] + n < r.area[TEXT_AREA + 1])
        };
        if !room {
            return Ok(false);
        }
        let saved_what = it.what;
        let (saved_c, saved_len) = (it.c, it.len);
        let saved_char_to_display = it.char_to_display;
        let saved_x = it.current_x;
        let saved_face_id = it.face_id;
        let saved_box_end = it.end_of_box_run_p;
        let saved_pos = it.position;
        let saved_object = it.object;
        it.what = DisplayElementType::Character;
        it.position = TextPos::new(0, 0);
        it.object = Value::NIL;
        it.len = 1;
        let window_p = self.frame_window_p(it.f);
        let mut char_width = 1;
        if default_face_p || window_p {
            let local_default_face_id =
                self.lookup_basic_face(Some(it.window), it.f, DEFAULT_FACE_ID)?;
            if window_p {
                let font = self.face_font_or_frame_font(it.f, local_default_face_id);
                let fo = self.font_object(font);
                char_width = if fo.average_width != 0 {
                    fo.average_width
                } else {
                    fo.space_width
                };
            }
            if default_face_p {
                it.face_id = local_default_face_id as i32;
            }
        }
        let indicator_column = self.fill_column_indicator_column(it, char_width);
        let saved_end_of_box_run = it.end_of_box_run_p;
        let mut should_keep_end_of_box_run = false;
        let face_id;
        if it.current_x == indicator_column {
            let c = self
                .find_symbol_value(sym::DISPLAY_FILL_COLUMN_INDICATOR_CHARACTER)
                .as_fixnum()
                .unwrap_or(32) as i32;
            it.c = c;
            it.char_to_display = c;
            it.face_id = self.merge_faces(
                it.window,
                sym::FILL_COLUMN_INDICATOR,
                0,
                saved_face_id as usize,
            )? as i32;
            face_id = it.face_id as usize;
        } else {
            it.c = 32;
            it.char_to_display = 32;
            face_id = it.face_id as usize;
            it.face_id = self.face_for_char(it.f, face_id, 0, -1, Value::NIL)? as i32;
            let column_width = self.frame(it.f).geometry.column_width as i32;
            if !(self.it_row(it).reversed_p && saved_x + column_width < it.last_visible_x) {
                should_keep_end_of_box_run = true;
            }
        }
        self.produce_glyphs_macro(it)?;
        if should_keep_end_of_box_run {
            it.end_of_box_run_p = saved_end_of_box_run;
        }
        if window_p {
            let font = self.face_font_or_frame_font(it.f, face_id);
            if n == 0 {
                let mut extra_line_spacing = it.extra_line_spacing;
                let extra_line_spacing_above = it.extra_line_spacing_above;
                let fo = self.font_object(font);
                let (fb_boff, vertical_centering) = (fo.baseline_offset, fo.vertical_centering);
                let mut boff = fb_boff;
                if vertical_centering {
                    boff = self.vcenter_baseline_offset(font, it.f) - boff;
                }
                it.object = saved_object;
                let (ascent, descent) = self.normal_char_ascent_descent(font, -1);
                it.ascent = ascent;
                it.descent = descent;
                let mut height = self.get_it_property(it, sym::LINE_HEIGHT)?;
                let mut total_height = Value::NIL;
                if height.is_cons()
                    && self.xcdr(height).is_cons()
                    && self.xcdr(self.xcdr(height)).is_nil()
                {
                    total_height = self.xcar(self.xcdr(height));
                    height = self.xcar(height);
                }
                let base = self.stack.len();
                self.stack.push(height);
                self.stack.push(total_height);
                let height = self.calc_line_height_property(it, height, font, boff, true)?;
                self.stack.push(height);
                if it.override_ascent >= 0 {
                    it.ascent = it.override_ascent;
                    it.descent = it.override_descent;
                    boff = it.override_boff;
                }
                if height == sym::T {
                    extra_line_spacing = 0;
                } else {
                    it.phys_ascent = it.ascent;
                    it.phys_descent = it.descent;
                    if let Some(h) = height.as_fixnum()
                        && h > i64::from(it.ascent + it.descent)
                    {
                        it.ascent = h as i32 - it.descent;
                    }
                    let spacing = if !total_height.is_nil() {
                        self.calc_line_height_property(it, total_height, font, boff, false)?
                    } else {
                        let spacing = self.get_it_property(it, sym::LINE_SPACING)?;
                        self.calc_line_height_property(it, spacing, font, boff, false)?
                    };
                    if let Some(s) = spacing.as_fixnum() {
                        extra_line_spacing = s as i32;
                        if !total_height.is_nil() {
                            extra_line_spacing -= it.phys_ascent + it.phys_descent;
                        }
                    }
                }
                self.stack.truncate(base);
                if extra_line_spacing > 0 {
                    it.descent += extra_line_spacing - extra_line_spacing_above;
                    it.ascent += extra_line_spacing_above;
                    if extra_line_spacing > it.max_extra_line_spacing {
                        it.max_extra_line_spacing = extra_line_spacing;
                    }
                }
                it.max_ascent = it.ascent;
                it.max_descent = it.descent;
                self.it_row_mut(it).height = 0;
            }
            let (max_ascent, max_descent) = (it.max_ascent, it.max_descent);
            let r = self.it_row_mut(it);
            let index = r.area[TEXT_AREA] + n;
            r.glyphs[index].ascent = max_ascent as i16;
            r.glyphs[index].descent = max_descent as i16;
        }
        it.override_ascent = -1;
        it.constrain_row_ascent_descent_p = false;
        it.current_x = saved_x;
        it.object = saved_object;
        it.position = saved_pos;
        it.what = saved_what;
        it.face_id = saved_face_id;
        it.len = saved_len;
        it.c = saved_c;
        it.char_to_display = saved_char_to_display;
        it.end_of_box_run_p = saved_box_end;
        Ok(true)
    }

    fn face_font_or_frame_font(&self, f: Value, face_id: usize) -> Value {
        let font = self
            .face_from_id(f, face_id)
            .expect("face is not realized")
            .font;
        if font.is_nil() {
            self.frame(f).font
        } else {
            font
        }
    }

    pub(crate) fn extend_face_to_end_of_line(&mut self, it: &mut It) -> LispResult<()> {
        let f = it.f;
        let w = it.window;
        let window_p = self.frame_window_p(f);
        let margins = self.window_left_margin_width(w) > 0 || self.window_right_margin_width(w) > 0;
        {
            let r = self.it_row(it);
            let grace = i32::from(!window_p && r.reversed_p && !r.continued_p);
            if it.current_x >= it.last_visible_x + grace && !margins {
                return Ok(());
            }
        }
        let count = self.specpdl.len();
        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
        let default_face_id = self.lookup_basic_face(Some(w), f, DEFAULT_FACE_ID)?;
        if self.face_from_id(f, default_face_id).is_none() {
            return Ok(());
        }
        let default_face_id = self.face_from_id(f, default_face_id).expect("face").id;
        let ends_at_zv_p = self.it_row(it).ends_at_zv_p;
        let extend_face_id = if it.face_id as usize == default_face_id || it.s.is_some() {
            it.face_id as usize
        } else if ends_at_zv_p {
            default_face_id
        } else {
            self.face_at_pos(it, LFACE_EXTEND_INDEX)? as usize
        };
        self.unbind_to(count)?;
        let face_id = if it.face_before_selective_p {
            it.saved_face_id as usize
        } else {
            extend_face_id
        };
        let face = self
            .face_from_id(f, face_id)
            .expect("face is not realized")
            .clone();
        let margin_fill_face_id = self.lookup_basic_face(Some(w), f, MARGIN_FACE_ID)?;
        let frame_background = self.frame(f).background_pixel as u64;
        let margin_background = self.face_background(f, margin_fill_face_id);
        let fill_column = self.var_true(sym::DISPLAY_FILL_COLUMN_INDICATOR);
        if window_p
            && self.it_row(it).displays_text_p
            && face.box_type == FaceBoxType::NoBox
            && face.underline == FaceUnderlineType::NoUnderline
            && !face.overline_p
            && !face.strike_through_p
            && face.background == frame_background
            && !self.it_row(it).reversed_p
            && !fill_column
            && !(margin_background != frame_background && margins)
        {
            return Ok(());
        }
        self.it_row_mut(it).fill_line_p = true;
        let orig_face_id = it.face_id;
        if !crate::character::ascii_char_p(it.c as u32) {
            it.face_id = self.face_for_char(f, face.id, 0, -1, Value::NIL)? as i32;
        }
        if window_p {
            if self.it_row(it).used[TEXT_AREA] == 0 {
                let r = self.it_row_mut(it);
                let index = r.area[TEXT_AREA];
                r.glyphs[index] = space_glyph();
                r.glyphs[index].face_id = face.id as u32;
                r.glyphs[index].frame = f;
                r.used[TEXT_AREA] = 1;
            }
            let tab_bar = self.frame(f).tab_bar_window;
            if !(self.it_row(it).mode_line_p || (self.is_window(tab_bar) && w == tab_bar)) {
                let font = self.face_font_or_frame_font(f, default_face_id);
                let (font_ascent, font_height, average_width, space_width) = {
                    let fo = self.font_object(font);
                    (fo.ascent, fo.height, fo.average_width, fo.space_width)
                };
                for (area, margin_width, margin_cols) in [
                    (
                        LEFT_MARGIN_AREA,
                        self.window_left_margin_width(w),
                        self.window(w).left_margin_cols,
                    ),
                    (
                        RIGHT_MARGIN_AREA,
                        self.window_right_margin_width(w),
                        self.window(w).right_margin_cols,
                    ),
                ] {
                    let used = i64::from(self.it_row(it).used[area]);
                    if margin_width > 0 && used < margin_cols {
                        let existing: i64 = self
                            .it_row(it)
                            .area_glyphs(area)
                            .iter()
                            .map(|g| i64::from(g.pixel_width))
                            .sum();
                        let remaining_pixels = (margin_width - existing) as i32;
                        if remaining_pixels > 0 {
                            let saved_face_id = it.face_id;
                            let saved_area = it.area;
                            let saved_pos = it.position;
                            it.face_id = margin_fill_face_id as i32;
                            it.area = if area == LEFT_MARGIN_AREA {
                                GlyphRowArea::LeftMargin
                            } else {
                                GlyphRowArea::RightMargin
                            };
                            it.position = TextPos::new(0, 0);
                            let stretch_ascent =
                                ((it.ascent + it.descent) * font_ascent) / font_height;
                            let height = it.ascent + it.descent;
                            self.append_stretch_glyph(
                                it,
                                Value::NIL,
                                remaining_pixels,
                                height,
                                stretch_ascent,
                            );
                            it.position = saved_pos;
                            it.face_id = saved_face_id;
                            it.area = saved_area;
                        }
                    }
                }
                let char_width = if average_width != 0 {
                    average_width
                } else {
                    space_width
                };
                let indicator_column = self.fill_column_indicator_column(it, char_width);
                let saved_char = it.char_to_display;
                let saved_pos = it.position;
                let saved_avoid_cursor = it.avoid_cursor_p;
                let saved_box_start = it.start_of_box_run_p;
                let save_object = it.object;
                let saved_face_id = it.face_id;
                it.face_id = extend_face_id as i32;
                it.avoid_cursor_p = true;
                it.object = Value::NIL;
                let stretch_height = it.ascent + it.descent;
                let stretch_ascent = (stretch_height * font_ascent) / font_height;
                if indicator_column >= 0
                    && indicator_column > it.current_x
                    && indicator_column < it.last_visible_x
                {
                    let stretch_width = indicator_column - it.current_x - char_width;
                    it.position = TextPos::new(0, 0);
                    if stretch_width > 0 {
                        self.append_stretch_glyph(
                            it,
                            Value::NIL,
                            stretch_width,
                            stretch_height,
                            stretch_ascent,
                        );
                    }
                    if it.current_x < indicator_column {
                        let save_face_id = it.face_id;
                        let save_ascent = it.ascent;
                        let save_descent = it.descent;
                        it.char_to_display = self
                            .find_symbol_value(sym::DISPLAY_FILL_COLUMN_INDICATOR_CHARACTER)
                            .as_fixnum()
                            .unwrap_or(32) as i32;
                        it.face_id =
                            self.merge_faces(w, sym::FILL_COLUMN_INDICATOR, 0, extend_face_id)?
                                as i32;
                        self.produce_glyphs_macro(it)?;
                        it.face_id = save_face_id;
                        it.ascent = save_ascent;
                        it.descent = save_descent;
                    }
                }
                if !self.it_row(it).reversed_p {
                    let stretch_width = it.last_visible_x - it.current_x;
                    if stretch_width > 0 {
                        it.position = TextPos::new(0, 0);
                        self.append_stretch_glyph(
                            it,
                            Value::NIL,
                            stretch_width,
                            stretch_height,
                            stretch_ascent,
                        );
                    }
                }
                it.char_to_display = saved_char;
                it.position = saved_pos;
                it.avoid_cursor_p = saved_avoid_cursor;
                it.start_of_box_run_p = saved_box_start;
                it.object = save_object;
                it.face_id = saved_face_id;
            }
            if self.it_row(it).reversed_p {
                let font = self.face_font_or_frame_font(f, face.id);
                let (font_ascent, font_height) = {
                    let fo = self.font_object(font);
                    (fo.ascent, fo.height)
                };
                let row_width: i32 = self
                    .it_row(it)
                    .area_glyphs(TEXT_AREA)
                    .iter()
                    .map(|g| i32::from(g.pixel_width))
                    .sum();
                let (lf, rf) = (
                    self.window_left_fringe_width(w),
                    self.window_right_fringe_width(w),
                );
                let mut stretch_width = if (lf == 0) == (rf == 0) || rf != 0 {
                    self.window_box_width(w, GlyphRowArea::Text) as i32
                } else {
                    it.last_visible_x - it.first_visible_x
                };
                stretch_width -= row_width;
                if stretch_width > 0 {
                    let stretch_ascent = ((it.ascent + it.descent) * font_ascent) / font_height;
                    let saved_pos = it.position;
                    it.position = TextPos::new(0, 0);
                    let saved_avoid_cursor = it.avoid_cursor_p;
                    it.avoid_cursor_p = true;
                    let saved_face_id = it.face_id;
                    let saved_box_start = it.start_of_box_run_p;
                    it.face_id = if self.it_row(it).ends_at_zv_p {
                        default_face_id as i32
                    } else {
                        face.id as i32
                    };
                    it.start_of_box_run_p = false;
                    let height = it.ascent + it.descent;
                    self.append_stretch_glyph(
                        it,
                        Value::NIL,
                        stretch_width,
                        height,
                        stretch_ascent,
                    );
                    it.position = saved_pos;
                    it.avoid_cursor_p = saved_avoid_cursor;
                    it.face_id = saved_face_id;
                    it.start_of_box_run_p = saved_box_start;
                }
                if stretch_width < 0 {
                    self.it_row_mut(it).x = stretch_width;
                }
            }
            it.face_id = orig_face_id;
        } else {
            unreachable!("tty frames have no glyph matrices");
        }
        Ok(())
    }

    fn trailing_whitespace_p(&self, charpos: i64) -> bool {
        let b = self.cur();
        let mut bytepos = b.text.char_to_byte(charpos as usize) as i64;
        let zv_byte = b.zv_byte as i64;
        let mut c = 0u8;
        while bytepos < zv_byte {
            c = b.text.byte_at(bytepos as usize);
            if c != b' ' && c != b'\t' {
                break;
            }
            bytepos += 1;
        }
        if (bytepos >= zv_byte || c == b'\n' || c == b'\r') && bytepos != b.pt_byte as i64 {
            return true;
        }
        false
    }

    fn highlight_trailing_whitespace(&mut self, it: &It) -> LispResult<()> {
        let (glyphs, reversed) = {
            let r = self.it_row(it);
            (r.area_glyphs(TEXT_AREA).to_vec(), r.reversed_p)
        };
        let used = glyphs.len() as isize;
        if used == 0 {
            return Ok(());
        }
        let is_blank = |g: &Glyph| {
            g.kind == GlyphType::Stretch || (g.kind == GlyphType::Char && g.u == GlyphU::Ch(32))
        };
        let (mut glyph, start, step): (isize, isize, isize) = if reversed {
            (0, used - 1, 1)
        } else {
            (used - 1, 0, -1)
        };
        let in_range = |g: isize| if reversed { g <= start } else { g >= start };
        while in_range(glyph)
            && matches!(
                glyphs[glyph as usize].kind,
                GlyphType::Char | GlyphType::Stretch
            )
            && glyphs[glyph as usize].object.is_nil()
        {
            glyph += step;
        }
        if in_range(glyph)
            && self.is_buffer(glyphs[glyph as usize].object)
            && is_blank(&glyphs[glyph as usize])
            && self.trailing_whitespace_p(glyphs[glyph as usize].charpos)
        {
            let face_id =
                self.lookup_named_face(Some(it.window), it.f, sym::TRAILING_WHITESPACE, false)?;
            if face_id < 0 {
                return Ok(());
            }
            let f = it.f;
            let mut changes = Vec::new();
            while in_range(glyph)
                && self.is_buffer(glyphs[glyph as usize].object)
                && is_blank(&glyphs[glyph as usize])
            {
                changes.push(glyph as usize);
                glyph += step;
            }
            let r = self.it_row_mut(it);
            let base = r.area[TEXT_AREA];
            for i in changes {
                r.glyphs[base + i].frame = f;
                r.glyphs[base + i].face_id = face_id as u32;
            }
        }
        Ok(())
    }

    pub(crate) fn unproduce_glyphs(&mut self, it: &It, n: i32) {
        let r = self.it_row_mut(it);
        let used = r.used[TEXT_AREA] as usize;
        let n = (n.max(0) as usize).min(used);
        let start = r.area[TEXT_AREA];
        r.glyphs.copy_within(start + n..start + used, start);
    }

    #[allow(clippy::too_many_arguments)]
    fn find_row_edges(
        &mut self,
        it: &It,
        min_pos: i64,
        min_bpos: i64,
        max_pos: i64,
        max_bpos: i64,
    ) {
        let zv = self.cur().zv as i64;
        let vpos = Self::row_vpos(it);
        let mut max_pos = max_pos;
        let mut max_bpos = max_bpos;
        let (start_charpos, used, ends_at_zv_p) = {
            let r = self.it_row(it);
            (r.start.pos.charpos, r.used[TEXT_AREA], r.ends_at_zv_p)
        };
        let minpos = if min_pos <= zv && min_pos < start_charpos {
            TextPos::new(min_pos, min_bpos)
        } else {
            self.it_row(it).start.pos
        };
        self.it_row_mut(it).minpos = minpos;
        if max_pos <= 0 {
            max_pos = it.current.pos.charpos;
            max_bpos = it.current.pos.bytepos;
        }
        let maxpos = if ends_at_zv_p {
            it.current.pos
        } else if used != 0 {
            let mut seen_this_string = false;
            if it.object.is_string()
                && vpos > 0
                && let Some(m) = self.window_matrix(it.window, WhichMatrix::Desired)
            {
                let r1 = &m.rows[vpos - 1];
                if !r1.mode_line_p && r1.ends_in_newline_from_string_p {
                    let g1 = r1.area_glyphs(TEXT_AREA);
                    if !r1.reversed_p {
                        let mut end = g1.len();
                        while end > 0 && g1[end - 1].object.is_nil() && g1[end - 1].charpos <= 0 {
                            end -= 1;
                        }
                        seen_this_string = end == 0 || g1[end - 1].object == it.object;
                    } else {
                        let mut e = 0;
                        while e < g1.len() && g1[e].object.is_nil() && g1[e].charpos <= 0 {
                            e += 1;
                        }
                        seen_this_string = e == g1.len() || g1[e].object == it.object;
                    }
                }
            }
            let r = self.it_row(it);
            if r.ends_in_newline_from_string_p && !seen_this_string {
                if r.end.pos.charpos > max_pos {
                    let p = self.inc_text_pos(TextPos::new(max_pos, max_bpos), it.multibyte_p);
                    max_pos = p.charpos;
                    max_bpos = p.bytepos;
                }
                TextPos::new(max_pos, max_bpos)
            } else if it.eol_pos.charpos > 0 {
                TextPos::new(it.eol_pos.charpos + 1, it.eol_pos.bytepos + 1)
            } else if r.continued_p {
                if it.charpos() == max_pos && it.method != ItMethod::FromBuffer {
                    TextPos::new(max_pos, max_bpos)
                } else {
                    self.inc_text_pos(TextPos::new(max_pos, max_bpos), it.multibyte_p)
                }
            } else if r.truncated_on_right_p {
                it.current.pos
            } else if max_pos == min_pos && it.method != ItMethod::FromBuffer {
                r.minpos
            } else {
                std::process::abort();
            }
        } else {
            it.current.pos
        };
        self.it_row_mut(it).maxpos = maxpos;
    }

    pub(crate) fn insert_left_trunc_glyphs(&mut self, it: &mut It) -> LispResult<()> {
        let mut truncate_it = it.clone();
        truncate_it.current_x = 0;
        truncate_it.wrap_prefix_width = 0;
        truncate_it.face_id = DEFAULT_FACE_ID as i32;
        truncate_it.glyph_row = Some(GlyphRowRef::Scratch);
        truncate_it.area = GlyphRowArea::Text;
        self.xdisp.scratch_glyph_row.used[TEXT_AREA] = 0;
        truncate_it.position = TextPos::new(-1, -1);
        truncate_it.object = Value::NIL;
        let dir = it.bidi_it.paragraph_dir;
        self.produce_special_glyphs(&mut truncate_it, DisplayElementType::Truncation, dir, true)?;
        let trunc: Vec<Glyph> = self.xdisp.scratch_glyph_row.area_glyphs(TEXT_AREA).to_vec();
        let tused = trunc.len();
        let window_p = self.frame_window_p(it.f);
        let right_fringe = self.window_right_fringe_width(it.window);
        let left_fringe = self.window_left_fringe_width(it.window);
        let truncation_pixel_width = i32::from(it.truncation_pixel_width);
        let r = self.it_row_mut(it);
        let base = r.area[TEXT_AREA];
        let used = r.used[TEXT_AREA] as usize;
        if !r.reversed_p {
            let mut toend = used;
            if window_p {
                r.x = 0;
                let mut w = 0;
                let mut g = 0;
                while g < toend && w < truncation_pixel_width {
                    w += i32::from(r.glyphs[base + g].pixel_width);
                    g += 1;
                }
                if g > tused {
                    r.glyphs.copy_within(base + g..base + toend, base + tused);
                    r.used[TEXT_AREA] -= (g - tused) as i16;
                    toend -= g - tused;
                }
                let used = r.used[TEXT_AREA] as usize;
                if r.truncated_on_right_p
                    && right_fringe == 0
                    && used >= 2
                    && r.glyphs[base + used - 2].kind == GlyphType::Stretch
                {
                    let extra = w - truncation_pixel_width;
                    r.glyphs[base + used - 2].pixel_width += extra as i16;
                }
            }
            let mut to = 0;
            for g in &trunc {
                r.glyphs[base + to] = *g;
                to += 1;
            }
            if !window_p {
                unreachable!("tty frames have no glyph matrices");
            }
            if to > toend {
                r.used[TEXT_AREA] = to as i16;
            }
        } else {
            let mut to = used as isize - 1;
            if window_p {
                let mut w = 0;
                let mut g = to;
                while g >= 0 && w < truncation_pixel_width {
                    w += i32::from(r.glyphs[base + g as usize].pixel_width);
                    g -= 1;
                }
                if to - g - tused as isize > 0 {
                    to = g + tused as isize;
                }
                if r.truncated_on_right_p
                    && left_fringe == 0
                    && used > 1
                    && r.glyphs[base + 1].kind == GlyphType::Stretch
                {
                    let extra = w - truncation_pixel_width;
                    r.glyphs[base + 1].pixel_width += extra as i16;
                }
            }
            let mut from = tused as isize - 1;
            while from >= 0 && to >= 0 {
                r.glyphs[base + to as usize] = trunc[from as usize];
                to -= 1;
                from -= 1;
            }
            if from >= 0 {
                let move_by = (from + 1) as usize;
                let used = r.used[TEXT_AREA] as usize;
                r.glyphs.copy_within(base..base + used, base + move_by);
                while from >= 0 {
                    r.glyphs[(base as isize + to + move_by as isize) as usize] =
                        trunc[from as usize];
                    to -= 1;
                    from -= 1;
                }
                r.used[TEXT_AREA] += move_by as i16;
            }
        }
        Ok(())
    }

    fn overlay_arrow_at_row(&self, it: &It) -> Value {
        let (start, end) = {
            let r = self.it_row(it);
            (r.minpos.charpos, r.maxpos.charpos)
        };
        let mut vlist = self.find_symbol_value(sym::OVERLAY_ARROW_VARIABLE_LIST);
        while vlist.is_cons() {
            let var = self.xcar(vlist);
            vlist = self.xcdr(vlist);
            if !var.is_symbol() {
                continue;
            }
            let val = self.find_symbol_value(var);
            if self.is_marker(val) && self.marker(val).buffer == self.current_buffer {
                let pos = self.marker(val).charpos as i64;
                if start <= pos && pos < end {
                    if self.frame_window_p(it.f) && self.window_left_fringe_width(it.window) > 0 {
                        let bitmap = self.get(var, sym::OVERLAY_ARROW_BITMAP);
                        if bitmap.is_symbol() {
                            let fringe_bitmap = self.lookup_fringe_bitmap(bitmap);
                            if fringe_bitmap != 0 {
                                return Value::fixnum(fringe_bitmap);
                            }
                        }
                        return Value::fixnum(-1);
                    }
                    return self.overlay_arrow_string_or_property(var);
                }
            }
        }
        Value::NIL
    }

    fn compute_char_face(&mut self, f: Value, ch: i32, prop: Value) -> LispResult<usize> {
        let ch = if self.cur().text.multibyte { ch } else { 0 };
        if prop.is_nil() {
            return self.face_for_char(f, DEFAULT_FACE_ID, ch as u32, -1, Value::NIL);
        }
        self.face_for_prop_on_default(f, prop)
    }

    fn get_overlay_arrow_glyph_row(
        &mut self,
        w: Value,
        overlay_arrow_string: Value,
        face_id: usize,
    ) -> LispResult<Vec<Glyph>> {
        let f = self.window(w).frame;
        let buffer = self.window(w).contents;
        let old = self.current_buffer;
        self.set_buffer_internal(buffer);
        let r = (|| -> LispResult<()> {
            let mut it = It::default();
            self.init_iterator(
                &mut it,
                w,
                -1,
                -1,
                Some(GlyphRowRef::Scratch),
                DEFAULT_FACE_ID,
            )?;
            self.xdisp.scratch_glyph_row.reversed_p = false;
            self.xdisp.scratch_glyph_row.used[TEXT_AREA] = 0;
            it.position = TextPos::new(0, 0);
            let multibyte_p = self.cur().text.multibyte;
            let bytes = self.string_bytes(overlay_arrow_string).to_vec();
            let mut p = 0;
            let mut char_num = 0;
            while p < bytes.len() {
                if multibyte_p {
                    let (c, len) = crate::character::string_char(&bytes[p..]);
                    it.c = c as i32;
                    it.char_to_display = c as i32;
                    it.len = len as i32;
                } else {
                    it.c = i32::from(bytes[p]);
                    it.char_to_display = it.c;
                    it.len = 1;
                    if !crate::character::ascii_char_p(it.c as u32) {
                        it.char_to_display = crate::character::byte8_to_char(it.c as u8) as i32;
                    }
                }
                p += it.len as usize;
                let ilisp = Value::fixnum(char_num);
                char_num += 1;
                let face = crate::textprop::f_get_text_property(
                    self,
                    &[ilisp, sym::FACE, overlay_arrow_string],
                )?;
                it.face_id = if face.is_nil() {
                    self.lookup_basic_face(Some(w), f, face_id)? as i32
                } else {
                    self.compute_char_face(f, it.char_to_display, face)? as i32
                };
                let n_glyphs_before = self.xdisp.scratch_glyph_row.used[TEXT_AREA];
                it.position = TextPos::new(-1, -1);
                self.produce_glyphs_macro(&mut it)?;
                if it.current_x > it.last_visible_x {
                    self.xdisp.scratch_glyph_row.used[TEXT_AREA] = n_glyphs_before;
                    break;
                }
            }
            Ok(())
        })();
        self.set_buffer_internal(old);
        r?;
        Ok(self.xdisp.scratch_glyph_row.area_glyphs(TEXT_AREA).to_vec())
    }

    fn fast_move_it_horizontally(&mut self, it: &mut It, nchars: i64) -> LispResult<MoveItResult> {
        let zv = self.cur().zv;
        let (nl, _) = self.find_newline(it.charpos() as usize, zv, 1);
        let mut nl_pos = nl as i64;
        let mut nl_bytepos = self.cur().text.char_to_byte(nl) as i64;
        let move_result;
        let new_pos;
        if nl_pos - it.charpos() > nchars {
            let pos = it.charpos() + nchars;
            new_pos = TextPos::new(pos, self.cur().text.char_to_byte(pos as usize) as i64);
            move_result = MoveItResult::XReached;
        } else {
            let (zv_byte, begv_byte) = (self.cur().zv_byte as i64, self.cur().begv_byte as i64);
            if nl_bytepos < zv_byte
                || (nl_bytepos > begv_byte
                    && self.cur().text.byte_at((nl_bytepos - 1) as usize) == b'\n')
            {
                nl_pos -= 1;
                nl_bytepos -= 1;
                move_result = MoveItResult::NewlineOrCr;
            } else {
                move_result = MoveItResult::PosMatchOrZv;
            }
            new_pos = TextPos::new(nl_pos, nl_bytepos);
        }
        self.reseat(it, new_pos, false)?;
        Ok(move_result)
    }

    fn record_max_min_pos(&self, it: &It, mm: &mut MinMax) {
        let composition_p = !it.string.is_string() && it.what == DisplayElementType::Composition;
        let current_pos = if composition_p {
            it.cmp_it.charpos
        } else {
            it.charpos()
        };
        let current_bpos = if composition_p {
            self.cur().text.char_to_byte(current_pos as usize) as i64
        } else {
            it.bytepos()
        };
        if current_pos < mm.min_pos {
            mm.min_pos = current_pos;
            mm.min_bpos = current_bpos;
        }
        if it.charpos() > mm.max_pos {
            mm.max_pos = it.charpos();
            mm.max_bpos = it.bytepos();
        }
    }

    fn grow_row_metrics(&mut self, row: GlyphRowRef, it: &It) {
        let r = self.glyph_row_mut(row);
        r.ascent = r.ascent.max(it.max_ascent);
        r.height = r.height.max(it.max_ascent + it.max_descent);
        r.phys_ascent = r.phys_ascent.max(it.max_phys_ascent);
        r.phys_height = r.phys_height.max(it.max_phys_ascent + it.max_phys_descent);
        r.extra_line_spacing = r.extra_line_spacing.max(it.max_extra_line_spacing);
    }

    fn window_has_margins(&self, w: Value) -> bool {
        self.window_left_margin_width(w) > 0 || self.window_right_margin_width(w) > 0
    }

    fn fringe_at_row_end(&self, w: Value, reversed: bool) -> i64 {
        if reversed {
            self.window_left_fringe_width(w)
        } else {
            self.window_right_fringe_width(w)
        }
    }

    fn finish_line_at_eol(&mut self, it: &mut It, row: GlyphRowRef) -> LispResult<()> {
        let used_before = self.glyph_row(row).used[TEXT_AREA];
        self.glyph_row_mut(row).ends_in_newline_from_string_p = it.object.is_string();
        if !self.it_overflow_newline_into_fringe(it) {
            self.append_space_for_newline(it, false)?;
        }
        self.extend_face_to_end_of_line(it)?;
        if used_before == 0 {
            let charpos = it.position.charpos;
            let r = self.glyph_row_mut(row);
            let i = r.area[TEXT_AREA];
            r.glyphs[i].charpos = charpos;
        }
        it.eol_pos = it.current.pos;
        self.set_iterator_to_next(it, true)?;
        it.continuation_lines_width = 0;
        Ok(())
    }

    pub(super) fn display_line(&mut self, it: &mut It, cursor_vpos: i64) -> LispResult<bool> {
        let row = it.glyph_row.expect("display_line needs a glyph row");
        let w = it.window;
        let f = it.f;
        let vpos = Self::row_vpos(it);
        let mut wrap_it: Option<It> = None;
        let mut prev_it: Option<It> = None;
        let mut wrap_data: ShelvedBidiCache = None;
        let mut prev_data: ShelvedBidiCache = None;
        let mut may_wrap = false;
        let mut wrap_x = 0;
        let mut wrap_row_used: i32 = -1;
        let mut wrap_row = (0, 0, 0, 0, 0);
        let zv = self.cur().zv as i64;
        let mut mm = MinMax {
            min_pos: zv + 1,
            min_bpos: 0,
            max_pos: 0,
            max_bpos: 0,
        };
        let mut wrap_mm = mm;
        let mut pending_handle_line_prefix = false;
        let tab_line = i64::from(self.window_wants_tab_line(w));
        let header_line = i64::from(self.window_wants_header_line(w));
        let hscroll_this_line = cursor_vpos >= 0
            && i64::from(it.vpos) == cursor_vpos - tab_line - header_line
            && self.hscrolling_current_line_p(w);
        let first_visible_x = it.first_visible_x;
        let last_visible_x = it.last_visible_x;
        let mut x_incr = 0;
        let mut this_line_subject_to_line_prefix = 0;
        let nrows = self
            .window_matrix(w, WhichMatrix::Desired)
            .map_or(0, |m| m.nrows);
        if vpos as i32 >= nrows {
            self.window_mut(w).nrows_scale_factor += 1;
            self.frame_mut(f).fonts_changed = true;
            return Ok(false);
        }
        self.prepare_desired_row(w, vpos, false);
        {
            let r = self.glyph_row_mut(row);
            r.y = it.current_y;
            r.start = it.start;
            r.continuation_lines_width = it.continuation_lines_width;
            r.displays_text_p = true;
            r.starts_in_middle_of_char_p = it.starts_in_middle_of_char_p;
        }
        it.starts_in_middle_of_char_p = false;
        it.stretch_adjust = 0;
        it.line_number_produced_p = false;
        let column_width = self.frame(f).geometry.column_width as i32;
        if hscroll_this_line {
            x_incr =
                (self.window_hscroll_limited(w) - self.window(w).min_hscroll) as i32 * column_width;
        }
        let line_number_needed = self.should_produce_line_number(it)?;
        if it.current_x < it.first_visible_x + x_incr {
            self.xdisp.this_line_min_pos = self.glyph_row(row).start.pos;
            if hscroll_this_line {
                it.first_visible_x += x_incr;
                it.last_visible_x += x_incr;
            }
            let move_result = if self.cur().long_line_optimizations_p
                && it.line_wrap == LineWrap::Truncate
                && self.window_hscroll_limited(w) > self.int_var(sym::LARGE_HSCROLL_THRESHOLD)
            {
                let chars_to_skip = i64::from(it.first_visible_x / column_width);
                let result = self.fast_move_it_horizontally(it, chars_to_skip)?;
                it.current_x = if result == MoveItResult::XReached {
                    it.first_visible_x
                } else {
                    it.first_visible_x - column_width
                };
                result
            } else {
                let to_x = it.first_visible_x;
                self.move_it_in_display_line_to(it, zv, to_x, MOVE_TO_POS | MOVE_TO_X)?
            };
            if it.current_x < it.first_visible_x
                && matches!(
                    move_result,
                    MoveItResult::NewlineOrCr | MoveItResult::PosMatchOrZv
                )
            {
                it.current_x = it.first_visible_x;
                it.wrap_prefix_width = 0;
            }
            it.line_number_produced_p = false;
            mm.min_pos = self.xdisp.this_line_min_pos.charpos;
            mm.min_bpos = self.xdisp.this_line_min_pos.bytepos;
            if line_number_needed {
                self.maybe_produce_line_number(it)?;
            }
        } else if it.area == GlyphRowArea::Text {
            if line_number_needed {
                self.maybe_produce_line_number(it)?;
            }
            self.handle_line_prefix(it)?;
            this_line_subject_to_line_prefix = i32::from(it.string_from_prefix_prop_p);
        } else {
            pending_handle_line_prefix = true;
        }
        {
            let r = self.glyph_row_mut(row);
            r.ascent = it.max_ascent;
            r.height = it.max_ascent + it.max_descent;
            r.phys_ascent = it.max_phys_ascent;
            r.phys_height = it.max_phys_ascent + it.max_phys_descent;
            r.extra_line_spacing = it.max_extra_line_spacing;
        }
        let window_p = self.frame_window_p(f);
        loop {
            if !self.get_next_display_element(it)? {
                let mut row_has_glyphs = false;
                if self.it_overflow_newline_into_fringe(it) {
                    self.glyph_row_mut(row).exact_window_width_line_p = true;
                } else {
                    let appended = self.append_space_for_newline(it, true)?;
                    let used = self.glyph_row(row).used[TEXT_AREA];
                    let blank = if (appended && used == 1) || used == 0 {
                        true
                    } else {
                        row_has_glyphs = row_text_area_empty(self.glyph_row(row));
                        row_has_glyphs
                    };
                    if blank {
                        let r = self.glyph_row_mut(row);
                        let i = r.area[TEXT_AREA];
                        r.glyphs[i].charpos = -1;
                        if !row_has_glyphs {
                            r.displays_text_p = false;
                        }
                        let contents = self.window(w).contents;
                        if !self
                            .buffer_local_value(sym::INDICATE_EMPTY_LINES, contents)
                            .is_nil()
                            && !self.window(w).mini
                        {
                            self.glyph_row_mut(row).indicate_empty_line_p = true;
                        }
                    }
                }
                it.continuation_lines_width = 0;
                it.font_height = Value::NIL;
                it.voffset = 0;
                self.glyph_row_mut(row).ends_at_zv_p = true;
                if self.glyph_row(row).reversed_p
                    || self.lookup_basic_face(Some(w), f, DEFAULT_FACE_ID)? != DEFAULT_FACE_ID
                    || self.window_has_margins(w)
                {
                    self.extend_face_to_end_of_line(it)?;
                }
                break;
            }
            let n_glyphs_before = i32::from(self.glyph_row(row).used[it.area.index()]);
            let mut x = it.current_x;
            let (mut ascent, mut descent, mut phys_ascent, mut phys_descent) = (0, 0, 0, 0);
            if it.line_wrap != LineWrap::Truncate {
                ascent = it.max_ascent;
                descent = it.max_descent;
                phys_ascent = it.max_phys_ascent;
                phys_descent = it.max_phys_descent;
                if it.line_wrap == LineWrap::WordWrap && it.area == GlyphRowArea::Text {
                    let next_may_wrap = self.char_can_wrap_after(it)?;
                    if may_wrap && self.char_can_wrap_before(it)? {
                        wrap_it = Some(self.save_it(it, &mut wrap_data));
                        wrap_x = x;
                        let r = self.glyph_row(row);
                        wrap_row_used = i32::from(r.used[TEXT_AREA]);
                        wrap_row = (
                            r.ascent,
                            r.height,
                            r.phys_ascent,
                            r.phys_height,
                            r.extra_line_spacing,
                        );
                        wrap_mm = mm;
                    }
                    may_wrap = next_may_wrap;
                    if may_wrap {
                        prev_it = Some(self.save_it(it, &mut prev_data));
                    }
                }
            }
            self.produce_glyphs_macro(it)?;
            if it.area != GlyphRowArea::Text {
                let area = it.area.index();
                self.grow_row_metrics(row, it);
                self.set_iterator_to_next(it, true)?;
                if !window_p
                    && it.area.index() != area
                    && it.nglyphs > 1
                    && i32::from(self.glyph_row(row).used[area]) < n_glyphs_before + it.nglyphs
                {
                    self.glyph_row_mut(row).used[area] = n_glyphs_before as i16;
                }
                if it.area == GlyphRowArea::Text && pending_handle_line_prefix {
                    if line_number_needed {
                        self.maybe_produce_line_number(it)?;
                    }
                    pending_handle_line_prefix = false;
                    self.handle_line_prefix(it)?;
                    this_line_subject_to_line_prefix = i32::from(it.string_from_prefix_prop_p);
                }
                continue;
            }
            let nglyphs = i32::from(self.glyph_row(row).used[TEXT_AREA]) - n_glyphs_before;
            let hpos_before = it.hpos;
            let x_before = x;
            if nglyphs > 0
                && (it.current_x < it.last_visible_x
                    || (it.line_wrap != LineWrap::Truncate
                        && nglyphs == 1
                        && this_line_subject_to_line_prefix != 0))
            {
                it.hpos += nglyphs;
                self.grow_row_metrics(row, it);
                if it.current_x - it.pixel_width < it.first_visible_x
                    && !line_number_needed
                    && !self.glyph_row(row).reversed_p
                {
                    self.glyph_row_mut(row).x = x - it.first_visible_x;
                }
                if it.bidi_p {
                    self.record_max_min_pos(it, &mut mm);
                }
            } else {
                let mut i = 0;
                while i < nglyphs {
                    let (glyph_index, reversed) = {
                        let r = self.glyph_row(row);
                        let base = r.area[TEXT_AREA];
                        let index = if !r.reversed_p {
                            base + (n_glyphs_before + i) as usize
                        } else {
                            base + (nglyphs - 1 - i) as usize
                        };
                        (index, r.reversed_p)
                    };
                    let new_x = x + i32::from(self.glyph_row(row).glyphs[glyph_index].pixel_width);
                    let edge_fringe = self.fringe_at_row_end(w, reversed);
                    if it.line_wrap != LineWrap::Truncate
                        && (new_x > it.last_visible_x
                            || (new_x == it.last_visible_x && window_p && edge_fringe != 0))
                        && (this_line_subject_to_line_prefix == 0
                            || (i != 0
                                && (x > it.last_visible_x
                                    || (x == it.last_visible_x && window_p && edge_fringe != 0))))
                    {
                        let mut back_to_wrap = false;
                        let start_hpos = if it.lnum_width != 0 {
                            it.lnum_width + 2
                        } else {
                            0
                        };
                        if it.hpos == start_hpos
                            || (new_x == it.last_visible_x && window_p && edge_fringe != 0)
                        {
                            'only: {
                                self.glyph_row_mut(row).continued_p = true;
                                it.current_x = new_x;
                                it.continuation_lines_width += new_x;
                                it.hpos += 1;
                                if i == nglyphs - 1 {
                                    if !self.it_overflow_newline_into_fringe(it)
                                        && wrap_row_used > 0
                                        && (!may_wrap || !self.char_can_wrap_before(it)?)
                                    {
                                        back_to_wrap = true;
                                        break 'only;
                                    }
                                    if it.bidi_p {
                                        self.record_max_min_pos(it, &mut mm);
                                    }
                                    self.set_iterator_to_next(it, true)?;
                                    if self.it_overflow_newline_into_fringe(it) {
                                        if !self.get_next_display_element(it)? {
                                            it.continuation_lines_width = 0;
                                            it.font_height = Value::NIL;
                                            it.voffset = 0;
                                            let r = self.glyph_row_mut(row);
                                            r.exact_window_width_line_p = true;
                                            r.continued_p = false;
                                            r.ends_at_zv_p = true;
                                        } else if it.at_end_of_line_p() {
                                            let r = self.glyph_row_mut(row);
                                            r.continued_p = false;
                                            r.exact_window_width_line_p = true;
                                        } else if wrap_row_used > 0
                                            && (!may_wrap || !self.char_can_wrap_before(it)?)
                                        {
                                            back_to_wrap = true;
                                            break 'only;
                                        }
                                    }
                                } else if it.bidi_p {
                                    self.record_max_min_pos(it, &mut mm);
                                }
                                if self.window_has_margins(w) {
                                    self.extend_face_to_end_of_line(it)?;
                                }
                            }
                        } else if self.glyph_row(row).glyphs[glyph_index].padding_p && !window_p {
                            unreachable!("tty frames have no glyph matrices");
                        } else if wrap_row_used > 0 {
                            back_to_wrap = true;
                        } else if matches!(
                            it.what,
                            DisplayElementType::Character
                                | DisplayElementType::Stretch
                                | DisplayElementType::Composition
                        ) && it.c == i32::from(b'\t')
                            && window_p
                        {
                            if edge_fringe == 0 {
                                let dir = it.bidi_it.paragraph_dir;
                                self.produce_special_glyphs(
                                    it,
                                    DisplayElementType::Continuation,
                                    dir,
                                    false,
                                )?;
                            }
                            it.continuation_lines_width += it.last_visible_x;
                            let last_visible_x = it.last_visible_x;
                            let r = self.glyph_row_mut(row);
                            r.ends_in_middle_of_char_p = true;
                            r.continued_p = true;
                            r.glyphs[glyph_index].pixel_width = (last_visible_x - x) as i16;
                            it.starts_in_middle_of_char_p = true;
                            if self.window_has_margins(w) {
                                self.extend_face_to_end_of_line(it)?;
                            }
                        } else {
                            if self.glyph_row(row).reversed_p {
                                let used = i32::from(self.glyph_row(row).used[TEXT_AREA]);
                                self.unproduce_glyphs(it, used - (n_glyphs_before + i));
                            }
                            self.glyph_row_mut(row).used[TEXT_AREA] = (n_glyphs_before + i) as i16;
                            it.current_x = x_before;
                            it.continuation_lines_width += x;
                            if !window_p || edge_fringe == 0 {
                                let dir = it.bidi_it.paragraph_dir;
                                self.produce_special_glyphs(
                                    it,
                                    DisplayElementType::Continuation,
                                    dir,
                                    false,
                                )?;
                            }
                            self.glyph_row_mut(row).continued_p = true;
                            self.extend_face_to_end_of_line(it)?;
                            if nglyphs > 1 && i > 0 {
                                self.glyph_row_mut(row).ends_in_middle_of_char_p = true;
                                it.starts_in_middle_of_char_p = true;
                            }
                            it.max_ascent = ascent;
                            it.max_descent = descent;
                            it.max_phys_ascent = phys_ascent;
                            it.max_phys_descent = phys_descent;
                        }
                        if back_to_wrap {
                            if self.glyph_row(row).reversed_p {
                                let used = i32::from(self.glyph_row(row).used[TEXT_AREA]);
                                self.unproduce_glyphs(it, used - wrap_row_used);
                            }
                            self.restore_it(it, prev_it.as_ref(), &mut prev_data);
                            {
                                let r = self.glyph_row_mut(row);
                                r.used[TEXT_AREA] = wrap_row_used as i16;
                                (
                                    r.ascent,
                                    r.height,
                                    r.phys_ascent,
                                    r.phys_height,
                                    r.extra_line_spacing,
                                ) = wrap_row;
                                r.continued_p = true;
                                r.ends_at_zv_p = false;
                                r.exact_window_width_line_p = false;
                            }
                            mm = wrap_mm;
                            self.extend_face_to_end_of_line(it)?;
                            self.restore_it(it, wrap_it.as_ref(), &mut wrap_data);
                            it.continuation_lines_width += wrap_x;
                        }
                        break;
                    } else if new_x > it.first_visible_x {
                        it.hpos += 1;
                        if it.bidi_p {
                            self.record_max_min_pos(it, &mut mm);
                        }
                        if x < it.first_visible_x && !reversed && !line_number_needed {
                            self.glyph_row_mut(row).x = x - it.first_visible_x;
                        }
                        if reversed
                            && new_x > it.last_visible_x
                            && !line_number_needed
                            && !(it.line_wrap == LineWrap::Truncate
                                && self.window_left_fringe_width(w) == 0)
                        {
                            self.glyph_row_mut(row).x = it.last_visible_x - new_x;
                        }
                    }
                    i += 1;
                    x = new_x;
                }
                if it.bidi_p && nglyphs == 0 {
                    self.record_max_min_pos(it, &mut mm);
                }
                self.grow_row_metrics(row, it);
                let r = self.glyph_row(row);
                if r.continued_p || r.ends_at_zv_p {
                    break;
                }
            }
            if it.at_end_of_line_p() {
                self.finish_line_at_eol(it, row)?;
                break;
            }
            self.set_iterator_to_next(it, true)?;
            if this_line_subject_to_line_prefix == 1 && !it.string_from_prefix_prop_p {
                this_line_subject_to_line_prefix = 2;
            } else if this_line_subject_to_line_prefix == 2 && !it.string_from_prefix_prop_p {
                this_line_subject_to_line_prefix = 0;
            }
            let reversed = self.glyph_row(row).reversed_p;
            let edge_fringe = self.fringe_at_row_end(w, reversed);
            let past_edge =
                if window_p && (edge_fringe != 0 || it.what == DisplayElementType::Image) {
                    it.current_x >= it.last_visible_x
                } else {
                    it.current_x > it.last_visible_x
                };
            if it.line_wrap == LineWrap::Truncate && past_edge {
                if !window_p || edge_fringe == 0 {
                    let used = i32::from(self.glyph_row(row).used[TEXT_AREA]);
                    let padding = |lisp: &Lisp, i: i32| {
                        let r = lisp.glyph_row(row);
                        r.glyphs[r.area[TEXT_AREA] + i as usize].padding_p
                    };
                    let mut i;
                    if !reversed {
                        i = used - 1;
                        while i > 0 {
                            if !padding(self, i) {
                                break;
                            }
                            i -= 1;
                        }
                    } else {
                        i = 0;
                        while i < used {
                            if !padding(self, i) {
                                break;
                            }
                            i += 1;
                        }
                        self.unproduce_glyphs(it, i + 1);
                        i = i32::from(self.glyph_row(row).used[TEXT_AREA]) - (i + 1);
                    }
                    if it.current_x > it.last_visible_x {
                        it.current_x = x_before;
                        if !window_p {
                            unreachable!("tty frames have no glyph matrices");
                        }
                        self.glyph_row_mut(row).used[TEXT_AREA] = i as i16;
                        let dir = it.bidi_it.paragraph_dir;
                        self.produce_special_glyphs(
                            it,
                            DisplayElementType::Truncation,
                            dir,
                            false,
                        )?;
                        it.hpos = hpos_before;
                    }
                } else if self.it_overflow_newline_into_fringe(it) {
                    if !self.get_next_display_element(it)? {
                        it.continuation_lines_width = 0;
                        it.font_height = Value::NIL;
                        it.voffset = 0;
                        let r = self.glyph_row_mut(row);
                        r.ends_at_zv_p = true;
                        r.exact_window_width_line_p = true;
                        break;
                    }
                    if it.at_end_of_line_p() {
                        self.glyph_row_mut(row).exact_window_width_line_p = true;
                        self.finish_line_at_eol(it, row)?;
                        break;
                    }
                    it.current_x = x_before;
                    it.hpos = hpos_before;
                }
                let margin_face_id = self.lookup_basic_face(Some(w), f, MARGIN_FACE_ID)?;
                let frame_background = self.frame(f).background_pixel as u64;
                if (self.lookup_basic_face(Some(w), f, DEFAULT_FACE_ID)? != DEFAULT_FACE_ID
                    || self.face_background(f, margin_face_id) != frame_background)
                    && self.window_has_margins(w)
                {
                    self.extend_face_to_end_of_line(it)?;
                }
                self.glyph_row_mut(row).truncated_on_right_p = true;
                it.continuation_lines_width = 0;
                self.reseat_at_next_visible_line_start(it, false)?;
                let zv_byte = self.cur().zv_byte as i64;
                let ends_at_zv_p = if it.bytepos() > 1 {
                    it.bytepos() >= zv_byte
                        && (zv_byte <= 1
                            || self.cur().text.byte_at((zv_byte - 1) as usize) != b'\n')
                } else {
                    false
                };
                self.glyph_row_mut(row).ends_at_zv_p = ends_at_zv_p;
                break;
            }
        }
        if wrap_data.is_some() {
            self.bidi_unshelve_cache(wrap_data.take(), true);
        }
        if prev_data.is_some() {
            self.bidi_unshelve_cache(prev_data.take(), true);
        }
        if it.first_visible_x != 0 && it.charpos() != self.glyph_row(row).start.pos.charpos {
            let r = self.glyph_row(row);
            let fringe = if r.reversed_p {
                self.window_right_fringe_width(w)
            } else {
                self.window_left_fringe_width(w)
            };
            let first_is_image = r.glyphs[r.area[TEXT_AREA]].kind == GlyphType::Image;
            if !window_p || (fringe == 0 && !first_is_image) {
                self.insert_left_trunc_glyphs(it)?;
            }
            self.glyph_row_mut(row).truncated_on_left_p = true;
        }
        self.glyph_row_mut(row).end = it.current;
        if !it.bidi_p {
            let r = self.glyph_row_mut(row);
            r.minpos = r.start.pos;
            r.maxpos = r.end.pos;
        } else {
            self.find_row_edges(it, mm.min_pos, mm.min_bpos, mm.max_pos, mm.max_bpos);
        }
        let displays_text = self.glyph_row(row).displays_text_p;
        if displays_text || !self.xdisp.overlay_arrow_seen {
            let overlay_arrow_string = self.overlay_arrow_at_row(it);
            if !overlay_arrow_string.is_nil() {
                if overlay_arrow_string.is_string() {
                    let left_margin_glyphs = {
                        let r = self.glyph_row(row);
                        r.area[TEXT_AREA] - r.area[LEFT_MARGIN_AREA]
                    };
                    let mut arrow =
                        self.get_overlay_arrow_glyph_row(w, overlay_arrow_string, MARGIN_FACE_ID)?;
                    let area = if left_margin_glyphs >= arrow.len() {
                        LEFT_MARGIN_AREA
                    } else {
                        arrow = self.get_overlay_arrow_glyph_row(
                            w,
                            overlay_arrow_string,
                            DEFAULT_FACE_ID,
                        )?;
                        TEXT_AREA
                    };
                    let r = self.glyph_row_mut(row);
                    let where_ = r.area[area];
                    for (k, g) in arrow.iter().enumerate() {
                        r.glyphs[where_ + k] = *g;
                    }
                    let mut p = where_ + arrow.len();
                    r.used[area] = r.used[area].max(arrow.len() as i16);
                    let mut p2 = p;
                    let end = where_ + r.used[area] as usize;
                    while p2 < end && r.glyphs[p2].padding_p {
                        p2 += 1;
                    }
                    if p2 > p {
                        while p2 < end {
                            r.glyphs[p] = r.glyphs[p2];
                            p += 1;
                            p2 += 1;
                        }
                        r.used[area] = (p2 - where_) as i16;
                    }
                } else {
                    let bitmap = overlay_arrow_string.as_fixnum().expect("fringe bitmap") as i32;
                    self.glyph_row_mut(row).overlay_arrow_bitmap = bitmap;
                }
                self.xdisp.overlay_arrow_seen = true;
            }
        }
        if self.var_true(sym::SHOW_TRAILING_WHITESPACE) {
            self.highlight_trailing_whitespace(it)?;
        }
        self.compute_line_metrics(it);
        {
            let ends_in_ellipsis_p = it.method == ItMethod::FromDisplayVector && it.ellipsis_p;
            let r = self.glyph_row_mut(row);
            r.ends_in_ellipsis_p = ends_in_ellipsis_p;
            r.left_user_fringe_bitmap = it.left_user_fringe_bitmap;
            r.left_user_fringe_face_id = it.left_user_fringe_face_id;
            r.right_user_fringe_bitmap = it.right_user_fringe_bitmap;
            r.right_user_fringe_face_id = it.right_user_fringe_face_id;
        }
        it.left_user_fringe_bitmap = 0;
        it.left_user_fringe_face_id = 0;
        it.right_user_fringe_bitmap = 0;
        it.right_user_fringe_face_id = 0;
        if self.window(w).mini
            && it.line_wrap == LineWrap::Truncate
            && window_p
            && !self.var_true(sym::CURSOR_IN_ECHO_AREA)
        {
            self.glyph_row_mut(row).redraw_fringe_bitmaps_p = true;
        }
        let cvpos = self.window(w).cursor.vpos;
        let pt = self.cur().pt as i64;
        let (start_charpos, end_charpos) = {
            let r = self.glyph_row(row);
            (r.start_charpos(), r.end_charpos())
        };
        if (cvpos < 0
            || (it.bidi_p
                && !self
                    .window_matrix(w, WhichMatrix::Desired)
                    .expect("desired matrix")
                    .rows[cvpos as usize]
                    .ends_at_zv_p))
            && pt >= start_charpos
            && pt <= end_charpos
        {
            let snapshot = self.glyph_row(row).clone();
            if self.row_for_charpos_p(&snapshot, pt)? {
                self.set_cursor_from_row(w, vpos, WhichMatrix::Desired, 0, 0, 0, 0)?;
            }
        }
        it.wrap_prefix_width = 0;
        it.current_x = 0;
        it.hpos = 0;
        it.current_y += self.glyph_row(row).height;
        if hscroll_this_line {
            it.first_visible_x = first_visible_x;
            it.last_visible_x = last_visible_x;
        }
        it.eol_pos = TextPos::new(0, 0);
        it.vpos += 1;
        if let GlyphRowRef::Window {
            window, desired, ..
        } = row
        {
            let next = GlyphRowRef::Window {
                window,
                desired,
                vpos: vpos + 1,
            };
            it.glyph_row = Some(next);
            let bottom = nrows - i32::from(self.window_wants_mode_line(w));
            if ((vpos + 1) as i32) < bottom {
                let reversed = self.glyph_row(row).reversed_p;
                self.glyph_row_mut(next).reversed_p = reversed;
            }
        }
        it.start = self.glyph_row(row).end;
        Ok(self.glyph_row(row).displays_text_p)
    }

    pub(super) fn row_for_charpos_p(&mut self, row: &GlyphRow, charpos: i64) -> LispResult<bool> {
        let mut result = true;
        if charpos == row.end.pos.charpos || charpos == row.end_charpos() {
            if row.end.string_pos.charpos >= 0 {
                if row.continued_p {
                    result = true;
                } else {
                    let glyphs = row.area_glyphs(TEXT_AREA);
                    result = false;
                    let mut g = glyphs.len() as isize - 1;
                    while g >= 0 {
                        if glyphs[g as usize].object.is_string() {
                            let prop = self.get_char_property(
                                Value::fixnum(charpos),
                                sym::DISPLAY,
                                Value::NIL,
                            )?;
                            result = !prop.is_nil()
                                && self.display_prop_string_p(prop, glyphs[g as usize].object);
                            if !result {
                                let s = glyphs[g as usize].object;
                                while g >= 0 && glyphs[g as usize].object == s {
                                    let gpos = glyphs[g as usize].charpos;
                                    if !self
                                        .get_char_property(Value::fixnum(gpos), sym::CURSOR, s)?
                                        .is_nil()
                                    {
                                        result = true;
                                        break;
                                    }
                                    g -= 1;
                                }
                            }
                            break;
                        }
                        g -= 1;
                    }
                }
            } else if row.end.dpvec_index > 0
                || row.end.overlay_string_index >= 0
                || row.ends_in_middle_of_char_p
            {
                result = !row.ends_in_ellipsis_p && row.continued_p;
            } else {
                result = row.ends_at_zv_p;
            }
        }
        Ok(result)
    }

    pub(super) fn cursor_row_p(&mut self, row: &GlyphRow) -> LispResult<bool> {
        let pt = self.cur().pt as i64;
        self.row_for_charpos_p(row, pt)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn set_cursor_from_row(
        &mut self,
        w: Value,
        vpos: usize,
        which: WhichMatrix,
        delta: i64,
        delta_bytes: i64,
        dy: i32,
        dvpos: i32,
    ) -> LispResult<bool> {
        let row = self
            .window_matrix(w, which)
            .expect("window has no glyph matrix")
            .rows[vpos]
            .clone();
        let base = row.area[TEXT_AREA] as isize;
        let used = row.used[TEXT_AREA] as isize;
        let at = |i: isize| -> &Glyph { &row.glyphs[(base + i) as usize] };
        let pw = |i: isize| i32::from(row.glyphs[(base + i) as usize].pixel_width);
        let rev = row.reversed_p;
        let mut glyph: isize = 0;
        let mut end: isize = used;
        let mut cursor: Option<isize> = None;
        let last_pos = row.start_charpos() + delta;
        let mut x = row.x;
        let pt_old = self.cur().pt as i64 - delta;
        let mut pos_before = row.start_charpos() + delta;
        let mut pos_after = row.end_charpos() + delta;
        let mut glyph_before = glyph - 1;
        let mut glyph_after = end;
        let mut glyphs_end = end;
        let mut match_with_avoid_cursor = false;
        let mut string_seen = false;
        let mut bpos_max = pos_before;
        let mut bpos_min = pos_after;
        let mut bpos_covered = 0;
        let mut string_from_text_prop = false;
        if row.mode_line_p {
            return Ok(false);
        }
        if row.displays_text_p {
            if !rev {
                while glyph < end && at(glyph).object.is_nil() && at(glyph).charpos < 0 {
                    x += pw(glyph);
                    glyph += 1;
                }
                while end > glyph && at(end - 1).object.is_nil() && at(end - 1).charpos <= 0 {
                    end -= 1;
                }
                glyph_before = glyph - 1;
                glyph_after = end;
            } else {
                end = glyph - 1;
                glyphs_end = end;
                glyph += used - 1;
                while glyph > end + 1 && at(glyph).object.is_nil() && at(glyph).charpos < 0 {
                    glyph -= 1;
                }
                if at(glyph).object.is_nil() && at(glyph).charpos < 0 {
                    glyph -= 1;
                }
                x = row.x;
                let mut g = end + 1;
                while g < glyph {
                    x += pw(g);
                    g += 1;
                }
                while end < glyph && at(end + 1).object.is_nil() && at(end + 1).charpos <= 0 {
                    end += 1;
                }
                glyph_before = glyph + 1;
                glyph_after = end;
            }
        } else if rev {
            cursor = Some(end - 1);
            x = -1;
        }
        if !rev {
            while glyph < end && !at(glyph).object.is_nil() {
                let g = *at(glyph);
                if self.is_buffer(g.object) {
                    let dpos = g.charpos - pt_old;
                    bpos_max = bpos_max.max(g.charpos);
                    bpos_min = bpos_min.min(g.charpos);
                    if !g.avoid_cursor_p {
                        if dpos == 0 {
                            match_with_avoid_cursor = false;
                            break;
                        }
                        if 0 > dpos && dpos > pos_before - pt_old {
                            pos_before = g.charpos;
                            glyph_before = glyph;
                        } else if 0 < dpos && dpos < pos_after - pt_old {
                            pos_after = g.charpos;
                            glyph_after = glyph;
                        }
                    } else if dpos == 0 {
                        match_with_avoid_cursor = true;
                    }
                } else if g.object.is_string() && !g.avoid_cursor_p {
                    let chprop =
                        self.get_char_property(Value::fixnum(g.charpos), sym::CURSOR, g.object)?;
                    if !chprop.is_nil() {
                        let prop_pos = self
                            .string_buffer_position_lim(g.object, pos_before, pos_after, false)?;
                        if prop_pos >= pos_before {
                            bpos_max = prop_pos;
                        }
                    }
                    if let Some(n) = chprop.as_fixnum() {
                        bpos_covered = bpos_max + n;
                        if bpos_max <= pt_old && bpos_covered >= pt_old {
                            cursor = Some(glyph);
                            break;
                        }
                    }
                    string_seen = true;
                }
                x += i32::from(g.pixel_width);
                glyph += 1;
            }
        } else if glyph > end {
            while !at(glyph).object.is_nil() {
                let g = *at(glyph);
                if self.is_buffer(g.object) {
                    let dpos = g.charpos - pt_old;
                    bpos_max = bpos_max.max(g.charpos);
                    bpos_min = bpos_min.min(g.charpos);
                    if !g.avoid_cursor_p {
                        if dpos == 0 {
                            match_with_avoid_cursor = false;
                            break;
                        }
                        if 0 > dpos && dpos > pos_before - pt_old {
                            pos_before = g.charpos;
                            glyph_before = glyph;
                        } else if 0 < dpos && dpos < pos_after - pt_old {
                            pos_after = g.charpos;
                            glyph_after = glyph;
                        }
                    } else if dpos == 0 {
                        match_with_avoid_cursor = true;
                    }
                } else if g.object.is_string() {
                    let chprop =
                        self.get_char_property(Value::fixnum(g.charpos), sym::CURSOR, g.object)?;
                    if !chprop.is_nil() {
                        let prop_pos = self
                            .string_buffer_position_lim(g.object, pos_before, pos_after, false)?;
                        if prop_pos >= pos_before {
                            bpos_max = prop_pos;
                        }
                    }
                    if let Some(n) = chprop.as_fixnum() {
                        bpos_covered = bpos_max + n;
                        if bpos_max <= pt_old && bpos_covered >= pt_old {
                            cursor = Some(glyph);
                            break;
                        }
                    }
                    string_seen = true;
                }
                glyph -= 1;
                if glyph == glyphs_end {
                    x -= 1;
                    break;
                }
                x -= pw(glyph);
            }
        }
        let ge = glyphs_end;
        let in_text = |g: isize| if rev { g > ge } else { g < ge };
        'compute_x: {
            if !(in_text(glyph) && self.is_buffer(at(glyph).object) && at(glyph).charpos == pt_old)
                && !(bpos_max <= pt_old && pt_old <= bpos_covered)
            {
                let empty_line_p = in_text(glyph)
                    && at(glyph).object.is_nil()
                    && at(glyph).charpos > 0
                    && !(row.continued_p || row.truncated_on_right_p);
                if row.ends_in_ellipsis_p && pos_after == last_pos {
                    if !rev {
                        let ellipsis_pos = at(glyph - 1).charpos;
                        while glyph > 0 && at(glyph - 1).charpos == ellipsis_pos {
                            glyph -= 1;
                            x -= pw(glyph);
                        }
                        x += pw(glyph);
                        glyph += 1;
                    } else {
                        let ellipsis_pos = at(glyph + 1).charpos;
                        while glyph < used - 1 && at(glyph + 1).charpos == ellipsis_pos {
                            glyph += 1;
                            x += pw(glyph);
                        }
                        x -= pw(glyph);
                        glyph -= 1;
                    }
                } else if match_with_avoid_cursor {
                    cursor = Some(glyph_after);
                    x = -1;
                } else if string_seen {
                    let incr: isize = if rev { -1 } else { 1 };
                    let mut pos = pos_before;
                    x = -1;
                    if row.ends_in_newline_from_string_p {
                        glyph_after = end;
                        pos_after = row.end_charpos() + delta;
                    }
                    let (start, stop) = if !rev {
                        (glyph_before.min(glyph_after), glyph_before.max(glyph_after))
                    } else {
                        (glyph_before.max(glyph_after), glyph_before.min(glyph_after))
                    };
                    let before_stop = |g: isize| if rev { g > stop } else { g < stop };
                    glyph = start + incr;
                    while before_stop(glyph) {
                        let g = *at(glyph);
                        if g.object.is_string() && !g.avoid_cursor_p {
                            let lim = pos_after + i64::from(pos_after == row.end_charpos() + delta);
                            string_from_text_prop = false;
                            let s = g.object;
                            let tem = self.string_buffer_position_lim(s, pos, lim, false)?;
                            if tem == 0 || pos <= tem {
                                if tem == 0
                                    || tem == pt_old
                                    || (tem - pt_old > 0 && tem < pos_after)
                                {
                                    let mut strpos = g.charpos;
                                    if tem != 0 {
                                        cursor = Some(glyph);
                                        string_from_text_prop = true;
                                    }
                                    while before_stop(glyph) && at(glyph).object == s {
                                        let gpos = at(glyph).charpos;
                                        let cprop = self.get_char_property(
                                            Value::fixnum(gpos),
                                            sym::CURSOR,
                                            s,
                                        )?;
                                        if !cprop.is_nil() {
                                            cursor = Some(glyph);
                                            break;
                                        }
                                        if tem != 0 && at(glyph).charpos < strpos {
                                            strpos = at(glyph).charpos;
                                            cursor = Some(glyph);
                                        }
                                        glyph += incr;
                                    }
                                    if tem == pt_old || (tem - pt_old > 0 && tem < pos_after) {
                                        break 'compute_x;
                                    }
                                }
                                if tem != 0 {
                                    pos = tem + 1;
                                }
                            }
                            while before_stop(glyph) && at(glyph).object == s {
                                glyph += incr;
                            }
                        } else {
                            glyph += incr;
                        }
                    }
                    if cursor.is_none()
                        && (if rev { glyph <= end } else { glyph >= end })
                        && (if rev {
                            end > glyphs_end
                        } else {
                            end < glyphs_end
                        })
                        && at(end).object.is_string()
                        && row.continued_p
                    {
                        return Ok(false);
                    }
                } else if row.truncated_on_left_p && pt_old < bpos_min {
                    cursor = Some(glyph_before);
                    x = -1;
                } else if (row.truncated_on_right_p && pt_old > bpos_max)
                    || (!empty_line_p && in_text(glyph_after))
                {
                    cursor = Some(glyph_after);
                    x = -1;
                }
            }
        }
        if let Some(c) = cursor {
            glyph = c;
        } else {
            let edge = if rev { used - 1 } else { 0 };
            if glyph == glyphs_end && pos_before == pos_after && at(edge).object.is_string() {
                glyph = edge;
            }
        }
        if x < 0 {
            x = row.x;
            let mut g = 0;
            while g < glyph {
                if g >= used {
                    std::process::abort();
                }
                x += pw(g);
                g += 1;
            }
        }
        let cur = self.window(w).cursor;
        if cur.vpos >= 0 && cur.vpos as usize != vpos {
            let crow = self
                .window_matrix(w, which)
                .expect("window has no glyph matrix")
                .rows[cur.vpos as usize]
                .clone();
            if crow.start_charpos() <= pt_old
                && pt_old <= crow.end_charpos()
                && self.cursor_row_p(&crow)?
            {
                if !in_text(glyph) {
                    return Ok(false);
                }
                let gl = *at(glyph);
                let keep_previous =
                    cur.hpos >= 0 && cur.hpos < i64::from(crow.used[TEXT_AREA]) && {
                        let g1 = crow.glyphs[crow.area[TEXT_AREA] + cur.hpos as usize];
                        (self.is_buffer(g1.object)
                            && (g1.charpos == pt_old
                                || (self.is_buffer(gl.object)
                                    && (g1.charpos - pt_old).abs() < (gl.charpos - pt_old).abs())))
                            || (g1.object.is_string()
                                && (!self
                                    .get_char_property(
                                        Value::fixnum(g1.charpos),
                                        sym::CURSOR,
                                        g1.object,
                                    )?
                                    .is_nil()
                                    || (g1.object == gl.object && string_from_text_prop)
                                    || (gl.object.is_nil() && gl.charpos != pt_old)))
                    };
                if keep_previous {
                    return Ok(false);
                }
                if !((self.is_buffer(gl.object) && gl.charpos == pt_old)
                    || (!row.continued_p
                        && gl.object.is_nil()
                        && gl.charpos == 0
                        && pt_old == row.end_charpos() - 1))
                    && crow.end_charpos() - crow.start_charpos()
                        < row.end_charpos() - row.start_charpos()
                {
                    return Ok(false);
                }
            }
        }
        let cursor_vpos = vpos as i64 + i64::from(dvpos);
        let cursor_y = i64::from(row.y + dy);
        {
            let win = self.window_mut(w);
            win.cursor.hpos = glyph as i64;
            win.cursor.x = i64::from(x);
            win.cursor.vpos = cursor_vpos;
            win.cursor.y = cursor_y;
        }
        if w == self.windows.selected_window {
            if !row.continued_p && !row.continuation_line_p() && row.x == 0 {
                let contents = self.window(w).contents;
                let (z, z_byte) = (self.cur().z() as i64, self.cur().z_byte() as i64);
                let xd = &mut self.xdisp;
                xd.this_line_buffer = contents;
                xd.this_line_start_pos = TextPos::new(
                    row.start_charpos() + delta,
                    row.minpos.bytepos + delta_bytes,
                );
                xd.this_line_end_pos = TextPos::new(
                    z - (row.end_charpos() + delta),
                    z_byte - (row.maxpos.bytepos + delta_bytes),
                );
                xd.this_line_y = cursor_y as i32;
                xd.this_line_pixel_height = row.height;
                xd.this_line_vpos = cursor_vpos as i32;
                xd.this_line_start_x = row.x;
            } else {
                self.xdisp.this_line_start_pos.charpos = 0;
            }
        }
        Ok(true)
    }

    pub fn try_window(&mut self, window: Value, pos: TextPos, flags: i32) -> LispResult<i32> {
        self.with_gc_deferred(|lisp| lisp.try_window_1(window, pos, flags))
    }

    fn try_window_1(&mut self, w: Value, pos: TextPos, flags: i32) -> LispResult<i32> {
        let f = self.window(w).frame;
        let cursor_vpos = self.window(w).cursor.vpos;
        let start = self.window(w).start;
        let buffer = self.current_buffer;
        self.set_marker_position(start, pos.charpos as usize, buffer);
        self.window_mut(w).cursor.vpos = -1;
        self.xdisp.overlay_arrow_seen = false;
        let mut it = It::default();
        self.start_display(&mut it, w, pos)?;
        let first = it.glyph_row.expect("window has no glyph matrix");
        self.glyph_row_mut(first).reversed_p = false;
        let mut last_text_row: Option<usize> = None;
        while it.current_y < it.last_visible_y {
            let last_row_scale = self.window(w).nrows_scale_factor;
            let last_col_scale = self.window(w).ncols_scale_factor;
            if self.display_line(&mut it, cursor_vpos)? {
                last_text_row = Some(Self::row_vpos(&it) - 1);
            }
            if self.frame(f).fonts_changed
                && !((flags & TRY_WINDOW_IGNORE_FONTS_CHANGE) != 0
                    && last_row_scale == self.window(w).nrows_scale_factor
                    && last_col_scale == self.window(w).ncols_scale_factor)
            {
                return Ok(0);
            }
        }
        let it_charpos = it.charpos();
        if (flags & TRY_WINDOW_CHECK_MARGINS) != 0
            && self.window(w).vscroll == 0
            && !self.window(w).mini
        {
            let mut top_scroll_margin = self.window_scroll_margin(w, MarginUnit::InPixels);
            let bot_scroll_margin = top_scroll_margin;
            if self.window_wants_header_line(w) {
                top_scroll_margin += self.current_header_line_height(w) as i32;
            }
            if self.window_wants_tab_line(w) {
                top_scroll_margin += self.current_tab_line_height(w) as i32;
            }
            self.start_display(&mut it, w, pos)?;
            let cursor_y = self.window(w).cursor.y;
            let begv = self.cur().begv as i64;
            let partial = self.partial_line_height(&it)?;
            if (cursor_y >= 0 && cursor_y < i64::from(top_scroll_margin) && pos.charpos > begv)
                || cursor_y > i64::from(it.last_visible_y - partial - bot_scroll_margin - 1)
            {
                self.window_mut(w).cursor.vpos = -1;
                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                    clear_glyph_matrix(m);
                }
                return Ok(-1);
            }
        }
        let (z, z_byte) = (self.cur().z() as i64, self.cur().z_byte() as i64);
        let (zv, zv_byte) = (self.cur().zv as i64, self.cur().zv_byte as i64);
        if self.window(w).window_end_pos <= 0 && z != it_charpos {
            self.window_mut(w).flags.update_mode_line = true;
        }
        if let Some(vpos) = last_text_row {
            self.adjust_window_ends(w, vpos, false);
        } else {
            let win = self.window_mut(w);
            win.window_end_bytepos = z_byte - zv_byte;
            win.window_end_pos = z - zv;
            win.window_end_vpos = 0;
        }
        self.window_mut(w).flags.window_end_valid = false;
        Ok(1)
    }

    pub(crate) fn adjust_window_ends(&mut self, w: Value, vpos: usize, current: bool) {
        let which = if current {
            WhichMatrix::Current
        } else {
            WhichMatrix::Desired
        };
        let (end_charpos, end_bytepos) = {
            let r = &self
                .window_matrix(w, which)
                .expect("window has no glyph matrix")
                .rows[vpos];
            (r.maxpos.charpos, r.maxpos.bytepos)
        };
        let (z, z_byte) = (self.cur().z() as i64, self.cur().z_byte() as i64);
        let win = self.window_mut(w);
        win.window_end_pos = z - end_charpos;
        win.window_end_bytepos = z_byte - end_bytepos;
        win.window_end_vpos = vpos as i64;
    }

    pub fn partial_line_height(&mut self, it_origin: &It) -> LispResult<i32> {
        let contents = self.window(it_origin.window).contents;
        if self.buffer(contents).long_line_optimizations_p
            && it_origin.line_wrap == LineWrap::Truncate
        {
            return Ok(0);
        }
        let mut it_data: ShelvedBidiCache = None;
        let mut it = self.save_it(it_origin, &mut it_data);
        let zv = self.cur().zv as i64;
        let to_y = it.last_visible_y;
        self.move_it_to(&mut it, zv, -1, to_y, -1, MOVE_TO_POS | MOVE_TO_Y)?;
        let partial_height = if it.what == DisplayElementType::Eob {
            let vis_height = it.last_visible_y - it.current_y;
            let height = it.ascent + it.descent;
            if vis_height < height { vis_height } else { 0 }
        } else {
            let last_line_y = it.current_y;
            self.move_it_by_lines(&mut it, 1)?;
            if it.current_y > it.last_visible_y {
                it.last_visible_y - last_line_y
            } else {
                0
            }
        };
        self.restore_it(&mut it, None, &mut it_data);
        Ok(partial_height)
    }
}
