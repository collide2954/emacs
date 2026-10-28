mod common;

use common::check;

#[test]
fn prefix_numeric_value() {
    check(&[
        ("(prefix-numeric-value nil)", "1"),
        ("(prefix-numeric-value '-)", "-1"),
        ("(prefix-numeric-value '(16))", "16"),
        ("(prefix-numeric-value 7)", "7"),
        ("(prefix-numeric-value '(a))", "1"),
        ("(prefix-numeric-value \"x\")", "1"),
    ]);
}

#[test]
fn funcall_interactively() {
    check(&[
        ("(interactive \"p\")", "nil"),
        ("(funcall-interactively (function list) 1 2)", "(1 2)"),
        (
            "(funcall-interactively (function (lambda (x) (* x 2))) 21)",
            "42",
        ),
        (
            "(condition-case e (funcall-interactively (function car) 1) (wrong-type-argument (car e)))",
            "wrong-type-argument",
        ),
        ("(funcall-interactively (function +) 1 2 3)", "6"),
    ]);
}
