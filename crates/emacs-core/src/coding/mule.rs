use super::engine::{
    DetectionInfo, Emit, MAX_ANNOTATION_LENGTH, Src, add_charset_data, add_composition_data,
    category_mask, omb,
};
use super::{
    ANNOTATE_CHARSET_MASK, ATTR_CHARSET_LIST, CAT_EMACS_MULE, COMPOSITION_RELATIVE,
    COMPOSITION_WITH_RULE, COMPOSITION_WITH_RULE_ALTCHARS, CodingResult, CodingSystem,
    ComposingState, CompositionStatus, MAX_COMPOSITION_COMPONENTS, MODE_LAST_BLOCK,
};
use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::value::Value;

const LEADING_CODE_PRIVATE_11: i32 = 0x9A;
const LEADING_CODE_PRIVATE_12: i32 = 0x9B;
const ISO_CODE_SO: i32 = 0x0E;
const ISO_CODE_SI: i32 = 0x0F;
const ISO_CODE_ESC: i32 = 0x1B;

fn byte8_to_char(c: i32) -> i32 {
    c + 0x3F_FF00
}

fn ascii_or_byte8(c: i32) -> i32 {
    if (c as u32) < 0x80 {
        c
    } else {
        byte8_to_char(c)
    }
}

fn composition_encode_rule(gref: i32, nref: i32) -> i32 {
    gref * 12 + nref
}

fn component_state(state: ComposingState) -> bool {
    matches!(
        state,
        ComposingState::ComponentChar | ComposingState::ComponentRule
    )
}

fn composition_end(charbuf: &mut [i32], cmp: &mut CompositionStatus) {
    let idx = charbuf.len() - cmp.length as usize;
    if cmp.old_form {
        charbuf[idx + 2] = cmp.nchars;
    } else if cmp.method > COMPOSITION_RELATIVE {
        charbuf[idx] = charbuf[idx + 2] - cmp.length;
    }
    cmp.state = ComposingState::No;
}

fn finish_composition(charbuf: &mut [i32], cmp: &mut CompositionStatus) -> i32 {
    let len = charbuf.len();
    let mut idx = len - cmp.length as usize;
    let new_chars;
    if cmp.old_form && cmp.nchars > 0 {
        charbuf[idx + 2] = cmp.nchars;
        if cmp.method == COMPOSITION_WITH_RULE && cmp.state == ComposingState::Char {
            let rule = charbuf[len - 1] + 0xA0;
            charbuf[len - 2] = byte8_to_char(rule);
            charbuf[len - 1] = -1;
            new_chars = 1;
        } else {
            new_chars = 0;
        }
    } else {
        charbuf[idx] = byte8_to_char(0x80);
        idx += 1;
        if cmp.method == COMPOSITION_WITH_RULE {
            charbuf[idx] = byte8_to_char(0xFF);
            charbuf[idx + 1] = -3;
            charbuf[idx + 2] = 0;
            new_chars = 1;
        } else {
            let nchars = charbuf[idx + 1] + 0xA0;
            let nbytes = charbuf[idx + 2] + 0xA0;
            charbuf[idx] = byte8_to_char(0xF2 + cmp.method);
            charbuf[idx + 1] = byte8_to_char(nbytes);
            charbuf[idx + 2] = byte8_to_char(nchars);
            charbuf[idx + 3] = -1;
            new_chars = 4;
        }
    }
    cmp.state = ComposingState::No;
    new_chars
}

fn maybe_finish(charbuf: &mut [i32], cmp: &mut CompositionStatus, char_offset: &mut i32) {
    if cmp.state != ComposingState::No {
        *char_offset += finish_composition(charbuf, cmp);
    }
}

fn leading_codes(id: i32) -> (i32, i32) {
    if id < 0xA0 {
        (id, 0)
    } else if id < 0xE0 {
        (0x9A, id)
    } else if id < 0xF0 {
        (0x9B, id)
    } else if id < 0xF5 {
        (0x9C, id)
    } else {
        (0x9D, id)
    }
}

struct MuleChar {
    c: i32,
    nbytes: usize,
    nchars: usize,
    id: i32,
}

impl Lisp {
    pub fn detect_coding_emacs_mule(
        &mut self,
        coding: &mut CodingSystem,
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let source = std::mem::take(&mut coding.source);
        let r = self.detect_coding_emacs_mule_1(coding, &source, info);
        coding.source = source;
        r
    }

    fn detect_coding_emacs_mule_1(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let mask = category_mask(CAT_EMACS_MULE);
        let mut s = Src::new(source, 0, coding.src_bytes, coding.src_multibyte);
        let mut found = 0;
        info.checked |= mask;
        s.pos += coding.head_ascii.max(0) as usize;
        'nms: {
            loop {
                s.base = s.pos;
                let mut c = omb!(s, self, coding, 'nms);
                if c < 0 {
                    continue;
                }
                if c == 0x80 {
                    let mut rejected = false;
                    loop {
                        let start = s.pos;
                        loop {
                            c = omb!(s, self, coding, 'nms);
                            if c < 0xA0 {
                                break;
                            }
                        }
                        if s.pos - start <= 4 {
                            rejected = true;
                            break;
                        }
                        found = mask;
                        if c != 0x80 {
                            break;
                        }
                    }
                    if rejected {
                        break;
                    }
                }
                if c < 0x80 {
                    if c < 0x20 && (c == ISO_CODE_ESC || c == ISO_CODE_SI || c == ISO_CODE_SO) {
                        break;
                    }
                } else {
                    let mut more_bytes = self.charsets.emacs_mule_bytes[c as usize] - 1;
                    while more_bytes > 0 {
                        c = omb!(s, self, coding, 'nms);
                        if c < 0xA0 {
                            s.pos -= 1;
                            break;
                        }
                        more_bytes -= 1;
                    }
                    if more_bytes != 0 {
                        break;
                    }
                    found = mask;
                }
            }
            info.rejected |= mask;
            return Ok(false);
        }
        if s.base < s.pos && coding.mode & MODE_LAST_BLOCK != 0 {
            info.rejected |= mask;
            return Ok(false);
        }
        info.found |= found;
        Ok(true)
    }

    fn emacs_mule_char(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
        at: usize,
        state: ComposingState,
        old_form: bool,
    ) -> LispResult<MuleChar> {
        let mut s = Src::new(source, at, coding.src_bytes, coding.src_multibyte);
        let mut mseq_found = false;
        let mut charset_id = -1;
        let mut c: i32;
        let invalid = MuleChar {
            c: -1,
            nbytes: 0,
            nchars: 0,
            id: -1,
        };
        'nms: {
            c = omb!(s, self, coding, 'nms);
            if c < 0 {
                c = -c;
                charset_id = self.charsets.emacs_mule_charset[0];
            } else {
                if c >= 0xA0 {
                    if state != ComposingState::No && old_form {
                        if state == ComposingState::Char {
                            if c == 0xA0 {
                                c = omb!(s, self, coding, 'nms);
                                c -= 0x80;
                                if c < 0 {
                                    return Ok(invalid);
                                }
                            } else {
                                c -= 0x20;
                            }
                            mseq_found = true;
                        } else {
                            return Ok(MuleChar {
                                c: -c,
                                nbytes: s.pos - at,
                                nchars: s.consumed_chars,
                                id: charset_id,
                            });
                        }
                    } else {
                        return Ok(invalid);
                    }
                }
                let code: u32;
                match self.charsets.emacs_mule_bytes[c as usize] {
                    2 => {
                        charset_id = self.charsets.emacs_mule_charset[c as usize];
                        if charset_id < 0 {
                            return Ok(invalid);
                        }
                        c = omb!(s, self, coding, 'nms);
                        if c < 0xA0 {
                            return Ok(invalid);
                        }
                        code = (c & 0x7F) as u32;
                    }
                    3 => {
                        if c == LEADING_CODE_PRIVATE_11 || c == LEADING_CODE_PRIVATE_12 {
                            c = omb!(s, self, coding, 'nms);
                            if c < 0xA0 {
                                return Ok(invalid);
                            }
                            charset_id = self.charsets.emacs_mule_charset[c as usize];
                            if charset_id < 0 {
                                return Ok(invalid);
                            }
                            c = omb!(s, self, coding, 'nms);
                            if c < 0xA0 {
                                return Ok(invalid);
                            }
                            code = (c & 0x7F) as u32;
                        } else {
                            charset_id = self.charsets.emacs_mule_charset[c as usize];
                            if charset_id < 0 {
                                return Ok(invalid);
                            }
                            c = omb!(s, self, coding, 'nms);
                            if c < 0xA0 {
                                return Ok(invalid);
                            }
                            let hi = ((c & 0x7F) as u32) << 8;
                            c = omb!(s, self, coding, 'nms);
                            if c < 0xA0 {
                                return Ok(invalid);
                            }
                            code = hi | (c & 0x7F) as u32;
                        }
                    }
                    4 => {
                        c = omb!(s, self, coding, 'nms);
                        if c < 0 {
                            return Ok(invalid);
                        }
                        charset_id = self.charsets.emacs_mule_charset[c as usize];
                        if charset_id < 0 {
                            return Ok(invalid);
                        }
                        c = omb!(s, self, coding, 'nms);
                        if c < 0xA0 {
                            return Ok(invalid);
                        }
                        let hi = ((c & 0x7F) as u32) << 8;
                        c = omb!(s, self, coding, 'nms);
                        if c < 0xA0 {
                            return Ok(invalid);
                        }
                        code = hi | (c & 0x7F) as u32;
                    }
                    1 => {
                        code = c as u32;
                        charset_id = if code < 0x80 {
                            self.charsets.ascii
                        } else {
                            self.charsets.eight_bit
                        };
                    }
                    _ => return Ok(invalid),
                }
                c = self.decode_char(charset_id, code)?;
                if c < 0 {
                    return Ok(invalid);
                }
            }
            return Ok(MuleChar {
                c: if mseq_found { -c } else { c },
                nbytes: s.pos - at,
                nchars: s.consumed_chars,
                id: charset_id,
            });
        }
        Ok(MuleChar {
            c: -2,
            nbytes: 0,
            nchars: 0,
            id: -1,
        })
    }

    pub fn decode_coding_emacs_mule(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let source = std::mem::take(&mut coding.source);
        let mut cmp = std::mem::take(&mut coding.cmp_status);
        let r = self.decode_coding_emacs_mule_1(coding, &source, &mut cmp);
        coding.cmp_status = cmp;
        coding.source = source;
        r
    }

    fn decode_coding_emacs_mule_1(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
        cmp: &mut CompositionStatus,
    ) -> LispResult<()> {
        let mut s = Src::new(
            source,
            coding.consumed,
            coding.src_bytes,
            coding.src_multibyte,
        );
        let charbuf_end = coding.charbuf_size - MAX_ANNOTATION_LENGTH * 3 - 1;
        let mut consumed_chars_base;
        let ascii = self.charsets.ascii;
        let mut char_offset = coding.produced_char as i32;
        let mut last_offset = char_offset;
        let mut last_id = ascii;
        let eol_dos = self.coding_eol_dos(coding);
        let mut byte_after_cr: i32 = -1;
        if cmp.state != ComposingState::No {
            let n = cmp.length as usize;
            coding.charbuf.extend_from_slice(&cmp.carryover[..n]);
            coding.annotated = true;
        }
        'nms: {
            'top: loop {
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
                    omb!(s, self, coding, 'nms)
                };
                let invalid = 'inv: {
                    if c < 0 || c == 0x80 {
                        maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                        if c < 0 {
                            coding.charbuf.push(-c);
                            char_offset += 1;
                        } else {
                            let current_src = s.pos;
                            c = omb!(s, self, coding, 'nms);
                            if c < 0 {
                                break 'inv true;
                            }
                            let method = c - 0xF2;
                            if (COMPOSITION_RELATIVE..=COMPOSITION_WITH_RULE_ALTCHARS)
                                .contains(&method)
                            {
                                c = omb!(s, self, coding, 'nms);
                                if c < 0 {
                                    break 'inv true;
                                }
                                let nbytes = c - 0xA0;
                                if nbytes < 3 || (method == COMPOSITION_RELATIVE && nbytes != 4) {
                                    break 'inv true;
                                }
                                c = omb!(s, self, coding, 'nms);
                                let nchars = c - 0xA0;
                                if nchars <= 0 || nchars >= MAX_COMPOSITION_COMPONENTS as i32 {
                                    break 'inv true;
                                }
                                cmp.old_form = false;
                                cmp.method = method;
                                cmp.state = if method == COMPOSITION_RELATIVE {
                                    ComposingState::Char
                                } else {
                                    ComposingState::ComponentChar
                                };
                                cmp.length = MAX_ANNOTATION_LENGTH as i32;
                                cmp.nchars = nchars;
                                cmp.ncomps = nbytes - 4;
                                add_composition_data(coding, nchars, nbytes, method);
                            } else if c < 0xA0 {
                                break 'inv true;
                            } else if c < 0xC0 || c == 0xFF {
                                cmp.old_form = true;
                                cmp.method = if c == 0xFF {
                                    COMPOSITION_WITH_RULE
                                } else {
                                    COMPOSITION_RELATIVE
                                };
                                cmp.state = ComposingState::Char;
                                cmp.length = MAX_ANNOTATION_LENGTH as i32;
                                cmp.nchars = 0;
                                cmp.ncomps = 0;
                                add_composition_data(coding, 0, 0, cmp.method);
                                if c < 0xC0 {
                                    s.pos = current_src;
                                }
                            } else {
                                break 'inv true;
                            }
                        }
                        continue 'top;
                    }
                    let id;
                    if c < 0x80 {
                        if eol_dos && c == b'\r' as i32 {
                            byte_after_cr = omb!(s, self, coding, 'nms);
                        }
                        id = ascii;
                        if cmp.state != ComposingState::No {
                            if cmp.old_form {
                                maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                            } else if component_state(cmp.state) {
                                cmp.ncomps -= 1;
                            }
                        }
                    } else {
                        let m =
                            self.emacs_mule_char(coding, source, s.base, cmp.state, cmp.old_form)?;
                        c = m.c;
                        if c == -1 {
                            break 'inv true;
                        }
                        if c == -2 {
                            break 'nms;
                        }
                        s.pos = s.base + m.nbytes;
                        s.consumed_chars = consumed_chars_base + m.nchars;
                        if component_state(cmp.state) {
                            cmp.ncomps -= m.nchars as i32;
                        }
                        id = m.id;
                    }
                    match cmp.state {
                        ComposingState::No => {
                            if last_id != id {
                                if last_id != ascii {
                                    add_charset_data(coding, char_offset - last_offset, last_id);
                                }
                                last_id = id;
                                last_offset = char_offset;
                            }
                            coding.charbuf.push(c);
                            char_offset += 1;
                        }
                        ComposingState::Char => {
                            if cmp.old_form {
                                if c >= 0 {
                                    maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                                    coding.charbuf.push(c);
                                    char_offset += 1;
                                } else {
                                    coding.charbuf.push(-c);
                                    cmp.nchars += 1;
                                    cmp.length += 1;
                                    if cmp.nchars == MAX_COMPOSITION_COMPONENTS as i32 {
                                        composition_end(&mut coding.charbuf, cmp);
                                    } else if cmp.method == COMPOSITION_WITH_RULE {
                                        cmp.state = ComposingState::Rule;
                                    }
                                }
                            } else {
                                coding.charbuf.push(c);
                                cmp.length += 1;
                                cmp.nchars -= 1;
                                if cmp.nchars == 0 {
                                    composition_end(&mut coding.charbuf, cmp);
                                }
                            }
                        }
                        ComposingState::Rule => {
                            if c >= 0 {
                                composition_end(&mut coding.charbuf, cmp);
                                coding.charbuf.push(c);
                                char_offset += 1;
                            } else {
                                let c = -c - 0xA0;
                                if !(0..81).contains(&c) {
                                    break 'inv true;
                                }
                                let gref = if c / 9 == 4 { 10 } else { c / 9 };
                                let nref = if c % 9 == 4 { 10 } else { c % 9 };
                                coding.charbuf.push(-2);
                                coding.charbuf.push(composition_encode_rule(gref, nref));
                                cmp.length += 2;
                                cmp.state = ComposingState::Char;
                            }
                        }
                        ComposingState::ComponentChar => {
                            coding.charbuf.push(c);
                            cmp.length += 1;
                            if cmp.ncomps == 0 {
                                cmp.state = ComposingState::Char;
                            } else if cmp.ncomps > 0 {
                                if cmp.method == COMPOSITION_WITH_RULE_ALTCHARS {
                                    cmp.state = ComposingState::ComponentRule;
                                }
                            } else {
                                maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                            }
                        }
                        ComposingState::ComponentRule => {
                            let gref = c - 0x20;
                            if !(0..81).contains(&gref) {
                                break 'inv true;
                            }
                            c = omb!(s, self, coding, 'nms);
                            let nref = c - 0x20;
                            if !(0..81).contains(&nref) {
                                break 'inv true;
                            }
                            coding.charbuf.push(-2);
                            coding.charbuf.push(composition_encode_rule(gref, nref));
                            cmp.length += 2;
                            cmp.ncomps -= 1;
                            if cmp.ncomps > 0 {
                                cmp.state = ComposingState::ComponentChar;
                            } else {
                                maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                            }
                        }
                    }
                    false
                };
                if invalid {
                    maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                    s.pos = s.base;
                    s.consumed_chars = consumed_chars_base;
                    let c = omb!(s, self, coding, 'nms);
                    coding.charbuf.push(ascii_or_byte8(c));
                    char_offset += 1;
                }
            }
        }
        if cmp.state != ComposingState::No {
            if coding.mode & MODE_LAST_BLOCK != 0 {
                maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
            } else {
                let n = cmp.length as usize;
                let start = coding.charbuf.len() - n;
                cmp.carryover[..n].copy_from_slice(&coding.charbuf[start..]);
                coding.charbuf.truncate(start);
            }
        }
        if last_id != ascii {
            add_charset_data(coding, char_offset - last_offset, last_id);
        }
        coding.consumed_char += consumed_chars_base;
        coding.consumed = s.base;
        Ok(())
    }

    pub fn encode_coding_emacs_mule(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let charbuf = std::mem::take(&mut coding.charbuf);
        let mut dst = std::mem::take(&mut coding.destination);
        let r = self.encode_coding_emacs_mule_1(coding, &charbuf, &mut dst);
        coding.destination = dst;
        coding.charbuf = charbuf;
        r
    }

    fn encode_coding_emacs_mule_1(
        &mut self,
        coding: &mut CodingSystem,
        charbuf: &[i32],
        dst: &mut Vec<u8>,
    ) -> LispResult<()> {
        let attrs = self.coding_id_attrs(coding.id);
        let mut charset_list = self.attr(attrs, ATTR_CHARSET_LIST);
        if charset_list != self.charsets.emacs_mule_charset_list {
            charset_list = self.charsets.emacs_mule_charset_list;
            self.set_attr(attrs, ATTR_CHARSET_LIST, charset_list);
        }
        let mut emit = Emit::new(coding.dst_multibyte);
        let mut preferred_charset_id = -1;
        let mut i = 0;
        while i < charbuf.len() {
            let mut c = charbuf[i];
            i += 1;
            if c < 0 {
                if charbuf.get(i).copied() == Some(ANNOTATE_CHARSET_MASK as i32) {
                    preferred_charset_id = charbuf.get(i + 3).copied().unwrap_or(-1);
                    if preferred_charset_id >= 0
                        && self
                            .memq(Value::fixnum(preferred_charset_id as i64), charset_list)
                            .is_nil()
                    {
                        preferred_charset_id = -1;
                    }
                }
                i += (-c - 1) as usize;
                continue;
            }
            let u = c as u32;
            if u < 0x80 {
                emit.ascii(dst, c);
                continue;
            }
            if character::char_byte8_p(u) {
                emit.one(dst, character::char_to_byte8(u) as i32);
                continue;
            }
            let mut found = None;
            if preferred_charset_id >= 0 && self.char_charset_p(c, preferred_charset_id)? {
                found = Some((
                    preferred_charset_id,
                    self.encode_char(preferred_charset_id, c)?,
                ));
            }
            if found.is_none() {
                found = self.char_charset(c, charset_list)?;
            }
            let (charset, code) = match found {
                Some(f) => f,
                None => {
                    c = coding.default_char as i32;
                    if (c as u32) < 0x80 {
                        emit.ascii(dst, c);
                        continue;
                    }
                    match self.char_charset(c, charset_list)? {
                        Some(f) => f,
                        None => {
                            emit.ascii(dst, b' ' as i32);
                            continue;
                        }
                    }
                }
            };
            let cs = self.charset(charset);
            let dimension = cs.dimension;
            let (lead0, lead1) = leading_codes(cs.emacs_mule_id);
            emit.one(dst, lead0);
            if lead1 != 0 {
                emit.one(dst, lead1);
            }
            if dimension == 1 {
                emit.one(dst, (code | 0x80) as i32);
            } else {
                let code = code | 0x8080;
                emit.one(dst, (code >> 8) as i32);
                emit.one(dst, (code & 0xFF) as i32);
            }
        }
        self.record_conversion_result(coding, CodingResult::Success);
        coding.produced_char += emit.produced_chars;
        coding.produced = dst.len();
        Ok(())
    }
}
