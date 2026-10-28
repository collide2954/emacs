use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub const UNCAUGHT: usize = usize::MAX;

#[derive(Clone, Copy, Debug)]
pub enum Flow {
    Signal {
        error: Value,
        target: usize,
    },
    Throw {
        tag: Value,
        value: Value,
        target: usize,
    },
}

impl Flow {
    pub fn target(&self) -> usize {
        match *self {
            Flow::Signal { target, .. } | Flow::Throw { target, .. } => target,
        }
    }

    pub fn values(&self) -> [Value; 2] {
        match *self {
            Flow::Signal { error, .. } => [error, Value::NIL],
            Flow::Throw { tag, value, .. } => [tag, value],
        }
    }
}

pub type LispResult<T = Value> = Result<T, Flow>;

#[derive(Clone, Copy, Debug)]
pub enum HandlerKind {
    Catch(Value),
    ConditionCase(Value),
    CatchAll,
    CatchAllDebuggable,
    HandlerBind {
        conditions: Value,
        handler: Value,
        skip: usize,
    },
    SkipConditions(usize),
}

#[derive(Clone, Copy, Debug)]
pub struct Handler {
    pub kind: HandlerKind,
    pub specpdl: usize,
    pub stack: usize,
    pub flow_roots: usize,
    pub eval_depth: i64,
}

pub const BUILTIN_ERRORS: &[(Value, &[Value], &str)] = &[
    (sym::QUIT, &[], "Quit"),
    (sym::MINIBUFFER_QUIT, &[sym::QUIT], "Quit"),
    (sym::USER_ERROR, &[sym::ERROR], ""),
    (
        sym::WRONG_LENGTH_ARGUMENT,
        &[sym::ERROR],
        "Wrong length argument",
    ),
    (
        sym::WRONG_TYPE_ARGUMENT,
        &[sym::ERROR],
        "Wrong type argument",
    ),
    (sym::TYPE_MISMATCH, &[sym::ERROR], "Types do not match"),
    (sym::ARGS_OUT_OF_RANGE, &[sym::ERROR], "Args out of range"),
    (
        sym::VOID_FUNCTION,
        &[sym::ERROR],
        "Symbol's function definition is void",
    ),
    (
        sym::CYCLIC_FUNCTION_INDIRECTION,
        &[sym::ERROR],
        "Symbol's chain of function indirections contains a loop",
    ),
    (
        sym::CYCLIC_VARIABLE_INDIRECTION,
        &[sym::ERROR],
        "Symbol's chain of variable indirections contains a loop",
    ),
    (sym::CIRCULAR_LIST, &[sym::ERROR], "List contains a loop"),
    (
        sym::VOID_VARIABLE,
        &[sym::ERROR],
        "Symbol's value as variable is void",
    ),
    (
        sym::SETTING_CONSTANT,
        &[sym::ERROR],
        "Attempt to set a constant symbol",
    ),
    (
        sym::TRAPPING_CONSTANT,
        &[sym::ERROR],
        "Attempt to trap writes to a constant symbol",
    ),
    (
        sym::INVALID_READ_SYNTAX,
        &[sym::ERROR],
        "Invalid read syntax",
    ),
    (sym::INVALID_FUNCTION, &[sym::ERROR], "Invalid function"),
    (
        sym::WRONG_NUMBER_OF_ARGUMENTS,
        &[sym::ERROR],
        "Wrong number of arguments",
    ),
    (
        sym::MALFORMED_KEYWORD_ARG_LIST,
        &[sym::ERROR],
        "Keyword lacks a corresponding value",
    ),
    (sym::NO_CATCH, &[sym::ERROR], "No catch for tag"),
    (
        sym::END_OF_FILE,
        &[sym::ERROR],
        "End of file during parsing",
    ),
    (sym::ARITH_ERROR, &[sym::ERROR], "Arithmetic error"),
    (
        sym::BEGINNING_OF_BUFFER,
        &[sym::ERROR],
        "Beginning of buffer",
    ),
    (sym::END_OF_BUFFER, &[sym::ERROR], "End of buffer"),
    (sym::BUFFER_READ_ONLY, &[sym::ERROR], "Buffer is read-only"),
    (
        sym::TEXT_READ_ONLY,
        &[sym::BUFFER_READ_ONLY, sym::ERROR],
        "Text is read-only",
    ),
    (
        sym::INHIBITED_INTERACTION,
        &[sym::ERROR],
        "User interaction while inhibited",
    ),
    (
        sym::DOMAIN_ERROR,
        &[sym::ARITH_ERROR, sym::ERROR],
        "Arithmetic domain error",
    ),
    (
        sym::RANGE_ERROR,
        &[sym::ARITH_ERROR, sym::ERROR],
        "Arithmetic range error",
    ),
    (
        sym::SINGULARITY_ERROR,
        &[sym::DOMAIN_ERROR, sym::ARITH_ERROR, sym::ERROR],
        "Arithmetic singularity error",
    ),
    (
        sym::OVERFLOW_ERROR,
        &[sym::RANGE_ERROR, sym::ARITH_ERROR, sym::ERROR],
        "Arithmetic overflow error",
    ),
    (
        sym::UNDERFLOW_ERROR,
        &[sym::RANGE_ERROR, sym::ARITH_ERROR, sym::ERROR],
        "Arithmetic underflow error",
    ),
    (
        sym::RECURSION_ERROR,
        &[sym::ERROR],
        "Excessive recursive calling error",
    ),
    (
        sym::EXCESSIVE_LISP_NESTING,
        &[sym::RECURSION_ERROR, sym::ERROR],
        "Lisp nesting exceeds `max-lisp-eval-depth'",
    ),
    (
        sym::EXCESSIVE_VARIABLE_BINDING,
        &[sym::RECURSION_ERROR, sym::ERROR],
        "Variable binding depth exceeds max-specpdl-size",
    ),
    (
        sym::PROTECTED_FIELD,
        &[sym::ERROR],
        "Attempt to modify a protected field",
    ),
    (
        sym::CODING_SYSTEM_ERROR,
        &[sym::ERROR],
        "Invalid coding system",
    ),
    (sym::FILE_ERROR, &[sym::ERROR], "File error"),
    (
        sym::FILE_ALREADY_EXISTS,
        &[sym::FILE_ERROR, sym::ERROR],
        "File already exists",
    ),
    (
        sym::FILE_DATE_ERROR,
        &[sym::FILE_ERROR, sym::ERROR],
        "Cannot set file date",
    ),
    (
        sym::FILE_MISSING,
        &[sym::FILE_ERROR, sym::ERROR],
        "File is missing",
    ),
    (
        sym::PERMISSION_DENIED,
        &[sym::FILE_ERROR, sym::ERROR],
        "Cannot access file or directory",
    ),
    (
        sym::FILE_NOTIFY_ERROR,
        &[sym::FILE_ERROR, sym::ERROR],
        "File notification error",
    ),
    (
        sym::REMOTE_FILE_ERROR,
        &[sym::FILE_ERROR, sym::ERROR],
        "Remote file error",
    ),
    (sym::SEARCH_FAILED, &[sym::ERROR], "Search failed"),
    (
        sym::USER_SEARCH_FAILED,
        &[sym::USER_ERROR, sym::SEARCH_FAILED, sym::ERROR],
        "Search failed",
    ),
    (sym::INVALID_REGEXP, &[sym::ERROR], "Invalid regexp"),
    (sym::SCAN_ERROR, &[sym::ERROR], "Scan error"),
    (
        sym::THREAD_BUFFER_KILLED,
        &[sym::ERROR],
        "Thread's current buffer killed",
    ),
    (sym::JSON_ERROR, &[sym::ERROR], "generic JSON error"),
    (
        sym::JSON_OUT_OF_MEMORY,
        &[sym::JSON_ERROR, sym::ERROR],
        "not enough memory for creating JSON object",
    ),
    (
        sym::JSON_PARSE_ERROR,
        &[sym::JSON_ERROR, sym::ERROR],
        "could not parse JSON stream",
    ),
    (
        sym::JSON_END_OF_FILE,
        &[sym::JSON_PARSE_ERROR, sym::JSON_ERROR, sym::ERROR],
        "end of JSON stream",
    ),
    (
        sym::JSON_TRAILING_CONTENT,
        &[sym::JSON_PARSE_ERROR, sym::JSON_ERROR, sym::ERROR],
        "trailing content after JSON stream",
    ),
    (
        sym::JSON_OBJECT_TOO_DEEP,
        &[sym::JSON_ERROR, sym::ERROR],
        "object cyclic or Lisp evaluation too deep",
    ),
    (
        sym::JSON_UTF8_DECODE_ERROR,
        &[sym::JSON_ERROR, sym::ERROR],
        "invalid utf-8 encoding",
    ),
    (
        sym::JSON_INVALID_SURROGATE_ERROR,
        &[sym::JSON_ERROR, sym::ERROR],
        "invalid surrogate pair",
    ),
    (
        sym::JSON_NUMBER_OUT_OF_RANGE,
        &[sym::JSON_ERROR, sym::ERROR],
        "number out of range",
    ),
    (
        sym::JSON_ESCAPE_SEQUENCE_ERROR,
        &[sym::JSON_PARSE_ERROR, sym::JSON_ERROR, sym::ERROR],
        "invalid escape sequence",
    ),
];

impl Lisp {
    pub fn signal_list(&mut self, symbol: Value, items: &[Value]) -> Flow {
        let data = self.list(items);
        self.signal(symbol, data)
    }

    pub fn wrong_type(&mut self, predicate: Value, value: Value) -> Flow {
        self.signal_list(sym::WRONG_TYPE_ARGUMENT, &[predicate, value])
    }

    pub fn args_out_of_range(&mut self, a: Value, b: Value) -> Flow {
        self.signal_list(sym::ARGS_OUT_OF_RANGE, &[a, b])
    }

    pub fn args_out_of_range3(&mut self, a: Value, b: Value, c: Value) -> Flow {
        self.signal_list(sym::ARGS_OUT_OF_RANGE, &[a, b, c])
    }

    pub fn error(&mut self, message: &str) -> Flow {
        let string = self.string(message);
        self.signal_list(sym::ERROR, &[string])
    }

    pub fn signal_error(&mut self, message: &str, arg: Value) -> Flow {
        let arg = if self.proper_list_p(arg) {
            arg
        } else {
            self.list(&[arg])
        };
        let string = self.string(message);
        let data = self.cons(string, arg);
        self.signal(sym::ERROR, data)
    }

    pub fn flow_condition(&mut self, flow: &Flow) -> Value {
        match *flow {
            Flow::Signal { error, .. } => error,
            Flow::Throw { tag, value, .. } => self.list(&[sym::NO_CATCH, tag, value]),
        }
    }
}
