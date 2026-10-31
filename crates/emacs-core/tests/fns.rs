mod common;

use common::check;

#[test]
fn overriding_plist_environment() {
    check(&[
        ("overriding-plist-environment", "nil"),
        ("(special-variable-p 'overriding-plist-environment)", "t"),
        ("(put 'opl-sym 'opl-prop 2)", "2"),
        (
            "(funcall (eval '(function (lambda () (let ((overriding-plist-environment '((opl-sym opl-prop 1)))) (list (get 'opl-sym 'opl-prop) (get 'opl-sym 'other))))) t))",
            "(1 nil)",
        ),
        ("(get 'opl-sym 'opl-prop)", "2"),
    ]);
}

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
        ("(length '(1 . 2))", "error: (wrong-type-argument listp 2)"),
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
fn sxhash_values() {
    check(&[
        ("(sxhash-equal \"abc\")", "8059383"),
        ("(sxhash-equal 5)", "4"),
        ("(sxhash-eq 1)", "0"),
        ("(sxhash-equal '(1 2))", "22"),
        ("(sxhash-equal [1 2])", "662"),
        (
            "(progn (set-buffer (get-buffer-create \"sx\")) (insert \"abc\") (= (sxhash-equal (point-marker)) (sxhash-equal (point-marker))))",
            "t",
        ),
        ("(= (sxhash-eql 1.5) (sxhash-equal 1.5))", "t"),
        (
            "(= (sxhash-eql (expt 2 70)) (sxhash-equal (expt 2 70)))",
            "t",
        ),
        (
            "(= (sxhash-equal-including-properties (propertize \"a\" 'x 1)) (sxhash-equal \"a\"))",
            "nil",
        ),
        (
            "(= (sxhash-equal-including-properties \"a\") (sxhash-equal \"a\"))",
            "t",
        ),
    ]);
}

#[test]
fn equal_including_properties() {
    check(&[
        (
            "(equal-including-properties (concat (propertize \"abc\" 'a 1) (propertize \"de\" 'a 1)) (propertize \"abcde\" 'a 1))",
            "t",
        ),
        (
            "(equal-including-properties (propertize \"ab\" 'a 1 'b 2) (propertize \"ab\" 'b 2 'a 1))",
            "t",
        ),
        (
            "(equal-including-properties (propertize \"ab\" 'a \"x\") (propertize \"ab\" 'a \"x\"))",
            "t",
        ),
        (
            "(equal-including-properties (propertize \"ab\" 'a 1) \"ab\")",
            "nil",
        ),
        (
            "(equal-including-properties (concat \"a\" (propertize \"b\" 'a 1)) (propertize \"ab\" 'a 1))",
            "nil",
        ),
    ]);
}

#[test]
fn buffer_line_statistics() {
    check(&[
        (
            "(progn (set-buffer (get-buffer-create \"bls\")) (insert \"123\\n12345\\n123\") (buffer-line-statistics))",
            "(3 5 3.6666666666666665)",
        ),
        (
            "(progn (set-buffer (get-buffer-create \"bls2\")) (buffer-line-statistics))",
            "(0 0 0.0)",
        ),
        (
            "(progn (set-buffer (get-buffer-create \"bls3\")) (insert \"\\n\\n\\n\") (buffer-line-statistics (current-buffer)))",
            "(3 0 0.0)",
        ),
        (
            "(buffer-line-statistics \"no such\")",
            "error: (error \"No buffer named no such\")",
        ),
    ]);
}

#[test]
fn circular_and_dotted_lists() {
    check(&[
        (
            "(progn (fset 'cyc (function (lambda (x) (let ((l (list x x x))) (setcdr (cdr (cdr l)) l) l)))) (fset 'err (function (lambda (f) (condition-case e (funcall f) (error (car e)))))) 'cyc)",
            "cyc",
        ),
        (
            "(mapcar 'err (list (function (lambda () (memq 2 (cyc 1)))) (function (lambda () (member \"a\" (cyc \"b\")))) (function (lambda () (memql 2.0 (cyc 1.0)))) (function (lambda () (assq 2 (cyc '(1 . 2))))) (function (lambda () (assoc \"a\" (cyc '(\"b\" . 2))))) (function (lambda () (rassq 3 (cyc '(1 . 2))))) (function (lambda () (rassoc \"a\" (cyc '(1 . \"b\"))))) (function (lambda () (delq 2 (cyc 1)))) (function (lambda () (delete \"a\" (cyc \"b\")))) (function (lambda () (reverse (cyc 1)))) (function (lambda () (nconc (cyc 1) 'tail))) (function (lambda () (plist-put (cyc 1) 2 3))) (function (lambda () (plist-member (cyc 1) 2))) (function (lambda () (length (cyc 1))))))",
            "(circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list)",
        ),
        (
            "(list (consp (memql 1.0 (cyc 1.0))) (plist-get (cyc 1) 2) (proper-list-p (cyc 1)) (>= (safe-length (cyc 1)) 3) (length< (cyc 1) 5))",
            "(t nil nil t nil)",
        ),
        (
            "(progn (setplist 'cyc-plist (cyc 'a)) (get 'cyc-plist 'b))",
            "nil",
        ),
        ("(delq 1 '(1 . 2))", "error: (wrong-type-argument listp 2)"),
        ("(reverse '(1 . 2))", "error: (wrong-type-argument listp 2)"),
        (
            "(rassq 2 '((0 . 1) . x))",
            "error: (wrong-type-argument listp ((0 . 1) . x))",
        ),
        (
            "(assoc 2 '((1) . x) (function equal))",
            "error: (wrong-type-argument listp ((1) . x))",
        ),
        ("(nconc 1 '(2))", "error: (wrong-type-argument consp 1)"),
        (
            "(plist-member '(a 1 . b) 'c)",
            "error: (wrong-type-argument plistp (a 1 . b))",
        ),
        (
            "(plist-put '(a 1 b) 'c 2)",
            "error: (wrong-type-argument plistp (a 1 b))",
        ),
        (
            "(memql (expt 2 70) (list 1 (expt 2 70)))",
            "(1180591620717411303424)",
        ),
        (
            "(memq 3 '(1 . 2))",
            "error: (wrong-type-argument listp (1 . 2))",
        ),
        ("(length= '(1 2) 2)", "t"),
        ("(length> '(1 2 3) 2)", "t"),
        ("(proper-list-p '(1 2))", "2"),
        ("(proper-list-p '(1 . 2))", "nil"),
        ("(safe-length 'a)", "0"),
    ]);
}

#[test]
fn take_and_nthcdr() {
    check(&[
        ("(ntake 1 'a)", "error: (wrong-type-argument listp a)"),
        (
            "(ntake 2 '(a . b))",
            "error: (wrong-type-argument listp (a . b))",
        ),
        ("(ntake (- most-negative-fixnum 1) (list 1 2))", "nil"),
        ("(ntake (1+ most-positive-fixnum) (list 1 2))", "(1 2)"),
        ("(ntake 'x nil)", "error: (wrong-type-argument integerp x)"),
        ("(ntake 0 'a)", "nil"),
        ("(ntake 1 (list 1 2))", "(1)"),
        (
            "(nthcdr 2 '(1 . 2))",
            "error: (wrong-type-argument listp (1 . 2))",
        ),
        ("(nthcdr 5 '(1 2))", "nil"),
        ("(nthcdr -1 '(1))", "(1)"),
        ("(nthcdr (expt 2 70) '(1 2))", "nil"),
        (
            "(let ((l (list 1 2 3))) (setcdr (cdr (cdr l)) l) (car (nthcdr 1000 l)))",
            "2",
        ),
        (
            "(let ((l (list 1 2 3))) (setcdr (cdr (cdr l)) l) (list (car (nthcdr (expt 2 70) l)) (car (nthcdr (1+ (expt 2 70)) l))))",
            "(2 3)",
        ),
        ("(nthcdr 200 (make-list 150 1))", "nil"),
    ]);
}

#[test]
fn random_numbers() {
    check(&[
        ("(random -1)", "error: (args-out-of-range -1)"),
        ("(random 0)", "error: (args-out-of-range 0)"),
        (
            "(random (- (expt 2 70)))",
            "error: (args-out-of-range -1180591620717411303424)",
        ),
        ("(random 1)", "0"),
        ("(equal (random \"seed\") (random \"seed\"))", "t"),
        (
            "(let ((n (random (expt 2 70)))) (and (integerp n) (<= 0 n) (< n (expt 2 70))))",
            "t",
        ),
        (
            "(let ((n (random))) (and (integerp n) (<= most-negative-fixnum n most-positive-fixnum)))",
            "t",
        ),
        (
            "(progn (get-buffer-create \" rn\") (string-match \"\\\\` rn-[0-9]+\\\\'\" (generate-new-buffer-name \" rn\")))",
            "0",
        ),
    ]);
}

#[cfg(windows)]
#[test]
fn w32_random_generator() {
    let mut r = emacs_core::sysdep::Random::default();
    r.srandom(1);
    assert_eq!(r.random(), -1_415_428_051);
}

#[cfg(unix)]
#[test]
fn glibc_random_generator() {
    let mut r = emacs_core::sysdep::Random::default();
    r.srandom(1);
    assert_eq!(r.random(), 1_804_289_383);
    assert_eq!(r.random(), 846_930_886);
}

#[test]
fn value_ordering() {
    check(&[
        (
            "(list (value< (position-symbol 'a 1) (position-symbol 'b 2)) (value< (position-symbol 'a 1) (position-symbol 'a 2)))",
            "(t nil)",
        ),
        (
            "(value< (position-symbol 'a 1) 'b)",
            "error: (type-mismatch #<symbol a at 1> b)",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (list (value< (position-symbol 'a 1) 'b) (value< 'a (position-symbol 'b 1))))",
            "(t t)",
        ),
        (
            "(progn (set-buffer (get-buffer-create \"vo1\")) (insert \"abcdef\") (let ((other (get-buffer-create \"vo2\"))) (list (value< (copy-marker 2) (copy-marker 3)) (value< (make-marker) (copy-marker 3)) (value< (current-buffer) other) (value< other (current-buffer)) (value< (set-marker (make-marker) 1 other) (copy-marker 1)))))",
            "(t t t nil nil)",
        ),
        (
            "(let ((a (list 1 2 3)) (b (list 1 2 3))) (setcdr (cdr (cdr a)) a) (setcdr (cdr (cdr b)) b) (car (condition-case e (value< a b) (error e))))",
            "circular-list",
        ),
        ("(value< (make-hash-table) (make-hash-table))", "nil"),
        (
            "(value< [1] (record 'a))",
            "error: (type-mismatch [1] #s(a))",
        ),
        (
            "(list (value< 1 1.5) (value< 2.0 1) (value< 0.0e+NaN 1))",
            "(t nil nil)",
        ),
        (
            "(list (value< (expt 2 70) 1) (value< 1 (expt 2 70)) (value< 1.0 (expt 2 70)))",
            "(nil t t)",
        ),
        ("(value< '(1 . 2) '(1 . 3))", "t"),
        ("(value< '(1 2) 'a)", "error: (type-mismatch (1 2) a)"),
        ("(value< '(1) nil)", "nil"),
        ("(value< nil '(1))", "t"),
        (
            "(list (string-lessp \"\\200\" (string-to-multibyte \"\\200a\")) (string-lessp (string-to-multibyte \"\\200a\") \"\\200\") (value< \"\\200\" \"\\200\"))",
            "(t nil nil)",
        ),
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
