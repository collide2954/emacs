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
