mod common;

use common::check;

#[test]
fn buffer_hash() {
    check(&[
        ("(set-buffer (get-buffer-create \"bh\"))", "#<buffer bh>"),
        (
            "(buffer-hash)",
            "\"da39a3ee5e6b4b0d3255bfef95601890afd80709\"",
        ),
        (
            "(progn (insert \"abc\") (buffer-hash))",
            "\"a9993e364706816aba3e25717850c26c9cd0d89d\"",
        ),
        (
            "(progn (narrow-to-region 2 3) (prog1 (buffer-hash) (widen)))",
            "\"a9993e364706816aba3e25717850c26c9cd0d89d\"",
        ),
        (
            "(progn (erase-buffer) (insert \"\u{e9}\") (buffer-hash \"bh\"))",
            "\"bf15be717ac1b080b4f1c456692825891ff5073d\"",
        ),
        (
            "(progn (erase-buffer) (insert (make-string 100 ?a)) (goto-char 51) (insert \"b\") (buffer-hash (current-buffer)))",
            "\"e17f5c7370f1dd97c49c9d3cbc1abf22033df9e1\"",
        ),
        (
            "(progn (erase-buffer) (insert (make-string 100000 ?x)) (buffer-hash))",
            "\"f6ee99edde6199a3e982c46ef72bdd5cb5e41ddf\"",
        ),
        (
            "(buffer-hash \"no such buffer\")",
            "error: (error \"No buffer named no such buffer\")",
        ),
        ("(buffer-hash 1)", "error: (wrong-type-argument stringp 1)"),
    ]);
}

#[test]
fn line_number_at_pos() {
    check(&[
        (
            "(progn (set-buffer (get-buffer-create \"lines\")) (insert \"one\\ntwo\\r\\nthree\\rfour\") nil)",
            "nil",
        ),
        ("(line-number-at-pos)", "3"),
        ("(line-number-at-pos 1)", "1"),
        ("(line-number-at-pos 4)", "1"),
        ("(line-number-at-pos 5)", "2"),
        ("(line-number-at-pos (point-max))", "3"),
        (
            "(line-number-at-pos 0)",
            "error: (args-out-of-range 0 1 20)",
        ),
        (
            "(line-number-at-pos 21)",
            "error: (args-out-of-range 21 1 20)",
        ),
        (
            "(line-number-at-pos 'x)",
            "error: (wrong-type-argument fixnump x)",
        ),
        (
            "(let ((selective-display t)) (line-number-at-pos (point-max)))",
            "5",
        ),
        (
            "(let ((selective-display 3)) (line-number-at-pos (point-max)))",
            "3",
        ),
        (
            "(progn (narrow-to-region 5 20) (line-number-at-pos 10))",
            "2",
        ),
        ("(line-number-at-pos 10 t)", "3"),
        ("(line-number-at-pos 2)", "1"),
        ("(line-number-at-pos 2 t)", "1"),
        ("(progn (widen) (line-number-at-pos (copy-marker 9)))", "2"),
    ]);
}

#[test]
fn sequences() {
    check(&[
        ("(length '(1 2 3))", "3"),
        ("(length \"héllo\")", "5"),
        ("(string-bytes \"héllo\")", "6"),
        ("(length [1 2])", "2"),
        (
            "(length '(1 . 2))",
            "error: (wrong-type-argument listp (1 . 2))",
        ),
        ("(safe-length '(1 2 . 3))", "2"),
        ("(proper-list-p '(1 2))", "2"),
        ("(proper-list-p '(1 . 2))", "nil"),
        ("(length< '(1 2) 3)", "t"),
        ("(length= \"ab\" 2)", "t"),
        ("(nth 1 '(a b c))", "b"),
        ("(nth 5 '(a b c))", "nil"),
        ("(nthcdr 2 '(a b c))", "(c)"),
        ("(elt [a b] 1)", "b"),
        ("(reverse '(1 2 3))", "(3 2 1)"),
        ("(nreverse (list 1 2 3))", "(3 2 1)"),
        ("(reverse [1 2])", "[2 1]"),
        ("(reverse \"abc\")", "\"cba\""),
        ("(append '(1) '(2) 3)", "(1 2 . 3)"),
        ("(append [1 2] nil)", "(1 2)"),
        ("(append)", "nil"),
        ("(concat \"ab\" '(?c) [?d])", "\"abcd\""),
        ("(concat \"a\" \"é\")", "\"aé\""),
        ("(vconcat '(1) [2] \"a\")", "[1 2 97]"),
        ("(substring \"hello\" 1 3)", "\"el\""),
        ("(substring \"hello\" -3)", "\"llo\""),
        ("(substring [1 2 3] 1)", "[2 3]"),
        (
            "(substring \"hello\" 2 1)",
            "error: (args-out-of-range \"hello\" 2 1)",
        ),
        ("(copy-sequence '(1 2))", "(1 2)"),
        ("(take 2 '(1 2 3))", "(1 2)"),
        ("(take 0 '(1 2 3))", "nil"),
        ("(ntake 2 (list 1 2 3))", "(1 2)"),
        ("(nconc (list 1) nil (list 2 3))", "(1 2 3)"),
        ("(fillarray (make-vector 2 0) 7)", "[7 7]"),
    ]);
}

#[test]
fn membership() {
    check(&[
        ("(member \"b\" '(\"a\" \"b\"))", "(\"b\")"),
        ("(memq 'c '(a b))", "nil"),
        ("(memql 1.0 '(1 1.0))", "(1.0)"),
        ("(assq 'b '((a . 1) (b . 2)))", "(b . 2)"),
        ("(assoc \"b\" '((\"a\" . 1) (\"b\" . 2)))", "(\"b\" . 2)"),
        ("(assoc 2 '((1 . a) (3 . b)) #'<)", "(1 . a)"),
        ("(assoc 2 '((3 . a) (1 . b)) #'>)", "(3 . a)"),
        ("(rassq 2 '((a . 1) (b . 2)))", "(b . 2)"),
        ("(rassoc \"x\" '((a . \"x\")))", "(a . \"x\")"),
        ("(delq 'a (list 'a 'b 'a))", "(b)"),
        ("(delete 1 (list 1 2 1))", "(2)"),
        ("(delete \"a\" [\"a\" \"b\"])", "[\"b\"]"),
        ("(copy-alist '((a . 1)))", "((a . 1))"),
    ]);
}

#[test]
fn sorting() {
    check(&[
        ("(sort (list 3 1 2) #'<)", "(1 2 3)"),
        ("(sort '(3 1 2))", "(1 2 3)"),
        (
            "(sort [\"b\" \"a\"] :lessp #'string-lessp)",
            "[\"a\" \"b\"]",
        ),
        ("(sort '((2 . a) (1 . b)) :key #'car)", "((1 . b) (2 . a))"),
        ("(sort (list 1 3 2) :reverse t)", "(3 2 1)"),
        (
            "(sort '((1 . a) (0 . b) (1 . c)) :key #'car)",
            "((0 . b) (1 . a) (1 . c))",
        ),
        ("(value< 'a 'b)", "t"),
        ("(value< '(1 2) '(1 3))", "t"),
        ("(value< 1 \"a\")", "error: (type-mismatch 1 \"a\")"),
    ]);
}

#[test]
fn equality() {
    check(&[
        ("(equal '(1 (2 \"x\")) '(1 (2 \"x\")))", "t"),
        ("(eq \"a\" \"a\")", "nil"),
        ("(eql 1.0 1.0)", "t"),
        ("(equal 0.0 -0.0)", "nil"),
        ("(eql 2.0 2)", "nil"),
        ("(equal [1 (2)] [1 (2)])", "t"),
        ("(equal (expt 2 70) (expt 2 70))", "t"),
        ("(eql (expt 2 70) (expt 2 70))", "t"),
        ("(eq (expt 2 70) (expt 2 70))", "nil"),
    ]);
}

#[test]
fn plists_and_mapping() {
    check(&[
        ("(plist-get '(a 1 b 2) 'b)", "2"),
        ("(plist-get '(\"a\" 1) \"a\" #'equal)", "1"),
        ("(plist-put (list 'a 1) 'b 2)", "(a 1 b 2)"),
        ("(plist-member '(a nil) 'a)", "(a nil)"),
        ("(mapcar #'1+ '(1 2 3))", "(2 3 4)"),
        ("(mapcar #'identity \"ab\")", "(97 98)"),
        ("(mapconcat #'identity '(\"a\" \"b\") \"-\")", "\"a-b\""),
        ("(mapconcat #'identity '(\"a\" \"b\"))", "\"ab\""),
        ("(mapcan #'list '(1 2))", "(1 2)"),
        ("(mapc #'ignore '(1 2))", "error: (void-function ignore)"),
    ]);
}

#[test]
fn hash_tables() {
    check(&[
        (
            "(let ((h (make-hash-table :test 'equal))) (puthash \"a\" 1 h) (gethash \"a\" h))",
            "1",
        ),
        (
            "(let ((h (make-hash-table))) (puthash 'a 1 h) (puthash 'b 2 h) (remhash 'a h) (hash-table-count h))",
            "1",
        ),
        (
            "(let ((h (make-hash-table))) (puthash 1 2 h) h)",
            "#s(hash-table data (1 2))",
        ),
        (
            "(let ((h (make-hash-table :test 'eq)) (r nil)) (puthash 'a 1 h) (puthash 'b 2 h) (maphash #'(lambda (k v) (setq r (cons k r))) h) r)",
            "(b a)",
        ),
        ("(gethash 'missing (make-hash-table) 'default)", "default"),
        ("(hash-table-test (make-hash-table :test 'equal))", "equal"),
        (
            "(make-hash-table :test 'bogus)",
            "error: (error \"Invalid hash table test\" bogus)",
        ),
        (
            "(read \"#s(hash-table test equal data (\\\"k\\\" 1))\")",
            "#s(hash-table test equal data (\"k\" 1))",
        ),
        (
            "(progn (define-hash-table-test 'ci #'(lambda (a b) (string-equal (downcase a) (downcase b))) #'(lambda (k) (sxhash-equal (downcase k)))) (let ((h (make-hash-table :test 'ci))) (puthash \"A\" 1 h) (gethash \"a\" h)))",
            "1",
        ),
    ]);
}

#[test]
fn strings() {
    check(&[
        ("(string-equal \"abc\" 'abc)", "t"),
        ("(string-lessp \"abc\" \"abd\")", "t"),
        ("(compare-strings \"abc\" nil nil \"abd\" nil nil)", "-3"),
        ("(compare-strings \"ABC\" nil nil \"abc\" nil nil t)", "t"),
        ("(string-version-lessp \"foo2\" \"foo10\")", "t"),
        ("(string-search \"lo\" \"hello\")", "3"),
        ("(string-search \"zz\" \"hello\")", "nil"),
        ("(string-distance \"kitten\" \"sitting\")", "3"),
        ("(base64-encode-string \"hello\")", "\"aGVsbG8=\""),
        ("(base64-decode-string \"aGVsbG8=\")", "\"hello\""),
        ("(base64url-encode-string \"hi?\" t)", "\"aGk_\""),
        ("(upcase \"abc\")", "\"ABC\""),
        ("(upcase ?a)", "65"),
        ("(capitalize \"hello world\")", "\"Hello World\""),
        ("(downcase \"ÀB\")", "\"àb\""),
        ("(string ?a ?b)", "\"ab\""),
        ("(string-width \"日本\")", "2"),
        (
            "(progn (set-char-table-range char-width-table '(#x4E00 . #x9FFF) 2) (string-width \"日本\"))",
            "4",
        ),
        ("(char-width ?a)", "1"),
        ("(char-width ?\\t)", "8"),
        ("(char-width ?\\n)", "0"),
        ("(char-width ?\\C-a)", "2"),
        ("(char-width #x80)", "4"),
        ("(multibyte-string-p \"é\")", "t"),
        ("(multibyte-string-p \"e\")", "nil"),
        ("(string-to-multibyte \"\\351\")", "\"\\351\""),
        ("(string-as-unibyte \"é\")", "\"\\303\\251\""),
        ("(string-make-unibyte \"é\")", "\"\\351\""),
    ]);
}

#[test]
fn char_tables() {
    check(&[
        (
            "(let ((ct (make-char-table 'test))) (set-char-table-range ct '(?a . ?z) 'lower) (list (aref ct ?m) (aref ct ?A)))",
            "(lower nil)",
        ),
        (
            "(let ((ct (make-char-table 'test 0))) (aset ct ?x 1) (char-table-range ct ?x))",
            "1",
        ),
        (
            "(let ((ct (make-char-table 'test)) (r nil)) (set-char-table-range ct '(?a . ?c) 1) (map-char-table #'(lambda (k v) (setq r (cons k r))) ct) r)",
            "((97 . 99))",
        ),
        (
            "(let* ((p (make-char-table 'test)) (c (make-char-table 'test))) (aset p ?q 'from-parent) (set-char-table-parent c p) (aref c ?q))",
            "from-parent",
        ),
        (
            "(let ((ct (make-char-table 'test))) (set-char-table-range ct '(#x10000 . #x10010) 'astral) (aref ct #x10005))",
            "astral",
        ),
    ]);
}

#[test]
fn features() {
    check(&[
        ("(progn (provide 'my-feature) (featurep 'my-feature))", "t"),
        ("(featurep 'not-a-feature)", "nil"),
        ("(require 'my-feature)", "my-feature"),
    ]);
}

#[test]
fn printing() {
    check(&[
        ("(prin1-to-string 'a)", "\"a\""),
        ("(prin1-to-string \"x\" t)", "\"x\""),
        (
            "(let ((print-length 2)) (prin1-to-string '(1 2 3)))",
            "\"(1 2 ...)\"",
        ),
        (
            "(let ((print-level 1)) (prin1-to-string '(1 (2))))",
            "\"(1 ...)\"",
        ),
        (
            "(let ((print-circle t) (x (list 1))) (prin1-to-string (list x x)))",
            "\"(#1=(1) #1#)\"",
        ),
        (
            "(error-message-string '(wrong-type-argument listp 1))",
            "\"Wrong type argument: listp, 1\"",
        ),
        ("(error-message-string '(error \"Foo\" 1))", "\"Foo: 1\""),
        (
            "(error-message-string '(end-of-file))",
            "\"End of file during parsing\"",
        ),
        ("(prin1-to-string (make-symbol \"g\"))", "\"g\""),
        (
            "(let ((print-gensym t)) (prin1-to-string (make-symbol \"g\")))",
            "\"#:g\"",
        ),
        ("(prin1-to-string '(quote x))", "\"'x\""),
        ("(prin1-to-string 1.0e+INF)", "\"1.0e+INF\""),
        (
            "(let ((float-output-format \"%.3f\")) (prin1-to-string 1.5))",
            "\"1.500\"",
        ),
    ]);
}

#[test]
fn equal_cycles() {
    check(&[
        (
            "(let ((a (list 1 2)) (b (list 1 2))) (setcdr (cdr a) a) (setcdr (cdr b) b) (equal a b))",
            "t",
        ),
        (
            "(let ((a (list nil)) (b (list nil))) (setcar a a) (setcar b b) (equal a b))",
            "t",
        ),
        (
            "(let ((a (list 1))) (setcdr a a) (equal a '(1 1 1)))",
            "nil",
        ),
        (
            "(let ((a (list 1 2)) (b (list 1 3))) (setcdr (cdr a) a) (setcdr (cdr b) b) (equal a b))",
            "nil",
        ),
    ]);
}
