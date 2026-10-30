mod common;

use common::check;

#[test]
fn raw_bytes_by_destination() {
    check(&[
        (
            "(equal (prin1-to-string \"\\377\") \"\\\"\\\\377\\\"\")",
            "t",
        ),
        (
            "(let ((print-escape-nonascii nil)) (equal (prin1-to-string \"a\\377\") \"\\\"a\\\\377\\\"\"))",
            "t",
        ),
        ("(equal (format \"%S\" \"\\377\") \"\\\"\\\\377\\\"\")", "t"),
        (
            "(progn (set-buffer (get-buffer-create \"pm\")) (prin1 \"a\\377\" (current-buffer)) (equal (buffer-string) \"\\\"a\\\\377\\\"\"))",
            "t",
        ),
        (
            "(progn (set-buffer (get-buffer-create \"pp\")) (princ \"a\\377\" (current-buffer)) (equal (buffer-string) \"a\\\\377\"))",
            "t",
        ),
        (
            "(progn (set-buffer (get-buffer-create \"pu\")) (set-buffer-multibyte nil) (prin1 \"\u{e9}\" (current-buffer)) (equal (buffer-string) \"\\\"\\\\x00e9\\\"\"))",
            "t",
        ),
        (
            "(let ((out nil)) (prin1 \"\\377\" (function (lambda (c) (setq out (cons c out))))) out)",
            "(34 255 34)",
        ),
        (
            "(let ((out nil)) (princ \"\\377\" (function (lambda (c) (setq out (cons c out))))) out)",
            "(255)",
        ),
        (
            "(equal (error-message-string (list 'error \"x\" \"\\377\")) \"x: \\\"\\\\377\\\"\")",
            "t",
        ),
    ]);
}
