use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_gui::display::Display;
use emacs_gui::protocol::{Command, Notice};
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

#[test]
fn invisible_frames_hide_their_windows() {
    on_big_stack(|| {
        let (tx, notices) = mpsc::channel();
        let (commands_tx, mut commands) = fmpsc::unbounded();
        let mut lisp = Lisp::boot(Startup::program(false));
        emacs_gui::install(&mut lisp, Display::new(commands_tx, notices));
        lisp.init_display();
        eval(
            &mut lisp,
            "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
        );
        eval(
            &mut lisp,
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"v\") (width . 20) (height . 5)))))",
        );
        let symbol = lisp.intern("f");
        let f = lisp.find_symbol_value(symbol);
        let id = lisp.frame(f).id as u64;
        tx.send(Notice::Map { frame: id }).unwrap();
        lisp.gobble_input().unwrap();
        assert_eq!(eval(&mut lisp, "(frame-visible-p f)"), "t");
        let visibility = |commands: &mut fmpsc::UnboundedReceiver<Command>| {
            let mut out = Vec::new();
            while let Ok(command) = commands.try_recv() {
                if let Command::SetVisible { frame, on } = command {
                    out.push((frame, on));
                }
            }
            out
        };
        visibility(&mut commands);
        eval(&mut lisp, "(make-frame-invisible f t)");
        assert_eq!(eval(&mut lisp, "(frame-visible-p f)"), "nil");
        assert_eq!(visibility(&mut commands), [(id, false)]);
        eval(&mut lisp, "(make-frame-invisible f t)");
        assert_eq!(visibility(&mut commands), []);
        eval(&mut lisp, "(make-frame-visible f)");
        assert_eq!(visibility(&mut commands), [(id, true)]);
        tx.send(Notice::Map { frame: id }).unwrap();
        lisp.gobble_input().unwrap();
        assert_eq!(eval(&mut lisp, "(frame-visible-p f)"), "t");
    });
}
