use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use num_bigint::BigInt;
use num_traits::Num as _;

use crate::character::{self, CHAR_CTL, CHAR_META, CHAR_MODIFIER_MASK, CHAR_SHIFT, NO_BREAK_SPACE};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::{BoolVector, Interned, LispString, Obarray, Vectorlike};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("read", 0, 1, f_read),
    Subr::fixed("read-positioning-symbols", 0, 1, f_read_positioning_symbols),
    Subr::fixed("read-from-string", 1, 3, f_read_from_string),
    Subr::fixed(
        "lread--substitute-object-in-subtree",
        3,
        3,
        f_lread_substitute_object_in_subtree,
    ),
    Subr::fixed("intern", 1, 2, f_intern),
    Subr::fixed("intern-soft", 1, 2, f_intern_soft),
    Subr::fixed("unintern", 2, 2, f_unintern),
    Subr::fixed("obarray-make", 0, 1, f_obarray_make),
    Subr::fixed("obarrayp", 1, 1, f_obarrayp),
    Subr::fixed("obarray-clear", 1, 1, f_obarray_clear),
    Subr::fixed("mapatoms", 1, 2, f_mapatoms),
    Subr::fixed("load", 1, 5, f_load),
    Subr::fixed("get-load-suffixes", 0, 0, f_get_load_suffixes),
    Subr::fixed("locate-file-internal", 2, 4, f_locate_file_internal),
    Subr::fixed("get-file-char", 0, 0, f_get_file_char),
];

pub fn init_vars(lisp: &mut Lisp) {
    let obarray = lisp.obarray;
    lisp.defvar(sym::OBARRAY, obarray);
    lisp.defvar(sym::VALUES, Value::NIL);
    lisp.defvar(sym::STANDARD_INPUT, sym::T);
    lisp.defvar(sym::READ_CIRCLE, sym::T);
    lisp.defvar(sym::LOAD_PATH, Value::NIL);
    let elc = lisp.string(".elc");
    let el = lisp.string(".el");
    let suffixes = lisp.list(&[elc, el]);
    lisp.defvar(sym::LOAD_SUFFIXES, suffixes);
    lisp.defvar(sym::MODULE_FILE_SUFFIX, Value::NIL);
    lisp.defvar(sym::DYNAMIC_LIBRARY_SUFFIXES, Value::NIL);
    let empty = lisp.string("");
    let rep = lisp.list(&[empty]);
    lisp.defvar(sym::LOAD_FILE_REP_SUFFIXES, rep);
    lisp.defvar(sym::LOAD_IN_PROGRESS, Value::NIL);
    lisp.defvar(sym::AFTER_LOAD_ALIST, Value::NIL);
    lisp.defvar(sym::LOAD_HISTORY, Value::NIL);
    lisp.defvar(sym::LOAD_FILE_NAME, Value::NIL);
    lisp.defvar(sym::LOAD_TRUE_FILE_NAME, Value::NIL);
    lisp.defvar(sym::USER_INIT_FILE, Value::NIL);
    lisp.defvar(sym::CURRENT_LOAD_LIST, Value::NIL);
    lisp.defvar(sym::LOAD_READ_FUNCTION, sym::READ);
    lisp.defvar(sym::LOAD_SOURCE_FILE_FUNCTION, Value::NIL);
    lisp.defvar_bool(sym::LOAD_FORCE_DOC_STRINGS, false);
    lisp.defvar(sym::SOURCE_DIRECTORY, Value::NIL);
    lisp.defvar(sym::PRELOADED_FILE_LIST, Value::NIL);
    lisp.defvar(sym::BYTE_BOOLEAN_VARS, Value::NIL);
    lisp.defvar(sym::EVAL_BUFFER_LIST, Value::NIL);
    lisp.defvar(sym::LREAD_UNESCAPED_CHARACTER_LITERALS, Value::NIL);
    lisp.defvar_bool(sym::LOAD_PREFER_NEWER, false);
    lisp.defvar_bool(sym::LOAD_NO_NATIVE, false);
    lisp.defvar_bool(sym::FORCE_LOAD_MESSAGES, false);
    lisp.defvar(sym::READ_SYMBOL_SHORTHANDS, Value::NIL);
    lisp.defvar(sym::LOAD_PATH_FILTER_FUNCTION, Value::NIL);
    lisp.defvar(sym::LOADS_IN_PROGRESS, Value::NIL);
    lisp.defvar(sym::READ_BUFFER_FUNCTION, Value::NIL);
    lisp.defvar(
        sym::INTERNAL__GET_DEFAULT_LEXICAL_BINDING_FUNCTION,
        Value::NIL,
    );
    let regexp = lisp.string("^;;;.\\([0-9]+\\)");
    lisp.defvar(sym::BYTECOMP_VERSION_REGEXP, regexp);
}

#[derive(Clone)]
pub enum SourceKind {
    Bytes {
        data: std::rc::Rc<Vec<u8>>,
        multibyte: bool,
    },
    Function(Value),
    Buffer(Value),
}

pub struct Source {
    pub kind: SourceKind,
    pub pos: usize,
    pub chars_read: usize,
    pub unread: Vec<i64>,
    pub from_file: bool,
}

impl Source {
    pub fn from_bytes(data: Vec<u8>, multibyte: bool) -> Source {
        Source {
            kind: SourceKind::Bytes {
                data: std::rc::Rc::new(data),
                multibyte,
            },
            pos: 0,
            chars_read: 0,
            unread: Vec::new(),
            from_file: false,
        }
    }

    pub fn multibyte(&self) -> bool {
        match &self.kind {
            SourceKind::Bytes { multibyte, .. } => *multibyte,
            _ => true,
        }
    }
}

enum Entry {
    ListStart,
    List { head: Value, tail: Value },
    ListDot { head: Value, tail: Value },
    Vector(Vec<Value>, bool),
    Record(Vec<Value>, bool),
    CharTable(Vec<Value>, bool),
    SubCharTable(Vec<Value>, bool),
    ByteCode(Vec<Value>, bool),
    StringProps(Vec<Value>, bool),
    Special(Value),
    Numbered { number: i64, placeholder: Value },
}

pub struct Reader<'a> {
    pub lisp: &'a mut Lisp,
    pub source: Source,
    stack: Vec<Entry>,
    objects_map: HashMap<i64, Value>,
    objects_completed: HashSet<Value>,
    locate_syms: bool,
}

impl<'a> Reader<'a> {
    pub fn new(lisp: &'a mut Lisp, source: Source) -> Reader<'a> {
        Reader {
            lisp,
            source,
            stack: Vec::new(),
            objects_map: HashMap::new(),
            objects_completed: HashSet::new(),
            locate_syms: false,
        }
    }

    fn root_values(&self) -> Vec<Value> {
        let mut out = Vec::new();
        for entry in &self.stack {
            match entry {
                Entry::ListStart => {}
                Entry::List { head, tail } | Entry::ListDot { head, tail } => {
                    out.push(*head);
                    out.push(*tail);
                }
                Entry::Vector(v, _)
                | Entry::Record(v, _)
                | Entry::CharTable(v, _)
                | Entry::SubCharTable(v, _)
                | Entry::ByteCode(v, _)
                | Entry::StringProps(v, _) => out.extend_from_slice(v),
                Entry::Special(s) => out.push(*s),
                Entry::Numbered { placeholder, .. } => out.push(*placeholder),
            }
        }
        out.extend(self.objects_map.values().copied());
        out.extend(self.objects_completed.iter().copied());
        if let SourceKind::Function(f) | SourceKind::Buffer(f) = self.source.kind {
            out.push(f);
        }
        out
    }

    fn with_roots<T>(&mut self, f: impl FnOnce(&mut Lisp) -> T) -> T {
        let roots = self.root_values();
        let base = self.lisp.stack.len();
        self.lisp.stack.extend(roots);
        let r = f(self.lisp);
        self.lisp.stack.truncate(base);
        r
    }

    pub fn readchar(&mut self) -> LispResult<i64> {
        if let Some(c) = self.source.unread.pop() {
            self.source.chars_read += 1;
            return Ok(c);
        }
        let kind = self.source.kind.clone();
        match kind {
            SourceKind::Bytes { data, multibyte } => {
                if self.source.pos >= data.len() {
                    return Ok(-1);
                }
                self.source.chars_read += 1;
                if multibyte {
                    let (c, len) = character::string_char(&data[self.source.pos..]);
                    self.source.pos += len;
                    Ok(c as i64)
                } else {
                    let b = data[self.source.pos];
                    self.source.pos += 1;
                    Ok(b as i64)
                }
            }
            SourceKind::Function(f) => {
                let v = self.with_roots(|l| l.funcall(f, &[]))?;
                self.source.chars_read += 1;
                Ok(v.as_fixnum().unwrap_or(-1))
            }
            SourceKind::Buffer(b) => {
                let c = self.lisp.buffer_read_char(b);
                if c >= 0 {
                    self.source.chars_read += 1;
                }
                Ok(c)
            }
        }
    }

    pub fn unreadchar(&mut self, c: i64) -> LispResult<()> {
        if c < 0 {
            return Ok(());
        }
        self.source.chars_read = self.source.chars_read.saturating_sub(1);
        match self.source.kind {
            SourceKind::Function(f) => {
                self.with_roots(|l| l.funcall(f, &[Value::fixnum(c)]))?;
            }
            SourceKind::Buffer(b) => self.lisp.buffer_unread_char(b),
            SourceKind::Bytes { .. } => self.source.unread.push(c),
        }
        Ok(())
    }

    fn end_of_file(&mut self) -> crate::error::Flow {
        if self.source.from_file {
            let name = self.lisp.plain_value(sym::LOAD_TRUE_FILE_NAME);
            return self.lisp.signal_list(sym::END_OF_FILE, &[name]);
        }
        if let SourceKind::Buffer(b) = self.source.kind {
            return self.lisp.signal_list(sym::END_OF_FILE, &[b]);
        }
        self.lisp.signal(sym::END_OF_FILE, Value::NIL)
    }

    fn invalid_syntax(&mut self, message: &str) -> crate::error::Flow {
        let s = self.lisp.string(message);
        if let SourceKind::Buffer(b) = self.source.kind {
            let (line, column) = self.lisp.buffer_line_column(b);
            return self.lisp.signal_list(
                sym::INVALID_READ_SYNTAX,
                &[s, Value::fixnum(line), Value::fixnum(column)],
            );
        }
        self.lisp.signal_list(sym::INVALID_READ_SYNTAX, &[s])
    }

    fn read_char_escape(&mut self, next: i64) -> LispResult<i64> {
        let mut modifiers: i64 = 0;
        let mut ncontrol = 0;
        let mut c = next;
        let mut chr: i64;
        loop {
            match c {
                -1 => return Err(self.end_of_file()),
                0x61 => chr = 7,
                0x62 => chr = 8,
                0x64 => chr = 127,
                0x65 => chr = 27,
                0x66 => chr = 12,
                0x6E => chr = 10,
                0x72 => chr = 13,
                0x74 => chr = 9,
                0x76 => chr = 11,
                0x0A => {
                    return Err(self.lisp.error("Invalid escape char syntax: \\<newline>"));
                }
                0x4D | 0x53 | 0x48 | 0x41 | 0x73 => {
                    let m = match c {
                        0x4D => CHAR_META,
                        0x53 => CHAR_SHIFT,
                        0x48 => character::CHAR_HYPER,
                        0x41 => character::CHAR_ALT,
                        _ => character::CHAR_SUPER,
                    };
                    let c1 = self.readchar()?;
                    if c1 != b'-' as i64 {
                        if c == 0x73 {
                            self.unreadchar(c1)?;
                            chr = 32;
                            break;
                        }
                        let msg = format!(
                            "Invalid escape char syntax: \\{} not followed by -",
                            c as u8 as char
                        );
                        return Err(self.lisp.error(&msg));
                    }
                    modifiers |= m;
                    let c1 = self.readchar()?;
                    if c1 == b'\\' as i64 {
                        c = self.readchar()?;
                        continue;
                    }
                    chr = c1;
                }
                0x43 | 0x5E => {
                    if c == 0x43 {
                        let c1 = self.readchar()?;
                        if c1 != b'-' as i64 {
                            return Err(self
                                .lisp
                                .error("Invalid escape char syntax: \\C not followed by -"));
                        }
                    }
                    ncontrol += 1;
                    let c1 = self.readchar()?;
                    if c1 == b'\\' as i64 {
                        c = self.readchar()?;
                        continue;
                    }
                    chr = c1;
                }
                0x30..=0x37 => {
                    let mut i = c - 0x30;
                    let mut count = 0;
                    while count < 2 {
                        let d = self.readchar()?;
                        if !(0x30..=0x37).contains(&d) {
                            self.unreadchar(d)?;
                            break;
                        }
                        i = (i << 3) + (d - 0x30);
                        count += 1;
                    }
                    if (0x80..0x100).contains(&i) {
                        i = character::byte8_to_char(i as u8) as i64;
                    }
                    chr = i;
                }
                0x78 => {
                    let mut i: i64 = 0;
                    let mut count = 0;
                    loop {
                        let d = self.readchar()?;
                        let digit = character::char_hexdigit(d);
                        if digit < 0 {
                            self.unreadchar(d)?;
                            break;
                        }
                        i = (i << 4) + digit;
                        if i > (CHAR_META | (CHAR_META - 1)) {
                            let msg = format!("Hex character out of range: \\x{i:x}...");
                            return Err(self.lisp.error(&msg));
                        }
                        if count < 3 {
                            count += 1;
                        }
                    }
                    if count == 0 {
                        return Err(self
                            .lisp
                            .error("Invalid escape char syntax: \\x not followed by hex digit"));
                    }
                    if count < 3 && i >= 0x80 {
                        i = character::byte8_to_char(i as u8) as i64;
                    }
                    modifiers |= i & CHAR_MODIFIER_MASK;
                    chr = i & !CHAR_MODIFIER_MASK;
                }
                0x55 | 0x75 => {
                    let n = if c == 0x55 { 8 } else { 4 };
                    let mut i: i64 = 0;
                    for _ in 0..n {
                        let d = self.readchar()?;
                        if d < 0 {
                            let msg = format!(
                                "Malformed Unicode escape: \\{}{i:x}",
                                if n == 4 { 'u' } else { 'U' }
                            );
                            return Err(self.lisp.error(&msg));
                        }
                        let digit = character::char_hexdigit(d);
                        if digit < 0 {
                            let msg = format!(
                                "Non-hex character used for Unicode escape: {} ({d})",
                                char::from_u32(d as u32).unwrap_or('?')
                            );
                            return Err(self.lisp.error(&msg));
                        }
                        i = (i << 4) + digit;
                    }
                    if i > 0x10FFFF {
                        let msg = format!("Non-Unicode character: 0x{i:x}");
                        return Err(self.lisp.error(&msg));
                    }
                    chr = i;
                }
                0x4E => {
                    let d = self.readchar()?;
                    if d != b'{' as i64 {
                        return Err(self.invalid_syntax("Expected opening brace after \\N"));
                    }
                    let mut name = String::new();
                    let mut whitespace = false;
                    loop {
                        let d = self.readchar()?;
                        if d < 0 {
                            return Err(self.end_of_file());
                        }
                        if d == b'}' as i64 {
                            break;
                        }
                        if d >= 0x80 {
                            let msg = format!("Invalid character U+{d:04X} in character name");
                            return Err(self.invalid_syntax(&msg));
                        }
                        let ch = d as u8 as char;
                        if ch.is_ascii_whitespace() {
                            if whitespace {
                                continue;
                            }
                            whitespace = true;
                            name.push(' ');
                        } else {
                            whitespace = false;
                            name.push(ch);
                        }
                        if name.len() >= 200 {
                            return Err(self.invalid_syntax("Character name too long"));
                        }
                    }
                    if name.is_empty() {
                        return Err(self.invalid_syntax("Empty character name"));
                    }
                    chr = self.character_name_to_code(&name)?;
                }
                other => chr = other,
            }
            break;
        }
        if chr < 0 {
            return Err(self.end_of_file());
        }
        while ncontrol > 0 {
            if (0x40..=0x5F).contains(&chr) || (0x61..=0x7A).contains(&chr) {
                chr &= 0x1F;
            } else if chr == 0x3F {
                chr = 127;
            } else {
                modifiers |= CHAR_CTL;
            }
            ncontrol -= 1;
        }
        Ok(chr | modifiers)
    }

    fn character_name_to_code(&mut self, name: &str) -> LispResult<i64> {
        if let Some(hex) = name.strip_prefix("U+")
            && let Ok(code) = i64::from_str_radix(hex, 16)
            && hex.chars().all(|c| c.is_ascii_hexdigit())
        {
            if code > character::MAX_UNICODE_CHAR as i64 || (0xD800..=0xDFFF).contains(&code) {
                return Err(self.invalid_syntax(&format!("\\N{{{name}}}")));
            }
            return Ok(code);
        }
        let s = self.lisp.string(name);
        let f = sym::CHAR_FROM_NAME;
        let fbound = !self.lisp.heap.symbol_ref(f).function.is_nil();
        let code = if fbound {
            self.with_roots(|l| {
                l.stack.push(s);
                let r = l.funcall(f, &[s, sym::T]);
                l.stack.pop();
                r
            })?
        } else {
            Value::NIL
        };
        match code.as_fixnum() {
            Some(c) => Ok(c),
            None => Err(self.invalid_syntax(&format!("\\N{{{name}}}"))),
        }
    }

    fn read_integer(&mut self, radix: u32) -> LispResult<Value> {
        let mut buf = String::new();
        let mut valid: i32 = -1;
        let mut c = self.readchar()?;
        if c == b'-' as i64 || c == b'+' as i64 {
            buf.push(c as u8 as char);
            c = self.readchar()?;
        }
        if c == b'0' as i64 {
            buf.push('0');
            valid = 1;
            loop {
                c = self.readchar()?;
                if c != b'0' as i64 {
                    break;
                }
            }
        }
        loop {
            let digit = digit_to_number(c, radix);
            if digit < -1 {
                break;
            }
            if digit == -1 {
                valid = 0;
            }
            if valid < 0 {
                valid = 1;
            }
            buf.push(char::from_u32(c as u32).unwrap_or('?'));
            c = self.readchar()?;
        }
        self.unreadchar(c)?;
        if valid != 1 {
            let msg = format!("integer, radix {radix}");
            return Err(self.invalid_syntax(&msg));
        }
        match string_to_number(self.lisp, buf.as_bytes(), radix) {
            Some((v, _)) => Ok(v),
            None => {
                let msg = format!("integer, radix {radix}");
                Err(self.invalid_syntax(&msg))
            }
        }
    }

    fn read_char_literal(&mut self) -> LispResult<Value> {
        let mut ch = self.readchar()?;
        if ch < 0 {
            return Err(self.end_of_file());
        }
        if ch == b' ' as i64 || ch == b'\t' as i64 {
            return Ok(Value::fixnum(ch));
        }
        if matches!(ch as u8, b'(' | b')' | b'[' | b']' | b'"' | b';') && ch < 128 {
            let list = self
                .lisp
                .plain_value(sym::LREAD_UNESCAPED_CHARACTER_LITERALS);
            let obj = Value::fixnum(ch);
            if self.lisp.memq(obj, list).is_nil() {
                let list = self.lisp.cons(obj, list);
                self.lisp
                    .set_plain_value(sym::LREAD_UNESCAPED_CHARACTER_LITERALS, list);
            }
        }
        if ch == b'\\' as i64 {
            let next = self.readchar()?;
            ch = self.read_char_escape(next)?;
        }
        let modifiers = ch & CHAR_MODIFIER_MASK;
        ch &= !CHAR_MODIFIER_MASK;
        if character::char_byte8_p(ch as u32) {
            ch = character::char_to_byte8(ch as u32) as i64;
        }
        ch |= modifiers;
        let nch = self.readchar()?;
        self.unreadchar(nch)?;
        if nch <= 32
            || matches!(
                nch as u8,
                b'"' | b'\'' | b';' | b'(' | b')' | b'[' | b']' | b'#' | b'?' | b'`' | b',' | b'.'
            ) && nch < 128
        {
            return Ok(Value::fixnum(ch));
        }
        Err(self.invalid_syntax("?"))
    }

    fn read_string_literal(&mut self) -> LispResult<Value> {
        let mut buf: Vec<u8> = Vec::new();
        let mut force_multibyte = false;
        let mut force_singlebyte = false;
        let mut nchars = 0usize;
        let mut ch;
        loop {
            ch = self.readchar()?;
            if ch < 0 || ch == b'"' as i64 {
                break;
            }
            if ch == b'\\' as i64 {
                ch = self.readchar()?;
                match ch {
                    0x73 => ch = 32,
                    0x20 | 0x0A => continue,
                    _ => ch = self.read_char_escape(ch)?,
                }
                let mut modifiers = ch & CHAR_MODIFIER_MASK;
                ch &= !CHAR_MODIFIER_MASK;
                if character::char_byte8_p(ch as u32) {
                    force_singlebyte = true;
                } else if !character::ascii_char_p(ch as u32) {
                    force_multibyte = true;
                } else {
                    if modifiers == CHAR_CTL && ch == 32 {
                        ch = 0;
                        modifiers = 0;
                    }
                    if modifiers & CHAR_SHIFT != 0 {
                        if (0x41..=0x5A).contains(&ch) {
                            modifiers &= !CHAR_SHIFT;
                        } else if (0x61..=0x7A).contains(&ch) {
                            ch -= 0x20;
                            modifiers &= !CHAR_SHIFT;
                        }
                    }
                    if modifiers & CHAR_META != 0 {
                        modifiers &= !CHAR_META;
                        ch = character::byte8_to_char((ch | 0x80) as u8) as i64;
                        force_singlebyte = true;
                    }
                }
                if modifiers != 0 {
                    return Err(self.invalid_syntax("Invalid modifier in string"));
                }
                character::push_char(&mut buf, ch as u32);
            } else {
                character::push_char(&mut buf, ch as u32);
                if character::char_byte8_p(ch as u32) {
                    force_singlebyte = true;
                } else if !character::ascii_char_p(ch as u32) {
                    force_multibyte = true;
                }
            }
            nchars += 1;
        }
        if ch < 0 {
            return Err(self.end_of_file());
        }
        if !force_multibyte && force_singlebyte {
            let bytes = character::str_as_unibyte(&buf);
            return Ok(self.lisp.heap.string(LispString::unibyte(bytes)));
        }
        let nbytes = buf.len();
        let multibyte = force_multibyte || nbytes != nchars;
        Ok(self.lisp.make_specified_string(buf, nchars, multibyte))
    }

    fn read_bool_vector(&mut self) -> LispResult<Value> {
        let mut length: usize = 0;
        loop {
            let c = self.readchar()?;
            if !(0x30..=0x39).contains(&c) {
                if c != b'"' as i64 {
                    return Err(self.invalid_syntax("#&"));
                }
                break;
            }
            length = length * 10 + (c - 0x30) as usize;
        }
        let size_in_chars = length.div_ceil(8);
        let s = self.read_string_literal()?;
        let ls = self.lisp.heap.string_ref(s);
        if ls.multibyte
            || !(size_in_chars == ls.chars || length == (ls.chars.saturating_sub(1)) * 8)
        {
            return Err(self.invalid_syntax("#&..."));
        }
        let data = ls.data.clone();
        let bits = (0..length)
            .map(|i| data.get(i / 8).is_some_and(|b| b & (1 << (i % 8)) != 0))
            .collect();
        Ok(self
            .lisp
            .heap
            .vectorlike(Vectorlike::BoolVector(BoolVector { bits })))
    }

    fn skip_lazy_string(&mut self) -> LispResult<bool> {
        let mut nskip: usize = 0;
        let mut digits = 0;
        loop {
            let c = self.readchar()?;
            if !(0x30..=0x39).contains(&c) {
                if nskip > 0 {
                    nskip -= 1;
                } else {
                    self.unreadchar(c)?;
                }
                break;
            }
            nskip = nskip * 10 + (c - 0x30) as usize;
            digits += 1;
            if digits == 2 && nskip == 0 {
                while self.readchar()? >= 0 {}
                return Ok(false);
            }
        }
        match &self.source.kind {
            SourceKind::Bytes { data, multibyte } => {
                if *multibyte {
                    let start = self.source.pos;
                    let mut pos = start;
                    let mut remaining = nskip;
                    while remaining > 0 && pos < data.len() {
                        let len = character::multibyte_length(&data[pos..]).unwrap_or(1);
                        pos += len;
                        remaining = remaining.saturating_sub(len);
                    }
                    self.source.pos = pos;
                } else {
                    self.source.pos = (self.source.pos + nskip).min(data.len());
                }
            }
            _ => {
                for _ in 0..nskip {
                    if self.readchar()? < 0 {
                        break;
                    }
                }
            }
        }
        Ok(true)
    }

    fn skip_space_and_comments(&mut self) -> LispResult<()> {
        loop {
            let mut c = self.readchar()?;
            if c == b';' as i64 {
                loop {
                    c = self.readchar()?;
                    if c < 0 || c == b'\n' as i64 {
                        break;
                    }
                }
            }
            if c < 0 {
                return Err(self.end_of_file());
            }
            if !(c <= 32 || c == NO_BREAK_SPACE as i64) {
                self.unreadchar(c)?;
                return Ok(());
            }
        }
    }

    pub fn read0(&mut self, locate_syms: bool) -> LispResult<Value> {
        self.locate_syms = locate_syms;
        let base_sp = self.stack.len();
        let r = self.read0_inner(base_sp);
        self.stack.truncate(base_sp);
        r
    }

    fn read0_inner(&mut self, base_sp: usize) -> LispResult<Value> {
        'read_obj: loop {
            let c = self.readchar()?;
            if c < 0 {
                return Err(self.end_of_file());
            }
            let mut obj: Value;
            match c {
                0x28 => {
                    self.stack.push(Entry::ListStart);
                    continue 'read_obj;
                }
                0x29 => {
                    if self.stack.len() <= base_sp {
                        return Err(self.invalid_syntax(")"));
                    }
                    match self.stack.pop().expect("entry") {
                        Entry::ListStart => obj = Value::NIL,
                        Entry::List { head, .. } => obj = head,
                        Entry::Record(elems, old) => {
                            self.locate_syms = old;
                            if elems.is_empty() {
                                return Err(self.invalid_syntax("#s"));
                            }
                            obj = if elems[0] == sym::HASH_TABLE {
                                self.hash_table_from_plist(&elems[1..])?
                            } else {
                                self.lisp.heap.vectorlike(Vectorlike::Record(elems))
                            };
                        }
                        Entry::StringProps(elems, old) => {
                            self.locate_syms = old;
                            obj = self.string_props_from_list(elems)?;
                        }
                        other => {
                            self.stack.push(other);
                            return Err(self.invalid_syntax(")"));
                        }
                    }
                }
                0x5B => {
                    self.stack.push(Entry::Vector(Vec::new(), self.locate_syms));
                    continue 'read_obj;
                }
                0x5D => {
                    if self.stack.len() <= base_sp {
                        return Err(self.invalid_syntax("]"));
                    }
                    match self.stack.pop().expect("entry") {
                        Entry::Vector(elems, old) => {
                            self.locate_syms = old;
                            obj = self.lisp.vector(elems);
                        }
                        Entry::ByteCode(elems, old) => {
                            self.locate_syms = old;
                            obj = self.bytecode_from_list(elems)?;
                        }
                        Entry::CharTable(elems, old) => {
                            self.locate_syms = old;
                            obj = self.char_table_from_list(elems)?;
                        }
                        Entry::SubCharTable(elems, old) => {
                            self.locate_syms = old;
                            obj = self.sub_char_table_from_list(elems)?;
                        }
                        other => {
                            self.stack.push(other);
                            return Err(self.invalid_syntax("]"));
                        }
                    }
                }
                0x23 => {
                    let mut rb = String::from("#");
                    let ch = self.read_and_buffer(&mut rb)?;
                    match ch {
                        0x27 => {
                            self.stack.push(Entry::Special(sym::FUNCTION));
                            continue 'read_obj;
                        }
                        0x23 => {
                            let empty = self.lisp.empty_unibyte;
                            obj = self.intern_bytes(Vec::new(), 0, false, empty)?;
                        }
                        0x73 => {
                            let ch = self.read_and_buffer(&mut rb)?;
                            if ch != b'(' as i64 {
                                self.unreadchar(ch)?;
                                return Err(self.invalid_syntax(&rb));
                            }
                            self.stack.push(Entry::Record(Vec::new(), self.locate_syms));
                            self.locate_syms = false;
                            continue 'read_obj;
                        }
                        0x5E => {
                            let ch = self.read_and_buffer(&mut rb)?;
                            if ch == b'^' as i64 {
                                let ch = self.read_and_buffer(&mut rb)?;
                                if ch == b'[' as i64 {
                                    self.stack
                                        .push(Entry::SubCharTable(Vec::new(), self.locate_syms));
                                    self.locate_syms = false;
                                    continue 'read_obj;
                                }
                                self.unreadchar(ch)?;
                                return Err(self.invalid_syntax(&rb));
                            } else if ch == b'[' as i64 {
                                self.stack
                                    .push(Entry::CharTable(Vec::new(), self.locate_syms));
                                self.locate_syms = false;
                                continue 'read_obj;
                            }
                            self.unreadchar(ch)?;
                            return Err(self.invalid_syntax(&rb));
                        }
                        0x28 => {
                            self.stack
                                .push(Entry::StringProps(Vec::new(), self.locate_syms));
                            self.locate_syms = false;
                            continue 'read_obj;
                        }
                        0x5B => {
                            self.stack
                                .push(Entry::ByteCode(Vec::new(), self.locate_syms));
                            self.locate_syms = false;
                            continue 'read_obj;
                        }
                        0x26 => obj = self.read_bool_vector()?,
                        0x21 => {
                            loop {
                                let c = self.readchar()?;
                                if c < 0 || c == b'\n' as i64 {
                                    break;
                                }
                            }
                            continue 'read_obj;
                        }
                        0x78 | 0x58 => obj = self.read_integer(16)?,
                        0x6F | 0x4F => obj = self.read_integer(8)?,
                        0x62 | 0x42 => obj = self.read_integer(2)?,
                        0x40 => {
                            if self.skip_lazy_string()? {
                                continue 'read_obj;
                            }
                            obj = Value::NIL;
                        }
                        0x24 => obj = self.lisp.plain_value(sym::LOAD_FILE_NAME),
                        0x3A | 0x5F => {
                            let c = self.readchar()?;
                            if c <= 32
                                || c == NO_BREAK_SPACE as i64
                                || (c < 128
                                    && matches!(
                                        c as u8,
                                        b'"' | b'\''
                                            | b';'
                                            | b'#'
                                            | b'('
                                            | b')'
                                            | b'['
                                            | b']'
                                            | b'`'
                                            | b','
                                    ))
                            {
                                self.unreadchar(c)?;
                                let empty = self.lisp.empty_unibyte;
                                obj = if ch == 0x3A {
                                    self.lisp.make_symbol(empty)
                                } else {
                                    self.intern_bytes(Vec::new(), 0, false, empty)?
                                };
                            } else {
                                obj = self.read_symbol(c, ch == 0x3A, ch == 0x5F)?;
                            }
                        }
                        d if (0x30..=0x39).contains(&d) => {
                            let mut n: i64 = d - 0x30;
                            let mut c;
                            loop {
                                c = self.read_and_buffer(&mut rb)?;
                                if !(0x30..=0x39).contains(&c) {
                                    break;
                                }
                                n = n.saturating_mul(10).saturating_add(c - 0x30);
                            }
                            if c == b'r' as i64 || c == b'R' as i64 {
                                if !(2..=36).contains(&n) {
                                    let msg = format!("integer, radix {n}");
                                    return Err(self.invalid_syntax(&msg));
                                }
                                obj = self.read_integer(n as u32)?;
                            } else if !self.lisp.plain_value(sym::READ_CIRCLE).is_nil() {
                                if c == b'=' as i64 {
                                    let placeholder = self.lisp.cons(Value::NIL, Value::NIL);
                                    self.objects_map.insert(n, placeholder);
                                    self.stack.push(Entry::Numbered {
                                        number: n,
                                        placeholder,
                                    });
                                    continue 'read_obj;
                                } else if c == b'#' as i64 {
                                    match self.objects_map.get(&n) {
                                        Some(v) => obj = *v,
                                        None => return Err(self.invalid_syntax(&rb)),
                                    }
                                } else {
                                    return Err(self.invalid_syntax(&rb));
                                }
                            } else {
                                return Err(self.invalid_syntax(&rb));
                            }
                        }
                        _ => return Err(self.invalid_syntax(&rb)),
                    }
                }
                0x3F => obj = self.read_char_literal()?,
                0x22 => obj = self.read_string_literal()?,
                0x27 => {
                    self.stack.push(Entry::Special(sym::QUOTE));
                    continue 'read_obj;
                }
                0x60 => {
                    self.stack.push(Entry::Special(sym::BACKQUOTE));
                    continue 'read_obj;
                }
                0x2C => {
                    let ch = self.readchar()?;
                    let s = if ch == b'@' as i64 {
                        sym::COMMA_AT
                    } else {
                        self.unreadchar(ch)?;
                        sym::COMMA
                    };
                    self.stack.push(Entry::Special(s));
                    continue 'read_obj;
                }
                0x3B => {
                    loop {
                        let c = self.readchar()?;
                        if c < 0 || c == b'\n' as i64 {
                            break;
                        }
                    }
                    continue 'read_obj;
                }
                _ => {
                    if c == b'.' as i64 {
                        let nch = self.readchar()?;
                        self.unreadchar(nch)?;
                        if nch <= 32
                            || nch == NO_BREAK_SPACE as i64
                            || (nch < 128
                                && matches!(
                                    nch as u8,
                                    b'"' | b'\'' | b';' | b'(' | b'[' | b'#' | b'?' | b'`' | b','
                                ))
                        {
                            if self.stack.len() > base_sp
                                && let Some(Entry::List { head, tail }) = self.stack.last()
                            {
                                let (head, tail) = (*head, *tail);
                                self.stack.pop();
                                self.stack.push(Entry::ListDot { head, tail });
                                continue 'read_obj;
                            }
                            return Err(self.invalid_syntax("."));
                        }
                    }
                    if c <= 32 || c == NO_BREAK_SPACE as i64 {
                        continue 'read_obj;
                    }
                    obj = self.read_symbol(c, false, false)?;
                }
            }
            loop {
                if self.stack.len() <= base_sp {
                    return Ok(obj);
                }
                match self.stack.last_mut().expect("entry") {
                    Entry::ListStart => {
                        let cell = self.lisp.cons(obj, Value::NIL);
                        *self.stack.last_mut().expect("entry") = Entry::List {
                            head: cell,
                            tail: cell,
                        };
                        continue 'read_obj;
                    }
                    Entry::List { tail, .. } => {
                        let old_tail = *tail;
                        let cell = self.lisp.cons(obj, Value::NIL);
                        self.lisp.heap.set_cdr(old_tail, cell);
                        if let Some(Entry::List { tail, .. }) = self.stack.last_mut() {
                            *tail = cell;
                        }
                        continue 'read_obj;
                    }
                    Entry::ListDot { .. } => {
                        self.skip_space_and_comments()?;
                        let ch = self.readchar()?;
                        if ch != b')' as i64 {
                            return Err(self.invalid_syntax("expected )"));
                        }
                        let Some(Entry::ListDot { head, tail }) = self.stack.pop() else {
                            unreachable!()
                        };
                        self.lisp.heap.set_cdr(tail, obj);
                        obj = head;
                    }
                    Entry::Vector(v, _)
                    | Entry::Record(v, _)
                    | Entry::CharTable(v, _)
                    | Entry::SubCharTable(v, _)
                    | Entry::ByteCode(v, _)
                    | Entry::StringProps(v, _) => {
                        v.push(obj);
                        continue 'read_obj;
                    }
                    Entry::Special(s) => {
                        let s = *s;
                        self.stack.pop();
                        obj = self.lisp.list(&[s, obj]);
                    }
                    Entry::Numbered { .. } => {
                        let Some(Entry::Numbered {
                            number,
                            placeholder,
                        }) = self.stack.pop()
                        else {
                            unreachable!()
                        };
                        if obj.is_cons() {
                            if obj == placeholder {
                                return Err(self.invalid_syntax("nonsensical self-reference"));
                            }
                            let (a, d) = (self.lisp.xcar(obj), self.lisp.xcdr(obj));
                            self.lisp.heap.set_car(placeholder, a);
                            self.lisp.heap.set_cdr(placeholder, d);
                            self.objects_completed.insert(placeholder);
                            obj = placeholder;
                        } else {
                            if !obj.is_symbol()
                                && !self.lisp.is_number(obj)
                                && !(obj.is_string()
                                    && self.lisp.heap.string_ref(obj).props.is_empty())
                            {
                                self.objects_completed.insert(obj);
                            }
                            let completed: HashSet<Value> = self.objects_completed.clone();
                            substitute_object(self.lisp, obj, placeholder, &completed)?;
                            self.objects_map.insert(number, obj);
                        }
                    }
                }
            }
        }
    }

    fn read_and_buffer(&mut self, rb: &mut String) -> LispResult<i64> {
        let c = self.readchar()?;
        if c < 0 {
            let msg = rb.clone();
            return Err(self.invalid_syntax(&msg));
        }
        rb.push(char::from_u32(c as u32).unwrap_or('?'));
        Ok(c)
    }

    fn read_symbol(
        &mut self,
        first: i64,
        uninterned: bool,
        skip_shorthand: bool,
    ) -> LispResult<Value> {
        let multibyte = self.source.multibyte();
        let start_position = self.source.chars_read.saturating_sub(1);
        let mut buf: Vec<u8> = Vec::new();
        let mut quoted = false;
        let mut nchars = 0usize;
        let mut c = first;
        loop {
            if c == b'\\' as i64 {
                c = self.readchar()?;
                if c < 0 {
                    return Err(self.end_of_file());
                }
                quoted = true;
            }
            if multibyte {
                character::push_char(&mut buf, c as u32);
            } else {
                buf.push(c as u8);
            }
            nchars += 1;
            c = self.readchar()?;
            let stop = c <= 32
                || c == NO_BREAK_SPACE as i64
                || (c < 128
                    && matches!(
                        c as u8,
                        b'"' | b'\'' | b';' | b'#' | b'(' | b')' | b'[' | b']' | b'`' | b','
                    ));
            if stop {
                break;
            }
        }
        self.unreadchar(c)?;
        let c0 = buf[0];
        if (c0.is_ascii_digit() || c0 == b'.' || c0 == b'-' || c0 == b'+')
            && !quoted
            && !uninterned
            && !skip_shorthand
            && let Some((value, len)) = string_to_number(self.lisp, &buf, 10)
            && len == buf.len()
        {
            return Ok(value);
        }
        let result = if uninterned {
            let name = self.lisp.make_specified_string(buf, nchars, multibyte);
            self.lisp.make_symbol(name)
        } else {
            let name_placeholder = Value::NIL;
            let mut bytes = buf;
            let mut chars = nchars;
            if !skip_shorthand && symbol_char_span(&bytes) < bytes.len() {
                let shorthands = self.lisp.plain_value(sym::READ_SYMBOL_SHORTHANDS);
                if let Some((longhand, lchars)) = self.apply_shorthand(&bytes, shorthands) {
                    bytes = longhand;
                    chars = lchars;
                }
            }
            self.intern_bytes(bytes, chars, multibyte, name_placeholder)?
        };
        if self.locate_syms && !result.is_nil() {
            return Ok(self.lisp.heap.vectorlike(Vectorlike::SymbolWithPos {
                symbol: result,
                position: Value::fixnum(start_position as i64),
            }));
        }
        Ok(result)
    }

    fn apply_shorthand(&mut self, bytes: &[u8], shorthands: Value) -> Option<(Vec<u8>, usize)> {
        let mut tail = shorthands;
        while tail.is_cons() {
            let pair = self.lisp.xcar(tail);
            tail = self.lisp.xcdr(tail);
            if !pair.is_cons() {
                continue;
            }
            let (short, long) = (self.lisp.xcar(pair), self.lisp.xcdr(pair));
            if !short.is_string() || !long.is_string() {
                continue;
            }
            let sb = self.lisp.string_bytes(short);
            if bytes.starts_with(sb) {
                let mut out = self.lisp.string_bytes(long).to_vec();
                out.extend_from_slice(&bytes[sb.len()..]);
                let chars = character::chars_in_multibyte(&out);
                return Some((out, chars));
            }
        }
        None
    }

    fn intern_bytes(
        &mut self,
        bytes: Vec<u8>,
        chars: usize,
        multibyte: bool,
        name: Value,
    ) -> LispResult<Value> {
        let obarray = self.lisp.plain_value(sym::OBARRAY);
        let obarray = self.lisp.check_obarray(obarray)?;
        if let Some(found) = self.lisp.obarray_lookup(obarray, &bytes) {
            return Ok(found);
        }
        let name = if name.is_string() && self.lisp.string_bytes(name) == bytes.as_slice() {
            name
        } else {
            self.lisp.make_specified_string(bytes, chars, multibyte)
        };
        Ok(self.lisp.intern_new(obarray, name))
    }

    fn hash_table_from_plist(&mut self, plist: &[Value]) -> LispResult<Value> {
        let mut test = Value::NIL;
        let mut weakness = Value::NIL;
        let mut data = Value::NIL;
        let mut i = 0;
        while i + 1 < plist.len() {
            let (k, v) = (plist[i], plist[i + 1]);
            if k == sym::TEST {
                test = v;
            } else if k == sym::WEAKNESS {
                weakness = v;
            } else if k == sym::DATA {
                data = v;
            }
            i += 2;
        }
        if !(data.is_nil() || data.is_cons()) {
            return Err(self.lisp.error("Hash table data is not a list"));
        }
        let items = self.lisp.list_to_vec(data)?;
        if !items.len().is_multiple_of(2) {
            return Err(self.lisp.error("Hash table data length is odd"));
        }
        let mut args = Vec::new();
        if !test.is_nil() {
            args.push(sym::K_TEST);
            args.push(test);
        }
        if !weakness.is_nil() {
            args.push(sym::K_WEAKNESS);
            args.push(weakness);
        }
        let make = self.lisp.intern("make-hash-table");
        let table = self.with_roots(|l| {
            l.stack.extend_from_slice(&items);
            let r = l.funcall(make, &args);
            l.stack.truncate(l.stack.len() - items.len());
            r
        })?;
        for pair in items.chunks(2) {
            let (k, v) = (pair[0], pair[1]);
            self.lisp.stack.push(table);
            let r = self.lisp.puthash(k, v, table);
            self.lisp.stack.pop();
            r?;
        }
        Ok(table)
    }

    fn string_props_from_list(&mut self, elems: Vec<Value>) -> LispResult<Value> {
        if elems.is_empty() || !elems[0].is_string() {
            return Err(self.invalid_syntax("#"));
        }
        let obj = elems[0];
        let rest = &elems[1..];
        if !rest.len().is_multiple_of(3) {
            return Err(self.invalid_syntax("Invalid string property list"));
        }
        for triple in rest.chunks(3) {
            self.lisp
                .set_text_properties(triple[0], triple[1], triple[2], obj)?;
        }
        Ok(obj)
    }

    fn bytecode_from_list(&mut self, elems: Vec<Value>) -> LispResult<Value> {
        let size = elems.len();
        let arglist = elems.first().copied().unwrap_or(Value::NIL);
        let code = elems.get(1).copied().unwrap_or(Value::NIL);
        let constants = elems.get(2).copied().unwrap_or(Value::NIL);
        let depth = elems.get(3).copied().unwrap_or(Value::NIL);
        let valid = (4..=6).contains(&size) || size == 3 && code.is_cons();
        let arglist_ok = arglist.is_fixnum() || arglist.is_cons() || arglist.is_nil();
        let bytecode = code.is_string()
            && self.lisp.is_vector(constants)
            && size > 3
            && depth.as_fixnum().is_some_and(|d| d >= 0);
        let interpreted = code.is_cons() && (constants.is_cons() || constants.is_nil());
        if !((valid || size >= 3) && size <= 6 && arglist_ok && (bytecode || interpreted)) {
            return Err(self.invalid_syntax("Invalid byte-code object"));
        }
        let mut elems = elems;
        if code.is_string() && self.lisp.string_multibyte(code) {
            let bytes = character::str_as_unibyte(self.lisp.string_bytes(code));
            elems[1] = self.lisp.unibyte_string(&bytes);
        }
        Ok(self.lisp.heap.vectorlike(Vectorlike::Closure(elems)))
    }

    fn char_table_from_list(&mut self, elems: Vec<Value>) -> LispResult<Value> {
        match self.lisp.char_table_from_slots(&elems) {
            Some(v) => Ok(v),
            None => Err(self.invalid_syntax("Invalid size char-table")),
        }
    }

    fn sub_char_table_from_list(&mut self, elems: Vec<Value>) -> LispResult<Value> {
        if elems.len() < 2 {
            return Err(self.lisp.error("Invalid size of sub-char-table"));
        }
        let depth = match elems[0].as_fixnum() {
            Some(d) if (1..=3).contains(&d) => d as u8,
            _ => return Err(self.lisp.error("Invalid depth in sub-char-table")),
        };
        if crate::chartab::CHARTAB_SIZE[depth as usize] != elems.len() - 2 {
            return Err(self.lisp.error("Invalid size in sub-char-table"));
        }
        let min_char = match elems[1].as_fixnum() {
            Some(m) if (0..=character::MAX_CHAR as i64).contains(&m) => m as u32,
            _ => {
                return Err(self
                    .lisp
                    .error("Invalid minimum character in sub-char-table"));
            }
        };
        Ok(self
            .lisp
            .make_sub_char_table(depth, min_char, elems[2..].to_vec()))
    }
}

fn symbol_char_span(s: &[u8]) -> usize {
    s.iter()
        .take_while(|&&c| {
            matches!(
                c,
                b'^' | b'*' | b'+' | b'-' | b'/' | b'<' | b'=' | b'>' | b'_' | b'|'
            )
        })
        .count()
}

fn digit_to_number(c: i64, base: u32) -> i64 {
    let digit = match c {
        0x30..=0x39 => c - 0x30,
        0x61..=0x7A => c - 0x61 + 10,
        0x41..=0x5A => c - 0x41 + 10,
        _ => return -2,
    };
    if digit < base as i64 { digit } else { -1 }
}

pub fn string_to_number(lisp: &mut Lisp, s: &[u8], base: u32) -> Option<(Value, usize)> {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut cp = 0usize;
    let negative = at(0) == b'-';
    let positive = at(0) == b'+';
    let signedp = negative || positive;
    if signedp {
        cp += 1;
    }
    let mut lead_int = false;
    let mut trail_int = false;
    let mut e_exp = false;
    let mut special: Option<f64> = None;
    let digit_start = cp;
    while digit_to_number(at(cp) as i64, base) >= 0 {
        lead_int = true;
        cp += 1;
    }
    let after_digits = cp;
    if at(cp) == b'.' {
        cp += 1;
    }
    if base == 10 {
        if at(cp).is_ascii_digit() {
            trail_int = true;
            while at(cp).is_ascii_digit() {
                cp += 1;
            }
        }
        if at(cp) == b'e' || at(cp) == b'E' {
            let ecp = cp;
            cp += 1;
            if at(cp) == b'+' || at(cp) == b'-' {
                cp += 1;
            }
            if at(cp).is_ascii_digit() {
                e_exp = true;
                while at(cp).is_ascii_digit() {
                    cp += 1;
                }
            } else if s.get(cp.wrapping_sub(1)) == Some(&b'+') && s.get(cp..cp + 3) == Some(b"INF")
            {
                e_exp = true;
                cp += 3;
                special = Some(f64::INFINITY);
            } else if s.get(cp.wrapping_sub(1)) == Some(&b'+') && s.get(cp..cp + 3) == Some(b"NaN")
            {
                e_exp = true;
                cp += 3;
                let mantissa: u64 = std::str::from_utf8(&s[digit_start..after_digits])
                    .ok()
                    .and_then(|d| d.parse().ok())
                    .unwrap_or(0);
                let bits = 0x7FF8_0000_0000_0000u64 | (mantissa & 0x0007_FFFF_FFFF_FFFF);
                special = Some(f64::from_bits(bits));
            } else {
                cp = ecp;
            }
        }
    }
    let float_syntax = trail_int || (lead_int && e_exp);
    if float_syntax {
        let value = match special {
            Some(v) => v,
            None => {
                let text = std::str::from_utf8(&s[usize::from(signedp)..cp]).ok()?;
                let text = if text.starts_with('.') {
                    format!("0{text}")
                } else {
                    text.to_string()
                };
                text.parse::<f64>().ok()?
            }
        };
        let v = lisp.float(if negative { -value } else { value });
        return Some((v, cp));
    }
    if !lead_int {
        return None;
    }
    let digits = std::str::from_utf8(&s[digit_start..after_digits]).ok()?;
    let mut n = BigInt::from_str_radix(digits, base).ok()?;
    if negative {
        n = -n;
    }
    Some((lisp.bignum(n), cp))
}

fn substitute_object(
    lisp: &mut Lisp,
    object: Value,
    placeholder: Value,
    completed: &HashSet<Value>,
) -> LispResult<()> {
    let mut seen: HashSet<Value> = HashSet::new();
    let r = substitute_recurse(lisp, object, placeholder, object, completed, &mut seen)?;
    if r != object {
        return Err(lisp.error("Unexpected mutation error in reader"));
    }
    Ok(())
}

fn substitute_recurse(
    lisp: &mut Lisp,
    subtree: Value,
    placeholder: Value,
    object: Value,
    completed: &HashSet<Value>,
    seen: &mut HashSet<Value>,
) -> LispResult<Value> {
    if subtree == placeholder {
        return Ok(object);
    }
    if subtree.is_symbol() || lisp.is_number(subtree) {
        return Ok(subtree);
    }
    if subtree.is_string() && lisp.heap.string_ref(subtree).props.is_empty() {
        return Ok(subtree);
    }
    if seen.contains(&subtree) {
        return Ok(subtree);
    }
    if completed.contains(&subtree) {
        seen.insert(subtree);
    }
    if subtree.is_cons() {
        let car = lisp.xcar(subtree);
        let new_car = substitute_recurse(lisp, car, placeholder, object, completed, seen)?;
        lisp.heap.set_car(subtree, new_car);
        let cdr = lisp.xcdr(subtree);
        let new_cdr = substitute_recurse(lisp, cdr, placeholder, object, completed, seen)?;
        lisp.heap.set_cdr(subtree, new_cdr);
        return Ok(subtree);
    }
    if subtree.is_string() {
        let runs = lisp.heap.string_ref(subtree).props.clone();
        for (i, run) in runs.iter().enumerate() {
            let p = substitute_recurse(lisp, run.plist, placeholder, object, completed, seen)?;
            lisp.heap.string_mut(subtree).props[i].plist = p;
        }
        return Ok(subtree);
    }
    if subtree.is_vectorlike() {
        let items = match lisp.heap.vectorlike_ref(subtree) {
            Vectorlike::Vector(v) | Vectorlike::Record(v) | Vectorlike::Closure(v) => v.clone(),
            Vectorlike::BoolVector(_) => return Ok(subtree),
            Vectorlike::HashTable(h) => {
                let entries = h.live_entries();
                for (k, v) in entries {
                    substitute_recurse(lisp, k, placeholder, object, completed, seen)?;
                    substitute_recurse(lisp, v, placeholder, object, completed, seen)?;
                }
                return Ok(subtree);
            }
            _ => return Ok(subtree),
        };
        for (i, item) in items.into_iter().enumerate() {
            let new = substitute_recurse(lisp, item, placeholder, object, completed, seen)?;
            if let Vectorlike::Vector(v) | Vectorlike::Record(v) | Vectorlike::Closure(v) =
                lisp.heap.vectorlike_mut(subtree)
            {
                v[i] = new;
            }
        }
        return Ok(subtree);
    }
    Ok(subtree)
}

impl Lisp {
    pub fn check_obarray(&mut self, obarray: Value) -> LispResult<Value> {
        if obarray.is_vectorlike() && self.is_obarray(obarray) {
            return Ok(obarray);
        }
        if self.is_vector(obarray) {
            let items = self.vector_items(obarray).to_vec();
            if let Some(&first) = items.first()
                && first.is_vectorlike()
                && self.is_obarray(first)
            {
                return Ok(first);
            }
            if items.is_empty() {
                return Err(self.wrong_type(sym::OBARRAYP, obarray));
            }
            let fresh = self
                .heap
                .vectorlike(Vectorlike::Obarray(Obarray::default()));
            if let Vectorlike::Vector(v) = self.heap.vectorlike_mut(obarray) {
                v[0] = fresh;
            }
            return Ok(fresh);
        }
        Err(self.wrong_type(sym::OBARRAYP, obarray))
    }

    pub fn read_from_source(
        &mut self,
        source: Source,
        locate_syms: bool,
    ) -> LispResult<(Value, Source)> {
        let mut reader = Reader::new(self, source);
        let value = reader.read0(locate_syms)?;
        Ok((value, reader.source))
    }

    pub fn read_from_str(&mut self, text: &str) -> LispResult {
        let s = self.string(text);
        let (data, multibyte) = {
            let ls = self.heap.string_ref(s);
            (ls.data.clone(), ls.multibyte)
        };
        let (value, _) = self.read_from_source(Source::from_bytes(data, multibyte), false)?;
        Ok(value)
    }

    pub fn eval_string(&mut self, text: &str) -> LispResult {
        let s = self.string(text);
        self.stack.push(s);
        let (data, multibyte) = {
            let ls = self.heap.string_ref(s);
            (ls.data.clone(), ls.multibyte)
        };
        let result = self.catch_all(|lisp| {
            let mut source = Source::from_bytes(data, multibyte);
            let mut last = Value::NIL;
            loop {
                let mut reader = Reader::new(lisp, source);
                let c = loop {
                    let c = reader.readchar()?;
                    if c == b';' as i64 {
                        loop {
                            let d = reader.readchar()?;
                            if d < 0 || d == b'\n' as i64 {
                                break;
                            }
                        }
                        continue;
                    }
                    if c >= 0 && (c <= 32 || c == NO_BREAK_SPACE as i64) {
                        continue;
                    }
                    break c;
                };
                if c < 0 {
                    break;
                }
                reader.unreadchar(c)?;
                let form = reader.read0(false)?;
                source = reader.source;
                lisp.stack.push(form);
                last = lisp.eval(form)?;
                lisp.stack.pop();
                lisp.stack.push(last);
            }
            Ok(last)
        });
        self.stack.pop();
        result
    }

    pub fn loadhist_attach(&mut self, x: Value) {
        if self.load_in_progress() || !self.plain_value(sym::CURRENT_LOAD_LIST).is_nil() {
            let list = self.plain_value(sym::CURRENT_LOAD_LIST);
            if list == sym::T {
                return;
            }
            let list = self.cons(x, list);
            self.set_plain_value(sym::CURRENT_LOAD_LIST, list);
        }
    }

    pub fn load_in_progress(&self) -> bool {
        !self.plain_value(sym::LOAD_IN_PROGRESS).is_nil()
    }

    pub fn load(
        &mut self,
        file: Value,
        noerror: Value,
        nomessage: Value,
        nosuffix: Value,
        must_suffix: Value,
    ) -> LispResult {
        f_load(self, &[file, noerror, nomessage, nosuffix, must_suffix])
    }

    pub fn load_for_require(
        &mut self,
        file: Value,
        noerror: Value,
        must_suffix: Value,
    ) -> LispResult {
        let count = self.specpdl.len();
        let old = self.plain_value(sym::AUTOLOAD_QUEUE);
        self.record_unwind(restore_autoload_queue, old);
        self.set_plain_value(sym::AUTOLOAD_QUEUE, sym::T);
        let r = self.load(file, noerror, sym::T, Value::NIL, must_suffix)?;
        self.set_plain_value(sym::AUTOLOAD_QUEUE, sym::T);
        self.unbind_to_value(count, Ok(r))
    }

    fn get_load_suffixes(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        let suffixes = self.plain_value(sym::LOAD_SUFFIXES);
        let reps = self.plain_value(sym::LOAD_FILE_REP_SUFFIXES);
        let mut s = suffixes;
        while s.is_cons() {
            let suffix = self.xcar(s);
            let suffix = self.string_to_rust(suffix);
            let mut r = reps;
            while r.is_cons() {
                let rep = self.xcar(r);
                let rep = self.string_to_rust(rep);
                out.push(format!("{suffix}{rep}"));
                r = self.xcdr(r);
            }
            s = self.xcdr(s);
        }
        out
    }

    pub fn locate_file(&mut self, path: Value, file: &str, suffixes: &[String]) -> Option<PathBuf> {
        let candidate = Path::new(file);
        let mut dirs: Vec<PathBuf> = Vec::new();
        if candidate.is_absolute() {
            dirs.push(PathBuf::new());
        } else {
            let mut p = path;
            while p.is_cons() {
                let dir = self.xcar(p);
                if dir.is_string() {
                    dirs.push(PathBuf::from(self.string_to_rust(dir)));
                } else if dir.is_nil() {
                    let cwd = self.plain_value(sym::DEFAULT_DIRECTORY);
                    if cwd.is_string() {
                        dirs.push(PathBuf::from(self.string_to_rust(cwd)));
                    } else {
                        dirs.push(std::env::current_dir().unwrap_or_default());
                    }
                }
                p = self.xcdr(p);
            }
        }
        for dir in dirs {
            for suffix in suffixes {
                let full = dir.join(format!("{file}{suffix}"));
                if full.is_file() {
                    return Some(full);
                }
            }
        }
        None
    }
}

fn restore_autoload_queue(lisp: &mut Lisp, old: Value) -> LispResult<()> {
    lisp.set_plain_value(sym::AUTOLOAD_QUEUE, old);
    Ok(())
}

pub fn decode_utf8_emacs(bytes: &[u8]) -> (Vec<u8>, usize) {
    let mut out = Vec::with_capacity(bytes.len());
    let mut chars = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            out.push(b);
            i += 1;
        } else if let Some(len) = character::multibyte_length(&bytes[i..])
            && b >= 0xC2
        {
            out.extend_from_slice(&bytes[i..i + len]);
            i += len;
        } else {
            character::push_char(&mut out, character::byte8_to_char(b));
            i += 1;
        }
        chars += 1;
    }
    (out, chars)
}

pub fn lexical_cookie(data: &[u8]) -> Option<bool> {
    let mut text = data;
    if text.starts_with(b"#!") {
        let end = text
            .iter()
            .position(|&b| b == b'\n')
            .map_or(text.len(), |p| p + 1);
        text = &text[end..];
    }
    if text.first() != Some(&b';') {
        return None;
    }
    let line_end = text.iter().position(|&b| b == b'\n').unwrap_or(text.len());
    let line = &text[..line_end];
    let line = String::from_utf8_lossy(line);
    let start = line.find("-*-")?;
    let rest = &line[start + 3..];
    let end = rest.find("-*-")?;
    let vars = &rest[..end];
    for part in vars.split(';') {
        let mut kv = part.splitn(2, ':');
        let key = kv.next()?.trim();
        let value = kv.next().map(str::trim).unwrap_or("");
        if key == "lexical-binding" {
            return Some(value != "nil");
        }
    }
    None
}

fn f_read(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    read_internal(lisp, args[0], false)
}

fn f_read_positioning_symbols(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    read_internal(lisp, args[0], true)
}

fn read_internal(lisp: &mut Lisp, stream: Value, locate_syms: bool) -> LispResult {
    let stream = if stream.is_nil() {
        lisp.plain_value(sym::STANDARD_INPUT)
    } else {
        stream
    };
    let stream = if stream == sym::T {
        sym::READ_MINIBUFFER
    } else {
        stream
    };
    if stream == sym::READ_MINIBUFFER {
        let prompt = lisp.string("Lisp expression: ");
        let f = sym::READ_MINIBUFFER;
        return lisp.funcall(f, &[prompt]);
    }
    if stream.is_string() {
        let (data, multibyte) = {
            let ls = lisp.heap.string_ref(stream);
            (ls.data.clone(), ls.multibyte)
        };
        lisp.stack.push(stream);
        let r = lisp.read_from_source(Source::from_bytes(data, multibyte), locate_syms);
        lisp.stack.pop();
        return r.map(|(v, _)| v);
    }
    if lisp.is_buffer(stream) || lisp.is_marker(stream) {
        let source = lisp.buffer_source(stream)?;
        lisp.stack.push(stream);
        let r = lisp.read_from_source(source, locate_syms);
        lisp.stack.pop();
        let (value, source) = r?;
        lisp.buffer_source_finish(stream, &source);
        return Ok(value);
    }
    let source = Source {
        kind: SourceKind::Function(stream),
        pos: 0,
        chars_read: 0,
        unread: Vec::new(),
        from_file: false,
    };
    lisp.stack.push(stream);
    let r = lisp.read_from_source(source, locate_syms);
    lisp.stack.pop();
    r.map(|(v, _)| v)
}

fn f_read_from_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (string, start, end) = (args[0], args[1], args[2]);
    lisp.check_string(string)?;
    let size = lisp.string_chars(string);
    let (from, to) = lisp.validate_subarray(string, start, end, size)?;
    let sub = lisp.substring_chars(string, from, to);
    let (data, multibyte) = {
        let ls = lisp.heap.string_ref(sub);
        (ls.data.clone(), ls.multibyte)
    };
    lisp.stack.push(string);
    let r = lisp.read_from_source(Source::from_bytes(data, multibyte), false);
    lisp.stack.pop();
    let (value, source) = r?;
    let consumed = source.chars_read;
    let index = Value::fixnum((from + consumed) as i64);
    Ok(lisp.cons(value, index))
}

fn f_lread_substitute_object_in_subtree(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (object, placeholder, completed) = (args[0], args[1], args[2]);
    let mut set = HashSet::new();
    if completed.is_vectorlike() && lisp.is_hash_table(completed) {
        if let Vectorlike::HashTable(h) = lisp.heap.vectorlike_ref(completed) {
            set.extend(h.live_entries().into_iter().map(|(k, _)| k));
        }
    } else if completed == sym::T {
        let mut all = HashSet::new();
        collect_all(lisp, object, &mut all);
        set = all;
    }
    substitute_object(lisp, object, placeholder, &set)?;
    Ok(Value::NIL)
}

fn collect_all(lisp: &Lisp, v: Value, out: &mut HashSet<Value>) {
    if v.is_cons() || v.is_vectorlike() {
        if !out.insert(v) {
            return;
        }
        if v.is_cons() {
            collect_all(lisp, lisp.xcar(v), out);
            collect_all(lisp, lisp.xcdr(v), out);
        } else if let Vectorlike::Vector(items) | Vectorlike::Record(items) =
            lisp.heap.vectorlike_ref(v)
        {
            for item in items.clone() {
                collect_all(lisp, item, out);
            }
        }
    }
}

fn f_intern(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (string, obarray) = (args[0], args[1]);
    let string = lisp.check_string(string)?;
    let obarray = if obarray.is_nil() {
        lisp.plain_value(sym::OBARRAY)
    } else {
        obarray
    };
    let obarray = lisp.check_obarray(obarray)?;
    let bytes = lisp.string_bytes(string).to_vec();
    if let Some(found) = lisp.obarray_lookup(obarray, &bytes) {
        return Ok(found);
    }
    let name = if lisp.var_true(sym::PURIFY_FLAG) {
        string
    } else {
        let s = lisp.heap.string_ref(string).clone();
        let mut s = s;
        s.props.clear();
        lisp.heap.string(s)
    };
    Ok(lisp.intern_new(obarray, name))
}

fn f_intern_soft(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (name, obarray) = (args[0], args[1]);
    let obarray = if obarray.is_nil() {
        lisp.plain_value(sym::OBARRAY)
    } else {
        obarray
    };
    let obarray = lisp.check_obarray(obarray)?;
    let bare = lisp.maybe_remove_pos_from_symbol(name);
    if bare.is_symbol() {
        let s = lisp.symbol_name(bare);
        let bytes = lisp.string_bytes(s).to_vec();
        return Ok(match lisp.obarray_lookup(obarray, &bytes) {
            Some(found) if found == bare => found,
            _ => Value::NIL,
        });
    }
    let string = lisp.check_string(name)?;
    let bytes = lisp.string_bytes(string).to_vec();
    Ok(lisp.obarray_lookup(obarray, &bytes).unwrap_or(Value::NIL))
}

fn f_unintern(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (name, obarray) = (args[0], args[1]);
    let obarray = if obarray.is_nil() {
        lisp.plain_value(sym::OBARRAY)
    } else {
        obarray
    };
    let obarray = lisp.check_obarray(obarray)?;
    let (bytes, symbol) = if name.is_symbol() {
        let s = lisp.symbol_name(name);
        (lisp.string_bytes(s).to_vec(), Some(name))
    } else {
        let s = lisp.check_string(name)?;
        (lisp.string_bytes(s).to_vec(), None)
    };
    let Some(found) = lisp.obarray_lookup(obarray, &bytes) else {
        return Ok(Value::NIL);
    };
    if symbol.is_some_and(|s| s != found) {
        return Ok(Value::NIL);
    }
    if let Vectorlike::Obarray(o) = lisp.heap.vectorlike_mut(obarray) {
        o.symbols.remove(&bytes);
    }
    lisp.heap.symbol_mut(found).interned = Interned::Uninterned;
    Ok(sym::T)
}

fn f_obarray_make(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if !args[0].is_nil() {
        lisp.check_fixnat(args[0])?;
    }
    Ok(lisp
        .heap
        .vectorlike(Vectorlike::Obarray(Obarray::default())))
}

fn f_obarrayp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(args[0].is_vectorlike() && lisp.is_obarray(args[0])))
}

fn f_obarray_clear(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let obarray = lisp.check_obarray(args[0])?;
    if let Vectorlike::Obarray(o) = lisp.heap.vectorlike_mut(obarray) {
        o.symbols.clear();
    }
    Ok(Value::NIL)
}

fn f_mapatoms(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, obarray) = (args[0], args[1]);
    let obarray = if obarray.is_nil() {
        lisp.plain_value(sym::OBARRAY)
    } else {
        obarray
    };
    let obarray = lisp.check_obarray(obarray)?;
    let symbols: Vec<Value> = match lisp.heap.vectorlike_ref(obarray) {
        Vectorlike::Obarray(o) => o.symbols.values().copied().collect(),
        _ => Vec::new(),
    };
    let base = lisp.stack.len();
    lisp.stack.extend_from_slice(&symbols);
    for s in symbols {
        if let Err(e) = lisp.funcall(function, &[s]) {
            lisp.stack.truncate(base);
            return Err(e);
        }
    }
    lisp.stack.truncate(base);
    Ok(Value::NIL)
}

fn f_get_load_suffixes(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let suffixes = lisp.get_load_suffixes();
    let values: Vec<Value> = suffixes.iter().map(|s| lisp.string(s)).collect();
    Ok(lisp.list(&values))
}

fn f_locate_file_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (filename, path, suffixes, _predicate) = (args[0], args[1], args[2], args[3]);
    lisp.check_string(filename)?;
    let mut sfx = Vec::new();
    let mut s = suffixes;
    while s.is_cons() {
        let item = lisp.xcar(s);
        sfx.push(lisp.string_to_rust(item));
        s = lisp.xcdr(s);
    }
    if sfx.is_empty() {
        sfx.push(String::new());
    }
    let name = lisp.string_to_rust(filename);
    match lisp.locate_file(path, &name, &sfx) {
        Some(found) => {
            let s = found.to_string_lossy().replace('\\', "/");
            Ok(lisp.string(&s))
        }
        None => Ok(Value::NIL),
    }
}

fn f_get_file_char(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(-1))
}

fn f_load(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (file, noerror, nomessage, nosuffix, must_suffix) =
        (args[0], args[1], args[2], args[3], args[4]);
    let file = lisp.check_string(file)?;
    let name = lisp.string_to_rust(file);
    let found = if name.is_empty() {
        None
    } else {
        let has_suffix = name.ends_with(".el") || name.ends_with(".elc");
        let has_dir = name.contains('/') || name.contains('\\');
        let must = !must_suffix.is_nil() && !has_suffix && !has_dir;
        let suffixes: Vec<String> = if !nosuffix.is_nil() {
            vec![String::new()]
        } else {
            let mut s = lisp.get_load_suffixes();
            if !must {
                let reps = lisp.plain_value(sym::LOAD_FILE_REP_SUFFIXES);
                let mut r = reps;
                while r.is_cons() {
                    let rep = lisp.xcar(r);
                    s.push(lisp.string_to_rust(rep));
                    r = lisp.xcdr(r);
                }
            }
            s
        };
        let path = lisp.plain_value(sym::LOAD_PATH);
        lisp.locate_file(path, &name, &suffixes)
    };
    let Some(found) = found else {
        if noerror.is_nil() {
            let msg = lisp.string("Cannot open load file");
            let detail = lisp.string("No such file or directory");
            return Err(lisp.signal_list(sym::FILE_MISSING, &[msg, detail, file]));
        }
        return Ok(Value::NIL);
    };
    let found_str = found.to_string_lossy().replace('\\', "/");
    let found_value = lisp.string(&found_str);
    lisp.stack.push(found_value);
    let count = lisp.specpdl.len();
    let loads = lisp.plain_value(sym::LOADS_IN_PROGRESS);
    let mut load_count = 0;
    let mut tem = loads;
    while tem.is_cons() {
        let item = lisp.xcar(tem);
        if lisp.equal(found_value, item)? {
            load_count += 1;
            if load_count > 3 {
                let data = lisp.cons(found_value, loads);
                return Err(lisp.signal_error("Recursive load", data));
            }
        }
        tem = lisp.xcdr(tem);
    }
    let new_loads = lisp.cons(found_value, loads);
    lisp.specbind(sym::LOADS_IN_PROGRESS, new_loads)?;
    lisp.specbind(sym::LEXICAL_BINDING, Value::NIL)?;
    lisp.specbind(sym::LREAD_UNESCAPED_CHARACTER_LITERALS, Value::NIL)?;
    let is_elc = found_str.ends_with(".elc");
    if !is_elc {
        let source_fn = lisp.plain_value(sym::LOAD_SOURCE_FILE_FUNCTION);
        if !source_fn.is_nil() {
            let noerr = if noerror.is_nil() { Value::NIL } else { sym::T };
            let nomsg = if nomessage.is_nil() {
                Value::NIL
            } else {
                sym::T
            };
            let r = lisp.funcall(source_fn, &[found_value, found_value, noerr, nomsg]);
            lisp.stack.pop();
            return lisp.unbind_to_value(count, r);
        }
    }
    let bytes = match std::fs::read(&found) {
        Ok(b) => b,
        Err(e) => {
            let msg = lisp.string("Opening input file");
            let detail = lisp.string(&e.to_string());
            return Err(lisp.signal_list(sym::FILE_ERROR, &[msg, detail, found_value]));
        }
    };
    if nomessage.is_nil() || lisp.var_true(sym::FORCE_LOAD_MESSAGES) {
        let kind = if is_elc { "" } else { " (source)" };
        lisp.message_raw(&format!("Loading {name}{kind}..."));
    }
    lisp.specbind(sym::LOAD_FILE_NAME, found_value)?;
    lisp.specbind(sym::LOAD_TRUE_FILE_NAME, found_value)?;
    lisp.specbind(sym::INHIBIT_FILE_NAME_OPERATION, Value::NIL)?;
    lisp.specbind(sym::LOAD_IN_PROGRESS, sym::T)?;
    let lexical = if is_elc {
        false
    } else {
        lexical_cookie(&bytes).unwrap_or(false)
    };
    let lexical_value = if lexical { sym::T } else { Value::NIL };
    lisp.set(sym::LEXICAL_BINDING, lexical_value)?;
    let (data, _) = decode_utf8_emacs(&bytes);
    let mut source = Source::from_bytes(data, true);
    source.from_file = true;
    let r = readevalloop(lisp, source, found_value, is_elc);
    lisp.stack.pop();
    lisp.unbind_to_value(count, r)?;
    let after = sym::DO_AFTER_LOAD_EVALUATION;
    if !lisp.heap.symbol_ref(after).function.is_nil() {
        lisp.funcall(after, &[found_value])?;
    }
    Ok(sym::T)
}

pub fn readevalloop(
    lisp: &mut Lisp,
    source: Source,
    sourcename: Value,
    is_elc: bool,
) -> LispResult {
    let count = lisp.specpdl.len();
    let macroexpand = sym::INTERNAL_MACROEXPAND_FOR_LOAD;
    let use_macroexpand = !lisp.heap.symbol_ref(macroexpand).function.is_nil() && !is_elc;
    if is_elc {
        lisp.specbind(sym::READ_SYMBOL_SHORTHANDS, Value::NIL)?;
    }
    let lex_bound = lisp.find_symbol_value(sym::LEXICAL_BINDING);
    let env = if lex_bound.is_nil() || lex_bound == sym::UNBOUND {
        Value::NIL
    } else {
        lisp.list_of_t
    };
    lisp.specbind(sym::INTERNAL_INTERPRETER_ENVIRONMENT, env)?;
    let dynvars = lisp.plain_value(sym::MACROEXP__DYNVARS);
    lisp.specbind(sym::MACROEXP__DYNVARS, dynvars)?;
    lisp.loadhist_initialize(sourcename)?;
    let mut source = source;
    loop {
        let mut reader = Reader::new(lisp, source);
        let c = loop {
            let c = reader.readchar()?;
            if c == b';' as i64 {
                loop {
                    let d = reader.readchar()?;
                    if d < 0 || d == b'\n' as i64 {
                        break;
                    }
                }
                continue;
            }
            if matches!(c, 32 | 9 | 10 | 12 | 13) || c == NO_BREAK_SPACE as i64 {
                continue;
            }
            break c;
        };
        if c < 0 {
            break;
        }
        reader.unreadchar(c)?;
        let form = reader.read0(false)?;
        source = reader.source;
        lisp.stack.push(form);
        let r = if use_macroexpand {
            eager_expand_eval(lisp, form, macroexpand)
        } else {
            lisp.eval(form)
        };
        lisp.stack.pop();
        r?;
    }
    build_load_history(lisp, sourcename, true)?;
    lisp.unbind_to(count)?;
    Ok(Value::NIL)
}

fn eager_expand_eval(lisp: &mut Lisp, form: Value, macroexpand: Value) -> LispResult {
    let val = lisp.funcall(macroexpand, &[form, Value::NIL])?;
    lisp.stack.push(val);
    let result = if lisp.car_safe(val) == sym::PROGN {
        let mut sub = lisp.xcdr(val);
        let mut last = Value::NIL;
        while sub.is_cons() {
            let item = lisp.xcar(sub);
            last = eager_expand_eval(lisp, item, macroexpand)?;
            sub = lisp.xcdr(sub);
        }
        Ok(last)
    } else {
        let expanded = lisp.funcall(macroexpand, &[val, sym::T])?;
        lisp.stack.push(expanded);
        let r = lisp.eval(expanded);
        lisp.stack.pop();
        r
    };
    lisp.stack.pop();
    result
}

impl Lisp {
    pub fn loadhist_initialize(&mut self, source: Value) -> LispResult<()> {
        let list = self.list(&[source]);
        self.specbind(sym::CURRENT_LOAD_LIST, list)
    }
}

fn build_load_history(lisp: &mut Lisp, filename: Value, entire: bool) -> LispResult<()> {
    let mut tail = lisp.plain_value(sym::LOAD_HISTORY);
    let mut prev = Value::NIL;
    let mut found = false;
    while tail.is_cons() {
        let tem = lisp.xcar(tail);
        let head = lisp.car_safe(tem);
        if lisp.equal(filename, head)? {
            found = true;
            if entire {
                let next = lisp.xcdr(tail);
                if prev.is_nil() {
                    lisp.set_plain_value(sym::LOAD_HISTORY, next);
                } else {
                    lisp.heap.set_cdr(prev, next);
                }
            }
        } else {
            prev = tail;
        }
        tail = lisp.xcdr(tail);
    }
    if entire || !found {
        let current = lisp.plain_value(sym::CURRENT_LOAD_LIST);
        if current.is_cons() {
            let reversed = lisp.reverse_list(current)?;
            let history = lisp.plain_value(sym::LOAD_HISTORY);
            let history = lisp.cons(reversed, history);
            lisp.set_plain_value(sym::LOAD_HISTORY, history);
        }
        lisp.set_plain_value(sym::CURRENT_LOAD_LIST, sym::T);
    }
    Ok(())
}
