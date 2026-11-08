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
            "error: (error \"Window is dedicated to \u{2018}sw\u{2019}\")",
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

#[test]
fn window_body_sizes() {
    check(&[
        ("(window-body-width)", "80"),
        ("(= (window-body-width nil t) (window-body-width))", "t"),
        (
            "(= (window-body-width nil 'remap) (window-body-width))",
            "t",
        ),
        ("(- (window-total-height) (window-body-height))", "1"),
        ("(window-body-height (minibuffer-window))", "1"),
        ("(window-body-height (minibuffer-window) t)", "1"),
        (
            "(progn (setq header-line-format \"h\") (- (window-total-height) (window-body-height)))",
            "2",
        ),
        (
            "(window-body-width 'foo)",
            "error: (wrong-type-argument window-live-p foo)",
        ),
        (
            "(window-body-height 'foo t)",
            "error: (wrong-type-argument window-live-p foo)",
        ),
    ]);
}

#[test]
fn window_line_height_without_an_up_to_date_matrix() {
    check(&[
        ("(window-line-height)", "nil"),
        ("(window-line-height 'mode-line)", "nil"),
        (
            "(window-line-height nil 'x)",
            "error: (wrong-type-argument window-live-p x)",
        ),
    ]);
    common::on_big_stack(|| {
        let mut lisp = emacs_core::lisp::Lisp::new_with(false);
        for (source, expected) in [
            ("(window-line-height)", "nil"),
            ("(window-line-height 0)", "nil"),
            ("(window-line-height 'header-line (selected-window))", "nil"),
        ] {
            assert_eq!(
                common::eval(&mut lisp, source),
                expected,
                "evaluating {source}"
            );
        }
    });
}

#[test]
fn set_window_start_and_point() {
    check(&[
        (
            "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"sp\")) (set-buffer \"sp\") (insert \"hello world\") (list (set-window-start nil 3) (window-start) (set-window-point nil 5) (window-point) (point)))",
            "(3 3 5 5 5)",
        ),
        (
            "(list (set-window-start nil 100 t) (window-start))",
            "(100 12)",
        ),
        (
            "(set-window-start nil 'x)",
            "error: (wrong-type-argument integer-or-marker-p x)",
        ),
        (
            "(set-window-start 'x 1)",
            "error: (wrong-type-argument window-live-p x)",
        ),
        (
            "(progn (set-buffer \"*scratch*\") (list (set-window-point nil 4) (point) (save-current-buffer (set-buffer \"sp\") (point))))",
            "(4 1 4)",
        ),
        (
            "(set-window-point nil 'x)",
            "error: (wrong-type-argument integer-or-marker-p x)",
        ),
        ("(set-window-point nil (copy-marker 1))", "1"),
        (
            "(progn (save-current-buffer (set-buffer \" *Minibuf-0*\") (insert \"abcdef\")) (list (set-window-point (minibuffer-window) 3) (window-point (minibuffer-window)) (set-window-point (minibuffer-window) 99) (window-point (minibuffer-window))))",
            "(3 3 99 7)",
        ),
        (
            "(set-window-point (minibuffer-window) 'z)",
            "error: (wrong-type-argument integer-or-marker-p z)",
        ),
    ]);
}

#[test]
fn window_fringes_scroll_bars_and_margins_on_a_tty() {
    check(&[
        ("(set-window-fringes nil 3 4 t t)", "nil"),
        ("(set-window-fringes nil -1)", "nil"),
        ("(window-fringes)", "(0 0 nil nil)"),
        ("(set-window-scroll-bars nil 10 'bogus)", "nil"),
        ("(window-scroll-bars)", "(nil 0 t nil 0 t nil)"),
        (
            "(window-scroll-bars 1)",
            "error: (wrong-type-argument window-live-p 1)",
        ),
        ("(set-window-margins nil 2 3)", "t"),
        ("(window-margins)", "(2 . 3)"),
        ("(set-window-margins nil 2 3)", "nil"),
        ("(set-window-margins nil 1000)", "nil"),
        ("(window-margins)", "(2 . 3)"),
        (
            "(set-window-margins nil -1)",
            "error: (args-out-of-range -1 0 2147483647)",
        ),
        (
            "(set-window-margins 'x 1)",
            "error: (wrong-type-argument window-live-p x)",
        ),
        ("(set-window-margins nil nil)", "t"),
        ("(window-margins)", "(nil)"),
        (
            "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-buffer (get-buffer-create \"mg\")) (setq left-margin-width 4) (set-window-buffer nil \"mg\") (window-margins))",
            "(4)",
        ),
    ]);
}
