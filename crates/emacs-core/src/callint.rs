use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_kboard(sym::PREFIX_ARG, Kvar::PrefixArg);
    lisp.defvar_kboard(sym::LAST_PREFIX_ARG, Kvar::LastPrefixArg);
    lisp.defvar(sym::CURRENT_PREFIX_ARG, Value::NIL);
    lisp.defvar(sym::COMMAND_HISTORY, Value::NIL);
    lisp.defvar(sym::COMMAND_DEBUG_STATUS, Value::NIL);
    lisp.defvar(sym::MARK_EVEN_IF_INACTIVE, sym::T);
    lisp.defvar(sym::MOUSE_LEAVE_BUFFER_HOOK, Value::NIL);
    lisp.defvar_bool(sym::INHIBIT_MOUSE_EVENT_CHECK, false);
}
