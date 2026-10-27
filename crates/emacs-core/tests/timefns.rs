mod common;

use common::check;

#[test]
fn time_arithmetic() {
    check(&[
        ("(time-add 1 2)", "3"),
        ("(time-add '(1 . 2) 1)", "(3 . 2)"),
        ("(time-subtract 5 '(1 . 2))", "(9 . 2)"),
        ("(time-subtract nil nil)", "(0 . 1000000000)"),
        ("(time-less-p 1 2)", "t"),
        ("(time-less-p '(3 . 2) 1)", "nil"),
        ("(time-equal-p '(2 . 1) 2)", "t"),
        ("(time-equal-p nil 0)", "nil"),
        ("(float-time 3)", "3.0"),
        ("(float-time '(1 . 4))", "0.25"),
        ("(consp (current-time))", "t"),
    ]);
}

#[test]
fn time_conversion() {
    check(&[
        (
            "(time-convert 3.5 t)",
            "(7881299347898368 . 2251799813685248)",
        ),
        ("(time-convert '(7 . 2) 'integer)", "3"),
        ("(time-convert 1 'list)", "(0 1 0 0)"),
        ("(time-convert '(3 . 2) 1000)", "(1500 . 1000)"),
        (
            "(time-convert '(1 2 3 4) t)",
            "(65538000003000004 . 1000000000000)",
        ),
        ("(time-convert 0.0 t)", "(0 . 1)"),
        (
            "(time-convert 1 0)",
            "error: (error \"Invalid time frequency\" 0)",
        ),
        (
            "(time-add 'x 1)",
            "error: (error \"Invalid time specification\")",
        ),
    ]);
}

#[test]
fn decoding_and_encoding() {
    check(&[
        ("(decode-time 0 t)", "(0 0 0 1 1 1970 4 nil 0)"),
        ("(decode-time 86399 t)", "(59 59 23 1 1 1970 4 nil 0)"),
        ("(decode-time 0 3600)", "(0 0 1 1 1 1970 4 nil 3600)"),
        (
            "(decode-time '(3 . 2) t t)",
            "((3 . 2) 0 0 1 1 1970 4 nil 0)",
        ),
        (
            "(decode-time 0 \"EST5EDT\")",
            "(0 0 19 31 12 1969 3 nil -18000)",
        ),
        ("(encode-time '(0 0 0 1 1 2000 nil nil t))", "946684800"),
        ("(encode-time '(0 0 0 32 12 1999 nil nil t))", "946684800"),
        ("(encode-time 0 0 0 1 1 1970 t)", "0"),
        ("(current-time-string 0 t)", "\"Thu Jan  1 00:00:00 1970\""),
        ("(current-time-zone 0 t)", "(0 \"UTC\")"),
        (
            "(current-time-zone 0 '(-18000 \"EST\"))",
            "(-18000 \"EST\")",
        ),
        ("(current-time-zone 0 -16200)", "(-16200 \"-0430\")"),
    ]);
}

#[test]
fn format_time_string() {
    check(&[
        (
            "(format-time-string \"%Y-%m-%d %H:%M:%S\" 0 t)",
            "\"1970-01-01 00:00:00\"",
        ),
        (
            "(format-time-string \"%a %b %e %j %u %w %U %W %V %G %g %C %y %q\" 0 t)",
            "\"Thu Jan  1 001 4 4 00 00 01 1970 70 19 70 1\"",
        ),
        (
            "(format-time-string \"%z %:z %::z %:::z %Z\" 0 3600)",
            "\"+0100 +01:00 +01:00:00 +01 +01\"",
        ),
        (
            "(format-time-string \"%-d %_m %5Y %^a %#Z\" 0 t)",
            "\"1  1 01970 THU utc\"",
        ),
        ("(format-time-string \"%N\" '(1 . 2) t)", "\"500000000\""),
        ("(format-time-string \"%3N\" '(1 . 2) t)", "\"500\""),
        ("(format-time-string \"%s\" 12345 t)", "\"12345\""),
        (
            "(format-time-string \"%F %T %R %D %r\" 0 t)",
            "\"1970-01-01 00:00:00 00:00 01/01/70 12:00:00 AM\"",
        ),
        ("(format-time-string \"100%% %Q %\" 0 t)", "\"100% %Q %\""),
        (
            "(format-time-string \"%B %A %p %P\" 50000 t)",
            "\"January Thursday PM pm\"",
        ),
        (
            "(format-time-string \"%Z\" 15000000 \"America/New_York\")",
            "\"EDT\"",
        ),
    ]);
}
