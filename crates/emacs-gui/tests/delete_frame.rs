use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_core::value::Value;
use emacs_gui::display::Display;
use emacs_gui::protocol::Command;
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

fn connected(display: Display) -> Lisp {
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(&mut lisp, display);
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    assert_eq!(
        eval(
            &mut lisp,
            "(progn (setq x-resource-name \"emacs\") (x-open-connection nil) nil)"
        ),
        "nil"
    );
    lisp
}

fn value_of(lisp: &mut Lisp, name: &str) -> Value {
    let symbol = lisp.intern(name);
    lisp.find_symbol_value(symbol)
}

fn check(lisp: &mut Lisp, cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(eval(lisp, source), *expected, "evaluating {source}");
    }
}

#[test]
fn deleting_the_initial_frame_deletes_its_terminal() {
    on_big_stack(|| {
        let mut lisp = connected(Display::new(fmpsc::unbounded().0, mpsc::channel().1));
        check(
            &mut lisp,
            &[
                (
                    "(progn (setq f0 (selected-frame) w0 (frame-root-window f0) t0 (frame-terminal f0) deleted nil after 'unset) nil)",
                    "nil",
                ),
                (
                    "(progn (setq delete-frame-functions (list (function (lambda (fr) (setq deleted fr)))) after-delete-frame-functions (list (function (lambda (fr) (setq after (frame-live-p fr)))))) nil)",
                    "nil",
                ),
                (
                    "(framep (setq f (x-create-frame '((name . \"a\") (width . 20) (height . 5)))))",
                    "pgtk",
                ),
                ("(delete-frame f0)", "nil"),
                (
                    "(list (eq deleted f0) after (frame-live-p f0) (memq f0 (frame-list)) (eq (selected-frame) f) (window-live-p w0) (terminal-live-p t0) (length (terminal-list)))",
                    "(t nil nil nil t nil nil 1)",
                ),
                ("(delete-frame f0)", "nil"),
                ("(eq (selected-window) (frame-selected-window f))", "t"),
                ("(eq (last-nonminibuffer-frame) f)", "t"),
                (
                    "(delete-frame f)",
                    "error: (error \"Attempt to delete the sole visible or iconified frame\")",
                ),
                (
                    "(delete-frame f t)",
                    "error: (error \"Attempt to delete the only frame\")",
                ),
            ],
        );
        assert_eq!(lisp.terminals.initial, Value::NIL);
    });
}

#[test]
fn deleting_a_frame_moves_its_active_minibuffer_to_the_new_selected_frame() {
    on_big_stack(|| {
        let mut lisp = connected(Display::new(fmpsc::unbounded().0, mpsc::channel().1));
        check(
            &mut lisp,
            &[(
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (setq f0 (selected-frame) f (x-create-frame '((name . \"a\") (width . 20) (height . 5)))) nil)",
                "nil",
            )],
        );
        lisp.minibuf.minibuf_level = 1;
        let minibuffer = lisp.get_minibuffer(1).unwrap();
        let symbol = lisp.intern("mb1");
        lisp.set_plain_value(symbol, minibuffer);
        check(
            &mut lisp,
            &[
                (
                    "(progn (save-current-buffer (set-buffer mb1) (insert \"abc\")) (set-window-buffer (minibuffer-window f0) mb1) (set-window-point (minibuffer-window f0) 3) (buffer-name (window-buffer (minibuffer-window f))))",
                    "\" *Minibuf-0*\"",
                ),
                ("(delete-frame f0)", "nil"),
                (
                    "(list (eq (window-buffer (minibuffer-window f)) mb1) (window-point (minibuffer-window f)))",
                    "(t 3)",
                ),
            ],
        );
        let f = value_of(&mut lisp, "f");
        assert_eq!(lisp.minibuf.mb_frame, f);
        assert_eq!(lisp.windows.minibuf_window, lisp.frame(f).minibuffer_window);
    });
}

#[test]
fn deleting_pgtk_frames_closes_their_windows_but_keeps_the_display() {
    on_big_stack(|| {
        let (commands, mut command_rx) = fmpsc::unbounded();
        let mut lisp = connected(Display::new(commands, mpsc::channel().1));
        check(
            &mut lisp,
            &[(
                "(progn (setq f (x-create-frame '((name . \"a\") (width . 20) (height . 5))) g (x-create-frame '((name . \"b\") (width . 20) (height . 5))) tp (frame-terminal g)) nil)",
                "nil",
            )],
        );
        let g = value_of(&mut lisp, "g");
        let g_id = lisp.frame(g).id as u64;
        let tp = value_of(&mut lisp, "tp");
        while command_rx.try_recv().is_ok() {}
        assert_eq!(lisp.display_info(tp).reference_count, 2);
        check(
            &mut lisp,
            &[
                ("(delete-frame g t)", "nil"),
                (
                    "(list (frame-live-p g) (memq g (frame-list)) (terminal-live-p tp))",
                    "(nil nil pgtk)",
                ),
            ],
        );
        let mut closed = Vec::new();
        while let Ok(command) = command_rx.try_recv() {
            if let Command::CloseFrame { frame } = command {
                closed.push(frame);
            }
        }
        assert_eq!(closed, vec![g_id]);
        assert!(!emacs_gui::pgtkterm::has_output(&lisp, g));
        assert_eq!(lisp.display_info(tp).reference_count, 1);
        check(
            &mut lisp,
            &[
                (
                    "(x-close-connection tp)",
                    "error: (error \"Display still has frames on it\")",
                ),
                ("(delete-frame f t)", "nil"),
                ("(terminal-live-p tp)", "pgtk"),
                ("(length (x-display-list))", "1"),
            ],
        );
        assert_eq!(lisp.terminal(tp).reference_count, 1);
        assert_eq!(lisp.display_info(tp).reference_count, 0);
        check(
            &mut lisp,
            &[
                ("(x-close-connection tp)", "nil"),
                ("(list (terminal-live-p tp) (x-display-list))", "(nil nil)"),
            ],
        );
    });
}

#[test]
fn deleting_a_pgtk_terminal_deletes_its_frames_without_running_lisp() {
    on_big_stack(|| {
        let mut lisp = connected(Display::new(fmpsc::unbounded().0, mpsc::channel().1));
        let initial_kboard = lisp.initial_kboard;
        check(
            &mut lisp,
            &[
                (
                    "(progn (setq f (x-create-frame '((name . \"a\") (width . 20) (height . 5))) tp (frame-terminal f) deleted nil dt nil) nil)",
                    "nil",
                ),
                (
                    "(progn (setq delete-frame-functions (list (function (lambda (fr) (setq deleted fr)))) delete-terminal-functions (list (function (lambda (tm) (setq dt tm))))) nil)",
                    "nil",
                ),
            ],
        );
        assert_ne!(lisp.current_kboard, initial_kboard);
        check(
            &mut lisp,
            &[
                ("(delete-terminal tp)", "nil"),
                (
                    "(list (frame-live-p f) (terminal-live-p tp) (x-display-list) deleted (eq dt tp) (length (terminal-list)))",
                    "(nil nil nil nil t 1)",
                ),
            ],
        );
        let f = value_of(&mut lisp, "f");
        let pending = lisp.pending_funcalls;
        let hooks: Vec<(Value, Value, Value)> = (0..2)
            .map(|i| {
                let call = lisp.nth(i, pending);
                (lisp.nth(0, call), lisp.nth(1, call), lisp.nth(2, call))
            })
            .collect();
        assert_eq!(
            hooks,
            vec![
                (
                    sym::RUN_HOOK_WITH_ARGS,
                    sym::AFTER_DELETE_FRAME_FUNCTIONS,
                    f
                ),
                (sym::RUN_HOOK_WITH_ARGS, sym::DELETE_FRAME_FUNCTIONS, f),
            ]
        );
        assert_eq!(lisp.current_kboard, initial_kboard);
    });
}
