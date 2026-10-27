mod common;

use common::check;

#[test]
fn list_completion() {
    check(&[
        ("(try-completion \"fo\" '(\"foo\" \"foobar\"))", "\"foo\""),
        ("(try-completion \"foo\" '(\"foo\"))", "t"),
        ("(try-completion \"foo\" '(\"foo\" \"foobar\"))", "\"foo\""),
        ("(try-completion \"x\" '(\"foo\"))", "nil"),
        (
            "(try-completion \"f\" '((\"foo\" . 1) bar \"fab\"))",
            "\"f\"",
        ),
        (
            "(all-completions \"f\" '(\"foo\" \"bar\" \"fab\"))",
            "(\"foo\" \"fab\")",
        ),
        (
            "(all-completions \"f\" '(\"foo\" \"fab\") (function (lambda (s) (equal s \"fab\"))))",
            "(\"fab\")",
        ),
        ("(test-completion \"foo\" '(\"foo\" \"bar\"))", "t"),
        ("(test-completion \"fo\" '(\"foo\"))", "nil"),
        (
            "(let ((completion-ignore-case t)) (test-completion \"FOO\" '(\"foo\")))",
            "t",
        ),
        (
            "(let ((completion-regexp-list '(\"b\"))) (all-completions \"\" '(\"ab\" \"cd\")))",
            "(\"ab\")",
        ),
        (
            "(try-completion \"foo\" (function (lambda (s p f) (list s p f))))",
            "(\"foo\" nil nil)",
        ),
        (
            "(test-completion \"foo\" (function (lambda (s p f) f)))",
            "lambda",
        ),
        ("(assoc-string \"FOO\" '((\"foo\" . 1)) t)", "(\"foo\" . 1)"),
        ("(assoc-string 'foo '(\"foo\"))", "\"foo\""),
        ("(assoc-string \"x\" '(1 (2) \"y\"))", "nil"),
    ]);
}

#[test]
fn table_completion() {
    check(&[
        (
            "(let ((h (make-hash-table :test 'equal))) (puthash \"foo\" 1 h) (puthash 'fab 2 h) (puthash 3 3 h) (all-completions \"f\" h))",
            "(\"foo\" \"fab\")",
        ),
        (
            "(let ((h (make-hash-table :test 'equal))) (puthash \"foo\" 1 h) (test-completion \"foo\" h (function (lambda (k v) v))))",
            "1",
        ),
        (
            "(let ((ob (obarray-make))) (intern \"foo\" ob) (intern \"fab\" ob) (sort (all-completions \"f\" ob) 'string-lessp))",
            "(\"fab\" \"foo\")",
        ),
        (
            "(let ((ob (obarray-make))) (intern \"foo\" ob) (list (test-completion \"foo\" ob) (test-completion \"fo\" ob)))",
            "(t nil)",
        ),
        (
            "(internal-complete-buffer \"*scr\" nil nil)",
            "\"*scratch*\"",
        ),
        ("(internal-complete-buffer \"\" nil t)", "(\"*scratch*\")"),
        ("(internal-complete-buffer \"*scratch*\" nil 'lambda)", "t"),
        (
            "(internal-complete-buffer \"\" nil 'metadata)",
            "(metadata (category . buffer) (cycle-sort-function . identity))",
        ),
    ]);
}

#[test]
fn flex_scoring() {
    check(&[
        (
            "(completion--flex-cost-gotoh \"goto\" \"eglot--goto\")",
            "(5 7 8 9 10)",
        ),
        ("(completion--flex-cost-gotoh \"xyz\" \"abc\")", "nil"),
        ("(completion--flex-cost-gotoh \"\" \"abc\")", "nil"),
        ("(completion--flex-cost-gotoh \"ab\" \"ab\")", "(0 0 1)"),
    ]);
}
