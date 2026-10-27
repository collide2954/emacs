mod common;

use common::check;

#[test]
fn builtin_charsets() {
    check(&[
        ("(charsetp 'ascii)", "t"),
        ("(charsetp 'no-such-charset)", "nil"),
        ("(charset-id-internal 'ascii)", "0"),
        ("(encode-char ?A 'ascii)", "65"),
        ("(decode-char 'ascii 65)", "65"),
        ("(decode-char 'ascii 200)", "nil"),
        ("(decode-char 'unicode #x3042)", "12354"),
        ("(decode-char 'eight-bit 200)", "4194248"),
        ("(encode-char 4194248 'eight-bit)", "200"),
        ("(split-char ?A)", "(ascii 65)"),
        ("(make-char 'ascii 66)", "66"),
        ("(char-charset ?A)", "ascii"),
        ("(iso-charset 1 94 ?B)", "ascii"),
        ("(find-charset-string \"abc\")", "(ascii)"),
        ("(sort-charsets '(unicode ascii))", "(ascii unicode)"),
        ("(get-unused-iso-final-char 1 94)", "48"),
        ("(charset-priority-list t)", "ascii"),
    ]);
}

#[test]
fn charset_plist_and_definition() {
    check(&[
        ("(set-charset-plist 'ascii '(:foo 1))", "(:foo 1)"),
        ("(charset-plist 'ascii)", "(:foo 1)"),
        (
            "(define-charset-internal 'test-cs 1 [32 127 0 0 0 0 0 0] nil nil nil nil nil nil nil nil #x10000 nil nil nil nil '(:name test-cs))",
            "nil",
        ),
        ("(charsetp 'test-cs)", "t"),
        ("(decode-char 'test-cs 65)", "65569"),
        ("(encode-char 65569 'test-cs)", "65"),
        ("(define-charset-alias 'test-alias 'test-cs)", "nil"),
        ("(decode-char 'test-alias 32)", "65536"),
    ]);
}
