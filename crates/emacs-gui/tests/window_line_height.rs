use std::sync::mpsc;

use emacs_core::dispnew::matrix::WhichMatrix;
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

fn displayed_frame() -> (Lisp, Value, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"h\") (width . 30) (height . 8)))) (select-frame f) (set-window-buffer nil (get-buffer-create \"text\")) (set-buffer \"text\") (setq bidi-paragraph-direction 'left-to-right) (setq mode-line-format \"%b\") (let ((i 0)) (while (< i 20) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    let w = lisp.frame(f).root_window;
    lisp.frame_mut(f).flags.visible = true;
    (lisp, f, w)
}

#[test]
fn window_line_height_reads_the_current_matrix() {
    on_big_stack(|| {
        let (mut lisp, _, w) = displayed_frame();
        assert_eq!(eval(&mut lisp, "(window-line-height)"), "nil");
        lisp.redisplay().unwrap();
        let (lh, mode_line_height, first) = {
            let m = lisp.window_matrix(w, WhichMatrix::Current).unwrap();
            let first = m.first_text_row();
            (
                m.rows[first].height,
                m.rows[m.mode_line_row()].height,
                first,
            )
        };
        assert_eq!(first, 0);
        let bottom = lisp.window_text_bottom_y(w) as i32;
        assert_eq!(
            eval(&mut lisp, "(window-line-height)"),
            format!("({lh} 0 0 0)")
        );
        assert_eq!(
            eval(&mut lisp, "(window-line-height 2)"),
            format!("({lh} 2 {} 0)", 2 * lh)
        );
        let last = (bottom + lh - 1) / lh - 1;
        let crop = (last + 1) * lh - bottom;
        let last_line = format!("({} {last} {} {crop})", lh - crop, last * lh);
        assert_eq!(eval(&mut lisp, "(window-line-height -1)"), last_line);
        assert_eq!(eval(&mut lisp, "(window-line-height 100)"), last_line);
        assert_eq!(
            eval(&mut lisp, "(window-line-height -2)"),
            format!("({lh} {} {} 0)", last - 1, (last - 1) * lh)
        );
        assert_eq!(
            eval(&mut lisp, "(window-line-height 'mode-line)"),
            format!("({mode_line_height} 0 {bottom} 0)")
        );
        assert_eq!(eval(&mut lisp, "(window-line-height 'header-line)"), "nil");
        assert_eq!(eval(&mut lisp, "(window-line-height 'tab-line)"), "nil");
        assert_eq!(
            eval(&mut lisp, "(window-line-height 'other)"),
            "error: (wrong-type-argument fixnump other)"
        );
        eval(&mut lisp, "(insert \"x\")");
        assert_eq!(eval(&mut lisp, "(window-line-height)"), "nil");
    });
}
