use crate::character::{MAX_5_BYTE_CHAR, MAX_CHAR, MAX_UNICODE_CHAR};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::Weakness;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const ATTR_ID: usize = 0;
pub const ATTR_NAME: usize = 1;
pub const ATTR_PLIST: usize = 2;
pub const ATTR_MAP: usize = 3;
pub const ATTR_DECODER: usize = 4;
pub const ATTR_ENCODER: usize = 5;
pub const ATTR_SUBSET: usize = 6;
pub const ATTR_SUPERSET: usize = 7;
pub const ATTR_UNIFY_MAP: usize = 8;
pub const ATTR_DEUNIFIER: usize = 9;
const ATTR_MAX: usize = 10;

const ARG_NAME: usize = 0;
const ARG_DIMENSION: usize = 1;
const ARG_CODE_SPACE: usize = 2;
const ARG_MIN_CODE: usize = 3;
const ARG_MAX_CODE: usize = 4;
const ARG_ISO_FINAL: usize = 5;
const ARG_ISO_REVISION: usize = 6;
const ARG_EMACS_MULE_ID: usize = 7;
const ARG_ASCII_COMPATIBLE_P: usize = 8;
const ARG_SUPPLEMENTARY_P: usize = 9;
const ARG_INVALID_CODE: usize = 10;
const ARG_CODE_OFFSET: usize = 11;
const ARG_MAP: usize = 12;
const ARG_SUBSET: usize = 13;
const ARG_SUPERSET: usize = 14;
const ARG_UNIFY_MAP: usize = 15;
const ARG_PLIST: usize = 16;
const ARG_MAX: usize = 17;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Method {
    Offset,
    Map,
    Subset,
    Superset,
}

#[derive(Clone, Debug)]
pub struct Charset {
    pub id: i32,
    pub dimension: i32,
    pub code_space: [i32; 15],
    pub code_space_mask: Option<Box<[u8; 256]>>,
    pub code_linear_p: bool,
    pub iso_chars_96: bool,
    pub ascii_compatible_p: bool,
    pub supplementary_p: bool,
    pub compact_codes_p: bool,
    pub unified_p: bool,
    pub iso_final: i32,
    pub iso_revision: i32,
    pub emacs_mule_id: i32,
    pub method: Method,
    pub min_code: u32,
    pub max_code: u32,
    pub char_index_offset: u32,
    pub min_char: i32,
    pub max_char: i32,
    pub invalid_code: u32,
    pub fast_map: [u8; 190],
    pub code_offset: i32,
}

impl Charset {
    fn empty() -> Charset {
        Charset {
            id: 0,
            dimension: 0,
            code_space: [0; 15],
            code_space_mask: None,
            code_linear_p: false,
            iso_chars_96: false,
            ascii_compatible_p: false,
            supplementary_p: false,
            compact_codes_p: false,
            unified_p: false,
            iso_final: -1,
            iso_revision: -1,
            emacs_mule_id: -1,
            method: Method::Offset,
            min_code: 0,
            max_code: 0,
            char_index_offset: 0,
            min_char: 0,
            max_char: 0,
            invalid_code: 0,
            fast_map: [0; 190],
            code_offset: 0,
        }
    }

    pub fn code_point_to_index(&self, code: u32) -> i64 {
        if self.code_linear_p {
            return code as i64 - self.min_code as i64;
        }
        let mask = self.code_space_mask.as_ref().expect("code space mask");
        if mask[(code >> 24) as usize] & 0x8 != 0
            && mask[((code >> 16) & 0xFF) as usize] & 0x4 != 0
            && mask[((code >> 8) & 0xFF) as usize] & 0x2 != 0
            && mask[(code & 0xFF) as usize] & 0x1 != 0
        {
            let cs = &self.code_space;
            ((code >> 24) as i64 - cs[12] as i64) * cs[11] as i64
                + (((code >> 16) & 0xFF) as i64 - cs[8] as i64) * cs[7] as i64
                + (((code >> 8) & 0xFF) as i64 - cs[4] as i64) * cs[3] as i64
                + ((code & 0xFF) as i64 - cs[0] as i64)
                - self.char_index_offset as i64
        } else {
            -1
        }
    }

    pub fn index_to_code_point(&self, idx: u32) -> u32 {
        if self.code_linear_p {
            return idx.wrapping_add(self.min_code);
        }
        let idx = idx.wrapping_add(self.char_index_offset);
        let cs = &self.code_space;
        let c = |i: usize| cs[i] as u32;
        (c(0) + idx % c(2))
            | ((c(4) + (idx / c(3) % c(6))) << 8)
            | ((c(8) + (idx / c(7) % c(10))) << 16)
            | ((c(12) + (idx / c(11))) << 24)
    }

    pub fn fast_map_ref(&self, c: i32) -> bool {
        if c < 0x10000 {
            self.fast_map[(c >> 10) as usize] & (1 << ((c >> 7) & 7)) != 0
        } else {
            let i = ((c >> 15) + 62) as usize;
            i < 190 && self.fast_map[i] & (1 << ((c >> 12) & 7)) != 0
        }
    }

    fn fast_map_set(&mut self, c: i32) {
        if c < 0x10000 {
            self.fast_map[(c >> 10) as usize] |= 1 << ((c >> 7) & 7);
        } else {
            let i = ((c >> 15) + 62) as usize;
            if i < 190 {
                self.fast_map[i] |= 1 << ((c >> 12) & 7);
            }
        }
    }
}

pub struct Charsets {
    pub hash_table: Value,
    pub table: Vec<Charset>,
    pub attributes: Vec<Value>,
    pub ascii: i32,
    pub eight_bit: i32,
    pub iso_8859_1: i32,
    pub unicode: i32,
    pub emacs: i32,
    pub jisx0201_roman: i32,
    pub jisx0208_1978: i32,
    pub jisx0208: i32,
    pub ksc5601: i32,
    pub unibyte: i32,
    pub ordered_list: Value,
    pub non_preferred_head: Value,
    pub ordered_list_tick: u64,
    pub iso_2022_charset_list: Value,
    pub emacs_mule_charset_list: Value,
    pub emacs_mule_charset: [i32; 256],
    pub emacs_mule_bytes: [i32; 256],
    pub iso_charset_table: Vec<i32>,
    pub char_unify_table: Value,
}

impl Default for Charsets {
    fn default() -> Self {
        Charsets {
            hash_table: Value::NIL,
            table: Vec::new(),
            attributes: Vec::new(),
            ascii: -1,
            eight_bit: -1,
            iso_8859_1: -1,
            unicode: -1,
            emacs: -1,
            jisx0201_roman: -1,
            jisx0208_1978: -1,
            jisx0208: -1,
            ksc5601: -1,
            unibyte: -1,
            ordered_list: Value::NIL,
            non_preferred_head: Value::NIL,
            ordered_list_tick: 0,
            iso_2022_charset_list: Value::NIL,
            emacs_mule_charset_list: Value::NIL,
            emacs_mule_charset: [-1; 256],
            emacs_mule_bytes: std::array::from_fn(|i| match i {
                0x9A | 0x9B => 3,
                0x9C | 0x9D => 4,
                _ => 1,
            }),
            iso_charset_table: vec![-1; 3 * 2 * 128],
            char_unify_table: Value::NIL,
        }
    }
}

impl Charsets {
    pub fn iso_charset(&self, dimension: i32, chars_96: bool, final_char: i32) -> i32 {
        self.iso_charset_table[iso_index(dimension, chars_96, final_char)]
    }

    pub fn roots(&self, out: &mut Vec<Value>) {
        out.push(self.hash_table);
        out.extend_from_slice(&self.attributes);
        out.push(self.ordered_list);
        out.push(self.non_preferred_head);
        out.push(self.iso_2022_charset_list);
        out.push(self.emacs_mule_charset_list);
        out.push(self.char_unify_table);
    }
}

fn iso_index(dimension: i32, chars_96: bool, final_char: i32) -> usize {
    ((dimension - 1) as usize * 2 + usize::from(chars_96)) * 128 + final_char as usize
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("charsetp", 1, 1, f_charsetp),
    Subr::fixed("map-charset-chars", 2, 5, f_map_charset_chars),
    Subr::many(
        "define-charset-internal",
        ARG_MAX as u16,
        f_define_charset_internal,
    ),
    Subr::fixed("define-charset-alias", 2, 2, f_define_charset_alias),
    Subr::fixed("charset-plist", 1, 1, f_charset_plist),
    Subr::fixed("set-charset-plist", 2, 2, f_set_charset_plist),
    Subr::fixed("unify-charset", 1, 3, f_unify_charset),
    Subr::fixed(
        "get-unused-iso-final-char",
        2,
        2,
        f_get_unused_iso_final_char,
    ),
    Subr::fixed("declare-equiv-charset", 4, 4, f_declare_equiv_charset),
    Subr::fixed("find-charset-region", 2, 3, f_find_charset_region),
    Subr::fixed("find-charset-string", 1, 2, f_find_charset_string),
    Subr::fixed("decode-char", 2, 2, f_decode_char),
    Subr::fixed("encode-char", 2, 2, f_encode_char),
    Subr::fixed("split-char", 1, 1, f_split_char),
    Subr::fixed("make-char", 1, 5, f_make_char),
    Subr::fixed("char-charset", 1, 2, f_char_charset),
    Subr::fixed("charset-after", 0, 1, f_charset_after),
    Subr::fixed("iso-charset", 3, 3, f_iso_charset),
    Subr::fixed("clear-charset-maps", 0, 0, f_clear_charset_maps),
    Subr::fixed("charset-priority-list", 0, 1, f_charset_priority_list),
    Subr::many("set-charset-priority", 1, f_set_charset_priority),
    Subr::fixed("charset-id-internal", 0, 1, f_charset_id_internal),
    Subr::fixed("sort-charsets", 1, 1, f_sort_charsets),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.charsets.hash_table = lisp.make_hash_table_with(sym::EQ, Weakness::None);
    lisp.defvar(sym::CHARSET_MAP_PATH, Value::NIL);
    lisp.defvar_bool(sym::INHIBIT_LOAD_CHARSET_MAP, false);
    lisp.defvar(sym::CHARSET_LIST, Value::NIL);
    lisp.defvar(sym::CURRENT_ISO639_LANGUAGE, Value::NIL);
    let ascii = lisp.define_charset_builtin(
        sym::ASCII,
        1,
        [0x00, 0x7F, 0, 0, 0, 0, 0, 0],
        0,
        127,
        b'B' as i32,
        -1,
        0,
        true,
        false,
        0,
    );
    lisp.charsets.ascii = ascii;
    let iso_8859_1 = lisp.define_charset_builtin(
        sym::ISO_8859_1,
        1,
        [0x00, 0xFF, 0, 0, 0, 0, 0, 0],
        0,
        255,
        -1,
        -1,
        -1,
        true,
        false,
        0,
    );
    lisp.charsets.iso_8859_1 = iso_8859_1;
    let unicode = lisp.define_charset_builtin(
        sym::UNICODE,
        3,
        [0x00, 0xFF, 0x00, 0xFF, 0x00, 0x10, 0, 0],
        0,
        MAX_UNICODE_CHAR,
        -1,
        0,
        -1,
        true,
        false,
        0,
    );
    lisp.charsets.unicode = unicode;
    let emacs = lisp.define_charset_builtin(
        sym::EMACS,
        3,
        [0x00, 0xFF, 0x00, 0xFF, 0x00, 0x3F, 0, 0],
        0,
        MAX_5_BYTE_CHAR,
        -1,
        0,
        -1,
        true,
        true,
        0,
    );
    lisp.charsets.emacs = emacs;
    let eight_bit = lisp.define_charset_builtin(
        sym::EIGHT_BIT,
        1,
        [0x80, 0xFF, 0, 0, 0, 0, 0, 0],
        128,
        255,
        -1,
        0,
        -1,
        false,
        true,
        MAX_5_BYTE_CHAR as i32 + 1,
    );
    lisp.charsets.eight_bit = eight_bit;
    lisp.charsets.unibyte = iso_8859_1;
}

fn read_hex(bytes: &[u8], pos: &mut usize, lookahead: i32, overflow: &mut bool) -> (u32, i32) {
    let getc = |pos: &mut usize| -> i32 {
        if *pos < bytes.len() {
            let c = bytes[*pos] as i32;
            *pos += 1;
            c
        } else {
            -1
        }
    };
    let mut c = if lookahead < 0 { getc(pos) } else { lookahead };
    loop {
        if c == b'#' as i32 {
            loop {
                c = getc(pos);
                if !(0 <= c && c != b'\n' as i32) {
                    break;
                }
            }
        } else if c == b'0' as i32 {
            c = getc(pos);
            if c < 0 || c == b'x' as i32 {
                break;
            }
        }
        if c < 0 {
            break;
        }
        c = getc(pos);
    }
    let mut n: u32 = 0;
    let mut v = false;
    if 0 <= c {
        loop {
            c = getc(pos);
            let digit = crate::character::char_hexdigit(c as i64);
            if digit < 0 {
                break;
            }
            v |= n > (u32::MAX >> 4);
            n = (n << 4).wrapping_add(digit as u32);
        }
    }
    *overflow |= v;
    (n, c)
}

impl Lisp {
    pub fn charset(&self, id: i32) -> &Charset {
        &self.charsets.table[id as usize]
    }

    pub fn charset_attrs(&self, id: i32) -> Value {
        self.charsets.attributes[id as usize]
    }

    fn charset_attr(&self, id: i32, idx: usize) -> Value {
        self.vector_items(self.charset_attrs(id))[idx]
    }

    fn set_charset_attr(&mut self, id: i32, idx: usize, val: Value) {
        let attrs = self.charset_attrs(id);
        let _ = self.aset_value(attrs, Value::fixnum(idx as i64), val);
    }

    pub fn charset_name(&self, id: i32) -> Value {
        self.charset_attr(id, ATTR_NAME)
    }

    pub fn charset_symbol_attributes(&mut self, symbol: Value) -> LispResult {
        let table = self.charsets.hash_table;
        self.gethash(symbol, table, Value::NIL)
    }

    pub fn charsetp(&mut self, object: Value) -> bool {
        if !object.is_symbol() {
            return false;
        }
        matches!(self.charset_symbol_attributes(object), Ok(v) if !v.is_nil())
    }

    pub fn check_charset_get_attr(&mut self, x: Value) -> LispResult {
        if x.is_symbol() {
            let attrs = self.charset_symbol_attributes(x)?;
            if !attrs.is_nil() {
                return Ok(attrs);
            }
        }
        Err(self.wrong_type(sym::CHARSETP, x))
    }

    pub fn check_charset_get_id(&mut self, x: Value) -> LispResult<i32> {
        let attrs = self.check_charset_get_attr(x)?;
        Ok(self.vector_items(attrs)[ATTR_ID].as_fixnum().unwrap_or(0) as i32)
    }

    fn charset_map_path(&mut self) -> LispResult {
        let path = self.bound_var(sym::CHARSET_MAP_PATH);
        if !path.is_nil() {
            return Ok(path);
        }
        let data = self.bound_var(sym::DATA_DIRECTORY);
        let name = self.string("charsets");
        let dir = if data.is_string() {
            self.funcall(sym::EXPAND_FILE_NAME, &[name, data])?
        } else {
            name
        };
        let path = self.list(&[dir]);
        self.set_plain_value(sym::CHARSET_MAP_PATH, path);
        Ok(path)
    }

    fn load_charset_map(
        &mut self,
        id: i32,
        entries: &[(u32, u32, i32)],
        control_flag: i32,
    ) -> LispResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let cs = self.charset(id).clone();
        let mut vec = Value::NIL;
        let mut table = Value::NIL;
        let base = self.stack.len();
        if control_flag == 1 {
            if cs.method == Method::Map {
                let n = cs.code_point_to_index(cs.max_code) + 1;
                vec = self.vector(vec![Value::fixnum(-1); n.max(0) as usize]);
                self.set_charset_attr(id, ATTR_DECODER, vec);
            } else if self.is_char_table(self.charsets.char_unify_table) {
                let t = self.charsets.char_unify_table;
                self.char_table_set_range(t, cs.min_char as u32, cs.max_char as u32, Value::NIL);
            }
        } else if control_flag == 2 {
            table = self.make_char_table(Value::NIL, Value::NIL)?;
            let idx = if cs.method == Method::Map {
                ATTR_ENCODER
            } else {
                ATTR_DEUNIFIER
            };
            self.set_charset_attr(id, idx, table);
        }
        self.stack.push(vec);
        self.stack.push(table);
        let mut min_char = entries[0].2;
        let mut max_char = entries[0].2;
        let mut nonascii_min_char = MAX_CHAR as i32;
        let mut fast_map = cs.fast_map;
        for &(from, to, from_c) in entries {
            let from_index = cs.code_point_to_index(from);
            let (to_index, to_c) = if from == to {
                (from_index, from_c)
            } else {
                let ti = cs.code_point_to_index(to);
                (ti, from_c + (ti - from_index) as i32)
            };
            if from_index < 0 || to_index < 0 {
                continue;
            }
            if to_c > max_char {
                max_char = to_c;
            } else if from_c < min_char {
                min_char = from_c;
            }
            match control_flag {
                1 => {
                    let mut c = from_c;
                    if cs.method == Method::Map {
                        for index in from_index..=to_index {
                            self.aset_value(vec, Value::fixnum(index), Value::fixnum(c as i64))?;
                            c += 1;
                        }
                    } else if self.is_char_table(self.charsets.char_unify_table) {
                        let t = self.charsets.char_unify_table;
                        for index in from_index..=to_index {
                            self.char_table_set(
                                t,
                                (cs.code_offset as i64 + index) as u32,
                                Value::fixnum(c as i64),
                            );
                            c += 1;
                        }
                    }
                }
                2 => {
                    for (c, index) in (from_c..).zip(from_index..=to_index) {
                        if self.char_table_ref(table, c as u32).is_nil() {
                            let v = if cs.method == Method::Map && cs.compact_codes_p {
                                cs.index_to_code_point(index as u32) as i64
                            } else {
                                index
                            };
                            self.char_table_set(table, c as u32, Value::fixnum(v));
                        }
                    }
                }
                _ => {
                    if cs.ascii_compatible_p {
                        if from_c >= 0x80 {
                            if from_c < nonascii_min_char {
                                nonascii_min_char = from_c;
                            }
                        } else if to_c >= 0x80 {
                            nonascii_min_char = 0x80;
                        }
                    }
                    let mut tmp = Charset::empty();
                    tmp.fast_map = fast_map;
                    for c in from_c..=to_c {
                        tmp.fast_map_set(c);
                    }
                    fast_map = tmp.fast_map;
                }
            }
        }
        self.stack.truncate(base);
        if control_flag == 0 {
            let c = &mut self.charsets.table[id as usize];
            c.min_char = if cs.ascii_compatible_p {
                nonascii_min_char
            } else {
                min_char
            };
            c.max_char = max_char;
            c.fast_map = fast_map;
        }
        Ok(())
    }

    fn load_charset_map_from_file(
        &mut self,
        id: i32,
        mapfile: Value,
        control_flag: i32,
    ) -> LispResult<()> {
        let (min_code, max_code) = {
            let c = self.charset(id);
            (c.min_code, c.max_code)
        };
        let map = self.unibyte_string(b".map");
        let txt = self.unibyte_string(b".txt");
        let suffixes = self.list(&[map, txt]);
        let count = self.specpdl.len();
        self.specbind(sym::FILE_NAME_HANDLER_ALIST, Value::NIL)?;
        let path = self.charset_map_path()?;
        let (fd, _) = self.openp(path, mapfile, suffixes, Value::NIL, false, false)?;
        let mut file = match fd {
            Ok(crate::lread::OpenpFd::File(file)) => file,
            Ok(_) => {
                let e = std::io::Error::from(std::io::ErrorKind::InvalidInput);
                return Err(self.file_error("Loading charset map", &e, mapfile));
            }
            Err(e) => return Err(self.file_error("Loading charset map", &e, mapfile)),
        };
        self.unbind_to(count)?;
        let mut bytes = Vec::new();
        let _ = std::io::Read::read_to_end(&mut file, &mut bytes);
        let mut entries = Vec::new();
        let mut pos = 0usize;
        let mut ch = -1i32;
        loop {
            let mut overflow = false;
            let (from, c1) = read_hex(&bytes, &mut pos, ch, &mut overflow);
            ch = c1;
            if ch < 0 {
                break;
            }
            let to;
            if ch == b'-' as i32 {
                let (t, c2) = read_hex(&bytes, &mut pos, -1, &mut overflow);
                ch = c2;
                if ch < 0 {
                    break;
                }
                to = t;
            } else {
                to = from;
                ch = -1;
            }
            let (c, c3) = read_hex(&bytes, &mut pos, ch, &mut overflow);
            ch = c3;
            if ch < 0 {
                break;
            }
            if overflow {
                continue;
            }
            if from < min_code || to > max_code || from > to || c > MAX_CHAR {
                continue;
            }
            entries.push((from, to, c as i32));
        }
        self.load_charset_map(id, &entries, control_flag)
    }

    fn load_charset_map_from_vector(
        &mut self,
        id: i32,
        vec: Value,
        control_flag: i32,
    ) -> LispResult<()> {
        let (min_code, max_code) = {
            let c = self.charset(id);
            (c.min_code, c.max_code)
        };
        let items = self.vector_items(vec).to_vec();
        if items.len() % 2 == 1 {
            return Ok(());
        }
        let mut entries = Vec::new();
        for pair in items.chunks(2) {
            let (from, to) = if pair[0].is_cons() {
                let a = self.xcar(pair[0]).as_fixnum().unwrap_or(0) as u32;
                let b = self.xcdr(pair[0]).as_fixnum().unwrap_or(0) as u32;
                (a, b)
            } else {
                let a = pair[0].as_fixnum().unwrap_or(0) as u32;
                (a, a)
            };
            let c = self.check_fixnat(pair[1])?;
            if from < min_code || to > max_code || from > to || c > MAX_CHAR as i64 {
                continue;
            }
            entries.push((from, to, c as i32));
        }
        self.load_charset_map(id, &entries, control_flag)
    }

    fn load_charset(&mut self, id: i32, control_flag: i32) -> LispResult<()> {
        let map = if self.charset(id).method == Method::Map {
            self.charset_attr(id, ATTR_MAP)
        } else {
            self.charset_attr(id, ATTR_UNIFY_MAP)
        };
        self.stack.push(map);
        let r = if map.is_string() {
            self.load_charset_map_from_file(id, map, control_flag)
        } else if self.is_vector(map) {
            self.load_charset_map_from_vector(id, map, control_flag)
        } else {
            Ok(())
        };
        self.stack.pop();
        r
    }

    fn maybe_unify_char(&mut self, c: i32, val: Value) -> LispResult<i32> {
        if let Some(n) = val.as_fixnum() {
            return Ok(n as i32);
        }
        if val.is_nil() {
            return Ok(c);
        }
        let id = self.check_charset_get_id(val)?;
        self.load_charset(id, 1)?;
        let table = self.charsets.char_unify_table;
        if self.is_char_table(table) {
            let v = self.char_table_ref(table, c as u32);
            if let Some(n) = v.as_fixnum() {
                return Ok(n as i32);
            }
        }
        Ok(c)
    }

    pub fn decode_char_fast(&mut self, id: i32, code: u32) -> LispResult<i32> {
        let cs = &self.charsets.table[id as usize];
        if code < 0x80 && cs.ascii_compatible_p {
            return Ok(code as i32);
        }
        if code < cs.min_code || code > cs.max_code {
            return Ok(-1);
        }
        if cs.unified_p {
            return self.decode_char(id, code);
        }
        match cs.method {
            Method::Offset if cs.code_linear_p => Ok((code - cs.min_code) as i32 + cs.code_offset),
            Method::Map if cs.code_linear_p => {
                let decoder = self.charset_attr(id, ATTR_DECODER);
                if self.is_vector(decoder) {
                    let idx = (code - self.charset(id).min_code) as usize;
                    Ok(self.vector_items(decoder)[idx].as_fixnum().unwrap_or(-1) as i32)
                } else {
                    self.decode_char(id, code)
                }
            }
            _ => self.decode_char(id, code),
        }
    }

    pub fn decode_char(&mut self, id: i32, code: u32) -> LispResult<i32> {
        let cs = self.charset(id).clone();
        if code < cs.min_code || code > cs.max_code {
            return Ok(-1);
        }
        match cs.method {
            Method::Subset => {
                let info = self.charset_attr(id, ATTR_SUBSET);
                let items = self.vector_items(info).to_vec();
                let parent = items[0].as_fixnum().unwrap_or(0) as i32;
                let code = code.wrapping_sub(items[3].as_fixnum().unwrap_or(0) as u32);
                if code < items[1].as_fixnum().unwrap_or(0) as u32
                    || code > items[2].as_fixnum().unwrap_or(0) as u32
                {
                    Ok(-1)
                } else {
                    self.decode_char_fast(parent, code)
                }
            }
            Method::Superset => {
                let mut parents = self.charset_attr(id, ATTR_SUPERSET);
                while parents.is_cons() {
                    let elt = self.xcar(parents);
                    let pid = self.xcar(elt).as_fixnum().unwrap_or(0) as i32;
                    let off = self.xcdr(elt).as_fixnum().unwrap_or(0) as u32;
                    let c = self.decode_char_fast(pid, code.wrapping_sub(off))?;
                    if c >= 0 {
                        return Ok(c);
                    }
                    parents = self.xcdr(parents);
                }
                Ok(-1)
            }
            _ => {
                let char_index = cs.code_point_to_index(code);
                if char_index < 0 {
                    return Ok(-1);
                }
                if cs.method == Method::Map {
                    let mut decoder = self.charset_attr(id, ATTR_DECODER);
                    if !self.is_vector(decoder) {
                        self.load_charset(id, 1)?;
                        decoder = self.charset_attr(id, ATTR_DECODER);
                    }
                    if self.is_vector(decoder) {
                        return Ok(self
                            .vector_items(decoder)
                            .get(char_index as usize)
                            .and_then(|v| v.as_fixnum())
                            .unwrap_or(-1) as i32);
                    }
                    Ok(-1)
                } else {
                    let c = char_index as i32 + cs.code_offset;
                    if cs.unified_p && (MAX_UNICODE_CHAR as i32) < c && c <= MAX_5_BYTE_CHAR as i32
                    {
                        let table = self.charsets.char_unify_table;
                        let val = if self.is_char_table(table) {
                            self.char_table_ref(table, c as u32)
                        } else {
                            Value::NIL
                        };
                        return self.maybe_unify_char(c, val);
                    }
                    Ok(c)
                }
            }
        }
    }

    pub fn encode_char_fast(&mut self, id: i32, c: i32) -> LispResult<u32> {
        let cs = &self.charsets.table[id as usize];
        if c < 0x80 && cs.ascii_compatible_p {
            return Ok(c as u32);
        }
        if cs.unified_p || cs.method == Method::Subset || cs.method == Method::Superset {
            return self.encode_char(id, c);
        }
        if c < cs.min_char || c > cs.max_char {
            return Ok(cs.invalid_code);
        }
        match cs.method {
            Method::Offset if cs.code_linear_p => Ok((c - cs.code_offset) as u32 + cs.min_code),
            Method::Map if cs.compact_codes_p => {
                let invalid = cs.invalid_code;
                let encoder = self.charset_attr(id, ATTR_ENCODER);
                if self.is_char_table(encoder) {
                    let v = self.char_table_ref(encoder, c as u32);
                    Ok(match v.as_fixnum() {
                        Some(n) => n as u32,
                        None => invalid,
                    })
                } else {
                    self.encode_char(id, c)
                }
            }
            _ => self.encode_char(id, c),
        }
    }

    pub fn encode_char(&mut self, id: i32, mut c: i32) -> LispResult<u32> {
        let cs = self.charset(id).clone();
        if cs.unified_p {
            let mut deunifier = self.charset_attr(id, ATTR_DEUNIFIER);
            if !self.is_char_table(deunifier) {
                self.load_charset(id, 2)?;
                deunifier = self.charset_attr(id, ATTR_DEUNIFIER);
            }
            if self.is_char_table(deunifier)
                && let Some(n) = self.char_table_ref(deunifier, c as u32).as_fixnum()
                && n >= 0
            {
                c = cs.code_offset + n as i32;
            }
        }
        match cs.method {
            Method::Subset => {
                let info = self.charset_attr(id, ATTR_SUBSET);
                let items = self.vector_items(info).to_vec();
                let parent = items[0].as_fixnum().unwrap_or(0) as i32;
                let code = self.encode_char_fast(parent, c)?;
                if code == self.charset(parent).invalid_code
                    || code < items[1].as_fixnum().unwrap_or(0) as u32
                    || code > items[2].as_fixnum().unwrap_or(0) as u32
                {
                    return Ok(cs.invalid_code);
                }
                return Ok(code.wrapping_add(items[3].as_fixnum().unwrap_or(0) as u32));
            }
            Method::Superset => {
                let mut parents = self.charset_attr(id, ATTR_SUPERSET);
                while parents.is_cons() {
                    let elt = self.xcar(parents);
                    let pid = self.xcar(elt).as_fixnum().unwrap_or(0) as i32;
                    let off = self.xcdr(elt).as_fixnum().unwrap_or(0) as u32;
                    let code = self.encode_char_fast(pid, c)?;
                    if code != self.charset(pid).invalid_code {
                        return Ok(code.wrapping_add(off));
                    }
                    parents = self.xcdr(parents);
                }
                return Ok(cs.invalid_code);
            }
            _ => {}
        }
        if !cs.fast_map_ref(c) || c < cs.min_char || c > cs.max_char {
            return Ok(cs.invalid_code);
        }
        if cs.method == Method::Map {
            let mut encoder = self.charset_attr(id, ATTR_ENCODER);
            if !self.is_char_table(encoder) {
                self.load_charset(id, 2)?;
                encoder = self.charset_attr(id, ATTR_ENCODER);
            }
            if !self.is_char_table(encoder) {
                return Ok(cs.invalid_code);
            }
            let val = self.char_table_ref(encoder, c as u32);
            let Some(n) = val.as_fixnum() else {
                return Ok(cs.invalid_code);
            };
            let code = n as u32;
            Ok(if cs.compact_codes_p {
                code
            } else {
                cs.index_to_code_point(code)
            })
        } else {
            Ok(cs.index_to_code_point((c - cs.code_offset) as u32))
        }
    }

    pub fn char_charset(&mut self, c: i32, charset_list: Value) -> LispResult<Option<(i32, u32)>> {
        let maybe_null = !charset_list.is_nil();
        let mut list = if maybe_null {
            charset_list
        } else {
            self.charsets.ordered_list
        };
        while list.is_cons() {
            let id = self.xcar(list).as_fixnum().unwrap_or(0) as i32;
            let code = self.encode_char_fast(id, c)?;
            if code != self.charset(id).invalid_code {
                return Ok(Some((id, code)));
            }
            list = self.xcdr(list);
            if !maybe_null
                && c <= MAX_UNICODE_CHAR as i32
                && list == self.charsets.non_preferred_head
            {
                return Ok(Some((self.charsets.unicode, c as u32)));
            }
        }
        if maybe_null {
            return Ok(None);
        }
        let id = if c <= MAX_5_BYTE_CHAR as i32 {
            self.charsets.emacs
        } else {
            self.charsets.eight_bit
        };
        let code = self.encode_char_fast(id, c)?;
        Ok(Some((id, code)))
    }

    pub fn char_charset_id(&mut self, c: i32) -> LispResult<i32> {
        if c < 0x80 {
            return Ok(self.charsets.ascii);
        }
        Ok(self
            .char_charset(c, Value::NIL)?
            .map_or(self.charsets.eight_bit, |r| r.0))
    }

    pub fn char_charset_p(&mut self, c: i32, id: i32) -> LispResult<bool> {
        let cs = &self.charsets.table[id as usize];
        if c < 0x80 && cs.ascii_compatible_p {
            return Ok(true);
        }
        if cs.unified_p || cs.method == Method::Subset || cs.method == Method::Superset {
            let invalid = cs.invalid_code;
            return Ok(self.encode_char(id, c)? != invalid);
        }
        if !cs.fast_map_ref(c) {
            return Ok(false);
        }
        if cs.method == Method::Offset {
            return Ok(c >= cs.min_char && c <= cs.max_char);
        }
        let invalid = cs.invalid_code;
        if cs.compact_codes_p {
            let encoder = self.charset_attr(id, ATTR_ENCODER);
            if self.is_char_table(encoder) {
                return Ok(!self.char_table_ref(encoder, c as u32).is_nil());
            }
        }
        Ok(self.encode_char(id, c)? != invalid)
    }

    #[allow(clippy::too_many_arguments)]
    fn define_charset_builtin(
        &mut self,
        name: Value,
        dimension: i64,
        code_space: [i64; 8],
        min_code: u32,
        max_code: u32,
        iso_final: i32,
        iso_revision: i32,
        emacs_mule_id: i32,
        ascii_compatible: bool,
        supplementary: bool,
        code_offset: i32,
    ) -> i32 {
        let space = self.vector(code_space.iter().map(|&b| Value::fixnum(b)).collect());
        self.stack.push(space);
        let iso = if iso_final < 0 {
            Value::NIL
        } else {
            Value::fixnum(iso_final as i64)
        };
        let mule = if emacs_mule_id < 0 {
            Value::NIL
        } else {
            Value::fixnum(emacs_mule_id as i64)
        };
        let k_dimension = self.intern(":dimension");
        let k_code_space = self.intern(":code-space");
        let k_iso_final = self.intern(":iso-final-char");
        let k_emacs_mule = self.intern(":emacs-mule-id");
        let k_code_offset = self.intern(":code-offset");
        let plist = self.list(&[
            sym::K_NAME,
            name,
            k_dimension,
            Value::fixnum(dimension),
            k_code_space,
            space,
            k_iso_final,
            iso,
            k_emacs_mule,
            mule,
            sym::K_ASCII_COMPATIBLE_P,
            self.bool(ascii_compatible),
            k_code_offset,
            Value::fixnum(code_offset as i64),
        ]);
        self.stack.push(plist);
        let mut args = vec![Value::NIL; ARG_MAX];
        args[ARG_NAME] = name;
        args[ARG_DIMENSION] = Value::fixnum(dimension);
        args[ARG_CODE_SPACE] = space;
        args[ARG_MIN_CODE] = Value::fixnum(min_code as i64);
        args[ARG_MAX_CODE] = Value::fixnum(max_code as i64);
        args[ARG_ISO_FINAL] = iso;
        args[ARG_ISO_REVISION] = Value::fixnum(iso_revision as i64);
        args[ARG_EMACS_MULE_ID] = mule;
        args[ARG_ASCII_COMPATIBLE_P] = self.bool(ascii_compatible);
        args[ARG_SUPPLEMENTARY_P] = self.bool(supplementary);
        args[ARG_CODE_OFFSET] = Value::fixnum(code_offset as i64);
        args[ARG_PLIST] = plist;
        f_define_charset_internal(self, &args).expect("builtin charset");
        self.stack.truncate(self.stack.len() - 2);
        self.check_charset_get_id(name).expect("charset id")
    }

    fn cons_to_unsigned(&mut self, v: Value) -> LispResult<u32> {
        let n = if let Some(n) = v.as_fixnum() {
            n
        } else if v.is_cons() {
            let hi = self.xcar(v).as_fixnum();
            let lo_v = self.xcdr(v);
            let lo = if lo_v.is_cons() {
                self.xcar(lo_v).as_fixnum()
            } else {
                lo_v.as_fixnum()
            };
            match (hi, lo) {
                (Some(h), Some(l)) if (0..1 << 16).contains(&l) => (h << 16) | l,
                _ => return Err(self.signal_list(sym::ARGS_OUT_OF_RANGE, &[v])),
            }
        } else if let Some(b) = self.bignum_ref(v) {
            use num_traits::ToPrimitive;
            b.to_i64().unwrap_or(-1)
        } else {
            return Err(self.wrong_type(sym::INTEGERP, v));
        };
        if !(0..=u32::MAX as i64).contains(&n) {
            let hi = self.integer(u32::MAX as i64);
            return Err(self.args_out_of_range(v, hi));
        }
        Ok(n as u32)
    }

    fn check_integer_range(&mut self, v: Value, lo: i64, hi: i64) -> LispResult<i64> {
        let n = self.check_fixnum(v)?;
        if n < lo || n > hi {
            let l = Value::fixnum(lo);
            let h = Value::fixnum(hi);
            return Err(self.signal_list(sym::ARGS_OUT_OF_RANGE, &[v, l, h]));
        }
        Ok(n)
    }

    fn check_iso_charset_parameter(
        &mut self,
        dimension: Value,
        chars: Value,
        final_char: Value,
    ) -> LispResult<bool> {
        let d = self.check_fixnum(dimension)?;
        let ch = self.check_fixnum(chars)?;
        let f = self.check_character(final_char)?;
        if !(1..=3).contains(&d) {
            return Err(self.error(&format!("Invalid DIMENSION {d}, it should be 1, 2, or 3")));
        }
        let chars_flag = ch == 96;
        if !(chars_flag || ch == 94) {
            return Err(self.error(&format!("Invalid CHARS {ch}, it should be 94 or 96")));
        }
        if !(b'0' as u32..=b'~' as u32).contains(&f) {
            let c = char::from_u32(f).unwrap_or('?');
            return Err(self.error(&format!("Invalid FINAL-CHAR `{c}', it should be `0'..`~'")));
        }
        Ok(chars_flag)
    }

    fn map_char_table_for_charset(
        &mut self,
        function: Value,
        table: Value,
        arg: Value,
        partial: Option<(i32, u32, u32)>,
    ) -> LispResult<()> {
        let ranges = self.map_char_table_collect(table);
        let base = self.stack.len();
        self.stack.push(table);
        let mut pending: Option<(u32, u32)> = None;
        let emit = |lisp: &mut Lisp, from: u32, to: u32| -> LispResult<()> {
            let range = lisp.cons(Value::fixnum(from as i64), Value::fixnum(to as i64));
            lisp.funcall(function, &[range, arg])?;
            Ok(())
        };
        let mut result = Ok(());
        'outer: for (from, to, val) in ranges {
            for c in from..=to {
                let include = match partial {
                    None => !val.is_nil(),
                    Some((id, lo, hi)) => {
                        let v = self.char_table_ref(table, c);
                        if v.is_nil() {
                            false
                        } else {
                            let code = match self.encode_char_fast(id, c as i32) {
                                Ok(code) => code,
                                Err(e) => {
                                    result = Err(e);
                                    break 'outer;
                                }
                            };
                            code >= lo && code <= hi
                        }
                    }
                };
                if partial.is_none() {
                    if include {
                        match pending {
                            Some((s, e)) if e + 1 == from => pending = Some((s, to)),
                            Some((s, e)) => {
                                if let Err(err) = emit(self, s, e) {
                                    result = Err(err);
                                    break 'outer;
                                }
                                pending = Some((from, to));
                            }
                            None => pending = Some((from, to)),
                        }
                    }
                    break;
                }
                if include {
                    match pending {
                        Some((s, e)) if e + 1 == c => pending = Some((s, c)),
                        Some((s, e)) => {
                            if let Err(err) = emit(self, s, e) {
                                result = Err(err);
                                break 'outer;
                            }
                            pending = Some((c, c));
                        }
                        None => pending = Some((c, c)),
                    }
                }
            }
        }
        if result.is_ok()
            && let Some((s, e)) = pending
        {
            result = emit(self, s, e);
        }
        self.stack.truncate(base);
        result
    }

    fn map_charset_chars(
        &mut self,
        function: Value,
        arg: Value,
        id: i32,
        from: u32,
        to: u32,
    ) -> LispResult<()> {
        let cs = self.charset(id).clone();
        let partial = from > cs.min_code || to < cs.max_code;
        match cs.method {
            Method::Offset => {
                let from_c = cs.code_point_to_index(from) as i32 + cs.code_offset;
                let to_c = cs.code_point_to_index(to) as i32 + cs.code_offset;
                if cs.unified_p {
                    if !self.is_char_table(self.charset_attr(id, ATTR_DEUNIFIER)) {
                        self.load_charset(id, 2)?;
                    }
                    let deunifier = self.charset_attr(id, ATTR_DEUNIFIER);
                    if self.is_char_table(deunifier) {
                        let p = partial.then_some((id, from, to));
                        self.map_char_table_for_charset(function, deunifier, arg, p)?;
                    }
                }
                let range = self.cons(Value::fixnum(from_c as i64), Value::fixnum(to_c as i64));
                self.funcall(function, &[range, arg])?;
            }
            Method::Map => {
                if !self.is_char_table(self.charset_attr(id, ATTR_ENCODER)) {
                    self.load_charset(id, 2)?;
                }
                let encoder = self.charset_attr(id, ATTR_ENCODER);
                if self.is_char_table(encoder) {
                    let p = partial.then_some((id, from, to));
                    self.map_char_table_for_charset(function, encoder, arg, p)?;
                }
            }
            Method::Subset => {
                let info = self.charset_attr(id, ATTR_SUBSET);
                let items = self.vector_items(info).to_vec();
                let parent = items[0].as_fixnum().unwrap_or(0) as i32;
                let offset = items[3].as_fixnum().unwrap_or(0) as u32;
                let mut f = from.wrapping_sub(offset);
                let lo = items[1].as_fixnum().unwrap_or(0) as u32;
                if f < lo {
                    f = lo;
                }
                let mut t = to.wrapping_sub(offset);
                let hi = items[2].as_fixnum().unwrap_or(0) as u32;
                if t > hi {
                    t = hi;
                }
                self.map_charset_chars(function, arg, parent, f, t)?;
            }
            Method::Superset => {
                let mut parents = self.charset_attr(id, ATTR_SUPERSET);
                while parents.is_cons() {
                    let elt = self.xcar(parents);
                    let pid = self.xcar(elt).as_fixnum().unwrap_or(0) as i32;
                    let offset = self.xcdr(elt).as_fixnum().unwrap_or(0);
                    let mut this_from = if from as i64 > offset {
                        (from as i64 - offset) as u32
                    } else {
                        0
                    };
                    let mut this_to = if to as i64 > offset {
                        (to as i64 - offset) as u32
                    } else {
                        0
                    };
                    let pc = self.charset(pid);
                    if this_from < pc.min_code {
                        this_from = pc.min_code;
                    }
                    if this_to > pc.max_code {
                        this_to = pc.max_code;
                    }
                    self.map_charset_chars(function, arg, pid, this_from, this_to)?;
                    parents = self.xcdr(parents);
                }
            }
        }
        Ok(())
    }

    fn find_charsets_in_text(
        &mut self,
        chars: &[u32],
        ascii_only_multibyte: bool,
        multibyte: bool,
        found: &mut [bool],
        table: Value,
    ) -> LispResult<()> {
        if ascii_only_multibyte {
            if multibyte {
                found[self.charsets.ascii as usize] = true;
            } else {
                for &c in chars {
                    let c = if table.is_nil() {
                        c
                    } else {
                        self.translate_char(table, c)
                    };
                    if c < 0x80 {
                        found[self.charsets.ascii as usize] = true;
                    } else {
                        found[self.charsets.eight_bit as usize] = true;
                    }
                }
            }
            return Ok(());
        }
        for &c in chars {
            let c = if table.is_nil() {
                c
            } else {
                self.translate_char(table, c)
            };
            let id = self.char_charset_id(c as i32)?;
            found[id as usize] = true;
        }
        Ok(())
    }

    pub fn translate_char(&self, table: Value, c: u32) -> u32 {
        if self.is_char_table(table) {
            return match self.char_table_ref(table, c).as_fixnum() {
                Some(n) if (0..=MAX_CHAR as i64).contains(&n) => n as u32,
                _ => c,
            };
        }
        let mut c = c;
        let mut tail = table;
        while tail.is_cons() {
            c = self.translate_char(self.xcar(tail), c);
            tail = self.xcdr(tail);
        }
        c
    }

    fn charsets_found_list(&mut self, found: &[bool]) -> Value {
        let mut val = Value::NIL;
        for i in (0..self.charsets.table.len()).rev() {
            if found[i] {
                let name = self.charset_name(i as i32);
                val = self.cons(name, val);
            }
        }
        val
    }
}

fn f_charsetp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = lisp.charsetp(args[0]);
    Ok(lisp.bool(v))
}

fn f_map_charset_chars(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let function = args[0];
    let id = lisp.check_charset_get_id(args[1])?;
    let arg = args.get(2).copied().unwrap_or(Value::NIL);
    let from_code = args.get(3).copied().unwrap_or(Value::NIL);
    let to_code = args.get(4).copied().unwrap_or(Value::NIL);
    let (min_code, max_code) = {
        let c = lisp.charset(id);
        (c.min_code, c.max_code)
    };
    let from = if from_code.is_nil() {
        min_code
    } else {
        (lisp.check_fixnat(from_code)? as u32).max(min_code)
    };
    let to = if to_code.is_nil() {
        max_code
    } else {
        (lisp.check_fixnat(to_code)? as u32).min(max_code)
    };
    lisp.map_charset_chars(function, arg, id, from, to)?;
    Ok(Value::NIL)
}

fn f_define_charset_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args.len() != ARG_MAX {
        return Err(lisp.signal_list(
            sym::WRONG_NUMBER_OF_ARGUMENTS,
            &[
                sym::DEFINE_CHARSET_INTERNAL,
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let attrs = lisp.vector(vec![Value::NIL; ATTR_MAX]);
    let base = lisp.stack.len();
    lisp.stack.push(attrs);
    let r = define_charset_internal_1(lisp, args, attrs);
    lisp.stack.truncate(base);
    r
}

fn define_charset_internal_1(lisp: &mut Lisp, args: &[Value], attrs: Value) -> LispResult {
    let name = lisp.check_symbol(args[ARG_NAME])?;
    lisp.aset_value(attrs, Value::fixnum(ATTR_NAME as i64), name)?;
    let mut cs = Charset::empty();
    let space = args[ARG_CODE_SPACE];
    let mut dimension = 0;
    let mut nchars: i32 = 1;
    let mut i = 0usize;
    loop {
        let min_obj = lisp.aref_value(space, Value::fixnum((i * 2) as i64))?;
        let max_obj = lisp.aref_value(space, Value::fixnum((i * 2 + 1) as i64))?;
        let min_byte = lisp.check_integer_range(min_obj, 0, 255)? as i32;
        let max_byte = lisp.check_integer_range(max_obj, min_byte as i64, 255)? as i32;
        cs.code_space[i * 4] = min_byte;
        cs.code_space[i * 4 + 1] = max_byte;
        cs.code_space[i * 4 + 2] = max_byte - min_byte + 1;
        if max_byte > 0 {
            dimension = i as i32 + 1;
        }
        if i == 3 {
            break;
        }
        nchars *= cs.code_space[i * 4 + 2];
        cs.code_space[i * 4 + 3] = nchars;
        i += 1;
    }
    let val = args[ARG_DIMENSION];
    cs.dimension = if val.is_nil() {
        dimension
    } else {
        lisp.check_integer_range(val, 1, 4)? as i32
    };
    cs.code_linear_p = cs.dimension == 1
        || (cs.code_space[2] == 256
            && (cs.dimension == 2
                || (cs.code_space[6] == 256 && (cs.dimension == 3 || cs.code_space[10] == 256))));
    if !cs.code_linear_p {
        let mut mask = Box::new([0u8; 256]);
        for i in 0..4 {
            for j in cs.code_space[i * 4]..=cs.code_space[i * 4 + 1] {
                mask[j as usize] |= 1 << i;
            }
        }
        cs.code_space_mask = Some(mask);
    }
    cs.iso_chars_96 = cs.code_space[2] == 96;
    cs.min_code = cs.code_space[0] as u32
        | (cs.code_space[4] as u32) << 8
        | (cs.code_space[8] as u32) << 16
        | (cs.code_space[12] as u32) << 24;
    cs.max_code = cs.code_space[1] as u32
        | (cs.code_space[5] as u32) << 8
        | (cs.code_space[9] as u32) << 16
        | (cs.code_space[13] as u32) << 24;
    cs.char_index_offset = 0;
    let val = args[ARG_MIN_CODE];
    if !val.is_nil() {
        let code = lisp.cons_to_unsigned(val)?;
        if code < cs.min_code || code > cs.max_code {
            let lo = lisp.integer(cs.min_code as i64);
            let hi = lisp.integer(cs.max_code as i64);
            return Err(lisp.signal_list(sym::ARGS_OUT_OF_RANGE, &[lo, hi, val]));
        }
        cs.char_index_offset = cs.code_point_to_index(code) as u32;
        cs.min_code = code;
    }
    let val = args[ARG_MAX_CODE];
    if !val.is_nil() {
        let code = lisp.cons_to_unsigned(val)?;
        if code < cs.min_code || code > cs.max_code {
            let lo = lisp.integer(cs.min_code as i64);
            let hi = lisp.integer(cs.max_code as i64);
            return Err(lisp.signal_list(sym::ARGS_OUT_OF_RANGE, &[lo, hi, val]));
        }
        cs.max_code = code;
    }
    cs.compact_codes_p = cs.max_code < 0x10000;
    let val = args[ARG_INVALID_CODE];
    if val.is_nil() {
        if cs.min_code > 0 {
            cs.invalid_code = 0;
        } else if cs.max_code < u32::MAX {
            cs.invalid_code = cs.max_code + 1;
        } else {
            return Err(lisp.error("Attribute :invalid-code must be specified"));
        }
    } else {
        cs.invalid_code = lisp.cons_to_unsigned(val)?;
    }
    let val = args[ARG_ISO_FINAL];
    if val.is_nil() {
        cs.iso_final = -1;
    } else {
        let n = lisp.check_fixnum(val)?;
        if n < b'0' as i64 || n > 127 {
            return Err(lisp.error(&format!("Invalid iso-final-char: {n}")));
        }
        cs.iso_final = n as i32;
    }
    let val = args[ARG_ISO_REVISION];
    cs.iso_revision = if val.is_nil() {
        -1
    } else {
        lisp.check_integer_range(val, -1, 63)? as i32
    };
    let val = args[ARG_EMACS_MULE_ID];
    if val.is_nil() {
        cs.emacs_mule_id = -1;
    } else {
        let n = lisp.check_fixnat(val)?;
        if (n > 0 && n <= 128) || n >= 256 {
            return Err(lisp.error(&format!("Invalid emacs-mule-id: {n}")));
        }
        cs.emacs_mule_id = n as i32;
    }
    cs.ascii_compatible_p = !args[ARG_ASCII_COMPATIBLE_P].is_nil();
    cs.supplementary_p = !args[ARG_SUPPLEMENTARY_P].is_nil();
    cs.unified_p = false;
    if !args[ARG_CODE_OFFSET].is_nil() {
        let offset = lisp.check_character(args[ARG_CODE_OFFSET])? as i32;
        cs.method = Method::Offset;
        cs.code_offset = offset;
        let i = cs.code_point_to_index(cs.max_code);
        if MAX_CHAR as i64 - (offset as i64) < i {
            return Err(lisp.error(&format!(
                "Unsupported max char: {i} + {} > MAX_CHAR ({MAX_CHAR})",
                cs.max_code
            )));
        }
        cs.max_char = i as i32 + offset;
        cs.min_char = cs.code_point_to_index(cs.min_code) as i32 + offset;
        let mut i = (cs.min_char >> 7) << 7;
        while i < 0x10000 && i <= cs.max_char {
            cs.fast_map_set(i);
            i += 128;
        }
        i = (i >> 12) << 12;
        while i <= cs.max_char {
            cs.fast_map_set(i);
            i += 0x1000;
        }
        if offset == 0 && cs.max_char >= 0x80 {
            cs.ascii_compatible_p = true;
        }
    } else if !args[ARG_MAP].is_nil() {
        lisp.aset_value(attrs, Value::fixnum(ATTR_MAP as i64), args[ARG_MAP])?;
        cs.method = Method::Map;
    } else if !args[ARG_SUBSET].is_nil() {
        let val = args[ARG_SUBSET];
        let parent = lisp.car(val)?;
        let parent_id = lisp.check_charset_get_id(parent)?;
        let pmin = lisp.nth(1, val);
        lisp.check_fixnat(pmin)?;
        let pmax = lisp.nth(2, val);
        lisp.check_fixnat(pmax)?;
        let poff = lisp.nth(3, val);
        lisp.check_fixnum(poff)?;
        let info = lisp.vector(vec![Value::fixnum(parent_id as i64), pmin, pmax, poff]);
        lisp.aset_value(attrs, Value::fixnum(ATTR_SUBSET as i64), info)?;
        cs.method = Method::Subset;
        let p = lisp.charset(parent_id);
        cs.fast_map = p.fast_map;
        cs.min_char = p.min_char;
        cs.max_char = p.max_char;
    } else if !args[ARG_SUPERSET].is_nil() {
        cs.method = Method::Superset;
        let val = crate::fns::f_copy_sequence(lisp, &[args[ARG_SUPERSET]])?;
        lisp.aset_value(attrs, Value::fixnum(ATTR_SUPERSET as i64), val)?;
        cs.min_char = MAX_CHAR as i32;
        cs.max_char = 0;
        let mut tail = val;
        while !tail.is_nil() {
            let elt = lisp.car(tail)?;
            let (this_id, offset) = if elt.is_cons() {
                let id = lisp.check_charset_get_id(lisp.xcar(elt))?;
                let off =
                    lisp.check_integer_range(lisp.xcdr(elt), i32::MIN as i64, i32::MAX as i64)?;
                (id, off)
            } else {
                (lisp.check_charset_get_id(elt)?, 0)
            };
            let pair = lisp.cons(Value::fixnum(this_id as i64), Value::fixnum(offset));
            lisp.heap.set_car(tail, pair);
            let this = lisp.charset(this_id);
            if cs.min_char > this.min_char {
                cs.min_char = this.min_char;
            }
            if cs.max_char < this.max_char {
                cs.max_char = this.max_char;
            }
            for i in 0..190 {
                cs.fast_map[i] |= this.fast_map[i];
            }
            tail = lisp.cdr(tail)?;
        }
    } else {
        return Err(lisp.error("None of :code-offset, :map, :parents are specified"));
    }
    let val = args[ARG_UNIFY_MAP];
    if !val.is_nil() && !val.is_string() && !lisp.is_vector(val) {
        return Err(lisp.wrong_type(sym::VECTORP, val));
    }
    lisp.aset_value(attrs, Value::fixnum(ATTR_UNIFY_MAP as i64), val)?;
    lisp.check_list(args[ARG_PLIST])?;
    lisp.aset_value(attrs, Value::fixnum(ATTR_PLIST as i64), args[ARG_PLIST])?;
    let table = lisp.charsets.hash_table;
    let existing = lisp.gethash(name, table, Value::NIL)?;
    let new_definition_p;
    let id;
    if !existing.is_nil() {
        new_definition_p = false;
        id = lisp.vector_items(existing)[ATTR_ID]
            .as_fixnum()
            .unwrap_or(0) as i32;
        lisp.puthash(name, attrs, table)?;
    } else {
        lisp.puthash(name, attrs, table)?;
        id = lisp.charsets.table.len() as i32;
        lisp.charsets.table.push(Charset::empty());
        lisp.charsets.attributes.push(Value::NIL);
        new_definition_p = true;
    }
    lisp.aset_value(
        attrs,
        Value::fixnum(ATTR_ID as i64),
        Value::fixnum(id as i64),
    )?;
    cs.id = id;
    lisp.charsets.table[id as usize] = cs.clone();
    lisp.charsets.attributes[id as usize] = attrs;
    if cs.method == Method::Map {
        lisp.load_charset(id, 0)?;
    }
    if cs.iso_final >= 0 {
        let idx = iso_index(cs.dimension, cs.iso_chars_96, cs.iso_final);
        lisp.charsets.iso_charset_table[idx] = id;
        if new_definition_p {
            let cell = lisp.list(&[Value::fixnum(id as i64)]);
            let l = lisp.charsets.iso_2022_charset_list;
            lisp.charsets.iso_2022_charset_list = lisp.nconc2(l, cell);
        }
        let c = &lisp.charsets;
        if c.iso_charset(1, false, b'J' as i32) == id {
            lisp.charsets.jisx0201_roman = id;
        } else if c.iso_charset(2, false, b'@' as i32) == id {
            lisp.charsets.jisx0208_1978 = id;
        } else if c.iso_charset(2, false, b'B' as i32) == id {
            lisp.charsets.jisx0208 = id;
        } else if c.iso_charset(2, false, b'C' as i32) == id {
            lisp.charsets.ksc5601 = id;
        }
    }
    if cs.emacs_mule_id >= 0 {
        let mid = cs.emacs_mule_id as usize;
        lisp.charsets.emacs_mule_charset[mid] = id;
        lisp.charsets.emacs_mule_bytes[mid] = if mid < 0xA0 {
            cs.dimension + 1
        } else {
            cs.dimension + 2
        };
        if new_definition_p {
            let cell = lisp.list(&[Value::fixnum(id as i64)]);
            let l = lisp.charsets.emacs_mule_charset_list;
            lisp.charsets.emacs_mule_charset_list = lisp.nconc2(l, cell);
        }
    }
    if new_definition_p {
        let list = lisp.bound_var(sym::CHARSET_LIST);
        let list = lisp.cons(name, list);
        lisp.set_plain_value(sym::CHARSET_LIST, list);
        if cs.supplementary_p {
            let cell = lisp.list(&[Value::fixnum(id as i64)]);
            let l = lisp.charsets.ordered_list;
            lisp.charsets.ordered_list = lisp.nconc2(l, cell);
        } else {
            let mut tail = lisp.charsets.ordered_list;
            while tail.is_cons() {
                let cid = lisp.xcar(tail).as_fixnum().unwrap_or(0) as i32;
                if lisp.charset(cid).supplementary_p {
                    break;
                }
                tail = lisp.xcdr(tail);
            }
            if tail == lisp.charsets.ordered_list {
                let l = lisp.charsets.ordered_list;
                lisp.charsets.ordered_list = lisp.cons(Value::fixnum(id as i64), l);
            } else if tail.is_nil() {
                let cell = lisp.list(&[Value::fixnum(id as i64)]);
                let l = lisp.charsets.ordered_list;
                lisp.charsets.ordered_list = lisp.nconc2(l, cell);
            } else {
                let rest = lisp.cons(lisp.xcar(tail), lisp.xcdr(tail));
                lisp.heap.set_cdr(tail, rest);
                lisp.heap.set_car(tail, Value::fixnum(id as i64));
            }
        }
        lisp.charsets.ordered_list_tick += 1;
    }
    Ok(Value::NIL)
}

fn f_define_charset_alias(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let attrs = lisp.check_charset_get_attr(args[1])?;
    let table = lisp.charsets.hash_table;
    lisp.puthash(args[0], attrs, table)?;
    let list = lisp.bound_var(sym::CHARSET_LIST);
    let list = lisp.cons(args[0], list);
    lisp.set_plain_value(sym::CHARSET_LIST, list);
    Ok(Value::NIL)
}

fn f_charset_plist(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let attrs = lisp.check_charset_get_attr(args[0])?;
    Ok(lisp.vector_items(attrs)[ATTR_PLIST])
}

fn f_set_charset_plist(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let attrs = lisp.check_charset_get_attr(args[0])?;
    lisp.aset_value(attrs, Value::fixnum(ATTR_PLIST as i64), args[1])?;
    Ok(args[1])
}

fn f_unify_charset(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let charset = args[0];
    let mut unify_map = args.get(1).copied().unwrap_or(Value::NIL);
    let deunify = args.get(2).copied().unwrap_or(Value::NIL);
    let id = lisp.check_charset_get_id(charset)?;
    let cs = lisp.charset(id).clone();
    let skip = if deunify.is_nil() {
        cs.unified_p && !lisp.charset_attr(id, ATTR_DEUNIFIER).is_nil()
    } else {
        !cs.unified_p
    };
    if skip {
        return Ok(Value::NIL);
    }
    lisp.charsets.table[id as usize].unified_p = false;
    if deunify.is_nil() {
        if cs.method != Method::Offset || cs.code_offset < 0x110000 {
            let name = lisp.symbol_name_string(charset);
            return Err(lisp.error(&format!("Can't unify charset: {name}")));
        }
        if unify_map.is_nil() {
            unify_map = lisp.charset_attr(id, ATTR_UNIFY_MAP);
        } else {
            if !unify_map.is_string() && !lisp.is_vector(unify_map) {
                let msg = lisp.string("Bad unify-map");
                return Err(lisp.signal_list(sym::ERROR, &[msg, unify_map]));
            }
            lisp.set_charset_attr(id, ATTR_UNIFY_MAP, unify_map);
        }
        let _ = unify_map;
        if lisp.charsets.char_unify_table.is_nil() {
            lisp.charsets.char_unify_table = lisp.make_char_table(Value::NIL, Value::NIL)?;
        }
        let t = lisp.charsets.char_unify_table;
        lisp.char_table_set_range(t, cs.min_char as u32, cs.max_char as u32, charset);
        lisp.charsets.table[id as usize].unified_p = true;
    } else if lisp.is_char_table(lisp.charsets.char_unify_table) {
        let min_char = lisp.decode_char_fast(id, cs.min_code)?;
        let max_char = lisp.decode_char_fast(id, cs.max_code)?;
        let t = lisp.charsets.char_unify_table;
        lisp.char_table_set_range(t, min_char as u32, max_char as u32, Value::NIL);
    }
    Ok(Value::NIL)
}

fn f_get_unused_iso_final_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let chars_flag =
        lisp.check_iso_charset_parameter(args[0], args[1], Value::fixnum(b'0' as i64))?;
    let d = args[0].as_fixnum().unwrap_or(1) as i32;
    for final_char in b'0'..=b'?' {
        if lisp.charsets.iso_charset(d, chars_flag, final_char as i32) < 0 {
            return Ok(Value::fixnum(final_char as i64));
        }
    }
    Ok(Value::NIL)
}

fn f_declare_equiv_charset(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let id = lisp.check_charset_get_id(args[3])?;
    let chars_flag = lisp.check_iso_charset_parameter(args[0], args[1], args[2])?;
    let d = args[0].as_fixnum().unwrap_or(1) as i32;
    let f = args[2].as_fixnum().unwrap_or(0) as i32;
    lisp.charsets.iso_charset_table[iso_index(d, chars_flag, f)] = id;
    Ok(Value::NIL)
}

fn f_find_charset_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (beg, end) = (args[0], args[1]);
    let table = args.get(2).copied().unwrap_or(Value::NIL);
    let mut from = lisp.position_arg(beg)?;
    let mut to = lisp.position_arg(end)?;
    if from > to {
        std::mem::swap(&mut from, &mut to);
    }
    let (begv, zv) = (lisp.cur().begv as i64, lisp.cur().zv as i64);
    if from < begv || to > zv {
        return Err(lisp.args_out_of_range(beg, end));
    }
    let multibyte = lisp.cur().text.multibyte;
    let mut chars = Vec::new();
    let mut ascii_like = true;
    {
        let text = &lisp.cur().text;
        let mut byte = text.char_to_byte(from as usize);
        let end_byte = text.char_to_byte(to as usize);
        while byte < end_byte {
            let len = text.char_len_at_byte(byte);
            if len > 1 {
                ascii_like = false;
            }
            chars.push(text.char_at_byte(byte));
            byte += len;
        }
    }
    if multibyte && chars.iter().any(|&c| c >= 0x80) {
        ascii_like = false;
    }
    let mut found = vec![false; lisp.charsets.table.len()];
    lisp.find_charsets_in_text(&chars, ascii_like, multibyte, &mut found, table)?;
    Ok(lisp.charsets_found_list(&found))
}

fn f_find_charset_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    let table = args.get(1).copied().unwrap_or(Value::NIL);
    let multibyte = lisp.string_multibyte(s);
    let ascii_like = lisp.string_chars(s) == lisp.string_bytes(s).len();
    let chars = lisp.string_char_vec(s);
    let mut found = vec![false; lisp.charsets.table.len()];
    lisp.find_charsets_in_text(&chars, ascii_like, multibyte, &mut found, table)?;
    Ok(lisp.charsets_found_list(&found))
}

fn f_decode_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let id = lisp.check_charset_get_id(args[0])?;
    let code = lisp.cons_to_unsigned(args[1])?;
    let c = lisp.decode_char_fast(id, code)?;
    Ok(if c >= 0 {
        Value::fixnum(c as i64)
    } else {
        Value::NIL
    })
}

fn f_encode_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let id = lisp.check_charset_get_id(args[1])?;
    let c = lisp.check_character(args[0])?;
    let code = lisp.encode_char_fast(id, c as i32)?;
    if code == lisp.charset(id).invalid_code {
        return Ok(Value::NIL);
    }
    Ok(lisp.integer(code as i64))
}

fn f_split_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])? as i32;
    let id = lisp.char_charset_id(c)?;
    let mut code = lisp.encode_char_fast(id, c)?;
    let dimension = lisp.charset(id).dimension;
    let mut val = Value::NIL;
    for _ in 0..dimension {
        val = lisp.cons(Value::fixnum((code & 0xFF) as i64), val);
        code >>= 8;
    }
    let name = lisp.charset_name(id);
    Ok(lisp.cons(name, val))
}

fn f_make_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let id = lisp.check_charset_get_id(args[0])?;
    let arg = |i: usize| args.get(i).copied().unwrap_or(Value::NIL);
    let cs = lisp.charset(id).clone();
    let dimension = cs.dimension;
    let byte = |lisp: &mut Lisp, v: Value| -> LispResult<u32> {
        let n = lisp.check_fixnat(v)?;
        if n >= 0x100 {
            return Err(lisp.args_out_of_range(Value::fixnum(0xFF), v));
        }
        Ok(n as u32)
    };
    let mut code: u32;
    if arg(1).is_nil() {
        code = if cs.ascii_compatible_p {
            0
        } else {
            cs.min_code
        };
    } else {
        code = byte(lisp, arg(1))?;
        if dimension > 1 {
            code <<= 8;
            if arg(2).is_nil() {
                code |= cs.code_space[((dimension - 2) * 4) as usize] as u32;
            } else {
                code |= byte(lisp, arg(2))?;
            }
            if dimension > 2 {
                code <<= 8;
                if arg(3).is_nil() {
                    code |= cs.code_space[((dimension - 3) * 4) as usize] as u32;
                } else {
                    code |= byte(lisp, arg(3))?;
                }
                if dimension > 3 {
                    code <<= 8;
                    if arg(4).is_nil() {
                        code |= cs.code_space[0] as u32;
                    } else {
                        code |= byte(lisp, arg(4))?;
                    }
                }
            }
        }
    }
    if cs.iso_final >= 0 {
        code &= 0x7F7F7F7F;
    }
    let c = lisp.decode_char_fast(id, code)?;
    if c < 0 {
        return Err(lisp.error("Invalid code(s)"));
    }
    Ok(Value::fixnum(c as i64))
}

fn f_char_charset(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])? as i32;
    let restriction = args.get(1).copied().unwrap_or(Value::NIL);
    let id = if restriction.is_nil() {
        lisp.char_charset_id(c)?
    } else if restriction.is_cons() {
        let mut tail = restriction;
        while tail.is_cons() {
            let item = lisp.xcar(tail);
            let rid = lisp.check_charset_get_id(item)?;
            if lisp.encode_char_fast(rid, c)? != lisp.charset(rid).invalid_code {
                return Ok(item);
            }
            tail = lisp.xcdr(tail);
        }
        return Ok(Value::NIL);
    } else {
        let list = lisp.coding_system_charset_list(restriction)?;
        match lisp.char_charset(c, list)? {
            Some((id, _)) => id,
            None => return Ok(Value::NIL),
        }
    };
    Ok(lisp.charset_name(id))
}

fn f_charset_after(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = args.first().copied().unwrap_or(Value::NIL);
    let f = lisp.intern("char-after");
    let ch = lisp.funcall(f, &[pos])?;
    let Some(c) = ch.as_fixnum() else {
        return Ok(ch);
    };
    let id = lisp.char_charset_id(c as i32)?;
    Ok(lisp.charset_name(id))
}

fn f_iso_charset(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let chars_flag = lisp.check_iso_charset_parameter(args[0], args[1], args[2])?;
    let d = args[0].as_fixnum().unwrap_or(1) as i32;
    let f = args[2].as_fixnum().unwrap_or(0) as i32;
    let id = lisp.charsets.iso_charset(d, chars_flag, f);
    Ok(if id >= 0 {
        lisp.charset_name(id)
    } else {
        Value::NIL
    })
}

fn f_clear_charset_maps(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_charset_priority_list(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let highestp = args.first().copied().unwrap_or(Value::NIL);
    let list = lisp.charsets.ordered_list;
    if !highestp.is_nil() {
        let first = lisp.car(list)?;
        return Ok(lisp.charset_name(first.as_fixnum().unwrap_or(0) as i32));
    }
    let mut names = Vec::new();
    let mut tail = list;
    while tail.is_cons() {
        let id = lisp.xcar(tail).as_fixnum().unwrap_or(0) as i32;
        names.push(lisp.charset_name(id));
        tail = lisp.xcdr(tail);
    }
    Ok(lisp.list(&names))
}

fn f_set_charset_priority(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut ids = Vec::new();
    for &a in args {
        ids.push(lisp.check_charset_get_id(a)?);
    }
    let mut old: Vec<i32> = Vec::new();
    let mut tail = lisp.charsets.ordered_list;
    while tail.is_cons() {
        old.push(lisp.xcar(tail).as_fixnum().unwrap_or(0) as i32);
        tail = lisp.xcdr(tail);
    }
    let mut head = Vec::new();
    for id in ids {
        if let Some(pos) = old.iter().position(|&x| x == id) {
            old.remove(pos);
            head.push(id);
        }
    }
    let old_list = lisp.list(
        &old.iter()
            .map(|&i| Value::fixnum(i as i64))
            .collect::<Vec<_>>(),
    );
    lisp.charsets.non_preferred_head = old_list;
    let head_values: Vec<Value> = head.iter().map(|&i| Value::fixnum(i as i64)).collect();
    lisp.charsets.ordered_list = lisp.list_with_tail(&head_values, old_list);
    lisp.charsets.ordered_list_tick += 1;
    lisp.charsets.unibyte = -1;
    let mut list_2022 = Vec::new();
    let mut list_mule = Vec::new();
    let mut tail = lisp.charsets.ordered_list;
    while tail.is_cons() {
        let item = lisp.xcar(tail);
        if !lisp
            .memq(item, lisp.charsets.iso_2022_charset_list)
            .is_nil()
        {
            list_2022.push(item);
        }
        if !lisp
            .memq(item, lisp.charsets.emacs_mule_charset_list)
            .is_nil()
        {
            list_mule.push(item);
        }
        if lisp.charsets.unibyte < 0 {
            let id = item.as_fixnum().unwrap_or(0) as i32;
            let cs = lisp.charset(id);
            if cs.dimension == 1 && cs.ascii_compatible_p && cs.max_char >= 0x80 {
                lisp.charsets.unibyte = id;
            }
        }
        tail = lisp.xcdr(tail);
    }
    lisp.charsets.iso_2022_charset_list = lisp.list(&list_2022);
    lisp.charsets.emacs_mule_charset_list = lisp.list(&list_mule);
    if lisp.charsets.unibyte < 0 {
        lisp.charsets.unibyte = lisp.charsets.iso_8859_1;
    }
    Ok(Value::NIL)
}

fn f_charset_id_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let charset = args.first().copied().unwrap_or(Value::NIL);
    let id = lisp.check_charset_get_id(charset)?;
    Ok(Value::fixnum(id as i64))
}

fn f_sort_charsets(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let charsets = args[0];
    let items = lisp.list_to_vec(charsets)?;
    if items.is_empty() {
        return Ok(Value::NIL);
    }
    let mut data: Vec<(Value, i32, usize)> = Vec::with_capacity(items.len());
    for item in items {
        let id = lisp.check_charset_get_id(item)?;
        data.push((item, id, 0));
    }
    let mut tail = lisp.charsets.ordered_list;
    let mut i = 0usize;
    while tail.is_cons() {
        let id = lisp.xcar(tail).as_fixnum().unwrap_or(0) as i32;
        for d in data.iter_mut() {
            if d.1 == id {
                d.2 = i;
            }
        }
        tail = lisp.xcdr(tail);
        i += 1;
    }
    data.sort_by_key(|d| d.2);
    let mut tail = charsets;
    for d in data {
        lisp.heap.set_car(tail, d.0);
        tail = lisp.xcdr(tail);
    }
    Ok(charsets)
}
