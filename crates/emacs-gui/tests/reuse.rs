use std::sync::mpsc;

use emacs_core::dispextern::TextPos;
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

const LINES: &str =
    "(let ((i 0)) (while (< i 20) (insert (format \"\u{e9}%c\\n\" (+ ?a i))) (setq i (1+ i))))";

fn line_start(line: i64) -> i64 {
    3 * line + 1
}

struct Frame {
    lisp: Lisp,
    f: Value,
    w: Value,
}

fn frame_showing(setup: &str) -> Frame {
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
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"r\") (width . 20) (height . 8)))) (select-frame f) (framep f))"
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
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (set-window-buffer nil (get-buffer-create \"r\")) (set-buffer \"r\") {setup} nil)"
            )
        ),
        "nil"
    );
    lisp.adjust_frame_glyphs(f);
    let fr = lisp.frame_mut(f);
    fr.fonts_changed = false;
    fr.flags.visible = true;
    fr.flags.garbaged = false;
    Frame { lisp, f, w }
}

impl Frame {
    fn byte(&self, charpos: i64) -> i64 {
        let buffer = self.lisp.window(self.w).contents;
        self.lisp.buffer(buffer).text.char_to_byte(charpos as usize) as i64
    }

    fn clear_desired(&mut self) {
        emacs_core::dispnew::matrix::clear_glyph_matrix(
            self.lisp
                .window_matrix_mut(self.w, WhichMatrix::Desired)
                .unwrap(),
        );
    }

    fn finish(&mut self) {
        self.lisp.window_mut(self.w).flags.must_be_updated_p = true;
        self.lisp.update_frame(self.f, false).unwrap();
        let root = self.lisp.frame(self.f).root_window;
        self.lisp.mark_window_display_accurate(root, true);
        self.lisp.windows.windows_or_buffers_changed = 0;
        self.lisp.windows.update_mode_lines = 0;
        self.lisp.frame_mut(self.f).flags.cursor_type_changed = false;
    }

    fn redisplay_from(&mut self, start: i64) {
        self.clear_desired();
        let pos = TextPos::new(start, self.byte(start));
        assert_eq!(self.lisp.try_window(self.w, pos, 0).unwrap(), 1);
        self.finish();
    }

    fn set_start(&mut self, start: i64) {
        let (marker, buffer) = (
            self.lisp.window(self.w).start,
            self.lisp.window(self.w).contents,
        );
        self.lisp
            .set_marker_position(marker, start as usize, buffer);
    }

    fn visible_starts(&mut self) -> Vec<i64> {
        let yb = self.lisp.window_box_height_no_mode_line(self.w) as i32;
        let m = self
            .lisp
            .window_matrix(self.w, WhichMatrix::Current)
            .unwrap();
        m.rows
            .iter()
            .take(m.nrows as usize - 1)
            .take_while(|r| r.enabled_p && r.displays_text_p && r.y + r.height <= yb)
            .map(|r| r.start_charpos())
            .collect()
    }
}

fn consecutive_from(first_line: i64, n: usize) -> Vec<i64> {
    (0..n as i64).map(|k| line_start(first_line + k)).collect()
}

#[test]
fn reusing_the_current_matrix_when_scrolling_forward() {
    on_big_stack(|| {
        let mut fr = frame_showing(&format!("{LINES} (goto-char {})", line_start(5)));
        fr.redisplay_from(1);
        let before = fr.visible_starts();
        assert_eq!(before, consecutive_from(0, before.len()));
        fr.set_start(line_start(2));
        assert!(fr.lisp.try_window_reusing_current_matrix(fr.w).unwrap());
        let current = fr.lisp.window_matrix(fr.w, WhichMatrix::Current).unwrap();
        assert_eq!(current.rows[0].start_charpos(), line_start(2));
        assert_eq!(current.rows[0].y, 0);
        assert_eq!(fr.lisp.window(fr.w).cursor.vpos, 3);
        fr.finish();
        let after = fr.visible_starts();
        assert_eq!(after, consecutive_from(2, before.len()));
    });
}

#[test]
fn reusing_the_current_matrix_when_scrolling_back() {
    on_big_stack(|| {
        let mut fr = frame_showing(&format!("{LINES} (goto-char {})", line_start(5)));
        fr.redisplay_from(line_start(4));
        let before = fr.visible_starts();
        fr.set_start(line_start(1));
        assert!(fr.lisp.try_window_reusing_current_matrix(fr.w).unwrap());
        let desired = fr.lisp.window_matrix(fr.w, WhichMatrix::Desired).unwrap();
        assert!(desired.rows[0].enabled_p && desired.rows[2].enabled_p);
        assert_eq!(desired.rows[0].start_charpos(), line_start(1));
        assert!(!desired.rows[3].enabled_p);
        assert_eq!(fr.lisp.window(fr.w).cursor.vpos, 4);
        fr.finish();
        assert_eq!(fr.visible_starts(), consecutive_from(1, before.len()));
    });
}

#[test]
fn try_window_id_redisplays_only_a_changed_line() {
    on_big_stack(|| {
        let mut fr = frame_showing(&format!(
            "(setq bidi-paragraph-direction 'left-to-right) {LINES} (goto-char {}) (insert \"Z\") (delete-region (1- (point)) (point))",
            line_start(3)
        ));
        fr.redisplay_from(1);
        let n = fr.visible_starts().len();
        eval(&mut fr.lisp, "(insert \"X\")");
        fr.clear_desired();
        assert_eq!(fr.lisp.try_window_id(fr.w).unwrap(), 3);
        let desired = fr.lisp.window_matrix(fr.w, WhichMatrix::Desired).unwrap();
        assert!(desired.rows[3].enabled_p);
        assert!(!desired.rows[2].enabled_p && !desired.rows[4].enabled_p);
        assert_eq!(fr.lisp.window(fr.w).cursor.vpos, 3);
        fr.finish();
        let mut expected = consecutive_from(0, n);
        for start in expected.iter_mut().skip(4) {
            *start += 1;
        }
        assert_eq!(fr.visible_starts(), expected);
    });
}

#[test]
fn try_window_id_scrolls_rows_up_after_a_deleted_line() {
    on_big_stack(|| {
        let mut fr = frame_showing(&format!(
            "(setq bidi-paragraph-direction 'left-to-right) {LINES} (goto-char {})",
            line_start(3)
        ));
        fr.redisplay_from(1);
        let n = fr.visible_starts().len();
        eval(
            &mut fr.lisp,
            &format!("(delete-region {} {})", line_start(3), line_start(4)),
        );
        fr.clear_desired();
        assert_eq!(fr.lisp.try_window_id(fr.w).unwrap(), 3);
        fr.finish();
        assert_eq!(fr.visible_starts(), consecutive_from(0, n));
        let current = fr.lisp.window_matrix(fr.w, WhichMatrix::Current).unwrap();
        assert_eq!(
            current.rows[3].area_glyphs(1)[1].u,
            emacs_core::dispextern::GlyphU::Ch(u32::from(b'e'))
        );
    });
}
