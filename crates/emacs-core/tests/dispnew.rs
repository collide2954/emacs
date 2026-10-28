mod common;

use common::{check, eval, on_big_stack};
use emacs_core::lisp::Lisp;

#[test]
fn sleep_for() {
    check(&[
        ("(sleep-for 0)", "nil"),
        ("(sleep-for -1 -5)", "nil"),
        ("(sleep-for 0 'x)", "error: (wrong-type-argument fixnump x)"),
        (
            "(sleep-for \"a\")",
            "error: (wrong-type-argument numberp \"a\")",
        ),
        (
            "(let ((start (float-time))) (sleep-for 0.05) (>= (- (float-time) start) 0.05))",
            "t",
        ),
        (
            "(let ((start (float-time))) (sleep-for 0 50) (>= (- (float-time) start) 0.05))",
            "t",
        ),
        (
            "(condition-case nil (progn (setq quit-flag t) (sleep-for 0.01) 'slept) (quit 'quit))",
            "quit",
        ),
        ("quit-flag", "nil"),
        (
            "(let ((inhibit-quit t)) (setq quit-flag t) (sleep-for 0.01) (prog1 quit-flag (setq quit-flag nil)))",
            "t",
        ),
        (
            "(let ((throw-on-input 'input)) (catch 'input (setq quit-flag 'input) (sleep-for 0.01) 'slept))",
            "t",
        ),
        ("quit-flag", "nil"),
    ]);
}

fn sit(lisp: &mut Lisp, timeout: &str, reading: bool) -> String {
    let timeout = lisp.read_from_str(timeout).expect("read timeout");
    match lisp.sit_for(timeout, reading) {
        Ok(value) => lisp.prin1_to_string(value),
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            format!("error: {}", lisp.prin1_to_string(condition))
        }
    }
}

#[test]
fn sit_for() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        assert_eq!(sit(&mut lisp, "0", false), "t");
        assert_eq!(sit(&mut lisp, "-3", false), "t");
        assert_eq!(sit(&mut lisp, "-99999999999999999999999", false), "t");
        assert_eq!(sit(&mut lisp, "0.0", false), "t");
        assert_eq!(sit(&mut lisp, "0.01", false), "t");
        assert_eq!(sit(&mut lisp, "0.01", true), "t");
        assert_eq!(
            sit(&mut lisp, "x", false),
            "error: (wrong-type-argument numberp x)"
        );
        eval(&mut lisp, "(setq quit-flag t)");
        assert_eq!(sit(&mut lisp, "5", false), "nil");
        eval(&mut lisp, "(setq quit-flag nil)");
        eval(&mut lisp, "(setq executing-kbd-macro \"x\")");
        assert_eq!(sit(&mut lisp, "5", false), "nil");
        eval(&mut lisp, "(setq executing-kbd-macro nil)");
        eval(&mut lisp, "(setq unread-command-events '(97))");
        let start = std::time::Instant::now();
        assert_eq!(sit(&mut lisp, "5", false), "t");
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
        eval(&mut lisp, "(setq unread-command-events nil)");
    });
}
