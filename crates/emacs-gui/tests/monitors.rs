use std::sync::mpsc;

use emacs_core::dispextern::EmacsRectangle;
use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
use emacs_gui::display::Display;
use emacs_gui::monitor::{GdkMonitor, MonitorSource};
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

fn rect(x: i32, y: i32, width: i32, height: i32) -> EmacsRectangle {
    EmacsRectangle {
        x,
        y,
        width,
        height,
    }
}

struct TwoMonitors;

impl MonitorSource for TwoMonitors {
    fn monitors(&self) -> Vec<GdkMonitor> {
        vec![
            GdkMonitor {
                handle: 11,
                geometry: rect(0, 0, 1536, 864),
                workarea: rect(0, 0, 1536, 816),
                width_mm: 344,
                height_mm: 193,
                model: Some("M1".to_string()),
                scale_factor: 1,
                primary: true,
            },
            GdkMonitor {
                handle: 22,
                geometry: rect(1536, 0, 1280, 720),
                workarea: rect(1536, 0, 1280, 700),
                width_mm: 300,
                height_mm: 170,
                model: Some("M2".to_string()),
                scale_factor: 2,
                primary: false,
            },
        ]
    }

    fn monitor_at_window(&self, window: u64) -> isize {
        if window == 77 { 22 } else { 11 }
    }
}

fn with_frame(notices: mpsc::Receiver<Notice>) -> (Lisp, Value) {
    let mut lisp = Lisp::boot(Startup::program(false));
    let mut display = Display::new(fmpsc::unbounded().0, notices);
    display.set_monitor_source(Box::new(TwoMonitors));
    emacs_gui::install(&mut lisp, display);
    lisp.init_display();
    eval(
        &mut lisp,
        "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
    );
    eval(
        &mut lisp,
        "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"m\") (width . 20) (height . 5)))) (select-frame f))",
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    (lisp, f)
}

#[test]
fn display_sizes_come_from_the_monitors() {
    on_big_stack(|| {
        let (_tx, notices) = mpsc::channel();
        let (mut lisp, _f) = with_frame(notices);
        assert_eq!(eval(&mut lisp, "(x-display-pixel-width)"), "5632");
        assert_eq!(eval(&mut lisp, "(x-display-pixel-height)"), "1440");
        assert_eq!(eval(&mut lisp, "(x-display-mm-width)"), "644");
        assert_eq!(eval(&mut lisp, "(x-display-mm-height)"), "193");
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-monitor-scale-factor \"M2\" 1.5)"),
            "1.5"
        );
        assert_eq!(eval(&mut lisp, "(x-display-pixel-width)"), "4224");
        assert_eq!(eval(&mut lisp, "(x-display-pixel-height)"), "1080");
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-monitor-scale-factor \"M2\" 3)"),
            "3"
        );
        assert_eq!(eval(&mut lisp, "(x-display-pixel-width)"), "8448");
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-monitor-scale-factor \"M2\" nil)"),
            "nil"
        );
        assert_eq!(eval(&mut lisp, "(x-display-pixel-width)"), "5632");
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-monitor-scale-factor \"M3\" nil)"),
            "nil"
        );
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-monitor-scale-factor \"M2\" 0)"),
            "error: (error \"Scale factor must be > 0\")"
        );
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-monitor-scale-factor \"M2\" -1.0)"),
            "error: (error \"Scale factor must be > 0\")"
        );
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-monitor-scale-factor \"M2\" 'x)"),
            "error: (wrong-type-argument numberp x)"
        );
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-monitor-scale-factor 1 2)"),
            "error: (wrong-type-argument stringp 1)"
        );
        assert_eq!(
            eval(&mut lisp, "(x-display-pixel-width 'x)"),
            "error: (wrong-type-argument frame-live-p x)"
        );
    });
}

#[test]
fn monitor_attributes_list_the_frames_on_each_monitor() {
    on_big_stack(|| {
        let (tx, notices) = mpsc::channel();
        let (mut lisp, f) = with_frame(notices);
        let frame = lisp.prin1_to_string(f);
        let attributes = |frames_on_1: &str, frames_on_2: &str| {
            format!(
                "(((name . \"M1\") (geometry 0 0 1536 864) (workarea 0 0 1536 816) (mm-size 344 193) (scale-factor . 1.0) (frames{frames_on_1}) (source . \"Gdk\")) ((name . \"M2\") (geometry 3072 0 2560 1440) (workarea 3072 0 2560 1400) (mm-size 300 170) (scale-factor . 2.0) (frames{frames_on_2}) (source . \"Gdk\")))"
            )
        };
        assert_eq!(
            eval(&mut lisp, "(pgtk-display-monitor-attributes-list)"),
            attributes(&format!(" {frame}"), "")
        );
        let id = lisp.frame(f).id as u64;
        tx.send(Notice::Realize {
            frame: id,
            window: 77,
        })
        .unwrap();
        lisp.gobble_input().unwrap();
        assert_eq!(
            eval(&mut lisp, "(pgtk-display-monitor-attributes-list)"),
            attributes("", &format!(" {frame}"))
        );
        assert_eq!(
            eval(&mut lisp, "(pgtk-display-monitor-attributes-list 'x)"),
            "error: (wrong-type-argument frame-live-p x)"
        );
    });
}
