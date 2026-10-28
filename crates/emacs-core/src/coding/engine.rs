use super::{
    ANNOTATE_CHARSET_MASK, ANNOTATE_COMPOSITION_MASK, ATTR_ASCII_COMPAT, ATTR_CATEGORY,
    ATTR_CCL_DECODER, ATTR_CCL_ENCODER, ATTR_DECODE_TBL, ATTR_ENCODE_TBL, ATTR_POST_READ,
    ATTR_PRE_WRITE, ATTR_TYPE, ATTR_UTF_BOM, CAT_RAW_TEXT, CAT_UTF_8_AUTO, CAT_UTF_8_SIG,
    CAT_UTF_16_AUTO, CAT_UTF_16_BE, CAT_UTF_16_BE_NOSIG, CAT_UTF_16_LE, CAT_UTF_16_LE_NOSIG,
    COMPOSITION_RELATIVE, COMPOSITION_WITH_RULE, CodingResult, CodingSystem, CodingType,
    FOR_UNIBYTE_MASK, MODE_LAST_BLOCK, MODE_SELECTIVE_DISPLAY,
};
use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub const EOL_SEEN_NONE: u32 = 0;
pub const EOL_SEEN_LF: u32 = 1;
pub const EOL_SEEN_CR: u32 = 2;
pub const EOL_SEEN_CRLF: u32 = 4;

pub const MAX_ANNOTATION_LENGTH: usize = 5;
const MAX_CHARBUF_SIZE: usize = 0x4000;
const MAX_CHARBUF_EXTRA_SIZE: usize = MAX_ANNOTATION_LENGTH * 3 + 1;
const MAX_EOL_CHECK_COUNT: i32 = 3;
const MAX_LOOKUP_MAX: i64 = 16384;

pub const fn category_mask(category: usize) -> u32 {
    1 << category
}

pub const CATEGORY_MASK_UTF_8: u32 = category_mask(CAT_UTF_8_AUTO)
    | category_mask(super::CAT_UTF_8_NOSIG)
    | category_mask(CAT_UTF_8_SIG);
pub const CATEGORY_MASK_UTF_16: u32 = category_mask(CAT_UTF_16_AUTO)
    | category_mask(CAT_UTF_16_BE)
    | category_mask(CAT_UTF_16_LE)
    | category_mask(CAT_UTF_16_BE_NOSIG)
    | category_mask(CAT_UTF_16_LE_NOSIG);
pub const CATEGORY_MASK_ISO_ESCAPE: u32 = category_mask(super::CAT_ISO_7)
    | category_mask(super::CAT_ISO_7_TIGHT)
    | category_mask(super::CAT_ISO_7_ELSE)
    | category_mask(super::CAT_ISO_8_ELSE);
pub const CATEGORY_MASK_ANY: u32 = (1 << CAT_RAW_TEXT) - 1;

#[derive(Default, Clone, Copy, Debug)]
pub struct DetectionInfo {
    pub checked: u32,
    pub found: u32,
    pub rejected: u32,
}

pub struct Src<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub end: usize,
    pub base: usize,
    pub multibyte: bool,
    pub consumed_chars: usize,
}

impl<'a> Src<'a> {
    pub fn new(data: &'a [u8], pos: usize, end: usize, multibyte: bool) -> Self {
        Src {
            data,
            pos,
            end,
            base: pos,
            multibyte,
            consumed_chars: 0,
        }
    }

    pub fn at(&self, i: usize) -> i32 {
        self.data.get(i).copied().unwrap_or(0) as i32
    }

    pub fn one_more_byte(&mut self, lisp: &mut Lisp, coding: &mut CodingSystem) -> Option<i32> {
        if self.pos == self.end {
            if self.base < self.pos {
                lisp.record_conversion_result(coding, CodingResult::InsufficientSrc);
            }
            return None;
        }
        let mut c = self.data[self.pos] as i32;
        self.pos += 1;
        if self.multibyte && c & 0x80 != 0 {
            if c & 0xFE == 0xC0 {
                c = ((c & 1) << 6) | self.at(self.pos);
                self.pos += 1;
            } else {
                self.pos -= 1;
                let (ch, len) = character::string_char(&self.data[self.pos..self.end]);
                self.pos += len;
                c = -(ch as i32);
                lisp.record_conversion_result(coding, CodingResult::InvalidSrc);
            }
        }
        self.consumed_chars += 1;
        Some(c)
    }
}

macro_rules! omb {
    ($s:expr, $lisp:expr, $coding:expr, $l:lifetime) => {
        match $s.one_more_byte($lisp, $coding) {
            Some(c) => c,
            None => break $l,
        }
    };
}
pub(crate) use omb;

pub struct Emit {
    pub multibyte: bool,
    pub produced_chars: usize,
}

impl Emit {
    pub fn new(multibyte: bool) -> Self {
        Emit {
            multibyte,
            produced_chars: 0,
        }
    }

    pub fn ascii(&mut self, dst: &mut Vec<u8>, c: i32) {
        self.produced_chars += 1;
        dst.push(c as u8);
    }

    pub fn one(&mut self, dst: &mut Vec<u8>, c: i32) {
        self.produced_chars += 1;
        if self.multibyte {
            let mut ch = c as u32 & 0xFF;
            if ch >= 0x80 {
                ch = character::byte8_to_char(ch as u8);
            }
            character::push_char(dst, ch);
        } else {
            dst.push(c as u8);
        }
    }

    pub fn two(&mut self, dst: &mut Vec<u8>, c1: i32, c2: i32) {
        self.one(dst, c1);
        self.one(dst, c2);
    }

    pub fn three(&mut self, dst: &mut Vec<u8>, c1: i32, c2: i32, c3: i32) {
        self.one(dst, c1);
        self.two(dst, c2, c3);
    }

    pub fn four(&mut self, dst: &mut Vec<u8>, c1: i32, c2: i32, c3: i32, c4: i32) {
        self.two(dst, c1, c2);
        self.two(dst, c3, c4);
    }
}

pub fn bytes_by_char_head(byte: u8) -> usize {
    if byte & 0x80 == 0 {
        1
    } else if byte & 0x20 == 0 {
        2
    } else if byte & 0x10 == 0 {
        3
    } else if byte & 0x08 == 0 {
        4
    } else {
        5
    }
}

fn merge_eol(eol_seen: &mut u32, this_eol: u32) -> bool {
    if *eol_seen == EOL_SEEN_NONE {
        *eol_seen = this_eol;
    } else if *eol_seen != this_eol {
        if (*eol_seen == EOL_SEEN_CR && this_eol == EOL_SEEN_CRLF)
            || (*eol_seen == EOL_SEEN_CRLF && this_eol == EOL_SEEN_CR)
        {
            *eol_seen = EOL_SEEN_CRLF;
        } else {
            *eol_seen = EOL_SEEN_LF;
            return false;
        }
    }
    true
}

pub fn detect_eol(source: &[u8], category: usize) -> u32 {
    let src_end = source.len();
    let mut src = 0usize;
    let mut total = 0;
    let mut eol_seen = EOL_SEEN_NONE;
    if category_mask(category) & CATEGORY_MASK_UTF_16 != 0 {
        let msb = usize::from(category == (super::CAT_UTF_16_LE | super::CAT_UTF_16_LE_NOSIG));
        let lsb = 1 - msb;
        while src + 1 < src_end {
            let c = source[src + lsb];
            if source[src + msb] == 0 && (c == b'\n' || c == b'\r') {
                let this_eol = if c == b'\n' {
                    EOL_SEEN_LF
                } else if src + 3 >= src_end
                    || source[src + msb + 2] != 0
                    || source[src + lsb + 2] != b'\n'
                {
                    EOL_SEEN_CR
                } else {
                    src += 2;
                    EOL_SEEN_CRLF
                };
                if !merge_eol(&mut eol_seen, this_eol) {
                    break;
                }
                total += 1;
                if total == MAX_EOL_CHECK_COUNT {
                    break;
                }
            }
            src += 2;
        }
    } else {
        while src < src_end {
            let c = source[src];
            src += 1;
            if c == b'\n' || c == b'\r' {
                let this_eol = if c == b'\n' {
                    EOL_SEEN_LF
                } else if src >= src_end || source[src] != b'\n' {
                    EOL_SEEN_CR
                } else {
                    src += 1;
                    EOL_SEEN_CRLF
                };
                if !merge_eol(&mut eol_seen, this_eol) {
                    break;
                }
                total += 1;
                if total == MAX_EOL_CHECK_COUNT {
                    break;
                }
            }
        }
    }
    eol_seen
}

pub fn push_char_string(dst: &mut Vec<u8>, c: i32) {
    character::push_char(dst, c as u32);
}

pub fn add_annotation_data(coding: &mut CodingSystem, len: i32, mask: u32, nchars: i32) {
    coding.charbuf.push(-len);
    coding.charbuf.push(mask as i32);
    coding.charbuf.push(nchars);
    coding.annotated = true;
}

pub fn add_composition_data(coding: &mut CodingSystem, nchars: i32, nbytes: i32, method: i32) {
    add_annotation_data(coding, 5, ANNOTATE_COMPOSITION_MASK, nchars);
    coding.charbuf.push(nbytes);
    coding.charbuf.push(method);
}

pub fn add_charset_data(coding: &mut CodingSystem, nchars: i32, id: i32) {
    add_annotation_data(coding, 4, ANNOTATE_CHARSET_MASK, nchars);
    coding.charbuf.push(id);
}

impl Lisp {
    pub fn record_conversion_result(&mut self, coding: &mut CodingSystem, result: CodingResult) {
        coding.result = result;
        let value = match result {
            CodingResult::InsufficientSrc => Some(sym::INSUFFICIENT_SOURCE),
            CodingResult::InvalidSrc => Some(sym::INVALID_SOURCE),
            CodingResult::Interrupt => Some(sym::INTERRUPTED),
            CodingResult::InsufficientDst | CodingResult::Success => None,
        };
        if let Some(v) = value {
            self.set_plain_value(sym::LAST_CODE_CONVERSION_ERROR, v);
        }
    }

    pub fn coding_id_eol_type(&self, id: i64) -> Value {
        self.vector_items(self.coding_id_spec(id))[2]
    }

    pub fn coding_attrs(&self, coding: &CodingSystem) -> Value {
        self.coding_id_attrs(coding.id)
    }

    fn inhibit_eol_conversion(&self) -> bool {
        self.var_true(sym::INHIBIT_EOL_CONVERSION)
    }

    fn eol_is_dos(&self, coding: &CodingSystem) -> bool {
        !self.inhibit_eol_conversion() && self.coding_id_eol_type(coding.id) == sym::DOS
    }

    pub fn coding_eol_dos(&self, coding: &CodingSystem) -> bool {
        self.eol_is_dos(coding)
    }

    pub fn check_ascii(&mut self, coding: &mut CodingSystem) -> usize {
        let eol_type = self.coding_id_eol_type(coding.id);
        let mut eol_seen = coding.eol_seen;
        let src = &coding.source;
        let mut p = 0usize;
        let end = coding.src_bytes;
        if self.inhibit_eol_conversion() || eol_type.is_symbol() {
            while p < end && src[p] & 0x80 == 0 {
                if src[p] == b'\n' {
                    eol_seen |= EOL_SEEN_LF;
                }
                p += 1;
            }
        } else if end > 0 {
            let last = end - 1;
            while p < last {
                let c = src[p];
                if c & 0x80 != 0 {
                    break;
                }
                p += 1;
                if c == b'\r' {
                    if src[p] == b'\n' {
                        eol_seen |= EOL_SEEN_CRLF;
                        p += 1;
                    } else {
                        eol_seen |= EOL_SEEN_CR;
                    }
                } else if c == b'\n' {
                    eol_seen |= EOL_SEEN_LF;
                }
            }
            if p == last {
                let c = src[p];
                if c & 0x80 == 0 {
                    if c == b'\r' {
                        eol_seen |= EOL_SEEN_CR;
                    } else if c == b'\n' {
                        eol_seen |= EOL_SEEN_LF;
                    }
                    p += 1;
                }
            }
        }
        coding.head_ascii = p as i64;
        coding.eol_seen = eol_seen;
        p
    }

    pub fn check_utf_8(&mut self, coding: &mut CodingSystem) -> i64 {
        let mut nchars = coding.head_ascii;
        if coding.head_ascii < 0 {
            self.check_ascii(coding);
            nchars = coding.head_ascii;
        }
        let src = &coding.source;
        let mut p = coding.head_ascii as usize;
        if coding.src_bytes == 0 {
            return nchars;
        }
        let end = coding.src_bytes - 1;
        let mut eol_seen = coding.eol_seen;
        let extra = |b: u8| b & 0xC0 == 0x80;
        while p < end {
            let c = src[p] as u32;
            if c < 0x80 {
                p += 1;
                if c < 0x20 {
                    if c == b'\r' as u32 {
                        if src[p] == b'\n' {
                            eol_seen |= EOL_SEEN_CRLF;
                            p += 1;
                            nchars += 1;
                        } else {
                            eol_seen |= EOL_SEEN_CR;
                        }
                    } else if c == b'\n' as u32 {
                        eol_seen |= EOL_SEEN_LF;
                    }
                }
            } else if c & 0xE0 == 0xC0 {
                if c < 0xC2 || p + 1 >= end || !extra(src[p + 1]) {
                    return -1;
                }
                p += 2;
            } else if c & 0xF0 == 0xE0 {
                if p + 2 >= end || !(extra(src[p + 1]) && extra(src[p + 2])) {
                    return -1;
                }
                let v = ((c & 0xF) << 12)
                    | ((src[p + 1] as u32 & 0x3F) << 6)
                    | (src[p + 2] as u32 & 0x3F);
                if v < 0x800 || (0xD800..0xE000).contains(&v) {
                    return -1;
                }
                p += 3;
            } else if c & 0xF8 == 0xF0 {
                if p + 3 >= end || !(extra(src[p + 1]) && extra(src[p + 2]) && extra(src[p + 3])) {
                    return -1;
                }
                let v = ((c & 0x7) << 18)
                    | ((src[p + 1] as u32 & 0x3F) << 12)
                    | ((src[p + 2] as u32 & 0x3F) << 6)
                    | (src[p + 3] as u32 & 0x3F);
                if !(0x10000..0x110000).contains(&v) {
                    return -1;
                }
                p += 4;
            } else {
                return -1;
            }
            nchars += 1;
        }
        if p == end {
            if src[p] >= 0x80 {
                return -1;
            }
            nchars += 1;
            if src[p] == b'\r' {
                eol_seen |= EOL_SEEN_CR;
            } else if src[p] == b'\n' {
                eol_seen |= EOL_SEEN_LF;
            }
        }
        coding.eol_seen = eol_seen;
        nchars
    }

    pub fn adjust_coding_eol_type(
        &mut self,
        coding: &mut CodingSystem,
        eol_seen: u32,
    ) -> LispResult {
        let eol_type = self.coding_id_eol_type(coding.id);
        if !self.is_vector(eol_type) {
            return Ok(eol_type);
        }
        let (idx, result) = if eol_seen & EOL_SEEN_LF != 0 {
            (0, sym::UNIX)
        } else if eol_seen & EOL_SEEN_CRLF != 0 {
            (1, sym::DOS)
        } else if eol_seen & EOL_SEEN_CR != 0 {
            (2, sym::MAC)
        } else {
            return Ok(eol_type);
        };
        let name = self.vector_items(eol_type)[idx];
        coding.id = self.coding_system_id(name)?;
        Ok(result)
    }

    pub(super) fn run_detector(
        &mut self,
        coding: &mut CodingSystem,
        info: &mut DetectionInfo,
        category: usize,
    ) -> LispResult<bool> {
        match category {
            0..=5 => self.detect_coding_iso_2022(coding, info),
            6..=8 => self.detect_coding_utf_8(coding, info),
            9..=13 => self.detect_coding_utf_16(coding, info),
            14 => self.detect_coding_charset(coding, info),
            15 => self.detect_coding_sjis(coding, info),
            16 => self.detect_coding_big5(coding, info),
            17 => self.detect_coding_ccl(coding, info),
            18 => self.detect_coding_emacs_mule(coding, info),
            _ => Ok(false),
        }
    }

    pub fn run_decoder(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        match coding.kind {
            CodingType::Utf8 => self.decode_coding_utf_8(coding),
            CodingType::Utf16 => self.decode_coding_utf_16(coding),
            CodingType::Charset => self.decode_coding_charset(coding),
            CodingType::RawText | CodingType::Undecided => self.decode_coding_raw_text(coding),
            CodingType::EmacsMule => self.decode_coding_emacs_mule(coding),
            CodingType::Iso2022 => self.decode_coding_iso_2022(coding),
            CodingType::ShiftJis => self.decode_coding_sjis(coding),
            CodingType::Big5 => self.decode_coding_big5(coding),
            CodingType::Ccl => self.decode_coding_ccl(coding),
        }
    }

    pub fn run_encoder(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        match coding.kind {
            CodingType::Utf8 => self.encode_coding_utf_8(coding),
            CodingType::Utf16 => self.encode_coding_utf_16(coding),
            CodingType::Charset => self.encode_coding_charset(coding),
            CodingType::RawText | CodingType::Undecided => self.encode_coding_raw_text(coding),
            CodingType::EmacsMule => self.encode_coding_emacs_mule(coding),
            CodingType::Iso2022 => self.encode_coding_iso_2022(coding),
            CodingType::ShiftJis => self.encode_coding_sjis(coding),
            CodingType::Big5 => self.encode_coding_big5(coding),
            CodingType::Ccl => self.encode_coding_ccl(coding),
        }
    }

    pub fn detect_coding(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let saved_mode = coding.mode;
        let mut found = Value::NIL;
        let eol_type = self.coding_id_eol_type(coding.id);
        coding.consumed = 0;
        coding.consumed_char = 0;
        coding.produced = 0;
        coding.produced_char = 0;
        let src_end = coding.src_bytes;
        coding.eol_seen = EOL_SEEN_NONE;
        let attrs = self.coding_id_attrs(coding.id);
        if self.attr(attrs, ATTR_TYPE) == sym::UNDECIDED {
            let mut info = DetectionInfo::default();
            let mut null_byte_found = false;
            let mut eight_bit_found = false;
            let inhibit_nbd = inhibit_flag(
                coding.undecided_inhibit_nbd,
                self.var_true(sym::INHIBIT_NULL_BYTE_DETECTION),
            );
            let inhibit_ied = inhibit_flag(
                coding.undecided_inhibit_ied,
                self.var_true(sym::INHIBIT_ISO_ESCAPE_DETECTION),
            );
            let prefer_utf_8 = coding.undecided_prefer_utf_8;
            let disable_ascii_optimization = self.var_true(sym::DISABLE_ASCII_OPTIMIZATION);
            let inhibit_eol = self.inhibit_eol_conversion();
            coding.head_ascii = 0;
            let mut src = 0usize;
            while src < src_end {
                let c = coding.source[src];
                if c & 0x80 != 0 {
                    eight_bit_found = true;
                    if null_byte_found {
                        break;
                    }
                } else if c < 0x20 {
                    if (c == 0x1B || c == 0x0F || c == 0x0E) && !inhibit_ied && info.checked == 0 {
                        if self.detect_coding_iso_2022(coding, &mut info)? {
                            if info.rejected & category_mask(super::CAT_ISO_7_ELSE) == 0 {
                                src = src_end;
                                coding.head_ascii = src as i64;
                            }
                            info.rejected |= !CATEGORY_MASK_ISO_ESCAPE;
                            break;
                        }
                    } else if c == 0 && !inhibit_nbd {
                        null_byte_found = true;
                        if eight_bit_found {
                            break;
                        }
                    } else if !disable_ascii_optimization && !inhibit_eol {
                        if c == b'\r' {
                            if src < src_end && coding.source.get(src + 1) == Some(&b'\n') {
                                coding.eol_seen |= EOL_SEEN_CRLF;
                                src += 1;
                                if !eight_bit_found {
                                    coding.head_ascii += 1;
                                }
                            } else {
                                coding.eol_seen |= EOL_SEEN_CR;
                            }
                        } else if c == b'\n' {
                            coding.eol_seen |= EOL_SEEN_LF;
                        }
                    }
                    if !eight_bit_found {
                        coding.head_ascii += 1;
                    }
                } else if !eight_bit_found {
                    coding.head_ascii += 1;
                }
                src += 1;
            }
            if null_byte_found
                || eight_bit_found
                || coding.head_ascii < coding.src_bytes as i64
                || info.found != 0
            {
                let mut chosen: Option<usize> = None;
                if coding.head_ascii == coding.src_bytes as i64 {
                    for i in 0..CAT_RAW_TEXT {
                        let category = self.codings.priorities[i];
                        if info.found & category_mask(category) != 0 {
                            chosen = Some(category);
                            break;
                        }
                    }
                } else {
                    if null_byte_found {
                        info.checked |= !CATEGORY_MASK_UTF_16;
                        info.rejected |= !CATEGORY_MASK_UTF_16;
                    } else if prefer_utf_8 && self.detect_coding_utf_8(coding, &mut info)? {
                        info.checked |= !CATEGORY_MASK_UTF_8;
                        info.rejected |= !CATEGORY_MASK_UTF_8;
                    }
                    for i in 0..CAT_RAW_TEXT {
                        let category = self.codings.priorities[i];
                        let this_id = self.codings.categories[category].id;
                        if this_id < 0 {
                            info.rejected |= category_mask(category);
                        } else if category >= CAT_RAW_TEXT {
                            continue;
                        } else if info.checked & category_mask(category) != 0 {
                            if info.found & category_mask(category) != 0 {
                                chosen = Some(category);
                                break;
                            }
                        } else if self.run_detector(coding, &mut info, category)?
                            && info.found & category_mask(category) != 0
                        {
                            chosen = Some(category);
                            break;
                        }
                    }
                }
                if let Some(category) = chosen {
                    let this_id = self.codings.categories[category].id;
                    if category == CAT_UTF_8_AUTO {
                        let systems = self.attr(self.coding_id_attrs(this_id), ATTR_UTF_BOM);
                        found = if systems.is_cons() {
                            if info.found & category_mask(CAT_UTF_8_SIG) != 0 {
                                self.xcar(systems)
                            } else {
                                self.xcdr(systems)
                            }
                        } else {
                            self.coding_id_name(this_id)
                        };
                    } else if category == CAT_UTF_16_AUTO {
                        let systems = self.attr(self.coding_id_attrs(this_id), ATTR_UTF_BOM);
                        if systems.is_cons() {
                            if info.found & category_mask(CAT_UTF_16_LE) != 0 {
                                found = self.xcar(systems);
                            } else if info.found & category_mask(CAT_UTF_16_BE) != 0 {
                                found = self.xcdr(systems);
                            }
                        } else {
                            found = self.coding_id_name(this_id);
                        }
                    } else {
                        found = self.coding_id_name(this_id);
                    }
                } else if null_byte_found {
                    found = sym::NO_CONVERSION;
                } else if info.rejected & CATEGORY_MASK_ANY == CATEGORY_MASK_ANY {
                    found = sym::RAW_TEXT;
                } else if info.rejected != 0 {
                    for i in 0..CAT_RAW_TEXT {
                        let category = self.codings.priorities[i];
                        if info.rejected & category_mask(category) == 0 {
                            let id = self.codings.categories[category].id;
                            found = self.coding_id_name(id);
                            break;
                        }
                    }
                }
            }
        } else if self.attr(attrs, ATTR_CATEGORY).as_fixnum() == Some(CAT_UTF_8_AUTO as i64) {
            let systems = self.attr(attrs, ATTR_UTF_BOM);
            if self.check_ascii(coding) == coding.src_bytes {
                if systems.is_cons() {
                    found = self.xcdr(systems);
                }
            } else {
                let mut info = DetectionInfo::default();
                if systems.is_cons() && self.detect_coding_utf_8(coding, &mut info)? {
                    found = if info.found & category_mask(CAT_UTF_8_SIG) != 0 {
                        self.xcar(systems)
                    } else {
                        self.xcdr(systems)
                    };
                }
            }
        } else if self.attr(attrs, ATTR_CATEGORY).as_fixnum() == Some(CAT_UTF_16_AUTO as i64) {
            let systems = self.attr(attrs, ATTR_UTF_BOM);
            coding.head_ascii = 0;
            if systems.is_cons() {
                let mut info = DetectionInfo::default();
                if self.detect_coding_utf_16(coding, &mut info)? {
                    if info.found & category_mask(CAT_UTF_16_LE) != 0 {
                        found = self.xcar(systems);
                    } else if info.found & category_mask(CAT_UTF_16_BE) != 0 {
                        found = self.xcdr(systems);
                    }
                }
            }
        }
        if !found.is_nil() {
            let specified_eol = if self.is_vector(eol_type) {
                EOL_SEEN_NONE
            } else if eol_type == sym::DOS {
                EOL_SEEN_CRLF
            } else if eol_type == sym::MAC {
                EOL_SEEN_CR
            } else {
                EOL_SEEN_LF
            };
            let fresh = self.setup_coding_system(found)?;
            reset_coding_identity(coding, fresh);
            if specified_eol != EOL_SEEN_NONE {
                self.adjust_coding_eol_type(coding, specified_eol)?;
            }
        }
        coding.mode = saved_mode;
        Ok(())
    }

    fn dst_byte(&self, coding: &CodingSystem, pos_byte: usize) -> u8 {
        if coding.dst_object.is_nil() {
            coding.destination[pos_byte]
        } else {
            self.cur().text.byte_at(pos_byte)
        }
    }

    pub fn decode_eol(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let mut eol_type = self.coding_id_eol_type(coding.id);
        if eol_type == sym::UNIX || self.inhibit_eol_conversion() {
            return Ok(());
        }
        let raw = coding.dst_object.is_nil();
        let pbeg = if raw { 0 } else { coding.dst_pos_byte };
        let pend = pbeg + coding.produced;
        if self.is_vector(eol_type) {
            let mut eol_seen = EOL_SEEN_NONE;
            let mut p = pbeg;
            while p < pend {
                let b = self.dst_byte(coding, p);
                if b == b'\n' {
                    eol_seen |= EOL_SEEN_LF;
                } else if b == b'\r' {
                    if p + 1 < pend && self.dst_byte(coding, p + 1) == b'\n' {
                        eol_seen |= EOL_SEEN_CRLF;
                        p += 1;
                    } else {
                        eol_seen |= EOL_SEEN_CR;
                    }
                }
                p += 1;
            }
            if eol_seen & EOL_SEEN_CRLF != 0
                && eol_seen & EOL_SEEN_CR != 0
                && eol_seen & EOL_SEEN_LF == 0
            {
                eol_seen = EOL_SEEN_CRLF;
            } else if eol_seen != EOL_SEEN_NONE
                && eol_seen != EOL_SEEN_LF
                && eol_seen != EOL_SEEN_CRLF
                && eol_seen != EOL_SEEN_CR
            {
                eol_seen = EOL_SEEN_LF;
            }
            if eol_seen != EOL_SEEN_NONE {
                eol_type = self.adjust_coding_eol_type(coding, eol_seen)?;
            }
        }
        if eol_type == sym::MAC {
            for p in pbeg..pend {
                if self.dst_byte(coding, p) == b'\r' {
                    if raw {
                        coding.destination[p] = b'\n';
                    } else {
                        self.cur_mut().text.set_byte(p, b'\n');
                    }
                }
            }
        } else if eol_type == sym::DOS {
            if raw {
                let mut out = Vec::with_capacity(coding.produced);
                let mut n = 0;
                let data = &coding.destination;
                let mut i = 0;
                let last = coding.produced;
                while i < last {
                    if data[i] == b'\r' && i + 1 < last && data[i + 1] == b'\n' {
                        n += 1;
                    } else {
                        out.push(data[i]);
                    }
                    i += 1;
                }
                coding.destination = out;
                coding.produced -= n;
                coding.produced_char -= n;
                return Ok(());
            }
            let mut n = 0;
            let mut pos = coding.dst_pos;
            let mut pos_byte = coding.dst_pos_byte;
            let mut pos_end = pos_byte + coding.produced;
            pos_end = pos_end.saturating_sub(1);
            while pos_byte < pos_end {
                let b = self.cur().text.byte_at(pos_byte);
                let incr = if coding.dst_multibyte {
                    bytes_by_char_head(b)
                } else {
                    1
                };
                if b == b'\r' && self.cur().text.byte_at(pos_byte + 1) == b'\n' {
                    self.del_range_2(pos, pos + 1);
                    n += 1;
                    pos_end -= 1;
                }
                pos += 1;
                pos_byte += incr;
            }
            coding.produced -= n;
            coding.produced_char -= n;
            if coding.mode & MODE_LAST_BLOCK == 0
                && coding.produced > 0
                && self.cur().text.byte_at(pos_end) == b'\r'
            {
                let attrs = self.coding_id_attrs(coding.id);
                if self.attr(attrs, ATTR_TYPE) == sym::UTF_16 && coding.carryover.len() <= 1 {
                    if coding.utf_16_big_endian {
                        coding.carryover.push(0);
                        coding.carryover.push(b'\r');
                    } else {
                        coding.carryover.push(b'\r');
                        coding.carryover.push(0);
                    }
                    coding.consumed -= 2;
                    coding.consumed_char -= 1;
                } else if coding.carryover.is_empty() {
                    coding.carryover.push(b'\r');
                    coding.consumed -= 1;
                    coding.consumed_char -= 1;
                }
                coding.produced -= 1;
                coding.produced_char -= 1;
                let pos = coding.dst_pos + coding.produced_char;
                self.del_range_2(pos, pos + 1);
                self.record_conversion_result(coding, CodingResult::InsufficientSrc);
            }
        }
        Ok(())
    }

    pub fn get_translation_table(
        &mut self,
        attrs: Value,
        encodep: bool,
    ) -> LispResult<(Value, usize)> {
        if self.bound_var(sym::ENABLE_CHARACTER_TRANSLATION).is_nil() {
            return Ok((Value::NIL, 0));
        }
        let (mut table, standard) = if encodep {
            (
                self.attr(attrs, ATTR_ENCODE_TBL),
                self.bound_var(sym::STANDARD_TRANSLATION_TABLE_FOR_ENCODE),
            )
        } else {
            (
                self.attr(attrs, ATTR_DECODE_TBL),
                self.bound_var(sym::STANDARD_TRANSLATION_TABLE_FOR_DECODE),
            )
        };
        if table.is_nil() {
            table = standard;
        } else {
            if table.is_symbol() {
                table = self.get(table, sym::TRANSLATION_TABLE);
            } else if table.is_cons() {
                table = crate::fns::f_copy_sequence(self, &[table])?;
                let mut val = table;
                while val.is_cons() {
                    let e = self.xcar(val);
                    if e.is_symbol() {
                        let t = self.get(e, sym::TRANSLATION_TABLE);
                        self.heap.set_car(val, t);
                    }
                    val = self.xcdr(val);
                }
            }
            if self.is_char_table(standard) {
                if table.is_cons() {
                    let cell = self.list(&[standard]);
                    table = self.nconc2(table, cell);
                } else {
                    table = self.list(&[table, standard]);
                }
            }
        }
        let mut max_lookup = 1usize;
        let mut consider = |lisp: &Lisp, t: Value| {
            if lisp.is_char_table(t) {
                let extras = &lisp.char_table(t).extras;
                if extras.len() > 1
                    && let Some(n) = extras[1].as_fixnum()
                    && n >= 0
                    && (max_lookup as i64) < n
                {
                    max_lookup = n.min(MAX_LOOKUP_MAX) as usize;
                }
            }
        };
        if self.is_char_table(table) {
            consider(self, table);
        } else if table.is_cons() {
            let mut tail = table;
            while tail.is_cons() {
                consider(self, self.xcar(tail));
                tail = self.xcdr(tail);
            }
        }
        Ok((table, max_lookup))
    }

    fn lookup_translation_table(&self, table: Value, c: &mut i32) -> Value {
        let mut trans = Value::NIL;
        let is_char = |v: Value| {
            v.as_fixnum()
                .is_some_and(|n| (0..=character::MAX_CHAR as i64).contains(&n))
        };
        if self.is_char_table(table) {
            trans = self.char_table_ref(table, *c as u32);
            if is_char(trans) {
                *c = trans.as_fixnum().unwrap_or(0) as i32;
                trans = Value::NIL;
            }
        } else if table.is_cons() {
            let mut tail = table;
            while tail.is_cons() {
                let t = self.xcar(tail);
                if self.is_char_table(t) {
                    trans = self.char_table_ref(t, *c as u32);
                    if is_char(trans) {
                        *c = trans.as_fixnum().unwrap_or(0) as i32;
                        trans = Value::NIL;
                    } else if !trans.is_nil() {
                        break;
                    }
                }
                tail = self.xcdr(tail);
            }
        }
        trans
    }

    fn get_translation(&self, trans: Value, buf: &[i32]) -> (Value, usize) {
        if trans.is_fixnum() || self.is_vector(trans) {
            return (trans, 1);
        }
        let mut t = trans;
        while t.is_cons() {
            let val = self.xcar(t);
            let from = self.xcar(val);
            let items = self.vector_items(from);
            let len = items.len();
            let mut i = 0;
            while i < len {
                if i == buf.len() {
                    return (sym::T, 0);
                }
                if items[i].as_fixnum().unwrap_or(-1) != buf[i] as i64 {
                    break;
                }
                i += 1;
            }
            if i == len {
                return (self.xcdr(val), len);
            }
            t = self.xcdr(t);
        }
        (Value::NIL, 0)
    }

    fn produce_chars(
        &mut self,
        coding: &mut CodingSystem,
        translation_table: Value,
        last_block: bool,
    ) -> LispResult<usize> {
        let mut out: Vec<u8> = Vec::new();
        let mut produced_chars = 0usize;
        let mut carryover = 0usize;
        if !coding.chars_at_source {
            let buf_end = coding.charbuf.len();
            let mut i = 0usize;
            while i < buf_end {
                let mut c = coding.charbuf[i];
                if c >= 0 {
                    let mut from_nchars = 1usize;
                    let mut to_nchars = 1usize;
                    let mut trans = self.lookup_translation_table(translation_table, &mut c);
                    if !trans.is_nil() {
                        let (t, n) = self.get_translation(trans, &coding.charbuf[i..buf_end]);
                        trans = t;
                        if n > 0 {
                            from_nchars = n;
                        }
                        if let Some(v) = trans.as_fixnum() {
                            c = v as i32;
                        } else if self.is_vector(trans) {
                            to_nchars = self.vector_items(trans).len();
                            c = self.vector_items(trans)[0].as_fixnum().unwrap_or(0) as i32;
                        } else if trans == sym::T && !last_block {
                            break;
                        }
                    }
                    for k in 0..to_nchars {
                        if k > 0 {
                            c = self.vector_items(trans)[k].as_fixnum().unwrap_or(0) as i32;
                        }
                        if coding.dst_multibyte || !character::char_byte8_p(c as u32) {
                            push_char_string(&mut out, c);
                        } else {
                            out.push(character::char_to_byte8(c as u32));
                        }
                    }
                    produced_chars += to_nchars;
                    i += from_nchars;
                } else {
                    i += (-c) as usize;
                }
            }
            carryover = buf_end.saturating_sub(i);
        } else {
            let src = &coding.source[..coding.consumed];
            if coding.src_multibyte != coding.dst_multibyte {
                if coding.src_multibyte {
                    let mut p = 0;
                    while p < src.len() {
                        let mut c = src[p] as i32;
                        p += 1;
                        if c & 0x80 != 0 {
                            if c & 0xFE == 0xC0 {
                                c = ((c & 1) << 6) | src.get(p).copied().unwrap_or(0) as i32;
                                p += 1;
                            } else {
                                p -= 1;
                                let (ch, len) = character::string_char(&src[p..]);
                                p += len;
                                c = -(ch as i32);
                            }
                        }
                        out.push(c as u8);
                        produced_chars += 1;
                    }
                } else {
                    let mut emit = Emit::new(true);
                    for &b in src {
                        emit.one(&mut out, b as i32);
                    }
                    produced_chars = emit.produced_chars;
                }
            } else {
                produced_chars = coding.consumed_char;
                out.extend_from_slice(src);
            }
        }
        let produced = out.len();
        if self.is_buffer(coding.dst_object) {
            if produced_chars > 0 {
                let pos = coding.dst_pos + coding.produced_char;
                let pos_byte = coding.dst_pos_byte + coding.produced;
                self.insert_from_gap(
                    pos,
                    pos_byte,
                    &out,
                    produced_chars,
                    coding.insert_before_markers,
                );
            }
        } else {
            coding.destination.extend_from_slice(&out);
        }
        coding.produced += produced;
        coding.produced_char += produced_chars;
        Ok(carryover)
    }

    fn produce_composition(
        &mut self,
        coding: &CodingSystem,
        at: usize,
        pos: usize,
    ) -> LispResult<()> {
        let cb = &coding.charbuf;
        let mut len = -cb[at] - MAX_ANNOTATION_LENGTH as i32;
        let to = pos + cb[at + 2] as usize;
        let method = cb[at + 4];
        let components = if method == COMPOSITION_RELATIVE {
            Value::NIL
        } else {
            if method == COMPOSITION_WITH_RULE {
                len = cb[at + 2] * 3 - 2;
            }
            let base = at + MAX_ANNOTATION_LENGTH;
            let mut args = Vec::new();
            let mut i = 0i32;
            let mut j = 0i32;
            while i < len && cb[base + i as usize] != -1 {
                let v = cb[base + i as usize];
                if v >= 0 {
                    args.push(Value::fixnum(v as i64));
                } else {
                    i += 1;
                    args.push(Value::fixnum((cb[base + i as usize] % 0x100) as i64));
                }
                i += 1;
                j += 1;
            }
            if i == j {
                crate::charfns::f_string(self, &args)?
            } else {
                self.vector(args)
            }
        };
        self.compose_text(pos, to, components, Value::NIL, coding.dst_object)
    }

    fn produce_annotation(&mut self, coding: &CodingSystem, mut pos: usize) -> LispResult<()> {
        if coding.dst_object.is_nil() {
            return Ok(());
        }
        let end = coding.charbuf.len();
        let mut i = 0;
        while i < end {
            let c = coding.charbuf[i];
            if c >= 0 {
                pos += 1;
                i += 1;
            } else {
                let len = (-c) as usize;
                if len > 2 {
                    match coding.charbuf[i + 1] as u32 {
                        ANNOTATE_COMPOSITION_MASK => self.produce_composition(coding, i, pos)?,
                        ANNOTATE_CHARSET_MASK => {
                            let from = pos - coding.charbuf[i + 2] as usize;
                            let name = self.charset_name(coding.charbuf[i + 3]);
                            crate::textprop::f_put_text_property(
                                self,
                                &[
                                    Value::fixnum(from as i64),
                                    Value::fixnum(pos as i64),
                                    sym::CHARSET,
                                    name,
                                    coding.dst_object,
                                ],
                            )?;
                        }
                        _ => {}
                    }
                }
                i += len;
            }
        }
        Ok(())
    }

    pub fn decode_coding(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let count = self.specpdl.len();
        let mut undo_list = sym::T;
        if self.is_buffer(coding.dst_object) {
            self.set_buffer_internal(coding.dst_object);
            if self.cur().text.modiff <= self.cur().text.save_modiff {
                self.record_first_change();
            }
            undo_list = self.undo_list();
            self.stack.push(undo_list);
            self.set_undo_list(sym::T);
            self.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
        }
        let r = self.decode_coding_1(coding);
        if self.is_buffer(coding.dst_object) && r.is_ok() {
            self.set_undo_list(undo_list);
            self.record_insert(coding.dst_pos, coding.produced_char);
        }
        if self.is_buffer(coding.dst_object) {
            self.stack.pop();
        }
        let u = self.unbind_to(count);
        r?;
        u
    }

    fn decode_coding_1(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        coding.consumed = 0;
        coding.consumed_char = 0;
        coding.produced = 0;
        coding.produced_char = 0;
        coding.chars_at_source = false;
        self.record_conversion_result(coding, CodingResult::Success);
        coding.charbuf_size = (coding.src_bytes + MAX_CHARBUF_EXTRA_SIZE).min(MAX_CHARBUF_SIZE);
        coding.charbuf = Vec::with_capacity(coding.charbuf_size);
        let attrs = self.coding_id_attrs(coding.id);
        let (translation_table, _) = self.get_translation_table(attrs, false)?;
        self.stack.push(translation_table);
        if coding.kind == CodingType::Ccl {
            let mut program = crate::ccl::CclProgram::default();
            let decoder = self.attr(attrs, ATTR_CCL_DECODER);
            self.setup_ccl_program(&mut program, decoder)?;
            coding.ccl = Some(Box::new(program));
        }
        let mut carryover = 0usize;
        loop {
            let pos = coding.dst_pos + coding.produced_char;
            coding.annotated = false;
            let used = coding.charbuf.len();
            coding.charbuf.drain(..used - carryover);
            self.run_decoder(coding)?;
            carryover = self.produce_chars(coding, translation_table, false)?;
            if coding.annotated {
                self.produce_annotation(coding, pos)?;
            }
            let more = coding.result == CodingResult::InsufficientDst
                || (coding.consumed < coding.src_bytes
                    && matches!(
                        coding.result,
                        CodingResult::Success | CodingResult::InvalidSrc
                    ));
            if !more {
                break;
            }
        }
        if carryover > 0 {
            let used = coding.charbuf.len();
            coding.charbuf.drain(..used - carryover);
            self.produce_chars(coding, translation_table, true)?;
        }
        coding.carryover.clear();
        if coding.consumed < coding.src_bytes {
            let mut nbytes = coding.src_bytes - coding.consumed;
            let mut src = coding.consumed;
            if coding.mode & MODE_LAST_BLOCK != 0 {
                coding.charbuf.clear();
                coding.chars_at_source = false;
                while nbytes > 0 {
                    nbytes -= 1;
                    let b = coding.source[src];
                    let c = if coding.src_multibyte && (b == 0xC0 || b == 0xC1) && nbytes > 0 {
                        let (ch, len) =
                            character::string_char(&coding.source[src..coding.src_bytes]);
                        src += len;
                        nbytes -= len - 1;
                        ch as i32
                    } else {
                        src += 1;
                        if b & 0x80 != 0 {
                            character::byte8_to_char(b) as i32
                        } else {
                            b as i32
                        }
                    };
                    coding.charbuf.push(c);
                }
                self.produce_chars(coding, Value::NIL, true)?;
            } else {
                let n = nbytes.min(64);
                coding.carryover = coding.source[src..src + n].to_vec();
            }
            coding.consumed = coding.src_bytes;
        }
        if self.coding_id_eol_type(coding.id) != sym::UNIX && !self.inhibit_eol_conversion() {
            self.decode_eol(coding)?;
        }
        self.stack.pop();
        Ok(())
    }

    fn handle_charset_annotation(
        &mut self,
        coding: &mut CodingSystem,
        pos: usize,
        limit: usize,
    ) -> LispResult<usize> {
        let val = crate::textprop::f_get_text_property(
            self,
            &[Value::fixnum(pos as i64), sym::CHARSET, coding.src_object],
        )?;
        let id = if !val.is_nil() && self.charsetp(val) {
            self.check_charset_get_id(val)?
        } else {
            -1
        };
        add_charset_data(coding, 0, id);
        let next = crate::textprop::f_next_single_property_change(
            self,
            &[
                Value::fixnum(pos as i64),
                sym::CHARSET,
                coding.src_object,
                Value::fixnum(limit as i64),
            ],
        )?;
        Ok(next.as_fixnum().unwrap_or(limit as i64) as usize)
    }

    fn consume_chars(
        &mut self,
        coding: &mut CodingSystem,
        translation_table: Value,
        max_lookup: usize,
    ) -> LispResult<()> {
        coding.charbuf.clear();
        let buf_end = coding
            .charbuf_size
            .saturating_sub(1 + MAX_ANNOTATION_LENGTH);
        let mut src = coding.consumed;
        let src_end = coding.src_bytes;
        let mut pos = (coding.src_pos + coding.consumed_char as i64) as usize;
        let end_pos = (coding.src_pos + coding.src_chars as i64) as usize;
        let multibytep = coding.src_multibyte;
        let mut eol_type = if self.inhibit_eol_conversion() {
            sym::UNIX
        } else {
            self.coding_id_eol_type(coding.id)
        };
        if self.is_vector(eol_type) {
            eol_type = sym::UNIX;
        }
        coding.common_flags &= !ANNOTATE_COMPOSITION_MASK;
        let (mut stop, stop_composition, mut stop_charset);
        if coding.src_object.is_nil() {
            stop = end_pos;
            stop_composition = end_pos;
            stop_charset = end_pos;
        } else {
            stop_composition = end_pos;
            stop = end_pos;
            if coding.common_flags & ANNOTATE_CHARSET_MASK != 0 {
                stop = pos;
                stop_charset = pos;
            } else {
                stop_charset = end_pos;
            }
        }
        let raw_or_ccl = matches!(
            coding.kind,
            CodingType::RawText | CodingType::Undecided | CodingType::Ccl
        );
        while coding.charbuf.len() < buf_end {
            if pos == stop {
                if pos == end_pos {
                    break;
                }
                if pos == stop_charset {
                    stop_charset = self.handle_charset_annotation(coding, pos, end_pos)?;
                }
                stop = stop_composition.min(stop_charset);
            }
            let source = &coding.source;
            let mut c: i32;
            if !multibytep {
                if raw_or_ccl {
                    c = source[src] as i32;
                    src += 1;
                    pos += 1;
                } else {
                    match character::multibyte_length(&source[src..src_end]) {
                        Some(bytes) => {
                            let (ch, len) = character::string_char(&source[src..src_end]);
                            c = ch as i32;
                            src += len;
                            pos += bytes;
                        }
                        None => {
                            c = character::byte8_to_char(source[src]) as i32;
                            src += 1;
                            pos += 1;
                        }
                    }
                }
            } else {
                let (ch, len) = character::string_char(&source[src..src_end]);
                c = ch as i32;
                src += len;
                pos += 1;
            }
            if c == b'\r' as i32 && coding.mode & MODE_SELECTIVE_DISPLAY != 0 {
                c = b'\n' as i32;
            }
            if eol_type != sym::UNIX && c == b'\n' as i32 {
                if eol_type == sym::DOS {
                    coding.charbuf.push(b'\r' as i32);
                } else {
                    c = b'\r' as i32;
                }
            }
            let trans = self.lookup_translation_table(translation_table, &mut c);
            if trans.is_nil() {
                coding.charbuf.push(c);
                continue;
            }
            let mut lookup = vec![c];
            let mut p = src;
            while lookup.len() < max_lookup && p < src_end {
                let (ch, len) = character::string_char(&coding.source[p..src_end]);
                lookup.push(ch as i32);
                p += len;
            }
            let (trans, n) = self.get_translation(trans, &lookup);
            let from_nchars = n.max(1);
            let mut to_nchars = 1;
            if let Some(v) = trans.as_fixnum() {
                c = v as i32;
            } else if self.is_vector(trans) {
                to_nchars = self.vector_items(trans).len();
                if buf_end - coding.charbuf.len() < to_nchars {
                    break;
                }
                c = self.vector_items(trans)[0].as_fixnum().unwrap_or(0) as i32;
            } else {
                break;
            }
            coding.charbuf.push(c);
            for k in 1..to_nchars {
                let v = self.vector_items(trans)[k].as_fixnum().unwrap_or(0) as i32;
                coding.charbuf.push(v);
            }
            for _ in 1..from_nchars {
                src += character::multibyte_length(&coding.source[src..src_end]).unwrap_or(1);
                pos += 1;
            }
        }
        coding.consumed = src;
        coding.consumed_char = (pos as i64 - coding.src_pos) as usize;
        coding.chars_at_source = false;
        Ok(())
    }

    pub fn encode_coding(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let attrs = self.coding_id_attrs(coding.id);
        let (translation_table, max_lookup) =
            if matches!(coding.kind, CodingType::RawText | CodingType::Undecided) {
                (Value::NIL, 0)
            } else {
                self.get_translation_table(attrs, true)?
            };
        self.stack.push(translation_table);
        if self.is_buffer(coding.dst_object) {
            self.set_buffer_internal(coding.dst_object);
            coding.dst_multibyte = self.cur().text.multibyte;
        }
        coding.consumed = 0;
        coding.consumed_char = 0;
        coding.produced = 0;
        coding.produced_char = 0;
        coding.destination.clear();
        self.record_conversion_result(coding, CodingResult::Success);
        coding.charbuf_size = (coding.src_chars + MAX_CHARBUF_EXTRA_SIZE).min(MAX_CHARBUF_SIZE);
        coding.charbuf = Vec::with_capacity(coding.charbuf_size);
        if coding.kind == CodingType::Ccl {
            let mut program = crate::ccl::CclProgram::default();
            let encoder = self.attr(attrs, ATTR_CCL_ENCODER);
            self.setup_ccl_program(&mut program, encoder)?;
            coding.ccl = Some(Box::new(program));
        }
        loop {
            self.consume_chars(coding, translation_table, max_lookup)?;
            let saved_mode = coding.mode;
            if coding.consumed_char < coding.src_chars {
                coding.mode &= !MODE_LAST_BLOCK;
            }
            self.run_encoder(coding)?;
            coding.mode = saved_mode;
            if coding.consumed_char >= coding.src_chars {
                break;
            }
        }
        if self.is_buffer(coding.dst_object) && coding.produced_char > 0 {
            let bytes = std::mem::take(&mut coding.destination);
            self.insert_from_gap(
                coding.dst_pos,
                coding.dst_pos_byte,
                &bytes,
                coding.produced_char,
                coding.insert_before_markers,
            );
            coding.destination = bytes;
        }
        self.stack.pop();
        Ok(())
    }

    pub fn code_conversion_save(&mut self, with_work_buf: bool, multibyte: bool) -> LispResult {
        let mut workbuf = Value::NIL;
        if with_work_buf {
            if self.codings.reused_workbuf_in_use {
                let base = self.codings.workbuf_name;
                let name = self.generate_new_buffer_name(base, Value::NIL)?;
                workbuf = self.get_buffer_create(name, true)?;
            } else {
                let reused = self.codings.reused_workbuf;
                if !self.is_buffer(reused) || !self.buffer_live_p(reused) {
                    let name = self.codings.workbuf_name;
                    self.codings.reused_workbuf = self.get_buffer_create(name, true)?;
                }
                workbuf = self.codings.reused_workbuf;
            }
        }
        let current = self.current_buffer;
        let arg = self.cons(current, workbuf);
        self.record_unwind(code_conversion_restore, arg);
        if !workbuf.is_nil() {
            self.set_buffer_internal(workbuf);
            if !self.local_binding_exists(sym::INHIBIT_MODIFICATION_HOOKS) {
                let f = self.intern("make-local-variable");
                self.funcall(f, &[sym::INHIBIT_MODIFICATION_HOOKS])?;
            }
            self.set(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
            self.widen();
            let z = self.cur().z();
            self.del_range(1, z)?;
            self.set_undo_list(sym::T);
            self.cur_mut().text.multibyte = multibyte;
            let flag = self.bool(multibyte);
            self.buffer_local_set(workbuf, sym::ENABLE_MULTIBYTE_CHARACTERS, flag);
            if workbuf == self.codings.reused_workbuf {
                self.codings.reused_workbuf_in_use = true;
            }
            self.set_buffer_internal(current);
        }
        Ok(workbuf)
    }

    fn snapshot_source(
        &mut self,
        coding: &mut CodingSystem,
        object: Value,
        from_byte: usize,
        to_byte: usize,
    ) {
        if object.is_string() {
            coding.source = self.string_bytes(object)[from_byte..to_byte].to_vec();
        } else if self.is_buffer(object) {
            coding.source = self.buffer(object).text.slice(from_byte - 1, to_byte - 1);
        }
    }

    fn markers_needing_adjustment(&self, buffer: Value, from: usize, to: usize) -> Vec<Value> {
        self.buffer(buffer)
            .text
            .markers
            .iter()
            .copied()
            .filter(|&m| {
                let mk = self.marker(m);
                mk.charpos == if mk.insertion_type { from } else { to }
            })
            .collect()
    }

    fn restore_point_and_markers(
        &mut self,
        coding: &CodingSystem,
        src_object: Value,
        saved: (usize, usize),
        range: (usize, usize, usize, usize),
        markers: &[Value],
    ) {
        let (from, from_byte, chars, bytes) = range;
        let (saved_pt, saved_pt_byte) = saved;
        self.set_buffer_internal(src_object);
        let multibyte = self.cur().text.multibyte;
        let (pt, pt_byte) = if saved_pt < from {
            (saved_pt, saved_pt_byte)
        } else if saved_pt < from + chars {
            (from, from_byte)
        } else if multibyte {
            (
                (saved_pt as i64 + coding.produced_char as i64 - chars as i64) as usize,
                (saved_pt_byte as i64 + coding.produced as i64 - bytes as i64) as usize,
            )
        } else {
            let d = coding.produced as i64 - bytes as i64;
            (
                (saved_pt as i64 + d) as usize,
                (saved_pt_byte as i64 + d) as usize,
            )
        };
        let b = self.cur_mut();
        b.pt = pt;
        b.pt_byte = pt_byte;
        for &m in markers {
            let insertion_type = self.marker(m).insertion_type;
            let mk = self.marker_mut(m);
            if insertion_type {
                mk.bytepos = from_byte;
                mk.charpos = from;
            } else {
                mk.bytepos = from_byte + coding.produced;
                mk.charpos = if multibyte {
                    from + coding.produced_char
                } else {
                    mk.bytepos
                };
            }
        }
    }

    pub fn decode_coding_gap(
        &mut self,
        coding: &mut CodingSystem,
        source: Vec<u8>,
    ) -> LispResult<()> {
        let count = self.specpdl.len();
        let r = self.decode_coding_gap_1(coding, source);
        let u = self.unbind_to(count);
        r?;
        u
    }

    fn decode_coding_gap_1(
        &mut self,
        coding: &mut CodingSystem,
        source: Vec<u8>,
    ) -> LispResult<()> {
        let mut bytes = source.len();
        let buffer = self.current_buffer;
        coding.src_object = buffer;
        coding.src_chars = bytes;
        coding.src_bytes = bytes;
        coding.src_pos = -(bytes as i64);
        coding.src_pos_byte = -(bytes as i64);
        coding.src_multibyte = false;
        coding.source = source;
        coding.dst_object = buffer;
        coding.dst_pos = self.cur().pt;
        coding.dst_pos_byte = self.cur().pt_byte;
        coding.head_ascii = -1;
        coding.detected_utf8_bytes = -1;
        coding.detected_utf8_chars = -1;
        coding.eol_seen = EOL_SEEN_NONE;
        if coding.require_detection() {
            self.detect_coding(coding)?;
        }
        let attrs = self.coding_id_attrs(coding.id);
        if !self.var_true(sym::DISABLE_ASCII_OPTIMIZATION)
            && !coding.src_multibyte
            && !self.attr(attrs, ATTR_ASCII_COMPAT).is_nil()
            && self.attr(attrs, ATTR_POST_READ).is_nil()
            && self.get_translation_table(attrs, false)?.0.is_nil()
        {
            let mut chars = coding.head_ascii;
            if chars < 0 {
                chars = self.check_ascii(coding) as i64;
            }
            if chars != bytes as i64 {
                if self.attr(attrs, ATTR_TYPE) == sym::UTF_8
                    && coding.detected_utf8_bytes == coding.src_bytes as i64
                {
                    chars = if coding.detected_utf8_chars >= 0 {
                        coding.detected_utf8_chars
                    } else {
                        self.check_utf_8(coding)
                    };
                    if coding.utf_8_bom != super::Bom::Without
                        && coding.head_ascii == 0
                        && coding.source.starts_with(&[0xEF, 0xBB, 0xBF])
                    {
                        chars -= 1;
                        bytes -= 3;
                        coding.src_bytes -= 3;
                    }
                } else {
                    chars = -1;
                }
            }
            if chars >= 0 {
                let mut eol_type = self.coding_id_eol_type(coding.id);
                if self.is_vector(eol_type) && coding.eol_seen != EOL_SEEN_NONE {
                    let seen = coding.eol_seen;
                    eol_type = self.adjust_coding_eol_type(coding, seen)?;
                }
                let start = coding.source.len() - coding.src_bytes;
                let mut data = coding.source[start..].to_vec();
                if eol_type == sym::MAC {
                    for b in data.iter_mut() {
                        if *b == b'\r' {
                            *b = b'\n';
                        }
                    }
                } else if eol_type == sym::DOS {
                    let mut out = Vec::with_capacity(data.len());
                    let mut src = data.len();
                    while src > 0 {
                        src -= 1;
                        out.push(data[src]);
                        if data[src] == b'\n' && src > 0 && data[src - 1] == b'\r' {
                            src -= 1;
                        }
                    }
                    out.reverse();
                    let diff = data.len() - out.len();
                    bytes -= diff;
                    chars -= diff as i64;
                    data = out;
                }
                coding.produced = bytes;
                coding.produced_char = chars as usize;
                let (pt, pt_byte) = (self.cur().pt, self.cur().pt_byte);
                let before_markers = coding.insert_before_markers;
                self.insert_from_gap(pt, pt_byte, &data, chars as usize, before_markers);
                return Ok(());
            }
        }
        self.code_conversion_save(false, false)?;
        coding.mode |= MODE_LAST_BLOCK;
        self.decode_coding(coding)?;
        let post_read = self.attr(attrs, ATTR_POST_READ);
        if !post_read.is_nil() {
            let prev_z = self.cur().z();
            let prev_z_byte = self.cur().z_byte();
            let undo_list = self.undo_list();
            let current = self.current_buffer;
            let arg = self.cons(undo_list, current);
            self.record_unwind(coding_restore_undo_list, arg);
            self.set_undo_list(sym::T);
            let b = self.cur_mut();
            b.pt = coding.dst_pos;
            b.pt_byte = coding.dst_pos_byte;
            let n = Value::fixnum(coding.produced_char as i64);
            let val = self.funcall(post_read, &[n])?;
            self.check_fixnat(val)?;
            let z = self.cur().z();
            let z_byte = self.cur().z_byte();
            coding.produced_char =
                (coding.produced_char as i64 + z as i64 - prev_z as i64) as usize;
            coding.produced =
                (coding.produced as i64 + z_byte as i64 - prev_z_byte as i64) as usize;
        }
        Ok(())
    }

    pub fn decode_coding_object(
        &mut self,
        coding: &mut CodingSystem,
        src_object: Value,
        range: (usize, usize, usize, usize),
        dst_object: Value,
    ) -> LispResult<()> {
        let base = self.stack.len();
        self.stack.push(src_object);
        self.stack.push(dst_object);
        let count = self.specpdl.len();
        let r = self.decode_coding_object_1(coding, src_object, range, dst_object);
        self.stack.push(coding.dst_object);
        let u = self.unbind_to(count);
        self.stack.truncate(base);
        r?;
        u
    }

    fn decode_coding_object_1(
        &mut self,
        coding: &mut CodingSystem,
        src_object: Value,
        range: (usize, usize, usize, usize),
        dst_object: Value,
    ) -> LispResult<()> {
        let (from, from_byte, to, to_byte) = range;
        let chars = to - from;
        let bytes = to_byte - from_byte;
        let old_deactivate_mark = self.bound_var(sym::DEACTIVATE_MARK);
        self.stack.push(old_deactivate_mark);
        let mut saved: Option<(usize, usize)> = None;
        let mut markers = Vec::new();
        coding.src_object = src_object;
        coding.src_chars = chars;
        coding.src_bytes = bytes;
        coding.src_multibyte = chars < bytes;
        if src_object.is_string() {
            self.snapshot_source(coding, src_object, from_byte, to_byte);
            coding.src_pos = from as i64;
            coding.src_pos_byte = from_byte as i64;
        } else if self.is_buffer(src_object) {
            self.set_buffer_internal(src_object);
            self.snapshot_source(coding, src_object, from_byte, to_byte);
            if src_object == dst_object {
                markers = self.markers_needing_adjustment(src_object, from, to);
                let b = self.cur();
                saved = Some((b.pt, b.pt_byte));
                let b = self.cur_mut();
                b.pt = from;
                b.pt_byte = from_byte;
                self.prepare_to_modify_buffer(from, to)?;
                self.del_range_2(from, to);
                coding.src_pos = -(chars as i64);
                coding.src_pos_byte = -(bytes as i64);
            } else {
                coding.src_pos = from as i64;
                coding.src_pos_byte = from_byte as i64;
            }
        } else {
            coding.src_pos = from as i64;
            coding.src_pos_byte = from_byte as i64;
            coding.source = coding.source[from_byte..to_byte].to_vec();
        }
        if coding.require_detection() {
            self.detect_coding(coding)?;
        }
        let attrs = self.coding_id_attrs(coding.id);
        let post_read = self.attr(attrs, ATTR_POST_READ);
        if dst_object == sym::T || (!post_read.is_nil() && dst_object.is_nil()) {
            coding.dst_multibyte = coding.common_flags & FOR_UNIBYTE_MASK == 0;
            coding.dst_object = self.code_conversion_save(true, coding.dst_multibyte)?;
            coding.dst_pos = 1;
            coding.dst_pos_byte = 1;
        } else if self.is_buffer(dst_object) {
            if src_object != dst_object {
                let current = self.current_buffer;
                self.set_buffer_internal(dst_object);
                let pt = self.cur().pt;
                self.prepare_to_modify_buffer(pt, pt)?;
                self.set_buffer_internal(current);
            }
            self.code_conversion_save(false, false)?;
            coding.dst_object = dst_object;
            let b = self.buffer(dst_object);
            coding.dst_pos = b.pt;
            coding.dst_pos_byte = b.pt_byte;
            coding.dst_multibyte = b.text.multibyte;
        } else {
            self.code_conversion_save(false, false)?;
            coding.dst_object = Value::NIL;
            coding.dst_multibyte = true;
        }
        self.stack.push(coding.dst_object);
        self.decode_coding(coding)?;
        if self.is_buffer(coding.dst_object) {
            self.set_buffer_internal(coding.dst_object);
            let deleted = if src_object == dst_object {
                to - from
            } else {
                0
            };
            self.signal_after_change(coding.dst_pos, deleted, coding.produced_char)?;
            self.update_compositions(
                coding.dst_pos,
                coding.dst_pos + coding.produced_char,
                crate::composite::CHECK_ALL,
            )?;
        }
        if !post_read.is_nil() {
            let prev_z = self.cur().z();
            let prev_z_byte = self.cur().z_byte();
            let undo_list = self.undo_list();
            let current = self.current_buffer;
            let arg = self.cons(undo_list, current);
            let count1 = self.specpdl.len();
            self.record_unwind(coding_restore_undo_list, arg);
            self.set_undo_list(sym::T);
            let b = self.cur_mut();
            b.pt = coding.dst_pos;
            b.pt_byte = coding.dst_pos_byte;
            let n = Value::fixnum(coding.produced_char as i64);
            let val = self.safe_calln(post_read, &[n])?;
            self.check_fixnat(val)?;
            let z = self.cur().z();
            let z_byte = self.cur().z_byte();
            coding.produced_char =
                (coding.produced_char as i64 + z as i64 - prev_z as i64) as usize;
            coding.produced =
                (coding.produced as i64 + z_byte as i64 - prev_z_byte as i64) as usize;
            self.unbind_to(count1)?;
        }
        if dst_object == sym::T {
            let (begv, zv) = (self.cur().begv, self.cur().zv);
            coding.dst_object = self.buffer_substring_value(begv, zv, true);
        } else if dst_object.is_nil() && self.is_buffer(coding.dst_object) {
            self.set_buffer_internal(coding.dst_object);
            let b = self.cur();
            let begv_byte = b.begv_byte;
            coding.destination = b.text.slice(begv_byte - 1, begv_byte - 1 + coding.produced);
        }
        if let Some(saved) = saved {
            self.restore_point_and_markers(
                coding,
                src_object,
                saved,
                (from, from_byte, chars, bytes),
                &markers,
            );
        }
        self.set_plain_value(sym::DEACTIVATE_MARK, old_deactivate_mark);
        Ok(())
    }

    pub fn encode_coding_object(
        &mut self,
        coding: &mut CodingSystem,
        src_object: Value,
        range: (usize, usize, usize, usize),
        dst_object: Value,
    ) -> LispResult<()> {
        let base = self.stack.len();
        self.stack.push(src_object);
        self.stack.push(dst_object);
        let count = self.specpdl.len();
        let r = self.encode_coding_object_1(coding, src_object, range, dst_object);
        self.stack.push(coding.dst_object);
        let u = self.unbind_to(count);
        self.stack.truncate(base);
        r?;
        u
    }

    fn encode_coding_object_1(
        &mut self,
        coding: &mut CodingSystem,
        src_object: Value,
        range: (usize, usize, usize, usize),
        dst_object: Value,
    ) -> LispResult<()> {
        let (from, from_byte, to, to_byte) = range;
        let chars = to - from;
        let bytes = to_byte - from_byte;
        let old_deactivate_mark = self.bound_var(sym::DEACTIVATE_MARK);
        self.stack.push(old_deactivate_mark);
        let mut saved: Option<(usize, usize)> = None;
        let mut markers = Vec::new();
        let mut kill_src_buffer = false;
        coding.src_object = src_object;
        coding.src_chars = chars;
        coding.src_bytes = bytes;
        coding.src_multibyte = chars < bytes;
        let attrs = self.coding_id_attrs(coding.id);
        let same_buffer = src_object == dst_object && self.is_buffer(src_object);
        if same_buffer {
            markers = self.markers_needing_adjustment(src_object, from, to);
        }
        let pre_write = self.attr(attrs, ATTR_PRE_WRITE);
        if !pre_write.is_nil() {
            let work = self.code_conversion_save(true, coding.src_multibyte)?;
            coding.src_object = work;
            self.stack.push(work);
            self.set_buffer_internal(work);
            if src_object.is_string() {
                let sub = crate::fns::f_substring(
                    self,
                    &[
                        src_object,
                        Value::fixnum(from as i64),
                        Value::fixnum(to as i64),
                    ],
                )?;
                self.insert_string(sub, false, false)?;
            } else if self.is_buffer(src_object) {
                let current = self.current_buffer;
                self.set_buffer_internal(src_object);
                let text = self.buffer_substring_value(from, to, true);
                self.set_buffer_internal(current);
                self.insert_string(text, false, false)?;
            } else {
                let data = coding.source[from_byte..to_byte].to_vec();
                let s = if chars < bytes {
                    self.heap
                        .string(crate::object::LispString::multibyte(data, chars))
                } else {
                    self.unibyte_string(&data)
                };
                self.insert_string(s, false, false)?;
            }
            if same_buffer {
                self.set_buffer_internal(src_object);
                let b = self.cur();
                saved = Some((b.pt, b.pt_byte));
                self.del_range(from, to)?;
                self.set_buffer_internal(work);
            }
            let z = self.cur().z();
            self.safe_calln(pre_write, &[Value::fixnum(1), Value::fixnum(z as i64)])?;
            if work != self.current_buffer {
                kill_src_buffer = true;
            }
            coding.src_object = self.current_buffer;
            self.stack.push(coding.src_object);
            let b = self.cur();
            coding.src_chars = b.z() - 1;
            coding.src_bytes = b.z_byte() - 1;
            coding.src_pos = 1;
            coding.src_pos_byte = 1;
            coding.src_multibyte = b.z() < b.z_byte();
            let object = coding.src_object;
            let end = 1 + coding.src_bytes;
            self.snapshot_source(coding, object, 1, end);
        } else if src_object.is_string() {
            self.code_conversion_save(false, false)?;
            self.snapshot_source(coding, src_object, from_byte, to_byte);
            coding.src_pos = from as i64;
            coding.src_pos_byte = from_byte as i64;
        } else if self.is_buffer(src_object) {
            self.code_conversion_save(false, false)?;
            self.set_buffer_internal(src_object);
            if same_buffer {
                let b = self.cur();
                saved = Some((b.pt, b.pt_byte));
                self.prepare_to_modify_buffer(from, to)?;
                let deleted = self.del_range_2(from, to);
                coding.src_object = deleted;
                self.stack.push(deleted);
                self.snapshot_source(coding, deleted, 0, bytes);
                coding.src_pos = 0;
                coding.src_pos_byte = 0;
            } else {
                self.snapshot_source(coding, src_object, from_byte, to_byte);
                coding.src_pos = from as i64;
                coding.src_pos_byte = from_byte as i64;
            }
        } else {
            self.code_conversion_save(false, false)?;
            coding.source = coding.source[from_byte..to_byte].to_vec();
            coding.src_pos = from as i64;
            coding.src_pos_byte = from_byte as i64;
        }
        if self.is_buffer(dst_object) {
            coding.dst_object = dst_object;
            if src_object == dst_object {
                coding.dst_pos = from;
                coding.dst_pos_byte = from_byte;
            } else {
                let current = self.current_buffer;
                self.set_buffer_internal(dst_object);
                let pt = self.cur().pt;
                self.prepare_to_modify_buffer(pt, pt)?;
                let b = self.cur();
                coding.dst_pos = b.pt;
                coding.dst_pos_byte = b.pt_byte;
                self.set_buffer_internal(current);
            }
            coding.dst_multibyte = self.buffer(dst_object).text.multibyte;
        } else {
            coding.dst_object = Value::NIL;
            coding.dst_multibyte = false;
        }
        self.encode_coding(coding)?;
        if dst_object == sym::T {
            if !coding.raw_destination {
                coding.dst_object = self.unibyte_string(&coding.destination[..coding.produced]);
            }
        } else if self.is_buffer(coding.dst_object) {
            let current = self.current_buffer;
            self.set_buffer_internal(dst_object);
            let deleted = if same_buffer { to - from } else { 0 };
            self.signal_after_change(coding.dst_pos, deleted, coding.produced_char)?;
            self.update_compositions(
                coding.dst_pos,
                coding.dst_pos + coding.produced_char,
                crate::composite::CHECK_ALL,
            )?;
            self.set_buffer_internal(current);
        }
        if let Some(saved) = saved {
            self.restore_point_and_markers(
                coding,
                src_object,
                saved,
                (from, from_byte, chars, bytes),
                &markers,
            );
        }
        if kill_src_buffer {
            let f = self.intern("kill-buffer");
            let object = coding.src_object;
            self.funcall(f, &[object])?;
        }
        self.set_plain_value(sym::DEACTIVATE_MARK, old_deactivate_mark);
        Ok(())
    }
}

pub(super) fn inhibit_flag(encoded: i32, var: bool) -> bool {
    0 < encoded + i32::from(var)
}

fn reset_coding_identity(coding: &mut CodingSystem, fresh: CodingSystem) {
    let keep_source = std::mem::take(&mut coding.source);
    let keep_destination = std::mem::take(&mut coding.destination);
    let saved = (
        coding.src_object,
        coding.src_pos,
        coding.src_pos_byte,
        coding.src_chars,
        coding.src_bytes,
        coding.src_multibyte,
        coding.dst_object,
        coding.dst_pos,
        coding.dst_pos_byte,
        coding.dst_multibyte,
        coding.head_ascii,
        coding.eol_seen,
        coding.detected_utf8_bytes,
        coding.detected_utf8_chars,
        coding.insert_before_markers,
        coding.raw_destination,
    );
    *coding = fresh;
    coding.source = keep_source;
    coding.destination = keep_destination;
    coding.src_object = saved.0;
    coding.src_pos = saved.1;
    coding.src_pos_byte = saved.2;
    coding.src_chars = saved.3;
    coding.src_bytes = saved.4;
    coding.src_multibyte = saved.5;
    coding.dst_object = saved.6;
    coding.dst_pos = saved.7;
    coding.dst_pos_byte = saved.8;
    coding.dst_multibyte = saved.9;
    coding.head_ascii = saved.10;
    coding.eol_seen = saved.11;
    coding.detected_utf8_bytes = saved.12;
    coding.detected_utf8_chars = saved.13;
    coding.insert_before_markers = saved.14;
    coding.raw_destination = saved.15;
}

fn code_conversion_restore(lisp: &mut Lisp, arg: Value) -> LispResult<()> {
    let current = lisp.xcar(arg);
    let workbuf = lisp.xcdr(arg);
    if !workbuf.is_nil() {
        if workbuf == lisp.codings.reused_workbuf {
            lisp.codings.reused_workbuf_in_use = false;
        } else {
            lisp.kill_buffer_internal(workbuf)?;
        }
    }
    if lisp.buffer_live_p(current) {
        lisp.set_buffer_internal(current);
    }
    Ok(())
}

fn coding_restore_undo_list(lisp: &mut Lisp, arg: Value) -> LispResult<()> {
    let undo_list = lisp.xcar(arg);
    let buffer = lisp.xcdr(arg);
    lisp.buffer_local_set(buffer, sym::BUFFER_UNDO_LIST, undo_list);
    Ok(())
}
