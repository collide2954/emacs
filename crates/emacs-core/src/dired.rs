use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::COMPLETION_IGNORED_EXTENSIONS, Value::NIL);
}
