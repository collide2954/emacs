use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;

pub fn syms_of_pgtkim(lisp: &mut Lisp) {
    let symbol = lisp.intern("pgtk-use-im-context-on-new-connection");
    lisp.defvar(symbol, sym::T);
    for name in [
        "free-form",
        "digits",
        "number",
        "phone",
        "email",
        "pin",
        "password",
    ] {
        lisp.intern(name);
    }
    let symbol = lisp.intern("pgtk-input-purpose");
    lisp.defvar(symbol, sym::TERMINAL);
}
