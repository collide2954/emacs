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
