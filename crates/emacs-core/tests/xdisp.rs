mod common;

use common::check;

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
