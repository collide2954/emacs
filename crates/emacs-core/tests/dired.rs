mod common;

use common::check;

fn fixture(tag: &str) -> String {
    let dir = std::env::temp_dir().join(format!("emacs-dired-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("bdir")).expect("create fixture");
    for (name, text) in [
        ("a.txt", "hello"),
        ("b.el", ""),
        ("c.elc", ""),
        ("d1.o", ""),
        ("d2.c", ""),
    ] {
        std::fs::write(dir.join(name), text).expect("write fixture");
    }
    dir.to_string_lossy().replace('\\', "/")
}

fn run(cases: &[(String, &str)]) {
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    check(&cases);
}

#[test]
fn listing() {
    let d = fixture("listing");
    run(&[
        (
            format!("(directory-files {d:?})"),
            "(\".\" \"..\" \"a.txt\" \"b.el\" \"bdir\" \"c.elc\" \"d1.o\" \"d2.c\")",
        ),
        (
            format!("(directory-files {d:?} nil \"\\\\.el\\\\'\")"),
            "(\"b.el\")",
        ),
        (
            format!(
                "(equal (directory-files {d:?} t \"^a\") (list (expand-file-name \"a.txt\" {d:?})))"
            ),
            "t",
        ),
        (
            format!("(directory-files {d:?} nil \"^b\" nil 1)"),
            "(\"b.el\")",
        ),
        (
            format!("(mapcar (function car) (directory-files-and-attributes {d:?} nil \"^[ab]\"))"),
            "(\"a.txt\" \"b.el\" \"bdir\")",
        ),
        (
            format!(
                "(condition-case e (directory-files (expand-file-name \"nonexistent\" {d:?})) (file-missing (list (car e) (nth 1 e))))"
            ),
            "(file-missing \"Opening directory\")",
        ),
    ]);
}

#[test]
fn attributes() {
    let d = fixture("attributes");
    let a = format!("(file-attributes (expand-file-name \"a.txt\" {d:?}))");
    let b = format!("(file-attributes (expand-file-name \"bdir\" {d:?}))");
    run(&[
        (
            format!("(let ((a {a})) (list (nth 0 a) (nth 7 a) (nth 9 a) (length a)))"),
            "(nil 5 t 12)",
        ),
        (
            format!("(let ((b {b})) (list (nth 0 b) (aref (nth 8 b) 0)))"),
            "(t 100)",
        ),
        (
            format!("(file-attributes (expand-file-name \"missing\" {d:?}))"),
            "nil",
        ),
        (
            format!(
                "(let ((a (file-attributes (expand-file-name \"a.txt\" {d:?}) 'string))) (list (stringp (nth 2 a)) (integerp (nth 2 (file-attributes (expand-file-name \"a.txt\" {d:?}))))))"
            ),
            "(t t)",
        ),
        ("(file-attributes-lessp '(\"a\") '(\"b\"))".to_string(), "t"),
        (
            "(equal (system-users) (list user-real-login-name))".to_string(),
            "t",
        ),
    ]);
}

#[test]
fn completion() {
    let d = fixture("completion");
    run(&[
        (
            format!("(file-name-all-completions \"b\" {d:?})"),
            "(\"bdir/\" \"b.el\")",
        ),
        (format!("(file-name-completion \"a\" {d:?})"), "\"a.txt\""),
        (format!("(file-name-completion \"a.txt\" {d:?})"), "t"),
        (format!("(file-name-completion \"b\" {d:?})"), "\"b\""),
        (format!("(file-name-completion \"d\" {d:?})"), "\"d\""),
        (
            format!(
                "(let ((completion-ignored-extensions '(\".o\"))) (file-name-completion \"d\" {d:?}))"
            ),
            "\"d2.c\"",
        ),
        (
            format!("(file-name-completion \"b\" {d:?} 'file-directory-p)"),
            "\"bdir/\"",
        ),
        (format!("(file-name-completion \"zz\" {d:?})"), "nil"),
    ]);
}
