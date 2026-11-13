use std::cell::RefCell;

use emacs_core::lisp::Lisp;
use emacs_core::terminal::WindowSystem;

pub mod display;
pub mod font;
pub mod ftcrfont;
pub mod gdkrgba;
pub mod hbfont;
pub mod keysym;
pub mod mouse;
pub mod pgtkdraw;
pub mod pgtkfns;
pub mod pgtkim;
pub mod pgtkselect;
pub mod pgtkterm;
pub mod protocol;
pub mod shell;
pub mod surface;

pub use shell::run;

use display::Display;

thread_local! {
    static DISPLAY: RefCell<Option<Display>> = const { RefCell::new(None) };
}

pub fn with_display<T>(f: impl FnOnce(&mut Display) -> T) -> Option<T> {
    DISPLAY.with(|d| d.borrow_mut().as_mut().map(f))
}

pub fn install(lisp: &mut Lisp, display: Display) {
    DISPLAY.with(|d| *d.borrow_mut() = Some(display));
    pgtkterm::syms_of_pgtkterm(lisp);
    pgtkfns::syms_of_pgtkfns(lisp);
    pgtkselect::syms_of_pgtkselect(lisp);
    pgtkim::syms_of_pgtkim(lisp);
    ftcrfont::syms_of_ftcrfont(lisp);
    lisp.window_system = Some(WindowSystem {
        mark,
        check_x_display_info: pgtkfns::check_pgtk_display_info,
        get_keysym_name: keysym::keysym_name,
        wait_for_input: pgtkterm::pgtk_wait_for_input,
    });
}

fn mark(roots: &mut Vec<emacs_core::value::Value>) {
    pgtkterm::mark_pgtkterm(roots);
    ftcrfont::mark_ftfont(roots);
}
