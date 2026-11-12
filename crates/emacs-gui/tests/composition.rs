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
        for (source, expected) in [
            (
                "(font-shape-gstring (composition-get-gstring 0 2 font \"e\\u0301\") nil)",
                "nil",
            ),
            (
                "(font-shape-gstring 1 nil)",
                "error: (error \"Invalid glyph-string: \" 1)",
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
