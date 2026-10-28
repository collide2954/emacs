mod common;

use common::check;

#[test]
fn messages_buffer() {
    check(&[
        ("(message \"first %d\" 1)", "\"first 1\""),
        ("(message \"second\")", "\"second\""),
        (
            "(save-current-buffer (set-buffer \"*Messages*\") (buffer-string))",
            "\"first 1\nsecond\n\"",
        ),
        ("(current-message)", "nil"),
        ("(message nil)", "nil"),
        ("(message \"\" 'ignored)", "\"\""),
        (
            "(let ((message-log-max nil)) (message \"unlogged\") (save-current-buffer (set-buffer \"*Messages*\") (buffer-string)))",
            "\"first 1\nsecond\n\"",
        ),
    ]);
}
