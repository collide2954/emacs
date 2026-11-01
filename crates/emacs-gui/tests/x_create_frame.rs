use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::xfaces::DEFAULT_FACE_ID;
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

#[test]
fn x_create_frame_makes_a_visible_pgtk_frame() {
    on_big_stack(|| {
        let mut lisp = Lisp::boot(Startup::program(false));
        let (commands, mut command_rx) = fmpsc::unbounded();
        let (_notice_tx, notices) = mpsc::channel();
        emacs_gui::install(&mut lisp, Display::new(commands, notices));
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
        let frames_before = eval(&mut lisp, "(length (frame-list))");
        assert_eq!(
            eval(
                &mut lisp,
                "(framep (setq f (x-create-frame '((name . \"test\") (width . 40) (height . 10)))))"
            ),
            "pgtk"
        );
        for (source, expected) in [
            ("(frame-live-p f)", "pgtk"),
            ("(window-system f)", "pgtk"),
            ("(and (memq f (frame-list)) t)", "t"),
            (
                "(length (frame-list))",
                &(frames_before.parse::<i64>().unwrap() + 1).to_string(),
            ),
            ("(frame-parameter f 'name)", "\"test\""),
            ("(frame-parameter f 'font-backend)", "(ftcr)"),
            ("(stringp (frame-parameter f 'font))", "t"),
            ("(frame-parameter f 'foreground-color)", "\"black\""),
            ("(frame-parameter f 'background-color)", "\"white\""),
            ("(frame-parameter f 'visibility)", "nil"),
            ("(frame-text-cols f)", "40"),
            ("(frame-text-lines f)", "10"),
            ("(frame-parameter f 'minibuffer)", "t"),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        let symbol = lisp.intern("f");
        let f = lisp.find_symbol_value(symbol);
        let face = lisp.face_from_id(f, DEFAULT_FACE_ID).unwrap();
        assert_eq!((face.foreground, face.background), (0x000000, 0xFFFFFF));
        assert_eq!(face.font, lisp.frame(f).font);
        let (width, height) = (
            lisp.frame(f).geometry.pixel_width,
            lisp.frame(f).geometry.pixel_height,
        );
        let mut opened = None;
        while let Ok(command) = command_rx.try_recv() {
            if let Command::OpenFrame {
                frame,
                title,
                width,
                height,
            } = command
            {
                opened = Some((frame, title, width, height));
            }
        }
        assert_eq!(
            opened,
            Some((
                lisp.frame(f).id as u64,
                "test".to_string(),
                width as u32,
                height as u32
            ))
        );
        assert_eq!(
            eval(&mut lisp, "(x-create-frame '((name . 5)))"),
            "error: (error \"Invalid frame name--not a string or nil\")"
        );
    });
}
