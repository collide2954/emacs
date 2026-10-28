use std::io::Write;

use emacs_core::emacs::{decode_command_line, find_source_root, os_bytes, version_text};
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;

const STACK_SIZE: usize = 1 << 30;

fn main() {
    let args: Vec<Vec<u8>> = std::env::args_os().map(|a| os_bytes(&a)).collect();
    let code = std::thread::Builder::new()
        .name("lisp".into())
        .stack_size(STACK_SIZE)
        .spawn(move || run(args))
        .expect("spawn lisp thread")
        .join()
        .unwrap_or(70);
    std::process::exit(code);
}

fn run(args: Vec<Vec<u8>>) -> i32 {
    let startup = match decode_command_line(args) {
        Ok(startup) => startup,
        Err(exit) => {
            let mut stdout = std::io::stdout();
            let _ = stdout.write_all(exit.stdout.as_bytes());
            let _ = stdout.flush();
            let mut stderr = std::io::stderr();
            let _ = stderr.write_all(exit.stderr.as_bytes());
            let _ = stderr.flush();
            return exit.code;
        }
    };
    let mut lisp = Lisp::boot(startup);
    match find_source_root() {
        Some(root) => lisp.set_source_directory(&root),
        None => {
            eprintln!("emacs: cannot find the Lisp directory; set EMACS_SOURCE_DIR");
            return 1;
        }
    }
    if lisp.startup.only_version {
        let _ = lisp.stdout.write_all(version_text().as_bytes());
        let _ = lisp.stdout.flush();
        return 0;
    }
    if let Some(file) = lisp.startup.load_file() {
        let file = lisp.unibyte_string(&file);
        let top_level = lisp.list(&[sym::LOAD, file]);
        lisp.set_plain_value(sym::TOP_LEVEL, top_level);
    }
    if !lisp.startup.no_loadup {
        let loadup = lisp.string("loadup.el");
        let top_level = lisp.list(&[sym::LOAD, loadup]);
        lisp.set_plain_value(sym::TOP_LEVEL, top_level);
    }
    let result = lisp.catch_all(|l| {
        let form = l.plain_value(sym::TOP_LEVEL);
        l.eval_with_lexical(form, sym::T)
    });
    let _ = lisp.stdout.flush();
    match result {
        Ok(_) => 0,
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            lisp.stack.push(condition);
            let message = match lisp.error_message_string(condition) {
                Ok(m) => m,
                Err(_) => lisp.prin1_to_string(condition),
            };
            let _ = writeln!(lisp.stderr, "{message}");
            let _ = lisp.stderr.flush();
            255
        }
    }
}
