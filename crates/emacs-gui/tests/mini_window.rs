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

fn displayed() -> Lisp {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (fset 'window--resize-root-window-vertically (function (lambda (window delta pixelwise) (set-window-new-pixel window (+ (window-pixel-height window) delta)) delta))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"m\") (width . 30) (height . 20)))) (select-frame f) (setq bidi-paragraph-direction 'left-to-right) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.frame_mut(f).flags.visible = true;
    lisp.redisplay().unwrap();
    lisp
}

#[test]
fn a_multi_line_message_grows_the_mini_window_and_an_exact_resize_shrinks_it() {
    on_big_stack(|| {
        let mut lisp = displayed();
        let h: i64 = eval(&mut lisp, "(frame-char-height f)").parse().unwrap();
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq resize-mini-windows 'grow-only r (frame-root-window f) m (minibuffer-window f) rh (window-pixel-height r) mh (window-pixel-height m)) (= mh (frame-char-height f)))"
            ),
            "t"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (message \"one\\ntwo\\nthree\") (list (- rh (window-pixel-height r)) (- (window-pixel-height m) mh) (window-body-height m) (= (window-pixel-top m) (+ (window-pixel-top r) (window-pixel-height r)))))"
            ),
            format!("({} {} 3 t)", 2 * h, 2 * h)
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq resize-mini-windows t) (message \"single\") (list (= (window-pixel-height r) rh) (= (window-pixel-height m) mh) (window-body-height m)))"
            ),
            "(t t 1)"
        );
    });
}
