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
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
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
fn gstrings_filled_from_a_font() {
    on_big_stack(|| {
        let (mut lisp, f) = pgtk_frame_selected();
        let font = lisp.face_from_id(f, DEFAULT_FACE_ID).unwrap().font;
        let code = lisp.font_encode_char(font, 'e' as i32);
        assert_eq!(
            eval(
                &mut lisp,
                "(let ((g (composition-get-gstring 0 2 font \"e\\u0301\"))) (list (eq (aref (aref g 0) 0) font) (aref (aref g 2) 0) (aref (aref g 2) 1) (aref (aref g 2) 2) (aref (aref g 3) 2) (aref (aref g 2) 3) (> (aref (aref g 2) 4) 0) (> (aref (aref g 2) 7) 0) (aref (aref g 2) 9) (aref g 1)))"
            ),
            format!("(t 0 0 101 769 {code} t t nil nil)")
        );
        assert_eq!(
            eval(&mut lisp, "(font-shape-gstring 1 nil)"),
            "error: (error \"Invalid glyph-string: \" 1)"
        );
    });
}

#[test]
fn the_harfbuzz_driver_shapes_gstrings() {
    on_big_stack(|| {
        let (mut lisp, f) = pgtk_frame_selected();
        let font = lisp.face_from_id(f, DEFAULT_FACE_ID).unwrap().font;
        let (a, b) = (
            lisp.font_encode_char(font, 'a' as i32),
            lisp.font_encode_char(font, 'b' as i32),
        );
        for (source, expected) in [
            ("(frame-parameter f 'font-backend)", "(ftcrhb)".to_string()),
            ("(font-get font :type)", "ftcrhb".to_string()),
            (
                "(let ((g (font-shape-gstring (composition-get-gstring 0 2 font \"e\\u0301\") nil)) (to 0) (i 2)) (while (and (< i (length g)) (aref g i)) (setq to (max to (aref (aref g i) 1)) i (1+ i))) (list (vectorp g) (integerp (aref g 1)) (aref (aref g 2) 0) to))",
                "(t t 0 1)".to_string(),
            ),
            (
                "(let ((g (font-shape-gstring (composition-get-gstring 0 2 font \"ab\") nil))) (list (aref (aref g 2) 0) (aref (aref g 2) 1) (aref (aref g 2) 2) (aref (aref g 2) 3) (aref (aref g 2) 9) (aref (aref g 3) 0) (aref (aref g 3) 1) (aref (aref g 3) 2) (aref (aref g 3) 3) (aref (aref g 3) 9) (= (aref (aref g 2) 4) (aref (aref g 3) 4))))",
                format!("(0 0 97 {a} nil 1 1 98 {b} nil t)"),
            ),
            (
                "(let ((g (font-shape-gstring (composition-get-gstring 1 3 font \"xab\") 'R2L))) (list (aref (aref g 2) 2) (aref (aref g 2) 3) (aref (aref g 3) 2) (aref (aref g 3) 3)))",
                format!("(97 {a} 98 {b})"),
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn automatic_compositions_get_a_font_object() {
    on_big_stack(|| {
        let (mut lisp, _f) = pgtk_frame_selected();
        assert_eq!(
            eval(
                &mut lisp,
                "(save-current-buffer (set-buffer \"*scratch*\") (erase-buffer) (insert \"xe\\u0301y\") (set-char-table-range composition-function-table #x301 (list (vector \"e.\" 1 'ignore))) (setq got 'unset) (let ((unicode-category-table (make-char-table nil 1)) (auto-composition-function (function (lambda (func from to font-object string dir) (setq got (list from to (eq font-object font) (fontp font-object 'font-object))) nil)))) (list (find-composition-internal 3 nil nil nil) (equal got (if (font-has-char-p font #x301) '(2 4 t t) 'unset)))))"
            ),
            "(nil t)"
        );
    });
}
