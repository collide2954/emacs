use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    if cfg!(windows) {
        lisp.set_plain_value(sym::TREE_SITTER__LIBRARY_ABI, Value::fixnum(-1));
    }
}
