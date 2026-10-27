#![allow(dead_code)]

use emacs_core::lisp::Lisp;

pub const STACK_SIZE: usize = 512 << 20;

pub fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, f)
            .expect("spawn test thread")
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

pub fn eval(lisp: &mut Lisp, source: &str) -> String {
    match lisp.eval_string(source) {
        Ok(value) => lisp.prin1_to_string(value),
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            format!("error: {}", lisp.prin1_to_string(condition))
        }
    }
}

pub fn check(cases: &[(&str, &str)]) {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        for (source, expected) in cases {
            assert_eq!(eval(&mut lisp, source), *expected, "evaluating {source}");
        }
    });
}

pub fn check_read(cases: &[(&str, &str)]) {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        for (source, expected) in cases {
            let printed = match lisp.read_from_str(source) {
                Ok(value) => lisp.prin1_to_string(value),
                Err(flow) => {
                    let condition = lisp.flow_condition(&flow);
                    format!("error: {}", lisp.prin1_to_string(condition))
                }
            };
            assert_eq!(printed, *expected, "reading {source}");
        }
    });
}
