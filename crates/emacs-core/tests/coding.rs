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
