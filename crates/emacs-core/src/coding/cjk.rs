use super::engine::{
    DetectionInfo, Emit, MAX_ANNOTATION_LENGTH, Src, add_charset_data, category_mask, omb,
};
use super::{
    ATTR_ASCII_COMPAT, ATTR_CCL_VALIDS, ATTR_CHARSET_LIST, CAT_BIG5, CAT_CCL, CAT_SJIS,
    CodingResult, CodingSystem, MODE_LAST_BLOCK, MODE_SAFE_ENCODING,
};
use crate::ccl::{
    CclProgram, STAT_INVALID_CMD, STAT_QUIT, STAT_SUSPEND_BY_DST, STAT_SUSPEND_BY_SRC, jis_to_sjis,
    sjis_to_jis,
};
use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::value::Value;

const INHIBIT_CHARACTER_SUBSTITUTION: u32 = b'?' as u32;

fn sjis_to_jis2(code: i32) -> i32 {
    let s1 = code >> 8;
    let s2 = code & 0xFF;
    let (j1, j2) = if s2 >= 0x9F {
        let j1 = match s1 {
            0xF0 => 0x28,
            0xF1 => 0x24,
            0xF2 => 0x2C,
            0xF3 => 0x2E,
            _ => 0x6E + (s1 - 0xF4) * 2,
        };
        (j1, s2 - 0x7E)
    } else {
        let j1 = if s1 <= 0xF2 {
            0x21 + (s1 - 0xF0) * 2
        } else if s1 <= 0xF4 {
            0x2D + (s1 - 0xF3) * 2
        } else {
            0x6F + (s1 - 0xF5) * 2
        };
        (j1, s2 - if s2 >= 0x7F { 0x20 } else { 0x1F })
    };
    (j1 << 8) | j2
}

fn jis_to_sjis2(code: i32) -> i32 {
    let j1 = code >> 8;
    let j2 = code & 0xFF;
    let (s1, s2) = if j1 & 1 != 0 {
        let s1 = if j1 <= 0x25 {
            0xF0 + (j1 - 0x21) / 2
        } else if j1 <= 0x2F {
            0xF3 + (j1 - 0x2D) / 2
        } else {
            0xF5 + (j1 - 0x6F) / 2
        };
        (s1, j2 + if j2 >= 0x60 { 0x20 } else { 0x1F })
    } else {
        let s1 = match j1 {
            0x28 => 0xF0,
            0x24 => 0xF1,
            0x2C => 0xF2,
            0x2E => 0xF3,
            _ => 0xF4 + (j1 - 0x6E) / 2,
        };
        (s1, j2 + 0x7E)
    };
    (s1 << 8) | s2
}

fn charset_ids(lisp: &Lisp, list: Value) -> Vec<i32> {
    let mut out = Vec::new();
    let mut tail = list;
    while tail.is_cons() {
        out.push(lisp.xcar(tail).as_fixnum().unwrap_or(0) as i32);
        tail = lisp.xcdr(tail);
    }
    out
}

fn ccl_status_result(status: i32) -> CodingResult {
    match status {
        STAT_SUSPEND_BY_SRC => CodingResult::InsufficientSrc,
        STAT_SUSPEND_BY_DST => CodingResult::InsufficientDst,
        STAT_QUIT | STAT_INVALID_CMD => CodingResult::Interrupt,
        _ => CodingResult::Success,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TwoByte {
    Sjis,
    Big5,
}

impl Lisp {
    pub fn detect_coding_sjis(
        &mut self,
        coding: &mut CodingSystem,
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let source = std::mem::take(&mut coding.source);
        let r = self.detect_two_byte(coding, &source, info, TwoByte::Sjis);
        coding.source = source;
        r
    }

    pub fn detect_coding_big5(
        &mut self,
        coding: &mut CodingSystem,
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let source = std::mem::take(&mut coding.source);
        let r = self.detect_two_byte(coding, &source, info, TwoByte::Big5);
        coding.source = source;
        r
    }

    fn detect_two_byte(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
        info: &mut DetectionInfo,
        kind: TwoByte,
    ) -> LispResult<bool> {
        let mut s = Src::new(source, 0, coding.src_bytes, coding.src_multibyte);
        let mut found = 0;
        let mask = category_mask(if kind == TwoByte::Sjis {
            CAT_SJIS
        } else {
            CAT_BIG5
        });
        let attrs = self.coding_id_attrs(coding.id);
        let list = self.attr(attrs, ATTR_CHARSET_LIST);
        let max_first = if charset_ids(self, list).len() <= 3 {
            0xEF
        } else {
            0xFC
        };
        info.checked |= mask;
        s.pos += coding.head_ascii.max(0) as usize;
        'nms: {
            loop {
                s.base = s.pos;
                let c = omb!(s, self, coding, 'nms);
                if c < 0x80 {
                    continue;
                }
                match kind {
                    TwoByte::Sjis => {
                        if (0x81..=0x9F).contains(&c) || (0xE0..=max_first).contains(&c) {
                            let c = omb!(s, self, coding, 'nms);
                            if !(0x40..=0xFC).contains(&c) || c == 0x7F {
                                break;
                            }
                            found = mask;
                        } else if (0xA0..0xE0).contains(&c) {
                            found = mask;
                        } else {
                            break;
                        }
                    }
                    TwoByte::Big5 => {
                        if c >= 0xA1 {
                            let c = omb!(s, self, coding, 'nms);
                            if c < 0x40 || (0x7F..=0xA0).contains(&c) {
                                return Ok(false);
                            }
                            found = mask;
                        } else {
                            break;
                        }
                    }
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

    pub fn decode_coding_sjis(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let source = std::mem::take(&mut coding.source);
        let r = self.decode_two_byte(coding, &source, TwoByte::Sjis);
        coding.source = source;
        r
    }

    pub fn decode_coding_big5(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let source = std::mem::take(&mut coding.source);
        let r = self.decode_two_byte(coding, &source, TwoByte::Big5);
        coding.source = source;
        r
    }

    fn decode_two_byte(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
        kind: TwoByte,
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
        let list = self.attr(attrs, ATTR_CHARSET_LIST);
        let ids = charset_ids(self, list);
        let roman = ids[0];
        let second = ids[1];
        let kanji = ids.get(2).copied().unwrap_or(-1);
        let kanji2 = ids.get(3).copied();
        let ascii = self.charsets.ascii;
        let mut char_offset = coding.produced_char as i32;
        let mut last_offset = char_offset;
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
                let c = if byte_after_cr >= 0 {
                    let v = byte_after_cr;
                    byte_after_cr = -1;
                    v
                } else {
                    omb!(s, self, coding, 'nms)
                };
                let decoded = 'inv: {
                    if c < 0 {
                        break 'inv None;
                    }
                    if c < 0x80 {
                        if eol_dos && c == b'\r' as i32 {
                            byte_after_cr = omb!(s, self, coding, 'nms);
                        }
                        break 'inv Some((c, roman));
                    }
                    match kind {
                        TwoByte::Sjis => {
                            if c == 0x80 || c == 0xA0 {
                                None
                            } else if (0xA1..=0xDF).contains(&c) {
                                Some((c & 0x7F, second))
                            } else if c <= 0xEF {
                                let c1 = omb!(s, self, coding, 'nms);
                                if !(0x40..=0xFC).contains(&c1) || c1 == 0x7F {
                                    break 'inv None;
                                }
                                Some((sjis_to_jis((c << 8) | c1), kanji))
                            } else if let Some(k2) = kanji2
                                && c <= 0xFC
                            {
                                let c1 = omb!(s, self, coding, 'nms);
                                if !(0x40..=0xFC).contains(&c1) || c1 == 0x7F {
                                    break 'inv None;
                                }
                                Some((sjis_to_jis2((c << 8) | c1), k2))
                            } else {
                                None
                            }
                        }
                        TwoByte::Big5 => {
                            if !(0xA1..=0xFE).contains(&c) {
                                break 'inv None;
                            }
                            let c1 = omb!(s, self, coding, 'nms);
                            if c1 < 0x40 || (0x7E < c1 && c1 < 0xA1) || c1 > 0xFE {
                                break 'inv None;
                            }
                            Some(((c << 8) | c1, second))
                        }
                    }
                };
                match decoded {
                    Some((code, charset)) => {
                        if charset != ascii && last_id != charset {
                            if last_id != ascii {
                                add_charset_data(coding, char_offset - last_offset, last_id);
                            }
                            last_id = charset;
                            last_offset = char_offset;
                        }
                        let ch = self.decode_char(charset, code as u32)?;
                        coding.charbuf.push(ch);
                        char_offset += 1;
                    }
                    None => {
                        s.pos = s.base;
                        s.consumed_chars = consumed_chars_base;
                        let c = omb!(s, self, coding, 'nms);
                        coding.charbuf.push(if c < 0 { -c } else { c + 0x3F_FF00 });
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

    fn coding_char_charset(
        &mut self,
        coding: &CodingSystem,
        c: i32,
        list: Value,
    ) -> LispResult<(i32, u32)> {
        if let Some(found) = self.char_charset(c, list)? {
            return Ok(found);
        }
        if coding.mode & MODE_SAFE_ENCODING != 0 {
            return Ok((self.charsets.ascii, INHIBIT_CHARACTER_SUBSTITUTION));
        }
        let default = coding.default_char as i32;
        Ok(self
            .char_charset(default, list)?
            .unwrap_or((self.charsets.ascii, (default & 0x7F) as u32)))
    }

    pub fn encode_coding_sjis(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        self.encode_two_byte(coding, TwoByte::Sjis)
    }

    pub fn encode_coding_big5(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        self.encode_two_byte(coding, TwoByte::Big5)
    }

    fn encode_two_byte(&mut self, coding: &mut CodingSystem, kind: TwoByte) -> LispResult<()> {
        let charbuf = std::mem::take(&mut coding.charbuf);
        let mut dst = std::mem::take(&mut coding.destination);
        let r = self.encode_two_byte_1(coding, &charbuf, &mut dst, kind);
        coding.destination = dst;
        coding.charbuf = charbuf;
        r
    }

    fn encode_two_byte_1(
        &mut self,
        coding: &mut CodingSystem,
        charbuf: &[i32],
        dst: &mut Vec<u8>,
        kind: TwoByte,
    ) -> LispResult<()> {
        let attrs = self.coding_id_attrs(coding.id);
        let list = self.attr(attrs, ATTR_CHARSET_LIST);
        let ids = charset_ids(self, list);
        let second = ids[1];
        let kanji = ids.get(2).copied().unwrap_or(-1);
        let kanji2 = ids.get(3).copied();
        let ascii_compatible = !self.attr(attrs, ATTR_ASCII_COMPAT).is_nil();
        let mut emit = Emit::new(coding.dst_multibyte);
        for &c in charbuf {
            let u = c as u32;
            if u < 0x80 && ascii_compatible {
                emit.ascii(dst, c);
                continue;
            }
            if character::char_byte8_p(u) {
                emit.one(dst, character::char_to_byte8(u) as i32);
                continue;
            }
            let (charset, code) = self.coding_char_charset(coding, c, list)?;
            let code = code as i32;
            match kind {
                TwoByte::Sjis if charset == kanji => {
                    let code = jis_to_sjis(code);
                    emit.two(dst, code >> 8, code & 0xFF);
                }
                TwoByte::Sjis if charset == second => emit.one(dst, code | 0x80),
                TwoByte::Sjis if Some(charset) == kanji2 => {
                    let c1 = code >> 8;
                    if c1 == 0x21
                        || (0x23..=0x25).contains(&c1)
                        || c1 == 0x28
                        || (0x2C..=0x2F).contains(&c1)
                        || c1 >= 0x6E
                    {
                        let code = jis_to_sjis2(code);
                        emit.two(dst, code >> 8, code & 0xFF);
                    } else {
                        emit.ascii(dst, code & 0x7F);
                    }
                }
                TwoByte::Big5 if charset == second => emit.two(dst, code >> 8, code & 0xFF),
                _ => emit.ascii(dst, code & 0x7F),
            }
        }
        self.record_conversion_result(coding, CodingResult::Success);
        coding.produced_char += emit.produced_chars;
        coding.produced = dst.len();
        Ok(())
    }

    pub fn detect_coding_ccl(
        &mut self,
        coding: &mut CodingSystem,
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let source = std::mem::take(&mut coding.source);
        let r = self.detect_coding_ccl_1(coding, &source, info);
        coding.source = source;
        r
    }

    fn detect_coding_ccl_1(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let mut s = Src::new(source, 0, coding.src_bytes, coding.src_multibyte);
        let mut found = 0;
        info.checked |= category_mask(CAT_CCL);
        let id = self.codings.categories[CAT_CCL].id;
        let attrs = self.coding_id_attrs(id);
        let valids_value = self.attr(attrs, ATTR_CCL_VALIDS);
        let valids = self.string_bytes(valids_value).to_vec();
        if !self.attr(attrs, ATTR_ASCII_COMPAT).is_nil() {
            s.pos += coding.head_ascii.max(0) as usize;
        }
        'nms: {
            loop {
                s.base = s.pos;
                let c = omb!(s, self, coding, 'nms);
                let v = if c < 0 {
                    0
                } else {
                    valids.get(c as usize).copied().unwrap_or(0)
                };
                if v == 0 {
                    break;
                }
                if v > 1 {
                    found = category_mask(CAT_CCL);
                }
            }
            info.rejected |= category_mask(CAT_CCL);
            return Ok(false);
        }
        info.found |= found;
        Ok(true)
    }

    pub fn decode_coding_ccl(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let source = std::mem::take(&mut coding.source);
        let mut ccl = coding.ccl.take().unwrap_or_default();
        let r = self.decode_coding_ccl_1(coding, &source, &mut ccl);
        coding.ccl = Some(ccl);
        coding.source = source;
        r
    }

    fn decode_coding_ccl_1(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
        ccl: &mut CclProgram,
    ) -> LispResult<()> {
        let attrs = self.coding_id_attrs(coding.id);
        let charset_list = self.attr(attrs, ATTR_CHARSET_LIST);
        let multibyte = coding.src_multibyte;
        let src_end = coding.src_bytes;
        let mut src = coding.consumed;
        let mut consumed_chars = 0usize;
        loop {
            let mut p = src;
            let mut chars = Vec::with_capacity(1024);
            let mut byteidx = Vec::with_capacity(1025);
            if multibyte {
                while chars.len() < 1024 && p < src_end {
                    byteidx.push(p - src);
                    let (c, len) = character::string_char(&source[p..src_end]);
                    chars.push(c as i32);
                    p += len;
                }
                byteidx.push(p - src);
            } else {
                while chars.len() < 1024 && p < src_end {
                    chars.push(source[p] as i32);
                    p += 1;
                }
            }
            if p == src_end && coding.mode & MODE_LAST_BLOCK != 0 {
                ccl.last_block = true;
            }
            let room = coding.charbuf_size.saturating_sub(coding.charbuf.len());
            let mut out = vec![0i32; room];
            self.ccl_driver(ccl, Some(&chars), Some(&mut out), charset_list)?;
            coding
                .charbuf
                .extend_from_slice(&out[..ccl.produced as usize]);
            let consumed = ccl.consumed as usize;
            src += if multibyte {
                byteidx[consumed]
            } else {
                consumed
            };
            consumed_chars += consumed;
            if p == src_end || ccl.status != STAT_SUSPEND_BY_SRC {
                break;
            }
        }
        self.record_conversion_result(coding, ccl_status_result(ccl.status));
        coding.consumed_char += consumed_chars;
        coding.consumed = src;
        Ok(())
    }

    pub fn encode_coding_ccl(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let mut ccl = coding.ccl.take().unwrap_or_default();
        let charbuf = std::mem::take(&mut coding.charbuf);
        let mut dst = std::mem::take(&mut coding.destination);
        let r = self.encode_coding_ccl_1(coding, &charbuf, &mut dst, &mut ccl);
        coding.destination = dst;
        coding.charbuf = charbuf;
        coding.ccl = Some(ccl);
        r
    }

    fn encode_coding_ccl_1(
        &mut self,
        coding: &mut CodingSystem,
        charbuf: &[i32],
        dst: &mut Vec<u8>,
        ccl: &mut CclProgram,
    ) -> LispResult<()> {
        let attrs = self.coding_id_attrs(coding.id);
        let charset_list = self.attr(attrs, ATTR_CHARSET_LIST);
        if coding.consumed_char == coding.src_chars && coding.mode & MODE_LAST_BLOCK != 0 {
            ccl.last_block = true;
        }
        let mut emit = Emit::new(coding.dst_multibyte);
        let mut pos = 0;
        loop {
            let mut out = [0i32; 1024];
            self.ccl_driver(ccl, Some(&charbuf[pos..]), Some(&mut out), charset_list)?;
            let produced = ccl.produced as usize;
            if emit.multibyte {
                for &v in &out[..produced] {
                    emit.one(dst, v & 0xFF);
                }
            } else {
                dst.extend(out[..produced].iter().map(|&v| (v & 0xFF) as u8));
                emit.produced_chars += produced;
            }
            pos += ccl.consumed as usize;
            if ccl.status == STAT_QUIT || ccl.status == STAT_INVALID_CMD || pos >= charbuf.len() {
                break;
            }
        }
        self.record_conversion_result(coding, ccl_status_result(ccl.status));
        coding.produced_char += emit.produced_chars;
        coding.produced = dst.len();
        Ok(())
    }
}
