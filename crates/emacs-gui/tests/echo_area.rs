use std::sync::mpsc;

use emacs_core::dispextern::{GlyphU, TEXT_AREA};
use emacs_core::dispnew::matrix::WhichMatrix;
use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
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

fn gui_frame() -> (Lisp, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"e\") (width . 30) (height . 8)))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.adjust_frame_glyphs(f);
    let fr = lisp.frame_mut(f);
    fr.fonts_changed = false;
    fr.flags.visible = true;
    fr.flags.garbaged = false;
    (lisp, f)
}

fn mini_row_text(lisp: &Lisp, f: Value, which: WhichMatrix) -> String {
    let w = lisp.frame(f).minibuffer_window;
    let m = lisp.window_matrix(w, which).unwrap();
    m.rows[0]
        .area_glyphs(TEXT_AREA)
        .iter()
        .filter_map(|g| match g.u {
            GlyphU::Ch(c) => char::from_u32(c),
            _ => None,
        })
        .collect()
}

#[test]
fn a_message_lands_in_the_echo_area_buffer_and_on_screen() {
    on_big_stack(|| {
        let (mut lisp, f) = gui_frame();
        assert_eq!(eval(&mut lisp, "(message \"hello %d\" 42)"), "\"hello 42\"");
        assert_eq!(eval(&mut lisp, "(current-message)"), "\"hello 42\"");
        let buffer = lisp.xdisp.echo_area_buffer[0];
        let name = lisp.buffer(buffer).name;
        let name = lisp.prin1_to_string(name);
        assert!(name.starts_with("\" *Echo Area "), "{name}");
        assert_eq!(
            eval(
                &mut lisp,
                &format!("(save-current-buffer (set-buffer {name}) (buffer-string))")
            ),
            "\"hello 42\""
        );
        assert!(mini_row_text(&lisp, f, WhichMatrix::Current).starts_with("hello 42"));
        let echo = lisp.xdisp.echo_area_buffer;
        assert_eq!(echo[0], echo[1]);
        assert_eq!(lisp.xdisp.echo_area_window, lisp.frame(f).minibuffer_window);
    });
}

#[test]
fn an_empty_message_clears_the_echo_area() {
    on_big_stack(|| {
        let (mut lisp, f) = gui_frame();
        eval(&mut lisp, "(message \"gone soon\")");
        assert_eq!(eval(&mut lisp, "(message nil)"), "nil");
        assert_eq!(eval(&mut lisp, "(current-message)"), "nil");
        assert!(lisp.xdisp.echo_area_buffer[0].is_nil());
        assert!(!mini_row_text(&lisp, f, WhichMatrix::Current).contains("gone"));
    });
}

#[test]
fn messages_stack_and_restore() {
    on_big_stack(|| {
        let (mut lisp, _) = gui_frame();
        eval(&mut lisp, "(message \"first\")");
        assert!(lisp.push_message().unwrap());
        eval(&mut lisp, "(message \"second\")");
        assert_eq!(eval(&mut lisp, "(current-message)"), "\"second\"");
        lisp.restore_message().unwrap();
        assert_eq!(eval(&mut lisp, "(current-message)"), "\"first\"");
        emacs_core::xdisp::pop_message_unwind(&mut lisp, Value::NIL).unwrap();
        lisp.check_message_stack();
    });
}
