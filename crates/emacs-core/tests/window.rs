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
fn set_window_buffer() {
    check(&[
        (
            "(progn (fset 'record-window-buffer (function (lambda (w) (setq recorded w)))) nil)",
            "nil",
        ),
        ("(set-window-buffer nil (get-buffer-create \"sw\"))", "nil"),
        (
            "(list (buffer-name (window-buffer)) (eq recorded (selected-window)))",
            "(\"sw\" t)",
        ),
        ("(setq recorded nil)", "nil"),
        ("(set-window-buffer nil \"sw\")", "nil"),
        ("recorded", "nil"),
        ("(set-window-dedicated-p nil t)", "t"),
        (
            "(set-window-buffer nil \"*scratch*\")",
            "error: (error \"Window is dedicated to `sw'\")",
        ),
        ("(set-window-dedicated-p nil 'weak)", "weak"),
        ("(set-window-buffer nil \"*scratch*\")", "nil"),
        ("(window-dedicated-p)", "nil"),
        (
            "(set-window-buffer nil \"no such\")",
            "error: (wrong-type-argument bufferp nil)",
        ),
        (
            "(let ((b (get-buffer-create \"dead\"))) (kill-buffer b) (set-window-buffer nil b))",
            "error: (error \"Attempt to display deleted buffer\")",
        ),
        (
            "(progn (save-current-buffer (set-buffer \" *Minibuf-0*\") (insert \"abc\")) (set-window-buffer (minibuffer-window) \"sw\") (save-current-buffer (set-buffer \" *Minibuf-0*\") (point)))",
            "1",
        ),
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
