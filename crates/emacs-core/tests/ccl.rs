mod common;

use common::check;

#[test]
fn ccl_programs() {
    check(&[
        ("(ccl-program-p [0 2 22])", "t"),
        ("(ccl-program-p [0 9 22])", "nil"),
        ("(ccl-program-p 'undefined-ccl)", "nil"),
        (
            "(let ((r (make-vector 8 0))) (ccl-execute [0 3 10753 22] r) r)",
            "[42 0 0 0 0 0 0 0]",
        ),
        (
            "(let ((r (make-vector 8 0))) (ccl-execute [0 5 10753 57 5 22] r) r)",
            "[42 47 0 0 0 0 0 0]",
        ),
        (
            "(ccl-execute-on-string [1 7 14 23 1 17 -1276 22] (make-vector 9 nil) \"abc\")",
            "\"bcd\"",
        ),
        (
            "(ccl-execute-on-string [1 7 14 23 1 17 -1276 22] (make-vector 9 nil) \"abc\" nil t)",
            "\"bcd\"",
        ),
    ]);
}

#[test]
fn ccl_registration() {
    check(&[
        ("(register-ccl-program 'test-ccl [0 3 10753 22])", "0"),
        ("(ccl-program-p 'test-ccl)", "t"),
        (
            "(let ((r (make-vector 8 0))) (ccl-execute 'test-ccl r) r)",
            "[42 0 0 0 0 0 0 0]",
        ),
        ("(register-ccl-program 'test-ccl [0 3 10754 22])", "0"),
        ("(register-code-conversion-map 'test-map [0 10 20])", "0"),
        ("(get 'test-map 'code-conversion-map-id)", "0"),
        ("(get 'test-map 'code-conversion-map)", "[0 10 20]"),
    ]);
}
