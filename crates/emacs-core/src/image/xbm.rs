use super::{Image, ImageKeyword, ImageValueType, PixContainer, lookup_rgb_color, query_color};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const XBM_TYPE: usize = 0;
const XBM_FILE: usize = 1;
const XBM_STRIDE: usize = 4;
const XBM_DATA: usize = 5;
const XBM_FOREGROUND: usize = 6;
const XBM_BACKGROUND: usize = 7;
const XBM_DATA_WIDTH: usize = 14;
const XBM_DATA_HEIGHT: usize = 15;

const XBM_TK_IDENT: i32 = 256;
const XBM_TK_NUMBER: i32 = 257;
const XBM_TK_OVERFLOW: i32 = 258;
const BUFSIZ: usize = 512;

fn xbm_format() -> [ImageKeyword; 16] {
    use ImageValueType as T;
    [
        ImageKeyword::new(":type", T::Symbol, true),
        ImageKeyword::new(":file", T::String, false),
        ImageKeyword::new(":width", T::PositiveInteger, false),
        ImageKeyword::new(":height", T::PositiveInteger, false),
        ImageKeyword::new(":stride", T::PositiveInteger, false),
        ImageKeyword::new(":data", T::DontCheck, false),
        ImageKeyword::new(":foreground", T::StringOrNil, false),
        ImageKeyword::new(":background", T::StringOrNil, false),
        ImageKeyword::new(":ascent", T::Ascent, false),
        ImageKeyword::new(":margin", T::NonNegativeIntegerOrPair, false),
        ImageKeyword::new(":relief", T::Integer, false),
        ImageKeyword::new(":conversion", T::DontCheck, false),
        ImageKeyword::new(":heuristic-mask", T::DontCheck, false),
        ImageKeyword::new(":mask", T::DontCheck, false),
        ImageKeyword::new(":data-width", T::PositiveInteger, false),
        ImageKeyword::new(":data-height", T::PositiveInteger, false),
    ]
}

fn fixnat(v: Value) -> i64 {
    v.as_fixnum().unwrap_or(0)
}

pub(super) fn xbm_image_p(lisp: &mut Lisp, object: Value) -> bool {
    let mut kw = xbm_format();
    if !lisp.parse_image_spec(object, &mut kw, sym::XBM) {
        return false;
    }
    debug_assert_eq!(kw[XBM_TYPE].value, sym::XBM);
    if kw[XBM_FILE].count {
        if kw[XBM_DATA].count {
            return false;
        }
    } else if !(kw[XBM_DATA].count && xbm_file_p(lisp, kw[XBM_DATA].value)) {
        if !kw[XBM_DATA_WIDTH].count || !kw[XBM_DATA_HEIGHT].count || !kw[XBM_DATA].count {
            return false;
        }
        let data = kw[XBM_DATA].value;
        let width = fixnat(kw[XBM_DATA_WIDTH].value);
        let height = fixnat(kw[XBM_DATA_HEIGHT].value);
        let stride = if kw[XBM_STRIDE].count {
            fixnat(kw[XBM_STRIDE].value)
        } else {
            width
        };
        if lisp.is_vector(data) {
            let items = lisp.vector_items(data).to_vec();
            if (items.len() as i64) < height {
                return false;
            }
            for &elt in items.iter().take(height as usize) {
                if elt.is_string() {
                    if (lisp.string_chars(elt) as i64) < stride / 8 {
                        return false;
                    }
                } else if lisp.is_bool_vector(elt) {
                    if (bool_vector_size(lisp, elt) as i64) < width {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        } else if data.is_string() {
            if (lisp.string_chars(data) as i64) < stride / 8 * height {
                return false;
            }
        } else if lisp.is_bool_vector(data) {
            if height > 1 && stride != (width + 7) / 8 * 8 {
                return false;
            }
            if (bool_vector_size(lisp, data) as i64) / height < stride {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn bool_vector_size(lisp: &Lisp, v: Value) -> usize {
    match lisp.heap.vectorlike_ref(v) {
        crate::object::Vectorlike::BoolVector(b) => b.bits.len(),
        _ => 0,
    }
}

fn bool_vector_bytes(lisp: &Lisp, v: Value) -> Vec<u8> {
    match lisp.heap.vectorlike_ref(v) {
        crate::object::Vectorlike::BoolVector(b) => {
            let mut bytes = vec![0u8; b.bits.len().div_ceil(8)];
            for (i, &bit) in b.bits.iter().enumerate() {
                if bit {
                    bytes[i / 8] |= 1 << (i % 8);
                }
            }
            bytes
        }
        _ => Vec::new(),
    }
}

fn char_hexdigit(c: u8) -> i32 {
    match c {
        b'0'..=b'9' => i32::from(c - b'0'),
        b'a'..=b'f' => i32::from(c - b'a' + 10),
        b'A'..=b'F' => i32::from(c - b'A' + 10),
        _ => -1,
    }
}

struct Scanner<'a> {
    bytes: &'a [u8],
    s: usize,
    sval: Vec<u8>,
    ival: i32,
}

impl Scanner<'_> {
    fn at_end(&self) -> bool {
        self.s >= self.bytes.len()
    }

    fn next(&mut self) -> u8 {
        let c = self.bytes[self.s];
        self.s += 1;
        c
    }

    fn peek(&self, offset: usize) -> u8 {
        self.bytes.get(self.s + offset).copied().unwrap_or(0)
    }

    fn scan(&mut self) -> i32 {
        loop {
            let mut c = 0u8;
            while !self.at_end() {
                c = self.next();
                if !(c.is_ascii_whitespace() || c == 0x0b) {
                    break;
                }
            }
            if self.at_end() {
                return 0;
            }
            if c.is_ascii_digit() {
                let mut value: i32 = 0;
                let mut overflow = false;
                let mut c = c;
                if c == b'0' && !self.at_end() {
                    c = self.next();
                    if c == b'x' || c == b'X' {
                        while !self.at_end() {
                            c = self.next();
                            let digit = char_hexdigit(c);
                            if digit < 0 {
                                break;
                            }
                            let (v, o) = value.overflowing_mul(16);
                            overflow |= o;
                            value = v.wrapping_add(digit);
                        }
                    } else if (b'0'..=b'7').contains(&c) {
                        value = i32::from(c - b'0');
                        while !self.at_end() {
                            c = self.next();
                            if !(b'0'..=b'7').contains(&c) {
                                break;
                            }
                            let (v, o) = value.overflowing_mul(8);
                            overflow |= o;
                            value = v.wrapping_add(i32::from(c - b'0'));
                        }
                    }
                } else {
                    value = i32::from(c - b'0');
                    while !self.at_end() {
                        c = self.next();
                        if !c.is_ascii_digit() {
                            break;
                        }
                        let (v, o) = value.overflowing_mul(10);
                        overflow |= o;
                        let (v, o) = v.overflowing_add(i32::from(c - b'0'));
                        overflow |= o;
                        value = v;
                    }
                }
                if !self.at_end() {
                    self.s -= 1;
                }
                self.ival = value;
                return if overflow {
                    XBM_TK_OVERFLOW
                } else {
                    XBM_TK_NUMBER
                };
            } else if c == b'\'' {
                let mut value: i32 = 0;
                let mut overflow = false;
                if self.at_end() {
                    return 0;
                }
                let c = self.next();
                if c != b'\\' || self.at_end() {
                    return 0;
                }
                let c = self.next();
                if c == b'x' {
                    while !self.at_end() {
                        let c = self.next();
                        if c == b'\'' {
                            self.ival = value;
                            return if overflow {
                                XBM_TK_OVERFLOW
                            } else {
                                XBM_TK_NUMBER
                            };
                        }
                        let digit = char_hexdigit(c);
                        if digit < 0 {
                            return 0;
                        }
                        let (v, o) = value.overflowing_mul(16);
                        overflow |= o;
                        value = v.wrapping_add(digit);
                    }
                }
                return 0;
            } else if c.is_ascii_alphabetic() || c == b'_' {
                self.sval.clear();
                self.sval.push(c);
                while !self.at_end() && self.sval.len() < BUFSIZ {
                    let c = self.next();
                    if !(c.is_ascii_alphanumeric() || c == b'_') {
                        break;
                    }
                    self.sval.push(c);
                }
                if !self.at_end() {
                    self.s -= 1;
                }
                return XBM_TK_IDENT;
            } else if c == b'/' && self.peek(0) == b'*' {
                self.s += 1;
                while self.peek(0) != 0 && (self.peek(0) != b'*' || self.peek(1) != b'/') {
                    self.s += 1;
                }
                if self.peek(0) != 0 {
                    self.s += 2;
                    continue;
                }
            }
            return i32::from(c);
        }
    }
}

pub(super) fn xbm_read_bitmap_data(
    lisp: &mut Lisp,
    f: Option<Value>,
    contents: &[u8],
    want_data: bool,
    inhibit_image_error: bool,
) -> LispResult<Option<(i32, i32, Vec<u8>)>> {
    let mut sc = Scanner {
        bytes: contents,
        s: 0,
        sval: Vec::new(),
        ival: 0,
    };
    let mut width = -1;
    let mut height = -1;
    let mut padding_p = false;
    let mut v10 = false;
    let mut la1 = sc.scan();
    macro_rules! matched {
        () => {
            la1 = sc.scan()
        };
    }
    macro_rules! expect {
        ($token:expr) => {
            if la1 != $token {
                return Ok(None);
            }
            matched!();
        };
    }
    macro_rules! expect_ident {
        ($ident:expr) => {
            if la1 == XBM_TK_IDENT && sc.sval == $ident.as_bytes() {
                matched!();
            } else {
                return Ok(None);
            }
        };
    }
    while la1 == i32::from(b'#') {
        matched!();
        expect_ident!("define");
        expect!(XBM_TK_IDENT);
        if la1 == XBM_TK_NUMBER {
            let q = match sc.sval.iter().rposition(|&b| b == b'_') {
                Some(i) => sc.sval[i + 1..].to_vec(),
                None => sc.sval.clone(),
            };
            if q == b"width" {
                width = sc.ival;
            } else if q == b"height" {
                height = sc.ival;
            }
        }
        expect!(XBM_TK_NUMBER);
    }
    if !lisp.check_image_size(f, width, height) {
        if !inhibit_image_error {
            lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        }
        return Ok(None);
    } else if !want_data {
        return Ok(Some((width, height, Vec::new())));
    }
    expect_ident!("static");
    if la1 == XBM_TK_IDENT {
        if sc.sval == b"unsigned" {
            matched!();
            expect_ident!("char");
        } else if sc.sval == b"short" {
            matched!();
            v10 = true;
            if width % 16 != 0 && width % 16 < 9 {
                padding_p = true;
            }
        } else if sc.sval == b"char" {
            matched!();
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    }
    expect!(XBM_TK_IDENT);
    expect!(i32::from(b'['));
    expect!(i32::from(b']'));
    expect!(i32::from(b'='));
    expect!(i32::from(b'{'));
    let bytes_per_line = (width + 7) / 8 + i32::from(padding_p);
    let nbytes = (bytes_per_line * height) as usize;
    let mut data = Vec::with_capacity(nbytes);
    if v10 {
        let mut i = 0;
        while i < nbytes {
            let val = sc.ival;
            expect!(XBM_TK_NUMBER);
            data.push(val as u8);
            if !padding_p || (i + 2) % bytes_per_line as usize != 0 {
                data.push((sc.ival >> 8) as u8);
            }
            if la1 == i32::from(b',') || la1 == i32::from(b'}') {
                matched!();
            } else {
                return Ok(None);
            }
            i += 2;
        }
    } else {
        for _ in 0..nbytes {
            let val = sc.ival;
            expect!(XBM_TK_NUMBER);
            data.push(val as u8);
            if la1 == i32::from(b',') || la1 == i32::from(b'}') {
                matched!();
            } else {
                return Ok(None);
            }
        }
    }
    data.resize(nbytes, 0);
    Ok(Some((width, height, data)))
}

fn create_pixmap_from_bitmap_data(img: &mut Image, data: &[u8], fg: u64, bg: u64) {
    let (r, g, b) = query_color(fg);
    let fg = lookup_rgb_color(r, g, b);
    let (r, g, b) = query_color(bg);
    let bg = lookup_rgb_color(r, g, b);
    let mut pimg = PixContainer::new(img.width, img.height, 0);
    let bytes_per_line = ((img.width + 7) / 8) as usize;
    for y in 0..img.height {
        let row = &data[y as usize * bytes_per_line..];
        for x in 0..img.width {
            let bit = (row[(x / 8) as usize] >> (x % 8)) & 1;
            pimg.put_pixel(x, y, if bit != 0 { fg } else { bg });
        }
    }
    img.pixmap = Some(pimg);
}

fn xbm_load_image(lisp: &mut Lisp, f: Value, img: &mut Image, contents: &[u8]) -> LispResult<bool> {
    let parsed = xbm_read_bitmap_data(lisp, Some(f), contents, true, false)?;
    let Some((width, height, data)) = parsed else {
        lisp.image_error("Error loading XBM image `%s'", &[img.spec])?;
        return Ok(false);
    };
    img.width = width;
    img.height = height;
    let mut foreground = img.face_foreground;
    let mut background = img.face_background;
    let (value, _) = lisp.image_spec_value(img.spec, sym::K_FOREGROUND);
    if !value.is_nil() {
        foreground = lisp.image_alloc_image_color(f, img, value, foreground)?;
    }
    let (value, _) = lisp.image_spec_value(img.spec, sym::K_BACKGROUND);
    if !value.is_nil() {
        background = lisp.image_alloc_image_color(f, img, value, background)?;
        img.background = background;
        img.background_valid = true;
    }
    create_pixmap_from_bitmap_data(img, &data, foreground, background);
    Ok(true)
}

fn xbm_file_p(lisp: &mut Lisp, data: Value) -> bool {
    if !data.is_string() {
        return false;
    }
    let bytes = lisp.string_bytes(data).to_vec();
    matches!(
        xbm_read_bitmap_data(lisp, None, &bytes, false, true),
        Ok(Some(_))
    )
}

pub(super) fn xbm_load(lisp: &mut Lisp, f: Value, img: &mut Image) -> LispResult<bool> {
    let (file_name, _) = lisp.image_spec_value(img.spec, sym::K_FILE);
    if file_name.is_string() {
        let Some(contents) = lisp.slurp_image(file_name, "XBM")? else {
            return Ok(false);
        };
        return xbm_load_image(lisp, f, img, &contents);
    }
    let mut foreground = img.face_foreground;
    let mut background = img.face_background;
    let (data, _) = lisp.image_spec_value(img.spec, sym::K_DATA);
    let in_memory_file_p = xbm_file_p(lisp, data);
    let mut fmt = xbm_format();
    lisp.parse_image_spec(img.spec, &mut fmt, sym::XBM);
    if !in_memory_file_p {
        img.width = fixnat(fmt[XBM_DATA_WIDTH].value) as i32;
        img.height = fixnat(fmt[XBM_DATA_HEIGHT].value) as i32;
        if !lisp.check_image_size(Some(f), img.width, img.height) {
            lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
            return Ok(false);
        }
    }
    if fmt[XBM_FOREGROUND].count && fmt[XBM_FOREGROUND].value.is_string() {
        let name = fmt[XBM_FOREGROUND].value;
        foreground = lisp.image_alloc_image_color(f, img, name, foreground)?;
    }
    if fmt[XBM_BACKGROUND].count && fmt[XBM_BACKGROUND].value.is_string() {
        let name = fmt[XBM_BACKGROUND].value;
        background = lisp.image_alloc_image_color(f, img, name, background)?;
    }
    if in_memory_file_p {
        let bytes = lisp.string_bytes(data).to_vec();
        return xbm_load_image(lisp, f, img, &bytes);
    }
    let nbytes = ((img.width + 7) / 8) as usize;
    let bits = if lisp.is_vector(data) {
        let mut bits = Vec::with_capacity(nbytes * img.height as usize);
        for i in 0..img.height as usize {
            let line = lisp.vector_items(data)[i];
            let mut row = if line.is_string() {
                lisp.string_bytes(line).to_vec()
            } else {
                bool_vector_bytes(lisp, line)
            };
            row.resize(nbytes, 0);
            bits.extend_from_slice(&row[..nbytes]);
        }
        bits
    } else if data.is_string() {
        lisp.string_bytes(data).to_vec()
    } else {
        bool_vector_bytes(lisp, data)
    };
    let mut bits = bits;
    bits.resize(nbytes * img.height.max(0) as usize, 0);
    create_pixmap_from_bitmap_data(img, &bits, foreground, background);
    Ok(true)
}
