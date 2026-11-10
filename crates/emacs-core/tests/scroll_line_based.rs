mod common;

use common::{eval, on_big_stack};

fn line_start(n: i64) -> i64 {
    1 + (0..n)
        .map(|i| format!("line {i}\n").len() as i64)
        .sum::<i64>()
}

#[test]
fn scrolling_a_text_frame_window_moves_its_start_by_lines() {
    on_big_stack(|| {
        let mut lisp = emacs_core::lisp::Lisp::new_with(false);
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"scroll\")) (set-buffer \"scroll\") (let ((i 0)) (while (< i 60) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1) (scroll-up 3) (list (window-start) (point)))",
            ),
            format!("({} {})", line_start(3), line_start(3))
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(condition-case e (scroll-down 2) (error (car e)))"
            ),
            "beginning-of-buffer"
        );
        let ht: i64 = eval(&mut lisp, "(1- (window-total-height))")
            .parse()
            .unwrap();
        assert_eq!(
            eval(
                &mut lisp,
                &format!(
                    "(progn (goto-char {}) (scroll-down 2) (list (window-start) (point)))",
                    line_start(40)
                )
            ),
            format!("({} {})", line_start(40 - ht / 2 - 2), line_start(40))
        );
    });
}
