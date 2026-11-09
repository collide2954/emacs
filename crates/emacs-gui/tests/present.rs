use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
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

fn presented(commands: &mut fmpsc::UnboundedReceiver<Command>) -> Vec<u64> {
    let mut frames = Vec::new();
    while let Ok(command) = commands.try_recv() {
        if let Command::Present { frame, .. } = command {
            frames.push(frame);
        }
    }
    frames
}

fn shown_frame() -> (Lisp, Value, fmpsc::UnboundedReceiver<Command>) {
    let (commands, mut receiver) = fmpsc::unbounded();
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(&mut lisp, Display::new(commands, mpsc::channel().1));
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    assert_eq!(
        eval(
            &mut lisp,
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"p\") (width . 30) (height . 8)))) (select-frame f) (set-window-buffer nil (get-buffer-create \"text\")) (set-buffer \"text\") (setq bidi-paragraph-direction 'left-to-right) (insert \"hello\") (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.adjust_frame_glyphs(f);
    let fr = lisp.frame_mut(f);
    fr.fonts_changed = false;
    fr.flags.visible = true;
    presented(&mut receiver);
    (lisp, f, receiver)
}

#[test]
fn a_finished_redisplay_presents_the_frame_once() {
    on_big_stack(|| {
        let (mut lisp, f, mut commands) = shown_frame();
        lisp.redisplay().unwrap();
        assert_eq!(presented(&mut commands), vec![lisp.frame(f).id as u64]);
        assert!(!lisp.buffer_flipping_blocked_p());
    });
}

#[test]
fn a_message_outside_redisplay_presents_the_frame() {
    on_big_stack(|| {
        let (mut lisp, f, mut commands) = shown_frame();
        lisp.redisplay().unwrap();
        presented(&mut commands);
        eval(&mut lisp, "(message \"hi\")");
        assert_eq!(presented(&mut commands), vec![lisp.frame(f).id as u64]);
    });
}
