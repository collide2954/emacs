use super::engine::{
    CATEGORY_MASK_UTF_8, CATEGORY_MASK_UTF_16, DetectionInfo, Emit, Src, category_mask, omb,
};
use super::{
    Bom, CAT_UTF_8_AUTO, CAT_UTF_8_NOSIG, CAT_UTF_8_SIG, CAT_UTF_16_AUTO, CAT_UTF_16_BE,
    CAT_UTF_16_BE_NOSIG, CAT_UTF_16_LE, CAT_UTF_16_LE_NOSIG, CodingResult, CodingSystem,
    MODE_LAST_BLOCK,
};
use crate::character::{self, MAX_CHAR, MAX_UNICODE_CHAR};
use crate::error::LispResult;
use crate::lisp::Lisp;

fn extra(c: i32) -> bool {
    c & 0xC0 == 0x80
}

fn byte8_or_ascii(c: i32) -> i32 {
    if (c as u32) < 0x80 { c } else { c + 0x3F_FF00 }
}

impl Lisp {
    pub fn detect_coding_utf_8(
        &mut self,
        coding: &mut CodingSystem,
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let source = std::mem::take(&mut coding.source);
        let mut s = Src::new(&source, 0, coding.src_bytes, coding.src_multibyte);
        let mut bom_found = false;
        let mut nchars = coding.head_ascii.max(0);
        info.checked |= CATEGORY_MASK_UTF_8;
        s.pos += nchars as usize;
        if s.pos == 0
            && s.pos + 3 < s.end
            && source[0] == 0xEF
            && source[1] == 0xBB
            && source[2] == 0xBF
        {
            bom_found = true;
            s.pos += 3;
            nchars += 1;
        }
        'nms: {
            loop {
                s.base = s.pos;
                let c = omb!(s, self, coding, 'nms);
                if c < 0 || c < 0x80 {
                    nchars += 1;
                    if c == b'\r' as i32 && s.pos < s.end && source[s.pos] == b'\n' {
                        s.pos += 1;
                        nchars += 1;
                    }
                    continue;
                }
                let c1 = omb!(s, self, coding, 'nms);
                if c1 < 0 || !extra(c1) {
                    break;
                }
                if c & 0xE0 == 0xC0 {
                    nchars += 1;
                    continue;
                }
                let c2 = omb!(s, self, coding, 'nms);
                if c2 < 0 || !extra(c2) {
                    break;
                }
                if c & 0xF0 == 0xE0 {
                    nchars += 1;
                    continue;
                }
                let c3 = omb!(s, self, coding, 'nms);
                if c3 < 0 || !extra(c3) {
                    break;
                }
                if c & 0xF8 == 0xF0 {
                    nchars += 1;
                    continue;
                }
                let c4 = omb!(s, self, coding, 'nms);
                if c4 < 0 || !extra(c4) {
                    break;
                }
                if c & 0xFC == 0xF8 && c < 0xF8 {
                    nchars += 1;
                    continue;
                }
                break;
            }
            info.rejected |= CATEGORY_MASK_UTF_8;
            coding.source = source;
            return Ok(false);
        }
        if s.base < s.pos && coding.mode & MODE_LAST_BLOCK != 0 {
            info.rejected |= CATEGORY_MASK_UTF_8;
            coding.source = source;
            return Ok(false);
        }
        if bom_found {
            info.found |= category_mask(CAT_UTF_8_AUTO)
                | category_mask(CAT_UTF_8_SIG)
                | category_mask(CAT_UTF_8_NOSIG);
        } else {
            info.rejected |= category_mask(CAT_UTF_8_SIG);
            if (nchars as usize) < s.end {
                info.found |= category_mask(CAT_UTF_8_AUTO) | category_mask(CAT_UTF_8_NOSIG);
            }
        }
        coding.detected_utf8_bytes = s.base as i64;
        coding.detected_utf8_chars = nchars;
        coding.source = source;
        Ok(true)
    }

    pub fn decode_coding_utf_8(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let source = std::mem::take(&mut coding.source);
        let mut s = Src::new(
            &source,
            coding.consumed,
            coding.src_bytes,
            coding.src_multibyte,
        );
        let charbuf_end = coding.charbuf_size;
        let mut consumed_chars_base = 0usize;
        let bom = coding.utf_8_bom;
        let eol_dos = self.coding_eol_dos(coding);
        let mut byte_after_cr: i32 = -1;
        'nms: {
            if bom != Bom::Without {
                s.base = s.pos;
                let c1 = omb!(s, self, coding, 'nms);
                if c1 & 0xF0 != 0xE0 {
                    s.pos = s.base;
                } else {
                    let c2 = omb!(s, self, coding, 'nms);
                    if !extra(c2) {
                        s.pos = s.base;
                    } else {
                        let c3 = omb!(s, self, coding, 'nms);
                        if !extra(c3) || c1 != 0xEF || c2 != 0xBB || c3 != 0xBF {
                            s.pos = s.base;
                        } else {
                            coding.utf_8_bom = Bom::Without;
                        }
                    }
                }
            }
            coding.utf_8_bom = Bom::Without;
            loop {
                s.base = s.pos;
                consumed_chars_base = s.consumed_chars;
                if coding.charbuf.len() >= charbuf_end {
                    if byte_after_cr >= 0 {
                        s.base -= 1;
                    }
                    break;
                }
                if s.multibyte
                    && !eol_dos
                    && coding.charbuf.len() + 6 < charbuf_end
                    && s.pos + 6 < s.end
                {
                    'fast: while coding.charbuf.len() + 6 < charbuf_end && s.pos + 6 < s.end {
                        for _ in 0..4 {
                            let c1 = source[s.pos] as i32;
                            if c1 & 0x80 != 0 {
                                break 'fast;
                            }
                            s.pos += 1;
                            s.consumed_chars += 1;
                            coding.charbuf.push(c1);
                        }
                    }
                    if s.pos != s.base {
                        continue;
                    }
                }
                let c1 = if byte_after_cr >= 0 {
                    let v = byte_after_cr;
                    byte_after_cr = -1;
                    v
                } else {
                    omb!(s, self, coding, 'nms)
                };
                let c = if c1 < 0 {
                    -c1
                } else if c1 < 0x80 {
                    if eol_dos && c1 == b'\r' as i32 {
                        byte_after_cr = omb!(s, self, coding, 'nms);
                    }
                    c1
                } else {
                    let decoded = 'inv: {
                        let c2 = omb!(s, self, coding, 'nms);
                        if c2 < 0 || !extra(c2) {
                            break 'inv None;
                        }
                        if c1 & 0xE0 == 0xC0 {
                            let c = ((c1 & 0x1F) << 6) | (c2 & 0x3F);
                            if c < 128 {
                                break 'inv None;
                            }
                            break 'inv Some(c);
                        }
                        let c3 = omb!(s, self, coding, 'nms);
                        if c3 < 0 || !extra(c3) {
                            break 'inv None;
                        }
                        if c1 & 0xF0 == 0xE0 {
                            let c = ((c1 & 0xF) << 12) | ((c2 & 0x3F) << 6) | (c3 & 0x3F);
                            if c < 0x800 || (0xD800..0xE000).contains(&c) {
                                break 'inv None;
                            }
                            break 'inv Some(c);
                        }
                        let c4 = omb!(s, self, coding, 'nms);
                        if c4 < 0 || !extra(c4) {
                            break 'inv None;
                        }
                        if c1 & 0xF8 == 0xF0 {
                            let c = ((c1 & 0x7) << 18)
                                | ((c2 & 0x3F) << 12)
                                | ((c3 & 0x3F) << 6)
                                | (c4 & 0x3F);
                            if c < 0x10000 {
                                break 'inv None;
                            }
                            break 'inv Some(c);
                        }
                        let c5 = omb!(s, self, coding, 'nms);
                        if c5 < 0 || !extra(c5) {
                            break 'inv None;
                        }
                        if c1 & 0xFC == 0xF8 {
                            let c = ((c1 & 0x3) << 24)
                                | ((c2 & 0x3F) << 18)
                                | ((c3 & 0x3F) << 12)
                                | ((c4 & 0x3F) << 6)
                                | (c5 & 0x3F);
                            if c > MAX_CHAR as i32 || c < 0x20_0000 {
                                break 'inv None;
                            }
                            break 'inv Some(c);
                        }
                        None
                    };
                    match decoded {
                        Some(c) => c,
                        None => {
                            s.pos = s.base;
                            s.consumed_chars = consumed_chars_base;
                            let c = omb!(s, self, coding, 'nms);
                            coding.charbuf.push(byte8_or_ascii(c));
                            continue;
                        }
                    }
                };
                coding.charbuf.push(c);
            }
        }
        coding.consumed_char += consumed_chars_base;
        coding.consumed = s.base;
        coding.source = source;
        Ok(())
    }

    pub fn encode_coding_utf_8(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let mut dst = std::mem::take(&mut coding.destination);
        let start = dst.len();
        let mut emit = Emit::new(coding.dst_multibyte);
        if coding.utf_8_bom != Bom::Without {
            emit.three(&mut dst, 0xEF, 0xBB, 0xBF);
            coding.utf_8_bom = Bom::Without;
        }
        if emit.multibyte {
            for &c in &coding.charbuf {
                let c = c as u32;
                if character::char_byte8_p(c) {
                    emit.one(&mut dst, character::char_to_byte8(c) as i32);
                } else {
                    for b in character::char_string(c) {
                        emit.one(&mut dst, b as i32);
                    }
                }
            }
        } else {
            for &c in &coding.charbuf {
                let c = c as u32;
                if character::char_byte8_p(c) {
                    dst.push(character::char_to_byte8(c));
                } else {
                    character::push_char(&mut dst, c);
                }
            }
            emit.produced_chars = dst.len() - start;
        }
        self.record_conversion_result(coding, CodingResult::Success);
        coding.produced_char += emit.produced_chars;
        coding.produced = dst.len();
        coding.destination = dst;
        Ok(())
    }

    pub fn detect_coding_utf_16(
        &mut self,
        coding: &mut CodingSystem,
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let source = &coding.source;
        let multibytep = coding.src_multibyte;
        let end = coding.src_bytes;
        let mut pos = 0usize;
        info.checked |= CATEGORY_MASK_UTF_16;
        if coding.mode & MODE_LAST_BLOCK != 0 && coding.src_chars & 1 != 0 {
            info.rejected |= CATEGORY_MASK_UTF_16;
            return Ok(false);
        }
        let byte = |i: usize| source.get(i).copied().unwrap_or(0) as i32;
        let two_more_bytes = |pos: &mut usize| -> Option<(i32, i32)> {
            let mut c1;
            loop {
                if *pos == end {
                    return None;
                }
                c1 = byte(*pos);
                *pos += 1;
                if multibytep && c1 & 0x80 != 0 {
                    if c1 & 0xFE == 0xC0 {
                        c1 = ((c1 & 1) << 6) | byte(*pos);
                        *pos += 1;
                    } else {
                        *pos += super::engine::bytes_by_char_head(c1 as u8) - 1;
                        c1 = -1;
                    }
                }
                if c1 >= 0 {
                    break;
                }
            }
            if *pos == end {
                return None;
            }
            let mut c2 = byte(*pos);
            *pos += 1;
            if multibytep && c2 & 0x80 != 0 {
                if c2 & 0xFE == 0xC0 {
                    c2 = ((c2 & 1) << 6) | byte(*pos);
                    *pos += 1;
                } else {
                    c2 = -1;
                }
            }
            Some((c1, c2))
        };
        let Some((c1, c2)) = two_more_bytes(&mut pos) else {
            return Ok(true);
        };
        if c1 == 0xFF && c2 == 0xFE {
            info.found |= category_mask(CAT_UTF_16_LE) | category_mask(CAT_UTF_16_AUTO);
            info.rejected |= category_mask(CAT_UTF_16_BE)
                | category_mask(CAT_UTF_16_BE_NOSIG)
                | category_mask(CAT_UTF_16_LE_NOSIG);
        } else if c1 == 0xFE && c2 == 0xFF {
            info.found |= category_mask(CAT_UTF_16_BE) | category_mask(CAT_UTF_16_AUTO);
            info.rejected |= category_mask(CAT_UTF_16_LE)
                | category_mask(CAT_UTF_16_BE_NOSIG)
                | category_mask(CAT_UTF_16_LE_NOSIG);
        } else if c2 < 0 {
            info.rejected |= CATEGORY_MASK_UTF_16;
            return Ok(false);
        } else {
            let mut e = [false; 256];
            let mut o = [false; 256];
            let mut e_num = 1;
            let mut o_num = 1;
            e[c1 as usize] = true;
            o[c2 as usize] = true;
            info.rejected |= category_mask(CAT_UTF_16_AUTO)
                | category_mask(CAT_UTF_16_BE)
                | category_mask(CAT_UTF_16_LE);
            while info.rejected & CATEGORY_MASK_UTF_16 != CATEGORY_MASK_UTF_16 {
                let Some((c1, c2)) = two_more_bytes(&mut pos) else {
                    return Ok(true);
                };
                if c2 < 0 {
                    break;
                }
                if !e[c1 as usize] {
                    e[c1 as usize] = true;
                    e_num += 1;
                    if e_num >= 128 {
                        info.rejected |= category_mask(CAT_UTF_16_BE_NOSIG);
                    }
                }
                if !o[c2 as usize] {
                    o[c2 as usize] = true;
                    o_num += 1;
                    if o_num >= 128 {
                        info.rejected |= category_mask(CAT_UTF_16_LE_NOSIG);
                    }
                }
            }
            return Ok(false);
        }
        Ok(true)
    }

    pub fn decode_coding_utf_16(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let source = std::mem::take(&mut coding.source);
        let mut s = Src::new(
            &source,
            coding.consumed,
            coding.src_bytes,
            coding.src_multibyte,
        );
        let charbuf_end = coding.charbuf_size - 2;
        let mut consumed_chars_base = 0usize;
        let bom = coding.utf_16_bom;
        let big_endian = coding.utf_16_big_endian;
        let mut surrogate = coding.utf_16_surrogate;
        let eol_dos = self.coding_eol_dos(coding);
        let mut byte_after_cr1: i32 = -1;
        let mut byte_after_cr2: i32 = -1;
        'nms: {
            if bom == Bom::With {
                s.base = s.pos;
                let c1 = omb!(s, self, coding, 'nms);
                let c2 = omb!(s, self, coding, 'nms);
                let c = (c1 << 8) | c2;
                if if big_endian { c != 0xFEFF } else { c != 0xFFFE } {
                    s.pos = s.base;
                }
                coding.utf_16_bom = Bom::Without;
            } else if bom == Bom::Detect {
                coding.utf_16_bom = Bom::Without;
            }
            loop {
                s.base = s.pos;
                consumed_chars_base = s.consumed_chars;
                if coding.charbuf.len() >= charbuf_end {
                    if byte_after_cr1 >= 0 {
                        s.base -= 2;
                    }
                    break;
                }
                let mut c1 = if byte_after_cr1 >= 0 {
                    let v = byte_after_cr1;
                    byte_after_cr1 = -1;
                    v
                } else {
                    omb!(s, self, coding, 'nms)
                };
                if c1 < 0 {
                    coding.charbuf.push(-c1);
                    continue;
                }
                let mut c2 = if byte_after_cr2 >= 0 {
                    let v = byte_after_cr2;
                    byte_after_cr2 = -1;
                    v
                } else {
                    omb!(s, self, coding, 'nms)
                };
                if c2 < 0 {
                    coding.charbuf.push(byte8_or_ascii(c1));
                    coding.charbuf.push(-c2);
                    continue;
                }
                let mut c = if big_endian {
                    (c1 << 8) | c2
                } else {
                    (c2 << 8) | c1
                };
                if surrogate != 0 {
                    if !(0xDC00..=0xDFFF).contains(&c) {
                        if big_endian {
                            c1 = surrogate >> 8;
                            c2 = surrogate & 0xFF;
                        } else {
                            c1 = surrogate & 0xFF;
                            c2 = surrogate >> 8;
                        }
                        coding.charbuf.push(c1);
                        coding.charbuf.push(c2);
                        if (0xD800..=0xDBFF).contains(&c) {
                            surrogate = c;
                            coding.utf_16_surrogate = c;
                        } else {
                            coding.charbuf.push(c);
                        }
                    } else {
                        c = ((surrogate - 0xD800) << 10) | (c - 0xDC00);
                        surrogate = 0;
                        coding.utf_16_surrogate = 0;
                        coding.charbuf.push(0x10000 + c);
                    }
                } else if (0xD800..=0xDBFF).contains(&c) {
                    surrogate = c;
                    coding.utf_16_surrogate = c;
                } else {
                    if eol_dos && c == b'\r' as i32 {
                        byte_after_cr1 = omb!(s, self, coding, 'nms);
                        byte_after_cr2 = omb!(s, self, coding, 'nms);
                    }
                    coding.charbuf.push(c);
                }
            }
        }
        coding.consumed_char += consumed_chars_base;
        coding.consumed = s.base;
        coding.source = source;
        Ok(())
    }

    pub fn encode_coding_utf_16(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let mut dst = std::mem::take(&mut coding.destination);
        let mut emit = Emit::new(coding.dst_multibyte);
        let big_endian = coding.utf_16_big_endian;
        if coding.utf_16_bom != Bom::Without {
            if big_endian {
                emit.two(&mut dst, 0xFE, 0xFF);
            } else {
                emit.two(&mut dst, 0xFF, 0xFE);
            }
            coding.utf_16_bom = Bom::Without;
        }
        for &c in &coding.charbuf {
            let mut c = c;
            if c > MAX_UNICODE_CHAR as i32 {
                c = coding.default_char as i32;
            }
            if c < 0x10000 {
                if big_endian {
                    emit.two(&mut dst, c >> 8, c & 0xFF);
                } else {
                    emit.two(&mut dst, c & 0xFF, c >> 8);
                }
            } else {
                c -= 0x10000;
                let c1 = (c >> 10) + 0xD800;
                let c2 = (c & 0x3FF) + 0xDC00;
                if big_endian {
                    emit.four(&mut dst, c1 >> 8, c1 & 0xFF, c2 >> 8, c2 & 0xFF);
                } else {
                    emit.four(&mut dst, c1 & 0xFF, c1 >> 8, c2 & 0xFF, c2 >> 8);
                }
            }
        }
        self.record_conversion_result(coding, CodingResult::Success);
        coding.produced = dst.len();
        coding.produced_char += emit.produced_chars;
        coding.destination = dst;
        Ok(())
    }
}
