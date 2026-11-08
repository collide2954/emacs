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

fn gui_frame() -> Lisp {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"c\") (width . 30) (height . 8)))) (select-frame f) (frame-after-make-frame f t) (framep f))"
        ),
        "pgtk"
    );
    lisp
}

#[test]
fn buffer_changes_run_the_global_and_local_hooks() {
    on_big_stack(|| {
        let mut lisp = gui_frame();
        eval(
            &mut lisp,
            "(progn (setq seen nil) (set-default 'window-buffer-change-functions (list (function (lambda (x) (setq seen (cons (if (eq x f) 'frame (if (framep x) 'other-frame 'window)) seen)))))))",
        );
        lisp.run_window_change_functions().unwrap();
        assert_eq!(
            eval(
                &mut lisp,
                "(and (memq 'frame seen) (null (memq 'window seen)))"
            ),
            "t"
        );
        eval(&mut lisp, "(setq seen nil)");
        lisp.run_window_change_functions().unwrap();
        assert_eq!(eval(&mut lisp, "seen"), "nil");
        eval(
            &mut lisp,
            "(progn (set-buffer (get-buffer-create \"other\")) (set (make-local-variable 'window-buffer-change-functions) (list (function (lambda (w) (setq seen (cons (if (windowp w) 'local 'odd) seen)))) t)) (set-window-buffer nil \"other\"))",
        );
        lisp.run_window_change_functions().unwrap();
        assert_eq!(eval(&mut lisp, "seen"), "(frame local)");
    });
}

#[test]
fn selection_and_state_hooks_follow_the_selected_window() {
    on_big_stack(|| {
        let mut lisp = gui_frame();
        lisp.run_window_change_functions().unwrap();
        eval(
            &mut lisp,
            "(progn (setq sel nil state nil) (set-default 'window-selection-change-functions (list (function (lambda (f) (setq sel (cons 'sel sel)))))) (setq window-state-change-hook (list (function (lambda () (setq state (cons 'state state)))))))",
        );
        lisp.run_window_change_functions().unwrap();
        assert_eq!(eval(&mut lisp, "(list sel state)"), "(nil nil)");
        eval(&mut lisp, "(select-window (minibuffer-window f))");
        lisp.run_window_change_functions().unwrap();
        assert_eq!(eval(&mut lisp, "(list sel state)"), "((sel) (state))");
    });
}
