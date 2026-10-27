mod common;

use common::check;

#[test]
fn font_specs() {
    check(&[
        (
            "(font-spec :family \"Mono\" :weight 'bold)",
            "#<font-spec nil nil Mono nil nil bold nil nil nil nil nil nil nil>",
        ),
        ("(type-of (font-spec))", "font-spec"),
        ("(fontp (font-spec))", "t"),
        ("(fontp (font-spec) 'font-entity)", "nil"),
        ("(font-get (font-spec :weight 'bold) :weight)", "bold"),
        (
            "(font-get (font-spec :weight 'demibold) :weight)",
            "demibold",
        ),
        (
            "(let ((s (font-spec))) (font-put s :slant 'italic) (font-get s :slant))",
            "italic",
        ),
        ("(font-get (font-spec :spacing 'm) :spacing)", "100"),
        ("(font-get (font-spec :foo 1) :foo)", "1"),
        (
            "(font-spec :weight 'zork)",
            "error: (error \"invalid font property\" (:weight . zork))",
        ),
    ]);
}

#[test]
fn font_names() {
    check(&[
        (
            "(font-get (font-spec :name \"Monospace-12\") :size)",
            "12.0",
        ),
        (
            "(font-get (font-spec :name \"Monospace-12\") :family)",
            "Monospace",
        ),
        (
            "(font-get (font-spec :name \"Monospace-12:weight=bold\") :weight)",
            "bold",
        ),
        (
            "(font-get (font-spec :name \"DejaVu Sans Mono Bold 10\") :weight)",
            "bold",
        ),
        (
            "(font-get (font-spec :name \"DejaVu Sans Mono Bold 10\") :size)",
            "10.0",
        ),
        (
            "(font-get (font-spec :name \"DejaVu Sans Mono Bold 10\") :family)",
            "DejaVu\\ Sans\\ Mono",
        ),
        (
            "(font-get (font-spec :name \"-misc-fixed-medium-r-normal--13-*-*-*-c-60-iso8859-1\") :registry)",
            "iso8859-1",
        ),
        (
            "(font-get (font-spec :name \"-misc-fixed-medium-r-normal--13-*-*-*-c-60-iso8859-1\") :weight)",
            "medium",
        ),
        (
            "(font-get (font-spec :name \"-misc-fixed-medium-r-normal--13-*-*-*-c-60-iso8859-1\") :size)",
            "13",
        ),
        (
            "(font-get (font-spec :name \"-misc-fixed-medium-r-normal--13-*-*-*-c-60-iso8859-1\") :spacing)",
            "110",
        ),
        (
            "(font-xlfd-name (font-spec :family \"fixed\" :size 12))",
            "\"-*-fixed-*-*-*-*-12-*-*-*-*-*-*-*\"",
        ),
        (
            "(font-xlfd-name (font-spec :family \"fixed\" :size 12) t)",
            "\"-*-fixed-*-12-*\"",
        ),
        (
            "(font-get (font-spec :name \"*-courier-*\") :family)",
            "courier",
        ),
    ]);
}
