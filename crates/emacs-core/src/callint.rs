use crate::error::LispResult;
use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::special("interactive", 0, f_interactive),
    Subr::many("funcall-interactively", 1, f_funcall_interactively),
    Subr::fixed("prefix-numeric-value", 1, 1, f_prefix_numeric_value),
];

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

fn f_interactive(_lisp: &mut Lisp, _args: Value) -> LispResult {
    Ok(Value::NIL)
}

fn f_funcall_interactively(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let count = lisp.specpdl.len();
    if !lisp.var_true(sym::MULTIPLE_TERMINALS_MERGE_KEYBOARDS) {
        lisp.temporarily_switch_to_single_kboard(None)?;
    }
    let r = lisp.funcall(args[0], &args[1..]);
    lisp.unbind_to_value(count, r)
}

fn f_prefix_numeric_value(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let raw = args[0];
    Ok(if raw.is_nil() {
        Value::fixnum(1)
    } else if raw == sym::MINUS {
        Value::fixnum(-1)
    } else if raw.is_cons() && lisp.xcar(raw).is_fixnum() {
        lisp.xcar(raw)
    } else if raw.is_fixnum() {
        raw
    } else {
        Value::fixnum(1)
    })
}
