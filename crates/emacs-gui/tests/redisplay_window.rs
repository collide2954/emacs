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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"r\") (width . 30) (height . 8)))) (select-frame f) (framep f))"
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
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"lines\")) (set-buffer \"lines\") (setq bidi-paragraph-direction 'left-to-right) (setq mode-line-format \"%b\") (let ((i 0)) (while (< i 20) (insert (format \"line %d\\n\" i)) (setq i (1+ i)))) (goto-char 1) {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    let fr = lisp.frame_mut(f);
    fr.fonts_changed = false;
    fr.flags.visible = true;
    fr.flags.garbaged = false;
    (lisp, f, w)
}

fn row_text(lisp: &Lisp, w: Value, vpos: usize) -> String {
    let m = lisp.window_matrix(w, WhichMatrix::Desired).unwrap();
    m.rows[vpos]
        .area_glyphs(TEXT_AREA)
        .iter()
        .filter_map(|g| match g.u {
            GlyphU::Ch(c) => char::from_u32(c),
            _ => None,
        })
        .collect()
}

fn window_start(lisp: &mut Lisp) -> String {
    eval(lisp, "(window-start)")
}

#[test]
fn displays_the_text_and_the_mode_line() {
    on_big_stack(|| {
        let (mut lisp, _, w) = frame_showing("");
        lisp.redisplay_window(w, false).unwrap();
        let (first, mode_line) = {
            let m = lisp.window_matrix(w, WhichMatrix::Desired).unwrap();
            (m.first_text_row(), m.mode_line_row())
        };
        assert!(row_text(&lisp, w, first).starts_with("line 0"));
        assert!(row_text(&lisp, w, first + 1).starts_with("line 1"));
        let m = lisp.window_matrix(w, WhichMatrix::Desired).unwrap();
        assert!(m.rows[mode_line].enabled_p && m.rows[mode_line].mode_line_p);
        assert!(row_text(&lisp, w, mode_line).starts_with("lines"));
        assert_eq!(window_start(&mut lisp), "1");
        assert_eq!(lisp.window(w).cursor.vpos, 0);
        assert!(lisp.window(w).flags.must_be_updated_p);
    });
}

#[test]
fn recenters_when_point_is_out_of_view() {
    on_big_stack(|| {
        let (mut lisp, _, w) = frame_showing("(goto-char (point-max))");
        lisp.redisplay_window(w, false).unwrap();
        let start: i64 = window_start(&mut lisp).parse().unwrap();
        assert!(start > 1, "{start}");
        assert!(lisp.window(w).cursor.vpos >= 0);
        assert_eq!(eval(&mut lisp, "(pos-visible-in-window-p (point))"), "t");
        assert_eq!(eval(&mut lisp, "(point)"), "151");
    });
}

#[test]
fn keeps_a_forced_window_start_and_moves_point_into_view() {
    on_big_stack(|| {
        let (mut lisp, _, w) = frame_showing("(set-window-start nil 15)");
        lisp.redisplay_window(w, false).unwrap();
        assert_eq!(window_start(&mut lisp), "15");
        let point: i64 = eval(&mut lisp, "(point)").parse().unwrap();
        assert!(point > 15, "{point}");
        assert_eq!(eval(&mut lisp, "(bolp)"), "t");
        assert_eq!(eval(&mut lisp, "(pos-visible-in-window-p (point))"), "t");
        assert!(lisp.window(w).cursor.vpos >= 0);
    });
}

#[test]
fn line_heights_come_from_the_current_matrix() {
    on_big_stack(|| {
        let (mut lisp, _, w) =
            frame_showing("(setq tab-line-format \"t\") (setq header-line-format \"h\")");
        {
            let m = lisp.window_matrix_mut(w, WhichMatrix::Current).unwrap();
            m.tab_line_p = true;
            let (mode_line, header_line) = (m.mode_line_row(), m.header_line_row());
            m.rows[0].height = 23;
            m.rows[header_line].height = 29;
            m.rows[mode_line].height = 37;
        }
        let win = lisp.window_mut(w);
        win.mode_line_height = -1;
        win.header_line_height = -1;
        win.tab_line_height = -1;
        assert_eq!(eval(&mut lisp, "(window-mode-line-height)"), "37");
        assert_eq!(eval(&mut lisp, "(window-header-line-height)"), "29");
        assert_eq!(eval(&mut lisp, "(window-tab-line-height)"), "23");
    });
}

#[test]
fn sets_the_frame_title_from_frame_title_format() {
    on_big_stack(|| {
        let (mut lisp, f, w) = frame_showing(
            "(modify-frame-parameters f '((name))) (setq frame-title-format \"%b title\")",
        );
        lisp.window_mut(w).flags.update_mode_line = true;
        lisp.redisplay_window(w, false).unwrap();
        assert_eq!(
            eval(&mut lisp, "(frame-parameter f 'name)"),
            "\"lines title\""
        );
        assert!(!lisp.frame(f).flags.explicit_name);
    });
}
