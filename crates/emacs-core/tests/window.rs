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
fn window_resize_primitives() {
    check(&[
        (
            "(progn (setq r (frame-root-window) m (minibuffer-window) rh (window-pixel-height r) rw (window-pixel-width r) rt (window-total-height r) tm (window-top-line r) mh (window-pixel-height m)) mh)",
            "1",
        ),
        ("(set-window-new-pixel r 10)", "10"),
        ("(set-window-new-pixel r 3 t)", "13"),
        ("(window-new-pixel r)", "13"),
        (
            "(set-window-new-pixel r -14 t)",
            "error: (args-out-of-range -14 -13 2147483634)",
        ),
        ("(set-window-new-total nil 5)", "5"),
        ("(set-window-new-total nil 2 t)", "7"),
        (
            "(set-window-new-total nil 'x)",
            "error: (wrong-type-argument fixnump x)",
        ),
        ("(set-window-new-normal nil 'ignore)", "ignore"),
        ("(window-new-normal)", "ignore"),
        (
            "(set-window-new-normal 'x)",
            "error: (wrong-type-argument window-valid-p x)",
        ),
        ("(window-resize-apply)", "nil"),
        (
            "(progn (set-window-new-pixel r 0) (window-resize-apply))",
            "nil",
        ),
        (
            "(progn (set-window-new-pixel r rh) (set-window-new-normal r 0.5) (list (window-resize-apply) (= (window-pixel-height r) rh) (window-normal-size r)))",
            "(t t 0.5)",
        ),
        (
            "(progn (set-window-new-pixel r rw) (set-window-new-normal r 'ignore) (list (window-resize-apply nil t) (= (window-pixel-width r) rw) (window-normal-size r t)))",
            "(t t 1.0)",
        ),
        (
            "(resize-mini-window-internal r)",
            "error: (error \"Not a valid minibuffer window\")",
        ),
        (
            "(progn (set-window-new-pixel r (- rh 2)) (set-window-new-pixel m 2) (resize-mini-window-internal m))",
            "error: (error \"Cannot resize mini window\")",
        ),
        (
            "(progn (set-window-new-pixel m (+ mh 2)) (list (resize-mini-window-internal m) (- rh (window-pixel-height r)) (- rt (window-total-height r)) (window-pixel-height m) (window-total-height m) (= (window-pixel-top m) (+ (window-pixel-top r) (window-pixel-height r))) (= (window-top-line m) (+ tm (window-total-height r)))))",
            "(t 2 2 3 3 t t)",
        ),
        (
            "(progn (set-window-new-total r rt) (set-window-new-total m 1) (list (window-resize-apply-total) (= (window-total-height r) rt) tm (= (window-top-line r) (frame-parameter nil 'menu-bar-lines)) (window-total-height m) (= (window-top-line m) (+ (window-top-line r) rt))))",
            "(t t 0 t 1 t)",
        ),
        (
            "(progn (set-window-new-total r 7) (set-window-new-total m 9) (list (window-resize-apply-total nil t) (window-total-width r) (window-total-width m)))",
            "(t 7 9)",
        ),
    ]);
}

#[test]
fn run_window_scroll_functions() {
    check(&[
        ("(run-window-scroll-functions)", "nil"),
        (
            "(run-window-scroll-functions 'x)",
            "error: (wrong-type-argument window-live-p x)",
        ),
        (
            "(save-current-buffer (set-buffer (get-buffer-create \"other\")) (let ((window-scroll-functions (list (function (lambda (w s) (setq got (list w s (buffer-name)))))))) (list (run-window-scroll-functions) got (buffer-name))))",
            "(nil (nil 1 \"*scratch*\") \"other\")",
        ),
        (
            "(let ((window-scroll-functions (list (function (lambda (w s) (setq got (eq w (selected-window)))))))) (run-window-scroll-functions (selected-window)) got)",
            "t",
        ),
    ]);
}

#[test]
fn split_and_delete_windows() {
    check(&[
        (
            "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (setq w (selected-window) h (window-pixel-height w) half (/ h 2)) (> half 2))",
            "t",
        ),
        (
            "(split-window-internal (minibuffer-window) 1 nil 0.5)",
            "error: (error \"Attempt to split minibuffer window\")",
        ),
        (
            "(split-window-internal w 'x nil 0.5)",
            "error: (wrong-type-argument fixnump x)",
        ),
        (
            "(split-window-internal w 0 nil 0.5)",
            "error: (error \"Size of new window too small (after split)\")",
        ),
        (
            "(split-window-internal w 1 'right 0.5)",
            "error: (error \"Size of new window too small (after split)\")",
        ),
        (
            "(split-window-internal w half nil 0.5 'x)",
            "error: (wrong-type-argument windowp x)",
        ),
        (
            "(progn (set-window-new-pixel w 0) (split-window-internal w half nil 0.5))",
            "error: (error \"Resizing old window failed\")",
        ),
        (
            "(progn (set-window-new-pixel w 1) (split-window-internal w half nil 0.5))",
            "error: (error \"Sum of sizes of old and new window don\u{2019}t fit\")",
        ),
        (
            "(progn (set-window-new-pixel w (- h half)) (set-window-new-normal w 0.5) (setq n (split-window-internal w half nil 0.5) p (window-parent w)) (list (window-live-p n) (eq (window-buffer n) (window-buffer w)) (eq (window-next-sibling w) n) (eq (window-prev-sibling n) w) (eq (window-parent n) p) (eq (frame-root-window) p) (window-live-p p) (window-valid-p p) (eq (window-top-child p) w) (= (window-pixel-height w) (- h half)) (= (window-pixel-height n) half) (= (window-pixel-top n) (+ (window-pixel-top w) (- h half))) (= (window-pixel-height p) h) (window-normal-size w) (window-normal-size n) (= (window-total-height n) half) (eq (selected-window) w) (= (window-pixel-width n) (window-pixel-width w))))",
            "(t t t t t t nil t t t t t t 0.5 0.5 t t t)",
        ),
        (
            "(split-window-internal w 1 nil 0.5 p)",
            "error: (error \"REFER is not a live window (does not show a buffer)\")",
        ),
        (
            "(progn (setq wh (window-pixel-height w) a (/ wh 2)) (set-window-new-pixel w (- wh a)) (set-window-new-normal w 0.25) (setq u (split-window-internal w a 'above 0.25)) (list (eq (window-parent u) p) (eq (window-top-child p) u) (eq (window-next-sibling u) w) (eq (window-prev-sibling w) u) (= (window-pixel-top u) (window-pixel-top p)) (= (window-pixel-top w) (+ (window-pixel-top u) a)) (= (window-pixel-top n) (+ (window-pixel-top w) (- wh a))) (window-normal-size u)))",
            "(t t t t t t t 0.25)",
        ),
        (
            "(progn (setq nw (window-pixel-width n) nh2 (/ nw 2)) (set-window-new-pixel n (- nw nh2)) (set-window-new-normal n 0.5) (setq m (split-window-internal n nh2 'left 0.5) q (window-parent m)) (list (eq (window-parent q) p) (eq (window-left-child q) m) (eq (window-next-sibling m) n) (eq (window-next-sibling w) q) (window-pixel-left m) (= (window-pixel-left n) nh2) (= (window-pixel-width q) nw) (= (window-pixel-height m) (window-pixel-height n)) (= (window-pixel-top m) (window-pixel-top n)) (window-normal-size q) (window-normal-size m t) (window-normal-size n t) (window-combination-limit q) (= (window-total-width m) nh2)))",
            "(t t t t 0 t t t t 0.5 0.5 0.5 nil t)",
        ),
        (
            "(delete-window-internal (frame-root-window))",
            "error: (error \"Attempt to delete minibuffer or sole ordinary window\")",
        ),
        (
            "(delete-window-internal (minibuffer-window))",
            "error: (error \"Attempt to delete minibuffer or sole ordinary window\")",
        ),
        (
            "(progn (setq fw (window-pixel-width p)) (set-window-new-pixel p fw) (set-window-new-pixel u fw) (set-window-new-pixel w fw) (set-window-new-pixel q fw) (set-window-new-pixel n fw) (list (delete-window-internal m) (window-live-p m) (window-valid-p m) (window-valid-p q) (eq (window-next-sibling w) n) (eq (window-parent n) p) (= (window-pixel-width n) fw) (window-pixel-left n) (window-normal-size n) (window-normal-size n t) (hash-table-count window-dead-windows-table) (eq (selected-window) w)))",
            "(nil nil nil nil t t t 0 0.5 1.0 1 t)",
        ),
        ("(delete-window-internal m)", "nil"),
        (
            "(progn (set-window-new-pixel p h) (set-window-new-pixel u 1) (list (condition-case err (delete-window-internal u) (error (car (cdr err)))) (eq (window-top-child p) u) (window-live-p u) (eq (window-prev-sibling w) u)))",
            "(\"Deletion failed\" t t t)",
        ),
        (
            "(progn (set-window-new-pixel p h) (set-window-new-pixel u (+ (window-pixel-height u) (window-pixel-height w))) (set-window-new-pixel n (window-pixel-height n)) (setq uh (window-pixel-height u) wh2 (window-pixel-height w)) (list (delete-window-internal w) (eq (selected-window) u) (eq (frame-selected-window) u) (eq (window-next-sibling u) n) (eq (window-prev-sibling n) u) (= (window-pixel-height u) (+ uh wh2)) (window-live-p w) (eq (current-buffer) (window-buffer u))))",
            "(nil t t t t t nil t)",
        ),
        (
            "(progn (fset 'window--pixel-to-total (function (lambda (&optional frame horizontal) nil))) (set-buffer (get-buffer-create \"lines\")) (let ((i 0)) (while (< i 60) (insert \"line\\n\") (setq i (1+ i)))) (set-window-buffer n \"lines\") (goto-char (point-min)) (forward-line 30) (setq start30 (point)) (set-window-start n start30) (setq top (- (window-top-line n) (frame-parameter nil 'menu-bar-lines))) (> top 0))",
            "t",
        ),
        (
            "(progn (setq ph (window-pixel-height p) pt (window-pixel-top p)) (list (delete-other-windows-internal n) (eq (frame-root-window) n) (window-live-p u) (window-valid-p p) (eq (selected-window) n) (= (window-pixel-height n) ph) (= (window-pixel-top n) pt) (= (window-start n) (save-current-buffer (set-buffer \"lines\") (save-excursion (goto-char start30) (forward-line (- top)) (point)))) (eq (window-next-sibling n) (minibuffer-window)) (eq (window-prev-sibling (minibuffer-window)) n) (window-parent n)))",
            "(nil t nil nil t t t t t t nil)",
        ),
        ("(delete-other-windows-internal n)", "nil"),
        (
            "(delete-other-windows-internal (minibuffer-window))",
            "error: (error \"Can\u{2019}t expand minibuffer to full frame\")",
        ),
        (
            "(delete-other-windows-internal n (minibuffer-window))",
            "error: (error \"Specified root is not an ancestor of specified window\")",
        ),
        (
            "(progn (set-window-new-pixel n (- ph 3)) (set-window-new-normal n 0.5) (setq m2 (split-window-internal n 3 nil 0.5 m)) (list (eq m2 m) (window-live-p m) (eq (window-buffer m) (get-buffer \"*scratch*\")) (hash-table-count window-dead-windows-table) (eq (window-next-sibling n) m) (= (window-pixel-height m) 3)))",
            "(t t t 2 t t)",
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
