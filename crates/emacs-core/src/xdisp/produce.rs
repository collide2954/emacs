use crate::bidi::BidiDir;
use crate::character;
use crate::dispextern::{
    DisplayElementType, GlyphRowArea, GlyphRowRef, GlyphlessMethod, It, ItMethod, LineWrap, TextPos,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::xfaces::DEFAULT_FACE_ID;

const NOT_AT_EOL: u32 = b'<' as u32;
const NOT_AT_BOL: u32 = b'>' as u32;
const LINE_BREAKABLE: u32 = b'|' as u32;
const DISP_TRUNC_GLYPH_SLOT: usize = 0;
const DISP_CONTINUE_GLYPH_SLOT: usize = 1;

impl Lisp {
    pub(super) fn glyph_row_ref(&self, _row: GlyphRowRef) -> &crate::dispextern::GlyphRow {
        unreachable!("glyph rows need glyph matrices")
    }

    fn it_displaying_whitespace(&self, it: &It) -> bool {
        if it.what == DisplayElementType::Character && (it.c == 32 || it.c == 9) {
            return true;
        }
        if it.string.is_string() {
            let b = self.string_bytes(it.string)[it.string_bytepos() as usize];
            return b == b' ' || b == b'\t';
        }
        if let Some(s) = &it.s {
            let b = s[it.bytepos() as usize];
            return b == b' ' || b == b'\t';
        }
        if it.bytepos() < self.cur().zv_byte as i64 {
            let b = self.cur().text.byte_at(it.bytepos() as usize);
            return b == b' ' || b == b'\t';
        }
        false
    }

    fn it_char_has_category(&self, it: &It, cat: u32) -> bool {
        let ch: u32 = if it.what == DisplayElementType::Character {
            it.c as u32
        } else if it.string.is_string() {
            u32::from(self.string_bytes(it.string)[it.string_bytepos() as usize])
        } else if let Some(s) = &it.s {
            u32::from(s[it.bytepos() as usize])
        } else if it.bytepos() < self.cur().zv_byte as i64 {
            u32::from(self.cur().text.byte_at(it.bytepos() as usize))
        } else {
            0
        };
        ch != 0 && self.regex_env(None).char_has_category(ch, cat)
    }

    fn glyph_row_reversed_p(&self, it: &It) -> bool {
        match it.glyph_row {
            Some(row) => self.glyph_row_ref(row).reversed_p,
            None => false,
        }
    }

    pub(super) fn char_can_wrap_before(&self, it: &It) -> LispResult<bool> {
        if !self.var_true(sym::WORD_WRAP_BY_CATEGORY) {
            return Ok(!self.it_displaying_whitespace(it));
        }
        let not_at_bol = if self.glyph_row_reversed_p(it) {
            NOT_AT_EOL
        } else {
            NOT_AT_BOL
        };
        Ok(!self.it_displaying_whitespace(it) && !self.it_char_has_category(it, not_at_bol))
    }

    pub(super) fn char_can_wrap_after(&self, it: &It) -> LispResult<bool> {
        if !self.var_true(sym::WORD_WRAP_BY_CATEGORY) {
            return Ok(self.it_displaying_whitespace(it));
        }
        let not_at_eol = if self.glyph_row_reversed_p(it) {
            NOT_AT_BOL
        } else {
            NOT_AT_EOL
        };
        Ok(self.it_displaying_whitespace(it)
            || (self.it_char_has_category(it, LINE_BREAKABLE)
                && !self.it_char_has_category(it, not_at_eol)))
    }

    pub fn line_bottom_y(&mut self, it: &It) -> LispResult<i32> {
        let mut it = it.clone();
        let mut line_height = it.max_ascent + it.max_descent;
        let line_top_y = it.current_y;
        if line_height == 0 {
            if self.xdisp.last_height != 0 {
                line_height = self.xdisp.last_height;
            } else if it.charpos() < self.cur().zv as i64 {
                self.move_it_by_lines(&mut it, 1)?;
                line_height = if it.max_ascent != 0 || it.max_descent != 0 {
                    it.max_ascent + it.max_descent
                } else {
                    self.xdisp.last_height
                };
            } else {
                let row = it.glyph_row.take();
                it.what = DisplayElementType::Character;
                it.c = 32;
                it.len = 1;
                self.produce_glyphs_macro(&mut it)?;
                line_height = it.ascent + it.descent;
                it.glyph_row = row;
            }
        }
        Ok(line_top_y + line_height)
    }

    pub fn default_line_pixel_height(&self, w: crate::value::Value) -> i32 {
        let f = self.window(w).frame;
        let mut height = self.frame(f).geometry.line_height as i32;
        let contents = self.window(w).contents;
        if self.frame(f).output_method != crate::terminal::OutputMethod::Initial
            && self.is_buffer(contents)
        {
            let mut val = self.buffer_local_value(sym::LINE_SPACING, contents);
            if val.is_nil() {
                val = self.default_value_unchecked(sym::LINE_SPACING);
            }
            if !val.is_nil() {
                if let Some(n) = val
                    .as_fixnum()
                    .filter(|&n| (0..=i64::from(i32::MAX)).contains(&n))
                {
                    height += n as i32;
                } else if val.is_float() {
                    let addon = (self.heap.float_value(val) * f64::from(height) + 0.5) as i32;
                    if addon >= 0 {
                        height += addon;
                    }
                }
            } else {
                height += self.frame(f).geometry.extra_line_spacing as i32;
            }
        }
        height
    }

    fn reset_box_start_end_flags(it: &mut It) {
        if it.area == GlyphRowArea::Text
            && !(it.what == DisplayElementType::Image && it.image_id < 0)
        {
            if it.face_box_p {
                it.start_of_box_run_p = false;
            }
            it.end_of_box_run_p = false;
        }
    }

    pub fn produce_glyphs_macro(&mut self, it: &mut It) -> LispResult<()> {
        if let Some(row) = it.glyph_row
            && it.bidi_p
        {
            let reversed = it.bidi_it.paragraph_dir == BidiDir::R2L;
            self.set_glyph_row_reversed_p(row, reversed);
        }
        if let Some(produce) = self.frame_rif_produce_glyphs(it.f) {
            produce(self, it)?;
        } else {
            self.produce_glyphs_tty(it)?;
        }
        if it.glyph_row.is_some() {
            self.xdisp.inhibit_free_realized_faces = true;
        }
        Self::reset_box_start_end_flags(it);
        Ok(())
    }

    fn set_glyph_row_reversed_p(&mut self, _row: GlyphRowRef, _reversed: bool) {
        unreachable!("glyph rows need glyph matrices")
    }

    fn frame_rif_produce_glyphs(
        &self,
        f: Value,
    ) -> Option<fn(&mut Lisp, &mut It) -> LispResult<()>> {
        if self.frame_window_p(f) {
            return Some(Lisp::gui_produce_glyphs);
        }
        None
    }

    fn frame_res_x_or_y(&self, f: Value, width_p: bool) -> f64 {
        if !self.frame_window_p(f) {
            return 0.0;
        }
        let info = self.display_info(self.frame(f).terminal);
        if width_p { info.resx } else { info.resy }
    }

    pub(super) fn take_vertical_position_into_account(it: &mut It) {
        if it.voffset != 0 {
            if it.voffset < 0 {
                it.ascent -= i32::from(it.voffset);
            } else {
                it.descent += i32::from(it.voffset);
            }
        }
    }

    fn numval(&self, x: Value) -> f64 {
        if self.is_number(x) {
            self.xfloatint(x)
        } else {
            -1.0
        }
    }

    pub(super) fn calc_pixel_width_or_height(
        &mut self,
        it: &It,
        prop: Value,
        font: Value,
        width_p: bool,
        mut align_to: Option<&mut i32>,
        mut lnum_added: Option<&mut bool>,
    ) -> LispResult<Option<f64>> {
        let lnum_pixel_width = if it.line_number_produced_p {
            it.lnum_pixel_width
        } else {
            0
        };
        let lnum_once = |lnum_added: &mut Option<&mut bool>| match lnum_added {
            Some(flag) if !**flag => {
                **flag = true;
                lnum_pixel_width
            }
            _ => 0,
        };
        if prop.is_nil() {
            return Ok(Some(0.0));
        }
        let w = it.window;
        let mut prop = prop;
        if prop.is_symbol() {
            let name = self.string_bytes(self.symbol_name(prop)).to_vec();
            if self.string_chars(self.symbol_name(prop)) == 2 {
                let pixels = match &name[..] {
                    [b'i', b'n', ..] => 1.0,
                    [b'm', b'm', ..] => 25.4,
                    [b'c', b'm', ..] => 2.54,
                    _ => 0.0,
                };
                if pixels > 0.0 {
                    let ppi = self.frame_res_x_or_y(it.f, width_p);
                    if ppi > 0.0 {
                        return Ok(Some(ppi / pixels));
                    }
                    return Ok(None);
                }
            }
            if prop == sym::HEIGHT {
                return Ok(Some(if font.is_nil() {
                    self.frame(it.f).geometry.line_height as f64
                } else {
                    f64::from(self.normal_char_height(font, -1))
                }));
            }
            if prop == sym::WIDTH {
                return Ok(Some(if font.is_nil() {
                    self.frame(it.f).geometry.column_width as f64
                } else {
                    let fo = self.font_object(font);
                    f64::from(if fo.average_width != 0 {
                        fo.average_width
                    } else {
                        fo.space_width
                    })
                }));
            }
            if prop == sym::TEXT {
                return Ok(Some(if width_p {
                    (self.window_box_width(w, GlyphRowArea::Text) - i64::from(lnum_pixel_width))
                        as f64
                } else {
                    (self.window(w).pixel_height
                        - self.window_bottom_divider_width(w)
                        - self.window_scroll_bar_area_height(w)
                        - self.window_mode_line_height(w)) as f64
                }));
            }
            if let Some(align) = align_to.as_deref_mut()
                && *align < 0
            {
                let left_text = self.window_box_left_offset(w, GlyphRowArea::Text) as i32;
                let anchor = if prop == sym::LEFT {
                    Some(left_text + lnum_once(&mut lnum_added))
                } else if prop == sym::RIGHT {
                    Some(self.window_box_right_offset(w, GlyphRowArea::Text) as i32)
                } else if prop == sym::CENTER {
                    Some(
                        left_text
                            + (lnum_once(&mut lnum_added)
                                + self.window_box_width(w, GlyphRowArea::Text) as i32)
                                / 2,
                    )
                } else if prop == sym::LEFT_FRINGE {
                    Some(if self.window_has_fringes_outside_margins(w) {
                        self.window_left_scroll_bar_area_width(w) as i32
                    } else {
                        self.window_box_right_offset(w, GlyphRowArea::LeftMargin) as i32
                    })
                } else if prop == sym::RIGHT_FRINGE {
                    Some(if self.window_has_fringes_outside_margins(w) {
                        self.window_box_right_offset(w, GlyphRowArea::RightMargin) as i32
                    } else {
                        self.window_box_right_offset(w, GlyphRowArea::Text) as i32
                    })
                } else if prop == sym::LEFT_MARGIN {
                    Some(self.window_box_left_offset(w, GlyphRowArea::LeftMargin) as i32)
                } else if prop == sym::RIGHT_MARGIN {
                    Some(self.window_box_left_offset(w, GlyphRowArea::RightMargin) as i32)
                } else if prop == sym::SCROLL_BAR {
                    Some(if self.window_has_vertical_scroll_bar_on_left(w) {
                        0
                    } else {
                        (self.window_box_right_offset(w, GlyphRowArea::RightMargin)
                            + if self.window_has_fringes_outside_margins(w) {
                                self.window_right_fringe_width(w)
                            } else {
                                0
                            }) as i32
                    })
                } else {
                    None
                };
                if let Some(anchor) = anchor {
                    *align_to.as_deref_mut().expect("align_to") = anchor;
                    return Ok(Some(0.0));
                }
            } else {
                let width = if prop == sym::LEFT_FRINGE {
                    Some(self.window_left_fringe_width(w))
                } else if prop == sym::RIGHT_FRINGE {
                    Some(self.window_right_fringe_width(w))
                } else if prop == sym::LEFT_MARGIN {
                    Some(self.window_left_margin_width(w))
                } else if prop == sym::RIGHT_MARGIN {
                    Some(self.window_right_margin_width(w))
                } else if prop == sym::SCROLL_BAR {
                    Some(self.window_scroll_bar_area_width(w))
                } else {
                    None
                };
                if let Some(width) = width {
                    return Ok(Some(width as f64));
                }
            }
            let contents = self.window(w).contents;
            prop = self.buffer_local_value(prop, contents);
            if prop == sym::UNBOUND {
                prop = Value::NIL;
            }
        }
        if self.is_number(prop) {
            let base_unit = if width_p {
                self.frame(it.f).geometry.column_width
            } else {
                self.frame(it.f).geometry.line_height
            } as f64;
            if width_p && align_to.as_deref().is_some_and(|a| *a < 0) {
                let offset = lnum_once(&mut lnum_added);
                return Ok(Some(self.xfloatint(prop) * base_unit + f64::from(offset)));
            }
            return Ok(Some(self.xfloatint(prop) * base_unit));
        }
        if prop.is_cons() {
            let mut car = self.xcar(prop);
            let mut cdr = self.xcdr(prop);
            if car.is_symbol() {
                if self.frame_window_p(it.f) && car == sym::IMAGE {
                    unreachable!("image sizes need image.c's lookup_image");
                }
                if car == sym::PLUS || car == sym::MINUS {
                    let mut first = true;
                    let mut pixels = 0.0;
                    while cdr.is_cons() {
                        let item = self.xcar(cdr);
                        let Some(px) = self.calc_pixel_width_or_height(
                            it,
                            item,
                            font,
                            width_p,
                            align_to.as_deref_mut(),
                            lnum_added.as_deref_mut(),
                        )?
                        else {
                            return Ok(None);
                        };
                        if first {
                            pixels = if car == sym::PLUS { px } else { -px };
                            first = false;
                        } else {
                            pixels += px;
                        }
                        cdr = self.xcdr(cdr);
                    }
                    if car == sym::MINUS {
                        pixels = -pixels;
                    }
                    return Ok(Some(pixels));
                }
                let contents = self.window(w).contents;
                car = self.buffer_local_value(car, contents);
                if car == sym::UNBOUND {
                    car = Value::NIL;
                }
            }
            if self.is_number(car) {
                let offset = if width_p && align_to.as_deref().is_some_and(|a| *a < 0) {
                    lnum_once(&mut lnum_added)
                } else {
                    0
                };
                let pixels = self.xfloatint(car);
                if cdr.is_nil() {
                    return Ok(Some(pixels + f64::from(offset)));
                }
                return Ok(self
                    .calc_pixel_width_or_height(it, cdr, font, width_p, align_to, lnum_added)?
                    .map(|fact| pixels * fact + f64::from(offset)));
            }
            return Ok(None);
        }
        Ok(None)
    }

    pub(super) fn produce_stretch_glyph(&mut self, it: &mut It) -> LispResult<()> {
        let mut width: i32;
        let mut height: i32;
        let mut align_to = -1;
        let mut zero_width_ok_p = false;
        let mut font = Value::NIL;
        let mut ascent = 0;
        let mut zero_height_ok_p = false;
        let window_p = self.frame_window_p(it.f);
        if window_p {
            let face_font = self
                .face_from_id(it.f, it.face_id as usize)
                .expect("face is not realized")
                .font;
            font = if face_font.is_nil() {
                self.frame(it.f).font
            } else {
                face_font
            };
            self.prepare_face_for_display(it.f, it.face_id as usize);
        }
        let plist = self.xcdr(it.object);
        let mut lnum_added_ignored = false;
        let align_prop = self.plist_get(plist, sym::K_ALIGN_TO);
        let prop = self.plist_get(plist, sym::K_WIDTH);
        let absolute = if prop.is_nil() {
            None
        } else {
            self.calc_pixel_width_or_height(it, prop, font, true, None, None)?
        };
        if let Some(tem) = absolute {
            zero_width_ok_p = true;
            width = tem as i32;
        } else if self.numval(self.plist_get(plist, sym::K_RELATIVE_WIDTH)) > 0.0 {
            let factor = self.numval(self.plist_get(plist, sym::K_RELATIVE_WIDTH));
            let object = if it.sp > 0 {
                it.stack[(it.sp - 1) as usize].string
            } else {
                it.string
            };
            let multibyte_p = if object.is_string() {
                self.string_multibyte(object)
            } else {
                it.multibyte_p
            };
            let bytes: Vec<u8> = if object.is_string() {
                self.string_bytes(object)[it.string_bytepos() as usize..].to_vec()
            } else {
                let text = &self.cur().text;
                let start = it.bytepos() as usize;
                let end = (start + crate::character::MAX_MULTIBYTE_LENGTH).min(self.cur().z_byte());
                text.slice(start - 1, end - 1)
            };
            let mut it2 = it.clone();
            if multibyte_p {
                let (c, len) = crate::character::string_char(&bytes);
                it2.c = c as i32;
                it2.char_to_display = c as i32;
                it2.len = len as i32;
                if window_p && !character::ascii_char_p(c) {
                    let pos = it.charpos();
                    let obj = if object.is_string() {
                        object
                    } else {
                        Value::NIL
                    };
                    it2.face_id =
                        self.face_for_char(it.f, it.face_id as usize, c, pos, obj)? as i32;
                }
            } else {
                it2.c = i32::from(bytes[0]);
                it2.char_to_display = it2.c;
                it2.len = 1;
                if !character::ascii_char_p(it2.c as u32) {
                    it2.char_to_display = character::byte8_to_char(it2.c as u8) as i32;
                }
            }
            it2.glyph_row = None;
            it2.what = DisplayElementType::Character;
            self.produce_glyphs_macro(&mut it2)?;
            width = (factor * f64::from(it2.pixel_width)) as i32;
        } else if !align_prop.is_nil()
            && let Some(tem) = self.calc_pixel_width_or_height(
                it,
                align_prop,
                font,
                true,
                Some(&mut align_to),
                Some(&mut lnum_added_ignored),
            )?
        {
            let mut x = it.current_x
                + if it.align_visually_p {
                    0
                } else {
                    it.continuation_lines_width
                };
            let x0 = x;
            let lnum = !self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS).is_nil()
                && it.line_number_produced_p;
            if lnum {
                x -= it.lnum_pixel_width;
                if x + it.stretch_adjust >= it.first_visible_x {
                    x += it.stretch_adjust;
                }
            }
            let mode_line_p = it
                .glyph_row
                .is_some_and(|row| self.glyph_row_ref(row).mode_line_p);
            let left = self.window_box_left_offset(it.window, GlyphRowArea::Text) as i32;
            if !mode_line_p {
                align_to = if align_to < 0 { 0 } else { align_to - left };
            } else if align_to < 0 {
                align_to = left;
            }
            width = (tem as i32 + align_to - x).max(0);
            let mut next_x = x + width;
            if lnum {
                if x < it.first_visible_x {
                    next_x -= it.first_visible_x - x;
                    it.stretch_adjust = it.first_visible_x - x;
                } else {
                    next_x -= it.stretch_adjust;
                }
            }
            width = next_x - x0;
            zero_width_ok_p = true;
        } else {
            width = self.frame(it.f).geometry.column_width as i32;
        }
        if width <= 0 && (width < 0 || !zero_width_ok_p) {
            width = 1;
        }
        if window_p {
            let default_height = if font.is_nil() {
                self.frame(it.f).geometry.line_height as i32
            } else {
                self.normal_char_height(font, i32::from(b' '))
            };
            let prop = self.plist_get(plist, sym::K_HEIGHT);
            let specified = if prop.is_nil() {
                None
            } else {
                self.calc_pixel_width_or_height(it, prop, font, false, None, None)?
            };
            if let Some(tem) = specified {
                height = tem as i32;
                zero_height_ok_p = true;
            } else if self.numval(self.plist_get(plist, sym::K_RELATIVE_HEIGHT)) > 0.0 {
                let factor = self.numval(self.plist_get(plist, sym::K_RELATIVE_HEIGHT));
                height = (f64::from(default_height) * factor) as i32;
            } else {
                height = default_height;
            }
            if height <= 0 && (height < 0 || !zero_height_ok_p) {
                height = 1;
            }
            let prop = self.plist_get(plist, sym::K_ASCENT);
            let numval = self.numval(prop);
            if numval > 0.0 && numval <= 100.0 {
                ascent = (f64::from(height) * numval / 100.0) as i32;
            } else if let Some(tem) = if prop.is_nil() {
                None
            } else {
                self.calc_pixel_width_or_height(it, prop, font, false, None, None)?
            } {
                ascent = (tem as i32).max(0).min(height);
            } else {
                let fo = self.font_object(font);
                ascent = (height * (fo.ascent)) / fo.height;
            }
        } else {
            height = 1;
        }
        if width > 0
            && it.area == GlyphRowArea::Text
            && it.line_wrap != LineWrap::Truncate
            && it.current_x + width > it.last_visible_x
        {
            width = it.last_visible_x - it.current_x;
            width -= i32::from(window_p);
        }
        if width > 0 && height > 0 && it.glyph_row.is_some() {
            unreachable!("appending stretch glyphs needs glyph rows");
        }
        it.pixel_width = width;
        if window_p {
            it.ascent = ascent;
            it.phys_ascent = ascent;
            it.descent = height - it.ascent;
            it.phys_descent = it.descent;
            it.nglyphs = i32::from(width > 0 && height > 0);
            Self::take_vertical_position_into_account(it);
        } else {
            it.nglyphs = width;
        }
        Ok(())
    }

    fn spec_glyph_lookup_face(&mut self, w: Value, lface_id: i32) -> LispResult<i32> {
        if lface_id > 0 {
            return Ok(self.merge_faces(w, sym::T, i64::from(lface_id), DEFAULT_FACE_ID)? as i32);
        }
        Ok(lface_id)
    }

    fn append_glyph_tty(&mut self, _it: &mut It) {
        unreachable!("appending glyphs needs glyph rows")
    }

    pub fn produce_glyphs_tty(&mut self, it: &mut It) -> LispResult<()> {
        match it.what {
            DisplayElementType::Stretch => self.produce_stretch_glyph(it)?,
            DisplayElementType::Composition => self.produce_composite_glyph_tty(it)?,
            DisplayElementType::Glyphless => self.produce_glyphless_glyph_tty(it, Value::NIL)?,
            _ => {
                let c = it.char_to_display;
                if (0o40..0o177).contains(&c) {
                    it.pixel_width = 1;
                    it.nglyphs = 1;
                    if it.glyph_row.is_some() {
                        self.append_glyph_tty(it);
                    }
                } else if c == 10 {
                    it.pixel_width = 0;
                    it.nglyphs = 0;
                } else if c == 9 {
                    let mut absolute_x = it.current_x
                        + if it.string_from_prefix_prop_p {
                            0
                        } else if it.continuation_lines_width != 0 {
                            it.continuation_lines_width - it.wrap_prefix_width
                        } else {
                            0
                        };
                    let x0 = absolute_x;
                    let lnum = !self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS).is_nil()
                        && it.line_number_produced_p;
                    if lnum {
                        absolute_x -= it.lnum_pixel_width;
                    }
                    let tab_width = i32::from(it.tab_width);
                    let mut next_tab_x = ((1 + absolute_x + tab_width - 1) / tab_width) * tab_width;
                    if lnum {
                        next_tab_x += it.lnum_pixel_width;
                    }
                    let nspaces = next_tab_x - x0;
                    if it.glyph_row.is_some() {
                        it.char_to_display = 32;
                        it.pixel_width = 1;
                        it.len = 1;
                        for _ in 0..nspaces {
                            self.append_glyph_tty(it);
                        }
                    }
                    it.pixel_width = nspaces;
                    it.nglyphs = nspaces;
                } else if character::char_byte8_p(c as u32) {
                    it.pixel_width = 1;
                    it.nglyphs = 1;
                    if it.glyph_row.is_some() {
                        self.append_glyph_tty(it);
                    }
                } else if self.tty_char_displayable(it.f, c)? {
                    it.pixel_width = self.character_width(i64::from(c)) as i32;
                    it.nglyphs = it.pixel_width;
                    if it.glyph_row.is_some() {
                        self.append_glyph_tty(it);
                    }
                } else {
                    let acronym = self.lookup_glyphless_char_display(-1, it);
                    self.produce_glyphless_glyph_tty(it, acronym)?;
                }
            }
        }
        if it.area == GlyphRowArea::Text {
            it.current_x += it.pixel_width;
            if it.continuation_lines_width != 0 && it.string_from_prefix_prop_p {
                it.wrap_prefix_width = it.current_x;
            }
        }
        it.ascent = 0;
        it.max_ascent = 0;
        it.phys_ascent = 0;
        it.max_phys_ascent = 0;
        it.descent = 1;
        it.max_descent = 1;
        it.phys_descent = 1;
        it.max_phys_descent = 1;
        Ok(())
    }

    fn tty_char_displayable(&mut self, f: Value, c: i32) -> LispResult<bool> {
        let terminal = self.frame(f).terminal;
        let charset_list = self.terminal(terminal).charset_list;
        if self.char_charset(c, charset_list)?.is_none() {
            return Ok(false);
        }
        let char_glyph = self.terminal_glyph_code(terminal, c as u32);
        Ok(char_glyph.is_nil() || matches!(char_glyph.as_fixnum(), Some(n) if n >= 0))
    }

    fn composite_glyph_is_emoji_sequence(&mut self, it: &It) -> LispResult<bool> {
        if it.cmp_it.ch < 0 {
            return Ok(false);
        }
        let gstring = self.composition_gstring_from_id(it.cmp_it.id as usize);
        let len = self.lgstring_char_len(gstring);
        let header = self.lgstring_header(gstring);
        let lgstring_char =
            |lisp: &Lisp, i: usize| lisp.aref_items(header)[i + 1].as_fixnum().unwrap_or(0);
        let first = lgstring_char(self, 0);
        let second = if len < 2 { 0 } else { lgstring_char(self, 1) };
        if (0x1F1E6..=0x1F1FF).contains(&first) && (0x1F1E6..=0x1F1FF).contains(&second) {
            return Ok(true);
        }
        let eligible = self.intern("auto-composition-emoji-tty-eligible-codepoints");
        let eligible = self.find_symbol_value(eligible);
        if crate::fns::f_memq(self, &[Value::fixnum(first), eligible])?.is_nil() {
            return Ok(false);
        }
        if second == 0xFE0F || (0x1F3FB..=0x1F3FF).contains(&second) {
            return Ok(true);
        }
        let third = if len < 3 { 0 } else { lgstring_char(self, 2) };
        let tails = self.intern("auto-composition-emoji-unqualified-tail-initials");
        let tails = self.find_symbol_value(tails);
        Ok(second == 0x200D && !crate::fns::f_memq(self, &[Value::fixnum(third), tails])?.is_nil())
    }

    fn produce_composite_glyph_tty(&mut self, it: &mut It) -> LispResult<()> {
        let first_char = if it.cmp_it.ch < 0 {
            it.pixel_width = self.compositions.table[it.cmp_it.id as usize].width;
            it.c
        } else {
            let gstring = self.composition_gstring_from_id(it.cmp_it.id as usize);
            it.pixel_width = self.composition_gstring_width(
                gstring,
                it.cmp_it.from as usize,
                it.cmp_it.to as usize,
                None,
            );
            let glyph = self.lgstring_glyph(gstring, it.cmp_it.from as usize);
            self.aref_items(glyph)[crate::composite::LGLYPH_IX_CHAR]
                .as_fixnum()
                .unwrap_or(0) as i32
        };
        let force_wide = self.intern("tty-display-emoji-force-wide");
        let padding = (it.pixel_width >= 2 && self.character_width(i64::from(first_char)) >= 2)
            || (self.var_true(force_wide) && self.composite_glyph_is_emoji_sequence(it)?);
        it.nglyphs = 1 + i32::from(padding);
        if it.glyph_row.is_some() {
            unreachable!("appending composite glyphs needs glyph rows");
        }
        Ok(())
    }

    fn produce_glyphless_glyph_tty(&mut self, it: &mut It, acronym: Value) -> LispResult<()> {
        let _face_id = self.merge_glyphless_glyph_face(it)?;
        let text: Vec<u8> = match it.glyphless_method {
            GlyphlessMethod::ThinSpace => b"    "[..1].to_vec(),
            GlyphlessMethod::EmptyBox => {
                let mut len = self.character_width(i64::from(it.c));
                if len == 0 {
                    len = 1;
                } else if len > 4 {
                    len = 4;
                }
                format!("[{}]", &"    "[..len as usize]).into_bytes()
            }
            GlyphlessMethod::Acronym => {
                let mut acronym = acronym;
                let table = self.find_symbol_value(sym::GLYPHLESS_CHAR_DISPLAY);
                if !acronym.is_string() && self.is_char_table(table) {
                    acronym = self.char_table_ref(table, it.c as u32);
                }
                if acronym.is_cons() {
                    acronym = self.xcdr(acronym);
                }
                let s: Vec<u8> = if acronym.is_string() {
                    self.string_bytes(acronym).to_vec()
                } else {
                    Vec::new()
                };
                if acronym.is_string() && self.string_chars(acronym) == 1 {
                    vec![s[0]]
                } else {
                    let mut buf = vec![b'['];
                    for &b in s.iter().take(6) {
                        if b == 0 || !b.is_ascii() {
                            break;
                        }
                        buf.push(b);
                    }
                    buf.push(b']');
                    buf
                }
            }
            GlyphlessMethod::HexCode => {
                let c = it.c as u32;
                if c < 0x10000 {
                    format!("\\u{c:04X}").into_bytes()
                } else if c <= 0x10FFFF {
                    format!("\\U{c:06X}").into_bytes()
                } else {
                    format!("\\x{c:06X}").into_bytes()
                }
            }
        };
        it.pixel_width = text.len() as i32;
        it.nglyphs = text.len() as i32;
        if it.glyph_row.is_some() {
            unreachable!("appending glyphless glyphs needs glyph rows");
        }
        Ok(())
    }

    pub(super) fn produce_special_glyphs(
        &mut self,
        it: &mut It,
        what: DisplayElementType,
        direction: BidiDir,
        left_edge_p: bool,
    ) -> LispResult<()> {
        let mut face_id = self.lookup_basic_face(Some(it.window), it.f, DEFAULT_FACE_ID)? as i32;
        let mut temp_it = it.clone();
        temp_it.object = Value::NIL;
        temp_it.current = crate::dispextern::DisplayPos::default();
        let (mut glyph_char, mut glyph_face);
        let (default_char, slot, mirror) = match what {
            DisplayElementType::Continuation => (
                if direction == BidiDir::R2L {
                    b'/'
                } else {
                    b'\\'
                },
                DISP_CONTINUE_GLYPH_SLOT,
                direction == BidiDir::R2L,
            ),
            DisplayElementType::Truncation => (
                b'$',
                DISP_TRUNC_GLYPH_SLOT,
                (it.bidi_it.paragraph_dir == BidiDir::R2L && !left_edge_p)
                    || (it.bidi_it.paragraph_dir == BidiDir::L2R && left_edge_p),
            ),
            _ => std::process::abort(),
        };
        let fallback = if what == DisplayElementType::Continuation {
            i32::from(b'/')
        } else {
            i32::from(b'$')
        };
        glyph_char = i32::from(default_char);
        glyph_face = face_id;
        let gc = if it.dp.is_nil() {
            Value::NIL
        } else {
            self.char_table(it.dp).extras[slot]
        };
        if !it.dp.is_nil() && self.glyph_code_p(gc) {
            if mirror {
                face_id = self.glyph_code_face(gc);
                let gchar = self.glyph_code_char(gc) as i32;
                let c = self.bidi_mirror_char(gchar)?;
                if c == gchar {
                    let table = self.find_symbol_value(sym::SPECIAL_MIRROR_TABLE);
                    let val = self.char_table_ref(table, gchar as u32);
                    if let Some(v) = val.as_fixnum() {
                        if (0..=i64::from(character::MAX_CHAR)).contains(&v) {
                            glyph_char = v as i32;
                        } else {
                            glyph_char = fallback;
                        }
                        glyph_face = face_id;
                    } else {
                        glyph_char = gchar;
                        glyph_face = self.glyph_code_face(gc);
                    }
                } else {
                    let id =
                        self.face_for_char(it.f, face_id as usize, c as u32, -1, Value::NIL)?;
                    glyph_char = c;
                    glyph_face = id as i32;
                }
            } else {
                glyph_char = self.glyph_code_char(gc) as i32;
                glyph_face = self.glyph_code_face(gc);
            }
            glyph_face = self.spec_glyph_lookup_face(it.window, glyph_face)?;
            if self.frame_window_p(it.f) {
                let new_id = self.face_for_char(
                    it.f,
                    glyph_face as usize,
                    glyph_char as u32,
                    -1,
                    Value::NIL,
                )?;
                glyph_face = new_id as i32;
            }
        }
        if self.frame_window_p(temp_it.f) && temp_it.glyph_row.is_some() {
            unreachable!("stretch padding before special glyphs needs glyph rows");
        }
        temp_it.dp = Value::NIL;
        temp_it.what = DisplayElementType::Character;
        temp_it.c = glyph_char;
        temp_it.char_to_display = glyph_char;
        temp_it.face_id = glyph_face;
        temp_it.len = character::char_bytes(glyph_char as u32) as i32;
        self.produce_glyphs_macro(&mut temp_it)?;
        it.pixel_width = temp_it.pixel_width;
        it.nglyphs = temp_it.nglyphs;
        Ok(())
    }

    pub(super) fn get_it_property(&mut self, it: &It, prop: Value) -> LispResult {
        let (position, object) = if it.object.is_string() {
            (Value::fixnum(it.string_charpos()), it.object)
        } else if self.is_buffer(it.object) {
            (Value::fixnum(it.charpos()), it.window)
        } else {
            return Ok(Value::NIL);
        };
        self.get_char_property(position, prop, object)
    }

    fn get_line_prefix_it_property(&mut self, it: &It, prop: Value) -> LispResult<(Value, bool)> {
        let prefix = self.get_it_property(it, prop)?;
        let from_buffer = self.is_buffer(it.object);
        if prefix.is_nil() && it.sp > 0 && it.object.is_string() {
            let contents = self.window(it.window).contents;
            let v = self.get_char_property(Value::fixnum(it.charpos()), prop, contents)?;
            return Ok((v, true));
        }
        Ok((prefix, from_buffer))
    }

    fn push_prefix_prop(
        &mut self,
        it: &mut It,
        prop: Value,
        from_buffer: bool,
    ) -> LispResult<bool> {
        let pos = if it.string.is_string() {
            it.current.string_pos
        } else {
            it.current.pos
        };
        let phony_display_string =
            from_buffer && it.string.is_string() && it.string_from_display_prop_p;
        let eob = if it.string.is_string() {
            self.string_chars(it.string) as i64
        } else {
            self.cur().zv as i64
        };
        if it.bidi_it.first_elt && it.bidi_it.charpos < eob {
            let pe = it.paragraph_embedding;
            self.bidi_paragraph_init(pe, &mut it.bidi_it, true)?;
        }
        self.push_it(it, Some(pos))?;
        if phony_display_string {
            it.string_from_display_prop_p = false;
        }
        if prop.is_string() {
            if self.string_chars(prop) == 0 {
                self.pop_it(it)?;
                return Ok(false);
            }
            it.string = prop;
            it.string_from_prefix_prop_p = true;
            it.multibyte_p = self.string_multibyte(prop);
            it.current.overlay_string_index = -1;
            it.current.string_pos = TextPos::new(0, 0);
            it.end_charpos = self.string_chars(prop) as i64;
            it.string_nchars = it.end_charpos;
            it.method = ItMethod::FromString;
            it.stop_charpos = 0;
            it.prev_stop = 0;
            it.base_level_stop = 0;
            it.cmp_it.id = -1;
            it.paragraph_embedding = if it.bidi_p && it.bidi_it.paragraph_dir == BidiDir::R2L {
                it.bidi_it.paragraph_dir
            } else {
                BidiDir::L2R
            };
            if it.bidi_p {
                it.bidi_it.string.lstring = it.string;
                it.bidi_it.string.s = None;
                it.bidi_it.string.schars = it.end_charpos;
                it.bidi_it.string.bufpos = it.charpos();
                it.bidi_it.string.from_disp_str = it.string_from_display_prop_p;
                it.bidi_it.string.unibyte = !it.multibyte_p;
                it.bidi_it.w = it.window;
                let fwp = self.frame_window_p(it.f);
                self.bidi_init_it(0, 0, fwp, &mut it.bidi_it)?;
            }
        } else if prop.is_cons() && self.xcar(prop) == sym::SPACE {
            it.method = ItMethod::FromStretch;
            it.object = prop;
            it.string_from_prefix_prop_p = true;
        } else if self.frame_window_p(it.f) && prop.is_cons() && self.xcar(prop) == sym::IMAGE {
            unreachable!("image prefixes need image.c's lookup_image");
        } else {
            self.pop_it(it)?;
            return Ok(false);
        }
        Ok(true)
    }

    pub(super) fn handle_line_prefix(&mut self, it: &mut It) -> LispResult<()> {
        let mut wrap_prop = false;
        let (mut prefix, from_buffer);
        if it.continuation_lines_width > 0 {
            (prefix, from_buffer) = self.get_line_prefix_it_property(it, sym::WRAP_PREFIX)?;
            if prefix.is_nil() {
                prefix = self.plain_value(sym::WRAP_PREFIX);
            }
            wrap_prop = true;
        } else {
            (prefix, from_buffer) = self.get_line_prefix_it_property(it, sym::LINE_PREFIX)?;
            if prefix.is_nil() {
                prefix = self.plain_value(sym::LINE_PREFIX);
            }
        }
        if !prefix.is_nil() && self.push_prefix_prop(it, prefix, from_buffer)? {
            it.line_wrap = LineWrap::Truncate;
            it.avoid_cursor_p = true;
            if wrap_prop && prefix.is_cons() && self.xcar(prefix) == sym::SPACE {
                it.align_visually_p = true;
            }
        }
        Ok(())
    }

    fn disable_line_numbers_overlay_at_eob(&mut self) -> LispResult<bool> {
        let zv = self.cur().zv;
        let overlays: Vec<Value> = self
            .cur()
            .overlays
            .iter()
            .copied()
            .filter(|&o| {
                let ov = self.overlay(o);
                ov.start <= zv && zv <= ov.end
            })
            .collect();
        for o in overlays {
            let tem = crate::overlay::f_overlay_get(self, &[o, sym::DISPLAY_LINE_NUMBERS_DISABLE])?;
            if !tem.is_nil() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn should_produce_line_number(&mut self, it: &It) -> LispResult<bool> {
        if self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS).is_nil() {
            return Ok(false);
        }
        if self.window(it.window).mini {
            return Ok(false);
        }
        let f = self.window(it.window).frame;
        if self.frame(f).flags.tooltip {
            return Ok(false);
        }
        let val = self.get_char_property(
            Value::fixnum(it.charpos()),
            sym::DISPLAY_LINE_NUMBERS_DISABLE,
            it.window,
        )?;
        if val.is_nil() && it.charpos() >= self.cur().zv as i64 {
            return Ok(!self.disable_line_numbers_overlay_at_eob()?);
        }
        Ok(val.is_nil())
    }
}
