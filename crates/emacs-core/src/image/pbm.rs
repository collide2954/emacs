use super::{Image, ImageKeyword, ImageValueType, PixContainer, lookup_rgb_color, query_color};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const PBM_FILE: usize = 1;
const PBM_DATA: usize = 2;
const PBM_FOREGROUND: usize = 9;
const PBM_BACKGROUND: usize = 10;

fn pbm_format() -> [ImageKeyword; 11] {
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
        ImageKeyword::new(":foreground", T::StringOrNil, false),
        ImageKeyword::new(":background", T::StringOrNil, false),
    ]
}

pub(super) fn pbm_image_p(lisp: &mut Lisp, object: Value) -> bool {
    let mut fmt = pbm_format();
    if !lisp.parse_image_spec(object, &mut fmt, sym::PBM) {
        return false;
    }
    usize::from(fmt[PBM_DATA].count) + usize::from(fmt[PBM_FILE].count) == 1
}

fn c_isspace(c: i32) -> bool {
    matches!(c, 0x20 | 0x09 | 0x0a | 0x0b | 0x0c | 0x0d)
}

fn c_isdigit(c: i32) -> bool {
    (i32::from(b'0')..=i32::from(b'9')).contains(&c)
}

struct Reader<'a> {
    bytes: &'a [u8],
    p: usize,
}

impl Reader<'_> {
    fn next_char(&mut self) -> i32 {
        while self.p < self.bytes.len() {
            let c = self.bytes[self.p];
            self.p += 1;
            if c != b'#' {
                return i32::from(c);
            }
            while self.p < self.bytes.len() {
                let c = self.bytes[self.p];
                self.p += 1;
                if c == b'\n' || c == b'\r' {
                    break;
                }
            }
        }
        -1
    }

    fn scan_number(&mut self) -> i32 {
        let mut c;
        loop {
            c = self.next_char();
            if c == -1 || !c_isspace(c) {
                break;
            }
        }
        let mut val = -1;
        if c_isdigit(c) {
            val = c - i32::from(b'0');
            loop {
                c = self.next_char();
                if c == -1 || !c_isdigit(c) {
                    break;
                }
                val = val.wrapping_mul(10).wrapping_add(c - i32::from(b'0'));
            }
        }
        val
    }

    fn scan_index(&mut self, two_byte: bool) -> i32 {
        let c0 = i32::from(self.bytes.get(self.p).copied().unwrap_or(0));
        self.p += 1;
        let mut n = c0;
        if two_byte {
            let c1 = i32::from(self.bytes.get(self.p).copied().unwrap_or(0));
            self.p += 1;
            n = (n << 8) + c1;
        }
        n
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PbmType {
    Mono,
    Gray,
    Color,
}

pub(super) fn pbm_load(lisp: &mut Lisp, f: Value, img: &mut Image) -> LispResult<bool> {
    let (specified_file, _) = lisp.image_spec_value(img.spec, sym::K_FILE);
    let contents = if specified_file.is_string() {
        match lisp.slurp_image(specified_file, "PBM")? {
            Some(contents) => contents,
            None => return Ok(false),
        }
    } else {
        let (data, _) = lisp.image_spec_value(img.spec, sym::K_DATA);
        if !data.is_string() {
            lisp.image_error("Invalid image data `%s'", &[data])?;
            return Ok(false);
        }
        lisp.string_bytes(data).to_vec()
    };
    let mut r = Reader {
        bytes: &contents,
        p: 0,
    };
    let fail = |lisp: &mut Lisp, img: &mut Image| -> LispResult<bool> {
        img.pixmap = None;
        let _ = lisp;
        Ok(false)
    };
    if contents.len() < 2 || contents[0] != b'P' {
        lisp.image_error("Not a PBM image: `%s'", &[img.spec])?;
        return fail(lisp, img);
    }
    r.p = 2;
    let (raw_p, kind) = match contents[1] {
        b'1' => (false, PbmType::Mono),
        b'2' => (false, PbmType::Gray),
        b'3' => (false, PbmType::Color),
        b'4' => (true, PbmType::Mono),
        b'5' => (true, PbmType::Gray),
        b'6' => (true, PbmType::Color),
        _ => {
            lisp.image_error("Not a PBM image: `%s'", &[img.spec])?;
            return fail(lisp, img);
        }
    };
    let width = r.scan_number();
    let height = r.scan_number();
    let mut max_color_idx = 0;
    if kind != PbmType::Mono {
        max_color_idx = r.scan_number();
        if !(0..=65535).contains(&max_color_idx) {
            lisp.image_error("Unsupported maximum PBM color value", &[])?;
            return fail(lisp, img);
        }
    }
    if !lisp.check_image_size(Some(f), width, height) {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        return fail(lisp, img);
    }
    let mut ximg = PixContainer::new(width, height, 0);
    let end = contents.len();
    if kind == PbmType::Mono {
        let mut fmt = pbm_format();
        lisp.parse_image_spec(img.spec, &mut fmt, sym::PBM);
        let mut fg = img.face_foreground;
        let mut bg = img.face_background;
        if fmt[PBM_FOREGROUND].count && fmt[PBM_FOREGROUND].value.is_string() {
            let name = fmt[PBM_FOREGROUND].value;
            fg = lisp.image_alloc_image_color(f, img, name, fg)?;
        }
        if fmt[PBM_BACKGROUND].count && fmt[PBM_BACKGROUND].value.is_string() {
            let name = fmt[PBM_BACKGROUND].value;
            bg = lisp.image_alloc_image_color(f, img, name, bg)?;
            img.background = bg;
            img.background_valid = true;
        }
        let (rr, gg, bb) = query_color(fg);
        fg = lookup_rgb_color(rr, gg, bb);
        let (rr, gg, bb) = query_color(bg);
        bg = lookup_rgb_color(rr, gg, bb);
        let mut c: u8 = 0;
        for y in 0..height {
            for x in 0..width {
                let g = if raw_p {
                    if x & 7 == 0 {
                        if r.p >= end {
                            super::image_clear_image(lisp, f, img);
                            lisp.image_error("Invalid image size in image `%s'", &[img.spec])?;
                            return fail(lisp, img);
                        }
                        c = contents[r.p];
                        r.p += 1;
                    }
                    let g = c & 0x80;
                    c <<= 1;
                    i32::from(g)
                } else {
                    let mut c;
                    loop {
                        c = r.next_char();
                        if c == -1 || !c_isspace(c) {
                            break;
                        }
                    }
                    if c == i32::from(b'0') || c == i32::from(b'1') {
                        c - i32::from(b'0')
                    } else {
                        0
                    }
                };
                ximg.put_pixel(x, y, if g != 0 { fg } else { bg });
            }
        }
    } else {
        let two_byte = 255 < max_color_idx;
        if raw_p {
            let per = (if two_byte { 2 } else { 1 }) * (if kind == PbmType::Color { 3 } else { 1 });
            let expected = i64::from(height)
                .checked_mul(i64::from(width))
                .and_then(|n| n.checked_mul(per));
            if expected.is_none_or(|n| ((end - r.p) as i64) < n) {
                super::image_clear_image(lisp, f, img);
                lisp.image_error("Invalid image size in image `%s'", &[img.spec])?;
                return fail(lisp, img);
            }
        }
        for y in 0..height {
            for x in 0..width {
                let (rr, gg, bb) = if kind == PbmType::Gray && raw_p {
                    let v = r.scan_index(two_byte);
                    (v, v, v)
                } else if kind == PbmType::Gray {
                    let v = r.scan_number();
                    (v, v, v)
                } else if raw_p {
                    let rr = r.scan_index(two_byte);
                    let gg = r.scan_index(two_byte);
                    let bb = r.scan_index(two_byte);
                    (rr, gg, bb)
                } else {
                    let rr = r.scan_number();
                    let gg = r.scan_number();
                    let bb = r.scan_number();
                    (rr, gg, bb)
                };
                if rr < 0 || gg < 0 || bb < 0 {
                    lisp.image_error("Invalid pixel value in image `%s'", &[img.spec])?;
                    return fail(lisp, img);
                }
                let scale = |v: i32| (f64::from(v) * 65535.0 / f64::from(max_color_idx)) as i32;
                ximg.put_pixel(x, y, lookup_rgb_color(scale(rr), scale(gg), scale(bb)));
            }
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
    Ok(true)
}
