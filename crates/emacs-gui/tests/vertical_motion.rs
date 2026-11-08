use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_gui::display::Display;
use iced::futures::channel::mpsc as fmpsc;

fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(512 << 20)
            .spawn_scoped(scope, f)
            .expect("spawn test thread")
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

fn eval(lisp: &mut Lisp, source: &str) -> String {
    match lisp.eval_string(source) {
        Ok(value) => lisp.prin1_to_string(value),
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            format!("error: {}", lisp.prin1_to_string(condition))
        }
    }
}

fn in_buffer(text: &str, body: &str) -> String {
    format!(
        "(save-current-buffer (set-buffer (get-buffer-create \"vm\")) (erase-buffer) (insert {text:?}) {body})"
    )
}

#[test]
fn vertical_motion_on_a_gui_frame() {
    on_big_stack(|| {
        let mut lisp = Lisp::boot(Startup::program(false));
        emacs_gui::install(
            &mut lisp,
            Display::new(fmpsc::unbounded().0, mpsc::channel().1),
        );
        lisp.init_display();
        eval(
            &mut lisp,
            "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-range char-script-table '(#x2190 . #x21FF) 'symbol) (set-char-table-extra-slot char-script-table 0 '(latin symbol)))",
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"v\") (width . 20) (height . 5)))) (select-frame f) (framep f))"
            ),
            "pgtk"
        );
        let long = "x".repeat(30);
        for (source, expected) in [
            (
                in_buffer(
                    "ab\ncd\nef",
                    "(goto-char 1) (list (vertical-motion 1) (point))",
                ),
                "(1 4)",
            ),
            (
                in_buffer(
                    "ab\ncd\nef",
                    "(goto-char (point-max)) (list (vertical-motion -2) (point))",
                ),
                "(-2 1)",
            ),
            (
                in_buffer(&long, "(goto-char 1) (list (vertical-motion 1) (point))"),
                "(1 21)",
            ),
            (
                in_buffer(
                    "abc\ndef",
                    "(goto-char 1) (list (vertical-motion '(2 . 1)) (point))",
                ),
                "(1 7)",
            ),
            (
                in_buffer(
                    "\tx",
                    "(goto-char 1) (list (vertical-motion '(8 . 0)) (point))",
                ),
                "(0 2)",
            ),
            (
                in_buffer(
                    &long,
                    "(setq display-line-numbers t) (goto-char 1) (prog1 (list (vertical-motion 1) (point)) (setq display-line-numbers nil))",
                ),
                "(1 18)",
            ),
        ] {
            assert_eq!(eval(&mut lisp, &source), expected, "evaluating {source}");
        }
    });
}
