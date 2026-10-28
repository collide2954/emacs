use crate::character::{self, MAX_CHAR};
use crate::editfns::f_constrain_to_field;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::regex::{char_leading_code, wctype_parse};
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

const ST_COMMENT_STYLE: i32 = 256 + 1;
const ST_STRING_STYLE: i32 = 256 + 2;
const SMAX: i32 = S_MAX as i32;

#[derive(Clone, Copy)]
pub struct GlState {
    pub object: Value,
    pub start: i64,
    pub stop: i64,
    pub use_global: bool,
    pub global_code: Value,
    pub current_syntax_table: Value,
    pub old_prop: Value,
    pub b_property: i64,
    pub e_property: i64,
    pub e_property_truncated: bool,
    pub intervals_initialized: bool,
}

pub struct Syntax {
    pub gl: GlState,
    pub find_start_pos: i64,
    pub find_start_value: i64,
    pub find_start_buffer: Value,
    pub find_start_begv: i64,
    pub find_start_modiff: i64,
}

impl Default for Syntax {
    fn default() -> Self {
        Syntax {
            gl: GlState {
                object: Value::NIL,
                start: 0,
                stop: 0,
                use_global: false,
                global_code: Value::NIL,
                current_syntax_table: Value::NIL,
                old_prop: Value::NIL,
                b_property: 0,
                e_property: 0,
                e_property_truncated: false,
                intervals_initialized: false,
            },
            find_start_pos: 0,
            find_start_value: 0,
            find_start_buffer: Value::NIL,
            find_start_begv: 0,
            find_start_modiff: 0,
        }
    }
}

impl Syntax {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend([
            self.gl.object,
            self.gl.global_code,
            self.gl.current_syntax_table,
            self.gl.old_prop,
            self.find_start_buffer,
        ]);
    }
}

struct ParseState {
    depth: i64,
    instring: i32,
    incomment: i64,
    comstyle: i32,
    quoted: bool,
    mindepth: i64,
    thislevelstart: i64,
    prevlevelstart: i64,
    location: i64,
    comstr_start: i64,
    levelstarts: Value,
    prev_syntax: i32,
}

impl Default for ParseState {
    fn default() -> Self {
        ParseState {
            depth: 0,
            instring: -1,
            incomment: 0,
            comstyle: 0,
            quoted: false,
            mindepth: 0,
            thislevelstart: 0,
            prevlevelstart: 0,
            location: 0,
            comstr_start: -1,
            levelstarts: Value::NIL,
            prev_syntax: SMAX,
        }
    }
}

struct Cursor {
    from: i64,
    prev_from: i64,
    prev_from_syntax: i32,
    prev_prev_from_syntax: i32,
    end: i64,
}

#[derive(Clone, Copy, PartialEq)]
enum Pc {
    Loop,
    StartQuoted,
    SymStarted,
    AtComment,
    StartInComment,
    StartInString,
    StartQuotedInString,
    Stop,
    EndQuoted,
    Done,
}

enum BackExit {
    Normal,
    Lossage,
    Done,
}

fn comstart_first(flags: i32) -> bool {
    (flags >> 16) & 1 != 0
}

fn comstart_second(flags: i32) -> bool {
    (flags >> 17) & 1 != 0
}

fn comend_first(flags: i32) -> bool {
    (flags >> 18) & 1 != 0
}

fn comend_second(flags: i32) -> bool {
    (flags >> 19) & 1 != 0
}

fn comstartend_first(flags: i32) -> bool {
    flags & 0x50000 != 0
}

fn prefix_flag(flags: i32) -> bool {
    (flags >> 20) & 1 != 0
}

fn comment_styleb(flags: i32) -> i32 {
    (flags >> 21) & 1
}

fn comment_stylec2(flags: i32) -> i32 {
    (flags >> 22) & 2
}

fn comment_nested(flags: i32) -> bool {
    (flags >> 22) & 1 != 0
}

fn comment_style(flags: i32, other_flags: i32) -> i32 {
    comment_styleb(flags) | comment_stylec2(flags) | comment_stylec2(other_flags)
}

fn code_of(flags: i32) -> u8 {
    (flags & 0xff) as u8
}

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
    Subr::fixed("forward-word", 0, 1, f_forward_word).interactive("^p"),
    Subr::fixed("skip-chars-forward", 1, 2, f_skip_chars_forward),
    Subr::fixed("skip-chars-backward", 1, 2, f_skip_chars_backward),
    Subr::fixed("skip-syntax-forward", 1, 2, f_skip_syntax_forward),
    Subr::fixed("skip-syntax-backward", 1, 2, f_skip_syntax_backward),
    Subr::fixed("forward-comment", 1, 1, f_forward_comment),
    Subr::fixed("scan-lists", 3, 3, f_scan_lists),
    Subr::fixed("scan-sexps", 2, 2, f_scan_sexps),
    Subr::fixed("backward-prefix-chars", 0, 0, f_backward_prefix_chars),
    Subr::fixed("parse-partial-sexp", 2, 6, f_parse_partial_sexp),
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

impl Lisp {
    fn syntax_var_p(&self, symbol: Value) -> bool {
        !self.plain_value(symbol).is_nil()
    }

    fn lookup_properties(&self) -> bool {
        self.syntax_var_p(sym::PARSE_SEXP_LOOKUP_PROPERTIES)
    }

    fn fetch_mb(&self, pos: i64) -> u32 {
        self.char_at_pos(pos as usize)
    }

    fn fetch_raw(&self, pos: i64) -> u32 {
        let text = &self.cur().text;
        text.char_at_byte(text.char_to_byte(pos as usize))
    }

    fn gl_entry(&self, c: u32) -> Value {
        let gl = &self.syntax.gl;
        if gl.use_global {
            gl.global_code
        } else if self.is_char_table(gl.current_syntax_table) {
            self.char_table_ref(gl.current_syntax_table, c)
        } else {
            Value::NIL
        }
    }

    fn gl_syntax_with_flags(&self, c: u32) -> i32 {
        let ent = self.gl_entry(c);
        if ent.is_cons() {
            self.xcar(ent).as_fixnum().unwrap_or(0) as i32
        } else {
            S_WHITESPACE as i32
        }
    }

    fn gl_syntax(&self, c: u32) -> u8 {
        code_of(self.gl_syntax_with_flags(c))
    }

    fn syntax_multibyte(&self, c: u32, multibyte_symbol_p: bool) -> u8 {
        if character::ascii_char_p(c) || !multibyte_symbol_p {
            self.gl_syntax(c)
        } else {
            S_SYMBOL
        }
    }

    pub fn setup_buffer_syntax_table(&mut self) {
        let table = self.cur().syntax_table;
        let gl = &mut self.syntax.gl;
        gl.use_global = false;
        gl.e_property_truncated = false;
        gl.current_syntax_table = table;
    }

    pub fn setup_syntax_table(&mut self, from: i64, count: i64) -> LispResult<()> {
        self.setup_buffer_syntax_table();
        let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
        let gl = &mut self.syntax.gl;
        gl.b_property = begv;
        gl.e_property = zv + 1;
        gl.object = Value::NIL;
        if self.lookup_properties() {
            let gl = &mut self.syntax.gl;
            gl.start = gl.b_property;
            gl.stop = gl.e_property;
            gl.intervals_initialized = false;
            if count > 0 {
                self.update_syntax_table_forward(from, Value::NIL)?;
            } else if from > begv {
                self.update_syntax_table(from - 1, count, Value::NIL)?;
                self.parse_sexp_propertize(from - 1)?;
            }
        }
        Ok(())
    }

    pub fn update_syntax_table(
        &mut self,
        charpos: i64,
        count: i64,
        object: Value,
    ) -> LispResult<()> {
        if !self.syntax.gl.intervals_initialized {
            self.syntax.gl.old_prop = Value::NIL;
            self.syntax.gl.intervals_initialized = true;
        }
        let Some(run) = self.interval_run(charpos, object, sym::SYNTAX_TABLE)? else {
            let gl = &mut self.syntax.gl;
            gl.b_property = gl.start;
            gl.e_property = gl.stop;
            return Ok(());
        };
        let prop = run.value;
        if prop != self.syntax.gl.old_prop {
            let is_table = self.is_syntax_table(prop);
            let buffer_table = self.cur().syntax_table;
            let gl = &mut self.syntax.gl;
            gl.current_syntax_table = prop;
            gl.old_prop = prop;
            if is_table {
                gl.use_global = false;
            } else if prop.is_cons() {
                gl.use_global = true;
                gl.global_code = prop;
            } else {
                gl.use_global = false;
                gl.current_syntax_table = buffer_table;
            }
        }
        let gl = &mut self.syntax.gl;
        if count > 0 {
            gl.b_property = run.start;
            gl.e_property = if run.at_last { gl.stop } else { run.end };
        } else {
            gl.e_property = run.end;
            gl.b_property = if run.at_first { gl.start } else { run.start };
        }
        Ok(())
    }

    fn parse_sexp_propertize(&mut self, charpos: i64) -> LispResult<()> {
        let zv = self.cur().zv as i64;
        let done = self.int_var(sym::SYNTAX_PROPERTIZE__DONE);
        if done <= charpos && done < zv {
            let modiffs = self.cur().text.chars_modiff;
            self.safe_calln(
                sym::INTERNAL__SYNTAX_PROPERTIZE,
                &[Value::fixnum(zv.min(1 + charpos))],
            )?;
            if modiffs != self.cur().text.chars_modiff {
                return Err(self.error("internal--syntax-propertize modified the buffer!"));
            }
            let done = self.int_var(sym::SYNTAX_PROPERTIZE__DONE);
            if done <= charpos && done < zv {
                return Err(
                    self.error("internal--syntax-propertize did not move syntax-propertize--done")
                );
            }
            self.setup_syntax_table(charpos, 1)?;
        } else if self.syntax.gl.e_property > done {
            self.syntax.gl.e_property = done;
            self.syntax.gl.e_property_truncated = true;
        } else if self.syntax.gl.e_property_truncated && self.syntax.gl.e_property < done {
            self.syntax.gl.e_property_truncated = false;
            self.update_syntax_table_forward(charpos, Value::NIL)?;
        }
        Ok(())
    }

    pub fn update_syntax_table_forward(&mut self, charpos: i64, object: Value) -> LispResult<()> {
        if self.syntax.gl.e_property_truncated {
            return self.parse_sexp_propertize(charpos);
        }
        self.update_syntax_table(charpos, 1, object)?;
        if object.is_nil() && self.syntax.gl.e_property > self.int_var(sym::SYNTAX_PROPERTIZE__DONE)
        {
            self.parse_sexp_propertize(charpos)?;
        }
        Ok(())
    }

    fn update_forward(&mut self, charpos: i64) -> LispResult<()> {
        if self.lookup_properties() && charpos >= self.syntax.gl.e_property {
            let object = self.syntax.gl.object;
            self.update_syntax_table_forward(charpos, object)?;
        }
        Ok(())
    }

    fn update_backward(&mut self, charpos: i64) -> LispResult<()> {
        if self.lookup_properties() && charpos < self.syntax.gl.b_property {
            let object = self.syntax.gl.object;
            self.update_syntax_table(charpos, -1, object)?;
        }
        Ok(())
    }

    fn update_syntax(&mut self, charpos: i64) -> LispResult<()> {
        self.update_backward(charpos)?;
        self.update_forward(charpos)
    }

    fn char_quoted(&mut self, mut charpos: i64) -> LispResult<bool> {
        let beg = self.cur().begv as i64;
        let mut quoted = false;
        let orig = charpos;
        while charpos > beg {
            charpos -= 1;
            self.update_backward(charpos)?;
            let c = self.fetch_mb(charpos);
            let code = self.gl_syntax(c);
            if !(code == S_CHARQUOTE || code == S_ESCAPE) {
                break;
            }
            quoted = !quoted;
        }
        self.update_syntax(orig)?;
        Ok(quoted)
    }

    fn scan_newline(&mut self, start: i64, limit: i64, count: i64) {
        let (pos, counted) = self.find_newline(start as usize, limit as usize, count);
        let target = if counted != count {
            limit as usize
        } else {
            pos
        };
        self.goto_char_internal(target);
    }

    fn find_defun_start(&mut self, pos: i64) -> LispResult<i64> {
        let opoint = self.cur().pt;
        let begv = self.cur().begv as i64;
        let fs = &self.syntax;
        if self.current_buffer == fs.find_start_buffer
            && pos <= fs.find_start_pos + 1000
            && pos >= fs.find_start_value
            && begv == fs.find_start_begv
            && self.cur().text.modiff == fs.find_start_modiff
        {
            return Ok(fs.find_start_value);
        }
        if self.syntax_var_p(sym::COMMENT_USE_SYNTAX_PPSS) {
            let modiffs = self.cur().text.chars_modiff;
            let ppss = self.funcall(sym::SYNTAX_PPSS, &[Value::fixnum(pos)])?;
            if modiffs != self.cur().text.chars_modiff {
                return Err(self.error("syntax-ppss modified the buffer!"));
            }
            self.goto_char_internal(opoint);
            let boc = self.nth(8, ppss);
            self.syntax.find_start_value = boc.as_fixnum().unwrap_or(pos);
        } else if !self.syntax_var_p(sym::OPEN_PAREN_IN_COLUMN_0_IS_DEFUN_START) {
            self.syntax.find_start_value = begv;
        } else {
            self.scan_newline(pos, begv, -1);
            self.setup_buffer_syntax_table();
            while self.cur().pt as i64 > self.cur().begv as i64 {
                let pt = self.cur().pt as i64;
                let c = self.fetch_mb(pt);
                if self.gl_syntax(c) == S_OPEN {
                    self.setup_syntax_table(pt + 1, -1)?;
                    let c = self.fetch_mb(self.cur().pt as i64);
                    if self.gl_syntax(c) == S_OPEN {
                        break;
                    }
                    self.setup_buffer_syntax_table();
                }
                let (pt, begv) = (self.cur().pt as i64, self.cur().begv as i64);
                self.scan_newline(pt, begv, -2);
            }
            self.syntax.find_start_value = self.cur().pt as i64;
            self.goto_char_internal(opoint);
        }
        self.syntax.find_start_buffer = self.current_buffer;
        self.syntax.find_start_modiff = self.cur().text.modiff;
        self.syntax.find_start_begv = self.cur().begv as i64;
        self.syntax.find_start_pos = pos;
        Ok(self.syntax.find_start_value)
    }

    fn prev_char_comend_first(&mut self, pos: i64) -> LispResult<bool> {
        let pos = pos - 1;
        self.update_backward(pos)?;
        let c = self.fetch_raw(pos);
        let val = comend_first(self.gl_syntax_with_flags(c));
        self.update_forward(pos + 1)?;
        Ok(val)
    }

    fn back_comment(
        &mut self,
        mut from: i64,
        stop: i64,
        comnested: bool,
        comstyle: i32,
    ) -> LispResult<(bool, i64)> {
        let mut string_style: i32 = -1;
        let mut string_lossage = false;
        let mut comment_lossage = false;
        let comment_end = from;
        let mut comstart_pos: i64 = 0;
        let mut defun_start: i64 = 0;
        let mut nesting: i64 = 1;
        let mut syntax: i32 = 0;
        let exit = 'scan: {
            while from != stop {
                from -= 1;
                self.update_backward(from)?;
                let prev_syntax = syntax;
                let c = self.fetch_mb(from);
                syntax = self.gl_syntax_with_flags(c);
                let mut code = self.gl_syntax(c);
                let com2start = comstart_first(syntax)
                    && comstart_second(prev_syntax)
                    && comstyle == comment_style(prev_syntax, syntax)
                    && (comment_nested(prev_syntax) || comment_nested(syntax)) == comnested;
                let mut com2end = comend_first(syntax) && comend_second(prev_syntax);
                let comstart = com2start || code == S_COMMENT;
                if from > stop && (com2end || comstart) {
                    let next = from - 1;
                    self.update_backward(next)?;
                    let next_c = self.fetch_mb(next);
                    let next_syntax = self.gl_syntax_with_flags(next_c);
                    if ((comstart || comnested)
                        && comend_second(syntax)
                        && comend_first(next_syntax))
                        || ((com2end || comnested)
                            && comstart_second(syntax)
                            && comstyle == comment_style(syntax, prev_syntax)
                            && comstart_first(next_syntax))
                    {
                        break 'scan BackExit::Lossage;
                    }
                }
                if com2start && comstart_pos == 0 {
                    com2end = false;
                }
                if com2end {
                    code = S_ENDCOMMENT;
                } else if com2start {
                    code = S_COMMENT;
                } else if code == S_COMMENT
                    && (comstyle != comment_style(syntax, 0) || comment_nested(syntax) != comnested)
                {
                    continue;
                }
                if (self.syntax_var_p(sym::COMMENT_END_CAN_BE_ESCAPED) || code != S_ENDCOMMENT)
                    && self.char_quoted(from)?
                {
                    continue;
                }
                match code {
                    S_STRING_FENCE | S_COMMENT_FENCE | S_STRING => {
                        let c = match code {
                            S_STRING_FENCE => ST_STRING_STYLE,
                            S_COMMENT_FENCE => ST_COMMENT_STYLE,
                            _ => c as i32,
                        };
                        if string_style == -1 {
                            string_style = c;
                        } else if string_style == c {
                            string_style = -1;
                        } else {
                            string_lossage = true;
                        }
                    }
                    S_COMMENT => {
                        if string_style != -1 || comment_lossage || string_lossage {
                            break 'scan BackExit::Lossage;
                        }
                        if !comnested {
                            comstart_pos = from;
                        } else {
                            nesting -= 1;
                            if nesting <= 0 {
                                break 'scan BackExit::Done;
                            }
                        }
                    }
                    S_ENDCOMMENT => {
                        if comment_style(syntax, 0) == comstyle
                            && ((com2end && comment_nested(prev_syntax)) || comment_nested(syntax))
                                == comnested
                        {
                            if comnested {
                                nesting += 1;
                            } else {
                                from = stop;
                            }
                        } else if comstart_pos != 0 || c != b'\n' as u32 {
                            comment_lossage = true;
                        }
                    }
                    S_OPEN
                        if self.syntax_var_p(sym::OPEN_PAREN_IN_COLUMN_0_IS_DEFUN_START)
                            && !self.syntax_var_p(sym::COMMENT_USE_SYNTAX_PPSS)
                            && (from == stop || self.fetch_raw(from - 1) == b'\n' as u32) =>
                    {
                        defun_start = from;
                        from = stop;
                    }
                    _ => {}
                }
            }
            BackExit::Normal
        };
        match exit {
            BackExit::Done => {}
            BackExit::Normal if comstart_pos == 0 => {
                from = comment_end;
                self.update_forward(comment_end)?;
            }
            BackExit::Normal => {
                from = comstart_pos;
                self.update_forward(from - 1)?;
            }
            BackExit::Lossage => {
                let mut adjusted = true;
                if defun_start == 0 {
                    defun_start = self.find_defun_start(comment_end)?;
                    adjusted = defun_start > self.cur().begv as i64;
                }
                loop {
                    let mut state = ParseState::default();
                    self.scan_sexps_forward(
                        &mut state,
                        defun_start,
                        comment_end,
                        i64::MIN,
                        false,
                        0,
                    )?;
                    defun_start = comment_end;
                    if !adjusted {
                        adjusted = true;
                        self.syntax.find_start_value = if state.levelstarts.is_cons() {
                            self.xcar(state.levelstarts).as_fixnum().unwrap_or(0)
                        } else if state.thislevelstart >= 0 {
                            state.thislevelstart
                        } else {
                            self.syntax.find_start_value
                        };
                    }
                    if state.incomment == (if comnested { 1 } else { -1 })
                        && state.comstyle == comstyle
                    {
                        from = state.comstr_start;
                    } else {
                        from = comment_end;
                        if state.incomment != 0 {
                            defun_start = state.comstr_start + 2;
                        }
                    }
                    if defun_start >= comment_end {
                        break;
                    }
                }
                self.update_forward(from - 1)?;
            }
        }
        Ok((from != comment_end, from))
    }

    pub fn scan_words(&mut self, mut from: i64, mut count: i64) -> LispResult<i64> {
        let beg = self.cur().begv as i64;
        let end = self.cur().zv as i64;
        self.setup_syntax_table(from, count)?;
        while count > 0 {
            let mut ch0;
            loop {
                if from == end {
                    return Ok(0);
                }
                self.update_forward(from)?;
                ch0 = self.fetch_mb(from);
                let code = self.gl_syntax(ch0);
                from += 1;
                if self.syntax_var_p(sym::WORDS_INCLUDE_ESCAPES)
                    && (code == S_ESCAPE || code == S_CHARQUOTE)
                {
                    break;
                }
                if code == S_WORD {
                    break;
                }
            }
            let func = self.word_boundary_function(ch0)?;
            if !func.is_nil() {
                let pos = self.funcall(func, &[Value::fixnum(from - 1), Value::fixnum(end)])?;
                if let Some(p) = pos.as_fixnum()
                    && from < p
                    && p <= self.cur().zv as i64
                {
                    from = p;
                }
            } else {
                loop {
                    if from == end {
                        break;
                    }
                    self.update_forward(from)?;
                    let ch1 = self.fetch_mb(from);
                    let code = self.gl_syntax(ch1);
                    if (code != S_WORD
                        && (!self.syntax_var_p(sym::WORDS_INCLUDE_ESCAPES)
                            || (code != S_ESCAPE && code != S_CHARQUOTE)))
                        || self.regex_env(None).word_boundary_p(ch0, ch1)
                    {
                        break;
                    }
                    from += 1;
                    ch0 = ch1;
                }
            }
            count -= 1;
        }
        while count < 0 {
            let mut ch1;
            loop {
                if from == beg {
                    return Ok(0);
                }
                from -= 1;
                self.update_backward(from)?;
                ch1 = self.fetch_mb(from);
                let code = self.gl_syntax(ch1);
                if self.syntax_var_p(sym::WORDS_INCLUDE_ESCAPES)
                    && (code == S_ESCAPE || code == S_CHARQUOTE)
                {
                    break;
                }
                if code == S_WORD {
                    break;
                }
            }
            let func = self.word_boundary_function(ch1)?;
            if !func.is_nil() {
                let pos = self.funcall(func, &[Value::fixnum(from), Value::fixnum(beg)])?;
                if let Some(p) = pos.as_fixnum()
                    && self.cur().begv as i64 <= p
                    && p < from
                {
                    from = p;
                }
            } else {
                loop {
                    if from == beg {
                        break;
                    }
                    from -= 1;
                    self.update_backward(from)?;
                    let ch0 = self.fetch_mb(from);
                    let code = self.gl_syntax(ch0);
                    if (code != S_WORD
                        && (!self.syntax_var_p(sym::WORDS_INCLUDE_ESCAPES)
                            || (code != S_ESCAPE && code != S_CHARQUOTE)))
                        || self.regex_env(None).word_boundary_p(ch0, ch1)
                    {
                        from += 1;
                        break;
                    }
                    ch1 = ch0;
                }
            }
            count += 1;
        }
        Ok(from)
    }

    fn word_boundary_function(&mut self, c: u32) -> LispResult {
        let table = self.plain_value(sym::FIND_WORD_BOUNDARY_FUNCTION_TABLE);
        let func = if self.is_char_table(table) {
            self.char_table_ref(table, c)
        } else {
            Value::NIL
        };
        if !func.is_symbol() {
            return Err(self.wrong_type(sym::SYMBOLP, func));
        }
        if self.heap.symbol_ref(func).function.is_nil() {
            Ok(Value::NIL)
        } else {
            Ok(func)
        }
    }

    fn forw_comment(
        &mut self,
        mut from: i64,
        stop: i64,
        mut nesting: i64,
        style: i32,
        prev_syntax: i32,
    ) -> LispResult<(bool, i64, i64, i32)> {
        if nesting <= 0 {
            nesting = -1;
        }
        let mut syntax = prev_syntax;
        let mut prev_syntax;
        let mut code = syntax & 0xff;
        let mut incomment_entry = syntax != 0 && from < stop;
        loop {
            if !incomment_entry {
                if from == stop {
                    let last = if code == S_ESCAPE as i32
                        || code == S_CHARQUOTE as i32
                        || comend_first(syntax)
                        || (nesting > 0 && comstart_first(syntax))
                    {
                        syntax
                    } else {
                        SMAX
                    };
                    return Ok((false, from, nesting, last));
                }
                let c = self.fetch_mb(from);
                prev_syntax = syntax;
                syntax = self.gl_syntax_with_flags(c);
                code = syntax & 0xff;
                let can_escape = self.syntax_var_p(sym::COMMENT_END_CAN_BE_ESCAPED);
                let mut ender = false;
                if code == S_ENDCOMMENT as i32 && comment_style(syntax, 0) == style {
                    ender = if !comment_nested(syntax) {
                        nesting < 0
                    } else if nesting > 0 {
                        nesting -= 1;
                        nesting == 0
                    } else {
                        false
                    };
                }
                if ender
                    && !(can_escape
                        && ((prev_syntax & 0xff) == S_ESCAPE as i32
                            || (prev_syntax & 0xff) == S_CHARQUOTE as i32))
                {
                    break;
                }
                if code == S_COMMENT_FENCE as i32 && style == ST_COMMENT_STYLE {
                    break;
                }
                if nesting > 0
                    && code == S_COMMENT as i32
                    && comment_nested(syntax)
                    && comment_style(syntax, 0) == style
                {
                    nesting += 1;
                }
                if can_escape && (code == S_ESCAPE as i32 || code == S_CHARQUOTE as i32) {
                    from += 1;
                    self.update_forward(from)?;
                    if from == stop {
                        continue;
                    }
                    syntax = SMAX;
                    code = syntax;
                }
                from += 1;
                self.update_forward(from)?;
            }
            incomment_entry = false;
            if from < stop && comend_first(syntax) {
                let c1 = self.fetch_mb(from);
                let other_syntax = self.gl_syntax_with_flags(c1);
                if comend_second(other_syntax)
                    && comment_style(syntax, other_syntax) == style
                    && (if comment_nested(syntax) || comment_nested(other_syntax) {
                        nesting > 0
                    } else {
                        nesting < 0
                    })
                {
                    syntax = SMAX;
                    nesting -= 1;
                    if nesting <= 0 {
                        break;
                    }
                    from += 1;
                    self.update_forward(from)?;
                }
            }
            if nesting > 0 && from < stop && comstart_first(syntax) {
                let c1 = self.fetch_mb(from);
                let other_syntax = self.gl_syntax_with_flags(c1);
                if comment_style(other_syntax, syntax) == style
                    && comstart_second(other_syntax)
                    && (comment_nested(syntax) || comment_nested(other_syntax))
                {
                    syntax = SMAX;
                    from += 1;
                    self.update_forward(from)?;
                    nesting += 1;
                }
            }
        }
        Ok((true, from, nesting, SMAX))
    }

    fn scan_error(&mut self, message: &str, a: i64, b: i64) -> crate::error::Flow {
        let msg = self.string(message);
        let data = self.list(&[msg, Value::fixnum(a), Value::fixnum(b)]);
        self.signal(sym::SCAN_ERROR, data)
    }

    fn scan_lists(
        &mut self,
        from0: i64,
        mut count: i64,
        mut depth: i64,
        sexpflag: bool,
    ) -> LispResult {
        let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
        let stop = if count > 0 { zv } else { begv };
        let mut mathexit = false;
        let mut min_depth = depth;
        let mut last_good = from0;
        let multibyte_symbol_p = sexpflag && self.syntax_var_p(sym::MULTIBYTE_SYNTAX_AS_SYMBOL);
        if depth > 0 {
            min_depth = 0;
        }
        let mut from = from0.clamp(begv, zv);
        self.setup_syntax_table(from, count)?;
        while count > 0 {
            'done: {
                while from < stop {
                    self.update_forward(from)?;
                    let mut c = self.fetch_mb(from);
                    let syntax = self.gl_syntax_with_flags(c);
                    let mut code = self.syntax_multibyte(c, multibyte_symbol_p);
                    let comstart = comstart_first(syntax);
                    let mut comnested = comment_nested(syntax);
                    let mut comstyle = comment_style(syntax, 0);
                    let prefix = prefix_flag(syntax);
                    if depth == min_depth {
                        last_good = from;
                    }
                    from += 1;
                    self.update_forward(from)?;
                    if from < stop && comstart {
                        c = self.fetch_mb(from);
                        let other_syntax = self.gl_syntax_with_flags(c);
                        if comstart_second(other_syntax)
                            && self.syntax_var_p(sym::PARSE_SEXP_IGNORE_COMMENTS)
                        {
                            code = S_COMMENT;
                            comstyle = comment_style(other_syntax, syntax);
                            comnested |= comment_nested(other_syntax);
                            from += 1;
                            self.update_forward(from)?;
                        }
                    }
                    if prefix {
                        continue;
                    }
                    match code {
                        S_ESCAPE | S_CHARQUOTE | S_WORD | S_SYMBOL => {
                            if code == S_ESCAPE || code == S_CHARQUOTE {
                                if from == stop {
                                    return Err(self.scan_error(
                                        "Unbalanced parentheses",
                                        last_good,
                                        from,
                                    ));
                                }
                                from += 1;
                            }
                            if depth != 0 || !sexpflag {
                                continue;
                            }
                            while from < stop {
                                self.update_forward(from)?;
                                c = self.fetch_mb(from);
                                match self.syntax_multibyte(c, multibyte_symbol_p) {
                                    S_CHARQUOTE | S_ESCAPE => {
                                        from += 1;
                                        if from == stop {
                                            return Err(self.scan_error(
                                                "Unbalanced parentheses",
                                                last_good,
                                                from,
                                            ));
                                        }
                                    }
                                    S_WORD | S_SYMBOL | S_QUOTE => {}
                                    _ => break 'done,
                                }
                                from += 1;
                            }
                            break 'done;
                        }
                        S_COMMENT_FENCE | S_COMMENT => {
                            if code == S_COMMENT_FENCE {
                                comstyle = ST_COMMENT_STYLE;
                            }
                            if !self.syntax_var_p(sym::PARSE_SEXP_IGNORE_COMMENTS) {
                                continue;
                            }
                            self.update_forward(from)?;
                            let (found, out, _, _) =
                                self.forw_comment(from, stop, comnested as i64, comstyle, 0)?;
                            from = out;
                            if !found {
                                if depth == 0 {
                                    break 'done;
                                }
                                return Err(self.scan_error(
                                    "Unbalanced parentheses",
                                    last_good,
                                    from,
                                ));
                            }
                            from += 1;
                            self.update_forward(from)?;
                        }
                        S_MATH | S_OPEN | S_CLOSE => {
                            let mut close = code == S_CLOSE;
                            if code == S_MATH {
                                if !sexpflag {
                                    continue;
                                }
                                if from != stop && c == self.fetch_mb(from) {
                                    from += 1;
                                }
                                if mathexit {
                                    mathexit = false;
                                    close = true;
                                } else {
                                    mathexit = true;
                                }
                            }
                            if close {
                                depth -= 1;
                                if depth == 0 {
                                    break 'done;
                                }
                                if depth < min_depth {
                                    return Err(self.scan_error(
                                        "Containing expression ends prematurely",
                                        last_good,
                                        from,
                                    ));
                                }
                            } else {
                                depth += 1;
                                if depth == 0 {
                                    break 'done;
                                }
                            }
                        }
                        S_STRING | S_STRING_FENCE => {
                            let stringterm = self.fetch_mb(from - 1);
                            loop {
                                if from >= stop {
                                    return Err(self.scan_error(
                                        "Unbalanced parentheses",
                                        last_good,
                                        from,
                                    ));
                                }
                                self.update_forward(from)?;
                                c = self.fetch_mb(from);
                                let c_code = self.syntax_multibyte(c, multibyte_symbol_p);
                                if if code == S_STRING {
                                    c == stringterm && c_code == S_STRING
                                } else {
                                    c_code == S_STRING_FENCE
                                } {
                                    break;
                                }
                                if c_code == S_CHARQUOTE || c_code == S_ESCAPE {
                                    from += 1;
                                }
                                from += 1;
                            }
                            from += 1;
                            if depth == 0 && sexpflag {
                                break 'done;
                            }
                        }
                        _ => {}
                    }
                }
                if depth != 0 {
                    return Err(self.scan_error("Unbalanced parentheses", last_good, from));
                }
                return Ok(Value::NIL);
            }
            count -= 1;
        }
        while count < 0 {
            'done2: {
                while from > stop {
                    from -= 1;
                    self.update_backward(from)?;
                    let c = self.fetch_mb(from);
                    let syntax = self.gl_syntax_with_flags(c);
                    let mut code = self.syntax_multibyte(c, multibyte_symbol_p);
                    if depth == min_depth {
                        last_good = from;
                    }
                    let mut comstyle = 0;
                    let mut comnested = comment_nested(syntax);
                    if code == S_ENDCOMMENT {
                        comstyle = comment_style(syntax, 0);
                    }
                    if from > stop
                        && comend_second(syntax)
                        && self.prev_char_comend_first(from)?
                        && self.syntax_var_p(sym::PARSE_SEXP_IGNORE_COMMENTS)
                    {
                        from -= 1;
                        self.update_backward(from)?;
                        code = S_ENDCOMMENT;
                        let c2 = self.fetch_mb(from);
                        let other_syntax = self.gl_syntax_with_flags(c2);
                        comstyle = comment_style(other_syntax, syntax);
                        comnested |= comment_nested(other_syntax);
                    }
                    if code != S_ENDCOMMENT && self.char_quoted(from)? {
                        from -= 1;
                        code = S_WORD;
                    } else if prefix_flag(syntax) {
                        continue;
                    }
                    match code {
                        S_WORD | S_SYMBOL | S_ESCAPE | S_CHARQUOTE => {
                            if depth != 0 || !sexpflag {
                                continue;
                            }
                            while from > stop {
                                let mut temp_pos = from - 1;
                                self.update_backward(from - 1)?;
                                let c1 = self.fetch_mb(temp_pos);
                                if self.syntax_multibyte(c1, multibyte_symbol_p) == S_ENDCOMMENT {
                                    break 'done2;
                                }
                                let quoted = self.char_quoted(from - 1)?;
                                if quoted {
                                    from -= 1;
                                    temp_pos -= 1;
                                    self.update_backward(from - 1)?;
                                }
                                let c1 = self.fetch_mb(temp_pos);
                                if !quoted
                                    && !matches!(
                                        self.syntax_multibyte(c1, multibyte_symbol_p),
                                        S_WORD | S_SYMBOL | S_QUOTE
                                    )
                                {
                                    break 'done2;
                                }
                                from -= 1;
                            }
                            break 'done2;
                        }
                        S_MATH | S_CLOSE | S_OPEN => {
                            let mut open = code == S_OPEN;
                            if code == S_MATH {
                                if !sexpflag {
                                    continue;
                                }
                                if from > begv {
                                    let temp_pos = from - 1;
                                    self.update_backward(from - 1)?;
                                    if from != stop && c == self.fetch_mb(temp_pos) {
                                        from -= 1;
                                    }
                                }
                                if mathexit {
                                    mathexit = false;
                                    open = true;
                                } else {
                                    mathexit = true;
                                }
                            }
                            if open {
                                depth -= 1;
                                if depth == 0 {
                                    break 'done2;
                                }
                                if depth < min_depth {
                                    return Err(self.scan_error(
                                        "Containing expression ends prematurely",
                                        last_good,
                                        from,
                                    ));
                                }
                            } else {
                                depth += 1;
                                if depth == 0 {
                                    break 'done2;
                                }
                            }
                        }
                        S_ENDCOMMENT => {
                            if !self.syntax_var_p(sym::PARSE_SEXP_IGNORE_COMMENTS) {
                                continue;
                            }
                            let (found, out) =
                                self.back_comment(from, stop, comnested, comstyle)?;
                            if found {
                                from = out;
                            }
                        }
                        S_COMMENT_FENCE | S_STRING_FENCE => {
                            loop {
                                if from == stop {
                                    return Err(self.scan_error(
                                        "Unbalanced parentheses",
                                        last_good,
                                        from,
                                    ));
                                }
                                from -= 1;
                                self.update_backward(from)?;
                                if !self.char_quoted(from)? {
                                    let c = self.fetch_mb(from);
                                    if self.syntax_multibyte(c, multibyte_symbol_p) == code {
                                        break;
                                    }
                                }
                            }
                            if code == S_STRING_FENCE && depth == 0 && sexpflag {
                                break 'done2;
                            }
                        }
                        S_STRING => {
                            let stringterm = self.fetch_mb(from);
                            loop {
                                if from == stop {
                                    return Err(self.scan_error(
                                        "Unbalanced parentheses",
                                        last_good,
                                        from,
                                    ));
                                }
                                from -= 1;
                                self.update_backward(from)?;
                                if !self.char_quoted(from)? {
                                    let c = self.fetch_mb(from);
                                    if c == stringterm
                                        && self.syntax_multibyte(c, multibyte_symbol_p) == S_STRING
                                    {
                                        break;
                                    }
                                }
                            }
                            if depth == 0 && sexpflag {
                                break 'done2;
                            }
                        }
                        _ => {}
                    }
                }
                if depth != 0 {
                    return Err(self.scan_error("Unbalanced parentheses", last_good, from));
                }
                return Ok(Value::NIL);
            }
            count += 1;
        }
        Ok(Value::fixnum(from))
    }

    fn inc_from(&mut self, k: &mut Cursor) -> LispResult<()> {
        k.prev_from = k.from;
        let temp = self.fetch_mb(k.prev_from);
        k.prev_prev_from_syntax = k.prev_from_syntax;
        k.prev_from_syntax = self.gl_syntax_with_flags(temp);
        k.from += 1;
        if k.from < k.end {
            self.update_forward(k.from)?;
        }
        Ok(())
    }

    fn in_2char_comment_start(
        &self,
        state: &mut ParseState,
        prev_from_syntax: i32,
        prev_from: i64,
        from: i64,
    ) -> bool {
        if comstart_first(prev_from_syntax) {
            let c1 = self.fetch_mb(from);
            let syntax = self.gl_syntax_with_flags(c1);
            if comstart_second(syntax) {
                state.comstyle = comment_style(syntax, prev_from_syntax);
                let comnested = comment_nested(prev_from_syntax) || comment_nested(syntax);
                state.incomment = if comnested { 1 } else { -1 };
                state.comstr_start = prev_from;
                return true;
            }
        }
        false
    }

    fn scan_sexps_forward(
        &mut self,
        state: &mut ParseState,
        from: i64,
        end: i64,
        targetdepth: i64,
        stopbefore: bool,
        commentstop: i32,
    ) -> LispResult<()> {
        let boundary_stop = commentstop == -1;
        let begv = self.cur().begv as i64;
        let mut k = Cursor {
            from,
            prev_from: from,
            prev_from_syntax: state.prev_syntax,
            prev_prev_from_syntax: SMAX,
            end,
        };
        if from != begv {
            k.prev_from -= 1;
        }
        let mut depth = state.depth;
        let start_quoted = state.quoted;
        let mut levels = [(-1i64, -1i64); 100];
        let mut cur = 0usize;
        let mut tem = state.levelstarts;
        while !tem.is_nil() {
            let temhd = self.car(tem)?;
            if let Some(n) = temhd.as_fixnum() {
                levels[cur].0 = n;
            }
            cur += 1;
            if cur == levels.len() {
                cur -= 1;
            }
            levels[cur] = (-1, -1);
            tem = self.cdr(tem)?;
        }
        levels[cur] = (-1, -1);
        state.quoted = false;
        let mut mindepth = depth;
        self.setup_syntax_table(from, 1)?;
        let mut nofence = false;
        let mut pc = if state.incomment != 0 {
            Pc::StartInComment
        } else if state.instring >= 0 {
            nofence = state.instring != ST_STRING_STYLE;
            if start_quoted {
                Pc::StartQuotedInString
            } else {
                Pc::StartInString
            }
        } else if start_quoted {
            Pc::StartQuoted
        } else if k.from < end
            && self.in_2char_comment_start(state, k.prev_from_syntax, k.prev_from, k.from)
        {
            self.inc_from(&mut k)?;
            k.prev_from_syntax = SMAX;
            Pc::AtComment
        } else {
            Pc::Loop
        };
        loop {
            match pc {
                Pc::Loop => {
                    if k.from >= end {
                        pc = Pc::Done;
                        continue;
                    }
                    self.inc_from(&mut k)?;
                    if k.from < end
                        && self.in_2char_comment_start(
                            state,
                            k.prev_from_syntax,
                            k.prev_from,
                            k.from,
                        )
                    {
                        self.inc_from(&mut k)?;
                        k.prev_from_syntax = SMAX;
                        pc = Pc::AtComment;
                        continue;
                    }
                    if prefix_flag(k.prev_from_syntax) {
                        continue;
                    }
                    let code = code_of(k.prev_from_syntax);
                    match code {
                        S_ESCAPE | S_CHARQUOTE => {
                            if stopbefore {
                                pc = Pc::Stop;
                                continue;
                            }
                            levels[cur].0 = k.prev_from;
                            pc = Pc::StartQuoted;
                        }
                        S_WORD | S_SYMBOL => {
                            if stopbefore {
                                pc = Pc::Stop;
                                continue;
                            }
                            levels[cur].0 = k.prev_from;
                            pc = Pc::SymStarted;
                        }
                        S_COMMENT_FENCE => {
                            state.comstyle = ST_COMMENT_STYLE;
                            state.incomment = -1;
                            state.comstr_start = k.prev_from;
                            pc = Pc::AtComment;
                        }
                        S_COMMENT => {
                            state.comstyle = comment_style(k.prev_from_syntax, 0);
                            state.incomment = if comment_nested(k.prev_from_syntax) {
                                1
                            } else {
                                -1
                            };
                            state.comstr_start = k.prev_from;
                            pc = Pc::AtComment;
                        }
                        S_OPEN => {
                            if stopbefore {
                                pc = Pc::Stop;
                                continue;
                            }
                            depth += 1;
                            levels[cur].0 = k.prev_from;
                            cur += 1;
                            if cur == levels.len() {
                                cur -= 1;
                            }
                            levels[cur] = (-1, -1);
                            if targetdepth == depth {
                                pc = Pc::Done;
                            }
                        }
                        S_CLOSE => {
                            depth -= 1;
                            if depth < mindepth {
                                mindepth = depth;
                            }
                            cur = cur.saturating_sub(1);
                            levels[cur].1 = levels[cur].0;
                            if targetdepth == depth {
                                pc = Pc::Done;
                            }
                        }
                        S_STRING | S_STRING_FENCE => {
                            state.comstr_start = k.from - 1;
                            if stopbefore {
                                pc = Pc::Stop;
                                continue;
                            }
                            levels[cur].0 = k.prev_from;
                            state.instring = if code == S_STRING {
                                self.fetch_mb(k.prev_from) as i32
                            } else {
                                ST_STRING_STYLE
                            };
                            pc = if boundary_stop {
                                Pc::Done
                            } else {
                                Pc::StartInString
                            };
                        }
                        _ => {}
                    }
                }
                Pc::StartQuoted => {
                    if k.from == end {
                        pc = Pc::EndQuoted;
                        continue;
                    }
                    self.inc_from(&mut k)?;
                    pc = Pc::SymStarted;
                }
                Pc::SymStarted => {
                    pc = 'sym: {
                        while k.from < end {
                            if self.in_2char_comment_start(
                                state,
                                k.prev_from_syntax,
                                k.prev_from,
                                k.from,
                            ) {
                                self.inc_from(&mut k)?;
                                k.prev_from_syntax = SMAX;
                                break 'sym Pc::AtComment;
                            }
                            let symchar = self.fetch_mb(k.from);
                            match self.gl_syntax(symchar) {
                                S_CHARQUOTE | S_ESCAPE => {
                                    self.inc_from(&mut k)?;
                                    if k.from == end {
                                        break 'sym Pc::EndQuoted;
                                    }
                                }
                                S_WORD | S_SYMBOL | S_QUOTE => {}
                                _ => break,
                            }
                            self.inc_from(&mut k)?;
                        }
                        levels[cur].1 = levels[cur].0;
                        Pc::Loop
                    };
                }
                Pc::AtComment => {
                    pc = if commentstop != 0 || boundary_stop {
                        Pc::Done
                    } else {
                        Pc::StartInComment
                    };
                }
                Pc::StartInComment => {
                    let prev = if k.from == begv {
                        0
                    } else {
                        k.prev_from_syntax
                    };
                    let (found, out, incomment, last_syntax) =
                        self.forw_comment(k.from, end, state.incomment, state.comstyle, prev)?;
                    k.from = out;
                    k.prev_from_syntax = last_syntax;
                    if !found {
                        state.incomment = incomment;
                        let code = last_syntax & 0xff;
                        pc = if code == S_ESCAPE as i32 || code == S_CHARQUOTE as i32 {
                            Pc::EndQuoted
                        } else {
                            Pc::Done
                        };
                        continue;
                    }
                    self.inc_from(&mut k)?;
                    state.incomment = 0;
                    state.comstyle = 0;
                    k.prev_from_syntax = SMAX;
                    pc = if boundary_stop { Pc::Done } else { Pc::Loop };
                }
                Pc::StartInString | Pc::StartQuotedInString => {
                    let mut quoted_entry = pc == Pc::StartQuotedInString;
                    if !quoted_entry {
                        nofence = state.instring != ST_STRING_STYLE;
                    }
                    pc = 'string: {
                        loop {
                            if !quoted_entry {
                                if k.from >= end {
                                    break 'string Pc::Done;
                                }
                                let c = self.fetch_mb(k.from);
                                let c_code = self.gl_syntax(c);
                                if nofence && c as i32 == state.instring && c_code == S_STRING {
                                    break;
                                }
                                match c_code {
                                    S_STRING_FENCE => {
                                        if !nofence {
                                            break;
                                        }
                                    }
                                    S_CHARQUOTE | S_ESCAPE => {
                                        self.inc_from(&mut k)?;
                                        quoted_entry = true;
                                    }
                                    _ => {}
                                }
                            }
                            if quoted_entry {
                                quoted_entry = false;
                                if k.from >= end {
                                    break 'string Pc::EndQuoted;
                                }
                            }
                            self.inc_from(&mut k)?;
                        }
                        state.instring = -1;
                        levels[cur].1 = levels[cur].0;
                        self.inc_from(&mut k)?;
                        if boundary_stop { Pc::Done } else { Pc::Loop }
                    };
                }
                Pc::Stop => {
                    k.from = k.prev_from;
                    k.prev_from_syntax = k.prev_prev_from_syntax;
                    pc = Pc::Done;
                }
                Pc::EndQuoted => {
                    state.quoted = true;
                    pc = Pc::Done;
                }
                Pc::Done => break,
            }
        }
        state.depth = depth;
        state.mindepth = mindepth;
        state.thislevelstart = levels[cur].1;
        state.prevlevelstart = if cur == 0 { -1 } else { levels[cur - 1].0 };
        state.location = k.from;
        let mut levelstarts = Value::NIL;
        while cur > 0 {
            cur -= 1;
            levelstarts = self.cons(Value::fixnum(levels[cur].0), levelstarts);
        }
        state.levelstarts = levelstarts;
        state.prev_syntax = if comstartend_first(k.prev_from_syntax) || state.quoted {
            k.prev_from_syntax
        } else {
            SMAX
        };
        Ok(())
    }

    fn internalize_parse_state(&mut self, external: Value) -> LispResult<ParseState> {
        let mut state = ParseState::default();
        if external.is_nil() {
            return Ok(state);
        }
        let mut external = external;
        let tem = self.car(external)?;
        state.depth = tem.as_fixnum().unwrap_or(0);
        for _ in 0..3 {
            external = self.cdr(external)?;
        }
        let tem = self.car(external)?;
        state.instring = if tem.is_nil() {
            -1
        } else {
            match tem.as_fixnum() {
                Some(n) if (0..=MAX_CHAR as i64).contains(&n) => n as i32,
                _ => ST_STRING_STYLE,
            }
        };
        external = self.cdr(external)?;
        let tem = self.car(external)?;
        state.incomment = if tem.is_nil() {
            0
        } else {
            tem.as_fixnum().unwrap_or(-1)
        };
        external = self.cdr(external)?;
        let tem = self.car(external)?;
        state.quoted = !tem.is_nil();
        external = self.cdr(external)?;
        external = self.cdr(external)?;
        let tem = self.car(external)?;
        state.comstyle = if tem.is_nil() {
            0
        } else {
            match tem.as_fixnum() {
                Some(n) if (0..=ST_COMMENT_STYLE as i64).contains(&n) => n as i32,
                _ => ST_COMMENT_STYLE,
            }
        };
        external = self.cdr(external)?;
        let tem = self.car(external)?;
        state.comstr_start = tem.as_fixnum().unwrap_or(-1);
        external = self.cdr(external)?;
        state.levelstarts = self.car(external)?;
        external = self.cdr(external)?;
        let tem = self.car(external)?;
        state.prev_syntax = if tem.is_nil() {
            SMAX
        } else {
            tem.as_fixnum().unwrap_or(0) as i32
        };
        Ok(state)
    }
}

fn f_forward_word(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let arg = if args[0].is_nil() {
        1
    } else {
        lisp.check_fixnum(args[0])?
    };
    let pt = lisp.cur().pt as i64;
    let orig_val = lisp.scan_words(pt, arg)?;
    let mut val = orig_val;
    if orig_val == 0 {
        val = if arg > 0 {
            lisp.cur().zv as i64
        } else {
            lisp.cur().begv as i64
        };
    }
    let pt = lisp.cur().pt as i64;
    let tmp = f_constrain_to_field(
        lisp,
        &[
            Value::fixnum(val),
            Value::fixnum(pt),
            Value::NIL,
            Value::NIL,
            Value::NIL,
        ],
    )?;
    let val = tmp.as_fixnum().unwrap_or(val);
    lisp.goto_char_internal(val as usize);
    Ok(lisp.bool(val == orig_val))
}

fn skip_limit(lisp: &mut Lisp, forwardp: bool, lim: Value) -> LispResult<i64> {
    let (begv, zv) = (lisp.cur().begv as i64, lisp.cur().zv as i64);
    let mut lim = if lim.is_nil() {
        if forwardp { zv } else { begv }
    } else {
        lisp.position_arg(lim)?
    };
    if lim > zv {
        lim = zv;
    }
    if lim < begv {
        lim = begv;
    }
    Ok(lim)
}

fn region_multibyte(lisp: &Lisp, lim: i64) -> bool {
    let b = lisp.cur();
    b.text.multibyte
        && (lim - b.pt as i64 != b.text.char_to_byte(lim as usize) as i64 - b.pt_byte as i64)
}

fn skip_chars(lisp: &mut Lisp, forwardp: bool, string: Value, lim: Value) -> LispResult {
    lisp.check_string(string)?;
    let lim = skip_limit(lisp, forwardp, lim)?;
    let multibyte = region_multibyte(lisp, lim);
    let string_multibyte = lisp.string_bytes(string).len() > lisp.string_chars(string);
    let str = lisp.string_bytes(string).to_vec();
    let size_byte = str.len();
    let mut fastmap = [false; 256];
    let mut char_ranges: Vec<(u32, u32)> = Vec::new();
    let mut negate = false;
    let mut i_byte = 0;
    if i_byte < size_byte && str[0] == b'^' {
        negate = true;
        i_byte += 1;
    }
    let mut classes: Vec<i32> = Vec::new();
    if !string_multibyte {
        let mut string_has_eight_bit = false;
        while i_byte < size_byte {
            let mut ch = i_byte;
            let cc = wctype_parse(&str, &mut ch);
            if cc == 0 {
                return Err(lisp.error("Invalid ISO C character class"));
            }
            if cc != -1 {
                if !classes.contains(&cc) {
                    classes.push(cc);
                }
                i_byte = ch;
                continue;
            }
            let mut c = str[i_byte] as usize;
            i_byte += 1;
            if c == b'\\' as usize {
                if i_byte == size_byte {
                    break;
                }
                c = str[i_byte] as usize;
                i_byte += 1;
            }
            if i_byte + 1 < size_byte && str[i_byte] == b'-' {
                i_byte += 1;
                let mut c2 = str[i_byte] as usize;
                i_byte += 1;
                if c2 == b'\\' as usize && i_byte < size_byte {
                    c2 = str[i_byte] as usize;
                    i_byte += 1;
                }
                if c <= c2 {
                    for slot in &mut fastmap[c..=c2] {
                        *slot = true;
                    }
                    if c2 >= 0x80 {
                        string_has_eight_bit = true;
                    }
                }
            } else {
                fastmap[c] = true;
                if c >= 0x80 {
                    string_has_eight_bit = true;
                }
            }
        }
        if multibyte && string_has_eight_bit {
            let mut himap = [false; 0x81];
            himap[..0x80].copy_from_slice(&fastmap[0x80..]);
            for slot in &mut fastmap[0x80..] {
                *slot = false;
            }
            let mut i = 0;
            while let Some(off) = himap[i..0x80].iter().position(|&b| b) {
                i += off;
                let c = character::byte8_to_char((i + 0x80) as u8);
                i += himap[i..].iter().take_while(|&&b| b).count();
                let c2 = character::byte8_to_char((i + 0x80 - 1) as u8);
                char_ranges.push((c, c2));
                let leading_code = char_leading_code(c);
                for slot in &mut fastmap[leading_code..=char_leading_code(c2)] {
                    *slot = true;
                }
            }
        }
    } else {
        while i_byte < size_byte {
            let mut leading_code = str[i_byte] as usize;
            let mut ch = i_byte;
            let cc = wctype_parse(&str, &mut ch);
            if cc == 0 {
                return Err(lisp.error("Invalid ISO C character class"));
            }
            if cc != -1 {
                if !classes.contains(&cc) {
                    classes.push(cc);
                }
                i_byte = ch;
                continue;
            }
            if leading_code == b'\\' as usize {
                i_byte += 1;
                if i_byte == size_byte {
                    break;
                }
                leading_code = str[i_byte] as usize;
            }
            let (mut c, len) = character::string_char(&str[i_byte..]);
            i_byte += len;
            if i_byte + 1 < size_byte && str[i_byte] == b'-' {
                i_byte += 1;
                let mut leading_code2 = str[i_byte] as usize;
                let (mut c2, len) = character::string_char(&str[i_byte..]);
                i_byte += len;
                if c2 == b'\\' as u32 && i_byte < size_byte {
                    leading_code2 = str[i_byte] as usize;
                    let (cc2, len) = character::string_char(&str[i_byte..]);
                    c2 = cc2;
                    i_byte += len;
                }
                if c > c2 {
                    continue;
                }
                if character::ascii_char_p(c) {
                    while c <= c2 && c < 0x80 {
                        fastmap[c as usize] = true;
                        c += 1;
                    }
                    leading_code = char_leading_code(c);
                }
                if !character::ascii_char_p(c) {
                    while leading_code <= leading_code2 {
                        fastmap[leading_code] = true;
                        leading_code += 1;
                    }
                    if c <= c2 {
                        char_ranges.push((c, c2));
                    }
                }
            } else if character::ascii_char_p(c) {
                fastmap[c as usize] = true;
            } else {
                fastmap[leading_code] = true;
                char_ranges.push((c, c));
            }
        }
        if !multibyte && !char_ranges.is_empty() {
            for slot in &mut fastmap[0x80..] {
                *slot = false;
            }
            for &(c1, c2) in &char_ranges {
                for c in c1..=c2 {
                    if character::ascii_char_p(c) {
                        fastmap[c as usize] = true;
                    } else if character::char_byte8_p(c) {
                        fastmap[character::char_to_byte8(c) as usize] = true;
                    }
                }
            }
        }
    }
    if negate {
        if !multibyte {
            for slot in &mut fastmap {
                *slot = !*slot;
            }
        } else {
            for slot in &mut fastmap[..0x80] {
                *slot = !*slot;
            }
            for slot in &mut fastmap[0x80..] {
                *slot = true;
            }
        }
    }
    let start_point = lisp.cur().pt as i64;
    let mut pos = start_point;
    lisp.setup_buffer_syntax_table();
    let in_ranges = |c: u32| char_ranges.iter().any(|&(lo, hi)| c >= lo && c <= hi);
    {
        let env = lisp.regex_env(None);
        let in_classes = |c: u32| classes.iter().any(|&cc| env.iswctype(c, cc));
        let text = &lisp.cur().text;
        if forwardp {
            while pos < lim {
                let byte_pos = text.char_to_byte(pos as usize);
                let first = text.byte_at(byte_pos);
                let c = if multibyte {
                    text.char_at_byte(byte_pos)
                } else {
                    first as u32
                };
                if !classes.is_empty() && in_classes(c) {
                    if negate {
                        break;
                    }
                    pos += 1;
                    continue;
                }
                if !fastmap[first as usize] {
                    break;
                }
                if multibyte && !character::ascii_char_p(c) && !(negate ^ in_ranges(c)) {
                    break;
                }
                pos += 1;
            }
        } else {
            while pos > lim {
                let byte_pos = text.char_to_byte(pos as usize - 1);
                let first = text.byte_at(byte_pos);
                let c = if multibyte {
                    text.char_at_byte(byte_pos)
                } else {
                    first as u32
                };
                if !classes.is_empty() && in_classes(c) {
                    if negate {
                        break;
                    }
                    pos -= 1;
                    continue;
                }
                if !fastmap[first as usize] {
                    break;
                }
                if multibyte && !character::ascii_char_p(c) && !(negate ^ in_ranges(c)) {
                    break;
                }
                pos -= 1;
            }
        }
    }
    lisp.goto_char_internal(pos as usize);
    Ok(Value::fixnum(lisp.cur().pt as i64 - start_point))
}

fn skip_syntaxes(lisp: &mut Lisp, forwardp: bool, string: Value, lim: Value) -> LispResult {
    lisp.check_string(string)?;
    let lim = skip_limit(lisp, forwardp, lim)?;
    let pt = lisp.cur().pt as i64;
    if if forwardp { pt >= lim } else { pt <= lim } {
        return Ok(Value::fixnum(0));
    }
    let str: Vec<u8> = if lisp.string_bytes(string).len() > lisp.string_chars(string) {
        lisp.string_char_vec(string)
            .iter()
            .map(|&c| (c & 0xff) as u8)
            .collect()
    } else {
        lisp.string_bytes(string).to_vec()
    };
    let mut fastmap = [false; 256];
    let mut negate = false;
    let mut i_byte = 0;
    if i_byte < str.len() && str[0] == b'^' {
        negate = true;
        i_byte += 1;
    }
    for &c in &str[i_byte..] {
        fastmap[syntax_spec_code(c) as usize] = true;
    }
    if negate {
        for slot in &mut fastmap {
            *slot = !*slot;
        }
    }
    let start_point = pt;
    let mut pos = pt;
    lisp.setup_syntax_table(pos, if forwardp { 1 } else { -1 })?;
    if forwardp {
        'done: loop {
            loop {
                if pos >= lim {
                    break 'done;
                }
                let c = lisp.fetch_raw(pos);
                if !fastmap[lisp.gl_syntax(c) as usize] {
                    break 'done;
                }
                pos += 1;
                if lisp.lookup_properties() && pos >= lisp.syntax.gl.e_property {
                    break;
                }
            }
            let object = lisp.syntax.gl.object;
            lisp.update_syntax_table_forward(pos, object)?;
        }
    } else {
        while pos > lim {
            lisp.update_backward(pos - 1)?;
            let c = lisp.fetch_raw(pos - 1);
            if !fastmap[lisp.gl_syntax(c) as usize] {
                break;
            }
            pos -= 1;
        }
    }
    lisp.goto_char_internal(pos as usize);
    Ok(Value::fixnum(lisp.cur().pt as i64 - start_point))
}

fn f_skip_chars_forward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    skip_chars(lisp, true, args[0], args[1])
}

fn f_skip_chars_backward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    skip_chars(lisp, false, args[0], args[1])
}

fn f_skip_syntax_forward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    skip_syntaxes(lisp, true, args[0], args[1])
}

fn f_skip_syntax_backward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    skip_syntaxes(lisp, false, args[0], args[1])
}

fn f_forward_comment(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let count = args[0];
    let function = lisp.plain_value(sym::FORWARD_COMMENT_FUNCTION);
    if !function.is_nil() {
        return lisp.funcall(function, &[count]);
    }
    let mut count1 = lisp.check_fixnum(count)?;
    let (begv, zv) = (lisp.cur().begv as i64, lisp.cur().zv as i64);
    let stop = if count1 > 0 { zv } else { begv };
    let mut from = lisp.cur().pt as i64;
    lisp.setup_syntax_table(from, count1)?;
    while count1 > 0 {
        let mut code;
        let mut comstyle;
        let mut comnested;
        loop {
            if from == stop {
                lisp.goto_char_internal(from as usize);
                return Ok(Value::NIL);
            }
            let c = lisp.fetch_mb(from);
            let syntax = lisp.gl_syntax_with_flags(c);
            code = lisp.gl_syntax(c);
            let comstart = comstart_first(syntax);
            comnested = comment_nested(syntax);
            comstyle = comment_style(syntax, 0);
            from += 1;
            lisp.update_forward(from)?;
            if from < stop && comstart {
                let c1 = lisp.fetch_mb(from);
                let other_syntax = lisp.gl_syntax_with_flags(c1);
                if comstart_second(other_syntax) {
                    code = S_COMMENT;
                    comstyle = comment_style(other_syntax, syntax);
                    comnested |= comment_nested(other_syntax);
                    from += 1;
                    lisp.update_forward(from)?;
                }
            }
            if !(code == S_WHITESPACE || (code == S_ENDCOMMENT && c == b'\n' as u32)) {
                break;
            }
        }
        if code == S_COMMENT_FENCE {
            comstyle = ST_COMMENT_STYLE;
        } else if code != S_COMMENT {
            from -= 1;
            lisp.goto_char_internal(from as usize);
            return Ok(Value::NIL);
        }
        let (found, out, _, _) = lisp.forw_comment(from, stop, comnested as i64, comstyle, 0)?;
        from = out;
        if !found {
            lisp.goto_char_internal(from as usize);
            return Ok(Value::NIL);
        }
        from += 1;
        lisp.update_forward(from)?;
        count1 -= 1;
    }
    while count1 < 0 {
        loop {
            if from <= stop {
                lisp.goto_char_internal(begv as usize);
                return Ok(Value::NIL);
            }
            from -= 1;
            let quoted = lisp.char_quoted(from)?;
            let c = lisp.fetch_mb(from);
            let syntax = lisp.gl_syntax_with_flags(c);
            let mut code = lisp.gl_syntax(c);
            let mut comstyle = 0;
            let mut comnested = comment_nested(syntax);
            if code == S_ENDCOMMENT {
                comstyle = comment_style(syntax, 0);
            }
            if from > stop
                && comend_second(syntax)
                && lisp.prev_char_comend_first(from)?
                && !lisp.char_quoted(from - 1)?
            {
                from -= 1;
                code = S_ENDCOMMENT;
                let c1 = lisp.fetch_mb(from);
                let other_syntax = lisp.gl_syntax_with_flags(c1);
                comstyle = comment_style(other_syntax, syntax);
                comnested |= comment_nested(other_syntax);
            }
            if code == S_COMMENT_FENCE {
                let mut fence_found = false;
                let ini = from;
                if from > stop {
                    loop {
                        from -= 1;
                        lisp.update_backward(from)?;
                        let c = lisp.fetch_mb(from);
                        if lisp.gl_syntax(c) == S_COMMENT_FENCE && !lisp.char_quoted(from)? {
                            fence_found = true;
                            break;
                        } else if from == stop {
                            break;
                        }
                    }
                }
                if !fence_found {
                    from = ini + 1;
                    lisp.goto_char_internal(from as usize);
                    return Ok(Value::NIL);
                }
                break;
            } else if code == S_ENDCOMMENT {
                let mut found = false;
                if !quoted || !lisp.syntax_var_p(sym::COMMENT_END_CAN_BE_ESCAPED) {
                    let (f, out) = lisp.back_comment(from, stop, comnested, comstyle)?;
                    found = f;
                    if found {
                        from = out;
                    }
                }
                if found {
                    break;
                }
                if c != b'\n' as u32 {
                    if lisp.gl_syntax(c) != code {
                        from += 1;
                    }
                    from += 1;
                    lisp.goto_char_internal(from as usize);
                    return Ok(Value::NIL);
                }
            } else if code != S_WHITESPACE || quoted {
                from += 1;
                lisp.goto_char_internal(from as usize);
                return Ok(Value::NIL);
            }
        }
        count1 += 1;
    }
    lisp.goto_char_internal(from as usize);
    Ok(sym::T)
}

fn f_scan_lists(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let from = lisp.check_fixnum(args[0])?;
    let count = lisp.check_fixnum(args[1])?;
    let depth = lisp.check_fixnum(args[2])?;
    lisp.scan_lists(from, count, depth, false)
}

fn f_scan_sexps(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let from = lisp.check_fixnum(args[0])?;
    let count = lisp.check_fixnum(args[1])?;
    lisp.scan_lists(from, count, 0, true)
}

fn f_backward_prefix_chars(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let beg = lisp.cur().begv as i64;
    let mut opoint = lisp.cur().pt as i64;
    let mut pos = opoint;
    if pos <= beg {
        lisp.goto_char_internal(opoint as usize);
        return Ok(Value::NIL);
    }
    lisp.setup_syntax_table(pos, -1)?;
    pos -= 1;
    loop {
        if lisp.char_quoted(pos)? {
            break;
        }
        let c = lisp.fetch_mb(pos);
        if !(lisp.gl_syntax(c) == S_QUOTE || prefix_flag(lisp.gl_syntax_with_flags(c))) {
            break;
        }
        opoint = pos;
        if pos <= beg {
            break;
        }
        pos -= 1;
    }
    lisp.goto_char_internal(opoint as usize);
    Ok(Value::NIL)
}

fn f_parse_partial_sexp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (from, to, targetdepth, stopbefore, oldstate, commentstop) =
        (args[0], args[1], args[2], args[3], args[4], args[5]);
    let target = if targetdepth.is_nil() {
        i64::MIN
    } else {
        lisp.check_fixnum(targetdepth)?
    };
    if lisp.fix_position(to)? < lisp.fix_position(from)? {
        return Err(lisp.error("End position is smaller than start position"));
    }
    let (start, end) = lisp.validate_region(from, to)?;
    let mut state = lisp.internalize_parse_state(oldstate)?;
    let commentstop = if commentstop.is_nil() {
        0
    } else if commentstop == sym::SYNTAX_TABLE {
        -1
    } else {
        1
    };
    lisp.scan_sexps_forward(
        &mut state,
        start as i64,
        end as i64,
        target,
        !stopbefore.is_nil(),
        commentstop,
    )?;
    lisp.goto_char_internal(state.location as usize);
    let opt = |n: i64| {
        if n < 0 { Value::NIL } else { Value::fixnum(n) }
    };
    let instring = if state.instring >= 0 {
        if state.instring == ST_STRING_STYLE {
            sym::T
        } else {
            Value::fixnum(state.instring as i64)
        }
    } else {
        Value::NIL
    };
    let incomment = if state.incomment < 0 {
        sym::T
    } else if state.incomment == 0 {
        Value::NIL
    } else {
        Value::fixnum(state.incomment)
    };
    let comstyle = if state.comstyle == 0 {
        Value::NIL
    } else if state.comstyle == ST_COMMENT_STYLE {
        sym::SYNTAX_TABLE
    } else {
        Value::fixnum(state.comstyle as i64)
    };
    let comstr_start = if state.incomment != 0 || state.instring >= 0 {
        Value::fixnum(state.comstr_start)
    } else {
        Value::NIL
    };
    let prev_syntax = if state.prev_syntax == SMAX {
        Value::NIL
    } else {
        Value::fixnum(state.prev_syntax as i64)
    };
    let quoted = lisp.bool(state.quoted);
    Ok(lisp.list(&[
        Value::fixnum(state.depth),
        opt(state.prevlevelstart),
        opt(state.thislevelstart),
        instring,
        incomment,
        quoted,
        Value::fixnum(state.mindepth),
        comstyle,
        comstr_start,
        state.levelstarts,
        prev_syntax,
    ]))
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
