mod common;

use common::check;

#[test]
fn initial_terminal_is_not_a_tty() {
    check(&[
        ("(tty-display-color-p)", "nil"),
        ("(tty-display-color-cells)", "0"),
        ("(tty-display-color-cells (selected-frame))", "0"),
        ("(tty-type)", "nil"),
        ("(controlling-tty-p)", "nil"),
        ("(tty-no-underline)", "nil"),
        ("(tty-enable-overline t)", "nil"),
        ("(tty-top-frame)", "nil"),
        (
            "(suspend-tty)",
            "error: (error \"Attempt to suspend a non-text terminal device\")",
        ),
        (
            "(resume-tty)",
            "error: (error \"Attempt to resume a non-text terminal device\")",
        ),
        (
            "(tty--set-output-buffer-size -1)",
            "error: (error \"Invalid output buffer size\")",
        ),
        (
            "(tty--set-output-buffer-size 10)",
            "error: (error \"Attempt to suspend a non-text terminal device\")",
        ),
        (
            "(tty--output-buffer-size)",
            "error: (error \"Not a tty terminal\")",
        ),
        ("(tty-frame-geometry)", "nil"),
        ("(tty-frame-edges nil 'outer-edges)", "nil"),
        (
            "(tty-frame-restack (selected-frame) (selected-frame))",
            "error: (error \"tty-frame-restack is not implemented\")",
        ),
        (
            "(list (tty-display-pixel-width) (tty-display-pixel-height))",
            "(80 25)",
        ),
        (
            "(tty-display-color-cells 'foo)",
            "error: (wrong-type-argument terminal-live-p foo)",
        ),
    ]);
}
