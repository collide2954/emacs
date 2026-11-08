mod common;

use common::check;

fn in_buffer(text: &str, body: &str) -> String {
    format!(
        "(save-current-buffer (set-buffer (get-buffer-create \"ind\")) (erase-buffer) (insert {text:?}) {body})"
    )
}

fn run(cases: &[(String, &str)]) {
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    check(&cases);
}

#[test]
fn columns() {
    run(&[
        (in_buffer("abc", "(current-column)"), "3"),
        (in_buffer("a\tb", "(current-column)"), "9"),
        (in_buffer("x\ny\tz", "(current-column)"), "9"),
        (in_buffer("", "(insert 1) (current-column)"), "2"),
        (
            in_buffer(
                "",
                "(insert 1) (setq ctl-arrow nil) (prog1 (current-column) (setq ctl-arrow t))",
            ),
            "4",
        ),
        (in_buffer("\u{e9}a", "(current-column)"), "2"),
        (
            in_buffer(
                "abcdef",
                "(put-text-property 3 5 'invisible t) (current-column)",
            ),
            "4",
        ),
        (
            in_buffer(
                "ab",
                "(put-text-property 1 2 'display \"XYZ\") (current-column)",
            ),
            "4",
        ),
        (
            in_buffer(
                "ab",
                "(put-text-property 1 2 'display '(space :width 5)) (current-column)",
            ),
            "6",
        ),
        (
            in_buffer(
                "ab",
                "(put-text-property 1 2 'display '(space :align-to 10)) (current-column)",
            ),
            "11",
        ),
        (
            in_buffer("abc", "(compose-region-internal 1 3) (current-column)"),
            "2",
        ),
    ]);
}

#[test]
fn indentation() {
    run(&[
        (in_buffer("\tx", "(current-indentation)"), "8"),
        (in_buffer("  foo", "(current-indentation)"), "2"),
        (in_buffer("foo\n   bar", "(current-indentation)"), "3"),
        (
            in_buffer("", "(list (indent-to 10) (buffer-string))"),
            "(10 \"\t  \")",
        ),
        (
            in_buffer(
                "",
                "(list (let ((indent-tabs-mode nil)) (indent-to 3)) (buffer-string))",
            ),
            "(3 \"   \")",
        ),
        (in_buffer("", "(indent-to 2 3)"), "3"),
        (in_buffer("abc", "(indent-to 2)"), "3"),
    ]);
}

#[test]
fn moving_to_columns() {
    run(&[
        (
            in_buffer("abcdefg", "(list (move-to-column 4) (point))"),
            "(4 5)",
        ),
        (
            in_buffer("abc", "(goto-char 1) (list (move-to-column 20) (point))"),
            "(3 4)",
        ),
        (
            in_buffer(
                "abc",
                "(goto-char 1) (list (move-to-column 6 t) (buffer-string))",
            ),
            "(6 \"abc   \")",
        ),
        (
            in_buffer("a\tb", "(goto-char 1) (list (move-to-column 4) (point))"),
            "(8 3)",
        ),
        (
            in_buffer(
                "a\tb",
                "(goto-char 1) (list (move-to-column 4 t) (point) (buffer-string))",
            ),
            "(4 5 \"a   \tb\")",
        ),
        (
            in_buffer("abc", "(move-to-column -1)"),
            "error: (wrong-type-argument wholenump -1)",
        ),
    ]);
}

#[test]
fn compute_motion() {
    run(&[
        (
            in_buffer(
                "abc\tdef\nxyz",
                "(compute-motion 1 '(0 . 0) 12 nil nil nil nil)",
            ),
            "(12 3 1 2 nil)",
        ),
        (
            in_buffer(
                "abc\tdef\nxyz",
                "(let ((truncate-partial-width-windows nil)) (compute-motion 1 '(0 . 0) 12 nil 5 nil nil))",
            ),
            "(12 3 3 2 nil)",
        ),
        (
            in_buffer(
                "abc\tdef\nxyz",
                "(compute-motion 1 '(0 . 0) 12 '(2 . 1) nil nil nil)",
            ),
            "(11 2 1 1 nil)",
        ),
        (
            in_buffer(
                "abc",
                "(delete-all-overlays) (overlay-put (make-overlay 2 2) 'before-string \"XY\") (prog1 (compute-motion 1 '(0 . 0) 4 nil nil nil nil) (delete-all-overlays))",
            ),
            "(4 5 0 4 nil)",
        ),
        (
            in_buffer("a\nb", "(compute-motion 1 '(0 . 0) 4 nil nil '(3 . 0) nil)"),
            "(4 -1 1 -2 nil)",
        ),
        (
            in_buffer(
                "a\n  x\nb",
                "(setq selective-display 1) (prog1 (compute-motion 1 '(0 . 0) 8 nil nil nil nil) (setq selective-display nil))",
            ),
            "(8 1 1 0 nil)",
        ),
        (
            in_buffer("abc", "(compute-motion 9 '(0 . 0) 1 nil nil nil nil)"),
            "error: (args-out-of-range 9 1 4)",
        ),
        (
            in_buffer("abc", "(compute-motion 1 '(0 . 0) 1 nil nil '(-1 . 0) nil)"),
            "error: (args-out-of-range -1 0)",
        ),
        (
            in_buffer("abc", "(compute-motion 1 '(0 . x) 1 nil nil nil nil)"),
            "error: (wrong-type-argument fixnump x)",
        ),
    ]);
}

const MOTION_TEXT: &str = "ab\ncd\nef";

fn long_line() -> String {
    "x".repeat(100)
}

#[test]
fn vertical_motion_in_batch_uses_vmotion() {
    run(&[
        (
            in_buffer(
                MOTION_TEXT,
                "(goto-char (point-max)) (list (vertical-motion -1) (point))",
            ),
            "(-1 4)",
        ),
        (
            in_buffer(
                MOTION_TEXT,
                "(goto-char 1) (list (vertical-motion 1) (point))",
            ),
            "(1 4)",
        ),
        (
            in_buffer(
                MOTION_TEXT,
                "(goto-char 1) (list (vertical-motion 5) (point))",
            ),
            "(2 9)",
        ),
        (
            in_buffer(
                MOTION_TEXT,
                "(goto-char 5) (list (vertical-motion 0) (point))",
            ),
            "(0 4)",
        ),
        (
            in_buffer(
                MOTION_TEXT,
                "(goto-char 1) (list (vertical-motion '(2 . 1)) (point))",
            ),
            "(1 4)",
        ),
        (
            in_buffer(
                &long_line(),
                "(goto-char 1) (list (vertical-motion 1) (point))",
            ),
            "(1 80)",
        ),
        (
            in_buffer(MOTION_TEXT, "(vertical-motion 'x)"),
            "error: (wrong-type-argument fixnump x)",
        ),
        (
            in_buffer(MOTION_TEXT, "(vertical-motion '(x . 1))"),
            "error: (wrong-type-argument numberp x)",
        ),
    ]);
}

#[test]
fn vertical_motion_interactively_uses_the_display_iterator() {
    common::on_big_stack(|| {
        let mut lisp = emacs_core::lisp::Lisp::new_with(false);
        let cases = [
            (
                in_buffer(
                    MOTION_TEXT,
                    "(goto-char (point-max)) (list (vertical-motion -1) (point))",
                ),
                "(-1 4)",
            ),
            (
                in_buffer(
                    MOTION_TEXT,
                    "(goto-char 1) (list (vertical-motion 1) (point))",
                ),
                "(1 4)",
            ),
            (
                in_buffer(
                    MOTION_TEXT,
                    "(goto-char 1) (list (vertical-motion 5) (point))",
                ),
                "(2 9)",
            ),
            (
                in_buffer(
                    MOTION_TEXT,
                    "(goto-char 5) (list (vertical-motion 0) (point))",
                ),
                "(0 4)",
            ),
            (
                in_buffer(
                    MOTION_TEXT,
                    "(goto-char 1) (list (vertical-motion '(1 . 1)) (point))",
                ),
                "(1 5)",
            ),
            (
                in_buffer(
                    &long_line(),
                    "(goto-char 1) (list (vertical-motion 1) (point))",
                ),
                "(1 80)",
            ),
            (
                in_buffer(
                    &long_line(),
                    "(goto-char 90) (list (vertical-motion -1) (point))",
                ),
                "(-1 1)",
            ),
            (
                in_buffer(
                    "",
                    "(insert \"a\" 1 \"b\") (goto-char 1) (list (vertical-motion '(3 . 0)) (point))",
                ),
                "(0 3)",
            ),
            (
                in_buffer(
                    "\tx",
                    "(goto-char 1) (list (vertical-motion '(8 . 0)) (point) (vertical-motion '(4 . 0)) (point))",
                ),
                "(0 2 0 1)",
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(
                common::eval(&mut lisp, &source),
                expected,
                "evaluating {source}"
            );
        }
    });
}

#[test]
fn vertical_motion_with_line_numbers() {
    common::on_big_stack(|| {
        let mut lisp = emacs_core::lisp::Lisp::new_with(false);
        let cases = [
            (
                in_buffer(
                    &long_line(),
                    "(setq display-line-numbers t) (goto-char 1) (prog1 (list (vertical-motion 1) (point)) (setq display-line-numbers nil))",
                ),
                "(1 76)",
            ),
            (
                "(save-current-buffer (set-buffer (window-buffer)) (setq display-line-numbers t) (prog1 (list (line-number-display-width) (line-number-display-width t) (line-number-display-width 'columns)) (setq display-line-numbers nil)))".to_string(),
                "(2 4 4.0)",
            ),
            (
                in_buffer(
                    MOTION_TEXT,
                    "(setq display-line-numbers 'relative) (goto-char 1) (prog1 (list (vertical-motion 2) (point)) (setq display-line-numbers nil))",
                ),
                "(2 7)",
            ),
            (
                in_buffer(MOTION_TEXT, "(line-number-display-width)"),
                "0",
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(
                common::eval(&mut lisp, &source),
                expected,
                "evaluating {source}"
            );
        }
    });
}

#[test]
fn garbage_collection_waits_while_the_display_iterator_is_live() {
    common::on_big_stack(|| {
        let mut lisp = emacs_core::lisp::Lisp::new_with(false);
        let source = in_buffer(
            MOTION_TEXT,
            "(setq gc-during-display 'unset) (setq fontification-functions (list (function (lambda (pos) (setq gc-during-display (garbage-collect)) (put-text-property pos (point-max) 'fontified t))))) (goto-char 1) (prog1 (list (vertical-motion 1) gc-during-display (consp (garbage-collect))) (setq fontification-functions nil))",
        );
        assert_eq!(common::eval(&mut lisp, &source), "(1 nil t)");
    });
}

#[test]
fn invisibility() {
    run(&[
        ("(invisible-p t)".to_string(), "t"),
        ("(invisible-p nil)".to_string(), "nil"),
        (
            in_buffer(
                "abc",
                "(setq buffer-invisibility-spec '((foo . t) bar)) (prog1 (list (invisible-p 'foo) (invisible-p 'bar) (invisible-p 'baz) (invisible-p '(baz foo))) (setq buffer-invisibility-spec t))",
            ),
            "(2 t nil 2)",
        ),
        (
            in_buffer(
                "abc",
                "(put-text-property 2 3 'face 'bold) (list (next-property-change 1 nil t) (next-property-change 3 nil t))",
            ),
            "(2 4)",
        ),
        (in_buffer("abc", "(next-property-change 1 nil t)"), "4"),
    ]);
}
