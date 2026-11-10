use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::termhooks::{EventKind, InputEvent};
use emacs_gui::display::Display;
use emacs_gui::protocol::Notice;
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

const STUBS: &str = "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'window--resize-root-window-vertically (function (lambda (window delta pixelwise) (set-window-new-pixel window (+ (window-pixel-height window) delta)) delta))) (fset 'internal-echo-keystrokes-prefix (function (lambda () nil))) (fset 'internal-timer-start-idle (function (lambda () nil))) (fset 'undo-auto--add-boundary (function (lambda () nil))) (fset 'undo-auto-amalgamate (function (lambda () nil))) (setq command-error-function (function (lambda (data context signal) (setq log (cons (list 'err data) log))))) (fset 'command-execute (function (lambda (cmd &rest _) (call-interactively cmd)))) (fset 'record-window-buffer (function (lambda (w) nil))) (fset 'minibuffer-mode (function (lambda () (kill-all-local-variables)))) (fset 'minibuffer-inactive-mode (function (lambda () (kill-all-local-variables)))) (fset 'add-to-history (function (lambda (var elt &rest _) (set var (cons elt (symbol-value var)))))) (setq g (make-sparse-keymap)) (use-global-map g) (setq mmap (make-sparse-keymap)) (define-key mmap [13] (function (lambda () (interactive) (setq log (cons (list 'exit (window-pixel-height (minibuffer-window))) log)) (throw 'exit nil)))) (define-key mmap [109] (function (lambda () (interactive) (message \"one\\ntwo\\nthree\") (setq log (cons (list 'msg (window-pixel-height (minibuffer-window))) log))))) (setq echo-area-clear-hook (list (function (lambda () (setq log (cons (list 'clear (current-message)) log)))))) (setq resize-mini-windows t log nil) nil)";

fn displayed(notices: mpsc::Receiver<Notice>) -> (Lisp, i64, u64) {
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(&mut lisp, Display::new(fmpsc::unbounded().0, notices));
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    assert_eq!(eval(&mut lisp, STUBS), "nil");
    assert_eq!(
        eval(
            &mut lisp,
            "(progn (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"mm\") (width . 30) (height . 20)))) (select-frame f) (setq bidi-paragraph-direction 'left-to-right) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.frame_mut(f).flags.visible = true;
    lisp.redisplay().unwrap();
    let h = eval(&mut lisp, "(frame-char-height f)").parse().unwrap();
    let id = lisp.frame(f).id as u64;
    (lisp, h, id)
}

#[test]
fn the_command_loop_clears_a_timed_message_and_resizes_the_mini_window() {
    on_big_stack(|| {
        let (mut lisp, h, _) = displayed(mpsc::channel().1);
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq minibuffer-message-timeout 0 unread-command-events (list 109 13)) (list (read-from-minibuffer \"Q: \" nil mmap) (nreverse log)))"
            ),
            format!("(\"\" ((msg {}) (clear nil) (exit {})))", 3 * h, h)
        );
    });
}

#[test]
fn reading_a_key_clears_an_untimed_message_and_resizes_the_mini_window() {
    on_big_stack(|| {
        let (tx, notices) = mpsc::channel();
        let (mut lisp, h, id) = displayed(notices);
        tx.send(Notice::Input(InputEvent {
            kind: EventKind::AsciiKeystroke,
            code: 13,
            frame: id,
            ..InputEvent::default()
        }))
        .unwrap();
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq minibuffer-message-timeout nil unread-command-events (list 109)) (list (read-from-minibuffer \"Q: \" nil mmap) (nreverse log)))"
            ),
            format!(
                "(\"\" ((msg {}) (clear \"one\ntwo\nthree\") (exit {})))",
                3 * h,
                h
            )
        );
    });
}
