use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_gui::display::Display;
use iced::futures::channel::mpsc as fmpsc;

const STACK_SIZE: usize = 512 << 20;

fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, f)
            .expect("spawn test thread")
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

fn display() -> Display {
    let (commands, _command_rx) = fmpsc::unbounded();
    let (_notice_tx, notices) = mpsc::channel();
    Display::new(commands, notices)
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

fn booted() -> Lisp {
    let mut lisp = Lisp::boot(Startup::program(true));
    emacs_gui::install(&mut lisp, display());
    let f = lisp.frames.selected_frame;
    let driver = emacs_gui::ftcrfont::ftcrfont_driver(&mut lisp);
    lisp.register_font_driver(driver, Some(f)).unwrap();
    lisp.font_update_drivers(f, sym::T).unwrap();
    lisp
}

#[test]
fn the_ftcr_driver_is_registered_globally() {
    on_big_stack(|| {
        let mut lisp = booted();
        let types: Vec<String> = lisp
            .fonts
            .driver_list
            .clone()
            .iter()
            .map(|driver| lisp.symbol_name_string(driver.type_))
            .collect();
        assert_eq!(types, ["ftcr"]);
        assert_eq!(eval(&mut lisp, "(frame-font-cache)"), "nil");
    });
}

#[test]
fn listing_fonts_through_fontdb() {
    on_big_stack(|| {
        let mut lisp = booted();
        for (source, expected) in [
            (
                "(let ((e (car (list-fonts (font-spec :family \"monospace\"))))) (list (font-get e :type) (font-get e :registry) (font-get e :spacing) (font-get e :size) (font-get e :avgwidth) (stringp (car (font-get e :font-entity))) (integerp (cdr (font-get e :font-entity)))))",
                "(ftcr iso10646-1 100 0 0 t t)",
            ),
            (
                "(setq mono (find-font (font-spec :family \"monospace\" :weight 'normal :slant 'normal :width 'normal)))",
                "",
            ),
            ("(font-get mono :weight)", "regular"),
            ("(font-get mono :slant)", "normal"),
            ("(font-get mono :width)", "normal"),
            (
                "(and (member (symbol-name (font-get mono :family)) (font-family-list)) t)",
                "t",
            ),
            (
                "(list-fonts (font-spec :family \"No Such Family Anywhere\"))",
                "nil",
            ),
            (
                "(and (list-fonts (font-spec :family (font-get mono :family) :registry 'iso8859-1)) t)",
                "t",
            ),
            (
                "(list-fonts (font-spec :family (font-get mono :family) :registry 'no-such-registry))",
                "nil",
            ),
            ("(font-has-char-p mono ?A)", "t"),
            ("(font-has-char-p mono #x10FFFD)", "nil"),
            (
                "(let ((fams (mapcar (function (lambda (e) (font-get e :family))) (list-fonts (font-spec :spacing 100))))) (and fams (null (memq nil fams)) t))",
                "t",
            ),
        ] {
            let got = eval(&mut lisp, source);
            if expected.is_empty() {
                assert!(
                    got.starts_with("#<font-entity ftcr "),
                    "{source} gave {got}"
                );
            } else {
                assert_eq!(got, expected, "evaluating {source}");
            }
        }
    });
}

#[test]
fn opening_fonts_through_cosmic_text() {
    on_big_stack(|| {
        let mut lisp = booted();
        for (source, expected) in [
            (
                "(progn (setq mono (find-font (font-spec :family \"monospace\" :weight 'normal :slant 'normal :width 'normal)) o (open-font mono 16)) (fontp o 'font-object))",
                "t",
            ),
            (
                "(let ((q (query-font o))) (list (stringp (aref q 0)) (stringp (aref q 1)) (aref q 2) (= (aref q 3) (aref q 6) (aref q 7)) (> (aref q 4) 0) (> (aref q 5) 0) (aref q 8)))",
                "(t t 16 t t t nil)",
            ),
            (
                "(and (string-match \"-regular-normal-normal-\\\\*-16-\\\\*-\\\\*-\\\\*-m-0-iso10646-1\\\\'\" (font-xlfd-name o)) t)",
                "t",
            ),
            (
                "(equal (aref (query-font o) 1) (car (font-get mono :font-entity)))",
                "t",
            ),
            ("(font-has-char-p o ?A)", "t"),
            ("(font-has-char-p o #x10FFFD)", "nil"),
            (
                "(font-match-p (font-spec :family (font-get mono :family)) o)",
                "t",
            ),
            ("(eq o (open-font mono 16))", "t"),
            ("(eq o (open-font mono 17))", "nil"),
            ("(close-font o)", "nil"),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        let name = lisp.string("monospace:pixelsize=16");
        let f = lisp.frames.selected_frame;
        let o = lisp.font_open_by_name(f, name).unwrap();
        assert!(lisp.font_object_p(o));
        let glyph = lisp.font_encode_char(o, 'A' as i32);
        assert_ne!(glyph, emacs_core::font::FONT_INVALID_CODE);
        let space = lisp.font_encode_char(o, ' ' as i32);
        let driver = lisp.font_object(o).driver.unwrap();
        let metrics = (driver.text_extents)(&mut lisp, o, &[glyph, space]);
        let space_width = lisp.font_object(o).space_width;
        assert_eq!(metrics.width, 2 * space_width);
        assert!(metrics.ascent > 0 && metrics.rbearing > metrics.lbearing);
    });
}
