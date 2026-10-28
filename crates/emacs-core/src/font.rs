use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::Vectorlike;
use crate::symbols as sym;
use crate::value::Value;

pub const FONT_TYPE_INDEX: usize = 0;
pub const FONT_FOUNDRY_INDEX: usize = 1;
pub const FONT_FAMILY_INDEX: usize = 2;
pub const FONT_ADSTYLE_INDEX: usize = 3;
pub const FONT_REGISTRY_INDEX: usize = 4;
pub const FONT_WEIGHT_INDEX: usize = 5;
pub const FONT_SLANT_INDEX: usize = 6;
pub const FONT_WIDTH_INDEX: usize = 7;
pub const FONT_SIZE_INDEX: usize = 8;
pub const FONT_DPI_INDEX: usize = 9;
pub const FONT_SPACING_INDEX: usize = 10;
pub const FONT_AVGWIDTH_INDEX: usize = 11;
pub const FONT_EXTRA_INDEX: usize = 12;
pub const FONT_SPEC_MAX: usize = 13;
pub const FONT_OBJLIST_INDEX: usize = FONT_SPEC_MAX;
pub const FONT_ENTITY_MAX: usize = 14;
pub const FONT_NAME_INDEX: usize = FONT_ENTITY_MAX;
pub const FONT_FULLNAME_INDEX: usize = 15;
pub const FONT_FILE_INDEX: usize = 16;
pub const FONT_OBJECT_MAX: usize = 17;

pub const FONT_SPACING_PROPORTIONAL: i64 = 0;
pub const FONT_SPACING_DUAL: i64 = 90;
pub const FONT_SPACING_MONO: i64 = 100;
pub const FONT_SPACING_CHARCELL: i64 = 110;

pub const PT_PER_INCH: f64 = 72.27;

const XLFD_FOUNDRY_INDEX: usize = 0;
const XLFD_FAMILY_INDEX: usize = 1;
const XLFD_WEIGHT_INDEX: usize = 2;
const XLFD_SLANT_INDEX: usize = 3;
const XLFD_SWIDTH_INDEX: usize = 4;
const XLFD_ADSTYLE_INDEX: usize = 5;
const XLFD_PIXEL_INDEX: usize = 6;
const XLFD_POINT_INDEX: usize = 7;
const XLFD_RESX_INDEX: usize = 8;
const XLFD_RESY_INDEX: usize = 9;
const XLFD_SPACING_INDEX: usize = 10;
const XLFD_AVGWIDTH_INDEX: usize = 11;
const XLFD_REGISTRY_INDEX: usize = 12;
const XLFD_ENCODING_INDEX: usize = 13;
const XLFD_LAST_INDEX: usize = 14;

const XLFD_FOUNDRY_MASK: u32 = 0x0001;
const XLFD_FAMILY_MASK: u32 = 0x0002;
const XLFD_WEIGHT_MASK: u32 = 0x0004;
const XLFD_SLANT_MASK: u32 = 0x0008;
const XLFD_SWIDTH_MASK: u32 = 0x0010;
const XLFD_ADSTYLE_MASK: u32 = 0x0020;
const XLFD_PIXEL_MASK: u32 = 0x0040;
const XLFD_POINT_MASK: u32 = 0x0080;
const XLFD_RESX_MASK: u32 = 0x0100;
const XLFD_RESY_MASK: u32 = 0x0200;
const XLFD_SPACING_MASK: u32 = 0x0400;
const XLFD_AVGWIDTH_MASK: u32 = 0x0800;
const XLFD_REGISTRY_MASK: u32 = 0x1000;
const XLFD_ENCODING_MASK: u32 = 0x2000;
const XLFD_SYMBOL_MASK: u32 =
    XLFD_FOUNDRY_MASK | XLFD_FAMILY_MASK | XLFD_ADSTYLE_MASK | XLFD_REGISTRY_MASK;
const XLFD_NULL_MASK: u32 = XLFD_FOUNDRY_MASK | XLFD_ADSTYLE_MASK;
const XLFD_LARGENUM_MASK: u32 =
    XLFD_POINT_MASK | XLFD_RESX_MASK | XLFD_RESY_MASK | XLFD_AVGWIDTH_MASK;
const XLFD_REGENC_MASK: u32 = XLFD_REGISTRY_MASK | XLFD_ENCODING_MASK;

pub const WEIGHT_TABLE: &[(i64, &[&str])] = &[
    (0, &["thin"]),
    (
        40,
        &["ultra-light", "ultralight", "extra-light", "extralight"],
    ),
    (50, &["light"]),
    (55, &["semi-light", "semilight", "demilight"]),
    (80, &["regular", "normal", "unspecified", "book"]),
    (100, &["medium"]),
    (
        180,
        &["semi-bold", "semibold", "demibold", "demi-bold", "demi"],
    ),
    (200, &["bold"]),
    (205, &["extra-bold", "extrabold", "ultra-bold", "ultrabold"]),
    (210, &["black", "heavy"]),
    (250, &["ultra-heavy", "ultraheavy"]),
];

pub const SLANT_TABLE: &[(i64, &[&str])] = &[
    (0, &["reverse-oblique", "ro"]),
    (10, &["reverse-italic", "ri"]),
    (100, &["normal", "r", "unspecified"]),
    (200, &["italic", "i", "ot"]),
    (210, &["oblique", "o"]),
];

pub const WIDTH_TABLE: &[(i64, &[&str])] = &[
    (50, &["ultra-condensed", "ultracondensed"]),
    (63, &["extra-condensed", "extracondensed"]),
    (75, &["condensed", "compressed", "narrow"]),
    (87, &["semi-condensed", "semicondensed", "demicondensed"]),
    (100, &["normal", "medium", "regular", "unspecified"]),
    (113, &["semi-expanded", "semiexpanded", "demiexpanded"]),
    (125, &["expanded"]),
    (150, &["extra-expanded", "extraexpanded"]),
    (200, &["ultra-expanded", "ultraexpanded", "wide"]),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontMetrics {
    pub lbearing: i32,
    pub rbearing: i32,
    pub width: i32,
    pub ascent: i32,
    pub descent: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FontKind {
    Spec,
    Entity,
    Object,
}

#[derive(Clone, Debug)]
pub struct Font {
    pub kind: FontKind,
    pub props: Vec<Value>,
}

impl Font {
    pub fn trace(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&self.props);
    }
}

pub struct Fonts {
    pub style_table: Value,
    pub charset_alist: Value,
    pub sort_shift_bits: [i32; FONT_SIZE_INDEX + 1],
}

impl Default for Fonts {
    fn default() -> Self {
        let mut sort_shift_bits = [0; FONT_SIZE_INDEX + 1];
        sort_shift_bits[FONT_TYPE_INDEX] = 0;
        sort_shift_bits[FONT_SLANT_INDEX] = 2;
        sort_shift_bits[FONT_WEIGHT_INDEX] = 9;
        sort_shift_bits[FONT_SIZE_INDEX] = 16;
        sort_shift_bits[FONT_WIDTH_INDEX] = 23;
        Fonts {
            style_table: Value::NIL,
            charset_alist: Value::NIL,
            sort_shift_bits,
        }
    }
}

impl Fonts {
    pub fn font_update_sort_order(&mut self, order: &[i32; 4]) {
        let mut shift_bits = 23;
        for &xlfd_idx in order {
            if xlfd_idx == XLFD_WEIGHT_INDEX as i32 {
                self.sort_shift_bits[FONT_WEIGHT_INDEX] = shift_bits;
            } else if xlfd_idx == XLFD_SLANT_INDEX as i32 {
                self.sort_shift_bits[FONT_SLANT_INDEX] = shift_bits;
            } else if xlfd_idx == XLFD_SWIDTH_INDEX as i32 {
                self.sort_shift_bits[FONT_WIDTH_INDEX] = shift_bits;
            } else {
                self.sort_shift_bits[FONT_SIZE_INDEX] = shift_bits;
            }
            shift_bits -= 7;
        }
    }
}

impl Fonts {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.push(self.style_table);
        out.push(self.charset_alist);
    }
}

#[derive(Clone, Copy)]
enum Validator {
    Symbol,
    Style,
    NonNeg,
    Spacing,
    Otf,
}

const FONT_PROPERTY_TABLE: &[(Value, Validator)] = &[
    (sym::K_TYPE, Validator::Symbol),
    (sym::K_FOUNDRY, Validator::Symbol),
    (sym::K_FAMILY, Validator::Symbol),
    (sym::K_ADSTYLE, Validator::Symbol),
    (sym::K_REGISTRY, Validator::Symbol),
    (sym::K_WEIGHT, Validator::Style),
    (sym::K_SLANT, Validator::Style),
    (sym::K_WIDTH, Validator::Style),
    (sym::K_SIZE, Validator::NonNeg),
    (sym::K_DPI, Validator::NonNeg),
    (sym::K_SPACING, Validator::Spacing),
    (sym::K_AVGWIDTH, Validator::NonNeg),
    (sym::K_LANG, Validator::Symbol),
    (sym::K_SCRIPT, Validator::Symbol),
    (sym::K_OTF, Validator::Otf),
];

pub fn get_font_prop_index(key: Value) -> Option<usize> {
    FONT_PROPERTY_TABLE.iter().position(|&(k, _)| k == key)
}

fn parse_matrix(p: &[u8]) -> i64 {
    let text = String::from_utf8_lossy(&p[1..]);
    let mut rest: &str = &text;
    let mut matrix = [0.0f64; 4];
    let mut i = 0;
    while i < 4 && !rest.is_empty() && !rest.starts_with(']') {
        let negative = rest.starts_with('~');
        let body = if negative { &rest[1..] } else { rest };
        let (value, used) = strtod(body);
        matrix[i] = if negative { -value } else { value };
        let consumed = used + usize::from(negative);
        if consumed == 0 {
            break;
        }
        rest = &rest[consumed..];
        i += 1;
    }
    if i == 4 { matrix[3] as i64 } else { -1 }
}

fn valid_multibyte(s: &[u8]) -> bool {
    let mut i = 0;
    while i < s.len() {
        if s[i] < 0x80 {
            i += 1;
            continue;
        }
        match crate::character::multibyte_length(&s[i..]) {
            Some(n) => i += n,
            None => return false,
        }
    }
    true
}

fn strtod(s: &str) -> (f64, usize) {
    let bytes = s.as_bytes();
    let mut end = 0;
    while end < bytes.len() && bytes[end].is_ascii_whitespace() {
        end += 1;
    }
    let start = end;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        let mut e = end + 1;
        if e < bytes.len() && (bytes[e] == b'+' || bytes[e] == b'-') {
            e += 1;
        }
        if e < bytes.len() && bytes[e].is_ascii_digit() {
            while e < bytes.len() && bytes[e].is_ascii_digit() {
                e += 1;
            }
            end = e;
        }
    }
    match s[start..end].parse::<f64>() {
        Ok(v) => (v, end),
        Err(_) => (0.0, 0),
    }
}

impl Lisp {
    pub fn is_font(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Font(_))
    }

    pub fn font(&self, v: Value) -> &Font {
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Font(f) => f,
            _ => unreachable!("not a font"),
        }
    }

    pub fn font_mut(&mut self, v: Value) -> &mut Font {
        match self.heap.vectorlike_mut(v) {
            Vectorlike::Font(f) => f,
            _ => unreachable!("not a font"),
        }
    }

    pub fn font_spec_p(&self, v: Value) -> bool {
        self.is_font(v) && self.font(v).kind == FontKind::Spec
    }

    pub fn font_entity_p(&self, v: Value) -> bool {
        self.is_font(v) && self.font(v).kind == FontKind::Entity
    }

    pub fn font_object_p(&self, v: Value) -> bool {
        self.is_font(v) && self.font(v).kind == FontKind::Object
    }

    pub fn font_aref(&self, font: Value, idx: usize) -> Value {
        self.font(font).props[idx]
    }

    pub fn font_aset(&mut self, font: Value, idx: usize, val: Value) {
        self.font_mut(font).props[idx] = val;
    }

    pub fn font_make_spec(&mut self) -> Value {
        self.heap.vectorlike(Vectorlike::Font(Box::new(Font {
            kind: FontKind::Spec,
            props: vec![Value::NIL; FONT_SPEC_MAX],
        })))
    }

    pub fn font_make_entity(&mut self) -> Value {
        self.heap.vectorlike(Vectorlike::Font(Box::new(Font {
            kind: FontKind::Entity,
            props: vec![Value::NIL; FONT_ENTITY_MAX],
        })))
    }

    pub fn font_make_object(&mut self, entity: Value, pixelsize: i64) -> LispResult {
        let mut props = vec![Value::NIL; FONT_OBJECT_MAX];
        if !entity.is_nil() {
            props[1..FONT_SPEC_MAX].copy_from_slice(&self.font(entity).props[1..FONT_SPEC_MAX]);
            let extra = self.font_aref(entity, FONT_EXTRA_INDEX);
            if !extra.is_nil() {
                props[FONT_EXTRA_INDEX] = crate::fns::f_copy_alist(self, &[extra])?;
            }
        }
        props[FONT_SIZE_INDEX] = Value::fixnum(pixelsize);
        Ok(self.heap.vectorlike(Vectorlike::Font(Box::new(Font {
            kind: FontKind::Object,
            props,
        }))))
    }

    pub fn font_intern_prop(&mut self, s: &[u8], force_symbol: bool) -> LispResult {
        if s == b"*" {
            return Ok(Value::NIL);
        }
        if !force_symbol && !s.is_empty() && s.iter().all(|b| b.is_ascii_digit()) {
            let mut n: i64 = 0;
            for &b in s {
                n = match n
                    .checked_mul(10)
                    .and_then(|n| n.checked_add((b - b'0') as i64))
                {
                    Some(n) if n <= crate::value::MOST_POSITIVE_FIXNUM => n,
                    _ => {
                        let text = self.unibyte_string(s);
                        return Err(self.signal_list(sym::OVERFLOW_ERROR, &[text]));
                    }
                };
            }
            return Ok(Value::fixnum(n));
        }
        let multibyte = !s.is_ascii() && valid_multibyte(s);
        let name = self.make_string_from_bytes(s.to_vec(), multibyte);
        crate::lread::f_intern(self, &[name, Value::NIL])
    }

    pub fn font_style_table_for(&self, prop: usize) -> Value {
        self.vector_items(self.fonts.style_table)[prop - FONT_WEIGHT_INDEX]
    }

    pub fn font_style_to_value(
        &mut self,
        prop: usize,
        val: Value,
        noerror: bool,
    ) -> LispResult<i64> {
        let table = self.font_style_table_for(prop);
        let rows: Vec<Value> = self.vector_items(table).to_vec();
        let len = rows.len();
        if val.is_symbol() {
            for (i, &row) in rows.iter().enumerate() {
                let items = self.vector_items(row);
                for (j, &name) in items.iter().enumerate().skip(1) {
                    if name == val {
                        let n = items[0].as_fixnum().unwrap_or(0);
                        return Ok((n << 8) | ((i as i64) << 4) | (j as i64 - 1));
                    }
                }
            }
            let s = self.symbol_name_string(val);
            for (i, &row) in rows.iter().enumerate() {
                let items = self.vector_items(row).to_vec();
                for (j, &elt) in items.iter().enumerate().skip(1) {
                    if self.symbol_name_string(elt).eq_ignore_ascii_case(&s) {
                        let n = items[0].as_fixnum().unwrap_or(0);
                        return Ok((n << 8) | ((i as i64) << 4) | (j as i64 - 1));
                    }
                }
            }
            if !noerror {
                return Ok(-1);
            }
            let elt = self.vector(vec![Value::fixnum(100), val]);
            let mut new_rows = rows;
            new_rows.push(elt);
            let new_table = self.vector(new_rows);
            let variable = match prop {
                FONT_WEIGHT_INDEX => sym::FONT_WEIGHT_TABLE,
                FONT_SLANT_INDEX => sym::FONT_SLANT_TABLE,
                _ => sym::FONT_WIDTH_TABLE,
            };
            self.set_plain_value(variable, new_table);
            let style = self.fonts.style_table;
            self.aset_value(
                style,
                Value::fixnum((prop - FONT_WEIGHT_INDEX) as i64),
                new_table,
            )?;
            Ok((100 << 8) | ((len as i64) << 4))
        } else {
            let numeric = val.as_fixnum().unwrap_or(0);
            let mut last_n = -1;
            for (i, &row) in rows.iter().enumerate() {
                let n = self.vector_items(row)[0].as_fixnum().unwrap_or(0);
                if numeric == n {
                    return Ok((n << 8) | ((i as i64) << 4));
                }
                if numeric < n {
                    if !noerror {
                        return Ok(-1);
                    }
                    return Ok(if i == 0 || n - numeric < numeric - last_n {
                        (n << 8) | ((i as i64) << 4)
                    } else {
                        (last_n << 8) | ((i as i64 - 1) << 4)
                    });
                }
                last_n = n;
            }
            if !noerror {
                return Ok(-1);
            }
            Ok((last_n << 8) | ((len as i64 - 1) << 4))
        }
    }

    pub fn font_style_symbolic(&self, font: Value, prop: usize, for_face: bool) -> Value {
        let val = self.font_aref(font, prop);
        if val.is_nil() {
            return Value::NIL;
        }
        let table = self.font_style_table_for(prop);
        let i = val.as_fixnum().unwrap_or(0) & 0xFF;
        let elt = self.vector_items(table)[((i >> 4) & 0xF) as usize];
        let items = self.vector_items(elt);
        if for_face {
            items[1]
        } else {
            items[((i & 0xF) + 1) as usize]
        }
    }

    pub fn font_weight_name_numeric(&mut self, name: Value) -> LispResult<i64> {
        Ok(self.font_style_to_value(FONT_WEIGHT_INDEX, name, false)? >> 8)
    }

    pub fn font_slant_name_numeric(&mut self, name: Value) -> LispResult<i64> {
        Ok(self.font_style_to_value(FONT_SLANT_INDEX, name, false)? >> 8)
    }

    pub fn font_width_name_numeric(&mut self, name: Value) -> LispResult<i64> {
        Ok(self.font_style_to_value(FONT_WIDTH_INDEX, name, false)? >> 8)
    }

    fn font_prop_validate_symbol(&mut self, prop: Value, val: Value) -> LispResult {
        let val = if self.is_string(val) {
            crate::lread::f_intern(self, &[val, Value::NIL])?
        } else {
            val
        };
        if !val.is_symbol() {
            return Ok(sym::ERROR);
        }
        if prop == sym::K_REGISTRY {
            let name = self.symbol_name(val);
            let down = crate::casefiddle::f_downcase(self, &[name])?;
            return crate::lread::f_intern(self, &[down, Value::NIL]);
        }
        Ok(val)
    }

    fn font_prop_validate_style(&mut self, style: Value, val: Value) -> LispResult {
        let prop = if style == sym::K_WEIGHT {
            FONT_WEIGHT_INDEX
        } else if style == sym::K_SLANT {
            FONT_SLANT_INDEX
        } else {
            FONT_WIDTH_INDEX
        };
        if let Some(n) = val.as_fixnum() {
            let table = self.font_style_table_for(prop);
            let rows = self.vector_items(table);
            let row = ((n >> 4) & 0xF) as usize;
            if row >= rows.len() {
                return Ok(sym::ERROR);
            }
            let elt = self.vector_items(rows[row]);
            if (n & 0xF) as usize + 1 >= elt.len() || elt[0].as_fixnum() != Some(n >> 8) {
                return Ok(sym::ERROR);
            }
            Ok(val)
        } else if val.is_symbol() {
            let n = self.font_style_to_value(prop, val, false)?;
            Ok(if n >= 0 { Value::fixnum(n) } else { sym::ERROR })
        } else {
            Ok(sym::ERROR)
        }
    }

    fn font_prop_validate_non_neg(&self, val: Value) -> Value {
        if val.as_fixnum().is_some_and(|n| n >= 0)
            || (val.is_float() && self.heap.float_value(val) >= 0.0)
        {
            val
        } else {
            sym::ERROR
        }
    }

    pub fn font_prop_validate_spacing(&self, val: Value) -> Value {
        if val.is_nil()
            || val
                .as_fixnum()
                .is_some_and(|n| (0..=FONT_SPACING_CHARCELL).contains(&n))
        {
            return val;
        }
        if val.is_symbol() {
            let name = self.symbol_name_string(val);
            if name.len() == 1 {
                match name.as_bytes()[0] {
                    b'c' | b'C' => return Value::fixnum(FONT_SPACING_CHARCELL),
                    b'm' | b'M' => return Value::fixnum(FONT_SPACING_MONO),
                    b'p' | b'P' => return Value::fixnum(FONT_SPACING_PROPORTIONAL),
                    b'd' | b'D' => return Value::fixnum(FONT_SPACING_DUAL),
                    _ => {}
                }
            }
        }
        sym::ERROR
    }

    fn font_prop_validate_otf(&self, val: Value) -> Value {
        if !val.is_cons() || !self.xcar(val).is_symbol() {
            return sym::ERROR;
        }
        let mut tail = self.xcdr(val);
        if tail.is_nil() {
            return val;
        }
        if !tail.is_cons() || !self.xcar(val).is_symbol() {
            return sym::ERROR;
        }
        for _ in 0..2 {
            tail = self.xcdr(tail);
            if tail.is_nil() {
                return val;
            }
            if !tail.is_cons() {
                return sym::ERROR;
            }
            let mut tmp = self.xcar(tail);
            while tmp.is_cons() {
                if !self.xcar(tmp).is_symbol() {
                    return sym::ERROR;
                }
                tmp = self.xcdr(tmp);
            }
            if !tmp.is_nil() {
                return sym::ERROR;
            }
        }
        val
    }

    pub fn font_prop_validate(&mut self, idx: usize, prop: Value, val: Value) -> LispResult {
        if val.is_nil() {
            return Ok(val);
        }
        let (idx, prop) = if prop.is_nil() {
            (idx, FONT_PROPERTY_TABLE[idx].0)
        } else {
            match get_font_prop_index(prop) {
                Some(i) => (i, prop),
                None => return Ok(val),
            }
        };
        let validated = match FONT_PROPERTY_TABLE[idx].1 {
            Validator::Symbol => self.font_prop_validate_symbol(prop, val)?,
            Validator::Style => self.font_prop_validate_style(prop, val)?,
            Validator::NonNeg => self.font_prop_validate_non_neg(val),
            Validator::Spacing => self.font_prop_validate_spacing(val),
            Validator::Otf => self.font_prop_validate_otf(val),
        };
        if validated == sym::ERROR {
            let msg = self.string("invalid font property");
            let cell = self.cons(prop, val);
            return Err(self.signal_list(sym::ERROR, &[msg, cell]));
        }
        Ok(validated)
    }

    pub fn font_put_extra(&mut self, font: Value, prop: Value, val: Value) -> LispResult {
        let mut extra = self.font_aref(font, FONT_EXTRA_INDEX);
        let slot = if extra.is_nil() {
            Value::NIL
        } else {
            self.assq(prop, extra)
        };
        if slot.is_nil() {
            if val == sym::UNBOUND {
                return Ok(val);
            }
            let mut prev = Value::NIL;
            while extra.is_cons() {
                let key = self.xcar(self.xcar(extra));
                if !crate::fns::f_string_lessp(self, &[prop, key])?.is_nil() {
                    break;
                }
                prev = extra;
                extra = self.xcdr(extra);
            }
            let cell = self.cons(prop, val);
            let link = self.cons(cell, extra);
            if prev.is_nil() {
                self.font_aset(font, FONT_EXTRA_INDEX, link);
            } else {
                self.heap.set_cdr(prev, link);
            }
            return Ok(val);
        }
        self.heap.set_cdr(slot, val);
        if val == sym::UNBOUND {
            let extra = self.font_aref(font, FONT_EXTRA_INDEX);
            let extra = crate::fns::f_delq(self, &[slot, extra])?;
            self.font_aset(font, FONT_EXTRA_INDEX, extra);
        }
        Ok(val)
    }

    fn font_expand_wildcards(
        &mut self,
        field: &mut [Value; XLFD_LAST_INDEX],
        n: usize,
    ) -> LispResult<bool> {
        let mut tmp = [Value::NIL; XLFD_LAST_INDEX];
        let mut range = [(0i64, 0i64, 0u32); XLFD_LAST_INDEX];
        let mut range_mask: u32 = 0;
        for _ in 0..=(14 - n) {
            range_mask = (range_mask << 1) | 1;
        }
        let mut range_from: i64 = 0;
        for (range_to, i) in ((14 - n as i64)..).zip(0..n) {
            let val = field[i];
            tmp[i] = val;
            if val.is_nil() {
                range[i] = (range_from, range_to, range_mask);
            } else {
                let (mut from, mut to, mut mask): (i64, i64, u32);
                if let Some(numeric) = val.as_fixnum() {
                    if i + 1 == n {
                        from = XLFD_ENCODING_INDEX as i64;
                        to = from;
                        mask = XLFD_ENCODING_MASK;
                    } else if numeric == 0 {
                        from = XLFD_PIXEL_INDEX as i64;
                        to = XLFD_AVGWIDTH_INDEX as i64;
                        mask = XLFD_PIXEL_MASK | XLFD_LARGENUM_MASK;
                    } else if numeric <= 48 {
                        from = XLFD_PIXEL_INDEX as i64;
                        to = from;
                        mask = XLFD_PIXEL_MASK;
                    } else {
                        from = XLFD_POINT_INDEX as i64;
                        to = XLFD_AVGWIDTH_INDEX as i64;
                        mask = XLFD_LARGENUM_MASK;
                    }
                } else if self.symbol_name_string(val).is_empty() {
                    from = XLFD_FOUNDRY_INDEX as i64;
                    to = XLFD_ADSTYLE_INDEX as i64;
                    mask = XLFD_NULL_MASK;
                } else if i == 0 {
                    from = XLFD_FOUNDRY_INDEX as i64;
                    to = from;
                    mask = XLFD_FOUNDRY_MASK;
                } else if i + 1 == n {
                    if self.symbol_name_string(val).ends_with('*') {
                        from = XLFD_REGISTRY_INDEX as i64;
                        to = XLFD_ENCODING_INDEX as i64;
                        mask = XLFD_REGENC_MASK;
                    } else {
                        from = XLFD_ENCODING_INDEX as i64;
                        to = from;
                        mask = XLFD_ENCODING_MASK;
                    }
                } else if range_from <= XLFD_WEIGHT_INDEX as i64
                    && range_to >= XLFD_WEIGHT_INDEX as i64
                    && self.font_weight_name_numeric(val)? >= 0
                {
                    from = XLFD_WEIGHT_INDEX as i64;
                    to = from;
                    mask = XLFD_WEIGHT_MASK;
                } else if range_from <= XLFD_SLANT_INDEX as i64
                    && range_to >= XLFD_SLANT_INDEX as i64
                    && self.font_slant_name_numeric(val)? >= 0
                {
                    from = XLFD_SLANT_INDEX as i64;
                    to = from;
                    mask = XLFD_SLANT_MASK;
                } else if range_from <= XLFD_SWIDTH_INDEX as i64
                    && range_to >= XLFD_SWIDTH_INDEX as i64
                    && self.font_width_name_numeric(val)? >= 0
                {
                    from = XLFD_SWIDTH_INDEX as i64;
                    to = from;
                    mask = XLFD_SWIDTH_MASK;
                } else if val == sym::C || val == sym::M || val == sym::P || val == sym::D {
                    from = XLFD_SPACING_INDEX as i64;
                    to = from;
                    mask = XLFD_SPACING_MASK;
                } else {
                    from = XLFD_FOUNDRY_INDEX as i64;
                    to = XLFD_ENCODING_INDEX as i64;
                    mask = XLFD_SYMBOL_MASK;
                }
                mask &= range_mask;
                while from < range_from {
                    mask &= !(1 << from);
                    from += 1;
                }
                while from < 14 && mask & (1 << from) == 0 {
                    from += 1;
                }
                while to > range_to {
                    mask &= !(1 << to);
                    to -= 1;
                }
                while to >= 0 && mask & (1 << to) == 0 {
                    to -= 1;
                }
                if from > to {
                    return Ok(false);
                }
                range[i] = (from, to, mask);
                if from > range_from || to < range_to {
                    range_from = from;
                    let mut from = from - 1;
                    let mut to = to - 1;
                    let mut j = i as i64 - 1;
                    while j >= 0 {
                        let r = &mut range[j as usize];
                        if !tmp[j as usize].is_nil() && r.0 < from {
                            while r.0 < from {
                                r.2 &= !(1 << r.0);
                                r.0 += 1;
                            }
                            while from < 14 && r.2 & (1 << from) == 0 {
                                from += 1;
                            }
                            r.0 = from;
                        } else {
                            from = r.0;
                        }
                        if r.1 > to {
                            while r.1 > to {
                                r.2 &= !(1 << r.1);
                                r.1 -= 1;
                            }
                            while to >= 0 && r.2 & (1 << to) == 0 {
                                to -= 1;
                            }
                            r.1 = to;
                        } else {
                            to = r.1;
                        }
                        if from > to {
                            return Ok(false);
                        }
                        j -= 1;
                        from -= 1;
                        to -= 1;
                    }
                }
            }
            range_from += 1;
            range_mask <<= 1;
        }
        let mut j: usize = 0;
        for i in 0..n {
            let from = range[i].0 as usize;
            if j < from {
                if i == 0 || !tmp[i - 1].is_nil() {
                    return Ok(false);
                }
                for f in field.iter_mut().take(from).skip(j) {
                    *f = Value::NIL;
                }
                j = from;
            }
            field[j] = tmp[i];
            j += 1;
        }
        if !tmp[n - 1].is_nil() && j < XLFD_REGISTRY_INDEX {
            return Ok(false);
        }
        for f in field.iter_mut().skip(j) {
            *f = Value::NIL;
        }
        if field[XLFD_ENCODING_INDEX].is_fixnum() {
            let s = crate::data::f_number_to_string(self, &[field[XLFD_ENCODING_INDEX]])?;
            field[XLFD_ENCODING_INDEX] = crate::lread::f_intern(self, &[s, Value::NIL])?;
        }
        Ok(true)
    }

    fn font_parse_xlfd_1(
        &mut self,
        name: &[u8],
        font: Value,
        mut segments: i64,
    ) -> LispResult<bool> {
        let len = name.len();
        if len == 0 {
            return Ok(false);
        }
        let mut f = [0usize; XLFD_LAST_INDEX + 1];
        let mut i = if name[0] == b'*' && (len == 1 || name[1] == b'-') {
            f[XLFD_FOUNDRY_INDEX] = 0;
            1
        } else {
            0
        };
        let mut p = i;
        while p < len {
            if name[p] == b'-' {
                if segments > XLFD_LAST_INDEX as i64 && i == XLFD_WEIGHT_INDEX {
                    segments -= 1;
                } else {
                    f[i] = p + 1;
                    i += 1;
                    if i == XLFD_LAST_INDEX {
                        break;
                    }
                }
            }
            p += 1;
        }
        f[i] = len;
        let field = |k: usize| -> &[u8] {
            let end = f[k + 1].saturating_sub(1).max(f[k]);
            &name[f[k]..end]
        };
        if i == XLFD_LAST_INDEX {
            let v = self.font_intern_prop(field(XLFD_FOUNDRY_INDEX), true)?;
            self.font_aset(font, FONT_FOUNDRY_INDEX, v);
            let v = self.font_intern_prop(field(XLFD_FAMILY_INDEX), true)?;
            self.font_aset(font, FONT_FAMILY_INDEX, v);
            for (xi, fi) in (XLFD_WEIGHT_INDEX..=XLFD_SWIDTH_INDEX).zip(FONT_WEIGHT_INDEX..) {
                let val = self.font_intern_prop(field(xi), true)?;
                if !val.is_nil() {
                    let n = self.font_style_to_value(fi, val, false)?;
                    if n < 0 {
                        return Ok(false);
                    }
                    self.font_aset(font, fi, Value::fixnum(n));
                }
            }
            let v = self.font_intern_prop(field(XLFD_ADSTYLE_INDEX), true)?;
            self.font_aset(font, FONT_ADSTYLE_INDEX, v);
            let registry = &name[f[XLFD_REGISTRY_INDEX]..len];
            if registry == b"*-*" {
                self.font_aset(font, FONT_REGISTRY_INDEX, Value::NIL);
            } else {
                let end = f[XLFD_LAST_INDEX];
                let v = self.font_intern_prop(&name[f[XLFD_REGISTRY_INDEX]..end], true)?;
                self.font_aset(font, FONT_REGISTRY_INDEX, v);
            }
            let pfield = &name[f[XLFD_PIXEL_INDEX]..];
            let pixel_size = if pfield.first() == Some(&b'[') {
                parse_matrix(pfield)
            } else {
                -1
            };
            if pfield.first() == Some(&b'[') && pixel_size >= 0 {
                self.font_aset(font, FONT_SIZE_INDEX, Value::fixnum(pixel_size));
            } else {
                let val = self.font_intern_prop(field(XLFD_PIXEL_INDEX), false)?;
                if val.is_fixnum() {
                    self.font_aset(font, FONT_SIZE_INDEX, val);
                } else if self.font_entity_p(font) {
                    return Ok(false);
                } else {
                    let p = &name[f[XLFD_POINT_INDEX]..];
                    let mut point_size = -1.0;
                    if p.first() == Some(&b'[') {
                        point_size = parse_matrix(p) as f64;
                    } else if p.first().is_some_and(|b| b.is_ascii_digit()) {
                        let digits: String = p
                            .iter()
                            .take_while(|b| b.is_ascii_digit())
                            .map(|&b| b as char)
                            .collect();
                        point_size = digits.parse::<i64>().unwrap_or(0) as f64 / 10.0;
                    }
                    if point_size >= 0.0 {
                        let v = self.float(point_size);
                        self.font_aset(font, FONT_SIZE_INDEX, v);
                    }
                }
            }
            let val = self.font_intern_prop(field(XLFD_RESY_INDEX), false)?;
            if !val.is_nil() && !val.is_fixnum() {
                return Ok(false);
            }
            self.font_aset(font, FONT_DPI_INDEX, val);
            let val = self.font_intern_prop(field(XLFD_SPACING_INDEX), false)?;
            if !val.is_nil() {
                let val = self.font_prop_validate_spacing(val);
                if !val.is_fixnum() {
                    return Ok(false);
                }
                self.font_aset(font, FONT_SPACING_INDEX, val);
            }
            let mut p = f[XLFD_AVGWIDTH_INDEX];
            if name.get(p) == Some(&b'~') {
                p += 1;
            }
            let end = f[XLFD_REGISTRY_INDEX].saturating_sub(1).max(p);
            let val = self.font_intern_prop(&name[p..end], false)?;
            if !val.is_nil() && !val.is_fixnum() {
                return Ok(false);
            }
            self.font_aset(font, FONT_AVGWIDTH_INDEX, val);
        } else {
            if self.font_entity_p(font) {
                return Ok(false);
            }
            let mut wild_card_found = false;
            let mut prop = [Value::NIL; XLFD_LAST_INDEX];
            for j in 0..i {
                if name.get(f[j]) == Some(&b'*') {
                    if let Some(&next) = name.get(f[j] + 1)
                        && next != b'-'
                    {
                        return Ok(false);
                    }
                    prop[j] = Value::NIL;
                    wild_card_found = true;
                } else if j + 1 < i {
                    prop[j] = self.font_intern_prop(field(j), false)?;
                } else {
                    prop[j] = self.font_intern_prop(&name[f[j]..f[i]], false)?;
                }
            }
            if !wild_card_found {
                return Ok(false);
            }
            if !self.font_expand_wildcards(&mut prop, i)? {
                return Ok(false);
            }
            self.font_aset(font, FONT_FOUNDRY_INDEX, prop[XLFD_FOUNDRY_INDEX]);
            self.font_aset(font, FONT_FAMILY_INDEX, prop[XLFD_FAMILY_INDEX]);
            for (xi, fi) in (XLFD_WEIGHT_INDEX..=XLFD_SWIDTH_INDEX).zip(FONT_WEIGHT_INDEX..) {
                if !prop[xi].is_nil() {
                    let n = self.font_style_to_value(fi, prop[xi], true)?;
                    if n < 0 {
                        return Ok(false);
                    }
                    self.font_aset(font, fi, Value::fixnum(n));
                }
            }
            self.font_aset(font, FONT_ADSTYLE_INDEX, prop[XLFD_ADSTYLE_INDEX]);
            let registry = prop[XLFD_REGISTRY_INDEX];
            let encoding = prop[XLFD_ENCODING_INDEX];
            let val = if registry.is_nil() {
                if encoding.is_nil() {
                    None
                } else {
                    Some(format!("*-{}", self.symbol_name_string(encoding)))
                }
            } else if encoding.is_nil() {
                Some(format!("{}-*", self.symbol_name_string(registry)))
            } else {
                Some(format!(
                    "{}-{}",
                    self.symbol_name_string(registry),
                    self.symbol_name_string(encoding)
                ))
            };
            if let Some(text) = val {
                let s = self.string(&text);
                let v = crate::lread::f_intern(self, &[s, Value::NIL])?;
                self.font_aset(font, FONT_REGISTRY_INDEX, v);
            }
            if prop[XLFD_PIXEL_INDEX].is_fixnum() {
                self.font_aset(font, FONT_SIZE_INDEX, prop[XLFD_PIXEL_INDEX]);
            } else if let Some(point) = prop[XLFD_POINT_INDEX].as_fixnum() {
                let v = self.float(point as f64 / 10.0);
                self.font_aset(font, FONT_SIZE_INDEX, v);
            }
            if prop[XLFD_RESX_INDEX].is_fixnum() {
                self.font_aset(font, FONT_DPI_INDEX, prop[XLFD_RESY_INDEX]);
            }
            if !prop[XLFD_SPACING_INDEX].is_nil() {
                let val = self.font_prop_validate_spacing(prop[XLFD_SPACING_INDEX]);
                if !val.is_fixnum() {
                    return Ok(false);
                }
                self.font_aset(font, FONT_SPACING_INDEX, val);
            }
            if prop[XLFD_AVGWIDTH_INDEX].is_fixnum() {
                self.font_aset(font, FONT_AVGWIDTH_INDEX, prop[XLFD_AVGWIDTH_INDEX]);
            }
        }
        Ok(true)
    }
}

impl Lisp {
    pub fn font_set_style(&mut self, font: Value, prop: usize, val: Value) -> LispResult<()> {
        let n = self.font_style_to_value(prop, val, true)?;
        self.font_aset(font, prop, Value::fixnum(n));
        Ok(())
    }

    pub fn font_parse_xlfd(&mut self, name: &[u8], font: Value) -> LispResult<bool> {
        if self.font_parse_xlfd_1(name, font, -1)? {
            return Ok(true);
        }
        let segments = name.iter().filter(|&&b| b == b'-').count() as i64;
        if segments > XLFD_LAST_INDEX as i64 {
            self.font_parse_xlfd_1(name, font, segments)
        } else {
            Ok(false)
        }
    }

    fn style_name_for_xlfd(&self, font: Value, prop: usize) -> String {
        let val = self.font_style_symbolic(font, prop, false);
        if val.is_nil() {
            return "*".to_string();
        }
        self.symbol_name_string(val)
            .chars()
            .filter(|&c| c != '-' && c != '?' && c != ',' && c != '"')
            .collect()
    }

    fn font_string_field(&self, val: Value) -> String {
        let val = if val.is_symbol() {
            self.symbol_name(val)
        } else {
            val
        };
        self.string_to_rust(val)
    }

    pub fn font_unparse_xlfd(&self, font: Value, pixel_size: i64, nbytes: usize) -> Option<String> {
        let mut f: [String; XLFD_REGISTRY_INDEX + 1] = Default::default();
        for i in FONT_FOUNDRY_INDEX..=FONT_REGISTRY_INDEX {
            let j = match i {
                FONT_FOUNDRY_INDEX => XLFD_FOUNDRY_INDEX,
                FONT_FAMILY_INDEX => XLFD_FAMILY_INDEX,
                FONT_ADSTYLE_INDEX => XLFD_ADSTYLE_INDEX,
                _ => XLFD_REGISTRY_INDEX,
            };
            let val = self.font_aref(font, i);
            f[j] = if val.is_nil() {
                if j == XLFD_REGISTRY_INDEX {
                    "*-*".to_string()
                } else {
                    "*".to_string()
                }
            } else {
                let s = self.font_string_field(val);
                if j == XLFD_REGISTRY_INDEX && !s.contains('-') {
                    if nbytes <= s.len() + 4 {
                        return None;
                    }
                    if s.ends_with('*') {
                        format!("{s}-*")
                    } else {
                        format!("{s}*-*")
                    }
                } else {
                    s
                }
            };
        }
        for (i, j) in (FONT_WEIGHT_INDEX..=FONT_WIDTH_INDEX).zip(XLFD_WEIGHT_INDEX..) {
            let name = self.style_name_for_xlfd(font, i);
            if name != "*" && nbytes <= name.len() + 1 {
                return None;
            }
            f[j] = name;
        }
        let val = self.font_aref(font, FONT_SIZE_INDEX);
        f[XLFD_PIXEL_INDEX] = if let Some(v) = val.as_fixnum() {
            let v = if v > 0 { v } else { pixel_size };
            if v > 0 {
                format!("{v}-*")
            } else {
                "*-*".to_string()
            }
        } else if val.is_float() {
            format!("*-{:.0}", self.heap.float_value(val) * 10.0)
        } else {
            "*-*".to_string()
        };
        f[XLFD_RESX_INDEX] = match self.font_aref(font, FONT_DPI_INDEX).as_fixnum() {
            Some(v) => format!("{v}-{v}"),
            None => "*-*".to_string(),
        };
        f[XLFD_SPACING_INDEX] = match self.font_aref(font, FONT_SPACING_INDEX).as_fixnum() {
            Some(s) if s <= FONT_SPACING_PROPORTIONAL => "p".to_string(),
            Some(s) if s <= FONT_SPACING_DUAL => "d".to_string(),
            Some(s) if s <= FONT_SPACING_MONO => "m".to_string(),
            Some(_) => "c".to_string(),
            None => "*".to_string(),
        };
        f[XLFD_AVGWIDTH_INDEX] = match self.font_aref(font, FONT_AVGWIDTH_INDEX).as_fixnum() {
            Some(v) => v.to_string(),
            None => "*".to_string(),
        };
        let name = format!(
            "-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}",
            f[XLFD_FOUNDRY_INDEX],
            f[XLFD_FAMILY_INDEX],
            f[XLFD_WEIGHT_INDEX],
            f[XLFD_SLANT_INDEX],
            f[XLFD_SWIDTH_INDEX],
            f[XLFD_ADSTYLE_INDEX],
            f[XLFD_PIXEL_INDEX],
            f[XLFD_RESX_INDEX],
            f[XLFD_SPACING_INDEX],
            f[XLFD_AVGWIDTH_INDEX],
            f[XLFD_REGISTRY_INDEX]
        );
        (name.len() < nbytes).then_some(name)
    }

    pub fn font_parse_fcname(&mut self, name: &[u8], font: Value) -> LispResult<bool> {
        let len = name.len();
        if len == 0 {
            return Ok(false);
        }
        let mut size_beg = None;
        let mut props_beg = None;
        let mut family_end = None;
        let mut p = 0;
        while p < len {
            if name[p] == b'\\' && p + 1 < len {
                p += 1;
            } else if name[p] == b':' {
                props_beg = Some(p);
                family_end = Some(p);
                break;
            } else if name[p] == b'-' {
                let mut decimal = false;
                let mut size_found = true;
                let mut q = p + 1;
                while q < len && name[q] != b':' {
                    if !name[q].is_ascii_digit() {
                        if name[q] != b'.' || decimal {
                            size_found = false;
                            break;
                        }
                        decimal = true;
                    }
                    q += 1;
                }
                if size_found {
                    family_end = Some(p);
                    size_beg = Some(p + 1);
                    break;
                }
            }
            p += 1;
        }
        if let Some(family_end) = family_end {
            let mut extra_props = Value::NIL;
            if family_end > 0 {
                let family = self.font_intern_prop(&name[..family_end], true)?;
                self.font_aset(font, FONT_FAMILY_INDEX, family);
            }
            if let Some(size_beg) = size_beg {
                let text = String::from_utf8_lossy(&name[size_beg..]);
                let (point_size, used) = strtod(&text);
                let v = self.float(point_size);
                self.font_aset(font, FONT_SIZE_INDEX, v);
                let size_end = size_beg + used;
                if name.get(size_end) == Some(&b':') && size_end + 1 < len {
                    props_beg = Some(size_end);
                }
            }
            if let Some(props_beg) = props_beg {
                let mut p = props_beg;
                while p < len {
                    let mut q = p + 1;
                    while q < len && name[q] != b'=' && name[q] != b':' {
                        q += 1;
                    }
                    if name.get(q) != Some(&b'=') {
                        let word = &name[p + 1..q];
                        let val = self.font_intern_prop(word, true)?;
                        match word {
                            b"thin" | b"ultra-light" | b"light" | b"semi-light" | b"book"
                            | b"medium" | b"normal" | b"semibold" | b"demibold" | b"bold"
                            | b"ultra-bold" | b"black" | b"heavy" | b"ultra-heavy" => {
                                self.font_set_style(font, FONT_WEIGHT_INDEX, val)?
                            }
                            b"roman" | b"italic" | b"oblique" => {
                                self.font_set_style(font, FONT_SLANT_INDEX, val)?
                            }
                            b"charcell" => self.font_aset(
                                font,
                                FONT_SPACING_INDEX,
                                Value::fixnum(FONT_SPACING_CHARCELL),
                            ),
                            b"mono" => self.font_aset(
                                font,
                                FONT_SPACING_INDEX,
                                Value::fixnum(FONT_SPACING_MONO),
                            ),
                            b"proportional" => self.font_aset(
                                font,
                                FONT_SPACING_INDEX,
                                Value::fixnum(FONT_SPACING_PROPORTIONAL),
                            ),
                            _ => {}
                        }
                    } else {
                        let (key, prop) = if q - p == 10 && &name[p + 1..p + 10] == b"pixelsize" {
                            (Value::NIL, Some(FONT_SIZE_INDEX))
                        } else {
                            let key = self.font_intern_prop(&name[p..q], true)?;
                            (key, get_font_prop_index(key))
                        };
                        let vp = q + 1;
                        let mut vq = vp;
                        while vq < len && name[vq] != b':' {
                            vq += 1;
                        }
                        let val = self.font_intern_prop(&name[vp..vq], false)?;
                        match prop {
                            Some(prop)
                                if (FONT_FOUNDRY_INDEX..FONT_EXTRA_INDEX).contains(&prop) =>
                            {
                                let v = self.font_prop_validate(prop, Value::NIL, val)?;
                                self.font_aset(font, prop, v);
                            }
                            _ => {
                                let cell = self.cons(key, val);
                                let item = self.list(&[cell]);
                                extra_props = self.nconc2(extra_props, item);
                            }
                        }
                        q = vq;
                    }
                    p = q;
                }
            }
            let _ = extra_props;
        } else {
            let mut weight = Value::NIL;
            let mut slant = Value::NIL;
            let mut width = Value::NIL;
            let mut size = Value::NIL;
            let mut p = len as i64 - 1;
            while p >= 0 && name[p as usize].is_ascii_digit() {
                p -= 1;
            }
            let mut end = if p < len as i64 - 1 && (p < 0 || name[p as usize] == b' ') {
                let text = String::from_utf8_lossy(&name[(p + 1) as usize..]);
                size = self.float(strtod(&text).0);
                p.max(0) as usize
            } else {
                len
            };
            if p < 0 {
                end = 0;
            }
            let mut family_end = None;
            let mut cur = end;
            while cur > 0 {
                let mut q = cur as i64 - 1;
                while q >= 0 {
                    if q > 0 && name[q as usize - 1] == b'\\' {
                        q -= 1;
                    } else if name[q as usize] == b' ' {
                        break;
                    }
                    q -= 1;
                }
                let word_start = (q + 1) as usize;
                let word = &name[word_start..cur];
                let save = |var: &mut Value, s: &str, lisp: &mut Lisp| -> LispResult<()> {
                    if var.is_nil() {
                        *var = lisp.font_intern_prop(s.as_bytes(), true)?;
                    }
                    Ok(())
                };
                match word {
                    b"Ultra-Light" => save(&mut weight, "ultra-light", self)?,
                    b"Light" => save(&mut weight, "light", self)?,
                    b"Book" => save(&mut weight, "book", self)?,
                    b"Medium" => save(&mut weight, "medium", self)?,
                    b"Semi-Bold" => save(&mut weight, "semi-bold", self)?,
                    b"Bold" => save(&mut weight, "bold", self)?,
                    b"Italic" => save(&mut slant, "italic", self)?,
                    b"Oblique" => save(&mut slant, "oblique", self)?,
                    b"Semi-Condensed" => save(&mut width, "semi-condensed", self)?,
                    b"Condensed" => save(&mut width, "condensed", self)?,
                    _ => {
                        family_end = Some(cur);
                        break;
                    }
                }
                cur = q.max(0) as usize;
                if q < 0 {
                    break;
                }
            }
            if let Some(family_end) = family_end {
                let family = self.font_intern_prop(&name[..family_end], true)?;
                self.font_aset(font, FONT_FAMILY_INDEX, family);
            }
            if !size.is_nil() {
                self.font_aset(font, FONT_SIZE_INDEX, size);
            }
            if !weight.is_nil() {
                self.font_set_style(font, FONT_WEIGHT_INDEX, weight)?;
            }
            if !slant.is_nil() {
                self.font_set_style(font, FONT_SLANT_INDEX, slant)?;
            }
            if !width.is_nil() {
                self.font_set_style(font, FONT_WIDTH_INDEX, width)?;
            }
        }
        Ok(true)
    }

    pub fn font_unparse_fcname(
        &self,
        font: Value,
        mut pixel_size: i64,
        nbytes: usize,
    ) -> Option<String> {
        let family = self.font_aref(font, FONT_FAMILY_INDEX);
        let family = if family.is_symbol() && !family.is_nil() {
            Some(self.symbol_name_string(family))
        } else {
            None
        };
        let val = self.font_aref(font, FONT_SIZE_INDEX);
        let point_size = if let Some(v) = val.as_fixnum() {
            if v != 0 {
                pixel_size = v;
            }
            -1
        } else {
            pixel_size = -1;
            if val.is_float() {
                self.heap.float_value(val) as i64
            } else {
                0
            }
        };
        let mut out = String::new();
        let push = |s: &str, out: &mut String| -> Option<()> {
            if out.len() + s.len() >= nbytes {
                return None;
            }
            out.push_str(s);
            Some(())
        };
        if let Some(family) = &family {
            push(family, &mut out)?;
        }
        if point_size > 0 {
            let s = if out.is_empty() {
                format!("{point_size}")
            } else {
                format!("-{point_size}")
            };
            push(&s, &mut out)?;
        } else if pixel_size > 0 {
            push(&format!(":pixelsize={pixel_size}"), &mut out)?;
        }
        let foundry = self.font_aref(font, FONT_FOUNDRY_INDEX);
        if !foundry.is_nil() {
            push(
                &format!(":foundry={}", self.symbol_name_string(foundry)),
                &mut out,
            )?;
        }
        for (i, style) in ["weight", "slant", "width"].iter().enumerate() {
            let v = self.font_style_symbolic(font, FONT_WEIGHT_INDEX + i, false);
            if !v.is_nil() {
                push(
                    &format!(":{style}={}", self.symbol_name_string(v)),
                    &mut out,
                )?;
            }
        }
        if let Some(dpi) = self.font_aref(font, FONT_DPI_INDEX).as_fixnum() {
            push(&format!(":dpi={dpi}"), &mut out)?;
        }
        if let Some(spacing) = self.font_aref(font, FONT_SPACING_INDEX).as_fixnum() {
            push(&format!(":spacing={spacing}"), &mut out)?;
        }
        if let Some(avg) = self.font_aref(font, FONT_AVGWIDTH_INDEX).as_fixnum() {
            push(
                if avg == 0 {
                    ":scalable=true"
                } else {
                    ":scalable=false"
                },
                &mut out,
            )?;
        }
        Some(out)
    }

    pub fn font_parse_name(&mut self, name: &[u8], font: Value) -> LispResult<bool> {
        if name.first() == Some(&b'-') || name.contains(&b'*') || name.contains(&b'?') {
            self.font_parse_xlfd(name, font)
        } else {
            self.font_parse_fcname(name, font)
        }
    }

    pub fn font_parse_family_registry(
        &mut self,
        family: Value,
        registry: Value,
        font_spec: Value,
    ) -> LispResult<()> {
        if !family.is_nil() && self.font_aref(font_spec, FONT_FAMILY_INDEX).is_nil() {
            self.check_string(family)?;
            let bytes = self.string_bytes(family).to_vec();
            if let Some(dash) = bytes.iter().position(|&b| b == b'-') {
                if bytes[0] != b'*'
                    && dash > 0
                    && self.font_aref(font_spec, FONT_FOUNDRY_INDEX).is_nil()
                {
                    let foundry = self.font_intern_prop(&bytes[..dash], true)?;
                    f_font_put(self, &[font_spec, sym::K_FOUNDRY, foundry])?;
                }
                let fam = self.font_intern_prop(&bytes[dash + 1..], true)?;
                f_font_put(self, &[font_spec, sym::K_FAMILY, fam])?;
            } else {
                let sym_family = crate::lread::f_intern(self, &[family, Value::NIL])?;
                self.font_aset(font_spec, FONT_FAMILY_INDEX, sym_family);
            }
        }
        if !registry.is_nil() {
            self.check_string(registry)?;
            let bytes = self.string_bytes(registry).to_vec();
            let mut registry = registry;
            if !bytes.contains(&b'-') {
                let asterisk = bytes.last() == Some(&b'*');
                let suffix = self.string(if asterisk { "-*" } else { "*-*" });
                registry = crate::fns::f_concat(self, &[registry, suffix])?;
            }
            let down = crate::casefiddle::f_downcase(self, &[registry])?;
            let v = crate::lread::f_intern(self, &[down, Value::NIL])?;
            self.font_aset(font_spec, FONT_REGISTRY_INDEX, v);
        }
        Ok(())
    }
}
pub const SUBRS: &[crate::subr::Subr] = &[
    crate::subr::Subr::fixed("fontp", 1, 2, f_fontp),
    crate::subr::Subr::many("font-spec", 0, f_font_spec),
    crate::subr::Subr::fixed("font-get", 2, 2, f_font_get),
    crate::subr::Subr::fixed("font-put", 3, 3, f_font_put),
    crate::subr::Subr::fixed("font-xlfd-name", 1, 3, f_font_xlfd_name),
];

fn build_style_table(lisp: &mut Lisp, entries: &[(i64, &[&str])]) -> Value {
    let mut rows = Vec::with_capacity(entries.len());
    for &(numeric, names) in entries {
        let mut elt = vec![Value::fixnum(numeric)];
        for name in names {
            elt.push(lisp.intern(name));
        }
        rows.push(lisp.vector(elt));
    }
    lisp.vector(rows)
}

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::FONT_ENCODING_ALIST, Value::NIL);
    let weight = build_style_table(lisp, WEIGHT_TABLE);
    lisp.defvar(sym::FONT_WEIGHT_TABLE, weight);
    lisp.make_constant(sym::FONT_WEIGHT_TABLE);
    let slant = build_style_table(lisp, SLANT_TABLE);
    lisp.defvar(sym::FONT_SLANT_TABLE, slant);
    lisp.make_constant(sym::FONT_SLANT_TABLE);
    let width = build_style_table(lisp, WIDTH_TABLE);
    lisp.defvar(sym::FONT_WIDTH_TABLE, width);
    lisp.make_constant(sym::FONT_WIDTH_TABLE);
    lisp.fonts.style_table = lisp.vector(vec![weight, slant, width]);
    let log = if std::env::var_os("EMACS_FONT_LOG").is_some() {
        Value::NIL
    } else {
        sym::T
    };
    lisp.defvar(sym::FONT_LOG, log);
    lisp.defvar_bool(sym::INHIBIT_COMPACTING_FONT_CACHES, cfg!(windows));
    lisp.defvar_bool(sym::XFT_IGNORE_COLOR_FONTS, true);
    lisp.defvar_bool(sym::QUERY_ALL_FONT_BACKENDS, false);
}

impl Lisp {
    pub fn check_font(&mut self, v: Value) -> LispResult<Value> {
        if self.is_font(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::FONT, v))
        }
    }

    pub fn check_font_spec(&mut self, v: Value) -> LispResult<Value> {
        if self.font_spec_p(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::FONT_SPEC, v))
        }
    }

    pub fn copy_font_spec(&mut self, font: Value) -> LispResult {
        self.check_font(font)?;
        let mut props = vec![Value::NIL; FONT_SPEC_MAX];
        props[1..FONT_EXTRA_INDEX].copy_from_slice(&self.font(font).props[1..FONT_EXTRA_INDEX]);
        let base = self.stack.len();
        let mut items = Vec::new();
        let mut tail = self.font_aref(font, FONT_EXTRA_INDEX);
        while tail.is_cons() {
            let elt = self.xcar(tail);
            let key = self.xcar(elt);
            if key != sym::K_FONT_ENTITY {
                let value = self.cdr_safe(elt);
                let cell = self.cons(key, value);
                self.stack.push(cell);
                items.push(cell);
            }
            tail = self.xcdr(tail);
        }
        props[FONT_EXTRA_INDEX] = self.list(&items);
        self.stack.truncate(base);
        Ok(self.heap.vectorlike(Vectorlike::Font(Box::new(Font {
            kind: FontKind::Spec,
            props,
        }))))
    }

    pub fn merge_font_spec(&mut self, from: Value, to: Value) -> LispResult {
        self.check_font(from)?;
        self.check_font(to)?;
        let to = self.copy_font_spec(to)?;
        for i in 0..FONT_EXTRA_INDEX {
            let v = self.font_aref(from, i);
            self.font_aset(to, i, v);
        }
        let mut extra = self.font_aref(to, FONT_EXTRA_INDEX);
        let mut tail = self.font_aref(from, FONT_EXTRA_INDEX);
        while tail.is_cons() {
            let elt = self.xcar(tail);
            let key = self.xcar(elt);
            if key != sym::FONT_ENTITY {
                let slot = self.assq(key, extra);
                let value = self.xcdr(elt);
                if !slot.is_nil() {
                    self.heap.set_cdr(slot, value);
                } else {
                    let cell = self.cons(key, value);
                    extra = self.cons(cell, extra);
                }
            }
            tail = self.xcdr(tail);
        }
        self.font_aset(to, FONT_EXTRA_INDEX, extra);
        Ok(to)
    }

    pub fn font_spec_from_name(&mut self, font_name: Value) -> LispResult {
        let spec = self.font_make_spec();
        self.check_string(font_name)?;
        let bytes = self.string_bytes(font_name).to_vec();
        if !self.font_parse_name(&bytes, spec)? {
            return Ok(Value::NIL);
        }
        self.font_put_extra(spec, sym::K_NAME, font_name)?;
        self.font_put_extra(spec, sym::K_USER_SPEC, font_name)?;
        Ok(spec)
    }

    pub fn font_clear_prop(&mut self, attrs: &mut [Value], prop: usize) -> LispResult<()> {
        let lface_font = crate::xfaces::LFACE_FONT_INDEX;
        let mut font = attrs[lface_font];
        if !self.is_font(font) {
            return Ok(());
        }
        if !f_font_get(self, &[font, sym::K_NAME])?.is_nil() {
            font = self.copy_font_spec(font)?;
            self.font_put_extra(font, sym::K_NAME, sym::UNBOUND)?;
        }
        if self.font_aref(font, prop).is_nil()
            && prop != FONT_FAMILY_INDEX
            && prop != FONT_FOUNDRY_INDEX
            && prop != FONT_WIDTH_INDEX
            && prop != FONT_SIZE_INDEX
        {
            return Ok(());
        }
        if font == attrs[lface_font] {
            font = self.copy_font_spec(font)?;
        }
        self.font_aset(font, prop, Value::NIL);
        if prop == FONT_FAMILY_INDEX || prop == FONT_FOUNDRY_INDEX {
            if prop == FONT_FAMILY_INDEX {
                self.font_aset(font, FONT_FOUNDRY_INDEX, Value::NIL);
                self.font_aset(font, FONT_WIDTH_INDEX, Value::NIL);
            }
            for idx in [
                FONT_ADSTYLE_INDEX,
                FONT_REGISTRY_INDEX,
                FONT_SIZE_INDEX,
                FONT_DPI_INDEX,
                FONT_SPACING_INDEX,
                FONT_AVGWIDTH_INDEX,
            ] {
                self.font_aset(font, idx, Value::NIL);
            }
        } else if prop == FONT_SIZE_INDEX {
            for idx in [FONT_DPI_INDEX, FONT_SPACING_INDEX, FONT_AVGWIDTH_INDEX] {
                self.font_aset(font, idx, Value::NIL);
            }
        } else if prop == FONT_WIDTH_INDEX {
            self.font_aset(font, FONT_AVGWIDTH_INDEX, Value::NIL);
        }
        attrs[lface_font] = font;
        Ok(())
    }
}

fn f_fontp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (object, extra_type) = (args[0], args.get(1).copied().unwrap_or(Value::NIL));
    let r = if extra_type.is_nil() {
        lisp.is_font(object)
    } else if extra_type == sym::FONT_SPEC {
        lisp.font_spec_p(object)
    } else if extra_type == sym::FONT_ENTITY {
        lisp.font_entity_p(object)
    } else if extra_type == sym::FONT_OBJECT {
        lisp.font_object_p(object)
    } else {
        return Err(lisp.wrong_type(sym::FONT_EXTRA_TYPE, extra_type));
    };
    Ok(lisp.bool(r))
}

pub(crate) fn f_font_spec(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let spec = lisp.font_make_spec();
    let base = lisp.stack.len();
    lisp.stack.push(spec);
    let r = font_spec_1(lisp, spec, args);
    lisp.stack.truncate(base);
    r?;
    Ok(spec)
}

fn font_spec_1(lisp: &mut Lisp, spec: Value, args: &[Value]) -> LispResult<()> {
    let mut i = 0;
    while i < args.len() {
        let key = lisp.check_symbol(args[i])?;
        if i + 1 >= args.len() {
            let name = lisp.symbol_name_string(key);
            return Err(lisp.error(&format!("No value for key `{name}'")));
        }
        let val = args[i + 1];
        if key == sym::K_NAME {
            lisp.check_string(val)?;
            let bytes = lisp.string_bytes(val).to_vec();
            if !lisp.font_parse_name(&bytes, spec)? {
                let text = lisp.string_to_rust(val);
                return Err(lisp.error(&format!("Invalid font name: {text}")));
            }
            lisp.font_put_extra(spec, key, val)?;
        } else {
            match get_font_prop_index(key) {
                Some(idx) => {
                    let val = lisp.font_prop_validate(idx, Value::NIL, val)?;
                    if idx < FONT_EXTRA_INDEX {
                        lisp.font_aset(spec, idx, val);
                    } else {
                        lisp.font_put_extra(spec, key, val)?;
                    }
                }
                None => {
                    let val = lisp.font_prop_validate(0, key, val)?;
                    lisp.font_put_extra(spec, key, val)?;
                }
            }
        }
        i += 2;
    }
    Ok(())
}

pub(crate) fn f_font_get(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let font = lisp.check_font(args[0])?;
    let key = lisp.check_symbol(args[1])?;
    let idx = get_font_prop_index(key);
    if let Some(idx) = idx {
        if (FONT_WEIGHT_INDEX..=FONT_WIDTH_INDEX).contains(&idx) {
            return Ok(lisp.font_style_symbolic(font, idx, false));
        }
        if idx < FONT_EXTRA_INDEX {
            return Ok(lisp.font_aref(font, idx));
        }
    }
    let extra = lisp.font_aref(font, FONT_EXTRA_INDEX);
    let val = lisp.assq(key, extra);
    if val.is_nil() && lisp.font_object_p(font) {
        if key == sym::K_OTF {
            return Ok(lisp.cons(Value::NIL, Value::NIL));
        }
        return Ok(Value::NIL);
    }
    Ok(lisp.cdr_safe(val))
}

pub(crate) fn f_font_put(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (font, prop, val) = (args[0], args[1], args[2]);
    match get_font_prop_index(prop) {
        Some(idx) if idx < FONT_EXTRA_INDEX => {
            lisp.check_font_spec(font)?;
            let v = lisp.font_prop_validate(idx, Value::NIL, val)?;
            lisp.font_aset(font, idx, v);
        }
        _ => {
            if prop == sym::K_NAME
                || prop == sym::K_SCRIPT
                || prop == sym::K_LANG
                || prop == sym::K_OTF
            {
                lisp.check_font_spec(font)?;
            } else {
                lisp.check_font(font)?;
            }
            let v = lisp.font_prop_validate(0, prop, val)?;
            lisp.font_put_extra(font, prop, v)?;
        }
    }
    Ok(val)
}

fn f_font_xlfd_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let font = lisp.check_font(args[0])?;
    let fold_wildcards = args.get(1).copied().unwrap_or(Value::NIL);
    let long_xlfds = args.get(2).copied().unwrap_or(Value::NIL);
    let mut pixel_size = 0;
    let mut name = None;
    if lisp.font_object_p(font) {
        let font_name = lisp.font_aref(font, FONT_NAME_INDEX);
        if lisp.is_string(font_name) && lisp.string_bytes(font_name).first() == Some(&b'-') {
            if fold_wildcards.is_nil() {
                return Ok(font_name);
            }
            name = Some(lisp.string_to_rust(font_name));
        } else {
            pixel_size = lisp
                .font_aref(font, FONT_SIZE_INDEX)
                .as_fixnum()
                .unwrap_or(0);
        }
    }
    let mut name = match name {
        Some(n) => n,
        None => {
            let limit = if long_xlfds.is_nil() { 256 } else { usize::MAX };
            match lisp.font_unparse_xlfd(font, pixel_size, limit) {
                Some(n) => n,
                None => return Ok(Value::NIL),
            }
        }
    };
    if !fold_wildcards.is_nil() {
        while let Some(pos) = name.find("-*-*") {
            name.replace_range(pos..pos + 2, "");
        }
    }
    Ok(lisp.string(&name))
}
