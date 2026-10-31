use crate::error::{Flow, HandlerKind, LispResult, UNCAUGHT};
use crate::lisp::{BacktraceArgs, Lisp, SpecBinding, UnwindFn};
use crate::object::{Forward, SymbolValue, TrappedWrite, Vectorlike};
use crate::subr::{Subr, SubrKind};
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::special("or", 0, f_or),
    Subr::special("and", 0, f_and),
    Subr::special("if", 2, f_if),
    Subr::special("cond", 0, f_cond),
    Subr::special("progn", 0, f_progn),
    Subr::special("prog1", 1, f_prog1),
    Subr::special("setq", 0, f_setq),
    Subr::special("quote", 1, f_quote),
    Subr::fixed("make-interpreted-closure", 3, 5, f_make_interpreted_closure),
    Subr::special("function", 1, f_function),
    Subr::fixed("defvaralias", 2, 3, f_defvaralias),
    Subr::fixed(
        "internal-delete-indirect-variable",
        1,
        1,
        f_internal_delete_indirect_variable,
    ),
    Subr::fixed("default-toplevel-value", 1, 1, f_default_toplevel_value),
    Subr::fixed(
        "set-default-toplevel-value",
        2,
        2,
        f_set_default_toplevel_value,
    ),
    Subr::fixed(
        "buffer-local-toplevel-value",
        1,
        2,
        f_buffer_local_toplevel_value,
    ),
    Subr::fixed(
        "set-buffer-local-toplevel-value",
        2,
        3,
        f_set_buffer_local_toplevel_value,
    ),
    Subr::fixed(
        "internal--define-uninitialized-variable",
        1,
        2,
        f_internal_define_uninitialized_variable,
    ),
    Subr::special("defvar", 1, f_defvar),
    Subr::fixed("defvar-1", 2, 3, f_defvar_1),
    Subr::special("defconst", 2, f_defconst),
    Subr::fixed("defconst-1", 2, 3, f_defconst_1),
    Subr::fixed(
        "internal-make-var-non-special",
        1,
        1,
        f_internal_make_var_non_special,
    ),
    Subr::special("let*", 1, f_let_star),
    Subr::special("let", 1, f_let),
    Subr::special("while", 1, f_while),
    Subr::fixed("macroexpand", 1, 2, f_macroexpand),
    Subr::special("catch", 1, f_catch),
    Subr::fixed("throw", 2, 2, f_throw),
    Subr::special("unwind-protect", 1, f_unwind_protect),
    Subr::special("condition-case", 2, f_condition_case),
    Subr::many("handler-bind-1", 1, f_handler_bind_1),
    Subr::fixed("signal", 1, 2, f_signal),
    Subr::fixed("commandp", 1, 2, f_commandp),
    Subr::fixed("autoload", 2, 5, f_autoload),
    Subr::fixed("autoload-do-load", 1, 3, f_autoload_do_load),
    Subr::fixed("eval", 1, 2, f_eval),
    Subr::many("apply", 1, f_apply),
    Subr::many("run-hooks", 0, f_run_hooks),
    Subr::many("run-hook-with-args", 1, f_run_hook_with_args),
    Subr::many(
        "run-hook-with-args-until-success",
        1,
        f_run_hook_with_args_until_success,
    ),
    Subr::many(
        "run-hook-with-args-until-failure",
        1,
        f_run_hook_with_args_until_failure,
    ),
    Subr::many("run-hook-wrapped", 2, f_run_hook_wrapped),
    Subr::fixed("functionp", 1, 1, f_functionp),
    Subr::fixed("debugger-trap", 0, 0, f_ignore).interactive(""),
    Subr::many("funcall", 1, f_funcall),
    Subr::fixed("func-arity", 1, 1, f_func_arity),
    Subr::fixed("special-variable-p", 1, 1, f_special_variable_p),
    Subr::fixed("backtrace-debug", 2, 3, f_backtrace_debug),
    Subr::fixed("mapbacktrace", 1, 2, f_mapbacktrace),
    Subr::fixed(
        "backtrace-frame--internal",
        3,
        3,
        f_backtrace_frame_internal,
    ),
    Subr::fixed("backtrace--frames-from-thread", 1, 1, f_ignore),
    Subr::fixed("backtrace-eval", 2, 3, f_backtrace_eval),
    Subr::fixed("backtrace--locals", 1, 2, f_backtrace_locals),
    Subr::fixed(
        "funcall-with-delayed-message",
        3,
        3,
        f_funcall_with_delayed_message,
    ),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_int(sym::MAX_LISP_EVAL_DEPTH, 1600);
    lisp.defvar(sym::QUIT_FLAG, Value::NIL);
    lisp.defvar(sym::INHIBIT_QUIT, Value::NIL);
    lisp.defvar_bool(sym::DEBUG_ON_QUIT, false);
    lisp.defvar(sym::DEBUG_ON_ERROR, Value::NIL);
    lisp.defvar(sym::DEBUG_IGNORED_ERRORS, Value::NIL);
    lisp.defvar_bool(sym::DEBUG_ON_NEXT_CALL, false);
    lisp.defvar_bool(sym::BACKTRACE_ON_REDISPLAY_ERROR, false);
    lisp.defvar_bool(sym::DEBUGGER_MAY_CONTINUE, true);
    lisp.defvar_bool(sym::DEBUGGER_STACK_FRAME_AS_LIST, false);
    lisp.defvar(sym::DEBUGGER, sym::DEBUG_EARLY);
    lisp.defvar(sym::SIGNAL_HOOK_FUNCTION, Value::NIL);
    lisp.defvar(sym::DEBUG_ON_SIGNAL, Value::NIL);
    lisp.defvar_bool(sym::BACKTRACE_ON_ERROR_NONINTERACTIVE, true);
    lisp.defvar(sym::INTERNAL_WHEN_ENTERED_DEBUGGER, Value::fixnum(-1));
    lisp.defvar(sym::INTERNAL_INTERPRETER_ENVIRONMENT, Value::NIL);
    lisp.defvar(sym::INTERNAL_MAKE_INTERPRETED_CLOSURE_FUNCTION, Value::NIL);
    lisp.defvar(sym::INHIBIT_DEBUGGER, Value::NIL);
    lisp.defvar(sym::RUN_HOOKS_ENABLED, sym::T);
    lisp.defvar(sym::AUTOLOAD_QUEUE, Value::NIL);
    lisp.defvar(sym::MACROEXP__DYNVARS, Value::NIL);
    lisp.defvar(sym::LEXICAL_BINDING, Value::NIL);
    lisp.defvar(sym::SIGNALING_FUNCTION, Value::NIL);
    lisp.defvar_bool(sym::DEBUG_ON_NEXT_CALL, false);
    lisp.defvar_int(sym::LISP_EVAL_DEPTH_RESERVE, 200);
}

impl Lisp {
    pub fn specpdl_index(&self) -> usize {
        self.specpdl.len()
    }

    pub fn lexenv(&self) -> Value {
        self.plain_value(sym::INTERNAL_INTERPRETER_ENVIRONMENT)
    }

    pub fn set_lexenv(&mut self, env: Value) {
        self.set_plain_value(sym::INTERNAL_INTERPRETER_ENVIRONMENT, env);
    }

    pub fn record_unwind(&mut self, f: UnwindFn, arg: Value) {
        self.specpdl.push(SpecBinding::Unwind(f, arg));
    }

    pub fn safe_funcall(&mut self, args: &[Value]) -> LispResult {
        let count = self.specpdl.len();
        self.specbind(sym::INHIBIT_REDISPLAY, sym::T)?;
        let list = self.list(args);
        self.stack.push(list);
        let r = self.condition_case_internal(
            sym::T,
            |l| {
                let items = l.list_to_vec(list)?;
                l.funcall(items[0], &items[1..])
            },
            |l, err| {
                l.add_to_log("Error muted by safe_call: %S signaled %S", &[list, err])?;
                Ok(Value::NIL)
            },
        );
        self.stack.pop();
        self.unbind_to_value(count, r)
    }

    pub fn safe_calln(&mut self, function: Value, args: &[Value]) -> LispResult {
        let mut call = Vec::with_capacity(args.len() + 1);
        call.push(function);
        call.extend_from_slice(args);
        self.safe_funcall(&call)
    }

    pub fn specbind(&mut self, symbol: Value, value: Value) -> LispResult<()> {
        let symbol = self.check_symbol(symbol)?;
        let mut target = symbol;
        loop {
            let s = self.heap.symbol_ref(target);
            match &s.value {
                SymbolValue::Alias(next) => target = *next,
                SymbolValue::Plain(old) => {
                    let old = *old;
                    let trapped = s.trapped_write != TrappedWrite::Untrapped;
                    self.specpdl.push(SpecBinding::Let {
                        symbol: target,
                        old,
                    });
                    if trapped {
                        self.set_internal(target, value, Value::NIL, SetMode::Bind)?;
                    } else {
                        self.set_plain_value(target, value);
                    }
                    return Ok(());
                }
                SymbolValue::Localized(l) => {
                    let per_buffer = l.per_buffer.is_some();
                    let old = self.find_symbol_value(target);
                    let buffer = self.current_buffer_value();
                    if self.local_binding_exists(target) {
                        self.specpdl.push(SpecBinding::LetLocal {
                            symbol: target,
                            old,
                            buffer,
                        });
                        self.set_internal(target, value, Value::NIL, SetMode::Bind)?;
                    } else {
                        self.specpdl.push(SpecBinding::LetDefault {
                            symbol: target,
                            old,
                            buffer,
                        });
                        if per_buffer {
                            self.set_default_internal(target, value, SetMode::Bind)?;
                        } else {
                            self.set_internal(target, value, Value::NIL, SetMode::Bind)?;
                        }
                    }
                    return Ok(());
                }
                SymbolValue::Forwarded(_) => {
                    let old = self.find_symbol_value(target);
                    self.specpdl.push(SpecBinding::Let {
                        symbol: target,
                        old,
                    });
                    self.set_internal(target, value, Value::NIL, SetMode::Bind)?;
                    return Ok(());
                }
            }
        }
    }

    pub fn unbind_to(&mut self, count: usize) -> LispResult<()> {
        while self.specpdl.len() > count {
            let binding = self.specpdl.pop().expect("specpdl entry");
            self.do_one_unbind(binding)?;
        }
        Ok(())
    }

    pub fn unbind_to_value(&mut self, count: usize, value: LispResult) -> LispResult {
        match value {
            Ok(v) => {
                self.stack.push(v);
                let r = self.unbind_to(count);
                self.stack.pop();
                r.map(|()| v)
            }
            Err(flow) => Err(flow),
        }
    }

    fn do_one_unbind(&mut self, binding: SpecBinding) -> LispResult<()> {
        match binding {
            SpecBinding::Let { symbol, old } => {
                let s = self.heap.symbol_ref(symbol);
                match s.value {
                    SymbolValue::Plain(_) if s.trapped_write == TrappedWrite::Untrapped => {
                        self.set_plain_value(symbol, old);
                        Ok(())
                    }
                    SymbolValue::Plain(_) => {
                        self.set_internal(symbol, old, Value::NIL, SetMode::Unbind)
                    }
                    _ => self.set_default_internal(symbol, old, SetMode::Unbind),
                }
            }
            SpecBinding::LetDefault { symbol, old, .. } => {
                self.set_default_internal(symbol, old, SetMode::Unbind)
            }
            SpecBinding::LetLocal {
                symbol,
                old,
                buffer,
            } => {
                if self.buffer_live_p(buffer) && self.local_variable_in(symbol, buffer) {
                    self.set_internal(symbol, old, buffer, SetMode::Unbind)
                } else {
                    Ok(())
                }
            }
            SpecBinding::UnwindForms(forms) => {
                self.stack.push(forms);
                let r = self.progn(forms);
                self.stack.pop();
                r.map(|_| ())
            }
            SpecBinding::Unwind(f, arg) => {
                self.stack.push(arg);
                let r = f(self, arg);
                self.stack.pop();
                r
            }
            SpecBinding::Backtrace { .. } | SpecBinding::Nop => Ok(()),
        }
    }

    fn check_eval_depth(&mut self) -> LispResult<()> {
        self.lisp_eval_depth += 1;
        let mut max = self.int_var(sym::MAX_LISP_EVAL_DEPTH);
        if self.lisp_eval_depth > max {
            if max < 100 {
                max = 100;
                self.set_forwarded_int(sym::MAX_LISP_EVAL_DEPTH, 100);
            }
            if self.lisp_eval_depth > max {
                let depth = Value::fixnum(self.lisp_eval_depth);
                let reserve = self.int_var(sym::LISP_EVAL_DEPTH_RESERVE);
                self.set_forwarded_int(sym::MAX_LISP_EVAL_DEPTH, max + reserve.max(0));
                let flow = self.signal_list(sym::EXCESSIVE_LISP_NESTING, &[depth]);
                self.set_forwarded_int(sym::MAX_LISP_EVAL_DEPTH, max);
                return Err(flow);
            }
        }
        Ok(())
    }

    pub fn set_forwarded_int(&mut self, symbol: Value, n: i64) {
        if let SymbolValue::Forwarded(Forward::Int(v)) = &mut self.heap.symbol_mut(symbol).value {
            *v = n;
        }
    }

    pub fn set_forwarded_bool(&mut self, symbol: Value, b: bool) {
        if let SymbolValue::Forwarded(Forward::Bool(v)) = &mut self.heap.symbol_mut(symbol).value {
            *v = b;
        }
    }

    pub fn eval(&mut self, form: Value) -> LispResult {
        let bare = self.maybe_remove_pos_from_symbol(form);
        if bare.is_symbol() {
            if bare.is_nil() || bare == sym::T {
                return Ok(bare);
            }
            let lexenv = self.lexenv();
            if lexenv.is_cons() {
                let binding = if self.var_true(sym::SYMBOLS_WITH_POS_ENABLED) {
                    crate::fns::f_assq(self, &[form, lexenv])?
                } else {
                    self.assq(form, lexenv)
                };
                if !binding.is_nil() {
                    return Ok(self.xcdr(binding));
                }
            }
            return self.symbol_value(bare);
        }
        if !form.is_cons() {
            return Ok(form);
        }
        self.maybe_gc();
        self.check_eval_depth()?;
        let original_fun = self.xcar(form);
        let original_args = self.xcdr(form);
        if !original_args.is_cons() && !original_args.is_nil() {
            self.lisp_eval_depth -= 1;
            return Err(self.wrong_type(sym::LISTP, original_args));
        }
        let count = self.specpdl.len();
        self.specpdl.push(SpecBinding::Backtrace {
            function: original_fun,
            args: BacktraceArgs::Unevalled(original_args),
            debug_on_exit: false,
        });
        let base = self.stack.len();
        let result = self.eval_call(original_fun, original_args, count, base);
        self.stack.truncate(base);
        match result {
            Ok(value) => {
                self.lisp_eval_depth -= 1;
                let value = self.maybe_debug_on_exit(count, value)?;
                self.specpdl.truncate(count);
                Ok(value)
            }
            Err(flow) => Err(flow),
        }
    }

    fn eval_call(
        &mut self,
        original_fun: Value,
        original_args: Value,
        count: usize,
        base: usize,
    ) -> LispResult {
        loop {
            let mut fun = original_fun;
            let bare = self.maybe_remove_pos_from_symbol(fun);
            if !bare.is_symbol() {
                let list = self.list(&[fun]);
                self.stack.push(list);
                fun = f_function(self, list)?;
                self.stack.push(fun);
            } else if !bare.is_nil() {
                fun = self.heap.symbol_ref(bare).function;
                if fun.is_symbol() {
                    fun = self.indirect_function(fun);
                }
            }
            if let Some(subr) = self.subr(fun) {
                let numargs = self.list_length_unchecked(original_args);
                let max = subr.max_args();
                if numargs < subr.min_args as usize || max.is_some_and(|m| (m as usize) < numargs) {
                    let n = Value::fixnum(numargs as i64);
                    return Err(
                        self.signal_list(sym::WRONG_NUMBER_OF_ARGUMENTS, &[original_fun, n])
                    );
                }
                return match subr.kind {
                    SubrKind::Special(f) => f(self, original_args),
                    SubrKind::Fixed(max, f) => {
                        let start = self.stack.len();
                        self.eval_args_onto_stack(original_args)?;
                        let len = self.stack.len() - start;
                        self.set_backtrace_args(count, start, len);
                        let mut args = [Value::NIL; 8];
                        args[..len].copy_from_slice(&self.stack[start..]);
                        f(self, &args[..max as usize])
                    }
                    SubrKind::Many(f) => {
                        let start = self.stack.len();
                        self.eval_args_onto_stack(original_args)?;
                        let len = self.stack.len() - start;
                        self.set_backtrace_args(count, start, len);
                        let args: Vec<Value> = self.stack[start..].to_vec();
                        f(self, &args)
                    }
                };
            }
            if fun.is_vectorlike() && self.is_closure(fun) {
                return self.apply_lambda(fun, original_args, count, base);
            }
            if fun.is_nil() {
                return Err(self.signal_list(sym::VOID_FUNCTION, &[original_fun]));
            }
            if !fun.is_cons() {
                return Err(self.signal_list(sym::INVALID_FUNCTION, &[original_fun]));
            }
            let funcar = self.xcar(fun);
            if !funcar.is_symbol() {
                return Err(self.signal_list(sym::INVALID_FUNCTION, &[original_fun]));
            }
            if funcar == sym::AUTOLOAD {
                self.stack.push(fun);
                self.autoload_do_load(fun, original_fun, Value::NIL)?;
                continue;
            }
            if funcar == sym::MACRO {
                let count1 = self.specpdl.len();
                let lexical = if self.lexenv().is_nil() {
                    Value::NIL
                } else {
                    sym::T
                };
                self.specbind(sym::LEXICAL_BINDING, lexical)?;
                let mut dynvars = self.plain_value(sym::MACROEXP__DYNVARS);
                let original_dynvars = dynvars;
                let mut p = self.lexenv();
                while p.is_cons() {
                    let e = self.xcar(p);
                    if e.is_symbol() {
                        dynvars = self.cons(e, dynvars);
                    }
                    p = self.xcdr(p);
                }
                if dynvars != original_dynvars {
                    self.specbind(sym::MACROEXP__DYNVARS, dynvars)?;
                }
                let expander = self.xcdr(fun);
                self.stack.push(fun);
                let expansion = self.apply1(expander, original_args)?;
                let expansion = self.unbind_to_value(count1, Ok(expansion))?;
                self.stack.push(expansion);
                return self.eval(expansion);
            }
            if funcar == sym::LAMBDA {
                return self.apply_lambda(fun, original_args, count, base);
            }
            return Err(self.signal_list(sym::INVALID_FUNCTION, &[original_fun]));
        }
    }

    fn eval_args_onto_stack(&mut self, mut args: Value) -> LispResult<()> {
        while args.is_cons() {
            let arg = self.xcar(args);
            args = self.xcdr(args);
            let value = self.eval(arg)?;
            self.stack.push(value);
        }
        Ok(())
    }

    fn set_backtrace_args(&mut self, count: usize, start: usize, len: usize) {
        if let Some(SpecBinding::Backtrace { args, .. }) = self.specpdl.get_mut(count) {
            *args = BacktraceArgs::Evaluated { start, len };
        }
    }

    pub(crate) fn maybe_debug_on_exit(&mut self, count: usize, value: Value) -> LispResult {
        if let Some(SpecBinding::Backtrace {
            debug_on_exit: true,
            ..
        }) = self.specpdl.get(count)
        {
            let args = self.list(&[sym::EXIT, value]);
            return self.call_debugger(args);
        }
        Ok(value)
    }

    fn apply_lambda(&mut self, fun: Value, args: Value, count: usize, _base: usize) -> LispResult {
        let start = self.stack.len();
        self.stack.push(fun);
        let start = start + 1;
        self.eval_args_onto_stack(args)?;
        let len = self.stack.len() - start;
        self.set_backtrace_args(count, start, len);
        let argv: Vec<Value> = self.stack[start..].to_vec();
        self.funcall_lambda(fun, &argv)
    }

    pub fn indirect_function(&self, mut object: Value) -> Value {
        while object.is_symbol() && !object.is_nil() {
            object = self.heap.symbol_ref(object).function;
        }
        object
    }

    pub fn list_length_unchecked(&self, mut list: Value) -> usize {
        let mut n = 0;
        while list.is_cons() {
            n += 1;
            list = self.xcdr(list);
        }
        n
    }

    pub fn progn(&mut self, mut body: Value) -> LispResult {
        let mut value = Value::NIL;
        while body.is_cons() {
            let form = self.xcar(body);
            body = self.xcdr(body);
            value = self.eval(form)?;
        }
        Ok(value)
    }

    pub fn funcall(&mut self, function: Value, args: &[Value]) -> LispResult {
        let base = self.stack.len();
        self.stack.push(function);
        self.stack.extend_from_slice(args);
        let result = self.funcall_rooted(base);
        self.stack.truncate(base);
        result
    }

    fn funcall_rooted(&mut self, base: usize) -> LispResult {
        self.check_eval_depth()?;
        let function = self.stack[base];
        let count = self.specpdl.len();
        self.specpdl.push(SpecBinding::Backtrace {
            function,
            args: BacktraceArgs::Evaluated {
                start: base + 1,
                len: self.stack.len() - base - 1,
            },
            debug_on_exit: false,
        });
        self.maybe_gc();
        let args: Vec<Value> = self.stack[base + 1..].to_vec();
        let value = self.funcall_general(function, &args)?;
        self.lisp_eval_depth -= 1;
        let value = self.maybe_debug_on_exit(count, value)?;
        self.specpdl.truncate(count);
        Ok(value)
    }

    pub fn funcall_general(&mut self, original_fun: Value, args: &[Value]) -> LispResult {
        let mut fun = original_fun;
        loop {
            let bare = self.maybe_remove_pos_from_symbol(fun);
            if bare.is_symbol() && !bare.is_nil() {
                fun = self.heap.symbol_ref(bare).function;
                if fun.is_symbol() {
                    fun = self.indirect_function(fun);
                }
            }
            if let Some(subr) = self.subr(fun) {
                return self.funcall_subr(fun, subr, args);
            }
            if fun.is_vectorlike() && self.is_closure(fun) {
                return self.funcall_lambda(fun, args);
            }
            if fun.is_nil() {
                return Err(self.signal_list(sym::VOID_FUNCTION, &[original_fun]));
            }
            if !fun.is_cons() {
                return Err(self.signal_list(sym::INVALID_FUNCTION, &[original_fun]));
            }
            let funcar = self.xcar(fun);
            if !funcar.is_symbol() {
                return Err(self.signal_list(sym::INVALID_FUNCTION, &[original_fun]));
            }
            if funcar == sym::LAMBDA {
                return self.funcall_lambda(fun, args);
            }
            if funcar == sym::AUTOLOAD {
                self.stack.push(fun);
                self.autoload_do_load(fun, original_fun, Value::NIL)?;
                fun = original_fun;
                continue;
            }
            return Err(self.signal_list(sym::INVALID_FUNCTION, &[original_fun]));
        }
    }

    pub fn funcall_subr(&mut self, fun: Value, subr: &'static Subr, args: &[Value]) -> LispResult {
        let n = args.len();
        if n >= subr.min_args as usize {
            match subr.kind {
                SubrKind::Fixed(max, f) if n <= max as usize => {
                    let mut buf = [Value::NIL; 8];
                    buf[..n].copy_from_slice(args);
                    return f(self, &buf[..max as usize]);
                }
                SubrKind::Many(f) => return f(self, args),
                _ => {}
            }
        }
        if subr.is_special_form() {
            Err(self.signal_list(sym::INVALID_FUNCTION, &[fun]))
        } else {
            let n = Value::fixnum(n as i64);
            Err(self.signal_list(sym::WRONG_NUMBER_OF_ARGUMENTS, &[fun, n]))
        }
    }

    pub fn funcall_lambda(&mut self, fun: Value, args: &[Value]) -> LispResult {
        let (mut syms_left, mut lexenv, body) = if fun.is_cons() {
            let cdr = self.xcdr(fun);
            if !cdr.is_cons() {
                return Err(self.signal_list(sym::INVALID_FUNCTION, &[fun]));
            }
            (self.xcar(cdr), Value::NIL, self.xcdr(cdr))
        } else {
            let slots = self.vector_items(fun);
            let arglist = slots.first().copied().unwrap_or(Value::NIL);
            let code = slots.get(1).copied().unwrap_or(Value::NIL);
            let constants = slots.get(2).copied().unwrap_or(Value::NIL);
            if let Some(template) = arglist.as_fixnum() {
                return crate::bytecode::exec_byte_code(self, fun, template, args);
            }
            let lexenv = if code.is_cons() {
                constants
            } else {
                Value::NIL
            };
            (arglist, lexenv, code)
        };
        let count = self.specpdl.len();
        let mut i = 0usize;
        let mut optional = false;
        let mut rest = false;
        let mut previous_rest = false;
        while syms_left.is_cons() {
            let next = self.xcar(syms_left);
            let next = self.maybe_remove_pos_from_symbol(next);
            if !next.is_symbol() {
                return Err(self.signal_list(sym::INVALID_FUNCTION, &[fun]));
            }
            if next == sym::AND_REST {
                if rest || previous_rest {
                    return Err(self.signal_list(sym::INVALID_FUNCTION, &[fun]));
                }
                rest = true;
                previous_rest = true;
            } else if next == sym::AND_OPTIONAL {
                if optional || rest || previous_rest {
                    return Err(self.signal_list(sym::INVALID_FUNCTION, &[fun]));
                }
                optional = true;
            } else {
                let arg = if rest {
                    let v = self.list(&args[i.min(args.len())..]);
                    i = args.len();
                    v
                } else if i < args.len() {
                    i += 1;
                    args[i - 1]
                } else if !optional {
                    let n = Value::fixnum(args.len() as i64);
                    return Err(self.signal_list(sym::WRONG_NUMBER_OF_ARGUMENTS, &[fun, n]));
                } else {
                    Value::NIL
                };
                if !lexenv.is_nil() {
                    let pair = self.cons(next, arg);
                    lexenv = self.cons(pair, lexenv);
                    self.stack.push(lexenv);
                } else {
                    self.specbind(next, arg)?;
                }
                previous_rest = false;
            }
            syms_left = self.xcdr(syms_left);
        }
        if !syms_left.is_nil() || previous_rest {
            return Err(self.signal_list(sym::INVALID_FUNCTION, &[fun]));
        }
        if i < args.len() {
            let n = Value::fixnum(args.len() as i64);
            return Err(self.signal_list(sym::WRONG_NUMBER_OF_ARGUMENTS, &[fun, n]));
        }
        if lexenv != self.lexenv() {
            self.specbind(sym::INTERNAL_INTERPRETER_ENVIRONMENT, lexenv)?;
        }
        let value = if fun.is_cons() || body.is_cons() {
            self.progn(body)
        } else {
            crate::bytecode::exec_byte_code(self, fun, 0, &[])
        };
        self.unbind_to_value(count, value)
    }

    pub fn maybe_remove_pos_from_symbol(&self, v: Value) -> Value {
        if v.is_vectorlike()
            && self.var_true(sym::SYMBOLS_WITH_POS_ENABLED)
            && let Vectorlike::SymbolWithPos { symbol, .. } = self.heap.vectorlike_ref(v)
        {
            return *symbol;
        }
        v
    }

    pub fn apply1(&mut self, function: Value, args: Value) -> LispResult {
        if args.is_nil() {
            self.funcall(function, &[])
        } else {
            self.stack.push(args);
            let argv = self.list_to_vec(args);
            self.stack.pop();
            let argv = argv?;
            self.funcall(function, &argv)
        }
    }

    pub fn call0(&mut self, function: Value) -> LispResult {
        self.funcall(function, &[])
    }

    pub fn signal(&mut self, error_symbol: Value, data: Value) -> Flow {
        match self.signal_or_quit(error_symbol, data, false) {
            Ok(_) => unreachable!("non-continuable signal returned"),
            Err(flow) => flow,
        }
    }

    fn process_quit_flag(&mut self) -> LispResult<()> {
        let flag = self.plain_value(sym::QUIT_FLAG);
        self.set_plain_value(sym::QUIT_FLAG, Value::NIL);
        if flag == sym::KILL_EMACS {
            crate::emacs::f_kill_emacs(self, &[Value::NIL, Value::NIL])?;
        }
        let throw_on_input = self.plain_value(sym::THROW_ON_INPUT);
        if throw_on_input == flag {
            return Err(self.throw(throw_on_input, sym::T));
        }
        self.quit()
    }

    pub fn probably_quit(&mut self) -> LispResult<()> {
        self.gc_inhibited += 1;
        let r = if self.var_true(sym::QUIT_FLAG) && !self.var_true(sym::INHIBIT_QUIT) {
            self.process_quit_flag()
        } else {
            Ok(())
        };
        self.gc_inhibited -= 1;
        r
    }

    pub fn maybe_quit(&mut self) -> LispResult<()> {
        if self.var_true(sym::QUIT_FLAG) {
            self.probably_quit()?;
        }
        Ok(())
    }

    pub fn quit(&mut self) -> LispResult<()> {
        self.signal_or_quit(sym::QUIT, Value::NIL, true)?;
        Ok(())
    }

    pub fn signal_or_quit(
        &mut self,
        error_symbol: Value,
        data: Value,
        continuable: bool,
    ) -> LispResult<Option<Flow>> {
        let oom = error_symbol.is_nil();
        let error = if oom {
            data
        } else if !error_symbol.is_symbol() && data.is_nil() {
            error_symbol
        } else {
            self.cons(error_symbol, data)
        };
        let base = self.stack.len();
        self.stack.push(error);
        let real_error_symbol = if error.is_cons() {
            self.xcar(error)
        } else {
            error_symbol
        };
        let hook = self.plain_value(sym::SIGNAL_HOOK_FUNCTION);
        if !hook.is_nil() && !oom {
            let hook_data = if data.is_nil() && error.is_cons() {
                self.xcdr(error)
            } else {
                data
            };
            let count = self.specpdl.len();
            let r = self.funcall(hook, &[real_error_symbol, hook_data]);
            self.unbind_to(count)?;
            r?;
        }
        let conditions = if real_error_symbol.is_symbol() {
            self.get(real_error_symbol, sym::ERROR_CONDITIONS)
        } else {
            Value::NIL
        };
        if conditions.is_nil() {
            self.stack.truncate(base);
            return Err(self.signal_error("Invalid error symbol", real_error_symbol));
        }
        self.stack.push(conditions);
        self.set_signaling_function();
        let mut clause = Value::NIL;
        let mut found = UNCAUGHT;
        let mut i = self.handlers.len() as isize - 1;
        while i >= 0 {
            let index = i as usize;
            let h = self.handlers[index];
            match h.kind {
                HandlerKind::CatchAll => clause = sym::T,
                HandlerKind::CatchAllDebuggable => clause = sym::DEBUG,
                HandlerKind::Catch(_) => {
                    i -= 1;
                    continue;
                }
                HandlerKind::ConditionCase(list) => {
                    clause = self.find_handler_clause(list, conditions);
                }
                HandlerKind::HandlerBind {
                    conditions: hconds,
                    handler,
                    skip,
                } => {
                    if !self.find_handler_clause(hconds, conditions).is_nil() {
                        let distance = self.handlers.len() - 1 - index;
                        let count = self.specpdl.len();
                        let pushed =
                            self.push_handler(HandlerKind::SkipConditions(distance + skip));
                        let r = self.funcall(handler, &[error]);
                        match r {
                            Ok(_) => {
                                self.unbind_to(count)?;
                                self.pop_handler(pushed);
                            }
                            Err(flow) => {
                                self.stack.truncate(base);
                                return Err(flow);
                            }
                        }
                    }
                    i -= 1;
                    continue;
                }
                HandlerKind::SkipConditions(n) => {
                    i -= n as isize + 2;
                    continue;
                }
            }
            if !clause.is_nil() {
                found = index;
                break;
            }
            i -= 1;
        }
        let wants_debug = !oom
            && (!self.plain_value(sym::DEBUG_ON_SIGNAL).is_nil()
                || clause.is_nil()
                || clause == sym::DEBUG
                || (clause.is_cons() && !self.memq(sym::DEBUG, clause).is_nil())
                || clause == sym::ERROR);
        if wants_debug {
            let called = self.maybe_call_debugger(conditions, error)?;
            if continuable && called {
                self.stack.truncate(base);
                return Ok(None);
            }
        }
        let level = self.handler_level(found);
        self.save_backtrace_args(level);
        self.stack.truncate(base);
        self.flow_roots.push(error);
        Err(Flow::Signal {
            error,
            target: found,
        })
    }

    fn set_signaling_function(&mut self) {
        let mut function = Value::NIL;
        let mut seen_top = false;
        for binding in self.specpdl.iter().rev() {
            if let SpecBinding::Backtrace { function: f, .. } = binding {
                if !seen_top {
                    seen_top = true;
                    continue;
                }
                if *f == sym::ERROR {
                    continue;
                }
                function = *f;
                break;
            }
        }
        self.set_plain_value(sym::SIGNALING_FUNCTION, function);
    }

    fn find_handler_clause(&self, handlers: Value, conditions: Value) -> Value {
        if !handlers.is_cons() {
            return handlers;
        }
        let mut h = handlers;
        while h.is_cons() {
            let handler = self.xcar(h);
            if !self.memq(handler, conditions).is_nil() || handler == sym::T {
                return handlers;
            }
            h = self.xcdr(h);
        }
        Value::NIL
    }

    fn maybe_call_debugger(&mut self, conditions: Value, error: Value) -> LispResult<bool> {
        if !self.plain_value(sym::INHIBIT_DEBUGGER).is_nil() {
            return Ok(false);
        }
        let quit = self.signal_quit_p(error);
        let wants = if quit {
            self.var_true(sym::DEBUG_ON_QUIT)
        } else {
            let list = self.plain_value(sym::DEBUG_ON_ERROR);
            self.wants_debugger(list, conditions)
        };
        if !wants || self.skip_debugger(conditions) {
            return Ok(false);
        }
        let args = self.list(&[sym::ERROR, error]);
        self.call_debugger(args)?;
        Ok(true)
    }

    fn wants_debugger(&self, list: Value, mut conditions: Value) -> bool {
        if list.is_nil() {
            return false;
        }
        if !list.is_cons() {
            return true;
        }
        while conditions.is_cons() {
            let this = self.xcar(conditions);
            if !self.memq(this, list).is_nil() {
                return true;
            }
            conditions = self.xcdr(conditions);
        }
        false
    }

    fn skip_debugger(&self, conditions: Value) -> bool {
        let mut tail = self.plain_value(sym::DEBUG_IGNORED_ERRORS);
        while tail.is_cons() {
            let item = self.xcar(tail);
            if item.is_symbol() && !self.memq(item, conditions).is_nil() {
                return true;
            }
            tail = self.xcdr(tail);
        }
        false
    }

    pub fn signal_quit_p(&self, error: Value) -> bool {
        let signal = self.car_safe(error);
        if signal == sym::QUIT {
            return true;
        }
        if signal.is_symbol() {
            let list = self.get(signal, sym::ERROR_CONDITIONS);
            return list.is_cons() && !self.memq(sym::QUIT, list).is_nil();
        }
        false
    }

    pub fn call_debugger(&mut self, args: Value) -> LispResult {
        let debugger = self.plain_value(sym::DEBUGGER);
        if debugger.is_nil() || debugger == sym::UNBOUND {
            return Ok(Value::NIL);
        }
        let count = self.specpdl.len();
        self.specbind(sym::DEBUG_ON_ERROR, Value::NIL)?;
        self.specbind(sym::DEBUG_ON_SIGNAL, Value::NIL)?;
        self.specbind(sym::INHIBIT_DEBUGGER, sym::T)?;
        let r = self.apply1(debugger, args);
        self.unbind_to_value(count, r)
    }

    pub fn throw(&mut self, tag: Value, value: Value) -> Flow {
        if !tag.is_nil()
            && let Some(index) = self.handlers.iter().rposition(|h| match h.kind {
                HandlerKind::CatchAll | HandlerKind::CatchAllDebuggable => true,
                HandlerKind::Catch(t) => self.eq(t, tag),
                _ => false,
            })
        {
            let level = self.handlers[index].specpdl;
            self.flow_roots.extend_from_slice(&[tag, value]);
            self.save_backtrace_args(level);
            return Flow::Throw {
                tag,
                value,
                target: index,
            };
        }
        self.signal_list(sym::NO_CATCH, &[tag, value])
    }

    pub fn eq(&self, a: Value, b: Value) -> bool {
        if a == b {
            return true;
        }
        if self.var_true(sym::SYMBOLS_WITH_POS_ENABLED) {
            let a = self.maybe_remove_pos_from_symbol(a);
            let b = self.maybe_remove_pos_from_symbol(b);
            return a == b;
        }
        false
    }

    pub fn internal_catch<F>(&mut self, tag: Value, body: F) -> LispResult
    where
        F: FnOnce(&mut Lisp) -> LispResult,
    {
        self.stack.push(tag);
        let index = self.push_handler(HandlerKind::Catch(tag));
        let result = body(self);
        match result {
            Ok(v) => {
                self.pop_handler(index);
                self.stack.pop();
                Ok(v)
            }
            Err(flow @ Flow::Throw { target, value, .. }) if target == index => {
                self.unwind_to_handler(index, flow)?;
                self.pop_handler(index);
                self.stack.pop();
                Ok(value)
            }
            Err(flow) => Err(flow),
        }
    }

    pub fn condition_case_internal<F, H>(
        &mut self,
        handlers: Value,
        body: F,
        handler: H,
    ) -> LispResult
    where
        F: FnOnce(&mut Lisp) -> LispResult,
        H: FnOnce(&mut Lisp, Value) -> LispResult,
    {
        let index = self.push_handler(HandlerKind::ConditionCase(handlers));
        match body(self) {
            Ok(v) => {
                self.pop_handler(index);
                Ok(v)
            }
            Err(flow @ Flow::Signal { error, target }) if target == index => {
                let level = self.handlers[index].flow_roots;
                self.unwind_to_handler(index, flow)?;
                self.handlers.truncate(index);
                let r = handler(self, error);
                self.finish_catch(level, r)
            }
            Err(flow) => Err(flow),
        }
    }

    pub fn autoload_do_load(
        &mut self,
        fundef: Value,
        funname: Value,
        macro_only: Value,
    ) -> LispResult {
        if !fundef.is_cons() || self.xcar(fundef) != sym::AUTOLOAD {
            return Ok(fundef);
        }
        let kind = self.nth(4, fundef);
        if macro_only == sym::MACRO && !(kind == sym::T || kind == sym::MACRO) {
            return Ok(fundef);
        }
        let funname = self.check_symbol(funname)?;
        let ignore_errors = if kind == sym::T || kind == sym::MACRO {
            Value::NIL
        } else {
            macro_only
        };
        let file = self.nth(1, fundef);
        self.stack.push(fundef);
        self.load_with_autoload_queue(file, ignore_errors)?;
        if funname.is_nil() || !ignore_errors.is_nil() {
            return Ok(Value::NIL);
        }
        let fun = self.indirect_function(funname);
        if self.equal(fun, fundef)? {
            let history = self.plain_value(sym::LOAD_HISTORY);
            let first = self.car_safe(self.car_safe(history));
            let file = if first.is_string() {
                self.string_to_rust(first)
            } else {
                String::new()
            };
            let name = self.symbol_name_string(funname);
            return Err(self.error(&format!(
                "Autoloading file {file} failed to define function {name}"
            )));
        }
        Ok(fun)
    }

    fn load_with_autoload_queue(&mut self, file: Value, noerror: Value) -> LispResult {
        let count = self.specpdl.len();
        let old = self.plain_value(sym::AUTOLOAD_QUEUE);
        self.record_unwind(un_autoload, old);
        self.set_plain_value(sym::AUTOLOAD_QUEUE, sym::T);
        let r = self.load(file, noerror, sym::T, Value::NIL, sym::T);
        let v = r?;
        self.set_plain_value(sym::AUTOLOAD_QUEUE, sym::T);
        self.unbind_to_value(count, Ok(v))
    }

    pub fn functionp(&mut self, object: Value) -> bool {
        let mut object = object;
        if self.symbolp(object) {
            let bare = self.maybe_remove_pos_from_symbol(object);
            if !bare.is_nil() && !self.heap.symbol_ref(bare).function.is_nil() {
                object = self.indirect_function(bare);
            }
            if object.is_cons() && self.eq(self.xcar(object), sym::AUTOLOAD) {
                let mut o = object;
                for _ in 0..4 {
                    if !o.is_cons() {
                        break;
                    }
                    o = self.xcdr(o);
                }
                return !(o.is_cons() && !self.xcar(o).is_nil());
            }
        }
        if let Some(subr) = self.subr(object) {
            return !subr.is_special_form();
        }
        if object.is_vectorlike() {
            return self.is_closure(object);
        }
        if object.is_cons() {
            return self.eq(self.xcar(object), sym::LAMBDA);
        }
        false
    }

    pub fn run_hook_with_args<F>(&mut self, args: &[Value], mut call: F) -> LispResult
    where
        F: FnMut(&mut Lisp, &[Value]) -> LispResult,
    {
        if self.plain_value(sym::RUN_HOOKS_ENABLED).is_nil() {
            return Ok(Value::NIL);
        }
        let hook = args[0];
        let val = self.find_symbol_value(hook);
        let mut ret = Value::NIL;
        if val == sym::UNBOUND || val.is_nil() {
            return Ok(ret);
        }
        let base = self.stack.len();
        self.stack.extend_from_slice(args);
        let mut argv = args.to_vec();
        if !val.is_cons() || self.functionp(val) {
            argv[0] = val;
            let r = call(self, &argv);
            self.stack.truncate(base);
            return r;
        }
        self.stack.push(val);
        let mut tail = val;
        while tail.is_cons() && ret.is_nil() {
            let item = self.xcar(tail);
            if item == sym::T {
                let globals = self.default_value(hook)?;
                if globals.is_nil() {
                    tail = self.xcdr(tail);
                    continue;
                }
                self.stack.push(globals);
                if !globals.is_cons() || self.xcar(globals) == sym::LAMBDA {
                    argv[0] = globals;
                    ret = match call(self, &argv) {
                        Ok(v) => v,
                        Err(e) => {
                            self.stack.truncate(base);
                            return Err(e);
                        }
                    };
                } else {
                    let mut g = globals;
                    while g.is_cons() && ret.is_nil() {
                        argv[0] = self.xcar(g);
                        if argv[0] != sym::T {
                            ret = match call(self, &argv) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.stack.truncate(base);
                                    return Err(e);
                                }
                            };
                        }
                        g = self.xcdr(g);
                    }
                }
            } else {
                argv[0] = item;
                ret = match call(self, &argv) {
                    Ok(v) => v,
                    Err(e) => {
                        self.stack.truncate(base);
                        return Err(e);
                    }
                };
            }
            tail = self.xcdr(tail);
        }
        self.stack.truncate(base);
        Ok(ret)
    }

    pub fn run_hook(&mut self, hook: Value) -> LispResult<()> {
        self.run_hook_with_args(&[hook], |l, a| l.funcall(a[0], &a[1..]).map(|_| Value::NIL))?;
        Ok(())
    }

    pub fn eval_with_lexical(&mut self, form: Value, lexical: Value) -> LispResult {
        let count = self.specpdl.len();
        let env = if lexical.is_cons() || lexical.is_nil() {
            lexical
        } else {
            self.list_of_t
        };
        self.specbind(sym::INTERNAL_INTERPRETER_ENVIRONMENT, env)?;
        let r = self.eval(form);
        self.unbind_to_value(count, r)
    }

    fn backtrace_frames(&self) -> Vec<usize> {
        self.specpdl
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, b)| matches!(b, SpecBinding::Backtrace { .. }))
            .map(|(i, _)| i)
            .collect()
    }

    fn backtrace_frame_args(&mut self, index: usize) -> (Value, Value, bool) {
        match self.specpdl[index] {
            SpecBinding::Backtrace {
                function,
                args: BacktraceArgs::Unevalled(list),
                ..
            } => (function, list, false),
            SpecBinding::Backtrace {
                function,
                args: BacktraceArgs::Evaluated { start, len },
                ..
            } => {
                let items: Vec<Value> = self.stack[start..start + len].to_vec();
                let list = self.list(&items);
                (function, list, true)
            }
            SpecBinding::Backtrace {
                function,
                args: BacktraceArgs::Saved(list),
                ..
            } => (function, list, true),
            _ => (Value::NIL, Value::NIL, false),
        }
    }

    fn save_backtrace_args(&mut self, level: usize) {
        for index in level..self.specpdl.len() {
            if let SpecBinding::Backtrace {
                args: BacktraceArgs::Evaluated { start, len },
                ..
            } = self.specpdl[index]
                && let Some(items) = self.stack.get(start..start + len)
            {
                let items = items.to_vec();
                let list = self.list(&items);
                if let SpecBinding::Backtrace { args, .. } = &mut self.specpdl[index] {
                    *args = BacktraceArgs::Saved(list);
                }
            }
        }
    }

    fn handler_level(&self, target: usize) -> usize {
        self.handlers.get(target).map_or(0, |h| h.specpdl)
    }

    fn get_backtrace_starting_at(&mut self, base: Value) -> Option<usize> {
        let frames = self.backtrace_frames();
        if base.is_nil() {
            return frames.first().copied();
        }
        let base = self.indirect_function_aware(base);
        frames.into_iter().find(|&i| match self.specpdl[i] {
            SpecBinding::Backtrace { function, .. } => function == base,
            _ => false,
        })
    }

    fn indirect_function_aware(&self, v: Value) -> Value {
        v
    }
}

fn un_autoload(lisp: &mut Lisp, oldqueue: Value) -> LispResult<()> {
    let queue = lisp.plain_value(sym::AUTOLOAD_QUEUE);
    lisp.set_plain_value(sym::AUTOLOAD_QUEUE, oldqueue);
    let mut q = queue;
    while q.is_cons() {
        let first = lisp.xcar(q);
        if first.is_cons() && lisp.xcar(first) == Value::fixnum(0) {
            let features = lisp.xcdr(first);
            lisp.set_plain_value(sym::FEATURES, features);
        } else {
            let history = lisp.get(first, sym::FUNCTION_HISTORY);
            let previous = lisp.car_safe(lisp.cdr_safe(history));
            lisp.fset(first, previous)?;
        }
        q = lisp.xcdr(q);
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SetMode {
    Set,
    Bind,
    Unbind,
    Makunbound,
}

fn f_ignore(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_or(lisp: &mut Lisp, mut args: Value) -> LispResult {
    let mut val = Value::NIL;
    while args.is_cons() {
        let arg = lisp.xcar(args);
        args = lisp.xcdr(args);
        val = lisp.eval(arg)?;
        if !val.is_nil() {
            break;
        }
    }
    Ok(val)
}

fn f_and(lisp: &mut Lisp, mut args: Value) -> LispResult {
    let mut val = sym::T;
    while args.is_cons() {
        let arg = lisp.xcar(args);
        args = lisp.xcdr(args);
        val = lisp.eval(arg)?;
        if val.is_nil() {
            break;
        }
    }
    Ok(val)
}

fn f_if(lisp: &mut Lisp, args: Value) -> LispResult {
    let cond = lisp.xcar(args);
    let cond = lisp.eval(cond)?;
    let rest = lisp.xcdr(args);
    if !cond.is_nil() {
        let then = lisp.car(rest)?;
        lisp.eval(then)
    } else {
        let else_forms = lisp.cdr(rest)?;
        lisp.progn(else_forms)
    }
}

fn f_cond(lisp: &mut Lisp, mut args: Value) -> LispResult {
    let mut val = args;
    while args.is_cons() {
        let clause = lisp.xcar(args);
        let test = lisp.car(clause)?;
        val = lisp.eval(test)?;
        if !val.is_nil() {
            let body = lisp.xcdr(clause);
            if !body.is_nil() {
                val = lisp.progn(body)?;
            }
            break;
        }
        args = lisp.xcdr(args);
    }
    Ok(val)
}

fn f_progn(lisp: &mut Lisp, body: Value) -> LispResult {
    lisp.progn(body)
}

fn f_prog1(lisp: &mut Lisp, args: Value) -> LispResult {
    let first = lisp.xcar(args);
    let val = lisp.eval(first)?;
    lisp.stack.push(val);
    let rest = lisp.xcdr(args);
    let r = lisp.progn(rest);
    lisp.stack.pop();
    r?;
    Ok(val)
}

fn f_setq(lisp: &mut Lisp, args: Value) -> LispResult {
    let mut val = args;
    let mut tail = args;
    let mut nargs = 0;
    while tail.is_cons() {
        let symbol = lisp.xcar(tail);
        tail = lisp.xcdr(tail);
        if !tail.is_cons() {
            let n = Value::fixnum(nargs + 1);
            return Err(lisp.signal_list(sym::WRONG_NUMBER_OF_ARGUMENTS, &[sym::SETQ, n]));
        }
        let arg = lisp.xcar(tail);
        tail = lisp.xcdr(tail);
        val = lisp.eval(arg)?;
        let lex_binding = if symbol.is_symbol() {
            let env = lisp.lexenv();
            lisp.assq(symbol, env)
        } else {
            Value::NIL
        };
        if !lex_binding.is_nil() {
            lisp.heap.set_cdr(lex_binding, val);
        } else {
            lisp.set(symbol, val)?;
        }
        nargs += 2;
    }
    Ok(val)
}

fn f_quote(lisp: &mut Lisp, args: Value) -> LispResult {
    if !lisp.xcdr(args).is_nil() {
        let len = lisp.length(args)?;
        return Err(lisp.signal_list(sym::WRONG_NUMBER_OF_ARGUMENTS, &[sym::QUOTE, len]));
    }
    Ok(lisp.xcar(args))
}

fn f_make_interpreted_closure(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (arglist, body, env, docstring, iform) = (args[0], args[1], args[2], args[3], args[4]);
    lisp.check_cons(body)?;
    lisp.check_list(arglist)?;
    lisp.check_list(iform)?;
    let ifcdr = lisp.cdr(iform)?;
    let value = if lisp.cdr(ifcdr)?.is_nil() {
        lisp.car(ifcdr)?
    } else {
        let a = lisp.xcar(ifcdr);
        let b = lisp.xcdr(ifcdr);
        lisp.vector(vec![a, b])
    };
    let slots = [arglist, body, env, Value::NIL, docstring, value];
    let len = if !iform.is_nil() {
        6
    } else if !docstring.is_nil() {
        5
    } else {
        3
    };
    Ok(lisp
        .heap
        .vectorlike(Vectorlike::Closure(slots[..len].to_vec())))
}

pub fn f_function(lisp: &mut Lisp, args: Value) -> LispResult {
    let quoted = lisp.xcar(args);
    if !lisp.xcdr(args).is_nil() {
        let len = lisp.length(args)?;
        return Err(lisp.signal_list(sym::WRONG_NUMBER_OF_ARGUMENTS, &[sym::FUNCTION, len]));
    }
    if quoted.is_cons() && lisp.xcar(quoted) == sym::LAMBDA {
        let cdr = lisp.xcdr(quoted);
        let arglist = lisp.car(cdr)?;
        let mut cdr = lisp.cdr(cdr)?;
        let mut docstring = Value::NIL;
        let mut iform = Value::NIL;
        if cdr.is_cons() {
            docstring = lisp.xcar(cdr);
            if docstring.is_string() {
                let tmp = lisp.xcdr(cdr);
                if !tmp.is_nil() {
                    cdr = tmp;
                } else {
                    docstring = Value::NIL;
                }
            } else if docstring.is_cons() && lisp.xcar(docstring) == sym::K_DOCUMENTATION {
                let form = lisp.xcdr(docstring);
                let form = lisp.car(form)?;
                docstring = lisp.eval(form)?;
                cdr = lisp.xcdr(cdr);
            } else {
                docstring = Value::NIL;
            }
        }
        if cdr.is_cons() {
            iform = lisp.xcar(cdr);
            if iform.is_cons() && lisp.xcar(iform) == sym::INTERACTIVE {
                cdr = lisp.xcdr(cdr);
            } else {
                iform = Value::NIL;
            }
        }
        if cdr.is_nil() {
            cdr = lisp.cons(Value::NIL, Value::NIL);
        }
        let env = lisp.lexenv();
        let maker = lisp.plain_value(sym::INTERNAL_MAKE_INTERPRETED_CLOSURE_FUNCTION);
        lisp.stack.push(docstring);
        let r = if env.is_nil() || maker.is_nil() {
            f_make_interpreted_closure(lisp, &[arglist, cdr, env, docstring, iform])
        } else {
            lisp.funcall(maker, &[arglist, cdr, env, docstring, iform])
        };
        lisp.stack.pop();
        r
    } else {
        Ok(quoted)
    }
}

fn f_defvaralias(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (new_alias, base_variable, docstring) = (args[0], args[1], args[2]);
    let new_alias = lisp.check_symbol(new_alias)?;
    let base_variable = lisp.check_symbol(base_variable)?;
    if lisp.heap.symbol_ref(new_alias).trapped_write == TrappedWrite::NoWrite {
        let name = lisp.symbol_name_string(new_alias);
        return Err(lisp.error(&format!("Cannot make a constant an alias: {name}")));
    }
    let mut s = base_variable;
    loop {
        if s == new_alias {
            return Err(lisp.signal_list(sym::CYCLIC_VARIABLE_INDIRECTION, &[base_variable]));
        }
        match lisp.heap.symbol_ref(s).value {
            SymbolValue::Alias(next) => s = next,
            _ => break,
        }
    }
    let refusal = match lisp.heap.symbol_ref(new_alias).value {
        SymbolValue::Forwarded(_) => Some("Cannot make a built-in variable an alias"),
        SymbolValue::Localized(_) => {
            Some("Don't know how to make a buffer-local variable an alias")
        }
        SymbolValue::Plain(_) | SymbolValue::Alias(_) => None,
    };
    if let Some(refusal) = refusal {
        let name = lisp.symbol_name_string(new_alias);
        return Err(lisp.error(&format!("{refusal}: {name}")));
    }
    if lisp.find_symbol_value(base_variable) == sym::UNBOUND {
        let value = lisp.find_symbol_value(new_alias);
        lisp.set_internal(base_variable, value, Value::NIL, SetMode::Bind)?;
    } else {
        let value = lisp.find_symbol_value(new_alias);
        if value != sym::UNBOUND && value != lisp.find_symbol_value(base_variable) {
            let message = lisp.string("Overwriting value of `%s' by aliasing to `%s'");
            lisp.stack.push(message);
            let formatted =
                crate::editfns::f_format_message(lisp, &[message, new_alias, base_variable]);
            lisp.stack.pop();
            let formatted = formatted?;
            let kind = lisp.list(&[sym::DEFVARALIAS, sym::LOSING_VALUE, new_alias]);
            lisp.funcall(sym::DISPLAY_WARNING, &[kind, formatted])?;
        }
    }
    let let_bound = lisp.specpdl.iter().any(|b| {
        matches!(
            *b,
            SpecBinding::Let { symbol, .. }
                | SpecBinding::LetLocal { symbol, .. }
                | SpecBinding::LetDefault { symbol, .. }
                if symbol == new_alias
        )
    });
    if let_bound {
        let name = lisp.symbol_name_string(new_alias);
        return Err(lisp.error(&format!(
            "Don't know how to make a let-bound variable an alias: {name}"
        )));
    }
    if lisp.heap.symbol_ref(new_alias).trapped_write == TrappedWrite::Trapped {
        lisp.notify_variable_watchers(new_alias, base_variable, sym::DEFVARALIAS, Value::NIL)?;
    }
    let trapped_write = lisp.heap.symbol_ref(base_variable).trapped_write;
    {
        let s = lisp.heap.symbol_mut(new_alias);
        s.special = true;
        s.value = SymbolValue::Alias(base_variable);
        s.trapped_write = trapped_write;
    }
    lisp.heap.symbol_mut(base_variable).special = true;
    lisp.loadhist_attach(new_alias);
    lisp.put(new_alias, sym::VARIABLE_DOCUMENTATION, docstring);
    Ok(base_variable)
}

fn f_internal_delete_indirect_variable(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.check_symbol(args[0])?;
    if !matches!(lisp.heap.symbol_ref(symbol).value, SymbolValue::Alias(_)) {
        let message = lisp.string("Cannot undeclare a variable that is not an alias");
        return Err(lisp.signal_list(sym::ERROR, &[message, symbol]));
    }
    lisp.heap.symbol_mut(symbol).value = SymbolValue::Plain(sym::UNBOUND);
    lisp.put(symbol, sym::VARIABLE_DOCUMENTATION, Value::NIL);
    lisp.set(symbol, sym::UNBOUND)?;
    Ok(symbol)
}

fn f_default_toplevel_value(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = args[0];
    let binding = lisp.default_toplevel_binding(symbol);
    let value = match binding {
        Some(index) => match lisp.specpdl[index] {
            SpecBinding::Let { old, .. } | SpecBinding::LetDefault { old, .. } => old,
            _ => Value::NIL,
        },
        None => lisp.default_value_raw(symbol)?,
    };
    if value != sym::UNBOUND {
        return Ok(value);
    }
    Err(lisp.signal_list(sym::VOID_VARIABLE, &[symbol]))
}

fn f_set_default_toplevel_value(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, value) = (args[0], args[1]);
    match lisp.default_toplevel_binding(symbol) {
        Some(index) => {
            if let SpecBinding::Let { old, .. } | SpecBinding::LetDefault { old, .. } =
                &mut lisp.specpdl[index]
            {
                *old = value;
            }
        }
        None => {
            lisp.set_default(symbol, value)?;
        }
    }
    Ok(Value::NIL)
}

fn f_buffer_local_toplevel_value(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, mut buffer) = (args[0], args[1]);
    if buffer.is_nil() {
        buffer = lisp.current_buffer_value();
    }
    if crate::data::f_local_variable_p(lisp, &[symbol, buffer])?.is_nil() {
        return Err(lisp.signal_list(sym::VOID_VARIABLE, &[symbol]));
    }
    match lisp.local_toplevel_binding(symbol, buffer) {
        Some(old) => Ok(*old),
        None => crate::buffer::f_buffer_local_value(lisp, &[symbol, buffer]),
    }
}

fn f_set_buffer_local_toplevel_value(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, value, buffer) = (args[0], args[1], args[2]);
    let buf = if buffer.is_nil() {
        lisp.current_buffer_value()
    } else {
        buffer
    };
    if let Some(old) = lisp.local_toplevel_binding(symbol, buf) {
        *old = value;
    } else if buffer.is_nil() {
        let symbol = crate::data::f_make_local_variable(lisp, &[symbol])?;
        lisp.set(symbol, value)?;
    } else {
        let count = lisp.specpdl.len();
        lisp.record_unwind_current_buffer();
        let r = crate::buffer::f_set_buffer(lisp, &[buffer])
            .and_then(|_| crate::data::f_make_local_variable(lisp, &[symbol]))
            .and_then(|symbol| lisp.set(symbol, value).map(|_| Value::NIL));
        lisp.unbind_to_value(count, r)?;
    }
    Ok(Value::NIL)
}

impl Lisp {
    fn local_toplevel_binding(&mut self, symbol: Value, buffer: Value) -> Option<&mut Value> {
        self.specpdl.iter_mut().find_map(|b| match b {
            SpecBinding::LetLocal {
                symbol: s,
                old,
                buffer: w,
            } if *w == buffer && *s == symbol => Some(old),
            _ => None,
        })
    }

    fn default_toplevel_binding(&self, symbol: Value) -> Option<usize> {
        let mut binding = None;
        for (index, b) in self.specpdl.iter().enumerate().rev() {
            match *b {
                SpecBinding::Let { symbol: s, .. } | SpecBinding::LetDefault { symbol: s, .. }
                    if s == symbol =>
                {
                    binding = Some(index);
                }
                _ => {}
            }
        }
        binding
    }
}

fn f_internal_define_uninitialized_variable(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, doc) = (args[0], args[1]);
    if !lisp.heap.symbol_ref(symbol).special && lisp.lexical_bound_p(symbol) {
        let message = lisp.string("Defining as dynamic an already lexical var");
        return Err(lisp.signal_list(sym::ERROR, &[message, symbol]));
    }
    lisp.heap.symbol_mut(symbol).special = true;
    if !doc.is_nil() {
        lisp.put(symbol, sym::VARIABLE_DOCUMENTATION, doc);
    }
    lisp.loadhist_attach(symbol);
    Ok(Value::NIL)
}

impl Lisp {
    fn lexical_bound_p(&self, symbol: Value) -> bool {
        for binding in self.specpdl.iter().rev() {
            if let SpecBinding::Let { symbol: s, old } = *binding
                && s == sym::INTERNAL_INTERPRETER_ENVIRONMENT
            {
                let mut env = old;
                while env.is_cons() {
                    let pair = self.xcar(env);
                    if pair.is_cons() && self.xcar(pair) == symbol {
                        return true;
                    }
                    env = self.xcdr(env);
                }
            }
        }
        false
    }

    pub fn defvar_internal(
        &mut self,
        symbol: Value,
        init: Value,
        docstring: Value,
        eval: bool,
    ) -> LispResult {
        let symbol = self.check_symbol(symbol)?;
        let bound = self.default_boundp(symbol);
        f_internal_define_uninitialized_variable(self, &[symbol, docstring])?;
        if !bound {
            let value = self.defvar_initial_value(init, eval)?;
            self.set_default(symbol, value)?;
        } else if let Some(index) = self.default_toplevel_binding(symbol) {
            let unbound = matches!(
                self.specpdl[index],
                SpecBinding::Let { old, .. } | SpecBinding::LetDefault { old, .. } if old == sym::UNBOUND
            );
            if unbound {
                let value = self.defvar_initial_value(init, eval)?;
                if let SpecBinding::Let { old, .. } | SpecBinding::LetDefault { old, .. } =
                    &mut self.specpdl[index]
                {
                    *old = value;
                }
            }
        }
        Ok(symbol)
    }

    fn defvar_initial_value(&mut self, init: Value, eval: bool) -> LispResult {
        if !eval {
            return Ok(init);
        }
        self.stack.push(init);
        let v = self.eval(init);
        self.stack.pop();
        v
    }

    fn default_boundp(&self, symbol: Value) -> bool {
        self.default_value_unchecked(symbol) != sym::UNBOUND
    }
}

fn f_defvar(lisp: &mut Lisp, args: Value) -> LispResult {
    let symbol = lisp.car(args)?;
    let tail = lisp.cdr(args)?;
    let symbol = lisp.maybe_remove_pos_from_symbol(symbol);
    let symbol = lisp.check_symbol(symbol)?;
    if !tail.is_nil() {
        let rest = lisp.xcdr(tail);
        if !lisp.cdr(rest)?.is_nil() {
            return Err(lisp.error("Too many arguments"));
        }
        let exp = lisp.xcar(tail);
        let doc = lisp.car(rest)?;
        lisp.defvar_internal(symbol, exp, doc, true)
    } else if !lisp.lexenv().is_nil() && !lisp.heap.symbol_ref(symbol).special {
        let env = lisp.lexenv();
        let env = lisp.cons(symbol, env);
        lisp.set_lexenv(env);
        Ok(symbol)
    } else {
        Ok(symbol)
    }
}

fn f_defvar_1(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, value, docstring) = (args[0], args[1], args[2]);
    let symbol = lisp.check_symbol(symbol)?;
    lisp.defvar_internal(symbol, value, docstring, false)
}

fn f_defconst(lisp: &mut Lisp, args: Value) -> LispResult {
    let symbol = lisp.car(args)?;
    let symbol = lisp.maybe_remove_pos_from_symbol(symbol);
    let symbol = lisp.check_symbol(symbol)?;
    let tail = lisp.cdr(args)?;
    let mut docstring = Value::NIL;
    let rest = lisp.cdr(tail)?;
    if !rest.is_nil() {
        if !lisp.cdr(rest)?.is_nil() {
            return Err(lisp.error("Too many arguments"));
        }
        docstring = lisp.xcar(rest);
    }
    let form = lisp.car(tail)?;
    lisp.stack.push(args);
    let value = lisp.eval(form);
    lisp.stack.pop();
    let value = value?;
    f_defconst_1(lisp, &[symbol, value, docstring])
}

fn f_defconst_1(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, initvalue, docstring) = (args[0], args[1], args[2]);
    let symbol = lisp.check_symbol(symbol)?;
    let value = initvalue;
    f_internal_define_uninitialized_variable(lisp, &[symbol, docstring])?;
    lisp.set_default(symbol, value)?;
    lisp.put(symbol, sym::RISKY_LOCAL_VARIABLE, sym::T);
    Ok(symbol)
}

fn f_internal_make_var_non_special(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.check_symbol(args[0])?;
    lisp.heap.symbol_mut(symbol).special = false;
    Ok(Value::NIL)
}

fn f_let_star(lisp: &mut Lisp, args: Value) -> LispResult {
    let count = lisp.specpdl.len();
    let base = lisp.stack.len();
    let lexenv = lisp.lexenv();
    let varlist = lisp.xcar(args);
    let mut li = lisp.for_each_tail(varlist);
    while li.tail.is_cons() {
        let elt = lisp.xcar(li.tail);
        let (var, val) = if lisp.symbolp(elt) {
            (elt, Value::NIL)
        } else {
            let var = lisp.car(elt)?;
            let rest = lisp.xcdr(elt);
            if !lisp.cdr(rest)?.is_nil() {
                return Err(lisp.signal_error("`let' bindings can have only one value-form", elt));
            }
            let form = lisp.car(rest)?;
            (var, lisp.eval(form)?)
        };
        let var = lisp.maybe_remove_pos_from_symbol(var);
        if !var.is_symbol() {
            return Err(lisp.wrong_type(sym::SYMBOLP, var));
        }
        let current = lisp.lexenv();
        if !lexenv.is_nil()
            && !lisp.heap.symbol_ref(var).special
            && lisp.memq(var, current).is_nil()
        {
            let pair = lisp.cons(var, val);
            let newenv = lisp.cons(pair, current);
            if current == lexenv {
                lisp.specbind(sym::INTERNAL_INTERPRETER_ENVIRONMENT, newenv)?;
            } else {
                lisp.set_lexenv(newenv);
            }
        } else {
            lisp.specbind(var, val)?;
        }
        lisp.tail_step(&mut li)?;
    }
    let all = lisp.xcar(args);
    lisp.check_list_end(li.tail, all)?;
    lisp.stack.truncate(base);
    let body = lisp.xcdr(args);
    let value = lisp.progn(body);
    lisp.unbind_to_value(count, value)
}

fn f_let(lisp: &mut Lisp, args: Value) -> LispResult {
    let count = lisp.specpdl.len();
    let base = lisp.stack.len();
    let mut varlist = lisp.xcar(args);
    let mut nvars = lisp.list_length(varlist)?;
    lisp.stack.push(varlist);
    let temps = lisp.stack.len();
    let mut argnum = 0;
    while argnum < nvars && varlist.is_cons() {
        lisp.maybe_quit()?;
        let elt = lisp.xcar(varlist);
        varlist = lisp.xcdr(varlist);
        lisp.stack[base] = varlist;
        let value = if lisp.symbolp(elt) {
            Value::NIL
        } else {
            let rest = lisp.cdr(elt)?;
            if !lisp.cdr(rest)?.is_nil() {
                return Err(lisp.signal_error("`let' bindings can have only one value-form", elt));
            }
            let form = lisp.car(rest)?;
            lisp.eval(form)?
        };
        lisp.stack.push(value);
        argnum += 1;
    }
    nvars = argnum;
    let mut lexenv = lisp.lexenv();
    varlist = lisp.xcar(args);
    argnum = 0;
    while argnum < nvars && varlist.is_cons() {
        let elt = lisp.xcar(varlist);
        varlist = lisp.xcdr(varlist);
        let var = if lisp.symbolp(elt) {
            elt
        } else {
            lisp.car(elt)?
        };
        let var = lisp.maybe_remove_pos_from_symbol(var);
        if !var.is_symbol() {
            return Err(lisp.wrong_type(sym::SYMBOLP, var));
        }
        let value = lisp.stack[temps + argnum];
        let current = lisp.lexenv();
        if !lexenv.is_nil()
            && !lisp.heap.symbol_ref(var).special
            && lisp.memq(var, current).is_nil()
        {
            let pair = lisp.cons(var, value);
            lexenv = lisp.cons(pair, lexenv);
            lisp.stack.push(lexenv);
        } else {
            lisp.specbind(var, value)?;
        }
        argnum += 1;
    }
    if lexenv != lisp.lexenv() {
        lisp.specbind(sym::INTERNAL_INTERPRETER_ENVIRONMENT, lexenv)?;
    }
    let body = lisp.xcdr(args);
    let value = lisp.progn(body);
    lisp.stack.truncate(base);
    lisp.unbind_to_value(count, value)
}

fn f_while(lisp: &mut Lisp, args: Value) -> LispResult {
    let test = lisp.xcar(args);
    let body = lisp.xcdr(args);
    loop {
        let t = lisp.eval(test)?;
        if t.is_nil() {
            break;
        }
        lisp.progn(body)?;
    }
    Ok(Value::NIL)
}

fn f_macroexpand(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (mut form, environment) = (args[0], args[1]);
    let base = lisp.stack.len();
    lisp.stack.push(form);
    let result = (|| loop {
        if !form.is_cons() {
            return Ok(form);
        }
        let mut def = lisp.xcar(form);
        let mut symbol = def;
        let mut tem = Value::NIL;
        loop {
            let bare = lisp.maybe_remove_pos_from_symbol(def);
            if !bare.is_symbol() {
                break;
            }
            symbol = def;
            tem = crate::fns::f_assq(lisp, &[symbol, environment])?;
            if tem.is_nil() {
                def = lisp.heap.symbol_ref(bare).function;
                if !def.is_nil() {
                    continue;
                }
            }
            break;
        }
        let expander = if tem.is_nil() {
            let def = lisp.autoload_do_load(def, symbol, sym::MACRO)?;
            if !def.is_cons() || lisp.xcar(def) != sym::MACRO {
                return Ok(form);
            }
            lisp.xcdr(def)
        } else {
            let e = lisp.xcdr(tem);
            if e.is_nil() {
                return Ok(form);
            }
            e
        };
        let rest = lisp.xcdr(form);
        let newform = lisp.apply1(expander, rest)?;
        if lisp.eq(form, newform) {
            return Ok(form);
        }
        form = newform;
        lisp.stack.push(form);
    })();
    lisp.stack.truncate(base);
    result
}

fn f_catch(lisp: &mut Lisp, args: Value) -> LispResult {
    let tag_form = lisp.xcar(args);
    let tag = lisp.eval(tag_form)?;
    let body = lisp.xcdr(args);
    lisp.internal_catch(tag, |l| l.progn(body))
}

fn f_throw(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Err(lisp.throw(args[0], args[1]))
}

fn f_unwind_protect(lisp: &mut Lisp, args: Value) -> LispResult {
    let count = lisp.specpdl.len();
    let unwind_forms = lisp.xcdr(args);
    lisp.specpdl.push(SpecBinding::UnwindForms(unwind_forms));
    let body = lisp.xcar(args);
    let value = lisp.eval(body);
    lisp.unbind_to_value(count, value)
}

fn f_condition_case(lisp: &mut Lisp, args: Value) -> LispResult {
    let var = lisp.xcar(args);
    let rest = lisp.xcdr(args);
    let bodyform = lisp.xcar(rest);
    let handlers = lisp.xcdr(rest);
    lisp.internal_lisp_condition_case(var, bodyform, handlers)
}

impl Lisp {
    pub fn internal_lisp_condition_case(
        &mut self,
        var: Value,
        bodyform: Value,
        handlers: Value,
    ) -> LispResult {
        let var = self.maybe_remove_pos_from_symbol(var);
        if !var.is_symbol() {
            return Err(self.wrong_type(sym::SYMBOLP, var));
        }
        let mut success_handler = Value::NIL;
        let mut clauses = Vec::new();
        let mut tail = handlers;
        while tail.is_cons() {
            let tem = self.xcar(tail);
            let valid = tem.is_nil()
                || (tem.is_cons() && {
                    let head = self.xcar(tem);
                    head.is_symbol() || head.is_cons()
                });
            if !valid {
                let printed = self.prin1_to_string(tem);
                return Err(self.error(&format!("Invalid condition handler: {printed}")));
            }
            if tem.is_cons() && self.xcar(tem) == sym::K_SUCCESS {
                success_handler = tem;
            } else {
                clauses.push(tem);
            }
            tail = self.xcdr(tail);
        }
        let first_handler = self.handlers.len();
        for &clause in clauses.iter().rev() {
            let mut condition = if clause.is_cons() {
                self.xcar(clause)
            } else {
                Value::NIL
            };
            if !condition.is_cons() {
                condition = self.list(&[condition]);
            }
            self.push_handler(HandlerKind::ConditionCase(condition));
        }
        let result = self.eval(bodyform);
        let (val, handler_body, level) = match result {
            Ok(val) => {
                self.pop_handler(first_handler);
                if success_handler.is_nil() {
                    return Ok(val);
                }
                (val, self.xcdr(success_handler), None)
            }
            Err(flow @ Flow::Signal { error, target })
                if target >= first_handler && target < first_handler + clauses.len() =>
            {
                let level = self.handlers[first_handler].flow_roots;
                self.unwind_to_handler(target, flow)?;
                self.handlers.truncate(first_handler);
                let chosen = clauses[clauses.len() - 1 - (target - first_handler)];
                (error, self.xcdr(chosen), Some(level))
            }
            Err(flow) => return Err(flow),
        };
        let r = self.run_condition_case_handler(var, val, handler_body);
        match level {
            Some(level) => self.finish_catch(level, r),
            None => r,
        }
    }

    fn run_condition_case_handler(
        &mut self,
        var: Value,
        val: Value,
        handler_body: Value,
    ) -> LispResult {
        if var.is_nil() {
            return self.progn(handler_body);
        }
        let count = self.specpdl.len();
        let lexenv = self.lexenv();
        if !lexenv.is_nil() {
            let pair = self.cons(var, val);
            let env = self.cons(pair, lexenv);
            self.specbind(sym::INTERNAL_INTERPRETER_ENVIRONMENT, env)?;
        } else {
            self.specbind(var, val)?;
        }
        let r = self.progn(handler_body);
        self.unbind_to_value(count, r)
    }
}

fn f_handler_bind_1(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let bodyfun = args[0];
    if args.len().is_multiple_of(2) {
        return Err(lisp.error("Trailing CONDITIONS without HANDLER in `handler-bind`"));
    }
    let first = lisp.handlers.len();
    let mut count = 0;
    let mut i = args.len() as isize - 2;
    while i > 0 {
        let index = i as usize;
        let (conditions, handler) = (args[index], args[index + 1]);
        i -= 2;
        if conditions.is_nil() {
            continue;
        }
        let conditions = if conditions.is_cons() {
            conditions
        } else {
            lisp.list(&[conditions])
        };
        lisp.push_handler(HandlerKind::HandlerBind {
            conditions,
            handler,
            skip: count,
        });
        count += 1;
    }
    let ret = lisp.call0(bodyfun);
    if ret.is_ok() {
        lisp.pop_handler(first);
    }
    ret
}

fn f_signal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (mut error_symbol, data) = (args[0], args[1]);
    if error_symbol.is_nil() && !data.is_cons() {
        error_symbol = sym::ERROR;
    }
    Err(lisp.signal(error_symbol, data))
}

pub(crate) fn f_commandp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, for_call_interactively) = (args[0], args[1]);
    let mut fun = function;
    fun = lisp.indirect_function(fun);
    if fun.is_nil() {
        return Ok(Value::NIL);
    }
    let mut genfun = false;
    if let Some(subr) = lisp.subr(fun) {
        if subr.interactive.is_some() {
            return Ok(sym::T);
        }
    } else if fun.is_vectorlike() && lisp.is_closure(fun) {
        if lisp.vector_items(fun).len() > 5 {
            return Ok(sym::T);
        }
        genfun = true;
    } else if fun.is_string() || lisp.is_vector(fun) {
        return Ok(if for_call_interactively.is_nil() {
            sym::T
        } else {
            Value::NIL
        });
    } else if fun.is_cons() {
        let funcar = lisp.xcar(fun);
        if funcar == sym::AUTOLOAD {
            let interactive = lisp.nth(3, fun);
            return Ok(if interactive.is_nil() {
                Value::NIL
            } else {
                sym::T
            });
        }
        if funcar == sym::LAMBDA {
            let body = lisp.xcdr(fun);
            let body = lisp.cdr_safe(body);
            let mut b = body;
            if b.is_cons() && lisp.xcar(b).is_string() {
                b = lisp.xcdr(b);
            }
            let first = lisp.car_safe(b);
            return Ok(if first.is_cons() && lisp.xcar(first) == sym::INTERACTIVE {
                sym::T
            } else {
                Value::NIL
            });
        }
        return Ok(Value::NIL);
    } else if !fun.is_vectorlike() || !lisp.is_record(fun) {
        return Ok(Value::NIL);
    } else {
        genfun = true;
    }
    if genfun {
        let iform = lisp.intern("interactive-form");
        if !lisp.heap.symbol_ref(iform).function.is_nil() {
            let r = lisp.funcall(iform, &[fun])?;
            return Ok(if r.is_nil() { Value::NIL } else { sym::T });
        }
    }
    Ok(Value::NIL)
}

fn f_autoload(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, file, docstring, interactive, kind) =
        (args[0], args[1], args[2], args[3], args[4]);
    let function = lisp.check_symbol(function)?;
    lisp.check_string(file)?;
    let current = lisp.heap.symbol_ref(function).function;
    if !current.is_nil() && !(current.is_cons() && lisp.xcar(current) == sym::AUTOLOAD) {
        return Ok(Value::NIL);
    }
    let definition = lisp.list(&[sym::AUTOLOAD, file, docstring, interactive, kind]);
    lisp.defalias(function, definition, Value::NIL)
}

fn f_autoload_do_load(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.autoload_do_load(args[0], args[1], args[2])
}

fn f_eval(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.eval_with_lexical(args[0], args[1])
}

fn f_apply(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = args.len();
    let spread = args[n - 1];
    if n == 1 {
        if spread.is_nil() {
            return Err(lisp.wrong_type(sym::CONSP, spread));
        }
        let function = lisp.car(spread)?;
        let rest = lisp.cdr(spread)?;
        return lisp.apply1(function, rest);
    }
    let function = args[0];
    let mut argv: Vec<Value> = args[1..n - 1].to_vec();
    let tail = lisp.list_to_vec(spread)?;
    argv.extend(tail);
    lisp.funcall(function, &argv)
}

fn funcall_nil(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.funcall(args[0], &args[1..])?;
    Ok(Value::NIL)
}

fn f_run_hooks(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    for &hook in args {
        lisp.run_hook_with_args(&[hook], funcall_nil)?;
    }
    Ok(Value::NIL)
}

fn f_run_hook_with_args(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.run_hook_with_args(args, funcall_nil)
}

fn f_run_hook_with_args_until_success(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.run_hook_with_args(args, |l, a| l.funcall(a[0], &a[1..]))
}

fn f_run_hook_with_args_until_failure(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let r = lisp.run_hook_with_args(args, |l, a| {
        let v = l.funcall(a[0], &a[1..])?;
        Ok(if v.is_nil() { sym::T } else { Value::NIL })
    })?;
    Ok(if r.is_nil() { sym::T } else { Value::NIL })
}

fn f_run_hook_wrapped(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut argv = args.to_vec();
    argv.swap(0, 1);
    let wrap = argv[0];
    argv[0] = args[0];
    let hook_args: Vec<Value> = std::iter::once(args[0])
        .chain(args[2..].iter().copied())
        .collect();
    lisp.run_hook_with_args(&hook_args, |l, a| {
        let mut call = Vec::with_capacity(a.len() + 1);
        call.push(a[0]);
        call.extend_from_slice(&a[1..]);
        l.funcall(wrap, &call)
    })
}

fn f_functionp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(if lisp.functionp(args[0]) {
        sym::T
    } else {
        Value::NIL
    })
}

fn f_funcall(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.funcall(args[0], &args[1..])
}

fn f_func_arity(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let function = args[0];
    let original = function;
    let mut function = function;
    if function.is_symbol() && !function.is_nil() {
        function = lisp.heap.symbol_ref(function).function;
        if function.is_symbol() {
            function = lisp.indirect_function(function);
        }
    }
    if function.is_cons() && lisp.xcar(function) == sym::MACRO {
        function = lisp.xcdr(function);
    }
    if let Some(subr) = lisp.subr(function) {
        let min = Value::fixnum(subr.min_args as i64);
        let max = match subr.kind {
            SubrKind::Fixed(max, _) => Value::fixnum(max as i64),
            SubrKind::Many(_) => sym::MANY,
            SubrKind::Special(_) => sym::UNEVALLED,
        };
        return Ok(lisp.cons(min, max));
    }
    if function.is_nil() {
        return Err(lisp.signal_list(sym::VOID_FUNCTION, &[original]));
    }
    if function.is_cons() {
        let head = lisp.xcar(function);
        if head == sym::LAMBDA {
            let arglist = lisp.car_safe(lisp.xcdr(function));
            return lisp.lambda_arity(function, arglist);
        }
        if head == sym::AUTOLOAD {
            let loaded = lisp.autoload_do_load(function, original, Value::NIL)?;
            return f_func_arity(lisp, &[loaded]);
        }
        return Err(lisp.signal_list(sym::INVALID_FUNCTION, &[original]));
    }
    if lisp.is_closure(function) {
        let arglist = lisp.vector_items(function)[0];
        if let Some(desc) = arglist.as_fixnum() {
            let mandatory = desc & 127;
            let nonrest = desc >> 8;
            let max = if desc & 128 != 0 {
                sym::MANY
            } else {
                Value::fixnum(nonrest)
            };
            return Ok(lisp.cons(Value::fixnum(mandatory), max));
        }
        return lisp.lambda_arity(function, arglist);
    }
    let iform = lisp.intern("func-arity");
    let _ = iform;
    Err(lisp.signal_list(sym::INVALID_FUNCTION, &[original]))
}

impl Lisp {
    fn lambda_arity(&mut self, fun: Value, arglist: Value) -> LispResult {
        let mut minargs = 0;
        let mut maxargs = 0;
        let mut optional = false;
        let mut tail = arglist;
        while tail.is_cons() {
            let next = self.xcar(tail);
            if next == sym::AND_REST {
                return Ok(self.cons(Value::fixnum(minargs), sym::MANY));
            } else if next == sym::AND_OPTIONAL {
                optional = true;
            } else {
                if !optional {
                    minargs += 1;
                }
                maxargs += 1;
            }
            tail = self.xcdr(tail);
        }
        if !tail.is_nil() {
            return Err(self.signal_list(sym::INVALID_FUNCTION, &[fun]));
        }
        Ok(self.cons(Value::fixnum(minargs), Value::fixnum(maxargs)))
    }
}

fn f_special_variable_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.check_symbol(args[0])?;
    Ok(if lisp.heap.symbol_ref(symbol).special {
        sym::T
    } else {
        Value::NIL
    })
}

fn nth_frame(lisp: &mut Lisp, nframes: Value, base: Value) -> LispResult<Option<usize>> {
    let n = lisp.check_fixnat(nframes)? as usize;
    let Some(start) = lisp.get_backtrace_starting_at(base) else {
        return Ok(None);
    };
    let frames: Vec<usize> = lisp
        .backtrace_frames()
        .into_iter()
        .filter(|&i| i <= start)
        .collect();
    let skip = if base.is_nil() { 0 } else { 1 };
    Ok(frames.get(n + skip).copied())
}

fn f_backtrace_debug(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (level, flag, base) = (args[0], args[1], args[2]);
    if let Some(index) = nth_frame(lisp, level, base)?
        && let SpecBinding::Backtrace { debug_on_exit, .. } = &mut lisp.specpdl[index]
    {
        *debug_on_exit = !flag.is_nil();
    }
    Ok(flag)
}

fn f_mapbacktrace(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, base) = (args[0], args[1]);
    let Some(start) = lisp.get_backtrace_starting_at(base) else {
        return Ok(Value::NIL);
    };
    let frames: Vec<usize> = lisp
        .backtrace_frames()
        .into_iter()
        .filter(|&i| i <= start)
        .collect();
    let skip = if base.is_nil() { 0 } else { 1 };
    for &index in frames.iter().skip(skip) {
        if index >= lisp.specpdl.len() {
            break;
        }
        let (fun, arglist, evaluated) = lisp.backtrace_frame_args(index);
        let flags = match lisp.specpdl[index] {
            SpecBinding::Backtrace {
                debug_on_exit: true,
                ..
            } => lisp.list(&[sym::K_DEBUG_ON_EXIT, sym::T]),
            _ => Value::NIL,
        };
        let evald = if evaluated { sym::T } else { Value::NIL };
        lisp.funcall(function, &[evald, fun, arglist, flags])?;
    }
    Ok(Value::NIL)
}

fn f_backtrace_frame_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, nframes, base) = (args[0], args[1], args[2]);
    let Some(index) = nth_frame(lisp, nframes, base)? else {
        return Ok(Value::NIL);
    };
    let (fun, arglist, evaluated) = lisp.backtrace_frame_args(index);
    let flags = match lisp.specpdl[index] {
        SpecBinding::Backtrace {
            debug_on_exit: true,
            ..
        } => lisp.list(&[sym::K_DEBUG_ON_EXIT, sym::T]),
        _ => Value::NIL,
    };
    let evald = if evaluated { sym::T } else { Value::NIL };
    lisp.funcall(function, &[evald, fun, arglist, flags])
}

fn f_backtrace_eval(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let exp = args[0];
    lisp.eval(exp)
}

fn f_backtrace_locals(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_funcall_with_delayed_message(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (timeout, message, function) = (args[0], args[1], args[2]);
    if !lisp.is_number(timeout) {
        return Err(lisp.wrong_type(sym::NUMBERP, timeout));
    }
    lisp.check_string(message)?;
    lisp.call0(function)
}
