use super::engine::{
    CATEGORY_MASK_ANY, CATEGORY_MASK_ISO_ESCAPE, CATEGORY_MASK_UTF_8, CATEGORY_MASK_UTF_16,
    DetectionInfo, EOL_SEEN_CR, EOL_SEEN_CRLF, EOL_SEEN_LF, category_mask, detect_eol,
    inhibit_flag,
};
use super::{
    ATTR_ASCII_COMPAT, ATTR_BASE_NAME, ATTR_CATEGORY, ATTR_CHARSET_LIST, ATTR_TRANS_TBL, ATTR_TYPE,
    CAT_ISO_7_ELSE, CAT_RAW_TEXT, CAT_UNDECIDED, CAT_UTF_8_AUTO, CAT_UTF_8_NOSIG, CAT_UTF_8_SIG,
    CAT_UTF_16_AUTO, CAT_UTF_16_BE, CAT_UTF_16_BE_NOSIG, CAT_UTF_16_LE, CAT_UTF_16_LE_NOSIG,
    MODE_LAST_BLOCK,
};
use crate::ccl::{jis_to_sjis, sjis_to_jis};
use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use std::collections::HashSet;

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

impl Lisp {
    pub fn detect_coding_system(
        &mut self,
        source: Vec<u8>,
        src_chars: usize,
        highest: bool,
        multibyte: bool,
        coding_system: Value,
    ) -> LispResult {
        let coding_system = if coding_system.is_nil() {
            sym::UNDECIDED
        } else {
            coding_system
        };
        let mut coding = self.setup_coding_system(coding_system)?;
        let attrs = self.coding_id_attrs(coding.id);
        let eol_type = self.coding_id_eol_type(coding.id);
        let src_bytes = source.len();
        coding.source = source;
        coding.src_chars = src_chars;
        coding.src_bytes = src_bytes;
        coding.src_multibyte = multibyte;
        coding.consumed = 0;
        coding.mode |= MODE_LAST_BLOCK;
        coding.head_ascii = 0;
        let mut info = DetectionInfo::default();
        let mut val: Vec<i64> = Vec::new();
        let mut null_byte_found = false;
        let mut eight_bit_found = false;
        let base_category = self.attr(attrs, ATTR_CATEGORY).as_fixnum().unwrap_or(0) as usize;
        if base_category == CAT_UNDECIDED {
            let mut category = 0usize;
            let mut this_id = -1i64;
            let inhibit_nbd = inhibit_flag(
                coding.undecided_inhibit_nbd,
                self.var_true(sym::INHIBIT_NULL_BYTE_DETECTION),
            );
            let inhibit_ied = inhibit_flag(
                coding.undecided_inhibit_ied,
                self.var_true(sym::INHIBIT_ISO_ESCAPE_DETECTION),
            );
            let prefer_utf_8 = coding.undecided_prefer_utf_8;
            let mut src = 0usize;
            while src < src_bytes {
                let c = coding.source[src];
                if c & 0x80 != 0 {
                    eight_bit_found = true;
                    if null_byte_found {
                        break;
                    }
                } else if c < 0x20 {
                    if (c == 0x1B || c == 0x0F || c == 0x0E) && !inhibit_ied && info.checked == 0 {
                        if self.detect_coding_iso_2022(&mut coding, &mut info)? {
                            if info.rejected & category_mask(CAT_ISO_7_ELSE) == 0 {
                                coding.head_ascii = src_bytes as i64;
                            }
                            info.rejected |= !CATEGORY_MASK_ISO_ESCAPE;
                            break;
                        }
                    } else if c == 0 && !inhibit_nbd {
                        null_byte_found = true;
                        if eight_bit_found {
                            break;
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
                || coding.head_ascii < src_bytes as i64
                || info.found != 0
            {
                if coding.head_ascii == src_bytes as i64 {
                    for i in 0..CAT_RAW_TEXT {
                        category = self.codings.priorities[i];
                        this_id = self.codings.categories[category].id;
                        if info.found & category_mask(category) != 0 {
                            break;
                        }
                    }
                } else {
                    if null_byte_found {
                        info.checked |= !CATEGORY_MASK_UTF_16;
                        info.rejected |= !CATEGORY_MASK_UTF_16;
                    } else if prefer_utf_8 && self.detect_coding_utf_8(&mut coding, &mut info)? {
                        info.checked |= !CATEGORY_MASK_UTF_8;
                        info.rejected |= !CATEGORY_MASK_UTF_8;
                    }
                    for i in 0..CAT_RAW_TEXT {
                        category = self.codings.priorities[i];
                        this_id = self.codings.categories[category].id;
                        let mask = category_mask(category);
                        if this_id < 0 {
                            info.rejected |= mask;
                        } else if category >= CAT_RAW_TEXT {
                            continue;
                        } else if info.checked & mask != 0 {
                            if highest && info.found & mask != 0 {
                                break;
                            }
                        } else if self.run_detector(&mut coding, &mut info, category)?
                            && highest
                            && info.found & mask != 0
                        {
                            if category == CAT_UTF_16_AUTO {
                                category = if info.found & category_mask(CAT_UTF_16_LE) != 0 {
                                    CAT_UTF_16_LE
                                } else {
                                    CAT_UTF_16_BE
                                };
                            }
                            break;
                        }
                    }
                }
            }
            if info.rejected & CATEGORY_MASK_ANY == CATEGORY_MASK_ANY || null_byte_found {
                info.found = category_mask(CAT_RAW_TEXT);
                val = vec![self.coding_system_id(sym::NO_CONVERSION)?];
            } else if info.rejected == 0 && info.found == 0 {
                info.found = CATEGORY_MASK_ANY;
                val = vec![self.codings.categories[CAT_UNDECIDED].id];
            } else if highest {
                if info.found != 0 {
                    info.found = category_mask(category);
                    val = vec![this_id];
                } else {
                    for i in 0..CAT_RAW_TEXT {
                        let p = self.codings.priorities[i];
                        if info.rejected & category_mask(p) == 0 {
                            info.found = category_mask(p);
                            val = vec![self.codings.categories[p].id];
                            break;
                        }
                    }
                }
            } else {
                let mask = info.rejected | info.found;
                let mut found = 0;
                for i in (0..CAT_RAW_TEXT).rev() {
                    let category = self.codings.priorities[i];
                    if mask & category_mask(category) == 0 {
                        found |= category_mask(category);
                        let id = self.codings.categories[category].id;
                        if id >= 0 {
                            val = vec![id];
                        }
                    }
                }
                for i in (0..CAT_RAW_TEXT).rev() {
                    let category = self.codings.priorities[i];
                    if info.found & category_mask(category) != 0 {
                        val.insert(0, self.codings.categories[category].id);
                    }
                }
                info.found |= found;
            }
        } else if base_category == CAT_UTF_8_AUTO {
            if self.detect_coding_utf_8(&mut coding, &mut info)? {
                let cat = if info.found & category_mask(CAT_UTF_8_SIG) != 0 {
                    CAT_UTF_8_SIG
                } else {
                    CAT_UTF_8_NOSIG
                };
                val = vec![self.codings.categories[cat].id];
            }
        } else if base_category == CAT_UTF_16_AUTO {
            if self.detect_coding_utf_16(&mut coding, &mut info)? {
                let cat = if info.found & category_mask(CAT_UTF_16_LE) != 0 {
                    CAT_UTF_16_LE
                } else if info.found & category_mask(CAT_UTF_16_BE) != 0 {
                    CAT_UTF_16_BE
                } else if info.rejected & category_mask(CAT_UTF_16_LE_NOSIG) != 0 {
                    CAT_UTF_16_BE_NOSIG
                } else {
                    CAT_UTF_16_LE_NOSIG
                };
                val = vec![self.codings.categories[cat].id];
            }
        } else {
            info.found = category_mask(base_category);
            val = vec![coding.id];
        }
        let none = u32::MAX;
        let (mut normal_eol, mut be_eol, mut le_eol) = (none, none, none);
        if self.is_vector(eol_type) {
            if info.found & !CATEGORY_MASK_UTF_16 != 0 {
                normal_eol = if null_byte_found {
                    EOL_SEEN_LF
                } else {
                    detect_eol(&coding.source, CAT_RAW_TEXT)
                };
            }
            if info.found & (category_mask(CAT_UTF_16_BE) | category_mask(CAT_UTF_16_BE_NOSIG)) != 0
            {
                be_eol = detect_eol(&coding.source, CAT_UTF_16_BE);
            }
            if info.found & (category_mask(CAT_UTF_16_LE) | category_mask(CAT_UTF_16_LE_NOSIG)) != 0
            {
                le_eol = detect_eol(&coding.source, CAT_UTF_16_LE);
            }
        } else {
            let eol = if eol_type == sym::UNIX {
                EOL_SEEN_LF
            } else if eol_type == sym::DOS {
                EOL_SEEN_CRLF
            } else {
                EOL_SEEN_CR
            };
            (normal_eol, be_eol, le_eol) = (eol, eol, eol);
        }
        let mut names = Vec::with_capacity(val.len());
        for id in val {
            let attrs = self.coding_id_attrs(id);
            let category = self.attr(attrs, ATTR_CATEGORY).as_fixnum().unwrap_or(0) as usize;
            let eol_type = self.coding_id_eol_type(id);
            let name = if self.is_vector(eol_type) {
                let this_eol = if category == CAT_UTF_16_BE || category == CAT_UTF_16_BE_NOSIG {
                    be_eol
                } else if category == CAT_UTF_16_LE || category == CAT_UTF_16_LE_NOSIG {
                    le_eol
                } else {
                    normal_eol
                };
                let items = self.vector_items(eol_type);
                match this_eol {
                    EOL_SEEN_LF => items[0],
                    EOL_SEEN_CRLF => items[1],
                    EOL_SEEN_CR => items[2],
                    _ => self.coding_id_name(id),
                }
            } else {
                self.coding_id_name(id)
            };
            names.push(name);
        }
        if highest {
            Ok(names.first().copied().unwrap_or(Value::NIL))
        } else {
            Ok(self.list(&names))
        }
    }

    fn region_bytes(
        &mut self,
        start: Value,
        end: Value,
    ) -> LispResult<(usize, usize, usize, usize)> {
        let (from, to) = self.validate_region(start, end)?;
        let text = &self.cur().text;
        Ok((from, text.char_to_byte(from), to, text.char_to_byte(to)))
    }

    fn char_encodable_p(&mut self, c: i32, attrs: Value) -> LispResult<bool> {
        let table = self.attr(attrs, ATTR_TRANS_TBL);
        let c = if table.is_nil() {
            c
        } else {
            self.translate_char(table, c as u32) as i32
        };
        let mut tail = self.attr(attrs, ATTR_CHARSET_LIST);
        while tail.is_cons() {
            let id = self.xcar(tail).as_fixnum().unwrap_or(0) as i32;
            if self.char_charset_p(c, id)? {
                return Ok(true);
            }
            tail = self.xcdr(tail);
        }
        Ok(false)
    }

    fn check_region_text(
        &mut self,
        start: Value,
        end: Value,
    ) -> LispResult<Option<(Vec<u8>, usize)>> {
        if self.is_string(start) {
            if !self.string_multibyte(start)
                || self.string_chars(start) == self.string_bytes(start).len()
            {
                return Ok(None);
            }
            return Ok(Some((self.string_bytes(start).to_vec(), 0)));
        }
        let s = self.fix_position(start)?;
        let e = self.fix_position(end)?;
        let z = self.cur().z() as i64;
        if !(1 <= s && s <= e && e <= z) {
            return Err(self.args_out_of_range(start, end));
        }
        if !self.cur().text.multibyte {
            return Ok(None);
        }
        let (s, e) = (s as usize, e as usize);
        let text = &self.cur().text;
        let start_byte = text.char_to_byte(s);
        let end_byte = text.char_to_byte(e);
        if e - s == end_byte - start_byte {
            return Ok(None);
        }
        Ok(Some((text.slice(start_byte - 1, end_byte - 1), s)))
    }

    pub(crate) fn check_coding_system_arg(&mut self, coding_system: Value) -> LispResult<()> {
        if self.coding_system_id(coding_system)? < 0 {
            self.check_coding_system(coding_system)?;
        }
        Ok(())
    }

    fn set_last_coding_system_used(&mut self, value: Value) {
        self.set_plain_value(sym::LAST_CODING_SYSTEM_USED, value);
    }

    pub fn code_convert_region(
        &mut self,
        start: Value,
        end: Value,
        coding_system: Value,
        dst_object: Value,
        encodep: bool,
        norecord: bool,
    ) -> LispResult {
        let coding_system = if coding_system.is_nil() {
            sym::NO_CONVERSION
        } else {
            self.check_coding_system_arg(coding_system)?;
            coding_system
        };
        let src_object = self.current_buffer;
        let dst_object = if dst_object.is_nil() {
            src_object
        } else {
            if dst_object != sym::T {
                self.check_buffer(dst_object)?;
            }
            dst_object
        };
        let range = self.region_bytes(start, end)?;
        let mut coding = self.setup_coding_system(coding_system)?;
        coding.mode |= MODE_LAST_BLOCK;
        if encodep {
            self.encode_coding_object(&mut coding, src_object, range, dst_object)?;
        } else {
            self.decode_coding_object(&mut coding, src_object, range, dst_object)?;
        }
        if !norecord {
            let name = self.coding_id_name(coding.id);
            self.set_last_coding_system_used(name);
        }
        Ok(if self.is_buffer(dst_object) {
            Value::fixnum(coding.produced_char as i64)
        } else {
            coding.dst_object
        })
    }

    pub fn code_convert_string(
        &mut self,
        string: Value,
        coding_system: Value,
        dst_object: Value,
        encodep: bool,
        nocopy: bool,
        norecord: bool,
    ) -> LispResult {
        self.check_string(string)?;
        if coding_system.is_nil() {
            if !norecord {
                self.set_last_coding_system_used(sym::NO_CONVERSION);
            }
            if dst_object.is_nil() {
                return if nocopy {
                    Ok(string)
                } else {
                    crate::fns::f_copy_sequence(self, &[string])
                };
            }
        }
        let coding_system = if coding_system.is_nil() {
            sym::NO_CONVERSION
        } else {
            self.check_coding_system_arg(coding_system)?;
            coding_system
        };
        let dst_object = if dst_object.is_nil() {
            sym::T
        } else {
            if dst_object != sym::T {
                self.check_buffer(dst_object)?;
            }
            dst_object
        };
        let mut coding = self.setup_coding_system(coding_system)?;
        coding.mode |= MODE_LAST_BLOCK;
        let chars = self.string_chars(string);
        let bytes = self.string_bytes(string).len();
        if dst_object == sym::T {
            let attrs = self.coding_id_attrs(coding.id);
            let data = self.string_bytes(string);
            let plain = if self.string_multibyte(string) {
                chars == bytes
            } else {
                data.iter().all(|&b| b < 0x80)
            };
            let has_eol = data.contains(if encodep { &b'\n' } else { &b'\r' });
            if !self.attr(attrs, ATTR_ASCII_COMPAT).is_nil()
                && plain
                && (self.coding_id_eol_type(coding.id) == sym::UNIX
                    || self.var_true(sym::INHIBIT_EOL_CONVERSION)
                    || !has_eol)
            {
                if !norecord {
                    self.set_last_coding_system_used(coding_system);
                }
                if nocopy {
                    return Ok(string);
                }
                let data = self.string_bytes(string).to_vec();
                return Ok(if encodep {
                    self.unibyte_string(&data)
                } else {
                    self.make_specified_string(data, bytes, true)
                });
            }
        }
        if encodep {
            self.encode_coding_object(&mut coding, string, (0, 0, chars, bytes), dst_object)?;
        } else {
            self.decode_coding_object(&mut coding, string, (0, 0, chars, bytes), dst_object)?;
        }
        if !norecord {
            let name = self.coding_id_name(coding.id);
            self.set_last_coding_system_used(name);
        }
        Ok(if self.is_buffer(dst_object) {
            Value::fixnum(coding.produced_char as i64)
        } else {
            coding.dst_object
        })
    }

    pub fn code_convert_string_norecord(
        &mut self,
        string: Value,
        coding_system: Value,
        encodep: bool,
    ) -> LispResult {
        self.code_convert_string(string, coding_system, sym::T, encodep, false, true)
    }

    fn charset_list_ids(&self, list: Value) -> Vec<i32> {
        let mut out = Vec::new();
        let mut tail = list;
        while tail.is_cons() {
            out.push(self.xcar(tail).as_fixnum().unwrap_or(0) as i32);
            tail = self.xcdr(tail);
        }
        out
    }
}

pub(super) fn f_detect_coding_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (from, from_byte, to, to_byte) = lisp.region_bytes(args[0], args[1])?;
    let text = &lisp.cur().text;
    let bytes = text.slice(from_byte - 1, to_byte - 1);
    let multibyte = text.multibyte;
    lisp.detect_coding_system(
        bytes,
        to - from,
        !arg(args, 2).is_nil(),
        multibyte,
        Value::NIL,
    )
}

pub(super) fn f_detect_coding_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let string = lisp.check_string(args[0])?;
    let bytes = lisp.string_bytes(string).to_vec();
    let chars = lisp.string_chars(string);
    let multibyte = lisp.string_multibyte(string);
    lisp.detect_coding_system(bytes, chars, !arg(args, 1).is_nil(), multibyte, Value::NIL)
}

pub(super) fn f_find_coding_systems_region_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let exclude = arg(args, 2);
    let Some((bytes, _)) = lisp.check_region_text(args[0], args[1])? else {
        return Ok(sym::T);
    };
    let mut attrs_list = Vec::new();
    let mut tail = lisp.bound_var(sym::CODING_SYSTEM_LIST);
    while tail.is_cons() {
        let cs = lisp.xcar(tail);
        if exclude.is_nil() || lisp.memq(cs, exclude).is_nil() {
            let spec = lisp.coding_system_spec(cs)?;
            let attrs = lisp.vector_items(spec)[0];
            if cs == lisp.attr(attrs, ATTR_BASE_NAME) {
                let (table, _) = lisp.get_translation_table(attrs, true)?;
                lisp.set_attr(attrs, ATTR_TRANS_TBL, table);
                attrs_list.insert(0, Some(attrs));
            }
        }
        tail = lisp.xcdr(tail);
    }
    let mut p = 0;
    let mut pend = bytes.len();
    while p < pend && bytes[p] < 0x80 {
        p += 1;
    }
    while p < pend && bytes[pend - 1] < 0x80 {
        pend -= 1;
    }
    let mut checked = HashSet::new();
    while p < pend {
        if bytes[p] < 0x80 {
            p += 1;
            continue;
        }
        let (c, len) = character::string_char(&bytes[p..pend]);
        p += len;
        if !checked.insert(c) {
            continue;
        }
        let mut i = 0;
        while i < attrs_list.len() {
            match attrs_list[i] {
                None => i += 1,
                Some(attrs) if lisp.char_encodable_p(c as i32, attrs)? => i += 1,
                Some(_) if i + 1 < attrs_list.len() => {
                    attrs_list.remove(i);
                }
                Some(_) => {
                    attrs_list[i] = None;
                    i += 1;
                }
            }
        }
    }
    let mut safe = lisp.list(&[sym::RAW_TEXT, sym::NO_CONVERSION]);
    for attrs in attrs_list.into_iter().flatten() {
        let name = lisp.attr(attrs, ATTR_BASE_NAME);
        safe = lisp.cons(name, safe);
    }
    Ok(safe)
}

pub(super) fn f_unencodable_char_position(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (start, end, coding_system, count, string) =
        (args[0], args[1], args[2], arg(args, 3), arg(args, 4));
    let cs = lisp.check_coding_system(coding_system)?;
    let coding = lisp.setup_coding_system(cs)?;
    let attrs = lisp.coding_id_attrs(coding.id);
    if lisp.attr(attrs, ATTR_TYPE) == sym::RAW_TEXT {
        return Ok(Value::NIL);
    }
    let ascii_compatible = !lisp.attr(attrs, ATTR_ASCII_COMPAT).is_nil();
    let charset_list = lisp.attr(attrs, ATTR_CHARSET_LIST);
    let (translation_table, _) = lisp.get_translation_table(attrs, true)?;
    let (bytes, mut from) = if string.is_nil() {
        let (from, from_byte, to, to_byte) = lisp.region_bytes(start, end)?;
        let text = &lisp.cur().text;
        if !text.multibyte || (ascii_compatible && to - from == to_byte - from_byte) {
            return Ok(Value::NIL);
        }
        (text.slice(from_byte - 1, to_byte - 1), from)
    } else {
        lisp.check_string(string)?;
        let size = lisp.string_chars(string);
        let (from, to) = lisp.validate_subarray(string, start, end, size)?;
        if !lisp.string_multibyte(string) {
            return Ok(Value::NIL);
        }
        let p = lisp.string_char_to_byte_index(string, from);
        let pend = lisp.string_char_to_byte_index(string, to);
        if ascii_compatible && to - from == pend - p {
            return Ok(Value::NIL);
        }
        (lisp.string_bytes(string)[p..pend].to_vec(), from)
    };
    let mut n = if count.is_nil() {
        1
    } else {
        lisp.check_fixnat(count)?
    };
    let mut positions = Vec::new();
    let mut p = 0;
    loop {
        if ascii_compatible {
            while p < bytes.len() && bytes[p] < 0x80 {
                p += 1;
                from += 1;
            }
        }
        if p >= bytes.len() {
            break;
        }
        let (c, len) = character::string_char(&bytes[p..]);
        p += len;
        if !(c < 0x80 && ascii_compatible) {
            let t = lisp.translate_char(translation_table, c) as i32;
            if lisp.char_charset(t, charset_list)?.is_none() {
                positions.push(Value::fixnum(from as i64));
                n -= 1;
                if n == 0 {
                    break;
                }
            }
        }
        from += 1;
    }
    if count.is_nil() {
        Ok(positions.first().copied().unwrap_or(Value::NIL))
    } else {
        Ok(lisp.list(&positions))
    }
}

pub(super) fn f_check_coding_systems_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let Some((bytes, mut pos)) = lisp.check_region_text(args[0], args[1])? else {
        return Ok(Value::NIL);
    };
    let mut entries: Vec<(Value, Value, Vec<usize>)> = Vec::new();
    let mut tail = args[2];
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        let spec = lisp.coding_system_spec(elt)?;
        if !lisp.is_vector(spec) {
            return Err(lisp.signal_list(sym::CODING_SYSTEM_ERROR, &[elt]));
        }
        let attrs = lisp.vector_items(spec)[0];
        let (table, _) = lisp.get_translation_table(attrs, true)?;
        lisp.set_attr(attrs, ATTR_TRANS_TBL, table);
        entries.insert(0, (elt, attrs, Vec::new()));
        tail = lisp.xcdr(tail);
    }
    let mut p = 0;
    let mut pend = bytes.len();
    while p < pend && bytes[p] < 0x80 {
        p += 1;
        pos += 1;
    }
    while p < pend && bytes[pend - 1] < 0x80 {
        pend -= 1;
    }
    while p < pend {
        if bytes[p] < 0x80 {
            p += 1;
        } else {
            let (c, len) = character::string_char(&bytes[p..pend]);
            p += len;
            for entry in entries.iter_mut() {
                if !lisp.char_encodable_p(c as i32, entry.1)? {
                    entry.2.push(pos);
                }
            }
        }
        pos += 1;
    }
    let mut list = Value::NIL;
    for (elt, _, positions) in entries {
        if !positions.is_empty() {
            let mut items = vec![elt];
            items.extend(positions.into_iter().map(|p| Value::fixnum(p as i64)));
            let item = lisp.list(&items);
            list = lisp.cons(item, list);
        }
    }
    Ok(list)
}

pub(super) fn f_decode_coding_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.code_convert_region(args[0], args[1], args[2], arg(args, 3), false, false)
}

pub(super) fn f_encode_coding_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.code_convert_region(args[0], args[1], args[2], arg(args, 3), true, false)
}

pub(super) fn f_decode_coding_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let nocopy = !arg(args, 2).is_nil();
    lisp.code_convert_string(args[0], args[1], arg(args, 3), false, nocopy, false)
}

pub(super) fn f_encode_coding_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let nocopy = !arg(args, 2).is_nil();
    lisp.code_convert_string(args[0], args[1], arg(args, 3), true, nocopy, false)
}

fn invalid_code(lisp: &mut Lisp, ch: i64) -> crate::error::Flow {
    lisp.error(&format!("Invalid code: {ch}"))
}

pub(super) fn f_decode_sjis_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let ch = lisp.check_fixnat(args[0])?;
    let cs = lisp.codings.sjis_coding_system;
    let spec = lisp.check_coding_system_get_spec(cs)?;
    let attrs = lisp.vector_items(spec)[0];
    if ch < 0x80 && !lisp.attr(attrs, ATTR_ASCII_COMPAT).is_nil() {
        return Ok(args[0]);
    }
    let list = lisp.attr(attrs, ATTR_CHARSET_LIST);
    let ids = lisp.charset_list_ids(list);
    let (c, charset) = if ch <= 0x7F {
        (ch as i32, ids[0])
    } else if (0xA0..0xDF).contains(&ch) {
        ((ch - 0x80) as i32, ids[1])
    } else {
        let c1 = ch >> 8;
        let c2 = ch & 0xFF;
        if c1 < 0x81
            || (c1 > 0x9F && c1 < 0xE0)
            || c1 > 0xEF
            || c2 < 0x40
            || c2 == 0x7F
            || c2 > 0xFC
        {
            return Err(invalid_code(lisp, ch));
        }
        (sjis_to_jis(ch as i32), ids[2])
    };
    let c = lisp.decode_char(charset, c as u32)?;
    if c < 0 {
        return Err(invalid_code(lisp, ch));
    }
    Ok(Value::fixnum(c as i64))
}

pub(super) fn f_encode_sjis_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])? as i32;
    let cs = lisp.codings.sjis_coding_system;
    let spec = lisp.check_coding_system_get_spec(cs)?;
    let attrs = lisp.vector_items(spec)[0];
    if c < 0x80 && !lisp.attr(attrs, ATTR_ASCII_COMPAT).is_nil() {
        return Ok(args[0]);
    }
    let list = lisp.attr(attrs, ATTR_CHARSET_LIST);
    match lisp.char_charset(c, list)? {
        Some((_, code)) => Ok(Value::fixnum(jis_to_sjis(code as i32) as i64)),
        None => Err(lisp.error(&format!(
            "Can't encode by shift_jis encoding: {}",
            char::from_u32(c as u32).unwrap_or('?')
        ))),
    }
}

pub(super) fn f_decode_big5_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let ch = lisp.check_fixnat(args[0])?;
    let cs = lisp.codings.big5_coding_system;
    let spec = lisp.check_coding_system_get_spec(cs)?;
    let attrs = lisp.vector_items(spec)[0];
    if ch < 0x80 && !lisp.attr(attrs, ATTR_ASCII_COMPAT).is_nil() {
        return Ok(args[0]);
    }
    let list = lisp.attr(attrs, ATTR_CHARSET_LIST);
    let ids = lisp.charset_list_ids(list);
    let charset = if ch <= 0x7F {
        ids[0]
    } else {
        let b1 = ch >> 8;
        let b2 = ch & 0x7F;
        if !(0xA1..=0xFE).contains(&b1) || b2 < 0x40 || (b2 > 0x7E && b2 < 0xA1) || b2 > 0xFE {
            return Err(invalid_code(lisp, ch));
        }
        ids[1]
    };
    let c = lisp.decode_char(charset, ch as u32)?;
    if c < 0 {
        return Err(invalid_code(lisp, ch));
    }
    Ok(Value::fixnum(c as i64))
}

pub(super) fn f_encode_big5_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])? as i32;
    let cs = lisp.codings.big5_coding_system;
    let spec = lisp.check_coding_system_get_spec(cs)?;
    let attrs = lisp.vector_items(spec)[0];
    if c < 0x80 && !lisp.attr(attrs, ATTR_ASCII_COMPAT).is_nil() {
        return Ok(args[0]);
    }
    let list = lisp.attr(attrs, ATTR_CHARSET_LIST);
    match lisp.char_charset(c, list)? {
        Some((_, code)) => Ok(Value::fixnum(code as i64)),
        None => Err(lisp.error(&format!(
            "Can't encode by Big5 encoding: {}",
            char::from_u32(c as u32).unwrap_or('?')
        ))),
    }
}

pub(crate) fn f_find_operation_coding_system(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args.len() < 2 {
        return Err(lisp.error("Too few arguments"));
    }
    let operation = args[0];
    let target_idx = if operation.is_symbol() {
        lisp.get(operation, sym::TARGET_IDX)
            .as_fixnum()
            .filter(|&n| n >= 0)
    } else {
        None
    };
    let Some(target_idx) = target_idx else {
        return Err(lisp.error("Invalid first argument"));
    };
    let name = lisp.symbol_name_string(operation);
    if args.len() as i64 <= 1 + target_idx {
        return Err(lisp.error(&format!("Too few arguments for operation `{name}'")));
    }
    let mut target = args[target_idx as usize + 1];
    let valid = lisp.is_string(target)
        || (operation == sym::INSERT_FILE_CONTENTS
            && target.is_cons()
            && lisp.is_string(lisp.xcar(target))
            && lisp.is_buffer(lisp.xcdr(target)))
        || (operation == sym::OPEN_NETWORK_STREAM && (target.is_fixnum() || target == sym::T));
    if !valid {
        return Err(lisp.error(&format!(
            "Invalid argument {} of operation `{name}'",
            target_idx + 1
        )));
    }
    if target.is_cons() {
        target = lisp.xcar(target);
    }
    let mut chain = if operation == sym::INSERT_FILE_CONTENTS || operation == sym::WRITE_REGION {
        lisp.bound_var(sym::FILE_CODING_SYSTEM_ALIST)
    } else if operation == sym::OPEN_NETWORK_STREAM {
        lisp.bound_var(sym::NETWORK_CODING_SYSTEM_ALIST)
    } else {
        lisp.bound_var(sym::PROCESS_CODING_SYSTEM_ALIST)
    };
    let base = lisp.stack.len();
    lisp.stack.push(chain);
    lisp.stack.push(target);
    let r = find_operation_coding_system_1(lisp, args, &mut chain, target);
    lisp.stack.truncate(base);
    r
}

fn find_operation_coding_system_1(
    lisp: &mut Lisp,
    args: &[Value],
    chain: &mut Value,
    target: Value,
) -> LispResult {
    while chain.is_cons() {
        let elt = lisp.xcar(*chain);
        if elt.is_cons() {
            let key = lisp.xcar(elt);
            let matched = if lisp.is_string(target) && lisp.is_string(key) {
                lisp.fast_string_match(key, target, Value::NIL)? >= 0
            } else {
                target.is_fixnum() && target == key
            };
            if matched {
                let val = lisp.xcdr(elt);
                if val.is_cons() {
                    return Ok(val);
                }
                if !val.is_symbol() {
                    return Ok(Value::NIL);
                }
                if lisp.coding_system_p(val)? {
                    return Ok(lisp.cons(val, val));
                }
                if !lisp.heap.symbol_ref(val).function.is_nil() {
                    let list = lisp.list(args);
                    let val = lisp.funcall(val, &[list])?;
                    if val.is_cons() {
                        return Ok(val);
                    }
                    if val.is_symbol() && lisp.coding_system_p(val)? {
                        return Ok(lisp.cons(val, val));
                    }
                }
                return Ok(Value::NIL);
            }
        }
        *chain = lisp.xcdr(*chain);
    }
    Ok(Value::NIL)
}
