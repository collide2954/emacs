use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
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

fn frame() -> (Lisp, Value, mpsc::Sender<Notice>) {
    let (tx, notices) = mpsc::channel();
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(&mut lisp, Display::new(fmpsc::unbounded().0, notices));
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    eval(
        &mut lisp,
        "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"c\") (width . 20) (height . 5)))))",
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    (lisp, f, tx)
}

fn surface_size(id: u64) -> Option<(u32, u32)> {
    emacs_gui::with_display(|d| d.surface(id).map(|s| (s.width(), s.height()))).flatten()
}

#[test]
fn a_configure_notice_resizes_the_frame_and_its_surface() {
    on_big_stack(|| {
        let (mut lisp, f, tx) = frame();
        let id = lisp.frame(f).id as u64;
        let g = lisp.frame(f).geometry.clone();
        let (width, height) = (
            g.pixel_width + 3 * g.column_width,
            g.pixel_height + 2 * g.line_height,
        );
        lisp.frame_mut(f).flags.garbaged = false;
        tx.send(Notice::Configure {
            frame: id,
            width: width as u32,
            height: height as u32,
            scale: 1.25,
        })
        .unwrap();
        lisp.gobble_input().unwrap();
        let after = lisp.frame(f).geometry.clone();
        assert_eq!((after.pixel_width, after.pixel_height), (width, height));
        assert_eq!(
            (after.text_cols, after.text_lines),
            (g.text_cols + 3, g.text_lines + 2)
        );
        assert!(lisp.frame(f).flags.garbaged);
        assert_eq!(surface_size(id), Some((width as u32, height as u32)));

        lisp.frame_mut(f).flags.garbaged = false;
        tx.send(Notice::Configure {
            frame: id,
            width: width as u32,
            height: height as u32,
            scale: 2.0,
        })
        .unwrap();
        lisp.gobble_input().unwrap();
        assert!(!lisp.frame(f).flags.garbaged);
        assert_eq!(lisp.frame(f).geometry.pixel_width, width);
    });
}

#[test]
fn a_configure_notice_for_an_unknown_window_is_ignored() {
    on_big_stack(|| {
        let (mut lisp, f, tx) = frame();
        let id = lisp.frame(f).id as u64;
        let before = surface_size(id);
        let width = lisp.frame(f).geometry.pixel_width;
        tx.send(Notice::Configure {
            frame: id + 1000,
            width: 999,
            height: 999,
            scale: 1.0,
        })
        .unwrap();
        lisp.gobble_input().unwrap();
        assert_eq!(lisp.frame(f).geometry.pixel_width, width);
        assert_eq!(surface_size(id), before);
    });
}
