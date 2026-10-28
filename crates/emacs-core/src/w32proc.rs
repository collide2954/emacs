use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    if !cfg!(windows) {
        return;
    }
    for name in ["high", "low", "cygwin", "msys", "w32-native"] {
        lisp.intern(name);
    }
    let local = lisp.intern("local");
    for (name, value) in [
        ("w32-quote-process-args", sym::T),
        ("w32-start-process-show-window", Value::NIL),
        ("w32-start-process-share-console", Value::NIL),
        ("w32-start-process-inherit-error-mode", sym::T),
        ("w32-downcase-file-names", Value::NIL),
        ("w32-get-true-file-attributes", local),
        ("w32-collate-ignore-punctuation", Value::NIL),
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar(symbol, value);
    }
    for name in ["w32-pipe-read-delay", "w32-pipe-buffer-size"] {
        let symbol = lisp.intern(name);
        lisp.defvar_int(symbol, 0);
    }
}
