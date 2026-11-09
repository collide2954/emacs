use std::sync::mpsc;

use emacs_core::dispnew::matrix::WhichMatrix;
use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
use emacs_core::window::CursorPos;
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"g\") (width . 20) (height . 5)))) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    (lisp, f)
}

#[test]
fn creating_a_frame_allocates_its_glyph_matrices() {
    on_big_stack(|| {
        let (lisp, f) = gui_frame();
        assert!(lisp.frame(f).flags.glyphs_initialized_p);
        let w = lisp.frame(f).root_window;
        let lines = lisp.window(w).total_lines as i32;
        let current = lisp.window_matrix(w, WhichMatrix::Current).unwrap();
        assert!(current.nrows >= lines, "{} < {lines}", current.nrows);
        assert!(lisp.window_matrix(w, WhichMatrix::Desired).is_some());
    });
}

#[test]
fn resizing_a_frame_reallocates_them_and_clamps_the_cursor() {
    on_big_stack(|| {
        let (mut lisp, f) = gui_frame();
        let w = lisp.frame(f).root_window;
        lisp.adjust_frame_glyphs(f);
        let before = lisp.window_matrix(w, WhichMatrix::Current).unwrap().nrows;
        let g = lisp.frame(f).geometry.clone();
        lisp.window_mut(w).cursor = CursorPos {
            x: 100_000,
            y: 100_000,
            hpos: 7,
            vpos: 3,
        };
        lisp.frame_mut(f).flags.garbaged = false;
        lisp.change_frame_size(
            f,
            g.pixel_width,
            g.pixel_height + 6 * g.line_height,
            false,
            true,
        )
        .unwrap();
        let after = lisp.window_matrix(w, WhichMatrix::Current).unwrap().nrows;
        assert!(after > before, "{after} <= {before}");
        let c = lisp.window(w).cursor;
        assert_eq!((c.x, c.hpos, c.y, c.vpos), (0, 0, 0, 0));
        assert!(lisp.frame(f).flags.garbaged);
    });
}
