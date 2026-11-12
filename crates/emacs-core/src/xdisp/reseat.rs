use std::rc::Rc;

use crate::bidi::{BidiDir, BidiIt, BidiStringData};
use crate::dispextern::{DisplayElementType, DisplayPos, GlyphlessMethod, It, ItMethod, TextPos};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub const DISP_INFINITY: i64 = 10_000_000;
pub const FACE_ID_BITS: u32 = 20;

impl Lisp {
    pub(super) fn reseat_at_next_visible_line_start(
        &mut self,
        it: &mut It,
        on_newline_p: bool,
    ) -> LispResult<()> {
        let mut skipped_p = false;
        let mut bidi_it_prev = BidiIt::default();
        let mut newline_found_p = self.forward_to_next_line_start(
            it,
            &mut skipped_p,
            if on_newline_p {
                Some(&mut bidi_it_prev)
            } else {
                None
            },
        )?;
        if it.selective > 0 {
            while it.charpos() < self.cur().zv as i64
                && self.indented_beyond_p(it.charpos(), it.selective)?
            {
                newline_found_p = self.forward_to_next_line_start(
                    it,
                    &mut skipped_p,
                    if on_newline_p {
                        Some(&mut bidi_it_prev)
                    } else {
                        None
                    },
                )?;
            }
        }
        if on_newline_p && newline_found_p {
            if it.string.is_string() {
                if it.string_charpos() > 0 {
                    if !it.bidi_p {
                        it.current.string_pos.charpos -= 1;
                        it.current.string_pos.bytepos -= 1;
                    } else {
                        it.bidi_it = bidi_it_prev;
                        it.current.string_pos =
                            TextPos::new(it.bidi_it.charpos, it.bidi_it.bytepos);
                    }
                }
            } else if it.charpos() > self.cur().begv as i64 {
                if !it.bidi_p {
                    it.current.pos.charpos -= 1;
                    it.current.pos.bytepos -= 1;
                } else {
                    it.bidi_it = bidi_it_prev;
                    it.current.pos = TextPos::new(it.bidi_it.charpos, it.bidi_it.bytepos);
                }
                let pos = it.current.pos;
                self.reseat(it, pos, false)?;
            }
        } else if skipped_p {
            let pos = it.current.pos;
            self.reseat(it, pos, false)?;
        }
        Ok(())
    }

    pub fn reseat(&mut self, it: &mut It, pos: TextPos, force_p: bool) -> LispResult<()> {
        let original_pos = it.charpos();
        self.reseat_1(it, pos, false)?;
        if self.cur().long_line_optimizations_p {
            let w = it.window;
            if it.medium_narrowing_begv == 0 {
                let point = self.window_point(w);
                it.medium_narrowing_begv = self.get_medium_narrowing_begv(w, point)?;
                it.medium_narrowing_zv = self.get_medium_narrowing_zv(w, point)?;
                it.large_narrowing_begv = self.get_large_narrowing_begv(point as usize) as i64;
                it.large_narrowing_zv = self.get_large_narrowing_zv(point as usize) as i64;
            } else if (pos.charpos < it.medium_narrowing_begv
                || pos.charpos > it.medium_narrowing_zv)
                && (!self.xdisp.redisplaying_p
                    || it.line_wrap == crate::dispextern::LineWrap::Truncate)
            {
                it.medium_narrowing_begv = self.get_medium_narrowing_begv(w, pos.charpos)?;
                it.medium_narrowing_zv = self.get_medium_narrowing_zv(w, pos.charpos)?;
                let point = self.window_point(w);
                it.large_narrowing_begv = self.get_large_narrowing_begv(point as usize) as i64;
                it.large_narrowing_zv = self.get_large_narrowing_zv(point as usize) as i64;
            }
        }
        if force_p || pos.charpos > it.stop_charpos || pos.charpos < original_pos {
            if it.bidi_p {
                if pos.charpos != it.prev_stop {
                    it.prev_stop = pos.charpos;
                }
                if pos.charpos < it.base_level_stop {
                    it.base_level_stop = 0;
                }
                self.handle_stop(it)?;
            } else {
                self.handle_stop(it)?;
                it.prev_stop = 0;
                it.base_level_stop = 0;
            }
        }
        Ok(())
    }

    pub fn reseat_1(&mut self, it: &mut It, pos: TextPos, set_stop_p: bool) -> LispResult<()> {
        it.current.pos = pos;
        it.position = pos;
        it.end_charpos = self.cur().zv as i64;
        it.dpvec = crate::dispextern::DpVec::None;
        it.current.dpvec_index = -1;
        it.current.overlay_string_index = -1;
        it.current.string_pos = TextPos::new(-1, -1);
        it.string = Value::NIL;
        it.method = ItMethod::FromBuffer;
        it.object = self.window(it.window).contents;
        it.area = crate::dispextern::GlyphRowArea::Text;
        it.multibyte_p = !self.plain_value(sym::ENABLE_MULTIBYTE_CHARACTERS).is_nil();
        it.sp = 0;
        it.string_from_display_prop_p = false;
        it.string_from_prefix_prop_p = false;
        it.from_disp_prop_p = false;
        it.face_before_selective_p = false;
        if it.bidi_p {
            let fwp = self.frame_window_p(it.f);
            let (c, b) = (it.charpos(), it.bytepos());
            self.bidi_init_it(c, b, fwp, &mut it.bidi_it)?;
            self.bidi_unshelve_cache(None, false);
            it.bidi_it.paragraph_dir = BidiDir::NeutralDir;
            it.bidi_it.string = BidiStringData::default();
            it.bidi_it.w = it.window;
        }
        if set_stop_p {
            it.stop_charpos = pos.charpos;
            it.base_level_stop = pos.charpos;
        }
        it.cmp_it.id = -1;
        it.min_width_property = Value::NIL;
        Ok(())
    }

    pub fn string_pos(&self, charpos: i64, string: Value) -> TextPos {
        if self.string_multibyte(string) {
            TextPos::new(
                charpos,
                self.string_char_to_byte_index(string, charpos as usize) as i64,
            )
        } else {
            TextPos::new(charpos, charpos)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reseat_to_string(
        &mut self,
        it: &mut It,
        s: Option<Rc<[u8]>>,
        string: Value,
        charpos: i64,
        precision: i64,
        field_width: i64,
        multibyte: i32,
    ) -> LispResult<()> {
        it.stop_charpos = -1;
        it.current = DisplayPos::default();
        it.current.overlay_string_index = -1;
        it.current.dpvec_index = -1;
        if multibyte >= 0 {
            it.multibyte_p = multibyte > 0;
        }
        let default_reordering = self.default_value_unchecked(sym::BIDI_DISPLAY_REORDERING);
        it.bidi_p = !self.var_true(sym::REDISPLAY__INHIBIT_BIDI) && !default_reordering.is_nil();
        let fwp = self.frame_window_p(it.f);
        let has_s = s.is_some();
        match s {
            None => {
                it.string = string;
                it.s = None;
                it.end_charpos = self.string_chars(string) as i64;
                it.string_nchars = it.end_charpos;
                it.method = ItMethod::FromString;
                it.current.string_pos = self.string_pos(charpos, string);
                if it.bidi_p {
                    it.bidi_it.string = BidiStringData {
                        lstring: string,
                        s: None,
                        schars: it.end_charpos,
                        bufpos: 0,
                        from_disp_str: false,
                        unibyte: !it.multibyte_p,
                    };
                    it.bidi_it.w = it.window;
                    let b = it.string_bytepos();
                    self.bidi_init_it(charpos, b, fwp, &mut it.bidi_it)?;
                }
            }
            Some(bytes) => {
                it.string = Value::NIL;
                it.current.string_pos = TextPos::new(-1, -1);
                if it.multibyte_p {
                    let bytepos = crate::character::char_offset_to_byte(&bytes, charpos as usize);
                    it.current.pos = TextPos::new(charpos, bytepos as i64);
                    it.end_charpos = crate::character::chars_in_multibyte(&bytes) as i64;
                } else {
                    it.current.pos = TextPos::new(charpos, charpos);
                    it.end_charpos = bytes.len() as i64;
                }
                it.string_nchars = it.end_charpos;
                if it.bidi_p {
                    it.bidi_it.string = BidiStringData {
                        lstring: Value::NIL,
                        s: Some(bytes.clone()),
                        schars: it.end_charpos,
                        bufpos: 0,
                        from_disp_str: false,
                        unibyte: !it.multibyte_p,
                    };
                    it.bidi_it.w = it.window;
                    let b = it.bytepos();
                    self.bidi_init_it(charpos, b, fwp, &mut it.bidi_it)?;
                }
                it.s = Some(bytes);
                it.method = ItMethod::FromCString;
            }
        }
        if precision > 0 && it.end_charpos - charpos > precision {
            it.end_charpos = charpos + precision;
            it.string_nchars = it.end_charpos;
            if it.bidi_p {
                it.bidi_it.string.schars = it.end_charpos;
            }
        }
        let field_width = if field_width < 0 {
            DISP_INFINITY
        } else {
            field_width
        };
        if field_width > it.end_charpos - charpos {
            it.end_charpos = charpos + field_width;
        }
        let standard = self.find_symbol_value(sym::STANDARD_DISPLAY_TABLE);
        if self.disp_table_p(standard) {
            it.dp = standard;
        }
        it.stop_charpos = charpos;
        it.prev_stop = charpos;
        it.base_level_stop = 0;
        if it.bidi_p {
            it.bidi_it.first_elt = true;
            it.bidi_it.paragraph_dir = BidiDir::NeutralDir;
            it.bidi_it.disp_pos = -1;
        }
        if !has_s && it.multibyte_p {
            let mut endpos = self.string_chars(it.string) as i64;
            if endpos > it.end_charpos {
                endpos = it.end_charpos;
            }
            let string = it.string;
            self.composition_compute_stop_pos(&mut it.cmp_it, charpos, endpos, string, true)?;
        }
        Ok(())
    }

    pub(super) fn char_composed_p(
        &mut self,
        it: &mut It,
        charpos: i64,
        end_charpos: i64,
    ) -> LispResult<bool> {
        if it.cmp_it.id >= 0 {
            return Ok(true);
        }
        if it.cmp_it.stop_pos != charpos {
            return Ok(false);
        }
        let level = if it.bidi_p {
            i32::from(it.bidi_it.resolved_level)
        } else {
            -1
        };
        let (w, string) = (it.window, it.string);
        let face = usize::try_from(it.face_id)
            .ok()
            .filter(|&id| self.face_from_id(it.f, id).is_some());
        self.composition_reseat_it(&mut it.cmp_it, charpos, end_charpos, w, level, face, string)
    }

    pub fn lookup_glyphless_char_display(&self, c: i32, it: &mut It) -> Value {
        let mut glyphless_method = Value::NIL;
        let table = self.find_symbol_value(sym::GLYPHLESS_CHAR_DISPLAY);
        if self.is_char_table(table) && !self.char_table(table).extras.is_empty() {
            glyphless_method = if c >= 0 {
                self.char_table_ref(table, c as u32)
            } else {
                self.char_table(table).extras[0]
            };
            if glyphless_method.is_cons() {
                glyphless_method = if self.frame_window_p(it.f) {
                    self.xcar(glyphless_method)
                } else {
                    self.xcdr(glyphless_method)
                };
            }
        }
        loop {
            if glyphless_method.is_nil() {
                if c >= 0 {
                    return Value::NIL;
                }
                glyphless_method = sym::EMPTY_BOX;
            }
            if glyphless_method == sym::ZERO_WIDTH {
                if c >= 0 {
                    return glyphless_method;
                }
                glyphless_method = sym::EMPTY_BOX;
            }
            if glyphless_method == sym::THIN_SPACE {
                it.glyphless_method = GlyphlessMethod::ThinSpace;
            } else if glyphless_method == sym::EMPTY_BOX {
                it.glyphless_method = GlyphlessMethod::EmptyBox;
            } else if glyphless_method == sym::HEX_CODE {
                it.glyphless_method = GlyphlessMethod::HexCode;
            } else if glyphless_method.is_string() {
                it.glyphless_method = GlyphlessMethod::Acronym;
            } else {
                glyphless_method = Value::NIL;
                continue;
            }
            it.what = DisplayElementType::Glyphless;
            return glyphless_method;
        }
    }

    pub(super) fn merge_escape_glyph_face(&mut self, it: &It) -> LispResult<i32> {
        let x = &self.xdisp;
        if it.f == x.last_escape_glyph_frame && i64::from(it.face_id) == x.last_escape_glyph_face_id
        {
            return Ok(x.last_escape_glyph_merged_face_id);
        }
        let face_id =
            self.merge_faces(it.window, sym::ESCAPE_GLYPH, 0, it.face_id as usize)? as i32;
        self.xdisp.last_escape_glyph_frame = it.f;
        self.xdisp.last_escape_glyph_face_id = i64::from(it.face_id);
        self.xdisp.last_escape_glyph_merged_face_id = face_id;
        Ok(face_id)
    }

    pub fn merge_glyphless_glyph_face(&mut self, it: &It) -> LispResult<i32> {
        let x = &self.xdisp;
        if it.f == x.last_glyphless_glyph_frame
            && i64::from(it.face_id) == x.last_glyphless_glyph_face_id
        {
            return Ok(x.last_glyphless_glyph_merged_face_id);
        }
        let face_id =
            self.merge_faces(it.window, sym::GLYPHLESS_CHAR, 0, it.face_id as usize)? as i32;
        self.xdisp.last_glyphless_glyph_frame = it.f;
        self.xdisp.last_glyphless_glyph_face_id = i64::from(it.face_id);
        self.xdisp.last_glyphless_glyph_merged_face_id = face_id;
        Ok(face_id)
    }

    pub fn forget_escape_and_glyphless_faces(&mut self) {
        self.xdisp.last_escape_glyph_frame = Value::NIL;
        self.xdisp.last_escape_glyph_face_id = 1 << FACE_ID_BITS;
        self.xdisp.last_glyphless_glyph_frame = Value::NIL;
        self.xdisp.last_glyphless_glyph_face_id = 1 << FACE_ID_BITS;
    }
}
