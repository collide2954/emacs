use emacs_core::lisp::Lisp;
use emacs_core::value::Value;

pub fn syms_of_pgtkselect(lisp: &mut Lisp) {
    for name in [
        "selection-converter-alist",
        "pgtk-lost-selection-functions",
        "pgtk-sent-selection-functions",
        "pgtk-sent-selection-hooks",
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar(symbol, Value::NIL);
    }
    let symbol = lisp.intern("pgtk-selection-timeout");
    lisp.defvar_int(symbol, 0);
    let symbol = lisp.intern("pgtk-selection-alias-alist");
    lisp.defvar(symbol, Value::NIL);
}
