use crate::character::{CHAR_MODIFIER_MASK, MAX_CHAR};
use crate::error::LispResult;
use crate::keyboard::{
    DOUBLE_MODIFIER, DOWN_MODIFIER, DRAG_MODIFIER, Kvar, SHIFT_MODIFIER, TRIPLE_MODIFIER,
    UP_MODIFIER,
};
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const READ_KEY_ELTS: usize = 30;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("read-key-sequence", 1, 6, f_read_key_sequence),
    Subr::fixed("read-key-sequence-vector", 1, 6, f_read_key_sequence_vector),
    Subr::fixed(
        "this-single-command-raw-keys",
        0,
        0,
        f_this_single_command_raw_keys,
    ),
];

const CURRENT_BINDING: usize = READ_KEY_ELTS;
const FIRST_EVENT: usize = READ_KEY_ELTS + 1;
const DELAYED_SWITCH_FRAME: usize = READ_KEY_ELTS + 2;
const ORIGINAL_UPPERCASE: usize = READ_KEY_ELTS + 3;
const FAKE_PREFIXED_KEYS: usize = READ_KEY_ELTS + 4;
const KEY: usize = READ_KEY_ELTS + 5;
const NEW_BINDING: usize = READ_KEY_ELTS + 6;
const PROMPT: usize = READ_KEY_ELTS + 7;
const SCRATCH: usize = READ_KEY_ELTS + 8;
const INDEC: usize = READ_KEY_ELTS + 9;
const FKEY: usize = READ_KEY_ELTS + 11;
const KEYTRAN: usize = READ_KEY_ELTS + 13;
const SLOTS: usize = READ_KEY_ELTS + 15;

const CLICK_REDUCIBLE: i64 =
    UP_MODIFIER | DOWN_MODIFIER | DRAG_MODIFIER | DOUBLE_MODIFIER | TRIPLE_MODIFIER;

#[derive(Clone, Copy)]
struct Keyremap {
    slot: usize,
    start: i64,
    end: i64,
}

#[derive(Clone, Copy)]
enum Goto {
    ReplayEntireSequence,
    ReplaySequence,
    LoopTop,
    ReplayKey,
    Done,
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn key_to_char(key: Value) -> u32 {
    (key.as_fixnum().unwrap_or(0) & ((1 << 22) - 1)) as u32
}

impl Lisp {
    fn ks(&self, b: usize, i: usize) -> Value {
        self.stack[b + i]
    }

    fn set_ks(&mut self, b: usize, i: usize, v: Value) {
        self.stack[b + i] = v;
    }

    fn keyremap_reset(&mut self, b: usize, remap: &Keyremap) {
        let parent = self.ks(b, remap.slot);
        self.set_ks(b, remap.slot + 1, parent);
    }

    fn keyremap_init(&mut self, b: usize, remap: &mut Keyremap, map: Value) {
        self.set_ks(b, remap.slot, map);
        self.set_ks(b, remap.slot + 1, map);
        remap.start = 0;
        remap.end = 0;
    }

    fn event_start(&self, event: Value) -> Value {
        let rest = self.cdr_safe(event);
        self.car_safe(rest)
    }

    fn posn_posn(&self, posn: Value) -> Value {
        let rest = self.cdr_safe(posn);
        self.car_safe(rest)
    }

    fn select_window_buffer(&mut self) -> LispResult<()> {
        let contents = self.window(self.windows.selected_window).contents;
        if contents != self.current_buffer {
            crate::buffer::f_set_buffer(self, &[contents])?;
        }
        Ok(())
    }

    fn kill_emacs_unless_selected_frame_live(&mut self) -> LispResult<()> {
        if !self.frame_live_p(self.frames.selected_frame) {
            crate::emacs::f_kill_emacs(self, &[Value::NIL, Value::NIL])?;
        }
        Ok(())
    }

    fn follow_key(&mut self, keymap: Value, key: Value) -> LispResult {
        let map = self.get_keymap(keymap, false, true)?;
        self.access_keymap(map, key, true, false)
    }

    fn active_maps(&mut self, first_event: Value, second_event: Value) -> LispResult {
        let mut position = if first_event.is_cons() {
            self.event_start(first_event)
        } else {
            Value::NIL
        };
        if first_event.is_symbol() && second_event.is_cons() {
            let start = self.event_start(second_event);
            if first_event == self.posn_posn(start) {
                position = start;
            }
        }
        let maps = self.current_active_maps(sym::T, position)?;
        Ok(self.cons(sym::KEYMAP, maps))
    }

    fn access_keymap_keyremap(
        &mut self,
        b: usize,
        map: Value,
        key: Value,
        prompt: Value,
        do_funcall: bool,
        (start, end): (i64, i64),
    ) -> LispResult {
        let mut next = self.access_keymap(map, key, true, false)?;
        if next.is_symbol() {
            let function = self.heap.symbol_ref(next).function;
            if !function.is_nil() && (self.arrayp(function) || self.keymapp(function)) {
                next = self.autoload_do_load(function, next, Value::NIL)?;
            }
        }
        if do_funcall && self.functionp(next) {
            let tem = next;
            self.set_ks(b, SCRATCH, tem);
            let items = self.stack[b + start as usize..=b + end as usize].to_vec();
            let remap = self.vector(items);
            let count = self.specpdl_index();
            self.specbind(sym::CURRENT_KEY_REMAP_SEQUENCE, remap)?;
            let r = self.funcall(next, &[prompt]);
            next = self.unbind_to_value(count, r)?;
            if !(next.is_nil() || self.is_vector(next) || next.is_string()) {
                return Err(self.signal_error("Function returns invalid key sequence", tem));
            }
        }
        Ok(next)
    }

    fn keyremap_step(
        &mut self,
        b: usize,
        fkey: &mut Keyremap,
        input: i64,
        doit: bool,
        prompt: Value,
    ) -> LispResult<Option<i64>> {
        let buf_start = fkey.start;
        let buf_end = fkey.end;
        let key = self.ks(b, fkey.end as usize);
        fkey.end += 1;
        let parent = self.ks(b, fkey.slot);
        let next = if self.keymapp(parent) {
            let map = self.ks(b, fkey.slot + 1);
            self.access_keymap_keyremap(b, map, key, prompt, doit, (buf_start, buf_end))?
        } else {
            Value::NIL
        };
        self.set_ks(b, SCRATCH, next);
        if (self.is_vector(next) || next.is_string()) && doit {
            let len = crate::fns::f_length(self, &[next])?
                .as_fixnum()
                .unwrap_or(0);
            let diff = len - (fkey.end - fkey.start);
            if READ_KEY_ELTS as i64 - input <= diff {
                return Err(self.error("Key sequence too long"));
            }
            if diff < 0 {
                for i in fkey.end..input {
                    let v = self.ks(b, i as usize);
                    self.set_ks(b, (i + diff) as usize, v);
                }
            } else if diff > 0 {
                for i in (fkey.end..input).rev() {
                    let v = self.ks(b, i as usize);
                    self.set_ks(b, (i + diff) as usize, v);
                }
            }
            for i in 0..len {
                let v = crate::data::f_aref(self, &[next, Value::fixnum(i)])?;
                self.set_ks(b, (fkey.start + i) as usize, v);
            }
            fkey.end += diff;
            fkey.start = fkey.end;
            self.keyremap_reset(b, fkey);
            return Ok(Some(diff));
        }
        let map = self.get_keymap(next, false, true)?;
        self.set_ks(b, fkey.slot + 1, map);
        if !map.is_cons() {
            fkey.start += 1;
            fkey.end = fkey.start;
            self.keyremap_reset(b, fkey);
        }
        Ok(None)
    }

    fn test_undefined(&mut self, binding: Value) -> LispResult<bool> {
        Ok(binding.is_nil()
            || binding == sym::UNDEFINED
            || (binding.is_symbol()
                && self.command_remapping(binding, Value::NIL, Value::NIL)? == sym::UNDEFINED))
    }

    pub fn init_raw_keybuf_count(&mut self) {
        self.keyboard.raw_keybuf.clear();
    }

    pub fn read_key_sequence(
        &mut self,
        prompt: Value,
        dont_downcase_last: bool,
        can_return_switch_frame: bool,
        fix_current_buffer: bool,
        prevent_redisplay: bool,
    ) -> LispResult<Option<Vec<Value>>> {
        let b = self.stack.len();
        self.stack.resize(b + SLOTS, Value::NIL);
        self.stack[b + PROMPT] = prompt;
        let r = self.read_key_sequence_1(
            b,
            prompt,
            dont_downcase_last,
            can_return_switch_frame,
            fix_current_buffer,
            prevent_redisplay,
        );
        let r = r.map(|t| t.map(|t| self.stack[b..b + t].to_vec()));
        self.stack.truncate(b);
        r
    }

    fn read_key_sequence_1(
        &mut self,
        b: usize,
        prompt: Value,
        dont_downcase_last: bool,
        can_return_switch_frame: bool,
        fix_current_buffer: bool,
        prevent_redisplay: bool,
    ) -> LispResult<Option<usize>> {
        let count = self.specpdl_index();
        let elts = READ_KEY_ELTS as i64;
        let mut t: i64 = 0;
        let mut echo_start = 0;
        let mut first_unbound: i64 = elts + 1;
        let mut mock_input: i64 = 0;
        let mut used_mouse_menu_history = [false; READ_KEY_ELTS];
        let mut indec = Keyremap {
            slot: INDEC,
            start: 0,
            end: 0,
        };
        let mut fkey = Keyremap {
            slot: FKEY,
            start: 0,
            end: 0,
        };
        let mut keytran = Keyremap {
            slot: KEYTRAN,
            start: 0,
            end: 0,
        };
        let mut shift_translated = false;
        let mut original_uppercase_position: i64 = -1;
        let mut starting_buffer = Value::NIL;
        let mut echo_local_start = 0;
        let mut keys_local_start = 0;
        let mut used_mouse_menu = false;
        let mut last_real_key_start: i64;

        if self.interactive() {
            if !prompt.is_nil() {
                self.set_kvar(Kvar::EchoPrompt, prompt);
                let kb = self.current_kboard;
                self.kboards[kb].immediate_echo = false;
                self.echo_now()?;
                if !self.echo_keystrokes_p() {
                    let kb = self.current_kboard;
                    self.kboards[kb].immediate_echo = false;
                }
            } else if self.var_true(sym::CURSOR_IN_ECHO_AREA) && self.echo_keystrokes_p() {
                self.echo_dash()?;
            }
        }
        if self.interactive() {
            echo_start = self.echo_length();
        }
        let keys_start = self.keyboard.this_command_key_count;
        self.keyboard.this_single_command_key_start = keys_start;

        let mut go = Goto::ReplayEntireSequence;
        'top: loop {
            match go {
                Goto::ReplayEntireSequence => {
                    let map = self.kvar(Kvar::InputDecodeMap);
                    self.keyremap_init(b, &mut indec, map);
                    let map = self.kvar(Kvar::LocalFunctionKeyMap);
                    self.keyremap_init(b, &mut fkey, map);
                    let map = self.bound_value(sym::KEY_TRANSLATION_MAP);
                    self.keyremap_init(b, &mut keytran, map);
                    go = Goto::ReplaySequence;
                }
                Goto::ReplaySequence => {
                    starting_buffer = self.current_buffer;
                    first_unbound = elts + 1;
                    let first_event = if mock_input > 0 {
                        self.ks(b, 0)
                    } else {
                        Value::NIL
                    };
                    let second_event = if mock_input > 1 {
                        self.ks(b, 1)
                    } else {
                        Value::NIL
                    };
                    self.set_ks(b, FIRST_EVENT, first_event);
                    let maps = self.active_maps(first_event, second_event)?;
                    self.set_ks(b, CURRENT_BINDING, maps);
                    t = 0;
                    self.set_plain_value(sym::LAST_NONMENU_EVENT, Value::NIL);
                    self.keyboard.this_command_key_count = keys_start;
                    if self.interactive() && t < mock_input {
                        self.echo_truncate(echo_start)?;
                    }
                    go = Goto::LoopTop;
                }
                Goto::LoopTop => {
                    let current_binding = self.ks(b, CURRENT_BINDING);
                    let keep_reading = if current_binding.is_nil() {
                        keytran.start < t
                    } else {
                        self.keymapp(current_binding)
                    };
                    if !keep_reading {
                        self.keyboard.read_key_sequence_cmd = current_binding;
                        go = Goto::Done;
                        continue 'top;
                    }
                    used_mouse_menu = false;
                    if first_unbound < keytran.start {
                        for i in first_unbound + 1..t {
                            let v = self.ks(b, i as usize);
                            self.set_ks(b, (i - first_unbound - 1) as usize, v);
                        }
                        mock_input = t - first_unbound - 1;
                        for remap in [&mut indec, &mut fkey, &mut keytran] {
                            remap.start -= first_unbound + 1;
                            remap.end = remap.start;
                            self.keyremap_reset(b, remap);
                        }
                        go = Goto::ReplaySequence;
                        continue 'top;
                    }
                    if t >= elts {
                        return Err(self.error("Key sequence too long"));
                    }
                    if self.interactive() {
                        echo_local_start = self.echo_length();
                    }
                    keys_local_start = self.keyboard.this_command_key_count;
                    go = Goto::ReplayKey;
                }
                Goto::ReplayKey => {
                    if self.interactive() && t < mock_input {
                        self.echo_truncate(echo_local_start)?;
                    }
                    self.keyboard.this_command_key_count = keys_local_start;
                    last_real_key_start = t;
                    if t < mock_input {
                        let key = self.ks(b, t as usize);
                        self.set_ks(b, KEY, key);
                        self.add_command_key(key);
                        let kb = self.current_kboard;
                        if self.kboards[kb].immediate_echo {
                            self.kboards[kb].immediate_echo = false;
                            self.echo_now()?;
                        }
                        used_mouse_menu = used_mouse_menu_history[t as usize];
                    } else if !self.plain_value(sym::EXECUTING_KBD_MACRO).is_nil()
                        && self.at_end_of_macro_p()?
                        && !self.requeued_events_pending_p()
                    {
                        t = 0;
                        go = Goto::Done;
                        continue 'top;
                    } else {
                        let interrupted_kboard = self.current_kboard;
                        let interrupted_frame = self.frames.selected_frame;
                        let commandflag = if prevent_redisplay {
                            -2
                        } else {
                            i32::from(prompt.is_nil())
                        };
                        let current_binding = self.ks(b, CURRENT_BINDING);
                        let last_nonmenu_event = self.plain_value(sym::LAST_NONMENU_EVENT);
                        let key = self.read_char(
                            commandflag,
                            current_binding,
                            last_nonmenu_event,
                            Some(&mut used_mouse_menu),
                            None,
                        )?;
                        self.set_ks(b, KEY, key);
                        used_mouse_menu_history[t as usize] = used_mouse_menu;
                        if key == Value::fixnum(-2) || interrupted_kboard != self.current_kboard {
                            if self.kboards[interrupted_kboard].deleted {
                                self.set_ks(b, DELAYED_SWITCH_FRAME, Value::NIL);
                                go = Goto::ReplayEntireSequence;
                                continue 'top;
                            }
                            let queue = Kvar::KbdQueue as usize;
                            let delayed = self.ks(b, DELAYED_SWITCH_FRAME);
                            if !delayed.is_nil() {
                                let q = self.kboards[interrupted_kboard].vars[queue];
                                let q = self.cons(delayed, q);
                                self.kboards[interrupted_kboard].vars[queue] = q;
                                self.set_ks(b, DELAYED_SWITCH_FRAME, Value::NIL);
                            }
                            while t > 0 {
                                t -= 1;
                                let k = self.ks(b, t as usize);
                                let q = self.kboards[interrupted_kboard].vars[queue];
                                let q = self.cons(k, q);
                                self.kboards[interrupted_kboard].vars[queue] = q;
                            }
                            let q = self.kboards[interrupted_kboard].vars[queue];
                            let needs_switch_frame = q.is_cons() && {
                                let head = self.xcar(q);
                                self.set_ks(b, KEY, head);
                                !(head.is_cons() && self.event_head_kind(head) == sym::SWITCH_FRAME)
                            };
                            if needs_switch_frame {
                                let event = self.make_lispy_switch_frame(interrupted_frame);
                                let q = self.kboards[interrupted_kboard].vars[queue];
                                let q = self.cons(event, q);
                                self.kboards[interrupted_kboard].vars[queue] = q;
                                mock_input = 0;
                            } else {
                                let key = self.ks(b, KEY);
                                if key.as_fixnum().is_some_and(|n| n != -2) {
                                    if fix_current_buffer {
                                        self.select_window_buffer()?;
                                    }
                                    mock_input = 1;
                                    self.set_ks(b, 0, key);
                                } else {
                                    mock_input = 0;
                                }
                            }
                            go = Goto::ReplayEntireSequence;
                            continue 'top;
                        }
                        if key == sym::T {
                            self.unbind_to(count)?;
                            return Ok(None);
                        }
                        if self.is_buffer(key) {
                            self.timer_resume_idle();
                            mock_input = t;
                            if fix_current_buffer {
                                self.kill_emacs_unless_selected_frame_live()?;
                                self.select_window_buffer()?;
                            }
                            go = Goto::ReplaySequence;
                            continue 'top;
                        }
                        if key.as_fixnum() == Some(self.keyboard.quit_char)
                            && self.current_buffer != starting_buffer
                        {
                            self.keyboard.raw_keybuf.push(key);
                            self.set_ks(b, t as usize, key);
                            t += 1;
                            mock_input = t;
                            self.set_plain_value(sym::QUIT_FLAG, Value::NIL);
                            go = Goto::ReplaySequence;
                            continue 'top;
                        }
                        self.set_plain_value(sym::QUIT_FLAG, Value::NIL);
                        if key.is_cons()
                            && self.event_head_kind(key) == sym::SWITCH_FRAME
                            && (t > 0 || !can_return_switch_frame)
                        {
                            self.set_ks(b, DELAYED_SWITCH_FRAME, key);
                            go = Goto::ReplayKey;
                            continue 'top;
                        }
                        if self.ks(b, FIRST_EVENT).is_nil() {
                            self.set_ks(b, FIRST_EVENT, key);
                            if fix_current_buffer {
                                self.select_window_buffer()?;
                            }
                            let maps = self.active_maps(key, Value::NIL)?;
                            self.set_ks(b, CURRENT_BINDING, maps);
                        }
                        let raw = if key.is_cons() {
                            crate::fns::f_copy_sequence(self, &[key])?
                        } else {
                            key
                        };
                        self.keyboard.raw_keybuf.push(raw);
                    }

                    let key = self.ks(b, KEY);
                    if key.is_cons() {
                        let kind = self.event_head_kind(key);
                        if kind == sym::MOUSE_CLICK || kind == sym::TOUCHSCREEN {
                            let start = self.event_start(key);
                            let window = self.car_safe(start);
                            let posn = self.posn_posn(start);
                            let fake_prefixed_keys = self.ks(b, FAKE_PREFIXED_KEYS);
                            let already_prefixed = !fake_prefixed_keys.is_nil()
                                && !self.memq(key, fake_prefixed_keys).is_nil();
                            if (posn.is_cons() || already_prefixed) && t > 0 {
                                last_real_key_start = t - 1;
                            }
                            if last_real_key_start == 0 && self.is_window(window) {
                                let contents = self.window(window).contents;
                                if self.is_buffer(contents) && contents != self.current_buffer {
                                    self.set_ks(b, t as usize, key);
                                    mock_input = t + 1;
                                    self.record_unwind_current_buffer();
                                    self.kill_emacs_unless_selected_frame_live()?;
                                    self.set_buffer_internal(contents);
                                    go = Goto::ReplaySequence;
                                    continue 'top;
                                }
                            }
                            if posn.is_symbol() && !already_prefixed {
                                if elts - t <= 1 {
                                    return Err(self.error("Key sequence too long"));
                                }
                                self.set_ks(b, t as usize, posn);
                                self.set_ks(b, t as usize + 1, key);
                                mock_input = t + 2;
                                let fake_prefixed_keys = self.cons(key, fake_prefixed_keys);
                                self.set_ks(b, FAKE_PREFIXED_KEYS, fake_prefixed_keys);
                                go = Goto::ReplayKey;
                                continue 'top;
                            }
                        } else {
                            let start = self.event_start(key);
                            if self.xcdr(key).is_cons()
                                && start.is_cons()
                                && self.xcdr(start).is_cons()
                            {
                                let posn = self.posn_posn(start);
                                if (posn == sym::MENU_BAR
                                    || posn == sym::TAB_BAR
                                    || posn == sym::TOOL_BAR)
                                    && mock_input <= t
                                {
                                    if elts - t <= 1 {
                                        return Err(self.error("Key sequence too long"));
                                    }
                                    self.set_ks(b, t as usize, posn);
                                    self.set_ks(b, t as usize + 1, key);
                                    let zapped = self.list(&[posn]);
                                    let rest = self.xcdr(start);
                                    self.heap.set_car(rest, zapped);
                                    mock_input = t + 2;
                                    go = Goto::ReplaySequence;
                                    continue 'top;
                                } else if posn.is_cons() && last_real_key_start == t && t > 0 {
                                    last_real_key_start = t - 1;
                                }
                            }
                        }
                    }

                    let current_binding = self.ks(b, CURRENT_BINDING);
                    let new_binding = self.follow_key(current_binding, key)?;
                    self.set_ks(b, NEW_BINDING, new_binding);
                    if !new_binding.is_nil() {
                        first_unbound = first_unbound.max(t + 1);
                    } else {
                        first_unbound = first_unbound.min(t);
                        let head = self.event_head(key);
                        if head.is_symbol() {
                            let breakdown = self.parse_modifiers(head)?;
                            let mut modifiers =
                                self.xcar(self.xcdr(breakdown)).as_fixnum().unwrap_or(0);
                            while modifiers & CLICK_REDUCIBLE != 0 {
                                if modifiers & TRIPLE_MODIFIER != 0 {
                                    modifiers ^= DOUBLE_MODIFIER | TRIPLE_MODIFIER;
                                } else if modifiers & DOUBLE_MODIFIER != 0 {
                                    modifiers &= !DOUBLE_MODIFIER;
                                } else if modifiers & DRAG_MODIFIER != 0 {
                                    modifiers &= !DRAG_MODIFIER;
                                } else {
                                    if indec.end > last_real_key_start {
                                        indec.start = last_real_key_start.min(indec.start);
                                        indec.end = indec.start;
                                        self.keyremap_reset(b, &indec);
                                        if fkey.end > last_real_key_start {
                                            fkey.start = last_real_key_start.min(fkey.start);
                                            fkey.end = fkey.start;
                                            self.keyremap_reset(b, &fkey);
                                            if keytran.end > last_real_key_start {
                                                keytran.start =
                                                    last_real_key_start.min(keytran.start);
                                                keytran.end = keytran.start;
                                                self.keyremap_reset(b, &keytran);
                                            }
                                        }
                                    }
                                    if t == last_real_key_start {
                                        mock_input = 0;
                                        go = Goto::ReplayKey;
                                    } else {
                                        mock_input = last_real_key_start;
                                        go = Goto::ReplaySequence;
                                    }
                                    continue 'top;
                                }
                                let base = self.xcar(breakdown);
                                let new_head = self.apply_modifiers(modifiers, base);
                                let start = self.event_start(key);
                                let new_click = self.list(&[new_head, start]);
                                self.set_ks(b, SCRATCH, new_click);
                                let new_binding = self.follow_key(current_binding, new_click)?;
                                self.set_ks(b, NEW_BINDING, new_binding);
                                if !new_binding.is_nil() {
                                    self.set_ks(b, KEY, new_click);
                                    break;
                                }
                            }
                        }
                    }
                    let new_binding = self.ks(b, NEW_BINDING);
                    self.set_ks(b, CURRENT_BINDING, new_binding);

                    let key = self.ks(b, KEY);
                    self.set_ks(b, t as usize, key);
                    t += 1;
                    if !used_mouse_menu {
                        self.set_plain_value(sym::LAST_NONMENU_EVENT, key);
                    }
                    let single_start = self.keyboard.this_command_key_count as i64 - t;
                    self.keyboard.this_single_command_key_start = single_start.max(0) as usize;

                    while indec.end < t {
                        let input = t.max(mock_input);
                        if let Some(diff) =
                            self.keyremap_step(b, &mut indec, input, true, prompt)?
                        {
                            mock_input = diff + input;
                            go = Goto::ReplaySequence;
                            continue 'top;
                        }
                    }

                    let current_binding = self.ks(b, CURRENT_BINDING);
                    if !self.keymapp(current_binding)
                        && !self.test_undefined(current_binding)?
                        && indec.start >= t
                    {
                        if fkey.start < t {
                            fkey.start = t;
                            fkey.end = t;
                            self.keyremap_reset(b, &fkey);
                        }
                    } else {
                        while fkey.end < indec.start {
                            let input = t.max(mock_input);
                            let current_binding = self.ks(b, CURRENT_BINDING);
                            let doit = fkey.end + 1 == t && self.test_undefined(current_binding)?;
                            if let Some(diff) =
                                self.keyremap_step(b, &mut fkey, input, doit, prompt)?
                            {
                                mock_input = diff + input;
                                indec.end += diff;
                                indec.start += diff;
                                go = Goto::ReplaySequence;
                                continue 'top;
                            }
                        }
                    }

                    while keytran.end < fkey.start {
                        let input = t.max(mock_input);
                        if let Some(diff) =
                            self.keyremap_step(b, &mut keytran, input, true, prompt)?
                        {
                            mock_input = diff + input;
                            indec.end += diff;
                            indec.start += diff;
                            fkey.end += diff;
                            fkey.start += diff;
                            go = Goto::ReplaySequence;
                            continue 'top;
                        }
                    }

                    let current_binding = self.ks(b, CURRENT_BINDING);
                    let key = self.ks(b, KEY);
                    if current_binding.is_nil()
                        && keytran.start >= t
                        && let Some(k) = key.as_fixnum()
                        && self.var_true(sym::TRANSLATE_UPPER_CASE_KEY_BINDINGS)
                    {
                        let c = k & !CHAR_MODIFIER_MASK;
                        let new_key = if k & SHIFT_MODIFIER != 0 {
                            Some(k & !SHIFT_MODIFIER)
                        } else if (0..=i64::from(MAX_CHAR)).contains(&c) {
                            let dc = i64::from(self.downcase_char_buffer(c as u32));
                            (dc != c).then_some(dc | (k & CHAR_MODIFIER_MASK))
                        } else {
                            None
                        };
                        if let Some(new_key) = new_key {
                            self.set_ks(b, ORIGINAL_UPPERCASE, key);
                            original_uppercase_position = t - 1;
                            self.set_ks(b, (t - 1) as usize, Value::fixnum(new_key));
                            mock_input = t.max(mock_input);
                            shift_translated = true;
                            go = Goto::ReplaySequence;
                            continue 'top;
                        }
                    }

                    if current_binding.is_nil() && self.help_char_p(self.event_head(key)) && t > 1 {
                        self.keyboard.read_key_sequence_cmd =
                            self.plain_value(sym::PREFIX_HELP_COMMAND);
                        go = Goto::Done;
                        continue 'top;
                    }

                    if current_binding.is_nil() && keytran.start >= t {
                        let breakdown = self.parse_modifiers(key)?;
                        self.set_ks(b, SCRATCH, breakdown);
                        let modifiers = if breakdown.is_cons() {
                            self.xcar(self.xcdr(breakdown)).as_fixnum().unwrap_or(0)
                        } else {
                            0
                        };
                        if self.var_true(sym::TRANSLATE_UPPER_CASE_KEY_BINDINGS)
                            && (modifiers & SHIFT_MODIFIER != 0
                                || (key.is_fixnum() && self.uppercasep(key_to_char(key))))
                        {
                            let new_key = if modifiers & SHIFT_MODIFIER != 0 {
                                let base = self.xcar(breakdown);
                                self.apply_modifiers(modifiers & !SHIFT_MODIFIER, base)
                            } else {
                                let dc = self.downcase_char_buffer(key_to_char(key));
                                Value::fixnum(i64::from(dc) | modifiers)
                            };
                            self.set_ks(b, ORIGINAL_UPPERCASE, key);
                            original_uppercase_position = t - 1;
                            self.set_ks(b, (t - 1) as usize, new_key);
                            mock_input = t.max(mock_input);
                            fkey.start = 0;
                            fkey.end = 0;
                            keytran.start = 0;
                            keytran.end = 0;
                            shift_translated = true;
                            go = Goto::ReplaySequence;
                            continue 'top;
                        }
                    }
                    go = Goto::LoopTop;
                }
                Goto::Done => {
                    let cmd = self.keyboard.read_key_sequence_cmd;
                    self.keyboard.read_key_sequence_remapped = if cmd.is_symbol() {
                        self.command_remapping(cmd, Value::NIL, Value::NIL)?
                    } else {
                        Value::NIL
                    };
                    self.keyboard.unread_switch_frame = self.ks(b, DELAYED_SWITCH_FRAME);
                    self.unbind_to(count)?;
                    if (dont_downcase_last || self.ks(b, CURRENT_BINDING).is_nil())
                        && t > 0
                        && t - 1 == original_uppercase_position
                    {
                        let original = self.ks(b, ORIGINAL_UPPERCASE);
                        self.set_ks(b, (t - 1) as usize, original);
                        shift_translated = false;
                    }
                    if shift_translated {
                        self.set_plain_value(sym::THIS_COMMAND_KEYS_SHIFT_TRANSLATED, sym::T);
                    }
                    while t < mock_input {
                        let k = self.ks(b, t as usize);
                        self.add_command_key(k);
                        t += 1;
                    }
                    self.echo_update()?;
                    return Ok(Some(t as usize));
                }
            }
        }
    }

    fn read_key_sequence_vs(&mut self, args: &[Value], allow_string: bool) -> LispResult {
        let (prompt, continue_echo, dont_downcase_last, can_return_switch_frame, cmd_loop) = (
            arg(args, 0),
            arg(args, 1),
            arg(args, 2),
            arg(args, 3),
            arg(args, 4),
        );
        let count = self.specpdl_index();
        if !prompt.is_nil() {
            self.check_string(prompt)?;
        }
        self.maybe_quit()?;
        let im = if cmd_loop.is_nil() {
            sym::T
        } else {
            Value::NIL
        };
        self.specbind(sym::INPUT_METHOD_EXIT_ON_FIRST_CHAR, im)?;
        self.specbind(sym::INPUT_METHOD_USE_ECHO_AREA, im)?;
        if continue_echo.is_nil() {
            self.keyboard.this_command_key_count = 0;
            self.keyboard.this_single_command_key_start = 0;
        }
        self.init_raw_keybuf_count();
        let keys = self.read_key_sequence(
            prompt,
            !dont_downcase_last.is_nil(),
            !can_return_switch_frame.is_nil(),
            false,
            false,
        )?;
        let keys = match keys {
            Some(keys) => keys,
            None => {
                self.set_plain_value(sym::QUIT_FLAG, sym::T);
                self.maybe_quit()?;
                Vec::new()
            }
        };
        let result = if allow_string {
            self.make_event_array(&keys)
        } else {
            self.vector(keys)
        };
        self.unbind_to_value(count, Ok(result))
    }
}

fn f_read_key_sequence(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.read_key_sequence_vs(args, true)
}

fn f_read_key_sequence_vector(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.read_key_sequence_vs(args, false)
}

fn f_this_single_command_raw_keys(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let keys = lisp.keyboard.raw_keybuf.clone();
    Ok(lisp.vector(keys))
}
