mod common;

use common::{eval, on_big_stack};
use emacs_core::lisp::Lisp;

const STUBS: &str = "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'internal-echo-keystrokes-prefix (function (lambda () nil))) (fset 'internal-timer-start-idle (function (lambda () nil))) (fset 'undo-auto--add-boundary (function (lambda () nil))) (fset 'undo-auto-amalgamate (function (lambda () nil))) (setq command-error-function (function (lambda (data context signal) (setq log (cons (list 'err data) log))))) (fset 'command-execute (function (lambda (cmd &rest _) (call-interactively cmd)))) (fset 'record-window-buffer (function (lambda (w) (let ((b (window-buffer w))) (if (minibufferp b) (set-window-prev-buffers w (cons (list b (copy-marker (window-start w)) (copy-marker (window-point w))) (delq (assq b (window-prev-buffers w)) (window-prev-buffers w))))))))) (fset 'minibuffer-mode (function (lambda () (kill-all-local-variables) (setq log (cons (list 'mode (buffer-name)) log))))) (fset 'minibuffer-inactive-mode (function (lambda () (kill-all-local-variables)))) (fset 'exit-minibuffer (function (lambda () (interactive) (throw 'exit nil)))) (fset 'add-to-history (function (lambda (var elt &rest _) (set var (cons elt (symbol-value var)))))) (setq g (make-sparse-keymap)) (use-global-map g) (setq mmap (make-sparse-keymap)) (define-key mmap [13] 'exit-minibuffer) (define-key mmap [97] 'self-insert-command) (define-key mmap [98] 'self-insert-command) (define-key mmap [99] 'self-insert-command) (define-key mmap [57] 'self-insert-command) (define-key mmap [120] 'self-insert-command) (define-key mmap [121] 'self-insert-command) (setq log nil) nil)";

fn reader() -> Lisp {
    let mut lisp = Lisp::new_with(false);
    assert_eq!(eval(&mut lisp, STUBS), "nil");
    lisp
}

fn check(lisp: &mut Lisp, cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(eval(lisp, source), *expected, "evaluating {source}");
    }
}

#[test]
fn reads_typed_input_in_the_minibuffer() {
    on_big_stack(|| {
        let mut lisp = reader();
        check(
            &mut lisp,
            &[(
                "(progn (setq minibuffer-setup-hook (list (function (lambda () (setq log (cons (list 'setup (minibuffer-depth) (buffer-name) (minibuffer-prompt) (buffer-substring-no-properties (point-min) (point-max)) (point) (text-properties-at 1) (minibufferp) (eq (selected-window) (minibuffer-window)) (eq (active-minibuffer-window) (minibuffer-window)) (eq (current-local-map) mmap) (minibuffer-prompt-end)) log)))))) (setq minibuffer-exit-hook (list (function (lambda () (setq log (cons (list 'exit (buffer-name) (minibuffer-contents)) log)))))) (setq unread-command-events (list 97 98 99 13) minibuffer-history nil) (let ((w (selected-window)) (b (current-buffer))) (list (read-from-minibuffer \"Name: \" nil mmap) (nreverse log) minibuffer-history (minibuffer-depth) (eq w (selected-window)) (eq b (current-buffer)) (buffer-name (window-buffer (minibuffer-window))) (save-current-buffer (set-buffer (get-buffer \" *Minibuf-1*\")) (buffer-string)))))",
                "(\"abc\" ((mode \" *Minibuf-1*\") (setup 1 \" *Minibuf-1*\" \"Name: \" \"Name: \" 7 (read-only t field t rear-nonsticky t front-sticky t) t t t t 7) (exit \" *Minibuf-1*\" \"abc\")) (\"abc\") 0 t t \" *Minibuf-0*\" \"\")",
            )],
        );
    });
}

#[test]
fn initial_contents_defaults_and_history() {
    on_big_stack(|| {
        let mut lisp = reader();
        check(
            &mut lisp,
            &[(
                "(progn (setq unread-command-events (list 57 13) minibuffer-history nil) (list (read-from-minibuffer \"E: \" '(\"(+ 1 2)\" . 4) mmap t) (progn (setq unread-command-events (list 13)) (read-from-minibuffer \"E: \" nil mmap t nil \"42\")) minibuffer-history (progn (setq unread-command-events (list 13)) (read-from-minibuffer \"E: \" nil mmap nil 'my-hist)) (boundp 'my-hist) my-hist))",
                "((+ 91 2) 42 (\"42\" \"(+ 91 2)\") \"\" t nil)",
            )],
        );
    });
}

#[test]
fn recursive_minibuffers_and_abort() {
    on_big_stack(|| {
        let mut lisp = reader();
        check(
            &mut lisp,
            &[
                (
                    "(progn (fset 'cmd-r (function (lambda () (interactive) (setq log (cons (list 'inner (read-from-minibuffer \"Inner: \" nil mmap) (minibuffer-depth) (minibuffer-prompt) (this-command-keys-vector)) log))))) (define-key mmap [114] 'cmd-r) (define-key mmap [113] 'abort-recursive-edit) (setq command-error-function (function (lambda (data context signal) (setq log (cons (list 'err data) log))))) (setq log nil unread-command-events (list 114 13)) (list (read-from-minibuffer \"Outer: \" nil mmap) (nreverse log)))",
                    "(\"\" ((mode \" *Minibuf-1*\") (err (user-error \"Command attempted to use minibuffer while in minibuffer\"))))",
                ),
                (
                    "(let ((enable-recursive-minibuffers t)) (setq log nil unread-command-events (list 114 120 13 121 13)) (list (read-from-minibuffer \"Outer: \" nil mmap) (nreverse log) (minibuffer-depth)))",
                    "(\"y\" ((mode \" *Minibuf-1*\") (mode \" *Minibuf-2*\") (inner \"x\" 1 \"Outer: \" [114])) 0)",
                ),
                (
                    "(progn (setq w (selected-window) unread-command-events (list 97 113)) (read-from-minibuffer \"Q: \" nil mmap))",
                    "error: (quit)",
                ),
                (
                    "(list (minibuffer-depth) (eq w (selected-window)) (buffer-name (window-buffer (minibuffer-window))) (save-current-buffer (set-buffer (get-buffer \" *Minibuf-1*\")) (buffer-string)))",
                    "(0 t \" *Minibuf-0*\" \"\")",
                ),
            ],
        );
    });
}
