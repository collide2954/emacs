mod common;

use common::check;

#[test]
fn keyboard_variables() {
    check(&[
        ("(keymapp function-key-map)", "t"),
        ("(keymapp key-translation-map)", "t"),
        ("(keymapp input-decode-map)", "t"),
        (
            "(eq (keymap-parent local-function-key-map) function-key-map)",
            "t",
        ),
        ("help-char", "8"),
        ("meta-prefix-char", "27"),
        ("echo-keystrokes", "1"),
        ("polling-period", "2.0"),
        ("auto-save-interval", "300"),
        ("input-method-function", "list"),
        ("command-error-function", "command-error-default-function"),
        (
            "selection-inhibit-update-commands",
            "(handle-switch-frame handle-select-window)",
        ),
        ("(car while-no-input-ignore-events)", "sleep-event"),
        (
            "(memq 'toolkit-theme-changed while-no-input-ignore-events)",
            "(toolkit-theme-changed)",
        ),
        ("(special-variable-p 'top-level)", "nil"),
        ("(special-variable-p 'this-command)", "t"),
        ("(local-variable-if-set-p 'deactivate-mark)", "t"),
        ("input-method-use-echo-area", "nil"),
    ]);
}

#[test]
fn event_heads() {
    check(&[
        ("(get 'focus-in 'event-kind)", "focus-in"),
        ("(get 'select-window 'event-kind)", "switch-frame"),
        ("(get 'scroll-bar-movement 'event-kind)", "mouse-movement"),
        ("(get 'touchscreen-begin 'event-kind)", "touchscreen"),
        (
            "(get 'delete-frame 'event-symbol-elements)",
            "(delete-frame)",
        ),
    ]);
}

#[test]
fn timers_run_while_waiting() {
    check(&[
        ("(defvar fired nil)", "fired"),
        (
            "(defalias 'timer-event-handler (function (lambda (timer) (setq fired (cons (list (aref timer 5) inhibit-quit) fired)) (setq deactivate-mark t))))",
            "timer-event-handler",
        ),
        (
            "(defvar ripe (vector nil 0 0 0 nil 'ripe nil nil 0 nil))",
            "ripe",
        ),
        (
            "(progn (setq timer-list (list 'junk (vector t 0 0 0 nil 'done nil nil 0 nil) (vector nil 0 'x 0 nil 'bad nil nil 0 nil) (vector nil 0 0 0 nil 'short) ripe (vector nil 100000 0 0 nil 'future nil nil 0 nil))) (setq deactivate-mark nil) (sleep-for 0.01) fired)",
            "((ripe t))",
        ),
        ("(aref ripe 0)", "t"),
        ("deactivate-mark", "nil"),
        (
            "(let* ((when (time-convert (time-add nil 0.05) 'list)) (soon (vector nil (nth 0 when) (nth 1 when) (nth 2 when) nil 'soon nil nil (nth 3 when) nil)) (start (float-time))) (setq fired nil timer-list (list soon)) (sleep-for 0.3) (list fired (< (- (float-time) start) 2)))",
            "(((soon t)) t)",
        ),
        (
            "(progn (setq fired nil timer-list nil timer-idle-list (list (vector nil 0 0 0 nil 'idle nil nil 0 nil))) (sleep-for 0.01) fired)",
            "nil",
        ),
        (
            "(progn (setq timer-list (list (vector nil 0 0 0 nil 'boom nil nil 0 nil))) (defalias 'timer-event-handler (function (lambda (timer) (signal 'error '(\"boom\"))))) (condition-case e (sleep-for 0.01) (error e)))",
            "(error \"boom\")",
        ),
    ]);
}

#[test]
fn input_modes() {
    let interrupt = if cfg!(unix) {
        "(t nil t 7)"
    } else {
        "(nil nil t 7)"
    };
    check(&[
        ("(current-input-mode)", "(nil nil t 7)"),
        ("(set-input-meta-mode 8)", "nil"),
        ("(set-input-meta-mode 'encoded (selected-frame))", "nil"),
        ("(set-output-flow-control t)", "nil"),
        ("(set-quit-char 300)", "nil"),
        ("(set-input-mode nil t nil ?q)", "nil"),
        ("(current-input-mode)", "(nil nil t 7)"),
        ("(set-input-interrupt-mode t)", "nil"),
        ("(current-input-mode)", interrupt),
        ("(set-input-interrupt-mode nil)", "nil"),
        ("(current-input-mode)", "(nil nil t 7)"),
    ]);
}

#[test]
fn terminal_local_variables() {
    check(&[
        ("last-command", "nil"),
        ("(setq last-command 'foo)", "foo"),
        ("last-command", "foo"),
        ("(default-value 'last-command)", "foo"),
        ("(let ((last-command 'bar)) last-command)", "bar"),
        ("last-command", "foo"),
        ("(set-default 'real-last-command 'baz)", "baz"),
        ("real-last-command", "baz"),
        ("(boundp 'overriding-terminal-local-map)", "t"),
        ("overriding-terminal-local-map", "nil"),
    ]);
}
