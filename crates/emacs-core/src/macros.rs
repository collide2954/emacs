use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::KBD_MACRO_TERMINATION_HOOK, Value::NIL);
    lisp.defvar_kboard(sym::DEFINING_KBD_MACRO, Kvar::DefiningKbdMacro);
    lisp.defvar(sym::EXECUTING_KBD_MACRO, Value::NIL);
    lisp.defvar_int(sym::EXECUTING_KBD_MACRO_INDEX, 0);
    lisp.defvar_kboard(sym::LAST_KBD_MACRO, Kvar::LastKbdMacro);
}
