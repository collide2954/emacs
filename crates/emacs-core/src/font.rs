use crate::doprnt::Arg;
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

pub const FONT_INVALID_CODE: u32 = 0xFFFFFFFF;
pub const FONT_PIXEL_SIZE_QUANTUM: i64 = 1;

pub type FontCacheHook = fn(&mut Lisp, Value) -> Value;
pub type FontListHook = fn(&mut Lisp, Value, Value) -> LispResult;
pub type FontFrameListHook = fn(&mut Lisp, Value) -> LispResult;
pub type FontOpenHook = fn(&mut Lisp, Value, Value, i32) -> LispResult;
pub type FontCloseHook = fn(&mut Lisp, Value);
pub type FontFaceHook = fn(&mut Lisp, Value, usize);
pub type FontHasCharHook = fn(&mut Lisp, Value, i32) -> i32;
pub type FontEncodeCharHook = fn(&mut Lisp, Value, i32) -> u32;
pub type FontTextExtentsHook = fn(&mut Lisp, Value, &[u32]) -> FontMetrics;
pub type FontDrawHook =
    fn(&mut Lisp, &mut crate::dispextern::GlyphString, usize, usize, i32, i32, bool) -> i32;
pub type FontCapabilityHook = fn(&mut Lisp, Value) -> LispResult;
pub type FontFrameHook = fn(&mut Lisp, Value) -> i32;
pub type FontFilterHook = fn(&mut Lisp, Value, Value) -> LispResult<()>;
pub type FontCachedOkHook = fn(&mut Lisp, Value, Value, Value) -> bool;
pub type FontCombiningHook = fn(&mut Lisp, Value) -> Value;

#[derive(Clone, Copy, Debug)]
pub struct FontDriver {
    pub type_: Value,
    pub case_sensitive: bool,
    pub get_cache: FontCacheHook,
    pub list: FontListHook,
    pub match_font: FontListHook,
    pub list_family: Option<FontFrameListHook>,
    pub free_entity: Option<FontCloseHook>,
    pub open_font: FontOpenHook,
    pub close_font: FontCloseHook,
    pub prepare_face: Option<FontFaceHook>,
    pub done_face: Option<FontFaceHook>,
    pub has_char: Option<FontHasCharHook>,
    pub encode_char: FontEncodeCharHook,
    pub text_extents: FontTextExtentsHook,
    pub draw: Option<FontDrawHook>,
    pub otf_capability: Option<FontCapabilityHook>,
    pub start_for_frame: Option<FontFrameHook>,
    pub end_for_frame: Option<FontFrameHook>,
    pub filter_properties: Option<FontFilterHook>,
    pub cached_font_ok: Option<FontCachedOkHook>,
    pub combining_capability: Option<FontCombiningHook>,
}

#[derive(Clone, Copy, Debug)]
pub struct FontDriverList {
    pub on: bool,
    pub driver: FontDriver,
}

#[derive(Clone, Debug, Default)]
pub struct FontObject {
    pub min_width: i32,
    pub max_width: i32,
    pub pixel_size: i32,
    pub height: i32,
    pub space_width: i32,
    pub average_width: i32,
    pub ascent: i32,
    pub descent: i32,
    pub underline_thickness: i32,
    pub underline_position: i32,
    pub vertical_centering: bool,
    pub baseline_offset: i32,
    pub relative_compose: i32,
    pub default_ascent: i32,
    pub encoding_charset: i32,
    pub repertory_charset: i32,
    pub driver: Option<FontDriver>,
    pub driver_data: Option<std::rc::Rc<dyn std::any::Any>>,
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
    pub object: Option<Box<FontObject>>,
}

impl Font {
    pub fn trace(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&self.props);
        if let Some(driver) = self.object.as_ref().and_then(|o| o.driver) {
            out.push(driver.type_);
        }
    }
}

pub struct Fonts {
    pub style_table: Value,
    pub charset_alist: Value,
    pub sort_shift_bits: [i32; FONT_SIZE_INDEX + 1],
    pub driver_list: Vec<FontDriver>,
    pub num_font_drivers: usize,
    pub scratch_font_spec: Value,
    pub scratch_font_prefer: Value,
    pub font_log_deferred: Value,
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
            driver_list: Vec::new(),
            num_font_drivers: 0,
            scratch_font_spec: Value::NIL,
            scratch_font_prefer: Value::NIL,
            font_log_deferred: Value::NIL,
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
        out.push(self.scratch_font_spec);
        out.push(self.scratch_font_prefer);
        out.push(self.font_log_deferred);
        out.extend(self.driver_list.iter().map(|driver| driver.type_));
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
            object: None,
        })))
    }

    pub fn font_make_entity(&mut self) -> Value {
        self.heap.vectorlike(Vectorlike::Font(Box::new(Font {
            kind: FontKind::Entity,
            props: vec![Value::NIL; FONT_ENTITY_MAX],
            object: None,
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
            object: Some(Box::default()),
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
    crate::subr::Subr::fixed("internal-char-font", 1, 2, f_internal_char_font),
    crate::subr::Subr::fixed("list-fonts", 1, 4, f_list_fonts),
    crate::subr::Subr::fixed("font-family-list", 0, 1, f_font_family_list),
    crate::subr::Subr::fixed("find-font", 1, 2, f_find_font),
    crate::subr::Subr::fixed("clear-font-cache", 0, 0, f_clear_font_cache),
    crate::subr::Subr::fixed("open-font", 1, 3, f_open_font),
    crate::subr::Subr::fixed("close-font", 1, 2, f_close_font),
    crate::subr::Subr::fixed("query-font", 1, 1, f_query_font),
    crate::subr::Subr::fixed("font-has-char-p", 2, 3, f_font_has_char_p),
    crate::subr::Subr::fixed("font-match-p", 2, 2, f_font_match_p),
    crate::subr::Subr::fixed("frame-font-cache", 0, 1, f_frame_font_cache),
    crate::subr::Subr::fixed("font-info", 1, 2, f_font_info),
];

fn f_internal_char_font(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (position, ch) = (args[0], args[1]);
    let c: i64;
    let f: Value;
    if position.is_nil() {
        c = lisp.check_character(ch)? as i64;
        f = lisp.frames.selected_frame;
        lisp.lookup_basic_face(None, f, crate::xfaces::DEFAULT_FACE_ID)?;
    } else {
        let fixed_pos = lisp.fix_position(position)?;
        let (begv, zv) = (lisp.cur().begv as i64, lisp.cur().zv as i64);
        if !(begv..zv).contains(&fixed_pos) {
            return Err(lisp.args_out_of_range3(position, Value::fixnum(begv), Value::fixnum(zv)));
        }
        let pos = fixed_pos as usize;
        let pos_byte = lisp.cur().text.char_to_byte(pos);
        c = if ch.is_nil() {
            lisp.cur().text.char_at_byte(pos_byte) as i64
        } else {
            lisp.check_fixnat(ch)?
        };
        let current = lisp.current_buffer;
        let window = crate::window::f_get_buffer_window(lisp, &[current, Value::NIL])?;
        if window.is_nil() {
            return Ok(Value::NIL);
        }
        f = lisp.window(window).frame;
        lisp.face_at_buffer_position(window, pos, pos + 100, false, None, 0)?;
    }
    if !(0..=crate::character::MAX_CHAR as i64).contains(&c) {
        return Ok(Value::NIL);
    }
    match lisp.frame(f).output_method {
        crate::terminal::OutputMethod::Initial => {
            let t = lisp.frame(f).terminal;
            Ok(lisp.terminal_glyph_code(t, c as u32))
        }
        crate::terminal::OutputMethod::Pgtk => unreachable!(),
    }
}

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
    lisp.fonts.charset_alist = Value::NIL;
    lisp.fonts.scratch_font_spec = lisp.font_make_spec();
    lisp.fonts.scratch_font_prefer = lisp.font_make_spec();
    lisp.fonts.font_log_deferred = lisp.vector(vec![Value::NIL; 3]);
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
            object: None,
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

pub fn f_font_spec(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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
            let name = lisp.string_bytes(lisp.symbol_name(key)).to_vec();
            return Err(lisp.verror("No value for key `%s'", &[Arg::Str(&name)]));
        }
        let val = args[i + 1];
        if key == sym::K_NAME {
            lisp.check_string(val)?;
            let bytes = lisp.string_bytes(val).to_vec();
            if !lisp.font_parse_name(&bytes, spec)? {
                return Err(lisp.verror("Invalid font name: %s", &[Arg::Str(&bytes)]));
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

pub fn f_font_get(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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

pub fn f_font_put(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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
            pixel_size = i64::from(lisp.font_object(font).pixel_size);
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

fn point_to_pixel(point: f64, dpi: f64) -> f64 {
    point * dpi / PT_PER_INCH + 0.5
}

fn characterp(v: Value) -> bool {
    v.as_fixnum()
        .is_some_and(|c| (0..=i64::from(crate::character::MAX_CHAR)).contains(&c))
}

impl Lisp {
    pub fn font_object(&self, v: Value) -> &FontObject {
        self.font(v).object.as_deref().expect("not a font object")
    }

    pub fn font_object_mut(&mut self, v: Value) -> &mut FontObject {
        self.font_mut(v)
            .object
            .as_deref_mut()
            .expect("not a font object")
    }

    fn font_object_driver(&self, v: Value) -> FontDriver {
        self.font_object(v)
            .driver
            .expect("font object without a driver")
    }

    fn font_vector_set(&mut self, v: Value, i: usize, x: Value) {
        if let Vectorlike::Vector(items) = self.heap.vectorlike_mut(v) {
            items[i] = x;
        }
    }

    fn font_memq(&self, elt: Value, mut list: Value) -> bool {
        while list.is_cons() {
            if self.xcar(list) == elt {
                return true;
            }
            list = self.xcdr(list);
        }
        false
    }

    fn font_assoc(&mut self, key: Value, mut list: Value) -> LispResult {
        while list.is_cons() {
            let elt = self.xcar(list);
            if elt.is_cons() {
                let car = self.xcar(elt);
                if car == key || self.equal(car, key)? {
                    return Ok(elt);
                }
            }
            list = self.xcdr(list);
        }
        Ok(Value::NIL)
    }

    fn font_list_to_vector(&mut self, mut list: Value) -> Value {
        let mut items = Vec::new();
        while list.is_cons() {
            items.push(self.xcar(list));
            list = self.xcdr(list);
        }
        self.vector(items)
    }

    pub fn font_build_object(&mut self, type_: Value, entity: Value, pixelsize: f64) -> LispResult {
        let pixel_size = pixelsize as i64;
        let font_object = self.font_make_object(entity, pixel_size)?;
        self.font_aset(font_object, FONT_TYPE_INDEX, type_);
        if let Some(xlfd_name) = self.font_unparse_xlfd(entity, pixel_size, usize::MAX) {
            let name = self.string(&xlfd_name);
            self.font_aset(font_object, FONT_NAME_INDEX, name);
        }
        match self.font_unparse_fcname(entity, pixel_size, 256) {
            Some(name) if !name.is_empty() => {
                let name = self.string(&name);
                self.font_aset(font_object, FONT_FULLNAME_INDEX, name);
            }
            _ => {
                let name = self.font_aref(font_object, FONT_NAME_INDEX);
                self.font_aset(font_object, FONT_FULLNAME_INDEX, name);
            }
        }
        Ok(font_object)
    }

    pub fn font_pixel_size(&self, f: Value, spec: Value) -> i64 {
        let size = self.font_aref(spec, FONT_SIZE_INDEX);
        if let Some(size) = size.as_fixnum() {
            return size;
        }
        if size.is_nil() {
            return 0;
        }
        if self.frame_window_p(f) {
            let point_size = self.heap.float_value(size);
            let dpi = match self.font_aref(spec, FONT_DPI_INDEX).as_fixnum() {
                Some(dpi) => dpi,
                None => self.frame_res(f) as i64,
            };
            return point_to_pixel(point_size, dpi as f64) as i64;
        }
        1
    }

    pub fn find_font_encoding(&mut self, fontname: Value) -> LispResult {
        let mut tail = self.find_symbol_value(sym::FONT_ENCODING_ALIST);
        while tail.is_cons() {
            let elt = self.xcar(tail);
            if elt.is_cons() && self.xcar(elt).is_string() {
                let regexp = self.xcar(elt);
                let canon = self.ascii_canon_table;
                if self.fast_string_match(regexp, fontname, canon)? >= 0 {
                    let encoding = self.xcdr(elt);
                    let valid = if encoding.is_symbol() {
                        self.charsetp(encoding)
                    } else {
                        encoding.is_cons() && {
                            let charset = self.xcar(encoding);
                            self.charsetp(charset)
                        }
                    };
                    if valid {
                        return Ok(encoding);
                    }
                }
            }
            tail = self.xcdr(tail);
        }
        Ok(Value::NIL)
    }

    fn font_charset_alist_append(&mut self, registry: Value, val: Value) {
        let entry = self.cons(registry, val);
        let cell = self.cons(entry, Value::NIL);
        let mut tail = self.fonts.charset_alist;
        if !tail.is_cons() {
            self.fonts.charset_alist = cell;
            return;
        }
        while self.xcdr(tail).is_cons() {
            tail = self.xcdr(tail);
        }
        self.heap.set_cdr(tail, cell);
    }

    pub fn font_registry_charsets(
        &mut self,
        registry: Value,
    ) -> LispResult<Option<(i32, Option<i32>)>> {
        let alist = self.fonts.charset_alist;
        let val = crate::minibuf::f_assoc_string(self, &[registry, alist, sym::T])?;
        let (encoding_id, repertory_id) = if !val.is_nil() {
            let val = self.xcdr(val);
            if val.is_nil() {
                return Ok(None);
            }
            (
                self.xcar(val).as_fixnum().unwrap_or(-1),
                self.xcdr(val).as_fixnum().unwrap_or(-1),
            )
        } else {
            let name = self.symbol_name(registry);
            let val = self.find_font_encoding(name)?;
            let ids = if val.is_symbol() && self.charsetp(val) {
                let id = i64::from(self.check_charset_get_id(val)?);
                Some((id, id))
            } else if val.is_cons() {
                let encoding = self.xcar(val);
                let repertory = self.xcdr(val);
                if !self.charsetp(encoding) {
                    None
                } else {
                    let encoding_id = i64::from(self.check_charset_get_id(encoding)?);
                    if repertory.is_nil() {
                        Some((encoding_id, -1))
                    } else if self.charsetp(repertory) {
                        Some((
                            encoding_id,
                            i64::from(self.check_charset_get_id(repertory)?),
                        ))
                    } else {
                        None
                    }
                }
            } else {
                None
            };
            match ids {
                Some((encoding_id, repertory_id)) => {
                    let val = self.cons(Value::fixnum(encoding_id), Value::fixnum(repertory_id));
                    self.font_charset_alist_append(registry, val);
                    (encoding_id, repertory_id)
                }
                None => {
                    self.font_charset_alist_append(registry, Value::NIL);
                    return Ok(None);
                }
            }
        };
        Ok(Some((
            encoding_id as i32,
            (repertory_id >= 0).then_some(repertory_id as i32),
        )))
    }

    fn font_rescale_ratio(&mut self, font_entity: Value) -> LispResult<f64> {
        let mut name = Value::NIL;
        let mut tail = self.find_symbol_value(sym::FACE_FONT_RESCALE_ALIST);
        while tail.is_cons() {
            let elt = self.xcar(tail);
            let ratio = self.cdr_safe(elt);
            if ratio.is_float() {
                let key = self.car_safe(elt);
                if key.is_string() {
                    if name.is_nil() {
                        name = f_font_xlfd_name(self, &[font_entity, Value::NIL, sym::T])?;
                    }
                    let canon = self.ascii_canon_table;
                    if !name.is_nil() && self.fast_string_match(key, name, canon)? >= 0 {
                        return Ok(self.heap.float_value(ratio));
                    }
                } else if self.font_spec_p(key) && self.font_match_p(key, font_entity)? {
                    return Ok(self.heap.float_value(ratio));
                }
            }
            tail = self.xcdr(tail);
        }
        Ok(1.0)
    }

    fn font_score(&mut self, entity: Value, spec_prop: &[Value]) -> LispResult<u32> {
        let mut score: u32 = 0;
        for (i, &prop) in spec_prop
            .iter()
            .enumerate()
            .take(FONT_WIDTH_INDEX + 1)
            .skip(FONT_WEIGHT_INDEX)
        {
            let value = self.font_aref(entity, i);
            if !prop.is_nil()
                && value != prop
                && let Some(value) = value.as_fixnum()
            {
                let diff = (value >> 8) - (prop.as_fixnum().unwrap_or(0) >> 8);
                score |= (diff.abs().min(127) as u32) << self.fonts.sort_shift_bits[i];
            }
        }
        let entity_size = self
            .font_aref(entity, FONT_SIZE_INDEX)
            .as_fixnum()
            .unwrap_or(0);
        if !spec_prop[FONT_SIZE_INDEX].is_nil() && entity_size > 0 {
            let mut pixel_size = spec_prop[FONT_SIZE_INDEX].as_fixnum().unwrap_or(0);
            if self
                .find_symbol_value(sym::FACE_FONT_RESCALE_ALIST)
                .is_cons()
            {
                pixel_size = (pixel_size as f64 * self.font_rescale_ratio(entity)?) as i64;
            }
            if pixel_size * 2 < entity_size || entity_size * 2 < pixel_size {
                return Ok(0xFFFFFFFF);
            }
            let mut diff = (pixel_size - entity_size).abs() << 1;
            if !spec_prop[FONT_DPI_INDEX].is_nil()
                && spec_prop[FONT_DPI_INDEX] != self.font_aref(entity, FONT_DPI_INDEX)
            {
                diff |= 1;
            }
            if !spec_prop[FONT_AVGWIDTH_INDEX].is_nil()
                && spec_prop[FONT_AVGWIDTH_INDEX] != self.font_aref(entity, FONT_AVGWIDTH_INDEX)
            {
                diff |= 1;
            }
            score |= (diff.min(127) as u32) << self.fonts.sort_shift_bits[FONT_SIZE_INDEX];
        }
        Ok(score)
    }

    fn font_vconcat_entity_vectors(&mut self, mut list: Value) -> Value {
        let mut items = Vec::new();
        while list.is_cons() {
            let vec = self.xcar(list);
            items.extend_from_slice(self.vector_items(vec));
            list = self.xcdr(list);
        }
        self.vector(items)
    }

    fn font_sort_entities(
        &mut self,
        list: Value,
        prefer: Value,
        f: Value,
        best_only: i32,
    ) -> LispResult {
        let mut prefer_prop = [Value::NIL; FONT_SPEC_MAX];
        for (i, prop) in prefer_prop
            .iter_mut()
            .enumerate()
            .take(FONT_AVGWIDTH_INDEX + 1)
            .skip(FONT_WEIGHT_INDEX)
        {
            *prop = self.font_aref(prefer, i);
        }
        if prefer_prop[FONT_SIZE_INDEX].is_float() {
            prefer_prop[FONT_SIZE_INDEX] = Value::fixnum(self.font_pixel_size(f, prefer));
        }
        let mut vec = Value::NIL;
        if self.xcdr(list).is_nil() {
            vec = self.xcar(list);
        } else if best_only == 0 {
            vec = self.font_vconcat_entity_vectors(list);
        }
        let base = self.stack.len();
        self.stack.push(vec);
        let mut best_score: u32 = 0xFFFFFFFF;
        let mut best_entity = Value::NIL;
        let mut tail = list;
        while tail.is_cons() {
            let mut font_driver_preference = 0;
            if best_only != 0 {
                vec = self.xcar(tail);
            }
            let entities = self.vector_items(vec).to_vec();
            let mut current_font_driver = self.font_aref(entities[0], FONT_TYPE_INDEX);
            let mut data: Vec<(u32, i32, Value)> = Vec::with_capacity(entities.len());
            for entity in entities {
                let score = if best_only <= 0 || self.font_has_char(f, entity, best_only) > 0 {
                    match self.font_score(entity, &prefer_prop) {
                        Ok(score) => score,
                        Err(e) => {
                            self.stack.truncate(base);
                            return Err(e);
                        }
                    }
                } else {
                    0xFFFFFFFF
                };
                if best_only != 0 && best_score > score {
                    best_score = score;
                    best_entity = entity;
                    if best_score == 0 {
                        break;
                    }
                }
                let entity_type = self.font_aref(entity, FONT_TYPE_INDEX);
                if current_font_driver != entity_type {
                    current_font_driver = entity_type;
                    font_driver_preference += 1;
                }
                data.push((score, font_driver_preference, entity));
            }
            if best_only == 0 {
                data.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
                for (i, &(_, _, entity)) in data.iter().enumerate() {
                    self.font_vector_set(vec, i, entity);
                }
                break;
            }
            vec = best_entity;
            tail = self.xcdr(tail);
        }
        self.stack.truncate(base);
        self.font_add_log("sort-by", prefer, vec)?;
        Ok(vec)
    }

    fn font_check_otf_features(
        &mut self,
        script: Value,
        langsys: Value,
        mut features: Value,
        table: Value,
    ) -> bool {
        let mut table = self.assq(script, table);
        if table.is_nil() {
            return false;
        }
        table = self.xcdr(table);
        if !langsys.is_nil() {
            table = self.assq(langsys, table);
            if table.is_nil() {
                return false;
            }
        } else {
            let val = self.assq(Value::NIL, table);
            table = if val.is_nil() { self.xcar(table) } else { val };
        }
        table = self.xcdr(table);
        let mut negative = false;
        while features.is_cons() {
            let feature = self.xcar(features);
            features = self.xcdr(features);
            if feature.is_nil() {
                negative = true;
                continue;
            }
            if !self.font_memq(feature, table) != negative {
                return false;
            }
        }
        true
    }

    fn font_check_otf(&mut self, spec: Value, otf_capability: Value) -> bool {
        let script = self.xcar(spec);
        let mut spec = self.xcdr(spec);
        let (mut langsys, mut gsub, mut gpos) = (Value::NIL, Value::NIL, Value::NIL);
        if !spec.is_nil() {
            langsys = self.xcar(spec);
            spec = self.xcdr(spec);
            if !spec.is_nil() {
                gsub = self.xcar(spec);
                spec = self.xcdr(spec);
                if !spec.is_nil() {
                    gpos = self.xcar(spec);
                }
            }
        }
        if !gsub.is_nil() {
            let table = self.xcar(otf_capability);
            if !self.font_check_otf_features(script, langsys, gsub, table) {
                return false;
            }
        }
        if !gpos.is_nil() {
            let table = self.xcdr(otf_capability);
            if !self.font_check_otf_features(script, langsys, gpos, table) {
                return false;
            }
        }
        true
    }

    pub fn font_match_p(&mut self, spec: Value, font: Value) -> LispResult<bool> {
        for i in FONT_FOUNDRY_INDEX..=FONT_REGISTRY_INDEX {
            let (s, v) = (self.font_aref(spec, i), self.font_aref(font, i));
            if !s.is_nil() && !v.is_nil() && s != v {
                return Ok(false);
            }
        }
        let mut props = self.font(spec).props.clone();
        if props[FONT_SIZE_INDEX].is_float() {
            let selected = self.frames.selected_frame;
            props[FONT_SIZE_INDEX] = Value::fixnum(self.font_pixel_size(selected, spec));
        }
        if self.font_score(font, &props)? > 0 {
            return Ok(false);
        }
        let mut extra = self.font_aref(spec, FONT_EXTRA_INDEX);
        let font_extra = self.font_aref(font, FONT_EXTRA_INDEX);
        while extra.is_cons() {
            let elt = self.xcar(extra);
            let key = self.xcar(elt);
            let val = self.xcdr(elt);
            if key == sym::K_LANG {
                let val2 = self.assq(key, font_extra);
                if val2.is_nil() {
                    return Ok(false);
                }
                let val2 = self.xcdr(val2);
                if val.is_cons() {
                    if !val2.is_cons() {
                        return Ok(false);
                    }
                    loop {
                        if !self.font_memq(val, val2) {
                            return Ok(false);
                        }
                    }
                } else if if val2.is_cons() {
                    let rest = self.xcdr(val2);
                    !self.font_memq(val, rest)
                } else {
                    val != val2
                } {
                    return Ok(false);
                }
            } else if key == sym::K_SCRIPT {
                let representative = self.find_symbol_value(sym::SCRIPT_REPRESENTATIVE_CHARS);
                let val2 = self.assq(val, representative);
                if val2.is_cons() {
                    let val2 = self.xcdr(val2);
                    if val2.is_cons() {
                        if !self.font_object_p(font) {
                            return Ok(false);
                        }
                        let mut chars = val2;
                        while chars.is_cons() {
                            let c = self.xcar(chars);
                            chars = self.xcdr(chars);
                            if !characterp(c) {
                                continue;
                            }
                            let c = c.as_fixnum().unwrap_or(0) as i32;
                            if self.font_encode_char(font, c) == FONT_INVALID_CODE {
                                return Ok(false);
                            }
                        }
                    } else if self.is_vector(val2) {
                        if !self.font_object_p(font) {
                            return Ok(false);
                        }
                        let chars = self.vector_items(val2).to_vec();
                        let mut found = false;
                        for c in chars {
                            if !characterp(c) {
                                continue;
                            }
                            let c = c.as_fixnum().unwrap_or(0) as i32;
                            if self.font_encode_char(font, c) != FONT_INVALID_CODE {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            return Ok(false);
                        }
                    }
                }
            } else if key == sym::K_OTF {
                if !self.font_object_p(font) {
                    return Ok(false);
                }
                let Some(otf_capability) = self.font_object_driver(font).otf_capability else {
                    return Ok(false);
                };
                let val2 = otf_capability(self, font)?;
                if val2.is_nil() || !self.font_check_otf(val, val2) {
                    return Ok(false);
                }
            }
            extra = self.xcdr(extra);
        }
        Ok(true)
    }

    fn font_prepare_cache(&mut self, f: Value, driver: &FontDriver) {
        let cache = (driver.get_cache)(self, f);
        let mut val = self.xcdr(cache);
        while val.is_cons() && self.xcar(self.xcar(val)) != driver.type_ {
            val = self.xcdr(val);
        }
        if val.is_nil() {
            let entry = self.list(&[driver.type_, Value::fixnum(1)]);
            let rest = self.xcdr(cache);
            let cell = self.cons(entry, rest);
            self.heap.set_cdr(cache, cell);
        } else {
            let val = self.xcdr(self.xcar(val));
            let n = self.xcar(val).as_fixnum().unwrap_or(0);
            self.heap.set_car(val, Value::fixnum(n + 1));
        }
    }

    fn font_finish_cache(&mut self, f: Value, driver: &FontDriver) {
        let mut cache = (driver.get_cache)(self, f);
        let mut val = self.xcdr(cache);
        while val.is_cons() && self.xcar(self.xcar(val)) != driver.type_ {
            cache = val;
            val = self.xcdr(val);
        }
        let tmp = self.xcdr(self.xcar(val));
        let n = self.xcar(tmp).as_fixnum().unwrap_or(0) - 1;
        self.heap.set_car(tmp, Value::fixnum(n));
        if n == 0 {
            let entry = self.xcar(val);
            self.font_clear_cache(f, entry, driver);
            let rest = self.xcdr(val);
            self.heap.set_cdr(cache, rest);
        }
    }

    fn font_get_cache(&mut self, f: Value, driver: &FontDriver) -> Value {
        let cache = (driver.get_cache)(self, f);
        let mut val = self.xcdr(cache);
        while val.is_cons() && self.xcar(self.xcar(val)) != driver.type_ {
            val = self.xcdr(val);
        }
        let entry = self.car_safe(val);
        self.cdr_safe(entry)
    }

    fn font_clear_cache(&mut self, _f: Value, cache: Value, driver: &FontDriver) {
        let mut tail = self.xcdr(self.xcdr(cache));
        while tail.is_cons() {
            let elt = self.xcar(tail);
            if elt.is_cons() && self.font_spec_p(self.xcar(elt)) {
                let entities = self.vector_items(self.xcdr(elt)).to_vec();
                for entity in entities {
                    if self.font_entity_p(entity)
                        && driver.type_ == self.font_aref(entity, FONT_TYPE_INDEX)
                    {
                        let mut objlist = self.font_aref(entity, FONT_OBJLIST_INDEX);
                        while objlist.is_cons() {
                            let val = self.xcar(objlist);
                            if !self.font_aref(val, FONT_TYPE_INDEX).is_nil() {
                                self.composition_gstring_cache_clear_font(val);
                                (driver.close_font)(self, val);
                            }
                            objlist = self.xcdr(objlist);
                        }
                        if let Some(free_entity) = driver.free_entity {
                            free_entity(self, entity);
                        }
                    }
                }
            }
            tail = self.xcdr(tail);
        }
        self.heap.set_cdr(cache, Value::NIL);
    }

    pub fn font_is_ignored(&mut self, name: &[u8]) -> LispResult<bool> {
        let mut tail = self.find_symbol_value(sym::FACE_IGNORED_FONTS);
        while tail.is_cons() {
            let regexp = self.xcar(tail);
            if regexp.is_string() && self.fast_c_string_match_ignore_case(regexp, name)? >= 0 {
                return Ok(true);
            }
            tail = self.xcdr(tail);
        }
        Ok(false)
    }

    fn font_delete_unmatched(&mut self, vec: Value, spec: Value, size: i64) -> LispResult {
        let entities = self.vector_items(vec).to_vec();
        let mut val = Vec::new();
        for &entity in entities.iter().rev() {
            if !self.find_symbol_value(sym::FACE_IGNORED_FONTS).is_nil() {
                match self.font_unparse_xlfd(entity, 0, 256) {
                    Some(name) => {
                        if self.font_is_ignored(name.as_bytes())? {
                            continue;
                        }
                    }
                    None => {
                        if let Some(xlfd) = self.font_unparse_xlfd(entity, 0, usize::MAX) {
                            let n = xlfd.len().min(std::mem::size_of::<usize>());
                            if self.font_is_ignored(&xlfd.as_bytes()[..n])? {
                                continue;
                            }
                        }
                    }
                }
            }
            if spec.is_nil() {
                val.push(entity);
                continue;
            }
            let mut matched = true;
            for prop in FONT_WEIGHT_INDEX..FONT_SIZE_INDEX {
                if let Some(required) = self.font_aref(spec, prop).as_fixnum() {
                    match self.font_aref(entity, prop).as_fixnum() {
                        None => {
                            matched = false;
                            break;
                        }
                        Some(candidate) => {
                            if candidate >> 8 != required >> 8 {
                                matched = false;
                                break;
                            }
                        }
                    }
                }
            }
            let entity_size = self
                .font_aref(entity, FONT_SIZE_INDEX)
                .as_fixnum()
                .unwrap_or(0);
            if matched
                && size != 0
                && entity_size > 0
                && (entity_size - size).abs() > FONT_PIXEL_SIZE_QUANTUM
            {
                matched = false;
            }
            for prop in [FONT_DPI_INDEX, FONT_AVGWIDTH_INDEX] {
                let (s, e) = (self.font_aref(spec, prop), self.font_aref(entity, prop));
                if matched && s.is_fixnum() && e.is_fixnum() && e.as_fixnum() != Some(0) && s != e {
                    matched = false;
                }
            }
            if matched {
                val.push(entity);
            }
        }
        val.reverse();
        Ok(self.vector(val))
    }

    pub fn font_list_entities(&mut self, f: Value, spec: Value) -> LispResult {
        let size_val = self.font_aref(spec, FONT_SIZE_INDEX);
        let size = if let Some(size) = size_val.as_fixnum() {
            size
        } else if size_val.is_float() {
            self.font_pixel_size(f, spec)
        } else {
            0
        };
        let ftype = self.font_aref(spec, FONT_TYPE_INDEX);
        let scratch = self.fonts.scratch_font_spec;
        for i in FONT_FOUNDRY_INDEX..=FONT_REGISTRY_INDEX {
            let v = self.font_aref(spec, i);
            self.font_aset(scratch, i, v);
        }
        let mut need_filtering = false;
        for i in FONT_WEIGHT_INDEX..FONT_EXTRA_INDEX {
            if i != FONT_SPACING_INDEX {
                self.font_aset(scratch, i, Value::NIL);
                if !self.font_aref(spec, i).is_nil() {
                    need_filtering = true;
                }
            }
        }
        let spacing = self.font_aref(spec, FONT_SPACING_INDEX);
        self.font_aset(scratch, FONT_SPACING_INDEX, spacing);
        let extra = self.font_aref(spec, FONT_EXTRA_INDEX);
        self.font_aset(scratch, FONT_EXTRA_INDEX, extra);
        let base = self.stack.len();
        let list = self.font_list_entities_1(f, spec, ftype, size, need_filtering);
        self.stack.truncate(base);
        let list = list?;
        self.font_add_log("list", spec, list)?;
        Ok(list)
    }

    fn font_list_entities_1(
        &mut self,
        f: Value,
        spec: Value,
        ftype: Value,
        size: i64,
        need_filtering: bool,
    ) -> LispResult {
        let scratch = self.fonts.scratch_font_spec;
        let drivers = self.frame(f).font_driver_list.clone();
        let mut list = Vec::new();
        for driver_list in drivers {
            let driver = driver_list.driver;
            if !driver_list.on || !(ftype.is_nil() || driver.type_ == ftype) {
                continue;
            }
            let cache = self.font_get_cache(f, &driver);
            self.font_aset(scratch, FONT_TYPE_INDEX, driver.type_);
            let cached = self.font_assoc(scratch, self.xcdr(cache))?;
            let mut val = if cached.is_cons() {
                self.xcdr(cached)
            } else {
                let val = (driver.list)(self, f, scratch)?;
                let val = self.font_list_to_vector(val);
                self.stack.push(val);
                let copy = self.copy_font_spec(scratch)?;
                self.font_aset(copy, FONT_TYPE_INDEX, driver.type_);
                let entry = self.cons(copy, val);
                let rest = self.xcdr(cache);
                let cell = self.cons(entry, rest);
                self.heap.set_cdr(cache, cell);
                val
            };
            let ignored = self.find_symbol_value(sym::FACE_IGNORED_FONTS);
            if !self.vector_items(val).is_empty() && (need_filtering || !ignored.is_nil()) {
                let filter = if need_filtering { spec } else { Value::NIL };
                val = self.font_delete_unmatched(val, filter, size)?;
            }
            if !self.vector_items(val).is_empty() {
                self.stack.push(val);
                list.push(val);
                if self
                    .find_symbol_value(sym::QUERY_ALL_FONT_BACKENDS)
                    .is_nil()
                {
                    break;
                }
            }
        }
        Ok(self.list(&list))
    }

    fn font_matching_entity(&mut self, f: Value, attrs: &[Value], spec: Value) -> LispResult {
        let work = self.copy_font_spec(spec)?;
        let base = self.stack.len();
        self.stack.push(work);
        let r = self.font_matching_entity_1(f, attrs, spec, work);
        self.stack.truncate(base);
        r
    }

    fn font_matching_entity_1(
        &mut self,
        f: Value,
        attrs: &[Value],
        spec: Value,
        work: Value,
    ) -> LispResult {
        use crate::xfaces::{LFACE_SLANT_INDEX, LFACE_SWIDTH_INDEX, LFACE_WEIGHT_INDEX};
        let ftype = self.font_aref(spec, FONT_TYPE_INDEX);
        if self.font_aref(spec, FONT_SIZE_INDEX).is_float() {
            let size = self.font_pixel_size(f, spec);
            self.font_aset(work, FONT_SIZE_INDEX, Value::fixnum(size));
        }
        self.font_set_style(work, FONT_WEIGHT_INDEX, attrs[LFACE_WEIGHT_INDEX])?;
        self.font_set_style(work, FONT_SLANT_INDEX, attrs[LFACE_SLANT_INDEX])?;
        self.font_set_style(work, FONT_WIDTH_INDEX, attrs[LFACE_SWIDTH_INDEX])?;
        let mut entity = Value::NIL;
        let drivers = self.frame(f).font_driver_list.clone();
        for driver_list in drivers {
            let driver = driver_list.driver;
            if !driver_list.on || !(ftype.is_nil() || driver.type_ == ftype) {
                continue;
            }
            let cache = self.font_get_cache(f, &driver);
            self.font_aset(work, FONT_TYPE_INDEX, driver.type_);
            let rest = self.xcdr(cache);
            entity = self.font_assoc(work, rest)?;
            if entity.is_cons() {
                let matches = self.xcdr(entity);
                entity = self
                    .vector_items(matches)
                    .first()
                    .copied()
                    .unwrap_or(Value::NIL);
            } else {
                entity = (driver.match_font)(self, f, work)?;
                if !entity.is_nil() {
                    self.stack.push(entity);
                    let copy = self.copy_font_spec(work)?;
                    let matched = self.vector(vec![entity]);
                    self.font_aset(copy, FONT_TYPE_INDEX, driver.type_);
                    let entry = self.cons(copy, matched);
                    let rest = self.xcdr(cache);
                    let cell = self.cons(entry, rest);
                    self.heap.set_cdr(cache, cell);
                }
            }
            if !entity.is_nil() {
                break;
            }
        }
        self.font_add_log("match", work, entity)?;
        Ok(entity)
    }

    pub fn font_open_entity(&mut self, f: Value, entity: Value, pixel_size: i64) -> LispResult {
        let mut pixel_size = pixel_size;
        let size = self
            .font_aref(entity, FONT_SIZE_INDEX)
            .as_fixnum()
            .unwrap_or(0);
        if size != 0 {
            pixel_size = size;
        }
        let ty = self.font_aref(entity, FONT_TYPE_INDEX);
        let Some(driver) = self
            .frame(f)
            .font_driver_list
            .iter()
            .find(|list| list.driver.type_ == ty)
            .map(|list| list.driver)
        else {
            return Ok(Value::NIL);
        };
        let mut objlist = self.font_aref(entity, FONT_OBJLIST_INDEX);
        while objlist.is_cons() {
            let font = self.xcar(objlist);
            if !self.font_aref(font, FONT_TYPE_INDEX).is_nil()
                && i64::from(self.font_object(font).pixel_size) == pixel_size
                && driver
                    .cached_font_ok
                    .is_none_or(|cached_font_ok| cached_font_ok(self, f, font, entity))
            {
                return Ok(font);
            }
            objlist = self.xcdr(objlist);
        }
        let mut psize = pixel_size;
        let font_object = loop {
            let font_object = (driver.open_font)(self, f, entity, psize as i32)?;
            if font_object.is_nil() {
                return Ok(Value::NIL);
            }
            let font = self.font_object(font_object);
            if font.average_width > 0 && font.height > 0 {
                break font_object;
            }
            if psize > pixel_size + 15 {
                return Ok(Value::NIL);
            }
            psize += 1;
        };
        self.font_aset(font_object, FONT_SIZE_INDEX, Value::fixnum(pixel_size));
        self.font_add_log("open", entity, font_object)?;
        let objects = self.font_aref(entity, FONT_OBJLIST_INDEX);
        let objects = self.cons(font_object, objects);
        self.font_aset(entity, FONT_OBJLIST_INDEX, objects);
        let font = self.font_object(font_object);
        let min_width = if font.min_width != 0 {
            font.min_width
        } else if font.average_width != 0 {
            font.average_width
        } else if font.space_width != 0 {
            font.space_width
        } else {
            1
        };
        let (font_ascent, font_descent) = self.get_font_ascent_descent(font_object);
        let height = (font_ascent + font_descent).max(1);
        if let Some(terminal) = self.frame_display_info(f) {
            let info = self.display_info_mut(terminal);
            info.n_fonts += 1;
            let mut changed = false;
            if info.n_fonts == 1 {
                info.smallest_char_width = min_width;
                info.smallest_font_height = height;
                changed = true;
            } else {
                if info.smallest_char_width > min_width {
                    info.smallest_char_width = min_width;
                    changed = true;
                }
                if info.smallest_font_height > height {
                    info.smallest_font_height = height;
                    changed = true;
                }
            }
            if changed {
                self.frame_mut(f).fonts_changed = true;
            }
        }
        Ok(font_object)
    }

    fn font_close_object(&mut self, f: Value, font_object: Value) -> LispResult<()> {
        if self.font_aref(font_object, FONT_TYPE_INDEX).is_nil() {
            return Ok(());
        }
        self.font_add_log("close", font_object, Value::NIL)?;
        let driver = self.font_object_driver(font_object);
        (driver.close_font)(self, font_object);
        if let Some(terminal) = self.frame_display_info(f) {
            self.display_info_mut(terminal).n_fonts -= 1;
        }
        Ok(())
    }

    pub fn font_has_char(&mut self, f: Value, font: Value, c: i32) -> i32 {
        if self.font_entity_p(font) {
            let ty = self.font_aref(font, FONT_TYPE_INDEX);
            let Some(driver) = self
                .frame(f)
                .font_driver_list
                .iter()
                .find(|list| list.driver.type_ == ty)
                .map(|list| list.driver)
            else {
                return 0;
            };
            return match driver.has_char {
                Some(has_char) => has_char(self, font, c),
                None => -1,
            };
        }
        let driver = self.font_object_driver(font);
        if let Some(has_char) = driver.has_char {
            let result = has_char(self, font, c);
            if result >= 0 {
                return result;
            }
        }
        i32::from((driver.encode_char)(self, font, c) != FONT_INVALID_CODE)
    }

    pub fn font_encode_char(&mut self, font_object: Value, c: i32) -> u32 {
        let driver = self.font_object_driver(font_object);
        (driver.encode_char)(self, font_object, c)
    }

    pub fn font_get_name(&self, font_object: Value) -> Value {
        self.font_aref(font_object, FONT_NAME_INDEX)
    }

    fn font_select_entity(
        &mut self,
        f: Value,
        entities: Value,
        attrs: &[Value],
        pixel_size: i64,
        c: i32,
    ) -> LispResult {
        use crate::xfaces::{
            LFACE_FONT_INDEX, LFACE_SLANT_INDEX, LFACE_SWIDTH_INDEX, LFACE_WEIGHT_INDEX,
        };
        let first = self.xcar(entities);
        if self.xcdr(entities).is_nil() && self.vector_items(first).len() == 1 {
            let font_entity = self.vector_items(first)[0];
            if c < 0 || self.font_has_char(f, font_entity, c) > 0 {
                return Ok(font_entity);
            }
            return Ok(Value::NIL);
        }
        let prefer = self.fonts.scratch_font_prefer;
        for i in FONT_WEIGHT_INDEX..=FONT_SIZE_INDEX {
            self.font_aset(prefer, i, Value::NIL);
        }
        let face_font = attrs[LFACE_FONT_INDEX];
        if self.is_font(face_font) {
            for i in FONT_WEIGHT_INDEX..=FONT_SIZE_INDEX {
                let v = self.font_aref(face_font, i);
                self.font_aset(prefer, i, v);
            }
        }
        if self.font_aref(prefer, FONT_WEIGHT_INDEX).is_nil() {
            self.font_set_style(prefer, FONT_WEIGHT_INDEX, attrs[LFACE_WEIGHT_INDEX])?;
        }
        if self.font_aref(prefer, FONT_SLANT_INDEX).is_nil() {
            self.font_set_style(prefer, FONT_SLANT_INDEX, attrs[LFACE_SLANT_INDEX])?;
        }
        if self.font_aref(prefer, FONT_WIDTH_INDEX).is_nil() {
            self.font_set_style(prefer, FONT_WIDTH_INDEX, attrs[LFACE_SWIDTH_INDEX])?;
        }
        self.font_aset(prefer, FONT_SIZE_INDEX, Value::fixnum(pixel_size));
        self.font_sort_entities(entities, prefer, f, c)
    }

    pub fn font_find_for_lface(
        &mut self,
        f: Value,
        attrs: &[Value],
        spec: Value,
        c: i32,
    ) -> LispResult {
        let spec_registry = self.font_aref(spec, FONT_REGISTRY_INDEX);
        let registry = if spec_registry.is_nil() {
            vec![sym::ISO8859_1, sym::ASCII_0]
        } else {
            vec![spec_registry]
        };
        if c >= 0 && !spec_registry.is_nil() {
            let Some((encoding, repertory)) = self.font_registry_charsets(spec_registry)? else {
                return Ok(Value::NIL);
            };
            let unencodable = match repertory {
                Some(repertory) => {
                    self.encode_char(repertory, c)? == self.charset(repertory).invalid_code
                }
                None => false,
            };
            if unencodable || c > self.charset(encoding).max_char {
                return Ok(Value::NIL);
            }
        }
        let work = self.copy_font_spec(spec)?;
        let base = self.stack.len();
        self.stack.push(work);
        let r = self.font_find_for_lface_1(f, attrs, spec, c, work, &registry);
        self.stack.truncate(base);
        r
    }

    fn font_find_for_lface_1(
        &mut self,
        f: Value,
        attrs: &[Value],
        spec: Value,
        c: i32,
        work: Value,
        registry: &[Value],
    ) -> LispResult {
        use crate::xfaces::{
            LFACE_FAMILY_INDEX, LFACE_FONT_INDEX, LFACE_FOUNDRY_INDEX, LFACE_HEIGHT_INDEX,
        };
        let ty = self.font_aref(spec, FONT_TYPE_INDEX);
        self.font_aset(work, FONT_TYPE_INDEX, ty);
        let mut pixel_size = self.font_pixel_size(f, spec);
        if pixel_size == 0
            && let Some(height) = attrs[LFACE_HEIGHT_INDEX].as_fixnum()
        {
            let pt = height as f64;
            pixel_size = point_to_pixel(pt / 10.0, self.frame_res(f)) as i64;
            if pixel_size < 1 {
                pixel_size = 1;
            }
        }
        self.font_aset(work, FONT_SIZE_INDEX, Value::NIL);
        let foundry0 = self.font_aref(work, FONT_FOUNDRY_INDEX);
        let foundry = if !foundry0.is_nil() {
            vec![foundry0]
        } else if attrs[LFACE_FOUNDRY_INDEX].is_string() {
            let name = self.string_bytes(attrs[LFACE_FOUNDRY_INDEX]).to_vec();
            vec![self.font_intern_prop(&name, true)?, Value::NIL]
        } else {
            vec![Value::NIL]
        };
        let adstyle0 = self.font_aref(work, FONT_ADSTYLE_INDEX);
        let adstyle = if !adstyle0.is_nil() {
            vec![adstyle0]
        } else if self.is_font(attrs[LFACE_FONT_INDEX]) {
            let face_adstyle = self.font_aref(attrs[LFACE_FONT_INDEX], FONT_ADSTYLE_INDEX);
            if face_adstyle.is_nil() {
                vec![Value::NIL]
            } else {
                vec![face_adstyle, Value::NIL]
            }
        } else {
            vec![Value::NIL]
        };
        let mut val = self.font_aref(work, FONT_FAMILY_INDEX);
        if val.is_nil() && attrs[LFACE_FAMILY_INDEX].is_string() {
            let name = self.string_bytes(attrs[LFACE_FAMILY_INDEX]).to_vec();
            val = self.font_intern_prop(&name, true)?;
        }
        let family = if val.is_nil() {
            vec![Value::NIL]
        } else {
            let alist = self.faces.alternative_font_family_alist;
            let mut alters = crate::minibuf::f_assoc_string(self, &[val, alist, sym::T])?;
            let mut family = Vec::new();
            if alters.is_nil() {
                family.push(val);
            } else {
                while alters.is_cons() {
                    family.push(self.xcar(alters));
                    alters = self.xcdr(alters);
                }
            }
            if self.font_aref(spec, FONT_FAMILY_INDEX).is_nil() {
                family.push(Value::NIL);
            }
            family
        };
        for &family in family.iter().take_while(|v| v.is_symbol()) {
            self.font_aset(work, FONT_FAMILY_INDEX, family);
            for &foundry in foundry.iter().take_while(|v| v.is_symbol()) {
                self.font_aset(work, FONT_FOUNDRY_INDEX, foundry);
                for &registry in registry.iter().take_while(|v| v.is_symbol()) {
                    self.font_aset(work, FONT_REGISTRY_INDEX, registry);
                    for &adstyle in adstyle.iter().take_while(|v| v.is_symbol()) {
                        self.font_aset(work, FONT_ADSTYLE_INDEX, adstyle);
                        let entities = self.font_list_entities(f, work)?;
                        if !entities.is_nil() {
                            self.stack.push(entities);
                            let val = self.font_select_entity(f, entities, attrs, pixel_size, c)?;
                            if !val.is_nil() {
                                return Ok(val);
                            }
                        }
                    }
                }
            }
        }
        Ok(Value::NIL)
    }

    pub fn font_open_for_lface(
        &mut self,
        f: Value,
        entity: Value,
        attrs: &[Value],
        spec: Value,
    ) -> LispResult {
        use crate::xfaces::LFACE_HEIGHT_INDEX;
        let entity_size = self.font_aref(entity, FONT_SIZE_INDEX).as_fixnum();
        let size = match entity_size {
            Some(size) if size > 0 => size,
            _ => {
                let size =
                    if self.font_spec_p(spec) && !self.font_aref(spec, FONT_SIZE_INDEX).is_nil() {
                        self.font_pixel_size(f, spec)
                    } else {
                        let pt = match attrs[LFACE_HEIGHT_INDEX].as_fixnum() {
                            Some(height) => height as f64,
                            None => {
                                let used = self
                                    .frame(f)
                                    .face_cache
                                    .as_ref()
                                    .map_or(0, |cache| cache.faces_by_id.len());
                                if used == 0 {
                                    self.recompute_basic_faces(f)?;
                                }
                                let def = self
                                    .face_from_id(f, crate::xfaces::DEFAULT_FACE_ID)
                                    .expect("default face");
                                def.lface[LFACE_HEIGHT_INDEX].as_fixnum().unwrap_or(0) as f64
                            }
                        };
                        point_to_pixel(pt / 10.0, self.frame_res(f)) as i64
                    };
                (size as f64 * self.font_rescale_ratio(entity)?) as i64
            }
        };
        self.font_open_entity(f, entity, size)
    }

    pub fn font_load_for_lface(&mut self, f: Value, attrs: &[Value], spec: Value) -> LispResult {
        let mut entity = self.font_find_for_lface(f, attrs, spec, -1)?;
        if entity.is_nil() {
            entity = self.font_matching_entity(f, attrs, spec)?;
            if entity.is_nil() {
                let name = f_font_get(self, &[spec, sym::K_USER_SPEC])?;
                if name.is_string() {
                    let p = self.string_bytes(name).to_vec();
                    if let Some(q) = p.iter().rposition(|&b| b == b'-')
                        && p.get(q + 1).is_some_and(u8::is_ascii_digit)
                    {
                        let digits = String::from_utf8_lossy(&p[q + 1..]).into_owned();
                        let (font_size, used) = strtod(&digits);
                        if font_size > 0.0 && used != 0 {
                            let lsize = f_font_get(self, &[spec, sym::K_SIZE])?;
                            let same = (lsize.is_float()
                                && self.heap.float_value(lsize) == font_size)
                                || lsize.as_fixnum().is_some_and(|n| n as f64 == font_size);
                            if same {
                                let family = self.font_intern_prop(&p[..q + 1 + used], true)?;
                                self.font_aset(spec, FONT_FAMILY_INDEX, family);
                                self.font_aset(spec, FONT_SIZE_INDEX, Value::NIL);
                                entity = self.font_matching_entity(f, attrs, spec)?;
                            }
                        }
                    }
                }
            }
            if entity.is_nil() {
                return Ok(Value::NIL);
            }
        }
        let base = self.stack.len();
        self.stack.push(entity);
        let entity = self.font_open_for_lface(f, entity, attrs, spec);
        self.stack.truncate(base);
        let entity = entity?;
        if !entity.is_nil() {
            let name = f_font_get(self, &[spec, sym::K_USER_SPEC])?;
            if name.is_string() {
                self.font_put_extra(entity, sym::K_USER_SPEC, name)?;
            }
        }
        Ok(entity)
    }

    pub fn font_prepare_for_face(&mut self, f: Value, face_id: usize) {
        let font = self.face_from_id(f, face_id).expect("face").font;
        if let Some(prepare_face) = self.font_object_driver(font).prepare_face {
            prepare_face(self, f, face_id);
        }
    }

    pub fn font_done_for_face(&mut self, f: Value, face_id: usize) {
        let font = self.face_from_id(f, face_id).expect("face").font;
        if let Some(done_face) = self.font_object_driver(font).done_face {
            done_face(self, f, face_id);
        }
    }

    pub fn font_open_by_spec(&mut self, f: Value, spec: Value) -> LispResult {
        use crate::xfaces::{
            LFACE_HEIGHT_INDEX, LFACE_SLANT_INDEX, LFACE_SWIDTH_INDEX, LFACE_VECTOR_SIZE,
            LFACE_WEIGHT_INDEX,
        };
        let mut attrs = [Value::NIL; LFACE_VECTOR_SIZE];
        attrs[LFACE_SWIDTH_INDEX] = sym::NORMAL;
        attrs[LFACE_WEIGHT_INDEX] = sym::NORMAL;
        attrs[LFACE_SLANT_INDEX] = sym::NORMAL;
        attrs[LFACE_HEIGHT_INDEX] = Value::fixnum(120);
        self.font_load_for_lface(f, &attrs, spec)
    }

    pub fn font_open_by_name(&mut self, f: Value, name: Value) -> LispResult {
        let spec = f_font_spec(self, &[sym::K_NAME, name])?;
        let base = self.stack.len();
        self.stack.push(spec);
        let ret = self.font_open_by_spec(f, spec);
        self.stack.truncate(base);
        let ret = ret?;
        if !ret.is_nil() {
            self.font_put_extra(ret, sym::K_USER_SPEC, name)?;
        }
        Ok(ret)
    }

    pub fn register_font_driver(&mut self, driver: FontDriver, f: Option<Value>) -> LispResult<()> {
        if f.is_some() && driver.draw.is_none() {
            let name = self.string_bytes(self.symbol_name(driver.type_)).to_vec();
            return Err(self.verror("Unusable font driver for a frame: %s", &[Arg::Str(&name)]));
        }
        let duplicated = match f {
            Some(f) => self
                .frame(f)
                .font_driver_list
                .iter()
                .any(|list| list.driver.type_ == driver.type_),
            None => self
                .fonts
                .driver_list
                .iter()
                .any(|d| d.type_ == driver.type_),
        };
        if duplicated {
            let name = self.string_bytes(self.symbol_name(driver.type_)).to_vec();
            return Err(self.verror("Duplicated font driver: %s", &[Arg::Str(&name)]));
        }
        match f {
            Some(f) => self
                .frame_mut(f)
                .font_driver_list
                .push(FontDriverList { on: false, driver }),
            None => {
                self.fonts.driver_list.push(driver);
                self.fonts.num_font_drivers += 1;
            }
        }
        Ok(())
    }

    pub fn free_font_driver_list(&mut self, f: Value) {
        self.frame_mut(f).font_driver_list.clear();
    }

    pub fn font_update_drivers(&mut self, f: Value, new_drivers: Value) -> LispResult {
        let base = self.stack.len();
        let r = self.font_update_drivers_1(f, new_drivers);
        self.stack.truncate(base);
        r
    }

    fn font_update_drivers_1(&mut self, f: Value, new_drivers: Value) -> LispResult {
        let mut all_drivers = Value::NIL;
        let types: Vec<Value> = self
            .frame(f)
            .font_driver_list
            .iter()
            .map(|list| list.driver.type_)
            .collect();
        for ty in types {
            all_drivers = self.cons(ty, all_drivers);
        }
        self.stack.push(all_drivers);
        let mut default_drivers = Value::NIL;
        let mut rest = all_drivers;
        while rest.is_cons() {
            let ty = self.xcar(rest);
            let superseded_by = crate::fns::f_get(self, &[ty, sym::FONT_DRIVER_SUPERSEDED_BY])?;
            if superseded_by.is_nil() || !self.font_memq(superseded_by, all_drivers) {
                default_drivers = self.cons(ty, default_drivers);
                self.stack.push(default_drivers);
            }
            rest = self.xcdr(rest);
        }
        let new_drivers = if new_drivers == sym::T {
            default_drivers
        } else {
            new_drivers
        };
        self.stack.push(new_drivers);
        for idx in 0..self.frame(f).font_driver_list.len() {
            let list = self.frame(f).font_driver_list[idx];
            let driver = list.driver;
            if self.font_memq(driver.type_, new_drivers) != list.on {
                if list.on {
                    if let Some(end_for_frame) = driver.end_for_frame {
                        end_for_frame(self, f);
                    }
                    self.font_finish_cache(f, &driver);
                    self.frame_mut(f).font_driver_list[idx].on = false;
                } else if driver
                    .start_for_frame
                    .is_none_or(|start_for_frame| start_for_frame(self, f) == 0)
                {
                    self.font_prepare_cache(f, &driver);
                    self.frame_mut(f).font_driver_list[idx].on = true;
                }
            }
        }
        if new_drivers.is_nil() {
            return Ok(Value::NIL);
        }
        let current = self.frame(f).font_driver_list.clone();
        let mut table: Vec<FontDriverList> = Vec::with_capacity(current.len());
        let mut tail = new_drivers;
        while tail.is_cons() {
            let ty = self.xcar(tail);
            if let Some(list) = current
                .iter()
                .find(|list| list.on && list.driver.type_ == ty)
                && !table.iter().any(|t| t.driver.type_ == ty)
            {
                table.push(*list);
            }
            tail = self.xcdr(tail);
        }
        table.extend(current.iter().filter(|list| !list.on).copied());
        self.frame_mut(f).font_driver_list = table;
        if !self
            .frame(f)
            .font_driver_list
            .first()
            .is_some_and(|list| list.on)
        {
            for idx in 0..self.frame(f).font_driver_list.len() {
                let driver = self.frame(f).font_driver_list[idx].driver;
                if !self.font_memq(driver.type_, default_drivers) {
                    continue;
                }
                if driver
                    .start_for_frame
                    .is_none_or(|start_for_frame| start_for_frame(self, f) == 0)
                {
                    self.font_prepare_cache(f, &driver);
                    self.frame_mut(f).font_driver_list[idx].on = true;
                }
            }
        }
        let active: Vec<Value> = self
            .frame(f)
            .font_driver_list
            .iter()
            .filter(|list| list.on)
            .map(|list| list.driver.type_)
            .collect();
        Ok(self.list(&active))
    }

    pub fn font_filter_properties(
        &mut self,
        font: Value,
        alist: Value,
        boolean_properties: &[&str],
        non_boolean_properties: &[&str],
    ) -> LispResult<()> {
        for &property in boolean_properties {
            let mut it = alist;
            while !it.is_nil() {
                let elt = self.xcar(it);
                let key = self.xcar(elt);
                let val = self.xcdr(elt);
                if self.symbol_name_string(key) == property {
                    let s = if let Some(n) = val.as_fixnum() {
                        if n != 0 { "true" } else { "false" }.to_string()
                    } else if val.is_symbol() {
                        self.symbol_name_string(val)
                    } else {
                        "true".to_string()
                    };
                    let val = if matches!(
                        s.as_str(),
                        "false" | "False" | "FALSE" | "FcFalse" | "off" | "OFF" | "Off"
                    ) {
                        Value::NIL
                    } else {
                        sym::T
                    };
                    f_font_put(self, &[font, key, val])?;
                }
                it = self.xcdr(it);
            }
        }
        for &property in non_boolean_properties {
            let mut it = alist;
            while !it.is_nil() {
                let elt = self.xcar(it);
                let key = self.xcar(elt);
                let val = self.xcdr(elt);
                if self.symbol_name_string(key) == property {
                    f_font_put(self, &[font, key, val])?;
                }
                it = self.xcdr(it);
            }
        }
        Ok(())
    }

    pub fn clear_font_cache(&mut self, f: Value) {
        let drivers = self.frame(f).font_driver_list.clone();
        for driver_list in drivers {
            if !driver_list.on {
                continue;
            }
            let driver = driver_list.driver;
            let cache = (driver.get_cache)(self, f);
            let mut val = self.xcdr(cache);
            while val.is_cons() && self.xcar(self.xcar(val)) != driver.type_ {
                val = self.xcdr(val);
            }
            let tmp = self.xcdr(self.xcar(val));
            if self.xcar(tmp).as_fixnum() == Some(0) {
                let entry = self.xcar(val);
                self.font_clear_cache(f, entry, &driver);
                let rest = self.xcdr(val);
                self.heap.set_cdr(cache, rest);
            }
        }
    }

    pub fn font_add_log(&mut self, action: &str, arg: Value, result: Value) -> LispResult<()> {
        if self.find_symbol_value(sym::FONT_LOG) == sym::T {
            return Ok(());
        }
        let deferred = self.fonts.font_log_deferred;
        let pending = self.vector_items(deferred)[0];
        if pending.is_string() {
            let pending_action = self.string_to_rust(pending);
            let (pending_arg, pending_result) = (
                self.vector_items(deferred)[1],
                self.vector_items(deferred)[2],
            );
            self.font_vector_set(deferred, 0, Value::NIL);
            self.font_add_log(&pending_action, pending_arg, pending_result)?;
        }
        let mut arg = arg;
        if self.is_font(arg) {
            let mut val = f_font_xlfd_name(self, &[arg, sym::T, sym::T])?;
            let mut tail = self.font_aref(arg, FONT_EXTRA_INDEX);
            while tail.is_cons() {
                let elt = self.xcar(tail);
                let key = self.xcar(elt);
                let value = self.xcdr(elt);
                let shown = if (key == sym::K_SCRIPT || key == sym::K_LANG) && value.is_symbol() {
                    Some(value)
                } else if key == sym::K_OTF && value.is_cons() && self.xcar(value).is_symbol() {
                    Some(self.xcar(value))
                } else {
                    None
                };
                if let Some(shown) = shown {
                    let equal = self.string("=");
                    let key_name = self.symbol_name(key);
                    let shown_name = self.symbol_name(shown);
                    let tail_str = crate::fns::f_concat(self, &[equal, shown_name])?;
                    val = crate::fns::f_concat(self, &[val, key_name, tail_str])?;
                }
                tail = self.xcdr(tail);
            }
            arg = val;
        }
        let mut result = result;
        if result.is_cons() {
            let first = self.xcar(result);
            if self.is_vector(first)
                && self
                    .vector_items(first)
                    .first()
                    .is_some_and(|&v| self.is_font(v))
            {
                result = self.font_vconcat_entity_vectors(result);
            }
        }
        if self.is_font(result) {
            let mut val = f_font_xlfd_name(self, &[result, sym::T, sym::T])?;
            if !self.font_spec_p(result) {
                let ty = self.font_aref(result, FONT_TYPE_INDEX);
                let type_name = self.symbol_name(ty);
                let colon = self.string(":");
                val = crate::fns::f_concat(self, &[type_name, colon, val])?;
            }
            result = val;
        } else if result.is_cons() {
            result = crate::fns::f_copy_sequence(self, &[result])?;
            let mut tail = result;
            while tail.is_cons() {
                let val = self.xcar(tail);
                if self.is_font(val) {
                    let name = f_font_xlfd_name(self, &[val, sym::T, sym::T])?;
                    self.heap.set_car(tail, name);
                }
                tail = self.xcdr(tail);
            }
        } else if self.is_vector(result) {
            result = crate::fns::f_copy_sequence(self, &[result])?;
            for i in 0..self.vector_items(result).len() {
                let val = self.vector_items(result)[i];
                if self.is_font(val) {
                    let name = f_font_xlfd_name(self, &[val, sym::T, sym::T])?;
                    self.font_vector_set(result, i, name);
                }
            }
        }
        let action = self.intern(action);
        let entry = self.list(&[action, arg, result]);
        let log = self.find_symbol_value(sym::FONT_LOG);
        let log = self.cons(entry, log);
        self.set_plain_value(sym::FONT_LOG, log);
        Ok(())
    }

    pub fn font_deferred_log(&mut self, action: &str, arg: Value, result: Value) {
        if self.find_symbol_value(sym::FONT_LOG) == sym::T {
            return;
        }
        let deferred = self.fonts.font_log_deferred;
        let action = self.string(action);
        self.font_vector_set(deferred, 0, action);
        self.font_vector_set(deferred, 1, arg);
        self.font_vector_set(deferred, 2, result);
    }
}

fn f_list_fonts(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let font_spec = args[0];
    let (frame, num, prefer) = (arg(args, 1), arg(args, 2), arg(args, 3));
    let f = lisp.decode_live_frame(frame)?;
    lisp.check_font_spec(font_spec)?;
    let mut n = 0;
    if !num.is_nil() {
        n = lisp.check_fixnum(num)?;
        if n <= 0 {
            return Ok(Value::NIL);
        }
    }
    if !prefer.is_nil() {
        lisp.check_font_spec(prefer)?;
    }
    let list = lisp.font_list_entities(f, font_spec)?;
    if list.is_nil() {
        return Ok(Value::NIL);
    }
    let first = lisp.xcar(list);
    if lisp.xcdr(list).is_nil() && lisp.vector_items(first).len() == 1 {
        let entity = lisp.vector_items(first)[0];
        return Ok(lisp.list(&[entity]));
    }
    let base = lisp.stack.len();
    lisp.stack.push(list);
    let vec = if !prefer.is_nil() {
        lisp.font_sort_entities(list, prefer, f, 0)
    } else {
        Ok(lisp.font_vconcat_entity_vectors(list))
    };
    lisp.stack.truncate(base);
    let items = lisp.vector_items(vec?).to_vec();
    let n = n as usize;
    Ok(if n == 0 || n >= items.len() {
        lisp.list(&items)
    } else {
        lisp.list(&items[..n])
    })
}

fn f_font_family_list(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    let drivers = lisp.frame(f).font_driver_list.clone();
    let mut list = Value::NIL;
    let base = lisp.stack.len();
    for driver_list in drivers {
        let Some(list_family) = driver_list.driver.list_family else {
            continue;
        };
        let mut val = match list_family(lisp, f) {
            Ok(val) => val,
            Err(e) => {
                lisp.stack.truncate(base);
                return Err(e);
            }
        };
        let tail = list;
        while val.is_cons() {
            let family = lisp.xcar(val);
            if !lisp.font_memq(family, tail) && family.is_symbol() {
                let name = lisp.symbol_name(family);
                list = lisp.cons(name, list);
                lisp.stack.push(list);
            }
            val = lisp.xcdr(val);
        }
    }
    lisp.stack.truncate(base);
    Ok(list)
}

fn f_find_font(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let val = f_list_fonts(lisp, &[args[0], arg(args, 1), Value::fixnum(1), Value::NIL])?;
    Ok(if val.is_cons() { lisp.xcar(val) } else { val })
}

fn f_clear_font_cache(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    for f in lisp.frame_list_vec() {
        lisp.clear_font_cache(f);
    }
    Ok(Value::NIL)
}

fn f_open_font(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (font_entity, size) = (args[0], arg(args, 1));
    let f = lisp.decode_live_frame(arg(args, 2))?;
    if !lisp.font_entity_p(font_entity) {
        return Err(lisp.wrong_type(sym::FONT_ENTITY, font_entity));
    }
    let isize = if size.is_nil() {
        lisp.font_aref(font_entity, FONT_SIZE_INDEX)
            .as_fixnum()
            .unwrap_or(0)
    } else {
        if !lisp.is_number(size) {
            return Err(lisp.wrong_type(sym::NUMBERP, size));
        }
        let isize = if size.is_float() {
            let dpi = if lisp.frame_window_p(f) {
                lisp.frame_res(f)
            } else {
                0.0
            };
            point_to_pixel(lisp.heap.float_value(size), dpi) as i64
        } else {
            match size.as_fixnum() {
                Some(n) => n,
                None => return Err(lisp.args_out_of_range(font_entity, size)),
            }
        };
        if !(i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(&isize) {
            return Err(lisp.args_out_of_range(font_entity, size));
        }
        if isize == 0 { 120 } else { isize }
    };
    lisp.font_open_entity(f, font_entity, isize)
}

fn check_font_object(lisp: &mut Lisp, v: Value) -> LispResult<Value> {
    if lisp.font_object_p(v) {
        Ok(v)
    } else {
        Err(lisp.wrong_type(sym::FONT_OBJECT, v))
    }
}

fn f_close_font(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let font_object = check_font_object(lisp, args[0])?;
    let f = lisp.decode_live_frame(arg(args, 1))?;
    lisp.font_close_object(f, font_object)?;
    Ok(Value::NIL)
}

fn font_capability(lisp: &mut Lisp, font_object: Value) -> LispResult {
    match lisp.font_object_driver(font_object).otf_capability {
        Some(otf_capability) => {
            let capability = otf_capability(lisp, font_object)?;
            Ok(lisp.cons(sym::OPENTYPE, capability))
        }
        None => Ok(Value::NIL),
    }
}

fn f_query_font(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let font_object = check_font_object(lisp, args[0])?;
    let capability = font_capability(lisp, font_object)?;
    let font = lisp.font_object(font_object);
    let metrics = [
        font.pixel_size,
        font.max_width,
        font.ascent,
        font.descent,
        font.space_width,
        font.average_width,
    ];
    let mut items = vec![
        lisp.font_aref(font_object, FONT_NAME_INDEX),
        lisp.font_aref(font_object, FONT_FILE_INDEX),
    ];
    items.extend(metrics.iter().map(|&m| Value::fixnum(i64::from(m))));
    items.push(capability);
    Ok(lisp.vector(items))
}

fn f_font_has_char_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (font, ch, frame) = (args[0], args[1], arg(args, 2));
    lisp.check_font(font)?;
    let c = lisp.check_character(ch)?;
    let f = if frame.is_nil() {
        lisp.frames.selected_frame
    } else {
        lisp.check_frame(frame)?
    };
    let r = lisp.font_has_char(f, font, c as i32) > 0;
    Ok(lisp.bool(r))
}

fn f_font_match_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (spec, font) = (args[0], args[1]);
    lisp.check_font_spec(spec)?;
    lisp.check_font(font)?;
    let r = lisp.font_match_p(spec, font)?;
    Ok(lisp.bool(r))
}

fn f_frame_font_cache(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    Ok(match lisp.frame_display_info(f) {
        Some(terminal) => lisp.display_info(terminal).name_list_element,
        None => Value::NIL,
    })
}

fn f_font_info(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut name = args[0];
    if !lisp.is_font(name) {
        lisp.check_string(name)?;
    }
    let f = lisp.decode_window_system_frame(arg(args, 1))?;
    let font_object = if name.is_string() {
        let fontset = lisp.fs_query_fontset(name, 0)?;
        if fontset >= 0 {
            name = lisp.fontset_ascii(fontset as usize);
        }
        lisp.font_open_by_name(f, name)?
    } else if lisp.font_object_p(name) {
        name
    } else if lisp.font_entity_p(name) {
        lisp.font_open_entity(f, name, 0)?
    } else {
        let lface = lisp
            .face_from_id(f, crate::xfaces::DEFAULT_FACE_ID)
            .expect("default face")
            .lface;
        let entity = lisp.font_matching_entity(f, &lface, name)?;
        if entity.is_nil() {
            Value::NIL
        } else {
            lisp.font_open_entity(f, entity, 0)?
        }
    };
    if font_object.is_nil() {
        return Ok(Value::NIL);
    }
    let base = lisp.stack.len();
    lisp.stack.push(font_object);
    let capability = font_capability(lisp, font_object);
    lisp.stack.truncate(base);
    let capability = capability?;
    let font = lisp.font_object(font_object);
    let metrics = [
        font.pixel_size,
        font.height,
        font.baseline_offset,
        font.relative_compose,
        font.default_ascent,
        font.max_width,
        font.ascent,
        font.descent,
        font.space_width,
        font.average_width,
    ];
    let mut items = vec![
        lisp.font_aref(font_object, FONT_NAME_INDEX),
        lisp.font_aref(font_object, FONT_FULLNAME_INDEX),
    ];
    items.extend(metrics.iter().map(|&m| Value::fixnum(i64::from(m))));
    items.push(lisp.font_aref(font_object, FONT_FILE_INDEX));
    items.push(capability);
    Ok(lisp.vector(items))
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}
