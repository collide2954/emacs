use super::engine::{
    DetectionInfo, Emit, MAX_ANNOTATION_LENGTH, Src, add_charset_data, category_mask, omb,
};
use super::{
    ATTR_ASCII_COMPAT, ATTR_CHARSET_LIST, ATTR_CHARSET_VALIDS, CAT_CHARSET, CodingResult,
    CodingSystem, MODE_SAFE_ENCODING,
};
use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const INHIBIT_CHARACTER_SUBSTITUTION: i32 = b'?' as i32;

fn byte8_or_ascii(c: i32) -> i32 {
    if (c as u32) < 0x80 { c } else { c + 0x3F_FF00 }
}

impl Lisp {
    pub fn decode_coding_raw_text(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let eol_dos = self.coding_eol_dos(coding);
        coding.chars_at_source = true;
        coding.consumed_char = coding.src_chars;
        coding.consumed = coding.src_bytes;
        if eol_dos && coding.src_bytes > 0 && coding.source[coding.src_bytes - 1] == b'\r' {
            coding.consumed_char -= 1;
            coding.consumed -= 1;
            self.record_conversion_result(coding, CodingResult::InsufficientSrc);
        } else {
            self.record_conversion_result(coding, CodingResult::Success);
        }
        Ok(())
    }

    pub fn encode_coding_raw_text(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let mut dst = std::mem::take(&mut coding.destination);
        let start = dst.len();
        let mut emit = Emit::new(coding.dst_multibyte);
        if emit.multibyte {
            if coding.src_multibyte {
                for &c in &coding.charbuf {
                    let u = c as u32;
                    if u < 0x80 {
                        emit.ascii(&mut dst, c);
                    } else if character::char_byte8_p(u) {
                        emit.one(&mut dst, character::char_to_byte8(u) as i32);
                    } else {
                        for b in character::char_string(u) {
                            emit.one(&mut dst, b as i32);
                        }
                    }
                }
            } else {
                for &c in &coding.charbuf {
                    emit.one(&mut dst, c);
                }
            }
        } else {
            if coding.src_multibyte {
                for &c in &coding.charbuf {
                    let u = c as u32;
                    if u < 0x80 {
                        dst.push(u as u8);
                    } else if character::char_byte8_p(u) {
                        dst.push(character::char_to_byte8(u));
                    } else {
                        character::push_char(&mut dst, u);
                    }
                }
            } else {
                dst.extend(coding.charbuf.iter().map(|&c| c as u8));
            }
            emit.produced_chars = dst.len() - start;
        }
        self.record_conversion_result(coding, CodingResult::Success);
        coding.produced_char += emit.produced_chars;
        coding.produced = dst.len();
        coding.destination = dst;
        Ok(())
    }

    pub fn detect_coding_charset(
        &mut self,
        coding: &mut CodingSystem,
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let source = std::mem::take(&mut coding.source);
        let r = self.detect_coding_charset_1(coding, &source, info);
        coding.source = source;
        r
    }

    fn detect_coding_charset_1(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let mut s = Src::new(source, 0, coding.src_bytes, coding.src_multibyte);
        let mut found = 0;
        let head_ascii = coding.head_ascii.max(0) as usize;
        info.checked |= category_mask(CAT_CHARSET);
        let id = self.codings.categories[CAT_CHARSET].id;
        let attrs = self.coding_id_attrs(id);
        let valids = self.attr(attrs, ATTR_CHARSET_VALIDS);
        let name = self.coding_id_name(id);
        let name = self.symbol_name_string(name);
        let check_latin_extra = name.starts_with("iso-8859-") || name.starts_with("iso-latin-");
        if !self.attr(attrs, ATTR_ASCII_COMPAT).is_nil() {
            s.pos += head_ascii;
        }
        'nms: {
            loop {
                s.base = s.pos;
                let mut c = omb!(s, self, coding, 'nms);
                if c < 0 {
                    continue;
                }
                let mut val = self.vector_items(valids)[c as usize];
                if val.is_nil() {
                    break;
                }
                if c >= 0x80 {
                    if c < 0xA0 && check_latin_extra {
                        let table = self.bound_var(sym::LATIN_EXTRA_CODE_TABLE);
                        if !self.is_vector(table) || self.vector_items(table)[c as usize].is_nil() {
                            break;
                        }
                    }
                    found = category_mask(CAT_CHARSET);
                }
                if let Some(id) = val.as_fixnum() {
                    let cs = self.charset(id as i32).clone();
                    let dim = cs.dimension;
                    let mut idx = 1;
                    while idx < dim {
                        if s.pos == s.end {
                            info.rejected |= category_mask(CAT_CHARSET);
                            return Ok(false);
                        }
                        c = omb!(s, self, coding, 'nms);
                        let k = ((dim - 1 - idx) * 4) as usize;
                        if c < cs.code_space[k] || c > cs.code_space[k + 1] {
                            break;
                        }
                        idx += 1;
                    }
                    if idx < dim {
                        break;
                    }
                } else {
                    let mut idx = 1;
                    while val.is_cons() {
                        let id = self.xcar(val).as_fixnum().unwrap_or(0) as i32;
                        let cs = self.charset(id).clone();
                        let dim = cs.dimension;
                        while idx < dim {
                            if s.pos == s.end {
                                info.rejected |= category_mask(CAT_CHARSET);
                                return Ok(false);
                            }
                            c = omb!(s, self, coding, 'nms);
                            let k = ((dim - 1 - idx) * 4) as usize;
                            if c < cs.code_space[k] || c > cs.code_space[k + 1] {
                                break;
                            }
                            idx += 1;
                        }
                        if idx == dim {
                            val = Value::NIL;
                            break;
                        }
                        val = self.xcdr(val);
                    }
                    if val.is_cons() {
                        break;
                    }
                }
            }
            info.rejected |= category_mask(CAT_CHARSET);
            return Ok(false);
        }
        info.found |= found;
        Ok(true)
    }

    pub fn decode_coding_charset(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let source = std::mem::take(&mut coding.source);
        let r = self.decode_coding_charset_1(coding, &source);
        coding.source = source;
        r
    }

    fn decode_coding_charset_1(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
    ) -> LispResult<()> {
        let mut s = Src::new(
            source,
            coding.consumed,
            coding.src_bytes,
            coding.src_multibyte,
        );
        let charbuf_end = coding.charbuf_size - MAX_ANNOTATION_LENGTH * 2;
        let mut consumed_chars_base;
        let attrs = self.coding_id_attrs(coding.id);
        let valids = self.attr(attrs, ATTR_CHARSET_VALIDS);
        let mut char_offset = coding.produced_char as i32;
        let mut last_offset = char_offset;
        let ascii = self.charsets.ascii;
        let mut last_id = ascii;
        let eol_dos = self.coding_eol_dos(coding);
        let mut byte_after_cr: i32 = -1;
        'nms: {
            loop {
                s.base = s.pos;
                consumed_chars_base = s.consumed_chars;
                if coding.charbuf.len() >= charbuf_end {
                    if byte_after_cr >= 0 {
                        s.base -= 1;
                    }
                    break;
                }
                let mut c = if byte_after_cr >= 0 {
                    let v = byte_after_cr;
                    byte_after_cr = -1;
                    v
                } else {
                    let v = omb!(s, self, coding, 'nms);
                    if eol_dos && v == b'\r' as i32 {
                        byte_after_cr = omb!(s, self, coding, 'nms);
                    }
                    v
                };
                let decoded = 'inv: {
                    if c < 0 {
                        break 'inv None;
                    }
                    let mut code = c as u32;
                    let mut len = 1;
                    let mut val = self.vector_items(valids)[c as usize];
                    if !val.is_fixnum() && !val.is_cons() {
                        break 'inv None;
                    }
                    let charset_id;
                    if let Some(id) = val.as_fixnum() {
                        charset_id = id as i32;
                        let dim = self.charset(charset_id).dimension;
                        while len < dim {
                            c = omb!(s, self, coding, 'nms);
                            code = (code << 8) | (c as u32);
                            len += 1;
                        }
                        c = self.decode_char(charset_id, code)?;
                    } else {
                        let mut id = -1;
                        while val.is_cons() {
                            id = self.xcar(val).as_fixnum().unwrap_or(0) as i32;
                            let dim = self.charset(id).dimension;
                            while len < dim {
                                c = omb!(s, self, coding, 'nms);
                                code = (code << 8) | (c as u32);
                                len += 1;
                            }
                            c = self.decode_char(id, code)?;
                            if c >= 0 {
                                break;
                            }
                            val = self.xcdr(val);
                        }
                        charset_id = id;
                    }
                    if c < 0 {
                        break 'inv None;
                    }
                    Some((c, charset_id))
                };
                match decoded {
                    Some((ch, charset_id)) => {
                        if charset_id != ascii && last_id != charset_id {
                            if last_id != ascii {
                                add_charset_data(coding, char_offset - last_offset, last_id);
                            }
                            last_id = charset_id;
                            last_offset = char_offset;
                        }
                        coding.charbuf.push(ch);
                        char_offset += 1;
                    }
                    None => {
                        s.pos = s.base;
                        s.consumed_chars = consumed_chars_base;
                        let c = omb!(s, self, coding, 'nms);
                        coding
                            .charbuf
                            .push(if c < 0 { -c } else { byte8_or_ascii(c) });
                        char_offset += 1;
                    }
                }
            }
        }
        if last_id != ascii {
            add_charset_data(coding, char_offset - last_offset, last_id);
        }
        coding.consumed_char += consumed_chars_base;
        coding.consumed = s.base;
        Ok(())
    }

    pub fn encode_coding_charset(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let attrs = self.coding_id_attrs(coding.id);
        let charset_list = self.attr(attrs, ATTR_CHARSET_LIST);
        let ascii_compatible = !self.attr(attrs, ATTR_ASCII_COMPAT).is_nil();
        let mut dst = std::mem::take(&mut coding.destination);
        let mut emit = Emit::new(coding.dst_multibyte);
        let charbuf = std::mem::take(&mut coding.charbuf);
        for &c in &charbuf {
            let u = c as u32;
            if ascii_compatible && u < 0x80 {
                emit.ascii(&mut dst, c);
            } else if character::char_byte8_p(u) {
                emit.one(&mut dst, character::char_to_byte8(u) as i32);
            } else {
                match self.char_charset(c, charset_list)? {
                    Some((id, code)) => {
                        let code = code as i32;
                        match self.charset(id).dimension {
                            1 => emit.one(&mut dst, code),
                            2 => emit.two(&mut dst, code >> 8, code & 0xFF),
                            3 => emit.three(&mut dst, code >> 16, (code >> 8) & 0xFF, code & 0xFF),
                            _ => emit.four(
                                &mut dst,
                                (code >> 24) & 0xFF,
                                (code >> 16) & 0xFF,
                                (code >> 8) & 0xFF,
                                code & 0xFF,
                            ),
                        }
                    }
                    None => {
                        let c = if coding.mode & MODE_SAFE_ENCODING != 0 {
                            INHIBIT_CHARACTER_SUBSTITUTION
                        } else {
                            coding.default_char as i32
                        };
                        emit.one(&mut dst, c);
                    }
                }
            }
        }
        coding.charbuf = charbuf;
        self.record_conversion_result(coding, CodingResult::Success);
        coding.produced_char += emit.produced_chars;
        coding.produced = dst.len();
        coding.destination = dst;
        Ok(())
    }
}
