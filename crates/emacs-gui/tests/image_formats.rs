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
