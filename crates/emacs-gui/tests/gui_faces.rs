use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::frame::ResourceType;
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_core::terminal::OutputMethod;
use emacs_core::value::Value;
use emacs_core::xfaces::DEFAULT_FACE_ID;
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

fn ok<T>(lisp: &mut Lisp, r: emacs_core::error::LispResult<T>) -> T {
    match r {
        Ok(v) => v,
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            panic!("{}", lisp.prin1_to_string(condition))
        }
    }
}

fn pgtk_frame(lisp: &mut Lisp) -> Value {
    eval(
        lisp,
        "(progn (setq x-resource-name \"emacs\") (x-open-connection nil))",
    );
    eval(
        lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    let check = lisp.window_system.unwrap().check_x_display_info;
    let terminal = check(lisp, Value::NIL).unwrap();
    let f = lisp.make_frame(true).unwrap();
    lisp.stack.push(f);
    lisp.frame_mut(f).terminal = terminal;
    lisp.frame_mut(f).output_method = OutputMethod::Pgtk;
    lisp.frame_mut(f).fontset = -1;
    let driver = emacs_gui::ftcrfont::ftcrfont_driver(lisp);
    let r = lisp.register_font_driver(driver, Some(f));
    ok(lisp, r);
    let r = lisp.gui_default_parameter(
        f,
        Value::NIL,
        sym::FONT_BACKEND,
        Value::NIL,
        Some("fontBackend"),
        Some("FontBackend"),
        ResourceType::String,
    );
    ok(lisp, r);
    let name = lisp.string("monospace-12");
    let r = lisp.gui_set_font(f, name, Value::NIL);
    ok(lisp, r);
    for (param, color) in [
        (sym::FOREGROUND_COLOR, "black"),
        (sym::BACKGROUND_COLOR, "white"),
    ] {
        let color = lisp.string(color);
        let r = lisp.store_frame_param(f, param, color);
        ok(lisp, r);
    }
    let r = lisp.init_frame_faces(f);
    ok(lisp, r);
    let symbol = lisp.intern("test-frame");
    lisp.set_plain_value(symbol, f);
    f
}

#[test]
fn a_frame_font_and_its_fontset() {
    on_big_stack(|| {
        let mut lisp = Lisp::boot(Startup::program(false));
        emacs_gui::install(
            &mut lisp,
            Display::new(fmpsc::unbounded().0, mpsc::channel().1),
        );
        let f = pgtk_frame(&mut lisp);
        assert_eq!(
            eval(&mut lisp, "(frame-parameter test-frame 'font-backend)"),
            "(ftcr)"
        );
        let font = lisp.frame(f).font;
        assert!(lisp.font_object_p(font));
        assert_eq!(lisp.font_object(font).pixel_size, 16);
        assert_eq!(
            eval(&mut lisp, "(frame-parameter test-frame 'font)"),
            lisp.prin1_to_string(lisp.font_aref(font, emacs_core::font::FONT_NAME_INDEX))
        );
        let fontset = lisp.frame(f).fontset;
        assert!(fontset > 0);
        assert_eq!(
            eval(
                &mut lisp,
                "(and (member (car (rassoc \"fontset-startup\" fontset-alias-alist)) (fontset-list)) t)"
            ),
            "t"
        );
        let average = lisp.font_object(font).average_width;
        let (ascent, descent) = lisp.get_font_ascent_descent(font);
        let geometry = &lisp.frame(f).geometry;
        assert_eq!(geometry.column_width, i64::from(average));
        assert_eq!(geometry.line_height, i64::from(ascent + descent));
        assert!(lisp.frame(f).fonts_changed);
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (modify-frame-parameters test-frame '((font . \"no such font anywhere\"))) 1)"
            ),
            "error: (error \"Font \u{2018}no such font anywhere\u{2019} is not defined\")"
        );
        assert_eq!(
            eval(&mut lisp, "(frame-parameter test-frame 'font-backend)"),
            "(ftcr)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(modify-frame-parameters test-frame '((font-backend . \"nope\")))"
            ),
            "nil"
        );
        assert_eq!(
            eval(&mut lisp, "(frame-parameter test-frame 'font-backend)"),
            "(ftcr)"
        );
        assert!(lisp.font_object_p(lisp.frame(f).font));
    });
}

#[test]
fn realizing_faces_on_a_gui_frame() {
    on_big_stack(|| {
        let mut lisp = Lisp::boot(Startup::program(false));
        emacs_gui::install(
            &mut lisp,
            Display::new(fmpsc::unbounded().0, mpsc::channel().1),
        );
        let f = pgtk_frame(&mut lisp);
        let font = lisp.frame(f).font;
        let fontset = lisp.frame(f).fontset;
        let face = lisp.face_from_id(f, DEFAULT_FACE_ID).unwrap();
        assert_eq!(face.font, font);
        assert_eq!((face.foreground, face.background), (0x000000, 0xFFFFFF));
        assert!(face.fontset > fontset as i32);
        assert!(!face.foreground_defaulted_p && !face.background_defaulted_p);
        let family =
            lisp.symbol_name_string(lisp.font_aref(font, emacs_core::font::FONT_FAMILY_INDEX));
        assert_eq!(
            eval(
                &mut lisp,
                "(internal-get-lisp-face-attribute 'default :family test-frame)"
            ),
            format!("{family:?}")
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(internal-get-lisp-face-attribute 'default :height test-frame)"
            ),
            "120"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(internal-get-lisp-face-attribute 'default :weight test-frame)"
            ),
            "regular"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(and (query-fontset (internal-get-lisp-face-attribute 'default :fontset test-frame)) t)"
            ),
            "t"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(internal-get-lisp-face-attribute 'default :font test-frame)"
            ),
            lisp.prin1_to_string(font)
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (mapc (function (lambda (a) (internal-set-lisp-face-attribute 'region (car a) (cdr a) test-frame))) (list (cons :background \"#123456\") (cons :foreground \"black\") (cons :inverse-video nil) (cons :underline \"red\") (cons :box '(:line-width 2 :color \"blue\" :style released-button)) (cons :overline t) (cons :strike-through \"green\"))) nil)"
            ),
            "nil"
        );
        let region = lisp.intern("region");
        let id = lisp.lookup_named_face(None, f, region, true).unwrap() as usize;
        let face = lisp.face_from_id(f, id).unwrap();
        assert_eq!(face.font, font);
        assert_eq!((face.foreground, face.background), (0x000000, 0x123456));
        assert_eq!(face.box_type, emacs_core::xfaces::FaceBoxType::RaisedBox);
        assert_eq!(
            (
                face.box_vertical_line_width,
                face.box_horizontal_line_width,
                face.box_color
            ),
            (2, 2, 0x0000FF)
        );
        assert!(face.use_box_color_for_shadows_p);
        assert_eq!(
            face.underline,
            emacs_core::xfaces::FaceUnderlineType::Single
        );
        assert_eq!(face.underline_color, 0xFF0000);
        assert!(face.overline_p && face.overline_color_defaulted_p);
        assert!(face.strike_through_p);
        assert_eq!(face.strike_through_color, 0x00FF00);
        assert!(face.fontset > fontset as i32);
    });
}
