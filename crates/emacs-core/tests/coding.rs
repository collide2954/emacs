mod common;

use common::check;

#[test]
fn builtin_coding_systems() {
    check(&[
        ("(coding-system-p 'no-conversion)", "t"),
        ("(coding-system-p 'undecided-unix)", "t"),
        ("(coding-system-p nil)", "t"),
        ("(coding-system-p 'bogus)", "nil"),
        ("(coding-system-base 'undecided-dos)", "undecided"),
        ("(coding-system-base nil)", "no-conversion"),
        (
            "(coding-system-eol-type 'undecided)",
            "[undecided-unix undecided-dos undecided-mac]",
        ),
        ("(coding-system-eol-type 'no-conversion)", "0"),
        ("(coding-system-eol-type 'undecided-mac)", "2"),
        (
            "(check-coding-system 'bogus)",
            "error: (coding-system-error bogus)",
        ),
        ("(coding-system-priority-list)", "(no-conversion undecided)"),
        ("coding-category-raw-text", "no-conversion"),
    ]);
}

#[test]
fn coding_system_registry() {
    check(&[
        ("(define-coding-system-alias 'binary 'no-conversion)", "nil"),
        ("(coding-system-base 'binary)", "no-conversion"),
        (
            "(coding-system-aliases 'no-conversion)",
            "(no-conversion binary)",
        ),
        ("(coding-system-put 'no-conversion :mnemonic ?X)", "88"),
        (
            "(plist-get (coding-system-plist 'no-conversion) :mnemonic)",
            "88",
        ),
        (
            "(define-coding-system-internal 'test-utf ?u 'utf-8 '(unicode) t nil nil nil nil nil nil '(:name test-utf) nil nil)",
            "nil",
        ),
        ("(coding-system-p 'test-utf-dos)", "t"),
        (
            "(coding-system-plist 'test-utf)",
            "(:ascii-compatible-p t :category coding-category-utf-8 :name test-utf)",
        ),
        ("(coding-system-priority-list t)", "test-utf"),
        ("(set-coding-system-priority 'undecided)", "nil"),
        ("(coding-system-priority-list t)", "undecided"),
        ("(car coding-category-list)", "coding-category-undecided"),
        ("(assoc \"test-utf\" coding-system-alist)", "(\"test-utf\")"),
    ]);
}

const TEST_UTF: &str = "(define-coding-system-internal 'test-utf ?u 'utf-8 '(unicode) t nil nil nil nil nil nil '(:name test-utf) nil nil)";
const TEST_LATIN1: &str = "(define-coding-system-internal 'test-latin1 ?1 'charset '(iso-8859-1) t nil nil nil nil nil nil '(:name test-latin1) nil)";

#[test]
fn decode_and_encode_strings() {
    check(&[
        (TEST_UTF, "nil"),
        (TEST_LATIN1, "nil"),
        (
            r#"(decode-coding-string "\303\251t\303\251" 'test-utf)"#,
            "\"été\"",
        ),
        ("last-coding-system-used", "test-utf"),
        (r#"(decode-coding-string "a\r\nb" 'test-utf)"#, "\"a\nb\""),
        ("last-coding-system-used", "test-utf-dos"),
        (
            r#"(encode-coding-string "été" 'test-utf-unix)"#,
            r#""\303\251t\303\251""#,
        ),
        (
            r#"(decode-coding-string "\351t\351" 'test-latin1-unix)"#,
            "#(\"été\" 0 3 (charset iso-8859-1))",
        ),
        (
            r#"(encode-coding-string "été" 'test-latin1-unix)"#,
            r#""\351t\351""#,
        ),
        (
            r#"(multibyte-string-p (decode-coding-string "a\351" 'no-conversion))"#,
            "nil",
        ),
        (r#"(decode-coding-string "abc" nil)"#, "\"abc\""),
        ("last-coding-system-used", "no-conversion"),
    ]);
}

#[test]
fn detect_coding() {
    check(&[
        (r#"(detect-coding-string "abc")"#, "(undecided)"),
        (r#"(detect-coding-string "a\r\nb" t)"#, "undecided-dos"),
        (TEST_UTF, "nil"),
        (
            r#"(detect-coding-string "\303\251t\303\251\n" t)"#,
            "test-utf-unix",
        ),
    ]);
}

#[test]
fn decode_coding_region_in_buffer() {
    check(&[
        ("(set-buffer (get-buffer-create \"dcr\"))", "#<buffer dcr>"),
        ("(insert \"a\\r\\nb\")", "nil"),
        ("(decode-coding-region 1 5 'undecided)", "3"),
        ("(buffer-string)", "\"a\nb\""),
        ("last-coding-system-used", "undecided-dos"),
    ]);
}

#[test]
fn encodability_queries() {
    check(&[
        (TEST_LATIN1, "nil"),
        (
            "(unencodable-char-position 0 3 'test-latin1 nil \"a\u{e9}\u{3042}\")",
            "2",
        ),
        (
            "(unencodable-char-position 0 3 'test-latin1 0 \"\u{3042}\u{e9}\u{3042}\")",
            "(0 2)",
        ),
        (
            "(check-coding-systems-region \"a\u{e9}\u{3042}\" nil '(test-latin1))",
            "((test-latin1 2))",
        ),
        ("(find-coding-systems-region-internal \"abc\" nil)", "t"),
        (
            "(memq 'test-latin1 (find-coding-systems-region-internal \"\u{e9}\" nil))",
            "(test-latin1 raw-text no-conversion)",
        ),
    ]);
}

#[test]
fn operation_coding_system() {
    check(&[
        (TEST_UTF, "nil"),
        (
            "(setq file-coding-system-alist '((\"\\\\.foo\\\\'\" . test-utf)))",
            "((\"\\\\.foo\\\\'\" . test-utf))",
        ),
        (
            "(find-operation-coding-system 'insert-file-contents \"a.foo\")",
            "(test-utf . test-utf)",
        ),
        (
            "(find-operation-coding-system 'insert-file-contents \"a.bar\")",
            "nil",
        ),
        (
            "(find-operation-coding-system 'bogus \"a\")",
            "error: (error \"Invalid first argument\")",
        ),
    ]);
}

#[test]
fn insert_file_contents_decodes() {
    let path = std::env::temp_dir().join(format!("emacs-ifc-{}.txt", std::process::id()));
    std::fs::write(&path, b"\xc3\xa9t\xc3\xa9\r\nx").expect("write temp file");
    let name = path.to_string_lossy().replace('\\', "/");
    let insert = format!("(car (cdr (insert-file-contents \"{name}\")))");
    let visit = format!("(car (cdr (insert-file-contents \"{name}\" t)))");
    check(&[
        (TEST_UTF, "nil"),
        (
            "(defalias 'make-lock-file-name (function (lambda (f) (concat (file-name-directory f) \".#\" (file-name-nondirectory f)))))",
            "make-lock-file-name",
        ),
        ("(set-buffer (get-buffer-create \"ifc\"))", "#<buffer ifc>"),
        (&insert, "5"),
        ("(buffer-string)", "\"été\nx\""),
        ("last-coding-system-used", "test-utf-dos"),
        ("(point)", "1"),
        (
            "(set-buffer (get-buffer-create \"ifc-visit\"))",
            "#<buffer ifc-visit>",
        ),
        (&visit, "5"),
        ("(buffer-modified-p)", "nil"),
        ("(stringp buffer-file-name)", "t"),
    ]);
    let _ = std::fs::remove_file(&path);
}
