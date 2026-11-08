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
fn get_display_property() {
    check(&[
        (
            "(progn (set-buffer (get-buffer-create \"gdp\")) (insert \"abc\") (put-text-property 1 2 'display '((height 2.0) (raise 0.5))) (list (get-display-property 1 'height) (get-display-property 1 'raise) (get-display-property 1 'foo) (get-display-property 2 'height)))",
            "(2.0 0.5 nil nil)",
        ),
        (
            "(progn (put-text-property 2 3 'display [(height 3) (raise 1)]) (get-display-property 2 'raise))",
            "1",
        ),
        (
            "(progn (put-text-property 3 4 'display '(height 4)) (list (get-display-property 3 'height) (get-display-property 3 'raise)))",
            "(4 nil)",
        ),
        ("(get-display-property 1 'raise nil '((raise 7)))", "7"),
        (
            "(get-display-property 1 'raise nil 'x)",
            "error: (wrong-type-argument listp x)",
        ),
        (
            "(let ((s (copy-sequence \"a\"))) (put-text-property 0 1 'display '(height 5) s) (get-display-property 0 'height s))",
            "5",
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
