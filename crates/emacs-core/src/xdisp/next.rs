use crate::bidi::BidiType;
use crate::character;
use crate::dispextern::{
    DisplayElementType, DpVec, GlyphRowArea, GlyphlessMethod, It, ItMethod,
    TEXT_PROP_DISTANCE_LIMIT,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const DISP_ESCAPE_GLYPH_SLOT: usize = 2;
const DISP_CTRL_GLYPH_SLOT: usize = 3;
const CHARACTERBITS: u32 = 22;
const SOFT_HYPHEN: i32 = 0xAD;
const HYPHEN: i32 = 0x2010;
const NON_BREAKING_HYPHEN: i32 = 0x2011;

impl Lisp {
    pub fn glyph_code_face(&self, gc: Value) -> i32 {
        if gc.is_cons() {
            self.xcdr(gc).as_fixnum().unwrap_or(0) as i32
        } else {
            (gc.as_fixnum().unwrap_or(0) >> CHARACTERBITS) as i32
        }
    }

    pub fn get_next_display_element(&mut self, it: &mut It) -> LispResult<bool> {
        let mut success_p;
        'get_next: loop {
            success_p = self.get_next_element_by_method(it)?;
            let mut skip_face_adjust = false;
            if it.what == DisplayElementType::Character {
                if it.bidi_p
                    && it.bidi_it.type_ == BidiType::StrongR
                    && !self.var_true(sym::INHIBIT_BIDI_MIRRORING)
                {
                    it.c = self.bidi_mirror_char(it.c)?;
                }
                if success_p && it.dpvec == DpVec::None {
                    let mut nonascii_space_p = false;
                    let mut nonascii_hyphen_p = false;
                    let mut c = it.c;
                    if !it.multibyte_p && !character::ascii_char_p(c as u32) {
                        if self.var_true(sym::UNIBYTE_DISPLAY_VIA_LANGUAGE_ENVIRONMENT) {
                            let unibyte = self.charsets.unibyte;
                            c = self.decode_char(unibyte, c as u32)?;
                            if c < 0 {
                                c = character::byte8_to_char(it.c as u8) as i32;
                            }
                        } else {
                            c = character::byte8_to_char(it.c as u8) as i32;
                        }
                    }
                    if !it.dp.is_nil() {
                        let dv = self.disp_char_vector(it.dp, c as u32);
                        if self.is_vector(dv) {
                            let size = self.vector_items(dv).len();
                            if size != 0 {
                                it.dpvec_char_len = it.len;
                                it.dpvec = DpVec::Lisp(dv);
                                it.dpend = size as i64;
                                it.current.dpvec_index = 0;
                                it.dpvec_face_id = -1;
                                it.saved_face_id = it.face_id;
                                it.method = ItMethod::FromDisplayVector;
                                it.ellipsis_p = false;
                            } else {
                                self.set_iterator_to_next(it, false)?;
                            }
                            continue 'get_next;
                        }
                    }
                    if !self.lookup_glyphless_char_display(c, it).is_nil() {
                        if it.what == DisplayElementType::Glyphless {
                            skip_face_adjust = true;
                        } else {
                            self.set_iterator_to_next(it, false)?;
                            continue 'get_next;
                        }
                    }
                    if !skip_face_adjust {
                        let nobreak = self.find_symbol_value(sym::NOBREAK_CHAR_DISPLAY);
                        if !character::ascii_char_p(c as u32) && !nobreak.is_nil() {
                            if self.blankp(c as u32) {
                                nonascii_space_p = true;
                            } else if c == SOFT_HYPHEN || c == HYPHEN || c == NON_BREAKING_HYPHEN {
                                nonascii_hyphen_p = true;
                            }
                        }
                        let is_control = if c < 32 || c == 127 {
                            it.area != GlyphRowArea::Text
                                || (c != 9
                                    && it.glyph_row.is_some()
                                    && (self.glyph_row_mode_line_p(it) || it.avoid_cursor_p))
                                || (c != 10 && c != 9)
                        } else {
                            nonascii_space_p
                                || nonascii_hyphen_p
                                || character::char_byte8_p(c as u32)
                                || !self.char_printable_p(c as u32)
                        };
                        if is_control {
                            let mut lface_id = 0;
                            let face_id;
                            let ctl_len;
                            if character::ascii_char_p(c as u32) && it.ctl_arrow_p {
                                let mut g = i32::from(b'^');
                                if !it.dp.is_nil() {
                                    let gc = self.char_table(it.dp).extras[DISP_CTRL_GLYPH_SLOT];
                                    if self.glyph_code_p(gc) {
                                        g = self.glyph_code_char(gc) as i32;
                                        lface_id = self.glyph_code_face(gc);
                                    }
                                }
                                face_id = if lface_id != 0 {
                                    self.merge_faces(
                                        it.window,
                                        sym::T,
                                        i64::from(lface_id),
                                        it.face_id as usize,
                                    )? as i32
                                } else {
                                    self.merge_escape_glyph_face(it)?
                                };
                                it.ctl_chars[0] = Value::fixnum(i64::from(g));
                                it.ctl_chars[1] = Value::fixnum(i64::from(c ^ 0o100));
                                ctl_len = 2;
                            } else if nonascii_space_p && nobreak == sym::T {
                                face_id = self.merge_faces(
                                    it.window,
                                    sym::NOBREAK_SPACE,
                                    0,
                                    it.face_id as usize,
                                )? as i32;
                                let ch = if self.var_true(sym::NOBREAK_CHAR_ASCII_DISPLAY) {
                                    32
                                } else {
                                    it.c
                                };
                                it.ctl_chars[0] = Value::fixnum(i64::from(ch));
                                ctl_len = 1;
                            } else if nonascii_hyphen_p && nobreak == sym::T {
                                face_id = self.merge_faces(
                                    it.window,
                                    sym::NOBREAK_HYPHEN,
                                    0,
                                    it.face_id as usize,
                                )? as i32;
                                let ch = if self.var_true(sym::NOBREAK_CHAR_ASCII_DISPLAY) {
                                    i32::from(b'-')
                                } else {
                                    it.c
                                };
                                it.ctl_chars[0] = Value::fixnum(i64::from(ch));
                                ctl_len = 1;
                            } else {
                                let mut escape_glyph = i32::from(b'\\');
                                if !it.dp.is_nil() {
                                    let gc = self.char_table(it.dp).extras[DISP_ESCAPE_GLYPH_SLOT];
                                    if self.glyph_code_p(gc) {
                                        escape_glyph = self.glyph_code_char(gc) as i32;
                                        lface_id = self.glyph_code_face(gc);
                                    }
                                }
                                face_id = if lface_id != 0 {
                                    self.merge_faces(
                                        it.window,
                                        sym::T,
                                        i64::from(lface_id),
                                        it.face_id as usize,
                                    )? as i32
                                } else {
                                    self.merge_escape_glyph_face(it)?
                                };
                                if nonascii_space_p || nonascii_hyphen_p {
                                    it.ctl_chars[0] = Value::fixnum(i64::from(escape_glyph));
                                    it.ctl_chars[1] =
                                        Value::fixnum(if nonascii_space_p { 32 } else { 45 });
                                    ctl_len = 2;
                                } else {
                                    if character::char_byte8_p(c as u32) {
                                        c = i32::from(character::char_to_byte8(c as u32));
                                    }
                                    let text = if self.var_true(sym::DISPLAY_RAW_BYTES_AS_HEX) {
                                        format!("x{c:02x}")
                                    } else {
                                        format!("{c:03o}")
                                    };
                                    it.ctl_chars[0] = Value::fixnum(i64::from(escape_glyph));
                                    for (i, b) in text.bytes().enumerate() {
                                        it.ctl_chars[i + 1] = Value::fixnum(i64::from(b));
                                    }
                                    ctl_len = text.len() as i64 + 1;
                                }
                            }
                            it.dpvec_char_len = it.len;
                            it.dpvec = DpVec::CtlChars;
                            it.dpend = ctl_len;
                            it.current.dpvec_index = 0;
                            it.dpvec_face_id = face_id;
                            it.saved_face_id = it.face_id;
                            it.method = ItMethod::FromDisplayVector;
                            it.ellipsis_p = false;
                            continue 'get_next;
                        }
                        it.char_to_display = c;
                    }
                } else if success_p {
                    it.char_to_display = it.c;
                }
            }
            if !skip_face_adjust
                && (it.what == DisplayElementType::Character
                    || it.what == DisplayElementType::Composition)
                && it.multibyte_p
                && success_p
                && self.frame_window_p(it.f)
            {
                let face_exists =
                    it.face_id >= 0 && self.face_from_id(it.f, it.face_id as usize).is_some();
                if !face_exists {
                    it.what = DisplayElementType::Glyphless;
                    it.glyphless_method = GlyphlessMethod::EmptyBox;
                } else if it.what == DisplayElementType::Composition && it.cmp_it.ch >= 0 {
                    let gstring = self.composition_gstring_from_id(it.cmp_it.id as usize);
                    let font = self.lgstring_font(gstring);
                    it.face_id = self.face_for_font(it.f, font, it.face_id as usize)? as i32;
                } else {
                    let pos = if it.s.is_some() {
                        -1
                    } else if it.string.is_string() {
                        it.string_charpos()
                    } else {
                        it.charpos()
                    };
                    let c = if it.what == DisplayElementType::Character {
                        it.char_to_display
                    } else {
                        let cmp = &self.compositions.table[it.cmp_it.id as usize];
                        let altchars =
                            cmp.method == crate::composite::CompositionMethod::WithRuleAltchars;
                        let (key, glyph_len) = (cmp.key, cmp.glyph_len);
                        let mut c = 32;
                        for i in 0..glyph_len {
                            let idx = if altchars { i * 2 } else { i };
                            c = self.aref_items(key)[idx].as_fixnum().unwrap_or(0) as i32;
                            if c != 9 {
                                break;
                            }
                        }
                        c
                    };
                    it.face_id =
                        self.face_for_char(it.f, it.face_id as usize, c as u32, pos, it.string)?
                            as i32;
                }
            }
            break;
        }
        if it.face_box_p && it.s.is_none() {
            if it.method == ItMethod::FromString && it.sp != 0 {
                let face_id = underlying_face_id_of(it);
                if face_id >= 0
                    && let Some(face_box) = self.face_box_state(it.f, face_id)
                {
                    if !face_box {
                        let string_face_id = self.face_before_or_after_it_pos(it, false)?;
                        if self.face_box_state(it.f, string_face_id) == Some(false) {
                            it.end_of_box_run_p = true;
                        }
                    } else {
                        let schars = self.string_chars(it.string) as i64;
                        if (it.string_charpos() >= schars - 1
                            || (it.what == DisplayElementType::Composition
                                && it.string_charpos() + it.cmp_it.nchars >= schars))
                            && ((it.current.overlay_string_index >= 0
                                && it.current.overlay_string_index == it.n_overlay_strings - 1)
                                || it.from_disp_prop_p)
                        {
                            let mut text_from_string = false;
                            let mut pos = it.current.pos;
                            if it.from_disp_prop_p {
                                let mut stackp = it.sp - 1;
                                while stackp >= 0 && it.stack[stackp as usize].string.is_string() {
                                    stackp -= 1;
                                }
                                if stackp < 0 {
                                    text_from_string = true;
                                    pos = it.stack[(it.sp - 1) as usize].position;
                                } else {
                                    pos = it.stack[stackp as usize].position;
                                }
                            } else {
                                pos = self.inc_text_pos(pos, it.multibyte_p);
                            }
                            if text_from_string {
                                let base_string = it.stack[(it.sp - 1) as usize].string;
                                if pos.charpos >= self.string_chars(base_string) as i64 - 1 {
                                    it.end_of_box_run_p = true;
                                } else {
                                    let next_face_id = self
                                        .face_at_string_position(
                                            it.window,
                                            base_string,
                                            pos.charpos,
                                            0,
                                            face_id as usize,
                                            false,
                                            0,
                                        )?
                                        .0;
                                    if self.face_box_state(it.f, next_face_id as i32) == Some(false)
                                    {
                                        it.end_of_box_run_p = true;
                                    }
                                }
                            } else if pos.charpos >= self.cur().zv as i64 {
                                it.end_of_box_run_p = true;
                            } else {
                                let (next_face_id, _) = self.face_at_buffer_position(
                                    it.window,
                                    pos.charpos as usize,
                                    (pos.charpos + TEXT_PROP_DISTANCE_LIMIT) as usize,
                                    false,
                                    None,
                                    0,
                                )?;
                                if self.face_box_state(it.f, next_face_id as i32) == Some(false) {
                                    it.end_of_box_run_p = true;
                                }
                            }
                        }
                    }
                }
            } else if it.method != ItMethod::FromDisplayVector {
                let face_id = self.face_before_or_after_it_pos(it, false)?;
                if face_id != it.face_id && self.face_box_state(it.f, face_id) == Some(false) {
                    it.end_of_box_run_p = true;
                }
            }
        }
        if !success_p && it.sp > 0 {
            self.set_iterator_to_next(it, false)?;
            success_p = self.get_next_display_element(it)?;
        }
        Ok(success_p)
    }

    fn face_box_state(&self, f: Value, face_id: i32) -> Option<bool> {
        if face_id < 0 {
            return None;
        }
        self.face_from_id(f, face_id as usize)
            .map(|face| face.box_type != crate::xfaces::FaceBoxType::NoBox)
    }

    fn glyph_row_mode_line_p(&self, it: &It) -> bool {
        match it.glyph_row {
            Some(row) => self.glyph_row_ref(row).mode_line_p,
            None => false,
        }
    }

    pub(super) fn get_next_element_by_method(&mut self, it: &mut It) -> LispResult<bool> {
        match it.method {
            ItMethod::FromBuffer => self.next_element_from_buffer(it),
            ItMethod::FromDisplayVector => self.next_element_from_display_vector(it),
            ItMethod::FromString => self.next_element_from_string(it),
            ItMethod::FromCString => self.next_element_from_c_string(it),
            ItMethod::FromImage => self.next_element_from_image(it),
            ItMethod::FromStretch => self.next_element_from_stretch(it),
            ItMethod::FromXwidget => unreachable!("xwidgets are not supported"),
        }
    }

    pub(super) fn dpvec_ref(&self, it: &It, idx: i64) -> Value {
        match it.dpvec {
            DpVec::Lisp(v) => self.vector_items(v)[idx as usize],
            DpVec::CtlChars => it.ctl_chars[idx as usize],
            DpVec::None => Value::NIL,
        }
    }
}

pub(super) fn underlying_face_id_of(it: &It) -> i32 {
    let mut face_id = it.base_face_id;
    let mut i = it.sp - 1;
    while i >= 0 {
        if it.stack[i as usize].string.is_nil() {
            face_id = it.stack[i as usize].face_id;
        }
        i -= 1;
    }
    face_id
}
