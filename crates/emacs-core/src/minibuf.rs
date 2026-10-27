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
        ]);
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("minibufferp", 0, 2, f_minibufferp),
    Subr::fixed("innermost-minibuffer-p", 0, 1, f_innermost_minibuffer_p),
    Subr::fixed("minibuffer-depth", 0, 0, f_minibuffer_depth),
    Subr::fixed("minibuffer-prompt", 0, 0, f_minibuffer_prompt),
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
}

pub fn init_minibuf_once(lisp: &mut Lisp) {
    lisp.minibuf = Minibuf::default();
    lisp.get_minibuffer(0).expect("initial minibuffer");
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
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

    pub fn minibufferp(&self, buffer: Value, live: bool) -> bool {
        if live {
            self.live_minibuffer_p(buffer)
        } else {
            !self.memq(buffer, self.minibuf.minibuffer_list).is_nil()
        }
    }
}

fn f_minibufferp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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

fn f_minibuffer_depth(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(lisp.minibuf.minibuf_level))
}

fn f_minibuffer_prompt(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let prompt = lisp.minibuf.minibuf_prompt;
    crate::fns::f_copy_sequence(lisp, &[prompt])
}
