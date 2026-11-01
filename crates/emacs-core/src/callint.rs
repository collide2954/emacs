use crate::error::LispResult;
use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::special("interactive", 0, f_interactive),
    Subr::many("funcall-interactively", 1, f_funcall_interactively),
    Subr::fixed("call-interactively", 1, 3, f_call_interactively),
    Subr::fixed("prefix-numeric-value", 1, 1, f_prefix_numeric_value),
];

const CALLINT_ARGFUNS: [&str; 7] = [
    "",
    "point",
    "mark",
    "region-beginning",
    "region-end",
    "use-region-beginning",
    "use-region-end",
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.callint_point_marker = crate::marker::f_make_marker(lisp, &[]).expect("point marker");
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

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn memchr(byte: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == byte)
}

impl Lisp {
    fn quotify_arg(&mut self, exp: Value) -> Value {
        if exp.is_cons() || (exp.is_symbol() && !exp.is_nil() && exp != sym::T) {
            return self.list(&[sym::QUOTE, exp]);
        }
        exp
    }

    fn quotify_args(&mut self, exp: Value) -> Value {
        let mut tail = exp;
        while tail.is_cons() {
            let next = self.xcdr(tail);
            let item = self.xcar(tail);
            let quoted = self.quotify_arg(item);
            self.heap.set_car(tail, quoted);
            tail = next;
        }
        exp
    }

    fn check_mark(&mut self, for_region: bool) -> LispResult<()> {
        let mark = self.cur().mark;
        let tem = self.marker(mark).buffer;
        if tem.is_nil() || tem != self.current_buffer {
            return Err(self.error(if for_region {
                "The mark is not set now, so there is no region"
            } else {
                "The mark is not set now"
            }));
        }
        if !self.plain_value(sym::TRANSIENT_MARK_MODE).is_nil()
            && self.plain_value(sym::MARK_EVEN_IF_INACTIVE).is_nil()
            && self.plain_value(sym::MARK_ACTIVE).is_nil()
        {
            return Err(self.signal(sym::MARK_INACTIVE, Value::NIL));
        }
        Ok(())
    }

    fn fix_command(&mut self, function: Value, values: Value) -> LispResult<()> {
        if !values.is_cons() || !function.is_symbol() {
            return Ok(());
        }
        let reps = self.get(function, sym::INTERACTIVE_ARGS);
        if reps.is_cons() {
            let mut i = 0;
            let mut vals = values;
            while !vals.is_nil() {
                let rep = self.assq(Value::fixnum(i), reps);
                if !rep.is_nil() {
                    let replacement = self.xcdr(rep);
                    self.heap.set_car(vals, replacement);
                }
                vals = self.xcdr(vals);
                i += 1;
            }
        }
        let arity = crate::eval::f_func_arity(self, &[function])?;
        let min = self.xcar(arity);
        if min.is_fixnum() && self.xcdr(arity).is_fixnum() {
            let mut last = Value::NIL;
            let mut last_i = 0;
            let mut i = 0;
            let mut tail = values;
            while tail.is_cons() {
                if !self.xcar(tail).is_nil() {
                    last = tail;
                    last_i = i;
                }
                tail = self.xcdr(tail);
                i += 1;
            }
            if last_i > 0 && last_i >= min.as_fixnum().unwrap_or(0) - 1 {
                self.heap.set_cdr(last, Value::NIL);
            }
        }
        Ok(())
    }

    fn callint_read_file_name(
        &mut self,
        default_filename: Value,
        mustmatch: Value,
        initial: Value,
        predicate: Value,
    ) -> LispResult {
        let message = self.callint_message;
        self.funcall(
            sym::READ_FILE_NAME,
            &[
                message,
                Value::NIL,
                default_filename,
                mustmatch,
                initial,
                predicate,
            ],
        )
    }

    fn restore_command_variables(&mut self, saved: usize) {
        let (this_command, original, real, last) = (
            self.stack[saved],
            self.stack[saved + 1],
            self.stack[saved + 2],
            self.stack[saved + 3],
        );
        self.set_plain_value(sym::THIS_COMMAND, this_command);
        self.set_plain_value(sym::THIS_ORIGINAL_COMMAND, original);
        self.set_plain_value(sym::REAL_THIS_COMMAND, real);
        self.set_kvar(Kvar::LastCommand, last);
    }

    fn read_key_for_callint(&mut self, vector: bool) -> LispResult<(Value, Value, Value)> {
        let message = self.callint_message;
        let count = self.specpdl_index();
        self.specbind(sym::CURSOR_IN_ECHO_AREA, sym::T)?;
        let len = Value::fixnum(self.string_chars(message) as i64);
        crate::textprop::f_put_text_property(
            self,
            &[
                Value::fixnum(0),
                len,
                sym::FACE,
                sym::MINIBUFFER_PROMPT,
                message,
            ],
        )?;
        let (key, description) = if vector {
            let key = crate::keyboard::keyseq::f_read_key_sequence_vector(
                self,
                &[
                    message,
                    Value::NIL,
                    sym::T,
                    Value::NIL,
                    Value::NIL,
                    Value::NIL,
                ],
            )?;
            self.stack.push(key);
            let description = self.key_description(key, Value::NIL)?;
            self.unbind_to(count)?;
            (key, description)
        } else {
            let key = crate::keyboard::keyseq::f_read_key_sequence(
                self,
                &[
                    message,
                    Value::NIL,
                    Value::NIL,
                    Value::NIL,
                    Value::NIL,
                    Value::NIL,
                ],
            )?;
            self.stack.push(key);
            self.unbind_to(count)?;
            let description = self.key_description(key, Value::NIL)?;
            (key, description)
        };
        self.stack.push(description);
        let length = crate::fns::f_length(self, &[key])?.as_fixnum().unwrap_or(0);
        let mut teml = crate::data::f_aref(self, &[key, Value::fixnum(length - 1)])?;
        if teml.is_cons() {
            teml = self.xcar(teml);
        }
        let mut up_event = Value::NIL;
        if teml.is_symbol() {
            let elements = self.get(teml, sym::EVENT_SYMBOL_ELEMENTS);
            let rest = self.cdr(elements)?;
            if !self.memq(sym::DOWN, rest).is_nil() {
                up_event = crate::keyboard::reader::f_read_event(
                    self,
                    &[Value::NIL, Value::NIL, Value::NIL],
                )?;
            }
        }
        Ok((key, description, up_event))
    }

    fn call_interactively(
        &mut self,
        function: Value,
        record_flag: Value,
        keys: Value,
    ) -> LispResult {
        let saved = self.stack.len();
        let saved_values = [
            self.plain_value(sym::THIS_COMMAND),
            self.plain_value(sym::THIS_ORIGINAL_COMMAND),
            self.plain_value(sym::REAL_THIS_COMMAND),
            self.kvar(Kvar::LastCommand),
        ];
        self.stack.extend_from_slice(&saved_values);
        let r = self.call_interactively_1(function, record_flag, keys, saved);
        self.stack.truncate(saved);
        r
    }

    fn call_interactively_1(
        &mut self,
        function: Value,
        record_flag: Value,
        keys: Value,
        saved: usize,
    ) -> LispResult {
        let mut arg_from_tty = false;
        let mut record_then_fail = false;
        let (keys, key_count) = if keys.is_nil() {
            (
                self.keyboard.this_command_keys,
                self.keyboard.this_command_key_count,
            )
        } else {
            if !self.is_vector(keys) {
                return Err(self.wrong_type(sym::VECTORP, keys));
            }
            (keys, self.vector_items(keys).len())
        };
        self.stack.push(keys);
        let prefix_arg = self.plain_value(sym::CURRENT_PREFIX_ARG);
        self.stack.push(prefix_arg);
        let enable = if function.is_symbol() {
            self.get(function, sym::ENABLE_RECURSIVE_MINIBUFFERS)
        } else {
            Value::NIL
        };
        let up_slot = self.stack.len();
        self.stack.push(Value::NIL);

        let form = self.funcall(sym::INTERACTIVE_FORM, &[function])?;
        if !form.is_cons() {
            return Err(self.wrong_type(sym::COMMANDP, function));
        }
        self.stack.push(form);
        let rest = self.xcdr(form);
        let specs = self.car(rest)?;

        if !specs.is_string() {
            let funval = crate::data::f_indirect_function(self, &[function, sym::T])?;
            let events = self.keyboard.num_input_events;
            let env = if funval.is_vectorlike()
                && let crate::object::Vectorlike::Closure(items) = self.heap.vectorlike_ref(funval)
                && items.get(1).is_some_and(|code| code.is_cons())
            {
                items.get(2).copied().unwrap_or(Value::NIL)
            } else {
                Value::NIL
            };
            let specs = self.eval_with_lexical(specs, env)?;
            self.stack.push(specs);
            if events != self.keyboard.num_input_events || !record_flag.is_nil() {
                let copy = crate::fns::f_copy_sequence(self, &[specs])?;
                let values = self.quotify_args(copy);
                self.stack.push(values);
                self.fix_command(function, values)?;
                let entry = self.cons(function, values);
                self.funcall(
                    sym::ADD_TO_HISTORY,
                    &[sym::COMMAND_HISTORY, entry, Value::NIL, sym::T],
                )?;
            }
            self.restore_command_variables(saved);
            return crate::eval::f_apply(self, &[sym::FUNCALL_INTERACTIVELY, function, specs]);
        }

        let string = self.string_bytes(specs).to_vec();
        let string_end = string.len();
        let at = |s: usize| string.get(s).copied().unwrap_or(0);

        let mut next_event = 0;
        if !self.var_true(sym::INHIBIT_MOUSE_EVENT_CHECK) {
            while next_event < key_count && !self.vector_items(keys)[next_event].is_cons() {
                next_event += 1;
            }
        }

        let mut s = 0;
        loop {
            match at(s) {
                b'+' => {
                    return Err(
                        self.error("`+' is not used in `interactive' for ordinary commands")
                    );
                }
                b'*' => {
                    if !self.plain_value(sym::BUFFER_READ_ONLY).is_nil() {
                        if !record_flag.is_nil() {
                            for &p in &string[s + 1..] {
                                if !(p == b'r' || p == b'R' || p == b'p' || p == b'P' || p == b'\n')
                                {
                                    crate::buffer::f_barf_if_buffer_read_only(self, &[Value::NIL])?;
                                }
                            }
                            record_then_fail = true;
                        } else {
                            crate::buffer::f_barf_if_buffer_read_only(self, &[Value::NIL])?;
                        }
                    }
                }
                b'-' => {}
                b'@' => {
                    let event = if next_event < key_count {
                        self.vector_items(keys)[next_event]
                    } else {
                        Value::NIL
                    };
                    let w = self.cdr_safe(event);
                    let w = if w.is_cons() {
                        self.xcar(w)
                    } else {
                        Value::NIL
                    };
                    let w = if w.is_cons() {
                        self.xcar(w)
                    } else {
                        Value::NIL
                    };
                    if event.is_cons() && self.is_window(w) {
                        if self.window(w).mini
                            && !(self.minibuf.minibuf_level > 0 && w == self.windows.minibuf_window)
                        {
                            return Err(self.error("Attempt to select inactive minibuffer window"));
                        }
                        self.run_hook(sym::MOUSE_LEAVE_BUFFER_HOOK)?;
                        crate::window::f_select_window(self, &[w, Value::NIL])?;
                    }
                }
                b'^' => {
                    self.call0(sym::HANDLE_SHIFT_SELECTION)?;
                }
                _ => break,
            }
            s += 1;
        }

        let mut nargs = 2;
        let mut tem = s;
        while tem < string_end {
            nargs += 1 + usize::from(string[tem] == b'r' || string[tem] == b'R');
            match memchr(b'\n', &string[tem..]) {
                Some(off) => tem += off + 1,
                None => break,
            }
        }

        let args_at = self.stack.len();
        self.stack.resize(args_at + 2 * nargs, Value::NIL);
        let vis_at = args_at + nargs;
        let mut varies = vec![0i8; nargs];

        let speccount = self.specpdl_index();
        if !enable.is_nil() {
            self.specbind(sym::ENABLE_RECURSIVE_MINIBUFFERS, sym::T)?;
        }

        let mut tem = s;
        let mut i = 2;
        while tem < string_end {
            let pnl = memchr(b'\n', &string[tem + 1..]).map(|off| tem + 1 + off);
            let sz = match pnl {
                Some(p) => p - (tem + 1),
                None => string_end - (tem + 1),
            };
            let prompt = self.make_string(&string[tem + 1..tem + 1 + sz]);
            self.stack[vis_at + 1] = prompt;
            let format_args = self.stack[vis_at + 1..vis_at + i].to_vec();
            self.callint_message = crate::editfns::f_format_message(self, &format_args)?;
            let message = self.callint_message;

            match string[tem] {
                code @ (b'a' | b'C') => {
                    let obarray = self.plain_value(sym::OBARRAY);
                    let predicate = if code == b'a' {
                        sym::FBOUNDP
                    } else {
                        sym::COMMANDP
                    };
                    let name = crate::minibuf::f_completing_read(
                        self,
                        &[
                            message,
                            obarray,
                            predicate,
                            sym::T,
                            Value::NIL,
                            Value::NIL,
                            Value::NIL,
                            Value::NIL,
                        ],
                    )?;
                    self.stack[vis_at + i] = name;
                    self.stack[args_at + i] = crate::lread::f_intern(self, &[name, Value::NIL])?;
                }
                b'b' => {
                    let mut buffer = self.current_buffer;
                    if self.windows.selected_window == self.windows.minibuf_window {
                        buffer =
                            crate::buffer::f_other_buffer(self, &[buffer, Value::NIL, Value::NIL])?;
                    }
                    self.stack[args_at + i] = buffer;
                    self.stack[args_at + i] = crate::minibuf::f_read_buffer(
                        self,
                        &[message, buffer, sym::T, Value::NIL],
                    )?;
                }
                b'B' => {
                    let current = self.current_buffer;
                    let other =
                        crate::buffer::f_other_buffer(self, &[current, Value::NIL, Value::NIL])?;
                    self.stack[args_at + i] = crate::minibuf::f_read_buffer(
                        self,
                        &[message, other, Value::NIL, Value::NIL],
                    )?;
                }
                b'c' => {
                    let len = Value::fixnum(self.string_chars(message) as i64);
                    crate::textprop::f_put_text_property(
                        self,
                        &[
                            Value::fixnum(0),
                            len,
                            sym::FACE,
                            sym::MINIBUFFER_PROMPT,
                            message,
                        ],
                    )?;
                    let c = crate::keyboard::reader::f_read_char(
                        self,
                        &[message, Value::NIL, Value::NIL],
                    )?;
                    self.stack[args_at + i] = c;
                    self.message3_nolog(Value::NIL)?;
                    if !c
                        .as_fixnum()
                        .is_some_and(|n| (0..=crate::character::MAX_CHAR as i64).contains(&n))
                    {
                        return Err(self.error("Non-character input-event"));
                    }
                    self.stack[vis_at + i] = crate::editfns::f_char_to_string(self, &[c])?;
                }
                b'd' => {
                    let point_marker = self.callint_point_marker;
                    let pt = Value::fixnum(self.cur().pt as i64);
                    crate::marker::f_set_marker(self, &[point_marker, pt, Value::NIL])?;
                    self.stack[args_at + i] = point_marker;
                    varies[i] = 1;
                }
                b'D' => {
                    let directory = self.plain_value(sym::DEFAULT_DIRECTORY);
                    self.stack[args_at + i] = self.callint_read_file_name(
                        directory,
                        sym::LAMBDA,
                        Value::NIL,
                        sym::FILE_DIRECTORY_P,
                    )?;
                }
                b'f' => {
                    self.stack[args_at + i] = self.callint_read_file_name(
                        Value::NIL,
                        sym::LAMBDA,
                        Value::NIL,
                        Value::NIL,
                    )?;
                }
                b'F' => {
                    self.stack[args_at + i] = self.callint_read_file_name(
                        Value::NIL,
                        Value::NIL,
                        Value::NIL,
                        Value::NIL,
                    )?;
                }
                b'G' => {
                    let empty = self.empty_unibyte;
                    self.stack[args_at + i] =
                        self.callint_read_file_name(Value::NIL, Value::NIL, empty, Value::NIL)?;
                }
                b'i' => {
                    varies[i] = -1;
                }
                code @ (b'k' | b'K') => {
                    let (key, description, up_event) = self.read_key_for_callint(code == b'K')?;
                    self.stack[args_at + i] = key;
                    self.stack[vis_at + i] = description;
                    if !up_event.is_nil() {
                        self.stack[up_slot] = up_event;
                    }
                }
                b'U' => {
                    let up_event = self.stack[up_slot];
                    if !up_event.is_nil() {
                        let key = self.vector(vec![up_event]);
                        self.stack[args_at + i] = key;
                        self.stack[up_slot] = Value::NIL;
                        self.stack[vis_at + i] = self.key_description(key, Value::NIL)?;
                    }
                }
                b'e' => {
                    if next_event >= key_count {
                        let name = if function.is_symbol() {
                            let name = self.symbol_name(function);
                            self.string_bytes(name).to_vec()
                        } else {
                            b"command".to_vec()
                        };
                        return Err(self.verror(
                            "%s must be bound to an event with parameters",
                            &[crate::doprnt::Arg::Str(&name)],
                        ));
                    }
                    self.stack[args_at + i] = self.vector_items(keys)[next_event];
                    varies[i] = -1;
                    if self.var_true(sym::INHIBIT_MOUSE_EVENT_CHECK) {
                        next_event += 1;
                    } else {
                        next_event += 1;
                        while next_event < key_count
                            && !self.vector_items(keys)[next_event].is_cons()
                        {
                            next_event += 1;
                        }
                    }
                }
                b'm' => {
                    self.check_mark(false)?;
                    self.stack[args_at + i] = self.cur().mark;
                    varies[i] = 2;
                }
                b'M' => {
                    self.stack[args_at + i] = crate::minibuf::f_read_string(
                        self,
                        &[message, Value::NIL, Value::NIL, Value::NIL, sym::T],
                    )?;
                }
                code @ (b'N' | b'n' | b'p') => {
                    if code == b'p' || (code == b'N' && !prefix_arg.is_nil()) {
                        self.stack[args_at + i] = f_prefix_numeric_value(self, &[prefix_arg])?;
                        varies[i] = -1;
                    } else {
                        let number = self.funcall(sym::READ_NUMBER, &[message])?;
                        self.stack[args_at + i] = number;
                        self.stack[vis_at + i] = crate::data::f_number_to_string(self, &[number])?;
                    }
                }
                b'P' => {
                    self.stack[args_at + i] = prefix_arg;
                    varies[i] = -1;
                }
                code @ (b'R' | b'r') => {
                    if code == b'R' && self.call0(sym::USE_REGION_P)?.is_nil() {
                        varies[i] = 5;
                        i += 1;
                        varies[i] = 6;
                    } else {
                        self.check_mark(true)?;
                        let point_marker = self.callint_point_marker;
                        let pt = self.cur().pt;
                        crate::marker::f_set_marker(
                            self,
                            &[point_marker, Value::fixnum(pt as i64), Value::NIL],
                        )?;
                        let mark_marker = self.cur().mark;
                        let mark = self.marker(mark_marker).charpos;
                        self.stack[args_at + i] =
                            if pt < mark { point_marker } else { mark_marker };
                        varies[i] = if code == b'R' { 5 } else { 3 };
                        i += 1;
                        self.stack[args_at + i] =
                            if pt > mark { point_marker } else { mark_marker };
                        varies[i] = if code == b'R' { 6 } else { 4 };
                    }
                }
                b's' => {
                    self.stack[args_at + i] = crate::minibuf::f_read_string(
                        self,
                        &[message, Value::NIL, Value::NIL, Value::NIL, Value::NIL],
                    )?;
                }
                b'S' => {
                    let name = crate::minibuf::f_read_string(
                        self,
                        &[message, Value::NIL, Value::NIL, Value::NIL, Value::NIL],
                    )?;
                    self.stack[vis_at + i] = name;
                    self.stack[args_at + i] = crate::lread::f_intern(self, &[name, Value::NIL])?;
                }
                b'v' => {
                    self.stack[args_at + i] =
                        crate::minibuf::f_read_variable(self, &[message, Value::NIL])?;
                    self.stack[vis_at + i] = self.minibuf.last_minibuf_string;
                }
                code @ (b'x' | b'X') => {
                    let reader = if code == b'x' {
                        sym::READ_MINIBUFFER
                    } else {
                        sym::EVAL_MINIBUFFER
                    };
                    self.stack[args_at + i] = self.funcall(reader, &[message])?;
                    self.stack[vis_at + i] = self.minibuf.last_minibuf_string;
                }
                b'Z' => {
                    if prefix_arg.is_nil() {
                        varies[i] = -1;
                    } else {
                        self.stack[args_at + i] =
                            crate::coding::f_read_non_nil_coding_system(self, &[message])?;
                        self.stack[vis_at + i] = self.minibuf.last_minibuf_string;
                    }
                }
                b'z' => {
                    self.stack[args_at + i] =
                        crate::coding::f_read_coding_system(self, &[message, Value::NIL])?;
                    self.stack[vis_at + i] = self.minibuf.last_minibuf_string;
                }
                first => {
                    let bytes_left = string_end - tem;
                    let letter = if bytes_left >= crate::character::bytes_by_char_head(first) {
                        crate::character::string_char(&string[tem..]).0
                    } else {
                        u32::from(first)
                    };
                    let letter = i64::from(letter);
                    return Err(self.verror(
                        "Invalid control letter `%c' (#o%03o, #x%04x) in interactive calling string",
                        &[
                            crate::doprnt::Arg::Int(letter),
                            crate::doprnt::Arg::Int(letter),
                            crate::doprnt::Arg::Int(letter),
                        ],
                    ));
                }
            }

            if varies[i] == 0 {
                arg_from_tty = true;
            }
            if self.stack[vis_at + i].is_nil() && self.stack[args_at + i].is_string() {
                self.stack[vis_at + i] = self.stack[args_at + i];
            }
            tem = match memchr(b'\n', &string[tem..]) {
                Some(off) => tem + off + 1,
                None => string_end,
            };
            i += 1;
        }
        self.unbind_to(speccount)?;
        self.maybe_quit()?;

        self.stack[args_at] = sym::FUNCALL_INTERACTIVELY;
        self.stack[args_at + 1] = function;

        if arg_from_tty || !record_flag.is_nil() {
            self.stack[vis_at + 1] = function;
            for i in 2..nargs {
                self.stack[vis_at + i] = if varies[i] > 0 {
                    let name = self.intern(CALLINT_ARGFUNS[varies[i] as usize]);
                    self.list(&[name])
                } else {
                    let value = self.stack[args_at + i];
                    self.quotify_arg(value)
                };
            }
            let items = self.stack[vis_at + 1..vis_at + nargs].to_vec();
            let entry = self.list(&items);
            self.funcall(
                sym::ADD_TO_HISTORY,
                &[sym::COMMAND_HISTORY, entry, Value::NIL, sym::T],
            )?;
        }

        for (i, kind) in varies.iter().enumerate().skip(2) {
            if (1..=4).contains(kind) {
                let marker = self.stack[args_at + i];
                self.stack[args_at + i] = Value::fixnum(self.marker(marker).charpos as i64);
            }
        }

        if record_then_fail {
            crate::buffer::f_barf_if_buffer_read_only(self, &[Value::NIL])?;
        }

        self.restore_command_variables(saved);
        self.specbind(sym::COMMAND_DEBUG_STATUS, Value::NIL)?;
        let call = self.stack[args_at..args_at + nargs].to_vec();
        let val = self.funcall(call[0], &call[1..]);
        self.unbind_to_value(speccount, val)
    }
}

fn f_call_interactively(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.call_interactively(args[0], arg(args, 1), arg(args, 2))
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
