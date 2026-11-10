#[cfg(windows)]
mod w32;

#[cfg(windows)]
pub use w32::run;

const ESCAPE_CHAR: char = '\\';

const MESSAGE_MAX: usize = 1023;

fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')
}

pub fn canon_filename(fname: &str) -> String {
    fname.replace('/', "\\")
}

fn skip_space(s: &str) -> &str {
    s.trim_start_matches(is_space)
}

pub fn get_next_token(src: &str) -> (String, &str) {
    let mut p = skip_space(src);
    let mut o = String::new();
    if let Some(quoted) = p.strip_prefix('"') {
        let mut escape_char_run = 0usize;
        p = quoted;
        loop {
            let mut chars = p.chars();
            match chars.next() {
                Some(c) if c == ESCAPE_CHAR && ESCAPE_CHAR != '"' => {
                    escape_char_run += 1;
                    p = chars.as_str();
                }
                Some('"') => {
                    while escape_char_run > 1 {
                        o.push(ESCAPE_CHAR);
                        escape_char_run -= 2;
                    }
                    if escape_char_run > 0 {
                        o.push('"');
                        p = chars.as_str();
                        escape_char_run = 0;
                    } else if chars.as_str().starts_with(ESCAPE_CHAR) && ESCAPE_CHAR == '"' {
                        o.push('"');
                        p = &chars.as_str()[ESCAPE_CHAR.len_utf8()..];
                    } else {
                        p = chars.as_str();
                        break;
                    }
                }
                None => {
                    if ESCAPE_CHAR == '\\' {
                        while escape_char_run > 1 {
                            o.push(ESCAPE_CHAR);
                            escape_char_run -= 2;
                        }
                    }
                    break;
                }
                Some(c) => {
                    if ESCAPE_CHAR == '\\' {
                        while escape_char_run > 0 {
                            o.push(ESCAPE_CHAR);
                            escape_char_run -= 1;
                        }
                    }
                    o.push(c);
                    p = chars.as_str();
                }
            }
        }
    } else {
        let end = p.find(is_space).unwrap_or(p.len());
        o.push_str(&p[..end]);
        p = &p[end..];
    }
    (o, p)
}

pub fn batch_file_p(progname: &str) -> bool {
    progname.rfind('.').is_some_and(|i| {
        [".bat", ".cmd"]
            .iter()
            .any(|ext| progname[i..].eq_ignore_ascii_case(ext))
    })
}

pub fn try_dequote_cmdline(cmdline: &str) -> Option<String> {
    enum State {
        Normal,
        AfterCaret,
        InsideQuote,
    }
    let mut new_cmdline = String::with_capacity(cmdline.len());
    let mut state = State::Normal;
    for c in cmdline.chars() {
        match state {
            State::Normal => match c {
                '"' => {
                    new_cmdline.push(c);
                    state = State::InsideQuote;
                }
                '^' => state = State::AfterCaret,
                '<' | '>' | '&' | '|' | '(' | ')' | '%' | '!' => return None,
                _ => new_cmdline.push(c),
            },
            State::AfterCaret => {
                new_cmdline.push(c);
                state = State::Normal;
            }
            State::InsideQuote => match c {
                '"' => {
                    new_cmdline.push(c);
                    state = State::Normal;
                }
                '%' | '!' => return None,
                _ => new_cmdline.push(c),
            },
        }
    }
    Some(new_cmdline)
}

pub fn atoi(s: &str) -> i32 {
    let s = skip_space(s);
    let (negative, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let mut value: i64 = 0;
    for d in digits.bytes().take_while(u8::is_ascii_digit) {
        value = (value * 10 + i64::from(d - b'0')).min(i64::from(i32::MAX) + 1);
    }
    let value = if negative { -value } else { value };
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switches {
    pub cmdline: Option<String>,
    pub pass_through_args: Vec<String>,
    pub envsize: i32,
}

pub fn parse_switches(
    args: &[String],
    envsize: i32,
    warn: &mut dyn FnMut(&str),
) -> Result<Switches, String> {
    let mut switches = Switches {
        cmdline: None,
        pass_through_args: Vec::new(),
        envsize,
    };
    let mut argc = args.len() as isize;
    let mut argv = 0;
    loop {
        argc -= 1;
        if argc <= 0 {
            break;
        }
        argv += 1;
        let arg = &args[argv];
        let b = arg.as_bytes();
        if matches!(b.first(), Some(b'-' | b'/')) && b.len() > 1 {
            if b[1].eq_ignore_ascii_case(&b'c') && b.len() == 2 {
                argc -= 1;
                if argc == 0 {
                    return Err(format!("error: expecting arg for {arg}\n"));
                }
                argv += 1;
                switches.cmdline = Some(args[argv].clone());
            } else if b[1].eq_ignore_ascii_case(&b'i') && b.len() == 2 {
                if switches.cmdline.is_some() {
                    warn(&format!("warning: {arg} ignored because of -c\n"));
                }
            } else if b[1].eq_ignore_ascii_case(&b'e') && b.get(2) == Some(&b':') {
                let requested_envsize = atoi(&arg[3..]);
                if requested_envsize > switches.envsize {
                    switches.envsize = requested_envsize;
                }
                if switches.envsize > 32768 {
                    switches.envsize = 32768;
                }
            } else {
                switches.pass_through_args.push(arg.clone());
            }
        } else {
            break;
        }
    }
    if argc > 0 {
        warn(&format!(
            "warning: extra args ignored after '{}'\n",
            args[argv - 1]
        ));
    }
    Ok(switches)
}

fn program_stem(name: &str) -> &str {
    let p = match name.rfind('\\') {
        Some(i) => &name[i + 1..],
        None => match name.find(':') {
            Some(i) => &name[i + 1..],
            None => name,
        },
    };
    match p.rfind('.') {
        Some(i) if p[i..].eq_ignore_ascii_case(".exe") => &p[..i],
        _ => p,
    }
}

pub fn same_program_name(path: &str, modname: &str) -> bool {
    program_stem(path).eq_ignore_ascii_case(program_stem(modname))
}

pub fn run_command_dot_com(progname: &str) -> bool {
    progname
        .rfind('\\')
        .is_some_and(|i| progname[i..].eq_ignore_ascii_case("command.com"))
}

pub fn shell_command_line(
    progname: &str,
    pass_through_args: &[String],
    cmdline: Option<&str>,
    run_command_dot_com: bool,
    envsize: i32,
    command_dir: &str,
) -> String {
    let mut buf = match cmdline {
        Some(_) => format!("\"{progname}\""),
        None => {
            let path = if run_command_dot_com { command_dir } else { "" };
            format!("\"{progname}\" {path}")
        }
    };
    for arg in pass_through_args {
        buf.push(' ');
        buf.push_str(arg);
    }
    match (cmdline, run_command_dot_com) {
        (Some(cmdline), true) => buf.push_str(&format!(" /e:{envsize} /c \"{cmdline}\"")),
        (Some(cmdline), false) => buf.push_str(&format!(" /c \"{cmdline}\"")),
        (None, true) => buf.push_str(&format!(" /e:{envsize}")),
        (None, false) => {}
    }
    buf
}

pub fn truncate_message(msg: &str) -> &str {
    &msg[..msg.floor_char_boundary(MESSAGE_MAX)]
}
