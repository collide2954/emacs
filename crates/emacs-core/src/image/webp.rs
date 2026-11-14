use std::io::Cursor;

use image_webp::WebPDecoder;

use super::{BOT_CORNER, LEFT_CORNER, RIGHT_CORNER, TOP_CORNER};
use super::{Image, ImageKeyword, ImageValueType, PixContainer, lookup_rgb_color, query_color};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const WEBP_DATA: usize = 1;
const WEBP_FILE: usize = 2;

fn webp_format() -> [ImageKeyword; 11] {
    use ImageValueType as T;
    [
        ImageKeyword::new(":type", T::Symbol, true),
        ImageKeyword::new(":data", T::String, false),
        ImageKeyword::new(":file", T::String, false),
        ImageKeyword::new(":ascent", T::Ascent, false),
        ImageKeyword::new(":margin", T::NonNegativeIntegerOrPair, false),
        ImageKeyword::new(":relief", T::Integer, false),
        ImageKeyword::new(":conversion", T::DontCheck, false),
        ImageKeyword::new(":heuristic-mask", T::DontCheck, false),
        ImageKeyword::new(":mask", T::DontCheck, false),
        ImageKeyword::new(":index", T::NonNegativeInteger, false),
        ImageKeyword::new(":background", T::StringOrNil, false),
    ]
}

pub(super) fn webp_image_p(lisp: &mut Lisp, object: Value) -> bool {
    let mut fmt = webp_format();
    if !lisp.parse_image_spec(object, &mut fmt, sym::WEBP) {
        return false;
    }
    usize::from(fmt[WEBP_FILE].count) + usize::from(fmt[WEBP_DATA].count) == 1
}

#[derive(Default)]
pub(crate) struct WebpAnimHandle {
    dec: Option<Box<WebPDecoder<Cursor<Vec<u8>>>>>,
}

fn webp_error(
    lisp: &mut Lisp,
    specified_data: Value,
    from_file: &str,
    from_data: &str,
    args: &[Value],
) -> LispResult<bool> {
    let format = if specified_data.is_nil() {
        from_file
    } else {
        from_data
    };
    lisp.image_error(format, args)?;
    Ok(false)
}

pub(super) fn webp_load(lisp: &mut Lisp, f: Value, img: &mut Image) -> LispResult<bool> {
    let (image_number, _) = lisp.image_spec_value(img.spec, sym::K_INDEX);
    let cache = (!image_number.is_nil()).then(|| {
        let plist = lisp.xcdr(img.spec);
        lisp.anim_get_animation_cache(plist)
    });
    let mut specified_data = Value::NIL;
    let mut specified_file = Value::NIL;
    let mut contents = Vec::new();
    let (mut has_alpha, mut has_animation) = (false, false);
    let mut still = None;
    let cached = cache.is_some_and(|c| lisp.images.anim[c].webp.dec.is_some());
    if !cached {
        specified_data = lisp.image_spec_value(img.spec, sym::K_DATA).0;
        if specified_data.is_nil() {
            specified_file = lisp.image_spec_value(img.spec, sym::K_FILE).0;
            match lisp.slurp_image(specified_file, "WebP")? {
                Some(bytes) => contents = bytes,
                None => return Ok(false),
            }
        } else if specified_data.is_string() {
            contents = lisp.string_bytes(specified_data).to_vec();
        } else {
            lisp.image_error("Invalid image data `%s'", &[specified_data])?;
            return Ok(false);
        }
        match WebPDecoder::new(Cursor::new(contents.clone())) {
            Ok(dec) => {
                has_alpha = dec.has_alpha();
                has_animation = dec.is_animated();
                still = Some(dec);
            }
            Err(_) => {
                return webp_error(
                    lisp,
                    specified_data,
                    "Error parsing WebP headers from file: `%s'",
                    "Error parsing WebP headers from image data",
                    &[specified_file],
                );
            }
        }
    }
    let size = contents.len();
    let (width, height);
    let mut frames = -1;
    let mut duration = -1;
    let mut decoded: Option<(Vec<u8>, usize)> = None;
    if cached || has_animation {
        let c = match cache {
            Some(c) => c,
            None => {
                let plist = lisp.xcdr(img.spec);
                lisp.anim_get_animation_cache(plist)
            }
        };
        let mut dec = match lisp.images.anim[c].webp.dec.take() {
            Some(dec) => {
                has_alpha = dec.has_alpha();
                has_animation = dec.is_animated();
                dec
            }
            None => {
                let Ok(dec) = WebPDecoder::new(Cursor::new(contents)) else {
                    return webp_error(
                        lisp,
                        specified_data,
                        "Error parsing WebP file: `%s'",
                        "Error parsing WebP image data",
                        &[specified_file],
                    );
                };
                let (w, h) = dec.dimensions();
                let entry = &mut lisp.images.anim[c];
                entry.width = w as i32;
                entry.height = h as i32;
                entry.frames = dec.num_frames() as i32;
                entry.byte_size = size as i64;
                Box::new(dec)
            }
        };
        let entry = &lisp.images.anim[c];
        width = entry.width;
        height = entry.height;
        frames = entry.frames;
        let idx = image_number
            .as_fixnum()
            .map_or(0, |n| n.min(i64::from(frames)));
        if i64::from(lisp.images.anim[c].index) >= idx {
            dec.reset_animation();
            lisp.images.anim[c].index = -1;
        }
        let samples = if has_alpha { 4 } else { 3 };
        let mut buf = vec![0u8; width.max(0) as usize * height.max(0) as usize * samples];
        while i64::from(lisp.images.anim[c].index) < idx && lisp.images.anim[c].index + 1 < frames {
            match dec.read_frame(&mut buf) {
                Ok(ms) => duration = ms as i32,
                Err(_) => {
                    let number = Value::fixnum(i64::from(lisp.images.anim[c].index) + 1);
                    lisp.images.anim[c].webp.dec = Some(dec);
                    return webp_error(
                        lisp,
                        specified_data,
                        "Error decoding frame #%d from WebP file: `%s'",
                        "Error decoding frame #%d from WebP image data",
                        &[number, specified_file],
                    );
                }
            }
            lisp.images.anim[c].index += 1;
            decoded = Some((buf.clone(), samples));
        }
        lisp.images.anim[c].webp.dec = Some(dec);
    } else {
        let mut dec = still.expect("parsed WebP decoder");
        let (w, h) = dec.dimensions();
        width = w as i32;
        height = h as i32;
        if let Some(len) = dec.output_buffer_size() {
            let mut buf = vec![0u8; len];
            if dec.read_image(&mut buf).is_ok() {
                decoded = Some((buf, if has_alpha { 4 } else { 3 }));
            }
        }
    }
    let Some((pixels, samples)) = decoded else {
        return webp_error(
            lisp,
            specified_data,
            "Error decoding WebP file: `%s'",
            "Error decoding WebP image data",
            &[specified_file],
        );
    };
    if !lisp.check_image_size(Some(f), width, height) {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        return Ok(false);
    }
    let mut ximg = PixContainer::new(width, height, 0);
    let mut bg = (0u16, 0u16, 0u16);
    if has_alpha {
        let (specified_bg, _) = lisp.image_spec_value(img.spec, sym::K_BACKGROUND);
        if specified_bg.is_string() {
            let name = lisp.string_bytes(specified_bg).to_vec();
            let (defined, color) = lisp.defined_color(f, &name)?;
            if defined {
                bg = (color.red, color.green, color.blue);
            }
        } else {
            let (r, g, b) = query_color(lisp.frame(f).background_pixel as u64);
            bg = (r as u16, g as u16, b as u16);
        }
        bg = (bg.0 >> 8, bg.1 >> 8, bg.2 >> 8);
    }
    img.corners[TOP_CORNER] = 0;
    img.corners[LEFT_CORNER] = 0;
    img.corners[BOT_CORNER] = height;
    img.corners[RIGHT_CORNER] = width;
    let mut p = 0;
    for y in 0..height {
        for x in 0..width {
            let (r, g, b);
            if has_alpha || has_animation {
                let alpha = if samples == 4 { pixels[p + 3] } else { u8::MAX };
                let a = f32::from(alpha) / f32::from(u8::MAX);
                let blend = |v: u8, back: u16| {
                    ((a * f32::from(v) + (1.0 - a) * f32::from(back)) as i32) << 8
                };
                r = blend(pixels[p], bg.0);
                g = blend(pixels[p + 1], bg.1);
                b = blend(pixels[p + 2], bg.2);
            } else {
                r = i32::from(pixels[p]) << 8;
                g = i32::from(pixels[p + 1]) << 8;
                b = i32::from(pixels[p + 2]) << 8;
            }
            p += samples;
            ximg.put_pixel(x, y, lookup_rgb_color(r, g, b));
        }
    }
    img.pixmap = Some(ximg);
    img.width = width;
    img.height = height;
    if has_animation {
        let rest = lisp.cons(Value::fixnum(i64::from(frames)), img.lisp_data);
        img.lisp_data = lisp.cons(sym::COUNT, rest);
        let delay = if duration != 0 {
            lisp.float(f64::from(duration) / 1000.0)
        } else {
            sym::T
        };
        let rest = lisp.cons(delay, img.lisp_data);
        img.lisp_data = lisp.cons(sym::DELAY, rest);
    }
    Ok(true)
}
