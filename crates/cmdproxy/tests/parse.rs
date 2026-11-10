use cmdproxy::{
    Switches, atoi, batch_file_p, canon_filename, get_next_token, parse_switches,
    run_command_dot_com, same_program_name, shell_command_line, truncate_message,
    try_dequote_cmdline,
};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn parse(list: &[&str], envsize: i32) -> (Result<Switches, String>, Vec<String>) {
    let mut warnings = Vec::new();
    let result = parse_switches(&args(list), envsize, &mut |w| warnings.push(w.to_string()));
    (result, warnings)
}

#[test]
fn tokens() {
    for (src, token, rest) in [
        ("  foo bar", "foo", " bar"),
        ("\ta\tb", "a", "\tb"),
        ("\"a b\" c", "a b", " c"),
        ("\"a\\\"b\"", "a\"b", ""),
        ("\"a\\\\\"b", "a\\", "b"),
        ("\"a\\\\\\\"b\"", "a\\\"b", ""),
        ("\"a\\b\"", "a\\b", ""),
        ("\"a\\\\b\" x", "a\\\\b", " x"),
        ("\"abc", "abc", ""),
        ("\"ab\\\\\\", "ab\\", ""),
        ("\"ab\\\\\\\\", "ab\\\\", ""),
        ("", "", ""),
        ("   ", "", ""),
        ("\"\" x", "", " x"),
        ("C:/x/y.exe -q", "C:/x/y.exe", " -q"),
    ] {
        assert_eq!(get_next_token(src), (token.to_string(), rest), "{src:?}");
    }
}

#[test]
fn file_names() {
    assert_eq!(canon_filename("C:/a/b\\c"), "C:\\a\\b\\c");
    for (name, batch) in [
        ("x.BAT", true),
        ("C:\\d\\x.cmd", true),
        ("x.exe", false),
        ("C:\\dir.bat\\x", false),
        ("x", false),
        ("x.bat.exe", false),
    ] {
        assert_eq!(batch_file_p(name), batch, "{name}");
    }
    for (path, modname, same) in [
        ("C:\\a\\cmdproxy.exe", "C:\\b\\CMDPROXY.EXE", true),
        ("cmdproxy", "C:\\x\\cmdproxy.exe", true),
        ("D:cmdproxy.EXE", "c:\\cmdproxy.exe", true),
        ("C:\\x\\cmd.exe", "C:\\x\\cmdproxy.exe", false),
        ("cmdproxy.com", "C:\\x\\cmdproxy.exe", false),
        ("C:\\x\\cmdproxy.exe.exe", "C:\\x\\cmdproxy.exe", false),
    ] {
        assert_eq!(same_program_name(path, modname), same, "{path} {modname}");
    }
    assert!(!run_command_dot_com("C:\\DOS\\COMMAND.COM"));
    assert!(!run_command_dot_com("C:\\Windows\\System32\\cmd.exe"));
}

#[test]
fn dequoting() {
    for (line, dequoted) in [
        ("echo hi", Some("echo hi")),
        ("a ^& b", Some("a & b")),
        ("a ^^ b", Some("a ^ b")),
        ("a^", Some("a")),
        ("\"a & b\"", Some("\"a & b\"")),
        ("\"a ^ b\"", Some("\"a ^ b\"")),
        ("\"a\" \"b|c\"", Some("\"a\" \"b|c\"")),
        ("a & b", None),
        ("a | b", None),
        ("a < b", None),
        ("a > b", None),
        ("a (b)", None),
        ("x %y%", None),
        ("x !y", None),
        ("\"%PATH%\"", None),
        ("\"a!\"", None),
        ("\"a&b\"", Some("\"a&b\"")),
        ("^\"a&b\"", None),
        ("", Some("")),
    ] {
        assert_eq!(try_dequote_cmdline(line).as_deref(), dequoted, "{line:?}");
    }
}

#[test]
fn numbers() {
    for (s, n) in [
        ("4096", 4096),
        ("  12x", 12),
        ("-5", -5),
        ("+7", 7),
        ("x", 0),
        ("", 0),
        ("99999999999", i32::MAX),
        ("-99999999999", i32::MIN),
    ] {
        assert_eq!(atoi(s), n, "{s:?}");
    }
}

#[test]
fn switches() {
    let switches = |cmdline: Option<&str>, pass: &[&str], envsize| Switches {
        cmdline: cmdline.map(str::to_string),
        pass_through_args: args(pass),
        envsize,
    };
    let none: Vec<String> = Vec::new();
    assert_eq!(
        parse(&["prog", "-c", "echo hi"], 1000),
        (Ok(switches(Some("echo hi"), &[], 1000)), none.clone())
    );
    assert_eq!(
        parse(&["prog", "/C", "x"], 1000),
        (Ok(switches(Some("x"), &[], 1000)), none.clone())
    );
    assert_eq!(
        parse(&["prog", "-c"], 1000),
        (
            Err("error: expecting arg for -c\n".to_string()),
            none.clone()
        )
    );
    assert_eq!(
        parse(&["prog", "-i", "/c"], 1000),
        (
            Err("error: expecting arg for /c\n".to_string()),
            none.clone()
        )
    );
    assert_eq!(
        parse(&["prog", "-c", "x", "/I"], 1000),
        (
            Ok(switches(Some("x"), &[], 1000)),
            args(&["warning: /I ignored because of -c\n"])
        )
    );
    assert_eq!(
        parse(&["prog", "-i", "-c", "x"], 1000),
        (Ok(switches(Some("x"), &[], 1000)), none.clone())
    );
    assert_eq!(
        parse(&["prog", "-e:5000", "-c", "x"], 1000),
        (Ok(switches(Some("x"), &[], 5000)), none.clone())
    );
    assert_eq!(
        parse(&["prog", "-e:100"], 1000),
        (Ok(switches(None, &[], 1000)), none.clone())
    );
    assert_eq!(
        parse(&["prog", "/E:40000"], 1000),
        (Ok(switches(None, &[], 32768)), none.clone())
    );
    assert_eq!(
        parse(&["prog", "-e:1"], 40000),
        (Ok(switches(None, &[], 32768)), none.clone())
    );
    assert_eq!(
        parse(&["prog", "/q", "-x", "-ci", "-e", "-c", "y"], 1000),
        (
            Ok(switches(Some("y"), &["/q", "-x", "-ci", "-e"], 1000)),
            none.clone()
        )
    );
    assert_eq!(
        parse(&["prog", "-", "foo"], 1000),
        (
            Ok(switches(None, &[], 1000)),
            args(&["warning: extra args ignored after 'prog'\n"])
        )
    );
    assert_eq!(
        parse(&["prog", "-c", "x", "foo", "bar"], 1000),
        (
            Ok(switches(Some("x"), &[], 1000)),
            args(&["warning: extra args ignored after 'x'\n"])
        )
    );
    assert_eq!(
        parse(&["prog", "-c", "x", "-c", "y"], 1000),
        (Ok(switches(Some("y"), &[], 1000)), none.clone())
    );
    assert_eq!(
        parse(&["prog", "", "-c", "x"], 1000),
        (
            Ok(switches(None, &[], 1000)),
            args(&["warning: extra args ignored after 'prog'\n"])
        )
    );
    assert_eq!(
        parse(&["prog"], 1000),
        (Ok(switches(None, &[], 1000)), none.clone())
    );
    assert_eq!(parse(&[], 1000), (Ok(switches(None, &[], 1000)), none));
}

#[test]
fn shell_command_lines() {
    let cmd = "C:\\W\\cmd.exe";
    let dos = "C:\\D\\command.com";
    for (progname, pass, cmdline, dot_com, dir, line) in [
        (
            cmd,
            &[][..],
            Some("echo hi"),
            false,
            "",
            "\"C:\\W\\cmd.exe\" /c \"echo hi\"",
        ),
        (
            cmd,
            &["/q", "/d"][..],
            Some("a \"b\" c"),
            false,
            "",
            "\"C:\\W\\cmd.exe\" /q /d /c \"a \"b\" c\"",
        ),
        (
            dos,
            &[][..],
            Some("echo hi"),
            true,
            "",
            "\"C:\\D\\command.com\" /e:1000 /c \"echo hi\"",
        ),
        (cmd, &[][..], None, false, "", "\"C:\\W\\cmd.exe\" "),
        (cmd, &["/k"][..], None, false, "", "\"C:\\W\\cmd.exe\"  /k"),
        (
            dos,
            &["/y"][..],
            None,
            true,
            "C:\\D\\",
            "\"C:\\D\\command.com\" C:\\D\\ /y /e:1000",
        ),
    ] {
        let pass = args(pass);
        assert_eq!(
            shell_command_line(progname, &pass, cmdline, dot_com, 1000, dir),
            line
        );
    }
}

#[test]
fn messages() {
    let long = "x".repeat(2000);
    assert_eq!(truncate_message(&long).len(), 1023);
    assert_eq!(truncate_message("short\n"), "short\n");
    let wide = "é".repeat(600);
    let cut = truncate_message(&wide);
    assert!(cut.len() <= 1023 && cut.len() >= 1022, "{}", cut.len());
}
