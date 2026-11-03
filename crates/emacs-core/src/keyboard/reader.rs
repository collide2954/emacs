use crate::character::{CHAR_CTL, CHAR_META};
use crate::error::LispResult;
use crate::keyboard::{Kvar, META_MODIFIER, make_ctrl_char};
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::timefns::{Timespec, current_timespec, dtotimespec, timespec_add};
use crate::value::Value;

pub const MIN_NUM_RECENT_KEYS: usize = 100;
pub const MAX_NUM_RECENT_KEYS: usize = 99999;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("read-char", 0, 3, f_read_char),
    Subr::fixed("read-event", 0, 3, f_read_event),
    Subr::fixed("read-char-exclusive", 0, 3, f_read_char_exclusive),
    Subr::fixed("lossage-size", 0, 1, f_lossage_size)
        .interactive("(list (read-number \"Set maximum keystrokes to: \" (lossage-size)))"),
    Subr::fixed("recent-keys", 0, 1, f_recent_keys),
    Subr::fixed("this-command-keys", 0, 0, f_this_command_keys),
    Subr::fixed("set--this-command-keys", 1, 1, f_set_this_command_keys),
    Subr::fixed("this-command-keys-vector", 0, 0, f_this_command_keys_vector),
    Subr::fixed("this-single-command-keys", 0, 0, f_this_single_command_keys),
    Subr::fixed("clear-this-command-keys", 0, 1, f_clear_this_command_keys),
    Subr::fixed("recursion-depth", 0, 0, f_recursion_depth),
];

#[derive(Clone, Copy)]
enum Step {
    Retry,
    WrongKboard,
    NonReread,
    RereadForInputMethod,
    RereadFirst,
    Exit,
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

impl Lisp {
    pub(crate) fn interactive(&self) -> bool {
        self.plain_value(sym::EXECUTING_KBD_MACRO).is_nil() && !self.noninteractive
    }

    pub(crate) fn event_head_kind(&self, event: Value) -> Value {
        let head = self.event_head(event);
        self.get(head, sym::EVENT_KIND)
    }

    pub fn echo_keystrokes_p(&self) -> bool {
        let echo = self.plain_value(sym::ECHO_KEYSTROKES);
        if echo.is_float() {
            self.heap.float_value(echo) > 0.0
        } else {
            echo.as_fixnum().is_some_and(|n| n > 0)
        }
    }

    pub fn help_char_p(&self, c: Value) -> bool {
        if c == self.plain_value(sym::HELP_CHAR) {
            return true;
        }
        let mut tail = self.plain_value(sym::HELP_EVENT_LIST);
        let mut n = 0;
        while tail.is_cons() && n < 1_000_000 {
            if self.xcar(tail) == c {
                return true;
            }
            tail = self.xcdr(tail);
            n += 1;
        }
        false
    }

    fn echo_add_key(&mut self, c: Value) -> LispResult<()> {
        let echo_string = self.kvar(Kvar::EchoString);
        let mut chars: Vec<u32> = Vec::new();
        if echo_string.is_string() && self.string_chars(echo_string) > 0 {
            chars.push(u32::from(b' '));
        }
        let c = self.event_head(c);
        if let Some(n) = c.as_fixnum() {
            chars.extend(crate::keymap::push_key_description(n));
        } else if c.is_symbol() {
            let name = self.symbol_name(c);
            chars.extend(self.string_char_vec(name));
        }
        let multibyte = chars.iter().any(|&ch| ch >= 0x80);
        let mut new_string = self.string_from_chars(&chars, multibyte);
        let base = self.stack.len();
        self.stack.push(new_string);
        if (echo_string.is_nil() || self.string_chars(echo_string) == 0) && self.help_char_p(c) {
            let str = self.string(" (Type ? for further options, C-q for quick help)");
            self.stack.push(str);
            let props = self.list(&[sym::FACE, sym::HELP_KEY_BINDING]);
            self.stack.push(props);
            self.add_text_properties(Value::fixnum(7), Value::fixnum(8), props, str, false)?;
            self.add_text_properties(Value::fixnum(30), Value::fixnum(33), props, str, false)?;
            new_string = crate::fns::f_concat(self, &[new_string, str])?;
        }
        let combined = crate::fns::f_concat(self, &[echo_string, new_string])?;
        self.stack.truncate(base);
        self.set_kvar(Kvar::EchoString, combined);
        Ok(())
    }

    pub(crate) fn echo_dash(&mut self) -> LispResult<()> {
        let echo_string = self.kvar(Kvar::EchoString);
        if echo_string.is_nil() {
            return Ok(());
        }
        let kb = self.current_kboard;
        if !self.kboards[kb].immediate_echo && self.string_chars(echo_string) == 0 {
            return Ok(());
        }
        let echo_prompt = self.kvar(Kvar::EchoPrompt);
        if echo_prompt.is_string()
            && self.string_chars(echo_prompt) == self.string_chars(echo_string)
        {
            return Ok(());
        }
        let len = self.string_chars(echo_string);
        if len > 1 {
            let prev_char =
                crate::data::f_aref(self, &[echo_string, Value::fixnum(len as i64 - 2)])?;
            let last_char =
                crate::data::f_aref(self, &[echo_string, Value::fixnum(len as i64 - 1)])?;
            let (prev_char, last_char) = (
                prev_char.as_fixnum().unwrap_or(0),
                last_char.as_fixnum().unwrap_or(0),
            );
            let help = !self.plain_value(sym::ECHO_KEYSTROKES_HELP).is_nil();
            if (last_char == i64::from(b'-') && prev_char != i64::from(b' '))
                || (help && last_char == i64::from(b')') && prev_char == i64::from(b'p'))
            {
                return Ok(());
            }
        }
        let dash = self.string("-");
        let echo_string = crate::fns::f_concat(self, &[echo_string, dash])?;
        self.set_kvar(Kvar::EchoString, echo_string);
        if !self.plain_value(sym::ECHO_KEYSTROKES_HELP).is_nil() {
            let appended = self.funcall(sym::HELP__APPEND_KEYSTROKES_HELP, &[echo_string])?;
            self.set_kvar(Kvar::EchoString, appended);
        }
        self.echo_now()
    }

    pub(crate) fn echo_update(&mut self) -> LispResult<()> {
        let kb = self.current_kboard;
        if !self.kboards[kb].immediate_echo {
            return Ok(());
        }
        let prompt = self.kvar(Kvar::EchoPrompt);
        let prefix = self.call0(sym::INTERNAL_ECHO_KEYSTROKES_PREFIX)?;
        let echo_string = if prompt.is_nil() {
            prefix
        } else if prefix.is_nil() {
            prompt
        } else {
            crate::fns::f_concat(self, &[prompt, prefix])?
        };
        self.set_kvar(Kvar::EchoString, echo_string);
        let mut i = 0;
        while i < self.keyboard.this_command_key_count {
            let c = self.vector_items(self.keyboard.this_command_keys)[i];
            if !(c.is_cons() && self.event_head_kind(c) == sym::MOUSE_MOVEMENT) {
                self.echo_add_key(c)?;
            }
            i += 1;
        }
        self.echo_now()
    }

    pub(crate) fn echo_now(&mut self) -> LispResult<()> {
        let kb = self.current_kboard;
        if !self.kboards[kb].immediate_echo {
            self.kboards[kb].immediate_echo = true;
            self.echo_update()?;
            self.echo_dash()?;
        }
        self.keyboard.echoing = true;
        let echo_string = self.kvar(Kvar::EchoString);
        let r = self.message3_nolog(echo_string);
        self.keyboard.echoing = false;
        r?;
        self.keyboard.echo_message_buffer = self.xdisp.echo_area_buffer[0];
        self.keyboard.echo_kboard = Some(self.current_kboard);
        if self.keyboard.waiting_for_input && !self.plain_value(sym::QUIT_FLAG).is_nil() {
            self.quit_throw_to_read_char(false)?;
        }
        Ok(())
    }

    pub fn echo_length(&self) -> usize {
        let echo_string = self.kvar(Kvar::EchoString);
        if echo_string.is_string() {
            self.string_chars(echo_string)
        } else {
            0
        }
    }

    pub fn echo_truncate(&mut self, nchars: usize) -> LispResult<()> {
        let es = self.kvar(Kvar::EchoString);
        if es.is_string() && self.string_chars(es) > nchars {
            let truncated = crate::fns::f_substring(
                self,
                &[es, Value::fixnum(0), Value::fixnum(nchars as i64)],
            )?;
            self.set_kvar(Kvar::EchoString, truncated);
        }
        self.truncate_echo_area(nchars);
        Ok(())
    }

    pub fn add_command_key(&mut self, key: Value) {
        let keys = self.keyboard.this_command_keys;
        let count = self.keyboard.this_command_key_count;
        if count >= self.vector_items(keys).len() {
            let mut items = self.vector_items(keys).to_vec();
            items.resize(items.len() * 2 + 1, Value::NIL);
            self.keyboard.this_command_keys = self.vector(items);
        }
        if let crate::object::Vectorlike::Vector(items) =
            self.heap.vectorlike_mut(self.keyboard.this_command_keys)
        {
            items[count] = key;
        }
        self.keyboard.this_command_key_count += 1;
    }

    fn timer_start_idle(&mut self) -> LispResult<()> {
        if self.keyboard.timer_idleness_start_time.is_some() {
            return Ok(());
        }
        let now = current_timespec();
        self.keyboard.timer_idleness_start_time = Some(now);
        self.keyboard.timer_last_idleness_start_time = Some(now);
        self.call0(sym::INTERNAL_TIMER_START_IDLE)?;
        Ok(())
    }

    fn timer_stop_idle(&mut self) {
        self.keyboard.timer_idleness_start_time = None;
    }

    pub(crate) fn timer_resume_idle(&mut self) {
        if self.keyboard.timer_idleness_start_time.is_some() {
            return;
        }
        self.keyboard.timer_idleness_start_time = self.keyboard.timer_last_idleness_start_time;
    }

    fn recent_key(&self, i: usize) -> Value {
        self.vector_items(self.keyboard.recent_keys)[i]
    }

    pub(super) fn set_recent_key(&mut self, i: usize, value: Value) {
        if let crate::object::Vectorlike::Vector(items) =
            self.heap.vectorlike_mut(self.keyboard.recent_keys)
        {
            items[i] = value;
        }
    }

    pub fn record_char(&mut self, c: Value) -> LispResult<()> {
        if self.plain_value(sym::RECORD_ALL_KEYS).is_nil()
            && !self.plain_value(sym::INHIBIT__RECORD_CHAR).is_nil()
        {
            return Ok(());
        }
        let mut recorded: i32 = 0;
        let limit = self.keyboard.lossage_limit;
        if c.is_cons() && (self.xcar(c) == sym::HELP_ECHO || self.xcar(c) == sym::MOUSE_MOVEMENT) {
            let ix1 = (self.keyboard.recent_keys_index + limit - 1) % limit;
            let ix2 = (ix1 + limit - 1) % limit;
            let ix3 = (ix2 + limit - 1) % limit;
            let (ev1, ev2, ev3) = (
                self.recent_key(ix1),
                self.recent_key(ix2),
                self.recent_key(ix3),
            );
            let help_of = |l: &Lisp, ev: Value| {
                let rest = l.cdr_safe(l.xcdr(ev));
                l.car_safe(rest)
            };
            let is = |l: &Lisp, ev: Value, head: Value| ev.is_cons() && l.xcar(ev) == head;
            if self.xcar(c) == sym::HELP_ECHO {
                let help = help_of(self, c);
                if !help.is_string()
                    || (is(self, ev1, sym::HELP_ECHO) && help_of(self, ev1) == help)
                {
                    recorded = 1;
                } else if is(self, ev1, sym::MOUSE_MOVEMENT)
                    && is(self, ev2, sym::HELP_ECHO)
                    && help_of(self, ev2) == help
                {
                    recorded = -1;
                } else if is(self, ev1, sym::MOUSE_MOVEMENT)
                    && is(self, ev2, sym::MOUSE_MOVEMENT)
                    && is(self, ev3, sym::HELP_ECHO)
                    && help_of(self, ev3) == help
                {
                    recorded = -2;
                }
            } else {
                let window_of = |l: &Lisp, ev: Value| {
                    let first = l.car_safe(l.xcdr(ev));
                    l.car_safe(first)
                };
                let window = window_of(self, c);
                if is(self, ev1, sym::MOUSE_MOVEMENT)
                    && window_of(self, ev1) == window
                    && is(self, ev2, sym::MOUSE_MOVEMENT)
                    && window_of(self, ev2) == window
                {
                    self.set_recent_key(ix1, c);
                    recorded = 1;
                }
            }
        } else if self.plain_value(sym::EXECUTING_KBD_MACRO).is_nil() {
            self.store_kbd_macro_char(c);
        }
        if self.plain_value(sym::EXECUTING_KBD_MACRO).is_nil() {
            if recorded == 0 {
                if self.keyboard.total_keys < limit {
                    self.keyboard.total_keys += 1;
                }
                let copy = if c.is_cons() {
                    crate::fns::f_copy_sequence(self, &[c])?
                } else {
                    c
                };
                self.set_recent_key(self.keyboard.recent_keys_index, copy);
                self.keyboard.recent_keys_index += 1;
                if self.keyboard.recent_keys_index >= limit {
                    self.keyboard.recent_keys_index = 0;
                }
            } else if recorded < 0 {
                while recorded < 0 && self.keyboard.total_keys > 0 {
                    recorded += 1;
                    if self.keyboard.total_keys < limit {
                        self.keyboard.total_keys -= 1;
                    }
                    self.keyboard.recent_keys_index =
                        (self.keyboard.recent_keys_index + limit - 1) % limit;
                    self.set_recent_key(self.keyboard.recent_keys_index, Value::NIL);
                }
            }
            let n = self.int_var(sym::NUM_NONMACRO_INPUT_EVENTS);
            self.set_forwarded_int(sym::NUM_NONMACRO_INPUT_EVENTS, n + 1);
        }
        Ok(())
    }

    pub fn show_help_echo(
        &mut self,
        help: Value,
        window: Value,
        object: Value,
        pos: Value,
    ) -> LispResult<()> {
        let mut help = help;
        let base = self.stack.len();
        self.stack.extend_from_slice(&[help, window, object, pos]);
        let r = (|| -> LispResult<()> {
            if !help.is_nil() && !help.is_string() {
                help = if self.functionp(help) {
                    self.safe_calln(help, &[window, object, pos])?
                } else {
                    self.safe_calln(sym::EVAL, &[help, sym::T])?
                };
                if !help.is_string() {
                    return Ok(());
                }
            }
            if !self.noninteractive && help.is_string() {
                let f = self.some_mouse_moved();
                help = self.funcall(sym::MOUSE_FIXUP_HELP_MESSAGE, &[help])?;
                if let Some(f) = f {
                    self.frame_mut(f).flags.mouse_moved = true;
                }
            }
            if help.is_string() || help.is_nil() {
                let show = self.plain_value(sym::SHOW_HELP_FUNCTION);
                if !show.is_nil() {
                    self.stack.push(help);
                    let inhibit = help.is_string()
                        && self.string_chars(help) > 0
                        && !crate::textprop::f_get_text_property(
                            self,
                            &[Value::fixnum(0), sym::HELP_ECHO_INHIBIT_SUBSTITUTION, help],
                        )?
                        .is_nil();
                    let substituted = if inhibit {
                        help
                    } else {
                        self.funcall(sym::SUBSTITUTE_COMMAND_KEYS, &[help])?
                    };
                    self.safe_calln(show, &[substituted])?;
                }
                self.xdisp.help_echo_showing_p = help.is_string();
            }
            Ok(())
        })();
        self.stack.truncate(base);
        r
    }

    fn kbd_queue_append(&mut self, kb: usize, c: Value) {
        let slot = Kvar::KbdQueue as usize;
        let mut last = self.kboards[kb].vars[slot];
        if last.is_cons() {
            while self.xcdr(last).is_cons() {
                last = self.xcdr(last);
            }
            if !self.xcdr(last).is_nil() {
                panic!("emacs_abort: improper kbd_queue");
            }
        }
        let cell = self.list(&[c]);
        if last.is_cons() {
            self.heap.set_cdr(last, cell);
        } else {
            self.kboards[kb].vars[slot] = cell;
        }
        self.kboards[kb].kbd_queue_has_data = true;
    }

    fn with_getcjmp(
        &mut self,
        f: impl FnOnce(&mut Lisp) -> LispResult,
    ) -> LispResult<Option<Value>> {
        let tag = self.keyboard.getcjmp_tag;
        let saved = self.keyboard.getcjmp;
        self.keyboard.getcjmp = tag;
        let r = self.internal_catch(tag, f);
        self.keyboard.getcjmp = saved;
        let v = r?;
        Ok(if v == tag { None } else { Some(v) })
    }

    fn read_char_quit(&mut self, cs: usize) -> LispResult<Option<Value>> {
        let mut c = self.stack[cs];
        if self.plain_value(sym::QUIT_FLAG) != self.plain_value(sym::THROW_ON_INPUT) {
            c = Value::fixnum(self.keyboard.quit_char);
        }
        self.stack[cs] = c;
        let selected = self.frames.selected_frame;
        self.keyboard.internal_last_event_frame = selected;
        self.set_plain_value(sym::LAST_EVENT_FRAME, selected);
        if !self.plain_value(sym::INHIBIT_QUIT).is_nil() {
            self.set_plain_value(sym::QUIT_FLAG, Value::NIL);
        }
        let kb = self.frame_kboard(selected).expect("frame kboard");
        if kb != self.current_kboard {
            if self.single_kboard {
                panic!("emacs_abort: quit on another kboard in single-kboard mode");
            }
            self.kbd_queue_append(kb, c);
            self.current_kboard = kb;
            return Ok(Some(Value::fixnum(-2)));
        }
        Ok(None)
    }

    fn read_event_from_main_queue(
        &mut self,
        end_time: Option<Timespec>,
        mut used_mouse_menu: Option<&mut bool>,
    ) -> LispResult {
        loop {
            if let Some(end) = end_time
                && end <= current_timespec()
            {
                return Ok(Value::NIL);
            }
            if end_time.is_none() {
                self.timer_start_idle()?;
            }
            let (mut c, kb, frame) =
                self.kbd_buffer_get_event(used_mouse_menu.as_deref_mut(), end_time)?;
            if !c.is_nil() && kb != self.current_kboard {
                self.kbd_queue_append(kb, c);
                if self.single_kboard {
                    if !frame.is_nil()
                        && (c.is_fixnum()
                            || (c.is_cons() && self.event_head_kind(c) == sym::MOUSE_CLICK))
                    {
                        unreachable!("a locked frame's message needs message3_frame");
                    }
                    continue;
                }
                self.current_kboard = kb;
                return Ok(Value::fixnum(-2));
            }
            if self.noninteractive && c.as_fixnum().is_some_and(|n| n < 0) {
                crate::emacs::f_kill_emacs(self, &[Value::fixnum(1), Value::NIL])?;
            }
            if let Some(n) = c.as_fixnum() {
                let extra = self.int_var(sym::EXTRA_KEYBOARD_MODIFIERS);
                let mut n = n;
                if extra & CHAR_CTL != 0 || ((extra & 0o177) < 0o40 && (extra & 0o177) != 0) {
                    n = make_ctrl_char(n);
                }
                n |= extra & !0xff7f & !CHAR_CTL;
                c = Value::fixnum(n);
            }
            return Ok(c);
        }
    }

    fn read_decoded_event_from_main_queue(
        &mut self,
        end_time: Option<Timespec>,
        used_mouse_menu: Option<&mut bool>,
    ) -> LispResult {
        self.read_event_from_main_queue(end_time, used_mouse_menu)
    }

    fn read_char_minibuf_menu_prompt(&mut self, _commandflag: i32, map: Value) -> LispResult {
        if self.plain_value(sym::MENU_PROMPTING).is_nil() {
            return Ok(Value::NIL);
        }
        let map = self.get_keymap(map, false, true)?;
        let name = crate::keymap::f_keymap_prompt(self, &[map])?;
        if !name.is_string() {
            return Ok(Value::NIL);
        }
        unreachable!("menu prompting in the minibuffer needs read_char_minibuf_menu_prompt")
    }

    pub fn read_char(
        &mut self,
        commandflag: i32,
        map: Value,
        prev_event: Value,
        used_mouse_menu: Option<&mut bool>,
        end_time: Option<Timespec>,
    ) -> LispResult {
        let base = self.stack.len();
        self.stack
            .extend_from_slice(&[Value::NIL, Value::NIL, Value::NIL, map, prev_event]);
        let r = self.read_char_1(
            commandflag,
            map,
            prev_event,
            used_mouse_menu,
            end_time,
            base,
        );
        self.stack.truncate(base);
        r
    }

    fn read_char_1(
        &mut self,
        commandflag: i32,
        map: Value,
        prev_event: Value,
        mut used_mouse_menu: Option<&mut bool>,
        end_time: Option<Timespec>,
        cs: usize,
    ) -> LispResult {
        let also_slot = cs + 1;
        let prev_msg_slot = cs + 2;
        let orig_kboard = self.current_kboard;
        let mut reread = false;
        let mut recorded = false;
        let mut step = Step::Retry;
        loop {
            match step {
                Step::Retry => {
                    recorded = false;
                    let post = self.plain_value(sym::UNREAD_POST_INPUT_METHOD_EVENTS);
                    if post.is_cons() {
                        let mut c = self.xcar(post);
                        let rest = self.xcdr(post);
                        self.set_plain_value(sym::UNREAD_POST_INPUT_METHOD_EVENTS, rest);
                        if c.is_cons()
                            && (self.xcar(c).is_symbol() || self.xcar(c).is_fixnum())
                            && self.xcdr(c).is_nil()
                        {
                            c = self.xcar(c);
                        }
                        self.stack[cs] = c;
                        reread = true;
                        step = Step::RereadFirst;
                        continue;
                    }
                    reread = false;
                    self.set_plain_value(sym::LAST_EVENT_DEVICE, Value::NIL);
                    let unread = self.plain_value(sym::UNREAD_COMMAND_EVENTS);
                    if unread.is_cons() {
                        let mut was_disabled = false;
                        let mut c = self.xcar(unread);
                        let rest = self.xcdr(unread);
                        self.set_plain_value(sym::UNREAD_COMMAND_EVENTS, rest);
                        if c.is_cons() && self.xcar(c) == sym::T {
                            c = self.xcdr(c);
                        } else {
                            if c.is_cons() && self.xcar(c) == sym::NO_RECORD {
                                c = self.xcdr(c);
                                recorded = true;
                            }
                            reread = true;
                        }
                        if c.is_cons()
                            && self.xcdr(c) == sym::DISABLED
                            && (self.xcar(c).is_symbol() || self.xcar(c).is_fixnum())
                        {
                            was_disabled = true;
                            c = self.xcar(c);
                        }
                        if let Some(used) = used_mouse_menu.as_deref_mut()
                            && (c == sym::TOOL_BAR
                                || c == sym::TAB_BAR
                                || c == sym::MENU_BAR
                                || was_disabled)
                        {
                            *used = true;
                        }
                        self.stack[cs] = c;
                        step = Step::RereadForInputMethod;
                        continue;
                    }
                    let im = self.plain_value(sym::UNREAD_INPUT_METHOD_EVENTS);
                    if im.is_cons() {
                        let mut c = self.xcar(im);
                        let rest = self.xcdr(im);
                        self.set_plain_value(sym::UNREAD_INPUT_METHOD_EVENTS, rest);
                        if c.is_cons()
                            && (self.xcar(c).is_symbol() || self.xcar(c).is_fixnum())
                            && self.xcdr(c).is_nil()
                        {
                            c = self.xcar(c);
                        }
                        self.stack[cs] = c;
                        reread = true;
                        step = Step::RereadForInputMethod;
                        continue;
                    }
                    let kbd_macro = self.plain_value(sym::EXECUTING_KBD_MACRO);
                    if !kbd_macro.is_nil() && !self.at_end_of_macro_p()? {
                        self.keyboard.internal_last_event_frame = sym::MACRO;
                        self.set_plain_value(sym::LAST_EVENT_FRAME, sym::MACRO);
                        let index = self.int_var(sym::EXECUTING_KBD_MACRO_INDEX);
                        let mut c = crate::data::f_aref(self, &[kbd_macro, Value::fixnum(index)])?;
                        if kbd_macro.is_string()
                            && let Some(n) = c.as_fixnum()
                            && n & 0x80 != 0
                            && n <= 0xff
                        {
                            c = Value::fixnum(CHAR_META | (n & !0x80));
                        }
                        self.set_forwarded_int(sym::EXECUTING_KBD_MACRO_INDEX, index + 1);
                        self.stack[cs] = c;
                        step = Step::RereadForInputMethod;
                        continue;
                    }
                    if !self.keyboard.unread_switch_frame.is_nil() {
                        self.stack[cs] = self.keyboard.unread_switch_frame;
                        self.keyboard.unread_switch_frame = Value::NIL;
                        step = Step::RereadFirst;
                        continue;
                    }
                    if commandflag >= 0 {
                        let echo_current =
                            self.keyboard.echo_message_buffer == self.xdisp.echo_area_buffer[0];
                        if self.keyboard.input_pending || self.detect_input_pending_run_timers()? {
                            self.swallow_events()?;
                        }
                        while !(self.keyboard.input_pending && self.keyboard.input_was_pending) {
                            self.keyboard.input_was_pending = self.keyboard.input_pending;
                            if self.xdisp.help_echo_showing_p
                                && self.windows.selected_window != self.windows.minibuf_window
                            {
                                self.redisplay_preserve_echo_area(5)?;
                            } else {
                                self.redisplay()?;
                            }
                            if !self.keyboard.input_pending {
                                break;
                            }
                            self.swallow_events()?;
                        }
                        if commandflag == 0 && echo_current {
                            self.keyboard.echo_message_buffer = self.xdisp.echo_area_buffer[0];
                        }
                    }
                    if !self.xdisp.echo_area_buffer[0].is_nil()
                        && (self.keyboard.echo_kboard != Some(self.current_kboard)
                            || self.keyboard.ok_to_echo_at_next_pause.is_none())
                    {
                        self.cancel_echoing();
                    } else {
                        self.echo_dash()?;
                    }
                    self.stack[cs] = Value::NIL;
                    if self.keymapp(map)
                        && self.interactive()
                        && !prev_event.is_nil()
                        && !prev_event.is_cons()
                        && !self.plain_value(sym::UNREAD_COMMAND_EVENTS).is_cons()
                        && !self.detect_input_pending_run_timers()?
                    {
                        let c = self.read_char_minibuf_menu_prompt(commandflag, map)?;
                        if c == Value::fixnum(-2) {
                            return Ok(c);
                        }
                        if !c.is_nil() {
                            self.stack[cs] = c;
                            step = Step::Exit;
                            continue;
                        }
                    }
                    if end_time.is_none() {
                        self.timer_start_idle()?;
                    }
                    let kb = self.current_kboard;
                    if self.minibuf.minibuf_level == 0
                        && end_time.is_none()
                        && !self.kboards[kb].immediate_echo
                        && (self.keyboard.this_command_key_count > 0
                            || !self.call0(sym::INTERNAL_ECHO_KEYSTROKES_PREFIX)?.is_nil())
                        && !self.noninteractive
                        && self.echo_keystrokes_p()
                        && self.echo_area_empty_or_ours()
                    {
                        if prev_event.is_cons() {
                            self.echo_now()?;
                        } else {
                            let echo = self.plain_value(sym::ECHO_KEYSTROKES);
                            match self.with_getcjmp(|l| l.sit_for(echo, true))? {
                                None => {
                                    if let Some(v) = self.read_char_quit(cs)? {
                                        return Ok(v);
                                    }
                                    step = Step::NonReread;
                                    continue;
                                }
                                Some(tem0) => {
                                    if tem0 == sym::T
                                        && !self.plain_value(sym::UNREAD_COMMAND_EVENTS).is_cons()
                                    {
                                        self.echo_now()?;
                                    }
                                }
                            }
                        }
                    }
                    let auto_save_interval = self.int_var(sym::AUTO_SAVE_INTERVAL);
                    if commandflag != 0
                        && commandflag != -2
                        && auto_save_interval > 0
                        && self.int_var(sym::NUM_NONMACRO_INPUT_EVENTS)
                            - self.keyboard.last_auto_save
                            > auto_save_interval.max(20)
                        && !self.detect_input_pending_run_timers()?
                    {
                        let no_message =
                            self.bool(!self.plain_value(sym::AUTO_SAVE_NO_MESSAGE).is_nil());
                        crate::fileio::f_do_auto_save(self, &[no_message, Value::NIL])?;
                        self.redisplay()?;
                    }
                    if self.keymapp(map)
                        && self.interactive()
                        && prev_event.is_cons()
                        && !matches!(self.xcar(prev_event), h if h == sym::MENU_BAR || h == sym::TAB_BAR || h == sym::TOOL_BAR)
                        && !self.plain_value(sym::UNREAD_COMMAND_EVENTS).is_cons()
                    {
                        unreachable!("mouse menus need read_char_x_menu_prompt");
                    }
                    if self.interactive() && self.stack[cs].is_nil() {
                        let sw = self.windows.selected_window;
                        if !self.window(sw).mini {
                            self.keyboard.last_non_minibuf_size = self.cur().z() as i64 - 1;
                        }
                        let mut buffer_size = (self.keyboard.last_non_minibuf_size >> 8) + 1;
                        let mut delay_level = 0;
                        while buffer_size > 64 {
                            delay_level += 1;
                            buffer_size -= buffer_size >> 2;
                        }
                        if delay_level < 4 {
                            delay_level = 4;
                        }
                        let timeout = self.plain_value(sym::AUTO_SAVE_TIMEOUT);
                        if commandflag != 0
                            && commandflag != -2
                            && self.int_var(sym::NUM_NONMACRO_INPUT_EVENTS)
                                > self.keyboard.last_auto_save
                            && timeout.as_fixnum().is_some_and(|n| n > 0)
                        {
                            let mut timeout = timeout.as_fixnum().unwrap_or(0);
                            timeout =
                                timeout.min(crate::value::MOST_POSITIVE_FIXNUM / delay_level * 4);
                            timeout = delay_level * timeout / 4;
                            match self.with_getcjmp(|l| l.sit_for(Value::fixnum(timeout), true))? {
                                None => {
                                    if let Some(v) = self.read_char_quit(cs)? {
                                        return Ok(v);
                                    }
                                    step = Step::NonReread;
                                    continue;
                                }
                                Some(tem0) => {
                                    if tem0 == sym::T
                                        && !self.plain_value(sym::UNREAD_COMMAND_EVENTS).is_cons()
                                    {
                                        let no_message = self.bool(
                                            !self.plain_value(sym::AUTO_SAVE_NO_MESSAGE).is_nil(),
                                        );
                                        crate::fileio::f_do_auto_save(
                                            self,
                                            &[no_message, Value::NIL],
                                        )?;
                                        self.redisplay()?;
                                    }
                                }
                            }
                        }
                        if !self.detect_input_pending_run_timers()? {
                            self.maybe_gc();
                        }
                    }
                    if self.stack[cs].is_nil() && self.current_kboard != orig_kboard {
                        return Ok(Value::fixnum(-2));
                    }
                    let unread = self.plain_value(sym::UNREAD_COMMAND_EVENTS);
                    if unread.is_cons() {
                        let mut c = self.xcar(unread);
                        let rest = self.xcdr(unread);
                        self.set_plain_value(sym::UNREAD_COMMAND_EVENTS, rest);
                        if c.is_cons() && self.xcar(c) == sym::T {
                            c = self.xcdr(c);
                        } else {
                            if c.is_cons() && self.xcar(c) == sym::NO_RECORD {
                                c = self.xcdr(c);
                                recorded = true;
                            }
                            reread = true;
                        }
                        self.stack[cs] = c;
                    }
                    if self.stack[cs].is_nil() {
                        let kb = self.current_kboard;
                        if self.kboards[kb].kbd_queue_has_data {
                            let queue = self.kvar(Kvar::KbdQueue);
                            if !queue.is_cons() {
                                panic!("emacs_abort: kbd_queue_has_data without a queue");
                            }
                            let c = self.xcar(queue);
                            self.stack[cs] = c;
                            let rest = self.xcdr(queue);
                            self.set_kvar(Kvar::KbdQueue, rest);
                            if rest.is_nil() {
                                self.kboards[kb].kbd_queue_has_data = false;
                            }
                            self.keyboard.input_pending = self.readable_events(0)?;
                            if c.is_cons() && self.event_head_kind(c) == sym::SWITCH_FRAME {
                                self.keyboard.internal_last_event_frame = self.xcar(self.xcdr(c));
                            }
                            let last = self.keyboard.internal_last_event_frame;
                            self.set_plain_value(sym::LAST_EVENT_FRAME, last);
                        }
                    }
                    if self.stack[cs].is_nil()
                        && !self.single_kboard
                        && let Some(kb) = self
                            .kboards
                            .iter()
                            .position(|kb| !kb.deleted && kb.kbd_queue_has_data)
                    {
                        self.current_kboard = kb;
                        return Ok(Value::fixnum(-2));
                    }
                    step = Step::WrongKboard;
                }
                Step::WrongKboard => {
                    if self.stack[cs].is_nil() {
                        let read = self.with_getcjmp(|l| {
                            l.read_decoded_event_from_main_queue(
                                end_time,
                                used_mouse_menu.as_deref_mut(),
                            )
                        })?;
                        match read {
                            None => {
                                if let Some(v) = self.read_char_quit(cs)? {
                                    return Ok(v);
                                }
                            }
                            Some(mut c) => {
                                if c.is_nil()
                                    && end_time.is_some_and(|end| end <= current_timespec())
                                {
                                    step = Step::Exit;
                                    continue;
                                }
                                if c == Value::fixnum(-2) {
                                    return Ok(c);
                                }
                                if c.is_cons() && self.xcar(c) == sym::T {
                                    c = self.xcdr(c);
                                } else if c.is_cons() && self.xcar(c) == sym::NO_RECORD {
                                    c = self.xcdr(c);
                                    recorded = true;
                                }
                                self.stack[cs] = c;
                            }
                        }
                    }
                    step = Step::NonReread;
                }
                Step::NonReread => {
                    if end_time.is_none() {
                        self.timer_stop_idle();
                    }
                    let mut c = self.stack[cs];
                    if c.is_nil() {
                        if commandflag >= 0
                            && !self.keyboard.input_pending
                            && !self.detect_input_pending_run_timers()?
                        {
                            self.redisplay()?;
                        }
                        step = Step::WrongKboard;
                        continue;
                    }
                    if self.is_buffer(c) {
                        step = Step::Exit;
                        continue;
                    }
                    let save = self.plain_value(sym::QUIT_FLAG);
                    self.set_plain_value(sym::QUIT_FLAG, Value::NIL);
                    let special = self.plain_value(sym::SPECIAL_EVENT_MAP);
                    let tem = match self.get_keymap(special, false, true) {
                        Ok(map) => self.access_keymap(map, c, false, false),
                        Err(e) => Err(e),
                    };
                    self.set_plain_value(sym::QUIT_FLAG, save);
                    let tem = tem?;
                    if !tem.is_nil() {
                        let prev_buffer = self.current_buffer;
                        self.set_plain_value(sym::LAST_INPUT_EVENT, c);
                        let keys = self.vector(vec![c]);
                        self.funcall(sym::COMMAND_EXECUTE, &[tem, Value::NIL, keys, sym::T])?;
                        let ignored = self.plain_value(sym::WHILE_NO_INPUT_IGNORE_EVENTS);
                        if c.is_cons()
                            && !self.memq(self.xcar(c), ignored).is_nil()
                            && end_time.is_none()
                        {
                            self.timer_resume_idle();
                        }
                        if self.current_buffer != prev_buffer {
                            self.stack[cs] = Value::fixnum(-2);
                            step = Step::Exit;
                        } else {
                            step = Step::Retry;
                        }
                        continue;
                    }
                    if let Some(n) = c.as_fixnum() {
                        if n == -1 {
                            step = Step::Exit;
                            continue;
                        }
                        let table = self.kvar(Kvar::KeyboardTranslateTable);
                        let applies = (table.is_string()
                            && (n as usize) < self.string_chars(table))
                            || (self.is_vector(table)
                                && (n as usize) < self.vector_items(table).len())
                            || (self.is_char_table(table)
                                && (0..=i64::from(crate::character::MAX_CHAR)).contains(&n));
                        if applies {
                            let d = crate::data::f_aref(self, &[table, c])?;
                            if !d.is_nil() {
                                c = d;
                                self.stack[cs] = c;
                            }
                        }
                    }
                    if c.is_cons() && self.xcdr(c).is_cons() {
                        let start = self.xcar(self.xcdr(c));
                        if start.is_cons() && self.xcdr(start).is_cons() {
                            let posn = self.xcar(self.xcdr(start));
                            if posn == sym::MENU_BAR
                                || posn == sym::TAB_BAR
                                || posn == sym::TOOL_BAR
                            {
                                let wrapped = self.list(&[posn]);
                                let cell = self.xcdr(start);
                                self.heap.set_car(cell, wrapped);
                                let unread = self.plain_value(sym::UNREAD_COMMAND_EVENTS);
                                if end_time.is_some() {
                                    let pair = self.cons(sym::T, c);
                                    let unread = self.cons(pair, unread);
                                    self.set_plain_value(sym::UNREAD_COMMAND_EVENTS, unread);
                                } else {
                                    self.stack[also_slot] = c;
                                    let unread = self.cons(c, unread);
                                    self.set_plain_value(sym::UNREAD_COMMAND_EVENTS, unread);
                                }
                                c = posn;
                                self.stack[cs] = c;
                            }
                        }
                    }
                    self.record_char(c)?;
                    recorded = true;
                    let also_record = self.stack[also_slot];
                    if !also_record.is_nil() {
                        self.record_char(also_record)?;
                    }
                    let input_method = self.plain_value(sym::INPUT_METHOD_FUNCTION);
                    if let Some(n) = c.as_fixnum()
                        && !input_method.is_nil()
                        && (0o40..256).contains(&n)
                        && n != 127
                    {
                        let message = self.current_message_value();
                        self.stack[prev_msg_slot] = message;
                        self.set_plain_value(sym::INPUT_METHOD_PREVIOUS_MESSAGE, message);
                    }
                    if !c.is_cons()
                        || (self.xcar(c) != sym::HELP_ECHO
                            && self.xcar(c) != sym::SWITCH_FRAME
                            && self.xcar(c) != sym::SELECT_WINDOW)
                    {
                        if !self.xdisp.echo_area_buffer[0].is_nil() {
                            self.safe_run_hooks(sym::ECHO_AREA_CLEAR_HOOK)?;
                            self.clear_message(true, false)?;
                            if self.minibuf.minibuf_level != 0
                                && self.windows.minibuf_window == self.xdisp.echo_area_window
                                && !self
                                    .is_number(self.plain_value(sym::MINIBUFFER_MESSAGE_TIMEOUT))
                            {
                                unreachable!("resizing the mini-window needs resize_mini_window");
                            }
                        } else {
                            let function = self.plain_value(sym::CLEAR_MESSAGE_FUNCTION);
                            if self.functionp(function) {
                                self.clear_message(true, false)?;
                            }
                        }
                    }
                    step = Step::RereadForInputMethod;
                }
                Step::RereadForInputMethod => {
                    let c = self.stack[cs];
                    let input_method = self.plain_value(sym::INPUT_METHOD_FUNCTION);
                    if let Some(n) = c.as_fixnum()
                        && !input_method.is_nil()
                        && prev_event.is_nil()
                        && (0o40..256).contains(&n)
                        && n != 127
                    {
                        let count = self.specpdl.len();
                        let kb = self.current_kboard;
                        let saved_immediate_echo = self.kboards[kb].immediate_echo;
                        let saved_ok_to_echo = self.keyboard.ok_to_echo_at_next_pause;
                        let saved_echo_string = self.kvar(Kvar::EchoString);
                        let saved_echo_prompt = self.kvar(Kvar::EchoPrompt);
                        let key_count = self.keyboard.this_command_key_count;
                        let command_key_start = self.keyboard.this_single_command_key_start;
                        let keys = if key_count > 0 {
                            let keys = self.keyboard.this_command_keys;
                            crate::fns::f_copy_sequence(self, &[keys])?
                        } else {
                            Value::NIL
                        };
                        let saved = self.stack.len();
                        self.stack
                            .extend_from_slice(&[saved_echo_string, saved_echo_prompt, keys]);
                        self.keyboard.this_command_key_count = 0;
                        self.keyboard.this_single_command_key_start = 0;
                        if !self.xdisp.echo_area_buffer[0].is_nil() {
                            self.safe_run_hooks(sym::ECHO_AREA_CLEAR_HOOK)?;
                        }
                        self.clear_message(true, false)?;
                        self.echo_truncate(0)?;
                        if !self.keymapp(map) {
                            self.specbind(sym::INPUT_METHOD_USE_ECHO_AREA, sym::T)?;
                        }
                        let tem = self.funcall(input_method, &[c]);
                        let tem = self.unbind_to_value(count, tem)?;
                        self.keyboard.this_command_key_count = key_count;
                        self.keyboard.this_single_command_key_start = command_key_start;
                        if key_count > 0 {
                            self.keyboard.this_command_keys = keys;
                        }
                        self.cancel_echoing();
                        self.keyboard.ok_to_echo_at_next_pause = saved_ok_to_echo;
                        self.set_kvar(Kvar::EchoString, saved_echo_string);
                        self.set_kvar(Kvar::EchoPrompt, saved_echo_prompt);
                        self.stack.truncate(saved);
                        if saved_immediate_echo {
                            self.echo_now()?;
                        }
                        if !tem.is_cons() {
                            let previous = self.stack[prev_msg_slot];
                            if !previous.is_nil() {
                                self.message_with_string("%s", previous, false)?;
                            }
                            step = Step::Retry;
                            continue;
                        }
                        self.stack[cs] = self.xcar(tem);
                        let rest = self.xcdr(tem);
                        let post = self.plain_value(sym::UNREAD_POST_INPUT_METHOD_EVENTS);
                        let post = self.nconc2(rest, post);
                        self.set_plain_value(sym::UNREAD_POST_INPUT_METHOD_EVENTS, post);
                    }
                    if !recorded {
                        let c = self.stack[cs];
                        self.record_char(c)?;
                        recorded = true;
                    }
                    step = Step::RereadFirst;
                }
                Step::RereadFirst => {
                    let mut c = self.stack[cs];
                    if c.is_cons() && self.xcar(c) == sym::HELP_ECHO {
                        let mut htem = self.cdr(self.xcdr(c))?;
                        let help = self.car(htem)?;
                        htem = self.cdr(htem)?;
                        let window = self.car(htem)?;
                        htem = self.cdr(htem)?;
                        let object = self.car(htem)?;
                        htem = self.cdr(htem)?;
                        let position = self.car(htem)?;
                        self.show_help_echo(help, window, object, position)?;
                        if end_time.is_none() {
                            self.timer_resume_idle();
                        }
                        step = Step::Retry;
                        continue;
                    }
                    if (!reread || self.keyboard.this_command_key_count == 0) && end_time.is_none()
                    {
                        if !(c.is_cons() && self.event_head_kind(c) == sym::MOUSE_MOVEMENT) {
                            self.keyboard.ok_to_echo_at_next_pause = Some(self.current_kboard);
                        }
                        self.add_command_key(c);
                        let also_record = self.stack[also_slot];
                        if !also_record.is_nil() {
                            self.add_command_key(also_record);
                        }
                        self.echo_update()?;
                    }
                    self.set_plain_value(sym::LAST_INPUT_EVENT, c);
                    self.keyboard.num_input_events += 1;
                    if !self.plain_value(sym::HELP_FORM).is_nil() && self.help_char_p(c) {
                        let count = self.specpdl.len();
                        let config =
                            crate::window::f_current_window_configuration(self, &[Value::NIL])?;
                        let saved = self.keyboard.help_form_saved_window_configs;
                        self.keyboard.help_form_saved_window_configs = self.cons(config, saved);
                        self.record_unwind(read_char_help_form_unwind, Value::NIL);
                        self.call0(sym::HELP_FORM_SHOW)?;
                        self.cancel_echoing();
                        loop {
                            c = self.read_char(0, Value::NIL, Value::NIL, None, None)?;
                            self.stack[cs] = c;
                            if c.is_cons() && self.event_head_kind(c) == sym::MOUSE_CLICK {
                                let configs = self.keyboard.help_form_saved_window_configs;
                                self.heap.set_car(configs, Value::NIL);
                            }
                            if !self.is_buffer(c) {
                                break;
                            }
                        }
                        self.unbind_to(count)?;
                        self.redisplay()?;
                        if c == Value::fixnum(0o40) {
                            self.cancel_echoing();
                            loop {
                                c = self.read_char(0, Value::NIL, Value::NIL, None, None)?;
                                self.stack[cs] = c;
                                if !self.is_buffer(c) {
                                    break;
                                }
                            }
                        }
                    }
                    step = Step::Exit;
                }
                Step::Exit => {
                    self.keyboard.input_was_pending = self.keyboard.input_pending;
                    return Ok(self.stack[cs]);
                }
            }
        }
    }

    fn echo_area_empty_or_ours(&self) -> bool {
        let echo = self.xdisp.echo_area_buffer[0];
        echo.is_nil()
            || self.buffer(echo).z() == 1
            || (self.keyboard.echo_kboard.is_some()
                && self.keyboard.ok_to_echo_at_next_pause == self.keyboard.echo_kboard)
            || (self.keyboard.echo_kboard.is_none()
                && self.keyboard.ok_to_echo_at_next_pause.is_some())
    }

    pub fn read_filtered_event(
        &mut self,
        no_switch_frame: bool,
        ascii_required: bool,
        error_nonascii: bool,
        input_method: bool,
        seconds: Value,
    ) -> LispResult {
        let mut delayed_switch_frame = Value::NIL;
        let end_time = if self.is_number(seconds) {
            let duration = if seconds.is_float() {
                self.heap.float_value(seconds)
            } else {
                seconds.as_fixnum().unwrap_or(0) as f64
            };
            Some(timespec_add(current_timespec(), dtotimespec(duration)))
        } else {
            None
        };
        let base = self.stack.len();
        let r = (|| -> LispResult {
            loop {
                let mut val = loop {
                    let val = self.read_char(
                        0,
                        Value::NIL,
                        if input_method { Value::NIL } else { sym::T },
                        None,
                        end_time,
                    )?;
                    if val != Value::fixnum(-2) {
                        break val;
                    }
                };
                if self.is_buffer(val) {
                    continue;
                }
                if no_switch_frame
                    && val.is_cons()
                    && self.event_head_kind(val) == sym::SWITCH_FRAME
                {
                    delayed_switch_frame = val;
                    self.stack.push(val);
                    continue;
                }
                if ascii_required && !(self.is_number(seconds) && val.is_nil()) {
                    if val.is_symbol() {
                        let tem = self.get(val, sym::EVENT_SYMBOL_ELEMENT_MASK);
                        if !tem.is_nil() {
                            let head = self.car(tem)?;
                            let tem1 = self.get(head, sym::ASCII_CHARACTER);
                            let rest = self.cdr(tem)?;
                            let mask = self.car(rest)?;
                            if let (Some(code), Some(mask)) = (tem1.as_fixnum(), mask.as_fixnum()) {
                                val = Value::fixnum(code | mask);
                            }
                        }
                    }
                    if !val.is_fixnum() {
                        if error_nonascii {
                            let unread = self.list(&[val]);
                            self.set_plain_value(sym::UNREAD_COMMAND_EVENTS, unread);
                            return Err(self.error("Non-character input-event"));
                        }
                        continue;
                    }
                }
                if !delayed_switch_frame.is_nil() {
                    self.keyboard.unread_switch_frame = delayed_switch_frame;
                }
                return Ok(val);
            }
        })();
        self.stack.truncate(base);
        r
    }

    fn barf_if_interaction_inhibited(&mut self) -> LispResult<()> {
        if !self.plain_value(sym::INHIBIT_INTERACTION).is_nil() {
            return Err(self.signal(sym::INHIBITED_INTERACTION, Value::NIL));
        }
        Ok(())
    }

    fn read_char_prompt(&mut self, prompt: Value) -> LispResult<()> {
        self.barf_if_interaction_inhibited()?;
        if !prompt.is_nil() {
            self.cancel_echoing();
            self.message_with_string("%s", prompt, false)?;
        }
        Ok(())
    }

    fn update_recent_keys(&mut self, new_size: usize, kept_keys: usize) {
        let osize = self.vector_items(self.keyboard.recent_keys).len();
        let mut v = vec![Value::NIL; new_size];
        for (i, slot) in v.iter_mut().enumerate().take(kept_keys) {
            let mut idx = self.keyboard.recent_keys_index as i64 - kept_keys as i64 + i as i64;
            while idx < 0 {
                idx += osize as i64;
            }
            *slot = self.recent_key(idx as usize);
        }
        self.keyboard.recent_keys = self.vector(v);
        self.keyboard.total_keys = kept_keys;
        self.keyboard.recent_keys_index = kept_keys % new_size;
        self.keyboard.lossage_limit = new_size;
    }

    pub fn make_event_array(&mut self, args: &[Value]) -> Value {
        for &a in args {
            match a.as_fixnum() {
                Some(n) if (n & !(-CHAR_META)) < 0o200 => {}
                _ => return self.vector(args.to_vec()),
            }
        }
        let bytes: Vec<u8> = args
            .iter()
            .map(|a| {
                let n = a.as_fixnum().unwrap_or(0);
                let mut b = n as u8;
                if n & CHAR_META != 0 {
                    b |= 0x80;
                }
                b
            })
            .collect();
        self.unibyte_string(&bytes)
    }
}

fn read_char_help_form_unwind(lisp: &mut Lisp, _arg: Value) -> LispResult<()> {
    let configs = lisp.keyboard.help_form_saved_window_configs;
    let window_config = lisp.xcar(configs);
    lisp.keyboard.help_form_saved_window_configs = lisp.xcdr(configs);
    if !window_config.is_nil() {
        crate::window::f_set_window_configuration(lisp, &[window_config, Value::NIL, Value::NIL])?;
    }
    Ok(())
}

pub(crate) fn f_read_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.read_char_prompt(arg(args, 0))?;
    let val = lisp.read_filtered_event(true, true, true, !arg(args, 1).is_nil(), arg(args, 2))?;
    Ok(match val.as_fixnum() {
        Some(n) => Value::fixnum(crate::charfns::char_resolve_modifier_mask(n)),
        None => Value::NIL,
    })
}

pub(crate) fn f_read_event(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.read_char_prompt(arg(args, 0))?;
    lisp.read_filtered_event(false, false, false, !arg(args, 1).is_nil(), arg(args, 2))
}

fn f_read_char_exclusive(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.read_char_prompt(arg(args, 0))?;
    let val = lisp.read_filtered_event(true, true, false, !arg(args, 1).is_nil(), arg(args, 2))?;
    Ok(match val.as_fixnum() {
        Some(n) => Value::fixnum(crate::charfns::char_resolve_modifier_mask(n)),
        None => Value::NIL,
    })
}

fn f_lossage_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let arg = arg(args, 0);
    if arg.is_nil() {
        return Ok(Value::fixnum(lisp.keyboard.lossage_limit as i64));
    }
    let Some(new_size) = arg.as_fixnum().filter(|&n| n >= 0) else {
        let message = lisp.string("Value must be a positive integer");
        return Err(lisp.signal_list(sym::USER_ERROR, &[message]));
    };
    let osize = lisp.vector_items(lisp.keyboard.recent_keys).len();
    if new_size as usize == osize {
        return Ok(Value::fixnum(lisp.keyboard.lossage_limit as i64));
    }
    if (new_size as usize) < MIN_NUM_RECENT_KEYS {
        let message = lisp.string(&format!("Value must be >= {MIN_NUM_RECENT_KEYS}"));
        return Err(lisp.signal_list(sym::USER_ERROR, &[message]));
    }
    if new_size as usize > MAX_NUM_RECENT_KEYS {
        let message = lisp.string(&format!("Value must be <= {MAX_NUM_RECENT_KEYS}"));
        return Err(lisp.signal_list(sym::USER_ERROR, &[message]));
    }
    let new_size = new_size as usize;
    let total = lisp.keyboard.total_keys;
    let kept_keys = if new_size > osize {
        total
    } else {
        new_size.min(total)
    };
    lisp.update_recent_keys(new_size, kept_keys);
    Ok(Value::fixnum(lisp.keyboard.lossage_limit as i64))
}

fn f_recent_keys(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let cmds = !arg(args, 0).is_nil();
    let (total, limit) = (lisp.keyboard.total_keys, lisp.keyboard.lossage_limit);
    if total == 0 || (cmds && total < limit) {
        let items = lisp.vector_items(lisp.keyboard.recent_keys)[..total].to_vec();
        return Ok(lisp.vector(items));
    }
    let mut es = Vec::new();
    let mut i = if total < limit {
        0
    } else {
        lisp.keyboard.recent_keys_index
    };
    loop {
        let e = lisp.recent_key(i);
        if cmds || !e.is_cons() || !lisp.xcar(e).is_nil() {
            es.push(e);
        }
        i += 1;
        if i >= limit {
            i = 0;
        }
        if i == lisp.keyboard.recent_keys_index {
            break;
        }
    }
    Ok(lisp.vector(es))
}

fn command_keys(lisp: &Lisp) -> Vec<Value> {
    lisp.vector_items(lisp.keyboard.this_command_keys)[..lisp.keyboard.this_command_key_count]
        .to_vec()
}

fn f_this_command_keys(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let keys = command_keys(lisp);
    Ok(lisp.make_event_array(&keys))
}

fn f_set_this_command_keys(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let keys = lisp.check_string(args[0])?;
    lisp.keyboard.this_command_key_count = 0;
    lisp.keyboard.this_single_command_key_start = 0;
    let chars = lisp.string_char_vec(keys);
    let byte8 = |c: u32| {
        if crate::character::char_byte8_p(c) {
            u32::from(crate::character::char_to_byte8(c))
        } else {
            c
        }
    };
    let key0 = chars.first().copied().map(byte8).unwrap_or(0);
    if key0 == 248 {
        lisp.add_command_key(Value::fixnum(i64::from(b'x') | META_MODIFIER));
    } else {
        lisp.add_command_key(Value::fixnum(i64::from(key0)));
    }
    for &c in chars.iter().skip(1) {
        lisp.add_command_key(Value::fixnum(i64::from(byte8(c))));
    }
    Ok(Value::NIL)
}

pub(crate) fn f_this_command_keys_vector(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let keys = command_keys(lisp);
    Ok(lisp.vector(keys))
}

fn f_this_single_command_keys(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let start = lisp.keyboard.this_single_command_key_start;
    let keys = command_keys(lisp);
    let keys = keys.get(start..).map(<[Value]>::to_vec).unwrap_or_default();
    Ok(lisp.vector(keys))
}

fn f_clear_this_command_keys(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.keyboard.this_command_key_count = 0;
    if arg(args, 0).is_nil() {
        let n = lisp.vector_items(lisp.keyboard.recent_keys).len();
        for i in 0..n {
            lisp.set_recent_key(i, Value::NIL);
        }
        lisp.keyboard.total_keys = 0;
        lisp.keyboard.recent_keys_index = 0;
    }
    Ok(Value::NIL)
}

fn f_recursion_depth(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(
        lisp.minibuf.command_loop_level + lisp.minibuf.minibuf_level,
    ))
}
