mod common;

use common::check;

fn in_buffer(text: &str, body: &str) -> String {
    format!(
        "(save-current-buffer (set-buffer (get-buffer-create \"sx\")) (erase-buffer) (insert {text:?}) {body})"
    )
}

fn with_comments(text: &str, body: &str) -> String {
    in_buffer(
        text,
        &format!(
            "(let ((st (copy-syntax-table))) (modify-syntax-entry ?\\; \"<\" st) (modify-syntax-entry ?\\n \">\" st) (set-syntax-table st)) {body}"
        ),
    )
}

#[test]
fn skip_chars() {
    let cases = [
        (
            in_buffer(
                "abc123 def",
                "(goto-char 1) (list (skip-chars-forward \"a-z\") (point) (skip-chars-forward \"^ \") (point) (skip-chars-backward \"0-9\") (point))",
            ),
            "(3 4 3 7 -3 4)",
        ),
        (
            in_buffer(
                "  foo",
                "(goto-char 1) (list (skip-chars-forward \"[:space:]\") (point))",
            ),
            "(2 3)",
        ),
        (
            in_buffer(
                "abcdef",
                "(goto-char 1) (list (skip-chars-forward \"a-z\" 3) (point) (skip-chars-backward \"a-z\" -5) (point))",
            ),
            "(2 3 -2 1)",
        ),
        (
            in_buffer(
                "\u{e9}\u{e9}a",
                "(goto-char 1) (list (skip-chars-forward \"\u{e9}\") (point) (progn (goto-char 1) (skip-chars-forward \"^a\")))",
            ),
            "(2 3 2)",
        ),
        (
            in_buffer("abc", "(skip-chars-forward \"[:foo:]\")"),
            "error: (error \"Invalid ISO C character class\")",
        ),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    check(&cases);
}

#[test]
fn skip_syntax() {
    let cases = [
        (
            in_buffer(
                "abc123 def",
                "(goto-char 1) (list (skip-syntax-forward \"w\") (point) (skip-syntax-forward \"w\") (skip-syntax-backward \"^ \") (point))",
            ),
            "(6 7 0 -6 1)",
        ),
        (
            in_buffer(
                "a_b-c d",
                "(goto-char 1) (list (skip-syntax-forward \"w_\") (point))",
            ),
            "(5 6)",
        ),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    check(&cases);
}

#[test]
fn words() {
    let cases = [(
        in_buffer(
            "abc123 def",
            "(goto-char 1) (list (forward-word) (point) (forward-word) (point) (forward-word) (point) (forward-word -1) (point) (forward-word -5) (point))",
        ),
        "(t 7 t 11 nil 11 t 8 nil 1)",
    )];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    check(&cases);
}

#[test]
fn sexps() {
    let text = "(a (b c) \"d)\" e) f";
    let cases = [
        (
            in_buffer(
                text,
                "(list (scan-sexps 1 1) (scan-lists 1 1 0) (scan-sexps 2 1) (scan-sexps 3 1) (scan-sexps 9 1) (scan-sexps 17 -1) (scan-lists 5 1 1) (scan-sexps 18 1) (scan-sexps 19 1) (scan-sexps 2 2))",
            ),
            "(17 17 3 9 14 1 9 19 nil 9)",
        ),
        (
            in_buffer("(a b", "(scan-sexps 1 1)"),
            "error: (scan-error \"Unbalanced parentheses\" 1 5)",
        ),
        (
            in_buffer("a)", "(scan-lists 1 1 0)"),
            "error: (scan-error \"Containing expression ends prematurely\" 2 3)",
        ),
        (
            in_buffer(
                "'foo",
                "(goto-char 2) (backward-prefix-chars) (list (point) (progn (let ((st (copy-syntax-table))) (modify-syntax-entry ?' \"'\" st) (set-syntax-table st)) (goto-char 2) (backward-prefix-chars) (point)))",
            ),
            "(2 1)",
        ),
        (
            in_buffer(
                "(a)",
                "(put-text-property 1 2 'syntax-table (string-to-syntax \"w\")) (setq syntax-propertize--done 1000) (list (scan-sexps 1 1) (let ((parse-sexp-lookup-properties t)) (scan-sexps 1 1)))",
            ),
            "(4 3)",
        ),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    check(&cases);
}

#[test]
fn partial_sexps() {
    let text = "(a (b \"c";
    let cases = [
        (
            in_buffer(text, "(list (parse-partial-sexp 1 9) (point))"),
            "((2 4 5 34 nil nil 0 nil 7 (1 4) nil) 9)",
        ),
        (
            in_buffer(text, "(list (parse-partial-sexp 1 9 nil t) (point))"),
            "((0 nil nil nil nil nil 0 nil nil nil nil) 1)",
        ),
        (
            in_buffer(text, "(list (parse-partial-sexp 1 9 1) (point))"),
            "((1 1 nil nil nil nil 0 nil nil (1) nil) 2)",
        ),
        (
            in_buffer(text, "(parse-partial-sexp 5 2)"),
            "error: (error \"End position is smaller than start position\")",
        ),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    check(&cases);
}

#[test]
fn comments() {
    let text = "a ; c\nb";
    let cases = [
        (
            with_comments(
                text,
                "(goto-char 2) (list (forward-comment 1) (point) (forward-comment -1) (point))",
            ),
            "(t 7 t 3)",
        ),
        (
            with_comments(
                text,
                "(list (scan-sexps 2 1) (let ((parse-sexp-ignore-comments t)) (scan-sexps 2 1)))",
            ),
            "(6 8)",
        ),
        (
            with_comments(text, "(parse-partial-sexp 1 5)"),
            "(0 nil 1 nil t nil 0 nil 3 nil nil)",
        ),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    check(&cases);
}

#[test]
fn fields() {
    let setup = "(insert (propertize \"def\" 'field 'f1)) (insert \"ghi\")";
    let cases = [
        (
            in_buffer(
                "abc",
                &format!(
                    "{setup} (list (field-beginning 5) (field-end 5) (field-string-no-properties 5) (field-beginning 2) (field-end 2) (field-beginning 4) (field-end 4))"
                ),
            ),
            "(4 7 \"def\" 1 4 1 4)",
        ),
        (
            in_buffer(
                "abc",
                &format!(
                    "{setup} (list (constrain-to-field 8 5) (constrain-to-field 2 5) (let ((inhibit-field-text-motion t)) (constrain-to-field 8 5)) (progn (goto-char 8) (constrain-to-field nil 5)) (point))"
                ),
            ),
            "(7 4 8 7 7)",
        ),
        (
            in_buffer("abc", &format!("{setup} (delete-field 5) (buffer-string)")),
            "\"abcghi\"",
        ),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    check(&cases);
}
