#![allow(dead_code)]

use emacs_core::lisp::Lisp;

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
    let mut lisp = Lisp::new();
    for (source, expected) in cases {
        assert_eq!(eval(&mut lisp, source), *expected, "evaluating {source}");
    }
}

pub fn check_read(cases: &[(&str, &str)]) {
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
}
