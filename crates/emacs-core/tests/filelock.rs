mod common;

use std::path::{Path, PathBuf};

use common::{eval, on_big_stack};
use emacs_core::lisp::Lisp;

const RAW_TEXT: &str = "(define-coding-system-internal 'raw-text ?t 'raw-text nil t nil nil nil nil nil t '(:name raw-text) nil)";
const SETUP: &str = "(progn (defalias 'make-lock-file-name (function (lambda (f) (concat (file-name-directory f) \".#\" (file-name-nondirectory f))))) (setq user-login-name \"tester\" system-name \"testhost\") nil)";

fn scratch(name: &str) -> (PathBuf, PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("emacs-filelock-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create directory");
    let file = dir.join("locked.txt");
    let lock = dir.join(".#locked.txt");
    let lisp = format!("{:?}", file.to_string_lossy().replace('\\', "/"));
    (file, lock, lisp)
}

fn lock_content(lock: &Path) -> Vec<u8> {
    match std::fs::read_link(lock) {
        Ok(target) => target.to_string_lossy().into_owned().into_bytes(),
        Err(_) => std::fs::read(lock).expect("read lock"),
    }
}

fn run(l: &mut Lisp, cases: &[(String, &str)]) {
    for (source, expected) in cases {
        assert_eq!(eval(l, source), *expected, "evaluating {source}");
    }
}

fn s(x: &str) -> String {
    x.to_string()
}

#[test]
fn lock_and_unlock() {
    let (file, lock, f) = scratch("basic");
    let pid = std::process::id();
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        run(
            l,
            &[
                (s(SETUP), "nil"),
                (format!("(file-locked-p {f})"), "nil"),
                (format!("(lock-file {f})"), "nil"),
                (format!("(file-locked-p {f})"), "t"),
            ],
        );
        let content = lock_content(&lock);
        let prefix = format!("tester@testhost.{pid}");
        assert!(content.starts_with(prefix.as_bytes()), "{content:?}");
        let rest = &content[prefix.len()..];
        assert!(
            rest.is_empty() || (rest[0] == b':' && rest[1..].iter().all(u8::is_ascii_digit)),
            "{content:?}"
        );
        run(
            l,
            &[
                (format!("(lock-file {f})"), "nil"),
                (format!("(file-locked-p {f})"), "t"),
                (format!("(unlock-file {f})"), "nil"),
                (format!("(file-locked-p {f})"), "nil"),
                (format!("(unlock-file {f})"), "nil"),
                (s("(lock-file 1)"), "error: (wrong-type-argument stringp 1)"),
                (
                    s("(unlock-file 1)"),
                    "error: (wrong-type-argument stringp 1)",
                ),
                (
                    s("(file-locked-p 1)"),
                    "error: (wrong-type-argument stringp 1)",
                ),
            ],
        );
        assert!(!lock.exists());
        run(
            l,
            &[(
                format!("(let ((create-lockfiles nil)) (lock-file {f}))"),
                "nil",
            )],
        );
        assert!(!lock.exists());
        assert!(!file.exists());
    });
}

#[test]
fn lock_owners() {
    let (_file, lock, f) = scratch("owners");
    let pid = std::process::id();
    let live = if cfg!(windows) { 4 } else { 1 };
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        run(
            l,
            &[
                (s(SETUP), "nil"),
                (
                    s(
                        "(defalias 'ask-user-about-lock (function (lambda (file who) (setq asked (list file who)) answer)))",
                    ),
                    "ask-user-about-lock",
                ),
                (s("(setq answer nil)"), "nil"),
            ],
        );
        std::fs::write(&lock, "other@elsewhere.123:456").expect("lock");
        run(
            l,
            &[
                (format!("(file-locked-p {f})"), "\"other\""),
                (format!("(lock-file {f})"), "nil"),
                (
                    format!("(list (equal (car asked) {f}) (nth 1 asked))"),
                    "(t \"other@elsewhere (pid 123)\")",
                ),
            ],
        );
        assert_eq!(lock_content(&lock), b"other@elsewhere.123:456");
        run(
            l,
            &[
                (s("(setq answer t)"), "t"),
                (format!("(lock-file {f})"), "nil"),
                (format!("(file-locked-p {f})"), "t"),
            ],
        );
        assert!(lock_content(&lock).starts_with(format!("tester@testhost.{pid}").as_bytes()));
        for (content, expected, kept) in [
            (format!("x@testhost.{live}"), "\"x\"", true),
            (format!("x@testhost.{live}:1"), "nil", false),
            (s("x@testhost.2147483646"), "nil", false),
            (format!("x@testhost.{pid}"), "t", true),
            (format!("x@testhost.{pid}:1"), "t", true),
            (s(""), "nil", false),
            (s("a@b@testhost.99999999999999999999"), "nil", false),
        ] {
            let _ = std::fs::remove_file(&lock);
            std::fs::write(&lock, &content).expect("lock");
            run(l, &[(format!("(file-locked-p {f})"), expected)]);
            assert_eq!(lock.exists(), kept, "{content}");
        }
        let _ = std::fs::remove_file(&lock);
        let mut cifs = format!("x@testhost.{live}").into_bytes();
        cifs.extend_from_slice(&[0o357, 0o200, 0o242]);
        cifs.push(b'0');
        std::fs::write(&lock, &cifs).expect("lock");
        run(l, &[(format!("(file-locked-p {f})"), "\"x\"")]);
        for bad in [
            "garbage",
            "x@host",
            "x@host.12x",
            "x@host.12:",
            "x@host.12;3",
        ] {
            let _ = std::fs::remove_file(&lock);
            std::fs::write(&lock, bad).expect("lock");
            run(
                l,
                &[(
                    format!(
                        "(condition-case e (file-locked-p {f}) (file-error (list (car e) (nth 1 e) (nth 2 e) (equal (nth 3 e) {f}))))"
                    ),
                    "(file-error \"Testing file lock\" \"Invalid argument\" t)",
                )],
            );
        }
        run(
            l,
            &[
                (
                    s(
                        "(defalias 'userlock--handle-unlock-error (function (lambda (err) (setq unlock-err err))))",
                    ),
                    "userlock--handle-unlock-error",
                ),
                (format!("(unlock-file {f})"), "nil"),
                (
                    format!(
                        "(list (car unlock-err) (nth 1 unlock-err) (nth 2 unlock-err) (equal (nth 3 unlock-err) {f}))"
                    ),
                    "(file-error \"Unlocking file\" \"Invalid argument\" t)",
                ),
            ],
        );
        assert!(lock.exists());
    });
}

#[test]
fn buffer_locking() {
    let (file, lock, f) = scratch("buffer");
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        run(
            l,
            &[
                (s(SETUP), "nil"),
                (s(RAW_TEXT), "nil"),
                (
                    s("(set-buffer (get-buffer-create \"vis\"))"),
                    "#<buffer vis>",
                ),
                (
                    format!(
                        "(progn (setq buffer-file-name (expand-file-name {f}) buffer-file-truename buffer-file-name) nil)"
                    ),
                    "nil",
                ),
                (s("(insert \"x\")"), "nil"),
                (format!("(file-locked-p {f})"), "t"),
                (s("(set-buffer-modified-p nil)"), "nil"),
                (format!("(file-locked-p {f})"), "nil"),
                (s("(restore-buffer-modified-p t)"), "t"),
                (format!("(file-locked-p {f})"), "t"),
                (
                    s("(let ((inhibit-modification-hooks t)) (set-buffer-modified-p nil))"),
                    "nil",
                ),
                (format!("(file-locked-p {f})"), "t"),
                (s("(unlock-buffer)"), "nil"),
                (format!("(file-locked-p {f})"), "t"),
                (s("(insert \"y\")"), "nil"),
                (s("(unlock-buffer)"), "nil"),
                (format!("(file-locked-p {f})"), "nil"),
                (s("(lock-buffer)"), "nil"),
                (format!("(file-locked-p {f})"), "t"),
                (
                    s("(lock-buffer 1)"),
                    "error: (wrong-type-argument stringp 1)",
                ),
                (s("(kill-buffer \"vis\")"), "t"),
                (format!("(file-locked-p {f})"), "nil"),
                (
                    s(
                        "(defalias 'userlock--ask-user-about-supersession-threat (function (lambda (fn) (setq threatened fn))))",
                    ),
                    "userlock--ask-user-about-supersession-threat",
                ),
                (
                    s("(set-buffer (get-buffer-create \"sup\"))"),
                    "#<buffer sup>",
                ),
                (
                    format!(
                        "(let ((coding-system-for-write 'raw-text-unix)) (insert \"abc\") (write-region nil nil {f} nil t) (setq buffer-file-truename buffer-file-name) (list (buffer-modified-p) (file-locked-p {f})))"
                    ),
                    "(nil nil)",
                ),
                (
                    format!(
                        "(let ((coding-system-for-write 'raw-text-unix) (write-region-post-annotation-function (function (lambda () (setq during (file-locked-p {f})))))) (write-region \"other\" nil (concat {f} \".2\")) (list during (file-locked-p {f})))"
                    ),
                    "(nil nil)",
                ),
                (
                    format!(
                        "(let ((coding-system-for-write 'raw-text-unix) (write-region-post-annotation-function (function (lambda () (setq during (file-locked-p {f})))))) (write-region \"other\" nil {f} nil nil) (list during (file-locked-p {f})))"
                    ),
                    "(t nil)",
                ),
                (s("(set-visited-file-modtime)"), "nil"),
            ],
        );
        std::fs::write(&file, "changed on disk").expect("rewrite");
        run(
            l,
            &[
                (s("(setq threatened nil)"), "nil"),
                (s("(insert \"z\")"), "nil"),
                (
                    format!("(list (equal threatened (expand-file-name {f})) (file-locked-p {f}))"),
                    "(t t)",
                ),
                (s("(setq threatened nil)"), "nil"),
                (s("(insert \"w\")"), "nil"),
                (s("threatened"), "nil"),
            ],
        );
        assert!(lock.exists());
    });
}

#[test]
fn lock_handlers() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        run(
            l,
            &[
                (s(SETUP), "nil"),
                (
                    s(
                        "(defalias 'lk-handler (function (lambda (op &rest args) (if (memq op '(lock-file unlock-file file-locked-p)) (cons op args) (let ((inhibit-file-name-handlers (cons 'lk-handler inhibit-file-name-handlers)) (inhibit-file-name-operation op)) (apply op args))))))",
                    ),
                    "lk-handler",
                ),
                (
                    s("(setq file-name-handler-alist (list (cons \"\\\\.gpg\\\\'\" 'lk-handler)))"),
                    "((\"\\\\.gpg\\\\'\" . lk-handler))",
                ),
                (s("(lock-file \"/x/a.gpg\")"), "(lock-file \"/x/a.gpg\")"),
                (s("(unlock-file \"/x/a.gpg\")"), "nil"),
                (
                    s("(file-locked-p \"/x/a.gpg\")"),
                    "(file-locked-p \"/x/a.gpg\")",
                ),
            ],
        );
    });
}
