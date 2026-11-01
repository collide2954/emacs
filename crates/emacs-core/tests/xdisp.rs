mod common;

use common::check;

#[test]
fn bar_heights_without_bar_windows() {
    check(&[
        ("(tab-bar-height)", "0"),
        ("(tab-bar-height nil t)", "0"),
        ("(tool-bar-height)", "0"),
        ("(tool-bar-height (selected-frame) t)", "0"),
        (
            "(tab-bar-height 'x)",
            "error: (wrong-type-argument framep x)",
        ),
        (
            "(tool-bar-height 'x)",
            "error: (wrong-type-argument framep x)",
        ),
    ]);
}

#[test]
fn set_buffer_redisplay_as_watcher() {
    check(&[
        ("(set-buffer-redisplay 'x 1 'set nil)", "nil"),
        (
            "(progn (defvar xd-watched 1) (add-variable-watcher 'xd-watched (symbol-function 'set-buffer-redisplay)) (setq xd-watched 2))",
            "2",
        ),
        (
            "(set-buffer-redisplay 'x 1 'set)",
            "error: (wrong-number-of-arguments set-buffer-redisplay 3)",
        ),
    ]);
}
