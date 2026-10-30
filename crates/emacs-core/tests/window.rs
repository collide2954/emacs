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
fn window_configurations() {
    check(&[
        (
            "(progn (fset 'record-window-buffer (function (lambda (w) (setq recorded w)))) (fset 'frame-windows-min-size (function (lambda (frame horizontal ignore pixelwise) 1))) nil)",
            "nil",
        ),
        (
            "(window-configuration-p (current-window-configuration))",
            "t",
        ),
        ("(window-configuration-p 1)", "nil"),
        ("(window-fringes)", "(0 0 nil nil)"),
        (
            "(window-fringes 1)",
            "error: (wrong-type-argument window-live-p 1)",
        ),
        (
            "(type-of (current-window-configuration))",
            "window-configuration",
        ),
        ("(current-window-configuration)", "#<window-configuration>"),
        (
            "(eq (window-configuration-frame (current-window-configuration)) (selected-frame))",
            "t",
        ),
        (
            "(window-configuration-frame 1)",
            "error: (wrong-type-argument window-configuration-p 1)",
        ),
        (
            "(set-window-configuration 1)",
            "error: (wrong-type-argument window-configuration-p 1)",
        ),
        (
            "(progn (set-window-buffer nil (get-buffer-create \"a\")) (set-buffer \"a\") (insert \"hello world\") (goto-char 3) (setq c1 (current-window-configuration) c2 (current-window-configuration)) (window-configuration-equal-p c1 c2))",
            "t",
        ),
        (
            "(window-configuration-equal-p 1 c1)",
            "error: (wrong-type-argument window-configuration-p 1)",
        ),
        (
            "(progn (set-window-buffer nil (get-buffer-create \"b\")) (window-configuration-equal-p c1 (current-window-configuration)))",
            "nil",
        ),
        (
            "(progn (setq recorded nil) (list (set-window-configuration c1) (buffer-name (window-buffer)) (buffer-name) (eq recorded (selected-window))))",
            "(t \"a\" \"a\" t)",
        ),
        ("(point)", "3"),
        (
            "(progn (goto-char 7) (set-window-configuration c1) (point))",
            "7",
        ),
        (
            "(progn (set-buffer \"b\") (set-window-configuration c1) (list (buffer-name) (point)))",
            "(\"a\" 7)",
        ),
        ("(hash-table-count window-dead-windows-table)", "0"),
        (
            "(progn (set-window-buffer nil (get-buffer-create \"k\")) (setq ck (current-window-configuration)) (set-window-buffer nil \"a\") (kill-buffer \"k\") (list (set-window-configuration ck) (buffer-name (window-buffer))))",
            "(t \"a\")",
        ),
        (
            "(progn (set-window-parameter nil 'clone-of 'x) (set-window-parameter nil 'foo 1) (setq cp (current-window-configuration)) (set-window-parameter nil 'clone-of 'y) (set-window-parameter nil 'foo 2) (set-window-configuration cp) (list (window-parameter nil 'clone-of) (window-parameter nil 'foo)))",
            "(x 2)",
        ),
        (
            "(progn (set-window-buffer nil \"a\") (list (byte-code \"\\300\\213\\207\" [((set-window-buffer nil \"b\") 42)] 1) (buffer-name (window-buffer))))",
            "(42 \"a\")",
        ),
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

#[test]
fn window_line_and_divider_sizes() {
    check(&[
        ("(window-scroll-bar-width)", "0"),
        ("(window-scroll-bar-height)", "0"),
        ("(window-right-divider-width)", "0"),
        ("(window-bottom-divider-width)", "0"),
        ("(window-bottom-divider-width (minibuffer-window))", "0"),
        ("(window-mode-line-height)", "1"),
        ("(window-mode-line-height (minibuffer-window))", "0"),
        ("(window-header-line-height)", "0"),
        ("(window-tab-line-height)", "0"),
        (
            "(progn (setq header-line-format \"h\") (window-header-line-height))",
            "1",
        ),
        (
            "(progn (setq tab-line-format \"t\") (window-tab-line-height))",
            "1",
        ),
        (
            "(progn (set-window-parameter nil 'mode-line-format 'none) (window-mode-line-height))",
            "0",
        ),
        (
            "(progn (set-window-parameter nil 'header-line-format 'none) (window-header-line-height))",
            "0",
        ),
        (
            "(window-mode-line-height 'foo)",
            "error: (wrong-type-argument window-live-p foo)",
        ),
        ("(window-scroll-bar-width (frame-root-window))", "0"),
    ]);
}
