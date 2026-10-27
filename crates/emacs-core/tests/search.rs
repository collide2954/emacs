mod common;

use common::check;

#[test]
fn string_match_basics() {
    check(&[
        ("(string-match \"b+\" \"aabbbc\")", "2"),
        ("(match-end 0)", "5"),
        ("(string-match \"\\\\(a\\\\)\\\\(b\\\\)?c\" \"ac\")", "0"),
        ("(match-data)", "(0 2 0 1)"),
        (
            "(string-match \"^\\\\(a\\\\)?\\\\(b\\\\)\\\\(c\\\\)?$\" \"b\")",
            "0",
        ),
        ("(match-data)", "(0 1 nil nil 0 1)"),
        ("(string-match \"foo\\\\|bar\" \"xbar\")", "1"),
        ("(string-match \"x*\" \"\")", "0"),
        ("(string-match \"a\" \"bab\" -2)", "1"),
        ("(string-match \"zz\" \"abc\")", "nil"),
        ("(string-match \"\\\\(a*\\\\)*b\" \"aab\")", "0"),
    ]);
}

#[test]
fn case_folding() {
    check(&[
        (
            "(let ((case-fold-search t)) (string-match \"ABC\" \"xabc\"))",
            "1",
        ),
        (
            "(let ((case-fold-search nil)) (string-match \"ABC\" \"xabc\"))",
            "nil",
        ),
        (
            "(let ((case-fold-search t)) (string-match \"[A-C]+\" \"xxabcz\"))",
            "2",
        ),
        ("(case-table-p (standard-case-table))", "t"),
        ("(aref (standard-case-table) ?A)", "97"),
    ]);
}

#[test]
fn classes_boundaries_and_backrefs() {
    check(&[
        ("(string-match \"[[:digit:]]+\" \"ab123\")", "2"),
        ("(match-end 0)", "5"),
        ("(string-match \"[^a-c]\" \"abcd\")", "3"),
        ("(string-match \"\\\\bfoo\\\\b\" \"a foo b\")", "2"),
        ("(string-match \"\\\\_<foo-bar\\\\_>\" \"(foo-bar)\")", "1"),
        ("(string-match \"\\\\<w\" \"a word\")", "2"),
        ("(string-match \"\\\\w+\\\\>\" \"!! hi\")", "3"),
        ("(string-match \"\\\\s-+\" \"ab  c\")", "2"),
        ("(string-match \"\\\\(ab\\\\)\\\\1\" \"xabab\")", "1"),
        ("(string-match \"a\\\\{2,3\\\\}\" \"caaaa\")", "1"),
        ("(match-end 0)", "4"),
        (
            "(progn (string-match \"a.*?b\" \"axxbyyb\") (match-end 0))",
            "4",
        ),
        (
            "(progn (string-match \"a.*b\" \"axxbyyb\") (match-end 0))",
            "7",
        ),
        (
            "(progn (string-match \"\\\\(?:ab\\\\)+\\\\(c\\\\)\" \"ababc\") (match-beginning 1))",
            "4",
        ),
        (
            "(progn (string-match \"\\\\(?2:a\\\\)\\\\(b\\\\)\" \"ab\") (list (match-beginning 1) (match-beginning 2) (match-beginning 3)))",
            "(nil 0 1)",
        ),
        (
            "(progn (posix-string-match \"a\\\\|ab\" \"ab\") (match-end 0))",
            "2",
        ),
        (
            "(progn (string-match \"a\\\\|ab\" \"ab\") (match-end 0))",
            "1",
        ),
        ("(string-match \"é+\" \"caféé\")", "3"),
        ("(match-end 0)", "5"),
    ]);
}

#[test]
fn errors_and_quoting() {
    check(&[
        (
            "(string-match \"\\\\(\" \"x\")",
            r#"error: (invalid-regexp "Unmatched ( or \\(")"#,
        ),
        (
            "(string-match \"[a\" \"x\")",
            r#"error: (invalid-regexp "Unmatched [ or [^")"#,
        ),
        ("(regexp-quote \"a.b*c\")", r#""a\\.b\\*c""#),
        ("(regexp-quote \"plain\")", "\"plain\""),
    ]);
}

#[test]
fn buffer_searching() {
    check(&[
        ("(set-buffer (get-buffer-create \"s\"))", "#<buffer s>"),
        ("(insert \"hello world\")", "nil"),
        ("(goto-char 1)", "1"),
        ("(re-search-forward \"o\\\\(r\\\\)?\" nil t)", "6"),
        ("(match-beginning 0)", "5"),
        ("(match-beginning 1)", "nil"),
        ("(re-search-forward \"wor\\\\(ld\\\\)\")", "12"),
        ("(match-beginning 1)", "10"),
        ("(search-backward \"o\")", "8"),
        ("(point)", "8"),
        ("(looking-at \"or\")", "t"),
        ("(replace-match \"OR\")", "nil"),
        ("(buffer-string)", "\"hello wORld\""),
        ("(point)", "10"),
        ("(search-forward \"zzz\" nil t)", "nil"),
        ("(search-forward \"zzz\")", "error: (search-failed \"zzz\")"),
        ("(goto-char 1)", "1"),
        ("(re-search-forward \"l+\" 4 t)", "4"),
        ("(goto-char 1)", "1"),
        ("(re-search-forward \"\\\\bw\" nil t)", "8"),
        ("(goto-char (point-max))", "12"),
        ("(re-search-backward \"l+\" nil t)", "10"),
        ("(match-end 0)", "11"),
    ]);
}

#[test]
fn match_data_round_trip() {
    check(&[
        (
            "(progn (string-match \"\\\\(b\\\\)\" \"abc\") (replace-match \"[\\\\1]\" t nil \"abc\"))",
            "\"a[b]c\"",
        ),
        ("(progn (string-match \"b\" \"abc\") (match-data))", "(1 2)"),
        (
            "(progn (set-match-data '(1 3 nil nil 5 6)) (list (match-beginning 0) (match-beginning 1) (match-end 2)))",
            "(1 nil 6)",
        ),
    ]);
}

#[test]
fn syntax_tables() {
    check(&[
        ("(char-syntax ?a)", "119"),
        ("(char-syntax ? )", "32"),
        ("(char-syntax ?\\()", "40"),
        ("(string-to-syntax \"()\")", "(4 . 41)"),
        ("(string-to-syntax \"w\")", "(2)"),
        (
            "(eq (string-to-syntax \"w\") (string-to-syntax \"w\"))",
            "t",
        ),
        ("(matching-paren ?\\()", "41"),
        ("(syntax-table-p (standard-syntax-table))", "t"),
        (
            "(syntax-table-p (set-syntax-table (copy-syntax-table)))",
            "t",
        ),
        (
            "(progn (modify-syntax-entry ?- \"w\") (char-syntax ?-))",
            "119",
        ),
        ("(string-match \"\\\\w+$\" \"x a-b\")", "2"),
        ("(match-end 0)", "5"),
    ]);
}

#[test]
fn categories() {
    check(&[
        ("(category-table-p (standard-category-table))", "t"),
        ("(define-category ?x \"test\")", "nil"),
        ("(modify-category-entry ?a ?x)", "nil"),
        ("(category-set-mnemonics (char-category-set ?a))", "\"x\""),
        ("(string-match \"\\\\cx\" \"bca\")", "2"),
        ("(string-match \"\\\\Cx\" \"aab\")", "2"),
    ]);
}
