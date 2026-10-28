use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_bool(sym::DELETE_EXITED_PROCESSES, true);
    lisp.defvar(sym::PROCESS_CONNECTION_TYPE, sym::T);
    lisp.defvar(sym::PROCESS_ADAPTIVE_READ_BUFFERING, Value::NIL);
    lisp.defvar_bool(sym::PROCESS_PRIORITIZE_LOWER_FDS, false);
    let interrupt = lisp.list(&[sym::INTERNAL_DEFAULT_INTERRUPT_PROCESS]);
    lisp.defvar(sym::INTERRUPT_PROCESS_FUNCTIONS, interrupt);
    let signal = lisp.list(&[sym::INTERNAL_DEFAULT_SIGNAL_PROCESS]);
    lisp.defvar(sym::SIGNAL_PROCESS_FUNCTIONS, signal);
    lisp.defvar(sym::INTERNAL__DAEMON_SOCKNAME, Value::NIL);
    lisp.defvar_int(sym::READ_PROCESS_OUTPUT_MAX, 65536);
    lisp.defvar_bool(sym::FAST_READ_PROCESS_OUTPUT, true);
    lisp.defvar_int(sym::PROCESS_ERROR_PAUSE_TIME, 1);
}
