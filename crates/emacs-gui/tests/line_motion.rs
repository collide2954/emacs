use std::sync::mpsc;

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

fn shown_frame() -> (Lisp, Value, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"l\") (width . 30) (height . 8)))) (select-frame f) (set-window-buffer nil (get-buffer-create \"text\")) (set-buffer \"text\") (setq bidi-paragraph-direction 'left-to-right) (let ((i 0)) (while (< i 20) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    let w = lisp.frame(f).root_window;
    lisp.frame_mut(f).flags.visible = true;
    (lisp, f, w)
}

#[test]
fn face_font_names_the_realized_font() {
    on_big_stack(|| {
        let (mut lisp, _, _) = shown_frame();
        assert_eq!(
            eval(
                &mut lisp,
                "(equal (face-font 'default) (aref (query-font (internal-get-lisp-face-attribute 'default :font f)) 0))"
            ),
            "t"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(equal (face-font 'default f ?a) (face-font 'default))"
            ),
            "t"
        );
        assert_eq!(eval(&mut lisp, "(face-font 'default t)"), "nil");
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (internal-set-lisp-face-attribute 'default :weight 'bold t) (face-font 'default t))"
            ),
            "(bold)"
        );
        assert_eq!(
            eval(&mut lisp, "(face-font 'default f 'x)"),
            "error: (wrong-type-argument characterp x)"
        );
    });
}

#[test]
fn window_vscroll_round_trips_in_lines_and_pixels() {
    on_big_stack(|| {
        let (mut lisp, f, w) = shown_frame();
        let lh = lisp.frame(f).geometry.line_height;
        assert_eq!(eval(&mut lisp, "(window-vscroll)"), "0");
        lisp.redisplay().unwrap();
        let b = lisp.window(w).contents;
        lisp.buffer_mut(b).prevent_redisplay_optimizations_p = false;
        assert_eq!(eval(&mut lisp, "(set-window-vscroll nil 2)"), "2");
        assert_eq!(lisp.window(w).vscroll, -2 * lh);
        assert!(lisp.buffer(b).prevent_redisplay_optimizations_p);
        assert!(lisp.window(w).flags.redisplay);
        assert_eq!(
            eval(&mut lisp, "(window-vscroll nil t)"),
            (2 * lh).to_string()
        );
        assert_eq!(eval(&mut lisp, "(set-window-vscroll nil 5 t)"), "5");
        let expected = lisp.float(5.0 / lh as f64);
        let expected = lisp.prin1_to_string(expected);
        assert_eq!(eval(&mut lisp, "(window-vscroll)"), expected);
        assert_eq!(eval(&mut lisp, "(set-window-vscroll nil -3 t t)"), "0");
        assert!(lisp.window(w).flags.preserve_vscroll_p);
        assert_eq!(
            eval(&mut lisp, "(set-window-vscroll nil 'x)"),
            "error: (wrong-type-argument numberp x)"
        );
    });
}

#[test]
fn line_pixel_height_measures_the_line_at_point() {
    on_big_stack(|| {
        let (mut lisp, f, _) = shown_frame();
        let lh = lisp.frame(f).geometry.line_height;
        assert_eq!(eval(&mut lisp, "(line-pixel-height)"), lh.to_string());
        assert_eq!(
            eval(&mut lisp, "(progn (goto-char 30) (line-pixel-height))"),
            lh.to_string()
        );
    });
}
