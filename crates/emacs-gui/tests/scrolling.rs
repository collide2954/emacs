use std::sync::mpsc;

use emacs_core::dispextern::TextPos;
use emacs_core::dispnew::matrix::WhichMatrix;
use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
use emacs_core::xdisp::scroll::{
    CURSOR_MOVEMENT_MUST_SCROLL, CURSOR_MOVEMENT_SUCCESS, SCROLLING_SUCCESS,
};
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

fn window_showing(setup: &str) -> (Lisp, Value) {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"s\") (width . 20) (height . 8)))) (select-frame f) (framep f))"
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
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"s\")) (set-buffer \"s\") {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    lisp.frame_mut(f).fonts_changed = false;
    (lisp, w)
}

const TWENTY_LINES: &str =
    "(let ((i 0)) (while (< i 20) (insert (format \"%c\\n\" (+ ?a i))) (setq i (1+ i))))";

fn line_start(line: i64) -> i64 {
    2 * line + 1
}

fn full_lines(lisp: &mut Lisp, w: Value) -> i64 {
    lisp.window_box_height(w) / i64::from(lisp.default_line_pixel_height(w))
}

fn start(lisp: &Lisp, w: Value) -> i64 {
    lisp.marker(lisp.window(w).start).charpos as i64
}

fn clear_desired(lisp: &mut Lisp, w: Value) {
    emacs_core::dispnew::matrix::clear_glyph_matrix(
        lisp.window_matrix_mut(w, WhichMatrix::Desired).unwrap(),
    );
}

fn make_display_current(lisp: &mut Lisp, w: Value) {
    let f = lisp.window(w).frame;
    let nrows = lisp.window_matrix(w, WhichMatrix::Desired).unwrap().nrows as usize;
    for vpos in 0..nrows {
        lisp.make_current(w, vpos);
    }
    let root = lisp.frame(f).root_window;
    lisp.mark_window_display_accurate(root, true);
    lisp.windows.windows_or_buffers_changed = 0;
    lisp.windows.update_mode_lines = 0;
}

#[test]
fn scrolling_conservatively_puts_point_on_the_last_full_line() {
    on_big_stack(|| {
        let (mut lisp, w) = window_showing(&format!("{TWENTY_LINES} (goto-char 1)"));
        let lines = full_lines(&mut lisp, w);
        assert!((2..8).contains(&lines));
        assert_eq!(lisp.try_window(w, TextPos::new(1, 1), 0).unwrap(), 1);
        clear_desired(&mut lisp, w);
        eval(&mut lisp, &format!("(goto-char {})", line_start(9)));
        assert_eq!(
            lisp.try_scrolling(w, false, 101, 0, false, false).unwrap(),
            SCROLLING_SUCCESS
        );
        assert_eq!(start(&lisp, w), line_start(9 - lines + 1));
        let win = lisp.window(w);
        assert_eq!((win.cursor.vpos, win.cursor.hpos), (lines - 1, 0));
        clear_desired(&mut lisp, w);
        eval(&mut lisp, &format!("(goto-char {})", line_start(2)));
        assert_eq!(
            lisp.try_scrolling(w, false, 101, 0, false, false).unwrap(),
            SCROLLING_SUCCESS
        );
        assert_eq!(start(&lisp, w), line_start(2));
        assert_eq!(lisp.window(w).cursor.vpos, 0);
    });
}

#[test]
fn cursor_movement_reuses_the_current_matrix() {
    on_big_stack(|| {
        let (mut lisp, w) = window_showing(&format!("{TWENTY_LINES} (goto-char 1)"));
        assert_eq!(lisp.try_window(w, TextPos::new(1, 1), 0).unwrap(), 1);
        make_display_current(&mut lisp, w);
        let startp = TextPos::new(1, 1);
        let mut scroll_step = false;
        eval(&mut lisp, &format!("(goto-char {})", line_start(1) + 1));
        assert_eq!(
            lisp.try_cursor_movement(w, startp, &mut scroll_step)
                .unwrap(),
            CURSOR_MOVEMENT_SUCCESS
        );
        let win = lisp.window(w);
        assert_eq!((win.cursor.vpos, win.cursor.hpos), (1, 1));
        make_display_current(&mut lisp, w);
        eval(&mut lisp, "(goto-char 1)");
        assert_eq!(
            lisp.try_cursor_movement(w, startp, &mut scroll_step)
                .unwrap(),
            CURSOR_MOVEMENT_SUCCESS
        );
        assert_eq!(lisp.window(w).cursor.vpos, 0);
        make_display_current(&mut lisp, w);
        eval(&mut lisp, &format!("(goto-char {})", line_start(15)));
        assert_eq!(
            lisp.try_cursor_movement(w, startp, &mut scroll_step)
                .unwrap(),
            CURSOR_MOVEMENT_MUST_SCROLL
        );
        assert!(!scroll_step);
    });
}

#[test]
fn window_start_moves_to_a_screen_line_of_a_continued_line() {
    on_big_stack(|| {
        let (mut lisp, w) = window_showing("(insert (make-string 45 ?a)) (goto-char 30)");
        let (marker, buffer) = (lisp.window(w).start, lisp.window(w).contents);
        lisp.set_marker_position(marker, 25, buffer);
        assert!(lisp.compute_window_start_on_continuation_line(w).unwrap());
        assert_eq!(start(&lisp, w), 21);
        lisp.set_marker_position(marker, 21, buffer);
        assert!(lisp.compute_window_start_on_continuation_line(w).unwrap());
        assert_eq!(start(&lisp, w), 21);
        lisp.set_marker_position(marker, 1, buffer);
        assert!(!lisp.compute_window_start_on_continuation_line(w).unwrap());
    });
}

#[test]
fn rows_containing_positions_and_cursor_visibility() {
    on_big_stack(|| {
        let (mut lisp, w) = window_showing(&format!("{TWENTY_LINES} (goto-char 1)"));
        assert_eq!(lisp.try_window(w, TextPos::new(1, 1), 0).unwrap(), 1);
        assert_eq!(
            lisp.row_containing_pos(w, line_start(1), WhichMatrix::Desired, 0, None, 0)
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            lisp.row_containing_pos(w, line_start(1) - 1, WhichMatrix::Desired, 0, None, 0)
                .unwrap(),
            Some(0)
        );
        assert_eq!(
            lisp.row_containing_pos(w, line_start(18), WhichMatrix::Desired, 0, None, 0)
                .unwrap(),
            None
        );
        assert!(
            lisp.cursor_row_fully_visible_p(w, false, false, false)
                .unwrap()
        );
        eval(&mut lisp, "(setq make-cursor-line-fully-visible nil)");
        assert!(
            lisp.cursor_row_fully_visible_p(w, false, false, true)
                .unwrap()
        );
    });
}

#[test]
fn window_starts_and_scroll_functions() {
    on_big_stack(|| {
        let (mut lisp, w) = window_showing(
            "(insert \"abcdef\") (put-text-property 1 3 'invisible t) (setq make-window-start-visible t)",
        );
        assert!(!lisp.window_start_acceptable_p(w, 1).unwrap());
        assert!(lisp.window_start_acceptable_p(w, 3).unwrap());
        eval(&mut lisp, "(setq make-window-start-visible nil)");
        assert!(lisp.window_start_acceptable_p(w, 1).unwrap());
        eval(
            &mut lisp,
            "(setq window-scroll-functions (list (function (lambda (win pos) (setq seen (list (eq win (selected-window)) pos))))))",
        );
        let startp = lisp
            .run_window_scroll_functions(w, TextPos::new(4, 4))
            .unwrap();
        assert_eq!(startp.charpos, 4);
        assert_eq!(start(&lisp, w), 4);
        assert_eq!(eval(&mut lisp, "seen"), "(t 4)");
    });
}
