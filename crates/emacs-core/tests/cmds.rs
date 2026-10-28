mod common;

use common::check;

const AMALGAMATE: (&str, &str) = (
    "(progn (fset 'undo-auto-amalgamate (function (lambda ()))) nil)",
    "nil",
);

#[test]
fn self_insert_command() {
    check(&[
        AMALGAMATE,
        ("(self-insert-command 3 97)", "nil"),
        ("(self-insert-command 1 98)", "nil"),
        (
            "(list (buffer-string) (point) last-command-event)",
            "(\"aaab\" 5 98)",
        ),
        ("(self-insert-command 0 99)", "nil"),
        ("(buffer-string)", "\"aaab\""),
        (
            "(condition-case e (self-insert-command -1 97) (error (car (cdr e))))",
            "\"Negative repetition argument -1\"",
        ),
        ("(self-insert-command 1 'foo)", "nil"),
        ("(buffer-string)", "\"aaab\""),
        (
            "(progn (setq post-self-insert-hook (list (function (lambda () (setq hooked last-command-event))))) (self-insert-command 1 233) (setq post-self-insert-hook nil) hooked)",
            "233",
        ),
        ("(buffer-string)", "\"aaabé\""),
    ]);
}

#[test]
fn overwrite_mode() {
    check(&[
        AMALGAMATE,
        ("(insert \"xyz\")", "nil"),
        ("(goto-char 1)", "1"),
        (
            "(setq overwrite-mode 'overwrite-mode-textual)",
            "overwrite-mode-textual",
        ),
        ("(self-insert-command 1 65)", "nil"),
        ("(list (buffer-string) (point))", "(\"Ayz\" 2)"),
        (
            "(setq overwrite-mode 'overwrite-mode-binary)",
            "overwrite-mode-binary",
        ),
        ("(self-insert-command 2 66)", "nil"),
        ("(list (buffer-string) (point))", "(\"ABB\" 4)"),
        ("(self-insert-command 1 67)", "nil"),
        ("(list (buffer-string) (point))", "(\"ABBC\" 5)"),
        ("(setq overwrite-mode nil)", "nil"),
    ]);
}

#[test]
fn delete_char() {
    check(&[
        AMALGAMATE,
        ("(insert \"abcd\")", "nil"),
        ("(delete-char -2)", "nil"),
        ("(buffer-string)", "\"ab\""),
        ("(delete-char -3)", "error: (beginning-of-buffer)"),
        (
            "(progn (fset 'kill-forward-chars (function (lambda (n) (setq killed n)))) (delete-char 1 t) killed)",
            "1",
        ),
    ]);
}

#[test]
fn show_cursor() {
    check(&[
        ("(internal-show-cursor-p)", "t"),
        ("(internal-show-cursor nil nil)", "nil"),
        ("(internal-show-cursor-p (selected-window))", "nil"),
        ("(internal-show-cursor (selected-window) 'yes)", "nil"),
        ("(internal-show-cursor-p)", "t"),
        ("(ding t)", "nil"),
        (
            "(internal-show-cursor 'foo t)",
            "error: (wrong-type-argument windowp foo)",
        ),
    ]);
}

#[test]
fn unibyte_neighbors() {
    check(&[
        (
            "(set-buffer (get-buffer-create \"unibyte\"))",
            "#<buffer unibyte>",
        ),
        ("(set-buffer-multibyte nil)", "nil"),
        ("(insert 200)", "nil"),
        ("(preceding-char)", "200"),
        ("(goto-char 1)", "1"),
        ("(following-char)", "200"),
    ]);
}
