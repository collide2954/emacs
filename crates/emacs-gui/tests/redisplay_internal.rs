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

fn frame_showing(setup: &str) -> (Lisp, Value, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"i\") (width . 30) (height . 8)))) (select-frame f) (framep f))"
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
                "(progn (set-window-buffer nil (get-buffer-create \"text\")) (set-buffer \"text\") (setq bidi-paragraph-direction 'left-to-right) (setq mode-line-format \"%b\") (let ((i 0)) (while (< i 20) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1) {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    let fr = lisp.frame_mut(f);
    fr.fonts_changed = false;
    fr.flags.visible = true;
    (lisp, f, w)
}

fn row_text(lisp: &Lisp, w: Value, which: WhichMatrix, vpos: usize) -> String {
    let m = lisp.window_matrix(w, which).unwrap();
    m.rows[vpos]
        .area_glyphs(TEXT_AREA)
        .iter()
        .filter_map(|g| match g.u {
            GlyphU::Ch(c) => char::from_u32(c),
            _ => None,
        })
        .collect()
}

fn current_text(lisp: &Lisp, w: Value) -> (String, String, String) {
    let m = lisp.window_matrix(w, WhichMatrix::Current).unwrap();
    let (first, mode_line) = (m.first_text_row(), m.mode_line_row());
    (
        row_text(lisp, w, WhichMatrix::Current, first),
        row_text(lisp, w, WhichMatrix::Current, first + 1),
        row_text(lisp, w, WhichMatrix::Current, mode_line),
    )
}

#[test]
fn a_full_redisplay_fills_the_current_matrix() {
    on_big_stack(|| {
        let (mut lisp, f, w) = frame_showing("");
        lisp.redisplay().unwrap();
        let (first, second, mode_line) = current_text(&lisp, w);
        assert!(first.starts_with("line 0"), "{first:?}");
        assert!(second.starts_with("line 1"), "{second:?}");
        assert!(mode_line.starts_with("text"), "{mode_line:?}");
        assert_eq!(lisp.windows.windows_or_buffers_changed, 0);
        assert_eq!(lisp.windows.update_mode_lines, 0);
        assert!(!lisp.frame(f).flags.redisplay);
        assert!(!lisp.frame(f).flags.garbaged);
        assert!(lisp.window(w).flags.window_end_valid);
        assert_eq!(lisp.window(w).last_point, 1);
        assert!(!lisp.xdisp.redisplaying_p);
    });
}

#[test]
fn an_edit_on_one_line_redisplays_that_line() {
    on_big_stack(|| {
        let (mut lisp, _, w) = frame_showing("");
        lisp.redisplay().unwrap();
        eval(&mut lisp, "(progn (goto-char 3) (insert \"XY\"))");
        lisp.redisplay().unwrap();
        let (first, second, _) = current_text(&lisp, w);
        assert!(first.starts_with("liXYne 0"), "{first:?}");
        assert!(second.starts_with("line 1"), "{second:?}");
        assert_eq!(lisp.window(w).last_point, 5);
    });
}

#[test]
fn moving_point_off_the_window_scrolls() {
    on_big_stack(|| {
        let (mut lisp, _, w) = frame_showing("");
        lisp.redisplay().unwrap();
        eval(&mut lisp, "(goto-char (point-max))");
        lisp.redisplay().unwrap();
        let start: i64 = eval(&mut lisp, "(window-start)").parse().unwrap();
        assert!(start > 1, "{start}");
        assert_eq!(lisp.window(w).last_point, 151);
        assert!(lisp.window(w).cursor.vpos >= 0);
    });
}

#[test]
fn a_message_reaches_the_mini_window() {
    on_big_stack(|| {
        let (mut lisp, f, _) = frame_showing("");
        lisp.redisplay().unwrap();
        eval(&mut lisp, "(message \"hi there\")");
        lisp.redisplay().unwrap();
        let mini = lisp.frame(f).minibuffer_window;
        let text = row_text(&lisp, mini, WhichMatrix::Current, 0);
        assert!(text.starts_with("hi there"), "{text:?}");
    });
}

#[test]
fn pre_redisplay_function_hears_about_each_redisplay() {
    on_big_stack(|| {
        let (mut lisp, _, _) = frame_showing("");
        eval(
            &mut lisp,
            "(progn (setq seen nil) (setq pre-redisplay-function (function (lambda (windows) (setq seen (cons windows seen))))))",
        );
        lisp.redisplay().unwrap();
        lisp.redisplay().unwrap();
        assert_eq!(eval(&mut lisp, "seen"), "(nil t)");
    });
}

#[test]
fn a_window_that_takes_too_long_is_skipped_and_reported() {
    on_big_stack(|| {
        let (mut lisp, _, w) = frame_showing("(setq max-redisplay-ticks 5)");
        lisp.redisplay().unwrap();
        let b = lisp.window(w).contents;
        assert_eq!(
            lisp.buffer(b).display_error_modiff,
            lisp.buffer(b).text.modiff
        );
        assert_eq!(
            eval(&mut lisp, "delayed-warnings-list"),
            "((error \"Window showing buffer text takes too long to redisplay\"))"
        );
        assert!(!lisp.xdisp.redisplaying_p);
    });
}

#[test]
fn redisplay_with_nothing_changed_keeps_the_display() {
    on_big_stack(|| {
        let (mut lisp, _, w) = frame_showing("");
        lisp.redisplay().unwrap();
        lisp.redisplay().unwrap();
        let (first, _, mode_line) = current_text(&lisp, w);
        assert!(first.starts_with("line 0"), "{first:?}");
        assert!(mode_line.starts_with("text"), "{mode_line:?}");
    });
}
