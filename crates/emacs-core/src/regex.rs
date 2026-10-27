use crate::character::{self, MAX_CHAR};
use crate::lisp::Lisp;
use crate::syntax::{S_SYMBOL, S_WHITESPACE, S_WORD, syntax_spec_code};
use crate::value::Value;

const NO_OP: u8 = 0;
const SUCCEED: u8 = 1;
const EXACTN: u8 = 2;
const ANYCHAR: u8 = 3;
const CHARSET: u8 = 4;
const CHARSET_NOT: u8 = 5;
const START_MEMORY: u8 = 6;
const STOP_MEMORY: u8 = 7;
const DUPLICATE: u8 = 8;
const BEGLINE: u8 = 9;
const ENDLINE: u8 = 10;
const BEGBUF: u8 = 11;
const ENDBUF: u8 = 12;
const JUMP: u8 = 13;
const ON_FAILURE_JUMP_MEMO: u8 = 14;
const ON_FAILURE_JUMP_NOMEMO: u8 = 15;
const ON_FAILURE_JUMP_PREMEMO: u8 = 16;
const ON_FAILURE_KEEP_STRING_JUMP: u8 = 17;
const ON_FAILURE_JUMP_LOOP: u8 = 18;
const ON_FAILURE_JUMP_NASTYLOOP: u8 = 19;
const ON_FAILURE_JUMP_SMART: u8 = 20;
const SUCCEED_N: u8 = 21;
const JUMP_N: u8 = 22;
const SET_NUMBER_AT: u8 = 23;
const WORDBEG: u8 = 24;
const WORDEND: u8 = 25;
const WORDBOUND: u8 = 26;
const NOTWORDBOUND: u8 = 27;
const SYMBEG: u8 = 28;
const SYMEND: u8 = 29;
const SYNTAXSPEC: u8 = 30;
const NOTSYNTAXSPEC: u8 = 31;
const AT_DOT: u8 = 32;
const CATEGORYSPEC: u8 = 33;
const NOTCATEGORYSPEC: u8 = 34;

const RE_DUP_MAX: i32 = 0xffff;
const MAX_BUF_SIZE: usize = 1 << 15;
const MAX_REGNUM: i32 = 255;
const RE_NREGS: usize = 30;
const MAX_FAILURE_ITEMS: usize = 40000 * 20 / 3;

const BIT_WORD: i32 = 0x1;
const BIT_LOWER: i32 = 0x2;
const BIT_PUNCT: i32 = 0x4;
const BIT_SPACE: i32 = 0x8;
const BIT_UPPER: i32 = 0x10;
const BIT_MULTIBYTE: i32 = 0x20;
const BIT_ALPHA: i32 = 0x40;
const BIT_ALNUM: i32 = 0x80;
const BIT_GRAPH: i32 = 0x100;
const BIT_PRINT: i32 = 0x200;
const BIT_BLANK: i32 = 0x400;

const RECC_ERROR: i32 = 0;
const RECC_ALNUM: i32 = 1;
const RECC_ALPHA: i32 = 2;
const RECC_WORD: i32 = 3;
const RECC_GRAPH: i32 = 4;
const RECC_PRINT: i32 = 5;
const RECC_LOWER: i32 = 6;
const RECC_UPPER: i32 = 7;
const RECC_PUNCT: i32 = 8;
const RECC_CNTRL: i32 = 9;
const RECC_DIGIT: i32 = 10;
const RECC_XDIGIT: i32 = 11;
const RECC_BLANK: i32 = 12;
const RECC_SPACE: i32 = 13;
const RECC_MULTIBYTE: i32 = 14;
const RECC_NONASCII: i32 = 15;
const RECC_ASCII: i32 = 16;
const RECC_UNIBYTE: i32 = 17;

const UC_LU: i64 = 1;
const UC_LL: i64 = 2;
const UC_LT: i64 = 3;
const UC_LM: i64 = 4;
const UC_LO: i64 = 5;
const UC_MN: i64 = 6;
const UC_MC: i64 = 7;
const UC_ME: i64 = 8;
const UC_ND: i64 = 9;
const UC_NL: i64 = 10;
const UC_ZS: i64 = 23;
const UC_ZL: i64 = 24;
const UC_ZP: i64 = 25;
const UC_CC: i64 = 26;
const UC_CS: i64 = 28;
const UC_CN: i64 = 30;

pub const REG_BADPAT: &str = "Invalid regular expression";
const REG_ECTYPE: &str = "Invalid character class name";
const REG_EESCAPE: &str = "Trailing backslash";
const REG_ESUBREG: &str = "Invalid back reference";
const REG_EBRACK: &str = "Unmatched [ or [^";
const REG_EPAREN: &str = "Unmatched ( or \\(";
const REG_EBRACE: &str = "Unmatched \\{";
const REG_BADBR: &str = "Invalid content of \\{\\}";
const REG_EEND: &str = "Premature end of regular expression";
const REG_ESIZE: &str = "Regular expression too big";
const REG_ERPAREN: &str = "Unmatched ) or \\)";
const REG_ESIZEBR: &str = "Invalid content of \\{\\}";

#[derive(Clone, Debug)]
pub struct Pattern {
    pub buffer: Vec<u8>,
    pub re_nsub: usize,
    pub multibyte: bool,
    pub target_multibyte: bool,
    pub used_syntax: bool,
    pub can_be_null: bool,
    pub fastmap: Vec<bool>,
    pub fastmap_accurate: bool,
    pub translate: Value,
}

impl Default for Pattern {
    fn default() -> Self {
        Pattern {
            buffer: Vec::new(),
            re_nsub: 0,
            multibyte: false,
            target_multibyte: false,
            used_syntax: false,
            can_be_null: false,
            fastmap: vec![false; 256],
            fastmap_accurate: false,
            translate: Value::NIL,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Registers {
    pub start: Vec<isize>,
    pub end: Vec<isize>,
}

pub struct Env<'a> {
    pub lisp: &'a Lisp,
    pub syntax_table: Value,
    pub category_table: Value,
    pub downcase_table: Value,
    pub upcase_table: Value,
    pub unicode_category_table: Value,
    pub char_script_table: Value,
    pub word_combining: Value,
    pub word_separating: Value,
    pub point: Option<usize>,
}

impl Env<'_> {
    fn table_ref(&self, table: Value, c: u32) -> Value {
        if table.is_vectorlike() && self.lisp.is_char_table(table) {
            self.lisp.char_table_ref(table, c)
        } else {
            Value::NIL
        }
    }

    pub fn syntax(&self, c: u32) -> u8 {
        let entry = self.table_ref(self.syntax_table, c);
        if entry.is_cons() {
            (self.lisp.xcar(entry).as_fixnum().unwrap_or(0) & 0xff) as u8
        } else {
            S_WHITESPACE
        }
    }

    pub fn translate(&self, table: Value, c: u32) -> u32 {
        if table.is_nil() {
            return c;
        }
        match self.table_ref(table, c).as_fixnum() {
            Some(n) if (0..=MAX_CHAR as i64).contains(&n) => n as u32,
            _ => c,
        }
    }

    fn downcase(&self, c: u32) -> u32 {
        match self.table_ref(self.downcase_table, c).as_fixnum() {
            Some(n) if n >= 0 => n as u32,
            _ => c,
        }
    }

    fn upcase(&self, c: u32) -> u32 {
        match self.table_ref(self.upcase_table, c).as_fixnum() {
            Some(n) if n >= 0 => n as u32,
            _ => c,
        }
    }

    pub fn uppercasep(&self, c: u32) -> bool {
        self.downcase(c) != c
    }

    pub fn lowercasep(&self, c: u32) -> bool {
        !self.uppercasep(c) && self.upcase(c) != c
    }

    fn general_category(&self, c: u32) -> Option<i64> {
        self.table_ref(self.unicode_category_table, c).as_fixnum()
    }

    fn alphabeticp(&self, c: u32) -> bool {
        matches!(
            self.general_category(c),
            Some(UC_LU | UC_LL | UC_LT | UC_LM | UC_LO | UC_MN | UC_MC | UC_ME | UC_NL)
        )
    }

    fn alphanumericp(&self, c: u32) -> bool {
        matches!(
            self.general_category(c),
            Some(UC_LU | UC_LL | UC_LT | UC_LM | UC_LO | UC_MN | UC_MC | UC_ME | UC_NL | UC_ND)
        )
    }

    fn graphicp(&self, c: u32) -> bool {
        match self.general_category(c) {
            Some(g) => !matches!(g, UC_ZS | UC_ZL | UC_ZP | UC_CC | UC_CS | UC_CN),
            None => false,
        }
    }

    fn printablep(&self, c: u32) -> bool {
        match self.general_category(c) {
            Some(g) => !matches!(g, UC_CC | UC_CS | UC_CN),
            None => false,
        }
    }

    fn blankp(&self, c: u32) -> bool {
        self.general_category(c) == Some(UC_ZS)
    }

    fn category_member(&self, set: Value, category: u32) -> bool {
        self.lisp.bool_vector_ref(set, category as usize)
    }

    pub fn char_has_category(&self, c: u32, category: u32) -> bool {
        let set = self.table_ref(self.category_table, c);
        self.category_member(set, category)
    }

    fn word_boundary_p(&self, c1: u32, c2: u32) -> bool {
        let (mut tail, default_result) = if self.table_ref(self.char_script_table, c1)
            == self.table_ref(self.char_script_table, c2)
        {
            (self.word_separating, false)
        } else {
            (self.word_combining, true)
        };
        let set1 = self.table_ref(self.category_table, c1);
        if set1.is_nil() {
            return default_result;
        }
        let set2 = self.table_ref(self.category_table, c2);
        if set2.is_nil() {
            return default_result;
        }
        let categoryp = |v: Value| matches!(v.as_fixnum(), Some(n) if (0x20..=0x7e).contains(&n));
        while tail.is_cons() {
            let elt = self.lisp.xcar(tail);
            if elt.is_cons() {
                let a = self.lisp.xcar(elt);
                let b = self.lisp.xcdr(elt);
                let first = a.is_nil()
                    || (categoryp(a)
                        && self.category_member(set1, a.as_fixnum().unwrap_or(0) as u32)
                        && !self.category_member(set2, a.as_fixnum().unwrap_or(0) as u32));
                let second = b.is_nil()
                    || (categoryp(b)
                        && !self.category_member(set1, b.as_fixnum().unwrap_or(0) as u32)
                        && self.category_member(set2, b.as_fixnum().unwrap_or(0) as u32));
                if first && second {
                    return !default_result;
                }
            }
            tail = self.lisp.xcdr(tail);
        }
        default_result
    }

    fn word_boundary(&self, c1: u32, c2: u32) -> bool {
        !(character::single_byte_char_p(c1) && character::single_byte_char_p(c2))
            && self.word_boundary_p(c1, c2)
    }

    fn isalnum(&self, c: u32) -> bool {
        if c < 0x80 {
            (c as u8).is_ascii_alphanumeric()
        } else {
            self.alphanumericp(c)
        }
    }

    fn isalpha(&self, c: u32) -> bool {
        if c < 0x80 {
            (c as u8).is_ascii_alphabetic()
        } else {
            self.alphabeticp(c)
        }
    }

    fn isblank(&self, c: u32) -> bool {
        if c < 0x80 {
            c == b' ' as u32 || c == b'\t' as u32
        } else {
            self.blankp(c)
        }
    }

    fn isgraph(&self, c: u32) -> bool {
        if character::single_byte_char_p(c) {
            c > b' ' as u32 && !(0o177..=0o240).contains(&c)
        } else {
            self.graphicp(c)
        }
    }

    fn isprint(&self, c: u32) -> bool {
        if character::single_byte_char_p(c) {
            c >= b' ' as u32 && !(0o177..=0o237).contains(&c)
        } else {
            self.printablep(c)
        }
    }

    fn ispunct(&self, c: u32) -> bool {
        if c < 0x80 {
            c > b' ' as u32 && c < 0o177 && !(c as u8).is_ascii_alphanumeric()
        } else {
            self.syntax(c) != S_WORD
        }
    }

    fn isspace(&self, c: u32) -> bool {
        self.syntax(c) == S_WHITESPACE
    }

    fn isword(&self, c: u32) -> bool {
        self.syntax(c) == S_WORD
    }

    fn iswctype(&self, c: u32, cc: i32) -> bool {
        match cc {
            RECC_ALNUM => self.isalnum(c),
            RECC_ALPHA => self.isalpha(c),
            RECC_BLANK => self.isblank(c),
            RECC_CNTRL => c < b' ' as u32,
            RECC_DIGIT => (b'0' as u32..=b'9' as u32).contains(&c),
            RECC_GRAPH => self.isgraph(c),
            RECC_LOWER => self.lowercasep(c),
            RECC_PRINT => self.isprint(c),
            RECC_PUNCT => self.ispunct(c),
            RECC_SPACE => self.isspace(c),
            RECC_UPPER => self.uppercasep(c),
            RECC_XDIGIT => character::char_hexdigit(c as i64) >= 0,
            RECC_ASCII => c < 0x80,
            RECC_NONASCII => c >= 0x80,
            RECC_UNIBYTE => character::single_byte_char_p(c),
            RECC_MULTIBYTE => !character::single_byte_char_p(c),
            RECC_WORD => self.isword(c),
            _ => false,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Text<'a> {
    pub s1: &'a [u8],
    pub s2: &'a [u8],
}

impl<'a> Text<'a> {
    pub fn single(s: &'a [u8]) -> Text<'a> {
        Text { s1: &[], s2: s }
    }

    pub fn len(&self) -> usize {
        self.s1.len() + self.s2.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn byte(&self, i: usize) -> u8 {
        if i < self.s1.len() {
            self.s1[i]
        } else {
            self.s2[i - self.s1.len()]
        }
    }

    fn slice_from(&self, i: usize) -> &'a [u8] {
        if i < self.s1.len() {
            &self.s1[i..]
        } else {
            &self.s2[i - self.s1.len()..]
        }
    }

    fn raw_char(&self, i: usize, multibyte: bool) -> (u32, usize) {
        if multibyte {
            character::string_char(self.slice_from(i))
        } else {
            (self.byte(i) as u32, 1)
        }
    }

    fn char_after(&self, i: usize, multibyte: bool) -> (u32, usize) {
        if multibyte {
            character::string_char(self.slice_from(i))
        } else {
            (unibyte_to_char(self.byte(i) as u32), 1)
        }
    }

    fn char_before(&self, i: usize, multibyte: bool) -> u32 {
        if multibyte {
            let len1 = self.s1.len();
            let (s, end) = if i <= len1 {
                (self.s1, i)
            } else {
                (self.s2, i - len1)
            };
            let mut t = end;
            while t > 0 {
                t -= 1;
                if char_head_p(s[t]) {
                    break;
                }
            }
            character::string_char(&s[t..]).0
        } else {
            unibyte_to_char(self.byte(i - 1) as u32)
        }
    }
}

fn char_head_p(b: u8) -> bool {
    b & 0xC0 != 0x80
}

fn bytes_by_char_head(b: u8) -> usize {
    if b & 0x80 == 0 {
        1
    } else if b & 0x20 == 0 {
        2
    } else if b & 0x10 == 0 {
        3
    } else if b & 0x08 == 0 {
        4
    } else {
        5
    }
}

fn unibyte_to_char(c: u32) -> u32 {
    if c < 0x80 {
        c
    } else {
        character::byte8_to_char(c as u8)
    }
}

fn char_to_byte_safe(c: u32) -> i32 {
    if c < 0x80 {
        c as i32
    } else if character::char_byte8_p(c) {
        (c - 0x3F_FF00) as i32
    } else {
        -1
    }
}

fn char_leading_code(c: u32) -> usize {
    character::char_string(c)[0] as usize
}

fn char_byte8_head_p(b: u8) -> bool {
    b == 0xC0 || b == 0xC1
}

fn extract_number(buf: &[u8], at: usize) -> i32 {
    (buf[at + 1] as i8 as i32) * 256 + buf[at] as i32
}

fn extract_address(buf: &[u8], at: usize) -> usize {
    (at as isize + 2 + extract_number(buf, at) as isize) as usize
}

fn store_number(buf: &mut [u8], at: usize, n: i32) {
    buf[at] = (n & 0xff) as u8;
    buf[at + 1] = ((n >> 8) & 0xff) as u8;
}

fn extract_character(buf: &[u8], at: usize) -> i32 {
    buf[at] as i32 | (buf[at + 1] as i32) << 8 | (buf[at + 2] as i32) << 16
}

fn charset_bitmap_size(buf: &[u8], p: usize) -> usize {
    (buf[p + 1] & 0x7F) as usize
}

fn charset_range_table_exists(buf: &[u8], p: usize) -> bool {
    buf[p + 1] & 0x80 != 0
}

fn charset_range_table(buf: &[u8], p: usize) -> usize {
    p + 4 + charset_bitmap_size(buf, p)
}

fn charset_range_table_bits(buf: &[u8], p: usize) -> i32 {
    let size = charset_bitmap_size(buf, p);
    buf[p + 2 + size] as i32 + buf[p + 3 + size] as i32 * 0x100
}

fn wctype_parse(pat: &[u8], p: &mut usize) -> i32 {
    let rest = &pat[*p..];
    if rest.len() < 4 || rest[0] != b'[' || rest[1] != b':' {
        return -1;
    }
    let mut it = 2;
    loop {
        if it + 1 >= rest.len() {
            return -1;
        }
        if rest[it] == b':' && rest[it + 1] == b']' {
            break;
        }
        it += 1;
    }
    *p += it + 2;
    match &rest[2..it] {
        b"word" => RECC_WORD,
        b"alnum" => RECC_ALNUM,
        b"alpha" => RECC_ALPHA,
        b"space" => RECC_SPACE,
        b"digit" => RECC_DIGIT,
        b"blank" => RECC_BLANK,
        b"upper" => RECC_UPPER,
        b"lower" => RECC_LOWER,
        b"punct" => RECC_PUNCT,
        b"ascii" => RECC_ASCII,
        b"graph" => RECC_GRAPH,
        b"print" => RECC_PRINT,
        b"cntrl" => RECC_CNTRL,
        b"xdigit" => RECC_XDIGIT,
        b"unibyte" => RECC_UNIBYTE,
        b"nonascii" => RECC_NONASCII,
        b"multibyte" => RECC_MULTIBYTE,
        _ => RECC_ERROR,
    }
}

fn wctype_to_bit(cc: i32) -> i32 {
    match cc {
        RECC_NONASCII | RECC_MULTIBYTE => BIT_MULTIBYTE,
        RECC_ALPHA => BIT_ALPHA,
        RECC_ALNUM => BIT_ALNUM,
        RECC_WORD => BIT_WORD,
        RECC_LOWER => BIT_LOWER,
        RECC_UPPER => BIT_UPPER,
        RECC_PUNCT => BIT_PUNCT,
        RECC_SPACE => BIT_SPACE,
        RECC_GRAPH => BIT_GRAPH,
        RECC_PRINT => BIT_PRINT,
        RECC_BLANK => BIT_BLANK,
        _ => 0,
    }
}

struct CompileStackElt {
    begalt: usize,
    fixup_alt_jump: Option<usize>,
    laststart: usize,
    regnum: i32,
}

struct RangeWork {
    table: Vec<i32>,
    bits: i32,
}

impl RangeWork {
    fn set(&mut self, from: i32, to: i32) {
        self.table.push(from);
        self.table.push(to);
    }
}

fn set_list_bit(map: &mut [u8; 32], c: i32) {
    map[(c / 8) as usize] |= 1 << (c % 8);
}

fn setup_ascii_range(
    env: &Env,
    translate: Value,
    work: &mut RangeWork,
    map: &mut [u8; 32],
    from: i32,
    to: i32,
) {
    for c0 in from..=to {
        let mut c1 = env.translate(translate, c0 as u32) as i32;
        if c1 >= 0x80 {
            work.set(c1, c1);
            c1 = char_to_byte_safe(c1 as u32);
            if c1 < 0 {
                c1 = c0;
            }
        }
        set_list_bit(map, c1);
    }
}

fn extend_adjacent(work: &mut RangeWork, used: usize, c: i32) -> bool {
    let mut i = work.table.len() as isize - 2;
    while i >= used as isize {
        let iu = i as usize;
        let from = work.table[iu];
        let to = work.table[iu + 1];
        if c >= from - 1 && c <= to + 1 {
            if c == from - 1 {
                work.table[iu] -= 1;
            } else if c == to + 1 {
                work.table[iu + 1] += 1;
            }
            return true;
        }
        i -= 2;
    }
    false
}

fn setup_unibyte_range(
    env: &Env,
    translate: Value,
    work: &mut RangeWork,
    map: &mut [u8; 32],
    from: i32,
    to: i32,
) {
    let used = work.table.len();
    for c0 in from..=to {
        let c1 = unibyte_to_char(c0 as u32);
        if character::char_byte8_p(c1) {
            set_list_bit(map, c0);
        } else {
            let c2 = env.translate(translate, c1) as i32;
            let mut b = c0;
            if c2 != c1 as i32 {
                let u = char_to_byte_safe(c2 as u32);
                if u >= 0 {
                    b = u;
                }
            }
            set_list_bit(map, b);
            if !extend_adjacent(work, used, c2) {
                work.set(c2, c2);
            }
        }
    }
}

fn setup_multibyte_range(
    env: &Env,
    translate: Value,
    work: &mut RangeWork,
    map: &mut [u8; 32],
    from: i32,
    to: i32,
) {
    let used = work.table.len();
    work.set(from, to);
    for c0 in from..=to {
        let c1 = env.translate(translate, c0 as u32) as i32;
        let mut c2 = char_to_byte_safe(c1 as u32);
        if c2 < 0 && c1 != c0 {
            c2 = char_to_byte_safe(c0 as u32);
        }
        if c2 >= 0 {
            set_list_bit(map, c2);
        }
        if c1 >= from && c1 <= to {
            continue;
        }
        if !extend_adjacent(work, used, c1) {
            work.set(c1, c1);
        }
    }
}

fn at_begline_loc_p(pattern: &[u8], p: usize) -> bool {
    let mut prev = p - 2;
    match pattern[prev] {
        b'(' | b'|' => {}
        b':' => {
            while prev > 0 && pattern[prev - 1].is_ascii_digit() {
                prev -= 1;
            }
            if !(prev > 1 && pattern[prev - 1] == b'?' && pattern[prev - 2] == b'(') {
                return false;
            }
            prev -= 2;
        }
        _ => return false,
    }
    let p = prev;
    while prev > 0 && pattern[prev - 1] == b'\\' {
        prev -= 1;
    }
    (p - prev) & 1 == 1
}

fn at_endline_loc_p(pattern: &[u8], p: usize) -> bool {
    pattern[p] == b'\\'
        && p + 1 < pattern.len()
        && (pattern[p + 1] == b')' || pattern[p + 1] == b'|')
}

fn store_op1(buf: &mut [u8], loc: usize, op: u8, arg: i32) {
    buf[loc] = op;
    store_number(buf, loc + 1, arg);
}

fn store_op2(buf: &mut [u8], loc: usize, op: u8, arg1: i32, arg2: i32) {
    buf[loc] = op;
    store_number(buf, loc + 1, arg1);
    store_number(buf, loc + 3, arg2);
}

fn insert_op1(buf: &mut Vec<u8>, loc: usize, op: u8, arg: i32) {
    buf.splice(loc..loc, [0u8; 3]);
    store_op1(buf, loc, op, arg);
}

fn insert_op2(buf: &mut Vec<u8>, loc: usize, op: u8, arg1: i32, arg2: i32) {
    buf.splice(loc..loc, [0u8; 5]);
    store_op2(buf, loc, op, arg1, arg2);
}

fn insert_jump(buf: &mut Vec<u8>, op: u8, loc: usize, to: usize) {
    insert_op1(buf, loc, op, to as i32 - loc as i32 - 3);
}

fn push_jump(buf: &mut Vec<u8>, op: u8, to: usize) {
    let loc = buf.len();
    buf.extend_from_slice(&[0, 0, 0]);
    store_op1(buf, loc, op, to as i32 - loc as i32 - 3);
}

fn fixup_alt_jump(buf: &mut [u8], fixup: Option<usize>, b: usize) {
    if let Some(at) = fixup {
        store_op1(buf, at, JUMP, b as i32 - at as i32 - 3);
    }
}

pub fn compile(
    env: &Env,
    pattern: &[u8],
    multibyte: bool,
    posix: bool,
    whitespace: Option<&[u8]>,
    translate: Value,
) -> Result<Pattern, &'static str> {
    let mut bufp = Pattern {
        multibyte,
        translate,
        ..Pattern::default()
    };
    let mut buf: Vec<u8> = Vec::with_capacity(32);
    let mut compile_stack: Vec<CompileStackElt> = Vec::new();
    let mut cur: &[u8] = pattern;
    let mut p = 0usize;
    let mut main_state: Option<(&[u8], usize)> = None;
    let mut pending_exact: Option<usize> = None;
    let mut laststart: Option<usize> = None;
    let mut begalt = 0usize;
    let mut fixup: Option<usize> = None;
    let mut work = RangeWork {
        table: Vec::new(),
        bits: 0,
    };
    let tr = |c: u32| env.translate(translate, c);

    macro_rules! patfetch {
        () => {{
            if p >= cur.len() {
                return Err(REG_EEND);
            }
            if multibyte {
                let (c, len) = character::string_char(&cur[p..]);
                p += len;
                c as i32
            } else {
                let c = cur[p] as i32;
                p += 1;
                c
            }
        }};
    }

    loop {
        if buf.len() > MAX_BUF_SIZE {
            return Err(REG_ESIZE);
        }
        if p >= cur.len() {
            if let Some((main, main_p)) = main_state.take() {
                cur = main;
                p = main_p;
                continue;
            }
            break;
        }
        let mut c = patfetch!();
        let mut normal = false;
        match c as u8 {
            _ if c > 0x7f => normal = true,
            b' ' => {
                let mut p1 = p;
                match whitespace {
                    Some(ws) if main_state.is_none() => {
                        while p1 < cur.len() && cur[p1] == b' ' {
                            p1 += 1;
                        }
                        if p1 < cur.len()
                            && (cur[p1] == b'*'
                                || cur[p1] == b'+'
                                || cur[p1] == b'?'
                                || (cur[p1] == b'\\' && p1 + 1 < cur.len() && cur[p1 + 1] == b'{'))
                        {
                            normal = true;
                        } else {
                            main_state = Some((cur, p1));
                            cur = ws;
                            p = 0;
                        }
                    }
                    _ => normal = true,
                }
            }
            b'^' => {
                if !(p == 1 || at_begline_loc_p(cur, p)) {
                    normal = true;
                } else {
                    laststart = None;
                    buf.push(BEGLINE);
                }
            }
            b'$' => {
                if !(p == cur.len() || at_endline_loc_p(cur, p)) {
                    normal = true;
                } else {
                    laststart = Some(buf.len());
                    buf.push(ENDLINE);
                }
            }
            b'+' | b'?' | b'*' => match laststart {
                None => normal = true,
                Some(ls) => {
                    let mut zero_times_ok = false;
                    let mut many_times_ok = false;
                    let mut greedy = true;
                    loop {
                        if c == b'?' as i32 && (zero_times_ok || many_times_ok) {
                            greedy = false;
                        } else {
                            zero_times_ok |= c != b'+' as i32;
                            many_times_ok |= c != b'?' as i32;
                        }
                        if !(p < cur.len() && (cur[p] == b'*' || cur[p] == b'+' || cur[p] == b'?'))
                        {
                            break;
                        }
                        c = cur[p] as i32;
                        p += 1;
                    }
                    if ls != buf.len() {
                        let b = buf.len();
                        if greedy {
                            if many_times_ok {
                                let simple = skip_one_char(&buf, ls) == Some(b);
                                let mut startoffset = 0;
                                let ofj = if simple
                                    || !analyze_first(env, &buf, None, ls, b, None, multibyte)
                                {
                                    ON_FAILURE_JUMP_PREMEMO
                                } else {
                                    ON_FAILURE_JUMP_LOOP
                                };
                                if !zero_times_ok && simple {
                                    startoffset = b - ls;
                                    let copy = buf[ls..b].to_vec();
                                    buf.extend_from_slice(&copy);
                                    zero_times_ok = true;
                                }
                                let b = buf.len();
                                if !zero_times_ok {
                                    push_jump(&mut buf, ofj, b + 6);
                                } else {
                                    let op = if simple { ON_FAILURE_JUMP_SMART } else { ofj };
                                    insert_jump(&mut buf, op, ls + startoffset, b + 6);
                                }
                                push_jump(&mut buf, JUMP, ls + startoffset);
                            } else {
                                insert_jump(&mut buf, ON_FAILURE_JUMP_NOMEMO, ls, b + 3);
                            }
                        } else if many_times_ok {
                            let emptyp = analyze_first(env, &buf, None, ls, b, None, multibyte);
                            if emptyp {
                                buf.push(NO_OP);
                            }
                            let op = if emptyp {
                                ON_FAILURE_JUMP_NASTYLOOP
                            } else {
                                ON_FAILURE_JUMP_PREMEMO
                            };
                            push_jump(&mut buf, op, ls);
                            if zero_times_ok {
                                let b = buf.len();
                                insert_jump(&mut buf, JUMP, ls, b);
                            }
                        } else {
                            insert_jump(&mut buf, JUMP, ls, b + 3);
                            insert_jump(&mut buf, ON_FAILURE_JUMP_NOMEMO, ls, ls + 6);
                        }
                    }
                    pending_exact = None;
                }
            },
            b'.' => {
                laststart = Some(buf.len());
                buf.push(ANYCHAR);
            }
            b'[' => {
                work.table.clear();
                work.bits = 0;
                if p >= cur.len() {
                    return Err(REG_EBRACK);
                }
                let start = buf.len();
                laststart = Some(start);
                let not = cur[p] == b'^';
                buf.push(if not { CHARSET_NOT } else { CHARSET });
                if not {
                    p += 1;
                }
                let p1 = p;
                let mut map = [0u8; 32];
                loop {
                    let p2 = p;
                    if p >= cur.len() {
                        return Err(REG_EBRACK);
                    }
                    let cc = wctype_parse(cur, &mut p);
                    if cc != -1 {
                        if cc == RECC_ERROR {
                            return Err(REG_ECTYPE);
                        }
                        if p >= cur.len() {
                            return Err(REG_EBRACK);
                        }
                        for ch in 0..0x80u32 {
                            if env.iswctype(ch, cc) {
                                set_list_bit(&mut map, ch as i32);
                                let c1 = tr(ch);
                                if c1 == ch {
                                    continue;
                                }
                                if c1 < 0x80 {
                                    set_list_bit(&mut map, c1 as i32);
                                } else {
                                    let u = char_to_byte_safe(c1);
                                    if u >= 0 {
                                        set_list_bit(&mut map, u);
                                    }
                                }
                            }
                        }
                        work.bits |= wctype_to_bit(cc);
                        if cc == RECC_SPACE || cc == RECC_WORD {
                            bufp.used_syntax = true;
                        }
                        continue;
                    }
                    let mut c = patfetch!();
                    if c == b']' as i32 && p2 != p1 {
                        break;
                    }
                    let mut c1;
                    if p < cur.len()
                        && cur[p] == b'-'
                        && cur.get(p + 1).copied().unwrap_or(0) != b']'
                    {
                        let _ = patfetch!();
                        c1 = patfetch!();
                        if character::char_byte8_p(c1 as u32)
                            && c >= 0x80
                            && !character::char_byte8_p(c as u32)
                        {
                            c = c1 + 1;
                        }
                    } else {
                        c1 = c;
                    }
                    if c <= c1 {
                        if c < 128 {
                            let ch = c1.min(127);
                            setup_ascii_range(env, translate, &mut work, &mut map, c, ch);
                            c = ch + 1;
                            if character::char_byte8_p(c1 as u32) {
                                c = character::byte8_to_char(128) as i32;
                            }
                        }
                        if c <= c1 {
                            if character::char_byte8_p(c as u32) {
                                let from = character::char_to_byte8(c as u32) as i32;
                                c1 = character::char_to_byte8(c1 as u32) as i32;
                                for b in from..=c1 {
                                    set_list_bit(&mut map, b);
                                }
                            } else if multibyte {
                                setup_multibyte_range(env, translate, &mut work, &mut map, c, c1);
                            } else {
                                setup_unibyte_range(env, translate, &mut work, &mut map, c, c1);
                            }
                        }
                    }
                }
                let mut count = 32usize;
                while count > 0 && map[count - 1] == 0 {
                    count -= 1;
                }
                buf.push(count as u8);
                buf.extend_from_slice(&map[..count]);
                if !work.table.is_empty() || work.bits != 0 {
                    buf[start + 1] |= 0x80;
                    buf.push((work.bits & 0xff) as u8);
                    buf.push((work.bits >> 8) as u8);
                    let n = (work.table.len() / 2) as i32;
                    buf.push((n & 0xff) as u8);
                    buf.push((n >> 8) as u8);
                    for &ch in &work.table {
                        buf.push((ch & 0xff) as u8);
                        buf.push(((ch >> 8) & 0xff) as u8);
                        buf.push((ch >> 16) as u8);
                    }
                }
            }
            b'\\' => {
                if p >= cur.len() {
                    return Err(REG_EESCAPE);
                }
                c = patfetch!();
                match c as u8 {
                    _ if c > 0x7f => normal = true,
                    b'(' => {
                        let mut shy = false;
                        let mut regnum: i32 = 0;
                        if p + 1 < cur.len() && cur[p] == b'?' {
                            let _ = patfetch!();
                            while !shy {
                                let ch = patfetch!();
                                match ch as u8 {
                                    b':' if ch < 0x80 => shy = true,
                                    b'0'..=b'9' if ch < 0x80 => {
                                        if ch == b'0' as i32 && regnum == 0 {
                                            return Err(REG_BADPAT);
                                        }
                                        regnum = regnum
                                            .checked_mul(10)
                                            .and_then(|r| r.checked_add(ch - b'0' as i32))
                                            .ok_or(REG_ESIZE)?;
                                    }
                                    _ => return Err(REG_BADPAT),
                                }
                            }
                        }
                        if !shy {
                            bufp.re_nsub += 1;
                            regnum = bufp.re_nsub as i32;
                        } else if regnum != 0 {
                            if regnum as usize > bufp.re_nsub {
                                bufp.re_nsub = regnum as usize;
                            } else if compile_stack.iter().any(|e| e.regnum == regnum) {
                                return Err(REG_BADPAT);
                            }
                        } else {
                            regnum = -(bufp.re_nsub as i32);
                        }
                        compile_stack.push(CompileStackElt {
                            begalt,
                            fixup_alt_jump: fixup,
                            laststart: buf.len(),
                            regnum,
                        });
                        if regnum <= MAX_REGNUM && regnum > 0 {
                            buf.push(START_MEMORY);
                            buf.push(regnum as u8);
                        }
                        fixup = None;
                        laststart = None;
                        begalt = buf.len();
                        pending_exact = None;
                    }
                    b')' => {
                        if compile_stack.is_empty() {
                            return Err(REG_ERPAREN);
                        }
                        let b = buf.len();
                        fixup_alt_jump(&mut buf, fixup, b);
                        let top = compile_stack.pop().expect("compile stack");
                        begalt = top.begalt;
                        fixup = top.fixup_alt_jump;
                        laststart = Some(top.laststart);
                        pending_exact = None;
                        if top.regnum <= MAX_REGNUM && top.regnum > 0 {
                            buf.push(STOP_MEMORY);
                            buf.push(top.regnum as u8);
                        }
                    }
                    b'|' => {
                        let b = buf.len();
                        insert_jump(&mut buf, ON_FAILURE_JUMP_NOMEMO, begalt, b + 6);
                        pending_exact = None;
                        let b = buf.len();
                        fixup_alt_jump(&mut buf, fixup, b);
                        fixup = Some(b);
                        buf.extend_from_slice(&[0, 0, 0]);
                        laststart = None;
                        begalt = buf.len();
                    }
                    b'{' => {
                        let beg_interval = p;
                        let mut lower_bound: i32 = 0;
                        let mut upper_bound: i32 = -1;
                        macro_rules! get_interval_count {
                            ($num:ident) => {{
                                if p >= cur.len() {
                                    return Err(REG_EBRACE);
                                }
                                c = patfetch!();
                                while (b'0' as i32..=b'9' as i32).contains(&c) {
                                    if $num < 0 {
                                        $num = 0;
                                    }
                                    if RE_DUP_MAX / 10
                                        - i32::from(RE_DUP_MAX % 10 < c - b'0' as i32)
                                        < $num
                                    {
                                        return Err(REG_ESIZEBR);
                                    }
                                    $num = $num * 10 + c - b'0' as i32;
                                    if p >= cur.len() {
                                        return Err(REG_EBRACE);
                                    }
                                    c = patfetch!();
                                }
                            }};
                        }
                        get_interval_count!(lower_bound);
                        if c == b',' as i32 {
                            get_interval_count!(upper_bound);
                        } else {
                            upper_bound = lower_bound;
                        }
                        if lower_bound < 0
                            || (0 <= upper_bound && upper_bound < lower_bound)
                            || c != b'\\' as i32
                        {
                            return Err(REG_BADBR);
                        }
                        if p >= cur.len() {
                            return Err(REG_EESCAPE);
                        }
                        let close = cur[p];
                        p += 1;
                        if close != b'}' {
                            return Err(REG_BADBR);
                        }
                        match laststart {
                            None => {
                                p = beg_interval;
                                c = b'{' as i32;
                                normal = true;
                            }
                            Some(ls) => {
                                if upper_bound == 0 {
                                    buf.truncate(ls);
                                } else if lower_bound == 1 && upper_bound == 1 {
                                } else {
                                    let nbytes: usize = if upper_bound < 0 {
                                        3
                                    } else if upper_bound > 1 {
                                        5
                                    } else {
                                        0
                                    };
                                    let mut startoffset = 0usize;
                                    let b = buf.len();
                                    if lower_bound == 0 {
                                        insert_jump(
                                            &mut buf,
                                            ON_FAILURE_JUMP_LOOP,
                                            ls,
                                            b + 3 + nbytes,
                                        );
                                    } else {
                                        let arg1 = (b + 5 + nbytes) as i32 - ls as i32 - 3;
                                        insert_op2(&mut buf, ls, SUCCEED_N, arg1, lower_bound);
                                        insert_op2(&mut buf, ls, SET_NUMBER_AT, 5, lower_bound);
                                        startoffset += 5;
                                    }
                                    if upper_bound < 0 {
                                        push_jump(&mut buf, JUMP, ls + startoffset);
                                    } else if upper_bound > 1 {
                                        let b = buf.len();
                                        buf.extend_from_slice(&[0; 5]);
                                        let arg1 = (ls + startoffset) as i32 - b as i32 - 3;
                                        store_op2(&mut buf, b, JUMP_N, arg1, upper_bound - 1);
                                        let b = buf.len();
                                        insert_op2(
                                            &mut buf,
                                            ls,
                                            SET_NUMBER_AT,
                                            (b - ls) as i32,
                                            upper_bound - 1,
                                        );
                                    }
                                }
                                pending_exact = None;
                            }
                        }
                    }
                    b'=' => {
                        laststart = Some(buf.len());
                        buf.push(AT_DOT);
                    }
                    b's' | b'S' => {
                        laststart = Some(buf.len());
                        let ch = patfetch!();
                        buf.push(if c == b's' as i32 {
                            SYNTAXSPEC
                        } else {
                            NOTSYNTAXSPEC
                        });
                        buf.push(if ch < 256 {
                            syntax_spec_code(ch as u8)
                        } else {
                            0xFF
                        });
                    }
                    b'c' | b'C' => {
                        laststart = Some(buf.len());
                        let ch = patfetch!();
                        buf.push(if c == b'c' as i32 {
                            CATEGORYSPEC
                        } else {
                            NOTCATEGORYSPEC
                        });
                        buf.push(ch as u8);
                    }
                    b'w' => {
                        laststart = Some(buf.len());
                        buf.push(SYNTAXSPEC);
                        buf.push(S_WORD);
                    }
                    b'W' => {
                        laststart = Some(buf.len());
                        buf.push(NOTSYNTAXSPEC);
                        buf.push(S_WORD);
                    }
                    b'<' => {
                        laststart = Some(buf.len());
                        buf.push(WORDBEG);
                    }
                    b'>' => {
                        laststart = Some(buf.len());
                        buf.push(WORDEND);
                    }
                    b'_' => {
                        laststart = Some(buf.len());
                        let ch = patfetch!();
                        if ch == b'<' as i32 {
                            buf.push(SYMBEG);
                        } else if ch == b'>' as i32 {
                            buf.push(SYMEND);
                        } else {
                            return Err(REG_BADPAT);
                        }
                    }
                    b'b' => {
                        laststart = Some(buf.len());
                        buf.push(WORDBOUND);
                    }
                    b'B' => {
                        laststart = Some(buf.len());
                        buf.push(NOTWORDBOUND);
                    }
                    b'`' => {
                        laststart = None;
                        buf.push(BEGBUF);
                    }
                    b'\'' => {
                        laststart = Some(buf.len());
                        buf.push(ENDBUF);
                    }
                    b'1'..=b'9' => {
                        let reg = c - b'0' as i32;
                        if reg as usize > bufp.re_nsub
                            || reg < 1
                            || compile_stack.iter().any(|e| e.regnum == reg)
                        {
                            return Err(REG_ESUBREG);
                        }
                        laststart = Some(buf.len());
                        buf.push(DUPLICATE);
                        buf.push(reg as u8);
                    }
                    _ => normal = true,
                }
            }
            _ => normal = true,
        }
        if normal {
            let b = buf.len();
            let start_new = match pending_exact {
                None => true,
                Some(pe) => {
                    pe + buf[pe] as usize + 1 != b
                        || buf[pe] as usize >= 256 - character::MAX_MULTIBYTE_LENGTH
                        || (p < cur.len() && (cur[p] == b'*' || cur[p] == b'+' || cur[p] == b'?'))
                        || (p + 1 < cur.len() && cur[p] == b'\\' && cur[p + 1] == b'{')
                }
            };
            if start_new {
                laststart = Some(buf.len());
                buf.push(EXACTN);
                buf.push(0);
                pending_exact = Some(buf.len() - 1);
            }
            let len;
            if multibyte {
                let t = tr(c as u32);
                let bytes = character::char_string(t);
                len = bytes.len();
                buf.extend_from_slice(&bytes);
            } else {
                let c1 = unibyte_to_char(c as u32);
                let mut out = c;
                if !character::char_byte8_p(c1) {
                    let c2 = tr(c1);
                    if c1 != c2 {
                        let u = char_to_byte_safe(c2);
                        if u >= 0 {
                            out = u;
                        }
                    }
                }
                buf.push(out as u8);
                len = 1;
            }
            let pe = pending_exact.expect("pending exact");
            buf[pe] += len as u8;
        }
    }
    let b = buf.len();
    fixup_alt_jump(&mut buf, fixup, b);
    if !compile_stack.is_empty() {
        return Err(REG_EPAREN);
    }
    if !posix {
        buf.push(SUCCEED);
    }
    if buf.len() > MAX_BUF_SIZE {
        return Err(REG_ESIZE);
    }
    bufp.buffer = buf;
    Ok(bufp)
}

fn skip_one_char(buf: &[u8], p: usize) -> Option<usize> {
    let op = buf[p];
    let p = p + 1;
    match op {
        ANYCHAR => Some(p),
        EXACTN => Some(p + buf[p] as usize + 1),
        CHARSET | CHARSET_NOT => {
            if charset_range_table_exists(buf, p - 1) {
                let rt = charset_range_table(buf, p - 1);
                let count = extract_number(buf, rt) as usize;
                Some(rt + 2 + count * 6)
            } else {
                Some(p + 1 + charset_bitmap_size(buf, p - 1))
            }
        }
        SYNTAXSPEC | NOTSYNTAXSPEC | CATEGORYSPEC | NOTCATEGORYSPEC => Some(p + 1),
        _ => None,
    }
}

fn forall_firstchar_1(
    buf: &[u8],
    mut p: usize,
    pend: usize,
    mut loop_beg: isize,
    mut loop_end: isize,
    f: &mut dyn FnMut(usize) -> bool,
) -> bool {
    loop {
        let p_orig = p;
        if p == pend {
            return false;
        } else if p as isize == loop_end {
            return true;
        } else if p as isize > loop_end {
            return false;
        }
        let (newp1, newp2);
        match buf[p] {
            NO_OP => {
                p += 1;
                continue;
            }
            SUCCEED | EXACTN | CHARSET | CHARSET_NOT | ANYCHAR | SYNTAXSPEC | NOTSYNTAXSPEC
            | CATEGORYSPEC | NOTCATEGORYSPEC => return f(p),
            AT_DOT | BEGBUF | WORDBOUND | NOTWORDBOUND | BEGLINE | ENDLINE | ENDBUF | WORDBEG
            | WORDEND | SYMBEG | SYMEND => {
                if f(p) {
                    return true;
                }
                p += 1;
                continue;
            }
            JUMP | JUMP_N => {
                let n1 = extract_address(buf, p + 1);
                if n1 > p {
                    p = n1;
                    continue;
                }
                match buf[n1] {
                    ON_FAILURE_JUMP_MEMO
                    | ON_FAILURE_JUMP_NOMEMO
                    | ON_FAILURE_JUMP_PREMEMO
                    | ON_FAILURE_KEEP_STRING_JUMP
                    | ON_FAILURE_JUMP_NASTYLOOP
                    | ON_FAILURE_JUMP_LOOP
                    | ON_FAILURE_JUMP_SMART
                    | SUCCEED_N => {
                        let n2 = extract_address(buf, n1 + 1);
                        match twoway(buf, p_orig, n1, n2, pend, loop_beg, loop_end, f) {
                            Some((a, b)) => {
                                newp1 = a;
                                newp2 = b;
                            }
                            None => return false,
                        }
                    }
                    _ => {
                        newp1 = n1;
                        newp2 = loop_end as usize;
                    }
                }
            }
            ON_FAILURE_JUMP_MEMO
            | ON_FAILURE_JUMP_NOMEMO
            | ON_FAILURE_JUMP_PREMEMO
            | ON_FAILURE_KEEP_STRING_JUMP
            | ON_FAILURE_JUMP_NASTYLOOP
            | ON_FAILURE_JUMP_LOOP
            | ON_FAILURE_JUMP_SMART => {
                let n1 = extract_address(buf, p + 1);
                let mut n2 = p + 3;
                if buf[n2] == JUMP {
                    let p3 = extract_address(buf, n2 + 1);
                    if p3 > p_orig || n1 > p_orig {
                        n2 = p3;
                    }
                }
                match twoway(buf, p_orig, n1, n2, pend, loop_beg, loop_end, f) {
                    Some((a, b)) => {
                        newp1 = a;
                        newp2 = b;
                    }
                    None => return false,
                }
            }
            SUCCEED_N => {
                let n1 = extract_address(buf, p + 1);
                let n2 = p + 5;
                match twoway(buf, p_orig, n1, n2, pend, loop_beg, loop_end, f) {
                    Some((a, b)) => {
                        newp1 = a;
                        newp2 = b;
                    }
                    None => return false,
                }
            }
            SET_NUMBER_AT => {
                let offset = extract_number(buf, p + 1);
                p += if offset == 5 && buf.get(p + 5) == Some(&SUCCEED_N) {
                    10
                } else {
                    5
                };
                continue;
            }
            START_MEMORY | STOP_MEMORY => {
                p += 2;
                continue;
            }
            DUPLICATE => {
                if !f(p) {
                    return false;
                }
                p += 2;
                continue;
            }
            _ => return false,
        }
        if newp1 <= p_orig {
            if (newp1 as isize) < loop_beg {
                return false;
            } else if newp1 as isize == loop_beg {
                return true;
            } else {
                loop_beg = newp1 as isize;
                loop_end = newp2 as isize;
            }
        }
        p = newp1;
    }
}

#[allow(clippy::too_many_arguments)]
fn twoway(
    buf: &[u8],
    p_orig: usize,
    n1: usize,
    n2: usize,
    pend: usize,
    loop_beg: isize,
    loop_end: isize,
    f: &mut dyn FnMut(usize) -> bool,
) -> Option<(usize, usize)> {
    let (a, b) = if n1 > n2 { (n2, n1) } else { (n1, n2) };
    if b <= p_orig {
        return None;
    }
    if !forall_firstchar_1(buf, b, pend, loop_beg, loop_end, f) {
        return None;
    }
    Some((a, b))
}

fn forall_firstchar(
    buf: &[u8],
    whole: Option<usize>,
    p: usize,
    pend: usize,
    f: &mut dyn FnMut(usize) -> bool,
) -> bool {
    match whole {
        Some(used) => forall_firstchar_1(buf, p, pend, -1, used as isize + 1, f),
        None => forall_firstchar_1(buf, p, pend, p as isize, pend as isize, f),
    }
}

fn analyze_first(
    env: &Env,
    buf: &[u8],
    whole: Option<usize>,
    p: usize,
    pend: usize,
    fastmap: Option<&mut Vec<bool>>,
    multibyte: bool,
) -> bool {
    match fastmap {
        None => {
            let mut f = |q: usize| -> bool {
                !matches!(
                    buf[q],
                    AT_DOT
                        | BEGBUF
                        | WORDBOUND
                        | NOTWORDBOUND
                        | BEGLINE
                        | ENDLINE
                        | ENDBUF
                        | WORDBEG
                        | WORDEND
                        | SYMBEG
                        | SYMEND
                )
            };
            !forall_firstchar(buf, whole, p, pend, &mut f)
        }
        Some(fastmap) => {
            let mut match_any_multibyte = false;
            let mut f = |q: usize| -> bool {
                analyze_first_fastmap(env, buf, q, fastmap, &mut match_any_multibyte, multibyte)
            };
            !forall_firstchar(buf, whole, p, pend, &mut f)
        }
    }
}

fn analyze_first_fastmap(
    env: &Env,
    buf: &[u8],
    p: usize,
    fastmap: &mut [bool],
    match_any: &mut bool,
    multibyte: bool,
) -> bool {
    match buf[p] {
        SUCCEED => false,
        DUPLICATE => true,
        EXACTN => {
            let first = buf[p + 2];
            fastmap[first as usize] = true;
            if multibyte {
                if char_byte8_head_p(first) {
                    let c = character::string_char(&buf[p + 2..]).0;
                    fastmap[character::char_to_byte8(c) as usize] = true;
                }
            } else {
                let c = unibyte_to_char(first as u32);
                fastmap[char_leading_code(c)] = true;
            }
            true
        }
        ANYCHAR => false,
        CHARSET | CHARSET_NOT => {
            let not = buf[p] == CHARSET_NOT;
            let nbits = charset_bitmap_size(buf, p) * 8;
            if not {
                for slot in fastmap.iter_mut().skip(nbits) {
                    *slot = true;
                }
            }
            let bitmap = p + 2;
            let bit = |j: usize| buf[bitmap + j / 8] & (1 << (j % 8)) != 0;
            for (j, slot) in fastmap.iter_mut().enumerate().take(nbits) {
                if bit(j) ^ not {
                    *slot = true;
                }
            }
            for j in 0x80..nbits {
                if bit(j) ^ not {
                    fastmap[char_leading_code(character::byte8_to_char(j as u8))] = true;
                }
            }
            let has_rt = charset_range_table_exists(buf, p);
            if not || (has_rt && charset_range_table_bits(buf, p) != 0) {
                if !*match_any {
                    for slot in fastmap.iter_mut().take(0xF8 + 1).skip(0xC0) {
                        *slot = true;
                    }
                    *match_any = true;
                }
            } else if !not && has_rt && !*match_any {
                let rt = charset_range_table(buf, p);
                let count = extract_number(buf, rt) as usize;
                let mut q = rt + 2;
                for _ in 0..count {
                    let lc1 = char_leading_code(extract_character(buf, q) as u32);
                    let lc2 = char_leading_code(extract_character(buf, q + 3) as u32);
                    for slot in fastmap.iter_mut().take(lc2 + 1).skip(lc1) {
                        *slot = true;
                    }
                    q += 6;
                }
            }
            true
        }
        SYNTAXSPEC | NOTSYNTAXSPEC => false,
        CATEGORYSPEC | NOTCATEGORYSPEC => {
            let not = buf[p] == NOTCATEGORYSPEC;
            let k = buf[p + 1] as u32;
            for (j, slot) in fastmap.iter_mut().enumerate().take(256) {
                if env.char_has_category(j as u32, k) ^ not {
                    *slot = true;
                }
            }
            if !*match_any {
                for slot in fastmap.iter_mut().take(0xF8 + 1).skip(0xC0) {
                    *slot = true;
                }
                *match_any = true;
            }
            true
        }
        _ => false,
    }
}

fn compile_fastmap(env: &Env, bufp: &mut Pattern) {
    let mut fastmap = vec![false; 256];
    bufp.fastmap_accurate = true;
    let used = bufp.buffer.len();
    bufp.can_be_null = analyze_first(
        env,
        &bufp.buffer,
        Some(used),
        0,
        used,
        Some(&mut fastmap),
        bufp.multibyte,
    );
    bufp.fastmap = fastmap;
}

fn execute_charset(
    env: &Env,
    buf: &[u8],
    p: usize,
    c: u32,
    corig: u32,
    unibyte: bool,
    canon_table: Value,
) -> (bool, usize) {
    let not = buf[p] == CHARSET_NOT;
    let mut rtp = None;
    let next = if charset_range_table_exists(buf, p) {
        let rt = charset_range_table(buf, p);
        let count = extract_number(buf, rt) as usize;
        rtp = Some((rt + 2, count));
        rt + 2 + count * 6
    } else {
        p + 2 + charset_bitmap_size(buf, p)
    };
    if unibyte && c < 256 {
        if (c as usize) < charset_bitmap_size(buf, p) * 8
            && buf[p + 2 + c as usize / 8] & (1 << (c % 8)) != 0
        {
            return (!not, next);
        }
    } else if let Some((start, count)) = rtp {
        let class_bits = charset_range_table_bits(buf, p);
        if class_bits & BIT_MULTIBYTE != 0
            || (class_bits & BIT_ALNUM != 0 && env.isalnum(c))
            || (class_bits & BIT_ALPHA != 0 && env.isalpha(c))
            || (class_bits & BIT_SPACE != 0 && env.isspace(c))
            || (class_bits & BIT_BLANK != 0 && env.isblank(c))
            || (class_bits & BIT_WORD != 0 && env.isword(c))
            || (class_bits & BIT_UPPER != 0
                && (env.uppercasep(corig) || (!canon_table.is_nil() && env.lowercasep(corig))))
            || (class_bits & BIT_LOWER != 0
                && (env.lowercasep(corig) || (!canon_table.is_nil() && env.uppercasep(corig))))
            || (class_bits & BIT_PUNCT != 0 && env.ispunct(c))
            || (class_bits & BIT_GRAPH != 0 && env.isgraph(c))
            || (class_bits & BIT_PRINT != 0 && env.isprint(c))
        {
            return (!not, next);
        }
        let mut q = start;
        for _ in 0..count {
            let range_start = extract_character(buf, q);
            let range_end = extract_character(buf, q + 3);
            if range_start <= c as i32 && c as i32 <= range_end {
                return (!not, next);
            }
            q += 6;
        }
    }
    (not, next)
}

fn mutually_exclusive_exactn(env: &Env, buf: &[u8], multibyte: bool, p1: usize, p2: usize) -> bool {
    let c = if buf[p2] == ENDLINE {
        b'\n' as u32
    } else if multibyte {
        character::string_char(&buf[p2 + 2..]).0
    } else {
        buf[p2 + 2] as u32
    };
    match buf[p1] {
        EXACTN => {
            let c1 = if multibyte {
                character::string_char(&buf[p1 + 2..]).0
            } else {
                buf[p1 + 2] as u32
            };
            c != c1
        }
        CHARSET | CHARSET_NOT => {
            !execute_charset(env, buf, p1, c, c, !multibyte || c < 0x80, Value::NIL).0
        }
        ANYCHAR => c == b'\n' as u32,
        _ => false,
    }
}

fn mutually_exclusive_charset(buf: &[u8], multibyte: bool, p1: usize, p2: usize) -> bool {
    if !multibyte || !charset_range_table_exists(buf, p2) {
        let p2_size = buf[p2 + 1] as usize;
        if buf[p1] == CHARSET {
            let p1_size = charset_bitmap_size(buf, p1);
            let mut idx = 0;
            while idx < p2_size && idx < p1_size {
                if buf[p2 + 2 + idx] & buf[p1 + 2 + idx] != 0 {
                    break;
                }
                idx += 1;
            }
            if idx == p2_size || idx == p1_size {
                return true;
            }
        } else if buf[p1] == CHARSET_NOT {
            let p1_size = charset_bitmap_size(buf, p1);
            let mut idx = 0;
            while idx < p2_size {
                if !(buf[p2 + 2 + idx] == 0
                    || (idx < p1_size && (buf[p2 + 2 + idx] & !buf[p1 + 2 + idx]) == 0))
                {
                    break;
                }
                idx += 1;
            }
            if idx == p2_size {
                return true;
            }
        }
    }
    false
}

fn mutually_exclusive_p(env: &Env, bufp: &Pattern, p1: usize, p2: usize) -> bool {
    let buf = &bufp.buffer;
    let multibyte = bufp.multibyte;
    let mut unconstrained = true;
    let mut f = |q: usize| -> bool {
        let constrain = |v: bool, unconstrained: &mut bool| {
            if !v {
                *unconstrained = false;
            }
            v
        };
        match buf[q] {
            SUCCEED => unconstrained,
            ENDLINE => constrain(
                mutually_exclusive_exactn(env, buf, multibyte, p1, q),
                &mut unconstrained,
            ),
            EXACTN => mutually_exclusive_exactn(env, buf, multibyte, p1, q),
            CHARSET => {
                if buf[p1] == EXACTN {
                    mutually_exclusive_exactn(env, buf, multibyte, q, p1)
                } else {
                    mutually_exclusive_charset(buf, multibyte, p1, q)
                }
            }
            CHARSET_NOT => match buf[p1] {
                EXACTN => mutually_exclusive_exactn(env, buf, multibyte, q, p1),
                CHARSET => mutually_exclusive_charset(buf, multibyte, q, p1),
                _ => false,
            },
            ANYCHAR => false,
            SYNTAXSPEC => buf[p1] == NOTSYNTAXSPEC && buf[p1 + 1] == buf[q + 1],
            NOTSYNTAXSPEC => buf[p1] == SYNTAXSPEC && buf[p1 + 1] == buf[q + 1],
            CATEGORYSPEC => buf[p1] == NOTCATEGORYSPEC && buf[p1 + 1] == buf[q + 1],
            NOTCATEGORYSPEC => buf[p1] == CATEGORYSPEC && buf[p1 + 1] == buf[q + 1],
            ENDBUF => true,
            WORDBEG => constrain(
                buf[p1] == NOTSYNTAXSPEC && buf[p1 + 1] == S_WORD,
                &mut unconstrained,
            ),
            WORDEND => constrain(
                buf[p1] == SYNTAXSPEC && buf[p1 + 1] == S_WORD,
                &mut unconstrained,
            ),
            SYMBEG => constrain(
                buf[p1] == NOTSYNTAXSPEC && (buf[p1 + 1] == S_SYMBOL || buf[p1 + 1] == S_WORD),
                &mut unconstrained,
            ),
            SYMEND => constrain(
                buf[p1] == SYNTAXSPEC && (buf[p1 + 1] == S_SYMBOL || buf[p1 + 1] == S_WORD),
                &mut unconstrained,
            ),
            AT_DOT | BEGBUF | WORDBOUND | NOTWORDBOUND | BEGLINE => {
                constrain(false, &mut unconstrained)
            }
            _ => false,
        }
    };
    let used = buf.len();
    forall_firstchar(buf, Some(used), p2, used, &mut f)
}

enum FailItem {
    Reg {
        n: usize,
        start: Option<usize>,
        end: Option<usize>,
    },
    Counter {
        at: usize,
        value: i32,
    },
    Point {
        pattern: usize,
        string: Option<usize>,
        prev_frame: usize,
    },
}

struct FailStack {
    items: Vec<FailItem>,
    frame: usize,
}

impl FailStack {
    fn empty(&self) -> bool {
        self.frame == 0
    }

    fn room(&self) -> bool {
        self.items.len() < MAX_FAILURE_ITEMS
    }

    fn push_point(&mut self, pattern: usize, string: Option<usize>) -> bool {
        if !self.room() {
            return false;
        }
        self.items.push(FailItem::Point {
            pattern,
            string,
            prev_frame: self.frame,
        });
        self.frame = self.items.len();
        true
    }

    fn point_at(&self, handle: usize) -> (usize, Option<usize>, usize) {
        match self.items[handle - 1] {
            FailItem::Point {
                pattern,
                string,
                prev_frame,
            } => (pattern, string, prev_frame),
            _ => unreachable!(),
        }
    }

    fn check_infinite_loop(&self, pat_cur: usize, d: usize) -> bool {
        let mut failure = self.frame;
        while failure > 0 {
            let (pattern, string, prev) = self.point_at(failure);
            if !(string == Some(d) || string.is_none()) {
                break;
            }
            if pattern == pat_cur {
                return true;
            }
            failure = prev;
        }
        false
    }
}

pub fn re_match(
    env: &Env,
    bufp: &mut Pattern,
    text: Text,
    pos: usize,
    regs: Option<&mut Registers>,
    stop: usize,
) -> isize {
    let num_regs = bufp.re_nsub + 1;
    let mut regstart: Vec<Option<usize>> = vec![None; num_regs];
    let mut regend: Vec<Option<usize>> = vec![None; num_regs];
    let mut best_regstart: Vec<Option<usize>> = vec![None; num_regs];
    let mut best_regend: Vec<Option<usize>> = vec![None; num_regs];
    let mut best_regs_set = false;
    let mut match_end: Option<usize> = None;
    let mut fail = FailStack {
        items: Vec::new(),
        frame: 0,
    };
    let translate = bufp.translate;
    let multibyte = bufp.multibyte;
    let target_multibyte = bufp.target_multibyte;
    let total = text.len();
    let pend = bufp.buffer.len();
    let mut p = 0usize;
    let mut d = pos;
    let tr = |c: u32| env.translate(translate, c);

    macro_rules! succeed {
        ($d:expr) => {{
            let d = $d;
            if let Some(regs) = regs {
                if regs.start.is_empty() {
                    let n = RE_NREGS.max(num_regs);
                    regs.start = vec![-1; n];
                    regs.end = vec![-1; n];
                } else if regs.start.len() < num_regs {
                    let n = regs.start.len();
                    let n = (n + (n >> 1)).max(num_regs);
                    regs.start.resize(n, -1);
                    regs.end.resize(n, -1);
                }
                regs.start[0] = pos as isize;
                regs.end[0] = d as isize;
                for reg in 1..num_regs {
                    match (regstart[reg], regend[reg]) {
                        (Some(s), Some(e)) => {
                            regs.start[reg] = s as isize;
                            regs.end[reg] = e as isize;
                        }
                        _ => {
                            regs.start[reg] = -1;
                            regs.end[reg] = -1;
                        }
                    }
                }
                for reg in num_regs..regs.start.len() {
                    regs.start[reg] = -1;
                    regs.end[reg] = -1;
                }
            }
            return (d - pos) as isize;
        }};
    }

    loop {
        let mut failed = false;
        if p == pend {
            if d != stop {
                let best_match_p = match match_end {
                    Some(m) => d > m,
                    None => true,
                };
                if !fail.empty() {
                    if !best_regs_set || best_match_p {
                        best_regs_set = true;
                        match_end = Some(d);
                        best_regstart[1..num_regs].copy_from_slice(&regstart[1..num_regs]);
                        best_regend[1..num_regs].copy_from_slice(&regend[1..num_regs]);
                    }
                    failed = true;
                } else if best_regs_set && !best_match_p {
                    d = match_end.expect("match end");
                    regstart[1..num_regs].copy_from_slice(&best_regstart[1..num_regs]);
                    regend[1..num_regs].copy_from_slice(&best_regend[1..num_regs]);
                }
            }
            if !failed {
                succeed!(d);
            }
        } else {
            let op = bufp.buffer[p];
            p += 1;
            let buf = &mut bufp.buffer;
            match op {
                NO_OP => {}
                SUCCEED => succeed!(d),
                EXACTN => {
                    let mut mcnt = buf[p] as isize;
                    p += 1;
                    let dfail = d;
                    if target_multibyte {
                        loop {
                            if d == stop {
                                d = dfail;
                                failed = true;
                                break;
                            }
                            let (pat_ch, pat_len) = if multibyte {
                                character::string_char(&buf[p..])
                            } else {
                                (unibyte_to_char(buf[p] as u32), 1)
                            };
                            let (buf_ch, buf_len) = text.char_after(d, true);
                            if tr(buf_ch) != pat_ch {
                                d = dfail;
                                failed = true;
                                break;
                            }
                            p += pat_len;
                            d += buf_len;
                            mcnt -= pat_len as isize;
                            if mcnt <= 0 {
                                break;
                            }
                        }
                    } else {
                        loop {
                            if d == stop {
                                d = dfail;
                                failed = true;
                                break;
                            }
                            let (pat_ch, pat_len) = if multibyte {
                                let (c, l) = character::string_char(&buf[p..]);
                                (char_to_byte_safe(c), l)
                            } else {
                                (buf[p] as i32, 1)
                            };
                            let raw = text.byte(d);
                            let as_mb = unibyte_to_char(raw as u32);
                            let buf_ch = if !character::char_byte8_p(as_mb) {
                                let u = char_to_byte_safe(tr(as_mb));
                                if u < 0 { raw as i32 } else { u }
                            } else {
                                raw as i32
                            };
                            if buf_ch != pat_ch {
                                d = dfail;
                                failed = true;
                                break;
                            }
                            p += pat_len;
                            d += 1;
                            mcnt -= pat_len as isize;
                            if mcnt <= 0 {
                                break;
                            }
                        }
                    }
                }
                ANYCHAR => {
                    if d == stop {
                        failed = true;
                    } else {
                        let (c, len) = text.raw_char(d, target_multibyte);
                        if tr(c) == b'\n' as u32 {
                            failed = true;
                        } else {
                            d += len;
                        }
                    }
                }
                CHARSET | CHARSET_NOT => {
                    if d == stop {
                        failed = true;
                    } else {
                        let (corig, len) = text.raw_char(d, target_multibyte);
                        let mut c = corig;
                        let mut unibyte_char = false;
                        if target_multibyte {
                            c = tr(c);
                            let c1 = char_to_byte_safe(c);
                            if c1 >= 0 {
                                unibyte_char = true;
                                c = c1 as u32;
                            }
                        } else {
                            let c1 = unibyte_to_char(c);
                            if !character::char_byte8_p(c1) {
                                let c1 = char_to_byte_safe(tr(c1));
                                if c1 >= 0 {
                                    unibyte_char = true;
                                    c = c1 as u32;
                                }
                            } else {
                                unibyte_char = true;
                            }
                        }
                        let (ok, next) =
                            execute_charset(env, buf, p - 1, c, corig, unibyte_char, translate);
                        if ok {
                            p = next;
                            d += len;
                        } else {
                            failed = true;
                        }
                    }
                }
                START_MEMORY => {
                    let n = buf[p] as usize;
                    if !fail.room() {
                        return -2;
                    }
                    fail.items.push(FailItem::Reg {
                        n,
                        start: regstart[n],
                        end: regend[n],
                    });
                    regstart[n] = Some(d);
                    p += 1;
                }
                STOP_MEMORY => {
                    let n = buf[p] as usize;
                    regend[n] = Some(d);
                    p += 1;
                }
                DUPLICATE => {
                    let regno = buf[p] as usize;
                    p += 1;
                    match (regstart[regno], regend[regno]) {
                        (Some(rs), Some(re)) => {
                            let dfail = d;
                            let mut d2 = rs;
                            while d2 < re {
                                if d == stop {
                                    failed = true;
                                    break;
                                }
                                if translate.is_nil() {
                                    if text.byte(d) != text.byte(d2) {
                                        failed = true;
                                        break;
                                    }
                                    d += 1;
                                    d2 += 1;
                                } else {
                                    let (c1, l1) = text.char_after(d, target_multibyte);
                                    let (c2, l2) = text.char_after(d2, target_multibyte);
                                    if tr(c1) != tr(c2) || l1 != l2 || d + l1 > stop {
                                        failed = true;
                                        break;
                                    }
                                    d += l1;
                                    d2 += l2;
                                }
                            }
                            if failed {
                                d = dfail;
                            }
                        }
                        _ => failed = true,
                    }
                }
                BEGLINE => {
                    if d != 0 && text.char_before(d, target_multibyte) != b'\n' as u32 {
                        failed = true;
                    }
                }
                ENDLINE => {
                    if d != total && text.byte(d) != b'\n' {
                        failed = true;
                    }
                }
                BEGBUF => {
                    if d != 0 {
                        failed = true;
                    }
                }
                ENDBUF => {
                    if d != total {
                        failed = true;
                    }
                }
                ON_FAILURE_KEEP_STRING_JUMP => {
                    p += 2;
                    if !fail.push_point(p - 3, None) {
                        return -2;
                    }
                }
                ON_FAILURE_JUMP_NASTYLOOP => {
                    p += 2;
                    if !fail.check_infinite_loop(p - 4, d) && !fail.push_point(p - 3, Some(d)) {
                        return -2;
                    }
                }
                ON_FAILURE_JUMP_LOOP => {
                    let mcnt = extract_number(buf, p);
                    p += 2;
                    if fail.check_infinite_loop(p - 3, d) {
                        p = (p as isize + mcnt as isize) as usize;
                    } else if !fail.push_point(p - 3, Some(d)) {
                        return -2;
                    }
                }
                ON_FAILURE_JUMP_MEMO | ON_FAILURE_JUMP_PREMEMO | ON_FAILURE_JUMP_NOMEMO => {
                    p += 2;
                    if !fail.push_point(p - 3, Some(d)) {
                        return -2;
                    }
                }
                ON_FAILURE_JUMP_SMART => {
                    let mcnt = extract_number(buf, p);
                    p += 2;
                    let p1 = p;
                    let p2 = (p as isize + mcnt as isize) as usize;
                    let p3 = p - 3;
                    p -= 3;
                    let jump_offset = extract_number(buf, p2 - 2);
                    if mutually_exclusive_p(env, bufp, p1, p2) {
                        bufp.buffer[p3] = ON_FAILURE_KEEP_STRING_JUMP;
                        store_number(&mut bufp.buffer, p2 - 2, jump_offset + 3);
                    } else {
                        bufp.buffer[p3] = ON_FAILURE_JUMP_PREMEMO;
                    }
                }
                JUMP => {
                    let mcnt = extract_number(buf, p);
                    p = (p as isize + 2 + mcnt as isize) as usize;
                }
                SUCCEED_N => {
                    let mcnt = extract_number(buf, p + 2);
                    if mcnt != 0 {
                        let counter = p + 2;
                        p += 4;
                        if !fail.room() {
                            return -2;
                        }
                        fail.items.push(FailItem::Counter {
                            at: counter,
                            value: mcnt,
                        });
                        store_number(buf, counter, mcnt - 1);
                    } else {
                        let off = extract_number(buf, p);
                        p += 2;
                        if fail.check_infinite_loop(p - 3, d) {
                            p = (p as isize + off as isize) as usize;
                        } else if !fail.push_point(p - 3, Some(d)) {
                            return -2;
                        }
                    }
                }
                JUMP_N => {
                    let mcnt = extract_number(buf, p + 2);
                    if mcnt != 0 {
                        let counter = p + 2;
                        if !fail.room() {
                            return -2;
                        }
                        fail.items.push(FailItem::Counter {
                            at: counter,
                            value: mcnt,
                        });
                        store_number(buf, counter, mcnt - 1);
                        let off = extract_number(buf, p);
                        p = (p as isize + 2 + off as isize) as usize;
                    } else {
                        p += 4;
                    }
                }
                SET_NUMBER_AT => {
                    let off = extract_number(buf, p);
                    p += 2;
                    let at = (p as isize + off as isize) as usize;
                    let value = extract_number(buf, p);
                    p += 2;
                    if !fail.room() {
                        return -2;
                    }
                    let old = extract_number(buf, at);
                    fail.items.push(FailItem::Counter { at, value: old });
                    store_number(buf, at, value);
                }
                WORDBOUND | NOTWORDBOUND => {
                    let mut not = op == NOTWORDBOUND;
                    if d == 0 || d == total {
                        not = !not;
                    } else {
                        let c1 = text.char_before(d, target_multibyte);
                        let s1 = env.syntax(c1);
                        let (c2, _) = text.char_after(d, target_multibyte);
                        let s2 = env.syntax(c2);
                        if (s1 == S_WORD) != (s2 == S_WORD)
                            || (s1 == S_WORD && env.word_boundary(c1, c2))
                        {
                            not = !not;
                        }
                    }
                    if !not {
                        failed = true;
                    }
                }
                WORDBEG => {
                    if d == total || d == stop {
                        failed = true;
                    } else {
                        let (c2, _) = text.char_after(d, target_multibyte);
                        let s2 = env.syntax(c2);
                        if s2 != S_WORD {
                            failed = true;
                        } else if d != 0 {
                            let c1 = text.char_before(d, target_multibyte);
                            let s1 = env.syntax(c1);
                            if s1 == S_WORD && !env.word_boundary(c1, c2) {
                                failed = true;
                            }
                        }
                    }
                }
                WORDEND => {
                    if d == 0 {
                        failed = true;
                    } else {
                        let c1 = text.char_before(d, target_multibyte);
                        let s1 = env.syntax(c1);
                        if s1 != S_WORD {
                            failed = true;
                        } else if d != total {
                            let (c2, _) = text.char_after(d, target_multibyte);
                            let s2 = env.syntax(c2);
                            if s2 == S_WORD && !env.word_boundary(c1, c2) {
                                failed = true;
                            }
                        }
                    }
                }
                SYMBEG => {
                    if d == total || d == stop {
                        failed = true;
                    } else {
                        let (c2, _) = text.raw_char(d, target_multibyte);
                        let s2 = env.syntax(c2);
                        if s2 != S_WORD && s2 != S_SYMBOL {
                            failed = true;
                        } else if d != 0 {
                            let c1 = text.char_before(d, target_multibyte);
                            let s1 = env.syntax(c1);
                            if s1 == S_WORD || s1 == S_SYMBOL {
                                failed = true;
                            }
                        }
                    }
                }
                SYMEND => {
                    if d == 0 {
                        failed = true;
                    } else {
                        let c1 = text.char_before(d, target_multibyte);
                        let s1 = env.syntax(c1);
                        if s1 != S_WORD && s1 != S_SYMBOL {
                            failed = true;
                        } else if d != total {
                            let (c2, _) = text.raw_char(d, target_multibyte);
                            let s2 = env.syntax(c2);
                            if s2 == S_WORD || s2 == S_SYMBOL {
                                failed = true;
                            }
                        }
                    }
                }
                SYNTAXSPEC | NOTSYNTAXSPEC => {
                    let not = op == NOTSYNTAXSPEC;
                    let code = buf[p];
                    p += 1;
                    if d == stop {
                        failed = true;
                    } else {
                        let (c, len) = text.char_after(d, target_multibyte);
                        if (env.syntax(c) != code) ^ not {
                            failed = true;
                        } else {
                            d += len;
                        }
                    }
                }
                AT_DOT => {
                    if env.point != Some(d) {
                        failed = true;
                    }
                }
                CATEGORYSPEC | NOTCATEGORYSPEC => {
                    let not = op == NOTCATEGORYSPEC;
                    let cat = buf[p] as u32;
                    p += 1;
                    if d == stop {
                        failed = true;
                    } else {
                        let (c, len) = text.char_after(d, target_multibyte);
                        if (!env.char_has_category(c, cat)) ^ not {
                            failed = true;
                        } else {
                            d += len;
                        }
                    }
                }
                _ => return -2,
            }
        }
        if !failed {
            continue;
        }
        let mut resumed = false;
        while !fail.empty() {
            while fail.items.len() > fail.frame {
                match fail.items.pop() {
                    Some(FailItem::Counter { at, value }) => {
                        store_number(&mut bufp.buffer, at, value);
                    }
                    Some(FailItem::Reg { n, start, end }) => {
                        regstart[n] = start;
                        regend[n] = end;
                    }
                    _ => unreachable!(),
                }
            }
            let (pat, string, prev) = match fail.items.pop() {
                Some(FailItem::Point {
                    pattern,
                    string,
                    prev_frame,
                }) => (pattern, string, prev_frame),
                _ => unreachable!(),
            };
            fail.frame = prev;
            match bufp.buffer[pat] {
                ON_FAILURE_KEEP_STRING_JUMP => {
                    p = extract_address(&bufp.buffer, pat + 1);
                }
                ON_FAILURE_JUMP_NASTYLOOP => {
                    if !fail.push_point(pat - 1, string) {
                        return -2;
                    }
                    d = string.expect("string position");
                    p = extract_address(&bufp.buffer, pat + 1);
                }
                ON_FAILURE_JUMP_PREMEMO
                | ON_FAILURE_JUMP_LOOP
                | ON_FAILURE_JUMP_MEMO
                | SUCCEED_N
                | ON_FAILURE_JUMP_NOMEMO => {
                    d = string.expect("string position");
                    p = extract_address(&bufp.buffer, pat + 1);
                }
                NO_OP => continue,
                _ => return -2,
            }
            resumed = true;
            break;
        }
        if !resumed {
            break;
        }
    }
    if best_regs_set {
        d = match_end.expect("match end");
        regstart[1..num_regs].copy_from_slice(&best_regstart[1..num_regs]);
        regend[1..num_regs].copy_from_slice(&best_regend[1..num_regs]);
        succeed!(d);
    }
    -1
}

pub fn re_search(
    env: &Env,
    bufp: &mut Pattern,
    text: Text,
    startpos: usize,
    range: isize,
    mut regs: Option<&mut Registers>,
    stop: usize,
) -> isize {
    let total = text.len() as isize;
    let mut startpos = startpos as isize;
    let mut range = range;
    let translate = bufp.translate;
    let multibyte = bufp.target_multibyte;
    if startpos < 0 || startpos > total {
        return -1;
    }
    let endpos = startpos + range;
    if endpos < 0 {
        range = -startpos;
    } else if endpos > total {
        range = total - startpos;
    }
    if !bufp.buffer.is_empty() && bufp.buffer[0] == BEGBUF && range > 0 {
        if startpos > 0 {
            return -1;
        }
        range = 0;
    }
    if !bufp.buffer.is_empty() && bufp.buffer[0] == AT_DOT && range > 0 {
        match env.point {
            Some(pt) => {
                range = pt as isize - startpos;
                if range < 0 {
                    return -1;
                }
            }
            None => return -1,
        }
    }
    if !bufp.fastmap_accurate {
        compile_fastmap(env, bufp);
    }
    let anchored_start = bufp.buffer.first() == Some(&BEGLINE);
    let tr = |c: u32| env.translate(translate, c);
    loop {
        let mut skip = false;
        if anchored_start && startpos > 0 && text.byte(startpos as usize - 1) != b'\n' {
            skip = true;
        }
        if !skip && startpos < total && !bufp.can_be_null {
            if range > 0 {
                let irange = range;
                while range > 0 {
                    let at = startpos as usize + (irange - range) as usize;
                    if multibyte {
                        let (c, len) = text.char_after(at, true);
                        let c = if translate.is_nil() { c } else { tr(c) };
                        if bufp.fastmap[char_leading_code(c)] {
                            break;
                        }
                        range -= len as isize;
                    } else {
                        let mut b = text.byte(at) as u32;
                        if !translate.is_nil() {
                            let ch = unibyte_to_char(b);
                            let translated = tr(ch);
                            if translated != ch {
                                let u = char_to_byte_safe(translated);
                                if u >= 0 {
                                    b = u as u32;
                                }
                            }
                        }
                        if bufp.fastmap[b as usize] {
                            break;
                        }
                        range -= 1;
                    }
                }
                startpos += irange - range;
            } else {
                let at = startpos as usize;
                if multibyte {
                    let (c, _) = text.char_after(at, true);
                    if !bufp.fastmap[char_leading_code(tr(c))] {
                        skip = true;
                    }
                } else {
                    let mut b = text.byte(at) as u32;
                    let ch = unibyte_to_char(b);
                    let translated = tr(ch);
                    if translated != ch {
                        let u = char_to_byte_safe(translated);
                        if u >= 0 {
                            b = u as u32;
                        }
                    }
                    let idx = tr(b) as usize;
                    if idx >= 256 || !bufp.fastmap[idx] {
                        skip = true;
                    }
                }
            }
        }
        if !skip {
            if range >= 0 && startpos == total && !bufp.can_be_null {
                return -1;
            }
            let val = re_match(
                env,
                bufp,
                text,
                startpos as usize,
                regs.as_deref_mut(),
                stop,
            );
            if val >= 0 {
                return startpos;
            }
            if val == -2 {
                return -2;
            }
        }
        if range == 0 {
            break;
        } else if range > 0 {
            if multibyte {
                let len = bytes_by_char_head(text.byte(startpos as usize)) as isize;
                range -= len;
                if range < 0 {
                    break;
                }
                startpos += len;
            } else {
                range -= 1;
                startpos += 1;
            }
        } else {
            range += 1;
            startpos -= 1;
            if multibyte {
                let mut len = 1isize;
                while startpos - len + 1 > 0
                    && !char_head_p(text.byte((startpos - len + 1) as usize))
                {
                    len += 1;
                }
                range += len - 1;
                if range > 0 {
                    break;
                }
                startpos -= len - 1;
            }
        }
    }
    -1
}
