use super::{BOT_CORNER, LEFT_CORNER, RIGHT_CORNER, TOP_CORNER};
use super::{Image, ImageKeyword, ImageValueType, PixContainer, lookup_rgb_color, query_color};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const GIF_DATA: usize = 1;
const GIF_FILE: usize = 2;
const GIF_LOCAL_DESCRIPTOR_EXTENSION: i32 = 249;
const CONTINUE_EXT_FUNC_CODE: i32 = 0;
const GRAPHICS_EXT_FUNC_CODE: i32 = 0xf9;
const DISPOSAL_UNSPECIFIED: i32 = 0;
const DISPOSE_DO_NOT: i32 = 1;
const DISPOSE_BACKGROUND: i32 = 2;
const DISPOSE_PREVIOUS: i32 = 3;
const NO_TRANSPARENT_COLOR: i32 = -1;
const D_GIF_ERR_OPEN_FAILED: i32 = 101;
const D_GIF_ERR_READ_FAILED: i32 = 102;
const D_GIF_ERR_NOT_GIF_FILE: i32 = 103;
const D_GIF_ERR_NO_IMAG_DSCR: i32 = 105;
const D_GIF_ERR_WRONG_RECORD: i32 = 107;
const D_GIF_ERR_IMAGE_DEFECT: i32 = 112;
const D_GIF_ERR_EOF_TOO_SOON: i32 = 113;
const INTERLACED_OFFSET: [i32; 4] = [0, 4, 2, 1];
const INTERLACED_JUMPS: [i32; 4] = [8, 8, 4, 2];

fn gif_format() -> [ImageKeyword; 11] {
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

pub(super) fn gif_image_p(lisp: &mut Lisp, object: Value) -> bool {
    let mut fmt = gif_format();
    if !lisp.parse_image_spec(object, &mut fmt, sym::GIF) {
        return false;
    }
    usize::from(fmt[GIF_FILE].count) + usize::from(fmt[GIF_DATA].count) == 1
}

fn gif_error_string(code: i32) -> Option<&'static str> {
    Some(match code {
        101 => "Failed to open given file",
        102 => "Failed to read from given file",
        103 => "Data is not in GIF format",
        104 => "No screen descriptor detected",
        105 => "No Image Descriptor detected",
        106 => "Neither global nor local color map",
        107 => "Wrong record type detected",
        108 => "Number of pixels bigger than width * height",
        109 => "Failed to allocate required memory",
        110 => "Failed to close given file",
        111 => "Given file was not opened for read",
        112 => "Image is defective, decoding aborted",
        113 => "Image EOF detected before image complete",
        _ => return None,
    })
}

pub(crate) struct ExtensionBlock {
    function: i32,
    bytes: Vec<u8>,
}

pub(crate) struct SavedImage {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    color_map: Option<Vec<[u8; 3]>>,
    raster: Vec<u8>,
    extensions: Vec<ExtensionBlock>,
}

pub(crate) struct GifFile {
    swidth: i32,
    sheight: i32,
    scolor_map: Option<Vec<[u8; 3]>>,
    images: Vec<SavedImage>,
    pending: Vec<ExtensionBlock>,
    error: i32,
    contents: Vec<u8>,
    pos: usize,
}

#[derive(Default)]
pub(crate) struct GifAnimHandle {
    gif: Option<Box<GifFile>>,
    pixmap: Option<Vec<u64>>,
}

struct GifRead;

impl GifFile {
    fn read(&mut self, n: usize) -> Result<&[u8], GifRead> {
        if n > self.contents.len() - self.pos {
            self.error = D_GIF_ERR_READ_FAILED;
            return Err(GifRead);
        }
        let start = self.pos;
        self.pos += n;
        Ok(&self.contents[start..start + n])
    }

    fn word(&mut self) -> Result<i32, GifRead> {
        let b = self.read(2)?;
        Ok(i32::from(u16::from_le_bytes([b[0], b[1]])))
    }

    fn color_map(&mut self, bits: u32) -> Result<Vec<[u8; 3]>, GifRead> {
        let count = 1usize << bits;
        let bytes = self.read(count * 3)?;
        Ok(bytes.chunks(3).map(|c| [c[0], c[1], c[2]]).collect())
    }

    fn open(contents: Vec<u8>) -> Result<Box<GifFile>, i32> {
        let mut gif = Box::new(GifFile {
            swidth: 0,
            sheight: 0,
            scolor_map: None,
            images: Vec::new(),
            pending: Vec::new(),
            error: 0,
            contents,
            pos: 0,
        });
        let stamp = gif.read(6).map_err(|_| D_GIF_ERR_READ_FAILED)?;
        if &stamp[..3] != b"GIF" {
            return Err(D_GIF_ERR_NOT_GIF_FILE);
        }
        gif.screen_desc().map_err(|_| gif.error)?;
        Ok(gif)
    }

    fn screen_desc(&mut self) -> Result<(), GifRead> {
        self.swidth = self.word()?;
        self.sheight = self.word()?;
        let buf = self.read(3)?;
        let flags = buf[0];
        if flags & 0x80 != 0 {
            let bits = u32::from(flags & 0x07) + 1;
            self.scolor_map = Some(self.color_map(bits)?);
        }
        Ok(())
    }

    fn sub_block(&mut self) -> Result<Option<Vec<u8>>, GifRead> {
        let len = usize::from(self.read(1)?[0]);
        if len == 0 {
            return Ok(None);
        }
        Ok(Some(self.read(len)?.to_vec()))
    }

    fn image_desc(&mut self) -> Result<(u8, u8), GifRead> {
        let left = self.word()?;
        let top = self.word()?;
        let width = self.word()?;
        let height = self.word()?;
        let flags = self.read(1)?[0];
        let color_map = if flags & 0x80 != 0 {
            Some(self.color_map(u32::from(flags & 0x07) + 1)?)
        } else {
            None
        };
        self.images.push(SavedImage {
            left,
            top,
            width,
            height,
            color_map,
            raster: Vec::new(),
            extensions: Vec::new(),
        });
        let code_size = self.read(1)?[0];
        if code_size > 8 {
            self.error = D_GIF_ERR_READ_FAILED;
            return Err(GifRead);
        }
        Ok((flags, code_size))
    }

    fn raster(&mut self, code_size: u8, pixels: usize) -> Result<Vec<u8>, GifRead> {
        let mut data = Vec::new();
        while let Some(block) = self.sub_block()? {
            data.extend_from_slice(&block);
        }
        let mut out = vec![0u8; pixels];
        let mut decoder = weezl::decode::Decoder::new(weezl::BitOrder::Lsb, code_size);
        let (mut inp, mut filled) = (0, 0);
        while filled < pixels {
            let r = decoder.decode_bytes(&data[inp..], &mut out[filled..]);
            inp += r.consumed_in;
            filled += r.consumed_out;
            match r.status {
                Ok(weezl::LzwStatus::Ok) => {}
                Ok(weezl::LzwStatus::Done) if filled < pixels => {
                    self.error = D_GIF_ERR_EOF_TOO_SOON;
                    return Err(GifRead);
                }
                Ok(weezl::LzwStatus::Done) => {}
                Ok(weezl::LzwStatus::NoProgress) | Err(_) => {
                    self.error = D_GIF_ERR_IMAGE_DEFECT;
                    return Err(GifRead);
                }
            }
        }
        Ok(out)
    }

    fn slurp(&mut self) -> Result<(), GifRead> {
        loop {
            let record = self.read(1)?[0];
            match record {
                b',' => {
                    let (flags, code_size) = self.image_desc()?;
                    let sp = self.images.last().expect("saved image");
                    let (width, height) = (sp.width, sp.height);
                    if width <= 0 || height <= 0 || width > i32::MAX / height {
                        return Err(GifRead);
                    }
                    let pixels = (width * height) as usize;
                    let decoded = self.raster(code_size, pixels)?;
                    let raster = if flags & 0x40 != 0 {
                        let mut raster = vec![0u8; pixels];
                        let mut line = 0usize;
                        for pass in 0..4 {
                            let mut j = INTERLACED_OFFSET[pass];
                            while j < height {
                                let w = width as usize;
                                let row = j as usize * w;
                                raster[row..row + w]
                                    .copy_from_slice(&decoded[line * w..line * w + w]);
                                line += 1;
                                j += INTERLACED_JUMPS[pass];
                            }
                        }
                        raster
                    } else {
                        decoded
                    };
                    let pending = std::mem::take(&mut self.pending);
                    let sp = self.images.last_mut().expect("saved image");
                    sp.raster = raster;
                    sp.extensions = pending;
                }
                b'!' => {
                    let function = i32::from(self.read(1)?[0]);
                    if let Some(bytes) = self.sub_block()? {
                        self.pending.push(ExtensionBlock { function, bytes });
                    }
                    while let Some(bytes) = self.sub_block()? {
                        self.pending.push(ExtensionBlock {
                            function: CONTINUE_EXT_FUNC_CODE,
                            bytes,
                        });
                    }
                }
                b';' => break,
                _ => {
                    self.error = D_GIF_ERR_WRONG_RECORD;
                    return Err(GifRead);
                }
            }
        }
        if self.images.is_empty() {
            self.error = D_GIF_ERR_NO_IMAG_DSCR;
            return Err(GifRead);
        }
        Ok(())
    }

    fn graphics_control(&self, index: usize) -> (i32, i32) {
        let defaults = (DISPOSAL_UNSPECIFIED, NO_TRANSPARENT_COLOR);
        let Some(ext) = self.images[index]
            .extensions
            .iter()
            .find(|e| e.function == GRAPHICS_EXT_FUNC_CODE)
        else {
            return defaults;
        };
        if ext.bytes.len() != 4 {
            return defaults;
        }
        let flags = ext.bytes[0];
        let disposal = i32::from((flags >> 2) & 0x07);
        let transparent = if flags & 0x01 != 0 {
            i32::from(ext.bytes[3])
        } else {
            NO_TRANSPARENT_COLOR
        };
        (disposal, transparent)
    }
}

fn gif_open_error(lisp: &mut Lisp, format: &str, object: Value, code: i32) -> LispResult<bool> {
    match gif_error_string(code) {
        Some(text) => {
            let text = lisp.string(text);
            lisp.image_error(&format!("{format}: %s"), &[object, text])?;
        }
        None => lisp.image_error(format, &[object])?,
    }
    Ok(false)
}

pub(super) fn gif_load(lisp: &mut Lisp, f: Value, img: &mut Image) -> LispResult<bool> {
    let (specified_bg, _) = lisp.image_spec_value(img.spec, sym::K_BACKGROUND);
    let (specified_file, _) = lisp.image_spec_value(img.spec, sym::K_FILE);
    let (specified_data, _) = lisp.image_spec_value(img.spec, sym::K_DATA);
    let (image_number, _) = lisp.image_spec_value(img.spec, sym::K_INDEX);
    let idx = image_number.as_fixnum().unwrap_or(0);
    let cache = (!image_number.is_nil()).then(|| {
        let plist = lisp.xcdr(img.spec);
        lisp.anim_get_animation_cache(plist)
    });
    let mut gif = None;
    let mut pixmap = None;
    if let Some(c) = cache {
        let entry = &mut lisp.images.anim[c];
        if entry.gif.gif.is_some() {
            gif = entry.gif.gif.take();
            pixmap = entry.gif.pixmap.take();
            if i64::from(entry.index) != idx - 1 {
                entry.index = -1;
            }
        }
    }
    let mut byte_size = 0;
    let reused = gif.is_some();
    let (width, height);
    let gif = match gif {
        Some(gif) => {
            width = gif.swidth;
            height = gif.sheight;
            img.width = width;
            img.height = height;
            gif
        }
        None => {
            let mut gif = if specified_data.is_nil() {
                let file = lisp.image_find_image_file(specified_file)?;
                if !file.is_string() {
                    lisp.image_error("Cannot find image file `%s'", &[specified_file])?;
                    return Ok(false);
                }
                let path = lisp.string_to_rust(file);
                let contents = match std::fs::read(&path) {
                    Ok(contents) => contents,
                    Err(_) => {
                        return gif_open_error(
                            lisp,
                            "Cannot open `%s'",
                            file,
                            D_GIF_ERR_OPEN_FAILED,
                        );
                    }
                };
                byte_size = contents.len() as i64;
                match GifFile::open(contents) {
                    Ok(gif) => gif,
                    Err(code) => return gif_open_error(lisp, "Cannot open `%s'", file, code),
                }
            } else {
                if !specified_data.is_string() {
                    lisp.image_error("Invalid image data `%s'", &[specified_data])?;
                    return Ok(false);
                }
                let contents = lisp.string_bytes(specified_data).to_vec();
                byte_size = contents.len() as i64;
                match GifFile::open(contents) {
                    Ok(gif) => gif,
                    Err(code) => {
                        return gif_open_error(
                            lisp,
                            "Cannot open memory source `%s'",
                            img.spec,
                            code,
                        );
                    }
                }
            };
            if !lisp.check_image_size(Some(f), gif.swidth, gif.sheight) {
                lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
                return gif_failure(lisp, cache);
            }
            if gif.slurp().is_err() || gif.images.is_empty() {
                let text = gif_error_string(gif.error);
                match (text, specified_data.is_nil()) {
                    (Some(text), true) => {
                        let text = lisp.string(text);
                        lisp.image_error("Error reading `%s' (%s)", &[img.spec, text])?;
                    }
                    (Some(text), false) => {
                        let text = lisp.string(text);
                        lisp.image_error("Error reading GIF data: %s", &[text])?;
                    }
                    (None, true) => lisp.image_error("Error reading `%s'", &[img.spec])?,
                    (None, false) => lisp.image_error("Error reading GIF data", &[])?,
                }
                return gif_failure(lisp, cache);
            }
            width = gif.swidth;
            height = gif.sheight;
            img.width = width;
            img.height = height;
            let fits = gif.images.iter().all(|s| {
                s.width >= 0
                    && s.height >= 0
                    && s.top >= 0
                    && s.left >= 0
                    && s.top + s.height <= height
                    && s.left + s.width <= width
            });
            if !fits {
                lisp.image_error("Subimage does not fit in image", &[])?;
                return gif_failure(lisp, cache);
            }
            gif
        }
    };
    if idx < 0 || idx >= gif.images.len() as i64 {
        lisp.image_error(
            "Invalid image number `%s' in image `%s'",
            &[Value::fixnum(idx), img.spec],
        )?;
        return gif_failure(lisp, cache);
    }
    let idx = idx as usize;
    if let Some(c) = cache
        && !reused
    {
        lisp.images.anim[c].byte_size = byte_size;
    }
    let first = &gif.images[0];
    img.corners[TOP_CORNER] = first.top;
    img.corners[LEFT_CORNER] = first.left;
    img.corners[BOT_CORNER] = first.top + first.height;
    img.corners[RIGHT_CORNER] = first.left + first.width;
    if !lisp.check_image_size(Some(f), width, height) {
        lisp.image_error("Invalid image size (see `max-image-size')", &[])?;
        return gif_failure(lisp, cache);
    }
    let mut ximg = PixContainer::new(width, height, 0);
    let (w, h) = (width as usize, height as usize);
    let mut pixmap = pixmap.unwrap_or_else(|| vec![0u64; w * h]);
    let (r, g, b) = query_color(lisp.frame(f).background_pixel as u64);
    let frame_bg = lookup_rgb_color(r, g, b);
    let corners = img.corners;
    for y in 0..h {
        for x in 0..w {
            let (xi, yi) = (x as i32, y as i32);
            if yi < corners[TOP_CORNER]
                || yi >= corners[BOT_CORNER]
                || xi < corners[LEFT_CORNER]
                || xi >= corners[RIGHT_CORNER]
            {
                pixmap[x + y * w] = frame_bg;
            }
        }
    }
    let mut bgcolor = 0;
    if specified_bg.is_string() {
        let fallback = lisp.frame(f).background_pixel as u64;
        let pixel = lisp.image_alloc_image_color(f, img, specified_bg, fallback)?;
        let (r, g, b) = query_color(pixel);
        bgcolor = lookup_rgb_color(r, g, b);
    }
    let mut start_frame = 0;
    if let Some(c) = cache {
        let entry = &mut lisp.images.anim[c];
        start_frame = (entry.index + 1) as usize;
        if start_frame > idx {
            start_frame = 0;
        }
        entry.index = idx as i32;
    }
    for j in start_frame..=idx {
        let (mut disposal, transparency_color_index) = gif.graphics_control(j);
        if j == 0 {
            disposal = DISPOSE_BACKGROUND;
        }
        if disposal == DISPOSAL_UNSPECIFIED || disposal == DISPOSE_PREVIOUS {
            disposal = DISPOSE_DO_NOT;
        }
        let sub = &gif.images[j];
        let color_map = sub.color_map.as_ref().or(gif.scolor_map.as_ref());
        let mut pixel_colors = [0u64; 256];
        if let Some(map) = color_map {
            for (i, color) in map.iter().enumerate().take(256) {
                pixel_colors[i] = if transparency_color_index == i as i32 {
                    if specified_bg.is_string() {
                        bgcolor
                    } else {
                        frame_bg
                    }
                } else {
                    lookup_rgb_color(
                        i32::from(color[0]) << 8,
                        i32::from(color[1]) << 8,
                        i32::from(color[2]) << 8,
                    )
                };
            }
        }
        for y in 0..sub.height {
            for x in 0..sub.width {
                let c = sub.raster[(y * sub.width + x) as usize];
                if transparency_color_index != i32::from(c) || disposal != DISPOSE_DO_NOT {
                    let at = (x + sub.left) as usize + (y + sub.top) as usize * w;
                    pixmap[at] = pixel_colors[usize::from(c)];
                }
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            ximg.put_pixel(x as i32, y as i32, pixmap[x + y * w]);
        }
    }
    img.lisp_data = Value::NIL;
    let extensions = &gif.images[idx].extensions;
    if !extensions.is_empty() {
        let mut delay = 0;
        let mut data = Value::NIL;
        for ext in extensions {
            let bytes = lisp.unibyte_string(&ext.bytes);
            data = lisp.cons(bytes, data);
            data = lisp.cons(Value::fixnum(i64::from(ext.function)), data);
            if ext.function == GIF_LOCAL_DESCRIPTOR_EXTENSION && ext.bytes.len() == 4 {
                delay = (i32::from(ext.bytes[2]) << 8) | i32::from(ext.bytes[1]);
            }
        }
        let mut lisp_data = lisp.list(&[sym::EXTENSION_DATA, data]);
        let delay = if delay != 0 {
            lisp.float(f64::from(delay) / 100.0)
        } else {
            sym::T
        };
        lisp_data = lisp.cons(delay, lisp_data);
        img.lisp_data = lisp.cons(sym::DELAY, lisp_data);
    }
    if gif.images.len() > 1 {
        let count = Value::fixnum(gif.images.len() as i64);
        let rest = lisp.cons(count, img.lisp_data);
        img.lisp_data = lisp.cons(sym::COUNT, rest);
    }
    if let Some(c) = cache {
        let entry = &mut lisp.images.anim[c];
        entry.gif.gif = Some(gif);
        entry.gif.pixmap = Some(pixmap);
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

fn gif_failure(lisp: &mut Lisp, cache: Option<usize>) -> LispResult<bool> {
    if let Some(c) = cache {
        lisp.images.anim[c].gif = GifAnimHandle::default();
    }
    Ok(false)
}
