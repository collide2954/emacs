use std::io::Write;

use emacs_core::emacs::find_source_root;
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;

const STACK_SIZE: usize = 1 << 30;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = std::thread::Builder::new()
        .name("lisp".into())
        .stack_size(STACK_SIZE)
        .spawn(move || run(&args))
        .expect("spawn lisp thread")
        .join()
        .unwrap_or(70);
    std::process::exit(code);
}

fn run(args: &[String]) -> i32 {
    let batch = args.iter().skip(1).any(|a| {
        matches!(
            a.as_str(),
            "--batch" | "-batch" | "--script" | "-script" | "-x"
        )
    });
    let mut lisp = Lisp::new_with(batch);
    lisp.set_command_line_args(args);
    match find_source_root() {
        Some(root) => lisp.set_source_directory(&root),
        None => {
            eprintln!("emacs: cannot find the Lisp directory; set EMACS_SOURCE_DIR");
            return 1;
        }
    }
    let loadup = lisp.string("loadup.el");
    let top_level = lisp.list(&[sym::LOAD, loadup]);
    lisp.set_plain_value(sym::TOP_LEVEL, top_level);
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
