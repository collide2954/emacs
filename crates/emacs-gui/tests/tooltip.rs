use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use emacs_core::dispextern::EmacsRectangle;
use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
use emacs_gui::display::Display;
use emacs_gui::monitor::{GdkMonitor, MonitorSource};
use emacs_gui::protocol::Command;
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

struct Screens {
    pointer: Arc<Mutex<(i32, i32)>>,
}

impl MonitorSource for Screens {
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

    fn monitor_at_window(&self, _window: u64) -> isize {
        11
    }

    fn pointer_position(&self) -> Option<(i32, i32)> {
        Some(*self.pointer.lock().unwrap())
    }
}

struct Session {
    lisp: Lisp,
    commands: fmpsc::UnboundedReceiver<Command>,
    pointer: Arc<Mutex<(i32, i32)>>,
    frame: u64,
}

#[derive(Debug, PartialEq)]
enum Seen {
    Open(u64, i32, i32, u32, u32),
    Move(u64, i32, i32),
    Visible(u64, bool),
    Close(u64),
    Tooltip(u64, Option<String>),
    Present(u64, u32, u32),
}

impl Session {
    fn new() -> Session {
        let pointer = Arc::new(Mutex::new((300, 200)));
        let (commands_tx, commands) = fmpsc::unbounded();
        let mut display = Display::new(commands_tx, mpsc::channel().1);
        display.set_monitor_source(Box::new(Screens {
            pointer: pointer.clone(),
        }));
        let mut lisp = Lisp::boot(Startup::program(false));
        emacs_gui::install(&mut lisp, display);
        lisp.init_display();
        eval(
            &mut lisp,
            "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)))",
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (fset 'face-set-after-frame-default (function (lambda (&rest _) nil))) (setq timers nil cancelled nil) (fset 'run-at-time (function (lambda (time repeat function &rest _) (let ((timer (list 'timer time repeat function))) (setq timers (cons timer timers)) timer)))) (fset 'cancel-timer (function (lambda (timer) (setq cancelled (cons timer cancelled)) nil))) (fset 'assq-delete-all (function (lambda (key alist) (let ((out nil)) (while alist (if (null (and (consp (car alist)) (eq (car (car alist)) key))) (setq out (cons (car alist) out))) (setq alist (cdr alist))) (nreverse out))))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"main\") (width . 40) (height . 10)))) (select-frame f) (framep f))"
            ),
            "pgtk"
        );
        let symbol = lisp.intern("f");
        let f = lisp.find_symbol_value(symbol);
        lisp.frame_mut(f).flags.visible = true;
        let frame = lisp.frame(f).id as u64;
        let mut session = Session {
            lisp,
            commands,
            pointer,
            frame,
        };
        session.seen();
        session
    }

    fn eval(&mut self, source: &str) -> String {
        eval(&mut self.lisp, source)
    }

    fn seen(&mut self) -> Vec<Seen> {
        let mut out = Vec::new();
        while let Ok(command) = self.commands.try_recv() {
            out.push(match command {
                Command::OpenPopup {
                    frame,
                    x,
                    y,
                    width,
                    height,
                } => Seen::Open(frame, x, y, width, height),
                Command::MoveFrame { frame, x, y } => Seen::Move(frame, x, y),
                Command::SetVisible { frame, on } => Seen::Visible(frame, on),
                Command::CloseFrame { frame } => Seen::Close(frame),
                Command::SetTooltip { frame, text } => Seen::Tooltip(frame, text),
                Command::Present {
                    frame,
                    width,
                    height,
                    ..
                } if frame != self.frame => Seen::Present(frame, width, height),
                _ => continue,
            });
        }
        out
    }

    fn redisplayed(&mut self) -> Vec<Seen> {
        self.lisp.redisplay().unwrap();
        self.seen()
    }

    fn tip_frames(&self) -> Vec<Value> {
        self.lisp
            .frames
            .list
            .iter()
            .copied()
            .filter(|&f| self.lisp.frame_live_p(f) && self.lisp.frame(f).flags.tooltip)
            .collect()
    }

    fn tip(&self) -> Value {
        let tips = self.tip_frames();
        assert_eq!(tips.len(), 1, "one tip frame");
        tips[0]
    }

    fn tip_id(&self) -> u64 {
        self.lisp.frame(self.tip()).id as u64
    }

    fn cell(&self) -> (i64, i64) {
        let g = &self.lisp.frame(self.tip()).geometry;
        (g.column_width, g.line_height)
    }
}

#[test]
fn system_tooltips_set_the_frame_widget_tooltip() {
    on_big_stack(|| {
        let mut s = Session::new();
        let id = s.frame;
        assert_eq!(s.eval("use-system-tooltips"), "t");
        assert_eq!(s.eval("(x-show-tip \"hello\")"), "nil");
        assert_eq!(s.seen(), [Seen::Tooltip(id, Some("hello".into()))]);
        assert_eq!(s.eval("timers"), "((timer 5 nil x-hide-tip))");
        assert_eq!(s.eval("(x-show-tip \"again\" f nil 3)"), "nil");
        assert_eq!(
            s.seen(),
            [
                Seen::Tooltip(id, None),
                Seen::Tooltip(id, Some("again".into()))
            ]
        );
        assert_eq!(s.eval("cancelled"), "((timer 5 nil x-hide-tip))");
        assert_eq!(s.eval("(car timers)"), "(timer 3 nil x-hide-tip)");
        assert_eq!(s.eval("(x-hide-tip)"), "t");
        assert_eq!(s.seen(), [Seen::Tooltip(id, None)]);
        assert_eq!(s.eval("(car cancelled)"), "(timer 3 nil x-hide-tip)");
        assert_eq!(s.eval("(x-hide-tip)"), "nil");
        assert_eq!(s.seen(), []);
        assert_eq!(s.eval("(x-show-tip \"\")"), "nil");
        assert_eq!(s.seen(), [Seen::Tooltip(id, Some(" ".into()))]);
        assert!(s.tip_frames().is_empty());
        for (source, expected) in [
            ("(x-show-tip 5)", "error: (wrong-type-argument stringp 5)"),
            (
                "(x-show-tip \"a\" nil nil -1)",
                "error: (wrong-type-argument wholenump -1)",
            ),
            (
                "(x-show-tip \"a\" nil nil nil 'x)",
                "error: (wrong-type-argument fixnump x)",
            ),
            (
                "(x-show-tip \"a\" nil nil nil nil 'y)",
                "error: (wrong-type-argument fixnump y)",
            ),
            (
                "(x-show-tip \"a\" 'z)",
                "error: (wrong-type-argument frame-live-p z)",
            ),
        ] {
            assert_eq!(s.eval(source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn tip_frames_are_created_moved_and_deleted() {
    on_big_stack(|| {
        let mut s = Session::new();
        let frames_before = s.eval("(length (frame-list))");
        s.eval("(setq use-system-tooltips nil)");
        assert_eq!(s.eval("(x-show-tip \"tip text\")"), "nil");
        let tip = s.tip();
        let tip_id = s.tip_id();
        let (cw, lh) = s.cell();
        let (width, height) = ((9 * cw + 6) as u32, (lh + 6) as u32);
        assert_eq!(s.seen(), [Seen::Open(tip_id, 305, 190, width, height)]);
        assert_eq!(s.redisplayed(), [Seen::Present(tip_id, width, height)]);
        assert_eq!(s.eval("(length (frame-list))"), frames_before);
        let main = s.frame;
        let symbol = s.lisp.intern("tip");
        s.lisp.set_plain_value(symbol, tip);
        for (source, expected) in [
            ("(frame-parameter tip 'tooltip)", "t"),
            ("(frame-parameter tip 'name)", "\"tooltip\""),
            ("(frame-parameter tip 'internal-border-width)", "3"),
            ("(frame-parameter tip 'border-width)", "1"),
            ("(frame-parameter tip 'background-color)", "\"lightyellow\""),
            ("(frame-parameter tip 'border-color)", "\"lightyellow\""),
            ("(frame-parameter tip 'display-type)", "color"),
            ("(frame-visible-p tip)", "t"),
            (
                "(save-current-buffer (set-buffer \" *tip*\") (list (buffer-string) truncate-lines left-margin-width right-margin-width))",
                "(\"tip text\" nil 0 0)",
            ),
            (
                "(eq (window-buffer (frame-root-window tip)) (get-buffer \" *tip*\"))",
                "t",
            ),
            ("timers", "((timer 5 nil x-hide-tip))"),
        ] {
            assert_eq!(s.eval(source), expected, "evaluating {source}");
        }
        let root = s.lisp.frame(tip).root_window;
        assert!(s.lisp.window(root).flags.pseudo_window_p);
        let (tip_pw, tip_ph) = {
            let g = &s.lisp.frame(tip).geometry;
            (g.pixel_width, g.pixel_height)
        };
        assert!(315 + tip_pw <= 1536 && 170 + tip_ph <= 864);
        s.eval("(garbage-collect)");
        assert_eq!(
            s.eval("(x-show-tip \"tip text\" nil nil nil 15 -30)"),
            "nil"
        );
        assert_eq!(s.seen(), [Seen::Move(tip_id, 315, 170)]);
        assert_eq!(s.eval("cancelled"), "((timer 5 nil x-hide-tip))");
        assert_eq!(s.eval("(length timers)"), "2");
        assert_eq!(s.eval("(frame-id tip)"), "nil");
        assert_eq!(s.eval("(x-show-tip \"other\")"), "nil");
        let seen = s.seen();
        let new_id = s.tip_id();
        assert_ne!(s.tip(), tip);
        let width = (6 * cw + 6) as u32;
        assert_eq!(
            seen,
            [
                Seen::Tooltip(main, None),
                Seen::Close(tip_id),
                Seen::Open(new_id, 305, 190, width, height)
            ]
        );
        assert_eq!(s.redisplayed(), [Seen::Present(new_id, width, height)]);
        assert_eq!(s.eval("(frame-live-p tip)"), "nil");
        assert_eq!(s.eval("(x-hide-tip)"), "t");
        assert_eq!(s.seen(), [Seen::Tooltip(main, None), Seen::Close(new_id)]);
        assert!(s.tip_frames().is_empty());
        assert_eq!(s.eval("(x-hide-tip)"), "t");
        assert_eq!(s.seen(), [Seen::Tooltip(main, None)]);
        assert_eq!(s.eval("(length (frame-list))"), frames_before);
    });
}

#[test]
fn hidden_tip_frames_are_reused() {
    on_big_stack(|| {
        let mut s = Session::new();
        s.eval("(setq use-system-tooltips nil tooltip-reuse-hidden-frame t)");
        assert_eq!(
            s.eval("(x-show-tip \"a\" nil '((foreground-color . \"red\")))"),
            "nil"
        );
        let tip = s.tip();
        let tip_id = s.tip_id();
        let (cw, lh) = s.cell();
        let (width, height) = ((2 * cw + 6) as u32, (lh + 6) as u32);
        assert_eq!(s.seen(), [Seen::Open(tip_id, 305, 190, width, height)]);
        assert_eq!(s.redisplayed(), [Seen::Present(tip_id, width, height)]);
        let main = s.frame;
        assert_eq!(s.eval("(x-hide-tip)"), "t");
        assert_eq!(
            s.seen(),
            [Seen::Tooltip(main, None), Seen::Visible(tip_id, false)]
        );
        assert_eq!(s.tip_frames().len(), 1);
        assert_eq!(s.eval("(x-hide-tip)"), "nil");
        assert_eq!(s.seen(), []);
        assert_eq!(
            s.eval("(x-show-tip \"bc\" nil '((foreground-color . \"red\")))"),
            "nil"
        );
        let width = (3 * cw + 6) as u32;
        assert_eq!(
            s.seen(),
            [Seen::Move(tip_id, 305, 190), Seen::Visible(tip_id, true)]
        );
        assert_eq!(s.redisplayed(), [Seen::Present(tip_id, width, height)]);
        assert_eq!(s.tip(), tip);
        assert_eq!(
            s.eval("(x-show-tip \"bc\" nil '((foreground-color . \"blue\")))"),
            "nil"
        );
        let seen = s.seen();
        let new_tip = s.tip();
        let new_id = s.tip_id();
        assert_ne!(new_tip, tip);
        assert_eq!(
            seen,
            [
                Seen::Tooltip(main, None),
                Seen::Close(tip_id),
                Seen::Open(new_id, 305, 190, width, height)
            ]
        );
        assert_eq!(s.redisplayed(), [Seen::Present(new_id, width, height)]);
        assert_eq!(
            s.eval("(x-show-tip \"bc\" nil '((foreground-color . \"blue\") (left . 4)))"),
            "nil"
        );
        assert_eq!(
            s.seen(),
            [
                Seen::Tooltip(main, None),
                Seen::Visible(new_id, false),
                Seen::Move(new_id, 4, 190),
                Seen::Visible(new_id, true)
            ]
        );
        assert_eq!(s.redisplayed(), [Seen::Present(new_id, width, height)]);
        assert_eq!(s.tip(), new_tip);
        assert_eq!(s.eval("(x-show-tip \"bc\" nil nil)"), "nil");
        let seen = s.seen();
        assert_ne!(s.tip(), new_tip);
        assert_eq!(seen[..2], [Seen::Tooltip(main, None), Seen::Close(new_id)]);
    });
}

#[test]
fn tips_are_placed_inside_the_pointer_monitor() {
    on_big_stack(|| {
        let mut s = Session::new();
        s.eval("(setq use-system-tooltips nil)");
        fn open(s: &mut Session, source: &str) -> (i32, i32, u32, u32) {
            assert_eq!(s.eval(source), "nil", "evaluating {source}");
            let opened: Vec<_> = s
                .seen()
                .into_iter()
                .filter_map(|seen| match seen {
                    Seen::Open(_, x, y, w, h) => Some((x, y, w, h)),
                    _ => None,
                })
                .collect();
            assert_eq!(opened.len(), 1, "evaluating {source}");
            opened[0]
        }
        let (x, y, width, height) =
            open(&mut s, "(x-show-tip \"x\" nil '((left . 10) (top . 20)))");
        assert_eq!((x, y), (10, 20));
        let (w, h) = (width as i32, height as i32);
        assert_eq!(
            open(
                &mut s,
                "(x-show-tip \"x\" nil '((right . 400) (bottom . 300)))"
            ),
            (400 - w, 300 - h, width, height)
        );
        *s.pointer.lock().unwrap() = (1530, 860);
        assert_eq!(
            open(&mut s, "(x-show-tip \"x\" nil '((name . \"a\")))"),
            (1525 - w, 870 - h, width, height)
        );
        *s.pointer.lock().unwrap() = (2, 4);
        assert_eq!(
            open(&mut s, "(x-show-tip \"x\" nil '((name . \"b\")) nil -5 -9)"),
            (0, 0, width, height)
        );
        *s.pointer.lock().unwrap() = (3100, 1435);
        assert_eq!(
            open(&mut s, "(x-show-tip \"x\" nil '((name . \"c\")))"),
            (3105, 1445 - h, width, height)
        );
        *s.pointer.lock().unwrap() = (9000, 9000);
        assert_eq!(
            open(&mut s, "(x-show-tip \"x\" nil '((name . \"d\")))"),
            (9000 - w - 5, 9010 - h, width, height)
        );
    });
}
