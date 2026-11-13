use std::sync::mpsc;

use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_gui::display::Display;
use iced::futures::channel::mpsc as fmpsc;

const STACK_SIZE: usize = 512 << 20;

fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, f)
            .expect("spawn test thread")
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

fn display() -> Display {
    let (commands, _command_rx) = fmpsc::unbounded();
    let (_notice_tx, notices) = mpsc::channel();
    Display::new(commands, notices)
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

fn booted(startup: Startup) -> Lisp {
    let mut lisp = Lisp::boot(startup);
    emacs_gui::install(&mut lisp, display());
    lisp.init_display();
    lisp
}

#[test]
fn install_defines_the_pgtk_variables() {
    on_big_stack(|| {
        let mut lisp = booted(Startup::program(false));
        assert_eq!(
            eval(
                &mut lisp,
                "(list (featurep 'pgtk) (featurep 'gtk) (featurep 'cairo) (stringp gtk-version-string) (stringp cairo-version-string))"
            ),
            "(t t t t t)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(list x-ctrl-keysym x-alt-keysym x-hyper-keysym x-meta-keysym x-super-keysym x-toolkit-scroll-bars pgtk-wait-for-event-timeout (hash-table-p pgtk-keysym-table) x-use-underline-position-properties x-underline-at-descent-line)"
            ),
            "(nil nil nil nil nil gtk 0.1 t t nil)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(list (get 'alt 'modifier-value) (get 'super 'modifier-value) (get 'hyper 'modifier-value) (get 'control 'modifier-value) (get 'meta 'modifier-value))"
            ),
            "(4194304 8388608 16777216 67108864 134217728)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(list x-cursor-fore-pixel x-gtk-use-old-file-dialog x-gtk-show-hidden-files x-gtk-file-dialog-help-text x-max-tooltip-size pgtk-use-im-context-on-new-connection pgtk-input-purpose)"
            ),
            "(nil nil nil t nil t terminal)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(list selection-converter-alist pgtk-lost-selection-functions pgtk-sent-selection-functions pgtk-sent-selection-hooks pgtk-selection-timeout pgtk-selection-alias-alist)"
            ),
            "(nil nil nil nil 0 nil)"
        );
    });
}

#[test]
fn x_open_connection_makes_a_pgtk_terminal() {
    on_big_stack(|| {
        let mut lisp = booted(Startup::program(false));
        assert_eq!(eval(&mut lisp, "(x-display-list)"), "nil");
        assert_eq!(
            eval(&mut lisp, "(x-open-connection 5)"),
            "error: (wrong-type-argument stringp 5)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq x-resource-name \"emacs\") (list (x-open-connection nil nil t) window-system))"
            ),
            "(nil pgtk)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(let ((names (x-display-list))) (list (length names) (stringp (car names)) (and (member (car names) (mapcar (function terminal-name) (terminal-list))) t)))"
            ),
            "(1 t t)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (garbage-collect) (stringp (car (x-display-list))))"
            ),
            "t"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (x-open-connection \"\") (list (length (x-display-list)) (length (terminal-list))))"
            ),
            "(2 3)"
        );
        assert_eq!(
            eval(&mut lisp, "(x-open-connection \"no-such-host:7\")"),
            "error: (error \"Display on no-such-host:7 not responding.\n\")"
        );
        assert_eq!(eval(&mut lisp, "(length (x-display-list))"), "2");
    });
}

#[test]
fn x_open_connection_without_a_system_name() {
    on_big_stack(|| {
        let mut startup = Startup::program(false);
        startup.build_details = false;
        let mut lisp = booted(startup);
        assert_eq!(eval(&mut lisp, "system-name"), "nil");
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq x-resource-name \"emacs\") (x-open-connection nil nil t) (x-open-connection \"\") (length (x-display-list)))"
            ),
            "2"
        );
        let invocation = eval(&mut lisp, "invocation-name");
        let expected = format!("{}@", invocation.trim_matches('"'));
        let names = emacs_gui::pgtkterm::with_display_list(|displays| {
            displays
                .iter()
                .map(|d| String::from_utf8_lossy(&d.x_id_name).into_owned())
                .collect::<Vec<_>>()
        });
        assert_eq!(names, [expected.clone(), expected]);
    });
}

#[test]
fn display_queries() {
    on_big_stack(|| {
        let mut lisp = booted(Startup::program(false));
        assert_eq!(
            eval(&mut lisp, "(x-display-planes)"),
            "error: (error \"Frames are not in use or not initialized\")"
        );
        assert_eq!(eval(&mut lisp, "(x-display-visual-class)"), "true-color");
        eval(
            &mut lisp,
            "(progn (setq x-resource-name \"emacs\") (x-open-connection nil))",
        );
        for (source, expected) in [
            ("(x-display-planes)", "32"),
            ("(x-display-color-cells)", "16777216"),
            ("(x-display-screens)", "1"),
            ("(x-server-max-request-size)", "nil"),
            ("(x-display-backing-store)", "nil"),
            ("(x-display-save-under)", "nil"),
            ("(x-display-visual-class 7)", "true-color"),
            (
                "(x-display-color-cells 5)",
                "error: (wrong-type-argument frame-live-p 5)",
            ),
            ("(x-display-planes (car (x-display-list)))", "32"),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
}

#[test]
fn x_get_resource_without_a_defaults_database() {
    on_big_stack(|| {
        let mut lisp = booted(Startup::program(false));
        assert_eq!(
            eval(&mut lisp, "(x-get-resource \"font\" \"Font\")"),
            "error: (error \"Window system is not in use or not initialized\")"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq x-resource-name \"emacs\") (x-open-connection nil) (x-get-resource \"font\" \"Font\"))"
            ),
            "nil"
        );
        assert_eq!(
            eval(&mut lisp, "(x-get-resource 1 \"Font\")"),
            "error: (wrong-type-argument stringp 1)"
        );
        assert_eq!(
            eval(&mut lisp, "(x-get-resource \"font\" \"Font\" \"menu\")"),
            "error: (error \"x-get-resource: must specify both COMPONENT and SUBCLASS or neither\")"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(list (progn (setq x-resource-name \"a.b*cd\" x-resource-class nil) (x-get-resource \"font\" \"Font\" \"menu\" \"Menu\")) x-resource-name x-resource-class)"
            ),
            "(nil \"a_b_cd\" \"Emacs\")"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq x-resource-name 5) (x-get-resource \"font\" \"Font\") x-resource-name)"
            ),
            "\"emacs\""
        );
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-resource \"font\" \"Monospace\")"),
            "error: (error \"gsettings not supported\")"
        );
        assert_eq!(
            eval(&mut lisp, "(pgtk-set-resource \"font\" 3)"),
            "error: (wrong-type-argument stringp 3)"
        );
    });
}

#[test]
fn color_primitives_need_a_window_frame() {
    on_big_stack(|| {
        let mut lisp = booted(Startup::program(false));
        assert_eq!(
            eval(&mut lisp, "(xw-display-color-p)"),
            "error: (error \"Frames are not in use or not initialized\")"
        );
        assert_eq!(eval(&mut lisp, "(x-display-grayscale-p)"), "t");
        assert_eq!(
            eval(
                &mut lisp,
                "(progn (setq x-resource-name \"emacs\") (x-open-connection nil) (xw-display-color-p))"
            ),
            "t"
        );
        assert_eq!(
            eval(&mut lisp, "(xw-color-values \"red\")"),
            "error: (error \"Window system frame should be used\")"
        );
        assert_eq!(
            eval(&mut lisp, "(xw-color-defined-p \"red\")"),
            "error: (error \"Window system frame should be used\")"
        );
    });
}

#[test]
fn init_display_chooses_pgtk_only_for_interactive_window_sessions() {
    on_big_stack(|| {
        let mut lisp = booted(Startup::program(false));
        assert_eq!(
            eval(
                &mut lisp,
                "(list initial-window-system inverse-video cursor-in-echo-area)"
            ),
            "(pgtk nil nil)"
        );
        let mut lisp = booted(Startup::program(true));
        assert_eq!(eval(&mut lisp, "initial-window-system"), "nil");
        let mut startup = Startup::program(false);
        startup.inhibit_window_system = true;
        let mut lisp = booted(startup);
        assert_eq!(eval(&mut lisp, "initial-window-system"), "nil");
        let mut lisp = Lisp::boot(Startup::program(false));
        lisp.init_display();
        assert_eq!(eval(&mut lisp, "initial-window-system"), "nil");
    });
}
