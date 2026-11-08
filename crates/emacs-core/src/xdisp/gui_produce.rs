use crate::composite::CompositionMethod;
use crate::dispextern::{DisplayElementType, GlyphRowArea, GlyphlessMethod, It};
use crate::error::LispResult;
use crate::font::{FONT_INVALID_CODE, FontMetrics};
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::xfaces::FaceBoxType;

const THIN_SPACE_WIDTH: i32 = 1;

#[derive(Clone, Copy)]
struct FaceBits {
    id: usize,
    box_type: FaceBoxType,
    box_horizontal_line_width: i32,
    box_vertical_line_width: i32,
    overline_p: bool,
    overstrike: bool,
    font: Value,
    ascii_face: usize,
}

#[derive(Clone, Copy)]
struct FontBits {
    ascent: i32,
    descent: i32,
    height: i32,
    pixel_size: i32,
    space_width: i32,
    baseline_offset: i32,
    vertical_centering: bool,
    default_ascent: i32,
}

impl FontBits {
    fn too_high(&self) -> bool {
        self.pixel_size > 0 && self.ascent + self.descent > 3 * self.pixel_size
    }
}

fn apply_face_box(it: &mut It, face: &FaceBits) {
    if face.box_type != FaceBoxType::NoBox {
        let thick = face.box_horizontal_line_width;
        if thick > 0 {
            it.ascent += thick;
            it.descent += thick;
        }
        let thick = face.box_vertical_line_width;
        if thick > 0 {
            if it.start_of_box_run_p {
                it.pixel_width += thick;
            }
            if it.end_of_box_run_p {
                it.pixel_width += thick;
            }
        }
    }
}

fn empty_metric(m: &FontMetrics) -> bool {
    m.width == 0 && m.rbearing == 0 && m.lbearing == 0
}

impl Lisp {
    fn face_bits(&self, f: Value, id: usize) -> FaceBits {
        let face = self.face_from_id(f, id).expect("face is not realized");
        FaceBits {
            id: face.id,
            box_type: face.box_type,
            box_horizontal_line_width: face.box_horizontal_line_width,
            box_vertical_line_width: face.box_vertical_line_width,
            overline_p: face.overline_p,
            overstrike: face.overstrike,
            font: face.font,
            ascii_face: face.ascii_face,
        }
    }

    fn font_bits(&self, font: Value) -> FontBits {
        let f = self.font_object(font);
        FontBits {
            ascent: f.ascent,
            descent: f.descent,
            height: f.height,
            pixel_size: f.pixel_size,
            space_width: f.space_width,
            baseline_offset: f.baseline_offset,
            vertical_centering: f.vertical_centering,
            default_ascent: f.default_ascent,
        }
    }

    fn vcenter_baseline_offset(&self, font: Value, f: Value) -> i32 {
        let fb = self.font_bits(font);
        let line_height = self.frame(f).geometry.line_height as i32;
        let frame_font = self.frame(f).font;
        let frame_descent = self.font_object(frame_font).descent;
        let frame_boff = self.frame(f).baseline_offset as i32;
        fb.descent + (line_height - fb.height + i32::from(line_height > fb.height)) / 2
            - (frame_descent - frame_boff)
    }

    fn overline_margin(&self) -> i32 {
        self.int_var(sym::OVERLINE_MARGIN).clamp(0, 1_000_000) as i32
    }

    fn char_table_set_p(&self, var: Value, c: i32) -> bool {
        let table = self.find_symbol_value(var);
        self.is_char_table(table) && !self.char_table_ref(table, c as u32).is_nil()
    }

    fn get_char_face_and_encoding(&mut self, f: Value, c: i32, face_id: usize) -> u32 {
        let font = self
            .face_from_id(f, face_id)
            .expect("face is not realized")
            .font;
        let mut code = 0;
        if !font.is_nil() {
            let encode_char = self
                .font_object(font)
                .driver
                .expect("font driver")
                .encode_char;
            code = encode_char(self, font, c);
            if code == FONT_INVALID_CODE {
                code = 0;
            }
        }
        self.prepare_face_for_display(f, face_id);
        code & 0xFFFF
    }

    fn char_metric(&mut self, font: Value, c: i32) -> Option<FontMetrics> {
        let char2b = self.get_char_glyph_code(c as u32, font)?;
        self.get_per_char_metric(font, char2b)
    }

    fn calc_line_height_property(
        &mut self,
        it: &mut It,
        val: Value,
        font: Value,
        boff: i32,
        override_p: bool,
    ) -> LispResult<Value> {
        let mut val = val;
        let mut font = font;
        let mut boff = boff;
        let mut override_p = override_p;
        if val.is_nil() || self.is_integer(val) || (override_p && val == sym::T) {
            return Ok(val);
        }
        let mut face_name = Value::NIL;
        let mut height;
        'scale: {
            if val.is_cons() {
                face_name = self.xcar(val);
                val = self.xcdr(val);
                if !self.is_number(val) {
                    val = Value::fixnum(1);
                }
                if face_name.is_nil() {
                    height = it.ascent + it.descent;
                    break 'scale;
                }
            }
            if face_name.is_nil() {
                font = self.frame(it.f).font;
                boff = self.frame(it.f).baseline_offset as i32;
            } else if face_name == sym::T {
                override_p = false;
            } else {
                let face_id = self.lookup_named_face(Some(it.window), it.f, face_name, false)?;
                let face_font = if face_id >= 0 {
                    self.face_from_id(it.f, face_id as usize)
                        .map(|face| face.font)
                } else {
                    None
                };
                match face_font {
                    Some(face_font) if !face_font.is_nil() => font = face_font,
                    _ => return Ok(Value::fixnum(-1)),
                }
                let fb = self.font_bits(font);
                boff = fb.baseline_offset;
                if fb.vertical_centering {
                    boff = self.vcenter_baseline_offset(font, it.f) - boff;
                }
            }
            let (ascent, descent) = self.normal_char_ascent_descent(font, -1);
            if override_p {
                it.override_ascent = ascent;
                it.override_descent = descent;
                it.override_boff = boff;
            }
            height = ascent + descent;
        }
        if val.is_float() {
            height = (self.heap.float_value(val) * f64::from(height)) as i32;
        } else if let Some(v) = val.as_fixnum() {
            height = (i64::from(height) * v) as i32;
        }
        Ok(Value::fixnum(i64::from(height)))
    }

    fn produce_glyphless_glyph_gui(&mut self, it: &mut It, acronym: Value) -> LispResult<()> {
        let ascii_font = self
            .face_from_id(it.f, it.face_id as usize)
            .map(|face| face.ascii_face)
            .and_then(|id| self.face_from_id(it.f, id))
            .map(|face| face.font)
            .filter(|font| !font.is_nil());
        let font = ascii_font.unwrap_or(self.frame(it.f).font);
        let (ascent, descent) = self.normal_char_ascent_descent(font, -1);
        let fb = self.font_bits(font);
        it.ascent = ascent + fb.baseline_offset;
        it.descent = descent - fb.baseline_offset;
        let base_height = it.ascent + it.descent;
        let base_width = self.font_object(font).average_width;
        let face_id = self.merge_glyphless_glyph_face(it)? as usize;
        match it.glyphless_method {
            GlyphlessMethod::ThinSpace => it.pixel_width = THIN_SPACE_WIDTH,
            GlyphlessMethod::EmptyBox => {
                let mut width = self.character_width(i64::from(it.c)) as i32;
                if width == 0 {
                    width = 1;
                } else if width > 4 {
                    width = 4;
                }
                it.pixel_width = base_width * width;
            }
            GlyphlessMethod::Acronym | GlyphlessMethod::HexCode => {
                let face_font = self.face_from_id(it.f, face_id).expect("face").font;
                let font = if face_font.is_nil() {
                    self.frame(it.f).font
                } else {
                    face_font
                };
                self.prepare_face_for_display(it.f, face_id);
                let text: Vec<u8> = if it.glyphless_method == GlyphlessMethod::Acronym {
                    let mut acronym = acronym;
                    let table = self.find_symbol_value(sym::GLYPHLESS_CHAR_DISPLAY);
                    if !acronym.is_string() && self.is_char_table(table) {
                        acronym = self.char_table_ref(table, it.c as u32);
                    }
                    if acronym.is_cons() {
                        acronym = self.xcar(acronym);
                    }
                    if acronym.is_string() {
                        self.string_bytes(acronym).to_vec()
                    } else {
                        Vec::new()
                    }
                } else if it.c < 0x10000 {
                    format!("{:04X}", it.c as u32).into_bytes()
                } else {
                    format!("{:06X}", it.c as u32).into_bytes()
                };
                let driver = self.font_object(font).driver.expect("font driver");
                let mut code = Vec::new();
                for &b in text.iter().take(6) {
                    if b == 0 || !b.is_ascii() {
                        break;
                    }
                    code.push((driver.encode_char)(self, font, i32::from(b)));
                }
                let upper_len = code.len().div_ceil(2);
                let upper = (driver.text_extents)(self, font, &code[..upper_len]);
                let lower = (driver.text_extents)(self, font, &code[upper_len..]);
                let width = upper.width.max(lower.width) + 4;
                it.pixel_width = if base_width >= width {
                    base_width
                } else {
                    width
                };
                let height = upper.ascent + upper.descent + lower.ascent + lower.descent + 5;
                let ascent = -(it.descent - (base_height + height + 1) / 2);
                let descent = it.descent - (base_height - height) / 2;
                if height > base_height {
                    it.ascent = ascent;
                    it.descent = descent;
                }
            }
        }
        it.phys_ascent = it.ascent;
        it.phys_descent = it.descent;
        if it.glyph_row.is_some() {
            unreachable!("appending glyphless glyphs needs glyph rows");
        }
        it.nglyphs = 1;
        Self::take_vertical_position_into_account(it);
        Ok(())
    }

    fn produce_character_glyph_gui(
        &mut self,
        it: &mut It,
        extra_line_spacing: &mut i32,
    ) -> LispResult<()> {
        let face = self.face_bits(it.f, it.face_id as usize);
        let font = face.font;
        if font.is_nil() {
            let acronym = self.lookup_glyphless_char_display(-1, it);
            let acronym = if acronym.is_string() {
                acronym
            } else {
                Value::NIL
            };
            return self.produce_glyphless_glyph_gui(it, acronym);
        }
        let fb = self.font_bits(font);
        let mut boff = fb.baseline_offset;
        if fb.vertical_centering {
            boff = self.vcenter_baseline_offset(font, it.f) - boff;
        }
        if it.char_to_display != 10 && it.char_to_display != 9 {
            it.nglyphs = 1;
            if it.override_ascent >= 0 {
                it.ascent = it.override_ascent;
                it.descent = it.override_descent;
                boff = it.override_boff;
            } else {
                it.ascent = fb.ascent + boff;
                it.descent = fb.descent - boff;
            }
            let mut pcm = self.char_metric(font, it.char_to_display);
            if let Some(m) = pcm.as_mut()
                && face.overstrike
            {
                m.rbearing += 1;
            }
            if pcm.as_ref().is_some_and(empty_metric) {
                pcm = None;
            }
            if let Some(m) = pcm {
                it.phys_ascent = m.ascent + boff;
                it.phys_descent = m.descent - boff;
                it.pixel_width = m.width;
                if it.override_ascent < 0 && fb.too_high() {
                    it.ascent = it.phys_ascent.max(0);
                    it.descent = it.phys_descent.max(0);
                }
            } else {
                it.glyph_not_available_p = true;
                it.phys_ascent = it.ascent;
                it.phys_descent = it.descent;
                it.pixel_width = fb.space_width;
            }
            if it.constrain_row_ascent_descent_p {
                if it.descent > it.max_descent {
                    it.ascent += it.descent - it.max_descent;
                    it.descent = it.max_descent;
                }
                if it.ascent > it.max_ascent {
                    it.descent = it.max_descent.min(it.descent + it.ascent - it.max_ascent);
                    it.ascent = it.max_ascent;
                }
                it.phys_ascent = it.phys_ascent.min(it.ascent);
                it.phys_descent = it.phys_descent.min(it.descent);
                *extra_line_spacing = 0;
            }
            let stretched_p = it.char_to_display == 32 && !it.space_width.is_nil();
            if stretched_p {
                it.pixel_width =
                    (f64::from(it.pixel_width) * self.xfloatint(it.space_width)) as i32;
            }
            apply_face_box(it, &face);
            if face.overline_p {
                it.ascent += self.overline_margin();
            }
            if it.constrain_row_ascent_descent_p {
                if it.ascent > it.max_ascent {
                    it.ascent = it.max_ascent;
                }
                if it.descent > it.max_descent {
                    it.descent = it.max_descent;
                }
            }
            Self::take_vertical_position_into_account(it);
            if it.glyph_row.is_some() {
                unreachable!("appending glyphs needs glyph rows");
            }
            if !stretched_p && it.pixel_width == 0 {
                it.pixel_width = 1;
            }
        } else if it.char_to_display == 10 {
            let mut total_height = Value::NIL;
            it.override_ascent = -1;
            it.pixel_width = 0;
            it.nglyphs = 0;
            let mut height = self.get_it_property(it, sym::LINE_HEIGHT)?;
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
            } else if fb.too_high() {
                it.ascent = fb.pixel_size + boff - 1;
                it.descent = (-boff + 1).max(0);
            } else {
                it.ascent = fb.ascent + boff;
                it.descent = fb.descent - boff;
            }
            if height == sym::T {
                if it.descent > it.max_descent {
                    it.ascent += it.descent - it.max_descent;
                    it.descent = it.max_descent;
                }
                if it.ascent > it.max_ascent {
                    it.descent = it.max_descent.min(it.descent + it.ascent - it.max_ascent);
                    it.ascent = it.max_ascent;
                }
                it.phys_ascent = it.phys_ascent.min(it.ascent);
                it.phys_descent = it.phys_descent.min(it.descent);
                it.constrain_row_ascent_descent_p = true;
                *extra_line_spacing = 0;
            } else {
                it.phys_ascent = it.ascent;
                it.phys_descent = it.descent;
                if (it.max_ascent > 0 || it.max_descent > 0)
                    && face.box_type != FaceBoxType::NoBox
                    && face.box_horizontal_line_width > 0
                {
                    it.ascent += face.box_horizontal_line_width;
                    it.descent += face.box_horizontal_line_width;
                }
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
                if let Some(n) = spacing.as_fixnum() {
                    *extra_line_spacing = n as i32;
                    if !total_height.is_nil() {
                        *extra_line_spacing -= it.phys_ascent + it.phys_descent;
                    }
                }
            }
            self.stack.truncate(base);
        } else if fb.space_width > 0 {
            let tab_width = i32::from(it.tab_width) * fb.space_width;
            let mut x = it.current_x
                + if it.string_from_prefix_prop_p {
                    0
                } else if it.continuation_lines_width != 0 {
                    it.continuation_lines_width - it.wrap_prefix_width
                } else {
                    0
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
            let mut next_tab_x = ((1 + x + tab_width - 1) / tab_width) * tab_width;
            if next_tab_x - x < fb.space_width {
                next_tab_x += tab_width;
            }
            if lnum {
                next_tab_x += it.lnum_pixel_width;
                if x < it.first_visible_x {
                    next_tab_x -= it.first_visible_x - x;
                    it.stretch_adjust = it.first_visible_x - x;
                } else {
                    next_tab_x -= it.stretch_adjust;
                }
            }
            it.pixel_width = next_tab_x - x0;
            it.nglyphs = 1;
            if fb.too_high() {
                let pcm = self.char_metric(font, 32).filter(|m| !empty_metric(m));
                if let Some(m) = pcm {
                    it.ascent = m.ascent + boff;
                    it.descent = m.descent - boff;
                } else {
                    it.ascent = fb.pixel_size + boff - 1;
                    it.descent = -boff + 1;
                }
                it.ascent = it.ascent.max(0);
                it.descent = it.descent.max(0);
            } else {
                it.ascent = fb.ascent + boff;
                it.descent = fb.descent - boff;
            }
            it.phys_ascent = it.ascent;
            it.phys_descent = it.descent;
            if it.glyph_row.is_some() {
                unreachable!("appending stretch glyphs needs glyph rows");
            }
        } else {
            it.pixel_width = 0;
            it.nglyphs = 1;
        }
        if fb.too_high() {
            let (font_ascent, font_descent) = self.normal_char_ascent_descent(font, -1);
            it.max_ascent = it.max_ascent.max(font_ascent);
            it.max_descent = it.max_descent.max(font_descent);
        }
        it.ascent = it.ascent.max(0);
        it.descent = it.descent.max(0);
        Ok(())
    }

    fn composition_glyph(&self, id: usize, n: usize) -> i32 {
        let cmp = &self.compositions.table[id];
        let idx = if cmp.method == CompositionMethod::WithRuleAltchars {
            n * 2
        } else {
            n
        };
        self.aref_items(cmp.key)[idx].as_fixnum().unwrap_or(0) as i32
    }

    fn compute_static_composition_metrics(
        &mut self,
        it: &It,
        id: usize,
        face: &mut FaceBits,
    ) -> LispResult<()> {
        let mut font = face.font;
        let (cmp_glyph_len, method, key) = {
            let cmp = &self.compositions.table[id];
            (cmp.glyph_len, cmp.method, cmp.key)
        };
        let mut glyph_len = cmp_glyph_len;
        let mut offsets = self.compositions.table[id].offsets.clone();
        let mut c = 9;
        if glyph_len > 0 {
            loop {
                c = self.composition_glyph(id, glyph_len - 1);
                if c == 9 {
                    glyph_len -= 1;
                    if glyph_len > 0 {
                        continue;
                    }
                }
                break;
            }
        }
        let right_padded = glyph_len < cmp_glyph_len;
        let mut i = 0;
        while i < glyph_len {
            c = self.composition_glyph(id, i);
            if c != 9 {
                break;
            }
            offsets[i * 2] = 0;
            offsets[i * 2 + 1] = 0;
            i += 1;
        }
        let left_padded = i > 0;
        let pos = if it.string.is_string() {
            it.string_charpos()
        } else {
            it.charpos()
        };
        let font_not_found_p = font.is_nil();
        if font_not_found_p {
            *face = self.face_bits(it.f, face.ascii_face);
            font = face.font;
        }
        let fb = self.font_bits(font);
        let mut boff = fb.baseline_offset;
        if fb.vertical_centering {
            boff = self.vcenter_baseline_offset(font, it.f) - boff;
        }
        let (mut font_ascent, mut font_descent) = self.normal_char_ascent_descent(font, -1);
        font_ascent += boff;
        font_descent -= boff;
        let font_height = font_ascent + font_descent;
        self.compositions.table[id].font = font;
        let mut pcm = None;
        if !font_not_found_p {
            let char2b = self.get_char_face_and_encoding(it.f, c, it.face_id as usize);
            pcm = self.get_per_char_metric(font, char2b);
        }
        let (mut width, mut ascent, mut descent, mut lbearing, mut rbearing);
        if let Some(mut m) = pcm {
            if face.overstrike {
                m.rbearing += 1;
            }
            width = if cmp_glyph_len > 0 { m.width } else { 0 };
            ascent = m.ascent;
            descent = m.descent;
            lbearing = m.lbearing;
            rbearing = m.rbearing;
        } else {
            width = if cmp_glyph_len > 0 { fb.space_width } else { 0 };
            ascent = fb.ascent;
            descent = fb.descent;
            lbearing = 0;
            rbearing = width;
        }
        let mut rightmost = width;
        let mut leftmost = 0;
        let mut lowest = -descent + boff;
        let mut highest = ascent + boff;
        if !font_not_found_p
            && fb.default_ascent != 0
            && self.char_table_set_p(sym::USE_DEFAULT_ASCENT, it.char_to_display)
        {
            highest = fb.default_ascent + boff;
        }
        if i * 2 + 1 < offsets.len() {
            offsets[i * 2] = 0;
            offsets[i * 2 + 1] = boff as i16;
        }
        let mut cmp_lbearing = lbearing;
        let mut cmp_rbearing = rbearing;
        i += 1;
        while i < glyph_len {
            let mut ch = self.composition_glyph(id, i);
            if ch == 9 {
                ch = 32;
            }
            let face_id = self.face_for_char(it.f, face.id, ch as u32, pos, it.string)?;
            font = self.face_from_id(it.f, face_id).expect("face").font;
            let pcm = if font.is_nil() {
                None
            } else {
                let char2b = self.get_char_face_and_encoding(it.f, ch, face_id);
                self.get_per_char_metric(font, char2b)
            };
            let Some(mut m) = pcm else {
                offsets[i * 2] = 0;
                offsets[i * 2 + 1] = 0;
                i += 1;
                continue;
            };
            if face.overstrike {
                m.rbearing += 1;
            }
            width = m.width;
            ascent = m.ascent;
            descent = m.descent;
            lbearing = m.lbearing;
            rbearing = m.rbearing;
            let (left, btm);
            if method != CompositionMethod::WithRuleAltchars {
                left = (leftmost + rightmost - width) / 2;
                let mut b = -descent + boff;
                let relative_compose = self.font_object(font).relative_compose;
                let ignore = self.find_symbol_value(sym::IGNORE_RELATIVE_COMPOSITION);
                if relative_compose != 0
                    && (!self.is_char_table(ignore)
                        || self.char_table_ref(ignore, ch as u32).is_nil())
                {
                    if -descent >= relative_compose {
                        b = highest + 1;
                    } else if ascent <= 0 {
                        b = lowest - 1 - ascent - descent;
                    }
                }
                btm = b;
            } else {
                let mut rule = self.aref_items(key)[i * 2 - 1].as_fixnum().unwrap_or(0) as i32;
                let mut xoff = rule >> 16;
                let mut yoff = (rule >> 8) & 0xFF;
                rule &= 0xFF;
                let mut gref = rule / 12;
                if gref > 12 {
                    gref = 11;
                }
                let nref = rule % 12;
                let (grefx, nrefx) = (gref % 3, nref % 3);
                let (grefy, nrefy) = (gref / 3, nref / 3);
                if xoff != 0 {
                    xoff = font_height * (xoff - 128) / 256;
                }
                if yoff != 0 {
                    yoff = font_height * (yoff - 128) / 256;
                }
                left = leftmost + grefx * (rightmost - leftmost) / 2 - nrefx * width / 2 + xoff;
                btm = (match grefy {
                    0 => highest,
                    1 => 0,
                    2 => lowest,
                    _ => (highest + lowest) / 2,
                }) - (match nrefy {
                    0 => ascent + descent,
                    1 => descent - boff,
                    2 => 0,
                    _ => (ascent + descent) / 2,
                }) + yoff;
            }
            offsets[i * 2] = left as i16;
            offsets[i * 2 + 1] = (btm + descent) as i16;
            if width > 0 {
                let right = left + width;
                if left < leftmost {
                    leftmost = left;
                }
                if right > rightmost {
                    rightmost = right;
                }
            }
            let top = btm + descent + ascent;
            if top > highest {
                highest = top;
            }
            if btm < lowest {
                lowest = btm;
            }
            if cmp_lbearing > left + lbearing {
                cmp_lbearing = left + lbearing;
            }
            if cmp_rbearing < left + rbearing {
                cmp_rbearing = left + rbearing;
            }
            i += 1;
        }
        if leftmost < 0 {
            for n in 0..cmp_glyph_len {
                offsets[n * 2] -= leftmost as i16;
            }
            rightmost -= leftmost;
            cmp_lbearing -= leftmost;
            cmp_rbearing -= leftmost;
        }
        if left_padded && cmp_lbearing < 0 {
            for n in 0..cmp_glyph_len {
                offsets[n * 2] -= cmp_lbearing as i16;
            }
            rightmost -= cmp_lbearing;
            cmp_rbearing -= cmp_lbearing;
            cmp_lbearing = 0;
        }
        if right_padded && rightmost < cmp_rbearing {
            rightmost = cmp_rbearing;
        }
        let cmp = &mut self.compositions.table[id];
        cmp.offsets = offsets;
        cmp.lbearing = cmp_lbearing;
        cmp.rbearing = cmp_rbearing;
        cmp.pixel_width = rightmost;
        cmp.ascent = highest.max(font_ascent);
        cmp.descent = (-lowest).max(font_descent);
        Ok(())
    }

    pub(super) fn gui_produce_glyphs(&mut self, it: &mut It) -> LispResult<()> {
        let mut extra_line_spacing = it.extra_line_spacing;
        let extra_line_spacing_above = it.extra_line_spacing_above;
        it.glyph_not_available_p = false;
        if it.what == DisplayElementType::Character {
            self.produce_character_glyph_gui(it, &mut extra_line_spacing)?;
        } else if it.what == DisplayElementType::Composition && it.cmp_it.ch < 0 {
            let mut face = self.face_bits(it.f, it.face_id as usize);
            let id = it.cmp_it.id as usize;
            it.nglyphs = 1;
            let cmp_font = self.compositions.table[id].font;
            if cmp_font.is_nil() || cmp_font != face.font {
                self.compute_static_composition_metrics(it, id, &mut face)?;
            }
            let cmp = &self.compositions.table[id];
            let (pixel_width, ascent, descent, glyph_len) =
                (cmp.pixel_width, cmp.ascent, cmp.descent, cmp.glyph_len);
            it.pixel_width = pixel_width;
            it.ascent = ascent;
            it.phys_ascent = ascent;
            it.descent = descent;
            it.phys_descent = descent;
            apply_face_box(it, &face);
            if face.overline_p {
                it.ascent += self.overline_margin();
            }
            Self::take_vertical_position_into_account(it);
            it.ascent = it.ascent.max(0);
            it.descent = it.descent.max(0);
            if it.glyph_row.is_some() && glyph_len > 0 {
                unreachable!("appending composite glyphs needs glyph rows");
            }
        } else if it.what == DisplayElementType::Composition {
            let face = self.face_bits(it.f, it.face_id as usize);
            it.nglyphs = 1;
            let gstring = self.composition_gstring_from_id(it.cmp_it.id as usize);
            let mut metrics = FontMetrics::default();
            it.pixel_width = self.composition_gstring_width(
                gstring,
                it.cmp_it.from as usize,
                it.cmp_it.to as usize,
                Some(&mut metrics),
            );
            if it.pixel_width == 0 {
                it.glyph_not_available_p = true;
                it.phys_ascent = it.ascent;
                it.phys_descent = it.descent;
                it.pixel_width = self.font_object(face.font).space_width;
            } else {
                it.ascent = metrics.ascent;
                it.phys_ascent = metrics.ascent;
                it.descent = metrics.descent;
                it.phys_descent = metrics.descent;
            }
            apply_face_box(it, &face);
            if face.overline_p {
                it.ascent += self.overline_margin();
            }
            Self::take_vertical_position_into_account(it);
            it.ascent = it.ascent.max(0);
            it.descent = it.descent.max(0);
            if it.glyph_row.is_some() {
                unreachable!("appending composite glyphs needs glyph rows");
            }
        } else if it.what == DisplayElementType::Glyphless {
            self.produce_glyphless_glyph_gui(it, Value::NIL)?;
        } else if it.what == DisplayElementType::Image {
            unreachable!("image glyphs need image.c");
        } else if it.what == DisplayElementType::Stretch {
            self.produce_stretch_glyph(it)?;
        } else if it.what == DisplayElementType::Xwidget {
            unreachable!("xwidgets are not supported");
        }
        if it.area == GlyphRowArea::Text {
            it.current_x += it.pixel_width;
            if it.continuation_lines_width != 0 && it.string_from_prefix_prop_p {
                it.wrap_prefix_width = it.current_x;
            }
        }
        if extra_line_spacing > 0 {
            it.descent += extra_line_spacing - extra_line_spacing_above;
            it.ascent += extra_line_spacing_above;
            if extra_line_spacing > it.max_extra_line_spacing {
                it.max_extra_line_spacing = extra_line_spacing;
            }
        }
        it.max_ascent = it.max_ascent.max(it.ascent);
        it.max_descent = it.max_descent.max(it.descent);
        it.max_phys_ascent = it.max_phys_ascent.max(it.phys_ascent);
        it.max_phys_descent = it.max_phys_descent.max(it.phys_descent);
        Ok(())
    }
}
