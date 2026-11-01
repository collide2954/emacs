mod common;

use common::check;

#[test]
fn setting_fonts_in_the_default_fontset() {
    check(&[
        (
            "(setq font-encoding-alist '((\"iso10646-1$\" . unicode)))",
            "((\"iso10646-1$\" . unicode))",
        ),
        (
            "(set-fontset-font t '(#x3000 . #x30FF) (font-spec :family \"Foo\" :registry \"iso10646-1\"))",
            "nil",
        ),
        ("(fontset-font t #x3042)", "(\"Foo\" . \"iso10646-1\")"),
        ("(fontset-font t #x3042 t)", "((\"Foo\" . \"iso10646-1\"))"),
        ("(fontset-font t #x4E00)", "nil"),
        (
            "(set-fontset-font t '(#x3000 . #x30FF) (font-spec :family \"Bar\" :registry \"iso10646-1\") nil 'append)",
            "nil",
        ),
        (
            "(set-fontset-font t '(#x3040 . #x309F) (font-spec :family \"Baz\" :registry \"iso10646-1\") nil 'prepend)",
            "nil",
        ),
        (
            "(fontset-font t #x3042 t)",
            "((\"Baz\" . \"iso10646-1\") (\"Foo\" . \"iso10646-1\") (\"Bar\" . \"iso10646-1\"))",
        ),
        (
            "(fontset-font t #x30A2 t)",
            "((\"Foo\" . \"iso10646-1\") (\"Bar\" . \"iso10646-1\"))",
        ),
        (
            "(set-fontset-font t nil (font-spec :family \"Fallback\" :registry \"iso10646-1\"))",
            "nil",
        ),
        (
            "(fontset-font t #x1F600)",
            "(\"Fallback\" . \"iso10646-1\")",
        ),
        (
            "(set-fontset-font t #x2603 '(\"Snow\" . \"iso10646-1\"))",
            "nil",
        ),
        ("(fontset-font t #x2603)", "(\"Snow\" . \"iso10646-1\")"),
        ("(set-fontset-font t #x2604 \"Comet-12\")", "nil"),
        (
            "(fontset-font t #x2604 t)",
            "((\"Fallback\" . \"iso10646-1\"))",
        ),
        ("(set-fontset-font t #x2605 nil)", "nil"),
        ("(fontset-font t #x2605)", "nil"),
        (
            "(set-fontset-font t ?a \"Mono\")",
            "error: (error \"Can\u{2019}t set a font for partial ASCII range\")",
        ),
        (
            "(set-fontset-font t '(?a . ?z) \"Mono\")",
            "error: (error \"Can\u{2019}t set a font for partial ASCII range\")",
        ),
        (
            "(set-fontset-font t 'kana \"Mono\")",
            "error: (error \"Invalid script or charset name: kana\")",
        ),
        (
            "(set-fontset-font t \"x\" \"Mono\")",
            "error: (error \"Invalid second argument for setting a font in a fontset\")",
        ),
        (
            "(set-fontset-font t #x3042 5)",
            "error: (error \"Invalid error symbol\" font)",
        ),
        (
            "(set-fontset-font \"no-such\" #x3042 \"Mono\")",
            "error: (error \"Fontset \u{2018}no-such\u{2019} does not exist\")",
        ),
        (
            "(set-fontset-font nil #x3042 \"Mono\")",
            "error: (error \"Can\u{2019}t use fontsets in non-GUI frames\")",
        ),
        (
            "(set-fontset-font t '(0 . #x7F) nil)",
            "error: (error \"Can\u{2019}t set ASCII font to nil\")",
        ),
        (
            "(fontset-font t 'x)",
            "error: (wrong-type-argument characterp x)",
        ),
    ]);
}

#[test]
fn charsets_and_new_fontsets() {
    check(&[
        (
            "(setq font-encoding-alist '((\"iso10646-1$\" . unicode)))",
            "((\"iso10646-1$\" . unicode))",
        ),
        (
            "(new-fontset \"-*-Fixed-medium-r-normal-*-16-*-*-*-*-*-fontset-test\" nil)",
            "\"-*-fixed-medium-r-normal-*-16-*-*-*-*-*-fontset-test\"",
        ),
        (
            "(car (rassoc \"fontset-test\" fontset-alias-alist))",
            "\"-*-fixed-medium-r-normal-*-16-*-*-*-*-*-fontset-test\"",
        ),
        (
            "(fontset-list)",
            "(\"-*-fixed-medium-r-normal-*-16-*-*-*-*-*-fontset-test\" \"-*-*-*-*-*-*-*-*-*-*-*-*-fontset-default\")",
        ),
        (
            "(assoc \"-*-fixed-medium-r-normal-*-16-*-*-*-*-*-fontset-test\" fontset-alias-alist)",
            "(\"-*-fixed-medium-r-normal-*-16-*-*-*-*-*-fontset-test\" . \"fontset-test\")",
        ),
        (
            "(set-fontset-font \"fontset-test\" 'unicode (font-spec :family \"Uni\" :registry \"iso10646-1\"))",
            "nil",
        ),
        (
            "(fontset-font \"fontset-test\" #x3042)",
            "(\"Uni\" . \"iso10646-1\")",
        ),
        (
            "(fontset-font \"fontset-test\" ?a)",
            "(\"Uni\" . \"iso10646-1\")",
        ),
        ("(fontset-font \"fontset-test\" #x110000)", "nil"),
        (
            "(new-fontset \"fontset-test\" (list (list 'unicode (font-spec :family \"A\" :registry \"iso10646-1\") (font-spec :family \"B\" :registry \"iso10646-1\"))))",
            "\"fontset-test\"",
        ),
        (
            "(fontset-font \"fontset-test\" #x3042 t)",
            "((\"A\" . \"iso10646-1\") (\"B\" . \"iso10646-1\"))",
        ),
        (
            "(new-fontset \"foo\" nil)",
            "error: (error \"Fontset name must be in XLFD format\")",
        ),
        (
            "(new-fontset \"-*-fixed-medium-r-normal-*-16-*-*-*-*-*-iso8859-1\" nil)",
            "error: (error \"Registry field of fontset name must be \\\"fontset-*\\\"\")",
        ),
        (
            "(new-fontset 5 nil)",
            "error: (wrong-type-argument stringp 5)",
        ),
    ]);
}
