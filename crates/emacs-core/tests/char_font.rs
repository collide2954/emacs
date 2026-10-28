mod common;

use common::{check, eval, on_big_stack};
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;

fn value(lisp: &mut Lisp, source: &str) -> Value {
    lisp.eval_string(source)
        .unwrap_or_else(|_| panic!("evaluating {source}"))
}

#[test]
fn internal_char_font_batch() {
    check(&[
        ("(internal-char-font nil 97)", "nil"),
        (
            "(internal-char-font nil -1)",
            "error: (wrong-type-argument characterp -1)",
        ),
        ("(internal-char-font 1)", "error: (args-out-of-range 1 1 1)"),
    ]);
}

#[test]
fn internal_char_font_initial_frame() {
    on_big_stack(|| {
        let mut lisp = Lisp::new_with(false);
        for (source, expected) in [
            ("(progn (insert \"abc\") (internal-char-font 1))", "nil"),
            ("(internal-char-font 2 120)", "nil"),
            (
                "(internal-char-font 1 -1)",
                "error: (wrong-type-argument wholenump -1)",
            ),
            ("(internal-char-font 3 4194304)", "nil"),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn overlays_and_faces_at_position() {
    on_big_stack(|| {
        let mut lisp = Lisp::new_with(false);
        value(
            &mut lisp,
            "(progn (insert \"abcdef\") (setq o1 (make-overlay 1 4) o2 (make-overlay 2 6) o3 (make-overlay 2 6) o4 (make-overlay 3 3)) (overlay-put o2 'priority '(5 . 1)) (overlay-put o3 'priority 5) (overlay-put o1 'face '(:weight bold)) nil)",
        );
        let o: Vec<Value> = ["o1", "o2", "o3"]
            .iter()
            .map(|s| value(&mut lisp, s))
            .collect();
        assert_eq!(lisp.overlays_at(2), (o.clone(), 3));
        assert_eq!(lisp.overlays_at(1), (vec![o[0]], 2));
        assert_eq!(lisp.overlays_at(5), (vec![o[1], o[2]], 7));
        assert_eq!(lisp.sort_overlays(&o, None), vec![o[0], o[2], o[1]]);
        let w = value(&mut lisp, "(selected-window)");
        let (bold, end) = lisp
            .face_at_buffer_position(w, 1, 101, false, None, 0)
            .expect("face at 1");
        assert_ne!(bold, 0);
        assert_eq!(end, 2);
        let plain = lisp
            .face_at_buffer_position(w, 5, 105, false, None, 0)
            .expect("face at 5");
        assert_eq!(plain, (0, 6));
    });
}
