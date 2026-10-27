mod common;

use common::check;

#[test]
fn eval_buffer_and_region() {
    check(&[
        ("(set-buffer (get-buffer-create \"evb\"))", "#<buffer evb>"),
        (
            "(insert \"(setq evb-x 1) (setq evb-y (+ evb-x 1))\")",
            "nil",
        ),
        ("(goto-char 5)", "5"),
        ("(eval-buffer)", "nil"),
        ("evb-y", "2"),
        ("(point)", "5"),
        ("(erase-buffer)", "nil"),
        ("(insert \"(setq r1 10) (setq r2 20)\")", "nil"),
        ("(eval-region 1 13)", "nil"),
        ("(list r1 (boundp 'r2))", "(10 nil)"),
        ("(eval-region 1 13 (get-buffer-create \"evb-out\"))", "nil"),
        (
            "(progn (set-buffer \"evb-out\") (buffer-string))",
            "\"\n10\n\"",
        ),
        (
            "(eval-buffer \"no-such-buffer-xyz\")",
            "error: (error \"No such buffer\")",
        ),
        (
            "(get-file-char)",
            "error: (error \"get-file-char misused\")",
        ),
    ]);
}

#[test]
fn function_type_predicates() {
    check(&[
        ("(vector-or-char-table-p [1])", "t"),
        ("(vector-or-char-table-p (make-char-table 'foo))", "t"),
        ("(vector-or-char-table-p \"s\")", "nil"),
        (
            "(closurep (make-byte-code nil \"\\300\\207\" [nil] 1))",
            "t",
        ),
        (
            "(byte-code-function-p (make-byte-code nil \"\\300\\207\" [nil] 1))",
            "t",
        ),
        (
            "(interpreted-function-p (make-byte-code nil \"\\300\\207\" [nil] 1))",
            "nil",
        ),
        ("(closurep 'car)", "nil"),
        ("(byte-code-function-p (symbol-function 'car))", "nil"),
    ]);
}

#[test]
fn eval_buffer_lexical_cookie() {
    check(&[
        (
            "(set-buffer (get-buffer-create \"lexb\"))",
            "#<buffer lexb>",
        ),
        (
            "(insert \";; -*- lexical-binding: t -*-\n(setq lexb-seen lexical-binding)\")",
            "nil",
        ),
        ("(eval-buffer)", "nil"),
        ("lexb-seen", "t"),
        ("(erase-buffer)", "nil"),
        (
            "(insert \";; -*- lexical-binding: nil -*-\n(setq lexb-seen lexical-binding)\")",
            "nil",
        ),
        ("(eval-buffer)", "nil"),
        ("lexb-seen", "nil"),
    ]);
}
