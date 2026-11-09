use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
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

fn pgtk_frame_selected() -> (Lisp, Value) {
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(
        &mut lisp,
        Display::new(fmpsc::unbounded().0, mpsc::channel().1),
    );
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-range char-script-table '(#x2190 . #x21FF) 'symbol) (set-char-table-extra-slot char-script-table 0 '(latin symbol)))",
    );
    assert_eq!(
        eval(
            &mut lisp,
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"c\") (width . 20) (height . 5)))) (select-frame f) (setq font (internal-get-lisp-face-attribute 'default :font f)) (fontp font 'font-object))"
        ),
        "t"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    (lisp, f)
}

#[test]
fn internal_char_font_on_a_gui_frame() {
    on_big_stack(|| {
        let (mut lisp, f) = pgtk_frame_selected();
        let font = lisp.face_from_id(f, DEFAULT_FACE_ID).unwrap().font;
        let code = lisp.font_encode_char(font, 'a' as i32);
        assert_eq!(
            eval(&mut lisp, "(cdr (internal-char-font nil ?a))"),
            code.to_string()
        );
        for (source, expected) in [
            (
                "(let ((r (internal-char-font nil ?a))) (list (eq (car r) font) (integerp (cdr r))))",
                "(t t)",
            ),
            (
                "(let ((r (internal-char-font nil #xE9))) (list (fontp (car r) 'font-object) (integerp (cdr r))))",
                "(t t)",
            ),
            ("(internal-char-font nil #x10FFFF)", "nil"),
            ("(internal-char-font nil #x110000)", "nil"),
            (
                "(internal-char-font nil #x400000)",
                "error: (wrong-type-argument characterp 4194304)",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn messages_on_a_new_frame_are_logged() {
    on_big_stack(|| {
        let (mut lisp, f) = pgtk_frame_selected();
        assert!(lisp.frame(f).flags.glyphs_initialized_p);
        assert_eq!(
            eval(
                &mut lisp,
                "(list (message \"hello %d\" 42) (save-current-buffer (set-buffer \"*Messages*\") (and (string-match \"hello 42\" (buffer-string)) t)))"
            ),
            "(\"hello 42\" t)"
        );
    });
}

#[test]
fn face_for_char_realizes_and_reuses_non_ascii_faces() {
    on_big_stack(|| {
        let (mut lisp, f) = pgtk_frame_selected();
        assert_eq!(
            lisp.face_for_char(f, DEFAULT_FACE_ID, 'a' as u32, -1, Value::NIL)
                .unwrap(),
            DEFAULT_FACE_ID
        );
        assert_eq!(
            lisp.face_for_char(f, DEFAULT_FACE_ID, 0x3FFF80, -1, Value::NIL)
                .unwrap(),
            DEFAULT_FACE_ID
        );
        let e_acute = lisp
            .face_for_char(f, DEFAULT_FACE_ID, 0xE9, -1, Value::NIL)
            .unwrap();
        assert_ne!(e_acute, DEFAULT_FACE_ID);
        let face = lisp.face_from_id(f, e_acute).unwrap();
        assert_eq!(face.ascii_face, DEFAULT_FACE_ID);
        assert!(!face.font.is_nil());
        assert_eq!(
            lisp.face_for_char(f, DEFAULT_FACE_ID, 0xE9, -1, Value::NIL)
                .unwrap(),
            e_acute
        );
        let none = lisp
            .face_for_char(f, DEFAULT_FACE_ID, 0x10FFFF, -1, Value::NIL)
            .unwrap();
        let face = lisp.face_from_id(f, none).unwrap();
        assert!(face.font.is_nil());
        assert_eq!(face.ascii_face, DEFAULT_FACE_ID);
        assert_eq!(
            lisp.face_for_char(f, DEFAULT_FACE_ID, 0x10FFFE, -1, Value::NIL)
                .unwrap(),
            none
        );
        let default_font = lisp.face_from_id(f, DEFAULT_FACE_ID).unwrap().font;
        let arrow = lisp
            .face_for_char(f, DEFAULT_FACE_ID, 0x2192, -1, Value::NIL)
            .unwrap();
        if lisp.font_has_char(f, default_font, 0x2192) > 0 {
            assert_eq!(arrow, DEFAULT_FACE_ID);
        } else {
            assert_ne!(arrow, DEFAULT_FACE_ID);
        }
    });
}
