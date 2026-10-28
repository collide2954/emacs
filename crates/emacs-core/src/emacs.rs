use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::LispResult;
use crate::fileio::{
    f_expand_file_name, f_file_name_as_directory, f_file_name_directory, f_file_name_nondirectory,
    f_find_file_name_handler, normalize_slashes,
};
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const PACKAGE_NAME: &str = "GNU Emacs";
pub const PACKAGE_BUGREPORT: &str = "bug-gnu-emacs@gnu.org";
pub const EMACS_VERSION: &str = "31.0.50";
pub const EMACS_COPYRIGHT: &str = "Copyright (C) 2026 Free Software Foundation, Inc.";

struct StandardArg {
    name: &'static str,
    longname: Option<&'static str>,
    priority: i32,
    nargs: i32,
}

const fn sa(
    name: &'static str,
    longname: Option<&'static str>,
    priority: i32,
    nargs: i32,
) -> StandardArg {
    StandardArg {
        name,
        longname,
        priority,
        nargs,
    }
}

const STANDARD_ARGS: &[StandardArg] = &[
    sa("-version", Some("--version"), 150, 0),
    sa("-chdir", Some("--chdir"), 130, 1),
    sa("-t", Some("--terminal"), 120, 1),
    sa("-nw", Some("--no-window-system"), 110, 0),
    sa("-nw", Some("--no-windows"), 110, 0),
    sa("-batch", Some("--batch"), 100, 0),
    sa("-script", Some("--script"), 100, 1),
    sa("-daemon", Some("--daemon"), 99, 0),
    sa("-bg-daemon", Some("--bg-daemon"), 99, 0),
    sa("-fg-daemon", Some("--fg-daemon"), 99, 0),
    sa("-help", Some("--help"), 90, 0),
    sa("-nl", Some("--no-loadup"), 70, 0),
    sa("-nsl", Some("--no-site-lisp"), 65, 0),
    sa("-no-build-details", Some("--no-build-details"), 63, 0),
    sa("-d", Some("--display"), 60, 1),
    sa("-display", None, 60, 1),
    sa("-Q", Some("--quick"), 55, 0),
    sa("-quick", None, 55, 0),
    sa("-x", None, 55, 0),
    sa("-q", Some("--no-init-file"), 50, 0),
    sa("-no-init-file", None, 50, 0),
    sa("-init-directory", Some("--init-directory"), 30, 1),
    sa("-no-x-resources", Some("--no-x-resources"), 40, 0),
    sa("-no-site-file", Some("--no-site-file"), 40, 0),
    sa("-no-comp-spawn", Some("--no-comp-spawn"), 60, 0),
    sa("-u", Some("--user"), 30, 1),
    sa("-user", None, 30, 1),
    sa("-debug-init", Some("--debug-init"), 20, 0),
    sa("-iconic", Some("--iconic"), 15, 0),
    sa("-D", Some("--basic-display"), 12, 0),
    sa("-basic-display", None, 12, 0),
    sa("-nbc", Some("--no-blinking-cursor"), 12, 0),
    sa("-nbi", Some("--no-bitmap-icon"), 10, 0),
    sa("-bg", Some("--background-color"), 10, 1),
    sa("-background", None, 10, 1),
    sa("-fg", Some("--foreground-color"), 10, 1),
    sa("-foreground", None, 10, 1),
    sa("-bd", Some("--border-color"), 10, 1),
    sa("-bw", Some("--border-width"), 10, 1),
    sa("-ib", Some("--internal-border"), 10, 1),
    sa("-ms", Some("--mouse-color"), 10, 1),
    sa("-cr", Some("--cursor-color"), 10, 1),
    sa("-fn", Some("--font"), 10, 1),
    sa("-font", None, 10, 1),
    sa("-fs", Some("--fullscreen"), 10, 0),
    sa("-fw", Some("--fullwidth"), 10, 0),
    sa("-fh", Some("--fullheight"), 10, 0),
    sa("-mm", Some("--maximized"), 10, 0),
    sa("-g", Some("--geometry"), 10, 1),
    sa("-geometry", None, 10, 1),
    sa("-T", Some("--title"), 10, 1),
    sa("-title", None, 10, 1),
    sa("-name", Some("--name"), 10, 1),
    sa("-xrm", Some("--xrm"), 10, 1),
    sa("-parent-id", Some("--parent-id"), 10, 1),
    sa("-r", Some("--reverse-video"), 5, 0),
    sa("-rv", None, 5, 0),
    sa("-reverse", None, 5, 0),
    sa("-hb", Some("--horizontal-scroll-bars"), 5, 0),
    sa("-vb", Some("--vertical-scroll-bars"), 5, 0),
    sa("-color", Some("--color"), 5, 0),
    sa("-no-splash", Some("--no-splash"), 3, 0),
    sa("-no-desktop", Some("--no-desktop"), 3, 0),
    sa("-temacs", Some("--temacs"), 1, 1),
    sa("-L", Some("--directory"), 0, 1),
    sa("-directory", None, 0, 1),
    sa("-l", Some("--load"), 0, 1),
    sa("-load", None, 0, 1),
    sa("-scriptload", None, 0, 1),
    sa("-f", Some("--funcall"), 0, 1),
    sa("-funcall", None, 0, 1),
    sa("-eval", Some("--eval"), 0, 1),
    sa("-execute", Some("--execute"), 0, 1),
    sa("-find-file", Some("--find-file"), 0, 1),
    sa("-visit", Some("--visit"), 0, 1),
    sa("-file", Some("--file"), 0, 1),
    sa("-insert", Some("--insert"), 0, 1),
    sa("-kill", Some("--kill"), -10, 0),
];

const USAGE_MESSAGE: &[&str] = &[
    "\n\
Run Emacs, the extensible, customizable, self-documenting real-time\n\
display editor.  The recommended way to start Emacs for normal editing\n\
is with no options at all.\n\
\n\
Run M-x info RET m emacs RET m emacs invocation RET inside Emacs to\n\
read the main documentation for these command-line arguments.\n\
\n\
Initialization options:\n\
\n",
    "--batch                     do not do interactive display; implies -q\n\
--chdir DIR                 change to directory DIR\n\
--daemon, --bg-daemon[=NAME] start a (named) server in the background\n\
--fg-daemon[=NAME]          start a (named) server in the foreground\n\
--debug-init                enable Emacs Lisp debugger for init file\n\
--display, -d DISPLAY       use X server DISPLAY\n",
    "--no-build-details          do not add build details such as time stamps\n\
--no-desktop                do not load a saved desktop\n\
--no-init-file, -q          load neither ~/.emacs nor default.el\n\
--no-loadup, -nl            do not load loadup.el into bare Emacs\n\
--no-site-file              do not load site-start.el\n\
--no-x-resources            do not load X resources\n\
--no-site-lisp, -nsl        do not add site-lisp directories to load-path\n\
--no-splash                 do not display a splash screen on startup\n\
--no-window-system, -nw     do not communicate with X, ignoring $DISPLAY\n\
--init-directory=DIR        use DIR when looking for the Emacs init files.\n",
    "--quick, -Q                 equivalent to:\n\
\x20                             -q --no-site-file --no-site-lisp --no-splash\n\
\x20                             --no-x-resources\n\
--script FILE               run FILE as an Emacs Lisp script\n\
-x                          to be used in #!/usr/bin/emacs -x\n\
\x20                             and has approximately the same meaning\n\
\t\t\t      as -Q --script\n\
--terminal, -t DEVICE       use DEVICE for terminal I/O\n\
--user, -u USER             load ~USER/.emacs instead of your own\n\
\n",
    "Action options:\n\
\n\
FILE                    visit FILE\n\
+LINE                   go to line LINE in next FILE\n\
+LINE:COLUMN            go to line LINE, column COLUMN, in next FILE\n\
--directory, -L DIR     prepend DIR to load-path (with :DIR, append DIR)\n\
--eval EXPR             evaluate Emacs Lisp expression EXPR\n\
--execute EXPR          evaluate Emacs Lisp expression EXPR\n",
    "--file FILE             visit FILE\n\
--find-file FILE        visit FILE\n\
--funcall, -f FUNC      call Emacs Lisp function FUNC with no arguments\n\
--insert FILE           insert contents of FILE into current buffer\n\
--kill                  exit without asking for confirmation\n\
--load, -l FILE         load Emacs Lisp FILE using the load function\n\
--visit FILE            visit FILE\n\
\n",
    "Display options:\n\
\n\
--background-color, -bg COLOR   window background color\n\
--basic-display, -D             disable many display features;\n\
\x20                                 used for debugging Emacs\n\
--border-color, -bd COLOR       main border color\n\
--border-width, -bw WIDTH       width of main border\n",
    "--color, --color=MODE           override color mode for character terminals;\n\
\x20                                 MODE defaults to `auto', and\n\
\x20                                 can also be `never', `always',\n\
\x20                                 or a mode name like `ansi8'\n\
--cursor-color, -cr COLOR       color of the Emacs cursor indicating point\n\
--font, -fn FONT                default font; must be fixed-width\n\
--foreground-color, -fg COLOR   window foreground color\n",
    "--fullheight, -fh               make the first frame high as the screen\n\
--fullscreen, -fs               make the first frame fullscreen\n\
--fullwidth, -fw                make the first frame wide as the screen\n\
--maximized, -mm                make the first frame maximized\n\
--geometry, -g GEOMETRY         window geometry\n",
    "--no-bitmap-icon, -nbi          do not use picture of gnu for Emacs icon\n\
--iconic                        start Emacs in iconified state\n\
--internal-border, -ib WIDTH    width between text and main border\n\
--line-spacing, -lsp PIXELS     additional space to put between lines\n\
--mouse-color, -ms COLOR        mouse cursor color in Emacs window\n\
--name NAME                     title for initial Emacs frame\n",
    "--no-blinking-cursor, -nbc      disable blinking cursor\n\
--reverse-video, -r, -rv        switch foreground and background\n\
--title, -T TITLE               title for initial Emacs frame\n\
--vertical-scroll-bars, -vb     enable vertical scroll bars\n\
--xrm XRESOURCES                set additional X resources\n\
--parent-id XID                 set parent window\n\
--help                          display this help and exit\n\
--version                       output version information and exit\n\
\n",
    "You can generally also specify long option names with a single -; for\n\
example, -batch as well as --batch.  You can use any unambiguous\n\
abbreviation for a --option.\n\
\n\
Various environment variables and window system resources also affect\n\
the operation of Emacs.  See the main documentation.\n\
\n\
Report bugs to bug-gnu-emacs@gnu.org.  First, please see the Bugs\n\
section of the Emacs manual or the file BUGS.\n",
];

pub type Argv = Vec<Option<Vec<u8>>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exit {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

fn fatal(message: &str) -> Exit {
    let mut stderr = format!("emacs: {message}");
    if !message.ends_with('\n') {
        stderr.push('\n');
    }
    Exit {
        code: 1,
        stdout: String::new(),
        stderr,
    }
}

#[derive(Clone, Debug)]
pub struct Startup {
    pub argv: Argv,
    pub argc: usize,
    pub skip_args: isize,
    pub original_pwd: Option<Vec<u8>>,
    pub only_version: bool,
    pub noninteractive: bool,
    pub noninteractive1: bool,
    pub inhibit_window_system: bool,
    pub no_loadup: bool,
    pub no_site_lisp: bool,
    pub build_details: bool,
}

impl Startup {
    pub fn program(noninteractive: bool) -> Startup {
        let argv0 = os_bytes(std::env::current_exe().unwrap_or_default().as_os_str());
        Startup {
            argv: vec![Some(argv0)],
            argc: 1,
            skip_args: 0,
            original_pwd: None,
            only_version: false,
            noninteractive,
            noninteractive1: noninteractive,
            inhibit_window_system: false,
            no_loadup: false,
            no_site_lisp: false,
            build_details: true,
        }
    }

    pub fn load_file(&mut self) -> Option<Vec<u8>> {
        argmatch(
            &self.argv,
            self.argc,
            "-l",
            Some("--load"),
            3,
            true,
            &mut self.skip_args,
        )
        .flatten()
    }
}

pub fn os_bytes(s: &std::ffi::OsStr) -> Vec<u8> {
    #[cfg(unix)]
    let bytes = std::os::unix::ffi::OsStrExt::as_bytes(s).to_vec();
    #[cfg(not(unix))]
    let bytes = s.to_string_lossy().into_owned().into_bytes();
    bytes
}

fn arg_at(argv: &Argv, i: isize) -> Option<&Vec<u8>> {
    if i < 0 {
        return None;
    }
    argv.get(i as usize).and_then(Option::as_ref)
}

pub fn argmatch(
    argv: &Argv,
    argc: usize,
    sstr: &str,
    lstr: Option<&str>,
    minlen: usize,
    valptr: bool,
    skipptr: &mut isize,
) -> Option<Option<Vec<u8>>> {
    if argc as isize <= *skipptr + 1 {
        return None;
    }
    let arg = arg_at(argv, *skipptr + 1)?.clone();
    if arg == sstr.as_bytes() {
        if valptr {
            let value = arg_at(argv, *skipptr + 2).cloned();
            *skipptr += 2;
            return Some(value);
        }
        *skipptr += 1;
        return Some(None);
    }
    let p = if valptr {
        arg.iter().position(|&c| c == b'=')
    } else {
        None
    };
    let arglen = p.unwrap_or(arg.len());
    let lstr = lstr?;
    if arglen < minlen || !strncmp_c(&arg, lstr.as_bytes(), arglen) {
        None
    } else if !valptr {
        *skipptr += 1;
        Some(None)
    } else if let Some(p) = p {
        *skipptr += 1;
        Some(Some(arg[p + 1..].to_vec()))
    } else if let Some(value) = arg_at(argv, *skipptr + 2).cloned() {
        *skipptr += 2;
        Some(Some(value))
    } else {
        None
    }
}

pub fn sort_args(argc: usize, argv: &mut Argv) -> Result<(), Exit> {
    let mut new: Argv = vec![None; argc];
    let mut options = vec![-1i32; argc];
    let mut priority = vec![0i32; argc];
    let mut stderr = String::new();
    let mut from = 1;
    while from < argc {
        options[from] = -1;
        priority[from] = 0;
        let Some(arg) = argv[from].clone() else {
            from += 1;
            continue;
        };
        if arg.first() == Some(&b'-') {
            if arg.get(1) == Some(&b'-') && arg.len() == 2 {
                while from < argc {
                    priority[from] = -100;
                    options[from] = -1;
                    from += 1;
                }
                break;
            }
            if let Some(s) = STANDARD_ARGS.iter().find(|s| arg == s.name.as_bytes()) {
                options[from] = s.nargs;
                priority[from] = s.priority;
                if from + s.nargs as usize >= argc {
                    return Err(fatal(&format!(
                        "Option '{}' requires an argument\n",
                        String::from_utf8_lossy(&arg)
                    )));
                }
                from += s.nargs as usize + 1;
                continue;
            }
            if arg.get(1) == Some(&b'-') {
                let equals = arg.iter().position(|&c| c == b'=');
                let thislen = equals.unwrap_or(arg.len());
                let mut matched: isize = -1;
                for (i, s) in STANDARD_ARGS.iter().enumerate() {
                    if let Some(longname) = s.longname
                        && strncmp_c(&arg, longname.as_bytes(), thislen)
                    {
                        matched = if matched == -1 { i as isize } else { -2 };
                    }
                }
                if matched >= 0 {
                    let s = &STANDARD_ARGS[matched as usize];
                    options[from] = s.nargs;
                    priority[from] = s.priority;
                    if equals.is_some() {
                        options[from] = 0;
                    }
                    if from + options[from] as usize >= argc {
                        return Err(fatal(&format!(
                            "Option '{}' requires an argument\n",
                            String::from_utf8_lossy(&arg)
                        )));
                    }
                    from += options[from] as usize;
                } else if matched == -2 {
                    stderr.push_str(&format!(
                        "Option '{}' matched multiple standard arguments\n",
                        String::from_utf8_lossy(&arg)
                    ));
                }
            }
        }
        from += 1;
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    new[0] = argv[0].clone();
    let mut to = 1;
    let mut incoming_used = 1;
    while incoming_used < argc {
        let mut best: isize = -1;
        let mut best_priority = -9999;
        let mut from = 1;
        while from < argc {
            if argv[from].is_some() && priority[from] > best_priority {
                best_priority = priority[from];
                best = from as isize;
            }
            if options[from] > 0 {
                from += options[from] as usize;
            }
            from += 1;
        }
        if best < 0 {
            unreachable!();
        }
        let best = best as usize;
        if !(options[best] == 0 && new[to - 1] == argv[best]) {
            new[to] = argv[best].clone();
            to += 1;
            for i in 0..options[best].max(0) as usize {
                new[to] = argv[best + i + 1].clone();
                to += 1;
            }
        }
        incoming_used += 1 + options[best].max(0) as usize;
        argv[best] = None;
        for i in 0..options[best].max(0) as usize {
            argv[best + i + 1] = None;
        }
    }
    while to < argc {
        new[to] = None;
        to += 1;
    }
    argv[..argc].clone_from_slice(&new);
    Ok(())
}

fn strncmp_c(a: &[u8], b: &[u8], n: usize) -> bool {
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
    }
    true
}

fn strerror(err: &std::io::Error) -> String {
    match err.kind() {
        std::io::ErrorKind::NotFound => "No such file or directory".to_string(),
        std::io::ErrorKind::PermissionDenied => "Permission denied".to_string(),
        std::io::ErrorKind::NotADirectory => "Not a directory".to_string(),
        _ => err.to_string(),
    }
}

pub fn usage_text(argv0: &[u8]) -> String {
    let mut out = format!(
        "Usage: {} [OPTION-OR-FILENAME]...\n",
        String::from_utf8_lossy(argv0)
    );
    for part in USAGE_MESSAGE {
        out.push_str(part);
    }
    out
}

pub fn version_text() -> String {
    format!(
        "{PACKAGE_NAME} {EMACS_VERSION}\n{EMACS_COPYRIGHT}\n{PACKAGE_NAME} comes with ABSOLUTELY NO WARRANTY.\nYou may redistribute copies of {PACKAGE_NAME}\nunder the terms of the GNU General Public License.\nFor more information about these matters, see the file named COPYING.\n"
    )
}

pub fn decode_command_line(args: Vec<Vec<u8>>) -> Result<Startup, Exit> {
    let mut argc = args.len();
    let mut argv: Argv = args.into_iter().map(Some).collect();
    let argv0 = argv[0].clone().unwrap_or_default();
    let mut skip_args: isize = 0;
    let mut temacs = None;
    while skip_args < argc as isize - 1 {
        if let Some(v) = argmatch(
            &argv,
            argc,
            "-temacs",
            Some("--temacs"),
            8,
            true,
            &mut skip_args,
        ) {
            temacs = v;
            break;
        }
        if argmatch(&argv, argc, "--", None, 2, false, &mut skip_args).is_some() {
            break;
        }
        skip_args += 1;
    }
    if let Some(temacs) = temacs {
        return Err(fatal(&format!(
            "Invalid temacs mode '{}'",
            String::from_utf8_lossy(&temacs)
        )));
    }
    sort_args(argc, &mut argv)?;
    let old_argc = argc;
    argc = 0;
    while argc < old_argc && argv[argc].is_some() {
        argc += 1;
    }
    skip_args = 0;
    let only_version = argmatch(
        &argv,
        argc,
        "-version",
        Some("--version"),
        3,
        false,
        &mut skip_args,
    )
    .is_some();
    let emacs_wd = crate::buffer::emacs_wd().ok();
    let mut original_pwd = None;
    if let Some(dir) = argmatch(
        &argv,
        argc,
        "-chdir",
        Some("--chdir"),
        4,
        true,
        &mut skip_args,
    ) && !only_version
    {
        let failure = |shown: &str, err: String| Exit {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "{}: Can't chdir to {}: {}\n",
                String::from_utf8_lossy(&argv0),
                shown,
                err
            ),
        };
        let Some(dir) = dir else {
            return Err(failure("(null)", "Bad address".to_string()));
        };
        let shown = String::from_utf8_lossy(&dir).into_owned();
        if let Err(e) = std::env::set_current_dir(&shown) {
            return Err(failure(&shown, strerror(&e)));
        }
        original_pwd = emacs_wd;
    }
    let mut inhibit_window_system = false;
    if !only_version
        && let Some(term) = argmatch(
            &argv,
            argc,
            "-t",
            Some("--terminal"),
            4,
            true,
            &mut skip_args,
        )
    {
        return Err(Exit {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "{}: {}: text terminals are not supported\n",
                String::from_utf8_lossy(&argv0),
                String::from_utf8_lossy(&term.unwrap_or_default())
            ),
        });
    }
    if argmatch(
        &argv,
        argc,
        "-nw",
        Some("--no-window-system"),
        6,
        false,
        &mut skip_args,
    )
    .is_some()
        || argmatch(
            &argv,
            argc,
            "-nw",
            Some("--no-windows"),
            6,
            false,
            &mut skip_args,
        )
        .is_some()
    {
        inhibit_window_system = true;
    }
    let mut noninteractive = argmatch(
        &argv,
        argc,
        "-batch",
        Some("--batch"),
        5,
        false,
        &mut skip_args,
    )
    .is_some()
        || only_version;
    if argmatch(
        &argv,
        argc,
        "-script",
        Some("--script"),
        3,
        true,
        &mut skip_args,
    )
    .is_some()
    {
        noninteractive = true;
        argv[(skip_args - 1) as usize] = Some(b"-scriptload".to_vec());
        skip_args -= 2;
        sort_args(argc, &mut argv)?;
    }
    if argmatch(
        &argv,
        argc,
        "-help",
        Some("--help"),
        3,
        false,
        &mut skip_args,
    )
    .is_some()
        && !only_version
    {
        return Err(Exit {
            code: 0,
            stdout: usage_text(&argv0),
            stderr: String::new(),
        });
    }
    let mut daemon_type = 0;
    if !only_version {
        if argmatch(
            &argv,
            argc,
            "-fg-daemon",
            Some("--fg-daemon"),
            10,
            false,
            &mut skip_args,
        )
        .is_some()
            || argmatch(
                &argv,
                argc,
                "-fg-daemon",
                Some("--fg-daemon"),
                10,
                true,
                &mut skip_args,
            )
            .is_some()
        {
            daemon_type = 1;
        } else if argmatch(
            &argv,
            argc,
            "-daemon",
            Some("--daemon"),
            5,
            false,
            &mut skip_args,
        )
        .is_some()
            || argmatch(
                &argv,
                argc,
                "-daemon",
                Some("--daemon"),
                5,
                true,
                &mut skip_args,
            )
            .is_some()
            || argmatch(
                &argv,
                argc,
                "-bg-daemon",
                Some("--bg-daemon"),
                10,
                false,
                &mut skip_args,
            )
            .is_some()
            || argmatch(
                &argv,
                argc,
                "-bg-daemon",
                Some("--bg-daemon"),
                10,
                true,
                &mut skip_args,
            )
            .is_some()
        {
            daemon_type = 2;
        }
    }
    if daemon_type > 0 {
        return Err(Exit {
            code: 1,
            stdout: String::new(),
            stderr: "This platform does not support daemon mode.\n".to_string(),
        });
    }
    let noninteractive1 = noninteractive;
    let no_loadup = argmatch(
        &argv,
        argc,
        "-nl",
        Some("--no-loadup"),
        6,
        false,
        &mut skip_args,
    )
    .is_some();
    let mut no_site_lisp = argmatch(
        &argv,
        argc,
        "-nsl",
        Some("--no-site-lisp"),
        11,
        false,
        &mut skip_args,
    )
    .is_some();
    let build_details = argmatch(
        &argv,
        argc,
        "-no-build-details",
        Some("--no-build-details"),
        7,
        false,
        &mut skip_args,
    )
    .is_none();
    let count_before = skip_args;
    if !no_site_lisp
        && (argmatch(&argv, argc, "-Q", Some("--quick"), 3, false, &mut skip_args).is_some()
            || argmatch(&argv, argc, "-quick", None, 2, false, &mut skip_args).is_some())
    {
        no_site_lisp = true;
    }
    if argmatch(&argv, argc, "-x", None, 1, true, &mut skip_args).is_some() {
        noninteractive = true;
        no_site_lisp = true;
        argv[(skip_args - 1) as usize] = Some(b"-scripteval".to_vec());
        sort_args(argc, &mut argv)?;
    }
    skip_args = count_before;
    Ok(Startup {
        argv,
        argc,
        skip_args,
        original_pwd,
        only_version,
        noninteractive,
        noninteractive1,
        inhibit_window_system,
        no_loadup,
        no_site_lisp,
        build_details,
    })
}

pub fn path_exec() -> String {
    let invocation = std::env::current_exe().unwrap_or_default();
    let dir = invocation
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut dir = normalize_slashes(&dir.to_string_lossy());
    if !dir.ends_with('/') {
        dir.push('/');
    }
    dir
}

impl Lisp {
    fn executable_on_path(&mut self, name: Value) -> LispResult<Option<Value>> {
        let path = self.find_symbol_value(sym::EXEC_PATH);
        let suffixes = self.find_symbol_value(sym::EXEC_SUFFIXES);
        let dirs = self.list_to_vec(path)?;
        let suffixes = self.list_to_vec(suffixes)?;
        let base = self.stack.len();
        self.stack.push(name);
        self.stack.extend_from_slice(&dirs);
        self.stack.extend_from_slice(&suffixes);
        let r = (|| -> LispResult<Option<Value>> {
            for &dir in &dirs {
                for &suffix in &suffixes {
                    let candidate = crate::fns::f_concat(self, &[name, suffix])?;
                    let found = f_expand_file_name(self, &[candidate, dir])?;
                    let text = self.string_to_rust(found);
                    if let Ok(meta) = std::fs::metadata(&text)
                        && !meta.is_dir()
                        && is_executable(&meta)
                    {
                        return Ok(Some(found));
                    }
                }
            }
            Ok(None)
        })();
        self.stack.truncate(base);
        r
    }

    pub fn init_cmdargs(&mut self, startup: &Startup) -> LispResult<()> {
        let base = self.stack.len();
        let r = self.init_cmdargs_1(startup);
        self.stack.truncate(base);
        r
    }

    fn init_cmdargs_1(&mut self, startup: &Startup) -> LispResult<()> {
        let argv0 = if cfg!(windows) {
            os_bytes(std::env::current_exe().unwrap_or_default().as_os_str())
        } else {
            startup.argv[0].clone().unwrap_or_default()
        };
        let mut raw_name = self.unibyte_string(&argv0);
        self.stack.push(raw_name);
        let slash_colon = self.string("/:");
        let handler = f_find_file_name_handler(self, &[raw_name, sym::T])?;
        if !handler.is_nil() {
            raw_name = crate::fns::f_concat(self, &[slash_colon, raw_name])?;
            self.stack.push(raw_name);
        }
        let name = f_file_name_nondirectory(self, &[raw_name])?;
        self.set_plain_value(sym::INVOCATION_NAME, name);
        let dir = f_file_name_directory(self, &[raw_name])?;
        self.set_plain_value(sym::INVOCATION_DIRECTORY, dir);
        if self.plain_value(sym::INVOCATION_DIRECTORY).is_nil() {
            let name = self.plain_value(sym::INVOCATION_NAME);
            if let Some(mut found) = self.executable_on_path(name)? {
                self.stack.push(found);
                let handler = f_find_file_name_handler(self, &[found, sym::T])?;
                if !handler.is_nil() {
                    found = crate::fns::f_concat(self, &[slash_colon, found])?;
                    self.stack.push(found);
                }
                let dir = f_file_name_directory(self, &[found])?;
                self.set_plain_value(sym::INVOCATION_DIRECTORY, dir);
            }
        }
        let dir = self.plain_value(sym::INVOCATION_DIRECTORY);
        if !dir.is_nil() && crate::fileio::f_file_name_absolute_p(self, &[dir])?.is_nil() {
            let odir = match &startup.original_pwd {
                Some(pwd) => self.unibyte_string(pwd),
                None => Value::NIL,
            };
            let dir = f_expand_file_name(self, &[dir, odir])?;
            self.set_plain_value(sym::INVOCATION_DIRECTORY, dir);
        }
        self.set_plain_value(sym::INSTALLATION_DIRECTORY, Value::NIL);
        let mut dir = self.plain_value(sym::INVOCATION_DIRECTORY);
        if !dir.is_nil() {
            if cfg!(windows) && self.string_bytes(dir).ends_with(b"/i386/") {
                if self.find_symbol_value(sym::PURIFY_FLAG).is_nil()
                    && !crate::data::f_fboundp(self, &[sym::FILE_TRUENAME])?.is_nil()
                {
                    dir = self.funcall(sym::FILE_TRUENAME, &[dir])?;
                    self.stack.push(dir);
                }
                let up = self.string("../..");
                dir = f_expand_file_name(self, &[up, dir])?;
                self.stack.push(dir);
            }
            let invocation_name = self.plain_value(sym::INVOCATION_NAME);
            let mut name = f_expand_file_name(self, &[invocation_name, dir])?;
            self.stack.push(name);
            loop {
                let lib_src = self.string("lib-src");
                let tem = f_expand_file_name(self, &[lib_src, dir])?;
                if !crate::fileio::f_file_exists_p(self, &[tem])?.is_nil() {
                    let etc = self.string("etc");
                    let tem = f_expand_file_name(self, &[etc, dir])?;
                    if !crate::fileio::f_file_exists_p(self, &[tem])?.is_nil() {
                        let installation = f_file_name_as_directory(self, &[dir])?;
                        self.set_plain_value(sym::INSTALLATION_DIRECTORY, installation);
                        break;
                    }
                }
                let lib_src = self.string("../lib-src");
                let tem = f_expand_file_name(self, &[lib_src, dir])?;
                if !crate::fileio::f_file_exists_p(self, &[tem])?.is_nil() {
                    let etc = self.string("../etc");
                    let tem = f_expand_file_name(self, &[etc, dir])?;
                    if !crate::fileio::f_file_exists_p(self, &[tem])?.is_nil() {
                        let up = self.string("..");
                        let tem = f_expand_file_name(self, &[up, dir])?;
                        let installation = f_file_name_as_directory(self, &[tem])?;
                        self.set_plain_value(sym::INSTALLATION_DIRECTORY, installation);
                        break;
                    }
                }
                let tem = crate::fileio::f_file_symlink_p(self, &[name])?;
                if tem.is_nil() {
                    break;
                }
                name = f_expand_file_name(self, &[tem, dir])?;
                self.stack.push(name);
                dir = f_file_name_directory(self, &[name])?;
                self.stack.push(dir);
            }
        }
        let mut args = Value::NIL;
        for i in (0..startup.argc).rev() {
            if i == 0 || i as isize > startup.skip_args {
                let bytes = if i == 0 {
                    Some(&argv0)
                } else {
                    startup.argv[i].as_ref()
                };
                if let Some(bytes) = bytes {
                    let s = self.unibyte_string(bytes);
                    args = self.cons(s, args);
                }
            }
        }
        self.set_plain_value(sym::COMMAND_LINE_ARGS, args);
        Ok(())
    }
}

#[cfg(unix)]
fn is_executable(meta: &std::fs::Metadata) -> bool {
    std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(meta: &std::fs::Metadata) -> bool {
    meta.is_file()
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("invocation-name", 0, 0, f_invocation_name),
    Subr::fixed("invocation-directory", 0, 0, f_invocation_directory),
    Subr::fixed("kill-emacs", 0, 2, f_kill_emacs).interactive("P"),
    Subr::fixed("daemonp", 0, 0, f_nil),
    Subr::fixed("daemon-initialized", 0, 0, f_daemon_initialized),
    Subr::many("call-process", 1, f_call_process),
    Subr::many("call-process-region", 3, f_call_process_region),
    Subr::fixed("getenv-internal", 1, 2, f_getenv_internal),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::COMMAND_LINE_ARGS, Value::NIL);
    let system_type = if cfg!(windows) {
        "windows-nt"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "freebsd") {
        "berkeley-unix"
    } else {
        "gnu/linux"
    };
    let st = lisp.intern(system_type);
    lisp.defvar(sym::SYSTEM_TYPE, st);
    let config = format!(
        "{}-{}",
        std::env::consts::ARCH,
        if cfg!(windows) {
            "w64-mingw32"
        } else if cfg!(target_os = "macos") {
            "apple-darwin"
        } else {
            "pc-linux-gnu"
        }
    );
    let sc = lisp.string(&config);
    lisp.defvar(sym::SYSTEM_CONFIGURATION, sc);
    let empty = lisp.string("");
    lisp.defvar(sym::SYSTEM_CONFIGURATION_OPTIONS, empty);
    let features = lisp.string("");
    lisp.defvar(sym::SYSTEM_CONFIGURATION_FEATURES, features);
    lisp.defvar(sym::KILL_EMACS_HOOK, Value::NIL);
    let sep = lisp.string(if cfg!(windows) { ";" } else { ":" });
    lisp.defvar(sym::PATH_SEPARATOR, sep);
    lisp.defvar(sym::INSTALLATION_DIRECTORY, Value::NIL);
    lisp.defvar(sym::SYSTEM_MESSAGES_LOCALE, Value::NIL);
    lisp.defvar(sym::SYSTEM_TIME_LOCALE, Value::NIL);
    lisp.defvar(sym::BEFORE_INIT_TIME, Value::NIL);
    lisp.defvar(sym::AFTER_INIT_TIME, Value::NIL);
    lisp.defvar_bool(sym::INHIBIT_X_RESOURCES, false);
    lisp.defvar(sym::INVOCATION_NAME, Value::NIL);
    lisp.defvar(sym::INVOCATION_DIRECTORY, Value::NIL);
    let copyright = lisp.string(EMACS_COPYRIGHT);
    lisp.defvar(sym::EMACS_COPYRIGHT, copyright);
    let bug = lisp.string(PACKAGE_BUGREPORT);
    lisp.defvar(sym::REPORT_EMACS_BUG_ADDRESS, bug);
    lisp.defvar(sym::DUMP_MODE, Value::NIL);
    lisp.defvar(sym::DYNAMIC_LIBRARY_ALIST, Value::NIL);
    let shell = if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    };
    let shell = lisp.string(&normalize_slashes(&shell));
    lisp.defvar(sym::SHELL_FILE_NAME, shell);
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs: Vec<Value> = std::env::split_paths(&path_var)
        .map(|p| {
            let s = normalize_slashes(&p.to_string_lossy());
            lisp.string(&s)
        })
        .collect();
    let exec_dir = lisp.string(&path_exec());
    dirs.push(exec_dir);
    let exec_path = lisp.list(&dirs);
    lisp.defvar(sym::EXEC_PATH, exec_path);
    let suffixes: Vec<Value> = if cfg!(windows) {
        [".exe", ".com", ".bat", ".cmd", ""]
            .iter()
            .map(|s| lisp.string(s))
            .collect()
    } else {
        vec![lisp.string("")]
    };
    let suffixes = lisp.list(&suffixes);
    lisp.defvar(sym::EXEC_SUFFIXES, suffixes);
    lisp.defvar(sym::EXEC_DIRECTORY, exec_dir);
    lisp.defvar(sym::DATA_DIRECTORY, Value::NIL);
    lisp.defvar(sym::DOC_DIRECTORY, Value::NIL);
    lisp.defvar(sym::CONFIGURE_INFO_DIRECTORY, Value::NIL);
    lisp.defvar(sym::SHARED_GAME_SCORE_DIRECTORY, Value::NIL);
    let env: Vec<Value> = std::env::vars_os()
        .map(|(k, v)| {
            let s = format!("{}={}", k.to_string_lossy(), v.to_string_lossy());
            lisp.string(&s)
        })
        .collect();
    let env = lisp.list(&env);
    lisp.defvar(sym::INITIAL_ENVIRONMENT, env);
    let items = lisp.list_to_vec(env).unwrap_or_default();
    let process_environment = lisp.list(&items);
    lisp.defvar(sym::PROCESS_ENVIRONMENT, process_environment);
    for (symbol, name) in [
        (sym::CTAGS_PROGRAM_NAME, "ctags"),
        (sym::ETAGS_PROGRAM_NAME, "etags"),
        (sym::HEXL_PROGRAM_NAME, "hexl"),
        (sym::EMACSCLIENT_PROGRAM_NAME, "emacsclient"),
        (sym::MOVEMAIL_PROGRAM_NAME, "movemail"),
        (sym::EBROWSE_PROGRAM_NAME, "ebrowse"),
        (sym::RCS2LOG_PROGRAM_NAME, "rcs2log"),
    ] {
        let v = lisp.string(name);
        lisp.defvar(symbol, v);
    }
    lisp.defvar(sym::COMMAND_LINE_MAX_LENGTH, Value::fixnum(32768));
}

impl Lisp {
    pub fn set_source_directory(&mut self, root: &Path) {
        let as_dir = |p: &Path| {
            let mut s = normalize_slashes(&p.to_string_lossy());
            if !s.ends_with('/') {
                s.push('/');
            }
            s
        };
        let root_s = as_dir(root);
        let etc = as_dir(&root.join("etc"));
        let lisp_dir = normalize_slashes(&root.join("lisp").to_string_lossy());
        let r = self.string(&root_s);
        self.set_plain_value(sym::SOURCE_DIRECTORY, r);
        self.set_plain_value(sym::INSTALLATION_DIRECTORY, r);
        let e = self.string(&etc);
        self.set_plain_value(sym::DATA_DIRECTORY, e);
        self.set_plain_value(sym::DOC_DIRECTORY, e);
        let l = self.string(&lisp_dir);
        let load_path = self.list(&[l]);
        self.set_plain_value(sym::LOAD_PATH, load_path);
    }

    pub fn getenv(&mut self, name: &str) -> Option<String> {
        let env = self.find_symbol_value(sym::PROCESS_ENVIRONMENT);
        let mut tail = env;
        while tail.is_cons() {
            let entry = self.xcar(tail);
            tail = self.xcdr(tail);
            if !entry.is_string() {
                continue;
            }
            let text = self.string_to_rust(entry);
            let matches = |key: &str| {
                if cfg!(windows) {
                    key.eq_ignore_ascii_case(name)
                } else {
                    key == name
                }
            };
            match text.split_once('=') {
                Some((key, value)) if matches(key) => return Some(value.to_string()),
                None if matches(&text) => return None,
                _ => {}
            }
        }
        None
    }
}

pub fn find_source_root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("EMACS_SOURCE_DIR") {
        return Some(PathBuf::from(dir));
    }
    let candidates = [std::env::current_exe().ok(), std::env::current_dir().ok()];
    for start in candidates.into_iter().flatten() {
        let mut dir: Option<&Path> = Some(start.as_path());
        while let Some(d) = dir {
            if d.join("lisp").join("subr.el").is_file() {
                return Some(d.to_path_buf());
            }
            dir = d.parent();
        }
    }
    None
}

fn f_nil(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_invocation_name(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let name = lisp.plain_value(sym::INVOCATION_NAME);
    crate::fns::f_copy_sequence(lisp, &[name])
}

fn f_invocation_directory(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let dir = lisp.plain_value(sym::INVOCATION_DIRECTORY);
    crate::fns::f_copy_sequence(lisp, &[dir])
}

fn f_daemon_initialized(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Err(lisp.error("This function can only be called if emacs is run as a daemon"))
}

pub(crate) fn f_kill_emacs(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let arg = args[0];
    let hook = lisp.find_symbol_value(sym::KILL_EMACS_HOOK);
    if !hook.is_nil() && hook != sym::UNBOUND {
        let _ = lisp.run_hook(sym::KILL_EMACS_HOOK);
    }
    let code = if let Some(n) = arg.as_fixnum() {
        n as i32
    } else if arg.is_string() {
        let text = lisp.string_to_rust(arg);
        let _ = lisp.stdout.write_all(text.as_bytes());
        0
    } else if arg.is_nil() {
        0
    } else {
        1
    };
    let _ = lisp.stdout.flush();
    let _ = lisp.stderr.flush();
    std::process::exit(code);
}

fn f_getenv_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = lisp.check_string(args[0])?;
    let name = lisp.string_to_rust(variable);
    let value = if args[1].is_cons() {
        let count = lisp.specpdl.len();
        lisp.specbind(sym::PROCESS_ENVIRONMENT, args[1])?;
        let v = lisp.getenv(&name);
        lisp.unbind_to(count)?;
        v
    } else {
        lisp.getenv(&name)
    };
    match value {
        Some(v) => Ok(lisp.string(&v)),
        None => Ok(Value::NIL),
    }
}

fn run_process(
    lisp: &mut Lisp,
    program: Value,
    input: Option<Vec<u8>>,
    destination: Value,
    display: Value,
    args: &[Value],
) -> LispResult {
    let _ = display;
    let program = lisp.check_string(program)?;
    let prog = lisp.string_to_rust(program);
    let mut cmd = std::process::Command::new(&prog);
    for &a in args {
        let a = lisp.check_string(a)?;
        cmd.arg(lisp.string_to_rust(a));
    }
    let dir = lisp.default_directory_string();
    if Path::new(&dir).is_dir() {
        cmd.current_dir(&dir);
    }
    cmd.env_clear();
    let env = lisp.find_symbol_value(sym::PROCESS_ENVIRONMENT);
    let mut tail = env;
    while tail.is_cons() {
        let entry = lisp.xcar(tail);
        tail = lisp.xcdr(tail);
        if entry.is_string() {
            let text = lisp.string_to_rust(entry);
            if let Some((k, v)) = text.split_once('=') {
                cmd.env(k, v);
            }
        }
    }
    cmd.stdin(if input.is_some() {
        std::process::Stdio::piped()
    } else {
        std::process::Stdio::null()
    });
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Err(lisp.file_error("Searching for program", &e, program));
        }
    };
    if let Some(data) = input
        && let Some(mut stdin) = child.stdin.take()
    {
        let _ = stdin.write_all(&data);
    }
    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return Err(lisp.file_error("Running program", &e, program)),
    };
    let (dest, stderr_dest) = if destination.is_cons() {
        (
            lisp.xcar(destination),
            lisp.car_safe(lisp.xcdr(destination)),
        )
    } else {
        (destination, Value::NIL)
    };
    let mut text = output.stdout.clone();
    if stderr_dest == sym::T || (stderr_dest.is_nil() && !destination.is_cons()) {
        text.extend_from_slice(&output.stderr);
    }
    if dest == sym::T || lisp.is_buffer(dest) || dest.is_string() {
        let count = lisp.specpdl.len();
        lisp.record_unwind_current_buffer();
        if lisp.is_buffer(dest) {
            lisp.set_buffer_internal(dest);
        } else if dest.is_string() {
            let b = lisp.get_buffer_create(dest, false)?;
            lisp.set_buffer_internal(b);
        }
        let normalized: Vec<u8> = if cfg!(windows) {
            String::from_utf8_lossy(&text)
                .replace("\r\n", "\n")
                .into_bytes()
        } else {
            text
        };
        let (data, chars) = crate::lread::decode_utf8_emacs(&normalized);
        lisp.insert_internal(&data, chars, &[], false, false)?;
        lisp.unbind_to(count)?;
    }
    Ok(Value::fixnum(output.status.code().unwrap_or(-1) as i64))
}

fn f_call_process(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let program = args[0];
    let infile = args.get(1).copied().unwrap_or(Value::NIL);
    let destination = args.get(2).copied().unwrap_or(Value::NIL);
    let display = args.get(3).copied().unwrap_or(Value::NIL);
    let input = if infile.is_string() {
        let path = lisp.expand_file_name(infile, Value::NIL)?;
        let p = lisp.string_to_rust(path);
        std::fs::read(p).ok()
    } else {
        None
    };
    let rest = if args.len() > 4 { &args[4..] } else { &[] };
    run_process(lisp, program, input, destination, display, rest)
}

fn f_call_process_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (start, end, program) = (args[0], args[1], args[2]);
    let delete = args.get(3).copied().unwrap_or(Value::NIL);
    let destination = args.get(4).copied().unwrap_or(Value::NIL);
    let display = args.get(5).copied().unwrap_or(Value::NIL);
    let input = if start.is_string() {
        lisp.string_to_rust(start).into_bytes()
    } else {
        let (s, e) = if start.is_nil() {
            (lisp.cur().begv, lisp.cur().zv)
        } else {
            lisp.validate_region(start, end)?
        };
        let text = lisp.buffer_substring_value(s, e, false);
        let t = lisp.string_to_rust(text).into_bytes();
        if !delete.is_nil() {
            lisp.del_range(s, e)?;
        }
        t
    };
    let rest = if args.len() > 6 { &args[6..] } else { &[] };
    run_process(lisp, program, Some(input), destination, display, rest)
}
