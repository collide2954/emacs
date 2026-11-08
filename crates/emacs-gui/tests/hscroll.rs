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

fn frame_showing(setup: &str) -> (Lisp, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"h\") (width . 30) (height . 8)))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    let w = lisp.frame(f).root_window;
    assert_eq!(
        eval(
            &mut lisp,
            &format!(
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"wide\")) (set-buffer \"wide\") (setq bidi-paragraph-direction 'left-to-right) (setq truncate-lines t) {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    let fr = lisp.frame_mut(f);
    fr.fonts_changed = false;
    fr.flags.visible = true;
    fr.flags.garbaged = false;
    (lisp, w)
}

#[test]
fn point_past_the_right_edge_hscrolls_to_center_it() {
    on_big_stack(|| {
        let (mut lisp, w) = frame_showing(
            "(let ((i 0)) (while (< i 100) (insert \"x\") (setq i (1+ i)))) (insert \"\\nshort\\n\") (goto-char 81)",
        );
        lisp.redisplay_window(w, false).unwrap();
        assert!(lisp.window(w).cursor.vpos >= 0);
        assert!(lisp.hscroll_windows(w).unwrap());
        let cw: i64 = eval(&mut lisp, "(frame-char-width)").parse().unwrap();
        let tw: i64 = eval(&mut lisp, "(window-body-width nil t)")
            .parse()
            .unwrap();
        let expected = (80 * cw - tw / 2) / cw;
        assert_eq!(eval(&mut lisp, "(window-hscroll)"), expected.to_string());
        assert_eq!(eval(&mut lisp, "(window-old-point)"), "81");
    });
}

#[test]
fn no_hscroll_when_point_is_visible_or_mode_is_off() {
    on_big_stack(|| {
        let (mut lisp, w) = frame_showing(
            "(let ((i 0)) (while (< i 100) (insert \"x\") (setq i (1+ i)))) (goto-char 3)",
        );
        lisp.redisplay_window(w, false).unwrap();
        assert!(!lisp.hscroll_windows(w).unwrap());
        assert_eq!(eval(&mut lisp, "(window-hscroll)"), "0");
        eval(
            &mut lisp,
            "(progn (setq auto-hscroll-mode nil) (goto-char 81))",
        );
        lisp.redisplay_window(w, false).unwrap();
        assert!(!lisp.hscroll_windows(w).unwrap());
        assert_eq!(eval(&mut lisp, "(window-hscroll)"), "0");
    });
}
