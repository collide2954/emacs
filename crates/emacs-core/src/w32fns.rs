use crate::lisp::Lisp;

pub fn init_vars(lisp: &mut Lisp) {
    if cfg!(windows) {
        let symbol = lisp.intern("w32-follow-system-dark-mode");
        lisp.defvar_bool(symbol, true);
    }
    #[cfg(windows)]
    {
        let symbol = lisp.intern("w32-ansi-code-page");
        lisp.defvar_int(symbol, i64::from(emacs_w32::ansi_code_page()));
        let symbol = lisp.intern("w32-multibyte-code-page");
        lisp.defvar_int(symbol, i64::from(emacs_w32::multibyte_code_page()));
    }
}
