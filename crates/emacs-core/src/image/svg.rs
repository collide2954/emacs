use std::sync::{Arc, OnceLock};

use resvg::tiny_skia;
use resvg::usvg;

use super::{
    Image, ImageKeyword, ImageValueType, PixContainer, lookup_rgb_color, scale_image_size,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const SVG_DATA: usize = 1;
const SVG_FILE: usize = 2;
const FRAME_SCALE_FACTOR: f64 = 1.0;
const RSVG_DEFAULT_DPI: f64 = 90.0;

fn svg_format() -> [ImageKeyword; 13] {
    use ImageValueType as T;
    [
        ImageKeyword::new(":type", T::Symbol, true),
        ImageKeyword::new(":data", T::String, false),
        ImageKeyword::new(":file", T::String, false),
        ImageKeyword::new(":base-uri", T::String, false),
        ImageKeyword::new(":css", T::String, false),
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

pub(super) fn svg_image_p(lisp: &mut Lisp, object: Value) -> bool {
    let mut fmt = svg_format();
    if !lisp.parse_image_spec(object, &mut fmt, sym::SVG) {
        return false;
    }
    usize::from(fmt[SVG_FILE].count) + usize::from(fmt[SVG_DATA].count) == 1
}

fn font_database() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

pub(super) fn svg_load(lisp: &mut Lisp, f: Value, img: &mut Image) -> LispResult<bool> {
    let (file_name, _) = lisp.image_spec_value(img.spec, sym::K_FILE);
    let (mut base_uri, _) = lisp.image_spec_value(img.spec, sym::K_BASE_URI);
    if file_name.is_string() {
        let file = lisp.image_find_image_file(file_name)?;
        if !file.is_string() {
            lisp.image_error("Cannot find image file `%s'", &[file_name])?;
            return Ok(false);
        }
        let path = lisp.string_to_rust(file);
        let Ok(contents) = std::fs::read(&path) else {
            lisp.image_error("Error loading SVG image `%s'", &[file])?;
            return Ok(false);
        };
        if !base_uri.is_string() {
            base_uri = file;
        }
        let base = lisp.string_to_rust(base_uri);
        svg_load_image(lisp, f, img, &contents, Some(base))
    } else {
        let (data, _) = lisp.image_spec_value(img.spec, sym::K_DATA);
        if !data.is_string() {
            lisp.image_error("Invalid image data `%s'", &[data])?;
            return Ok(false);
        }
        if !base_uri.is_string() {
            let buffer = lisp.current_buffer;
            base_uri = lisp.buffer_local_value(sym::BUFFER_FILE_NAME, buffer);
        }
        let base = base_uri.is_string().then(|| lisp.string_to_rust(base_uri));
        let contents = lisp.string_bytes(data).to_vec();
        svg_load_image(lisp, f, img, &contents, base)
    }
}

fn svg_error(lisp: &mut Lisp, error: &usvg::Error) -> LispResult<bool> {
    let message = error.to_string();
    let message = message.trim_end();
    if message.is_empty() {
        lisp.image_error("Error parsing SVG image", &[])?;
    } else {
        let message = lisp.string(message);
        lisp.image_error("Error parsing SVG image: %s", &[message])?;
    }
    Ok(false)
}

enum CssLength {
    Absolute(f64),
    Percent,
    Unknown,
}

fn parse_css_length(text: &str) -> Option<(f64, &str)> {
    let text = text.trim();
    let bytes = text.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
        end += 1;
    }
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        let mut exp = end + 1;
        if exp < bytes.len() && (bytes[exp] == b'+' || bytes[exp] == b'-') {
            exp += 1;
        }
        if exp < bytes.len() && bytes[exp].is_ascii_digit() {
            end = exp;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
        }
    }
    let value = text[..end].parse::<f64>().ok()?;
    Some((value, text[end..].trim()))
}

fn svg_css_length_to_pixels(value: f64, unit: &str, dpi: f64, font_size: f64) -> CssLength {
    CssLength::Absolute(match unit {
        "" | "px" => value,
        "cm" => dpi * value / 2.54,
        "mm" => dpi * value / 25.4,
        "pt" => dpi * value / 72.0,
        "pc" => dpi * value / 6.0,
        "in" => value * dpi,
        "em" => value * font_size,
        "ex" | "ch" => value * font_size / 2.0,
        "%" => return CssLength::Percent,
        _ => return CssLength::Unknown,
    })
}

fn intrinsic_size(
    lisp: &mut Lisp,
    text: &str,
    dpi: f64,
    font_size: f64,
) -> LispResult<Option<(f64, f64)>> {
    let options = usvg::roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let Ok(doc) = usvg::roxmltree::Document::parse_with_options(text, options) else {
        return Ok(None);
    };
    let root = doc.root_element();
    let mut length = |name: &str| -> LispResult<Option<CssLength>> {
        let Some((value, unit)) = root.attribute(name).and_then(parse_css_length) else {
            return Ok(None);
        };
        let length = svg_css_length_to_pixels(value, unit, dpi, font_size);
        if let CssLength::Unknown = length {
            let unit = lisp.string(unit);
            lisp.image_error("Unknown RSVG unit, code: %s", &[unit])?;
        }
        Ok(Some(length))
    };
    let (width, height) = (length("width")?, length("height")?);
    if let (Some(CssLength::Absolute(w)), Some(CssLength::Absolute(h))) = (&width, &height) {
        return Ok(Some((*w, *h)));
    }
    let pixels = |l: &Option<CssLength>| match l {
        Some(CssLength::Absolute(v)) => *v,
        _ => 0.0,
    };
    let (explicit_width, explicit_height) = (pixels(&width), pixels(&height));
    let viewbox = root.attribute("viewBox").and_then(|v| {
        let n: Vec<f64> = v
            .split(|c: char| c.is_ascii_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        (n.len() == 4 && n[2] > 0.0 && n[3] > 0.0).then(|| (n[2], n[3]))
    });
    let (w, h) = match viewbox {
        _ if explicit_width > 0.0 && explicit_height > 0.0 => (explicit_width, explicit_height),
        Some((vw, vh)) if explicit_width > 0.0 => (explicit_width, explicit_width * vh / vw),
        Some((vw, vh)) if explicit_height > 0.0 => (explicit_height * vw / vh, explicit_height),
        Some((vw, vh)) => (vw, vh),
        None => (0.0, 0.0),
    };
    Ok((w > 0.0 && h > 0.0).then_some((w, h)))
}

fn root_element_start(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut i = 0;
    let find = |from: usize, pat: &str| text[from..].find(pat).map(|p| from + p + pat.len());
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() || text[i..].starts_with('\u{feff}') {
            i += text[i..].chars().next().map_or(1, char::len_utf8);
        } else if text[i..].starts_with("<?") {
            match find(i, "?>") {
                Some(end) => i = end,
                None => return i,
            }
        } else if text[i..].starts_with("<!--") {
            match find(i, "-->") {
                Some(end) => i = end,
                None => return i,
            }
        } else if text[i..].starts_with("<!") {
            let close = text[i..].find('>').map(|p| i + p);
            let open = text[i..].find('[').map(|p| i + p);
            i = match (open, close) {
                (Some(open), Some(close)) if open < close => match find(open, "]") {
                    Some(end) => find(end, ">").unwrap_or(text.len()),
                    None => return i,
                },
                (_, Some(close)) => close + 1,
                _ => return i,
            };
        } else {
            return i;
        }
    }
    i
}

fn svg_load_image(
    lisp: &mut Lisp,
    f: Value,
    img: &mut Image,
    contents: &[u8],
    filename: Option<String>,
) -> LispResult<bool> {
    let terminal = lisp.frame(f).terminal;
    let dpi = lisp.display_info(terminal).resx;
    let (user_css, _) = lisp.image_spec_value(img.spec, sym::K_CSS);
    let user_css = if user_css.is_string() {
        String::from_utf8_lossy(lisp.string_bytes(user_css)).into_owned()
    } else {
        String::new()
    };
    let family = String::from_utf8_lossy(&img.face_font_family).into_owned();
    let css = format!(
        "svg{{  font-family: \"{family}\";  font-size:{}px;}}{user_css}",
        img.face_font_size
    );
    let dpi = if dpi > 0.0 { dpi } else { RSVG_DEFAULT_DPI };
    let mut opt = usvg::Options {
        dpi: dpi as f32,
        resources_dir: filename
            .as_ref()
            .and_then(|name| std::path::Path::new(name).parent().map(|p| p.to_path_buf())),
        fontdb: font_database(),
        style_sheet: Some(css),
        ..Default::default()
    };
    if !family.is_empty() {
        opt.font_family = family;
    }
    if img.face_font_size > 0 {
        opt.font_size = img.face_font_size as f32;
    }
    let decompressed;
    let source: &[u8] = if contents.starts_with(&[0x1f, 0x8b]) {
        match usvg::decompress_svgz(contents) {
            Ok(data) => {
                decompressed = data;
                &decompressed
            }
            Err(e) => return svg_error(lisp, &e),
        }
    } else {
        contents
    };
    let text = match std::str::from_utf8(source) {
        Ok(text) => text,
        Err(_) => return svg_error(lisp, &usvg::Error::NotAnUtf8Str),
    };
    let tree = match usvg::Tree::from_str(text, &opt) {
        Ok(tree) => tree,
        Err(e) => return svg_error(lisp, &e),
    };
    let (viewbox_width, viewbox_height) =
        match intrinsic_size(lisp, text, dpi, f64::from(img.face_font_size))? {
            Some(size) => size,
            None => {
                let bbox = tree.root().abs_bounding_box();
                (
                    f64::from(bbox.x() + bbox.width()),
                    f64::from(bbox.y() + bbox.height()),
                )
            }
        };
    let (width, height) = lisp.compute_image_size(f, viewbox_width, viewbox_height, img)?;
    let width = scale_image_size(f64::from(width), 1.0, FRAME_SCALE_FACTOR);
    let height = scale_image_size(f64::from(height), 1.0, FRAME_SCALE_FACTOR);
    if !lisp.check_image_size(Some(f), width, height) {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        return Ok(false);
    }
    let mut foreground = img.face_foreground;
    let mut background = img.face_background;
    let (value, _) = lisp.image_spec_value(img.spec, sym::K_FOREGROUND);
    if !value.is_nil() {
        foreground = lisp.image_alloc_image_color(f, img, value, img.face_foreground)?;
    }
    let (value, _) = lisp.image_spec_value(img.spec, sym::K_BACKGROUND);
    if !value.is_nil() {
        background = lisp.image_alloc_image_color(f, img, value, img.face_background)?;
        img.background = background;
        img.background_valid = true;
    }
    let (color, fill) = (foreground & 0xFF_FFFF, background & 0xFF_FFFF);
    let root = root_element_start(text);
    let wrapped = format!(
        "{}<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" xmlns:xi=\"http://www.w3.org/2001/XInclude\" style=\"color: #{color:06X};\" width=\"{width}\" height=\"{height}\" preserveAspectRatio=\"none\" viewBox=\"0 0 {viewbox_width:.6} {viewbox_height:.6}\"><rect width=\"100%\" height=\"100%\" fill=\"#{fill:06X}\"/>{}</svg>",
        &text[..root],
        &text[root..]
    );
    let tree = match usvg::Tree::from_str(&wrapped, &opt) {
        Ok(tree) => tree,
        Err(e) => return svg_error(lisp, &e),
    };
    let Some(mut pixmap) = tiny_skia::Pixmap::new(width.max(1) as u32, height.max(1) as u32) else {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        return Ok(false);
    };
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let mut ximg = PixContainer::new(width, height, 0);
    for y in 0..height {
        for x in 0..width {
            let c = pixmap
                .pixel(x as u32, y as u32)
                .map(|p| p.demultiply())
                .expect("pixel in the rendered SVG");
            ximg.put_pixel(
                x,
                y,
                lookup_rgb_color(
                    i32::from(c.red()) << 8,
                    i32::from(c.green()) << 8,
                    i32::from(c.blue()) << 8,
                ),
            );
        }
    }
    img.width = width;
    img.height = height;
    img.pixmap = Some(ximg);
    if !img.background_valid {
        lisp.image_background(f, img)?;
    }
    Ok(true)
}
