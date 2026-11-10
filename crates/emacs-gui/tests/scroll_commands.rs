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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"s\") (width . 30) (height . 8)))) (select-frame f) (set-window-buffer nil (get-buffer-create \"text\")) (set-buffer \"text\") (setq bidi-paragraph-direction 'left-to-right) (let ((i 0)) (while (< i 20) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.frame_mut(f).flags.visible = true;
    lisp.redisplay().unwrap();
    (lisp, f)
}

fn line_start(n: i64) -> i64 {
    1 + (0..n)
        .map(|i| format!("line {i}\n").len() as i64)
        .sum::<i64>()
}

fn start_and_point(lisp: &mut Lisp) -> String {
    eval(lisp, "(list (window-start) (point))")
}

#[test]
fn scroll_up_and_down_by_screens_and_lines() {
    on_big_stack(|| {
        let (mut lisp, _) = displayed();
        let lines = lisp.window(lisp.windows.selected_window).total_lines - 1;
        let screen = lines - 2;
        assert_eq!(eval(&mut lisp, "(scroll-up)"), "nil");
        let s = line_start(screen);
        assert_eq!(start_and_point(&mut lisp), format!("({s} {s})"));
        assert!(lisp.window(lisp.windows.selected_window).flags.force_start);
        lisp.redisplay().unwrap();
        assert_eq!(eval(&mut lisp, "(scroll-down)"), "nil");
        assert_eq!(start_and_point(&mut lisp), format!("(1 {s})"));
        lisp.redisplay().unwrap();
        assert_eq!(eval(&mut lisp, "(scroll-up 2)"), "nil");
        assert_eq!(
            start_and_point(&mut lisp),
            format!("({} {s})", line_start(2))
        );
        lisp.redisplay().unwrap();
        assert_eq!(eval(&mut lisp, "(scroll-down '-)"), "nil");
        assert_eq!(
            start_and_point(&mut lisp),
            format!("({} {})", line_start(2 + screen), line_start(2 + screen))
        );
    });
}

#[test]
fn scrolling_past_either_end_signals() {
    on_big_stack(|| {
        let (mut lisp, _) = displayed();
        assert_eq!(
            eval(
                &mut lisp,
                "(condition-case e (scroll-down) (error (car e)))"
            ),
            "beginning-of-buffer"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(condition-case e (scroll-up 100) (error (car e)))"
            ),
            "end-of-buffer"
        );
        assert_eq!(
            eval(&mut lisp, "(progn (scroll-up \"x\") (window-start))"),
            line_start(1).to_string()
        );
    });
}
