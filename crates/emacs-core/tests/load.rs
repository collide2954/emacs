mod common;

use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use common::{eval, on_big_stack};
use emacs_core::lisp::Lisp;

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Capture {
    fn take(&self) -> String {
        let text = String::from_utf8_lossy(&self.0.borrow()).into_owned();
        self.0.borrow_mut().clear();
        text
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("emacs-load-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create directory");
    dir
}

fn dir_name(path: &Path) -> String {
    format!("{}/", path.to_string_lossy().replace('\\', "/"))
}

fn set_mtime(path: &Path, secs: u64) {
    let file = std::fs::File::options()
        .write(true)
        .open(path)
        .expect("open for mtime");
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
        .expect("set mtime");
}

fn run(cases: Vec<(String, String)>) {
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        for (source, expected) in &cases {
            assert_eq!(eval(&mut lisp, source), *expected, "evaluating {source}");
        }
    });
}

#[test]
fn locate_file_internal() {
    let a = temp_dir("locate-a");
    let b = temp_dir("locate-b");
    std::fs::write(a.join("foo.el"), "").expect("write");
    std::fs::write(b.join("foo.elc"), "").expect("write");
    std::fs::write(b.join("bar.txt"), "").expect("write");
    std::fs::create_dir(a.join("sub")).expect("mkdir");
    let (la, lb) = (dir_name(&a), dir_name(&b));
    let handler = "(defalias 'lf-handler (function (lambda (op &rest args) (if (eq op 'file-readable-p) (equal (file-name-nondirectory (car args)) \"x.el\") (let ((inhibit-file-name-handlers (cons 'lf-handler inhibit-file-name-handlers)) (inhibit-file-name-operation op)) (apply op args))))))";
    let cases = vec![
        (format!("(setq la {la:?} lb {lb:?})"), format!("{lb:?}")),
        (
            "(equal (locate-file-internal \"foo\" (list la lb) '(\".elc\" \".el\")) (expand-file-name \"foo.el\" la))".into(),
            "t".into(),
        ),
        ("(locate-file-internal \"sub\" (list la) nil)".into(), "nil".into()),
        (
            "(equal (locate-file-internal \"sub\" (list la) nil (function (lambda (f) (if (file-directory-p f) 'dir-ok)))) (expand-file-name \"sub\" la))".into(),
            "t".into(),
        ),
        (
            "(locate-file-internal \"sub\" (list la) nil 'file-exists-p)".into(),
            "nil".into(),
        ),
        (
            "(equal (locate-file-internal \"bar.txt\" (list la lb) nil 'file-exists-p) (expand-file-name \"bar.txt\" lb))".into(),
            "t".into(),
        ),
        (
            "(equal (locate-file-internal \"foo\" (list la) '(\".el\") 4) (expand-file-name \"foo.el\" la))".into(),
            "t".into(),
        ),
        ("(locate-file-internal \"sub\" (list la) nil 0)".into(), "nil".into()),
        (
            "(let ((default-directory la)) (equal (locate-file-internal \"foo.el\" nil) (expand-file-name \"foo.el\" la)))".into(),
            "t".into(),
        ),
        (
            "(let ((default-directory la)) (equal (locate-file-internal \"foo.el\" '(nil)) (expand-file-name \"foo.el\" la)))".into(),
            "t".into(),
        ),
        (
            "(locate-file-internal \"foo\" (list la) '(\".el\" 3))".into(),
            "error: (wrong-type-argument stringp 3)".into(),
        ),
        (
            "(locate-file-internal 'foo nil)".into(),
            "error: (wrong-type-argument stringp foo)".into(),
        ),
        (handler.into(), "lf-handler".into()),
        (
            "(let ((file-name-handler-alist (list (cons \"/magic/\" 'lf-handler)))) (equal (locate-file-internal \"x\" '(\"/magic/\") '(\".elc\" \".el\")) (expand-file-name \"x.el\" \"/magic/\")))".into(),
            "t".into(),
        ),
    ];
    run(cases);
}

#[test]
fn load_search() {
    let dir = temp_dir("search");
    std::fs::write(
        dir.join("n.el"),
        "(setq n-loaded (list load-file-name load-true-file-name load-in-progress))\n",
    )
    .expect("write");
    std::fs::write(dir.join("plain"), "(setq plain-loaded t)\n").expect("write");
    let ll = dir_name(&dir);
    let cases = vec![
        (format!("(setq ll {ll:?})"), format!("{ll:?}")),
        (
            "(load \"\")".into(),
            "error: (file-missing \"Cannot open load file\" \"No such file or directory\" \"\")"
                .into(),
        ),
        ("(load \"no-such-file-xyz\" t)".into(), "nil".into()),
        (
            "(load \"no-such-file-xyz\")".into(),
            "error: (file-missing \"Cannot open load file\" \"No such file or directory\" \"no-such-file-xyz\")".into(),
        ),
        (
            "(let ((load-path (list ll))) (load \"plain\" t t nil t))".into(),
            "nil".into(),
        ),
        (
            "(let ((load-path (list ll))) (list (load \"plain\" nil t) plain-loaded))".into(),
            "(t t)".into(),
        ),
        (
            "(condition-case e (load (concat ll \"n\") nil t t) (file-missing (equal e (list 'file-missing \"Cannot open load file\" \"No such file or directory\" (substitute-in-file-name (concat ll \"n\"))))))".into(),
            "t".into(),
        ),
        (
            "(let ((user-init-file t)) (load (concat ll \"n\") nil t) (equal user-init-file (expand-file-name \"n.el\" ll)))".into(),
            "t".into(),
        ),
        (
            "(equal n-loaded (list (expand-file-name \"n.el\" ll) (expand-file-name \"n.el\" ll) t))"
                .into(),
            "t".into(),
        ),
        (
            "(let ((load-path nil) (load-path-filter-function (function (lambda (path file suffixes) (setq lpf-args (list path file suffixes)) (list ll))))) (load \"n\" nil t))".into(),
            "t".into(),
        ),
        ("lpf-args".into(), "(nil \"n\" (\".elc\" \".el\" \"\"))".into()),
        (
            "(defalias 'load-handler (function (lambda (op &rest args) (cons op args))))".into(),
            "load-handler".into(),
        ),
        (
            "(let ((file-name-handler-alist (list (cons \"\\\\.zz\\\\'\" 'load-handler)))) (load \"x.zz\" 1 2 3 4))".into(),
            "(load \"x.zz\" 1 2 3 4)".into(),
        ),
        (
            "(defalias 'do-after-load-evaluation (function (lambda (f) (setq dale f))))".into(),
            "do-after-load-evaluation".into(),
        ),
        (
            "(let ((purify-flag t) (preloaded-file-list nil)) (load (concat ll \"n\") nil t) (list (equal preloaded-file-list (list (substitute-in-file-name (concat ll \"n\")))) (equal (car n-loaded) (substitute-in-file-name (concat ll \"n.el\"))) (equal (car (cdr n-loaded)) (expand-file-name \"n.el\" ll)) (equal dale (car n-loaded))))".into(),
            "(t t t t)".into(),
        ),
    ];
    run(cases);
}

#[test]
fn load_compiled() {
    let dir = temp_dir("compiled");
    let elc = |body: &str| {
        format!(";ELC\x1e\0\0\0\n;;; Compiled\n;;; in Emacs version 31.0.50\n{body}\n")
    };
    std::fs::write(dir.join("bad.elc"), "(setq bad-loaded t)\n").expect("write");
    std::fs::write(dir.join("comp.el"), elc("(setq comp-lex lexical-binding)")).expect("write");
    std::fs::write(dir.join("src.el"), "(setq src-lex lexical-binding)\n").expect("write");
    std::fs::write(dir.join("pn.el"), "(setq pn-from 'source)\n").expect("write");
    std::fs::write(dir.join("pn.elc"), elc("(setq pn-from 'compiled)")).expect("write");
    set_mtime(&dir.join("pn.elc"), 1_000_000_000);
    set_mtime(&dir.join("pn.el"), 1_100_000_000);
    let lc = dir_name(&dir);
    on_big_stack(move || {
        let capture = Capture::default();
        let mut lisp = Lisp::new();
        lisp.stderr = Box::new(capture.clone());
        let l = &mut lisp;
        let cases = [
            (format!("(setq lc {lc:?})"), format!("{lc:?}")),
            (
                "(condition-case e (progn (load (concat lc \"bad.elc\") nil t) 'loaded) (error (equal e (list 'error (format \"File `%s' was not compiled in Emacs\" (expand-file-name \"bad.elc\" lc))))))".into(),
                "t".into(),
            ),
            ("(boundp 'bad-loaded)".into(), "nil".into()),
            (
                "(defalias 'record-source-load (function (lambda (&rest args) (setq lsff args) t)))"
                    .into(),
                "record-source-load".into(),
            ),
            (
                "(let ((load-source-file-function 'record-source-load)) (list (load (concat lc \"comp.el\") nil t) (boundp 'lsff) comp-lex))".into(),
                "(t nil nil)".into(),
            ),
            (
                "(let ((load-source-file-function 'record-source-load) (force-load-messages t)) (load (concat lc \"src\") nil t) (list (equal (car lsff) (expand-file-name \"src.el\" lc)) (equal (car (cdr lsff)) (car lsff)) (nthcdr 2 lsff)))".into(),
                "(t t (nil nil))".into(),
            ),
            (
                "(let ((internal--get-default-lexical-binding-function (function (lambda (f) (setq gdlb f) t)))) (load (concat lc \"src.el\") nil t) (list src-lex (equal gdlb (substitute-in-file-name (concat lc \"src.el\")))))".into(),
                "(t t)".into(),
            ),
        ];
        for (source, expected) in &cases {
            assert_eq!(eval(l, source), *expected, "evaluating {source}");
        }
        capture.take();
        assert_eq!(
            eval(l, "(progn (load (concat lc \"pn\") nil t) pn-from)"),
            "compiled"
        );
        let found = eval(l, "(expand-file-name \"pn.el\" lc)");
        let found = found.trim_matches('"');
        assert_eq!(
            capture.take(),
            format!(
                "Source file \u{2018}{found}\u{2019} newer than byte-compiled file; using older file\n"
            )
        );
        assert_eq!(
            eval(
                l,
                "(let ((load-prefer-newer t)) (load (concat lc \"pn\") nil t) pn-from)"
            ),
            "source"
        );
        assert_eq!(capture.take(), "");
        assert_eq!(
            eval(l, "(progn (load (concat lc \"pn\")) pn-from)"),
            "compiled"
        );
        let file = eval(l, "(substitute-in-file-name (concat lc \"pn\"))");
        let file = file.trim_matches('"');
        assert_eq!(
            capture.take(),
            format!("Loading {file} (compiled; note, source file is newer)...\n")
        );
        assert_eq!(
            eval(
                l,
                "(defalias 'byte-run--unescaped-character-literals-warning (function (lambda () \"uw\")))"
            ),
            "byte-run--unescaped-character-literals-warning"
        );
        assert_eq!(eval(l, "(load (concat lc \"src.el\") nil t)"), "t");
        let file = eval(l, "(substitute-in-file-name (concat lc \"src.el\"))");
        let file = file.trim_matches('"');
        assert_eq!(
            capture.take(),
            format!("Loading \u{2018}{file}\u{2019}: uw\n")
        );
    });
}

#[test]
fn load_emacs_mule_elc() {
    let dir = temp_dir("mule");
    let mut bytes = b";ELC\x13\0\0\0\n;;; in Emacs version 19\n(setq mule-str \"".to_vec();
    bytes.extend_from_slice(b"\x81\xe9\" mule-forced load-force-doc-strings)\n");
    std::fs::write(dir.join("mule.elc"), bytes).expect("write");
    let lm = dir_name(&dir);
    run(vec![
        (format!("(setq lm {lm:?})"), format!("{lm:?}")),
        (
            "(define-charset-internal 'test-latin1 1 [32 127 0 0 0 0 0 0] nil nil nil nil 129 nil nil nil 160 nil nil nil nil '(:name test-latin1))".into(),
            "nil".into(),
        ),
        ("(load (concat lm \"mule\") nil t)".into(), "t".into()),
        ("(list mule-str mule-forced)".into(), "(\"\u{e9}\" t)".into()),
    ]);
}

#[test]
fn load_source_text_mode() {
    let dir = temp_dir("text");
    std::fs::write(
        dir.join("crlf.el"),
        b"(setq crlf-str \"a\r\nb\")\r\n(setq crlf-z 1)\r\n;\x1a\n(setq crlf-after-z 2)\n",
    )
    .expect("write");
    let lt = dir_name(&dir);
    let expected = if cfg!(windows) {
        "(\"a\nb\" 1 nil)"
    } else {
        "(\"a\r\nb\" 1 t)"
    };
    run(vec![
        (format!("(setq lt {lt:?})"), format!("{lt:?}")),
        ("(load (concat lt \"crlf\") nil t)".into(), "t".into()),
        (
            "(list crlf-str crlf-z (boundp 'crlf-after-z))".into(),
            expected.into(),
        ),
    ]);
}

#[test]
fn bytecomp_version_regexp() {
    run(vec![(
        "bytecomp-version-regexp".into(),
        "\"^;;;.\\\\(in Emacs version\\\\|bytecomp version FSF\\\\)\"".into(),
    )]);
}
