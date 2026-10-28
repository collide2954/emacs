use crate::lisp::Lisp;

pub fn init_vars(lisp: &mut Lisp) {
    if cfg!(windows) {
        let symbol = lisp.intern("w32-follow-system-dark-mode");
        lisp.defvar_bool(symbol, true);
    }
}
