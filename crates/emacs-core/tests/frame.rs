mod common;

use common::check;

#[test]
fn initial_terminal_and_frame() {
    check(&[
        ("(framep (selected-frame))", "t"),
        ("(frame-live-p (selected-frame))", "t"),
        ("(window-system)", "nil"),
        ("(length (frame-list))", "1"),
        ("(frame-parameter nil 'name)", "\"F1\""),
        ("(frame-parameter nil 'minibuffer)", "t"),
        ("(frame-parameter nil 'menu-bar-lines)", "1"),
        (
            "(frame-parameter nil 'background-color)",
            "\"unspecified-bg\"",
        ),
        (
            "(frame-parameter nil 'foreground-color)",
            "\"unspecified-fg\"",
        ),
        ("(frame-text-cols)", "80"),
        ("(frame-text-lines)", "25"),
        ("(terminal-name)", "\"initial_terminal\""),
        ("(terminal-live-p nil)", "t"),
        ("(frame-terminal)", "#<terminal 0 on initial_terminal>"),
        ("(frame-initial-p)", "t"),
        ("(eq terminal-frame (selected-frame))", "t"),
        ("(eq (next-frame) (selected-frame))", "t"),
        ("(frame-visible-p (selected-frame))", "t"),
    ]);
}

#[test]
fn frame_visibility() {
    check(&[
        ("(eq (make-frame-visible) (selected-frame))", "t"),
        (
            "(progn (setq buffer-display-time nil) (make-frame-visible) (null buffer-display-time))",
            "nil",
        ),
        (
            "(make-frame-invisible)",
            "error: (error \"Attempt to make invisible the sole visible or iconified frame\")",
        ),
        ("(make-frame-invisible nil t)", "nil"),
        ("(iconify-frame)", "nil"),
        ("(raise-frame)", "nil"),
        ("(lower-frame)", "nil"),
        (
            "(make-frame-visible 'x)",
            "error: (wrong-type-argument frame-live-p x)",
        ),
    ]);
}

#[test]
fn frame_parameters_round_trip() {
    check(&[
        ("(modify-frame-parameters nil '((foo . 1)))", "nil"),
        ("(frame-parameter nil 'foo)", "1"),
        ("(cdr (assq 'foo (frame-parameters)))", "1"),
        ("(modify-frame-parameters nil '((name . \"Hi\")))", "nil"),
        ("(frame-parameter nil 'name)", "\"Hi\""),
        ("(modify-frame-parameters nil '((name)))", "nil"),
        ("(frame-parameter nil 'name)", "\"F2\""),
        (
            "(modify-frame-parameters nil '((minibuffer . nil)))",
            "error: (error \"Can't change the `minibuffer' parameter of this frame\")",
        ),
        ("(frame-parameter nil 'minibuffer)", "t"),
    ]);
}

#[test]
fn initial_windows() {
    check(&[
        ("(selected-window)", "#<window 1 on *scratch*>"),
        ("(eq (frame-root-window) (selected-window))", "t"),
        ("(window-live-p (frame-root-window))", "t"),
        ("(window-minibuffer-p (minibuffer-window))", "t"),
        (
            "(window-buffer (minibuffer-window))",
            "#<buffer  *Minibuf-0*>",
        ),
        ("(minibufferp (window-buffer (minibuffer-window)))", "t"),
        ("(window-total-height)", "24"),
        ("(window-total-width)", "80"),
        ("(window-total-height (minibuffer-window))", "1"),
        (
            "(window-next-sibling (frame-root-window))",
            "#<window 2 on  *Minibuf-0*>",
        ),
        ("(window-point)", "1"),
        ("(window-start)", "1"),
        (
            "(select-window (selected-window))",
            "#<window 1 on *scratch*>",
        ),
        ("(window-parameter nil 'x)", "nil"),
        ("(set-window-parameter nil 'x 3)", "3"),
        ("(window-parameters)", "((x . 3))"),
        ("(minibuffer-depth)", "0"),
        ("(window-dedicated-p)", "nil"),
        ("(frame-first-window)", "#<window 1 on *scratch*>"),
    ]);
}
