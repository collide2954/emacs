use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub struct Minibuf {
    pub minibuffer_list: Value,
    pub command_loop_level_list: Value,
    pub minibuf_level: i64,
    pub minibuf_prompt: Value,
    pub minibuf_save_list: Value,
    pub command_loop_level: i64,
    pub mb_frame: Value,
    pub last_minibuf_string: Value,
    pub minibuf_prompt_width: i64,
    pub exp_mb_frame: Value,
}

impl Default for Minibuf {
    fn default() -> Self {
        Minibuf {
            minibuffer_list: Value::NIL,
            command_loop_level_list: Value::NIL,
            minibuf_level: 0,
            minibuf_prompt: Value::NIL,
            minibuf_save_list: Value::NIL,
            command_loop_level: 0,
            mb_frame: Value::NIL,
            last_minibuf_string: Value::NIL,
            minibuf_prompt_width: 0,
            exp_mb_frame: Value::NIL,
        }
    }
}

impl Minibuf {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.minibuffer_list,
            self.command_loop_level_list,
            self.minibuf_prompt,
            self.minibuf_save_list,
            self.mb_frame,
            self.last_minibuf_string,
            self.exp_mb_frame,
        ]);
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("active-minibuffer-window", 0, 0, f_active_minibuffer_window),
    Subr::fixed("set-minibuffer-window", 1, 1, f_set_minibuffer_window),
    Subr::fixed("minibufferp", 0, 2, f_minibufferp),
    Subr::fixed("innermost-minibuffer-p", 0, 1, f_innermost_minibuffer_p),
    Subr::fixed(
        "minibuffer-innermost-command-loop-p",
        0,
        1,
        f_minibuffer_innermost_command_loop_p,
    ),
    Subr::fixed("abort-minibuffers", 0, 0, f_abort_minibuffers).interactive(""),
    Subr::fixed("minibuffer-prompt-end", 0, 0, f_minibuffer_prompt_end),
    Subr::fixed("minibuffer-contents", 0, 0, f_minibuffer_contents),
    Subr::fixed(
        "minibuffer-contents-no-properties",
        0,
        0,
        f_minibuffer_contents_no_properties,
    ),
    Subr::fixed("read-from-minibuffer", 1, 7, f_read_from_minibuffer),
    Subr::fixed("read-string", 1, 5, f_read_string),
    Subr::fixed("read-command", 1, 2, f_read_command),
    Subr::fixed("read-variable", 1, 2, f_read_variable),
    Subr::fixed("read-buffer", 1, 4, f_read_buffer),
    Subr::fixed("minibuffer-depth", 0, 0, f_minibuffer_depth),
    Subr::fixed("minibuffer-prompt", 0, 0, f_minibuffer_prompt),
    Subr::fixed("try-completion", 2, 3, f_try_completion),
    Subr::fixed("all-completions", 2, 3, f_all_completions),
    Subr::fixed("test-completion", 2, 3, f_test_completion),
    Subr::fixed("internal-complete-buffer", 3, 3, f_internal_complete_buffer),
    Subr::fixed("assoc-string", 2, 3, f_assoc_string),
    Subr::fixed("completing-read", 2, 8, f_completing_read),
    Subr::fixed(
        "completion--flex-cost-gotoh",
        2,
        2,
        f_completion_flex_cost_gotoh,
    ),
];

pub fn init_vars(lisp: &mut Lisp) {
    for symbol in [
        sym::READ_EXPRESSION_HISTORY,
        sym::MINIBUFFER_SETUP_HOOK,
        sym::MINIBUFFER_EXIT_HOOK,
        sym::MINIBUFFER_COMPLETION_TABLE,
        sym::MINIBUFFER_COMPLETION_PREDICATE,
        sym::MINIBUFFER_COMPLETION_CONFIRM,
        sym::MINIBUFFER_COMPLETING_FILE_NAME,
        sym::MINIBUFFER_HELP_FORM,
        sym::MINIBUFFER_HISTORY_POSITION,
        sym::COMPLETION_REGEXP_LIST,
        sym::READ_HIDE_CHAR,
    ] {
        lisp.defvar(symbol, Value::NIL);
    }
    lisp.defvar(sym::MINIBUFFER_FOLLOWS_SELECTED_FRAME, sym::T);
    lisp.defvar_bool(sym::READ_BUFFER_COMPLETION_IGNORE_CASE, false);
    lisp.defvar(sym::HISTORY_LENGTH, Value::fixnum(100));
    lisp.defvar_bool(sym::HISTORY_DELETE_DUPLICATES, false);
    lisp.defvar(sym::HISTORY_ADD_NEW_INPUT, sym::T);
    lisp.defvar_bool(sym::COMPLETION_IGNORE_CASE, false);
    lisp.defvar_bool(sym::ENABLE_RECURSIVE_MINIBUFFERS, false);
    lisp.defvar(sym::MINIBUFFER_HISTORY_VARIABLE, Value::fixnum(0));
    lisp.defvar_bool(sym::MINIBUFFER_AUTO_RAISE, false);
    lisp.defvar_bool(sym::MINIBUFFER_ALLOW_TEXT_PROPERTIES, false);
    let props = lisp.list(&[sym::READ_ONLY, sym::T]);
    lisp.defvar(sym::MINIBUFFER_PROMPT_PROPERTIES, props);
    lisp.defvar_bool(sym::INHIBIT_INTERACTION, false);
    lisp.defvar_bool(sym::READ_MINIBUFFER_RESTORE_WINDOWS, true);
    lisp.defvar(sym::READ_BUFFER_FUNCTION, Value::NIL);
    lisp.set_plain_value(sym::MINIBUFFER_DEFAULT, Value::NIL);
    lisp.set_plain_value(sym::CUSTOM_VARIABLE_HISTORY, Value::NIL);
    lisp.set_plain_value(sym::BUFFER_NAME_HISTORY, Value::NIL);
}

pub fn init_minibuf_once(lisp: &mut Lisp) {
    lisp.minibuf = Minibuf::default();
    lisp.get_minibuffer(0).expect("initial minibuffer");
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn minibuffer_ent_greater(lisp: &mut Lisp, ent1: Value, ent2: Value) -> LispResult<bool> {
    let buffer1 = lisp.car(ent1)?;
    let buffer2 = lisp.car(ent2)?;
    Ok(lisp.this_minibuffer_depth(buffer1) > lisp.this_minibuffer_depth(buffer2))
}

impl Lisp {
    pub fn get_minibuffer(&mut self, depth: i64) -> LispResult {
        let n = Value::fixnum(depth);
        let mut tail = crate::fns::f_nthcdr(self, &[n, self.minibuf.minibuffer_list])?;
        let mut cll_tail = crate::fns::f_nthcdr(self, &[n, self.minibuf.command_loop_level_list])?;
        if tail.is_nil() {
            tail = self.list(&[Value::NIL]);
            let list = self.minibuf.minibuffer_list;
            self.minibuf.minibuffer_list = self.nconc2(list, tail);
            cll_tail = self.list(&[Value::NIL]);
            let list = self.minibuf.command_loop_level_list;
            self.minibuf.command_loop_level_list = self.nconc2(list, cll_tail);
        }
        let level = if depth != 0 {
            self.minibuf.command_loop_level
        } else {
            0
        };
        self.heap.set_car(cll_tail, Value::fixnum(level));
        let mut buf = self.car_safe(tail);
        if buf.is_nil() || !self.buffer_live_p(buf) {
            let name = self.string(&format!(" *Minibuf-{depth}*"));
            buf = self.get_buffer_create(name, false)?;
            self.heap.set_car(tail, buf);
            crate::buffer::f_buffer_enable_undo(self, &[buf])?;
        } else {
            crate::overlay::f_delete_all_overlays(self, &[buf])?;
            self.reset_buffer(buf);
        }
        Ok(buf)
    }

    pub fn set_minibuffer_mode(&mut self, buf: Value, depth: i64) -> LispResult<()> {
        let count = self.specpdl.len();
        self.record_unwind_current_buffer();
        let r = self.set_minibuffer_mode_1(buf, depth);
        let u = self.unbind_to(count);
        r?;
        u
    }

    fn set_minibuffer_mode_1(&mut self, buf: Value, depth: i64) -> LispResult<()> {
        crate::buffer::f_set_buffer(self, &[buf])?;
        if depth > 0 {
            if !self.heap.symbol_ref(sym::MINIBUFFER_MODE).function.is_nil() {
                self.funcall(sym::MINIBUFFER_MODE, &[])?;
            }
        } else if !self
            .heap
            .symbol_ref(sym::MINIBUFFER_INACTIVE_MODE)
            .function
            .is_nil()
        {
            self.funcall(sym::MINIBUFFER_INACTIVE_MODE, &[])?;
        } else {
            crate::buffer::f_kill_all_local_variables(self, &[Value::NIL])?;
        }
        Ok(())
    }

    fn minibuf_follows_frame(&mut self) -> LispResult<bool> {
        let value =
            crate::eval::f_default_toplevel_value(self, &[sym::MINIBUFFER_FOLLOWS_SELECTED_FRAME])?;
        Ok(value == sym::T)
    }

    fn minibuf_moves_frame_when_opened(&mut self) -> LispResult<bool> {
        let value =
            crate::eval::f_default_toplevel_value(self, &[sym::MINIBUFFER_FOLLOWS_SELECTED_FRAME])?;
        Ok(!value.is_nil())
    }

    fn choose_minibuf_frame(&mut self) {
        let sf = self.frames.selected_frame;
        if self.is_frame(sf) && self.frame_live_p(sf) {
            let mw = self.frame(sf).minibuffer_window;
            if self.window_live_p(mw) && self.windows.minibuf_window != mw {
                self.windows.minibuf_window = mw;
            }
        }
    }

    fn zip_minibuffer_stacks(
        &mut self,
        dest_window: Value,
        source_window: Value,
    ) -> LispResult<()> {
        let base = self.stack.len();
        let r = self.zip_minibuffer_stacks_1(dest_window, source_window);
        self.stack.truncate(base);
        r
    }

    fn zip_minibuffer_stacks_1(
        &mut self,
        dest_window: Value,
        source_window: Value,
    ) -> LispResult<()> {
        let (dw_contents, dw_prev) = {
            let dw = self.window(dest_window);
            (dw.contents, dw.prev_buffers)
        };
        if !self.live_minibuffer_p(dw_contents) && dw_prev.is_nil() {
            let sw_contents = self.window(source_window).contents;
            self.set_window_buffer(dest_window, sw_contents, false, false)?;
            let start = crate::window::f_window_start(self, &[source_window])?;
            crate::window::f_set_window_start(self, &[dest_window, start, Value::NIL])?;
            let point = crate::window::f_window_point(self, &[source_window])?;
            crate::window::f_set_window_point(self, &[dest_window, point])?;
            let sw_prev = self.window(source_window).prev_buffers;
            self.window_mut(dest_window).prev_buffers = sw_prev;
            let minibuffer = self.nth_minibuffer(0)?;
            self.set_window_buffer(source_window, minibuffer, false, false)?;
            self.window_mut(source_window).prev_buffers = Value::NIL;
            return Ok(());
        }
        self.funcall(sym::RECORD_WINDOW_BUFFER, &[dest_window])?;
        self.funcall(sym::RECORD_WINDOW_BUFFER, &[source_window])?;
        let (dw_prev, sw_prev) = (
            self.window(dest_window).prev_buffers,
            self.window(source_window).prev_buffers,
        );
        let mut acc = self.merge_c(dw_prev, sw_prev, minibuffer_ent_greater)?;
        self.stack.push(acc);
        if !acc.is_nil() {
            let d_ent = self.car(acc)?;
            acc = self.cdr(acc)?;
            self.stack.push(acc);
            let buffer = self.car(d_ent)?;
            self.set_window_buffer(dest_window, buffer, false, false)?;
            let rest = self.cdr(d_ent)?;
            let start = self.car(rest)?;
            crate::window::f_set_window_start(self, &[dest_window, start, Value::NIL])?;
            let rest = self.cdr(d_ent)?;
            let rest = self.cdr(rest)?;
            let point = self.car(rest)?;
            crate::window::f_set_window_point(self, &[dest_window, point])?;
        }
        self.window_mut(dest_window).prev_buffers = acc;
        self.window_mut(source_window).prev_buffers = Value::NIL;
        let minibuffer = self.nth_minibuffer(0)?;
        self.set_window_buffer(source_window, minibuffer, false, false)
    }

    pub fn move_minibuffers_onto_frame(
        &mut self,
        of: Value,
        frame: Value,
        for_deletion: bool,
    ) -> LispResult<()> {
        let f = frame;
        self.windows.minibuf_window = self.frame(f).minibuffer_window;
        if !(self.minibuf.minibuf_level != 0
            && (for_deletion
                || self.minibuf_follows_frame()?
                || self.frame(of).output_method == crate::terminal::OutputMethod::Initial))
        {
            return Ok(());
        }
        let (fmw, omw) = (
            self.frame(f).minibuffer_window,
            self.frame(of).minibuffer_window,
        );
        if self.frame_live_p(f) && fmw != omw && self.window_live_p(fmw) && self.window_live_p(omw)
        {
            self.zip_minibuffer_stacks(fmw, omw)?;
            if for_deletion && self.minibuf.mb_frame != of {
                self.minibuf.mb_frame = frame;
            }
        }
        Ok(())
    }

    pub fn set_initial_minibuffer_mode(&mut self) -> LispResult<()> {
        let minibuf = self.get_minibuffer(0)?;
        self.set_minibuffer_mode(minibuf, 0)
    }

    fn live_minibuffer_p(&self, buffer: Value) -> bool {
        let list = self.minibuf.minibuffer_list;
        if buffer == self.car_safe(list) {
            return false;
        }
        let mut tem = self.cdr_safe(list);
        for _ in 1..=self.minibuf.minibuf_level {
            if self.car_safe(tem) == buffer {
                return true;
            }
            tem = self.cdr_safe(tem);
        }
        false
    }

    pub fn this_minibuffer_depth(&self, buffer: Value) -> i64 {
        let buffer = if buffer.is_nil() {
            self.current_buffer
        } else {
            buffer
        };
        let mut bufs = self.cdr_safe(self.minibuf.minibuffer_list);
        for i in 1..=self.minibuf.minibuf_level {
            if self.car_safe(bufs) == buffer {
                return i;
            }
            bufs = self.cdr_safe(bufs);
        }
        0
    }

    pub fn is_minibuffer(&self, depth: i64, buf: Value) -> bool {
        let mut tail = self.minibuf.minibuffer_list;
        for _ in 0..depth {
            tail = self.cdr_safe(tail);
        }
        !tail.is_nil() && self.car_safe(tail) == buf
    }

    pub fn minibufferp(&self, buffer: Value, live: bool) -> bool {
        if live {
            self.live_minibuffer_p(buffer)
        } else {
            !self.memq(buffer, self.minibuf.minibuffer_list).is_nil()
        }
    }
}

pub(crate) fn f_minibufferp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = arg(args, 0);
    let buffer = if buffer.is_nil() {
        lisp.current_buffer
    } else if lisp.is_string(buffer) {
        crate::buffer::f_get_buffer(lisp, &[buffer])?
    } else {
        lisp.check_buffer(buffer)?
    };
    let r = lisp.minibufferp(buffer, !arg(args, 1).is_nil());
    Ok(lisp.bool(r))
}

fn f_innermost_minibuffer_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = if arg(args, 0).is_nil() {
        lisp.current_buffer
    } else {
        arg(args, 0)
    };
    let n = Value::fixnum(lisp.minibuf.minibuf_level);
    let tail = crate::fns::f_nthcdr(lisp, &[n, lisp.minibuf.minibuffer_list])?;
    let innermost = lisp.car_safe(tail);
    Ok(lisp.bool(buffer == innermost))
}

impl Lisp {
    fn nth_minibuffer(&mut self, depth: i64) -> LispResult {
        let list = self.minibuf.minibuffer_list;
        let tail = crate::fns::f_nthcdr(self, &[Value::fixnum(depth), list])?;
        self.car(tail)
    }

    fn minibuf_c_loop_level(&self, depth: i64) -> i64 {
        self.nth(depth as usize, self.minibuf.command_loop_level_list)
            .as_fixnum()
            .unwrap_or(0)
    }

    fn string_to_object(&mut self, val: Value, defalt: Value) -> LispResult {
        let mut val = val;
        if val.is_string() && self.string_chars(val) == 0 {
            if defalt.is_string() {
                val = defalt;
            } else if defalt.is_cons() && self.xcar(defalt).is_string() {
                val = self.xcar(defalt);
            }
        }
        let expr_and_pos = crate::lread::f_read_from_string(self, &[val, Value::NIL, Value::NIL])?;
        let pos = self.cdr(expr_and_pos)?.as_fixnum().unwrap_or(0) as usize;
        if pos != self.string_chars(val) {
            let bytes = self.string_bytes(val);
            let pos_byte = if self.string_multibyte(val) {
                crate::character::char_offset_to_byte(bytes, pos)
            } else {
                pos
            };
            if bytes[pos_byte..]
                .iter()
                .any(|&c| c != b' ' && c != b'\t' && c != b'\n')
            {
                let msg = self.string("Trailing garbage following expression");
                return Err(self.signal_list(sym::INVALID_READ_SYNTAX, &[msg]));
            }
        }
        self.car(expr_and_pos)
    }

    fn read_minibuf_noninteractive(
        &mut self,
        prompt: Value,
        expflag: bool,
        defalt: Value,
    ) -> LispResult {
        use std::io::{Read, Write};
        let hide_char = match self.plain_value(sym::READ_HIDE_CHAR).as_fixnum() {
            Some(n) if (0..=crate::character::MAX_CHAR as i64).contains(&n) => n as u8,
            _ => 0,
        };
        let bytes = self.string_bytes(prompt).to_vec();
        let _ = self.stdout.write_all(&bytes);
        let _ = self.stdout.flush();
        let mut line = Vec::new();
        let mut last = None;
        loop {
            let mut byte = [0u8; 1];
            match self.stdin.read(&mut byte) {
                Ok(1) => {
                    if byte[0] == b'\n' || byte[0] == b'\r' {
                        last = Some(byte[0]);
                        break;
                    }
                    if hide_char != 0 {
                        let _ = self.stdout.write_all(&[hide_char]);
                    }
                    line.push(byte[0]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                _ => break,
            }
        }
        if hide_char != 0 {
            let _ = self.stdout.write_all(b"\n");
        }
        if line.is_empty() && last.is_none() {
            let msg = self.string("Error reading from stdin");
            return Err(self.signal_list(sym::END_OF_FILE, &[msg]));
        }
        let val = self.make_string(&line);
        if expflag {
            let defalt = if defalt.is_cons() {
                self.xcar(defalt)
            } else {
                defalt
            };
            return self.string_to_object(val, defalt);
        }
        Ok(val)
    }

    fn read_minibuf(&mut self, req: MinibufRequest) -> LispResult {
        let count = self.specpdl.len();
        let base = self.stack.len();
        let calling_frame = self.frames.selected_frame;
        let calling_window = self.windows.selected_window;
        self.stack.extend_from_slice(&[
            req.map,
            req.initial,
            req.prompt,
            req.histvar,
            req.histpos,
            req.defalt,
            calling_frame,
            calling_window,
        ]);
        let r = self.read_minibuf_1(count, req, calling_frame, calling_window);
        let r = self.unbind_to_value(count, r);
        self.stack.truncate(base);
        r
    }

    fn read_minibuf_1(
        &mut self,
        count: usize,
        req: MinibufRequest,
        calling_frame: Value,
        calling_window: Value,
    ) -> LispResult {
        let MinibufRequest {
            initial,
            prompt,
            expflag,
            defalt,
            ..
        } = req;
        self.specbind(sym::MINIBUFFER_DEFAULT, defalt)?;
        self.specbind(sym::INHIBIT_READ_ONLY, Value::NIL)?;
        let this_command = self.plain_value(sym::THIS_COMMAND);
        self.specbind(sym::CURRENT_MINIBUFFER_COMMAND, this_command)?;
        let completing_file_name = self.plain_value(sym::MINIBUFFER_COMPLETING_FILE_NAME);
        self.specbind(sym::MINIBUFFER_COMPLETING_FILE_NAME, completing_file_name)?;
        if completing_file_name == sym::LAMBDA {
            self.set_plain_value(sym::MINIBUFFER_COMPLETING_FILE_NAME, Value::NIL);
        }
        let mut initial = initial;
        let mut pos = 0;
        if !initial.is_nil() {
            if initial.is_cons() {
                let backup_n = self.xcdr(initial);
                initial = self.xcar(initial);
                self.check_string(initial)?;
                if !backup_n.is_nil() {
                    let n = self.check_fixnum(backup_n)?;
                    let len = self.string_chars(initial) as i64;
                    pos = if n < 1 { -len } else { n - 1 - len };
                }
            } else {
                self.check_string(initial)?;
            }
        }
        let ambient_dir = self.plain_value(sym::DEFAULT_DIRECTORY);
        self.stack.push(ambient_dir);
        let prompt = if prompt.is_string() {
            prompt
        } else {
            self.empty_unibyte
        };
        if !self.var_true(sym::ENABLE_RECURSIVE_MINIBUFFERS) && self.minibuf.minibuf_level > 0 {
            let str = self.string("Command attempted to use minibuffer while in minibuffer");
            if self.windows.selected_window == self.windows.minibuf_window {
                return Err(self.signal_list(sym::USER_ERROR, &[str]));
            }
            return Err(self.throw(sym::EXIT, str));
        }
        if self.noninteractive() && self.plain_value(sym::EXECUTING_KBD_MACRO).is_nil() {
            return self.read_minibuf_noninteractive(prompt, expflag, defalt);
        }
        let req = MinibufRequest {
            initial,
            prompt,
            ..req
        };
        let calling = (calling_frame, calling_window);
        self.read_minibuf_interactive(count, req, pos, calling, ambient_dir)
    }

    fn read_minibuf_interactive(
        &mut self,
        count: usize,
        req: MinibufRequest,
        pos: i64,
        (calling_frame, calling_window): (Value, Value),
        ambient_dir: Value,
    ) -> LispResult {
        let MinibufRequest {
            map,
            initial,
            prompt,
            expflag,
            histvar,
            histpos,
            defalt,
            allow_props,
            inherit_input_method,
        } = req;
        let minibuffer = self.get_minibuffer(self.minibuf.minibuf_level + 1)?;
        self.minibuf.minibuf_level += 1;
        self.record_unwind_current_buffer();
        self.choose_minibuf_frame();
        let mini_frame = self.window(self.windows.minibuf_window).frame;
        self.stack.push(mini_frame);
        if self.minibuf.minibuf_level > 1 {
            let mb_frame = self.minibuf.mb_frame;
            let sf_mini_window = self.frame(self.frames.selected_frame).minibuffer_window;
            if self.window_live_p(self.frame(mb_frame).minibuffer_window)
                && self.window(sf_mini_window).frame != mb_frame
                && self.minibuf_moves_frame_when_opened()?
                && !self.minibuf_follows_frame()?
            {
                let of_mini_window = self.frame(mb_frame).minibuffer_window;
                self.zip_minibuffer_stacks(self.windows.minibuf_window, of_mini_window)?;
                let of_selected = self.frame(mb_frame).selected_window;
                if self.window(of_selected).mini {
                    let first = crate::window::f_frame_first_window(self, &[mb_frame])?;
                    crate::window::f_set_frame_selected_window(
                        self,
                        &[mb_frame, first, Value::NIL],
                    )?;
                }
            }
        }
        let sf_mini_window = self.frame(self.frames.selected_frame).minibuffer_window;
        self.minibuf.mb_frame = self.window(sf_mini_window).frame;
        self.funcall(sym::RECORD_WINDOW_BUFFER, &[self.windows.minibuf_window])?;
        self.record_unwind(minibuffer_unwind, Value::NIL);
        if self.var_true(sym::READ_MINIBUFFER_RESTORE_WINDOWS) {
            let config = crate::window::f_current_window_configuration(self, &[Value::NIL])?;
            let data = self.list(&[config, sym::T, sym::T]);
            self.record_unwind(crate::window::restore_window_configuration, data);
            if mini_frame != self.frames.selected_frame {
                let config = crate::window::f_current_window_configuration(self, &[mini_frame])?;
                let data = self.list(&[config, Value::NIL, sym::T]);
                self.record_unwind(crate::window::restore_window_configuration, data);
            }
        } else if mini_frame != self.frames.selected_frame {
            let sf = self.frames.selected_frame;
            let data = self.cons(sf, self.frame(sf).focus_frame);
            self.record_unwind(crate::window::restore_focus_frame, data);
        }
        crate::frame::f_make_frame_visible(self, &[mini_frame])?;
        if self.var_true(sym::MINIBUFFER_AUTO_RAISE) {
            crate::frame::f_raise_frame(self, &[mini_frame])?;
        }
        if !self.var_true(sym::MULTIPLE_TERMINALS_MERGE_KEYBOARDS) {
            self.temporarily_switch_to_single_kboard(Some(mini_frame))?;
        }
        let saved = [
            calling_window,
            calling_frame,
            self.windows.minibuf_window,
            self.plain_value(sym::OVERRIDING_LOCAL_MAP),
            self.plain_value(sym::MINIBUFFER_HISTORY_VARIABLE),
            self.plain_value(sym::MINIBUFFER_HISTORY_POSITION),
            self.plain_value(sym::CURRENT_PREFIX_ARG),
            self.plain_value(sym::HELP_FORM),
            Value::fixnum(self.minibuf.minibuf_prompt_width),
            self.minibuf.minibuf_prompt,
        ];
        for value in saved {
            self.minibuf.minibuf_save_list = self.cons(value, self.minibuf.minibuf_save_list);
        }
        let keys = crate::keyboard::reader::f_this_command_keys_vector(self, &[])?;
        self.minibuf.minibuf_save_list = self.cons(keys, self.minibuf.minibuf_save_list);
        self.record_unwind(read_minibuf_unwind, Value::NIL);
        self.record_unwind(run_exit_minibuf_hook, minibuffer);
        self.minibuf.minibuf_prompt_width = 0;
        self.minibuf.minibuf_prompt = crate::fns::f_copy_sequence(self, &[prompt])?;
        self.set_plain_value(sym::MINIBUFFER_HISTORY_POSITION, histpos);
        self.set_plain_value(sym::MINIBUFFER_HISTORY_VARIABLE, histvar);
        let help_form = self.plain_value(sym::MINIBUFFER_HELP_FORM);
        self.set_plain_value(sym::HELP_FORM, help_form);
        if !self
            .plain_value(sym::MINIBUFFER_COMPLETING_FILE_NAME)
            .is_nil()
        {
            self.set_plain_value(sym::MINIBUFFER_COMPLETING_FILE_NAME, sym::LAMBDA);
        }
        let histvar_symbol = self.check_symbol(histvar)?;
        if self.find_symbol_value(histvar_symbol) == sym::UNBOUND {
            crate::data::f_set(self, &[histvar, Value::NIL])?;
        }
        let mut input_method = Value::NIL;
        let mut enable_multibyte = Value::NIL;
        if inherit_input_method {
            input_method = crate::data::f_symbol_value(self, &[sym::CURRENT_INPUT_METHOD])?;
            enable_multibyte = self.plain_value(sym::ENABLE_MULTIBYTE_CHARACTERS);
        }
        self.stack.push(input_method);
        self.set_minibuffer_mode(minibuffer, self.minibuf.minibuf_level)?;
        crate::buffer::f_set_buffer(self, &[minibuffer])?;
        let cb = self.current_buffer;
        self.buffer_local_set(cb, sym::TRUNCATE_LINES, Value::NIL);
        if ambient_dir.is_string() {
            self.buffer_local_set(cb, sym::DEFAULT_DIRECTORY, ambient_dir);
        } else {
            let live: Vec<Value> = self
                .buffers
                .iter()
                .copied()
                .filter(|&b| self.buffer_live_p(b))
                .collect();
            for buf in live {
                let dir = self.buffer_local_value(sym::DEFAULT_DIRECTORY, buf);
                if dir.is_string() {
                    self.buffer_local_set(cb, sym::DEFAULT_DIRECTORY, dir);
                    break;
                }
            }
        }
        if mini_frame != self.frames.selected_frame {
            let sf = self.frames.selected_frame;
            crate::frame::f_redirect_frame_focus(self, &[sf, mini_frame])?;
        }
        let sw = self.windows.selected_window;
        self.set_plain_value(sym::MINIBUFFER_SCROLL_WINDOW, sw);
        if self.minibuf.minibuf_level == 1 || self.windows.minibuf_window != sw {
            self.windows.minibuf_selected_window = sw;
        }
        let empty_minibuf = self.nth_minibuffer(0)?;
        self.set_minibuffer_mode(empty_minibuf, 0)?;
        let mw = self.windows.minibuf_window;
        self.set_window_buffer(mw, self.current_buffer, false, false)?;
        crate::window::f_select_window(self, &[mw, Value::NIL])?;
        let w = self.window_mut(mw);
        w.hscroll = 0;
        w.flags.suspend_auto_hscroll = false;
        let count1 = self.specpdl.len();
        let r = self.insert_minibuf_prompt(inherit_input_method, enable_multibyte);
        self.unbind_to_value(count1, r)?;
        self.minibuf.minibuf_prompt_width = self.current_column()?;
        if !initial.is_nil() {
            crate::editfns::f_insert(self, &[initial])?;
            crate::editfns::f_forward_char(self, &[Value::fixnum(pos)])?;
        }
        self.clear_message(true, true)?;
        let cb = self.current_buffer;
        self.buffer_mut(cb).keymap = map;
        if input_method.is_string()
            && !crate::data::f_fboundp(self, &[sym::ACTIVATE_INPUT_METHOD])?.is_nil()
        {
            self.funcall(sym::ACTIVATE_INPUT_METHOD, &[input_method])?;
        }
        self.run_hook(sym::MINIBUFFER_SETUP_HOOK)?;
        let cb = self.current_buffer;
        self.buffer_local_set(cb, sym::BUFFER_UNDO_LIST, Value::NIL);
        self.recursive_edit_1()?;
        let mw = self.windows.minibuf_window;
        let sf = self.frames.selected_frame;
        if self.window(mw).cursor.vpos >= 0
            && !self.noninteractive
            && self.frame(sf).output_method != crate::terminal::OutputMethod::Initial
        {
            let w = self.window_mut(mw);
            w.cursor.hpos = 0;
            w.cursor.x = 0;
            w.flags.must_be_updated_p = true;
            self.update_frame(sf, true)?;
            let f = self.window(mw).frame;
            self.flush_frame(f);
        }
        crate::buffer::f_set_buffer(self, &[minibuffer])?;
        let val = if allow_props || self.var_true(sym::MINIBUFFER_ALLOW_TEXT_PROPERTIES) {
            f_minibuffer_contents(self, &[])?
        } else {
            f_minibuffer_contents_no_properties(self, &[])?
        };
        self.stack.push(val);
        self.minibuf.last_minibuf_string = val;
        let histstring = if self.string_chars(val) != 0 {
            val
        } else if defalt.is_string() {
            defalt
        } else if defalt.is_cons() && self.xcar(defalt).is_string() {
            self.xcar(defalt)
        } else {
            Value::NIL
        };
        self.unbind_to(count)?;
        if self.frame_live_p(calling_frame) {
            let cf_mini_window = self.frame(calling_frame).minibuffer_window;
            if self.frames.selected_frame != calling_frame
                || (self.window_live_p(cf_mini_window)
                    && self.window(cf_mini_window).frame != calling_frame)
            {
                self.funcall(
                    sym::SELECT_FRAME_SET_INPUT_FOCUS,
                    &[calling_frame, Value::NIL],
                )?;
            }
        }
        if !(self.plain_value(sym::HISTORY_ADD_NEW_INPUT).is_nil() || histstring.is_nil()) {
            self.funcall(sym::ADD_TO_HISTORY, &[histvar, histstring])?;
        }
        if expflag {
            return self.string_to_object(val, defalt);
        }
        Ok(val)
    }

    fn insert_minibuf_prompt(
        &mut self,
        inherit_input_method: bool,
        enable_multibyte: Value,
    ) -> LispResult {
        self.specbind(sym::INHIBIT_READ_ONLY, sym::T)?;
        self.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
        crate::buffer::f_erase_buffer(self, &[])?;
        let flag = if inherit_input_method {
            enable_multibyte
        } else {
            sym::T
        };
        let cb = self.current_buffer;
        self.buffer_mut(cb).text.multibyte = !flag.is_nil();
        self.buffer_local_set(cb, sym::ENABLE_MULTIBYTE_CHARACTERS, flag);
        let prompt = self.minibuf.minibuf_prompt;
        crate::editfns::f_insert(self, &[prompt])?;
        let pt = self.cur().pt;
        if pt > 1 {
            let (beg, end) = (Value::fixnum(1), Value::fixnum(pt as i64));
            for prop in [sym::FRONT_STICKY, sym::REAR_NONSTICKY, sym::FIELD] {
                crate::textprop::f_put_text_property(self, &[beg, end, prop, sym::T, Value::NIL])?;
            }
            let mut list = self.plain_value(sym::MINIBUFFER_PROMPT_PROPERTIES);
            while list.is_cons() {
                let key = self.xcar(list);
                list = self.xcdr(list);
                if list.is_cons() {
                    let val = self.xcar(list);
                    list = self.xcdr(list);
                    if key == sym::FACE {
                        crate::textprop::f_add_face_text_property(
                            self,
                            &[beg, end, val, sym::T, Value::NIL],
                        )?;
                    } else {
                        crate::textprop::f_put_text_property(
                            self,
                            &[beg, end, key, val, Value::NIL],
                        )?;
                    }
                }
            }
        }
        Ok(Value::NIL)
    }

    fn pop_minibuf_save_list(&mut self) -> LispResult {
        let list = self.minibuf.minibuf_save_list;
        let head = self.car(list)?;
        self.minibuf.minibuf_save_list = self.cdr(list)?;
        Ok(head)
    }
}

#[derive(Clone, Copy)]
struct MinibufRequest {
    map: Value,
    initial: Value,
    prompt: Value,
    expflag: bool,
    histvar: Value,
    histpos: Value,
    defalt: Value,
    allow_props: bool,
    inherit_input_method: bool,
}

fn run_exit_minibuf_hook(lisp: &mut Lisp, minibuf: Value) -> LispResult<()> {
    let count = lisp.specpdl.len();
    lisp.record_unwind_current_buffer();
    if lisp.buffer_live_p(minibuf) {
        crate::buffer::f_set_buffer(lisp, &[minibuf])?;
    }
    lisp.safe_run_hooks(sym::MINIBUFFER_EXIT_HOOK)?;
    lisp.unbind_to(count)
}

fn read_minibuf_unwind(lisp: &mut Lisp, _arg: Value) -> LispResult<()> {
    let base = lisp.stack.len();
    let r = read_minibuf_unwind_1(lisp);
    lisp.stack.truncate(base);
    r
}

fn erase_expired_minibuffer(lisp: &mut Lisp) -> LispResult {
    lisp.specbind(sym::INHIBIT_READ_ONLY, sym::T)?;
    lisp.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
    let old_deactivate_mark = lisp.plain_value(sym::DEACTIVATE_MARK);
    lisp.stack.push(old_deactivate_mark);
    crate::buffer::f_erase_buffer(lisp, &[])?;
    lisp.set_plain_value(sym::DEACTIVATE_MARK, old_deactivate_mark);
    Ok(Value::NIL)
}

fn read_minibuf_unwind_1(lisp: &mut Lisp) -> LispResult<()> {
    let saved_selected_frame = lisp.frames.selected_frame;
    lisp.stack.push(saved_selected_frame);
    let expired_mb = lisp.nth_minibuffer(lisp.minibuf.minibuf_level)?;
    if expired_mb.is_nil() {
        std::process::abort();
    }
    lisp.minibuf.exp_mb_frame = Value::NIL;
    for frame in lisp.frames.list.clone() {
        let window = lisp.frame(frame).minibuffer_window;
        if lisp.window_live_p(window) {
            let w = lisp.window(window);
            if w.frame == frame && w.contents == expired_mb {
                lisp.minibuf.exp_mb_frame = frame;
                break;
            }
        }
    }
    let exp_mb_frame = lisp.minibuf.exp_mb_frame;
    if exp_mb_frame != saved_selected_frame && !exp_mb_frame.is_nil() {
        lisp.do_switch_frame(exp_mb_frame, false, sym::T)?;
    }
    crate::buffer::f_set_buffer(lisp, &[expired_mb])?;
    lisp.minibuf.minibuf_level -= 1;
    let key_vec = lisp.pop_minibuf_save_list()?;
    lisp.keyboard.this_command_key_count = lisp.vector_items(key_vec).len();
    lisp.keyboard.this_command_keys = key_vec;
    lisp.minibuf.minibuf_prompt = lisp.pop_minibuf_save_list()?;
    let width = lisp.pop_minibuf_save_list()?;
    lisp.minibuf.minibuf_prompt_width = width.as_fixnum().unwrap_or(0);
    for symbol in [
        sym::HELP_FORM,
        sym::CURRENT_PREFIX_ARG,
        sym::MINIBUFFER_HISTORY_POSITION,
        sym::MINIBUFFER_HISTORY_VARIABLE,
        sym::OVERRIDING_LOCAL_MAP,
    ] {
        let value = lisp.pop_minibuf_save_list()?;
        lisp.set_plain_value(symbol, value);
    }
    let future_mini_window = lisp.pop_minibuf_save_list()?;
    let calling_frame = lisp.pop_minibuf_save_list()?;
    let calling_window = lisp.pop_minibuf_save_list()?;
    lisp.stack
        .extend_from_slice(&[future_mini_window, calling_frame, calling_window]);
    let count = lisp.specpdl.len();
    let r = erase_expired_minibuffer(lisp);
    lisp.unbind_to_value(count, r)?;
    if lisp.minibuf.minibuf_level == 0
        || lisp.frames.selected_frame != lisp.window(future_mini_window).frame
    {
        lisp.resize_mini_window(lisp.windows.minibuf_window, false)?;
    }
    for frame1 in lisp.frames.list.clone() {
        if lisp.frame_parent_frame(frame1).is_nil()
            && lisp.get_frame_param(frame1, sym::DELETE_BEFORE).is_nil()
        {
            continue;
        }
        let val = lisp.get_frame_param(frame1, sym::MINIBUFFER_EXIT);
        if val.is_nil() {
            continue;
        }
        if val == sym::ICONIFY_FRAME {
            crate::frame::f_iconify_frame(lisp, &[frame1])?;
        } else if val == sym::DELETE_FRAME {
            crate::frame::f_delete_frame(lisp, &[frame1, Value::NIL])?;
        } else {
            crate::frame::f_make_frame_invisible(lisp, &[frame1, Value::NIL])?;
        }
    }
    lisp.call0(sym::MINIBUFFER_INACTIVE_MODE)?;
    lisp.choose_minibuf_frame();
    let mw = lisp.windows.minibuf_window;
    if lisp.window(mw).prev_buffers.is_nil() {
        if lisp.window(mw).frame != calling_frame {
            let prev = crate::window::f_previous_window(lisp, &[mw, Value::NIL, Value::NIL])?;
            if prev != mw && lisp.window(prev).frame == lisp.window(mw).frame {
                let sf = lisp.frames.selected_frame;
                crate::window::f_set_frame_selected_window(lisp, &[sf, prev, Value::NIL])?;
            }
        } else if lisp.window_live_p(calling_window) {
            crate::window::f_set_frame_selected_window(
                lisp,
                &[calling_frame, calling_window, Value::NIL],
            )?;
        }
    }
    let exp_mb_frame = lisp.minibuf.exp_mb_frame;
    if exp_mb_frame != saved_selected_frame && !exp_mb_frame.is_nil() {
        lisp.do_switch_frame(saved_selected_frame, false, sym::T)?;
    }
    Ok(())
}

fn minibuffer_unwind(lisp: &mut Lisp, _arg: Value) -> LispResult<()> {
    let exp_mb_frame = lisp.minibuf.exp_mb_frame;
    if exp_mb_frame.is_nil() || !lisp.frame_live_p(exp_mb_frame) {
        return Ok(());
    }
    let window = lisp.frame(exp_mb_frame).minibuffer_window;
    if !lisp.window_live_p(window) {
        return Ok(());
    }
    let prev_buffers = lisp.window(window).prev_buffers;
    if !prev_buffers.is_nil() {
        let entry = lisp.car(prev_buffers)?;
        let buffer = lisp.car(entry)?;
        if lisp.is_buffer(buffer) && lisp.buffer_live_p(buffer) {
            let base = lisp.stack.len();
            lisp.stack.push(entry);
            let r = (|| -> LispResult<()> {
                lisp.window_mut(window).prev_buffers = lisp.cdr(prev_buffers)?;
                lisp.set_window_buffer(window, buffer, false, false)?;
                let rest = lisp.cdr(entry)?;
                let start = lisp.car(rest)?;
                crate::window::f_set_window_start(lisp, &[window, start, Value::NIL])?;
                let rest = lisp.cdr(rest)?;
                let point = lisp.car(rest)?;
                crate::window::f_set_window_point(lisp, &[window, point])?;
                Ok(())
            })();
            lisp.stack.truncate(base);
            return r;
        }
    }
    let minibuffer = lisp.nth_minibuffer(0)?;
    lisp.set_window_buffer(window, minibuffer, false, false)
}

fn f_active_minibuffer_window(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    if lisp.minibuf.minibuf_level == 0 {
        return Ok(Value::NIL);
    }
    let level = lisp.minibuf.minibuf_level;
    let innermost = lisp.nth_minibuffer(level)?;
    assert!(
        !innermost.is_nil(),
        "no buffer for the innermost minibuffer"
    );
    for frame in lisp.frames.list.clone() {
        if !lisp.frame_live_p(frame) {
            continue;
        }
        let mw = lisp.frame(frame).minibuffer_window;
        if lisp.window_live_p(mw) && lisp.window(mw).contents == innermost {
            return Ok(mw);
        }
    }
    Ok(lisp.windows.minibuf_window)
}

fn f_set_minibuffer_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let window = args[0];
    if !lisp.is_window(window) {
        return Err(lisp.wrong_type(sym::WINDOWP, window));
    }
    if !lisp.window(window).mini {
        return Err(lisp.error("Window is not a minibuffer window"));
    }
    lisp.windows.minibuf_window = window;
    Ok(window)
}

fn f_minibuffer_innermost_command_loop_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = if args[0].is_nil() {
        lisp.current_buffer
    } else {
        args[0]
    };
    let depth = lisp.this_minibuffer_depth(buffer);
    let r = depth != 0 && lisp.minibuf_c_loop_level(depth) == lisp.minibuf.command_loop_level;
    Ok(lisp.bool(r))
}

fn f_abort_minibuffers(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let minibuf_depth = lisp.this_minibuffer_depth(Value::NIL);
    if minibuf_depth == 0 {
        return Err(lisp.error("Not in a minibuffer"));
    }
    if f_minibuffer_innermost_command_loop_p(lisp, &[Value::NIL])?.is_nil() {
        return Err(lisp.error("Not in most nested command loop"));
    }
    if minibuf_depth < lisp.minibuf.minibuf_level {
        let fmt = lisp.string("Abort %s minibuffer levels? ");
        let levels = Value::fixnum(lisp.minibuf.minibuf_level - minibuf_depth + 1);
        let question = crate::editfns::f_format(lisp, &[fmt, levels])?;
        if !lisp.funcall(sym::YES_OR_NO_P, &[question])?.is_nil() {
            lisp.funcall(sym::MINIBUFFER_QUIT_RECURSIVE_EDIT, &[levels])?;
        }
    } else {
        lisp.funcall(sym::MINIBUFFER_QUIT_RECURSIVE_EDIT, &[])?;
    }
    Ok(Value::NIL)
}

fn minibuffer_prompt_end(lisp: &mut Lisp) -> LispResult<usize> {
    let beg = lisp.cur().begv;
    if lisp
        .memq(lisp.current_buffer, lisp.minibuf.minibuffer_list)
        .is_nil()
    {
        return Ok(beg);
    }
    let begv = Value::fixnum(beg as i64);
    let (_, end) = lisp.find_field(begv, Value::NIL, Value::NIL, Value::NIL)?;
    if end == lisp.cur().zv
        && lisp
            .get_char_property(begv, sym::FIELD, Value::NIL)?
            .is_nil()
    {
        Ok(beg)
    } else {
        Ok(end)
    }
}

fn f_minibuffer_prompt_end(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(minibuffer_prompt_end(lisp)? as i64))
}

fn f_minibuffer_contents(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let prompt_end = minibuffer_prompt_end(lisp)?;
    let zv = lisp.cur().zv;
    Ok(lisp.buffer_substring_value(prompt_end, zv, true))
}

fn f_minibuffer_contents_no_properties(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let prompt_end = minibuffer_prompt_end(lisp)?;
    let zv = lisp.cur().zv;
    Ok(lisp.buffer_substring_value(prompt_end, zv, false))
}

fn f_read_from_minibuffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (prompt, initial_contents, keymap, read, hist, default_value, inherit_input_method) = (
        args[0], args[1], args[2], args[3], args[4], args[5], args[6],
    );
    if lisp.var_true(sym::INHIBIT_INTERACTION) {
        return Err(lisp.signal(sym::INHIBITED_INTERACTION, Value::NIL));
    }
    lisp.check_string(prompt)?;
    let keymap = if keymap.is_nil() {
        lisp.plain_value(sym::MINIBUFFER_LOCAL_MAP)
    } else {
        lisp.get_keymap(keymap, true, false)?
    };
    let (mut histvar, mut histpos) = if hist.is_symbol() {
        (hist, Value::NIL)
    } else {
        (lisp.car_safe(hist), lisp.cdr_safe(hist))
    };
    if histvar.is_nil() {
        histvar = sym::MINIBUFFER_HISTORY;
    }
    if histpos.is_nil() {
        histpos = Value::fixnum(0);
    }
    let allow_props = lisp.var_true(sym::MINIBUFFER_ALLOW_TEXT_PROPERTIES);
    lisp.read_minibuf(MinibufRequest {
        map: keymap,
        initial: initial_contents,
        prompt,
        expflag: !read.is_nil(),
        histvar,
        histpos,
        defalt: default_value,
        allow_props,
        inherit_input_method: !inherit_input_method.is_nil(),
    })
}

pub(crate) fn f_read_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (prompt, initial_input, history, default_value, inherit_input_method) =
        (args[0], args[1], args[2], args[3], args[4]);
    let count = lisp.specpdl.len();
    let r = (|| -> LispResult {
        lisp.specbind(sym::MINIBUFFER_COMPLETION_TABLE, Value::NIL)?;
        let mut val = f_read_from_minibuffer(
            lisp,
            &[
                prompt,
                initial_input,
                Value::NIL,
                Value::NIL,
                history,
                default_value,
                inherit_input_method,
            ],
        )?;
        if val.is_string() && lisp.string_chars(val) == 0 && !default_value.is_nil() {
            val = if default_value.is_cons() {
                lisp.xcar(default_value)
            } else {
                default_value
            };
        }
        Ok(val)
    })();
    lisp.unbind_to_value(count, r)
}

fn default_string(lisp: &Lisp, default_value: Value) -> Value {
    if default_value.is_nil() {
        Value::NIL
    } else if default_value.is_symbol() {
        lisp.symbol_name(default_value)
    } else {
        default_value
    }
}

fn f_read_command(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let default_string = default_string(lisp, args[1]);
    let obarray = lisp.plain_value(sym::OBARRAY);
    let name = f_completing_read(
        lisp,
        &[
            args[0],
            obarray,
            sym::COMMANDP,
            sym::T,
            Value::NIL,
            Value::NIL,
            default_string,
            Value::NIL,
        ],
    )?;
    if name.is_nil() {
        return Ok(name);
    }
    crate::lread::f_intern(lisp, &[name, Value::NIL])
}

pub(crate) fn f_read_variable(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let default_string = default_string(lisp, args[1]);
    let obarray = lisp.plain_value(sym::OBARRAY);
    let name = f_completing_read(
        lisp,
        &[
            args[0],
            obarray,
            sym::CUSTOM_VARIABLE_P,
            sym::T,
            Value::NIL,
            sym::CUSTOM_VARIABLE_HISTORY,
            default_string,
            Value::NIL,
        ],
    )?;
    if name.is_nil() {
        return Ok(name);
    }
    crate::lread::f_intern(lisp, &[name, Value::NIL])
}

pub(crate) fn f_read_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (mut prompt, mut def, require_match, predicate) = (args[0], args[1], args[2], args[3]);
    if lisp.is_buffer(def) {
        def = lisp.buffer(def).name;
    }
    let count = lisp.specpdl.len();
    let r = (|| -> LispResult {
        let ignore_case = lisp.var_true(sym::READ_BUFFER_COMPLETION_IGNORE_CASE);
        let ignore_case = lisp.bool(ignore_case);
        lisp.specbind(sym::COMPLETION_IGNORE_CASE, ignore_case)?;
        let function = lisp.plain_value(sym::READ_BUFFER_FUNCTION);
        if function.is_nil() {
            if !def.is_nil() {
                if prompt.is_string() {
                    let s = lisp.string_bytes(prompt).to_vec();
                    let mut len = s.len();
                    if len >= 2 && s[len - 2] == b':' && s[len - 1] == b' ' {
                        len -= 2;
                    } else if len >= 1 && (s[len - 1] == b':' || s[len - 1] == b' ') {
                        len -= 1;
                    }
                    let data = s[..len].to_vec();
                    let string = if lisp.string_multibyte(prompt) {
                        let chars = crate::character::chars_in_multibyte(&data);
                        crate::object::LispString::multibyte(data, chars)
                    } else {
                        crate::object::LispString::unibyte(data)
                    };
                    prompt = lisp.heap.string(string);
                }
                let shown = if def.is_cons() { lisp.xcar(def) } else { def };
                prompt = lisp.funcall(sym::FORMAT_PROMPT, &[prompt, shown])?;
            }
            f_completing_read(
                lisp,
                &[
                    prompt,
                    sym::INTERNAL_COMPLETE_BUFFER,
                    predicate,
                    require_match,
                    Value::NIL,
                    sym::BUFFER_NAME_HISTORY,
                    def,
                    Value::NIL,
                ],
            )
        } else if predicate.is_nil() {
            lisp.funcall(function, &[prompt, def, require_match])
        } else {
            lisp.funcall(function, &[prompt, def, require_match, predicate])
        }
    })();
    lisp.unbind_to_value(count, r)
}

fn f_minibuffer_depth(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(lisp.minibuf.minibuf_level))
}

fn f_minibuffer_prompt(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let prompt = lisp.minibuf.minibuf_prompt;
    crate::fns::f_copy_sequence(lisp, &[prompt])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TableType {
    Function,
    List,
    Obarray,
    Hash,
}

fn completion_ignore_case(lisp: &Lisp) -> Value {
    lisp.bool(lisp.var_true(sym::COMPLETION_IGNORE_CASE))
}

fn compare_strings(lisp: &mut Lisp, args: [Value; 7]) -> LispResult {
    crate::fns::f_compare_strings(lisp, &args)
}

fn table_type(lisp: &mut Lisp, collection: Value) -> TableType {
    if lisp.is_hash_table(collection) {
        TableType::Hash
    } else if lisp.is_obarray(collection) {
        TableType::Obarray
    } else if collection.is_nil() || (collection.is_cons() && !lisp.functionp(collection)) {
        TableType::List
    } else {
        TableType::Function
    }
}

fn completion_candidates(
    lisp: &mut Lisp,
    collection: Value,
    kind: TableType,
) -> Vec<(Value, Value)> {
    match kind {
        TableType::List => {
            let mut out = Vec::new();
            let mut tail = collection;
            while tail.is_cons() {
                out.push((lisp.xcar(tail), Value::NIL));
                tail = lisp.xcdr(tail);
            }
            out
        }
        TableType::Obarray => match lisp.heap.vectorlike_ref(collection) {
            crate::object::Vectorlike::Obarray(o) => {
                o.symbols.values().map(|&s| (s, Value::NIL)).collect()
            }
            _ => Vec::new(),
        },
        TableType::Hash => lisp.hash_table(collection).live_entries(),
        TableType::Function => Vec::new(),
    }
}

fn root_candidates(lisp: &mut Lisp, candidates: &[(Value, Value)]) {
    for &(k, v) in candidates {
        lisp.stack.push(k);
        lisp.stack.push(v);
    }
}

fn candidate_string(lisp: &Lisp, elt: Value, kind: TableType) -> Value {
    let eltstring = if kind == TableType::List && elt.is_cons() {
        lisp.xcar(elt)
    } else {
        elt
    };
    if eltstring.is_symbol() {
        lisp.symbol_name(eltstring)
    } else {
        eltstring
    }
}

fn match_regexps(
    lisp: &mut Lisp,
    string: Value,
    regexps: Value,
    ignore_case: bool,
) -> LispResult<bool> {
    let mut regexps = regexps;
    while regexps.is_cons() {
        let re = lisp.xcar(regexps);
        lisp.check_string(re)?;
        let table = if ignore_case {
            lisp.cur().case_canon_table
        } else {
            Value::NIL
        };
        let val = lisp.fast_string_match(re, string, table)?;
        if val == -2 {
            return Err(lisp.error("Stack overflow in regexp matcher"));
        }
        if val < 0 {
            return Ok(false);
        }
        regexps = lisp.xcdr(regexps);
    }
    Ok(true)
}

fn candidate_accepted(
    lisp: &mut Lisp,
    elt: Value,
    hash_value: Value,
    eltstring: Value,
    predicate: Value,
    kind: TableType,
) -> LispResult<bool> {
    let regexps = lisp.find_symbol_value(sym::COMPLETION_REGEXP_LIST);
    let ignore = lisp.var_true(sym::COMPLETION_IGNORE_CASE);
    if !match_regexps(lisp, eltstring, regexps, ignore)? {
        return Ok(false);
    }
    if !predicate.is_nil() {
        let tem = if predicate == sym::COMMANDP {
            crate::eval::f_commandp(lisp, &[elt, Value::NIL])?
        } else if kind == TableType::Hash {
            lisp.funcall(predicate, &[elt, hash_value])?
        } else {
            lisp.funcall(predicate, &[elt])?
        };
        if tem.is_nil() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn completion_collection(lisp: &mut Lisp, collection: Value) -> LispResult<Value> {
    if lisp.is_vector(collection) {
        lisp.check_obarray(collection)
    } else {
        Ok(collection)
    }
}

pub(crate) fn f_try_completion(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (string, predicate) = (args[0], arg(args, 2));
    let collection = completion_collection(lisp, args[1])?;
    let kind = table_type(lisp, collection);
    lisp.check_string(string)?;
    if kind == TableType::Function {
        return lisp.funcall(collection, &[string, predicate, Value::NIL]);
    }
    let base = lisp.stack.len();
    lisp.stack.push(collection);
    let r = try_completion(lisp, string, collection, predicate, kind);
    lisp.stack.truncate(base);
    r
}

fn try_completion(
    lisp: &mut Lisp,
    string: Value,
    collection: Value,
    predicate: Value,
    kind: TableType,
) -> LispResult {
    let candidates = completion_candidates(lisp, collection, kind);
    root_candidates(lisp, &candidates);
    let best_slot = lisp.stack.len();
    lisp.stack.push(Value::NIL);
    let zero = Value::fixnum(0);
    let mut bestmatchsize = 0usize;
    let mut matchcount = 0;
    let string_chars = lisp.string_chars(string);
    for (elt, hash_value) in candidates {
        let eltstring = candidate_string(lisp, elt, kind);
        if !(eltstring.is_string() && string_chars <= lisp.string_chars(eltstring)) {
            continue;
        }
        let ic = completion_ignore_case(lisp);
        let tem = compare_strings(
            lisp,
            [
                eltstring,
                zero,
                Value::fixnum(string_chars as i64),
                string,
                zero,
                Value::NIL,
                ic,
            ],
        )?;
        if tem != sym::T {
            continue;
        }
        if !candidate_accepted(lisp, elt, hash_value, eltstring, predicate, kind)? {
            continue;
        }
        let bestmatch = lisp.stack[best_slot];
        if bestmatch.is_nil() {
            matchcount = 1;
            lisp.stack[best_slot] = eltstring;
            bestmatchsize = lisp.string_chars(eltstring);
            continue;
        }
        let eltchars = lisp.string_chars(eltstring);
        let compare = bestmatchsize.min(eltchars);
        let lcompare = Value::fixnum(compare as i64);
        let ic = completion_ignore_case(lisp);
        let tem = compare_strings(
            lisp,
            [bestmatch, zero, lcompare, eltstring, zero, lcompare, ic],
        )?;
        let matchsize = if tem == sym::T {
            compare
        } else {
            tem.as_fixnum().unwrap_or(0).unsigned_abs() as usize - 1
        };
        let old_bestmatch = bestmatch;
        let ignore = lisp.var_true(sym::COMPLETION_IGNORE_CASE);
        if ignore {
            let best_chars = lisp.string_chars(bestmatch);
            let replace = if matchsize == eltchars && matchsize < best_chars {
                true
            } else if (matchsize == eltchars) == (matchsize == best_chars) {
                let len = Value::fixnum(string_chars as i64);
                let a = compare_strings(
                    lisp,
                    [eltstring, zero, len, string, zero, Value::NIL, Value::NIL],
                )?;
                a == sym::T && {
                    let b = compare_strings(
                        lisp,
                        [bestmatch, zero, len, string, zero, Value::NIL, Value::NIL],
                    )?;
                    b != sym::T
                }
            } else {
                false
            };
            if replace {
                lisp.stack[best_slot] = eltstring;
            }
        }
        let differs = bestmatchsize != eltchars
            || bestmatchsize != matchsize
            || (lisp.var_true(sym::COMPLETION_IGNORE_CASE)
                && compare_strings(
                    lisp,
                    [
                        old_bestmatch,
                        zero,
                        lcompare,
                        eltstring,
                        zero,
                        lcompare,
                        Value::NIL,
                    ],
                )? != sym::T);
        if differs && matchcount <= 1 {
            matchcount += 1;
        }
        bestmatchsize = matchsize;
        if matchsize <= string_chars
            && !lisp.var_true(sym::COMPLETION_IGNORE_CASE)
            && matchcount > 1
        {
            break;
        }
    }
    let bestmatch = lisp.stack[best_slot];
    if bestmatch.is_nil() {
        return Ok(Value::NIL);
    }
    if matchcount == 1 && lisp.equal(bestmatch, string)? {
        return Ok(sym::T);
    }
    crate::fns::f_substring(
        lisp,
        &[bestmatch, zero, Value::fixnum(bestmatchsize as i64)],
    )
}

pub(crate) fn f_all_completions(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (string, predicate) = (args[0], arg(args, 2));
    let collection = completion_collection(lisp, args[1])?;
    let kind = table_type(lisp, collection);
    lisp.check_string(string)?;
    if kind == TableType::Function {
        return lisp.funcall(collection, &[string, predicate, sym::T]);
    }
    let base = lisp.stack.len();
    lisp.stack.push(collection);
    let r = all_completions(lisp, string, collection, predicate, kind);
    lisp.stack.truncate(base);
    r
}

fn all_completions(
    lisp: &mut Lisp,
    string: Value,
    collection: Value,
    predicate: Value,
    kind: TableType,
) -> LispResult {
    let candidates = completion_candidates(lisp, collection, kind);
    root_candidates(lisp, &candidates);
    let all_slot = lisp.stack.len();
    lisp.stack.push(Value::NIL);
    let zero = Value::fixnum(0);
    let string_chars = lisp.string_chars(string);
    for (elt, hash_value) in candidates {
        let eltstring = candidate_string(lisp, elt, kind);
        if !(eltstring.is_string() && string_chars <= lisp.string_chars(eltstring)) {
            continue;
        }
        let len = Value::fixnum(string_chars as i64);
        let ic = completion_ignore_case(lisp);
        let tem = compare_strings(lisp, [eltstring, zero, len, string, zero, len, ic])?;
        if tem != sym::T {
            continue;
        }
        if !candidate_accepted(lisp, elt, hash_value, eltstring, predicate, kind)? {
            continue;
        }
        let all = lisp.stack[all_slot];
        lisp.stack[all_slot] = lisp.cons(eltstring, all);
    }
    let all = lisp.stack[all_slot];
    crate::fns::f_nreverse(lisp, &[all])
}

pub(crate) fn f_test_completion(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (string, collection, predicate) = (args[0], args[1], arg(args, 2));
    lisp.check_string(string)?;
    let base = lisp.stack.len();
    lisp.stack.push(collection);
    let r = test_completion(lisp, string, collection, predicate);
    lisp.stack.truncate(base);
    r
}

fn test_completion(
    lisp: &mut Lisp,
    string: Value,
    collection: Value,
    predicate: Value,
) -> LispResult {
    let mut tem;
    let mut hash_arg = Value::NIL;
    let hash = lisp.is_hash_table(collection);
    if collection.is_nil() || (collection.is_cons() && !lisp.functionp(collection)) {
        let ic = completion_ignore_case(lisp);
        tem = f_assoc_string(lisp, &[string, collection, ic])?;
        if tem.is_nil() {
            return Ok(Value::NIL);
        }
    } else if lisp.is_obarray(collection) || lisp.is_vector(collection) {
        let collection = lisp.check_obarray(collection)?;
        let bytes = lisp.string_bytes(string).to_vec();
        tem = lisp
            .obarray_lookup(collection, &bytes)
            .unwrap_or(Value::fixnum(0));
        if lisp.var_true(sym::COMPLETION_IGNORE_CASE) && !tem.is_symbol() {
            let symbols: Vec<Value> = match lisp.heap.vectorlike_ref(collection) {
                crate::object::Vectorlike::Obarray(o) => o.symbols.values().copied().collect(),
                _ => Vec::new(),
            };
            for obj in symbols {
                let name = lisp.symbol_name(obj);
                let zero = Value::fixnum(0);
                let r = compare_strings(
                    lisp,
                    [string, zero, Value::NIL, name, zero, Value::NIL, sym::T],
                )?;
                if r == sym::T {
                    tem = obj;
                    break;
                }
            }
        }
        if !tem.is_symbol() {
            return Ok(Value::NIL);
        }
    } else if hash {
        match lisp.hash_lookup(collection, string)? {
            Some(slot) => {
                let (k, v) =
                    lisp.hash_table(collection).entries[slot].unwrap_or((Value::NIL, Value::NIL));
                tem = k;
                hash_arg = v;
            }
            None => {
                let entries = lisp.hash_table(collection).live_entries();
                root_candidates(lisp, &entries);
                let mut found = None;
                for (k, v) in entries {
                    let strkey = if k.is_symbol() {
                        lisp.symbol_name(k)
                    } else {
                        k
                    };
                    if !strkey.is_string() {
                        continue;
                    }
                    let ic = completion_ignore_case(lisp);
                    let r = compare_strings(
                        lisp,
                        [
                            string,
                            Value::NIL,
                            Value::NIL,
                            strkey,
                            Value::NIL,
                            Value::NIL,
                            ic,
                        ],
                    )?;
                    if r == sym::T {
                        found = Some((k, v));
                        break;
                    }
                }
                let Some((k, v)) = found else {
                    return Ok(Value::NIL);
                };
                tem = k;
                hash_arg = v;
            }
        }
    } else {
        return lisp.funcall(collection, &[string, predicate, sym::LAMBDA]);
    }
    lisp.stack.push(tem);
    lisp.stack.push(hash_arg);
    let regexps = lisp.find_symbol_value(sym::COMPLETION_REGEXP_LIST);
    let ignore = lisp.var_true(sym::COMPLETION_IGNORE_CASE);
    if !match_regexps(lisp, string, regexps, ignore)? {
        return Ok(Value::NIL);
    }
    if !predicate.is_nil() {
        return if hash {
            lisp.funcall(predicate, &[tem, hash_arg])
        } else {
            lisp.funcall(predicate, &[tem])
        };
    }
    Ok(sym::T)
}

impl Lisp {
    pub fn buffer_alist(&mut self) -> Value {
        let live: Vec<Value> = self
            .buffers
            .iter()
            .copied()
            .filter(|&b| self.buffer_live_p(b))
            .collect();
        let base = self.stack.len();
        let mut cells = Vec::with_capacity(live.len());
        for b in live {
            let name = self.buffer(b).name;
            let cell = self.cons(name, b);
            self.stack.push(cell);
            cells.push(cell);
        }
        let alist = self.list(&cells);
        self.stack.truncate(base);
        alist
    }
}

fn first_byte(lisp: &Lisp, s: Value) -> u8 {
    lisp.string_bytes(s).first().copied().unwrap_or(0)
}

fn f_internal_complete_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (string, predicate, flag) = (args[0], args[1], args[2]);
    let alist = lisp.buffer_alist();
    let base = lisp.stack.len();
    lisp.stack.push(alist);
    let r = internal_complete_buffer(lisp, string, predicate, flag, alist);
    lisp.stack.truncate(base);
    r
}

fn internal_complete_buffer(
    lisp: &mut Lisp,
    string: Value,
    predicate: Value,
    flag: Value,
    alist: Value,
) -> LispResult {
    if flag.is_nil() {
        f_try_completion(lisp, &[string, alist, predicate])
    } else if flag == sym::T {
        let res = f_all_completions(lisp, &[string, alist, predicate])?;
        if lisp.string_chars(string) > 0 {
            return Ok(res);
        }
        let mut bufs = res;
        while bufs.is_cons() && first_byte(lisp, lisp.xcar(bufs)) == b' ' {
            bufs = lisp.xcdr(bufs);
        }
        if bufs.is_nil() {
            let n_res = lisp.list_to_vec(res)?.len();
            let n_all = lisp.list_to_vec(alist)?.len();
            return Ok(if n_res == n_all { res } else { bufs });
        }
        let res = bufs;
        while lisp.xcdr(bufs).is_cons() {
            let next = lisp.xcdr(bufs);
            if first_byte(lisp, lisp.xcar(next)) == b' ' {
                let after = lisp.xcdr(next);
                lisp.heap.set_cdr(bufs, after);
            } else {
                bufs = next;
            }
        }
        Ok(res)
    } else if flag == sym::LAMBDA {
        f_test_completion(lisp, &[string, alist, predicate])
    } else if flag == sym::METADATA {
        let category = lisp.cons(sym::CATEGORY, sym::BUFFER);
        lisp.stack.push(category);
        let sort = lisp.cons(sym::CYCLE_SORT_FUNCTION, sym::IDENTITY);
        lisp.stack.pop();
        Ok(lisp.list(&[sym::METADATA, category, sort]))
    } else {
        Ok(Value::NIL)
    }
}

pub(crate) fn f_assoc_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (mut key, list, case_fold) = (args[0], args[1], arg(args, 2));
    if key.is_symbol() {
        key = lisp.symbol_name(key);
    }
    let mut tail = list;
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        let mut thiscar = if elt.is_cons() { lisp.xcar(elt) } else { elt };
        if thiscar.is_symbol() {
            thiscar = lisp.symbol_name(thiscar);
        } else if !thiscar.is_string() {
            tail = lisp.xcdr(tail);
            continue;
        }
        let zero = Value::fixnum(0);
        let tem = compare_strings(
            lisp,
            [thiscar, zero, Value::NIL, key, zero, Value::NIL, case_fold],
        )?;
        if tem == sym::T {
            return Ok(elt);
        }
        tail = lisp.xcdr(tail);
    }
    Ok(Value::NIL)
}

pub(crate) fn f_completing_read(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let function = lisp.find_symbol_value(sym::COMPLETING_READ_FUNCTION);
    if function == sym::UNBOUND {
        return Err(lisp.signal_list(sym::VOID_VARIABLE, &[sym::COMPLETING_READ_FUNCTION]));
    }
    let call: Vec<Value> = (0..8).map(|i| arg(args, i)).collect();
    lisp.funcall(function, &call)
}

impl Lisp {
    pub(crate) fn buffer_downcase(&self, c: u32) -> u32 {
        let table = self.cur().downcase_table;
        if self.is_char_table(table)
            && let Some(n) = self.char_table_ref(table, c).as_fixnum()
            && n >= 0
        {
            return n as u32;
        }
        c
    }
}

fn f_completion_flex_cost_gotoh(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    const FLEX_MAX_STR_SIZE: usize = 512;
    const FLEX_MAX_PAT_SIZE: usize = 128;
    const FLEX_MAX_MATRIX_SIZE: usize = FLEX_MAX_PAT_SIZE * FLEX_MAX_STR_SIZE;
    let (pat, s) = (args[0], args[1]);
    lisp.check_string(pat)?;
    lisp.check_string(s)?;
    let patlen = lisp.string_chars(pat);
    let strlen = lisp.string_chars(s);
    let width = strlen + 1;
    let size = (patlen + 1) * width;
    let gap_open_cost: i32 = 10;
    let gap_extend_cost: i32 = 1;
    let pos_inf: i32 = i32::MAX / 2;
    if patlen == 0 || strlen == 0 || size > FLEX_MAX_MATRIX_SIZE {
        return Ok(Value::NIL);
    }
    let ignore_case = lisp.var_true(sym::COMPLETION_IGNORE_CASE);
    if !ignore_case {
        let p = lisp.string_bytes(pat);
        let sb = lisp.string_bytes(s);
        let mut pi = 0;
        let mut si = 0;
        while si < strlen && pi < patlen {
            if sb.get(si) == p.get(pi) {
                pi += 1;
            }
            si += 1;
        }
        if pi < patlen {
            return Ok(Value::NIL);
        }
    }
    let mut m = vec![pos_inf; size];
    let mut d = vec![pos_inf; size];
    for cell in d.iter_mut().take(width) {
        *cell = gap_open_cost / 2;
    }
    d[0] = 0;
    let idx = |i: isize, j: isize| ((i + 1) as usize) * width + (j + 1) as usize;
    let pat_chars = lisp.string_char_vec(pat);
    let str_chars = lisp.string_char_vec(s);
    let mut prev_match = 0usize;
    for (ix, &pat_char) in pat_chars.iter().enumerate() {
        let i = ix as isize;
        let mut match_seen = false;
        let row_start = prev_match;
        for (jx, &str_char) in str_chars.iter().enumerate().skip(row_start) {
            let j = jx as isize;
            let cmatch = if ignore_case {
                lisp.buffer_downcase(pat_char) == lisp.buffer_downcase(str_char)
            } else {
                pat_char == str_char
            };
            if cmatch {
                if !match_seen {
                    match_seen = true;
                    prev_match = jx;
                }
                m[idx(i, j)] = m[idx(i - 1, j - 1)].min(d[idx(i - 1, j - 1)]);
            }
            d[idx(i, j)] =
                (m[idx(i, j - 1)] + gap_open_cost).min(d[idx(i, j - 1)] + gap_extend_cost);
        }
    }
    let mut best_cost = pos_inf;
    let mut lastcol: isize = -1;
    for j in 0..strlen as isize {
        let cost = m[idx(patlen as isize - 1, j)];
        if cost < best_cost {
            best_cost = cost;
            lastcol = j;
        }
    }
    if lastcol < 0 || best_cost >= pos_inf {
        return Ok(Value::NIL);
    }
    let mut positions = vec![lastcol];
    let mut l = lastcol;
    for i in (0..patlen as isize - 1).rev() {
        loop {
            l -= 1;
            if !(l >= 0 && m[idx(i, l)] >= d[idx(i, l)]) {
                break;
            }
        }
        positions.push(l);
    }
    positions.reverse();
    let items: Vec<Value> = positions
        .into_iter()
        .map(|p| Value::fixnum(p as i64))
        .collect();
    let matches = lisp.list(&items);
    Ok(lisp.cons(Value::fixnum(i64::from(best_cost)), matches))
}
