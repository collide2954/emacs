mod common;

use common::check;

#[test]
fn arguments() {
    check(&[
        ("(funcall #[514 \"\\1\\1\\134\\207\" [] 4] 3 4)", "7"),
        (
            "(funcall #[514 \"\\1\\1\\134\\207\" [] 4] 3)",
            "error: (wrong-number-of-arguments (2 . 2) 1)",
        ),
        ("(funcall #[513 \"\\207\" [] 2] 1)", "nil"),
        ("(funcall #[513 \"\\207\" [] 2] 1 2)", "2"),
        ("(funcall #[385 \"\\207\" [] 2] 1 2 3)", "(2 3)"),
        ("(funcall #[385 \"\\207\" [] 2] 1)", "nil"),
        ("(funcall #[(x) \"\\010\\207\" [x] 1] 42)", "42"),
        ("(defvar x 'global)", "x"),
        (
            "(list (funcall #[(x) \"\\010\\207\" [x] 1] 42) x)",
            "(42 global)",
        ),
        ("(func-arity #[385 \"\\207\" [] 2])", "(1 . many)"),
    ]);
}

#[test]
fn byte_code_function() {
    check(&[
        ("(byte-code \"\\300\\207\" [hello] 1)", "hello"),
        ("(byte-code \"\\201\\1\\0\\207\" [a b] 1)", "b"),
        ("(byte-code 1 [] 0)", "error: (error \"Invalid byte-code\")"),
        (
            "(byte-code \"\\300\\207\" [a] -1)",
            "error: (error \"Invalid byte-code\")",
        ),
        (
            "(byte-code \"\\0\\207\" [] 1)",
            "error: (error \"Invalid byte opcode: op=0, ptr=0\")",
        ),
        (
            "(byte-code \"\\300\\203\\6\\0\\301\\207\\302\\207\" [nil yes no] 1)",
            "no",
        ),
        (
            "(byte-code \"\\300\\203\\6\\0\\301\\207\\302\\207\" [t yes no] 1)",
            "yes",
        ),
        (
            "(byte-code \"\\300\\301\\211\\300\\125\\204\\21\\0\\1\\1\\134\\262\\2\\123\\202\\2\\0\\210\\207\" [0 100000] 4)",
            "5000050000",
        ),
    ]);
}

#[test]
fn variables_and_calls() {
    check(&[
        ("(defvar bc-x 10)", "bc-x"),
        (
            "(list (byte-code \"\\301\\030\\010\\051\\207\" [bc-x 5] 2) bc-x)",
            "(5 10)",
        ),
        ("(byte-code \"\\301\\020\\010\\207\" [bc-y 7] 2)", "7"),
        ("bc-y", "7"),
        ("(byte-code \"\\300\\301\\302\\042\\207\" [+ 1 2] 3)", "3"),
        ("(byte-code \"\\300\\301\\041\\207\" [car (a b)] 2)", "a"),
        (
            "(byte-code \"\\300\\301\\041\\207\" [#[257 \"\\211\\134\\207\" [] 3] 5] 2)",
            "10",
        ),
        (
            "(byte-code \"\\300\\040\\207\" [#[257 \"\\211\\134\\207\" [] 3]] 2)",
            "error: (wrong-number-of-arguments (1 . 1) 0)",
        ),
        (
            "(defalias 'bc-twice #[257 \"\\211\\134\\207\" [] 3])",
            "bc-twice",
        ),
        ("(byte-code \"\\300\\301\\041\\207\" [bc-twice 21] 2)", "42"),
        (
            "(byte-code \"\\300\\301\\041\\207\" [no-such-function 1] 2)",
            "error: (void-function no-such-function)",
        ),
        (
            "(let ((f (eval '(function (lambda (x) x)) t))) (funcall (make-byte-code 0 \"\\300\\301\\302\\041\\001\\207\" (vector 'marker f 42) 4)))",
            "marker",
        ),
        (
            "(let ((f (eval '(function (lambda (x) x)) t))) (funcall (make-byte-code 0 \"\\300\\301\\302\\041\\207\" (vector 'marker f 42) 4)))",
            "42",
        ),
    ]);
}

#[test]
fn handlers() {
    check(&[
        (
            "(byte-code \"\\300\\061\\11\\0\\301\\302\\041\\060\\207\\207\" [(error) car 1] 3)",
            "(wrong-type-argument listp 1)",
        ),
        (
            "(byte-code \"\\300\\061\\11\\0\\301\\302\\041\\060\\207\\207\" [(error) car (a)] 3)",
            "a",
        ),
        (
            "(byte-code \"\\300\\061\\11\\0\\301\\302\\041\\060\\207\\207\" [(arith-error) car 1] 3)",
            "error: (wrong-type-argument listp 1)",
        ),
        (
            "(byte-code \"\\300\\062\\12\\0\\301\\300\\302\\042\\060\\207\\207\" [tag throw 42] 4)",
            "42",
        ),
        (
            "(catch 'outer (byte-code \"\\300\\062\\12\\0\\301\\303\\302\\042\\060\\207\\207\" [tag throw 42 outer] 4))",
            "42",
        ),
        ("(defvar bc-log nil)", "bc-log"),
        (
            "(list (byte-code \"\\300\\216\\301\\051\\207\" [(lambda () (setq bc-log 'ran)) value] 2) bc-log)",
            "(value ran)",
        ),
        (
            "(list (byte-code \"\\300\\216\\301\\051\\207\" [((setq bc-log 'forms)) value] 2) bc-log)",
            "(value forms)",
        ),
        (
            "(progn (setq bc-log nil) (list (catch 'tag (byte-code \"\\300\\216\\301\\302\\303\\042\\207\" [((setq bc-log 'unwound)) throw tag thrown] 3)) bc-log))",
            "(thrown unwound)",
        ),
        (
            "(byte-code \"\\300\\301\\215\\207\" [tag (throw 'tag 'caught)] 2)",
            "caught",
        ),
    ]);
}

#[test]
fn stack_and_lists() {
    check(&[
        (
            "(byte-code \"\\301\\302\\303\\257\\3\\207\" [nil 1 2 3] 3)",
            "(1 2 3)",
        ),
        (
            "(byte-code \"\\301\\302\\303\\266\\202\\207\" [nil 1 2 3] 3)",
            "3",
        ),
        ("(byte-code \"\\301\\302\\262\\1\\207\" [nil 1 2] 2)", "2"),
        (
            "(byte-code \"\\301\\302\\303\\002\\207\" [nil 1 2 3] 4)",
            "1",
        ),
        (
            "(byte-code \"\\300\\301\\302\\303\\260\\4\\207\" [\"a\" \"b\" \"c\" \"d\"] 4)",
            "\"abcd\"",
        ),
        (
            "(byte-code \"\\300\\301\\302\\105\\207\" [1 2 3] 3)",
            "(1 2 3)",
        ),
        ("(byte-code \"\\300\\301\\070\\207\" [1 (a b c)] 2)", "b"),
        ("(byte-code \"\\300\\301\\234\\207\" [(a b c) 2] 2)", "c"),
        (
            "(byte-code \"\\300\\100\\207\" [1] 1)",
            "error: (wrong-type-argument listp 1)",
        ),
        (
            "(byte-code \"\\300\\124\\207\" [2305843009213693951] 1)",
            "2305843009213693952",
        ),
        (
            "(byte-code \"\\300\\123\\207\" [-2305843009213693952] 1)",
            "-2305843009213693953",
        ),
        (
            "(byte-code \"\\300\\301\\137\\207\" [2305843009213693951 2] 2)",
            "4611686018427387902",
        ),
        (
            "(byte-code \"\\300\\301\\245\\207\" [7 0] 2)",
            "error: (arith-error)",
        ),
        ("(byte-code \"\\300\\301\\246\\207\" [-7 2] 2)", "-1"),
        ("(byte-code \"\\300\\301\\135\\207\" [1 2.5] 2)", "2.5"),
        ("(byte-code \"\\300\\301\\110\\207\" [[a b] 1] 2)", "b"),
        (
            "(byte-code \"\\300\\301\\110\\207\" [[a b] 2] 2)",
            "error: (args-out-of-range [a b] 2)",
        ),
        (
            "(byte-code \"\\301\\300\\267\\302\\207\\207\\303\\207\\304\\207\" [#s(hash-table test eq data (a 6 b 8)) a none is-a is-b] 2)",
            "is-a",
        ),
        (
            "(byte-code \"\\301\\300\\267\\302\\207\\207\\303\\207\\304\\207\" [#s(hash-table test eq data (a 6 b 8)) b none is-a is-b] 2)",
            "is-b",
        ),
        (
            "(byte-code \"\\301\\300\\267\\302\\207\\207\\303\\207\\304\\207\" [#s(hash-table test eq data (a 6 b 8)) c none is-a is-b] 2)",
            "none",
        ),
    ]);
}

#[test]
fn buffers() {
    check(&[
        (
            "(save-current-buffer (set-buffer (get-buffer-create \" bc\")) (byte-code \"\\300\\143\\210\\140\\207\" [\"abc\"] 1))",
            "4",
        ),
        (
            "(save-current-buffer (set-buffer \" bc\") (byte-code \"\\162\\300\\161\\210\\145\\144\\173\\051\\207\" [\" bc\"] 2))",
            "\"abc\"",
        ),
        (
            "(save-current-buffer (set-buffer \" bc\") (byte-code \"\\212\\300\\142\\210\\147\\051\\207\" [2] 2))",
            "98",
        ),
        ("(save-current-buffer (set-buffer \" bc\") (point))", "4"),
        (
            "(save-current-buffer (set-buffer \" bc\") (list (byte-code \"\\214\\300\\301\\175\\210\\145\\144\\173\\051\\207\" [2 3] 2) (point-min) (point-max)))",
            "(\"b\" 1 4)",
        ),
    ]);
}
