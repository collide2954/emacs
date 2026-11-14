use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::image::{CrFilter, CrFormat, DEFAULT_IMAGE_ASCENT};
use emacs_core::lisp::Lisp;
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"i\") (width . 30) (height . 8)))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    (lisp, f)
}

fn spec(lisp: &mut Lisp, source: &str) -> Value {
    let value = lisp.eval_string(source).expect("image spec");
    lisp.stack.push(value);
    value
}

fn messages(lisp: &mut Lisp) -> String {
    eval(
        lisp,
        "(save-current-buffer (set-buffer \"*Messages*\") (buffer-string))",
    )
}

const RED: u64 = 0xFF0000;
const BLUE: u64 = 0x0000FF;

#[test]
fn image_specs_are_validated_per_type() {
    on_big_stack(|| {
        let (mut lisp, _) = frame();
        for (source, expected) in [
            (
                "(imagep '(image :type xbm :data-width 8 :data-height 2 :data \"ab\"))",
                "t",
            ),
            (
                "(imagep '(image :type xbm :data-width 8 :data-height 2 :data \"a\"))",
                "nil",
            ),
            (
                "(imagep '(image :type xbm :data-width 8 :data-height 2))",
                "nil",
            ),
            (
                "(imagep '(image :type xbm :data-width 8 :data-height 2 :data [\"a\" \"b\"]))",
                "t",
            ),
            (
                "(imagep '(image :type xbm :data-width 8 :data-height 2 :data [\"a\"]))",
                "nil",
            ),
            (
                "(imagep '(image :type xbm :data-width 8 :data-height 2 :data [\"a\" 5]))",
                "nil",
            ),
            (
                "(imagep (list 'image :type 'xbm :data-width 3 :data-height 2 :stride 8 :data (make-bool-vector 16 t)))",
                "t",
            ),
            (
                "(imagep (list 'image :type 'xbm :data-width 3 :data-height 2 :data (make-bool-vector 16 t)))",
                "nil",
            ),
            (
                "(imagep '(image :type xbm :data \"#define a_width 4\n#define a_height 3\nstatic char a_bits[] = {0x1,0x2,0x3};\"))",
                "t",
            ),
            ("(imagep '(image :type xbm :file \"x.xbm\"))", "t"),
            (
                "(imagep '(image :type xbm :file \"x.xbm\" :data \"x\"))",
                "nil",
            ),
            ("(imagep '(image :type xbm :file 5))", "nil"),
            (
                "(imagep '(image :type xbm :file \"x\" :ascent center :margin (1 . 2) :relief -3 :foo bar))",
                "t",
            ),
            ("(imagep '(image :type xbm :file \"x\" :ascent 101))", "nil"),
            ("(imagep '(image :type xbm :file \"x\" :margin -1))", "nil"),
            ("(imagep '(image :type xbm :file \"x\" :file \"y\"))", "nil"),
            ("(imagep '(image :type xbm :file))", "nil"),
            ("(imagep '(image :type nosuch :file \"x\"))", "nil"),
            ("(imagep '(image :file \"x\"))", "nil"),
            ("(imagep '(image))", "nil"),
            ("(imagep 5)", "nil"),
            ("(imagep '(image :type pbm :data \"P1 1 1 1\"))", "t"),
            (
                "(imagep '(image :type pbm :data \"P1 1 1 1\" :file \"x\"))",
                "nil",
            ),
            ("(imagep '(image :type pbm :ascent 3))", "nil"),
            ("(init-image-library 'xbm)", "t"),
            ("(init-image-library 'pbm)", "t"),
            ("(init-image-library 'nosuch)", "nil"),
            (
                "(and (memq 'xbm image-types) (memq 'pbm image-types) t)",
                "t",
            ),
            ("(image-transforms-p f)", "(scale rotate90)"),
            (
                "(image-transforms-p 'x)",
                "error: (wrong-type-argument frame-live-p x)",
            ),
            (
                "(image-size 5)",
                "error: (error \"Invalid image specification\")",
            ),
            (
                "(image-mask-p 5)",
                "error: (error \"Invalid image specification\")",
            ),
            ("(image-metadata 5)", "nil"),
            (
                "(image-flush 5)",
                "error: (error \"Invalid image specification\")",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn image_sizes_follow_margins_scaling_and_rotation() {
    on_big_stack(|| {
        let (mut lisp, _) = frame();
        eval(
            &mut lisp,
            "(setq s '(image :type xbm :data-width 8 :data-height 2 :data \"ab\"))",
        );
        for (source, expected) in [
            ("(image-size s t)", "(8 . 2)"),
            (
                "(equal (image-size s) (cons (/ 8.0 (frame-char-width)) (/ 2.0 (frame-char-height))))",
                "t",
            ),
            ("(image-size (append s '(:margin 3)) t)", "(14 . 8)"),
            ("(image-size (append s '(:margin (1 . 2))) t)", "(10 . 6)"),
            ("(image-size (append s '(:relief -2)) t)", "(12 . 6)"),
            (
                "(image-size (append s '(:relief 2 :margin 1)) t)",
                "(14 . 8)",
            ),
            ("(image-size (append s '(:scale 2)) t)", "(16 . 4)"),
            ("(image-size (append s '(:width 20)) t)", "(20 . 5)"),
            ("(image-size (append s '(:height 4)) t)", "(16 . 4)"),
            ("(image-size (append s '(:width 4 :height 9)) t)", "(4 . 9)"),
            ("(image-size (append s '(:max-width 4)) t)", "(4 . 1)"),
            ("(image-size (append s '(:max-height 1)) t)", "(4 . 1)"),
            (
                "(image-size (append s '(:width 16 :max-width 4)) t)",
                "(16 . 4)",
            ),
            ("(image-size (append s '(:rotation 90)) t)", "(2 . 8)"),
            ("(image-size (append s '(:rotation 180)) t)", "(8 . 2)"),
            (
                "(image-size (append s '(:rotation 270 :flip t)) t)",
                "(2 . 8)",
            ),
            ("(image-size (append s '(:rotation 450)) t)", "(2 . 8)"),
            ("(image-size (append s '(:rotation 45)) t)", "(8 . 2)"),
            ("(image-size (append s '(:scale default)) t)", "(8 . 2)"),
            (
                "(let ((image-scaling-factor 2)) (image-size (append s '(:scale default)) t))",
                "(16 . 4)",
            ),
            (
                "(let ((image-scaling-factor 'x)) (image-size (append s '(:scale default)) t))",
                "(8 . 2)",
            ),
            (
                "(image-size '(image :type xbm :file \"no-such-image-file.xbm\") t)",
                "(30 . 30)",
            ),
            (
                "(image-size '(image :type xbm :file \"no-such-image-file.xbm\" :width 7 :height 9) t)",
                "(7 . 9)",
            ),
            (
                "(image-size '(image :type pbm :data \"P1\n3 2\n1 0 1\n0 1 0\n\") t)",
                "(3 . 2)",
            ),
            (
                "(image-size '(image :type pbm :data \"P2 2 1 255 0 255\") t)",
                "(2 . 1)",
            ),
            (
                "(image-size '(image :type pbm :data \"P3 1 1 255 255 0 0\") t)",
                "(1 . 1)",
            ),
            (
                "(image-size (list 'image :type 'pbm :data (concat \"P4\n9 1\n\" (unibyte-string 255 128))) t)",
                "(9 . 1)",
            ),
            (
                "(image-size (list 'image :type 'pbm :data (concat \"P6 2 2 255 \" (unibyte-string 0))) t)",
                "(30 . 30)",
            ),
            (
                "(image-size '(image :type pbm :data \"P7 1 1\") t)",
                "(30 . 30)",
            ),
            ("(image-metadata s)", "nil"),
            ("(image-mask-p s)", "nil"),
            ("(image-mask-p (append s '(:mask heuristic)))", "t"),
            ("(image-mask-p (append s '(:heuristic-mask t)))", "t"),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        let log = messages(&mut lisp);
        for line in [
            "No native support for rotation by 45 degrees",
            "Invalid \u{2018}image-scaling-factor\u{2019}: x",
            "Cannot find image file \u{2018}no-such-image-file.xbm\u{2019}",
            "Invalid image size in image \u{2018}(image :type pbm :data",
            "Not a PBM image: \u{2018}(image :type pbm :data P7 1 1)\u{2019}",
        ] {
            assert!(log.contains(line), "{line:?} not in {log:?}");
        }
    });
}

#[test]
fn xbm_files_load_from_disk() {
    on_big_stack(|| {
        let (mut lisp, f) = frame();
        let dir = std::env::temp_dir().join(format!("emacs-images-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let chars = dir.join("chars.xbm");
        std::fs::write(
            &chars,
            "/* five by three */\n#define t_width 5\n#define t_height 3\n#define t_x_hot 1\nstatic unsigned char t_bits[] = {\n 0x1f, 0x00, 0x15 };\n",
        )
        .unwrap();
        let shorts = dir.join("shorts.xbm");
        std::fs::write(
            &shorts,
            "#define s_width 9\n#define s_height 1\nstatic short s_bits[] = { 0x01ff };\n",
        )
        .unwrap();
        let broken = dir.join("broken.xbm");
        std::fs::write(
            &broken,
            "#define b_width 4\n#define b_height 1\nstatic int b_bits[] = { 1 };\n",
        )
        .unwrap();
        let path = |p: &std::path::Path| p.to_string_lossy().replace('\\', "/");
        assert_eq!(
            eval(
                &mut lisp,
                &format!(
                    "(image-size '(image :type xbm :file \"{}\") t)",
                    path(&chars)
                )
            ),
            "(5 . 3)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                &format!(
                    "(image-size '(image :type xbm :file \"{}\") t)",
                    path(&shorts)
                )
            ),
            "(9 . 1)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                &format!(
                    "(image-size '(image :type xbm :file \"{}\") t)",
                    path(&broken)
                )
            ),
            "(30 . 30)"
        );
        let s = spec(
            &mut lisp,
            &format!(
                "'(image :type xbm :file \"{}\" :foreground \"red\" :background \"blue\")",
                path(&chars)
            ),
        );
        let id = lisp.lookup_image(f, s, -1).unwrap();
        let img = lisp.image(f, id).unwrap();
        let pixmap = img.pixmap.as_ref().unwrap();
        let row = |y: i32| (0..5).map(|x| pixmap.get_pixel(x, y)).collect::<Vec<_>>();
        assert_eq!(row(0), [RED; 5]);
        assert_eq!(row(1), [BLUE; 5]);
        assert_eq!(row(2), [RED, BLUE, RED, BLUE, RED]);
        assert!(img.background_valid);
        assert_eq!(img.background, BLUE);
        let _ = std::fs::remove_dir_all(&dir);
        let log = messages(&mut lisp);
        assert!(log.contains("Error loading XBM image"), "{log:?}");
    });
}

#[test]
fn bitmaps_and_pixmaps_hold_their_pixels() {
    on_big_stack(|| {
        let (mut lisp, f) = frame();
        let s = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 8 :data-height 1 :data (unibyte-string #xa5) :foreground \"red\" :background \"blue\")",
        );
        let id = lisp.lookup_image(f, s, -1).unwrap();
        assert_eq!(lisp.lookup_image(f, s, -1).unwrap(), id);
        let img = lisp.image(f, id).unwrap();
        assert_eq!((img.width, img.height), (8, 1));
        assert_eq!(img.ascent, DEFAULT_IMAGE_ASCENT);
        assert!(img.mask.is_none());
        let pixmap = img.pixmap.as_ref().unwrap();
        assert_eq!(
            (0..8).map(|x| pixmap.get_pixel(x, 0)).collect::<Vec<_>>(),
            [RED, BLUE, RED, BLUE, BLUE, RED, BLUE, RED]
        );
        let plain = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 2 :data-height 1 :data (unibyte-string 1))",
        );
        let id = lisp.lookup_image(f, plain, -1).unwrap();
        let pixmap = lisp.image(f, id).unwrap().pixmap.as_ref().unwrap();
        assert_eq!(
            (pixmap.get_pixel(0, 0), pixmap.get_pixel(1, 0)),
            (0x000000, 0xFFFFFF)
        );
        let vector = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 2 :data-height 2 :data (vector (unibyte-string 1) (bool-vector nil t)))",
        );
        let id = lisp.lookup_image(f, vector, -1).unwrap();
        let pixmap = lisp.image(f, id).unwrap().pixmap.as_ref().unwrap();
        assert_eq!(
            [
                pixmap.get_pixel(0, 0),
                pixmap.get_pixel(1, 0),
                pixmap.get_pixel(0, 1),
                pixmap.get_pixel(1, 1)
            ],
            [0x000000, 0xFFFFFF, 0xFFFFFF, 0x000000]
        );
        let masked = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 8 :data-height 1 :data (unibyte-string #xa5) :foreground \"red\" :background \"blue\" :mask 'heuristic :ascent 'center :margin 2)",
        );
        let id = lisp.lookup_image(f, masked, -1).unwrap();
        let img = lisp.image(f, id).unwrap();
        let mask = img.mask.as_ref().unwrap();
        assert_eq!(
            (0..8).map(|x| mask.get_pixel(x, 0)).collect::<Vec<_>>(),
            [0, 255, 0, 255, 255, 0, 255, 0]
        );
        assert!(img.background_transparent);
        assert_eq!(img.ascent, -1);
        assert_eq!((img.hmargin, img.vmargin), (2, 2));
        let keyed = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 8 :data-height 1 :data (unibyte-string #xa5) :foreground \"red\" :background \"blue\" :mask '(heuristic (0 0 65535)))",
        );
        let id = lisp.lookup_image(f, keyed, -1).unwrap();
        let mask = lisp.image(f, id).unwrap().mask.as_ref().unwrap();
        assert_eq!(
            (0..8).map(|x| mask.get_pixel(x, 0)).collect::<Vec<_>>(),
            [255, 0, 255, 0, 0, 255, 0, 255]
        );
        let gray = spec(&mut lisp, "'(image :type pbm :data \"P2 2 1 255 128 0\")");
        let id = lisp.lookup_image(f, gray, -1).unwrap();
        let img = lisp.image(f, id).unwrap();
        let pixmap = img.pixmap.as_ref().unwrap();
        assert_eq!(
            (pixmap.get_pixel(0, 0), pixmap.get_pixel(1, 0)),
            (0x808080, 0x000000)
        );
        assert!(img.background_valid);
        let color = spec(
            &mut lisp,
            "'(image :type pbm :data \"P3 2 1 15 15 0 0 0 0 15\")",
        );
        let id = lisp.lookup_image(f, color, -1).unwrap();
        let pixmap = lisp.image(f, id).unwrap().pixmap.as_ref().unwrap();
        assert_eq!(
            (pixmap.get_pixel(0, 0), pixmap.get_pixel(1, 0)),
            (RED, BLUE)
        );
        let mono = spec(
            &mut lisp,
            "(list 'image :type 'pbm :data (concat \"P4 3 2 \" (unibyte-string #xa0 #x40)) :foreground \"red\")",
        );
        let id = lisp.lookup_image(f, mono, -1).unwrap();
        let pixmap = lisp.image(f, id).unwrap().pixmap.as_ref().unwrap();
        assert_eq!(
            (0..3)
                .flat_map(|y| (0..3).map(move |x| (x, y)))
                .take(6)
                .map(|(x, y)| pixmap.get_pixel(x, y % 2))
                .collect::<Vec<_>>(),
            [RED, 0xFFFFFF, RED, 0xFFFFFF, RED, 0xFFFFFF]
        );
    });
}

#[test]
fn conversions_rewrite_the_pixels() {
    on_big_stack(|| {
        let (mut lisp, f) = frame();
        let disabled = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 2 :data-height 1 :data (unibyte-string 1) :foreground \"red\" :background \"blue\" :conversion 'disabled)",
        );
        let id = lisp.lookup_image(f, disabled, -1).unwrap();
        let pixmap = lisp.image(f, id).unwrap().pixmap.as_ref().unwrap();
        assert_eq!(
            (pixmap.get_pixel(0, 0), pixmap.get_pixel(1, 0)),
            (0x8F8F8F, 0x828282)
        );
        let laplace = spec(
            &mut lisp,
            "(list 'image :type 'pbm :data \"P1 3 3 1 1 1 1 1 1 1 1 1\" :conversion 'laplace)",
        );
        let id = lisp.lookup_image(f, laplace, -1).unwrap();
        let pixmap = lisp.image(f, id).unwrap().pixmap.as_ref().unwrap();
        let pixels: Vec<u64> = (0..3)
            .flat_map(|y| (0..3).map(move |x| (x, y)))
            .map(|(x, y)| pixmap.get_pixel(x, y))
            .collect();
        let edge = 0x7F7F7F;
        assert_eq!(
            pixels,
            [edge, edge, edge, edge, 0xAFAFAF, edge, edge, edge, edge]
        );
        let custom = spec(
            &mut lisp,
            "(list 'image :type 'pbm :data \"P1 3 3 1 1 1 1 1 1 1 1 1\" :conversion '(edge-detection :matrix [1 0 0 0 0 0 0 0 -1] :color-adjustment 45000))",
        );
        let id = lisp.lookup_image(f, custom, -1).unwrap();
        let pixmap = lisp.image(f, id).unwrap().pixmap.as_ref().unwrap();
        assert_eq!(pixmap.get_pixel(1, 1), 0xAFAFAF);
    });
}

#[test]
fn transforms_and_display_preparation() {
    on_big_stack(|| {
        let (mut lisp, f) = frame();
        let big = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 8 :data-height 2 :data \"ab\" :scale 2)",
        );
        let id = lisp.lookup_image(f, big, -1).unwrap();
        let img = lisp.image(f, id).unwrap();
        assert_eq!((img.original_width, img.original_height), (8, 2));
        assert_eq!((img.width, img.height), (16, 4));
        let cr = img.cr_data.as_ref().unwrap();
        assert_eq!(cr.matrix, [0.5, 0.0, 0.0, 0.5, 0.0, 0.0]);
        assert_eq!(cr.filter, CrFilter::Nearest);
        assert!(cr.surface.is_none());
        lisp.prepare_image_for_display(f, id).unwrap();
        let img = lisp.image(f, id).unwrap();
        let surface = img.cr_data.as_ref().unwrap().surface.as_ref().unwrap();
        assert_eq!(
            (surface.width, surface.height, surface.format),
            (8, 2, CrFormat::Rgb24)
        );
        let small = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 8 :data-height 2 :data \"ab\" :scale 0.5)",
        );
        let id = lisp.lookup_image(f, small, -1).unwrap();
        let img = lisp.image(f, id).unwrap();
        assert_eq!((img.width, img.height), (4, 1));
        assert_eq!(img.cr_data.as_ref().unwrap().filter, CrFilter::Best);
        let turned = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 8 :data-height 2 :data \"ab\" :rotation 90)",
        );
        let id = lisp.lookup_image(f, turned, -1).unwrap();
        let img = lisp.image(f, id).unwrap();
        assert_eq!((img.width, img.height), (2, 8));
        let m = img.cr_data.as_ref().unwrap().matrix;
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(
            close(m[0], 0.0)
                && close(m[1], -1.0)
                && close(m[2], 1.0)
                && close(m[3], 0.0)
                && close(m[4], 0.0)
                && close(m[5], 2.0),
            "{m:?}"
        );
        let masked = spec(
            &mut lisp,
            "(list 'image :type 'xbm :data-width 2 :data-height 1 :data (unibyte-string 1) :foreground \"red\" :mask 'heuristic)",
        );
        let id = lisp.lookup_image(f, masked, -1).unwrap();
        lisp.prepare_image_for_display(f, id).unwrap();
        let img = lisp.image(f, id).unwrap();
        let surface = img.cr_data.as_ref().unwrap().surface.as_ref().unwrap();
        assert_eq!(surface.format, CrFormat::Argb32);
        let px = |x: usize| u32::from_ne_bytes(surface.data[x * 4..x * 4 + 4].try_into().unwrap());
        assert_eq!((px(0), px(1)), (0x00000000, 0xFFFFFFFF));
    });
}

#[test]
fn the_cache_is_shared_flushed_and_cleared() {
    on_big_stack(|| {
        let (mut lisp, _) = frame();
        eval(&mut lisp, "(clear-image-cache t)");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "0");
        eval(
            &mut lisp,
            "(setq s '(image :type xbm :data-width 8 :data-height 2 :data \"ab\") m (append s '(:mask heuristic)))",
        );
        eval(&mut lisp, "(image-size s)");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "64");
        eval(&mut lisp, "(image-size m)");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "144");
        assert_eq!(eval(&mut lisp, "(image-flush s)"), "nil");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "80");
        eval(
            &mut lisp,
            "(setq g (x-create-frame '((name . \"j\") (width . 10) (height . 3))))",
        );
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "160");
        assert_eq!(eval(&mut lisp, "(clear-image-cache)"), "nil");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "0");
        eval(&mut lisp, "(image-size s nil g)");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "128");
        eval(&mut lisp, "(delete-frame g)");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "64");
        assert_eq!(eval(&mut lisp, "(clear-image-cache nil '(x))"), "nil");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "64");
        assert_eq!(eval(&mut lisp, "(clear-image-cache \"other-file\")"), "nil");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "64");
        assert_eq!(eval(&mut lisp, "(clear-image-cache t)"), "nil");
        assert_eq!(eval(&mut lisp, "(image-cache-size)"), "0");
        assert_eq!(
            eval(&mut lisp, "(clear-image-cache nil 5)"),
            "error: (wrong-type-argument consp 5)"
        );
        eval(&mut lisp, "(garbage-collect)");
        assert_eq!(eval(&mut lisp, "(image-size s t)"), "(8 . 2)");
    });
}
