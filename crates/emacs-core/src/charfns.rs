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
    let vector = lisp.vector(vec![Value::NIL; 16]);
    lisp.defvar(sym::TRANSLATION_TABLE_VECTOR, vector);
    let auto_fill = lisp
        .make_char_table(sym::AUTO_FILL_CHARS, Value::NIL)
        .expect("auto-fill-chars");
    lisp.char_table_set(auto_fill, b' ' as u32, sym::T);
    lisp.char_table_set(auto_fill, b'\n' as u32, sym::T);
    lisp.defvar(sym::AUTO_FILL_CHARS, auto_fill);
    let width = lisp
        .make_char_table(Value::NIL, Value::fixnum(1))
        .expect("char-width-table");
    lisp.char_table_set_range(width, 0x80, 0x9F, Value::fixnum(4));
    lisp.char_table_set_range(
        width,
        character::MAX_5_BYTE_CHAR + 1,
        MAX_CHAR,
        Value::fixnum(4),
    );
    lisp.defvar(sym::CHAR_WIDTH_TABLE, width);
    let ambiguous = lisp
        .make_char_table(Value::NIL, Value::NIL)
        .expect("ambiguous-width-chars");
    lisp.defvar(sym::AMBIGUOUS_WIDTH_CHARS, ambiguous);
    let printable = lisp
        .make_char_table(Value::NIL, Value::NIL)
        .expect("printable-chars");
    lisp.char_table_set_range(printable, 32, 126, sym::T);
    lisp.char_table_set_range(printable, 160, character::MAX_5_BYTE_CHAR, sym::T);
    lisp.defvar(sym::PRINTABLE_CHARS, printable);
    lisp.put(
        sym::CHAR_SCRIPT_TABLE,
        sym::CHAR_TABLE_EXTRA_SLOTS,
        Value::fixnum(1),
    );
    let script_table = lisp
        .make_char_table(sym::CHAR_SCRIPT_TABLE, Value::NIL)
        .expect("char-script-table");
    lisp.defvar(sym::CHAR_SCRIPT_TABLE, script_table);
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

fn sanitize_width(width: i64) -> usize {
    if (0..=1000).contains(&width) {
        width as usize
    } else {
        1000
    }
}

impl Lisp {
    pub fn sane_tab_width(&self) -> usize {
        match self.bound_var(sym::TAB_WIDTH).as_fixnum() {
            Some(n) if 0 < n && n <= 1000 => n as usize,
            _ => 8,
        }
    }

    pub fn char_width_of(&self, c: u32) -> usize {
        if c < 0x80 {
            let ctl = if self.bound_var(sym::CTL_ARROW).is_nil() {
                4
            } else {
                2
            };
            return match c {
                9 => self.sane_tab_width(),
                10 => 0,
                0..0x20 | 0x7F => ctl,
                _ => 1,
            };
        }
        let table = self.bound_var(sym::CHAR_WIDTH_TABLE);
        if self.is_char_table(table) {
            let w = self.char_table_ref(table, c);
            if let Some(n) = w.as_fixnum() {
                return sanitize_width(n);
            }
        }
        1
    }
}

fn f_char_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    Ok(Value::fixnum(lisp.char_width_of(c) as i64))
}

pub(crate) fn f_string_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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

pub(crate) fn f_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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
