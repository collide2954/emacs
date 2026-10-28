use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    if cfg!(windows) {
        lisp.set_plain_value(sym::LIBPNG_VERSION, Value::fixnum(-1));
        lisp.set_plain_value(sym::LIBGIF_VERSION, Value::fixnum(-1));
        lisp.set_plain_value(sym::LIBJPEG_VERSION, Value::fixnum(-1));
    }
}
