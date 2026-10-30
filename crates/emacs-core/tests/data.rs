mod common;

use common::check;

#[test]
fn symbols_with_position_as_symbols() {
    check(&[
        ("(defvar swp-var 7)", "swp-var"),
        (
            "(setq swp (position-symbol 'swp-var 12))",
            "#<symbol swp-var at 12>",
        ),
        (
            "(special-variable-p swp)",
            "error: (wrong-type-argument symbolp #<symbol swp-var at 12>)",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (list (special-variable-p swp) (symbol-value swp) (boundp swp) (get swp 'nothing)))",
            "(t 7 t nil)",
        ),
        (
            "(setq swp-lambda (list (position-symbol 'lambda 3) '(x) 'x) swp-car (position-symbol 'car 5))",
            "#<symbol car at 5>",
        ),
        (
            "(list (functionp swp-lambda) (functionp swp-car))",
            "(nil nil)",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (list (functionp swp-lambda) (functionp swp-car)))",
            "(t t)",
        ),
        (
            "(defalias 'swp-mac (cons 'macro (function (lambda (x) (list 'quote x)))))",
            "swp-mac",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (macroexpand (list (position-symbol 'swp-mac 3) 5)))",
            "'5",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (macroexpand (list (position-symbol 'swp-env 3) 5) (list (cons 'swp-env (function (lambda (x) (list 'quote (list x x))))))))",
            "'(5 5)",
        ),
        (
            "(car (macroexpand (list (position-symbol 'swp-mac 3) 5)))",
            "#<symbol swp-mac at 3>",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (eval (position-symbol 'swp-var 3)))",
            "7",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (eval (list 'let '((q 5)) (position-symbol 'q 2)) t))",
            "5",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (eval (list (position-symbol 'car 1) ''(a b))))",
            "a",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (funcall (position-symbol 'car 1) '(a b)))",
            "a",
        ),
        (
            "(funcall (position-symbol 'car 1) '(a b))",
            "error: (invalid-function #<symbol car at 1>)",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (byte-code \"\\300\\071\\207\" (vector (position-symbol 'x 1)) 1))",
            "t",
        ),
        (
            "(byte-code \"\\300\\071\\207\" (vector (position-symbol 'x 1)) 1)",
            "nil",
        ),
    ]);
}

#[test]
fn arithmetic() {
    check(&[
        ("(+ 1 2 3)", "6"),
        ("(+)", "0"),
        ("(- 5)", "-5"),
        ("(- 10 1 2)", "7"),
        ("(* 2 3.0)", "6.0"),
        ("(/ 7 2)", "3"),
        ("(/ 7 2.0)", "3.5"),
        ("(/ -7 2)", "-3"),
        ("(/ 5 0)", "error: (arith-error)"),
        ("(/ 5.0 0)", "1.0e+INF"),
        ("(/ 8 2 2.0)", "2.0"),
        ("(% 7 -3)", "1"),
        ("(mod -7 3)", "2"),
        ("(mod 5.5 2)", "1.5"),
        ("(1+ most-positive-fixnum)", "2305843009213693952"),
        ("(* most-positive-fixnum 2)", "4611686018427387902"),
        ("(- most-negative-fixnum 1)", "-2305843009213693953"),
        ("(- (1+ most-positive-fixnum) 1)", "2305843009213693951"),
        ("(max 1 2.0)", "2.0"),
        ("(min 3 1 2)", "1"),
        ("(= 1 1.0)", "t"),
        ("(< 1 2 3)", "t"),
        ("(< 1 3 2)", "nil"),
        ("(/= 1 2)", "t"),
        ("(logand 12 10)", "8"),
        ("(logior 12 10)", "14"),
        ("(logxor 12 10)", "6"),
        ("(ash 1 10)", "1024"),
        ("(ash -8 -1)", "-4"),
        ("(ash 1 70)", "1180591620717411303424"),
        ("(lognot 5)", "-6"),
        ("(logcount 7)", "3"),
        (
            "(+ 'a 1)",
            "error: (wrong-type-argument number-or-marker-p a)",
        ),
        (
            "(logand 1.0 1)",
            "error: (wrong-type-argument integer-or-marker-p 1.0)",
        ),
        (
            "(% 1.0 2)",
            "error: (wrong-type-argument integer-or-marker-p 1.0)",
        ),
    ]);
}

#[test]
fn float_functions() {
    check(&[
        ("(truncate 2.7)", "2"),
        ("(round 2.5)", "2"),
        ("(round 3.5)", "4"),
        ("(round -2.5)", "-2"),
        ("(floor -1.5)", "-2"),
        ("(ceiling 1.2)", "2"),
        ("(floor 7 2)", "3"),
        ("(floor -7 2)", "-4"),
        ("(round 5 2)", "2"),
        ("(round 7 2)", "4"),
        (
            "(truncate 1.0e+INF)",
            "error: (overflow-error truncate 1.0e+INF)",
        ),
        ("(abs -3)", "3"),
        ("(abs -2.5)", "2.5"),
        ("(sqrt 4)", "2.0"),
        ("(expt 2 10)", "1024"),
        ("(expt 2.0 3)", "8.0"),
        ("(expt 2 -1)", "0.5"),
        ("(expt 2 100)", "1267650600228229401496703205376"),
        ("(float 3)", "3.0"),
        ("(isnan 0.0e+NaN)", "t"),
        ("(ffloor 2.5)", "2.0"),
        ("(fround 2.5)", "2.0"),
        ("(logb 8)", "3"),
        ("(frexp 8.0)", "(0.5 . 4)"),
        ("(ldexp 0.5 4)", "8.0"),
    ]);
}

#[test]
fn types() {
    check(&[
        ("(type-of 1)", "integer"),
        ("(type-of 1.0)", "float"),
        ("(type-of \"a\")", "string"),
        ("(type-of 'a)", "symbol"),
        ("(type-of nil)", "symbol"),
        ("(type-of '(1))", "cons"),
        ("(type-of [1])", "vector"),
        ("(type-of (symbol-function 'car))", "subr"),
        ("(cl-type-of nil)", "null"),
        ("(cl-type-of t)", "boolean"),
        ("(cl-type-of 1)", "fixnum"),
        ("(cl-type-of (symbol-function 'if))", "special-form"),
        ("(type-of (make-hash-table))", "hash-table"),
        ("(type-of (make-bool-vector 3 t))", "bool-vector"),
        ("(type-of (record 'foo 1))", "foo"),
        ("(type-of (make-char-table 'x))", "char-table"),
        ("(integerp (expt 2 70))", "t"),
        ("(natnump -1)", "nil"),
        ("(sequencep [1])", "t"),
        ("(arrayp \"s\")", "t"),
        ("(char-or-string-p 65)", "t"),
    ]);
}

#[test]
fn symbols() {
    check(&[
        ("(symbol-name 'abc)", "\"abc\""),
        ("(fboundp 'car)", "t"),
        ("(boundp 'unbound-thing)", "nil"),
        ("(setplist 'zz '(a 1))", "(a 1)"),
        ("(get 'zz 'a)", "1"),
        ("(put 'zz 'b 2)", "2"),
        ("(symbol-plist 'zz)", "(a 1 b 2)"),
        ("(eq (intern \"abc\") 'abc)", "t"),
        ("(eq (make-symbol \"abc\") 'abc)", "nil"),
        ("(intern-soft \"never-interned-xyz\")", "nil"),
        ("(keywordp :foo)", "t"),
        ("(keywordp 'foo)", "nil"),
        ("(fset 'f1 'f2)", "f2"),
        ("(fset 'f2 'f1)", "error: (cyclic-function-indirection f2)"),
        ("(fmakunbound 'f1)", "f1"),
        ("(indirect-function 'car)", "#<subr car>"),
        ("(subr-arity (symbol-function 'cons))", "(2 . 2)"),
        ("(subr-name (symbol-function 'cons))", "\"cons\""),
    ]);
}

#[test]
fn arrays() {
    check(&[
        ("(aref [1 2 3] 1)", "2"),
        ("(aref \"abc\" 0)", "97"),
        ("(aref \"héllo\" 1)", "233"),
        ("(aref [1] 5)", "error: (args-out-of-range [1] 5)"),
        ("(let ((v (vector 1 2))) (aset v 0 9) v)", "[9 2]"),
        (
            "(let ((s (copy-sequence \"abc\"))) (aset s 1 ?é) (list (multibyte-string-p s) (aref s 1)))",
            "(nil 233)",
        ),
        (
            "(let ((s (copy-sequence \"abc\"))) (aset s 2 255) (list (multibyte-string-p s) (aref s 2)))",
            "(nil 255)",
        ),
        (
            "(aset (copy-sequence \"abc\") 1 256)",
            "error: (error \"Attempt to store non-byte value into unibyte string\")",
        ),
        (
            "(aset (copy-sequence \"abc\") 1 #x3fff80)",
            "error: (error \"Attempt to store non-byte value into unibyte string\")",
        ),
        (
            "(let ((s (copy-sequence \"abçdef\"))) (aset s 4 ?E) s)",
            "\"abçdEf\"",
        ),
        (
            "(aset (copy-sequence \"abçdef\") 2 ?c)",
            "error: (error \"Attempt to replace non-ASCII char in multibyte string\")",
        ),
        (
            "(aset (copy-sequence \"abçdef\") 1 #xe9)",
            "error: (error \"Attempt to store non-ASCII char into multibyte string\")",
        ),
        (
            "(aset (copy-sequence \"abc\") 5 'x)",
            "error: (args-out-of-range \"abc\" 5)",
        ),
        (
            "(aset (copy-sequence \"abc\") 1 'x)",
            "error: (wrong-type-argument characterp x)",
        ),
        (
            "(let ((b (make-bool-vector 3 nil))) (aset b 1 t) b)",
            "#&3\"\u{2}\"",
        ),
        ("(bool-vector-count-population (bool-vector t nil t))", "2"),
        ("(string-to-number \"12\")", "12"),
        ("(string-to-number \" 1.5\")", "1.5"),
        ("(string-to-number \"ff\" 16)", "255"),
        ("(string-to-number \"abc\")", "0"),
        ("(number-to-string 1.5)", "\"1.5\""),
        (
            "(number-to-string (expt 2 70))",
            "\"1180591620717411303424\"",
        ),
    ]);
}
