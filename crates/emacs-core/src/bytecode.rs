use std::collections::HashMap;
use std::io::Write;
use std::sync::OnceLock;

use crate::error::{Flow, HandlerKind, LispResult};
use crate::lisp::{ALL_SUBRS, BacktraceArgs, Lisp, SpecBinding};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::{MOST_NEGATIVE_FIXNUM, MOST_POSITIVE_FIXNUM, Value};

pub const SUBRS: &[Subr] = &[
    Subr::fixed("byte-code", 3, 3, f_byte_code),
    Subr::fixed("internal-stack-stats", 0, 0, f_internal_stack_stats),
];

const B_STACK_REF: u8 = 0;
const B_STACK_REF1: u8 = 1;
const B_STACK_REF5: u8 = 5;
const B_STACK_REF6: u8 = 6;
const B_STACK_REF7: u8 = 7;
const B_VARREF: u8 = 0o10;
const B_VARREF6: u8 = 0o16;
const B_VARREF7: u8 = 0o17;
const B_VARSET: u8 = 0o20;
const B_VARSET6: u8 = 0o26;
const B_VARSET7: u8 = 0o27;
const B_VARBIND: u8 = 0o30;
const B_VARBIND6: u8 = 0o36;
const B_VARBIND7: u8 = 0o37;
const B_CALL: u8 = 0o40;
const B_CALL6: u8 = 0o46;
const B_CALL7: u8 = 0o47;
const B_UNBIND: u8 = 0o50;
const B_UNBIND6: u8 = 0o56;
const B_UNBIND7: u8 = 0o57;
const B_POPHANDLER: u8 = 0o60;
const B_PUSHCONDITIONCASE: u8 = 0o61;
const B_PUSHCATCH: u8 = 0o62;
const B_NTH: u8 = 0o70;
const B_SYMBOLP: u8 = 0o71;
const B_CONSP: u8 = 0o72;
const B_STRINGP: u8 = 0o73;
const B_LISTP: u8 = 0o74;
const B_EQ: u8 = 0o75;
const B_MEMQ: u8 = 0o76;
const B_NOT: u8 = 0o77;
const B_CAR: u8 = 0o100;
const B_CDR: u8 = 0o101;
const B_CONS: u8 = 0o102;
const B_LIST1: u8 = 0o103;
const B_LIST4: u8 = 0o106;
const B_LENGTH: u8 = 0o107;
const B_AREF: u8 = 0o110;
const B_ASET: u8 = 0o111;
const B_SYMBOL_VALUE: u8 = 0o112;
const B_SYMBOL_FUNCTION: u8 = 0o113;
const B_SET: u8 = 0o114;
const B_FSET: u8 = 0o115;
const B_GET: u8 = 0o116;
const B_SUBSTRING: u8 = 0o117;
const B_CONCAT2: u8 = 0o120;
const B_CONCAT4: u8 = 0o122;
const B_SUB1: u8 = 0o123;
const B_ADD1: u8 = 0o124;
const B_EQLSIGN: u8 = 0o125;
const B_GTR: u8 = 0o126;
const B_LSS: u8 = 0o127;
const B_LEQ: u8 = 0o130;
const B_GEQ: u8 = 0o131;
const B_DIFF: u8 = 0o132;
const B_NEGATE: u8 = 0o133;
const B_PLUS: u8 = 0o134;
const B_MAX: u8 = 0o135;
const B_MIN: u8 = 0o136;
const B_MULT: u8 = 0o137;
const B_POINT: u8 = 0o140;
const B_SAVE_CURRENT_BUFFER_OBSOLETE: u8 = 0o141;
const B_GOTO_CHAR: u8 = 0o142;
const B_INSERT: u8 = 0o143;
const B_POINT_MAX: u8 = 0o144;
const B_POINT_MIN: u8 = 0o145;
const B_CHAR_AFTER: u8 = 0o146;
const B_FOLLOWING_CHAR: u8 = 0o147;
const B_PRECEDING_CHAR: u8 = 0o150;
const B_CURRENT_COLUMN: u8 = 0o151;
const B_INDENT_TO: u8 = 0o152;
const B_EOLP: u8 = 0o154;
const B_EOBP: u8 = 0o155;
const B_BOLP: u8 = 0o156;
const B_BOBP: u8 = 0o157;
const B_CURRENT_BUFFER: u8 = 0o160;
const B_SET_BUFFER: u8 = 0o161;
const B_SAVE_CURRENT_BUFFER: u8 = 0o162;
const B_INTERACTIVE_P: u8 = 0o164;
const B_FORWARD_CHAR: u8 = 0o165;
const B_FORWARD_WORD: u8 = 0o166;
const B_SKIP_CHARS_FORWARD: u8 = 0o167;
const B_SKIP_CHARS_BACKWARD: u8 = 0o170;
const B_FORWARD_LINE: u8 = 0o171;
const B_CHAR_SYNTAX: u8 = 0o172;
const B_BUFFER_SUBSTRING: u8 = 0o173;
const B_DELETE_REGION: u8 = 0o174;
const B_NARROW_TO_REGION: u8 = 0o175;
const B_WIDEN: u8 = 0o176;
const B_END_OF_LINE: u8 = 0o177;
const B_CONSTANT2: u8 = 0o201;
const B_GOTO: u8 = 0o202;
const B_GOTOIFNIL: u8 = 0o203;
const B_GOTOIFNONNIL: u8 = 0o204;
const B_GOTOIFNILELSEPOP: u8 = 0o205;
const B_GOTOIFNONNILELSEPOP: u8 = 0o206;
const B_RETURN: u8 = 0o207;
const B_DISCARD: u8 = 0o210;
const B_DUP: u8 = 0o211;
const B_SAVE_EXCURSION: u8 = 0o212;
const B_SAVE_WINDOW_EXCURSION: u8 = 0o213;
const B_SAVE_RESTRICTION: u8 = 0o214;
const B_CATCH: u8 = 0o215;
const B_UNWIND_PROTECT: u8 = 0o216;
const B_CONDITION_CASE: u8 = 0o217;
const B_TEMP_OUTPUT_BUFFER_SETUP: u8 = 0o220;
const B_TEMP_OUTPUT_BUFFER_SHOW: u8 = 0o221;
const B_SET_MARKER: u8 = 0o223;
const B_MATCH_BEGINNING: u8 = 0o224;
const B_MATCH_END: u8 = 0o225;
const B_UPCASE: u8 = 0o226;
const B_DOWNCASE: u8 = 0o227;
const B_STRINGEQLSIGN: u8 = 0o230;
const B_STRINGLSS: u8 = 0o231;
const B_EQUAL: u8 = 0o232;
const B_NTHCDR: u8 = 0o233;
const B_ELT: u8 = 0o234;
const B_MEMBER: u8 = 0o235;
const B_ASSQ: u8 = 0o236;
const B_NREVERSE: u8 = 0o237;
const B_SETCAR: u8 = 0o240;
const B_SETCDR: u8 = 0o241;
const B_CAR_SAFE: u8 = 0o242;
const B_CDR_SAFE: u8 = 0o243;
const B_NCONC: u8 = 0o244;
const B_QUO: u8 = 0o245;
const B_REM: u8 = 0o246;
const B_NUMBERP: u8 = 0o247;
const B_INTEGERP: u8 = 0o250;
const B_LIST_N: u8 = 0o257;
const B_CONCAT_N: u8 = 0o260;
const B_INSERT_N: u8 = 0o261;
const B_STACK_SET: u8 = 0o262;
const B_STACK_SET2: u8 = 0o263;
const B_DISCARD_N: u8 = 0o266;
const B_SWITCH: u8 = 0o267;
const B_CONSTANT: u8 = 0o300;

const BC_STACK_SIZE: usize = 512 * 1024;
const BC_FRAME_SIZE: usize = 4;
const SMALL_LIST_LEN_MAX: i64 = 127;

#[derive(Default)]
pub struct Bytecode {
    pub used: usize,
    pub frames: usize,
}

struct Frame {
    vector: Value,
    code: Vec<u8>,
    pc: usize,
    quitcounter: u8,
    handlers: Vec<(usize, usize)>,
}

impl Frame {
    fn fetch(&mut self) -> usize {
        let byte = self.code.get(self.pc).copied().unwrap_or(0);
        self.pc += 1;
        usize::from(byte)
    }

    fn fetch2(&mut self) -> usize {
        let lo = self.fetch();
        let hi = self.fetch();
        lo | hi << 8
    }
}

fn primitive(name: &str) -> Option<&'static Subr> {
    static TABLE: OnceLock<HashMap<&'static str, &'static Subr>> = OnceLock::new();
    TABLE
        .get_or_init(|| {
            ALL_SUBRS
                .iter()
                .flat_map(|subrs| subrs.iter())
                .map(|subr| (subr.name, subr))
                .collect()
        })
        .get(name)
        .copied()
}

fn fixnum_overflow_p(n: i64) -> bool {
    !(MOST_NEGATIVE_FIXNUM..=MOST_POSITIVE_FIXNUM).contains(&n)
}

fn bcall0(lisp: &mut Lisp, f: Value) -> LispResult<()> {
    lisp.funcall(f, &[])?;
    Ok(())
}

fn restore_window_configuration(lisp: &mut Lisp, configuration: Value) -> LispResult<()> {
    lisp.call_primitive("set-window-configuration", &[configuration])?;
    Ok(())
}

fn f_byte_code(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (mut bytestr, vector, maxdepth) = (args[0], args[1], args[2]);
    if !(bytestr.is_string()
        && lisp.is_vector(vector)
        && maxdepth.as_fixnum().is_some_and(|d| d >= 0))
    {
        return Err(lisp.error("Invalid byte-code"));
    }
    if lisp.string_multibyte(bytestr) {
        bytestr = lisp.call_primitive("string-as-unibyte", &[bytestr])?;
    }
    let fun = lisp.call_primitive("make-byte-code", &[Value::NIL, bytestr, vector, maxdepth])?;
    exec_byte_code(lisp, fun, 0, &[])
}

fn f_internal_stack_stats(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let nframes = lisp.bc.frames + 1;
    let _ =
        std::io::stderr().write_all(format!("{nframes} stack frames, {nframes} runs\n").as_bytes());
    Ok(Value::NIL)
}

pub fn exec_byte_code(
    lisp: &mut Lisp,
    fun: Value,
    args_template: i64,
    args: &[Value],
) -> LispResult {
    let base = lisp.stack.len();
    lisp.stack.push(fun);
    let slots = lisp.vector_items(fun);
    let slot = |i: usize| slots.get(i).copied().unwrap_or(Value::NIL);
    let (bytestr, vector, maxdepth) = (slot(1), slot(2), slot(3));
    lisp.stack.push(vector);
    let needed = maxdepth.as_fixnum().unwrap_or(0).max(0) as usize + BC_FRAME_SIZE;
    if lisp.bc.used + needed > BC_STACK_SIZE {
        lisp.stack.truncate(base);
        return Err(lisp.error("Bytecode stack overflow"));
    }
    lisp.bc.used += needed;
    lisp.bc.frames += 1;
    let mut frame = Frame {
        vector,
        code: lisp.string_bytes(bytestr).to_vec(),
        pc: 0,
        quitcounter: 1,
        handlers: Vec::new(),
    };
    let result = lisp
        .bc_push_args(args_template, args)
        .and_then(|()| lisp.bc_run(&mut frame));
    lisp.bc.used -= needed;
    lisp.bc.frames -= 1;
    lisp.stack.truncate(base);
    result
}

impl Lisp {
    pub fn call_primitive(&mut self, name: &'static str, args: &[Value]) -> LispResult {
        match primitive(name) {
            Some(subr) => self.funcall_subr(Value::NIL, subr, args),
            None => {
                let symbol = self.intern(name);
                Err(self.signal_list(sym::VOID_FUNCTION, &[symbol]))
            }
        }
    }

    fn bc_push_args(&mut self, args_template: i64, args: &[Value]) -> LispResult<()> {
        let rest = args_template & 128 != 0;
        let mandatory = args_template & 127;
        let nonrest = args_template >> 8;
        let nargs = args.len() as i64;
        if !(mandatory <= nargs && (rest || nargs <= nonrest)) {
            let arity = self.cons(Value::fixnum(mandatory), Value::fixnum(nonrest));
            return Err(self.signal_list(
                sym::WRONG_NUMBER_OF_ARGUMENTS,
                &[arity, Value::fixnum(nargs)],
            ));
        }
        let pushedargs = nonrest.min(nargs) as usize;
        self.stack.extend_from_slice(&args[..pushedargs]);
        if nonrest < nargs {
            let rest_args = self.list(&args[nonrest as usize..]);
            self.stack.push(rest_args);
        } else {
            for _ in (nargs - i64::from(rest))..nonrest {
                self.stack.push(Value::NIL);
            }
        }
        Ok(())
    }

    fn bc_run(&mut self, frame: &mut Frame) -> LispResult {
        loop {
            match self.bc_step(frame) {
                Ok(None) => {}
                Ok(Some(value)) => return Ok(value),
                Err(flow) => {
                    let target = flow.target();
                    let Some(pos) = frame
                        .handlers
                        .iter()
                        .rposition(|&(index, _)| index == target)
                    else {
                        return Err(flow);
                    };
                    let (index, dest) = frame.handlers[pos];
                    let level = self.handlers[index].flow_roots;
                    let value = match flow {
                        Flow::Signal { error, .. } => error,
                        Flow::Throw { value, .. } => value,
                    };
                    self.unwind_to_handler(index, flow)?;
                    self.handlers.truncate(index);
                    frame.handlers.truncate(pos);
                    self.stack.push(value);
                    self.flow_roots.truncate(level);
                    frame.quitcounter = u8::MAX;
                    frame.pc = 0;
                    self.bc_branch(frame, dest)?;
                }
            }
        }
    }

    fn bc_branch(&mut self, frame: &mut Frame, dest: usize) -> LispResult<()> {
        frame.quitcounter = frame.quitcounter.wrapping_add(u8::from(dest < frame.pc));
        if frame.quitcounter == 0 {
            frame.quitcounter = 1;
            self.maybe_gc();
            self.maybe_quit()?;
        }
        frame.pc = dest;
        Ok(())
    }

    fn bc_top(&self) -> Value {
        self.stack.last().copied().unwrap_or_default()
    }

    fn bc_set_top(&mut self, value: Value) {
        if let Some(top) = self.stack.last_mut() {
            *top = value;
        }
    }

    fn bc_pop(&mut self) -> Value {
        self.stack.pop().unwrap_or_default()
    }

    fn bc_ref(&self, n: usize) -> Value {
        self.stack[self.stack.len() - 1 - n]
    }

    fn bc_constant(&self, frame: &Frame, i: usize) -> Value {
        self.vector_items(frame.vector)
            .get(i)
            .copied()
            .unwrap_or_default()
    }

    fn bc_replace(&mut self, n: usize, value: Value) {
        let start = self.stack.len() - n;
        self.bc_finish(start, value);
    }

    fn bc_finish(&mut self, start: usize, value: Value) {
        self.stack.truncate(start);
        self.stack.push(value);
    }

    fn bc_call_top(&mut self, name: &'static str, n: usize) -> LispResult<()> {
        let start = self.stack.len() - n;
        let args = self.stack[start..].to_vec();
        let value = self.call_primitive(name, &args)?;
        self.bc_finish(start, value);
        Ok(())
    }

    fn record_in_backtrace(&mut self, function: Value, nargs: usize) {
        let start = self.stack.len() - nargs;
        self.specpdl.push(SpecBinding::Backtrace {
            function,
            args: BacktraceArgs::Evaluated { start, len: nargs },
            debug_on_exit: false,
        });
    }

    fn bc_list_error(
        &mut self,
        function: Value,
        nargs: usize,
        predicate: Value,
        value: Value,
    ) -> Flow {
        self.record_in_backtrace(function, nargs);
        self.wrong_type(predicate, value)
    }

    fn bc_compare(
        &mut self,
        fixnum: fn(i64, i64) -> bool,
        numbers: fn((bool, bool, bool)) -> bool,
    ) -> LispResult<()> {
        let (v1, v2) = (self.bc_ref(1), self.bc_ref(0));
        let result = match (v1.as_fixnum(), v2.as_fixnum()) {
            (Some(a), Some(b)) => fixnum(a, b),
            _ => numbers(self.arithcompare(v1, v2)?),
        };
        let result = self.bool(result);
        self.bc_replace(2, result);
        Ok(())
    }

    fn bc_arith(
        &mut self,
        name: &'static str,
        fixnum: fn(i64, i64) -> Option<i64>,
    ) -> LispResult<()> {
        let (v1, v2) = (self.bc_ref(1), self.bc_ref(0));
        if let (Some(a), Some(b)) = (v1.as_fixnum(), v2.as_fixnum())
            && let Some(res) = fixnum(a, b)
            && !fixnum_overflow_p(res)
        {
            self.bc_replace(2, Value::fixnum(res));
            return Ok(());
        }
        self.bc_call_top(name, 2)
    }

    fn bc_call(&mut self, nargs: usize) -> LispResult<()> {
        let fn_index = self.stack.len() - 1 - nargs;
        self.maybe_quit()?;
        self.lisp_eval_depth += 1;
        let mut max = self.int_var(sym::MAX_LISP_EVAL_DEPTH);
        if self.lisp_eval_depth > max {
            if max < 100 {
                max = 100;
                self.set_forwarded_int(sym::MAX_LISP_EVAL_DEPTH, 100);
            }
            if self.lisp_eval_depth > max {
                return Err(self.error("Lisp nesting exceeds `max-lisp-eval-depth'"));
            }
        }
        let call_fun = self.stack[fn_index];
        let count1 = self.specpdl.len();
        self.specpdl.push(SpecBinding::Backtrace {
            function: call_fun,
            args: BacktraceArgs::Evaluated {
                start: fn_index + 1,
                len: nargs,
            },
            debug_on_exit: false,
        });
        self.maybe_gc();
        let args = self.stack[fn_index + 1..].to_vec();
        let original_fun = call_fun;
        let mut fun = call_fun;
        if fun.is_symbol() {
            fun = self.heap.symbol_ref(fun).function;
        }
        let template = if self.is_closure(fun) {
            self.vector_items(fun).first().and_then(|a| a.as_fixnum())
        } else {
            None
        };
        let val = if let Some(template) = template {
            exec_byte_code(self, fun, template, &args)?
        } else if let Some(subr) = self.subr(fun) {
            self.funcall_subr(fun, subr, &args)?
        } else {
            self.funcall_general(original_fun, &args)?
        };
        self.lisp_eval_depth -= 1;
        self.stack.truncate(fn_index + 1 + nargs);
        let val = self.maybe_debug_on_exit(count1, val)?;
        self.specpdl.pop();
        self.bc_finish(fn_index, val);
        Ok(())
    }

    fn bc_push_handler(&mut self, frame: &mut Frame, catch: bool) {
        let tag = self.bc_pop();
        let kind = if catch {
            HandlerKind::Catch(tag)
        } else {
            HandlerKind::ConditionCase(tag)
        };
        let index = self.push_handler(kind);
        let dest = frame.fetch2();
        frame.handlers.push((index, dest));
    }

    fn bc_step(&mut self, frame: &mut Frame) -> LispResult<Option<Value>> {
        let op = frame.fetch() as u8;
        match op {
            B_VARREF..=B_VARREF7 => {
                let arg = match op {
                    B_VARREF6 => frame.fetch(),
                    B_VARREF7 => frame.fetch2(),
                    _ => usize::from(op - B_VARREF),
                };
                let symbol = self.bc_constant(frame, arg);
                let value = self.symbol_value(symbol)?;
                self.stack.push(value);
            }
            B_VARSET..=B_VARSET7 => {
                let arg = match op {
                    B_VARSET6 => frame.fetch(),
                    B_VARSET7 => frame.fetch2(),
                    _ => usize::from(op - B_VARSET),
                };
                let symbol = self.bc_constant(frame, arg);
                let start = self.stack.len() - 1;
                let value = self.bc_top();
                self.set(symbol, value)?;
                self.stack.truncate(start);
            }
            B_VARBIND..=B_VARBIND7 => {
                let arg = match op {
                    B_VARBIND6 => frame.fetch(),
                    B_VARBIND7 => frame.fetch2(),
                    _ => usize::from(op - B_VARBIND),
                };
                let symbol = self.bc_constant(frame, arg);
                let start = self.stack.len() - 1;
                let value = self.bc_top();
                self.specbind(symbol, value)?;
                self.stack.truncate(start);
            }
            B_CALL..=B_CALL7 => {
                let arg = match op {
                    B_CALL6 => frame.fetch(),
                    B_CALL7 => frame.fetch2(),
                    _ => usize::from(op - B_CALL),
                };
                self.bc_call(arg)?;
            }
            B_UNBIND..=B_UNBIND7 => {
                let arg = match op {
                    B_UNBIND6 => frame.fetch(),
                    B_UNBIND7 => frame.fetch2(),
                    _ => usize::from(op - B_UNBIND),
                };
                let count = self.specpdl.len().saturating_sub(arg);
                let mark = self.stack.len();
                self.unbind_to(count)?;
                self.stack.truncate(mark);
            }
            B_POPHANDLER => {
                if let Some(index) = self.handlers.len().checked_sub(1) {
                    if frame.handlers.last().is_some_and(|&(i, _)| i == index) {
                        frame.handlers.pop();
                    }
                    self.pop_handler(index);
                }
            }
            B_PUSHCONDITIONCASE => self.bc_push_handler(frame, false),
            B_PUSHCATCH => self.bc_push_handler(frame, true),
            B_NTH => {
                let (v1, v2) = (self.bc_ref(1), self.bc_ref(0));
                match v1.as_fixnum() {
                    Some(n) if (0..=SMALL_LIST_LEN_MAX).contains(&n) => {
                        let mut list = v2;
                        let mut n = n;
                        while n > 0 && list.is_cons() {
                            list = self.xcdr(list);
                            n -= 1;
                        }
                        if list.is_cons() {
                            let car = self.xcar(list);
                            self.bc_replace(2, car);
                        } else if list.is_nil() {
                            self.bc_replace(2, Value::NIL);
                        } else {
                            return Err(self.bc_list_error(sym::NTH, 2, sym::LISTP, list));
                        }
                    }
                    _ => self.bc_call_top("nth", 2)?,
                }
            }
            B_SYMBOLP => {
                let v = self.bool(self.bc_top().is_symbol());
                self.bc_set_top(v);
            }
            B_CONSP => {
                let v = self.bool(self.bc_top().is_cons());
                self.bc_set_top(v);
            }
            B_STRINGP => {
                let v = self.bool(self.bc_top().is_string());
                self.bc_set_top(v);
            }
            B_LISTP => {
                let top = self.bc_top();
                let v = self.bool(top.is_cons() || top.is_nil());
                self.bc_set_top(v);
            }
            B_EQ => {
                let v1 = self.bc_pop();
                let top = self.bc_top();
                let v = self.bool(self.eq(v1, top));
                self.bc_set_top(v);
            }
            B_MEMQ => self.bc_call_top("memq", 2)?,
            B_NOT => {
                let v = self.bool(self.bc_top().is_nil());
                self.bc_set_top(v);
            }
            B_CAR => {
                let top = self.bc_top();
                if top.is_cons() {
                    let car = self.xcar(top);
                    self.bc_set_top(car);
                } else if !top.is_nil() {
                    return Err(self.bc_list_error(sym::CAR, 1, sym::LISTP, top));
                }
            }
            B_CDR => {
                let top = self.bc_top();
                if top.is_cons() {
                    let cdr = self.xcdr(top);
                    self.bc_set_top(cdr);
                } else if !top.is_nil() {
                    return Err(self.bc_list_error(sym::CDR, 1, sym::LISTP, top));
                }
            }
            B_CONS => {
                let v = self.cons(self.bc_ref(1), self.bc_ref(0));
                self.bc_replace(2, v);
            }
            B_LIST1..=B_LIST4 | B_LIST_N => {
                let n = if op == B_LIST_N {
                    frame.fetch()
                } else {
                    usize::from(op - B_LIST1) + 1
                };
                let items = self.stack[self.stack.len() - n..].to_vec();
                let v = self.list(&items);
                self.bc_replace(n, v);
            }
            B_LENGTH => self.bc_call_top("length", 1)?,
            B_AREF => {
                let (arrayval, idxval) = (self.bc_ref(1), self.bc_ref(0));
                let Some(idx) = idxval.as_fixnum() else {
                    return Err(self.bc_list_error(sym::AREF, 2, sym::FIXNUMP, idxval));
                };
                if self.is_vector(arrayval) || self.is_record(arrayval) {
                    let items = self.vector_items(arrayval);
                    match usize::try_from(idx).ok().and_then(|i| items.get(i)) {
                        Some(&v) => self.bc_replace(2, v),
                        None => {
                            self.record_in_backtrace(sym::AREF, 2);
                            return Err(self.args_out_of_range(arrayval, idxval));
                        }
                    }
                } else {
                    self.bc_call_top("aref", 2)?;
                }
            }
            B_ASET => {
                let (arrayval, idxval, newelt) = (self.bc_ref(2), self.bc_ref(1), self.bc_ref(0));
                let Some(idx) = idxval.as_fixnum() else {
                    return Err(self.bc_list_error(sym::ASET, 3, sym::FIXNUMP, idxval));
                };
                if self.is_vector(arrayval) || self.is_record(arrayval) {
                    let size = self.vector_items(arrayval).len();
                    if usize::try_from(idx).is_ok_and(|i| i < size) {
                        self.aset_value(arrayval, idxval, newelt)?;
                        self.bc_replace(3, newelt);
                    } else {
                        self.record_in_backtrace(sym::ASET, 3);
                        return Err(self.args_out_of_range(arrayval, idxval));
                    }
                } else {
                    self.bc_call_top("aset", 3)?;
                }
            }
            B_SYMBOL_VALUE => self.bc_call_top("symbol-value", 1)?,
            B_SYMBOL_FUNCTION => self.bc_call_top("symbol-function", 1)?,
            B_SET => self.bc_call_top("set", 2)?,
            B_FSET => self.bc_call_top("fset", 2)?,
            B_GET => self.bc_call_top("get", 2)?,
            B_SUBSTRING => self.bc_call_top("substring", 3)?,
            B_CONCAT2..=B_CONCAT4 => self.bc_call_top("concat", usize::from(op - B_CONCAT2) + 2)?,
            B_CONCAT_N => {
                let n = frame.fetch();
                self.bc_call_top("concat", n)?;
            }
            B_SUB1 => match self.bc_top().as_fixnum() {
                Some(n) if n != MOST_NEGATIVE_FIXNUM => self.bc_set_top(Value::fixnum(n - 1)),
                _ => self.bc_call_top("1-", 1)?,
            },
            B_ADD1 => match self.bc_top().as_fixnum() {
                Some(n) if n != MOST_POSITIVE_FIXNUM => self.bc_set_top(Value::fixnum(n + 1)),
                _ => self.bc_call_top("1+", 1)?,
            },
            B_EQLSIGN => self.bc_compare(|a, b| a == b, |(_, eq, _)| eq)?,
            B_GTR => self.bc_compare(|a, b| a > b, |(_, _, gt)| gt)?,
            B_LSS => self.bc_compare(|a, b| a < b, |(lt, _, _)| lt)?,
            B_LEQ => self.bc_compare(|a, b| a <= b, |(lt, eq, _)| lt || eq)?,
            B_GEQ => self.bc_compare(|a, b| a >= b, |(_, eq, gt)| gt || eq)?,
            B_DIFF => self.bc_arith("-", |a, b| a.checked_sub(b))?,
            B_NEGATE => match self.bc_top().as_fixnum() {
                Some(n) if n != MOST_NEGATIVE_FIXNUM => self.bc_set_top(Value::fixnum(-n)),
                _ => self.bc_call_top("-", 1)?,
            },
            B_PLUS => self.bc_arith("+", |a, b| a.checked_add(b))?,
            B_MAX | B_MIN => {
                let (v1, v2) = (self.bc_ref(1), self.bc_ref(0));
                match (v1.as_fixnum(), v2.as_fixnum()) {
                    (Some(a), Some(b)) => {
                        let take_v2 = if op == B_MAX { b > a } else { b < a };
                        self.bc_replace(2, if take_v2 { v2 } else { v1 });
                    }
                    _ => self.bc_call_top(if op == B_MAX { "max" } else { "min" }, 2)?,
                }
            }
            B_MULT => self.bc_arith("*", |a, b| a.checked_mul(b))?,
            B_QUO => self.bc_arith("/", |a, b| if b == 0 { None } else { a.checked_div(b) })?,
            B_REM => {
                let (v1, v2) = (self.bc_ref(1), self.bc_ref(0));
                match (v1.as_fixnum(), v2.as_fixnum()) {
                    (Some(a), Some(b)) if b != 0 => self.bc_replace(2, Value::fixnum(a % b)),
                    _ => self.bc_call_top("%", 2)?,
                }
            }
            B_POINT => self.stack.push(Value::fixnum(self.cur().pt as i64)),
            B_GOTO_CHAR => self.bc_call_top("goto-char", 1)?,
            B_INSERT => self.bc_call_top("insert", 1)?,
            B_INSERT_N => {
                let n = frame.fetch();
                self.bc_call_top("insert", n)?;
            }
            B_POINT_MAX => self.stack.push(Value::fixnum(self.cur().zv as i64)),
            B_POINT_MIN => self.stack.push(Value::fixnum(self.cur().begv as i64)),
            B_CHAR_AFTER => self.bc_call_top("char-after", 1)?,
            B_FOLLOWING_CHAR => self.bc_call_top("following-char", 0)?,
            B_PRECEDING_CHAR => self.bc_call_top("preceding-char", 0)?,
            B_CURRENT_COLUMN => self.bc_call_top("current-column", 0)?,
            B_INDENT_TO => {
                let start = self.stack.len() - 1;
                let column = self.bc_top();
                let v = self.call_primitive("indent-to", &[column, Value::NIL])?;
                self.bc_finish(start, v);
            }
            B_EOLP => self.bc_call_top("eolp", 0)?,
            B_EOBP => self.bc_call_top("eobp", 0)?,
            B_BOLP => self.bc_call_top("bolp", 0)?,
            B_BOBP => self.bc_call_top("bobp", 0)?,
            B_CURRENT_BUFFER => self.bc_call_top("current-buffer", 0)?,
            B_SET_BUFFER => self.bc_call_top("set-buffer", 1)?,
            B_SAVE_CURRENT_BUFFER_OBSOLETE | B_SAVE_CURRENT_BUFFER => {
                self.record_unwind_current_buffer();
            }
            B_INTERACTIVE_P => {
                let function = self.intern("interactive-p");
                let v = self.funcall(function, &[])?;
                self.stack.push(v);
            }
            B_FORWARD_CHAR => self.bc_call_top("forward-char", 1)?,
            B_FORWARD_WORD => self.bc_call_top("forward-word", 1)?,
            B_SKIP_CHARS_FORWARD => self.bc_call_top("skip-chars-forward", 2)?,
            B_SKIP_CHARS_BACKWARD => self.bc_call_top("skip-chars-backward", 2)?,
            B_FORWARD_LINE => self.bc_call_top("forward-line", 1)?,
            B_CHAR_SYNTAX => self.bc_call_top("char-syntax", 1)?,
            B_BUFFER_SUBSTRING => self.bc_call_top("buffer-substring", 2)?,
            B_DELETE_REGION => self.bc_call_top("delete-region", 2)?,
            B_NARROW_TO_REGION => self.bc_call_top("narrow-to-region", 2)?,
            B_WIDEN => self.bc_call_top("widen", 0)?,
            B_END_OF_LINE => self.bc_call_top("end-of-line", 1)?,
            B_CONSTANT2 => {
                let i = frame.fetch2();
                let v = self.bc_constant(frame, i);
                self.stack.push(v);
            }
            B_GOTO => {
                let dest = frame.fetch2();
                self.bc_branch(frame, dest)?;
            }
            B_GOTOIFNIL => {
                let v1 = self.bc_pop();
                let dest = frame.fetch2();
                if v1.is_nil() {
                    self.bc_branch(frame, dest)?;
                }
            }
            B_GOTOIFNONNIL => {
                let dest = frame.fetch2();
                if !self.bc_pop().is_nil() {
                    self.bc_branch(frame, dest)?;
                }
            }
            B_GOTOIFNILELSEPOP => {
                let dest = frame.fetch2();
                if self.bc_top().is_nil() {
                    self.bc_branch(frame, dest)?;
                } else {
                    self.bc_pop();
                }
            }
            B_GOTOIFNONNILELSEPOP => {
                let dest = frame.fetch2();
                if !self.bc_top().is_nil() {
                    self.bc_branch(frame, dest)?;
                } else {
                    self.bc_pop();
                }
            }
            B_RETURN => return Ok(Some(self.bc_top())),
            B_DISCARD => {
                self.bc_pop();
            }
            B_DUP => {
                let v = self.bc_top();
                self.stack.push(v);
            }
            B_SAVE_EXCURSION => self.record_unwind_protect_excursion(),
            B_SAVE_WINDOW_EXCURSION => {
                let start = self.stack.len() - 1;
                let count1 = self.specpdl.len();
                let configuration =
                    self.call_primitive("current-window-configuration", &[Value::NIL])?;
                self.record_unwind(restore_window_configuration, configuration);
                let body = self.bc_top();
                let r = self.progn(body);
                let v = self.unbind_to_value(count1, r)?;
                self.bc_finish(start, v);
            }
            B_SAVE_RESTRICTION => self.record_unwind_restriction(),
            B_CATCH => {
                let start = self.stack.len() - 2;
                let (tag, body) = (self.bc_ref(1), self.bc_ref(0));
                let v = self.internal_catch(tag, |l| l.eval(body))?;
                self.bc_finish(start, v);
            }
            B_UNWIND_PROTECT => {
                let handler = self.bc_pop();
                if self.functionp(handler) {
                    self.record_unwind(bcall0, handler);
                } else {
                    self.specpdl.push(SpecBinding::UnwindForms(handler));
                }
            }
            B_CONDITION_CASE => {
                let start = self.stack.len() - 3;
                let (var, body, handlers) = (self.bc_ref(2), self.bc_ref(1), self.bc_ref(0));
                let v = self.internal_lisp_condition_case(var, body, handlers)?;
                self.bc_finish(start, v);
            }
            B_TEMP_OUTPUT_BUFFER_SETUP | B_TEMP_OUTPUT_BUFFER_SHOW => {
                return Err(self.error(&format!(
                    "Byte opcode {op} (temp-output-buffer, obsolete since Emacs 24.1) is not supported"
                )));
            }
            B_SET_MARKER => self.bc_call_top("set-marker", 3)?,
            B_MATCH_BEGINNING => self.bc_call_top("match-beginning", 1)?,
            B_MATCH_END => self.bc_call_top("match-end", 1)?,
            B_UPCASE => self.bc_call_top("upcase", 1)?,
            B_DOWNCASE => self.bc_call_top("downcase", 1)?,
            B_STRINGEQLSIGN => self.bc_call_top("string-equal", 2)?,
            B_STRINGLSS => self.bc_call_top("string-lessp", 2)?,
            B_EQUAL => self.bc_call_top("equal", 2)?,
            B_NTHCDR => self.bc_call_top("nthcdr", 2)?,
            B_ELT => {
                let (v1, v2) = (self.bc_ref(1), self.bc_ref(0));
                match v2.as_fixnum() {
                    Some(n) if v1.is_cons() && (0..=SMALL_LIST_LEN_MAX).contains(&n) => {
                        let mut list = v1;
                        let mut n = n;
                        while n > 0 && list.is_cons() {
                            list = self.xcdr(list);
                            n -= 1;
                        }
                        if list.is_cons() {
                            let car = self.xcar(list);
                            self.bc_replace(2, car);
                        } else if list.is_nil() {
                            self.bc_replace(2, Value::NIL);
                        } else {
                            return Err(self.bc_list_error(sym::ELT, 2, sym::LISTP, list));
                        }
                    }
                    _ => self.bc_call_top("elt", 2)?,
                }
            }
            B_MEMBER => self.bc_call_top("member", 2)?,
            B_ASSQ => self.bc_call_top("assq", 2)?,
            B_NREVERSE => self.bc_call_top("nreverse", 1)?,
            B_SETCAR | B_SETCDR => {
                let (cell, newval) = (self.bc_ref(1), self.bc_ref(0));
                if !cell.is_cons() {
                    let function = if op == B_SETCAR {
                        sym::SETCAR
                    } else {
                        sym::SETCDR
                    };
                    return Err(self.bc_list_error(function, 2, sym::CONSP, cell));
                }
                if op == B_SETCAR {
                    self.heap.set_car(cell, newval);
                } else {
                    self.heap.set_cdr(cell, newval);
                }
                self.bc_replace(2, newval);
            }
            B_CAR_SAFE => {
                let v = self.car_safe(self.bc_top());
                self.bc_set_top(v);
            }
            B_CDR_SAFE => {
                let v = self.cdr_safe(self.bc_top());
                self.bc_set_top(v);
            }
            B_NCONC => self.bc_call_top("nconc", 2)?,
            B_NUMBERP => {
                let top = self.bc_top();
                let v = self.bool(top.is_fixnum() || top.is_float() || self.is_bignum(top));
                self.bc_set_top(v);
            }
            B_INTEGERP => {
                let top = self.bc_top();
                let v = self.bool(top.is_fixnum() || self.is_bignum(top));
                self.bc_set_top(v);
            }
            B_STACK_REF1..=B_STACK_REF5 => {
                let v = self.bc_ref(usize::from(op - B_STACK_REF));
                self.stack.push(v);
            }
            B_STACK_REF6 => {
                let n = frame.fetch();
                let v = self.bc_ref(n);
                self.stack.push(v);
            }
            B_STACK_REF7 => {
                let n = frame.fetch2();
                let v = self.bc_ref(n);
                self.stack.push(v);
            }
            B_STACK_SET | B_STACK_SET2 => {
                let n = if op == B_STACK_SET {
                    frame.fetch()
                } else {
                    frame.fetch2()
                };
                let index = self.stack.len() - 1 - n;
                let v = self.bc_pop();
                if index < self.stack.len() {
                    self.stack[index] = v;
                }
            }
            B_DISCARD_N => {
                let mut n = frame.fetch();
                let len = self.stack.len();
                if n & 0x80 != 0 {
                    n &= 0x7F;
                    self.stack[len - 1 - n] = self.stack[len - 1];
                }
                self.stack.truncate(len - n);
            }
            B_SWITCH => {
                let start = self.stack.len() - 2;
                let (v1, jmp_table) = (self.bc_ref(1), self.bc_ref(0));
                let found = self.gethash(v1, jmp_table, sym::UNBOUND)?;
                self.stack.truncate(start);
                if found != sym::UNBOUND {
                    let dest = found.as_fixnum().unwrap_or(0) as usize;
                    self.bc_branch(frame, dest)?;
                }
            }
            B_CONSTANT..=u8::MAX => {
                let v = self.bc_constant(frame, usize::from(op - B_CONSTANT));
                self.stack.push(v);
            }
            _ => {
                return Err(self.error(&format!(
                    "Invalid byte opcode: op={op}, ptr={}",
                    frame.pc - 1
                )));
            }
        }
        Ok(None)
    }
}
