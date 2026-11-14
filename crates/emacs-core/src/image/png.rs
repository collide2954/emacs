use super::{
    Image, ImageKeyword, ImageValueType, PIX_MASK_DRAW, PIX_MASK_RETAIN, PixContainer,
    lookup_rgb_color, query_color,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const PNG_DATA: usize = 1;
const PNG_FILE: usize = 2;
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

fn png_format() -> [ImageKeyword; 10] {
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
        ImageKeyword::new(":background", T::StringOrNil, false),
    ]
}

pub(super) fn png_image_p(lisp: &mut Lisp, object: Value) -> bool {
    let mut fmt = png_format();
    if !lisp.parse_image_spec(object, &mut fmt, sym::PNG) {
        return false;
    }
    usize::from(fmt[PNG_FILE].count) + usize::from(fmt[PNG_DATA].count) == 1
}

fn png_error(lisp: &mut Lisp, error: &png::DecodingError) -> LispResult<bool> {
    let msg = lisp.string(&error.to_string());
    lisp.image_error("PNG error: %s", &[msg])?;
    Ok(false)
}

fn png_warning(lisp: &mut Lisp, msg: &str) -> LispResult<()> {
    let msg = lisp.string(msg);
    lisp.image_error("PNG warning: %s", &[msg])
}

fn composite_8(fg: u16, alpha: u16, bg: u16) -> u16 {
    let temp = u32::from(fg) * u32::from(alpha) + u32::from(bg) * u32::from(255 - alpha) + 128;
    ((temp + (temp >> 8)) >> 8) as u16 & 0xff
}

fn composite_16(fg: u16, alpha: u16, bg: u16) -> u16 {
    let temp = u64::from(fg) * u64::from(alpha) + u64::from(bg) * u64::from(65535 - alpha) + 32768;
    ((temp + (temp >> 16)) >> 16) as u16
}

fn file_background(info: &png::Info<'_>) -> Result<Option<(u32, u32, u32)>, &'static str> {
    let Some(buf) = info.bkgd.as_deref() else {
        return Ok(None);
    };
    let depth = info.bit_depth as u32;
    let be = |i: usize| u32::from(u16::from_be_bytes([buf[i], buf[i + 1]]));
    match info.color_type {
        png::ColorType::Indexed => {
            let palette = info.palette.as_deref().unwrap_or(&[]);
            let index = usize::from(buf[0]);
            if index >= palette.len() / 3 {
                return Err("invalid index");
            }
            let entry = &palette[index * 3..index * 3 + 3];
            Ok(Some((
                u32::from(entry[0]),
                u32::from(entry[1]),
                u32::from(entry[2]),
            )))
        }
        png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha => {
            if depth <= 8 && (buf[0] != 0 || u32::from(buf[1]) >= 1 << depth) {
                return Err("invalid gray level");
            }
            let gray = be(0);
            Ok(Some((gray, gray, gray)))
        }
        png::ColorType::Rgb | png::ColorType::Rgba => {
            if depth <= 8 && (buf[0] != 0 || buf[2] != 0 || buf[4] != 0) {
                return Err("invalid color");
            }
            Ok(Some((be(0), be(2), be(4))))
        }
    }
}

fn query_frame_background_color(lisp: &Lisp, f: Value) -> (u32, u32, u32) {
    let (r, g, b) = query_color(lisp.frame(f).background_pixel as u64);
    (r as u32, g as u32, b as u32)
}

pub(super) fn png_load(lisp: &mut Lisp, f: Value, img: &mut Image) -> LispResult<bool> {
    let (specified_file, _) = lisp.image_spec_value(img.spec, sym::K_FILE);
    let (specified_data, _) = lisp.image_spec_value(img.spec, sym::K_DATA);
    let contents = if specified_data.is_nil() {
        let file = lisp.image_find_image_file(specified_file)?;
        if !file.is_string() {
            lisp.image_error("Cannot find image file `%s'", &[specified_file])?;
            return Ok(false);
        }
        let path = lisp.string_to_rust(file);
        let Ok(contents) = std::fs::read(path) else {
            lisp.image_error("Cannot open image file `%s'", &[file])?;
            return Ok(false);
        };
        if contents.len() < PNG_SIGNATURE.len() || contents[..8] != PNG_SIGNATURE {
            lisp.image_error("Not a PNG file: `%s'", &[file])?;
            return Ok(false);
        }
        contents
    } else {
        if !specified_data.is_string() {
            lisp.image_error("Invalid image data `%s'", &[specified_data])?;
            return Ok(false);
        }
        let contents = lisp.string_bytes(specified_data).to_vec();
        if contents.len() < PNG_SIGNATURE.len() || contents[..8] != PNG_SIGNATURE {
            lisp.image_error("Not a PNG image: `%s'", &[img.spec])?;
            return Ok(false);
        }
        contents
    };
    let mut decoder = png::Decoder::new(std::io::Cursor::new(&contents[..]));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = match decoder.read_info() {
        Ok(reader) => reader,
        Err(e) => return png_error(lisp, &e),
    };
    let info = reader.info();
    let (width, height) = (info.width, info.height);
    let bit_depth = info.bit_depth as u32;
    let color_type = info.color_type;
    let has_trns = info.trns.is_some();
    let transparent_p = match info.trns.as_deref() {
        Some(alpha) if color_type == png::ColorType::Indexed => {
            alpha.iter().all(|&a| a == 0 || a == 255)
        }
        Some(_) => true,
        None => false,
    };
    let file_bg = file_background(info);
    if let Err(msg) = file_bg {
        png_warning(lisp, msg)?;
    }
    let file_bg = file_bg.ok().flatten();
    if !(width <= i32::MAX as u32
        && height <= i32::MAX as u32
        && lisp.check_image_size(Some(f), width as i32, height as i32))
    {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        return Ok(false);
    }
    let (width, height) = (width as i32, height as i32);
    let mut ximg = PixContainer::new(width, height, 0);
    let mut background = None;
    if !transparent_p {
        let (specified_bg, _) = lisp.image_spec_value(img.spec, sym::K_BACKGROUND);
        let color = if specified_bg.is_string() {
            let name = lisp.string_bytes(specified_bg).to_vec();
            let (defined, color) = lisp.defined_color(f, &name)?;
            defined.then_some((
                u32::from(color.red),
                u32::from(color.green),
                u32::from(color.blue),
            ))
        } else {
            Some(query_frame_background_color(lisp, f))
        };
        if let Some((r, g, b)) = color {
            let shift = if bit_depth == 16 { 0 } else { 8 };
            background = Some((r >> shift, g >> shift, b >> shift));
        }
    }
    let has_alpha = matches!(
        color_type,
        png::ColorType::GrayscaleAlpha | png::ColorType::Rgba
    );
    let compose = background.filter(|_| has_alpha || has_trns);
    let Some(size) = reader.output_buffer_size() else {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        return Ok(false);
    };
    let mut buf = vec![0u8; size];
    let frame = match reader.next_frame(&mut buf) {
        Ok(frame) => frame,
        Err(e) => return png_error(lisp, &e),
    };
    if let Err(e) = reader.finish() {
        return png_error(lisp, &e);
    }
    let (out_color, out_depth) = (frame.color_type, frame.bit_depth);
    let sixteen = out_depth == png::BitDepth::Sixteen;
    let samples = out_color.samples();
    let sample = |i: usize| -> u16 {
        if sixteen {
            u16::from_be_bytes([buf[i * 2], buf[i * 2 + 1]])
        } else {
            u16::from(buf[i])
        }
    };
    let out_alpha = matches!(
        out_color,
        png::ColorType::GrayscaleAlpha | png::ColorType::Rgba
    );
    let channels = if out_alpha && compose.is_none() { 4 } else { 3 };
    let mut mask_img =
        (channels == 4 && transparent_p).then(|| PixContainer::new(width, height, 1));
    let row_samples = frame.line_size / if sixteen { 2 } else { 1 };
    for y in 0..height {
        for x in 0..width {
            let at = y as usize * row_samples + x as usize * samples;
            let (mut r, mut g, mut b, a) = match out_color {
                png::ColorType::Grayscale => {
                    let v = sample(at);
                    (v, v, v, None)
                }
                png::ColorType::GrayscaleAlpha => {
                    let v = sample(at);
                    (v, v, v, Some(sample(at + 1)))
                }
                png::ColorType::Rgba => (
                    sample(at),
                    sample(at + 1),
                    sample(at + 2),
                    Some(sample(at + 3)),
                ),
                _ => (sample(at), sample(at + 1), sample(at + 2), None),
            };
            if let (Some((br, bg, bb)), Some(alpha)) = (compose, a) {
                let full = if sixteen { 65535 } else { 255 };
                let mix = |fg: u16, back: u32| -> u16 {
                    if alpha == full {
                        fg
                    } else if alpha == 0 {
                        back as u16
                    } else if sixteen {
                        composite_16(fg, alpha, back as u16)
                    } else {
                        composite_8(fg, alpha, back as u16)
                    }
                };
                r = mix(r, br);
                g = mix(g, bg);
                b = mix(b, bb);
            }
            let chop = |v: u16| -> i32 {
                if sixteen {
                    i32::from(v >> 8)
                } else {
                    i32::from(v)
                }
            };
            ximg.put_pixel(
                x,
                y,
                lookup_rgb_color(chop(r) << 8, chop(g) << 8, chop(b) << 8),
            );
            if channels == 4
                && let (Some(mask), Some(alpha)) = (mask_img.as_mut(), a)
            {
                let alpha = chop(alpha);
                mask.put_pixel(
                    x,
                    y,
                    if alpha > 0 {
                        PIX_MASK_DRAW
                    } else {
                        PIX_MASK_RETAIN
                    },
                );
            }
        }
    }
    if lisp
        .image_spec_value(img.spec, sym::K_BACKGROUND)
        .0
        .is_nil()
    {
        let bkgd = if compose.is_some() {
            file_bg.and(compose)
        } else {
            file_bg
        };
        if let Some((r, g, b)) = bkgd {
            let name = lisp.string(&format!("#{r:04x}{g:04x}{b:04x}"));
            let base = lisp.stack.len();
            lisp.stack.push(name);
            let pixel = lisp.image_alloc_image_color(f, img, name, 0);
            lisp.stack.truncate(base);
            img.background = pixel?;
            img.background_valid = true;
        }
    }
    img.width = width;
    img.height = height;
    img.pixmap = Some(ximg);
    if !img.background_valid {
        lisp.image_background(f, img)?;
    }
    if let Some(mask) = mask_img.take() {
        img.mask = Some(mask);
        Lisp::image_background_transparent(img);
    }
    Ok(true)
}
