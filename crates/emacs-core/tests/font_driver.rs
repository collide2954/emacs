mod common;

use common::{eval, on_big_stack};
use emacs_core::dispextern::GlyphString;
use emacs_core::error::LispResult;
use emacs_core::font::{
    FONT_FAMILY_INDEX, FONT_INVALID_CODE, FONT_REGISTRY_INDEX, FONT_SIZE_INDEX, FONT_SLANT_INDEX,
    FONT_SPACING_INDEX, FONT_TYPE_INDEX, FONT_WEIGHT_INDEX, FONT_WIDTH_INDEX, FontDriver,
    FontMetrics,
};
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_core::terminal::{DisplayInfo, OutputMethod};
use emacs_core::value::Value;

fn var(lisp: &mut Lisp, name: &str) -> Value {
    let symbol = lisp.intern(name);
    lisp.find_symbol_value(symbol)
}

fn bump(lisp: &mut Lisp, name: &str) {
    let symbol = lisp.intern(name);
    let n = lisp.find_symbol_value(symbol).as_fixnum().unwrap_or(0);
    lisp.set_plain_value(symbol, Value::fixnum(n + 1));
}

fn mock_get_cache(lisp: &mut Lisp, _f: Value) -> Value {
    var(lisp, "mock-font-cache")
}

fn mock_entity(lisp: &mut Lisp, family: &str, weight: &str, spacing: i64) -> LispResult {
    let entity = lisp.font_make_entity();
    let ty = lisp.intern("mock");
    lisp.font_aset(entity, FONT_TYPE_INDEX, ty);
    let family = lisp.intern(family);
    lisp.font_aset(entity, FONT_FAMILY_INDEX, family);
    lisp.font_aset(entity, FONT_REGISTRY_INDEX, sym::ISO10646_1);
    let weight = lisp.intern(weight);
    lisp.font_set_style(entity, FONT_WEIGHT_INDEX, weight)?;
    let normal = lisp.intern("normal");
    lisp.font_set_style(entity, FONT_SLANT_INDEX, normal)?;
    lisp.font_set_style(entity, FONT_WIDTH_INDEX, normal)?;
    lisp.font_aset(entity, FONT_SIZE_INDEX, Value::fixnum(0));
    lisp.font_aset(entity, FONT_SPACING_INDEX, Value::fixnum(spacing));
    Ok(entity)
}

const FAMILIES: [(&str, &str, i64); 3] = [
    ("Alpha", "normal", 100),
    ("Alpha", "bold", 100),
    ("Beta", "normal", 0),
];

fn mock_list(lisp: &mut Lisp, _f: Value, spec: Value) -> LispResult {
    bump(lisp, "mock-list-calls");
    let family = lisp.font_aref(spec, FONT_FAMILY_INDEX);
    let wanted = if family.is_nil() {
        None
    } else {
        Some(lisp.symbol_name_string(family).to_ascii_lowercase())
    };
    let mut entities = Vec::new();
    for (name, weight, spacing) in FAMILIES {
        if wanted
            .as_deref()
            .is_none_or(|w| w == name.to_ascii_lowercase())
        {
            entities.push(mock_entity(lisp, name, weight, spacing)?);
        }
    }
    Ok(lisp.list(&entities))
}

fn mock_match(lisp: &mut Lisp, f: Value, spec: Value) -> LispResult {
    let list = mock_list(lisp, f, spec)?;
    Ok(lisp.car_safe(list))
}

fn mock_list_family(lisp: &mut Lisp, _f: Value) -> LispResult {
    let alpha = lisp.intern("Alpha");
    let beta = lisp.intern("Beta");
    Ok(lisp.list(&[alpha, beta]))
}

fn mock_open_font(lisp: &mut Lisp, _f: Value, entity: Value, pixel_size: i32) -> LispResult {
    let ty = lisp.font_aref(entity, FONT_TYPE_INDEX);
    let object = lisp.font_build_object(ty, entity, f64::from(pixel_size))?;
    let driver = mock_driver(lisp);
    let font = lisp.font_object_mut(object);
    font.pixel_size = pixel_size;
    font.ascent = pixel_size * 4 / 5;
    font.descent = pixel_size / 5;
    font.height = font.ascent + font.descent;
    font.min_width = pixel_size / 2;
    font.max_width = pixel_size / 2;
    font.average_width = pixel_size / 2;
    font.space_width = pixel_size / 2;
    font.driver = Some(driver);
    Ok(object)
}

fn mock_close_font(lisp: &mut Lisp, _font: Value) {
    bump(lisp, "mock-close-calls");
}

fn mock_has_char(lisp: &mut Lisp, font: Value, c: i32) -> i32 {
    if lisp.font_entity_p(font) {
        i32::from(c < 0x100)
    } else {
        -1
    }
}

fn mock_encode_char(_lisp: &mut Lisp, _font: Value, c: i32) -> u32 {
    if c < 128 { c as u32 } else { FONT_INVALID_CODE }
}

fn mock_text_extents(lisp: &mut Lisp, font: Value, codes: &[u32]) -> FontMetrics {
    let width = lisp.font_object(font).average_width;
    FontMetrics {
        width: width * codes.len() as i32,
        ..FontMetrics::default()
    }
}

fn mock_draw(
    _lisp: &mut Lisp,
    _s: &mut GlyphString,
    _from: usize,
    _to: usize,
    _x: i32,
    _y: i32,
    _with_background: bool,
) -> i32 {
    0
}

fn mock_driver(lisp: &mut Lisp) -> FontDriver {
    FontDriver {
        type_: lisp.intern("mock"),
        case_sensitive: false,
        get_cache: mock_get_cache,
        list: mock_list,
        match_font: mock_match,
        list_family: Some(mock_list_family),
        free_entity: None,
        open_font: mock_open_font,
        close_font: mock_close_font,
        prepare_face: None,
        done_face: None,
        has_char: Some(mock_has_char),
        encode_char: mock_encode_char,
        text_extents: mock_text_extents,
        draw: Some(mock_draw),
        otf_capability: None,
        start_for_frame: None,
        end_for_frame: None,
        filter_properties: None,
        cached_font_ok: None,
        combining_capability: None,
    }
}

fn error_text(lisp: &mut Lisp, r: LispResult<()>) -> String {
    match r {
        Ok(()) => "ok".to_string(),
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            lisp.prin1_to_string(condition)
        }
    }
}

fn setup(lisp: &mut Lisp) {
    eval(
        lisp,
        "(setq mock-font-cache (list t) mock-list-calls 0 mock-close-calls 0)",
    );
}

#[test]
fn registering_font_drivers() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        setup(&mut lisp);
        let f = lisp.frames.selected_frame;
        let driver = mock_driver(&mut lisp);
        let r = lisp.register_font_driver(driver, None);
        assert_eq!(error_text(&mut lisp, r), "ok");
        let r = lisp.register_font_driver(driver, None);
        assert_eq!(
            error_text(&mut lisp, r),
            "(error \"Duplicated font driver: mock\")"
        );
        let undrawable = FontDriver {
            draw: None,
            ..driver
        };
        let r = lisp.register_font_driver(undrawable, Some(f));
        assert_eq!(
            error_text(&mut lisp, r),
            "(error \"Unusable font driver for a frame: mock\")"
        );
        let r = lisp.register_font_driver(driver, Some(f));
        assert_eq!(error_text(&mut lisp, r), "ok");
        let r = lisp.register_font_driver(driver, Some(f));
        assert_eq!(
            error_text(&mut lisp, r),
            "(error \"Duplicated font driver: mock\")"
        );
        assert_eq!(eval(&mut lisp, "mock-font-cache"), "(t)");
        let active = lisp.font_update_drivers(f, sym::T).unwrap();
        assert_eq!(lisp.prin1_to_string(active), "(mock)");
        assert_eq!(eval(&mut lisp, "mock-font-cache"), "(t (mock 1))");
        let none = lisp.intern("none");
        let wanted = lisp.list(&[none]);
        let active = lisp.font_update_drivers(f, wanted).unwrap();
        assert_eq!(lisp.prin1_to_string(active), "(mock)");
        let active = lisp.font_update_drivers(f, Value::NIL).unwrap();
        assert_eq!(lisp.prin1_to_string(active), "nil");
        assert_eq!(eval(&mut lisp, "mock-font-cache"), "(t)");
        let mock = lisp.intern("mock");
        let other = lisp.intern("other");
        lisp.put(mock, sym::FONT_DRIVER_SUPERSEDED_BY, other);
        let active = lisp.font_update_drivers(f, sym::T).unwrap();
        assert_eq!(lisp.prin1_to_string(active), "(mock)");
        assert_eq!(eval(&mut lisp, "mock-font-cache"), "(t (mock 1))");
        assert_eq!(eval(&mut lisp, "(frame-font-cache)"), "nil");
    });
}

#[test]
fn listing_and_opening_fonts_through_a_driver() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        setup(&mut lisp);
        let f = lisp.frames.selected_frame;
        let driver = mock_driver(&mut lisp);
        lisp.register_font_driver(driver, Some(f)).unwrap();
        lisp.font_update_drivers(f, sym::T).unwrap();
        for (source, expected) in [
            (
                "(mapcar (function (lambda (e) (font-get e :weight))) (list-fonts (font-spec :family \"alpha\")))",
                "(normal bold)",
            ),
            ("mock-list-calls", "1"),
            (
                "(mapcar (function (lambda (e) (font-get e :weight))) (list-fonts (font-spec :family \"alpha\")))",
                "(normal bold)",
            ),
            (
                "(mapcar (function (lambda (e) (font-get e :weight))) (list-fonts (font-spec :family \"alpha\" :weight 'bold)))",
                "(bold)",
            ),
            ("mock-list-calls", "1"),
            ("(list-fonts (font-spec :family \"gamma\"))", "nil"),
            ("(list-fonts (font-spec :family \"gamma\"))", "nil"),
            ("mock-list-calls", "2"),
            ("(cdr (nth 2 (nth 1 mock-font-cache)))", "[]"),
            ("(length (list-fonts (font-spec)))", "3"),
            ("(length (list-fonts (font-spec) nil 2))", "2"),
            ("(list-fonts (font-spec) nil 0)", "nil"),
            (
                "(list-fonts (font-spec) nil 'x)",
                "error: (wrong-type-argument fixnump x)",
            ),
            (
                "(list-fonts 'x)",
                "error: (wrong-type-argument font-spec x)",
            ),
            (
                "(font-get (car (list-fonts (font-spec) nil nil (font-spec :weight 'bold))) :weight)",
                "bold",
            ),
            (
                "(font-get (find-font (font-spec :family \"beta\")) :family)",
                "Beta",
            ),
            ("(find-font (font-spec :family \"gamma\"))", "nil"),
            ("(font-family-list)", "(\"Beta\" \"Alpha\")"),
            (
                "(setq beta (open-font (find-font (font-spec :family \"beta\")) 10))",
                "#<font-object \"-*-Beta-normal-normal-normal-*-10-*-*-*-p-*-iso10646-1\">",
            ),
            (
                "(query-font beta)",
                "[\"-*-Beta-normal-normal-normal-*-10-*-*-*-p-*-iso10646-1\" nil 10 5 8 2 5 5 nil]",
            ),
            (
                "(font-xlfd-name beta t)",
                "\"-*-Beta-normal-normal-normal-*-10-*-p-*-iso10646-1\"",
            ),
            ("(fontp beta 'font-object)", "t"),
            ("(font-get beta :size)", "10"),
            (
                "(eq beta (open-font (find-font (font-spec :family \"beta\")) 10))",
                "t",
            ),
            (
                "(eq beta (open-font (find-font (font-spec :family \"beta\")) 12))",
                "nil",
            ),
            ("(font-has-char-p beta ?a)", "t"),
            ("(font-has-char-p beta ?\\x3042)", "nil"),
            (
                "(font-has-char-p (find-font (font-spec :family \"beta\")) ?\\xe9)",
                "t",
            ),
            ("(font-match-p (font-spec :family 'Beta) beta)", "t"),
            ("(font-match-p (font-spec :family 'Alpha) beta)", "nil"),
            ("(font-match-p (font-spec :weight 'bold) beta)", "nil"),
            (
                "(font-match-p beta beta)",
                "error: (wrong-type-argument font-spec #<font-object \"-*-Beta-normal-normal-normal-*-10-*-*-*-p-*-iso10646-1\">)",
            ),
            (
                "(open-font (font-spec))",
                "error: (wrong-type-argument font-entity #<font-spec nil nil nil nil nil nil nil nil nil nil nil nil nil>)",
            ),
            (
                "(query-font (font-spec))",
                "error: (wrong-type-argument font-object #<font-spec nil nil nil nil nil nil nil nil nil nil nil nil nil>)",
            ),
            ("(close-font beta)", "nil"),
            ("mock-close-calls", "1"),
            ("(clear-font-cache)", "nil"),
            ("(length (cdr (nth 1 mock-font-cache)))", "5"),
            (
                "(font-info \"Beta\")",
                "error: (error \"Window system frame should be used\")",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        lisp.font_update_drivers(f, Value::NIL).unwrap();
        assert_eq!(eval(&mut lisp, "mock-close-calls"), "3");
        assert_eq!(eval(&mut lisp, "mock-font-cache"), "(t)");
        assert_eq!(eval(&mut lisp, "(list-fonts (font-spec))"), "nil");
    });
}

fn window_frame(lisp: &mut Lisp) -> (Value, Value) {
    let terminal = lisp.create_terminal(OutputMethod::Pgtk).unwrap();
    lisp.stack.push(terminal);
    lisp.terminal_mut(terminal).name = Some("mock".to_string());
    let name = lisp.string("mock");
    let element = lisp.list(&[name]);
    let mut info = DisplayInfo::new(element);
    info.resx = 96.0;
    info.resy = 96.0;
    lisp.terminal_mut(terminal).display_info = Some(Box::new(info));
    let f = lisp.make_frame(true).unwrap();
    lisp.stack.push(f);
    lisp.frame_mut(f).terminal = terminal;
    lisp.frame_mut(f).output_method = OutputMethod::Pgtk;
    (f, terminal)
}

#[test]
fn opening_fonts_by_name_on_a_window_frame() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        setup(&mut lisp);
        let (f, terminal) = window_frame(&mut lisp);
        let driver = mock_driver(&mut lisp);
        lisp.register_font_driver(driver, Some(f)).unwrap();
        lisp.font_update_drivers(f, sym::T).unwrap();
        let name = lisp.string("Alpha:pixelsize=14");
        let alpha = lisp.font_open_by_name(f, name).unwrap();
        lisp.stack.push(alpha);
        assert_eq!(
            lisp.prin1_to_string(alpha),
            "#<font-object \"-*-Alpha-normal-normal-normal-*-14-*-*-*-m-*-iso10646-1\">"
        );
        assert_eq!(lisp.font_object(alpha).pixel_size, 14);
        let user_spec =
            emacs_core::font::f_font_get(&mut lisp, &[alpha, sym::K_USER_SPEC]).unwrap();
        assert_eq!(lisp.prin1_to_string(user_spec), "\"Alpha:pixelsize=14\"");
        let info = lisp.terminal(terminal).display_info.as_ref().unwrap();
        assert_eq!(
            (
                info.n_fonts,
                info.smallest_char_width,
                info.smallest_font_height
            ),
            (1, 7, 13)
        );
        assert!(lisp.frame(f).fonts_changed);
        lisp.frame_mut(f).fonts_changed = false;
        let name = lisp.string("Beta-12");
        let beta = lisp.font_open_by_name(f, name).unwrap();
        lisp.stack.push(beta);
        assert_eq!(lisp.font_object(beta).pixel_size, 16);
        let info = lisp.terminal(terminal).display_info.as_ref().unwrap();
        assert_eq!(
            (
                info.n_fonts,
                info.smallest_char_width,
                info.smallest_font_height
            ),
            (2, 7, 13)
        );
        assert!(!lisp.frame(f).fonts_changed);
        let name = lisp.string("Gamma-12");
        assert!(lisp.font_open_by_name(f, name).unwrap().is_nil());
        let normal = lisp.intern("normal");
        let mut attrs = [Value::NIL; emacs_core::xfaces::LFACE_VECTOR_SIZE];
        attrs[emacs_core::xfaces::LFACE_WEIGHT_INDEX] = lisp.intern("bold");
        attrs[emacs_core::xfaces::LFACE_SLANT_INDEX] = normal;
        attrs[emacs_core::xfaces::LFACE_SWIDTH_INDEX] = normal;
        attrs[emacs_core::xfaces::LFACE_HEIGHT_INDEX] = Value::fixnum(150);
        let alpha_family = lisp.intern("Alpha");
        let spec =
            emacs_core::font::f_font_spec(&mut lisp, &[sym::K_FAMILY, alpha_family]).unwrap();
        lisp.stack.push(spec);
        let bold = lisp.font_load_for_lface(f, &attrs, spec).unwrap();
        lisp.stack.push(bold);
        assert_eq!(
            lisp.prin1_to_string(bold),
            "#<font-object \"-*-Alpha-bold-normal-normal-*-20-*-*-*-m-*-iso10646-1\">"
        );
        let mock_frame = lisp.intern("mock-frame");
        lisp.set_plain_value(mock_frame, f);
        for (source, expected) in [
            ("(frame-font-cache mock-frame)", "(\"mock\")"),
            (
                "(font-info \"Alpha:pixelsize=20\" mock-frame)",
                "[\"-*-Alpha-normal-normal-normal-*-20-*-*-*-m-*-iso10646-1\" \"Alpha:pixelsize=20:weight=normal:slant=normal:width=normal:spacing=100\" 20 20 0 0 0 10 16 4 10 10 nil nil]",
            ),
            ("(font-info \"Gamma\" mock-frame)", "nil"),
            (
                "(aref (font-info (open-font (find-font (font-spec :family \"Beta\") mock-frame) 30 mock-frame) mock-frame) 2)",
                "30",
            ),
            (
                "(equal (font-spec :family \"a\" :weight 'bold) (font-spec :family \"a\" :weight 'bold))",
                "t",
            ),
            (
                "(equal (font-spec :family \"a\") (font-spec :family \"b\"))",
                "nil",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}
