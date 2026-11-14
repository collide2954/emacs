use std::io::Cursor;

use tiff::ColorType;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

use super::{Image, ImageKeyword, ImageValueType, PixContainer, lookup_rgb_color};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const TIFF_DATA: usize = 1;
const TIFF_FILE: usize = 2;
const EXTRASAMPLE_UNASSALPHA: u16 = 2;

fn tiff_format() -> [ImageKeyword; 11] {
    use ImageValueType as T;
    [
        ImageKeyword::new(":type", T::Symbol, true),
        ImageKeyword::new(":data", T::String, false),
        ImageKeyword::new(":file", T::String, false),
        ImageKeyword::new(":ascent", T::Ascent, false),
        ImageKeyword::new(":margin", T::NonNegativeIntegerOrPair, false),
        ImageKeyword::new(":relief", T::Integer, false),
        ImageKeyword::new(":conversions", T::DontCheck, false),
        ImageKeyword::new(":heuristic-mask", T::DontCheck, false),
        ImageKeyword::new(":mask", T::DontCheck, false),
        ImageKeyword::new(":background", T::StringOrNil, false),
        ImageKeyword::new(":index", T::NonNegativeInteger, false),
    ]
}

pub(super) fn tiff_image_p(lisp: &mut Lisp, object: Value) -> bool {
    let mut fmt = tiff_format();
    if !lisp.parse_image_spec(object, &mut fmt, sym::TIFF) {
        return false;
    }
    usize::from(fmt[TIFF_FILE].count) + usize::from(fmt[TIFF_DATA].count) == 1
}

fn tiff_error_handler(lisp: &mut Lisp, title: &str, message: &str) -> LispResult<()> {
    let title = lisp.string(title);
    let message = lisp.string(message);
    lisp.image_error("TIFF error: %s %s", &[title, message])
}

fn bitdepth_16_to_8(n: u16) -> u8 {
    ((u32::from(n) + 128) / 257) as u8
}

fn ua_to_aa(value: u8, alpha: u8) -> u8 {
    ((u32::from(value) * u32::from(alpha) + 127) / 255) as u8
}

fn ycbcr_to_rgb(y: u8, cb: u8, cr: u8) -> [u8; 3] {
    let (y, cb, cr) = (f64::from(y), f64::from(cb) - 128.0, f64::from(cr) - 128.0);
    let clamp = |v: f64| v.round().clamp(0.0, 255.0) as u8;
    [
        clamp(y + 1.402 * cr),
        clamp(y - 0.344_136 * cb - 0.714_136 * cr),
        clamp(y + 1.772 * cb),
    ]
}

fn unsupported(what: &str) -> String {
    format!("Sorry, can not handle {what}")
}

fn rgb_raster(
    decoder: &mut Decoder<Cursor<&[u8]>>,
    width: usize,
    height: usize,
) -> Result<Vec<[u8; 3]>, String> {
    let color = decoder.colortype().map_err(|e| e.to_string())?;
    let extra = decoder
        .find_tag(Tag::ExtraSamples)
        .map_err(|e| e.to_string())?
        .map(|v| v.into_u16_vec())
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let unassociated = extra.first() == Some(&EXTRASAMPLE_UNASSALPHA);
    let data = decoder.read_image().map_err(|e| e.to_string())?;
    let n = width * height;
    let mut out = Vec::with_capacity(n);
    match (color, data) {
        (ColorType::Gray(bits @ (1 | 2 | 4)), DecodingResult::U8(buf)) => {
            let range = (1u32 << bits) - 1;
            let row_bytes = (width * usize::from(bits)).div_ceil(8);
            for y in 0..height {
                for x in 0..width {
                    let bit = x * usize::from(bits);
                    let byte = buf[y * row_bytes + bit / 8];
                    let sample = (u32::from(byte) >> (8 - usize::from(bits) - bit % 8)) & range;
                    let v = (sample * 255 / range) as u8;
                    out.push([v, v, v]);
                }
            }
        }
        (ColorType::Gray(8), DecodingResult::U8(buf)) => {
            out.extend(buf.iter().take(n).map(|&v| [v, v, v]));
        }
        (ColorType::Gray(16), DecodingResult::U16(buf)) => {
            out.extend(buf.iter().take(n).map(|&v| {
                let v = (v >> 8) as u8;
                [v, v, v]
            }));
        }
        (
            ColorType::Multiband {
                bit_depth: 8,
                num_samples,
            },
            DecodingResult::U8(buf),
        ) => {
            let step = usize::from(num_samples);
            out.extend(buf.chunks(step).take(n).map(|p| [p[0], p[0], p[0]]));
        }
        (
            ColorType::Multiband {
                bit_depth: 16,
                num_samples,
            },
            DecodingResult::U16(buf),
        ) => {
            let step = usize::from(num_samples);
            out.extend(buf.chunks(step).take(n).map(|p| {
                let v = (p[0] >> 8) as u8;
                [v, v, v]
            }));
        }
        (ColorType::RGB(8), DecodingResult::U8(buf)) => {
            out.extend(buf.chunks(3).take(n).map(|p| [p[0], p[1], p[2]]));
        }
        (ColorType::RGB(16), DecodingResult::U16(buf)) => {
            out.extend(buf.chunks(3).take(n).map(|p| {
                [
                    bitdepth_16_to_8(p[0]),
                    bitdepth_16_to_8(p[1]),
                    bitdepth_16_to_8(p[2]),
                ]
            }));
        }
        (ColorType::RGBA(8), DecodingResult::U8(buf)) => {
            out.extend(buf.chunks(4).take(n).map(|p| {
                if unassociated {
                    [
                        ua_to_aa(p[0], p[3]),
                        ua_to_aa(p[1], p[3]),
                        ua_to_aa(p[2], p[3]),
                    ]
                } else {
                    [p[0], p[1], p[2]]
                }
            }));
        }
        (ColorType::RGBA(16), DecodingResult::U16(buf)) => {
            out.extend(buf.chunks(4).take(n).map(|p| {
                let rgb = [
                    bitdepth_16_to_8(p[0]),
                    bitdepth_16_to_8(p[1]),
                    bitdepth_16_to_8(p[2]),
                ];
                if unassociated {
                    let a = bitdepth_16_to_8(p[3]);
                    rgb.map(|v| ua_to_aa(v, a))
                } else {
                    rgb
                }
            }));
        }
        (ColorType::CMYK(8), DecodingResult::U8(buf)) => {
            out.extend(buf.chunks(4).take(n).map(cmyk_to_rgb));
        }
        (ColorType::CMYKA(8), DecodingResult::U8(buf)) => {
            out.extend(buf.chunks(5).take(n).map(cmyk_to_rgb));
        }
        (ColorType::YCbCr(8), DecodingResult::U8(buf)) => {
            out.extend(
                buf.chunks(3)
                    .take(n)
                    .map(|p| ycbcr_to_rgb(p[0], p[1], p[2])),
            );
        }
        (color, _) => return Err(unsupported(&format!("{color:?} images"))),
    }
    if out.len() < n {
        return Err("Read error on strip".to_string());
    }
    Ok(out)
}

fn cmyk_to_rgb(p: &[u8]) -> [u8; 3] {
    let k = 255 - u32::from(p[3]);
    let ink = |c: u8| ((k * (255 - u32::from(c))) / 255) as u8;
    [ink(p[0]), ink(p[1]), ink(p[2])]
}

fn stored_position(orientation: u16, x: usize, y: usize, w: usize, h: usize) -> (usize, usize) {
    match orientation {
        2 | 6 => (w - 1 - x, y),
        3 | 7 => (w - 1 - x, h - 1 - y),
        4 | 8 => (x, h - 1 - y),
        _ => (x, y),
    }
}

pub(super) fn tiff_load(lisp: &mut Lisp, f: Value, img: &mut Image) -> LispResult<bool> {
    let (specified_file, _) = lisp.image_spec_value(img.spec, sym::K_FILE);
    let (specified_data, _) = lisp.image_spec_value(img.spec, sym::K_DATA);
    let (contents, title) = if specified_data.is_nil() {
        let file = lisp.image_find_image_file(specified_file)?;
        if !file.is_string() {
            lisp.image_error("Cannot find image file `%s'", &[specified_file])?;
            return Ok(false);
        }
        let path = lisp.string_to_rust(file);
        match std::fs::read(&path) {
            Ok(contents) => (contents, path),
            Err(e) => {
                tiff_error_handler(lisp, &path, &e.to_string())?;
                lisp.image_error("Cannot open `%s'", &[file])?;
                return Ok(false);
            }
        }
    } else {
        if !specified_data.is_string() {
            lisp.image_error("Invalid image data `%s'", &[specified_data])?;
            return Ok(false);
        }
        (
            lisp.string_bytes(specified_data).to_vec(),
            "memory_source".to_string(),
        )
    };
    let mut decoder = match Decoder::new(Cursor::new(&contents[..])) {
        Ok(decoder) => decoder,
        Err(e) => {
            tiff_error_handler(lisp, &title, &e.to_string())?;
            if specified_data.is_nil() {
                let file = lisp.image_find_image_file(specified_file)?;
                lisp.image_error("Cannot open `%s'", &[file])?;
            } else {
                lisp.image_error("Cannot open memory source for `%s'", &[img.spec])?;
            }
            return Ok(false);
        }
    };
    let (image, _) = lisp.image_spec_value(img.spec, sym::K_INDEX);
    if let Some(ino) = image.as_fixnum()
        && !(u32::try_from(ino).is_ok() && decoder.seek_to_image(ino as usize).is_ok())
    {
        lisp.image_error(
            "Invalid image number `%s' in image `%s'",
            &[image, img.spec],
        )?;
        return Ok(false);
    }
    let (width, height) = decoder.dimensions().unwrap_or((0, 0));
    let (width, height) = (width as i32, height as i32);
    if !lisp.check_image_size(Some(f), width, height) {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        return Ok(false);
    }
    let mut ximg = PixContainer::new(width, height, 0);
    let (w, h) = (width as usize, height as usize);
    let orientation = decoder
        .find_tag_unsigned::<u16>(Tag::Orientation)
        .ok()
        .flatten()
        .unwrap_or(1);
    let raster = rgb_raster(&mut decoder, w, h);
    let mut count = 1;
    while decoder.seek_to_image(count).is_ok() {
        count += 1;
    }
    if count > 1 {
        let rest = lisp.cons(Value::fixnum(count as i64), img.lisp_data);
        img.lisp_data = lisp.cons(sym::COUNT, rest);
    }
    let raster = match raster {
        Ok(raster) => raster,
        Err(message) => {
            tiff_error_handler(lisp, &title, &message)?;
            lisp.image_error("Error reading TIFF image `%s'", &[img.spec])?;
            return Ok(false);
        }
    };
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = stored_position(orientation, x, y, w, h);
            let [r, g, b] = raster[sy * w + sx];
            ximg.put_pixel(
                x as i32,
                y as i32,
                lookup_rgb_color(i32::from(r) << 8, i32::from(g) << 8, i32::from(b) << 8),
            );
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
