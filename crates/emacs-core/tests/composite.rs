mod common;

use common::{check, eval, on_big_stack};
use emacs_core::composite::CompositionIt;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;

#[test]
fn long_lines_narrow_the_backward_scans_for_compositions() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (set-buffer \"*scratch*\") (erase-buffer) (insert \"x\" (make-string 3000 ?a) \"y\") (setq unicode-category-table (make-char-table nil 1)) (point))"
            ),
            "3003"
        );
        let w = lisp.windows.selected_window;
        let mut cmp_it = CompositionIt {
            parent_w: w,
            ..CompositionIt::default()
        };
        lisp.composition_compute_stop_pos(&mut cmp_it, 2950, -1, Value::NIL, true)
            .unwrap();
        assert_eq!(cmp_it.stop_pos, 1);
        lisp.cur_mut().long_line_optimizations_p = true;
        let near = lisp.get_small_narrowing_begv(w, 2950).unwrap();
        assert!(near > 2 && near < 2950, "{near}");
        let mut cmp_it = CompositionIt {
            parent_w: w,
            ..CompositionIt::default()
        };
        lisp.composition_compute_stop_pos(&mut cmp_it, 2950, -1, Value::NIL, true)
            .unwrap();
        assert_eq!(cmp_it.stop_pos, near + 1);
        lisp.cur_mut().long_line_optimizations_p = false;
        let find = "(progn (set-char-table-range composition-function-table ?a (list (vector \"a+\" 0 'ignore))) (let ((auto-composition-function (function (lambda (f from to font string dir) (copy-sequence (composition-get-gstring from to nil string)))))) (let ((r (find-composition-internal 2950 nil nil nil))) (list (car r) (nth 1 r)))))";
        assert_eq!(eval(&mut lisp, find), "(2 3002)");
        lisp.cur_mut().long_line_optimizations_p = true;
        let pt = lisp.window_point(w);
        let head = lisp.get_small_narrowing_begv(w, pt).unwrap();
        assert!(head > 2 && head < 2950, "{head}");
        assert_eq!(eval(&mut lisp, find), format!("({head} 3002)"));
    });
}

#[test]
fn static_compositions() {
    check(&[
        (
            "text-property-default-nonsticky",
            "((composition . t) (syntax-table . t) (display . t))",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (compose-string-internal s 0 2) (find-composition-internal 0 nil s nil))",
            "(0 2 t)",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (compose-string-internal s 0 2) (find-composition-internal 1 nil s t))",
            "(0 2 [97 98] t nil 1)",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (compose-string-internal s 0 2 \"x\") (find-composition-internal 0 nil s t))",
            "(0 2 [120] t nil 1)",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (compose-string-internal s 0 2 [97 12 98]) (find-composition-internal 0 nil s t))",
            "(0 2 [97 12 98] nil nil 2)",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (compose-string-internal s 0 2 [97 98]) (find-composition-internal 0 nil s t))",
            "(0 2 [97 98] t nil 1)",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (compose-string-internal s 0 2 [99 100]) (find-composition-internal 0 nil s t))",
            "(0 2)",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (compose-string-internal s 1 3) (find-composition-internal 0 3 s nil))",
            "nil",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (compose-string-internal s 1 3) (insert \"xyz\") (find-composition-internal 0 3 s nil))",
            "(1 3 t)",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (compose-string-internal s 0 1) (find-composition-internal 3 0 s nil))",
            "(0 1 t)",
        ),
        (
            "(find-composition-internal 9 nil \"abc\" nil)",
            "error: (args-out-of-range \"abc\" 9)",
        ),
        (
            "(compose-region-internal 1 1 'sym)",
            "error: (wrong-type-argument vectorp sym)",
        ),
    ]);
}

#[test]
fn buffer_compositions() {
    check(&[
        (
            "(save-current-buffer (set-buffer (get-buffer-create \"cmp\")) (insert \"abcd\") (compose-region-internal 2 4) (list (find-composition-internal 2 nil nil nil) (find-composition-internal 1 3 nil nil) (find-composition-internal 4 1 nil nil) (find-composition-internal 1 nil nil nil)))",
            "((2 4 t) (2 4 t) (2 4 t) nil)",
        ),
        (
            "(save-current-buffer (set-buffer \"cmp\") (goto-char 3) (insert \"X\") (find-composition-internal 2 nil nil nil))",
            "(2 3 nil)",
        ),
        (
            "(save-current-buffer (set-buffer \"cmp\") (erase-buffer) (insert \"abcd\") (compose-region-internal 2 4 nil 'xc-mod) (defalias 'xc-mod (function (lambda (from to) (setq xc-called (list from to))))) (setq xc-called nil) (goto-char 3) (insert \"Y\") xc-called)",
            "nil",
        ),
        (
            "(save-current-buffer (set-buffer \"cmp\") (erase-buffer) (insert \"abcd\") (compose-region-internal 2 4 nil 'xc-mod) (setq xc-called nil) (goto-char 4) (insert \"Y\") xc-called)",
            "(2 4)",
        ),
    ]);
}

#[test]
fn glyph_strings() {
    check(&[
        (
            "(let ((g (composition-get-gstring 0 2 nil \"ab\"))) (list (length g) (aref (aref g 0) 1) (aref (aref g 0) 2) (aref g 1) (aref (aref g 2) 2) (aref (aref g 2) 4) (aref g 4)))",
            "(10 97 98 nil 97 1 nil)",
        ),
        (
            "(composition-get-gstring 0 0 nil \"ab\")",
            "error: (error \"Attempt to shape zero-length text\")",
        ),
        (
            "(composition-get-gstring 0 1 nil (string-to-unibyte \"\\377\"))",
            "error: (error \"Attempt to shape unibyte text\")",
        ),
        ("(clear-composition-cache)", "nil"),
        (
            "(composition-sort-rules (list [nil 0 f] [nil 2 g] [nil 1 h]))",
            "([nil 2 g] [nil 1 h] [nil 0 f])",
        ),
        ("(composition-sort-rules (list [nil 0 f]))", "([nil 0 f])"),
        (
            "(composition-sort-rules (list [nil 0 f] 3))",
            "error: (error \"Invalid composition rule in RULES argument\")",
        ),
    ]);
}

#[test]
fn automatic_composition() {
    check(&[(
        "(save-current-buffer (set-buffer \"*scratch*\") (erase-buffer) (insert \"xaby\") (set-char-table-range composition-function-table ?a (list (vector \"ab\" 0 'ignore))) (let ((unicode-category-table (make-char-table nil 1)) (auto-composition-function (function (lambda (f from to font string dir) (copy-sequence (composition-get-gstring from to nil string)))))) (let ((r (find-composition-internal 2 nil nil nil))) (list (car r) (nth 1 r) (vectorp (nth 2 r)) (point)))))",
        "(2 4 t 5)",
    )]);
}
