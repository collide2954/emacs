use crate::character::{
    CHAR_ALT, CHAR_CTL, CHAR_HYPER, CHAR_META, CHAR_MODIFIER_MASK, CHAR_SHIFT, CHAR_SUPER,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

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
];

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
