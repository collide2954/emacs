use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
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

fn displayed() -> (Lisp, Value) {
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(
        &mut lisp,
        Display::new(fmpsc::unbounded().0, mpsc::channel().1),
    );
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    assert_eq!(
        eval(
            &mut lisp,
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"r\") (width . 30) (height . 8)))) (select-frame f) (set-window-buffer nil (get-buffer-create \"text\")) (set-buffer \"text\") (setq bidi-paragraph-direction 'left-to-right) (let ((i 0)) (while (< i 20) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.frame_mut(f).flags.visible = true;
    lisp.redisplay().unwrap();
    let w = lisp.frame(f).root_window;
    (lisp, w)
}

fn line_start(n: i64) -> i64 {
    1 + (0..n)
        .map(|i| format!("line {i}\n").len() as i64)
        .sum::<i64>()
}

#[test]
fn recenter_moves_the_window_start_around_point() {
    on_big_stack(|| {
        let (mut lisp, w) = displayed();
        let lines = lisp.window(w).total_lines - 1;
        let at = |n: i64| format!("(progn (goto-char {}) ", line_start(n));
        assert_eq!(
            eval(&mut lisp, &format!("{}(recenter) (window-start))", at(10))),
            line_start(10 - lines / 2).to_string()
        );
        assert!(lisp.window(w).flags.optional_new_start);
        assert!(!lisp.window(w).flags.window_end_valid);
        assert_eq!(
            eval(
                &mut lisp,
                &format!("{}(recenter 0) (window-start))", at(10))
            ),
            line_start(10).to_string()
        );
        assert_eq!(
            eval(
                &mut lisp,
                &format!("{}(recenter 2) (window-start))", at(10))
            ),
            line_start(8).to_string()
        );
        assert_eq!(
            eval(
                &mut lisp,
                &format!("{}(recenter -1) (window-start))", at(10))
            ),
            line_start(10 - (lines - 1)).to_string()
        );
        assert_eq!(
            eval(
                &mut lisp,
                &format!("{}(recenter '(4)) (window-start))", at(10))
            ),
            line_start(10 - lines / 2).to_string()
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(save-current-buffer (set-buffer (get-buffer-create \"other\")) (recenter))"
            ),
            "error: (error \"\u{2018}recenter\u{2019}ing a window that does not display current-buffer\")"
        );
    });
}

#[test]
fn move_to_window_line_counts_from_the_window_start() {
    on_big_stack(|| {
        let (mut lisp, w) = displayed();
        let lines = lisp.window(w).total_lines - 1;
        eval(
            &mut lisp,
            &format!("(progn (goto-char {}) (recenter 0))", line_start(5)),
        );
        assert_eq!(
            eval(&mut lisp, "(list (move-to-window-line nil) (point))"),
            format!("({} {})", lines / 2, line_start(5 + lines / 2))
        );
        assert_eq!(
            eval(&mut lisp, "(list (move-to-window-line 0) (point))"),
            format!("(0 {})", line_start(5))
        );
        assert_eq!(
            eval(&mut lisp, "(list (move-to-window-line -1) (point))"),
            format!("({} {})", lines - 1, line_start(5 + lines - 1))
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(save-current-buffer (set-buffer (get-buffer-create \"other\")) (move-to-window-line 0))"
            ),
            "error: (error \"move-to-window-line called from unrelated buffer\")"
        );
    });
}
