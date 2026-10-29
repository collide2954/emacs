mod common;

use std::path::{Path, PathBuf};

use common::{check, eval, on_big_stack};
use emacs_core::lisp::Lisp;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("emacs-auto-save-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch directory");
    dir
}

fn lisp_string(path: &Path) -> String {
    format!("{:?}", path.to_string_lossy().replace('\\', "/"))
}

fn ok(lisp: &mut Lisp, source: &str, expected: &str) {
    assert_eq!(eval(lisp, source), expected, "evaluating {source}");
}

#[test]
fn auto_save_state() {
    check(&[
        (
            "(progn (set-buffer (get-buffer-create \"a\")) (recent-auto-save-p))",
            "nil",
        ),
        (
            "(progn (insert \"abc\") (set-buffer-auto-saved) (list (recent-auto-save-p) buffer-saved-size))",
            "(t 3)",
        ),
        ("(progn (insert \"d\") (recent-auto-save-p))", "t"),
        (
            "(progn (set-buffer-modified-p nil) (recent-auto-save-p))",
            "nil",
        ),
        ("(clear-buffer-auto-save-failure)", "nil"),
    ]);
}

#[test]
fn do_auto_save() {
    let dir = scratch("buffers");
    let a = dir.join("#a#");
    let b = dir.join("#b#");
    let list = dir.join("list").join("saves");
    let missing = dir.join("missing").join("#d#");
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        ok(
            l,
            "(progn (define-coding-system-internal 'utf-8-emacs ?U 'utf-8 '(emacs) t nil nil nil nil nil nil '(:name utf-8-emacs) nil nil) (define-coding-system-alias 'unix 'undecided-unix))",
            "nil",
        );
        ok(
            l,
            &format!("(setq auto-save-list-file-name {})", lisp_string(&list)),
            &lisp_string(&list),
        );
        ok(
            l,
            "(defalias 'make-directory (function (lambda (dir parents) (make-directory-internal dir))))",
            "make-directory",
        );
        ok(
            l,
            &format!(
                "(progn (set-buffer (get-buffer-create \"a\")) (insert \"alpha\") (setq buffer-auto-save-file-name {}) (set-buffer (get-buffer-create \"b\")) (setq buffer-file-name \"/visited/b\") (insert \"beta\") (setq buffer-auto-save-file-name {}) (set-buffer (get-buffer-create \"c\")) (insert \"gamma\") (do-auto-save t t))",
                lisp_string(&a),
                lisp_string(&b)
            ),
            "nil",
        );
        assert!(!a.exists() && !b.exists());
        assert_eq!(
            std::fs::read_to_string(&list).expect("auto-save list"),
            format!(
                "\n{}\n/visited/b\n{}\n",
                a.to_string_lossy().replace('\\', "/"),
                b.to_string_lossy().replace('\\', "/")
            )
        );
        ok(l, "(buffer-name)", "\"c\"");
        ok(
            l,
            "(progn (defvar noted nil) (defalias 'boom (function (lambda () (signal 'error '(\"boom\"))))) (defalias 'note (function (lambda () (setq noted t)))) (setq auto-save-hook '(boom note)) (do-auto-save t))",
            "nil",
        );
        ok(l, "(list auto-save-hook noted)", "((note) t)");
        assert_eq!(std::fs::read_to_string(&a).expect("a saved"), "alpha");
        assert_eq!(std::fs::read_to_string(&b).expect("b saved"), "beta");
        ok(
            l,
            "(save-current-buffer (set-buffer \"a\") (list (recent-auto-save-p) buffer-saved-size (buffer-modified-p)))",
            "(t 5 autosaved)",
        );
        std::fs::remove_file(&a).expect("remove a");
        ok(l, "(do-auto-save t)", "nil");
        assert!(!a.exists());
        ok(
            l,
            "(save-current-buffer (set-buffer \"a\") (insert \"x\") (setq buffer-saved-size -1) (do-auto-save t))",
            "nil",
        );
        assert!(!a.exists());
        ok(
            l,
            "(save-current-buffer (set-buffer \"*Messages*\") (erase-buffer))",
            "nil",
        );
        ok(
            l,
            "(save-current-buffer (set-buffer \"b\") (insert \"!\") (do-auto-save) (set-buffer \"*Messages*\") (buffer-string))",
            "\"Auto-saving...done\n\"",
        );
        assert_eq!(std::fs::read_to_string(&b).expect("b saved"), "beta!");
        ok(
            l,
            "(defalias 'display-warning (function (lambda (type message level) (setq warned (list type message level)))))",
            "display-warning",
        );
        ok(
            l,
            &format!(
                "(progn (defvar warned nil) (set-buffer (get-buffer-create \"d\")) (insert \"delta\") (setq buffer-auto-save-file-name {}) (do-auto-save t t) (list (car warned) (nth 2 warned) (string-match \"\\\\`Auto-saving d: Opening output file: \" (nth 1 warned)) (recent-auto-save-p)))",
                lisp_string(&missing)
            ),
            "(auto-save :error 0 t)",
        );
        ok(
            l,
            "(progn (set-buffer (get-buffer-create \"e\")) (setq buffer-file-name \"/visited/e\" buffer-auto-save-file-name \"/never/e\") (insert \"epsilon\") (setq buffer-saved-size 8000) (save-current-buffer (set-buffer \"*Messages*\") (erase-buffer)) (do-auto-save nil t) (list buffer-saved-size (save-current-buffer (set-buffer \"*Messages*\") (buffer-string))))",
            "(-1 \"\")",
        );
    });
}
