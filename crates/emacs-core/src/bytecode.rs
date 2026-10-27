use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::value::Value;

pub fn exec_byte_code(
    lisp: &mut Lisp,
    fun: Value,
    _args_template: Value,
    _args: &[Value],
) -> LispResult {
    Err(lisp.signal_error("Byte-code execution is not available yet", fun))
}
