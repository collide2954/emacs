mod common;

use common::check;
#[cfg(windows)]
use common::{eval, on_big_stack};
#[cfg(windows)]
use emacs_core::lisp::Lisp;

const RANGE_ERROR: &str =
    "error: (error \"Not an in-range integer, integral float, or cons of integers\")";

#[test]
fn user_and_group_primitives() {
    check(&[
        ("(stringp (user-login-name))", "t"),
        ("(stringp (user-real-login-name))", "t"),
        ("(eq (user-login-name) user-login-name)", "t"),
        ("(eq (user-real-login-name) user-real-login-name)", "t"),
        ("(stringp (user-full-name))", "t"),
        ("(eq (user-full-name) user-full-name)", "t"),
        ("(integerp (user-uid))", "t"),
        ("(integerp (user-real-uid))", "t"),
        ("(integerp (group-gid))", "t"),
        ("(integerp (group-real-gid))", "t"),
        ("(stringp (user-login-name (user-real-uid)))", "t"),
        ("(stringp (group-name (group-real-gid)))", "t"),
        (
            "(equal (user-login-name (float (user-real-uid))) (user-login-name (user-real-uid)))",
            "t",
        ),
        (
            "(equal (user-login-name (cons 0 (user-real-uid))) (user-login-name (user-real-uid)))",
            "t",
        ),
        ("(user-login-name -1)", RANGE_ERROR),
        ("(user-login-name 1.5)", RANGE_ERROR),
        ("(user-login-name 'x)", RANGE_ERROR),
        ("(user-login-name 4294967296)", RANGE_ERROR),
        (
            "(group-name 'x)",
            "error: (error \"Invalid GID specification\")",
        ),
        ("(group-name -1)", RANGE_ERROR),
        (
            "(user-full-name 'x)",
            "error: (error \"Invalid UID specification\")",
        ),
        ("(user-full-name \"no such user here\")", "nil"),
        (
            "(progn (setq user-login-name \"x\") (user-login-name))",
            "\"x\"",
        ),
        (
            "(progn (setq user-login-name nil) (stringp (user-login-name)))",
            "t",
        ),
    ]);
}

#[cfg(windows)]
fn whoami_user_rid() -> u32 {
    let out = std::process::Command::new("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .expect("whoami");
    let text = String::from_utf8_lossy(&out.stdout);
    let sid = text
        .trim()
        .rsplit(',')
        .next()
        .expect("sid")
        .trim_matches('"');
    sid.rsplit('-')
        .next()
        .expect("rid")
        .parse()
        .expect("numeric rid")
}

#[cfg(windows)]
#[test]
fn windows_users_come_from_the_process_token() {
    let rid = whoami_user_rid();
    let username = std::env::var("USERNAME").expect("USERNAME");
    let login = std::env::var("LOGNAME").unwrap_or_else(|_| username.clone());
    let full = std::env::var("NAME").unwrap_or_default();
    on_big_stack(move || {
        let mut lisp = Lisp::new();
        let real = eval(&mut lisp, "(user-real-login-name)");
        assert!(
            real.trim_matches('"').eq_ignore_ascii_case(&username),
            "{real} vs {username}"
        );
        let cases = [
            ("(user-uid)".to_string(), rid.to_string()),
            ("(user-real-uid)".to_string(), rid.to_string()),
            (
                "(= (group-gid) (group-real-gid))".to_string(),
                "t".to_string(),
            ),
            ("(user-login-name)".to_string(), format!("{login:?}")),
            ("(user-login-name (user-uid))".to_string(), real.clone()),
            (
                "(user-login-name (1+ (user-uid)))".to_string(),
                "nil".to_string(),
            ),
            ("(user-full-name)".to_string(), format!("{full:?}")),
            (
                "(user-full-name (user-uid))".to_string(),
                "\"\"".to_string(),
            ),
            (
                "(user-full-name (1+ (user-uid)))".to_string(),
                "nil".to_string(),
            ),
            (
                format!("(user-full-name {:?})", username.to_uppercase()),
                "\"\"".to_string(),
            ),
            (
                "(group-name (1+ (group-gid)))".to_string(),
                "nil".to_string(),
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(eval(&mut lisp, &source), expected, "evaluating {source}");
        }
        let dir = std::env::temp_dir().join(format!("emacs-user-identity-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("owned.txt");
        std::fs::write(&file, b"x").unwrap();
        let name = file.to_string_lossy().replace('\\', "/");
        assert_eq!(
            eval(
                &mut lisp,
                &format!(
                    "(let ((a (file-attributes {name:?})) (s (file-attributes {name:?} 'string))) (list (= (nth 2 a) (user-uid)) (= (nth 3 a) (group-gid)) (equal (nth 2 s) (user-real-login-name)) (equal (nth 3 s) (group-name (group-gid)))))"
                )
            ),
            "(t t t t)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    });
}
