use super::engine::{
    DetectionInfo, Emit, MAX_ANNOTATION_LENGTH, Src, add_charset_data, add_composition_data,
    category_mask, omb,
};
use super::{
    ANNOTATE_CHARSET_MASK, ANNOTATE_COMPOSITION_MASK, ATTR_ASCII_COMPAT, ATTR_CHARSET_LIST,
    ATTR_ISO_INITIAL, ATTR_SAFE_CHARSETS, CAT_ISO_7, CAT_ISO_7_ELSE, CAT_ISO_7_TIGHT, CAT_ISO_8_1,
    CAT_ISO_8_2, CAT_ISO_8_ELSE, COMPOSITION_RELATIVE, COMPOSITION_WITH_ALTCHARS,
    COMPOSITION_WITH_RULE, COMPOSITION_WITH_RULE_ALTCHARS, CodingResult, CodingSystem,
    ComposingState, CompositionStatus, ISO_FLAG_DESIGNATE_AT_BOL, ISO_FLAG_DESIGNATION,
    ISO_FLAG_DIRECTION, ISO_FLAG_FULL_SUPPORT, ISO_FLAG_INIT_AT_BOL, ISO_FLAG_LATIN_EXTRA,
    ISO_FLAG_LEVEL_4, ISO_FLAG_LOCKING_SHIFT, ISO_FLAG_LONG_FORM, ISO_FLAG_RESET_AT_CNTL,
    ISO_FLAG_RESET_AT_EOL, ISO_FLAG_REVISION, ISO_FLAG_SEVEN_BITS, ISO_FLAG_SINGLE_SHIFT,
    ISO_FLAG_USE_OLDJIS, ISO_FLAG_USE_ROMAN, MAX_COMPOSITION_COMPONENTS, MODE_DIRECTION,
    MODE_LAST_BLOCK, MODE_SAFE_ENCODING,
};
use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const ISO_CODE_SO: i32 = 0x0E;
const ISO_CODE_SI: i32 = 0x0F;
const ISO_CODE_SS2_7: i32 = 0x19;
const ISO_CODE_ESC: i32 = 0x1B;
const ISO_CODE_SS2: i32 = 0x8E;
const ISO_CODE_SS3: i32 = 0x8F;
const ISO_CODE_CSI: i32 = 0x9B;
const INHIBIT_CHARACTER_SUBSTITUTION: i32 = b'?' as i32;

const MASK_ISO_7BIT: u32 = category_mask(CAT_ISO_7) | category_mask(CAT_ISO_7_TIGHT);
const MASK_ISO_8BIT: u32 = category_mask(CAT_ISO_8_1) | category_mask(CAT_ISO_8_2);
const MASK_ISO_ELSE: u32 = category_mask(CAT_ISO_7_ELSE) | category_mask(CAT_ISO_8_ELSE);
const MASK_ISO: u32 = MASK_ISO_7BIT | MASK_ISO_8BIT | MASK_ISO_ELSE;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Control0,
    ShiftOut,
    ShiftIn,
    SingleShift27,
    Escape,
    Control1,
    SingleShift2,
    SingleShift3,
    Csi,
    Space20Or7F,
    GraphicPlane0,
    A0OrFF,
    GraphicPlane1,
}

fn iso_code_class(c: i32) -> Class {
    match c {
        ISO_CODE_SO => Class::ShiftOut,
        ISO_CODE_SI => Class::ShiftIn,
        ISO_CODE_SS2_7 => Class::SingleShift27,
        ISO_CODE_ESC => Class::Escape,
        ISO_CODE_SS2 => Class::SingleShift2,
        ISO_CODE_SS3 => Class::SingleShift3,
        ISO_CODE_CSI => Class::Csi,
        0x20 | 0x7F => Class::Space20Or7F,
        0xA0 | 0xFF => Class::A0OrFF,
        0x00..0x20 => Class::Control0,
        0x21..0x7F => Class::GraphicPlane0,
        0x80..0xA0 => Class::Control1,
        _ => Class::GraphicPlane1,
    }
}

fn ascii_or_byte8(c: i32) -> i32 {
    if (c as u32) < 0x80 { c } else { c + 0x3F_FF00 }
}

fn composition_encode_rule(gref: i32, nref: i32) -> i32 {
    gref * 12 + nref
}

fn safe_charset_p(coding: &CodingSystem, id: i32) -> bool {
    id >= 0
        && (id as i64) <= coding.max_charset_id
        && coding
            .safe_charsets
            .get(id as usize)
            .copied()
            .unwrap_or(255)
            != 255
}

fn iso_request(coding: &CodingSystem, id: i32) -> i32 {
    if id >= 0 && (id as i64) <= coding.max_charset_id {
        match coding.safe_charsets.get(id as usize).copied() {
            Some(v) if v != 255 => v as i32,
            _ => -1,
        }
    } else {
        -1
    }
}

fn invoked_charset(coding: &CodingSystem, plane: usize) -> i32 {
    let reg = coding.iso_invocation[plane];
    if reg < 0 {
        -1
    } else {
        coding.iso_designation[reg as usize]
    }
}

fn state_up(state: ComposingState) -> ComposingState {
    match state {
        ComposingState::Char => ComposingState::Rule,
        ComposingState::ComponentChar => ComposingState::ComponentRule,
        s => s,
    }
}

fn state_down(state: ComposingState) -> ComposingState {
    match state {
        ComposingState::Rule => ComposingState::Char,
        ComposingState::ComponentRule => ComposingState::ComponentChar,
        s => s,
    }
}

fn finish_composition(charbuf: &mut [i32], cmp: &mut CompositionStatus) -> i32 {
    let len = charbuf.len();
    let mut idx = len - cmp.length as usize;
    charbuf[idx] = ISO_CODE_ESC;
    charbuf[idx + 1] = match cmp.method {
        COMPOSITION_RELATIVE => b'0',
        COMPOSITION_WITH_RULE => b'2',
        COMPOSITION_WITH_ALTCHARS => b'3',
        _ => b'4',
    } as i32;
    charbuf[idx + 2] = -2;
    charbuf[idx + 3] = 0;
    charbuf[idx + 4] = -1;
    idx += 5;
    let mut new_chars = cmp.nchars;
    if cmp.method >= COMPOSITION_WITH_RULE {
        while idx < len {
            let elt = charbuf[idx];
            if elt == -2 {
                let rule = charbuf[idx + 1];
                let gref = (rule % 0x100) / 12;
                let nref = (rule % 0x100) % 12;
                if rule < 0x100 {
                    let gref = if gref == 10 { 4 } else { gref };
                    let nref = if nref == 10 { 4 } else { nref };
                    charbuf[idx] = 32 + gref * 9 + nref;
                    charbuf[idx + 1] = -1;
                    new_chars += 1;
                } else {
                    charbuf[idx] = 32 + 81 + gref;
                    charbuf[idx + 1] = 32 + nref;
                    new_chars += 2;
                }
                idx += 1;
            } else if elt == -1 {
                charbuf[idx] = ISO_CODE_ESC;
                idx += 1;
                charbuf[idx] = b'0' as i32;
                new_chars += 2;
            }
            idx += 1;
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

enum Step {
    Continue,
    Invalid,
    BreakLoop,
}

struct IsoOut<'a> {
    coding: &'a mut CodingSystem,
    dst: &'a mut Vec<u8>,
    emit: Emit,
}

impl IsoOut<'_> {
    fn flag(&self, f: i64) -> bool {
        self.coding.iso_flags & f != 0
    }

    fn ascii(&mut self, c: i32) {
        self.emit.ascii(self.dst, c);
    }

    fn one(&mut self, c: i32) {
        self.emit.one(self.dst, c);
    }

    fn designation(&mut self, lisp: &Lisp, id: i32, reg: usize) {
        let cs = lisp.charset(id);
        let final_char = cs.iso_final & 0xFF;
        let dimension = cs.dimension;
        let chars_96 = cs.iso_chars_96;
        let revision = if self.flag(ISO_FLAG_REVISION) {
            cs.iso_revision
        } else {
            -1
        };
        let i94 = b"()*+";
        let i96 = b",-./";
        if revision >= 0 {
            self.ascii(ISO_CODE_ESC);
            self.ascii(b'&' as i32);
            self.one(b'@' as i32 + revision);
        }
        self.ascii(ISO_CODE_ESC);
        if dimension == 1 {
            let b = if chars_96 { i96[reg] } else { i94[reg] };
            self.ascii(b as i32);
        } else {
            self.ascii(b'$' as i32);
            if !chars_96 {
                if self.flag(ISO_FLAG_LONG_FORM)
                    || reg != 0
                    || final_char < b'@' as i32
                    || final_char > b'B' as i32
                {
                    self.ascii(i94[reg] as i32);
                }
            } else {
                self.ascii(i96[reg] as i32);
            }
        }
        self.ascii(final_char);
        self.coding.iso_designation[reg] = id;
    }

    fn single_shift(&mut self, esc_final: u8, code: i32) {
        if self.flag(ISO_FLAG_SEVEN_BITS) {
            self.ascii(ISO_CODE_ESC);
            self.ascii(esc_final as i32);
        } else {
            self.one(code);
        }
        self.coding.iso_single_shifting = true;
    }

    fn shift_in(&mut self) {
        self.ascii(ISO_CODE_SI);
        self.coding.iso_invocation[0] = 0;
    }

    fn shift_out(&mut self) {
        self.ascii(ISO_CODE_SO);
        self.coding.iso_invocation[0] = 1;
    }

    fn locking_shift(&mut self, reg: i32) {
        self.ascii(ISO_CODE_ESC);
        self.ascii(b'n' as i32);
        self.coding.iso_invocation[0] = reg;
    }

    fn invocation_designation(&mut self, lisp: &Lisp, id: i32) {
        let reg = match (0..4).find(|&r| self.coding.iso_designation[r] == id) {
            Some(r) => r as i32,
            None => {
                let reg = iso_request(self.coding, id).max(0);
                self.designation(lisp, id, reg as usize);
                reg
            }
        };
        if self.coding.iso_invocation[0] != reg && self.coding.iso_invocation[1] != reg {
            match reg {
                0 => self.shift_in(),
                1 => self.shift_out(),
                2 => {
                    if self.flag(ISO_FLAG_SINGLE_SHIFT) {
                        self.single_shift(b'N', ISO_CODE_SS2);
                    } else {
                        self.locking_shift(2);
                    }
                }
                3 => {
                    if self.flag(ISO_FLAG_SINGLE_SHIFT) {
                        self.single_shift(b'O', ISO_CODE_SS3);
                    } else {
                        self.locking_shift(3);
                    }
                }
                _ => {}
            }
        }
    }

    fn dimension1(&mut self, lisp: &Lisp, mut id: i32, c1: i32) {
        if self.flag(ISO_FLAG_USE_ROMAN)
            && id == lisp.charsets.ascii
            && lisp.charsets.jisx0201_roman >= 0
        {
            id = lisp.charsets.jisx0201_roman;
        }
        loop {
            if self.coding.iso_single_shifting {
                if self.flag(ISO_FLAG_SEVEN_BITS) {
                    self.ascii(c1 & 0x7F);
                } else {
                    self.one(c1 | 0x80);
                }
                self.coding.iso_single_shifting = false;
                return;
            } else if id == invoked_charset(self.coding, 0) {
                self.ascii(c1 & 0x7F);
                return;
            } else if id == invoked_charset(self.coding, 1) {
                self.one(c1 | 0x80);
                return;
            }
            self.invocation_designation(lisp, id);
        }
    }

    fn dimension2(&mut self, lisp: &Lisp, mut id: i32, c1: i32, c2: i32) {
        if self.flag(ISO_FLAG_USE_OLDJIS)
            && id == lisp.charsets.jisx0208
            && lisp.charsets.jisx0208_1978 >= 0
        {
            id = lisp.charsets.jisx0208_1978;
        }
        loop {
            if self.coding.iso_single_shifting {
                if self.flag(ISO_FLAG_SEVEN_BITS) {
                    self.ascii(c1 & 0x7F);
                    self.ascii(c2 & 0x7F);
                } else {
                    self.one(c1 | 0x80);
                    self.one(c2 | 0x80);
                }
                self.coding.iso_single_shifting = false;
                return;
            } else if id == invoked_charset(self.coding, 0) {
                self.ascii(c1 & 0x7F);
                self.ascii(c2 & 0x7F);
                return;
            } else if id == invoked_charset(self.coding, 1) {
                self.one(c1 | 0x80);
                self.one(c2 | 0x80);
                return;
            }
            self.invocation_designation(lisp, id);
        }
    }

    fn reset_plane_and_register(&mut self, lisp: &Lisp, initial: &[i32; 4]) {
        if self.coding.iso_invocation[0] != 0 {
            self.shift_in();
        }
        for (reg, &init) in initial.iter().enumerate() {
            if init >= 0 && self.coding.iso_designation[reg] != init {
                self.designation(lisp, init, reg);
            }
        }
    }
}

impl Lisp {
    fn iso_initial(&self, coding: &CodingSystem) -> [i32; 4] {
        let attrs = self.coding_id_attrs(coding.id);
        let initial = self.attr(attrs, ATTR_ISO_INITIAL);
        let items = self.vector_items(initial);
        let mut out = [-1; 4];
        for (o, v) in out.iter_mut().zip(items) {
            *o = v.as_fixnum().unwrap_or(-1) as i32;
        }
        out
    }

    fn decode_designation(
        &self,
        coding: &mut CodingSystem,
        reg: usize,
        dim: i32,
        chars_96: i32,
        final_char: i32,
    ) -> i32 {
        let id = if !(b'0' as i32..128).contains(&final_char) {
            -1
        } else {
            self.charsets.iso_charset(dim, chars_96 != 0, final_char)
        };
        if id < 0 || !safe_charset_p(coding, id) {
            coding.iso_designation[reg] = -2;
            return -1;
        }
        let prev = coding.iso_designation[reg];
        let mut id = id;
        if id == self.charsets.jisx0201_roman {
            if coding.iso_flags & ISO_FLAG_USE_ROMAN != 0 {
                id = self.charsets.ascii;
            }
        } else if id == self.charsets.jisx0208_1978 && coding.iso_flags & ISO_FLAG_USE_OLDJIS != 0 {
            id = self.charsets.jisx0208;
        }
        coding.iso_designation[reg] = id;
        if prev == -2 && id == self.charsets.ascii {
            return -1;
        }
        chars_96
    }

    pub fn detect_coding_iso_2022(
        &mut self,
        coding: &mut CodingSystem,
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let source = std::mem::take(&mut coding.source);
        let r = self.detect_coding_iso_2022_1(coding, &source, info);
        coding.source = source;
        r
    }

    fn detect_coding_iso_2022_1(
        &mut self,
        coding: &mut CodingSystem,
        source: &[u8],
        info: &mut DetectionInfo,
    ) -> LispResult<bool> {
        let mut s = Src::new(source, 0, coding.src_bytes, coding.src_multibyte);
        let mut single_shifting = false;
        let mut rejected = 0u32;
        let mut found = 0u32;
        let mut composition_count: i64 = -1;
        info.checked |= MASK_ISO;
        for i in CAT_ISO_7..=CAT_ISO_8_ELSE {
            let id = self.codings.categories[i].id;
            if id < 0 {
                continue;
            }
            let attrs = self.coding_id_attrs(id);
            if self.codings.categories[i].iso_flags & ISO_FLAG_FULL_SUPPORT != 0
                && self.attr(attrs, ATTR_CHARSET_LIST) != self.charsets.iso_2022_charset_list
            {
                self.setup_iso_safe_charsets(attrs)?;
            }
            let val = self.attr(attrs, ATTR_SAFE_CHARSETS);
            let bytes = self.string_bytes(val).to_vec();
            let this = &mut self.codings.categories[i];
            this.max_charset_id = bytes.len() as i64 - 1;
            this.safe_charsets = bytes;
        }
        let inhibit = self.var_true(sym::INHIBIT_ISO_ESCAPE_DETECTION);
        s.pos += coding.head_ascii.max(0) as usize;
        'nms: {
            while rejected != MASK_ISO {
                s.base = s.pos;
                let mut c = omb!(s, self, coding, 'nms);
                let check_extra_latin = 'sw: {
                    match c {
                        ISO_CODE_ESC => {
                            if inhibit {
                                break 'sw false;
                            }
                            single_shifting = false;
                            c = omb!(s, self, coding, 'nms);
                            if c == b'N' as i32 || c == b'O' as i32 {
                                single_shifting = true;
                                rejected |= MASK_ISO_7BIT | MASK_ISO_8BIT;
                            } else if c == b'1' as i32 {
                                if composition_count < 0
                                    || composition_count > MAX_COMPOSITION_COMPONENTS as i64
                                {
                                    break 'sw false;
                                }
                                composition_count = -1;
                                found |= MASK_ISO;
                            } else if (b'0' as i32..=b'4' as i32).contains(&c) {
                                composition_count = 0;
                            } else {
                                let id;
                                if (b'(' as i32..=b'/' as i32).contains(&c) {
                                    let c1 = omb!(s, self, coding, 'nms);
                                    let cand = if (0x20..0x80).contains(&c1) {
                                        self.charsets.iso_charset(1, c >= b',' as i32, c1)
                                    } else {
                                        -1
                                    };
                                    if cand < 0 {
                                        if c1 >= 0x80 {
                                            rejected |=
                                                MASK_ISO_7BIT | category_mask(CAT_ISO_7_ELSE);
                                        }
                                        break 'sw false;
                                    }
                                    id = cand;
                                } else if c == b'$' as i32 {
                                    c = omb!(s, self, coding, 'nms);
                                    if (b'@' as i32..=b'B' as i32).contains(&c) {
                                        id = self.charsets.iso_charset(2, false, c);
                                    } else if (b'(' as i32..=b'/' as i32).contains(&c) {
                                        let c1 = omb!(s, self, coding, 'nms);
                                        let cand = if (0x20..0x80).contains(&c1) {
                                            self.charsets.iso_charset(2, c >= b',' as i32, c1)
                                        } else {
                                            -1
                                        };
                                        if cand < 0 {
                                            if c1 >= 0x80 {
                                                rejected |=
                                                    MASK_ISO_7BIT | category_mask(CAT_ISO_7_ELSE);
                                            }
                                            break 'sw false;
                                        }
                                        id = cand;
                                    } else {
                                        if c >= 0x80 {
                                            rejected |=
                                                MASK_ISO_7BIT | category_mask(CAT_ISO_7_ELSE);
                                        }
                                        break 'sw false;
                                    }
                                } else {
                                    if c >= 0x80 {
                                        rejected |= MASK_ISO_7BIT | category_mask(CAT_ISO_7_ELSE);
                                    }
                                    break 'sw false;
                                }
                                rejected |= MASK_ISO_8BIT;
                                for cat in
                                    [CAT_ISO_7, CAT_ISO_7_TIGHT, CAT_ISO_7_ELSE, CAT_ISO_8_ELSE]
                                {
                                    if safe_charset_p(&self.codings.categories[cat], id) {
                                        found |= category_mask(cat);
                                    } else {
                                        rejected |= category_mask(cat);
                                    }
                                }
                            }
                            false
                        }
                        ISO_CODE_SO | ISO_CODE_SI => {
                            if inhibit {
                                break 'sw false;
                            }
                            single_shifting = false;
                            rejected |= MASK_ISO_7BIT | MASK_ISO_8BIT;
                            false
                        }
                        ISO_CODE_CSI => {
                            single_shifting = false;
                            rejected |= MASK_ISO_7BIT | category_mask(CAT_ISO_7_ELSE);
                            found |= category_mask(CAT_ISO_8_ELSE);
                            true
                        }
                        ISO_CODE_SS2 | ISO_CODE_SS3 => {
                            if inhibit {
                                break 'sw false;
                            }
                            single_shifting = false;
                            rejected |= MASK_ISO_7BIT | category_mask(CAT_ISO_7_ELSE);
                            if self.codings.categories[CAT_ISO_8_1].iso_flags
                                & ISO_FLAG_SINGLE_SHIFT
                                != 0
                            {
                                found |= category_mask(CAT_ISO_8_1);
                                single_shifting = true;
                            }
                            if self.codings.categories[CAT_ISO_8_2].iso_flags
                                & ISO_FLAG_SINGLE_SHIFT
                                != 0
                            {
                                found |= category_mask(CAT_ISO_8_2);
                                single_shifting = true;
                            }
                            !single_shifting
                        }
                        _ => {
                            if c < 0 {
                                break 'sw false;
                            }
                            if c < 0x80 {
                                if composition_count >= 0 {
                                    composition_count += 1;
                                }
                                single_shifting = false;
                                break 'sw false;
                            }
                            rejected |= MASK_ISO_7BIT | category_mask(CAT_ISO_7_ELSE);
                            if c >= 0xA0 {
                                found |= category_mask(CAT_ISO_8_1);
                                if !single_shifting && rejected & category_mask(CAT_ISO_8_2) == 0 {
                                    let mut len: i64 = 1;
                                    while s.pos < s.end {
                                        s.base = s.pos;
                                        let c = omb!(s, self, coding, 'nms);
                                        if c < 0xA0 {
                                            s.pos = s.base;
                                            break;
                                        }
                                        len += 1;
                                    }
                                    if len & 1 != 0 && s.pos < s.end {
                                        rejected |= category_mask(CAT_ISO_8_2);
                                        if composition_count >= 0 {
                                            composition_count += len;
                                        }
                                    } else {
                                        found |= category_mask(CAT_ISO_8_2);
                                        if composition_count >= 0 {
                                            composition_count += len / 2;
                                        }
                                    }
                                }
                                break 'sw false;
                            }
                            true
                        }
                    }
                };
                if check_extra_latin {
                    let table = self.bound_var(sym::LATIN_EXTRA_CODE_TABLE);
                    if !self.is_vector(table)
                        || self
                            .vector_items(table)
                            .get(c as usize)
                            .is_none_or(|v| v.is_nil())
                    {
                        rejected = MASK_ISO;
                        continue;
                    }
                    if self.codings.categories[CAT_ISO_8_1].iso_flags & ISO_FLAG_LATIN_EXTRA != 0 {
                        found |= category_mask(CAT_ISO_8_1);
                    } else {
                        rejected |= category_mask(CAT_ISO_8_1);
                    }
                    rejected |= category_mask(CAT_ISO_8_2);
                }
            }
            info.rejected |= MASK_ISO;
            return Ok(false);
        }
        info.rejected |= rejected;
        info.found |= found & !rejected;
        Ok(true)
    }

    pub fn decode_coding_iso_2022(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let source = std::mem::take(&mut coding.source);
        let mut cmp = std::mem::take(&mut coding.cmp_status);
        let r = self.decode_coding_iso_2022_1(coding, &source, &mut cmp);
        coding.cmp_status = cmp;
        coding.source = source;
        r
    }

    fn decode_coding_iso_2022_1(
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
        let charbuf_end = coding.charbuf_size - MAX_ANNOTATION_LENGTH * 3;
        let mut consumed_chars_base;
        let mut charset_id_0 = invoked_charset(coding, 0);
        let mut charset_id_1 = invoked_charset(coding, 1);
        let attrs = self.coding_id_attrs(coding.id);
        let ascii = self.charsets.ascii;
        let mut char_offset = coding.produced_char as i32;
        let mut last_offset = char_offset;
        let mut last_id = ascii;
        let eol_dos = self.coding_eol_dos(coding);
        let mut byte_after_cr: i32 = -1;
        self.setup_iso_safe_charsets(attrs)?;
        let safe = self.attr(attrs, ATTR_SAFE_CHARSETS);
        coding.safe_charsets = self.string_bytes(safe).to_vec();
        if cmp.state != ComposingState::No {
            let n = cmp.length as usize;
            coding.charbuf.extend_from_slice(&cmp.carryover[..n]);
            coding.annotated = true;
        }
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
                let mut c1 = if byte_after_cr >= 0 {
                    let v = byte_after_cr;
                    byte_after_cr = -1;
                    v
                } else {
                    omb!(s, self, coding, 'nms)
                };
                let step = 'step: {
                    if c1 < 0 {
                        break 'step Step::Invalid;
                    }
                    if coding.iso_extsegment_len > 0 {
                        coding.charbuf.push(ascii_or_byte8(c1));
                        char_offset += 1;
                        coding.iso_extsegment_len -= 1;
                        break 'step Step::Continue;
                    }
                    if coding.iso_embedded_utf_8 {
                        if c1 == ISO_CODE_ESC {
                            if s.pos + 1 >= s.end {
                                self.record_conversion_result(
                                    coding,
                                    CodingResult::InsufficientSrc,
                                );
                                break 'nms;
                            }
                            coding.charbuf.push(ISO_CODE_ESC);
                            char_offset += 1;
                            if source[s.pos] == b'%' && source[s.pos + 1] == b'@' {
                                s.pos += 2;
                                s.consumed_chars += 2;
                                char_offset += 2;
                                coding.charbuf.push(b'%' as i32);
                                coding.charbuf.push(b'@' as i32);
                                coding.iso_embedded_utf_8 = false;
                            }
                        } else {
                            coding.charbuf.push(ascii_or_byte8(c1));
                            char_offset += 1;
                        }
                        break 'step Step::Continue;
                    }
                    if matches!(
                        cmp.state,
                        ComposingState::Rule | ComposingState::ComponentRule
                    ) && c1 != ISO_CODE_ESC
                    {
                        let mut rule = c1 - 32;
                        if rule < 0 {
                            break 'step Step::Invalid;
                        }
                        if rule < 81 {
                            let gref = if rule / 9 == 4 { 10 } else { rule / 9 };
                            let nref = if rule % 9 == 4 { 10 } else { rule % 9 };
                            rule = composition_encode_rule(gref, nref);
                        } else {
                            let b = omb!(s, self, coding, 'nms);
                            let (gref, nref) = (rule - 81, b - 32);
                            if !((0..12).contains(&gref) && (0..12).contains(&nref)) {
                                break 'step Step::Invalid;
                            }
                            rule = composition_encode_rule(gref, nref) + 0x100;
                        }
                        coding.charbuf.push(-2);
                        coding.charbuf.push(rule);
                        cmp.length += 2;
                        cmp.state = state_down(cmp.state);
                        break 'step Step::Continue;
                    }
                    let flags = coding.iso_flags;
                    let charset = 'cls: {
                        let mut esc = match iso_code_class(c1) {
                            Class::Space20Or7F => {
                                break 'cls if charset_id_0 < 0
                                    || !self.charset(charset_id_0).iso_chars_96
                                {
                                    ascii
                                } else {
                                    charset_id_0
                                };
                            }
                            Class::GraphicPlane0 => {
                                break 'cls if charset_id_0 < 0 {
                                    ascii
                                } else {
                                    charset_id_0
                                };
                            }
                            Class::A0OrFF | Class::GraphicPlane1 => {
                                if iso_code_class(c1) == Class::A0OrFF
                                    && (charset_id_1 < 0
                                        || !self.charset(charset_id_1).iso_chars_96
                                        || flags & ISO_FLAG_SEVEN_BITS != 0)
                                {
                                    break 'step Step::Invalid;
                                }
                                if charset_id_1 < 0 {
                                    break 'step Step::Invalid;
                                }
                                break 'cls charset_id_1;
                            }
                            Class::Control0 => {
                                if eol_dos && c1 == b'\r' as i32 {
                                    byte_after_cr = omb!(s, self, coding, 'nms);
                                }
                                maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                                break 'cls ascii;
                            }
                            Class::Control1 => break 'step Step::Invalid,
                            Class::ShiftOut => {
                                if flags & ISO_FLAG_LOCKING_SHIFT == 0
                                    || coding.iso_designation[1] < 0
                                {
                                    break 'step Step::Invalid;
                                }
                                coding.iso_invocation[0] = 1;
                                charset_id_0 = invoked_charset(coding, 0);
                                break 'step Step::Continue;
                            }
                            Class::ShiftIn => {
                                if flags & ISO_FLAG_LOCKING_SHIFT == 0 {
                                    break 'step Step::Invalid;
                                }
                                coding.iso_invocation[0] = 0;
                                charset_id_0 = invoked_charset(coding, 0);
                                break 'step Step::Continue;
                            }
                            Class::SingleShift27 | Class::SingleShift2 => {
                                if iso_code_class(c1) == Class::SingleShift27
                                    && flags & ISO_FLAG_SEVEN_BITS == 0
                                {
                                    break 'step Step::Invalid;
                                }
                                if flags & ISO_FLAG_SINGLE_SHIFT == 0 {
                                    break 'step Step::Invalid;
                                }
                                b'N' as i32
                            }
                            Class::SingleShift3 => {
                                if flags & ISO_FLAG_SINGLE_SHIFT == 0 {
                                    break 'step Step::Invalid;
                                }
                                b'O' as i32
                            }
                            Class::Csi => b'[' as i32,
                            Class::Escape => omb!(s, self, coding, 'nms),
                        };
                        loop {
                            match u8::try_from(esc).unwrap_or(0) {
                                b'&' => {
                                    c1 = omb!(s, self, coding, 'nms);
                                    if !(b'@' as i32..=b'~' as i32).contains(&c1) {
                                        break 'step Step::Invalid;
                                    }
                                    c1 = omb!(s, self, coding, 'nms);
                                    if c1 != ISO_CODE_ESC {
                                        break 'step Step::Invalid;
                                    }
                                    esc = omb!(s, self, coding, 'nms);
                                }
                                b'$' => {
                                    if flags & ISO_FLAG_DESIGNATION == 0 {
                                        break 'step Step::Invalid;
                                    }
                                    c1 = omb!(s, self, coding, 'nms);
                                    let (reg, chars96) =
                                        if (b'@' as i32..=b'B' as i32).contains(&c1) {
                                            (0, 0)
                                        } else if (0x28..=0x2B).contains(&c1) {
                                            let r = (c1 - 0x28) as usize;
                                            c1 = omb!(s, self, coding, 'nms);
                                            (r, 0)
                                        } else if (0x2C..=0x2F).contains(&c1) {
                                            let r = (c1 - 0x2C) as usize;
                                            c1 = omb!(s, self, coding, 'nms);
                                            (r, 1)
                                        } else {
                                            break 'step Step::Invalid;
                                        };
                                    let chars96 =
                                        self.decode_designation(coding, reg, 2, chars96, c1);
                                    if reg == 0 {
                                        charset_id_0 = invoked_charset(coding, 0);
                                    } else if reg == 1 {
                                        charset_id_1 = invoked_charset(coding, 1);
                                    }
                                    if chars96 < 0 {
                                        break 'step Step::Invalid;
                                    }
                                    break 'step Step::Continue;
                                }
                                b'n' | b'o' => {
                                    let reg = if esc == b'n' as i32 { 2 } else { 3 };
                                    if flags & ISO_FLAG_LOCKING_SHIFT == 0
                                        || coding.iso_designation[reg] < 0
                                    {
                                        break 'step Step::Invalid;
                                    }
                                    coding.iso_invocation[0] = reg as i32;
                                    charset_id_0 = invoked_charset(coding, 0);
                                    break 'step Step::Continue;
                                }
                                b'N' | b'O' => {
                                    let reg = if esc == b'N' as i32 { 2 } else { 3 };
                                    if flags & ISO_FLAG_SINGLE_SHIFT == 0
                                        || coding.iso_designation[reg] < 0
                                    {
                                        break 'step Step::Invalid;
                                    }
                                    let id = coding.iso_designation[reg];
                                    let charset = if id < 0 { ascii } else { id };
                                    c1 = omb!(s, self, coding, 'nms);
                                    if c1 < 0x20
                                        || (0x80..0xA0).contains(&c1)
                                        || (flags & ISO_FLAG_SEVEN_BITS == 0
                                            && if flags & ISO_FLAG_LEVEL_4 != 0 {
                                                c1 >= 0x80
                                            } else {
                                                c1 < 0x80
                                            })
                                    {
                                        break 'step Step::Invalid;
                                    }
                                    break 'cls charset;
                                }
                                b'0' | b'2' | b'3' | b'4' => {
                                    if coding.common_flags & ANNOTATE_COMPOSITION_MASK == 0 {
                                        break 'step Step::Invalid;
                                    }
                                    if last_id != ascii {
                                        add_charset_data(
                                            coding,
                                            char_offset - last_offset,
                                            last_id,
                                        );
                                        last_id = ascii;
                                        last_offset = char_offset;
                                    }
                                    if esc == b'0' as i32
                                        && ((cmp.state == ComposingState::ComponentChar
                                            && cmp.method == COMPOSITION_WITH_ALTCHARS)
                                            || (cmp.state == ComposingState::ComponentRule
                                                && cmp.method == COMPOSITION_WITH_RULE_ALTCHARS))
                                    {
                                        coding.charbuf.push(-1);
                                        coding.charbuf.push(-1);
                                        cmp.state = ComposingState::Char;
                                        cmp.length += 2;
                                    } else {
                                        maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                                        cmp.method = match esc as u8 {
                                            b'0' => COMPOSITION_RELATIVE,
                                            b'2' => COMPOSITION_WITH_RULE,
                                            b'3' => COMPOSITION_WITH_ALTCHARS,
                                            _ => COMPOSITION_WITH_RULE_ALTCHARS,
                                        };
                                        cmp.state = if esc <= b'2' as i32 {
                                            ComposingState::Char
                                        } else {
                                            ComposingState::ComponentChar
                                        };
                                        add_composition_data(coding, 0, 0, cmp.method);
                                        cmp.length = MAX_ANNOTATION_LENGTH as i32;
                                        cmp.nchars = 0;
                                        cmp.ncomps = 0;
                                        coding.annotated = true;
                                    }
                                    break 'step Step::Continue;
                                }
                                b'1' => {
                                    if cmp.state == ComposingState::No {
                                        break 'step Step::Invalid;
                                    }
                                    if cmp.nchars == 0
                                        || ((cmp.state == ComposingState::Char)
                                            == (cmp.method == COMPOSITION_WITH_RULE))
                                    {
                                        maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                                        break 'step Step::Invalid;
                                    }
                                    let head = coding.charbuf.len() - cmp.length as usize;
                                    if cmp.method == COMPOSITION_WITH_ALTCHARS {
                                        coding.charbuf[head] -= cmp.ncomps + 2;
                                    } else if cmp.method == COMPOSITION_WITH_RULE_ALTCHARS {
                                        coding.charbuf[head] -= cmp.ncomps * 3;
                                    }
                                    coding.charbuf[head + 2] = cmp.nchars;
                                    char_offset += cmp.nchars;
                                    cmp.state = ComposingState::No;
                                    break 'step Step::Continue;
                                }
                                b'[' => {
                                    if flags & ISO_FLAG_DIRECTION == 0 {
                                        break 'step Step::Invalid;
                                    }
                                    c1 = omb!(s, self, coding, 'nms);
                                    match u8::try_from(c1).unwrap_or(0) {
                                        b']' => coding.mode &= !MODE_DIRECTION,
                                        b'0' | b'1' => {
                                            c1 = omb!(s, self, coding, 'nms);
                                            if c1 != b']' as i32 {
                                                break 'step Step::Invalid;
                                            }
                                            coding.mode &= !MODE_DIRECTION;
                                        }
                                        b'2' => {
                                            c1 = omb!(s, self, coding, 'nms);
                                            if c1 != b']' as i32 {
                                                break 'step Step::Invalid;
                                            }
                                            coding.mode |= MODE_DIRECTION;
                                        }
                                        _ => break 'step Step::Invalid,
                                    }
                                    break 'step Step::Continue;
                                }
                                b'%' => {
                                    c1 = omb!(s, self, coding, 'nms);
                                    if c1 == b'/' as i32 {
                                        let dim = omb!(s, self, coding, 'nms);
                                        if !(b'0' as i32..=b'4' as i32).contains(&dim) {
                                            break 'step Step::Invalid;
                                        }
                                        let m = omb!(s, self, coding, 'nms);
                                        if m < 128 {
                                            break 'step Step::Invalid;
                                        }
                                        let l = omb!(s, self, coding, 'nms);
                                        if l < 128 {
                                            break 'step Step::Invalid;
                                        }
                                        let size = (m - 128) * 128 + (l - 128);
                                        if coding.charbuf.len() + 6 > charbuf_end {
                                            break 'step Step::BreakLoop;
                                        }
                                        coding.charbuf.extend_from_slice(&[
                                            ISO_CODE_ESC,
                                            b'%' as i32,
                                            b'/' as i32,
                                            dim,
                                            m + 0x3F_FF00,
                                            l + 0x3F_FF00,
                                        ]);
                                        coding.iso_extsegment_len = size as i64;
                                    } else if c1 == b'G' as i32 {
                                        if coding.charbuf.len() + 3 > charbuf_end {
                                            break 'step Step::BreakLoop;
                                        }
                                        coding.charbuf.extend_from_slice(&[
                                            ISO_CODE_ESC,
                                            b'%' as i32,
                                            b'G' as i32,
                                        ]);
                                        coding.iso_embedded_utf_8 = true;
                                    } else {
                                        break 'step Step::Invalid;
                                    }
                                    break 'step Step::Continue;
                                }
                                _ => {
                                    if flags & ISO_FLAG_DESIGNATION == 0 {
                                        break 'step Step::Invalid;
                                    }
                                    let (reg, chars96) = if (0x28..=0x2B).contains(&esc) {
                                        let r = (esc - 0x28) as usize;
                                        c1 = omb!(s, self, coding, 'nms);
                                        (r, 0)
                                    } else if (0x2C..=0x2F).contains(&esc) {
                                        let r = (esc - 0x2C) as usize;
                                        c1 = omb!(s, self, coding, 'nms);
                                        (r, 1)
                                    } else {
                                        break 'step Step::Invalid;
                                    };
                                    let chars96 =
                                        self.decode_designation(coding, reg, 1, chars96, c1);
                                    if reg == 0 {
                                        charset_id_0 = invoked_charset(coding, 0);
                                    } else if reg == 1 {
                                        charset_id_1 = invoked_charset(coding, 1);
                                    }
                                    if chars96 < 0 {
                                        break 'step Step::Invalid;
                                    }
                                    break 'step Step::Continue;
                                }
                            }
                        }
                    };
                    if cmp.state == ComposingState::No && charset != ascii && last_id != charset {
                        if last_id != ascii {
                            add_charset_data(coding, char_offset - last_offset, last_id);
                        }
                        last_id = charset;
                        last_offset = char_offset;
                    }
                    let dimension = self.charset(charset).dimension;
                    if dimension > 1 {
                        let c2 = omb!(s, self, coding, 'nms);
                        if c2 < 0x20 || (0x80..0xA0).contains(&c2) || (c1 & 0x80) != (c2 & 0x80) {
                            break 'step Step::Invalid;
                        }
                        if dimension == 2 {
                            c1 = (c1 << 8) | c2;
                        } else {
                            let c3 = omb!(s, self, coding, 'nms);
                            if c3 < 0x20 || (0x80..0xA0).contains(&c3) || (c1 & 0x80) != (c3 & 0x80)
                            {
                                break 'step Step::Invalid;
                            }
                            c1 = (c1 << 16) | (c2 << 8) | c2;
                        }
                    }
                    c1 &= 0x7F_7F7F;
                    let c = self.decode_char(charset, c1 as u32)?;
                    if c < 0 {
                        maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                        for &b in &source[s.base..s.pos] {
                            coding.charbuf.push(ascii_or_byte8(b as i32));
                            char_offset += 1;
                        }
                    } else if cmp.state == ComposingState::No {
                        coding.charbuf.push(c);
                        char_offset += 1;
                    } else if (if cmp.state == ComposingState::Char {
                        cmp.nchars
                    } else {
                        cmp.ncomps
                    }) >= MAX_COMPOSITION_COMPONENTS as i32
                    {
                        maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                        coding.charbuf.push(c);
                        char_offset += 1;
                    } else {
                        coding.charbuf.push(c);
                        cmp.length += 1;
                        if cmp.state == ComposingState::Char {
                            cmp.nchars += 1;
                        } else {
                            cmp.ncomps += 1;
                        }
                        if cmp.method == COMPOSITION_WITH_RULE
                            || (cmp.method == COMPOSITION_WITH_RULE_ALTCHARS
                                && cmp.state == ComposingState::ComponentChar)
                        {
                            cmp.state = state_up(cmp.state);
                        }
                    }
                    Step::Continue
                };
                match step {
                    Step::Continue => {}
                    Step::BreakLoop => break,
                    Step::Invalid => {
                        maybe_finish(&mut coding.charbuf, cmp, &mut char_offset);
                        s.pos = s.base;
                        s.consumed_chars = consumed_chars_base;
                        let c = omb!(s, self, coding, 'nms);
                        coding
                            .charbuf
                            .push(if c < 0 { -c } else { ascii_or_byte8(c) });
                        char_offset += 1;
                        coding.iso_invocation[0] = 0;
                        coding.iso_designation[0] = ascii;
                        charset_id_0 = ascii;
                    }
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
        } else if last_id != ascii {
            add_charset_data(coding, char_offset - last_offset, last_id);
        }
        coding.consumed_char += consumed_chars_base;
        coding.consumed = s.base;
        Ok(())
    }

    pub fn encode_coding_iso_2022(&mut self, coding: &mut CodingSystem) -> LispResult<()> {
        let charbuf = std::mem::take(&mut coding.charbuf);
        let mut dst = std::mem::take(&mut coding.destination);
        let r = self.encode_coding_iso_2022_1(coding, &charbuf, &mut dst);
        coding.destination = dst;
        coding.charbuf = charbuf;
        r
    }

    fn encode_designation_at_bol(&mut self, out: &mut IsoOut, charbuf: &[i32]) -> LispResult<()> {
        let attrs = self.coding_id_attrs(out.coding.id);
        let mut charset_list = self.attr(attrs, ATTR_CHARSET_LIST);
        if charset_list == sym::ISO_2022 {
            charset_list = self.charsets.iso_2022_charset_list;
        }
        let mut r = [-1i32; 4];
        let mut found = 0;
        for &c in charbuf {
            if found >= 4 || c == b'\n' as i32 {
                break;
            }
            if c < 0 {
                continue;
            }
            if let Some((id, _)) = self.char_charset(c, charset_list)? {
                let reg = iso_request(out.coding, id);
                if reg >= 0 && r[reg as usize] < 0 {
                    found += 1;
                    r[reg as usize] = id;
                }
            }
        }
        if found > 0 {
            for (reg, &id) in r.iter().enumerate() {
                if id >= 0 && out.coding.iso_designation[reg] != id {
                    out.designation(self, id, reg);
                }
            }
        }
        Ok(())
    }

    fn encode_iso_character(&mut self, out: &mut IsoOut, charset: i32, c: i32) -> LispResult<()> {
        let code = self.encode_char(charset, c)? as i32;
        if self.charset(charset).dimension == 1 {
            out.dimension1(self, charset, code);
        } else {
            out.dimension2(self, charset, code >> 8, code & 0xFF);
        }
        Ok(())
    }

    fn encode_coding_iso_2022_1(
        &mut self,
        coding: &mut CodingSystem,
        charbuf: &[i32],
        dst: &mut Vec<u8>,
    ) -> LispResult<()> {
        let flags = coding.iso_flags;
        let mut bol_designation = flags & ISO_FLAG_DESIGNATE_AT_BOL != 0 && coding.iso_bol;
        let attrs = self.coding_id_attrs(coding.id);
        let mut eol_type = if self.var_true(sym::INHIBIT_EOL_CONVERSION) {
            sym::UNIX
        } else {
            self.coding_id_eol_type(coding.id)
        };
        if self.is_vector(eol_type) {
            eol_type = sym::UNIX;
        }
        self.setup_iso_safe_charsets(attrs)?;
        let charset_list = self.attr(attrs, ATTR_CHARSET_LIST);
        let safe = self.attr(attrs, ATTR_SAFE_CHARSETS);
        coding.safe_charsets = self.string_bytes(safe).to_vec();
        let ascii_compatible = !self.attr(attrs, ATTR_ASCII_COMPAT).is_nil()
            && flags & (ISO_FLAG_DESIGNATION | ISO_FLAG_LOCKING_SHIFT) == 0;
        let initial = self.iso_initial(coding);
        let ascii = self.charsets.ascii;
        let mut out = IsoOut {
            emit: Emit::new(coding.dst_multibyte),
            coding,
            dst,
        };
        let mut preferred_charset_id = -1;
        let mut i = 0;
        while i < charbuf.len() {
            if bol_designation {
                self.encode_designation_at_bol(&mut out, &charbuf[i..])?;
                bol_designation = false;
            }
            let mut c = charbuf[i];
            i += 1;
            if c < 0 {
                if charbuf.get(i).copied() == Some(ANNOTATE_CHARSET_MASK as i32) {
                    preferred_charset_id = charbuf.get(i + 2).copied().unwrap_or(-1);
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
            if c < 0x20 || c == 0x7F {
                if c == b'\n' as i32 || (c == b'\r' as i32 && eol_type == sym::MAC) {
                    if flags & ISO_FLAG_RESET_AT_EOL != 0 {
                        out.reset_plane_and_register(self, &initial);
                    }
                    if flags & ISO_FLAG_INIT_AT_BOL != 0 {
                        out.coding.iso_designation = initial;
                    }
                    bol_designation = flags & ISO_FLAG_DESIGNATE_AT_BOL != 0;
                } else if flags & ISO_FLAG_RESET_AT_CNTL != 0 {
                    out.reset_plane_and_register(self, &initial);
                }
                out.ascii(c);
            } else if c < 0x80 {
                if ascii_compatible {
                    out.ascii(c);
                } else {
                    self.encode_iso_character(&mut out, ascii, c)?;
                }
            } else if character::char_byte8_p(c as u32) {
                out.one(character::char_to_byte8(c as u32) as i32);
            } else {
                let mut charset = None;
                if preferred_charset_id >= 0 && self.char_charset_p(c, preferred_charset_id)? {
                    charset = Some(preferred_charset_id);
                }
                if charset.is_none() {
                    charset = self.char_charset(c, charset_list)?.map(|(id, _)| id);
                }
                if charset.is_none() {
                    if out.coding.mode & MODE_SAFE_ENCODING != 0 {
                        c = INHIBIT_CHARACTER_SUBSTITUTION;
                        charset = Some(ascii);
                    } else {
                        c = out.coding.default_char as i32;
                        charset = self.char_charset(c, charset_list)?.map(|(id, _)| id);
                    }
                }
                let charset = match charset {
                    Some(id) => id,
                    None => {
                        c = INHIBIT_CHARACTER_SUBSTITUTION;
                        ascii
                    }
                };
                self.encode_iso_character(&mut out, charset, c)?;
            }
        }
        if out.coding.mode & MODE_LAST_BLOCK != 0 && flags & ISO_FLAG_RESET_AT_EOL != 0 {
            out.reset_plane_and_register(self, &initial);
        }
        let produced_chars = out.emit.produced_chars;
        let produced = out.dst.len();
        let coding = out.coding;
        self.record_conversion_result(coding, CodingResult::Success);
        coding.iso_bol = bol_designation;
        coding.produced_char += produced_chars;
        coding.produced = produced;
        Ok(())
    }
}
