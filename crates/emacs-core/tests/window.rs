mod common;

use common::check;

#[test]
fn window_variables() {
    check(&[
        ("window-persistent-parameters", "((clone-of . t))"),
        ("window-combination-limit", "window-size"),
        ("recenter-redisplay", "tty"),
        ("next-screen-context-lines", "2"),
        ("mode-line-in-non-selected-windows", "t"),
        ("(hash-table-p window-dead-windows-table)", "t"),
        ("(hash-table-weakness window-dead-windows-table)", "value"),
        ("(get 'scroll-up 'scroll-command)", "t"),
        ("(get 'scroll-down 'scroll-command)", "t"),
    ]);
}

#[test]
fn window_cycling() {
    check(&[
        ("(window-list)", "(#<window 1 on *scratch*>)"),
        (
            "(window-list nil t)",
            "(#<window 1 on *scratch*> #<window 2 on  *Minibuf-0*>)",
        ),
        ("(window-list nil 'never)", "(#<window 1 on *scratch*>)"),
        (
            "(window-list-1 (minibuffer-window) t)",
            "(#<window 2 on  *Minibuf-0*> #<window 1 on *scratch*>)",
        ),
        ("(eq (next-window) (selected-window))", "t"),
        ("(eq (next-window nil t) (minibuffer-window))", "t"),
        (
            "(eq (next-window (minibuffer-window) t) (selected-window))",
            "t",
        ),
        ("(eq (previous-window nil t) (minibuffer-window))", "t"),
        (
            "(eq (get-buffer-window \"*scratch*\") (selected-window))",
            "t",
        ),
        ("(get-buffer-window \" *Minibuf-0*\")", "nil"),
        ("(get-buffer-window \"no such buffer\")", "nil"),
        (
            "(window-list nil nil (minibuffer-window))",
            "(#<window 1 on *scratch*>)",
        ),
        (
            "(window-list (selected-frame) nil 'x)",
            "error: (wrong-type-argument windowp x)",
        ),
    ]);
}
