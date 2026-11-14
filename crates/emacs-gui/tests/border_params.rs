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

fn frame(lisp: &mut Lisp, params: &str) -> Value {
    assert_eq!(
        eval(
            lisp,
            &format!("(framep (setq f (x-create-frame '({params}))))")
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    lisp.find_symbol_value(symbol)
}

#[test]
fn internal_border_width_and_border_color_reach_the_frame() {
    on_big_stack(|| {
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
        eval(
            &mut lisp,
            "(progn (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil))",
        );
        let plain = frame(&mut lisp, "(name . \"p\") (width . 20) (height . 5)");
        let plain_width = lisp.frame(plain).geometry.pixel_width;
        let f = frame(
            &mut lisp,
            "(name . \"b\") (width . 20) (height . 5) (internal-border-width . 4)",
        );
        assert_eq!(lisp.frame(f).geometry.internal_border_width, 4);
        assert_eq!(
            eval(&mut lisp, "(frame-parameter f 'internal-border-width)"),
            "4"
        );
        assert_eq!(lisp.frame(f).geometry.pixel_width, plain_width + 8);
        let text_width = lisp.frame(f).geometry.text_width;
        eval(
            &mut lisp,
            "(modify-frame-parameters f '((internal-border-width . 2)))",
        );
        assert_eq!(lisp.frame(f).geometry.internal_border_width, 2);
        assert_eq!(lisp.frame(f).geometry.text_width, text_width);
        assert_eq!(
            eval(&mut lisp, "(frame-parameter f 'internal-border-width)"),
            "2"
        );
        eval(
            &mut lisp,
            "(modify-frame-parameters f '((internal-border-width . -3)))",
        );
        assert_eq!(lisp.frame(f).geometry.internal_border_width, 0);
        assert_eq!(
            eval(
                &mut lisp,
                "(modify-frame-parameters f '((internal-border-width . x)))"
            ),
            "error: (wrong-type-argument integerp x)"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(modify-frame-parameters f '((internal-border-width . 100000000000)))"
            ),
            "error: (args-out-of-range 100000000000 0 2147483647)"
        );
        eval(
            &mut lisp,
            "(modify-frame-parameters f '((border-color . \"red\")))",
        );
        assert_eq!(
            emacs_gui::pgtkterm::with_output(&lisp, f, |o| o.border_pixel),
            0xFF0000
        );
        assert_eq!(
            eval(&mut lisp, "(frame-parameter f 'border-color)"),
            "\"red\""
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(modify-frame-parameters f '((border-color . 5)))"
            ),
            "error: (wrong-type-argument stringp 5)"
        );
    });
}
