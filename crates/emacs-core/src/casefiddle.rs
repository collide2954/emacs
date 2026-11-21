use crate::character::{self, CHAR_MODIFIER_MASK, MAX_CHAR};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::syntax::{S_SYMBOL, S_WORD};
use crate::value::Value;

const REGION_SPEC: &str = "(list (region-beginning) (region-end) (region-noncontiguous-p))";

pub const SUBRS: &[Subr] = &[
    Subr::fixed("upcase", 1, 1, f_upcase),
    Subr::fixed("downcase", 1, 1, f_downcase),
    Subr::fixed("capitalize", 1, 1, f_capitalize),
    Subr::fixed("upcase-initials", 1, 1, f_upcase_initials),
    Subr::fixed("upcase-region", 2, 3, f_upcase_region).interactive(REGION_SPEC),
    Subr::fixed("downcase-region", 2, 3, f_downcase_region).interactive(REGION_SPEC),
    Subr::fixed("capitalize-region", 2, 3, f_capitalize_region).interactive(REGION_SPEC),
    Subr::fixed("upcase-initials-region", 2, 3, f_upcase_initials_region).interactive(REGION_SPEC),
    Subr::fixed("upcase-word", 1, 1, f_upcase_word).interactive("p"),
    Subr::fixed("downcase-word", 1, 1, f_downcase_word).interactive("p"),
    Subr::fixed("capitalize-word", 1, 1, f_capitalize_word).interactive("p"),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::REGION_EXTRACT_FUNCTION, Value::NIL);
    lisp.defvar_bool(sym::CASE_SYMBOLS_AS_WORDS, false);
    lisp.make_variable_buffer_local_builtin(sym::CASE_SYMBOLS_AS_WORDS);
}

pub fn upcase_char(c: u32) -> u32 {
    match char::from_u32(c) {
        Some(ch) => {
            let mut up = ch.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => u as u32,
                _ => c,
            }
        }
        None => c,
    }
}

pub fn downcase_char(c: u32) -> u32 {
    match char::from_u32(c) {
        Some(ch) => {
            let mut low = ch.to_lowercase();
            match (low.next(), low.next()) {
                (Some(l), None) => l as u32,
                _ => c,
            }
        }
        None => c,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CaseAction {
    Up = 0,
    Down = 1,
    Capitalize = 2,
    CapitalizeUp = 3,
}

struct CasingContext {
    titlecase_char_table: Value,
    specialcase_char_tables: [Value; 3],
    flag: CaseAction,
    inbuffer: bool,
    inword: bool,
    downcase_last: bool,
}

enum Cased {
    Char(u32),
    Special(Vec<u8>, usize),
}

const CASING_STR_BUF_SIZE: usize = if 6 < character::MAX_MULTIBYTE_LENGTH {
    character::MAX_MULTIBYTE_LENGTH
} else {
    6
};

const GREEK_CAPITAL_LETTER_SIGMA: u32 = 0x03A3;
const GREEK_SMALL_LETTER_FINAL_SIGMA: u32 = 0x03C2;

fn make_char_multibyte(c: u32) -> u32 {
    if character::ascii_char_p(c) {
        c
    } else {
        character::byte8_to_char(c as u8)
    }
}

fn make_char_unibyte(c: u32) -> u8 {
    if character::ascii_char_p(c) {
        c as u8
    } else {
        character::char_to_byte8(c)
    }
}

impl Lisp {
    fn prepare_casing_context(
        &mut self,
        flag: CaseAction,
        inbuffer: bool,
    ) -> LispResult<CasingContext> {
        let titlecase_char_table = if flag < CaseAction::Capitalize {
            Value::NIL
        } else {
            self.uniprop_table(sym::TITLECASE)?
        };
        self.stack.push(titlecase_char_table);
        let up = if flag == CaseAction::Down {
            Value::NIL
        } else {
            self.uniprop_table(sym::SPECIAL_UPPERCASE)?
        };
        self.stack.push(up);
        let down = if flag == CaseAction::Up {
            Value::NIL
        } else {
            self.uniprop_table(sym::SPECIAL_LOWERCASE)?
        };
        self.stack.push(down);
        let title = if flag < CaseAction::Capitalize {
            Value::NIL
        } else {
            self.uniprop_table(sym::SPECIAL_TITLECASE)?
        };
        self.stack.push(title);
        let downcase_table = self.cur().downcase_table;
        if self.char_table(downcase_table).extras[1].is_nil() {
            self.set_case_table(downcase_table, false)?;
        }
        if inbuffer && flag >= CaseAction::Capitalize {
            self.setup_buffer_syntax_table();
        }
        Ok(CasingContext {
            titlecase_char_table,
            specialcase_char_tables: [up, down, title],
            flag,
            inbuffer,
            inword: false,
            downcase_last: false,
        })
    }

    fn case_ch_is_word(&self, syntax: u8) -> bool {
        syntax == S_WORD || (self.var_true(sym::CASE_SYMBOLS_AS_WORDS) && syntax == S_SYMBOL)
    }

    fn syntax_prefix_flag_p(&self, c: u32) -> bool {
        (self.syntax_with_flags(c) >> 20) & 1 != 0
    }

    fn case_character_impl(&mut self, ctx: &mut CasingContext, ch: u32, with_buf: bool) -> Cased {
        let was_inword = ctx.inword;
        ctx.inword = self.case_ch_is_word(self.syntax_class(ch))
            && (!ctx.inbuffer || was_inword || !self.syntax_prefix_flag_p(ch));
        let flag = match ctx.flag {
            CaseAction::Capitalize if was_inword => CaseAction::Down,
            CaseAction::CapitalizeUp if was_inword => return Cased::Char(ch),
            CaseAction::CapitalizeUp => CaseAction::Capitalize,
            flag => flag,
        };
        let table = ctx.specialcase_char_tables[flag as usize];
        if with_buf && !table.is_nil() {
            let prop = self.char_table_ref_mut(table, ch);
            if prop.is_string() && self.string_bytes(prop).len() <= CASING_STR_BUF_SIZE {
                return Cased::Special(self.string_bytes(prop).to_vec(), self.string_chars(prop));
            }
        }
        if flag == CaseAction::Down {
            ctx.downcase_last = true;
            return Cased::Char(self.downcase_char_buffer(ch));
        }
        ctx.downcase_last = false;
        if !ctx.titlecase_char_table.is_nil() {
            let prop = self.char_table_ref_mut(ctx.titlecase_char_table, ch);
            if let Some(n) = prop.as_fixnum()
                && (0..=i64::from(MAX_CHAR)).contains(&n)
            {
                return Cased::Char(n as u32);
            }
        }
        Cased::Char(self.upcase_char_buffer(ch))
    }

    fn case_single_character(&mut self, ctx: &mut CasingContext, ch: u32) -> u32 {
        match self.case_character_impl(ctx, ch, false) {
            Cased::Char(c) => c,
            Cased::Special(..) => ch,
        }
    }

    fn case_character(
        &mut self,
        ctx: &mut CasingContext,
        ch: u32,
        next: Option<u32>,
    ) -> (bool, Vec<u8>, usize) {
        let was_inword = ctx.inword;
        let (changed, mut data, mut len_chars) = match self.case_character_impl(ctx, ch, true) {
            Cased::Special(data, chars) => (true, data, chars),
            Cased::Char(cased) => (cased != ch, character::char_string(cased), 1),
        };
        if was_inword
            && ch == GREEK_CAPITAL_LETTER_SIGMA
            && changed
            && next.is_none_or(|n| !self.case_ch_is_word(self.syntax_class(n)))
        {
            data = character::char_string(GREEK_SMALL_LETTER_FINAL_SIGMA);
            len_chars = 1;
        }
        (changed, data, len_chars)
    }

    fn do_casify_natnum(&mut self, ctx: &mut CasingContext, obj: Value) -> Value {
        let flagbits = CHAR_MODIFIER_MASK;
        let ch = obj.as_fixnum().unwrap_or(0);
        if !(0 <= ch && ch <= flagbits + i64::from(MAX_CHAR)) {
            return obj;
        }
        let flags = ch & flagbits;
        let ch = (ch & !flagbits) as u32;
        let multibyte = ch >= 256 || self.cur().text.multibyte;
        let ch = if multibyte {
            ch
        } else {
            make_char_multibyte(ch)
        };
        let cased = self.case_single_character(ctx, ch);
        if cased == ch {
            return obj;
        }
        let cased = if multibyte {
            cased
        } else {
            u32::from(make_char_unibyte(cased))
        };
        Value::fixnum(i64::from(cased) | flags)
    }

    fn do_casify_multibyte_string(&mut self, ctx: &mut CasingContext, obj: Value) -> Value {
        let src = self.string_bytes(obj).to_vec();
        let mut size = self.string_chars(obj);
        let mut dst = Vec::with_capacity(src.len());
        let mut n = 0;
        let mut i = 0;
        while size > 0 {
            let (ch, len) = character::string_char(&src[i..]);
            i += len;
            let next = if size > 1 {
                Some(character::string_char(&src[i..]).0)
            } else {
                None
            };
            let (_, data, chars) = self.case_character(ctx, ch, next);
            n += chars;
            dst.extend_from_slice(&data);
            size -= 1;
        }
        self.make_specified_string(dst, n, true)
    }

    fn ascii_casify_character(&mut self, downcase: bool, c: u32) -> LispResult<u32> {
        let table = self.uniprop_table(if downcase {
            sym::LOWERCASE
        } else {
            sym::UPPERCASE
        })?;
        if !self.is_char_table(table) {
            return Ok(c);
        }
        let cased = self.char_table_ref_mut(table, c);
        Ok(match cased.as_fixnum() {
            Some(n) if n >= 0 => n as u32,
            _ => c,
        })
    }

    fn do_casify_unibyte_string(&mut self, ctx: &mut CasingContext, obj: Value) -> LispResult {
        let size = self.string_chars(obj);
        let obj = crate::fns::f_copy_sequence(self, &[obj])?;
        self.stack.push(obj);
        for i in 0..size {
            let ch = make_char_multibyte(u32::from(self.string_bytes(obj)[i]));
            let mut cased = self.case_single_character(ctx, ch);
            if ch == cased {
                continue;
            }
            if character::ascii_char_p(ch) && !character::single_byte_char_p(cased) {
                cased = self.ascii_casify_character(ctx.downcase_last, ch)?;
            }
            self.heap.string_mut(obj).data[i] = make_char_unibyte(cased);
        }
        Ok(obj)
    }

    fn casify_object(&mut self, flag: CaseAction, obj: Value) -> LispResult {
        let base = self.stack.len();
        self.stack.push(obj);
        let r = self.casify_object_1(flag, obj);
        self.stack.truncate(base);
        r
    }

    fn casify_object_1(&mut self, flag: CaseAction, obj: Value) -> LispResult {
        let mut ctx = self.prepare_casing_context(flag, false)?;
        if obj.as_fixnum().is_some_and(|n| n >= 0) {
            return Ok(self.do_casify_natnum(&mut ctx, obj));
        }
        if !obj.is_string() {
            return Err(self.wrong_type(sym::CHAR_OR_STRING_P, obj));
        }
        if self.string_chars(obj) == 0 {
            return Ok(obj);
        }
        if self.string_multibyte(obj) {
            Ok(self.do_casify_multibyte_string(&mut ctx, obj))
        } else {
            self.do_casify_unibyte_string(&mut ctx, obj)
        }
    }

    fn do_casify_unibyte_region(
        &mut self,
        ctx: &mut CasingContext,
        start: usize,
        end: usize,
    ) -> (Option<usize>, usize) {
        let mut first = None;
        let mut last = 0;
        for pos in start..end {
            let ch = make_char_multibyte(u32::from(self.cur().text.byte_at(pos)));
            let cased = self.case_single_character(ctx, ch);
            if cased == ch {
                continue;
            }
            last = pos + 1;
            first.get_or_insert(pos);
            self.cur_mut().text.set_byte(pos, make_char_unibyte(cased));
        }
        (first, last)
    }

    fn do_casify_multibyte_region(
        &mut self,
        ctx: &mut CasingContext,
        start: usize,
        end: usize,
    ) -> (i64, Option<usize>, usize) {
        let mut first = None;
        let mut last = 0;
        let mut pos = start;
        let mut pos_byte = self.cur().text.char_to_byte(pos);
        let mut size = end - pos;
        let mut opoint = self.cur().pt as i64;
        let mut added: i64 = 0;
        while size > 0 {
            let (ch, len) = {
                let t = &self.cur().text;
                (t.char_at_byte(pos_byte), t.char_len_at_byte(pos_byte))
            };
            let next = if size > 1 {
                Some(self.cur().text.char_at_byte(pos_byte + len))
            } else {
                None
            };
            let (changed, data, len_chars) = self.case_character(ctx, ch, next);
            if !changed {
                pos_byte += len;
                pos += 1;
                size -= 1;
                continue;
            }
            last = pos + len_chars;
            first.get_or_insert(pos);
            if len_chars == 1 && data.len() == len {
                let t = &mut self.cur_mut().text;
                for (k, &byte) in data.iter().enumerate() {
                    t.set_byte(pos_byte + k, byte);
                }
            } else {
                self.replace_range_2(
                    pos,
                    pos_byte,
                    pos + 1,
                    pos_byte + len,
                    &data,
                    len_chars,
                    false,
                );
                added += len_chars as i64 - 1;
                if opoint > pos as i64 {
                    opoint += len_chars as i64 - 1;
                }
            }
            pos_byte += data.len();
            pos += len_chars;
            size -= 1;
        }
        if self.cur().pt as i64 != opoint {
            let b = self.cur_mut();
            b.pt = opoint as usize;
            b.pt_byte = b.text.char_to_byte(b.pt);
        }
        (added, first, last)
    }

    fn casify_region(&mut self, flag: CaseAction, b: Value, e: Value) -> LispResult<usize> {
        let (start, end) = self.validate_region(b, e)?;
        if start == end {
            return Ok(end);
        }
        self.modify_text(start, end)?;
        let base = self.stack.len();
        let r = self.casify_region_1(flag, start, end);
        self.stack.truncate(base);
        r
    }

    fn casify_region_1(&mut self, flag: CaseAction, start: usize, end: usize) -> LispResult<usize> {
        let mut ctx = self.prepare_casing_context(flag, true)?;
        let orig_end = end;
        let deleted = self.buffer_substring_value(start, end, true);
        self.record_delete(start, deleted, false);
        let (added, first, last) = if !self.cur().text.multibyte {
            self.record_insert(start, end - start);
            let (first, last) = self.do_casify_unibyte_region(&mut ctx, start, end);
            (0, first, last)
        } else {
            let len = end - start;
            let (added, first, last) = self.do_casify_multibyte_region(&mut ctx, start, end);
            self.record_insert(start, (len as i64 + added) as usize);
            (added, first, last)
        };
        if let Some(first) = first {
            let lendel = (last - first) as i64 - added;
            self.signal_after_change(first, lendel as usize, last - first)?;
            self.update_compositions(first, last, crate::composite::CHECK_ALL)?;
        }
        Ok((orig_end as i64 + added) as usize)
    }

    fn casify_pnc_region(
        &mut self,
        flag: CaseAction,
        beg: Value,
        end: Value,
        region_noncontiguous_p: Value,
    ) -> LispResult {
        if region_noncontiguous_p.is_nil() {
            self.casify_region(flag, beg, end)?;
            return Ok(Value::NIL);
        }
        let function = self.plain_value(sym::REGION_EXTRACT_FUNCTION);
        let bounds = self.funcall(function, &[sym::BOUNDS])?;
        let base = self.stack.len();
        let r = self.casify_bounds(flag, bounds);
        self.stack.truncate(base);
        r?;
        Ok(Value::NIL)
    }

    fn casify_bounds(&mut self, flag: CaseAction, bounds: Value) -> LispResult<()> {
        let mut li = self.for_each_tail(bounds);
        while li.tail.is_cons() {
            let elt = self.xcar(li.tail);
            if !elt.is_cons() {
                return Err(self.wrong_type(sym::CONSP, elt));
            }
            let (b, e) = (self.xcar(elt), self.xcdr(elt));
            self.casify_region(flag, b, e)?;
            self.tail_step(&mut li)?;
        }
        self.check_list_end(li.tail, li.tail)
    }

    fn casify_word(&mut self, flag: CaseAction, arg: Value) -> LispResult {
        let n = self.check_fixnum(arg)?;
        let pt = self.cur().pt;
        let mut farend = self.scan_words(pt as i64, n)?;
        if farend == 0 {
            farend = if n <= 0 {
                self.cur().begv as i64
            } else {
                self.cur().zv as i64
            };
        }
        let newpt = self.casify_region(flag, Value::fixnum(pt as i64), Value::fixnum(farend))?;
        self.goto_char_internal(newpt);
        Ok(Value::NIL)
    }
}

pub(crate) fn f_upcase(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_object(CaseAction::Up, args[0])
}

pub(crate) fn f_downcase(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_object(CaseAction::Down, args[0])
}

fn f_capitalize(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_object(CaseAction::Capitalize, args[0])
}

pub(crate) fn f_upcase_initials(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_object(CaseAction::CapitalizeUp, args[0])
}

pub(crate) fn f_upcase_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_pnc_region(CaseAction::Up, args[0], args[1], args[2])
}

fn f_downcase_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_pnc_region(CaseAction::Down, args[0], args[1], args[2])
}

fn f_capitalize_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_pnc_region(CaseAction::Capitalize, args[0], args[1], args[2])
}

pub(crate) fn f_upcase_initials_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_pnc_region(CaseAction::CapitalizeUp, args[0], args[1], args[2])
}

fn f_upcase_word(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_word(CaseAction::Up, args[0])
}

fn f_downcase_word(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_word(CaseAction::Down, args[0])
}

fn f_capitalize_word(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_word(CaseAction::Capitalize, args[0])
}
