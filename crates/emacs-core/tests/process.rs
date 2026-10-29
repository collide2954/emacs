mod common;

use common::check;
use emacs_core::process::{ULong, parse_omp_threads};

#[test]
fn num_processors() {
    check(&[
        ("(natnump (num-processors))", "t"),
        ("(> (num-processors 'current) 0)", "t"),
        ("(>= (num-processors 'all) (num-processors 'current))", "t"),
        (
            "(num-processors 1 2)",
            "error: (wrong-number-of-arguments num-processors 2)",
        ),
    ]);
}

#[test]
fn buffer_processes() {
    check(&[
        ("(get-buffer-process (current-buffer))", "nil"),
        ("(get-buffer-process \"*scratch*\")", "nil"),
        ("(get-buffer-process \"no such buffer\")", "nil"),
        ("(get-buffer-process nil)", "nil"),
        (
            "(get-buffer-process 1)",
            "error: (wrong-type-argument stringp 1)",
        ),
    ]);
}

#[test]
fn omp_threads() {
    let parse = |s: Option<&str>| parse_omp_threads(s.map(Into::into));
    assert_eq!(parse(None), 0);
    assert_eq!(parse(Some("")), 0);
    assert_eq!(parse(Some(" 4 ")), 4);
    assert_eq!(parse(Some("\t8,2")), 8);
    assert_eq!(parse(Some("4x")), 0);
    assert_eq!(parse(Some("-4")), 0);
    assert_eq!(parse(Some("x4")), 0);
    assert_eq!(parse(Some("99999999999999999999999")), ULong::MAX);
}
