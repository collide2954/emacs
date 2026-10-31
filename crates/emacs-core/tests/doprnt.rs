mod common;

use common::{check, on_big_stack};
use emacs_core::doprnt::Arg;
use emacs_core::lisp::Lisp;

fn doprnt(lisp: &mut Lisp, format: &str, args: &[Arg]) -> Result<Vec<u8>, String> {
    lisp.doprnt(format, args).map_err(|flow| {
        let condition = lisp.flow_condition(&flow);
        lisp.prin1_to_string(condition)
    })
}

fn text(lisp: &mut Lisp, format: &str, args: &[Arg]) -> String {
    match doprnt(lisp, format, args) {
        Ok(bytes) => String::from_utf8(bytes).expect("utf-8 output"),
        Err(e) => format!("error: {e}"),
    }
}

#[test]
fn doprnt_strings_and_chars() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        assert_eq!(text(l, "plain", &[]), "plain");
        assert_eq!(
            text(
                l,
                "%s|%5s|%-5s|%S",
                &[
                    Arg::Str(b"ab"),
                    Arg::Str(b"ab"),
                    Arg::Str(b"ab"),
                    Arg::Str(b"cd")
                ]
            ),
            "ab|   ab|ab   |cd"
        );
        assert_eq!(
            text(l, "%.1s|%05s", &[Arg::Str(b"abc"), Arg::Str(b"x")]),
            "abc|    x"
        );
        assert_eq!(text(l, "[%4s]", &[Arg::Str(b"\x01")]), "[  \x01]");
        assert_eq!(text(l, "[%4s]", &[Arg::Str(b"\t")]), "[\t]");
        assert_eq!(text(l, "[%s]", &[Arg::Str(b"ab\0cd")]), "[ab]");
        assert_eq!(
            text(l, "%c%c", &[Arg::Int(0x41), Arg::Int(0xE9)]),
            "A\u{e9}"
        );
        assert_eq!(
            text(l, "[%3c|%-3c]", &[Arg::Int(0x61), Arg::Int(0x62)]),
            "[  a|b  ]"
        );
        assert_eq!(text(l, "100%% %s", &[Arg::Str(b"done")]), "100% done");
        assert_eq!(
            doprnt(l, "%c", &[Arg::Int(0x3F_FFFF)]),
            Ok(vec![0xC1, 0xBF])
        );
        assert_eq!(text(l, "\u{e9}t\u{e9}", &[]), "\u{e9}t\u{e9}");
        assert!(
            l.eval_string("(progn (put 'display-table 'char-table-extra-slots 6) (setq buffer-display-table (make-char-table 'display-table)) (aset buffer-display-table ?a [?x ?y ?z]))")
                .is_ok()
        );
        assert_eq!(
            text(l, "[%4s|%-4c]", &[Arg::Str(b"a"), Arg::Int(0x61)]),
            "[ a|a ]"
        );
    });
}

#[test]
fn char_width_uses_display_table() {
    check(&[
        ("(char-width ?a)", "1"),
        ("(put 'display-table 'char-table-extra-slots 6)", "6"),
        (
            "(let ((buffer-display-table (make-char-table 'display-table))) (aset buffer-display-table ?a [?x ?y ?z]) (aset buffer-display-table ?b [(?\\t . 0) -1]) (list (char-width ?a) (char-width ?b) (char-width ?c)))",
            "(3 8 1)",
        ),
        (
            "(let ((standard-display-table (make-char-table 'display-table))) (aset standard-display-table ?a [?x ?y]) (char-width ?a))",
            "2",
        ),
    ]);
}

#[test]
fn doprnt_numbers() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        assert_eq!(
            text(
                l,
                "%d|%5d|%-5d|%05d|%+d|% d|%.3d|%5.3d|%.0d|%-05d|%05.3d",
                &[
                    Arg::Int(-42),
                    Arg::Int(42),
                    Arg::Int(42),
                    Arg::Int(-42),
                    Arg::Int(7),
                    Arg::Int(7),
                    Arg::Int(7),
                    Arg::Int(7),
                    Arg::Int(0),
                    Arg::Int(42),
                    Arg::Int(42),
                ]
            ),
            "-42|   42|42   |-0042|+7| 7|007|  007||42   |  042"
        );
        assert_eq!(
            text(
                l,
                "%o|%u|%x|%X|%04x|%+u|% x|%ld|%lld|%td",
                &[
                    Arg::Uint(8),
                    Arg::Uint(4_294_967_295),
                    Arg::Uint(255),
                    Arg::Uint(255),
                    Arg::Uint(26),
                    Arg::Uint(5),
                    Arg::Uint(5),
                    Arg::Int(-1),
                    Arg::Int(i64::MIN),
                    Arg::Int(12),
                ]
            ),
            "10|4294967295|ff|FF|001a|5|5|-1|-9223372036854775808|12"
        );
        assert_eq!(
            text(
                l,
                "%e|%f|%g|%.2f|%10.3e|%-8.1f|%+g|%08.2f|%g",
                &[
                    Arg::Float(1.5),
                    Arg::Float(1.5),
                    Arg::Float(1.5),
                    Arg::Float(1.23456),
                    Arg::Float(12345.678),
                    Arg::Float(2.26),
                    Arg::Float(1e20),
                    Arg::Float(-1.23456),
                    Arg::Float(0.0001),
                ]
            ),
            "1.500000e+00|1.500000|1.5|1.23| 1.235e+04|2.3     |+1e+20|-0001.23|0.0001"
        );
    });
}

#[test]
fn doprnt_quotes() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        assert_eq!(
            text(l, "`%s' isn't", &[Arg::Str(b"a`b'c")]),
            "\u{2018}a`b'c\u{2019} isn\u{2019}t"
        );
        assert!(l.eval_string("(setq text-quoting-style 'grave)").is_ok());
        assert_eq!(text(l, "`%s' isn't", &[Arg::Str(b"x")]), "`x' isn't");
        assert!(l.eval_string("(setq text-quoting-style 'straight)").is_ok());
        assert_eq!(text(l, "`%s' isn't", &[Arg::Str(b"x")]), "'x' isn't");
    });
}

#[test]
fn doprnt_errors() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        assert_eq!(
            text(l, "ab %q `cd'", &[]),
            "error: (error \"Invalid format operation %q `cd'\")"
        );
        assert_eq!(
            text(l, "ab %5", &[]),
            "error: (error \"Invalid format operation %5\")"
        );
        assert_eq!(
            text(l, "ab %", &[]),
            "error: (error \"Invalid format operation %\")"
        );
        assert_eq!(
            text(l, "%99999999999d", &[Arg::Int(1)]),
            "error: (error \"Format width or precision too large\")"
        );
    });
}

#[test]
fn error_converts_quotes_in_format_only() {
    check(&[
        (
            "(progn (get-buffer-create \"b'q\") (set-buffer (get-buffer-create \"a\")) (rename-buffer \"b'q\"))",
            "error: (error \"Buffer name \u{2018}b'q\u{2019} is in use\")",
        ),
        (
            "(let ((text-quoting-style 'grave)) (rename-buffer \"b'q\"))",
            "error: (error \"Buffer name `b'q' is in use\")",
        ),
        (
            "(let ((text-quoting-style 'straight)) (rename-buffer \"b'q\"))",
            "error: (error \"Buffer name 'b'q' is in use\")",
        ),
        (
            "(char-table-range (make-char-table 'foo) \"x\")",
            "error: (error \"Invalid RANGE argument to \u{2018}char-table-range\u{2019}\")",
        ),
        (
            "(format \"%d\" 'x)",
            "error: (error \"Format specifier doesn\u{2019}t match argument type\")",
        ),
        (
            "(handler-bind-1 (function car) 'error)",
            "error: (error \"Trailing CONDITIONS without HANDLER in \u{2018}handler-bind\u{2018}\")",
        ),
        (
            "(progn (define-category ?a \"x\") (define-category ?a \"y\"))",
            "error: (error \"Category \u{2018}a\u{2019} is already defined\")",
        ),
        (
            "(unibyte-char-to-multibyte 256)",
            "error: (error \"Not a unibyte character: 256\")",
        ),
        (
            "(format \"%y\" 1)",
            "error: (error \"Invalid format operation %y\")",
        ),
        (
            "(format (string-to-unibyte \"%\\351\") 1)",
            "error: (error \"Invalid format operation char #o351\")",
        ),
        (
            "(format \"%\u{e9}\" 1)",
            "error: (error \"Invalid format operation %\u{e9}\")",
        ),
        (
            "(read \"?\\\\Mx\")",
            "error: (error \"Invalid escape char syntax: \\\\M not followed by -\")",
        ),
        (
            "(read \"\\\"\\\\u00g0\\\"\")",
            "error: (error \"Non-hex character used for Unicode escape: g (103)\")",
        ),
        (
            "(string-to-syntax \"\u{e9}\")",
            "error: (error \"Invalid syntax description letter: \u{c3}\")",
        ),
        (
            "(condition-case nil 1 \"foo\")",
            "error: (error \"Invalid condition handler: foo\")",
        ),
    ]);
}
