use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

use super::{
    Image, ImageKeyword, ImageValueType, PixContainer, image_clear_image, lookup_rgb_color,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const JPEG_DATA: usize = 1;
const JPEG_FILE: usize = 2;
const JPEG_MAX_DIMENSION: usize = 65500;

fn jpeg_format() -> [ImageKeyword; 10] {
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
    ]
}

pub(super) fn jpeg_image_p(lisp: &mut Lisp, object: Value) -> bool {
    let mut fmt = jpeg_format();
    if !lisp.parse_image_spec(object, &mut fmt, sym::JPEG) {
        return false;
    }
    usize::from(fmt[JPEG_FILE].count) + usize::from(fmt[JPEG_DATA].count) == 1
}

fn output_colorspace(input: ColorSpace) -> ColorSpace {
    match input {
        ColorSpace::Luma => ColorSpace::Luma,
        ColorSpace::CMYK | ColorSpace::YCCK => ColorSpace::CMYK,
        _ => ColorSpace::RGB,
    }
}

fn jpeg_failure(lisp: &mut Lisp, f: Value, img: &mut Image, message: &str) -> LispResult<bool> {
    let message = lisp.string(message);
    lisp.image_error("Error reading JPEG image `%s': %s", &[img.spec, message])?;
    image_clear_image(lisp, f, img);
    Ok(false)
}

pub(super) fn jpeg_load(lisp: &mut Lisp, f: Value, img: &mut Image) -> LispResult<bool> {
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
            lisp.image_error("Cannot open `%s'", &[file])?;
            return Ok(false);
        };
        contents
    } else if !specified_data.is_string() {
        lisp.image_error("Invalid image data `%s'", &[specified_data])?;
        return Ok(false);
    } else {
        lisp.string_bytes(specified_data).to_vec()
    };
    let options = DecoderOptions::default()
        .set_max_width(JPEG_MAX_DIMENSION)
        .set_max_height(JPEG_MAX_DIMENSION);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(&contents[..]), options);
    if let Err(e) = decoder.decode_headers() {
        return jpeg_failure(lisp, f, img, &e.to_string());
    }
    let input = decoder.input_colorspace().unwrap_or(ColorSpace::YCbCr);
    let out = output_colorspace(input);
    decoder.set_options(options.jpeg_set_out_colorspace(out));
    let (width, height) = decoder.dimensions().unwrap_or((0, 0));
    let (width, height) = (width as i32, height as i32);
    img.width = width;
    img.height = height;
    let ncomp = out.num_components();
    let (ir, ig, ib) = if ncomp > 2 {
        (0, 1, 2)
    } else if ncomp > 1 {
        (0, 1, 0)
    } else {
        (0, 0, 0)
    };
    if !lisp.check_image_size(Some(f), width, height) {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        image_clear_image(lisp, f, img);
        return Ok(false);
    }
    let mut ximg = PixContainer::new(width, height, 0);
    let pixels = match decoder.decode() {
        Ok(pixels) => pixels,
        Err(e) => return jpeg_failure(lisp, f, img, &e.to_string()),
    };
    let row_stride = width as usize * ncomp;
    for y in 0..height {
        let row = &pixels[y as usize * row_stride..];
        for x in 0..width {
            let off = x as usize * ncomp;
            let r = i32::from(row[off + ir]) << 8;
            let g = i32::from(row[off + ig]) << 8;
            let b = i32::from(row[off + ib]) << 8;
            ximg.put_pixel(x, y, lookup_rgb_color(r, g, b));
        }
    }
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
