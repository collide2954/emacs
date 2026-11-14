use std::collections::HashMap;

use super::{
    CLEAR_IMAGE_MASK, Image, ImageKeyword, ImageValueType, PIX_MASK_DRAW, PIX_MASK_RETAIN,
    PixContainer, image_clear_image, image_clear_image_1, lookup_rgb_color, query_color,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const XPM_FILE: usize = 1;
const XPM_DATA: usize = 2;
const XPM_COLOR_SYMBOLS: usize = 9;
const BUFSIZ: usize = 8192;
const XPM_TK_IDENT: i32 = 256;
const XPM_TK_STRING: i32 = 257;
const XPM_TK_EOF: i32 = 258;
const XPM_COLOR_KEY_S: i32 = 0;
const XPM_COLOR_KEY_M: i32 = 1;
const XPM_COLOR_KEY_G4: i32 = 2;
const XPM_COLOR_KEY_G: i32 = 3;
const XPM_COLOR_KEY_C: i32 = 4;
const XPM_COLOR_KEY_STRINGS: [&[u8]; 5] = [b"s", b"m", b"g4", b"g", b"c"];

fn xpm_format() -> [ImageKeyword; 11] {
    use ImageValueType as T;
    [
        ImageKeyword::new(":type", T::Symbol, true),
        ImageKeyword::new(":file", T::String, false),
        ImageKeyword::new(":data", T::String, false),
        ImageKeyword::new(":ascent", T::Ascent, false),
        ImageKeyword::new(":margin", T::NonNegativeIntegerOrPair, false),
        ImageKeyword::new(":relief", T::Integer, false),
        ImageKeyword::new(":conversion", T::DontCheck, false),
        ImageKeyword::new(":heuristic-mask", T::DontCheck, false),
        ImageKeyword::new(":mask", T::DontCheck, false),
        ImageKeyword::new(":color-symbols", T::DontCheck, false),
        ImageKeyword::new(":background", T::StringOrNil, false),
    ]
}

fn xpm_valid_color_symbols_p(lisp: &Lisp, color_symbols: Value) -> bool {
    let mut color_symbols = color_symbols;
    while color_symbols.is_cons() {
        let entry = lisp.xcar(color_symbols);
        if !entry.is_cons() || !lisp.xcar(entry).is_string() || !lisp.xcdr(entry).is_string() {
            break;
        }
        color_symbols = lisp.xcdr(color_symbols);
    }
    color_symbols.is_nil()
}

pub(super) fn xpm_image_p(lisp: &mut Lisp, object: Value) -> bool {
    let mut fmt = xpm_format();
    lisp.parse_image_spec(object, &mut fmt, sym::XPM)
        && usize::from(fmt[XPM_FILE].count) + usize::from(fmt[XPM_DATA].count) == 1
        && (!fmt[XPM_COLOR_SYMBOLS].count
            || xpm_valid_color_symbols_p(lisp, fmt[XPM_COLOR_SYMBOLS].value))
}

fn c_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

struct Scanner<'a> {
    data: &'a [u8],
    s: usize,
    beg: usize,
    len: usize,
}

impl Scanner<'_> {
    fn scan(&mut self) -> i32 {
        let end = self.data.len();
        while self.s < end {
            let mut c;
            loop {
                c = self.data[self.s];
                self.s += 1;
                if !(c_isspace(c) && self.s < end) {
                    break;
                }
            }
            if c.is_ascii_alphabetic() || c == b'_' || c == b'-' || c == b'+' {
                self.beg = self.s - 1;
                while self.s < end {
                    let c = self.data[self.s];
                    if !(c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b'+') {
                        break;
                    }
                    self.s += 1;
                }
                self.len = self.s - self.beg;
                return XPM_TK_IDENT;
            } else if c == b'"' {
                self.beg = self.s;
                while self.s < end && self.data[self.s] != b'"' {
                    self.s += 1;
                }
                self.len = self.s - self.beg;
                if self.s < end {
                    self.s += 1;
                }
                return XPM_TK_STRING;
            } else if c == b'/' {
                if self.s < end && self.data[self.s] == b'*' {
                    self.s += 1;
                    loop {
                        while self.s < end {
                            let c = self.data[self.s];
                            self.s += 1;
                            if c == b'*' {
                                break;
                            }
                        }
                        if !(self.s < end && self.data[self.s] != b'/') {
                            break;
                        }
                    }
                    if self.s < end {
                        self.s += 1;
                    }
                } else {
                    return i32::from(c);
                }
            } else {
                return i32::from(c);
            }
        }
        XPM_TK_EOF
    }

    fn token(&self) -> &[u8] {
        &self.data[self.beg..self.beg + self.len]
    }
}

fn xpm_str_to_color_key(s: &[u8]) -> i32 {
    XPM_COLOR_KEY_STRINGS
        .iter()
        .position(|k| *k == s)
        .map_or(-1, |i| i as i32)
}

fn xpm_str_to_int(buf: &[u8], at: &mut usize) -> i32 {
    let mut p = *at;
    while p < buf.len() && c_isspace(buf[p]) {
        p += 1;
    }
    let start = p;
    if p < buf.len() && (buf[p] == b'+' || buf[p] == b'-') {
        p += 1;
    }
    let digits = p;
    while p < buf.len() && buf[p].is_ascii_digit() {
        p += 1;
    }
    if p == digits {
        return -1;
    }
    let Ok(result) = std::str::from_utf8(&buf[start..p])
        .unwrap_or("")
        .parse::<i64>()
    else {
        return -1;
    };
    if result < i64::from(i32::MIN) || result > i64::from(i32::MAX) {
        return -1;
    }
    if p < buf.len() && !c_isspace(buf[p]) && buf[p] != 0 {
        return -1;
    }
    *at = p;
    result as i32
}

struct Strtok {
    buf: Vec<u8>,
    pos: usize,
}

impl Strtok {
    fn is_delim(c: u8) -> bool {
        c == b' ' || c == b'\t'
    }

    fn next(&mut self) -> Option<(usize, usize)> {
        while self.pos < self.buf.len() && Self::is_delim(self.buf[self.pos]) {
            self.pos += 1;
        }
        if self.pos >= self.buf.len() || self.buf[self.pos] == 0 {
            return None;
        }
        let start = self.pos;
        while self.pos < self.buf.len()
            && self.buf[self.pos] != 0
            && !Self::is_delim(self.buf[self.pos])
        {
            self.pos += 1;
        }
        let end = self.pos;
        if self.pos < self.buf.len() && self.buf[self.pos] != 0 {
            self.buf[self.pos] = 0;
            self.pos += 1;
        }
        Some((start, end))
    }

    fn c_str(&self, start: usize) -> &[u8] {
        let end = self.buf[start..]
            .iter()
            .position(|&c| c == 0)
            .map_or(self.buf.len(), |p| start + p);
        &self.buf[start..end]
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ColorVal {
    Unspecified,
    Transparent,
    Pixel(u64),
}

fn xpm_failure(lisp: &mut Lisp, f: Value, img: &mut Image, created: bool) -> LispResult<bool> {
    lisp.image_error("Invalid XPM3 file (%s)", &[img.spec])?;
    if created {
        image_clear_image(lisp, f, img);
    }
    Ok(false)
}

fn call_named(lisp: &mut Lisp, name: &str, frame: Value) -> LispResult {
    let function = lisp.intern(name);
    lisp.funcall(function, &[frame])
}

fn color_pixel(lisp: &mut Lisp, f: Value, name: &[u8]) -> LispResult<Option<u64>> {
    let (defined, color) = lisp.defined_color(f, name)?;
    Ok(defined.then(|| {
        lookup_rgb_color(
            i32::from(color.red),
            i32::from(color.green),
            i32::from(color.blue),
        )
    }))
}

fn xpm_load_image(lisp: &mut Lisp, f: Value, img: &mut Image, contents: &[u8]) -> LispResult<bool> {
    let mut sc = Scanner {
        data: contents,
        s: 0,
        beg: 0,
        len: 0,
    };
    macro_rules! fail {
        ($created:expr) => {
            return xpm_failure(lisp, f, img, $created)
        };
    }
    if !(contents.len() >= 9 && &contents[..9] == b"/* XPM */") {
        fail!(false);
    }
    sc.s = 9;
    let mut la1 = sc.scan();
    macro_rules! expect {
        ($token:expr, $created:expr) => {
            if la1 != $token {
                fail!($created);
            }
            la1 = sc.scan();
        };
    }
    macro_rules! expect_ident {
        ($ident:expr) => {
            if la1 == XPM_TK_IDENT && sc.token() == $ident {
                la1 = sc.scan();
            } else {
                fail!(false);
            }
        };
    }
    expect_ident!(b"static");
    expect_ident!(b"char");
    expect!(i32::from(b'*'), false);
    expect!(XPM_TK_IDENT, false);
    expect!(i32::from(b'['), false);
    expect!(i32::from(b']'), false);
    expect!(i32::from(b'='), false);
    expect!(i32::from(b'{'), false);
    expect!(XPM_TK_STRING, false);
    if sc.len >= BUFSIZ {
        fail!(false);
    }
    let header = sc.token().to_vec();
    let mut next_int = 0;
    let width = xpm_str_to_int(&header, &mut next_int);
    if width <= 0 {
        fail!(false);
    }
    let height = xpm_str_to_int(&header, &mut next_int);
    if height <= 0 {
        fail!(false);
    }
    let mut num_colors = xpm_str_to_int(&header, &mut next_int);
    if num_colors <= 0 {
        fail!(false);
    }
    let chars_per_pixel = xpm_str_to_int(&header, &mut next_int);
    if chars_per_pixel <= 0 {
        fail!(false);
    }
    if !lisp.check_image_size(Some(f), width, height) {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        fail!(false);
    }
    let mut ximg = PixContainer::new(width, height, 0);
    let mut mask_img = PixContainer::new(width, height, 1);
    let mut have_mask = false;
    expect!(i32::from(b','), true);
    let frame = f;
    let color_p = call_named(lisp, "xw-display-color-p", frame)?;
    let best_key = if !color_p.is_nil() {
        XPM_COLOR_KEY_C
    } else if !call_named(lisp, "x-display-grayscale-p", frame)?.is_nil() {
        let planes = call_named(lisp, "x-display-planes", frame)?;
        if planes.as_fixnum().unwrap_or(0) > 2 {
            XPM_COLOR_KEY_G
        } else {
            XPM_COLOR_KEY_G4
        }
    } else {
        XPM_COLOR_KEY_M
    };
    let (color_symbols, _) = lisp.image_spec_value(img.spec, sym::K_COLOR_SYMBOLS);
    let cpp = chars_per_pixel as usize;
    let mut table_v = [ColorVal::Unspecified; 256];
    let mut table_h: HashMap<Vec<u8>, ColorVal> = HashMap::new();
    while num_colors > 0 {
        num_colors -= 1;
        expect!(XPM_TK_STRING, true);
        if sc.len <= cpp || sc.len >= BUFSIZ + cpp {
            fail!(true);
        }
        let chars = contents[sc.beg..sc.beg + cpp].to_vec();
        let mut tok = Strtok {
            buf: contents[sc.beg + cpp..sc.beg + sc.len].to_vec(),
            pos: 0,
        };
        let Some((first, end)) = tok.next() else {
            fail!(true);
        };
        let mut key = xpm_str_to_color_key(&tok.buf[first..end]);
        if key < 0 {
            fail!(true);
        }
        let mut symbol_color = None;
        let mut max_color: Option<usize> = None;
        let mut max_key = 0;
        loop {
            let Some((color, _)) = tok.next() else {
                fail!(true);
            };
            let mut next_key = -1;
            let mut more = false;
            while let Some((s, e)) = tok.next() {
                next_key = xpm_str_to_color_key(&tok.buf[s..e]);
                if next_key >= 0 {
                    more = true;
                    break;
                }
                let nul = tok.buf[color..]
                    .iter()
                    .position(|&c| c == 0)
                    .map_or(tok.buf.len(), |p| color + p);
                if nul < tok.buf.len() {
                    tok.buf[nul] = b' ';
                }
            }
            if key == XPM_COLOR_KEY_S {
                if symbol_color.is_none() {
                    symbol_color = Some(tok.c_str(color).to_vec());
                }
            } else if max_key < key && key <= best_key {
                max_key = key;
                max_color = Some(color);
            }
            key = next_key;
            if !more {
                break;
            }
        }
        let mut color_val = ColorVal::Unspecified;
        if !color_symbols.is_nil()
            && let Some(name) = symbol_color
        {
            let name = lisp.unibyte_string(&name);
            let specified = crate::fns::f_assoc(lisp, &[name, color_symbols, Value::NIL])?;
            if specified.is_cons() && lisp.xcdr(specified).is_string() {
                let value = lisp.string_bytes(lisp.xcdr(specified)).to_vec();
                if value.eq_ignore_ascii_case(b"None") {
                    color_val = ColorVal::Transparent;
                } else if let Some(pixel) = color_pixel(lisp, f, &value)? {
                    color_val = ColorVal::Pixel(pixel);
                }
            }
        }
        if color_val == ColorVal::Unspecified
            && let Some(start) = max_color
        {
            let name = tok.c_str(start).to_vec();
            if name.eq_ignore_ascii_case(b"None") {
                color_val = ColorVal::Transparent;
            } else if let Some(pixel) = color_pixel(lisp, f, &name)? {
                color_val = ColorVal::Pixel(pixel);
            }
        }
        if color_val != ColorVal::Unspecified {
            if cpp == 1 {
                table_v[usize::from(chars[0])] = color_val;
            } else {
                table_h.insert(chars, color_val);
            }
        }
        expect!(i32::from(b','), true);
    }
    let (r, g, b) = query_color(lisp.frame(f).foreground_pixel as u64);
    let frame_fg = lookup_rgb_color(r, g, b);
    for y in 0..height {
        expect!(XPM_TK_STRING, true);
        if sc.len < width as usize * cpp {
            fail!(true);
        }
        let row = sc.beg;
        for x in 0..width {
            let at = row + x as usize * cpp;
            let key = &contents[at..at + cpp];
            let color_val = if cpp == 1 {
                table_v[usize::from(key[0])]
            } else {
                table_h.get(key).copied().unwrap_or(ColorVal::Unspecified)
            };
            ximg.put_pixel(
                x,
                y,
                match color_val {
                    ColorVal::Pixel(pixel) => pixel,
                    _ => frame_fg,
                },
            );
            let mask = if color_val == ColorVal::Transparent {
                have_mask = true;
                PIX_MASK_RETAIN
            } else {
                PIX_MASK_DRAW
            };
            mask_img.put_pixel(x, y, mask);
        }
        if y + 1 < height {
            expect!(i32::from(b','), true);
        }
    }
    img.width = width;
    img.height = height;
    img.pixmap = Some(ximg);
    if lisp
        .image_spec_value(img.spec, sym::K_BACKGROUND)
        .0
        .is_nil()
        && !img.background_valid
    {
        lisp.image_background(f, img)?;
    }
    if have_mask {
        img.mask = Some(mask_img);
        Lisp::image_background_transparent(img);
    } else {
        image_clear_image_1(img, CLEAR_IMAGE_MASK);
    }
    Ok(true)
}

pub(super) fn xpm_load(lisp: &mut Lisp, f: Value, img: &mut Image) -> LispResult<bool> {
    let (file_name, _) = lisp.image_spec_value(img.spec, sym::K_FILE);
    if file_name.is_string() {
        let Some(contents) = lisp.slurp_image(file_name, "XPM")? else {
            return Ok(false);
        };
        xpm_load_image(lisp, f, img, &contents)
    } else {
        let (data, _) = lisp.image_spec_value(img.spec, sym::K_DATA);
        if !data.is_string() {
            lisp.image_error("Invalid image data `%s'", &[data])?;
            return Ok(false);
        }
        let contents = lisp.string_bytes(data).to_vec();
        xpm_load_image(lisp, f, img, &contents)
    }
}
