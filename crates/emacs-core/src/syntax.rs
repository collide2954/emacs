use crate::character::{self, MAX_CHAR};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const S_WHITESPACE: u8 = 0;
pub const S_PUNCT: u8 = 1;
pub const S_WORD: u8 = 2;
pub const S_SYMBOL: u8 = 3;
pub const S_OPEN: u8 = 4;
pub const S_CLOSE: u8 = 5;
pub const S_QUOTE: u8 = 6;
pub const S_STRING: u8 = 7;
pub const S_MATH: u8 = 8;
pub const S_ESCAPE: u8 = 9;
pub const S_CHARQUOTE: u8 = 10;
pub const S_COMMENT: u8 = 11;
pub const S_ENDCOMMENT: u8 = 12;
pub const S_INHERIT: u8 = 13;
pub const S_COMMENT_FENCE: u8 = 14;
pub const S_STRING_FENCE: u8 = 15;
pub const S_MAX: u8 = 16;

const SYNTAX_CODE_SPEC: [u8; 16] = *b" .w_()'\"$\\/<>@!|";

pub fn syntax_spec_code(c: u8) -> u8 {
    match c {
        b' ' | b'-' => S_WHITESPACE,
        b'.' => S_PUNCT,
        b'w' | b'W' => S_WORD,
        b'_' => S_SYMBOL,
        b'(' => S_OPEN,
        b')' => S_CLOSE,
        b'\'' => S_QUOTE,
        b'"' => S_STRING,
        b'$' => S_MATH,
        b'\\' => S_ESCAPE,
        b'/' => S_CHARQUOTE,
        b'<' => S_COMMENT,
        b'>' => S_ENDCOMMENT,
        b'@' => S_INHERIT,
        b'!' => S_COMMENT_FENCE,
        b'|' => S_STRING_FENCE,
        _ => 0xFF,
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("syntax-table-p", 1, 1, f_syntax_table_p),
    Subr::fixed("syntax-table", 0, 0, f_syntax_table),
    Subr::fixed("standard-syntax-table", 0, 0, f_standard_syntax_table),
    Subr::fixed("copy-syntax-table", 0, 1, f_copy_syntax_table),
    Subr::fixed("set-syntax-table", 1, 1, f_set_syntax_table),
    Subr::fixed("char-syntax", 1, 1, f_char_syntax),
    Subr::fixed("syntax-class-to-char", 1, 1, f_syntax_class_to_char),
    Subr::fixed("matching-paren", 1, 1, f_matching_paren),
    Subr::fixed("string-to-syntax", 1, 1, f_string_to_syntax),
    Subr::fixed("modify-syntax-entry", 2, 3, f_modify_syntax_entry)
        .interactive("cSet syntax for character: \nsSet syntax for %s to: "),
    Subr::fixed(
        "internal-describe-syntax-value",
        1,
        1,
        f_internal_describe_syntax_value,
    ),
];

pub fn init_vars(lisp: &mut Lisp) {
    let mut objects = Vec::with_capacity(S_MAX as usize);
    for i in 0..S_MAX {
        objects.push(lisp.list(&[Value::fixnum(i as i64)]));
    }
    lisp.syntax_code_objects = lisp.vector(objects);
    lisp.put(
        sym::SYNTAX_TABLE,
        sym::CHAR_TABLE_EXTRA_SLOTS,
        Value::fixnum(0),
    );
    let whitespace = lisp.syntax_code_object(S_WHITESPACE);
    let table = lisp
        .make_char_table(sym::SYNTAX_TABLE, whitespace)
        .expect("standard syntax table");
    lisp.set_plain_value(sym::STANDARD_SYNTAX_TABLE_INTERNAL, table);
    let punct = lisp.syntax_code_object(S_PUNCT);
    for c in 0..b' ' as u32 {
        lisp.char_table_set(table, c, punct);
    }
    lisp.char_table_set(table, 0o177, punct);
    for c in [b' ', b'\t', b'\n', 0o15, 0o14] {
        lisp.char_table_set(table, c as u32, whitespace);
    }
    let word = lisp.syntax_code_object(S_WORD);
    for c in (b'a'..=b'z').chain(b'A'..=b'Z').chain(b'0'..=b'9') {
        lisp.char_table_set(table, c as u32, word);
    }
    lisp.char_table_set(table, b'$' as u32, word);
    lisp.char_table_set(table, b'%' as u32, word);
    for (open, close) in [(b'(', b')'), (b'[', b']'), (b'{', b'}')] {
        let o = lisp.cons(Value::fixnum(S_OPEN as i64), Value::fixnum(close as i64));
        lisp.char_table_set(table, open as u32, o);
        let c = lisp.cons(Value::fixnum(S_CLOSE as i64), Value::fixnum(open as i64));
        lisp.char_table_set(table, close as u32, c);
    }
    let string = lisp.cons(Value::fixnum(S_STRING as i64), Value::NIL);
    lisp.char_table_set(table, b'"' as u32, string);
    let escape = lisp.cons(Value::fixnum(S_ESCAPE as i64), Value::NIL);
    lisp.char_table_set(table, b'\\' as u32, escape);
    let symbol = lisp.syntax_code_object(S_SYMBOL);
    for &c in b"_-+*/&|<>=" {
        lisp.char_table_set(table, c as u32, symbol);
    }
    for &c in b".,;:?!#@~^'`" {
        lisp.char_table_set(table, c as u32, punct);
    }
    lisp.char_table_set_range(table, 0x80, MAX_CHAR, word);
    lisp.defvar(sym::COMMENT_USE_SYNTAX_PPSS, sym::T);
    lisp.defvar_bool(sym::PARSE_SEXP_IGNORE_COMMENTS, false);
    lisp.defvar_bool(sym::PARSE_SEXP_LOOKUP_PROPERTIES, false);
    lisp.defvar_int(sym::SYNTAX_PROPERTIZE__DONE, -1);
    lisp.make_variable_buffer_local_builtin(sym::SYNTAX_PROPERTIZE__DONE);
    lisp.defvar_bool(sym::WORDS_INCLUDE_ESCAPES, false);
    lisp.defvar_bool(sym::MULTIBYTE_SYNTAX_AS_SYMBOL, false);
    lisp.defvar_bool(sym::OPEN_PAREN_IN_COLUMN_0_IS_DEFUN_START, true);
    let fwb = lisp
        .make_char_table(Value::NIL, Value::NIL)
        .expect("word boundary table");
    lisp.defvar(sym::FIND_WORD_BOUNDARY_FUNCTION_TABLE, fwb);
    lisp.defvar_bool(sym::COMMENT_END_CAN_BE_ESCAPED, false);
    lisp.make_variable_buffer_local_builtin(sym::COMMENT_END_CAN_BE_ESCAPED);
    lisp.defvar(sym::FORWARD_COMMENT_FUNCTION, Value::NIL);
}

impl Lisp {
    pub fn syntax_code_object(&self, code: u8) -> Value {
        self.vector_items(self.syntax_code_objects)[code as usize]
    }

    pub fn standard_syntax_table(&self) -> Value {
        self.plain_value(sym::STANDARD_SYNTAX_TABLE_INTERNAL)
    }

    pub fn is_syntax_table(&self, v: Value) -> bool {
        self.is_char_table(v) && self.char_table(v).purpose == sym::SYNTAX_TABLE
    }

    fn check_syntax_table(&mut self, v: Value) -> LispResult<Value> {
        if self.is_syntax_table(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::SYNTAX_TABLE_P, v))
        }
    }

    pub fn syntax_entry(&self, c: u32) -> Value {
        let table = self.cur().syntax_table;
        if self.is_char_table(table) {
            self.char_table_ref(table, c)
        } else {
            Value::NIL
        }
    }

    pub fn syntax_with_flags(&self, c: u32) -> i64 {
        let entry = self.syntax_entry(c);
        if entry.is_cons() {
            self.xcar(entry).as_fixnum().unwrap_or(0)
        } else {
            S_WHITESPACE as i64
        }
    }

    pub fn syntax_class(&self, c: u32) -> u8 {
        (self.syntax_with_flags(c) & 0xff) as u8
    }

    pub fn string_to_syntax(&mut self, string: Value) -> LispResult {
        self.check_string(string)?;
        let bytes = self.string_bytes(string).to_vec();
        let first = bytes.first().copied().unwrap_or(0);
        let mut val = syntax_spec_code(first) as i64;
        if val == 0xFF {
            let text = format!(
                "Invalid syntax description letter: {}",
                char::from_u32(first as u32).unwrap_or('?')
            );
            return Err(self.error(&text));
        }
        if val == S_INHERIT as i64 {
            return Ok(Value::NIL);
        }
        let mut i = 1;
        let mut matching = Value::NIL;
        if i < bytes.len() {
            let (ch, len) = if self.string_multibyte(string) {
                character::string_char(&bytes[i..])
            } else {
                (bytes[i] as u32, 1)
            };
            if ch != b' ' as u32 {
                matching = Value::fixnum(ch as i64);
            }
            i += len;
        }
        while i < bytes.len() {
            match bytes[i] {
                b'1' => val |= 1 << 16,
                b'2' => val |= 1 << 17,
                b'3' => val |= 1 << 18,
                b'4' => val |= 1 << 19,
                b'p' => val |= 1 << 20,
                b'b' => val |= 1 << 21,
                b'n' => val |= 1 << 22,
                b'c' => val |= 1 << 23,
                _ => {}
            }
            i += 1;
        }
        if val < S_MAX as i64 && matching.is_nil() {
            return Ok(self.syntax_code_object(val as u8));
        }
        Ok(self.cons(Value::fixnum(val), matching))
    }
}

fn f_syntax_table_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.is_syntax_table(args[0])))
}

fn f_syntax_table(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.cur().syntax_table)
}

fn f_standard_syntax_table(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.standard_syntax_table())
}

fn f_copy_syntax_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = args.first().copied().unwrap_or(Value::NIL);
    let table = if table.is_nil() {
        lisp.standard_syntax_table()
    } else {
        lisp.check_syntax_table(table)?
    };
    let copy = crate::fns::f_copy_sequence(lisp, &[table])?;
    lisp.char_table_mut(copy).default = Value::NIL;
    if lisp.char_table(copy).parent.is_nil() {
        let standard = lisp.standard_syntax_table();
        lisp.char_table_mut(copy).parent = standard;
    }
    Ok(copy)
}

fn f_set_syntax_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = lisp.check_syntax_table(args[0])?;
    lisp.cur_mut().syntax_table = table;
    Ok(table)
}

fn f_char_syntax(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut c = lisp.check_character(args[0])?;
    if !lisp.cur().text.multibyte && character::single_byte_char_p(c) && c >= 0x80 {
        c = character::byte8_to_char(c as u8);
    }
    let code = lisp.syntax_class(c);
    let spec = SYNTAX_CODE_SPEC.get(code as usize).copied().unwrap_or(b' ');
    Ok(Value::fixnum(spec as i64))
}

fn f_syntax_class_to_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let syn = lisp.check_fixnum(args[0])?;
    if !(0..SYNTAX_CODE_SPEC.len() as i64).contains(&syn) {
        let max = Value::fixnum(SYNTAX_CODE_SPEC.len() as i64 - 1);
        return Err(lisp.args_out_of_range(max, args[0]));
    }
    Ok(Value::fixnum(SYNTAX_CODE_SPEC[syn as usize] as i64))
}

fn f_matching_paren(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    let code = lisp.syntax_class(c);
    if code == S_OPEN || code == S_CLOSE {
        let entry = lisp.syntax_entry(c);
        return Ok(lisp.cdr_safe(entry));
    }
    Ok(Value::NIL)
}

fn f_string_to_syntax(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.string_to_syntax(args[0])
}

fn f_modify_syntax_entry(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = args[0];
    let newentry = args[1];
    let table = args.get(2).copied().unwrap_or(Value::NIL);
    if c.is_cons() {
        lisp.check_character(lisp.xcar(c))?;
        lisp.check_character(lisp.xcdr(c))?;
    } else {
        lisp.check_character(c)?;
    }
    let table = if table.is_nil() {
        lisp.cur().syntax_table
    } else {
        lisp.check_syntax_table(table)?
    };
    let entry = lisp.string_to_syntax(newentry)?;
    if c.is_cons() {
        let from = lisp.check_character(lisp.xcar(c))?;
        let to = lisp.check_character(lisp.xcdr(c))?;
        lisp.char_table_set_range(table, from, to, entry);
    } else {
        let ch = lisp.check_character(c)?;
        lisp.char_table_set(table, ch, entry);
    }
    lisp.clear_regexp_cache();
    Ok(Value::NIL)
}

fn f_internal_describe_syntax_value(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let syntax = args[0];
    if syntax.is_nil() {
        lisp.insert_str("default")?;
        return Ok(syntax);
    }
    if lisp.is_char_table(syntax) {
        lisp.insert_str("deeper char-table ...")?;
        return Ok(syntax);
    }
    if !syntax.is_cons() {
        lisp.insert_str("invalid")?;
        return Ok(syntax);
    }
    let first = lisp.xcar(syntax);
    let match_lisp = lisp.xcdr(syntax);
    let match_ok = match_lisp.is_nil()
        || matches!(match_lisp.as_fixnum(), Some(n) if (0..=MAX_CHAR as i64).contains(&n));
    let Some(flags) = first.as_fixnum().filter(|_| match_ok) else {
        lisp.insert_str("invalid")?;
        return Ok(syntax);
    };
    let flags = flags & i32::MAX as i64;
    let code = (flags & 0o377) as u8;
    let bit = |n: u32| (flags >> n) & 1 != 0;
    let (start1, start2, end1, end2) = (bit(16), bit(17), bit(18), bit(19));
    let (prefix, comstyleb, comnested, comstylec) = (bit(20), bit(21), bit(22), bit(23));
    if code >= S_MAX {
        lisp.insert_str("invalid")?;
        return Ok(syntax);
    }
    let mut head = vec![SYNTAX_CODE_SPEC[code as usize] as u32];
    match match_lisp.as_fixnum() {
        Some(m) => head.push(m as u32),
        None => head.push(b' ' as u32),
    }
    for (flag, ch) in [
        (start1, b'1'),
        (start2, b'2'),
        (end1, b'3'),
        (end2, b'4'),
        (prefix, b'p'),
        (comstyleb, b'b'),
        (comstylec, b'c'),
        (comnested, b'n'),
    ] {
        if flag {
            head.push(ch as u32);
        }
    }
    lisp.insert_char_codes(&head, false, false)?;
    lisp.insert_str("\twhich means: ")?;
    let name = match code {
        S_WHITESPACE => "whitespace",
        S_PUNCT => "punctuation",
        S_WORD => "word",
        S_SYMBOL => "symbol",
        S_OPEN => "open",
        S_CLOSE => "close",
        S_QUOTE => "prefix",
        S_STRING => "string",
        S_MATH => "math",
        S_ESCAPE => "escape",
        S_CHARQUOTE => "charquote",
        S_COMMENT => "comment",
        S_ENDCOMMENT => "endcomment",
        S_INHERIT => "inherit",
        S_COMMENT_FENCE => "comment fence",
        _ => "string fence",
    };
    lisp.insert_str(name)?;
    if let Some(m) = match_lisp.as_fixnum() {
        lisp.insert_str(", matches ")?;
        lisp.insert_char_codes(&[m as u32], false, false)?;
    }
    if start1 {
        lisp.insert_str(",\n\t  is the first character of a comment-start sequence")?;
    }
    if start2 {
        lisp.insert_str(",\n\t  is the second character of a comment-start sequence")?;
    }
    if end1 {
        lisp.insert_str(",\n\t  is the first character of a comment-end sequence")?;
    }
    if end2 {
        lisp.insert_str(",\n\t  is the second character of a comment-end sequence")?;
    }
    if comstyleb {
        lisp.insert_str(" (comment style b)")?;
    }
    if comstylec {
        lisp.insert_str(" (comment style c)")?;
    }
    if comnested {
        lisp.insert_str(" (nestable)")?;
    }
    if prefix {
        let doc = lisp.string(",\n\t  is a prefix character for `backward-prefix-chars'");
        let substituted = lisp.funcall(sym::SUBSTITUTE_COMMAND_KEYS, &[doc])?;
        lisp.insert_string(substituted, false, false)?;
    }
    Ok(syntax)
}
