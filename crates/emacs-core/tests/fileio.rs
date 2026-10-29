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
            (FILE_SETUP.to_string(), "nil"),
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

const TEST_UTF: &str = "(define-coding-system-internal 'test-utf ?u 'utf-8 '(unicode) t nil nil nil nil nil nil '(:name test-utf) nil nil)";
const FILE_SETUP: &str = "(progn (define-coding-system-internal 'raw-text ?t 'raw-text nil t nil nil nil nil nil t '(:name raw-text) nil) (defalias 'make-lock-file-name (function (lambda (f) (concat (file-name-directory f) \".#\" (file-name-nondirectory f))))) nil)";

fn lisp_path(path: &std::path::Path) -> String {
    format!("{:?}", path.to_string_lossy().replace('\\', "/"))
}

#[test]
fn write_region_encoding() {
    let dir = std::env::temp_dir().join(format!("emacs-write-encode-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create directory");
    let names = [
        "utf.txt", "dos.txt", "eol.txt", "raw.txt", "str.txt", "uni.txt", "dflt.txt", "sel.txt",
    ];
    let paths: Vec<_> = names.iter().map(|n| dir.join(n)).collect();
    let p: Vec<String> = paths.iter().map(|p| lisp_path(p)).collect();
    let system_eol = if cfg!(windows) { "dos" } else { "unix" };
    let expected_dflt = format!("raw-text-{system_eol}");
    let expected_eol = format!("test-utf-{system_eol}");
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        let cases: Vec<(String, &str)> = vec![
            (TEST_UTF.to_string(), "nil"),
            (FILE_SETUP.to_string(), "nil"),
            (
                format!(
                    "(let ((coding-system-for-write 'test-utf-unix)) (write-region \"\u{e9}t\u{e9}\\n\" nil {}))",
                    p[0]
                ),
                "nil",
            ),
            ("last-coding-system-used".to_string(), "test-utf-unix"),
            (
                format!(
                    "(let ((coding-system-for-write 'test-utf-dos)) (write-region \"a\\nb\" nil {}))",
                    p[1]
                ),
                "nil",
            ),
            (
                format!(
                    "(let ((coding-system-for-write 'test-utf)) (write-region \"a\\nb\" nil {}) last-coding-system-used)",
                    p[2]
                ),
                &expected_eol,
            ),
            (
                format!(
                    "(save-current-buffer (set-buffer (get-buffer-create \"raw\")) (insert \"a\" (string-to-multibyte \"\\351\\200\") \"b\") (let ((coding-system-for-write 'no-conversion)) (write-region nil nil {})) last-coding-system-used)",
                    p[3]
                ),
                "no-conversion",
            ),
            (
                format!(
                    "(let ((coding-system-for-write 'no-conversion)) (write-region (string-to-multibyte \"\\351\") nil {}))",
                    p[4]
                ),
                "nil",
            ),
            (
                format!(
                    "(save-current-buffer (set-buffer (get-buffer-create \"uni\")) (set-buffer-multibyte nil) (insert \"\\351\\n\") (write-region nil nil {}) last-coding-system-used)",
                    p[5]
                ),
                &expected_dflt,
            ),
            (
                format!(
                    "(save-current-buffer (set-buffer (get-buffer-create \"dflt\")) (insert \"a\\nb\") (write-region nil nil {}) last-coding-system-used)",
                    p[6]
                ),
                &expected_dflt,
            ),
            (
                format!(
                    "(save-current-buffer (set-buffer (get-buffer-create \"sel\")) (insert \"a\\rb\\n\") (setq selective-display t) (let ((coding-system-for-write 'raw-text-unix)) (write-region nil nil {})))",
                    p[7]
                ),
                "nil",
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(eval(l, &source), expected, "evaluating {source}");
        }
    });
    let read = |i: usize| std::fs::read(&paths[i]).expect("written");
    let crlf = |s: &[u8]| -> Vec<u8> {
        if cfg!(windows) {
            s.iter()
                .flat_map(|&b| {
                    if b == b'\n' {
                        vec![b'\r', b'\n']
                    } else {
                        vec![b]
                    }
                })
                .collect()
        } else {
            s.to_vec()
        }
    };
    assert_eq!(read(0), b"\xc3\xa9t\xc3\xa9\n");
    assert_eq!(read(1), b"a\r\nb");
    assert_eq!(read(2), crlf(b"a\nb"));
    assert_eq!(read(3), b"a\xe9\x80b");
    assert_eq!(read(4), b"\xe9");
    assert_eq!(read(5), crlf(b"\xe9\n"));
    assert_eq!(read(6), crlf(b"a\nb"));
    assert_eq!(read(7), b"a\nb\n");
}

#[test]
fn write_region_positions() {
    let dir = std::env::temp_dir().join(format!("emacs-write-pos-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create directory");
    let file = dir.join("pos.txt");
    let empty = dir.join("empty.txt");
    let narrowed = dir.join("narrowed.txt");
    let f = lisp_path(&file);
    let e = lisp_path(&empty);
    let n = lisp_path(&narrowed);
    let (file2, empty2, narrowed2) = (file.clone(), empty.clone(), narrowed.clone());
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        let cases: Vec<(String, String)> = vec![
            (FILE_SETUP.to_string(), "nil".to_string()),
            (
                format!(
                    "(let ((coding-system-for-write 'raw-text-unix)) (write-region \"abcdef\" nil {f}) (write-region \"XY\" nil {f} 2) (write-region \"Z\" nil {f} t))"
                ),
                "nil".to_string(),
            ),
            (
                format!(
                    "(condition-case err (let ((coding-system-for-write 'raw-text-unix)) (write-region \"Q\" nil {f} nil nil nil 'excl)) (file-already-exists (list (car err) (nth 1 err) (equal (nth 2 err) (expand-file-name {f})) (nthcdr 3 err))))"
                ),
                "(file-already-exists \"File exists\" t nil)".to_string(),
            ),
            (
                format!(
                    "(let ((coding-system-for-write 'raw-text-unix)) (write-region \"Q\" nil {f} -1))"
                ),
                "error: (wrong-type-argument file-offset -1)".to_string(),
            ),
            (
                "(set-buffer (get-buffer-create \"ann\"))".to_string(),
                "#<buffer ann>".to_string(),
            ),
            ("(insert \"abc\")".to_string(), "nil".to_string()),
            (
                format!(
                    "(let ((coding-system-for-write 'raw-text-unix) (write-region-annotate-functions (list (function (lambda (s e) (list (cons 1 \"[\") (cons 4 \"]\"))))))) (write-region 1 1 {e}))"
                ),
                "nil".to_string(),
            ),
            ("(narrow-to-region 2 3)".to_string(), "nil".to_string()),
            (
                format!(
                    "(let ((coding-system-for-write 'raw-text-unix) (wr-post nil) (write-region-post-annotation-function (function (lambda () (setq wr-post (buffer-name))))) (write-region-annotate-functions (list (function (lambda (s e) (setq wr-range (list s e)) (list (cons 2 \"<\") (cons 3 \">\"))))))) (write-region nil nil {n}) (list wr-post wr-range (point-min) (point-max)))"
                ),
                "(\"ann\" (nil nil) 2 3)".to_string(),
            ),
            (
                "(list write-region-annotations-so-far)".to_string(),
                "(nil)".to_string(),
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(eval(l, &source), expected, "evaluating {source}");
        }
    });
    assert_eq!(std::fs::read(&file2).expect("written"), b"abXYefZ");
    assert_eq!(std::fs::read(&empty2).expect("written"), b"[");
    assert_eq!(std::fs::read(&narrowed2).expect("written"), b"a<b>c");
}

#[test]
fn visited_file_modtime() {
    let dir = std::env::temp_dir().join(format!("emacs-modtime-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create directory");
    let file = dir.join("visited.txt");
    let f = lisp_path(&file);
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        let l = &mut lisp;
        let run = |l: &mut Lisp, cases: &[(String, &str)]| {
            for (source, expected) in cases {
                assert_eq!(eval(l, source), *expected, "evaluating {source}");
            }
        };
        let s = |x: &str| x.to_string();
        run(
            l,
            &[
                (s(FILE_SETUP), "nil"),
                (s("(set-buffer (get-buffer-create \"v\"))"), "#<buffer v>"),
                (s("(visited-file-modtime)"), "0"),
                (s("(verify-visited-file-modtime)"), "t"),
                (s("(set-visited-file-modtime -1)"), "nil"),
                (s("(visited-file-modtime)"), "-1"),
                (s("(set-visited-file-modtime 0)"), "nil"),
                (s("(visited-file-modtime)"), "0"),
                (
                    s("(set-visited-file-modtime 1)"),
                    "error: (args-out-of-range 1 -1 0)",
                ),
                (s("(set-visited-file-modtime '(0 5))"), "nil"),
                (s("(visited-file-modtime)"), "(5000000000 . 1000000000)"),
                (
                    s("(let ((current-time-list t)) (visited-file-modtime))"),
                    "(0 5 0 0)",
                ),
                (
                    s("(set-visited-file-modtime)"),
                    "error: (wrong-type-argument stringp nil)",
                ),
                (s("(verify-visited-file-modtime)"), "t"),
                (
                    s("(verify-visited-file-modtime 'nope)"),
                    "error: (wrong-type-argument bufferp nope)",
                ),
                (
                    format!(
                        "(progn (insert \"abc\") (write-region nil nil {f} nil t) (equal buffer-file-name (expand-file-name {f})))"
                    ),
                    "t",
                ),
                (s("(verify-visited-file-modtime)"), "t"),
                (
                    s("(equal (visited-file-modtime) (nth 5 (file-attributes buffer-file-name)))"),
                    "t",
                ),
                (
                    s("(verify-visited-file-modtime (get-buffer-create \"other\"))"),
                    "t",
                ),
            ],
        );
        std::fs::write(&file, "changed on disk").expect("rewrite");
        run(
            l,
            &[
                (s("(verify-visited-file-modtime)"), "nil"),
                (
                    s(
                        "(save-current-buffer (set-buffer \"other\") (verify-visited-file-modtime (get-buffer \"v\")))",
                    ),
                    "nil",
                ),
                (s("(set-visited-file-modtime)"), "nil"),
                (s("(verify-visited-file-modtime)"), "t"),
            ],
        );
        std::fs::remove_file(&file).expect("remove");
        run(
            l,
            &[
                (s("(verify-visited-file-modtime)"), "nil"),
                (s("(set-visited-file-modtime)"), "nil"),
                (s("(verify-visited-file-modtime)"), "nil"),
                (
                    s(
                        "(defalias 'mt-handler (function (lambda (op &rest args) (if (memq op '(verify-visited-file-modtime set-visited-file-modtime)) (cons op args) (let ((inhibit-file-name-handlers (cons 'mt-handler inhibit-file-name-handlers)) (inhibit-file-name-operation op)) (apply op args))))))",
                    ),
                    "mt-handler",
                ),
                (
                    s(
                        "(let ((file-name-handler-alist (list (cons \"\\\\.bad\\\\'\" (function (lambda (op &rest args) 'notastring)))))) (expand-file-name \"/x/a.bad\"))",
                    ),
                    "error: (error \"Invalid handler in `file-name-handler-alist'\")",
                ),
                (
                    s("(setq file-name-handler-alist (list (cons \"\\\\.gpg\\\\'\" 'mt-handler)))"),
                    "((\"\\\\.gpg\\\\'\" . mt-handler))",
                ),
                (s("(setq buffer-file-name \"/x/a.gpg\")"), "\"/x/a.gpg\""),
                (
                    s("(verify-visited-file-modtime)"),
                    "(verify-visited-file-modtime nil)",
                ),
                (
                    s("(verify-visited-file-modtime (current-buffer))"),
                    "(verify-visited-file-modtime #<buffer v>)",
                ),
                (
                    s("(set-visited-file-modtime)"),
                    "(set-visited-file-modtime nil)",
                ),
            ],
        );
    });
}
