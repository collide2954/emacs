use crate::character::{
    CHAR_ALT, CHAR_CTL, CHAR_HYPER, CHAR_META, CHAR_MODIFIER_MASK, CHAR_SHIFT, CHAR_SUPER,
};
use crate::doprnt::Arg;
use crate::error::{HandlerKind, LispResult};
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::terminal::OutputMethod;
use crate::timefns::{Timespec, current_timespec, timespec_sub};
use crate::value::Value;

pub mod cmdloop;
pub mod events;
pub mod keyseq;
pub mod menu;
pub mod position;
pub mod reader;

pub const UP_MODIFIER: i64 = 1;
pub const DOWN_MODIFIER: i64 = 2;
pub const DRAG_MODIFIER: i64 = 4;
pub const CLICK_MODIFIER: i64 = 8;
pub const DOUBLE_MODIFIER: i64 = 16;
pub const TRIPLE_MODIFIER: i64 = 32;
pub const ALT_MODIFIER: i64 = CHAR_ALT;
pub const SUPER_MODIFIER: i64 = CHAR_SUPER;
pub const HYPER_MODIFIER: i64 = CHAR_HYPER;
pub const SHIFT_MODIFIER: i64 = CHAR_SHIFT;
pub const CTRL_MODIFIER: i64 = CHAR_CTL;
pub const META_MODIFIER: i64 = CHAR_META;
const INTMASK: i64 = crate::value::MOST_POSITIVE_FIXNUM;

const MODIFIER_NAMES: [Option<&str>; 28] = [
    Some("up"),
    Some("down"),
    Some("drag"),
    Some("click"),
    Some("double"),
    Some("triple"),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some("alt"),
    Some("super"),
    Some("hyper"),
    Some("shift"),
    Some("control"),
    Some("meta"),
];

pub const SUBRS: &[Subr] = &[
    Subr::fixed(
        "internal-event-symbol-parse-modifiers",
        1,
        1,
        f_event_symbol_parse_modifiers,
    ),
    Subr::fixed("event-convert-list", 1, 1, f_event_convert_list),
    Subr::fixed("internal-handle-focus-in", 1, 1, f_internal_handle_focus_in),
    Subr::fixed("set-input-interrupt-mode", 1, 1, f_set_input_interrupt_mode),
    Subr::fixed("set-output-flow-control", 1, 2, f_set_output_flow_control),
    Subr::fixed("set-input-meta-mode", 1, 2, f_set_input_meta_mode),
    Subr::fixed("set-quit-char", 1, 1, f_set_quit_char),
    Subr::fixed("set-input-mode", 3, 4, f_set_input_mode),
    Subr::fixed("current-input-mode", 0, 0, f_current_input_mode),
    Subr::fixed("discard-input", 0, 0, f_discard_input),
    Subr::fixed(
        "command-error-default-function",
        3,
        3,
        f_command_error_default_function,
    ),
];

impl Lisp {
    pub fn init_keyboard(&mut self) {
        self.minibuf.command_loop_level = -1;
    }

    fn top_level_1(&mut self) -> LispResult {
        if !self.plain_value(sym::TOP_LEVEL).is_nil() {
            self.condition_case_internal(
                sym::ERROR,
                |l| l.top_level_2(),
                |l, data| l.cmd_error(data),
            )?;
        } else if !self.plain_value(sym::PURIFY_FLAG).is_nil() {
            self.message1("Bare impure Emacs (standard Lisp code not loaded)")?;
        } else {
            self.message1("Bare Emacs (standard Lisp code not loaded)")?;
        }
        Ok(Value::NIL)
    }

    fn top_level_2(&mut self) -> LispResult {
        let setup_handler = self.noninteractive;
        let first = self.handlers.len();
        if setup_handler {
            let conditions = self.list(&[sym::ERROR]);
            self.push_handler(HandlerKind::HandlerBind {
                conditions,
                handler: sym::DEBUG_EARLY__HANDLER,
                skip: 0,
            });
        }
        let top_level = self.plain_value(sym::TOP_LEVEL);
        let res = self.eval_with_lexical(top_level, sym::T);
        if setup_handler && res.is_ok() {
            self.pop_handler(first);
        }
        res
    }

    fn cmd_error(&mut self, data: Value) -> LispResult {
        let count = self.specpdl.len();
        let base = self.stack.len();
        let error_symbol = self.car_safe(data);
        let conditions = crate::fns::f_get(self, &[error_symbol, sym::ERROR_CONDITIONS])?;
        if self.memq(sym::MINIBUFFER_QUIT, conditions).is_nil() {
            self.set_plain_value(sym::EXECUTING_KBD_MACRO, Value::NIL);
        }
        self.specbind(sym::STANDARD_OUTPUT, sym::T)?;
        self.specbind(sym::STANDARD_INPUT, sym::T)?;
        self.set_kvar(Kvar::PrefixArg, Value::NIL);
        self.set_kvar(Kvar::LastPrefixArg, Value::NIL);
        self.cancel_echoing();
        let old_level = self.plain_value(sym::PRINT_LEVEL);
        let old_length = self.plain_value(sym::PRINT_LENGTH);
        self.stack.extend_from_slice(&[old_level, old_length]);
        self.set_plain_value(sym::PRINT_LEVEL, Value::fixnum(10));
        self.set_plain_value(sym::PRINT_LENGTH, Value::fixnum(10));
        self.cmd_error_internal(data, Some(""))?;
        self.set_plain_value(sym::PRINT_LEVEL, old_level);
        self.set_plain_value(sym::PRINT_LENGTH, old_length);
        self.stack.truncate(base);
        self.set_plain_value(sym::QUIT_FLAG, Value::NIL);
        self.set_plain_value(sym::INHIBIT_QUIT, Value::NIL);
        self.unbind_to(count)?;
        Ok(Value::fixnum(0))
    }

    pub fn cmd_error_internal(&mut self, data: Value, context: Option<&str>) -> LispResult<()> {
        if self.signal_quit_p(data) {
            self.signaling_function = Value::NIL;
        }
        self.set_plain_value(sym::QUIT_FLAG, Value::NIL);
        self.set_plain_value(sym::INHIBIT_QUIT, sym::T);
        let function = self.plain_value(sym::COMMAND_ERROR_FUNCTION);
        if !function.is_nil() {
            let context = match context {
                Some(context) => self.make_string(context.as_bytes()),
                None => self.empty_unibyte,
            };
            let signal = self.signaling_function;
            self.funcall(function, &[data, context, signal])?;
        }
        self.signaling_function = Value::NIL;
        Ok(())
    }
}

fn f_discard_input(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    if !lisp.kvar(Kvar::DefiningKbdMacro).is_nil() {
        crate::macros::f_cancel_kbd_macro_events(lisp, &[])?;
        lisp.end_kbd_macro();
    }
    lisp.set_plain_value(sym::UNREAD_COMMAND_EVENTS, Value::NIL);
    lisp.keyboard.input_pending = false;
    Ok(Value::NIL)
}

fn f_command_error_default_function(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (data, context, signal) = (args[0], args[1], args[2]);
    let sf = lisp.frames.selected_frame;
    let error_symbol = lisp.car_safe(data);
    let conditions = crate::fns::f_get(lisp, &[error_symbol, sym::ERROR_CONDITIONS])?;
    let is_minibuffer_quit = !lisp.memq(sym::MINIBUFFER_QUIT, conditions).is_nil();
    lisp.check_string(context)?;
    let context = lisp.string_to_rust(context);
    let initial = lisp.frame(sf).output_method == OutputMethod::Initial;
    let glyphs_initialized_p = lisp.frame(sf).flags.glyphs_initialized_p;
    if !is_minibuffer_quit && (!glyphs_initialized_p || initial || lisp.noninteractive) {
        lisp.print_error_message(data, sym::EXTERNAL_DEBUGGING_OUTPUT, Some(&context), signal)?;
        crate::print::f_terpri(lisp, &[sym::EXTERNAL_DEBUGGING_OUTPUT, Value::NIL])?;
        crate::emacs::f_kill_emacs(lisp, &[Value::fixnum(-1), Value::NIL])?;
    } else {
        lisp.clear_message(true, false)?;
        lisp.message_log_maybe_newline()?;
        if is_minibuffer_quit {
            crate::dispnew::f_ding(lisp, &[sym::T])?;
        } else {
            f_discard_input(lisp, &[])?;
            lisp.bitch_at_user()?;
        }
        lisp.print_error_message(data, sym::T, Some(&context), signal)?;
    }
    Ok(Value::NIL)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kvar {
    OverridingTerminalLocalMap,
    LastCommand,
    RealLastCommand,
    KeyboardTranslateTable,
    LastRepeatableCommand,
    PrefixArg,
    LastPrefixArg,
    KbdQueue,
    DefiningKbdMacro,
    LastKbdMacro,
    SystemKeyAlist,
    SystemKeySyms,
    WindowSystem,
    LocalFunctionKeyMap,
    InputDecodeMap,
    DefaultMinibufferFrame,
    EchoString,
    EchoPrompt,
}

pub const KVAR_COUNT: usize = Kvar::EchoPrompt as usize + 1;

#[derive(Clone, Debug)]
pub struct Kboard {
    pub vars: [Value; KVAR_COUNT],
    pub reference_count: i32,
    pub kbd_queue_has_data: bool,
    pub immediate_echo: bool,
    pub kbd_macro_buffer: Vec<Value>,
    pub kbd_macro_ptr: usize,
    pub kbd_macro_end: usize,
    pub deleted: bool,
}

impl Lisp {
    pub fn kboard_slot(&self, slot: usize) -> Value {
        self.kboards[self.current_kboard].vars[slot]
    }

    pub fn set_kboard_slot(&mut self, slot: usize, value: Value) {
        let current = self.current_kboard;
        self.kboards[current].vars[slot] = value;
    }

    pub fn kvar(&self, var: Kvar) -> Value {
        self.kboard_slot(var as usize)
    }

    pub fn set_kvar(&mut self, var: Kvar, value: Value) {
        self.set_kboard_slot(var as usize, value);
    }

    pub fn init_kboard(&mut self, window_system: Value) -> Kboard {
        let mut kb = Kboard {
            vars: [Value::NIL; KVAR_COUNT],
            reference_count: 0,
            kbd_queue_has_data: false,
            immediate_echo: false,
            kbd_macro_buffer: Vec::new(),
            kbd_macro_ptr: 0,
            kbd_macro_end: 0,
            deleted: false,
        };
        kb.vars[Kvar::WindowSystem as usize] = window_system;
        let decode = self.make_sparse_keymap(Value::NIL);
        kb.vars[Kvar::InputDecodeMap as usize] = decode;
        self.stack.push(decode);
        let local = self.make_sparse_keymap(Value::NIL);
        kb.vars[Kvar::LocalFunctionKeyMap as usize] = local;
        self.stack.push(local);
        let parent = self.plain_value(sym::FUNCTION_KEY_MAP);
        let _ = self.set_keymap_parent(local, parent);
        self.stack.truncate(self.stack.len() - 2);
        kb
    }

    pub fn allocate_kboard(&mut self, window_system: Value) -> usize {
        let kb = self.init_kboard(window_system);
        self.kboards.push(kb);
        self.kboards.len() - 1
    }

    pub fn delete_kboard(&mut self, kb: usize) {
        if self.kboards[kb].deleted {
            panic!("emacs_abort: deleting a kboard twice");
        }
        self.kboards[kb].deleted = true;
        let selected = self.frames.selected_frame;
        if kb == self.current_kboard && self.is_frame(selected) && self.frame_live_p(selected) {
            self.current_kboard = self.frame_kboard(selected).expect("frame kboard");
            self.single_kboard = false;
            if self.current_kboard == kb {
                panic!("emacs_abort: the selected frame's kboard is being deleted");
            }
        }
        let k = &mut self.kboards[kb];
        k.kbd_macro_buffer = Vec::new();
        k.vars = [Value::NIL; KVAR_COUNT];
        k.kbd_queue_has_data = false;
    }

    pub fn not_single_kboard_state(&mut self, kboard: usize) {
        if kboard == self.current_kboard {
            self.single_kboard = false;
        }
    }

    pub fn push_kboard(&mut self, k: usize) {
        self.kboard_stack.push(self.current_kboard);
        self.current_kboard = k;
    }

    pub fn pop_kboard(&mut self) {
        let p = *self.kboard_stack.last().expect("kboard stack entry");
        let found = self
            .terminals
            .list
            .iter()
            .any(|&t| self.terminal(t).kboard == Some(p));
        if found {
            self.current_kboard = p;
        } else {
            let selected = self.frames.selected_frame;
            self.current_kboard = self.frame_kboard(selected).expect("frame kboard");
            self.single_kboard = false;
        }
        self.kboard_stack.pop();
    }

    pub fn temporarily_switch_to_single_kboard(&mut self, f: Option<Value>) -> LispResult<()> {
        let was_locked = self.single_kboard;
        if was_locked {
            if let Some(f) = f
                && self.frame_kboard(f) != Some(self.current_kboard)
            {
                let id = self.terminal(self.frame(f).terminal).id;
                return Err(self.verror(
                    "Terminal %d is locked, cannot read from it",
                    &[Arg::Int(id)],
                ));
            }
            self.push_kboard(self.current_kboard);
        } else if let Some(f) = f {
            self.current_kboard = self.frame_kboard(f).expect("frame kboard");
        }
        self.single_kboard = true;
        self.record_unwind(restore_kboard_configuration, self.bool(was_locked));
        Ok(())
    }
}

pub const READABLE_EVENTS_DO_TIMERS_NOW: i32 = 1 << 0;

pub struct Keyboard {
    pub input_pending: bool,
    pub interrupt_input: bool,
    pub interrupts_deferred: bool,
    pub quit_char: i64,
    pub timers_run: u32,
    pub timer_idleness_start_time: Option<Timespec>,
    pub last_auto_save: i64,
    pub kbd_buffer: std::collections::VecDeque<crate::termhooks::KbdEvent>,
    pub internal_last_event_frame: Value,
    pub accent_key_syms: Value,
    pub func_key_syms: Value,
    pub virtual_core_keyboard_name: Value,
    pub virtual_core_pointer_name: Value,
    pub force_quit_count: i32,
    pub waiting_for_input: bool,
    pub echoing: bool,
    pub ok_to_echo_at_next_pause: Option<usize>,
    pub echo_kboard: Option<usize>,
    pub echo_message_buffer: Value,
    pub button_down_time: i64,
    pub ignore_mouse_drag_p: bool,
    pub button_down_location: Value,
    pub frame_relative_event_pos: Value,
    pub down_mouse_line_number_width: i32,
    pub last_mouse_button: i64,
    pub last_mouse_x: i64,
    pub last_mouse_y: i64,
    pub double_click_count: i64,
    pub mouse_syms: Value,
    pub wheel_syms: Value,
    pub lispy_mouse_stem: Value,
    pub getcjmp: Value,
    pub getcjmp_tag: Value,
    pub this_command_keys: Value,
    pub this_command_key_count: usize,
    pub this_single_command_key_start: usize,
    pub recent_keys: Value,
    pub recent_keys_index: usize,
    pub total_keys: usize,
    pub lossage_limit: usize,
    pub unread_switch_frame: Value,
    pub input_was_pending: bool,
    pub last_non_minibuf_size: i64,
    pub timer_last_idleness_start_time: Option<Timespec>,
    pub help_form_saved_window_configs: Value,
    pub raw_keybuf: Vec<Value>,
    pub read_key_sequence_cmd: Value,
    pub read_key_sequence_remapped: Value,
    pub num_input_events: u64,
    pub last_point_position: usize,
    pub menu: menu::MenuState,
}

impl Default for Keyboard {
    fn default() -> Self {
        Keyboard {
            input_pending: false,
            interrupt_input: false,
            interrupts_deferred: false,
            quit_char: i64::from(b'g' & 0o37),
            timers_run: 0,
            timer_idleness_start_time: None,
            last_auto_save: 0,
            kbd_buffer: std::collections::VecDeque::new(),
            internal_last_event_frame: Value::NIL,
            accent_key_syms: Value::NIL,
            func_key_syms: Value::NIL,
            virtual_core_keyboard_name: Value::NIL,
            virtual_core_pointer_name: Value::NIL,
            force_quit_count: 0,
            waiting_for_input: false,
            echoing: false,
            ok_to_echo_at_next_pause: None,
            echo_kboard: None,
            echo_message_buffer: Value::NIL,
            button_down_time: 0,
            ignore_mouse_drag_p: false,
            button_down_location: Value::NIL,
            frame_relative_event_pos: Value::NIL,
            down_mouse_line_number_width: 0,
            last_mouse_button: 0,
            last_mouse_x: 0,
            last_mouse_y: 0,
            double_click_count: 0,
            mouse_syms: Value::NIL,
            wheel_syms: Value::NIL,
            lispy_mouse_stem: Value::NIL,
            getcjmp: Value::NIL,
            getcjmp_tag: Value::NIL,
            this_command_keys: Value::NIL,
            this_command_key_count: 0,
            this_single_command_key_start: 0,
            recent_keys: Value::NIL,
            recent_keys_index: 0,
            total_keys: 0,
            lossage_limit: 3 * reader::MIN_NUM_RECENT_KEYS,
            unread_switch_frame: Value::NIL,
            input_was_pending: false,
            last_non_minibuf_size: 0,
            timer_last_idleness_start_time: None,
            help_form_saved_window_configs: Value::NIL,
            raw_keybuf: Vec::new(),
            read_key_sequence_cmd: Value::NIL,
            read_key_sequence_remapped: Value::NIL,
            num_input_events: 0,
            last_point_position: 0,
            menu: menu::MenuState::default(),
        }
    }
}

impl Keyboard {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.internal_last_event_frame,
            self.accent_key_syms,
            self.func_key_syms,
            self.virtual_core_keyboard_name,
            self.virtual_core_pointer_name,
            self.echo_message_buffer,
            self.getcjmp,
            self.getcjmp_tag,
            self.this_command_keys,
            self.recent_keys,
            self.unread_switch_frame,
            self.help_form_saved_window_configs,
            self.read_key_sequence_cmd,
            self.read_key_sequence_remapped,
            self.button_down_location,
            self.frame_relative_event_pos,
            self.mouse_syms,
            self.wheel_syms,
            self.lispy_mouse_stem,
        ]);
        self.menu.roots(out);
        out.extend_from_slice(&self.raw_keybuf);
        for event in &self.kbd_buffer {
            event.trace(out);
        }
    }
}

fn f_internal_handle_focus_in(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let event = args[0];
    let rest = lisp.cdr_safe(event);
    let frame = lisp.car_safe(rest);
    if lisp.car_safe(event) != sym::FOCUS_IN || !rest.is_cons() || !lisp.is_frame(frame) {
        return Err(lisp.error("Invalid focus-in event"));
    }
    let switching =
        frame != lisp.keyboard.internal_last_event_frame && frame != lisp.frames.selected_frame;
    lisp.keyboard.internal_last_event_frame = frame;
    if switching || !lisp.keyboard.unread_switch_frame.is_nil() {
        lisp.keyboard.unread_switch_frame = lisp.make_lispy_switch_frame(frame);
    }
    Ok(Value::NIL)
}

fn keys_of_keyboard(lisp: &mut Lisp, special: Value) {
    lisp.initial_define_lispy_key(special, "delete-frame", "handle-delete-frame");
    lisp.initial_define_lispy_key(special, "iconify-frame", "ignore");
    lisp.initial_define_lispy_key(special, "make-frame-visible", "ignore");
    lisp.initial_define_lispy_key(special, "save-session", "handle-save-session");
    lisp.initial_define_lispy_key(special, "thread-event", "thread-handle-event");
    lisp.initial_define_lispy_key(special, "file-notify", "file-notify-handle-event");
    lisp.initial_define_lispy_key(special, "config-changed-event", "ignore");
    if cfg!(windows) {
        lisp.initial_define_lispy_key(special, "language-change", "ignore");
    }
    lisp.initial_define_lispy_key(special, "focus-in", "handle-focus-in");
    lisp.initial_define_lispy_key(special, "focus-out", "handle-focus-out");
    lisp.initial_define_lispy_key(special, "move-frame", "handle-move-frame");
    lisp.initial_define_lispy_key(special, "sleep-event", "ignore");
}

pub fn f_set_input_interrupt_mode(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let new_interrupt_input = cfg!(unix) && !args[0].is_nil();
    if new_interrupt_input != lisp.keyboard.interrupt_input {
        lisp.keyboard.interrupt_input = new_interrupt_input;
    }
    Ok(Value::NIL)
}

fn f_set_output_flow_control(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.decode_tty_terminal(args[1])? {
        None => Ok(Value::NIL),
        Some(t) => match t {},
    }
}

fn f_set_input_meta_mode(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.decode_tty_terminal(args[1])? {
        None => Ok(Value::NIL),
        Some(t) => match t {},
    }
}

fn f_set_quit_char(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    match lisp.get_named_terminal(crate::term::DEV_TTY) {
        None => Ok(Value::NIL),
        Some(t) => match t {},
    }
}

fn f_set_input_mode(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (interrupt, flow, meta, quit) = (args[0], args[1], args[2], args[3]);
    f_set_input_interrupt_mode(lisp, &[interrupt])?;
    f_set_output_flow_control(lisp, &[flow, Value::NIL])?;
    f_set_input_meta_mode(lisp, &[meta, Value::NIL])?;
    if !quit.is_nil() {
        f_set_quit_char(lisp, &[quit])?;
    }
    Ok(Value::NIL)
}

fn f_current_input_mode(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let sf = lisp.frames.selected_frame;
    let interrupt = lisp.bool(lisp.keyboard.interrupt_input);
    let (flow, meta) = match lisp.frame(sf).output_method {
        crate::terminal::OutputMethod::Initial | crate::terminal::OutputMethod::Pgtk => {
            (Value::NIL, sym::T)
        }
    };
    let quit = Value::fixnum(lisp.keyboard.quit_char);
    Ok(lisp.list(&[interrupt, flow, meta, quit]))
}

impl Lisp {
    fn safe_run_hooks_error(&mut self, error: Value, hook: Value, fun: Value) -> LispResult {
        let format = self.string("Error in %s (%S): %S");
        crate::editfns::f_message(self, &[format, hook, fun, error])?;
        if hook.is_symbol() {
            let mut found = false;
            let mut newval = Value::NIL;
            let mut val = self.find_symbol_value(hook);
            while val.is_cons() {
                let item = self.xcar(val);
                if fun == item {
                    found = true;
                } else {
                    newval = self.cons(item, newval);
                }
                val = self.xcdr(val);
            }
            if found {
                let newval = crate::fns::f_nreverse(self, &[newval])?;
                self.set(hook, newval)?;
                return Ok(newval);
            }
            newval = Value::NIL;
            let mut val = match self.default_value_raw(hook)? {
                v if v == sym::UNBOUND => Value::NIL,
                v => v,
            };
            while val.is_cons() {
                let item = self.xcar(val);
                if fun == item {
                    found = true;
                } else {
                    newval = self.cons(item, newval);
                }
                val = self.xcdr(val);
            }
            if found {
                let newval = crate::fns::f_nreverse(self, &[newval])?;
                self.set_default(hook, newval)?;
                return Ok(newval);
            }
        }
        Ok(Value::NIL)
    }

    fn safe_run_hook_funcall(&mut self, args: &[Value]) -> LispResult {
        let (fun, hook) = (args[0], args[1]);
        self.condition_case_internal(
            sym::T,
            |l| l.funcall(fun, &args[2..]),
            |l, error| l.safe_run_hooks_error(error, hook, fun),
        )?;
        Ok(Value::NIL)
    }

    pub fn safe_run_hooks(&mut self, hook: Value) -> LispResult<()> {
        let count = self.specpdl.len();
        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
        let r = self.run_hook_with_args(&[hook, hook], |l, args| l.safe_run_hook_funcall(args));
        self.unbind_to_value(count, r)?;
        Ok(())
    }

    pub fn safe_run_hooks_2(&mut self, hook: Value, arg1: Value, arg2: Value) -> LispResult<()> {
        let count = self.specpdl.len();
        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
        let r = self.run_hook_with_args(&[hook, hook, arg1, arg2], |l, args| {
            l.safe_run_hook_funcall(args)
        });
        self.unbind_to_value(count, r)?;
        Ok(())
    }

    pub fn record_auto_save(&mut self) {
        self.keyboard.last_auto_save = self.int_var(sym::NUM_NONMACRO_INPUT_EVENTS);
    }

    fn decode_timer(&self, timer: Value) -> Option<Timespec> {
        if !(self.is_vector(timer) && self.vector_items(timer).len() == 10) {
            return None;
        }
        let vec = self.vector_items(timer);
        if !vec[0].is_nil() {
            return None;
        }
        if !vec[2].is_fixnum() {
            return None;
        }
        self.list4_to_timespec(vec[1], vec[2], vec[3], vec[8])
    }

    fn timer_check_2(
        &mut self,
        mut timers: Value,
        mut idle_timers: Value,
    ) -> LispResult<Option<Timespec>> {
        while self.pending_funcalls.is_cons() {
            let funcall = self.xcar(self.pending_funcalls);
            self.pending_funcalls = self.xcdr(self.pending_funcalls);
            let (function, args) = (self.xcar(funcall), self.xcdr(funcall));
            self.safe_calln(sym::APPLY, &[function, args])?;
        }
        if !(timers.is_cons() || idle_timers.is_cons()) {
            return Ok(None);
        }
        let now = current_timespec();
        let idleness_now = match self.keyboard.timer_idleness_start_time {
            Some(start) => timespec_sub(now, start),
            None => (0, 0),
        };
        loop {
            'step: {
                let mut timer = Value::NIL;
                let mut idle_timer = Value::NIL;
                let mut timer_difference = None;
                let mut idle_timer_difference = None;
                let mut timer_ripe = false;
                let mut idle_timer_ripe = false;
                if timers.is_cons() {
                    timer = self.xcar(timers);
                    let Some(timer_time) = self.decode_timer(timer) else {
                        timers = self.xcdr(timers);
                        break 'step;
                    };
                    timer_ripe = timer_time <= now;
                    timer_difference = Some(if timer_ripe {
                        timespec_sub(now, timer_time)
                    } else {
                        timespec_sub(timer_time, now)
                    });
                }
                if idle_timers.is_cons() {
                    idle_timer = self.xcar(idle_timers);
                    let Some(idle_timer_time) = self.decode_timer(idle_timer) else {
                        idle_timers = self.xcdr(idle_timers);
                        break 'step;
                    };
                    idle_timer_ripe = idle_timer_time <= idleness_now;
                    idle_timer_difference = Some(if idle_timer_ripe {
                        timespec_sub(idleness_now, idle_timer_time)
                    } else {
                        timespec_sub(idle_timer_time, idleness_now)
                    });
                }
                let ordinary_first = match (timer_difference, idle_timer_difference) {
                    (None, _) => false,
                    (Some(_), None) => true,
                    (Some(t), Some(i)) => {
                        (!idle_timer_ripe && timer_ripe)
                            || (idle_timer_ripe == timer_ripe
                                && if timer_ripe { i < t } else { t < i })
                    }
                };
                let (chosen_timer, difference, ripe) = if ordinary_first {
                    (timer, timer_difference, timer_ripe)
                } else {
                    (idle_timer, idle_timer_difference, idle_timer_ripe)
                };
                if ripe {
                    if self.vector_items(chosen_timer)[0].is_nil() {
                        let count = self.specpdl.len();
                        let old_deactivate_mark = self.bound_var(sym::DEACTIVATE_MARK);
                        self.aset_value(chosen_timer, Value::fixnum(0), sym::T)?;
                        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
                        self.funcall(sym::TIMER_EVENT_HANDLER, &[chosen_timer])?;
                        self.set_plain_value(sym::DEACTIVATE_MARK, old_deactivate_mark);
                        self.keyboard.timers_run = self.keyboard.timers_run.wrapping_add(1);
                        self.unbind_to(count)?;
                    }
                    return Ok(Some((0, 0)));
                }
                return Ok(difference);
            }
            if !(timers.is_cons() || idle_timers.is_cons()) {
                break;
            }
        }
        Ok(None)
    }

    pub fn timer_check(&mut self) -> LispResult<Option<Timespec>> {
        let tem = self.plain_value(sym::INHIBIT_QUIT);
        self.set_plain_value(sym::INHIBIT_QUIT, sym::T);
        let list = self.plain_value(sym::TIMER_LIST);
        let timers = crate::fns::f_copy_sequence(self, &[list])?;
        let base = self.stack.len();
        self.stack.push(timers);
        let idle_timers = if self.keyboard.timer_idleness_start_time.is_some() {
            let list = self.plain_value(sym::TIMER_IDLE_LIST);
            crate::fns::f_copy_sequence(self, &[list])?
        } else {
            Value::NIL
        };
        self.stack.push(idle_timers);
        self.set_plain_value(sym::INHIBIT_QUIT, tem);
        let nexttime = loop {
            match self.timer_check_2(timers, idle_timers) {
                Ok(Some((0, 0))) => {}
                r => break r,
            }
        };
        self.stack.truncate(base);
        nexttime
    }

    pub fn detect_input_pending(&mut self) -> LispResult<bool> {
        Ok(self.keyboard.input_pending || self.get_input_pending(0)?)
    }

    pub fn detect_input_pending_run_timers(&mut self, do_display: bool) -> LispResult<bool> {
        let old_timers_run = self.keyboard.timers_run;
        if !self.keyboard.input_pending {
            self.get_input_pending(READABLE_EVENTS_DO_TIMERS_NOW)?;
        }
        if old_timers_run != self.keyboard.timers_run && do_display {
            self.redisplay_preserve_echo_area(8)?;
        }
        Ok(self.keyboard.input_pending)
    }

    pub fn requeued_command_events_pending_p(&self) -> bool {
        self.plain_value(sym::UNREAD_COMMAND_EVENTS).is_cons()
    }
}

fn restore_kboard_configuration(lisp: &mut Lisp, was_locked: Value) -> LispResult<()> {
    let was_locked = !was_locked.is_nil();
    lisp.single_kboard = was_locked;
    if was_locked {
        let prev = lisp.current_kboard;
        lisp.pop_kboard();
        if lisp.single_kboard && lisp.current_kboard != prev {
            unreachable!();
        }
    }
    Ok(())
}

fn init_while_no_input_ignore_events(lisp: &mut Lisp) -> Value {
    let events = lisp.list(&[
        sym::SELECT_WINDOW,
        sym::HELP_ECHO,
        sym::MOVE_FRAME,
        sym::ICONIFY_FRAME,
        sym::MAKE_FRAME_VISIBLE,
        sym::FOCUS_IN,
        sym::FOCUS_OUT,
        sym::CONFIG_CHANGED_EVENT,
        sym::SELECTION_REQUEST,
        sym::MONITORS_CHANGED,
        sym::TOOLKIT_THEME_CHANGED,
    ]);
    let events = lisp.cons(sym::FILE_NOTIFY, events);
    let events = lisp.cons(sym::THREAD_EVENT, events);
    lisp.cons(sym::SLEEP_EVENT, events)
}

pub fn init_vars(lisp: &mut Lisp) {
    let stem = lisp.string("mouse");
    lisp.keyboard.lispy_mouse_stem = stem;
    let location = lisp.vector(vec![Value::NIL; 5]);
    lisp.keyboard.button_down_location = location;
    let mouse_syms = lisp.vector(vec![Value::NIL; 5]);
    lisp.keyboard.mouse_syms = mouse_syms;
    let wheel_syms = lisp.vector(vec![Value::NIL; events::LISPY_WHEEL_NAMES.len()]);
    lisp.keyboard.wheel_syms = wheel_syms;
    let message = lisp.string("Back to top level");
    lisp.defvar(sym::INTERNAL__TOP_LEVEL_MESSAGE, message);
    lisp.defvar_bool(sym::KILL_EMACS_ON_SIGINT, true);
    lisp.set_plain_value(sym::INPUT_METHOD_EXIT_ON_FIRST_CHAR, Value::NIL);
    lisp.set_plain_value(sym::INPUT_METHOD_USE_ECHO_AREA, Value::NIL);
    for (var, kind) in [
        (sym::MOUSE_MOVEMENT, sym::MOUSE_MOVEMENT),
        (sym::SCROLL_BAR_MOVEMENT, sym::MOUSE_MOVEMENT),
        (sym::SWITCH_FRAME, sym::SWITCH_FRAME),
        (sym::FOCUS_IN, sym::FOCUS_IN),
        (sym::FOCUS_OUT, sym::FOCUS_OUT),
        (sym::MOVE_FRAME, sym::MOVE_FRAME),
        (sym::DELETE_FRAME, sym::DELETE_FRAME),
        (sym::ICONIFY_FRAME, sym::ICONIFY_FRAME),
        (sym::MAKE_FRAME_VISIBLE, sym::MAKE_FRAME_VISIBLE),
        (sym::SELECT_WINDOW, sym::SWITCH_FRAME),
        (sym::TOUCHSCREEN_BEGIN, sym::TOUCHSCREEN),
        (sym::TOUCHSCREEN_END, sym::TOUCHSCREEN),
    ] {
        lisp.put(var, sym::EVENT_KIND, kind);
        let elements = lisp.list(&[var]);
        lisp.put(var, sym::EVENT_SYMBOL_ELEMENTS, elements);
    }
    for symbol in [
        sym::LAST_COMMAND_EVENT,
        sym::LAST_NONMENU_EVENT,
        sym::LAST_INPUT_EVENT,
        sym::UNREAD_COMMAND_EVENTS,
        sym::UNREAD_POST_INPUT_METHOD_EVENTS,
        sym::UNREAD_INPUT_METHOD_EVENTS,
    ] {
        lisp.defvar(symbol, Value::NIL);
    }
    lisp.defvar(sym::META_PREFIX_CHAR, Value::fixnum(0o33));
    lisp.defvar_kboard(sym::LAST_COMMAND, Kvar::LastCommand);
    lisp.defvar_kboard(sym::REAL_LAST_COMMAND, Kvar::RealLastCommand);
    lisp.defvar_kboard(sym::LAST_REPEATABLE_COMMAND, Kvar::LastRepeatableCommand);
    for symbol in [
        sym::THIS_COMMAND,
        sym::REAL_THIS_COMMAND,
        sym::CURRENT_MINIBUFFER_COMMAND,
        sym::THIS_COMMAND_KEYS_SHIFT_TRANSLATED,
        sym::THIS_ORIGINAL_COMMAND,
    ] {
        lisp.defvar(symbol, Value::NIL);
    }
    lisp.defvar_int(sym::AUTO_SAVE_INTERVAL, 300);
    lisp.defvar_bool(sym::AUTO_SAVE_NO_MESSAGE, false);
    lisp.defvar(sym::AUTO_SAVE_TIMEOUT, Value::fixnum(30));
    lisp.defvar(sym::ECHO_KEYSTROKES, Value::fixnum(1));
    lisp.defvar_bool(sym::ECHO_KEYSTROKES_HELP, true);
    let period = lisp.float(2.0);
    lisp.defvar(sym::POLLING_PERIOD, period);
    lisp.defvar(sym::DOUBLE_CLICK_TIME, Value::fixnum(500));
    lisp.defvar_int(sym::DOUBLE_CLICK_FUZZ, 3);
    lisp.defvar_int(sym::NUM_INPUT_KEYS, 0);
    lisp.defvar_int(sym::NUM_NONMACRO_INPUT_EVENTS, 0);
    lisp.defvar(sym::LAST_EVENT_FRAME, Value::NIL);
    lisp.defvar(sym::LAST_EVENT_DEVICE, Value::NIL);
    lisp.defvar(sym::TTY_ERASE_CHAR, Value::NIL);
    lisp.defvar(sym::HELP_CHAR, Value::fixnum((b'H' & 0o37) as i64));
    lisp.defvar(sym::HELP_EVENT_LIST, Value::NIL);
    lisp.defvar(sym::HELP_FORM, Value::NIL);
    lisp.defvar(sym::PREFIX_HELP_COMMAND, Value::NIL);
    lisp.defvar(sym::TOP_LEVEL, Value::NIL);
    lisp.heap.symbol_mut(sym::TOP_LEVEL).special = false;
    lisp.defvar_kboard(sym::KEYBOARD_TRANSLATE_TABLE, Kvar::KeyboardTranslateTable);
    lisp.defvar_bool(sym::CANNOT_SUSPEND, false);
    lisp.defvar_bool(sym::MENU_PROMPTING, true);
    lisp.defvar(sym::MENU_PROMPT_MORE_CHAR, Value::fixnum(b' ' as i64));
    lisp.defvar_int(sym::EXTRA_KEYBOARD_MODIFIERS, 0);
    lisp.defvar(sym::DEACTIVATE_MARK, Value::NIL);
    lisp.make_variable_buffer_local_builtin(sym::DEACTIVATE_MARK);
    lisp.defvar(sym::PRE_COMMAND_HOOK, Value::NIL);
    lisp.defvar(sym::POST_COMMAND_HOOK, Value::NIL);
    lisp.set_plain_value(sym::ECHO_AREA_CLEAR_HOOK, Value::NIL);
    lisp.defvar(sym::MENU_BAR_FINAL_ITEMS, Value::NIL);
    lisp.defvar(sym::TAB_BAR_SEPARATOR_IMAGE_EXPRESSION, Value::NIL);
    lisp.defvar(sym::TOOL_BAR_SEPARATOR_IMAGE_EXPRESSION, Value::NIL);
    lisp.defvar_kboard(
        sym::OVERRIDING_TERMINAL_LOCAL_MAP,
        Kvar::OverridingTerminalLocalMap,
    );
    lisp.defvar(sym::OVERRIDING_LOCAL_MAP, Value::NIL);
    lisp.defvar(sym::OVERRIDING_LOCAL_MAP_MENU_FLAG, Value::NIL);
    let special = lisp.list(&[sym::KEYMAP]);
    lisp.defvar(sym::SPECIAL_EVENT_MAP, special);
    keys_of_keyboard(lisp, special);
    lisp.defvar(sym::TRACK_MOUSE, Value::NIL);
    lisp.defvar_kboard(sym::SYSTEM_KEY_ALIST, Kvar::SystemKeyAlist);
    lisp.defvar_kboard(sym::LOCAL_FUNCTION_KEY_MAP, Kvar::LocalFunctionKeyMap);
    lisp.defvar_kboard(sym::INPUT_DECODE_MAP, Kvar::InputDecodeMap);
    let function_key_map = lisp.make_sparse_keymap(Value::NIL);
    lisp.defvar(sym::FUNCTION_KEY_MAP, function_key_map);
    let key_translation_map = lisp.make_sparse_keymap(Value::NIL);
    lisp.defvar(sym::KEY_TRANSLATION_MAP, key_translation_map);
    for symbol in [
        sym::DELAYED_WARNINGS_LIST,
        sym::TIMER_LIST,
        sym::TIMER_IDLE_LIST,
    ] {
        lisp.defvar(symbol, Value::NIL);
    }
    lisp.defvar(sym::INPUT_METHOD_FUNCTION, sym::LIST);
    for symbol in [
        sym::INPUT_METHOD_PREVIOUS_MESSAGE,
        sym::SHOW_HELP_FUNCTION,
        sym::DISABLE_POINT_ADJUSTMENT,
        sym::GLOBAL_DISABLE_POINT_ADJUSTMENT,
    ] {
        lisp.defvar(symbol, Value::NIL);
    }
    lisp.defvar(sym::MINIBUFFER_MESSAGE_TIMEOUT, Value::fixnum(2));
    lisp.defvar(sym::THROW_ON_INPUT, Value::NIL);
    lisp.defvar(
        sym::COMMAND_ERROR_FUNCTION,
        sym::COMMAND_ERROR_DEFAULT_FUNCTION,
    );
    lisp.defvar(sym::ENABLE_DISABLED_MENUS_AND_BUTTONS, Value::NIL);
    lisp.defvar(sym::SELECT_ACTIVE_REGIONS, sym::T);
    lisp.defvar(sym::SAVED_REGION_SELECTION, Value::NIL);
    let inhibit = lisp.list(&[sym::HANDLE_SWITCH_FRAME, sym::HANDLE_SELECT_WINDOW]);
    lisp.defvar(sym::SELECTION_INHIBIT_UPDATE_COMMANDS, inhibit);
    lisp.defvar(sym::DEBUG_ON_EVENT, sym::SIGUSR2);
    lisp.defvar_bool(sym::ATTEMPT_STACK_OVERFLOW_RECOVERY, true);
    lisp.defvar_bool(sym::ATTEMPT_ORDERLY_SHUTDOWN_ON_FATAL_SIGNAL, true);
    let ignore = init_while_no_input_ignore_events(lisp);
    lisp.defvar(sym::WHILE_NO_INPUT_IGNORE_EVENTS, ignore);
    lisp.defvar_bool(sym::TRANSLATE_UPPER_CASE_KEY_BINDINGS, true);
    lisp.defvar_bool(sym::INPUT_PENDING_P_FILTER_EVENTS, true);
    lisp.defvar_bool(sym::MWHEEL_COALESCE_SCROLL_EVENTS, true);
    lisp.defvar(sym::DISPLAY_MONITORS_CHANGED_FUNCTIONS, Value::NIL);
    lisp.defvar_bool(sym::INHIBIT__RECORD_CHAR, false);
    lisp.defvar_bool(sym::RECORD_ALL_KEYS, false);
    lisp.defvar(sym::POST_SELECT_REGION_HOOK, Value::NIL);
    lisp.defvar_bool(sym::DISABLE_INHIBIT_TEXT_CONVERSION, false);
    lisp.defvar(sym::CURRENT_KEY_REMAP_SEQUENCE, Value::NIL);
    lisp.defvar_bool(sym::MULTIPLE_TERMINALS_MERGE_KEYBOARDS, false);
    lisp.initial_kboard = lisp.allocate_kboard(sym::T);
    lisp.current_kboard = lisp.initial_kboard;
    lisp.keyboard.virtual_core_pointer_name = lisp.string("Virtual core pointer");
    lisp.keyboard.virtual_core_keyboard_name = lisp.string("Virtual core keyboard");
    let limit = lisp.keyboard.lossage_limit;
    lisp.keyboard.recent_keys = lisp.vector(vec![Value::NIL; limit]);
    lisp.keyboard.this_command_keys = lisp.vector(vec![Value::NIL; 40]);
    let name = lisp.string("read-char-getcjmp");
    lisp.keyboard.getcjmp_tag = lisp.make_symbol(name);
}

pub fn parse_modifiers_uncached(name: &[u8]) -> (i64, usize) {
    let mut modifiers = 0;
    let mut i = 0;
    while i + 1 < name.len() {
        let mut this_mod_end = 0;
        let mut this_mod = 0;
        let rest = &name[i..];
        match name[i] {
            b'A' => (this_mod_end, this_mod) = (i + 1, ALT_MODIFIER),
            b'C' => (this_mod_end, this_mod) = (i + 1, CTRL_MODIFIER),
            b'H' => (this_mod_end, this_mod) = (i + 1, HYPER_MODIFIER),
            b'M' => (this_mod_end, this_mod) = (i + 1, META_MODIFIER),
            b'S' => (this_mod_end, this_mod) = (i + 1, SHIFT_MODIFIER),
            b's' => (this_mod_end, this_mod) = (i + 1, SUPER_MODIFIER),
            _ => {
                for (word, bit) in [
                    ("drag", DRAG_MODIFIER),
                    ("down", DOWN_MODIFIER),
                    ("double", DOUBLE_MODIFIER),
                    ("triple", TRIPLE_MODIFIER),
                    ("up", UP_MODIFIER),
                ] {
                    if rest.len() > word.len() && rest.starts_with(word.as_bytes()) {
                        this_mod_end = i + word.len();
                        this_mod = bit;
                    }
                }
            }
        }
        if this_mod_end == 0 {
            break;
        }
        if this_mod_end >= name.len() || name[this_mod_end] != b'-' {
            break;
        }
        modifiers |= this_mod;
        i = this_mod_end + 1;
    }
    let rest = &name[i..];
    if modifiers & (DOWN_MODIFIER | DRAG_MODIFIER | DOUBLE_MODIFIER | TRIPLE_MODIFIER) == 0
        && rest.len() == 7
        && rest.starts_with(b"mouse-")
        && rest[6].is_ascii_digit()
    {
        modifiers |= CLICK_MODIFIER;
    }
    if modifiers & (DOUBLE_MODIFIER | TRIPLE_MODIFIER) == 0
        && rest.len() > 6
        && rest.starts_with(b"wheel-")
    {
        modifiers |= CLICK_MODIFIER;
    }
    (modifiers, i)
}

pub fn make_ctrl_char(c: i64) -> i64 {
    let upper = c & !0o177;
    if !(0..0x80).contains(&(c & !CHAR_MODIFIER_MASK)) || c & !0o177 & !CHAR_MODIFIER_MASK != 0 {
        return c | CTRL_MODIFIER;
    }
    let mut c = c & 0o177;
    if (0o100..0o140).contains(&c) {
        let oc = c;
        c &= !0o140;
        if (b'A' as i64..=b'Z' as i64).contains(&oc) {
            c |= SHIFT_MODIFIER;
        }
    } else if (b'a' as i64..=b'z' as i64).contains(&c) {
        c &= !0o140;
    } else if c >= b' ' as i64 {
        c |= CTRL_MODIFIER;
    }
    c | (upper & !CTRL_MODIFIER)
}

impl Lisp {
    fn lispy_modifier_list(&mut self, modifiers: i64) -> Value {
        let mut list = Value::NIL;
        for (i, name) in MODIFIER_NAMES.iter().enumerate() {
            if (1i64 << i) > modifiers {
                break;
            }
            if modifiers & (1 << i) != 0
                && let Some(name) = name
            {
                let s = self.intern(name);
                list = self.cons(s, list);
            }
        }
        list
    }

    pub fn parse_modifiers(&mut self, symbol: Value) -> LispResult<Value> {
        if let Some(n) = symbol.as_fixnum() {
            let base = Value::fixnum(n & ((1 << 22) - 1));
            let mask = Value::fixnum(n & CHAR_MODIFIER_MASK);
            return Ok(self.list(&[base, mask]));
        }
        if !symbol.is_symbol() {
            return Ok(Value::NIL);
        }
        let cached = self.get(symbol, sym::EVENT_SYMBOL_ELEMENT_MASK);
        if cached.is_cons() {
            return Ok(cached);
        }
        let name = self.symbol_name(symbol);
        let bytes = self.string_bytes(name).to_vec();
        let (modifiers, end) = parse_modifiers_uncached(&bytes);
        let base_name = String::from_utf8_lossy(&bytes[end..]).into_owned();
        let unmodified = self.intern(&base_name);
        let elements = self.list(&[unmodified, Value::fixnum(modifiers & INTMASK)]);
        self.put(symbol, sym::EVENT_SYMBOL_ELEMENT_MASK, elements);
        let mods = self.lispy_modifier_list(modifiers);
        let elts = self.cons(unmodified, mods);
        self.put(symbol, sym::EVENT_SYMBOL_ELEMENTS, elts);
        Ok(elements)
    }

    pub fn apply_modifiers(&mut self, modifiers: i64, base: Value) -> Value {
        let modifiers = modifiers & INTMASK;
        if let Some(n) = base.as_fixnum() {
            return Value::fixnum(n | modifiers);
        }
        let cache = self.get(base, sym::MODIFIER_CACHE);
        let idx = Value::fixnum(modifiers & !CLICK_MODIFIER);
        let entry = self.assq(idx, cache);
        let new_symbol = if entry.is_cons() {
            self.xcdr(entry)
        } else {
            let mut prefix = String::new();
            for (bit, text) in [
                (ALT_MODIFIER, "A-"),
                (CTRL_MODIFIER, "C-"),
                (HYPER_MODIFIER, "H-"),
                (META_MODIFIER, "M-"),
                (SHIFT_MODIFIER, "S-"),
                (SUPER_MODIFIER, "s-"),
                (DOUBLE_MODIFIER, "double-"),
                (TRIPLE_MODIFIER, "triple-"),
                (UP_MODIFIER, "up-"),
                (DOWN_MODIFIER, "down-"),
                (DRAG_MODIFIER, "drag-"),
            ] {
                if modifiers & bit != 0 {
                    prefix.push_str(text);
                }
            }
            let base_name = self.symbol_name_string(base);
            let new_symbol = self.intern(&format!("{prefix}{base_name}"));
            let entry = self.cons(idx, new_symbol);
            let cache = self.cons(entry, cache);
            self.put(base, sym::MODIFIER_CACHE, cache);
            new_symbol
        };
        if self.get(new_symbol, sym::EVENT_KIND).is_nil() {
            let kind = self.get(base, sym::EVENT_KIND);
            if !kind.is_nil() {
                self.put(new_symbol, sym::EVENT_KIND, kind);
            }
        }
        new_symbol
    }

    pub fn reorder_modifiers(&mut self, symbol: Value) -> LispResult<Value> {
        let parsed = self.parse_modifiers(symbol)?;
        let base = self.xcar(parsed);
        let mask = self.xcar(self.xcdr(parsed)).as_fixnum().unwrap_or(0);
        Ok(self.apply_modifiers(mask, base))
    }

    pub fn lucid_event_type_list_p(&self, object: Value) -> bool {
        if !object.is_cons() {
            return false;
        }
        let head = self.xcar(object);
        if [
            sym::HELP_ECHO,
            sym::VERTICAL_LINE,
            sym::MODE_LINE,
            sym::TAB_LINE,
            sym::HEADER_LINE,
        ]
        .contains(&head)
        {
            return false;
        }
        let mut tail = object;
        while tail.is_cons() {
            let elt = self.xcar(tail);
            if !(elt.is_fixnum() || elt.is_symbol()) {
                return false;
            }
            tail = self.xcdr(tail);
        }
        tail.is_nil()
    }

    pub fn event_convert_list(&mut self, event_desc: Value) -> LispResult {
        let mut base = Value::NIL;
        let mut modifiers = 0;
        let mut tail = event_desc;
        while tail.is_cons() {
            let elt = self.xcar(tail);
            let mut this = 0;
            if elt.is_symbol() && self.xcdr(tail).is_cons() {
                this = self.parse_solitary_modifier(elt);
            }
            if this != 0 {
                modifiers |= this;
            } else if !base.is_nil() {
                return Err(self.error("Two bases given in one event"));
            } else {
                base = elt;
            }
            tail = self.xcdr(tail);
        }
        if base.is_symbol() && !base.is_nil() {
            let name = self.symbol_name(base);
            if self.string_chars(name) == 1 {
                base = Value::fixnum(self.string_char_vec(name)[0] as i64);
            }
        }
        if let Some(mut b) = base.as_fixnum() {
            if modifiers & SHIFT_MODIFIER != 0 && (b'a' as i64..=b'z' as i64).contains(&b) {
                b -= 0x20;
                modifiers &= !SHIFT_MODIFIER;
            }
            if modifiers & CTRL_MODIFIER != 0 {
                return Ok(Value::fixnum(
                    (modifiers & !CTRL_MODIFIER) | make_ctrl_char(b),
                ));
            }
            return Ok(Value::fixnum(modifiers | b));
        }
        if base.is_symbol() {
            return Ok(self.apply_modifiers(modifiers, base));
        }
        Err(self.error("Invalid base event"))
    }

    pub fn parse_solitary_modifier(&self, symbol: Value) -> i64 {
        if !symbol.is_symbol() {
            return 0;
        }
        let name = self.symbol_name_string(symbol);
        match name.as_str() {
            "A" | "alt" => ALT_MODIFIER,
            "C" | "ctrl" | "control" => CTRL_MODIFIER,
            "click" => CLICK_MODIFIER,
            "H" | "hyper" => HYPER_MODIFIER,
            "M" | "meta" => META_MODIFIER,
            "S" | "shift" => SHIFT_MODIFIER,
            "s" | "super" => SUPER_MODIFIER,
            "drag" => DRAG_MODIFIER,
            "down" => DOWN_MODIFIER,
            "double" => DOUBLE_MODIFIER,
            "triple" => TRIPLE_MODIFIER,
            "up" => UP_MODIFIER,
            _ => 0,
        }
    }

    pub fn event_head(&self, event: Value) -> Value {
        if event.is_cons() {
            self.xcar(event)
        } else {
            event
        }
    }
}

fn f_event_symbol_parse_modifiers(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = args[0];
    lisp.parse_modifiers(symbol)?;
    Ok(lisp.get(symbol, sym::EVENT_SYMBOL_ELEMENTS))
}

fn f_event_convert_list(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.event_convert_list(args[0])
}
