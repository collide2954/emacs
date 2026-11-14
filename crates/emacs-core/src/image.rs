mod algorithms;
mod gif;
mod jpeg;
mod pbm;
mod png;
mod svg;
mod tiff;
mod webp;
mod xbm;
mod xpm;

use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::timefns::{Timespec, current_timespec, dtotimespec, timespec_sub};
use crate::value::Value;

pub const DEFAULT_IMAGE_WIDTH: i32 = 30;
pub const DEFAULT_IMAGE_HEIGHT: i32 = 30;
pub const DEFAULT_IMAGE_ASCENT: i32 = 50;
pub const CENTERED_IMAGE_ASCENT: i32 = -1;
pub const TOP_CORNER: usize = 0;
pub const LEFT_CORNER: usize = 1;
pub const BOT_CORNER: usize = 2;
pub const RIGHT_CORNER: usize = 3;
pub const PIX_MASK_RETAIN: u64 = 0;
pub const PIX_MASK_DRAW: u64 = 255;
const IMAGE_CACHE_BUCKETS_SIZE: u64 = 1009;
const MAX_IMAGE_SIZE: f64 = 10.0;
const PATH_BITMAPS: &str = if cfg!(windows) {
    "."
} else {
    "/usr/include/X11/bitmaps"
};
const CLEAR_IMAGE_PIXMAP: u32 = 1 << 0;
const CLEAR_IMAGE_MASK: u32 = 1 << 1;
const CLEAR_IMAGE_COLORS: u32 = 1 << 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PixContainer {
    pub width: i32,
    pub height: i32,
    pub bits_per_pixel: u8,
    pub bytes_per_line: usize,
    pub data: Vec<u8>,
}

impl PixContainer {
    pub fn new(width: i32, height: i32, depth: i32) -> PixContainer {
        let bits_per_pixel = if depth == 1 { 8 } else { 32 };
        let w = width.max(0) as usize;
        let bytes_per_line = if depth == 1 { (w + 3) & !3 } else { w * 4 };
        PixContainer {
            width,
            height,
            bits_per_pixel,
            bytes_per_line,
            data: vec![0; bytes_per_line * height.max(0) as usize],
        }
    }

    pub fn put_pixel(&mut self, x: i32, y: i32, pixel: u64) {
        let row = y as usize * self.bytes_per_line;
        if self.bits_per_pixel == 32 {
            let at = row + x as usize * 4;
            self.data[at..at + 4].copy_from_slice(&(pixel as u32).to_ne_bytes());
        } else {
            self.data[row + x as usize] = pixel as u8;
        }
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> u64 {
        let row = y as usize * self.bytes_per_line;
        if self.bits_per_pixel == 32 {
            let at = row + x as usize * 4;
            u64::from(u32::from_ne_bytes(
                self.data[at..at + 4].try_into().expect("pixel bytes"),
            ))
        } else {
            u64::from(self.data[row + x as usize])
        }
    }

    fn size_in_bytes(&self) -> i64 {
        i64::from(self.height) * self.bytes_per_line as i64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrFilter {
    Nearest,
    Good,
    Best,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrFormat {
    Rgb24,
    Argb32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CrSurface {
    pub width: i32,
    pub height: i32,
    pub stride: usize,
    pub format: CrFormat,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CrData {
    pub matrix: [f64; 6],
    pub filter: CrFilter,
    pub surface: Option<CrSurface>,
}

impl Default for CrData {
    fn default() -> CrData {
        CrData {
            matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            filter: CrFilter::Good,
            surface: None,
        }
    }
}

pub struct ImageType {
    pub type_: Value,
    pub valid_p: fn(&mut Lisp, Value) -> bool,
    pub load_img: fn(&mut Lisp, Value, &mut Image) -> LispResult<bool>,
    pub free_img: fn(&mut Lisp, Value, &mut Image),
}

static IMAGE_TYPES: &[ImageType] = &[
    ImageType {
        type_: sym::XBM,
        valid_p: xbm::xbm_image_p,
        load_img: xbm::xbm_load,
        free_img: image_clear_image,
    },
    ImageType {
        type_: sym::PBM,
        valid_p: pbm::pbm_image_p,
        load_img: pbm::pbm_load,
        free_img: image_clear_image,
    },
    ImageType {
        type_: sym::XPM,
        valid_p: xpm::xpm_image_p,
        load_img: xpm::xpm_load,
        free_img: image_clear_image,
    },
    ImageType {
        type_: sym::GIF,
        valid_p: gif::gif_image_p,
        load_img: gif::gif_load,
        free_img: image_clear_image,
    },
    ImageType {
        type_: sym::JPEG,
        valid_p: jpeg::jpeg_image_p,
        load_img: jpeg::jpeg_load,
        free_img: image_clear_image,
    },
    ImageType {
        type_: sym::TIFF,
        valid_p: tiff::tiff_image_p,
        load_img: tiff::tiff_load,
        free_img: image_clear_image,
    },
    ImageType {
        type_: sym::PNG,
        valid_p: png::png_image_p,
        load_img: png::png_load,
        free_img: image_clear_image,
    },
    ImageType {
        type_: sym::WEBP,
        valid_p: webp::webp_image_p,
        load_img: webp::webp_load,
        free_img: image_clear_image,
    },
    ImageType {
        type_: sym::SVG,
        valid_p: svg::svg_image_p,
        load_img: svg::svg_load,
        free_img: image_clear_image,
    },
];

pub struct Image {
    pub timestamp: Timespec,
    pub pixmap: Option<PixContainer>,
    pub mask: Option<PixContainer>,
    pub cr_data: Option<CrData>,
    pub colors: Vec<u64>,
    pub background: u64,
    pub face_foreground: u64,
    pub face_background: u64,
    pub face_font_size: i32,
    pub face_font_family: Vec<u8>,
    pub face_font_height: i32,
    pub face_font_width: i32,
    pub background_transparent: bool,
    pub background_valid: bool,
    pub background_transparent_valid: bool,
    pub refresh: u32,
    pub original_width: i32,
    pub original_height: i32,
    pub width: i32,
    pub height: i32,
    pub scale: f64,
    pub corners: [i32; 4],
    pub ascent: i32,
    pub spec: Value,
    pub dependencies: Value,
    pub relief: i32,
    pub hmargin: i32,
    pub vmargin: i32,
    pub type_: &'static ImageType,
    pub load_failed_p: bool,
    pub lisp_data: Value,
    pub hash: u64,
    pub id: usize,
}

#[derive(Default)]
pub struct ImageCache {
    buckets: Vec<Vec<usize>>,
    images: Vec<Option<Box<Image>>>,
    refcount: usize,
    scaling_col_width: i64,
}

pub(crate) struct AnimCache {
    pub(crate) spec: Value,
    pub(crate) gif: gif::GifAnimHandle,
    pub(crate) webp: webp::WebpAnimHandle,
    pub(crate) index: i32,
    pub(crate) frames: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) byte_size: i64,
    pub(crate) update_time: Timespec,
}

#[derive(Default)]
pub struct Images {
    caches: Vec<Option<ImageCache>>,
    anim: Vec<AnimCache>,
}

impl Images {
    pub fn roots(&self, roots: &mut Vec<Value>) {
        for cache in self.caches.iter().flatten() {
            for img in cache.images.iter().flatten() {
                roots.extend_from_slice(&[img.spec, img.dependencies, img.lisp_data]);
            }
        }
        roots.extend(self.anim.iter().map(|cache| cache.spec));
    }
}

impl Lisp {
    fn anim_prune_animation_cache(&mut self, clear: Value) {
        let old = timespec_sub(current_timespec(), (60, 0));
        self.images.anim.retain(|cache| {
            !(clear == sym::T || (clear.is_nil() && old > cache.update_time) || clear == cache.spec)
        });
    }

    pub(crate) fn anim_get_animation_cache(&mut self, spec: Value) -> usize {
        self.anim_prune_animation_cache(Value::NIL);
        let index = match self.images.anim.iter().position(|c| c.spec == spec) {
            Some(index) => index,
            None => {
                self.images.anim.push(AnimCache {
                    spec,
                    gif: gif::GifAnimHandle::default(),
                    webp: webp::WebpAnimHandle::default(),
                    index: -1,
                    frames: -1,
                    width: -1,
                    height: -1,
                    byte_size: 0,
                    update_time: (0, 0),
                });
                self.images.anim.len() - 1
            }
        };
        self.images.anim[index].update_time = current_timespec();
        index
    }

    pub fn image_prune_animation_caches(&mut self, clear: bool) {
        self.anim_prune_animation_cache(if clear { sym::T } else { Value::NIL });
    }
}

pub(crate) struct ImageKeyword {
    pub name: &'static str,
    pub kind: ImageValueType,
    pub mandatory_p: bool,
    pub count: bool,
    pub value: Value,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageValueType {
    DontCheck,
    String,
    StringOrNil,
    Symbol,
    PositiveInteger,
    NonNegativeInteger,
    NonNegativeIntegerOrPair,
    Ascent,
    Integer,
}

impl ImageKeyword {
    pub(crate) const fn new(name: &'static str, kind: ImageValueType, mandatory_p: bool) -> Self {
        ImageKeyword {
            name,
            kind,
            mandatory_p,
            count: false,
            value: Value::NIL,
        }
    }
}

fn ranged_fixnum(v: Value, lo: i64, hi: i64) -> bool {
    v.as_fixnum().is_some_and(|n| (lo..=hi).contains(&n))
}

pub(crate) fn rgb_to_ulong(r: u64, g: u64, b: u64) -> u64 {
    (r << 16) | (g << 8) | b
}

pub(crate) fn red_from_ulong(color: u64) -> u64 {
    (color >> 16) & 0xff
}

pub(crate) fn green_from_ulong(color: u64) -> u64 {
    (color >> 8) & 0xff
}

pub(crate) fn blue_from_ulong(color: u64) -> u64 {
    color & 0xff
}

pub(crate) fn lookup_rgb_color(r: i32, g: i32, b: i32) -> u64 {
    rgb_to_ulong((r >> 8) as u64, (g >> 8) as u64, (b >> 8) as u64)
}

pub(crate) fn query_color(pixel: u64) -> (i32, i32, i32) {
    (
        (red_from_ulong(pixel) * 257) as i32,
        (green_from_ulong(pixel) * 257) as i32,
        (blue_from_ulong(pixel) * 257) as i32,
    )
}

fn lookup_image_type(type_: Value) -> Option<&'static ImageType> {
    IMAGE_TYPES.iter().find(|t| t.type_ == type_)
}

pub(crate) fn four_corners_best(pimg: &PixContainer, corners: &[i32; 4]) -> u64 {
    let (width, height) = (pimg.width, pimg.height);
    let corner_pixels = if corners[BOT_CORNER] >= 0 {
        [
            pimg.get_pixel(corners[LEFT_CORNER], corners[TOP_CORNER]),
            pimg.get_pixel(corners[RIGHT_CORNER] - 1, corners[TOP_CORNER]),
            pimg.get_pixel(corners[RIGHT_CORNER] - 1, corners[BOT_CORNER] - 1),
            pimg.get_pixel(corners[LEFT_CORNER], corners[BOT_CORNER] - 1),
        ]
    } else {
        [
            pimg.get_pixel(0, 0),
            pimg.get_pixel(width - 1, 0),
            pimg.get_pixel(width - 1, height - 1),
            pimg.get_pixel(0, height - 1),
        ]
    };
    let mut best = 0;
    let mut best_count = 0;
    for i in 0..4 {
        let n = corner_pixels
            .iter()
            .filter(|&&p| p == corner_pixels[i])
            .count();
        if n > best_count {
            best = corner_pixels[i];
            best_count = n;
        }
    }
    best
}

pub(crate) fn image_clear_image(_lisp: &mut Lisp, _f: Value, img: &mut Image) {
    img.lisp_data = Value::NIL;
    image_clear_image_1(
        img,
        CLEAR_IMAGE_PIXMAP | CLEAR_IMAGE_MASK | CLEAR_IMAGE_COLORS,
    );
}

pub(crate) fn image_clear_image_1(img: &mut Image, flags: u32) {
    if flags & CLEAR_IMAGE_PIXMAP != 0 && img.pixmap.take().is_some() {
        img.background_valid = false;
    }
    if flags & CLEAR_IMAGE_MASK != 0 && img.mask.take().is_some() {
        img.background_transparent_valid = false;
    }
    if flags & CLEAR_IMAGE_COLORS != 0 && !img.colors.is_empty() {
        img.colors.clear();
    }
    img.cr_data = None;
}

fn scale_image_size(size: f64, divisor: f64, multiplier: f64) -> i32 {
    if divisor != 0.0 {
        let scaled = size * multiplier / divisor;
        if scaled < f64::from(i32::MAX) {
            return scaled.ceil() as i32;
        }
    }
    i32::MAX
}

type Matrix3x3 = [[f64; 3]; 3];

fn matrix3x3_mult(a: &Matrix3x3, b: &Matrix3x3) -> Matrix3x3 {
    let mut result = [[0.0; 3]; 3];
    for (i, row) in result.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    result
}

impl Lisp {
    pub(crate) fn image_error(&mut self, format: &str, args: &[Value]) -> LispResult<()> {
        self.add_to_log(format, args)
    }

    pub fn is_image_spec(&self, object: Value) -> bool {
        object.is_cons() && self.xcar(object) == sym::IMAGE
    }

    pub fn valid_image_p(&mut self, object: Value) -> bool {
        if !self.is_image_spec(object) {
            return false;
        }
        let mut tail = self.xcdr(object);
        let mut seen = 0usize;
        while tail.is_cons() && seen < 1 << 20 {
            seen += 1;
            if self.xcar(tail) == sym::K_TYPE {
                tail = self.xcdr(tail);
                if tail.is_cons() {
                    let value = self.xcar(tail);
                    if let Some(t) = lookup_image_type(value) {
                        return (t.valid_p)(self, object);
                    }
                }
                break;
            }
            tail = self.xcdr(tail);
            if !tail.is_cons() {
                return false;
            }
            tail = self.xcdr(tail);
        }
        false
    }

    pub(crate) fn parse_image_spec(
        &mut self,
        spec: Value,
        keywords: &mut [ImageKeyword],
        type_: Value,
    ) -> bool {
        if !self.is_image_spec(spec) {
            return false;
        }
        let mut plist = self.xcdr(spec);
        while plist.is_cons() {
            let key = self.xcar(plist);
            plist = self.xcdr(plist);
            if !key.is_symbol() {
                return false;
            }
            if !plist.is_cons() {
                return false;
            }
            let value = self.xcar(plist);
            let name = self.string_bytes(self.symbol_name(key)).to_vec();
            if let Some(i) = keywords
                .iter()
                .position(|k| k.name.as_bytes() == name.as_slice())
            {
                keywords[i].value = value;
                if keywords[i].count {
                    return false;
                }
                keywords[i].count = true;
                let ok = match keywords[i].kind {
                    ImageValueType::String => value.is_string(),
                    ImageValueType::StringOrNil => value.is_string() || value.is_nil(),
                    ImageValueType::Symbol => value.is_symbol(),
                    ImageValueType::PositiveInteger => ranged_fixnum(value, 1, i64::from(i32::MAX)),
                    ImageValueType::NonNegativeInteger => value.as_fixnum().is_some_and(|n| n >= 0),
                    ImageValueType::NonNegativeIntegerOrPair => {
                        ranged_fixnum(value, 0, i64::from(i32::MAX))
                            || (value.is_cons()
                                && ranged_fixnum(self.xcar(value), 0, i64::from(i32::MAX))
                                && ranged_fixnum(self.xcdr(value), 0, i64::from(i32::MAX)))
                    }
                    ImageValueType::Ascent => {
                        (value.is_symbol() && value == sym::CENTER) || ranged_fixnum(value, 0, 100)
                    }
                    ImageValueType::DontCheck => true,
                    ImageValueType::Integer => {
                        ranged_fixnum(value, i64::from(i32::MIN), i64::from(i32::MAX))
                    }
                };
                if !ok {
                    return false;
                }
                if key == sym::K_TYPE && !(type_ == value || type_ == sym::NATIVE_IMAGE) {
                    return false;
                }
            }
            if self.xcdr(plist).is_nil() {
                return keywords.iter().all(|k| !k.mandatory_p || k.count);
            }
            plist = self.xcdr(plist);
        }
        false
    }

    pub fn image_spec_value(&self, spec: Value, key: Value) -> (Value, bool) {
        let mut tail = self.xcdr(spec);
        while tail.is_cons() {
            if self.xcar(tail) == key {
                let rest = self.xcdr(tail);
                let value = if rest.is_cons() {
                    self.xcar(rest)
                } else {
                    Value::NIL
                };
                return (value, true);
            }
            tail = self.xcdr(tail);
            if !tail.is_cons() {
                break;
            }
            tail = self.xcdr(tail);
        }
        (Value::NIL, false)
    }

    fn make_image(&mut self, spec: Value, hash: u64) -> Box<Image> {
        let (file, _) = self.image_spec_value(spec, sym::K_FILE);
        let dependencies = if file.is_nil() {
            Value::NIL
        } else {
            self.list(&[file])
        };
        let (type_symbol, _) = self.image_spec_value(spec, sym::K_TYPE);
        Box::new(Image {
            timestamp: (0, 0),
            pixmap: None,
            mask: None,
            cr_data: None,
            colors: Vec::new(),
            background: 0,
            face_foreground: 0,
            face_background: 0,
            face_font_size: 0,
            face_font_family: Vec::new(),
            face_font_height: 0,
            face_font_width: 0,
            background_transparent: false,
            background_valid: false,
            background_transparent_valid: false,
            refresh: 0,
            original_width: 0,
            original_height: 0,
            width: 0,
            height: 0,
            scale: 0.0,
            corners: [0, 0, -1, 0],
            ascent: DEFAULT_IMAGE_ASCENT,
            spec,
            dependencies,
            relief: 0,
            hmargin: 0,
            vmargin: 0,
            type_: lookup_image_type(type_symbol).expect("valid image type"),
            load_failed_p: false,
            lisp_data: Value::NIL,
            hash,
            id: 0,
        })
    }

    fn image_cache_mut(&mut self, f: Value) -> Option<&mut ImageCache> {
        let index = self.frame(f).image_cache?;
        self.images.caches.get_mut(index).and_then(Option::as_mut)
    }

    fn image_cache(&self, f: Value) -> Option<&ImageCache> {
        let index = self.frame(f).image_cache?;
        self.images.caches.get(index).and_then(Option::as_ref)
    }

    pub fn image(&self, f: Value, id: usize) -> Option<&Image> {
        self.image_cache(f)?.images.get(id)?.as_deref()
    }

    pub fn image_mut(&mut self, f: Value, id: usize) -> Option<&mut Image> {
        self.image_cache_mut(f)?.images.get_mut(id)?.as_deref_mut()
    }

    fn take_image(&mut self, f: Value, id: usize) -> Option<Box<Image>> {
        self.image_cache_mut(f)?.images.get_mut(id)?.take()
    }

    fn put_image(&mut self, f: Value, img: Box<Image>) {
        let id = img.id;
        if let Some(slot) = self.image_cache_mut(f).and_then(|c| c.images.get_mut(id)) {
            *slot = Some(img);
        }
    }

    fn free_image(&mut self, f: Value, id: usize) {
        let Some(mut img) = self.take_image(f, id) else {
            return;
        };
        if let Some(c) = self.image_cache_mut(f) {
            let bucket = (img.hash % IMAGE_CACHE_BUCKETS_SIZE) as usize;
            c.buckets[bucket].retain(|&i| i != id);
        }
        let free = img.type_.free_img;
        free(self, f, &mut img);
    }

    pub(crate) fn check_image_size(&self, f: Option<Value>, width: i32, height: i32) -> bool {
        if width <= 0 || height <= 0 {
            return false;
        }
        let max = self.plain_value(sym::MAX_IMAGE_SIZE);
        if let Some(n) = max.as_fixnum() {
            i64::from(width) <= n && i64::from(height) <= n
        } else if max.is_float() {
            let (w, h) = match f {
                Some(f) => (
                    self.frame(f).geometry.pixel_width as f64,
                    self.frame(f).geometry.pixel_height as f64,
                ),
                None => (1024.0, 1024.0),
            };
            let m = self.heap.float_value(max);
            f64::from(width) <= m * w && f64::from(height) <= m * h
        } else {
            true
        }
    }

    pub(crate) fn image_alloc_image_color(
        &mut self,
        f: Value,
        img: &mut Image,
        color_name: Value,
        dflt: u64,
    ) -> LispResult<u64> {
        let name = self.string_bytes(color_name).to_vec();
        let (defined, color) = self.defined_color(f, &name)?;
        if defined {
            img.colors.push(color.pixel as u64);
            Ok(color.pixel as u64)
        } else {
            Ok(dflt)
        }
    }

    pub(crate) fn image_background(&mut self, f: Value, img: &mut Image) -> LispResult<u64> {
        if !img.background_valid {
            let Some(pixmap) = img.pixmap.as_ref() else {
                return Ok(img.background);
            };
            let bg = four_corners_best(pixmap, &img.corners);
            let name = self.string(&format!(
                "#{:04x}{:04x}{:04x}",
                red_from_ulong(bg) * 0x101,
                green_from_ulong(bg) * 0x101,
                blue_from_ulong(bg) * 0x101
            ));
            let base = self.stack.len();
            self.stack.push(name);
            let r = self.image_alloc_image_color(f, img, name, 0);
            self.stack.truncate(base);
            img.background = r?;
            img.background_valid = true;
        }
        Ok(img.background)
    }

    pub(crate) fn image_background_transparent(img: &mut Image) -> bool {
        if !img.background_transparent_valid {
            img.background_transparent = match img.mask.as_ref() {
                Some(mask) => four_corners_best(mask, &img.corners) == PIX_MASK_RETAIN,
                None => false,
            };
            img.background_transparent_valid = true;
        }
        img.background_transparent
    }

    pub fn image_ascent(&self, img: &Image, face: &crate::xfaces::Face, y: i32, h: i32) -> i32 {
        let height = if h == img.height {
            img.height + img.vmargin
        } else if y == 0 {
            h + img.vmargin
        } else {
            h
        };
        if img.ascent == CENTERED_IMAGE_ASCENT {
            if face.font.is_nil() {
                height / 2
            } else {
                let font = self.font_object(face.font);
                (height + font.ascent - font.descent + 1) / 2
            }
        } else {
            (f64::from(height) * (f64::from(img.ascent) / 100.0)) as i32
        }
    }

    pub fn prepare_image_for_display(&mut self, f: Value, id: usize) -> LispResult<()> {
        self.with_gc_deferred(|lisp| {
            let Some(mut img) = lisp.take_image(f, id) else {
                return Ok(());
            };
            let r = lisp.prepare_image_for_display_1(f, &mut img);
            lisp.put_image(f, img);
            r
        })
    }

    fn prepare_image_for_display_1(&mut self, f: Value, img: &mut Image) -> LispResult<()> {
        img.timestamp = current_timespec();
        if img.pixmap.is_none() && !img.load_failed_p {
            let load = img.type_.load_img;
            img.load_failed_p = !load(self, f, img)?;
        }
        if !img.load_failed_p && img.cr_data.as_ref().is_none_or(|cr| cr.surface.is_none()) {
            if !img.background_valid {
                self.image_background(f, img)?;
            }
            Lisp::image_background_transparent(img);
            cr_put_image_to_cr_data(img);
            if img.cr_data.is_none() {
                img.load_failed_p = true;
                let free = img.type_.free_img;
                free(self, f, img);
            }
        }
        Ok(())
    }

    fn search_image_cache(
        &mut self,
        f: Value,
        spec: Value,
        hash: u64,
        colors: Option<(u64, u64, i32, &[u8])>,
    ) -> LispResult<Option<usize>> {
        let bucket = (hash % IMAGE_CACHE_BUCKETS_SIZE) as usize;
        let Some(ids) = self.image_cache(f).map(|c| c.buckets[bucket].clone()) else {
            return Ok(None);
        };
        let scale = self.image_compute_scale(f, spec, None)?;
        for id in ids {
            let Some(img) = self.image(f, id) else {
                continue;
            };
            if img.hash != hash {
                continue;
            }
            let (img_spec, img_scale) = (img.spec, img.scale);
            let colors_match = match colors {
                None => true,
                Some((fg, bg, size, family)) => {
                    img.face_foreground == fg
                        && img.face_background == bg
                        && img.face_font_size == size
                        && img.face_font_family.as_slice() == family
                }
            };
            if self.equal(img_spec, spec)? && scale == img_scale && colors_match {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    fn filter_image_spec(&mut self, spec: Value) -> Value {
        let (type_, _) = self.image_spec_value(spec, sym::K_TYPE);
        let is_canvas = type_ == sym::CANVAS;
        let mut out = Value::NIL;
        let mut spec = if spec.is_cons() {
            self.xcdr(spec)
        } else {
            spec
        };
        let base = self.stack.len();
        while spec.is_cons() {
            let key = self.xcar(spec);
            spec = self.xcdr(spec);
            if spec.is_cons() {
                let value = self.xcar(spec);
                spec = self.xcdr(spec);
                let skip = if is_canvas {
                    key == sym::K_DATA
                } else {
                    key == sym::K_ANIMATE_BUFFER
                        || key == sym::K_ANIMATE_TARDINESS
                        || key == sym::K_ANIMATE_POSITION
                };
                if !skip {
                    out = self.cons(value, out);
                    out = self.cons(key, out);
                    self.stack.push(out);
                }
            }
        }
        self.stack.truncate(base);
        out
    }

    fn uncache_image(&mut self, f: Value, spec: Value) -> LispResult<()> {
        let filtered = self.filter_image_spec(spec);
        let hash = self.sxhash_equal(filtered, 0);
        while let Some(id) = self.search_image_cache(f, spec, hash, None)? {
            self.free_image(f, id);
            self.set_frame_garbaged(f);
        }
        Ok(())
    }

    pub fn share_image_cache(&mut self, f: Value) -> usize {
        let width = 10.max(self.frame(f).geometry.column_width);
        let terminal = self.frame(f).terminal;
        for &x in &self.frames.list {
            if self.frame(x).terminal == terminal
                && let Some(index) = self.frame(x).image_cache
                && self.images.caches[index]
                    .as_ref()
                    .is_some_and(|c| c.scaling_col_width == width)
            {
                return index;
            }
        }
        let cache = ImageCache {
            buckets: vec![Vec::new(); IMAGE_CACHE_BUCKETS_SIZE as usize],
            images: Vec::new(),
            refcount: 0,
            scaling_col_width: width,
        };
        if let Some(i) = self.images.caches.iter().position(Option::is_none) {
            self.images.caches[i] = Some(cache);
            i
        } else {
            self.images.caches.push(Some(cache));
            self.images.caches.len() - 1
        }
    }

    pub fn acquire_image_cache(&mut self, f: Value) {
        let index = self.share_image_cache(f);
        self.frame_mut(f).image_cache = Some(index);
        if let Some(c) = self.images.caches[index].as_mut() {
            c.refcount += 1;
        }
    }

    pub fn release_image_cache(&mut self, f: Value) {
        let Some(index) = self.frame(f).image_cache else {
            return;
        };
        let refcount = {
            let c = self.images.caches[index].as_mut().expect("image cache");
            c.refcount -= 1;
            c.refcount
        };
        if refcount == 0 {
            self.free_image_cache(f);
            self.images.caches[index] = None;
        }
        self.frame_mut(f).image_cache = None;
    }

    pub fn rescale_image_cache(&mut self, f: Value) -> LispResult<()> {
        let iwidth = 10.max(self.frame(f).geometry.column_width);
        let Some(index) = self.frame(f).image_cache else {
            return Ok(());
        };
        let (width, refcount) = {
            let c = self.images.caches[index].as_ref().expect("image cache");
            (c.scaling_col_width, c.refcount)
        };
        if iwidth != width {
            if refcount == 1 {
                self.images.caches[index]
                    .as_mut()
                    .expect("image cache")
                    .scaling_col_width = iwidth;
                self.clear_image_cache(f, sym::AUTO)?;
            } else {
                self.images.caches[index]
                    .as_mut()
                    .expect("image cache")
                    .refcount -= 1;
                self.acquire_image_cache(f);
            }
        }
        Ok(())
    }

    fn free_image_cache(&mut self, f: Value) {
        let used = self.image_cache(f).map_or(0, |c| c.images.len());
        for id in 0..used {
            self.free_image(f, id);
        }
    }

    pub fn clear_image_cache(&mut self, f: Value, filter: Value) -> LispResult<()> {
        let Some(index) = self.frame(f).image_cache else {
            return Ok(());
        };
        if self.frame(f).flags.inhibit_clear_image_cache {
            return Ok(());
        }
        let used = self.image_cache(f).map_or(0, |c| c.images.len());
        let mut nfreed = 0;
        if !filter.is_nil() {
            for id in 0..used {
                let Some(dependencies) = self.image(f, id).map(|img| img.dependencies) else {
                    continue;
                };
                if filter == sym::T || !self.member(filter, dependencies)?.is_nil() {
                    self.free_image(f, id);
                    nfreed += 1;
                }
            }
        } else if let Some(delay) = self
            .plain_value(sym::IMAGE_CACHE_EVICTION_DELAY)
            .as_fixnum()
        {
            let nimages = (0..used).filter(|&id| self.image(f, id).is_some()).count() as f64;
            let mut delay = delay as f64;
            if nimages > 40.0 {
                delay = 1600.0 * delay / nimages / nimages;
            }
            delay = delay.max(1.0);
            let old = timespec_sub(current_timespec(), dtotimespec(delay));
            for id in 0..used {
                if self.image(f, id).is_some_and(|img| img.timestamp < old) {
                    self.free_image(f, id);
                    nfreed += 1;
                }
            }
        }
        if nfreed > 0 {
            for fr in self.frames.list.clone() {
                if self.frame(fr).image_cache == Some(index) {
                    self.clear_current_matrices(fr);
                }
            }
            self.windows.windows_or_buffers_changed = 19;
        }
        Ok(())
    }

    pub fn clear_image_caches(&mut self, filter: Value) -> LispResult<()> {
        for f in self.frames.list.clone() {
            if self.frame_window_p(f) {
                self.clear_image_cache(f, filter)?;
            }
        }
        Ok(())
    }

    fn image_frame_cache_size(&self, f: Value) -> i64 {
        let Some(c) = self.image_cache(f) else {
            return 0;
        };
        c.images
            .iter()
            .flatten()
            .map(|img| {
                img.pixmap.as_ref().map_or(0, PixContainer::size_in_bytes)
                    + img.mask.as_ref().map_or(0, PixContainer::size_in_bytes)
            })
            .sum()
    }

    fn postprocess_image(&mut self, f: Value, img: &mut Image) -> LispResult<()> {
        if img.pixmap.is_none() {
            return Ok(());
        }
        let spec = img.spec;
        let (mask, _) = self.image_spec_value(spec, sym::K_HEURISTIC_MASK);
        if !mask.is_nil() {
            self.image_build_heuristic_mask(f, img, mask)?;
        } else {
            let (mask, found_p) = self.image_spec_value(spec, sym::K_MASK);
            if mask == sym::HEURISTIC {
                self.image_build_heuristic_mask(f, img, sym::T)?;
            } else if mask.is_cons() && self.xcar(mask) == sym::HEURISTIC {
                let rest = self.xcdr(mask);
                let how = if rest.is_cons() {
                    self.xcar(rest)
                } else {
                    rest
                };
                self.image_build_heuristic_mask(f, img, how)?;
            } else if mask.is_nil() && found_p && img.mask.is_some() {
                image_clear_image_1(img, CLEAR_IMAGE_MASK);
            }
        }
        let (conversion, _) = self.image_spec_value(spec, sym::K_CONVERSION);
        if conversion == sym::DISABLED {
            self.image_disable_image(f, img)?;
        } else if conversion == sym::LAPLACE {
            self.image_laplace(img);
        } else if conversion == sym::EMBOSS {
            self.image_emboss(img);
        } else if conversion.is_cons() && self.xcar(conversion) == sym::EDGE_DETECTION {
            let tem = self.xcdr(conversion);
            if tem.is_cons() {
                let matrix = self.plist_get(tem, sym::K_MATRIX);
                let adjust = self.plist_get(tem, sym::K_COLOR_ADJUSTMENT);
                self.image_edge_detection(img, matrix, adjust);
            }
        }
        Ok(())
    }

    fn image_get_dimension(&self, img: &Image, symbol: Value) -> i32 {
        let (value, _) = self.image_spec_value(img.spec, symbol);
        if let Some(n) = value.as_fixnum()
            && n >= 0
        {
            return n.min(i64::from(i32::MAX)) as i32;
        }
        if value.is_cons() && self.is_number(self.xcar(value)) {
            let dim = self.xcdr(value);
            let factor = self.xfloatint(self.xcar(value));
            if dim == sym::EM {
                return scale_image_size(f64::from(img.face_font_size), 1.0, factor);
            }
            if dim == sym::CH {
                return scale_image_size(f64::from(img.face_font_height), 1.0, factor);
            }
            if dim == sym::CW {
                return scale_image_size(f64::from(img.face_font_width), 1.0, factor);
            }
        }
        -1
    }

    fn image_compute_scale(
        &mut self,
        f: Value,
        spec: Value,
        img: Option<&mut Image>,
    ) -> LispResult<f64> {
        let mut scale = 1.0;
        let (value, _) = self.image_spec_value(spec, sym::K_SCALE);
        let mut auto_dependency = false;
        if value == sym::DEFAULT {
            let mut sval = self.plain_value(sym::IMAGE_SCALING_FACTOR);
            loop {
                if sval == sym::AUTO {
                    auto_dependency = true;
                    let column_width = self.frame(f).geometry.column_width;
                    scale = if column_width > 10 {
                        f64::from(column_width as f32 / 10.0f32)
                    } else {
                        1.0
                    };
                    break;
                } else if self.is_number(sval) {
                    scale = self.xfloatint(sval);
                    break;
                } else {
                    let factor = self.plain_value(sym::IMAGE_SCALING_FACTOR);
                    self.image_error("Invalid `image-scaling-factor': %s", &[factor])?;
                    sval = sym::AUTO;
                }
            }
        } else if self.is_number(value) {
            let dval = self.xfloatint(value);
            if dval >= 0.0 {
                scale = dval;
            }
        }
        if let Some(img) = img {
            if auto_dependency {
                img.dependencies = self.cons(sym::AUTO, img.dependencies);
            }
            img.scale = scale;
        }
        Ok(scale)
    }

    fn compute_image_size(
        &mut self,
        f: Value,
        width: f64,
        height: f64,
        img: &mut Image,
    ) -> LispResult<(i32, i32)> {
        let scale = self.image_compute_scale(f, img.spec, Some(img))?;
        let mut desired_width = self.image_get_dimension(img, sym::K_WIDTH);
        let max_width = if desired_width < 0 {
            self.image_get_dimension(img, sym::K_MAX_WIDTH)
        } else {
            desired_width = scale_image_size(f64::from(desired_width), 1.0, scale);
            -1
        };
        let mut desired_height = self.image_get_dimension(img, sym::K_HEIGHT);
        let max_height = if desired_height < 0 {
            self.image_get_dimension(img, sym::K_MAX_HEIGHT)
        } else {
            desired_height = scale_image_size(f64::from(desired_height), 1.0, scale);
            -1
        };
        if 0 <= desired_width && 0 <= desired_height {
            return Ok((desired_width, desired_height));
        }
        if 0 <= desired_width {
            desired_height = scale_image_size(f64::from(desired_width), width, height);
        } else if 0 <= desired_height {
            desired_width = scale_image_size(f64::from(desired_height), height, width);
        } else {
            desired_width = scale_image_size(width, 1.0, scale);
            desired_height = scale_image_size(height, 1.0, scale);
        }
        if 0 <= max_width && max_width < desired_width {
            desired_width = max_width;
            desired_height = scale_image_size(f64::from(desired_width), width, height);
        }
        if 0 <= max_height && max_height < desired_height {
            desired_height = max_height;
            desired_width = scale_image_size(f64::from(desired_height), height, width);
        }
        Ok((desired_width, desired_height))
    }

    fn compute_image_rotation(&mut self, img: &Image) -> LispResult<f64> {
        let (value, found) = self.image_spec_value(img.spec, sym::K_ROTATION);
        if !found {
            return Ok(0.0);
        }
        if !self.is_number(value) {
            self.image_error("Invalid image `:rotation' parameter", &[])?;
            return Ok(0.0);
        }
        let reduced = crate::data::f_mod(self, &[value, Value::fixnum(360)])?;
        Ok(self.xfloatint(reduced))
    }

    fn image_set_transform(&mut self, f: Value, img: &mut Image) -> LispResult<()> {
        let (mut width, mut height) = if self.image_spec_value(img.spec, sym::K_TYPE).0 == sym::SVG
        {
            (img.width, img.height)
        } else {
            self.compute_image_size(f, f64::from(img.width), f64::from(img.height), img)?
        };
        let rotation = self.compute_image_rotation(img)?;
        let flip = !self.image_spec_value(img.spec, sym::K_FLIP).0.is_nil();
        let (s, _) = self.image_spec_value(img.spec, sym::K_TRANSFORM_SMOOTHING);
        let smoothing = if s.is_nil() {
            width < img.width || height < img.height
        } else {
            true
        };
        let mut matrix: Matrix3x3 = [
            [f64::from(img.width) / f64::from(width), 0.0, 0.0],
            [0.0, f64::from(img.height) / f64::from(height), 0.0],
            [0.0, 0.0, 1.0],
        ];
        img.width = width;
        img.height = height;
        let mut rotate_flag = -1;
        if rotation == 0.0 && !flip {
            rotate_flag = 0;
        } else {
            let mut cos_r = 0.0;
            let mut sin_r = 0.0;
            if rotation == 0.0 {
                cos_r = 1.0;
                sin_r = 0.0;
                rotate_flag = 1;
            } else if rotation == 90.0 {
                width = img.height;
                height = img.width;
                cos_r = 0.0;
                sin_r = 1.0;
                rotate_flag = 1;
            } else if rotation == 180.0 {
                cos_r = -1.0;
                sin_r = 0.0;
                rotate_flag = 1;
            } else if rotation == 270.0 {
                width = img.height;
                height = img.width;
                cos_r = 0.0;
                sin_r = -1.0;
                rotate_flag = 1;
            }
            if 0 < rotate_flag {
                let mut t: Matrix3x3 = [
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [f64::from(img.width) * 0.5, f64::from(img.height) * 0.5, 1.0],
                ];
                let u = matrix3x3_mult(&t, &matrix);
                let rot: Matrix3x3 = [[cos_r, -sin_r, 0.0], [sin_r, cos_r, 0.0], [0.0, 0.0, 1.0]];
                let v = matrix3x3_mult(&rot, &u);
                t[2][0] = f64::from(width) * -0.5;
                t[2][1] = f64::from(height) * -0.5;
                if flip {
                    t[0][0] = -t[0][0];
                    t[2][0] = -t[2][0];
                }
                matrix = matrix3x3_mult(&t, &v);
                img.width = width;
                img.height = height;
            }
        }
        if rotate_flag < 0 {
            let angle = self.float(rotation);
            self.image_error("No native support for rotation by %g degrees", &[angle])?;
        }
        let filter = if img.type_.type_ == sym::CANVAS {
            if smoothing {
                CrFilter::Good
            } else {
                CrFilter::Nearest
            }
        } else if smoothing {
            CrFilter::Best
        } else {
            CrFilter::Nearest
        };
        img.cr_data = Some(CrData {
            matrix: [
                matrix[0][0],
                matrix[0][1],
                matrix[1][0],
                matrix[1][1],
                matrix[2][0],
                matrix[2][1],
            ],
            filter,
            surface: None,
        });
        Ok(())
    }

    pub fn lookup_image(&mut self, f: Value, spec: Value, face_id: i64) -> LispResult<usize> {
        let base = self.stack.len();
        self.stack.push(spec);
        let r = self.with_gc_deferred(|lisp| lisp.lookup_image_1(f, spec, face_id));
        self.stack.truncate(base);
        r
    }

    fn lookup_image_1(&mut self, f: Value, spec: Value, face_id: i64) -> LispResult<usize> {
        if self.frame(f).face_cache.is_none() {
            self.init_frame_faces(f)?;
        }
        let used = self.frame(f).face_cache.as_ref().map_or(0, |c| c.used());
        if used == 0 {
            self.recompute_basic_faces(f)?;
        }
        let used = self.frame(f).face_cache.as_ref().map_or(0, |c| c.used()) as i64;
        let face_id = if face_id < 0 || face_id >= used {
            crate::xfaces::DEFAULT_FACE_ID
        } else {
            face_id as usize
        };
        let (foreground, background, font, family) = {
            let face = self.face_from_id(f, face_id).expect("realized face");
            (
                face.foreground,
                face.background,
                face.font,
                face.lface[crate::xfaces::LFACE_FAMILY_INDEX],
            )
        };
        let (font_size, font_height, font_width) = {
            let font = self.font_object(font);
            (font.pixel_size, font.height, font.average_width)
        };
        let family = if family.is_string() {
            self.string_bytes(family).to_vec()
        } else {
            Vec::new()
        };
        let filtered = self.filter_image_spec(spec);
        let hash = self.sxhash_equal(filtered, 0);
        let mut found = self.search_image_cache(
            f,
            spec,
            hash,
            Some((foreground, background, font_size, family.as_slice())),
        )?;
        if let Some(id) = found
            && self.image(f, id).is_some_and(|img| img.load_failed_p)
        {
            self.free_image(f, id);
            found = None;
        }
        let id = match found {
            Some(id) => id,
            None => {
                let mut img = self.make_image(spec, hash);
                let id = self.cache_image(f, hash);
                img.id = id;
                img.face_foreground = foreground;
                img.face_background = background;
                img.face_font_size = font_size;
                img.face_font_height = font_height;
                img.face_font_width = font_width;
                img.face_font_family = family;
                let r = self.load_new_image(f, spec, &mut img);
                self.put_image(f, img);
                r?;
                id
            }
        };
        if let Some(img) = self.image_mut(f, id) {
            img.timestamp = current_timespec();
        }
        Ok(id)
    }

    fn load_new_image(&mut self, f: Value, spec: Value, img: &mut Image) -> LispResult<()> {
        let load = img.type_.load_img;
        img.load_failed_p = !load(self, f, img)?;
        if img.load_failed_p {
            let (value, _) = self.image_spec_value(spec, sym::K_WIDTH);
            img.width = value.as_fixnum().map_or(DEFAULT_IMAGE_WIDTH, |n| n as i32);
            let (value, _) = self.image_spec_value(spec, sym::K_HEIGHT);
            img.height = value.as_fixnum().map_or(DEFAULT_IMAGE_HEIGHT, |n| n as i32);
            return Ok(());
        }
        let (ascent, _) = self.image_spec_value(spec, sym::K_ASCENT);
        if let Some(n) = ascent.as_fixnum() {
            img.ascent = n as i32;
        } else if ascent == sym::CENTER {
            img.ascent = CENTERED_IMAGE_ASCENT;
        }
        let (margin, _) = self.image_spec_value(spec, sym::K_MARGIN);
        if let Some(n) = margin.as_fixnum() {
            img.vmargin = n as i32;
            img.hmargin = n as i32;
        } else if margin.is_cons() {
            img.hmargin = self.xcar(margin).as_fixnum().unwrap_or(0) as i32;
            img.vmargin = self.xcdr(margin).as_fixnum().unwrap_or(0) as i32;
        }
        let (relief, _) = self.image_spec_value(spec, sym::K_RELIEF);
        let relief_bound = i64::from(i32::MAX - img.hmargin.max(img.vmargin));
        if ranged_fixnum(relief, -relief_bound, relief_bound) {
            img.relief = relief.as_fixnum().unwrap_or(0) as i32;
            img.hmargin += img.relief.abs();
            img.vmargin += img.relief.abs();
        }
        if !img.background_valid {
            let (bg, _) = self.image_spec_value(img.spec, sym::K_BACKGROUND);
            if !bg.is_nil() {
                let face_background = img.face_background;
                img.background = self.image_alloc_image_color(f, img, bg, face_background)?;
                img.background_valid = true;
            }
        }
        img.original_width = img.width;
        img.original_height = img.height;
        if img.type_.type_ != sym::POSTSCRIPT {
            self.postprocess_image(f, img)?;
        }
        self.image_set_transform(f, img)
    }

    fn cache_image(&mut self, f: Value, hash: u64) -> usize {
        if self.frame(f).image_cache.is_none() {
            self.acquire_image_cache(f);
        }
        let c = self.image_cache_mut(f).expect("image cache");
        let i = match c.images.iter().position(Option::is_none) {
            Some(i) => i,
            None => {
                c.images.push(None);
                c.images.len() - 1
            }
        };
        let bucket = (hash % IMAGE_CACHE_BUCKETS_SIZE) as usize;
        c.buckets[bucket].insert(0, i);
        i
    }

    pub(crate) fn image_find_image_file(&mut self, file: Value) -> LispResult<Value> {
        let images = self.string("images");
        let data_directory = self.plain_value(sym::DATA_DIRECTORY);
        let dir = crate::fileio::f_expand_file_name(self, &[images, data_directory])?;
        let bitmap_path = self.plain_value(sym::X_BITMAP_FILE_PATH);
        let search_path = self.cons(dir, bitmap_path);
        let base = self.stack.len();
        self.stack.push(search_path);
        let r = self.openp(
            search_path,
            file,
            Value::NIL,
            Value::fixnum(4),
            false,
            false,
        );
        self.stack.truncate(base);
        let (fd, found) = r?;
        Ok(match fd {
            Ok(_) => found,
            Err(_) => Value::NIL,
        })
    }

    pub(crate) fn slurp_image(
        &mut self,
        filename: Value,
        image_type: &str,
    ) -> LispResult<Option<Vec<u8>>> {
        let file = self.image_find_image_file(filename)?;
        if !file.is_string() {
            self.image_error("Cannot find image file `%s'", &[filename])?;
            return Ok(None);
        }
        let path = self.string_to_rust(file);
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(_) => {
                let kind = self.unibyte_string(image_type.as_bytes());
                self.image_error("Error loading %s image `%s'", &[kind, file])?;
                Ok(None)
            }
        }
    }
}

fn cr_put_image_to_cr_data(img: &mut Image) {
    let Some(pimg) = img.pixmap.as_mut() else {
        img.cr_data = None;
        return;
    };
    let has_mask = img.mask.is_some();
    if let Some(mask) = img.mask.as_mut() {
        for y in 0..pimg.height {
            for x in 0..pimg.width {
                let color = pimg.get_pixel(x, y);
                let alpha = mask.get_pixel(x, y);
                let r = (red_from_ulong(color) * alpha + 0x7f) / 0xff;
                let g = (green_from_ulong(color) * alpha + 0x7f) / 0xff;
                let b = (blue_from_ulong(color) * alpha + 0x7f) / 0xff;
                pimg.put_pixel(x, y, (alpha << 24) | (r << 16) | (g << 8) | b);
            }
        }
        mask.data = Vec::new();
    }
    let surface = CrSurface {
        width: pimg.width,
        height: pimg.height,
        stride: pimg.bytes_per_line,
        format: if has_mask {
            CrFormat::Argb32
        } else {
            CrFormat::Rgb24
        },
        data: std::mem::take(&mut pimg.data),
    };
    let previous = img.cr_data.take().unwrap_or_default();
    img.cr_data = Some(CrData {
        matrix: previous.matrix,
        filter: previous.filter,
        surface: Some(surface),
    });
}

pub static SUBRS: &[Subr] = &[
    Subr::fixed("imagep", 1, 1, f_imagep),
    Subr::fixed("image-size", 1, 3, f_image_size),
    Subr::fixed("image-mask-p", 1, 2, f_image_mask_p),
    Subr::fixed("image-metadata", 1, 2, f_image_metadata),
    Subr::fixed("clear-image-cache", 0, 2, f_clear_image_cache),
    Subr::fixed("image-flush", 1, 2, f_image_flush),
    Subr::fixed("image-transforms-p", 0, 1, f_image_transforms_p),
    Subr::fixed("image-cache-size", 0, 0, f_image_cache_size),
    Subr::fixed("init-image-library", 1, 1, f_init_image_library),
];

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn f_imagep(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let valid = lisp.valid_image_p(args[0]);
    Ok(lisp.bool(valid))
}

fn f_image_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (spec, pixels, frame) = (args[0], arg(args, 1), arg(args, 2));
    if !lisp.valid_image_p(spec) {
        return Err(lisp.error("Invalid image specification"));
    }
    let f = lisp.decode_window_system_frame(frame)?;
    let id = lisp.lookup_image(f, spec, -1)?;
    let img = lisp.image(f, id).expect("cached image");
    let width = img.width + 2 * img.hmargin;
    let height = img.height + 2 * img.vmargin;
    if pixels.is_nil() {
        let g = &lisp.frame(f).geometry;
        let (cw, lh) = (g.column_width as f64, g.line_height as f64);
        let w = lisp.float(f64::from(width) / cw);
        let h = lisp.float(f64::from(height) / lh);
        Ok(lisp.cons(w, h))
    } else {
        Ok(lisp.cons(
            Value::fixnum(i64::from(width)),
            Value::fixnum(i64::from(height)),
        ))
    }
}

fn f_image_mask_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (spec, frame) = (args[0], arg(args, 1));
    if !lisp.valid_image_p(spec) {
        return Err(lisp.error("Invalid image specification"));
    }
    let f = lisp.decode_window_system_frame(frame)?;
    let id = lisp.lookup_image(f, spec, -1)?;
    let mask = lisp.image(f, id).is_some_and(|img| img.mask.is_some());
    Ok(lisp.bool(mask))
}

fn f_image_metadata(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (spec, frame) = (args[0], arg(args, 1));
    if !lisp.valid_image_p(spec) {
        return Ok(Value::NIL);
    }
    let f = lisp.decode_window_system_frame(frame)?;
    let id = lisp.lookup_image(f, spec, -1)?;
    Ok(lisp.image(f, id).map_or(Value::NIL, |img| img.lisp_data))
}

fn f_clear_image_cache(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (filter, animation_filter) = (arg(args, 0), arg(args, 1));
    if !animation_filter.is_nil() {
        if !animation_filter.is_cons() {
            return Err(lisp.wrong_type(sym::CONSP, animation_filter));
        }
        let plist = lisp.xcdr(animation_filter);
        lisp.anim_prune_animation_cache(plist);
        return Ok(Value::NIL);
    }
    if !(filter.is_nil() || lisp.is_frame(filter)) {
        lisp.clear_image_caches(filter)?;
    } else {
        let f = lisp.decode_window_system_frame(filter)?;
        lisp.clear_image_cache(f, sym::T)?;
    }
    lisp.image_prune_animation_caches(true);
    Ok(Value::NIL)
}

fn f_image_flush(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (spec, frame) = (args[0], arg(args, 1));
    if !lisp.valid_image_p(spec) {
        return Err(lisp.error("Invalid image specification"));
    }
    if frame == sym::T {
        for f in lisp.frames.list.clone() {
            if lisp.frame_window_p(f) {
                lisp.uncache_image(f, spec)?;
            }
        }
    } else {
        let f = lisp.decode_window_system_frame(frame)?;
        lisp.uncache_image(f, spec)?;
    }
    Ok(Value::NIL)
}

fn f_image_transforms_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    if lisp.frame_window_p(f) {
        return Ok(lisp.list(&[sym::SCALE, sym::ROTATE90]));
    }
    Ok(Value::NIL)
}

fn f_image_cache_size(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let mut total = 0;
    for f in lisp.frames.list.clone() {
        if lisp.frame_window_p(f) {
            total += lisp.image_frame_cache_size(f);
        }
    }
    total += lisp.images.anim.iter().map(|c| c.byte_size).sum::<i64>();
    Ok(Value::fixnum(total))
}

fn f_init_image_library(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lookup_image_type(args[0]).is_some()))
}

pub fn init_vars(lisp: &mut Lisp) {
    if cfg!(windows) {
        lisp.set_plain_value(sym::LIBPNG_VERSION, Value::fixnum(-1));
        lisp.set_plain_value(sym::LIBGIF_VERSION, Value::fixnum(-1));
        lisp.set_plain_value(sym::LIBJPEG_VERSION, Value::fixnum(-1));
    }
    let mut image_types = Value::NIL;
    for type_ in [
        sym::PBM,
        sym::XBM,
        sym::XPM,
        sym::JPEG,
        sym::TIFF,
        sym::GIF,
        sym::PNG,
        sym::WEBP,
        sym::SVG,
    ] {
        image_types = lisp.cons(type_, image_types);
    }
    lisp.defvar(sym::IMAGE_TYPES, image_types);
    let max_image_size = lisp.float(MAX_IMAGE_SIZE);
    lisp.defvar(sym::MAX_IMAGE_SIZE, max_image_size);
    lisp.defvar_bool(sym::CROSS_DISABLED_IMAGES, false);
    let bitmaps = lisp.unibyte_string(PATH_BITMAPS.as_bytes());
    let bitmap_path = lisp.list(&[bitmaps]);
    lisp.defvar(sym::X_BITMAP_FILE_PATH, bitmap_path);
    lisp.defvar(sym::IMAGE_CACHE_EVICTION_DELAY, Value::fixnum(300));
    lisp.defvar(sym::IMAGE_SCALING_FACTOR, sym::AUTO);
}
