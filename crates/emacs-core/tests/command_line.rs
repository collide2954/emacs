mod common;

#[cfg(windows)]
use common::check;
use common::{eval, on_big_stack};
use emacs_core::emacs::{
    Argv, Exit, Startup, argmatch, decode_command_line, sort_args, version_text,
};
use emacs_core::lisp::Lisp;

fn argv(args: &[&str]) -> Argv {
    args.iter().map(|a| Some(a.as_bytes().to_vec())).collect()
}

fn shown(argv: &Argv) -> Vec<String> {
    argv.iter()
        .map(|a| match a {
            Some(a) => String::from_utf8_lossy(a).into_owned(),
            None => "<null>".to_string(),
        })
        .collect()
}

fn decode(args: &[&str]) -> Result<Startup, Exit> {
    decode_command_line(args.iter().map(|a| a.as_bytes().to_vec()).collect())
}

fn boot_eval(args: &[&str], cases: &[(&str, &str)]) {
    let startup = decode(args).expect("decode");
    on_big_stack(move || {
        let mut lisp = Lisp::boot(startup);
        for (source, expected) in cases {
            assert_eq!(eval(&mut lisp, source), *expected, "evaluating {source}");
        }
    });
}

#[test]
fn sorting_by_priority() {
    let mut a = argv(&["emacs", "foo", "--batch", "-l", "x", "-Q"]);
    sort_args(a.len(), &mut a).expect("sort");
    assert_eq!(shown(&a), ["emacs", "--batch", "-Q", "foo", "-l", "x"]);
    let mut a = argv(&["emacs", "--batch", "--batch", "f"]);
    sort_args(a.len(), &mut a).expect("sort");
    assert_eq!(shown(&a), ["emacs", "--batch", "f", "<null>"]);
    let mut a = argv(&["emacs", "f", "--", "--batch", "-Q"]);
    sort_args(a.len(), &mut a).expect("sort");
    assert_eq!(shown(&a), ["emacs", "f", "--", "--batch", "-Q"]);
    let mut a = argv(&["emacs", "x", "--eval=(f)", "--chdir=d", "-kill"]);
    sort_args(a.len(), &mut a).expect("sort");
    assert_eq!(
        shown(&a),
        ["emacs", "--chdir=d", "x", "--eval=(f)", "-kill"]
    );
    let mut a = argv(&["emacs", "--chdir"]);
    assert_eq!(
        sort_args(a.len(), &mut a),
        Err(Exit {
            code: 1,
            stdout: String::new(),
            stderr: "emacs: Option '--chdir' requires an argument\n".to_string(),
        })
    );
}

#[test]
fn matching_options() {
    let a = argv(&["emacs", "--bat", "--eval=x", "-l", "f", "--fun"]);
    let mut skip = 0;
    assert_eq!(
        argmatch(&a, a.len(), "-batch", Some("--batch"), 5, false, &mut skip),
        Some(None)
    );
    assert_eq!(skip, 1);
    assert_eq!(
        argmatch(&a, a.len(), "-eval", Some("--eval"), 3, true, &mut skip),
        Some(Some(b"x".to_vec()))
    );
    assert_eq!(skip, 2);
    assert_eq!(
        argmatch(&a, a.len(), "-l", Some("--load"), 3, true, &mut skip),
        Some(Some(b"f".to_vec()))
    );
    assert_eq!(skip, 4);
    assert_eq!(
        argmatch(&a, a.len(), "-f", Some("--funcall"), 3, true, &mut skip),
        None
    );
    assert_eq!(
        argmatch(&a, a.len(), "-f", Some("--funcall"), 6, false, &mut skip),
        None
    );
    let mut skip = 0;
    let b = argv(&["emacs", "--ba"]);
    assert_eq!(
        argmatch(&b, b.len(), "-batch", Some("--batch"), 5, false, &mut skip),
        None
    );
}

#[test]
fn decoding_startup_options() {
    let s = decode(&["emacs", "--eval", "(f)", "--batch"]).expect("decode");
    assert!(s.noninteractive && s.noninteractive1);
    assert_eq!(s.skip_args, 1);
    assert_eq!(shown(&s.argv), ["emacs", "--batch", "--eval", "(f)"]);
    let s = decode(&["emacs", "a", "--script", "f.el"]).expect("decode");
    assert!(s.noninteractive);
    assert_eq!(s.skip_args, 0);
    assert_eq!(shown(&s.argv), ["emacs", "-scriptload", "f.el", "a"]);
    let s = decode(&["emacs", "-x", "f"]).expect("decode");
    assert!(s.noninteractive && !s.noninteractive1 && s.no_site_lisp);
    assert_eq!(s.skip_args, 0);
    assert_eq!(shown(&s.argv), ["emacs", "-scripteval", "f"]);
    let s = decode(&["emacs", "-Q", "--batch", "-nw"]).expect("decode");
    assert!(s.no_site_lisp && s.inhibit_window_system && s.noninteractive);
    assert_eq!(s.skip_args, 2);
    let mut s = decode(&["emacs", "-l", "foo", "-nl", "--no-build-details"]).expect("decode");
    assert!(s.no_loadup && !s.build_details);
    assert_eq!(s.load_file(), Some(b"foo".to_vec()));
    let s = decode(&["emacs", "--version", "--help", "--daemon"]).expect("decode");
    assert!(s.only_version && s.noninteractive);
    assert!(version_text().starts_with("GNU Emacs 31.0.50\nCopyright (C) "));
    let s = decode(&["emacs", "--", "--batch"]).expect("decode");
    assert!(!s.noninteractive);
}

#[test]
fn options_that_exit() {
    let help = decode(&["emacs", "--he"]).expect_err("help");
    assert_eq!(help.code, 0);
    assert!(
        help.stdout
            .starts_with("Usage: emacs [OPTION-OR-FILENAME]...\n\nRun Emacs, the extensible")
    );
    assert!(
        help.stdout
            .ends_with("section of the Emacs manual or the file BUGS.\n")
    );
    assert_eq!(
        decode(&["emacs", "--temacs=pdump"])
            .expect_err("temacs")
            .stderr,
        "emacs: Invalid temacs mode 'pdump'\n"
    );
    assert_eq!(
        decode(&["emacs", "--fg-daemon=x"])
            .expect_err("daemon")
            .stderr,
        "This platform does not support daemon mode.\n"
    );
    let missing = decode(&["emacs", "--chdir", "no/such/dir/at/all"]).expect_err("chdir");
    assert_eq!(missing.code, 1);
    assert_eq!(
        missing.stderr,
        "emacs: Can't chdir to no/such/dir/at/all: No such file or directory\n"
    );
    assert_eq!(
        decode(&["emacs", "-t", "/dev/tty"]).expect_err("tty").code,
        1
    );
}

#[test]
fn command_line_args_and_invocation() {
    boot_eval(
        &["emacs", "--eval", "(f)", "--batch", "-Q"],
        &[
            ("(cdr command-line-args)", "(\"-Q\" \"--eval\" \"(f)\")"),
            ("(multibyte-string-p (car command-line-args))", "nil"),
            ("(equal (invocation-name) invocation-name)", "t"),
            ("(eq (invocation-name) invocation-name)", "nil"),
            (
                "(and (file-name-absolute-p invocation-directory) (directory-name-p invocation-directory))",
                "t",
            ),
            ("installation-directory", "nil"),
            ("noninteractive", "t"),
            ("undo-outer-limit", "24000000"),
            (
                "(daemon-initialized)",
                "error: (error \"This function can only be called if emacs is run as a daemon\")",
            ),
        ],
    );
    boot_eval(
        &["emacs", "-x", "script"],
        &[
            ("(cdr command-line-args)", "(\"-scripteval\" \"script\")"),
            ("noninteractive", "nil"),
        ],
    );
}

#[cfg(windows)]
#[test]
fn windows_file_names() {
    check(&[
        (
            "(file-name-directory \"C:\\\\foo\\\\bar.el\")",
            "\"c:/foo/\"",
        ),
        ("(file-name-directory \"c:foo\")", "\"c:/\""),
        ("(file-name-directory \"/:c:foo\")", "\"/:C://\""),
        ("(file-name-directory \"foo\")", "nil"),
        ("(file-name-nondirectory \"c:foo\")", "\"foo\""),
        ("(file-name-nondirectory \"/:c:foo\")", "\"foo\""),
        (
            "(file-name-nondirectory \"C:\\\\foo\\\\bar.el\")",
            "\"bar.el\"",
        ),
        (
            "(let ((w32-downcase-file-names t)) (file-name-directory \"C:/Foo/é\"))",
            "\"c:/foo/\"",
        ),
        ("(string-match \"\\\\`[a-z]:/\" invocation-directory)", "0"),
    ]);
}
