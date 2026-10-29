mod common;

use common::{eval, on_big_stack};
use emacs_core::lisp::Lisp;

#[test]
fn write_region_handlers() {
    let dir = std::env::temp_dir().join(format!("emacs-write-region-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create directory");
    let plain = dir.join("plain.txt");
    let plain_lisp = format!("{:?}", plain.to_string_lossy().replace('\\', "/"));
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        for (source, expected) in [
            (
                "(defalias 'wr-handler (function (lambda (op &rest args) (if (eq op 'write-region) (progn (setq wr-seen (cons op args)) 'handled) (let ((inhibit-file-name-handlers (cons 'wr-handler inhibit-file-name-handlers)) (inhibit-file-name-operation op)) (apply op args))))))".to_string(),
                "wr-handler",
            ),
            (
                "(setq file-name-handler-alist (list (cons \"\\\\.gpg\\\\'\" 'wr-handler)))".to_string(),
                "((\"\\\\.gpg\\\\'\" . wr-handler))",
            ),
            (
                format!("(save-current-buffer (set-buffer (get-buffer-create \"wr\")) (insert \"abc\") (write-region 1 4 {plain_lisp} nil 1))"),
                "nil",
            ),
            (
                "(save-current-buffer (set-buffer \"wr\") (write-region 1 4 \"/x/secret.gpg\" nil 1))".to_string(),
                "handled",
            ),
            (
                "(list (car wr-seen) (nth 1 wr-seen) (nth 2 wr-seen) (nth 5 wr-seen) (equal (nth 3 wr-seen) (expand-file-name \"/x/secret.gpg\")) (equal (nth 6 wr-seen) (nth 3 wr-seen)))".to_string(),
                "(write-region 1 4 1 t t)",
            ),
        ] {
            assert_eq!(eval(l, &source), expected, "evaluating {source}");
        }
    });
    assert_eq!(std::fs::read_to_string(&plain).expect("written"), "abc");
}
