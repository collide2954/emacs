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
    let mut opt = usvg::Options::default();
    if dpi > 0.0 {
        opt.dpi = dpi as f32;
    }
    opt.resources_dir = filename
        .as_ref()
        .and_then(|name| std::path::Path::new(name).parent().map(|p| p.to_path_buf()));
    opt.fontdb = font_database();
    opt.style_sheet = Some(css);
    if !family.is_empty() {
        opt.font_family = family;
    }
    if img.face_font_size > 0 {
        opt.font_size = img.face_font_size as f32;
    }
    let tree = match usvg::Tree::from_data(contents, &opt) {
        Ok(tree) => tree,
        Err(e) => return svg_error(lisp, &e),
    };
    let size = tree.size();
    let (viewbox_width, viewbox_height) = (f64::from(size.width()), f64::from(size.height()));
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
