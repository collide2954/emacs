use std::path::{Path, PathBuf};
use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_core::value::Value;
use emacs_gui::display::Display;
use iced::futures::channel::mpsc as fmpsc;

fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(512 << 20)
            .spawn_scoped(scope, f)
            .expect("spawn test thread")
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

fn eval(lisp: &mut Lisp, source: &str) -> String {
    match lisp.eval_string(source) {
        Ok(value) => lisp.prin1_to_string(value),
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            format!("error: {}", lisp.prin1_to_string(condition))
        }
    }
}

fn frame() -> (Lisp, Value) {
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(
        &mut lisp,
        Display::new(fmpsc::unbounded().0, mpsc::channel().1),
    );
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    assert_eq!(
        eval(
            &mut lisp,
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"i\") (width . 30) (height . 8) (foreground-color . \"black\") (background-color . \"white\")))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    (lisp, f)
}

fn messages(lisp: &mut Lisp) -> String {
    eval(
        lisp,
        "(save-current-buffer (set-buffer \"*Messages*\") (buffer-string))",
    )
}

struct Png<'a> {
    width: u32,
    height: u32,
    color: png::ColorType,
    depth: png::BitDepth,
    palette: Option<&'a [u8]>,
    trns: Option<&'a [u8]>,
    bkgd: Option<&'a [u8]>,
    data: &'a [u8],
}

impl Png<'_> {
    fn rgb(width: u32, height: u32, data: &[u8]) -> Png<'_> {
        Png {
            width,
            height,
            color: png::ColorType::Rgb,
            depth: png::BitDepth::Eight,
            palette: None,
            trns: None,
            bkgd: None,
            data,
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut e = png::Encoder::new(&mut out, self.width, self.height);
        e.set_color(self.color);
        e.set_depth(self.depth);
        if let Some(palette) = self.palette {
            e.set_palette(palette.to_vec());
        }
        if let Some(trns) = self.trns {
            e.set_trns(trns.to_vec());
        }
        let mut w = e.write_header().unwrap();
        if let Some(bkgd) = self.bkgd {
            w.write_chunk(png::chunk::bKGD, bkgd).unwrap();
        }
        w.write_image_data(self.data).unwrap();
        w.finish().unwrap();
        out
    }
}

struct Images {
    lisp: Lisp,
    f: Value,
    dir: PathBuf,
}

impl Images {
    fn new() -> Images {
        let (lisp, f) = frame();
        let dir = std::env::temp_dir().join(format!(
            "emacs-image-formats-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Images { lisp, f, dir }
    }

    fn file(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        lisp_path(&path)
    }

    fn data_spec(&mut self, kind: Value, bytes: &[u8], extra: &[Value]) -> Value {
        let data = self.lisp.unibyte_string(bytes);
        let mut items = vec![sym::IMAGE, sym::K_TYPE, kind, sym::K_DATA, data];
        items.extend_from_slice(extra);
        let spec = self.lisp.list(&items);
        self.lisp.stack.push(spec);
        spec
    }

    fn file_spec(&mut self, kind: &str, path: &str, extra: &str) -> Value {
        let spec = self
            .lisp
            .eval_string(&format!("'(image :type {kind} :file \"{path}\" {extra})"))
            .unwrap();
        self.lisp.stack.push(spec);
        spec
    }

    fn pixels(&mut self, spec: Value) -> Vec<u64> {
        let id = self.lisp.lookup_image(self.f, spec, -1).unwrap();
        let img = self.lisp.image(self.f, id).unwrap();
        let pixmap = img.pixmap.as_ref().expect("loaded pixmap");
        (0..img.height)
            .flat_map(|y| (0..img.width).map(move |x| (x, y)))
            .map(|(x, y)| pixmap.get_pixel(x, y))
            .collect()
    }

    fn mask(&mut self, spec: Value) -> Option<Vec<u64>> {
        let id = self.lisp.lookup_image(self.f, spec, -1).unwrap();
        let img = self.lisp.image(self.f, id).unwrap();
        let mask = img.mask.as_ref()?;
        Some(
            (0..img.height)
                .flat_map(|y| (0..img.width).map(move |x| (x, y)))
                .map(|(x, y)| mask.get_pixel(x, y))
                .collect(),
        )
    }

    fn background(&mut self, spec: Value) -> (bool, u64) {
        let id = self.lisp.lookup_image(self.f, spec, -1).unwrap();
        let img = self.lisp.image(self.f, id).unwrap();
        (img.background_valid, img.background)
    }
}

impl Drop for Images {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn lisp_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[test]
fn png_pixels_load_from_files_and_data_at_every_depth() {
    on_big_stack(|| {
        let mut t = Images::new();
        let rgb = Png::rgb(2, 1, &[255, 0, 0, 0, 0, 255]).encode();
        let path = t.file("rgb.png", &rgb);
        let from_file = t.file_spec("png", &path, "");
        assert_eq!(t.pixels(from_file), [0xFF0000, 0x0000FF]);
        assert_eq!(t.mask(from_file), None);
        assert_eq!(
            eval(
                &mut t.lisp,
                &format!("(image-size '(image :type png :file \"{path}\") t)")
            ),
            "(2 . 1)"
        );
        let from_data = t.data_spec(sym::PNG, &rgb, &[]);
        assert_eq!(t.pixels(from_data), [0xFF0000, 0x0000FF]);
        let gray = Png {
            color: png::ColorType::Grayscale,
            ..Png::rgb(2, 1, &[0x40, 0xC0])
        }
        .encode();
        let gray = t.data_spec(sym::PNG, &gray, &[]);
        assert_eq!(t.pixels(gray), [0x404040, 0xC0C0C0]);
        let low = Png {
            color: png::ColorType::Grayscale,
            depth: png::BitDepth::Two,
            ..Png::rgb(4, 1, &[0b0001_1011])
        }
        .encode();
        let low = t.data_spec(sym::PNG, &low, &[]);
        assert_eq!(t.pixels(low), [0x000000, 0x555555, 0xAAAAAA, 0xFFFFFF]);
        let deep = Png {
            depth: png::BitDepth::Sixteen,
            ..Png::rgb(1, 1, &[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC])
        }
        .encode();
        let deep = t.data_spec(sym::PNG, &deep, &[]);
        assert_eq!(t.pixels(deep), [0x12569A]);
        assert_eq!(eval(&mut t.lisp, "(and (memq 'png image-types) t)"), "t");
    });
}

#[test]
fn png_alpha_is_composited_and_binary_transparency_becomes_a_mask() {
    on_big_stack(|| {
        let mut t = Images::new();
        let rgba = Png {
            color: png::ColorType::Rgba,
            ..Png::rgb(2, 1, &[255, 0, 0, 128, 0, 0, 255, 255])
        }
        .encode();
        let over_frame = t.data_spec(sym::PNG, &rgba, &[]);
        assert_eq!(t.pixels(over_frame), [0xFF7F7F, 0x0000FF]);
        assert_eq!(t.mask(over_frame), None);
        let black = t.lisp.string("black");
        let over_black = t.data_spec(sym::PNG, &rgba, &[sym::K_BACKGROUND, black]);
        assert_eq!(t.pixels(over_black), [0x800000, 0x0000FF]);
        let palette = [255, 0, 0, 0, 255, 0];
        let keyed = Png {
            color: png::ColorType::Indexed,
            palette: Some(&palette),
            trns: Some(&[0]),
            ..Png::rgb(2, 1, &[0, 1])
        }
        .encode();
        let keyed = t.data_spec(sym::PNG, &keyed, &[]);
        assert_eq!(t.pixels(keyed), [0xFF0000, 0x00FF00]);
        assert_eq!(t.mask(keyed), Some(vec![0, 255]));
        let partial = Png {
            color: png::ColorType::Indexed,
            palette: Some(&palette),
            trns: Some(&[128]),
            ..Png::rgb(2, 1, &[0, 1])
        }
        .encode();
        let partial = t.data_spec(sym::PNG, &partial, &[]);
        assert_eq!(t.pixels(partial), [0xFF7F7F, 0x00FF00]);
        assert_eq!(t.mask(partial), None);
        let gray_key = Png {
            color: png::ColorType::Grayscale,
            trns: Some(&[0, 0x40]),
            ..Png::rgb(2, 1, &[0x40, 0x80])
        }
        .encode();
        let gray_key = t.data_spec(sym::PNG, &gray_key, &[]);
        assert_eq!(t.pixels(gray_key), [0x404040, 0x808080]);
        assert_eq!(t.mask(gray_key), Some(vec![0, 255]));
        let deep = Png {
            color: png::ColorType::Rgba,
            depth: png::BitDepth::Sixteen,
            ..Png::rgb(1, 1, &[0xFF, 0xFF, 0, 0, 0, 0, 0x80, 0])
        }
        .encode();
        let deep = t.data_spec(sym::PNG, &deep, &[]);
        assert_eq!(t.pixels(deep), [0xFF7F7F]);
    });
}

#[test]
fn png_backgrounds_come_from_bkgd_and_failures_are_logged() {
    on_big_stack(|| {
        let mut t = Images::new();
        let keyed = Png {
            depth: png::BitDepth::Sixteen,
            trns: Some(&[0, 0, 0, 0, 0, 0]),
            bkgd: Some(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]),
            ..Png::rgb(1, 1, &[0, 0, 0, 0, 0, 0])
        }
        .encode();
        let keyed = t.data_spec(sym::PNG, &keyed, &[]);
        assert_eq!(t.background(keyed), (true, 0x12569A));
        let opaque = Png {
            depth: png::BitDepth::Sixteen,
            bkgd: Some(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]),
            ..Png::rgb(1, 1, &[0, 0, 0, 0, 0, 0])
        }
        .encode();
        let opaque = t.data_spec(sym::PNG, &opaque, &[]);
        assert_eq!(t.background(opaque), (true, 0x12569A));
        let composed = Png {
            color: png::ColorType::Rgba,
            bkgd: Some(&[0, 0x12, 0, 0x34, 0, 0x56]),
            ..Png::rgb(1, 1, &[1, 2, 3, 128])
        }
        .encode();
        let composed = t.data_spec(sym::PNG, &composed, &[]);
        assert_eq!(t.background(composed), (true, 0x000000));
        let invalid = Png {
            bkgd: Some(&[1, 0, 0, 0, 0, 0]),
            ..Png::rgb(2, 2, &[7, 7, 7, 7, 7, 7, 7, 7, 7, 1, 1, 1])
        }
        .encode();
        let invalid = t.data_spec(sym::PNG, &invalid, &[]);
        assert_eq!(t.background(invalid), (true, 0x070707));
        let plain = Png::rgb(2, 2, &[9, 9, 9, 9, 9, 9, 9, 9, 9, 1, 1, 1]).encode();
        let plain = t.data_spec(sym::PNG, &plain, &[]);
        assert_eq!(t.background(plain), (true, 0x090909));
        let rgb = Png::rgb(1, 1, &[1, 2, 3]).encode();
        let truncated = t.data_spec(sym::PNG, &rgb[..rgb.len() - 20], &[]);
        assert_eq!(
            eval(
                &mut t.lisp,
                "(image-size '(image :type png :data \"not a png\") t)"
            ),
            "(30 . 30)"
        );
        let text = t.file("text.png", b"plain text here");
        for source in [
            format!("(image-size '(image :type png :file \"{text}\") t)"),
            "(image-size '(image :type png :file \"nope.png\") t)".to_string(),
        ] {
            assert_eq!(eval(&mut t.lisp, &source), "(30 . 30)");
        }
        let id = t.lisp.lookup_image(t.f, truncated, -1).unwrap();
        assert!(t.lisp.image(t.f, id).unwrap().pixmap.is_none());
        let log = messages(&mut t.lisp);
        assert!(
            log.contains("Not a PNG image: \u{2018}(image :type png :data not a png)\u{2019}"),
            "{log:?}"
        );
        assert!(
            log.to_lowercase().contains(&format!(
                "not a png file: \u{2018}{}\u{2019}",
                text.to_lowercase()
            )),
            "{log:?}"
        );
        assert!(
            log.contains("Cannot find image file \u{2018}nope.png\u{2019}"),
            "{log:?}"
        );
        assert!(log.contains("PNG error: "), "{log:?}");
        assert!(log.contains("PNG warning: invalid color"), "{log:?}");
    });
}

fn jpeg(width: u32, height: u32, color: image::ExtendedColorType, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 100)
        .encode(data, width, height, color)
        .unwrap();
    out
}

fn near(pixel: u64, rgb: u64) -> bool {
    (0..3).all(|i| {
        let shift = i * 8;
        ((pixel >> shift) & 0xff).abs_diff((rgb >> shift) & 0xff) <= 3
    })
}

#[test]
fn jpeg_pixels_load_from_files_and_data() {
    on_big_stack(|| {
        let mut t = Images::new();
        let gray = jpeg(8, 8, image::ExtendedColorType::L8, &[0x80; 64]);
        let gray_spec = t.data_spec(sym::JPEG, &gray, &[]);
        assert_eq!(t.pixels(gray_spec), [0x808080; 64]);
        assert_eq!(t.mask(gray_spec), None);
        assert_eq!(t.background(gray_spec), (true, 0x808080));
        let color: Vec<u8> = [200, 100, 50].repeat(64);
        let color = jpeg(8, 8, image::ExtendedColorType::Rgb8, &color);
        let path = t.file("color.jpg", &color);
        let from_file = t.file_spec("jpeg", &path, "");
        let pixels = t.pixels(from_file);
        assert_eq!(pixels.len(), 64);
        assert!(
            pixels.iter().all(|&p| near(p, 0xC86432)),
            "{:06x?}",
            &pixels[..4]
        );
        assert_eq!(
            eval(
                &mut t.lisp,
                &format!("(image-size '(image :type jpeg :file \"{path}\") t)")
            ),
            "(8 . 8)"
        );
        assert_eq!(eval(&mut t.lisp, "(and (memq 'jpeg image-types) t)"), "t");
        assert_eq!(
            eval(
                &mut t.lisp,
                "(list (imagep '(image :type jpeg :data \"x\" :conversions disabled)) (imagep '(image :type jpeg :file \"a\" :data \"x\")) (imagep '(image :type jpeg)))"
            ),
            "(t nil nil)"
        );
        for source in [
            "(image-size '(image :type jpeg :data \"not a jpeg\") t)",
            "(image-size '(image :type jpeg :file \"nope.jpg\") t)",
        ] {
            assert_eq!(eval(&mut t.lisp, source), "(30 . 30)");
        }
        let log = messages(&mut t.lisp);
        assert!(
            log.contains(
                "Error reading JPEG image \u{2018}(image :type jpeg :data not a jpeg)\u{2019}: "
            ),
            "{log:?}"
        );
        assert!(
            log.contains("Cannot find image file \u{2018}nope.jpg\u{2019}"),
            "{log:?}"
        );
    });
}

const GIF_PALETTE: [u8; 12] = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];

fn gif_frame(
    (left, top, width, height): (u16, u16, u16, u16),
    buffer: &[u8],
    delay: u16,
    dispose: gif::DisposalMethod,
    transparent: Option<u8>,
) -> gif::Frame<'_> {
    gif::Frame {
        left,
        top,
        width,
        height,
        delay,
        dispose,
        transparent,
        buffer: std::borrow::Cow::Borrowed(buffer),
        ..gif::Frame::default()
    }
}

fn encode_gif(width: u16, height: u16, frames: &[gif::Frame<'_>], repeat: bool) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = gif::Encoder::new(&mut out, width, height, &GIF_PALETTE).unwrap();
        if repeat {
            e.set_repeat(gif::Repeat::Infinite).unwrap();
        }
        for frame in frames {
            e.write_frame(frame).unwrap();
        }
    }
    out
}

fn animation() -> Vec<u8> {
    use gif::DisposalMethod::{Any, Background, Keep};
    encode_gif(
        2,
        1,
        &[
            gif_frame((0, 0, 2, 1), &[0, 1], 0, Any, None),
            gif_frame((1, 0, 1, 1), &[2], 10, Keep, None),
            gif_frame((0, 0, 2, 1), &[3, 0], 0, Keep, Some(0)),
            gif_frame((0, 0, 2, 1), &[0, 1], 0, Background, Some(0)),
        ],
        true,
    )
}

#[test]
fn gif_frames_are_composited_with_disposal_and_transparency() {
    on_big_stack(|| {
        let mut t = Images::new();
        let data = animation();
        let expected = [
            [0xFF0000, 0x00FF00],
            [0xFF0000, 0x0000FF],
            [0xFFFFFF, 0x0000FF],
            [0xFFFFFF, 0x00FF00],
        ];
        for (index, pixels) in expected.iter().enumerate() {
            let spec = t.data_spec(
                sym::GIF,
                &data,
                &[sym::K_INDEX, Value::fixnum(index as i64)],
            );
            assert_eq!(t.pixels(spec), pixels, "frame {index}");
            assert_eq!(t.mask(spec), None);
        }
        let plain = t.data_spec(sym::GIF, &data, &[]);
        assert_eq!(t.pixels(plain), [0xFF0000, 0x00FF00]);
        let black = t.lisp.string("black");
        let over_black = t.data_spec(
            sym::GIF,
            &data,
            &[sym::K_INDEX, Value::fixnum(3), sym::K_BACKGROUND, black],
        );
        assert_eq!(t.pixels(over_black), [0x000000, 0x00FF00]);
        let small = encode_gif(
            3,
            2,
            &[gif_frame(
                (1, 1, 1, 1),
                &[2],
                0,
                gif::DisposalMethod::Any,
                None,
            )],
            false,
        );
        let small = t.data_spec(sym::GIF, &small, &[]);
        assert_eq!(
            t.pixels(small),
            [0xFFFFFF, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF, 0x0000FF, 0xFFFFFF]
        );
        let mut interlaced = gif_frame(
            (0, 0, 1, 4),
            &[0, 2, 1, 3],
            0,
            gif::DisposalMethod::Any,
            None,
        );
        interlaced.interlaced = true;
        let interlaced = encode_gif(1, 4, &[interlaced], false);
        let path = t.file("interlaced.gif", &interlaced);
        let interlaced = t.file_spec("gif", &path, "");
        assert_eq!(
            t.pixels(interlaced),
            [0xFF0000, 0x00FF00, 0x0000FF, 0xFFFFFF]
        );
        assert_eq!(eval(&mut t.lisp, "(and (memq 'gif image-types) t)"), "t");
    });
}

#[test]
fn gif_metadata_animation_caches_and_failures() {
    on_big_stack(|| {
        let mut t = Images::new();
        let data = animation();
        let before: i64 = eval(&mut t.lisp, "(image-cache-size)").parse().unwrap();
        let first = t.data_spec(sym::GIF, &data, &[sym::K_INDEX, Value::fixnum(0)]);
        let second = t.data_spec(sym::GIF, &data, &[sym::K_INDEX, Value::fixnum(1)]);
        let plain = t.data_spec(sym::GIF, &data, &[]);
        for (name, spec) in [("first", first), ("second", second), ("plain", plain)] {
            let symbol = t.lisp.intern(name);
            t.lisp.set_plain_value(symbol, spec);
        }
        let describe = "(function (lambda (spec) (let ((m (image-metadata spec))) (list (plist-get m 'count) (plist-get m 'delay) (mapcar (function (lambda (x) (if (stringp x) (append x nil) x))) (car (cdr (memq 'extension-data m))))))))";
        assert_eq!(
            eval(&mut t.lisp, &format!("(funcall {describe} first)")),
            "(4 t (249 (0 0 0 0) 0 (1 0 0) 255 (78 69 84 83 67 65 80 69 50 46 48)))"
        );
        assert_eq!(
            eval(&mut t.lisp, &format!("(funcall {describe} second)")),
            "(4 0.1 (249 (4 10 0 0)))"
        );
        let after: i64 = eval(&mut t.lisp, "(image-cache-size)").parse().unwrap();
        assert!(after >= before + 2 * data.len() as i64, "{before} {after}");
        assert_eq!(eval(&mut t.lisp, "(clear-image-cache nil second)"), "nil");
        let pruned: i64 = eval(&mut t.lisp, "(image-cache-size)").parse().unwrap();
        assert_eq!(pruned, after - data.len() as i64);
        assert_eq!(
            eval(&mut t.lisp, "(clear-image-cache nil 5)"),
            "error: (wrong-type-argument consp 5)"
        );
        let out_of_range = t.data_spec(sym::GIF, &data, &[sym::K_INDEX, Value::fixnum(5)]);
        let id = t.lisp.lookup_image(t.f, out_of_range, -1).unwrap();
        assert!(t.lisp.image(t.f, id).unwrap().pixmap.is_none());
        let truncated = t.data_spec(sym::GIF, &data[..data.len() - 8], &[]);
        let id = t.lisp.lookup_image(t.f, truncated, -1).unwrap();
        assert!(t.lisp.image(t.f, id).unwrap().pixmap.is_none());
        for source in [
            "(image-size '(image :type gif :data \"not a gif\") t)",
            "(image-size '(image :type gif :file \"nope.gif\") t)",
        ] {
            assert_eq!(eval(&mut t.lisp, source), "(30 . 30)");
        }
        let log = messages(&mut t.lisp);
        for expected in [
            "Cannot open memory source \u{2018}(image :type gif :data not a gif)\u{2019}: Data is not in GIF format",
            "Cannot find image file \u{2018}nope.gif\u{2019}",
            "Invalid image number \u{2018}5\u{2019} in image",
            "Error reading GIF data: ",
        ] {
            assert!(log.contains(expected), "{expected:?} in {log:?}");
        }
    });
}

fn tiff_image<C: tiff::encoder::colortype::ColorType>(
    e: &mut tiff::encoder::TiffEncoder<std::io::Cursor<&mut Vec<u8>>>,
    (width, height): (u32, u32),
    tags: &[(tiff::tags::Tag, &[u16])],
    data: &[C::Inner],
) where
    [C::Inner]: tiff::encoder::TiffValue,
{
    let mut image = e.new_image::<C>(width, height).unwrap();
    for &(tag, value) in tags {
        image.encoder().write_tag(tag, value).unwrap();
    }
    image.write_data(data).unwrap();
}

fn tiff_bytes(
    write: impl FnOnce(&mut tiff::encoder::TiffEncoder<std::io::Cursor<&mut Vec<u8>>>),
) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = tiff::encoder::TiffEncoder::new(std::io::Cursor::new(&mut out)).unwrap();
        write(&mut e);
    }
    out
}

#[test]
fn tiff_pixels_follow_libtiff_rgba_conversion() {
    use tiff::encoder::colortype::{CMYK8, Gray8, Gray16, RGB8, RGBA8};
    use tiff::tags::Tag;
    on_big_stack(|| {
        let mut t = Images::new();
        let rgb = tiff_bytes(|e| tiff_image::<RGB8>(e, (2, 1), &[], &[255, 0, 0, 0, 0, 255]));
        let path = t.file("rgb.tif", &rgb);
        let from_file = t.file_spec("tiff", &path, "");
        assert_eq!(t.pixels(from_file), [0xFF0000, 0x0000FF]);
        assert_eq!(t.mask(from_file), None);
        assert_eq!(
            eval(
                &mut t.lisp,
                &format!("(image-size '(image :type tiff :file \"{path}\") t)")
            ),
            "(2 . 1)"
        );
        let from_data = t.data_spec(sym::TIFF, &rgb, &[]);
        assert_eq!(t.pixels(from_data), [0xFF0000, 0x0000FF]);
        let gray = tiff_bytes(|e| tiff_image::<Gray8>(e, (2, 1), &[], &[0x40, 0xC0]));
        let gray = t.data_spec(sym::TIFF, &gray, &[]);
        assert_eq!(t.pixels(gray), [0x404040, 0xC0C0C0]);
        let inverted = tiff_bytes(|e| {
            tiff_image::<Gray8>(
                e,
                (2, 1),
                &[(Tag::PhotometricInterpretation, &[0])],
                &[0x40, 0xC0],
            )
        });
        let inverted = t.data_spec(sym::TIFF, &inverted, &[]);
        assert_eq!(t.pixels(inverted), [0xBFBFBF, 0x3F3F3F]);
        let deep = tiff_bytes(|e| tiff_image::<Gray16>(e, (2, 1), &[], &[0x12F0, 0xFFFF]));
        let deep = t.data_spec(sym::TIFF, &deep, &[]);
        assert_eq!(t.pixels(deep), [0x121212, 0xFFFFFF]);
        let associated = tiff_bytes(|e| tiff_image::<RGBA8>(e, (1, 1), &[], &[200, 100, 50, 128]));
        let associated = t.data_spec(sym::TIFF, &associated, &[]);
        assert_eq!(t.pixels(associated), [0xC86432]);
        let unassociated = tiff_bytes(|e| {
            tiff_image::<RGBA8>(
                e,
                (1, 1),
                &[(Tag::ExtraSamples, &[2])],
                &[200, 100, 50, 128],
            )
        });
        let unassociated = t.data_spec(sym::TIFF, &unassociated, &[]);
        assert_eq!(t.pixels(unassociated), [0x643219]);
        let cmyk =
            tiff_bytes(|e| tiff_image::<CMYK8>(e, (2, 1), &[], &[0, 255, 255, 0, 0, 0, 0, 128]));
        let cmyk = t.data_spec(sym::TIFF, &cmyk, &[]);
        assert_eq!(t.pixels(cmyk), [0xFF0000, 0x7F7F7F]);
        let square = [1, 0, 0, 2, 0, 0, 3, 0, 0, 4, 0, 0];
        for (orientation, expected) in [
            (1u16, [1, 2, 3, 4]),
            (2, [2, 1, 4, 3]),
            (3, [4, 3, 2, 1]),
            (4, [3, 4, 1, 2]),
            (5, [1, 2, 3, 4]),
            (7, [4, 3, 2, 1]),
        ] {
            let oriented = tiff_bytes(|e| {
                tiff_image::<RGB8>(e, (2, 2), &[(Tag::Orientation, &[orientation])], &square)
            });
            let oriented = t.data_spec(sym::TIFF, &oriented, &[]);
            let expected: Vec<u64> = expected.iter().map(|&r| r << 16).collect();
            assert_eq!(t.pixels(oriented), expected, "orientation {orientation}");
        }
        assert_eq!(eval(&mut t.lisp, "(and (memq 'tiff image-types) t)"), "t");
    });
}

#[test]
fn tiff_directories_are_indexed_and_counted() {
    use tiff::encoder::colortype::RGB8;
    on_big_stack(|| {
        let mut t = Images::new();
        let pages = tiff_bytes(|e| {
            tiff_image::<RGB8>(e, (1, 1), &[], &[255, 0, 0]);
            tiff_image::<RGB8>(e, (2, 1), &[], &[0, 255, 0, 0, 0, 255]);
        });
        let first = t.data_spec(sym::TIFF, &pages, &[]);
        assert_eq!(t.pixels(first), [0xFF0000]);
        let second = t.data_spec(sym::TIFF, &pages, &[sym::K_INDEX, Value::fixnum(1)]);
        assert_eq!(t.pixels(second), [0x00FF00, 0x0000FF]);
        let symbol = t.lisp.intern("second");
        t.lisp.set_plain_value(symbol, second);
        assert_eq!(eval(&mut t.lisp, "(image-metadata second)"), "(count 2)");
        let single = tiff_bytes(|e| tiff_image::<RGB8>(e, (1, 1), &[], &[1, 2, 3]));
        let single = t.data_spec(sym::TIFF, &single, &[]);
        let symbol = t.lisp.intern("single");
        t.lisp.set_plain_value(symbol, single);
        assert_eq!(eval(&mut t.lisp, "(image-metadata single)"), "nil");
        let missing = t.data_spec(sym::TIFF, &pages, &[sym::K_INDEX, Value::fixnum(4)]);
        let id = t.lisp.lookup_image(t.f, missing, -1).unwrap();
        assert!(t.lisp.image(t.f, id).unwrap().pixmap.is_none());
        for source in [
            "(image-size '(image :type tiff :data \"not a tiff\") t)",
            "(image-size '(image :type tiff :file \"nope.tif\") t)",
        ] {
            assert_eq!(eval(&mut t.lisp, source), "(30 . 30)");
        }
        let log = messages(&mut t.lisp);
        for expected in [
            "TIFF error: memory_source ",
            "Cannot open memory source for \u{2018}(image :type tiff :data not a tiff)\u{2019}",
            "Cannot find image file \u{2018}nope.tif\u{2019}",
            "Invalid image number \u{2018}4\u{2019} in image",
        ] {
            assert!(log.contains(expected), "{expected:?} in {log:?}");
        }
    });
}
