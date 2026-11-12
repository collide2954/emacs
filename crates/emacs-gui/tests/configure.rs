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
fn a_configure_notice_resizes_split_windows() {
    on_big_stack(|| {
        let (mut lisp, f, tx) = frame();
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (fset 'record-window-buffer (function (lambda (w) nil))) (fset 'window--resize-root-window (function (lambda (window delta horizontal ignore pixelwise) (let ((top (window-top-child window))) (if horizontal (let ((size (+ (window-pixel-width window) delta))) (set-window-new-pixel window size) (set-window-new-pixel top size) (set-window-new-pixel (window-next-sibling top) size)) (set-window-new-pixel window (+ (window-pixel-height window) delta)) (set-window-new-pixel top (window-pixel-height top)) (set-window-new-pixel (window-next-sibling top) (+ (window-pixel-height (window-next-sibling top)) delta))))))) (setq w (frame-root-window f) h (window-pixel-height w) ww (window-pixel-width w) half (/ h 2)) (set-window-new-pixel w (- h half)) (set-window-new-normal w 0.5) (setq n (split-window-internal w half nil 0.5)) (list (window-live-p n) (eq (window-parent w) (frame-root-window f))))"
            ),
            "(t t)"
        );
        let id = lisp.frame(f).id as u64;
        let g = lisp.frame(f).geometry.clone();
        let (width, height) = (
            g.pixel_width + 3 * g.column_width,
            g.pixel_height + 2 * g.line_height,
        );
        tx.send(Notice::Configure {
            frame: id,
            width: width as u32,
            height: height as u32,
            scale: 1.0,
        })
        .unwrap();
        lisp.gobble_input().unwrap();
        let check = format!(
            "(let ((r (frame-root-window f))) (list (= (window-pixel-height r) (+ h {lh})) (= (window-pixel-height w) (- h half)) (= (window-pixel-height n) (+ half {lh})) (= (window-pixel-top n) (+ (window-pixel-top w) (window-pixel-height w))) (= (window-pixel-width r) (+ ww {cw})) (= (window-pixel-width w) (window-pixel-width r)) (= (window-pixel-width n) (window-pixel-width r)) (= (window-pixel-top (minibuffer-window f)) (+ (window-pixel-top r) (window-pixel-height r)))))",
            lh = 2 * g.line_height,
            cw = 3 * g.column_width
        );
        assert_eq!(eval(&mut lisp, &check), "(t t t t t t t t)");
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
