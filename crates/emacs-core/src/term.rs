use crate::lisp::Lisp;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    for (name, value) in [
        ("system-uses-terminfo", false),
        ("visible-cursor", true),
        ("tty-menu-calls-mouse-position-function", false),
        ("tty-cursor-movement-use-TAB-BS", false),
        ("tty-cursor-movement-use-TAB", true),
        ("tty-display-emoji-force-wide", true),
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar_bool(symbol, value);
    }
    for name in [
        "suspend-tty-functions",
        "resume-tty-functions",
        "auto-composition-emoji-tty-eligible-codepoints",
        "auto-composition-emoji-unqualified-tail-initials",
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar(symbol, Value::NIL);
    }
}
