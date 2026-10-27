use crate::character::{
    self, CHAR_ALT, CHAR_CTL, CHAR_HYPER, CHAR_META, CHAR_SHIFT, CHAR_SUPER, MAX_CHAR,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("characterp", 1, 2, f_characterp),
    Subr::fixed("max-char", 0, 1, f_max_char),
    Subr::fixed(
        "unibyte-char-to-multibyte",
        1,
        1,
        f_unibyte_char_to_multibyte,
    ),
    Subr::fixed(
        "multibyte-char-to-unibyte",
        1,
        1,
        f_multibyte_char_to_unibyte,
    ),
    Subr::fixed("char-width", 1, 1, f_char_width),
    Subr::fixed("string-width", 1, 3, f_string_width),
    Subr::many("string", 0, f_string),
    Subr::many("unibyte-string", 0, f_unibyte_string),
    Subr::fixed("char-resolve-modifiers", 1, 1, f_char_resolve_modifiers),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::TRANSLATION_TABLE_FOR_INPUT, Value::NIL);
    lisp.defvar(sym::AUTO_FILL_CHARS, Value::NIL);
    lisp.defvar(sym::CHAR_WIDTH_TABLE, Value::NIL);
    lisp.defvar(sym::AMBIGUOUS_WIDTH_CHARS, Value::NIL);
    lisp.defvar(sym::PRINTABLE_CHARS, Value::NIL);
    lisp.defvar(sym::CHAR_SCRIPT_TABLE, Value::NIL);
    lisp.defvar(sym::SCRIPT_REPRESENTATIVE_CHARS, Value::NIL);
    lisp.defvar(sym::UNICODE_CATEGORY_TABLE, Value::NIL);
}

fn f_characterp(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(
        if args[0]
            .as_fixnum()
            .is_some_and(|n| (0..=MAX_CHAR as i64).contains(&n))
        {
            sym::T
        } else {
            Value::NIL
        },
    )
}

fn f_max_char(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(Value::fixnum(if args[0].is_nil() {
        MAX_CHAR as i64
    } else {
        character::MAX_UNICODE_CHAR as i64
    }))
}

fn f_unibyte_char_to_multibyte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    if c >= 0x100 {
        return Err(lisp.error("Not a unibyte character: %d"));
    }
    if c < 0x80 {
        return Ok(args[0]);
    }
    Ok(Value::fixnum(character::byte8_to_char(c as u8) as i64))
}

fn f_multibyte_char_to_unibyte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    if c < 0x80 {
        return Ok(args[0]);
    }
    if character::char_byte8_p(c) {
        return Ok(Value::fixnum(character::char_to_byte8(c) as i64));
    }
    Ok(Value::fixnum(-1))
}

pub fn char_width(c: u32) -> usize {
    if c == 9 {
        return 8;
    }
    if c < 0x20 || c == 0x7F {
        return 2;
    }
    if c < 0x7F {
        return 1;
    }
    if character::char_byte8_p(c) {
        return 4;
    }
    if (0x0300..=0x036F).contains(&c) || c == 0x200B || (0x200C..=0x200F).contains(&c) {
        return 0;
    }
    let wide = (0x1100..=0x115F).contains(&c)
        || (0x2E80..=0xA4CF).contains(&c)
        || (0xAC00..=0xD7A3).contains(&c)
        || (0xF900..=0xFAFF).contains(&c)
        || (0xFE30..=0xFE4F).contains(&c)
        || (0xFF00..=0xFF60).contains(&c)
        || (0xFFE0..=0xFFE6).contains(&c)
        || (0x1F300..=0x1F64F).contains(&c)
        || (0x1F900..=0x1F9FF).contains(&c)
        || (0x20000..=0x3FFFD).contains(&c);
    if wide { 2 } else { 1 }
}

impl Lisp {
    pub fn char_width_of(&self, c: u32) -> usize {
        let table = self.plain_value(sym::CHAR_WIDTH_TABLE);
        if self.is_char_table(table) {
            let w = self.char_table_ref(table, c);
            if let Some(n) = w.as_fixnum() {
                return n.max(0) as usize;
            }
        }
        char_width(c)
    }
}

fn f_char_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    Ok(Value::fixnum(lisp.char_width_of(c) as i64))
}

fn f_string_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    let chars = lisp.string_char_vec(s);
    let multibyte = lisp.string_multibyte(s);
    let (from, to) = lisp.validate_subarray(s, args[1], args[2], chars.len())?;
    let width: usize = chars[from..to]
        .iter()
        .map(|&c| {
            if !multibyte && c >= 0x80 {
                4
            } else {
                lisp.char_width_of(c)
            }
        })
        .sum();
    Ok(Value::fixnum(width as i64))
}

fn f_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut chars = Vec::with_capacity(args.len());
    for &a in args {
        chars.push(lisp.check_character(a)?);
    }
    let multibyte = chars.iter().any(|&c| c >= 0x80);
    if multibyte {
        let mut data = Vec::new();
        for &c in &chars {
            character::push_char(&mut data, c);
        }
        return Ok(lisp.make_specified_string(data, chars.len(), true));
    }
    let bytes: Vec<u8> = chars.iter().map(|&c| c as u8).collect();
    Ok(lisp.unibyte_string(&bytes))
}

fn f_unibyte_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut bytes = Vec::with_capacity(args.len());
    for &a in args {
        match a.as_fixnum() {
            Some(n) if (0..=255).contains(&n) => bytes.push(n as u8),
            _ => {
                return Err(lisp.args_out_of_range3(a, Value::fixnum(0), Value::fixnum(255)));
            }
        }
    }
    Ok(lisp.unibyte_string(&bytes))
}

pub fn char_resolve_modifier_mask(c: i64) -> i64 {
    let mut c = c;
    if c & CHAR_MODIFIER_NON_ALPHA == 0 {
        return c;
    }
    let base = c & !character::CHAR_MODIFIER_MASK;
    if c & CHAR_SHIFT != 0 {
        if (0x41..=0x5A).contains(&base) {
            c &= !CHAR_SHIFT;
        } else if (0x61..=0x7A).contains(&base) {
            c = (c & !CHAR_SHIFT) - 0x20;
        } else if base <= 0x20 {
            c &= !CHAR_SHIFT;
        }
    }
    if c & CHAR_CTL != 0 {
        let base = c & !character::CHAR_MODIFIER_MASK;
        if base == 0x20 {
            c &= !0x7F & !CHAR_CTL;
        } else if base == 0x3F {
            c = 127 | (c & !0x7F & !CHAR_CTL);
        } else if (0x40..=0x5F).contains(&base) || (0x61..=0x7A).contains(&base) {
            c = (base & 0x1F) | (c & !0x7F & !CHAR_CTL);
        }
    }
    c
}

const CHAR_MODIFIER_NON_ALPHA: i64 = CHAR_SHIFT | CHAR_CTL;
const _UNUSED: i64 = CHAR_ALT | CHAR_SUPER | CHAR_HYPER | CHAR_META;

fn f_char_resolve_modifiers(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_fixnum(args[0])?;
    Ok(Value::fixnum(char_resolve_modifier_mask(c)))
}
