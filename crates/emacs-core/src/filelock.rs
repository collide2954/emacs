use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::TEMPORARY_FILE_DIRECTORY, Value::NIL);
    lisp.defvar_bool(sym::CREATE_LOCKFILES, true);
}
