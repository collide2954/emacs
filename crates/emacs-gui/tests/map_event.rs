use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
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

fn window_commands(commands: &mut fmpsc::UnboundedReceiver<Command>) -> Vec<String> {
    let mut out = Vec::new();
    while let Ok(command) = commands.try_recv() {
        match command {
            Command::SetFullscreen { frame, on } => out.push(format!("fullscreen {frame} {on}")),
            Command::SetMaximized { frame, on } => out.push(format!("maximized {frame} {on}")),
            _ => {}
        }
    }
    out
}

fn frame_with(
    parameters: &str,
) -> (
    Lisp,
    Value,
    mpsc::Sender<Notice>,
    fmpsc::UnboundedReceiver<Command>,
) {
    let (tx, notices) = mpsc::channel();
    let (commands, mut receiver) = fmpsc::unbounded();
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(&mut lisp, Display::new(commands, notices));
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    eval(
        &mut lisp,
        &format!(
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"m\") (width . 20) (height . 5) {parameters}))))"
        ),
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    window_commands(&mut receiver);
    (lisp, f, tx, receiver)
}

#[test]
fn mapping_a_frame_makes_it_visible_and_settles_fullscreen() {
    on_big_stack(|| {
        let (mut lisp, f, tx, mut commands) = frame_with("");
        let id = lisp.frame(f).id as u64;
        assert_eq!(eval(&mut lisp, "(frame-visible-p f)"), "nil");
        lisp.windows.windows_or_buffers_changed = 0;
        tx.send(Notice::Map { frame: id }).unwrap();
        lisp.gobble_input().unwrap();
        assert_eq!(eval(&mut lisp, "(frame-visible-p f)"), "t");
        assert_eq!(eval(&mut lisp, "(frame-parameter f 'fullscreen)"), "nil");
        assert_ne!(lisp.windows.windows_or_buffers_changed, 0);
        assert_eq!(
            window_commands(&mut commands),
            [
                format!("fullscreen {id} false"),
                format!("maximized {id} false")
            ]
        );
        tx.send(Notice::Map { frame: id }).unwrap();
        lisp.gobble_input().unwrap();
        assert!(window_commands(&mut commands).is_empty());
        assert_eq!(eval(&mut lisp, "(input-pending-p t)"), "nil");
    });
}

#[test]
fn a_maximized_frame_is_maximized_when_first_mapped() {
    on_big_stack(|| {
        let (mut lisp, f, tx, mut commands) = frame_with("(fullscreen . maximized)");
        let id = lisp.frame(f).id as u64;
        tx.send(Notice::Map { frame: id }).unwrap();
        lisp.gobble_input().unwrap();
        assert_eq!(
            window_commands(&mut commands),
            [
                format!("fullscreen {id} false"),
                format!("maximized {id} true")
            ]
        );
        assert_eq!(
            eval(&mut lisp, "(frame-parameter f 'fullscreen)"),
            "maximized"
        );
    });
}

#[test]
fn mapping_an_iconified_frame_deiconifies_it() {
    on_big_stack(|| {
        let (mut lisp, f, tx, _commands) = frame_with("");
        let id = lisp.frame(f).id as u64;
        let frame = lisp.prin1_to_string(f);
        lisp.frame_mut(f).flags.iconified = true;
        tx.send(Notice::Map { frame: id }).unwrap();
        lisp.gobble_input().unwrap();
        assert!(!lisp.frame(f).flags.iconified);
        assert_eq!(eval(&mut lisp, "(frame-visible-p f)"), "t");
        let (event, _, _, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
        assert_eq!(
            lisp.prin1_to_string(event),
            format!("(make-frame-visible ({frame}))")
        );
    });
}
