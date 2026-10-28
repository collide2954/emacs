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
