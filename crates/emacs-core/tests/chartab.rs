mod common;

use common::check;

#[test]
fn uniprop_compressed_blocks() {
    check(&[
        (
            "(progn (setq tbl (make-char-table 'char-code-property-table)) (set-char-table-range tbl '(#x100 . #x17f) (string 2 5 138 7)) (set-char-table-range tbl '(#x180 . #x1ff) (string 1 3 9 0 4)) (set-char-table-range tbl '(#x200 . #x27f) (string 2 5 138 7)) (set-char-table-range tbl '(#x280 . #x2ff) (string 2 5 138 7)) t)",
            "t",
        ),
        (
            "(list (aref tbl #x100) (aref tbl #x109) (aref tbl #x10a) (aref tbl #x10b) (aref tbl #x17f))",
            "(5 5 7 nil nil)",
        ),
        (
            "(numberp (string-match \"#\\\\^\\\\^\\\\[3 256 5 5 5 5 5 5 5 5 5 5 7 nil\" (prin1-to-string tbl)))",
            "t",
        ),
        (
            "(list (char-table-range tbl #x182) (char-table-range tbl #x183) (char-table-range tbl #x184) (char-table-range tbl #x185) (char-table-range tbl #x186))",
            "(nil 9 nil 4 nil)",
        ),
        (
            "(numberp (string-match \"#\\\\^\\\\^\\\\[3 384 nil nil nil 9 nil 4 nil\" (prin1-to-string tbl)))",
            "t",
        ),
        (
            "(progn (aset tbl #x201 3) (list (aref tbl #x200) (aref tbl #x201) (aref tbl #x209) (aref tbl #x20a)))",
            "(5 3 5 7)",
        ),
        (
            "(progn (set-char-table-range tbl '(#x281 . #x282) 8) (list (aref tbl #x280) (aref tbl #x281) (aref tbl #x282) (aref tbl #x283) (aref tbl #x28a)))",
            "(5 8 8 5 7)",
        ),
        (
            "(progn (set-char-table-range tbl '(0 . 127) (string 2 4 194 9)) (list (aref tbl ?A) (aref tbl ?B) (aref tbl ?C)))",
            "(4 9 nil)",
        ),
        (
            "(let ((cat (make-char-table 'char-code-property-table))) (set-char-table-range cat '(#x300 . #x37f) (string 2 1 132 12)) (let ((unicode-category-table cat)) (list (string-match \"[[:alpha:]]\" (string #x300)) (string-match \"[[:alpha:]]\" (string #x304)))))",
            "(0 nil)",
        ),
    ]);
}

#[test]
fn uniprop_decoders_and_encoders() {
    check(&[
        (
            "(progn (setq tbl (make-char-table 'char-code-property-table)) (set-char-table-range tbl '(#x100 . #x17f) (string 2 5 138 7)) (set-char-table-range tbl '(#x180 . #x1ff) (string 1 3 9 0 4)) (set-char-table-extra-slot tbl 1 0) (set-char-table-extra-slot tbl 4 [a b c d e f g h]) t)",
            "t",
        ),
        (
            "(list (get-unicode-property-internal tbl #x100) (get-unicode-property-internal tbl #x10a) (get-unicode-property-internal tbl #x10b) (get-unicode-property-internal tbl #x183))",
            "(f h a 9)",
        ),
        (
            "(progn (set-char-table-extra-slot tbl 2 1) (list (put-unicode-property-internal tbl #x103 'g) (aref tbl #x103) (get-unicode-property-internal tbl #x103)))",
            "(nil 6 g)",
        ),
        (
            "(put-unicode-property-internal tbl #x103 'zz)",
            "error: (wrong-type-argument \"Unicode property value\" zz)",
        ),
        (
            "(progn (set-char-table-extra-slot tbl 2 2) (put-unicode-property-internal tbl #x104 42) (list (aref tbl #x104) (char-table-extra-slot tbl 4)))",
            "(8 [a b c d e f g h 8])",
        ),
        (
            "(put-unicode-property-internal tbl #x104 'x)",
            "error: (wrong-type-argument fixnump x)",
        ),
        (
            "(progn (set-char-table-extra-slot tbl 2 0) (put-unicode-property-internal tbl #x105 ?z) (aref tbl #x105))",
            "122",
        ),
        (
            "(put-unicode-property-internal tbl #x105 'x)",
            "error: (wrong-type-argument integerp x)",
        ),
        (
            "(progn (set-char-table-extra-slot tbl 1 5) (prog1 (get-unicode-property-internal tbl #x100) (set-char-table-extra-slot tbl 1 0)))",
            "5",
        ),
        (
            "(get-unicode-property-internal (make-char-table 'foo) ?a)",
            "error: (error \"Invalid Unicode property table\")",
        ),
        (
            "(put-unicode-property-internal (make-char-table 'foo) ?a 1)",
            "error: (error \"Invalid Unicode property table\")",
        ),
    ]);
}

#[test]
fn unicode_property_table_internal() {
    let dir = std::env::temp_dir().join(format!("emacs-uniprop-{}", std::process::id()));
    let intl = dir.join("international");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&intl).expect("create directory");
    std::fs::write(
        intl.join("uni-test.el"),
        "(setcdr (assq 'testprop char-code-property-alist) (let ((tb (make-char-table 'char-code-property-table))) (set-char-table-range tb '(#x100 . #x17f) (string 2 5 138 7)) (set-char-table-range tb '(0 . 127) (string 2 4 194 9)) (set-char-table-extra-slot tb 1 0) (set-char-table-extra-slot tb 4 [a b c d e f g h]) tb))\n",
    )
    .expect("write");
    let ll = format!("{}/", dir.to_string_lossy().replace('\\', "/"));
    check(&[
        (
            "(progn (define-coding-system-internal 'utf-8-emacs ?U 'utf-8 '(emacs) t nil nil nil nil nil nil '(:name utf-8-emacs) nil nil) (define-coding-system-alias 'unix 'undecided-unix) t)",
            "t",
        ),
        ("(unicode-property-table-internal 'no-such-prop)", "nil"),
        (
            "(let ((char-code-property-alist (list (cons 'p 5)))) (unicode-property-table-internal 'p))",
            "5",
        ),
        (
            "(let ((char-code-property-alist '(p))) (unicode-property-table-internal 'p))",
            "nil",
        ),
        (
            "(let ((char-code-property-alist (list (cons 'p \"no-such-uni.el\")))) (unicode-property-table-internal 'p))",
            "\"no-such-uni.el\"",
        ),
        (
            &format!(
                "(let ((load-path (list {ll:?})) (char-code-property-alist (list (cons 'testprop \"uni-test.el\")))) (let ((tb (unicode-property-table-internal 'testprop))) (list (char-table-p tb) (eq tb (cdr (assq 'testprop char-code-property-alist))) (get-unicode-property-internal tb #x100) (get-unicode-property-internal tb ?B) (numberp (string-match \"#\\\\^\\\\^\\\\[3 0 4 4 4\" (prin1-to-string tb))))))"
            ),
            "(t t f 9 t)",
        ),
        (
            "(let* ((tb (make-char-table 'char-code-property-table)) (char-code-property-alist (list (cons 'q tb)))) (set-char-table-extra-slot tb 1 7) (eq (unicode-property-table-internal 'q) tb))",
            "t",
        ),
    ]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn map_char_table_like_c() {
    check(&[
        (
            "(let ((tb (make-char-table 'char-code-property-table)) (acc nil)) (set-char-table-range tb '(#x100 . #x17f) (string 2 5 138 7)) (set-char-table-extra-slot tb 1 0) (set-char-table-extra-slot tb 4 [z0 z1 z2 z3 z4 z5 z6 z7]) (map-char-table (function (lambda (k v) (setq acc (cons (cons (if (consp k) (cons (car k) (cdr k)) k) v) acc)))) tb) (nreverse acc))",
            "(((256 . 265) . z5) (266 . z7))",
        ),
        (
            "(let ((ct (make-char-table 'foo)) (keys nil)) (set-char-table-range ct '(#x41 . #x43) 1) (set-char-table-range ct '(#x61 . #x63) 2) (map-char-table (function (lambda (k v) (setq keys (cons k keys)))) ct) (eq (car keys) (car (cdr keys))))",
            "t",
        ),
        (
            "(let ((ct (make-char-table 'foo)) (acc nil)) (set-char-table-range ct '(#x41 . #x43) 1) (aset ct #x44 1) (aset ct #x3000 'w) (set-char-table-range ct '(#x10000 . #x10ffff) 'x) (map-char-table (function (lambda (k v) (setq acc (cons (cons (if (consp k) (cons (car k) (cdr k)) k) v) acc)))) ct) (nreverse acc))",
            "(((65 . 68) . 1) (12288 . w) ((65536 . 1114111) . x))",
        ),
        (
            "(let ((parent (make-char-table 'foo)) (ct (make-char-table 'foo)) (acc nil)) (set-char-table-range parent '(#x41 . #x45) 'p) (set-char-table-parent ct parent) (aset ct #x43 'c) (map-char-table (function (lambda (k v) (setq acc (cons (cons (if (consp k) (cons (car k) (cdr k)) k) v) acc)))) ct) (nreverse acc))",
            "(((65 . 66) . p) (67 . c) ((68 . 69) . p))",
        ),
    ]);
}

#[test]
fn copy_sequence_char_table_ascii() {
    check(&[(
        "(let ((a (make-char-table 'foo))) (aset a ?x 1) (let ((b (copy-sequence a))) (aset b ?x 2) (list (aref a ?x) (aref b ?x))))",
        "(1 2)",
    )]);
}
