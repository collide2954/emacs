use crate::error::LispResult;
use crate::keyboard::{
    ALT_MODIFIER, CLICK_MODIFIER, CTRL_MODIFIER, DOUBLE_MODIFIER, DOWN_MODIFIER, DRAG_MODIFIER,
    HYPER_MODIFIER, Kvar, META_MODIFIER, READABLE_EVENTS_DO_TIMERS_NOW, SHIFT_MODIFIER,
    SUPER_MODIFIER, TRIPLE_MODIFIER, UP_MODIFIER, make_ctrl_char,
};
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::termhooks::{EventKind, KbdEvent, ScrollBarPart};
use crate::timefns::{Timespec, current_timespec, timespec_sub};
use crate::value::Value;

pub const KBD_BUFFER_SIZE: usize = 4096;
pub const READABLE_EVENTS_FILTER_EVENTS: i32 = 1 << 1;
pub const READABLE_EVENTS_IGNORE_SQUEEZABLES: i32 = 1 << 2;
const FUNCTION_KEY_OFFSET: u32 = 0xff00;
const ISO_FUNCTION_KEY_OFFSET: u32 = 0xfe00;

const LISPY_ACCENT_CODES: [u32; 20] = [0; 20];

const LISPY_ACCENT_KEYS: [&str; 20] = [
    "dead-circumflex",
    "dead-grave",
    "dead-tilde",
    "dead-diaeresis",
    "dead-macron",
    "dead-degree",
    "dead-acute",
    "dead-cedilla",
    "dead-breve",
    "dead-ogonek",
    "dead-caron",
    "dead-doubleacute",
    "dead-abovedot",
    "dead-abovering",
    "dead-iota",
    "dead-belowdot",
    "dead-voiced-sound",
    "dead-semivoiced-sound",
    "dead-hook",
    "dead-horn",
];

const LISPY_FUNCTION_KEYS: [Option<&str>; 256] = {
    let mut t: [Option<&str>; 256] = [None; 256];
    t[0x08] = Some("backspace");
    t[0x09] = Some("tab");
    t[0x0a] = Some("linefeed");
    t[0x0b] = Some("clear");
    t[0x0d] = Some("return");
    t[0x13] = Some("pause");
    t[0x1b] = Some("escape");
    t[0x21] = Some("kanji");
    t[0x22] = Some("muhenkan");
    t[0x23] = Some("henkan");
    t[0x24] = Some("romaji");
    t[0x25] = Some("hiragana");
    t[0x26] = Some("katakana");
    t[0x27] = Some("hiragana-katakana");
    t[0x28] = Some("zenkaku");
    t[0x29] = Some("hankaku");
    t[0x2a] = Some("zenkaku-hankaku");
    t[0x2b] = Some("touroku");
    t[0x2c] = Some("massyo");
    t[0x2d] = Some("kana-lock");
    t[0x2e] = Some("kana-shift");
    t[0x2f] = Some("eisu-shift");
    t[0x30] = Some("eisu-toggle");
    t[0x50] = Some("home");
    t[0x51] = Some("left");
    t[0x52] = Some("up");
    t[0x53] = Some("right");
    t[0x54] = Some("down");
    t[0x55] = Some("prior");
    t[0x56] = Some("next");
    t[0x57] = Some("end");
    t[0x58] = Some("begin");
    t[0x60] = Some("select");
    t[0x61] = Some("print");
    t[0x62] = Some("execute");
    t[0x63] = Some("insert");
    t[0x65] = Some("undo");
    t[0x66] = Some("redo");
    t[0x67] = Some("menu");
    t[0x68] = Some("find");
    t[0x69] = Some("cancel");
    t[0x6a] = Some("help");
    t[0x6b] = Some("break");
    t[0x74] = Some("backtab");
    t[0x7f] = Some("kp-numlock");
    t[0x80] = Some("kp-space");
    t[0x89] = Some("kp-tab");
    t[0x8d] = Some("kp-enter");
    t[0x91] = Some("kp-f1");
    t[0x92] = Some("kp-f2");
    t[0x93] = Some("kp-f3");
    t[0x94] = Some("kp-f4");
    t[0x95] = Some("kp-home");
    t[0x96] = Some("kp-left");
    t[0x97] = Some("kp-up");
    t[0x98] = Some("kp-right");
    t[0x99] = Some("kp-down");
    t[0x9a] = Some("kp-prior");
    t[0x9b] = Some("kp-next");
    t[0x9c] = Some("kp-end");
    t[0x9d] = Some("kp-begin");
    t[0x9e] = Some("kp-insert");
    t[0x9f] = Some("kp-delete");
    t[0xaa] = Some("kp-multiply");
    t[0xab] = Some("kp-add");
    t[0xac] = Some("kp-separator");
    t[0xad] = Some("kp-subtract");
    t[0xae] = Some("kp-decimal");
    t[0xaf] = Some("kp-divide");
    t[0xb0] = Some("kp-0");
    t[0xb1] = Some("kp-1");
    t[0xb2] = Some("kp-2");
    t[0xb3] = Some("kp-3");
    t[0xb4] = Some("kp-4");
    t[0xb5] = Some("kp-5");
    t[0xb6] = Some("kp-6");
    t[0xb7] = Some("kp-7");
    t[0xb8] = Some("kp-8");
    t[0xb9] = Some("kp-9");
    t[0xbd] = Some("kp-equal");
    let names = [
        "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12", "f13", "f14",
        "f15", "f16", "f17", "f18", "f19", "f20", "f21", "f22", "f23", "f24", "f25", "f26", "f27",
        "f28", "f29", "f30", "f31", "f32", "f33", "f34", "f35",
    ];
    let mut i = 0;
    while i < names.len() {
        t[0xbe + i] = Some(names[i]);
        i += 1;
    }
    t[0xff] = Some("delete");
    t
};

const ISO_LISPY_FUNCTION_KEYS: [Option<&str>; 53] = {
    let mut t: [Option<&str>; 53] = [None; 53];
    let names = [
        "iso-lefttab",
        "iso-move-line-up",
        "iso-move-line-down",
        "iso-partial-line-up",
        "iso-partial-line-down",
        "iso-partial-space-left",
        "iso-partial-space-right",
        "iso-set-margin-left",
        "iso-set-margin-right",
        "iso-release-margin-left",
        "iso-release-margin-right",
        "iso-release-both-margins",
        "iso-fast-cursor-left",
        "iso-fast-cursor-right",
        "iso-fast-cursor-up",
        "iso-fast-cursor-down",
        "iso-continuous-underline",
        "iso-discontinuous-underline",
        "iso-emphasize",
        "iso-center-object",
        "iso-enter",
    ];
    let mut i = 0;
    while i < names.len() {
        t[0x20 + i] = Some(names[i]);
        i += 1;
    }
    t
};

pub const SUBRS: &[Subr] = &[
    Subr::fixed("input-pending-p", 0, 1, f_input_pending_p),
    Subr::fixed("insert-special-event", 1, 1, f_insert_special_event),
    Subr::fixed("internal--track-mouse", 1, 1, f_internal_track_mouse),
];

fn tracking_off(lisp: &mut Lisp, old_track_mouse: Value) -> LispResult<()> {
    lisp.set_plain_value(sym::TRACK_MOUSE, old_track_mouse);
    if old_track_mouse.is_nil() && !lisp.readable_events(READABLE_EVENTS_DO_TIMERS_NOW)? {
        lisp.redisplay_preserve_echo_area(6)?;
        lisp.get_input_pending(READABLE_EVENTS_DO_TIMERS_NOW)?;
    }
    Ok(())
}

fn f_internal_track_mouse(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let bodyfun = args[0];
    let count = lisp.specpdl.len();
    let old = lisp.plain_value(sym::TRACK_MOUSE);
    lisp.record_unwind(tracking_off, old);
    lisp.set_plain_value(sym::TRACK_MOUSE, sym::T);
    let val = lisp.funcall(bodyfun, &[]);
    lisp.unbind_to_value(count, val)
}

pub(crate) const LISPY_WHEEL_NAMES: [&str; 4] =
    ["wheel-up", "wheel-down", "wheel-left", "wheel-right"];

#[derive(Clone, Copy)]
enum SymbolTable {
    Accent,
    Function,
    System,
    Mouse,
    Wheel,
}

enum NameTable {
    Strs(&'static [&'static str]),
    Options(&'static [Option<&'static str>]),
    Sized(usize),
    None,
}

impl NameTable {
    fn get(&self, i: usize) -> Option<&'static str> {
        match self {
            NameTable::Strs(t) => t.get(i).copied(),
            NameTable::Options(t) => t.get(i).copied().flatten(),
            NameTable::Sized(_) | NameTable::None => None,
        }
    }

    fn size(&self) -> i64 {
        match self {
            NameTable::Strs(t) => t.len() as i64,
            NameTable::Options(t) => t.len() as i64,
            NameTable::Sized(n) => *n as i64,
            NameTable::None => i64::MAX,
        }
    }
}

impl Lisp {
    fn symbol_table(&self, table: SymbolTable) -> Value {
        match table {
            SymbolTable::Accent => self.keyboard.accent_key_syms,
            SymbolTable::Function => self.keyboard.func_key_syms,
            SymbolTable::System => self.kvar(Kvar::SystemKeySyms),
            SymbolTable::Mouse => self.keyboard.mouse_syms,
            SymbolTable::Wheel => self.keyboard.wheel_syms,
        }
    }

    fn set_symbol_table(&mut self, table: SymbolTable, value: Value) {
        match table {
            SymbolTable::Accent => self.keyboard.accent_key_syms = value,
            SymbolTable::Function => self.keyboard.func_key_syms = value,
            SymbolTable::System => self.set_kvar(Kvar::SystemKeySyms, value),
            SymbolTable::Mouse => self.keyboard.mouse_syms = value,
            SymbolTable::Wheel => self.keyboard.wheel_syms = value,
        }
    }

    fn save_line_number_display_width(&mut self, event: &KbdEvent) -> LispResult<()> {
        let w = if self.is_window(event.frame_or_window) {
            event.frame_or_window
        } else if self.is_frame(event.frame_or_window) {
            self.frame(event.frame_or_window).selected_window
        } else {
            self.windows.selected_window
        };
        self.keyboard.down_mouse_line_number_width = self.line_number_display_width(w)?.0;
        Ok(())
    }

    fn line_number_mode_hscroll(&mut self, start_pos: Value, end_pos: Value) -> LispResult<bool> {
        if self.car_safe(start_pos) != self.car_safe(end_pos)
            || self.list_to_vec(start_pos)?.len() < 7
            || self.list_to_vec(end_pos)?.len() < 7
        {
            return Ok(false);
        }
        let start_col_row = self.list_to_vec(start_pos)?[6];
        let end_col_row = self.list_to_vec(end_pos)?[6];
        let mut window = self.car_safe(end_pos);
        if !self.window_valid_p(window) {
            window = self.windows.selected_window;
        }
        let (col_width, _) = self.line_number_display_width(window)?;
        let start_col = self.car_safe(start_col_row);
        let end_col = self.car_safe(end_col_row);
        Ok(start_col == end_col
            && self.keyboard.down_mouse_line_number_width >= 0
            && col_width != self.keyboard.down_mouse_line_number_width)
    }

    fn mouse_event_frame(&self, event: &KbdEvent) -> Value {
        if self.is_window(event.frame_or_window) {
            self.window(event.frame_or_window).frame
        } else if self.is_frame(event.frame_or_window) {
            event.frame_or_window
        } else {
            panic!("emacs_abort: mouse event without a frame or window")
        }
    }

    fn double_click_p(&self, event: &KbdEvent, button: i64) -> bool {
        let f = self.mouse_event_frame(event);
        let fuzz = if self.frame(f).window_p() {
            self.int_var(sym::DOUBLE_CLICK_FUZZ)
        } else {
            self.int_var(sym::DOUBLE_CLICK_FUZZ) / 8
        };
        let (x, y) = (
            event.x.as_fixnum().unwrap_or(0),
            event.y.as_fixnum().unwrap_or(0),
        );
        let time = self.find_symbol_value(sym::DOUBLE_CLICK_TIME);
        button == self.keyboard.last_mouse_button
            && (x - self.keyboard.last_mouse_x).abs() <= fuzz
            && (y - self.keyboard.last_mouse_y).abs() <= fuzz
            && self.keyboard.button_down_time != 0
            && (time == sym::T
                || time.as_fixnum().is_some_and(|limit| {
                    limit >= 0
                        && event
                            .timestamp
                            .wrapping_sub(self.keyboard.button_down_time as u64)
                            < limit as u64
                }))
    }

    fn make_lispy_mouse_click(&mut self, event: &mut KbdEvent) -> LispResult {
        let button = i64::from(event.code);
        let f = event.frame_or_window;
        if !self.frame_live_p(f) {
            return Ok(Value::NIL);
        }
        let (x, y) = (
            event.x.as_fixnum().unwrap_or(0),
            event.y.as_fixnum().unwrap_or(0),
        );
        if !self.frame(f).window_p() {
            let (column, row) = (x, y);
            if row >= 0
                && row < self.frame(f).geometry.menu_bar_lines
                && event.modifiers & DOWN_MODIFIER != 0
            {
                let items = self.frame(f).menu_bar_items;
                let items = if self.is_vector(items) {
                    self.vector_items(items).to_vec()
                } else {
                    Vec::new()
                };
                let mut item = Value::NIL;
                for &[key, string, _, pos] in items.as_chunks::<4>().0 {
                    if string.is_nil() {
                        break;
                    }
                    let pos = pos.as_fixnum().unwrap_or(0);
                    if column >= pos && column < pos + self.string_chars(string) as i64 {
                        item = key;
                        break;
                    }
                }
                if item.is_nil() {
                    return Ok(Value::NIL);
                }
                let xy = self.cons(event.x, event.y);
                let position =
                    self.list(&[f, sym::MENU_BAR, xy, Value::fixnum(event.timestamp as i64)]);
                return Ok(self.list(&[item, position]));
            }
        }
        let base = self.stack.len();
        let r = self.make_lispy_mouse_click_1(event, button, f, x, y);
        self.stack.truncate(base);
        r
    }

    fn make_lispy_mouse_click_1(
        &mut self,
        event: &mut KbdEvent,
        button: i64,
        f: Value,
        x: i64,
        y: i64,
    ) -> LispResult {
        let mut position = self.make_lispy_position(Some(f), x, y, event.timestamp as i64)?;
        self.stack.push(position);
        if event.arg.is_cons() && self.xcar(event.arg) == sym::TAB_BAR {
            let object = self.list(&[self.xcdr(event.arg)]);
            position = self.nconc2(position, object);
        }
        let size = self.vector_items(self.keyboard.button_down_location).len() as i64;
        if button >= size {
            let incr = (button - size + 1) as usize;
            let location = self.keyboard.button_down_location;
            self.keyboard.button_down_location = crate::fns::larger_vector(self, location, incr);
            let mouse_syms = self.keyboard.mouse_syms;
            self.keyboard.mouse_syms = crate::fns::larger_vector(self, mouse_syms, incr);
        }
        let location = self.keyboard.button_down_location;
        let start_pos = self.vector_items(location)[button as usize];
        self.stack.push(start_pos);
        self.vector_set(location, button as usize, Value::NIL);
        let is_double = self.double_click_p(event, button);
        self.keyboard.last_mouse_button = button;
        self.keyboard.last_mouse_x = x;
        self.keyboard.last_mouse_y = y;
        if event.modifiers & DOWN_MODIFIER != 0 {
            if is_double {
                self.keyboard.double_click_count += 1;
                event.modifiers |= if self.keyboard.double_click_count > 2 {
                    TRIPLE_MODIFIER
                } else {
                    DOUBLE_MODIFIER
                };
            } else {
                self.keyboard.double_click_count = 1;
            }
            self.keyboard.button_down_time = event.timestamp as i64;
            let copy = crate::fns::f_copy_alist(self, &[position])?;
            let location = self.keyboard.button_down_location;
            self.vector_set(location, button as usize, copy);
            self.keyboard.frame_relative_event_pos = self.cons(event.x, event.y);
            self.keyboard.ignore_mouse_drag_p = false;
            self.save_line_number_display_width(event)?;
        } else if event.modifiers & UP_MODIFIER != 0 {
            if !start_pos.is_cons() {
                return Ok(Value::NIL);
            }
            let mut click_or_drag_modifier = CLICK_MODIFIER;
            if self.keyboard.ignore_mouse_drag_p {
                self.keyboard.ignore_mouse_drag_p = false;
            } else {
                let fuzz = self.int_var(sym::DOUBLE_CLICK_FUZZ);
                let pos = self.keyboard.frame_relative_event_pos;
                let xdiff = x - self.car_safe(pos).as_fixnum().unwrap_or(0);
                let ydiff = y - self.cdr_safe(pos).as_fixnum().unwrap_or(0);
                let start_window = self.car_safe(start_pos);
                let start_area = self.car_safe(self.cdr_safe(start_pos));
                let window = self.car_safe(position);
                let area = self.car_safe(self.cdr_safe(position));
                let same = 0 < fuzz
                    && -fuzz < xdiff
                    && xdiff < fuzz
                    && -fuzz < ydiff
                    && ydiff < fuzz
                    && (start_area == area
                        || self.line_number_mode_hscroll(start_pos, position)?
                        || start_window != window);
                if !same {
                    self.keyboard.button_down_time = 0;
                    click_or_drag_modifier = DRAG_MODIFIER;
                    self.keyboard.down_mouse_line_number_width = -1;
                } else if (start_window != window || start_area != area)
                    && start_area.is_fixnum()
                    && self.window_live_p(start_window)
                    && !crate::data::f_fboundp(self, &[sym::WINDOW_EDGES])?.is_nil()
                {
                    let edges = self.funcall(
                        sym::WINDOW_EDGES,
                        &[start_window, sym::T, Value::NIL, sym::T],
                    )?;
                    let edges = self.list_to_vec(edges)?;
                    let edge = |i: usize| edges.get(i).and_then(|v| v.as_fixnum()).unwrap_or(0);
                    let pos = self.keyboard.frame_relative_event_pos;
                    let mut new_x = self.car_safe(pos).as_fixnum().unwrap_or(0);
                    let mut new_y = self.cdr_safe(pos).as_fixnum().unwrap_or(0);
                    if new_x < edge(0) {
                        new_x = edge(0);
                    } else if new_x >= edge(2) {
                        new_x = edge(2) - 1;
                    }
                    if new_y < edge(1) {
                        new_y = edge(1);
                    } else if new_y >= edge(3) {
                        new_y = edge(3) - 1;
                    }
                    position = self.make_lispy_position(
                        Some(event.frame_or_window),
                        new_x,
                        new_y,
                        event.timestamp as i64,
                    )?;
                    self.stack.push(position);
                }
            }
            let count = self.keyboard.double_click_count;
            event.modifiers = (event.modifiers & !UP_MODIFIER)
                | click_or_drag_modifier
                | match count {
                    ..2 => 0,
                    2 => DOUBLE_MODIFIER,
                    _ => TRIPLE_MODIFIER,
                };
        } else {
            panic!("emacs_abort: a mouse click without a down or up modifier");
        }
        let size = self.vector_items(self.keyboard.mouse_syms).len();
        let stem = self.keyboard.lispy_mouse_stem;
        let head = self.modify_event_symbol(
            button,
            event.modifiers,
            sym::MOUSE_CLICK,
            stem,
            NameTable::Sized(size),
            SymbolTable::Mouse,
        );
        Ok(if event.modifiers & DRAG_MODIFIER != 0 {
            self.list(&[head, start_pos, position])
        } else if event.modifiers & (DOUBLE_MODIFIER | TRIPLE_MODIFIER) != 0 {
            let count = Value::fixnum(self.keyboard.double_click_count);
            self.list(&[head, position, count])
        } else {
            self.list(&[head, position])
        })
    }

    fn make_lispy_movement(
        &mut self,
        frame: Value,
        bar_window: Value,
        x: Value,
        y: Value,
        t: u64,
    ) -> LispResult {
        if !frame.is_nil() && !bar_window.is_nil() {
            unreachable!("scroll-bar movement events need scroll bars");
        }
        let frame = (!frame.is_nil()).then_some(frame);
        let position = self.make_lispy_position(
            frame,
            x.as_fixnum().unwrap_or(0),
            y.as_fixnum().unwrap_or(0),
            t as i64,
        )?;
        Ok(self.list(&[sym::MOUSE_MOVEMENT, position]))
    }

    fn make_lispy_wheel(&mut self, event: &mut KbdEvent) -> LispResult {
        let f = event.frame_or_window;
        if !self.frame_live_p(f) {
            return Ok(Value::NIL);
        }
        let (x, y) = (
            event.x.as_fixnum().unwrap_or(0),
            event.y.as_fixnum().unwrap_or(0),
        );
        let base = self.stack.len();
        let position = self.make_lispy_position(Some(f), x, y, event.timestamp as i64);
        let position = match position {
            Ok(position) => position,
            Err(e) => {
                self.stack.truncate(base);
                return Err(e);
            }
        };
        self.stack.push(position);
        let mut symbol_num = if event.modifiers & UP_MODIFIER != 0 {
            event.modifiers &= !UP_MODIFIER;
            0
        } else if event.modifiers & DOWN_MODIFIER != 0 {
            event.modifiers &= !DOWN_MODIFIER;
            1
        } else {
            panic!("emacs_abort: a wheel event without a down or up modifier")
        };
        if event.kind == EventKind::HorizWheel {
            symbol_num += 2;
        }
        if self.double_click_p(event, -(1 + symbol_num)) {
            self.keyboard.double_click_count += 1;
            event.modifiers |= if self.keyboard.double_click_count > 2 {
                TRIPLE_MODIFIER
            } else {
                DOUBLE_MODIFIER
            };
        } else {
            self.keyboard.double_click_count = 1;
            event.modifiers |= CLICK_MODIFIER;
        }
        self.keyboard.button_down_time = event.timestamp as i64;
        self.keyboard.last_mouse_button = -(1 + symbol_num);
        self.keyboard.last_mouse_x = x;
        self.keyboard.last_mouse_y = y;
        let head = self.modify_event_symbol(
            symbol_num,
            event.modifiers,
            sym::MOUSE_CLICK,
            Value::NIL,
            NameTable::Strs(&LISPY_WHEEL_NAMES),
            SymbolTable::Wheel,
        );
        let count = Value::fixnum(self.keyboard.double_click_count);
        let arg = event.arg;
        let r = if arg.is_cons() {
            let rest = self.xcdr(arg);
            let delta = if rest.is_cons() && self.xcdr(rest).is_cons() {
                let dx = self.xcar(rest);
                let dy = self.xcar(self.xcdr(rest));
                self.cons(dx, dy)
            } else {
                Value::NIL
            };
            let lines = self.xcar(arg);
            self.list(&[head, position, count, lines, delta])
        } else if self.is_number(arg) {
            self.list(&[head, position, count, arg])
        } else if event.modifiers & (DOUBLE_MODIFIER | TRIPLE_MODIFIER) != 0 {
            self.list(&[head, position, count])
        } else {
            self.list(&[head, position])
        };
        self.stack.truncate(base);
        Ok(r)
    }

    fn modify_event_symbol(
        &mut self,
        symbol_num: i64,
        modifiers: i64,
        symbol_kind: Value,
        name_alist_or_stem: Value,
        name_table: NameTable,
        table: SymbolTable,
    ) -> Value {
        let table_size = name_table.size();
        let symbol_int = Value::fixnum(symbol_num & 0xffffff);
        if symbol_num < 0 || symbol_num >= table_size {
            return Value::NIL;
        }
        let mut symbol_table = self.symbol_table(table);
        let mut value = if symbol_table.is_cons() {
            let cell = self.assq(symbol_int, symbol_table);
            self.cdr_safe(cell)
        } else {
            if !self.is_vector(symbol_table)
                || self.vector_items(symbol_table).len() as i64 != table_size
            {
                symbol_table = self.vector(vec![Value::NIL; table_size as usize]);
                self.set_symbol_table(table, symbol_table);
            }
            self.vector_items(symbol_table)[symbol_num as usize]
        };
        if value.is_nil() {
            if name_alist_or_stem.is_cons() {
                let cell = self.assq(symbol_int, name_alist_or_stem);
                value = self.cdr_safe(cell);
            } else if name_alist_or_stem.is_string() {
                let stem = String::from_utf8_lossy(self.string_bytes(name_alist_or_stem));
                let name = format!("{stem}-{}", (symbol_num & 0xffffff) + 1);
                value = self.intern(&name);
            } else if let Some(name) = name_table.get(symbol_num as usize) {
                value = self.intern(name);
            }
            if value.is_nil()
                && let Some(window_system) = self.window_system
                && let Some(name) = (window_system.get_keysym_name)(symbol_num as u32)
            {
                value = self.intern(name);
            }
            if value.is_nil() {
                value = self.intern(&format!("key-{symbol_num}"));
            }
            if symbol_table.is_cons() {
                let cell = self.cons(symbol_int, value);
                let table_value = self.cons(cell, symbol_table);
                self.set_symbol_table(table, table_value);
            } else if let crate::object::Vectorlike::Vector(items) =
                self.heap.vectorlike_mut(symbol_table)
            {
                items[symbol_num as usize] = value;
            }
            self.apply_modifiers(modifiers & CLICK_MODIFIER, value);
            self.put(value, sym::EVENT_KIND, symbol_kind);
        }
        self.apply_modifiers(modifiers, value)
    }

    pub fn make_lispy_switch_frame(&mut self, frame: Value) -> Value {
        self.list(&[sym::SWITCH_FRAME, frame])
    }

    pub fn make_lispy_event(&mut self, event: &mut KbdEvent) -> LispResult {
        Ok(match event.kind {
            EventKind::DeleteWindow
            | EventKind::Iconify
            | EventKind::Deiconify
            | EventKind::MoveFrame => {
                let head = match event.kind {
                    EventKind::DeleteWindow => sym::DELETE_FRAME,
                    EventKind::Iconify => sym::ICONIFY_FRAME,
                    EventKind::Deiconify => sym::MAKE_FRAME_VISIBLE,
                    _ => sym::MOVE_FRAME,
                };
                let frame = self.list(&[event.frame_or_window]);
                self.list(&[head, frame])
            }
            EventKind::NoEvent => Value::NIL,
            EventKind::Help => {
                let frame = event.frame_or_window;
                let object = event.arg;
                let position = Value::fixnum(event.timestamp as i64);
                let mut window = event.x;
                let help = event.y;
                event.kind = EventKind::NoEvent;
                if !self.is_window(window) {
                    window = Value::NIL;
                }
                let rest = self.list(&[frame, help, window, object, position]);
                self.cons(sym::HELP_ECHO, rest)
            }
            EventKind::FocusIn => self.list(&[sym::FOCUS_IN, event.frame_or_window]),
            EventKind::FocusOut => self.list(&[sym::FOCUS_OUT, event.frame_or_window]),
            EventKind::AsciiKeystroke | EventKind::MultibyteCharKeystroke => {
                let mut c = i64::from(event.code);
                if event.kind == EventKind::AsciiKeystroke {
                    c &= 0o377;
                }
                if event.modifiers & !SHIFT_MODIFIER != 0 {
                    if self.uppercasep(c as u32) && event.modifiers & SHIFT_MODIFIER == 0 {
                        c = i64::from(self.downcase_char_buffer(c as u32));
                    } else if self.lowercasep(c as u32) && event.modifiers & SHIFT_MODIFIER != 0 {
                        c = i64::from(self.upcase_char_buffer(c as u32));
                    }
                }
                if event.kind == EventKind::AsciiKeystroke && event.modifiers & CTRL_MODIFIER != 0 {
                    c = make_ctrl_char(c);
                    event.modifiers &= !CTRL_MODIFIER;
                }
                c |= event.modifiers
                    & (META_MODIFIER
                        | ALT_MODIFIER
                        | HYPER_MODIFIER
                        | SUPER_MODIFIER
                        | CTRL_MODIFIER);
                if event.code == 0o40 && event.modifiers & SHIFT_MODIFIER != 0 {
                    c |= SHIFT_MODIFIER;
                }
                self.keyboard.button_down_time = 0;
                Value::fixnum(c)
            }
            EventKind::NonAsciiKeystroke => {
                self.keyboard.button_down_time = 0;
                let code = event.code;
                for (i, &accent) in LISPY_ACCENT_CODES.iter().enumerate() {
                    if code == accent {
                        return Ok(self.modify_event_symbol(
                            i as i64,
                            event.modifiers,
                            sym::FUNCTION_KEY,
                            Value::NIL,
                            NameTable::Strs(&LISPY_ACCENT_KEYS),
                            SymbolTable::Accent,
                        ));
                    }
                }
                if (ISO_FUNCTION_KEY_OFFSET..FUNCTION_KEY_OFFSET).contains(&code) {
                    return Ok(self.modify_event_symbol(
                        i64::from(code - ISO_FUNCTION_KEY_OFFSET),
                        event.modifiers,
                        sym::FUNCTION_KEY,
                        Value::NIL,
                        NameTable::Options(&ISO_LISPY_FUNCTION_KEYS),
                        SymbolTable::Function,
                    ));
                }
                if (FUNCTION_KEY_OFFSET..FUNCTION_KEY_OFFSET + LISPY_FUNCTION_KEYS.len() as u32)
                    .contains(&code)
                    && LISPY_FUNCTION_KEYS[(code - FUNCTION_KEY_OFFSET) as usize].is_some()
                {
                    return Ok(self.modify_event_symbol(
                        i64::from(code - FUNCTION_KEY_OFFSET),
                        event.modifiers,
                        sym::FUNCTION_KEY,
                        Value::NIL,
                        NameTable::Options(&LISPY_FUNCTION_KEYS),
                        SymbolTable::Function,
                    ));
                }
                if self.kvar(Kvar::SystemKeySyms).is_nil() {
                    let table = self.cons(Value::NIL, Value::NIL);
                    self.set_kvar(Kvar::SystemKeySyms, table);
                }
                let alist = self.kvar(Kvar::SystemKeyAlist);
                self.modify_event_symbol(
                    i64::from(code),
                    event.modifiers,
                    sym::FUNCTION_KEY,
                    alist,
                    NameTable::None,
                    SymbolTable::System,
                )
            }
            EventKind::SelectWindow => {
                let window = self.list(&[event.frame_or_window]);
                self.list(&[sym::SELECT_WINDOW, window])
            }
            EventKind::TabBar | EventKind::ToolBar => {
                let mut res = event.arg;
                let location = if event.kind == EventKind::TabBar {
                    sym::TAB_BAR
                } else {
                    sym::TOOL_BAR
                };
                if res.is_symbol() {
                    res = self.apply_modifiers(event.modifiers, res);
                }
                let position = self.list(&[event.frame_or_window, location]);
                self.list(&[res, position])
            }
            EventKind::SaveSession => self.list(&[sym::SAVE_SESSION, event.arg]),
            EventKind::Sleep => self.cons(sym::SLEEP_EVENT, event.arg),
            EventKind::ConfigChanged => {
                self.list(&[sym::CONFIG_CHANGED_EVENT, event.arg, event.frame_or_window])
            }
            EventKind::PreeditText => self.list(&[sym::PREEDIT_TEXT, event.arg]),
            EventKind::MouseClick => return self.make_lispy_mouse_click(event),
            EventKind::Wheel | EventKind::HorizWheel => return self.make_lispy_wheel(event),
            EventKind::ScrollBarClick
            | EventKind::HorizontalScrollBarClick
            | EventKind::DragNDrop
            | EventKind::TouchEnd
            | EventKind::TouchscreenBegin
            | EventKind::TouchscreenUpdate
            | EventKind::TouchscreenEnd
            | EventKind::Pinch => {
                unreachable!("mouse and touch events need make_lispy_position")
            }
            EventKind::UserSignal => unreachable!("user signals need find_user_signal_name"),
            _ => panic!("emacs_abort: make_lispy_event of {:?}", event.kind),
        })
    }

    fn event_to_kboard(&self, event: &KbdEvent) -> Option<usize> {
        if matches!(
            event.kind,
            EventKind::SelectionRequest | EventKind::SelectionClear
        ) {
            return None;
        }
        let mut obj = event.frame_or_window;
        if self.is_window(obj) {
            obj = self.window(obj).frame;
        }
        if self.is_frame(obj) && self.frame_live_p(obj) {
            self.frame_kboard(obj)
        } else {
            None
        }
    }

    fn is_ignored_event(&self, event: &KbdEvent) -> bool {
        let ignore_event = match event.kind {
            EventKind::FocusIn => sym::FOCUS_IN,
            EventKind::FocusOut => sym::FOCUS_OUT,
            EventKind::Help => sym::HELP_ECHO,
            EventKind::Iconify => sym::ICONIFY_FRAME,
            EventKind::Deiconify => sym::MAKE_FRAME_VISIBLE,
            EventKind::SelectionRequest => sym::SELECTION_REQUEST,
            EventKind::FileNotify => sym::FILE_NOTIFY,
            EventKind::Sleep => sym::SLEEP_EVENT,
            EventKind::MonitorsChanged => sym::MONITORS_CHANGED,
            EventKind::ToolkitThemeChanged => sym::TOOLKIT_THEME_CHANGED,
            _ => Value::NIL,
        };
        let list = self.plain_value(sym::WHILE_NO_INPUT_IGNORE_EVENTS);
        !self.memq(ignore_event, list).is_nil()
    }

    pub fn some_mouse_moved(&self) -> Option<Value> {
        if self.plain_value(sym::TRACK_MOUSE).is_nil() || self.keyboard.ignore_mouse_drag_p {
            return None;
        }
        self.frames
            .list
            .iter()
            .copied()
            .find(|&f| self.frame(f).flags.mouse_moved)
    }

    pub fn readable_events(&mut self, flags: i32) -> LispResult<bool> {
        if flags & READABLE_EVENTS_DO_TIMERS_NOW != 0 {
            self.timer_check()?;
        }
        if !self.keyboard.kbd_buffer.is_empty() {
            if flags & (READABLE_EVENTS_FILTER_EVENTS | READABLE_EVENTS_IGNORE_SQUEEZABLES) != 0 {
                let filter = !self
                    .plain_value(sym::INPUT_PENDING_P_FILTER_EVENTS)
                    .is_nil();
                for event in self.keyboard.kbd_buffer.iter() {
                    let filtered = flags & READABLE_EVENTS_FILTER_EVENTS != 0
                        && ((!filter
                            && matches!(event.kind, EventKind::FocusIn | EventKind::FocusOut))
                            || (filter && self.is_ignored_event(event)));
                    let squeezable = flags & READABLE_EVENTS_IGNORE_SQUEEZABLES != 0
                        && matches!(
                            event.kind,
                            EventKind::ScrollBarClick | EventKind::HorizontalScrollBarClick
                        )
                        && event.part == ScrollBarPart::Handle
                        && event.modifiers == 0;
                    if !filtered && !squeezable {
                        return Ok(true);
                    }
                }
            } else {
                return Ok(true);
            }
        }
        if flags & READABLE_EVENTS_IGNORE_SQUEEZABLES == 0 && self.some_mouse_moved().is_some() {
            return Ok(true);
        }
        if self.single_kboard {
            if self.kboards[self.current_kboard].kbd_queue_has_data {
                return Ok(true);
            }
        } else if self
            .kboards
            .iter()
            .any(|kb| !kb.deleted && kb.kbd_queue_has_data)
        {
            return Ok(true);
        }
        Ok(false)
    }

    pub fn get_input_pending(&mut self, flags: i32) -> LispResult<bool> {
        self.keyboard.input_pending =
            !self.plain_value(sym::QUIT_FLAG).is_nil() || self.readable_events(flags)?;
        if !self.keyboard.input_pending
            && (!self.keyboard.interrupt_input || self.keyboard.interrupts_deferred)
        {
            self.gobble_input()?;
            self.keyboard.input_pending =
                !self.plain_value(sym::QUIT_FLAG).is_nil() || self.readable_events(flags)?;
        }
        Ok(self.keyboard.input_pending)
    }

    pub fn gobble_input(&mut self) -> LispResult<i32> {
        let mut nread = 0;
        let mut err = false;
        let terminals = self.terminals.list.clone();
        let base = self.stack.len();
        self.stack.extend_from_slice(&terminals);
        let r = (|| -> LispResult<()> {
            for t in terminals {
                if !self.terminals.list.contains(&t) {
                    continue;
                }
                let Some(read_socket) = self.terminal(t).hooks.read_socket else {
                    continue;
                };
                let mut hold_quit = KbdEvent::default();
                let nr = loop {
                    let nr = read_socket(self, t, &mut hold_quit)?;
                    if nr <= 0 {
                        break nr;
                    }
                    nread += nr;
                };
                if nr == -1 {
                    err = true;
                } else if nr == -2 {
                    if self.terminals.list.len() == 1 {
                        unreachable!("losing the last terminal needs terminate_due_to_signal");
                    }
                    crate::terminal::f_delete_terminal(self, &[t, sym::NOELISP])?;
                }
                if nr >= 0 {
                    let frames = self.frames.list.clone();
                    for f in frames {
                        if self.frame(f).terminal == t {
                            self.frame_make_pointer_visible(f);
                        }
                    }
                }
                if hold_quit.kind != EventKind::NoEvent {
                    self.kbd_buffer_store_event(hold_quit)?;
                }
            }
            Ok(())
        })();
        self.stack.truncate(base);
        r?;
        if err && nread == 0 {
            nread = -1;
        }
        Ok(nread)
    }

    pub fn kbd_buffer_store_event(&mut self, event: KbdEvent) -> LispResult<()> {
        self.kbd_buffer_store_buffered_event(event, None)
    }

    pub fn gen_help_event(
        &mut self,
        help: Value,
        frame: Value,
        window: Value,
        object: Value,
        pos: i64,
    ) -> LispResult<()> {
        let x = if self.is_window(window) {
            window
        } else {
            frame
        };
        self.kbd_buffer_store_event(KbdEvent {
            kind: EventKind::Help,
            frame_or_window: frame,
            arg: object,
            x,
            y: help,
            timestamp: pos as u64,
            ..KbdEvent::default()
        })
    }

    pub fn kbd_buffer_store_buffered_event(
        &mut self,
        event: KbdEvent,
        hold_quit: Option<&mut KbdEvent>,
    ) -> LispResult<()> {
        if event.kind == EventKind::NoEvent {
            panic!("emacs_abort: storing NO_EVENT");
        }
        if let Some(hold) = &hold_quit
            && hold.kind != EventKind::NoEvent
        {
            return Ok(());
        }
        if event.kind == EventKind::AsciiKeystroke {
            let mut c = i64::from(event.code & 0o377);
            if event.modifiers & CTRL_MODIFIER != 0 {
                c = make_ctrl_char(c);
            }
            c |= event.modifiers & (META_MODIFIER | ALT_MODIFIER | HYPER_MODIFIER | SUPER_MODIFIER);
            if c == self.keyboard.quit_char {
                let kb = self
                    .frame_kboard(event.frame_or_window)
                    .expect("frame kboard");
                if self.single_kboard && kb != self.current_kboard {
                    let switch = self.make_lispy_switch_frame(event.frame_or_window);
                    let queue = self.list(&[switch, Value::fixnum(c)]);
                    self.kboards[kb].vars[Kvar::KbdQueue as usize] = queue;
                    self.kboards[kb].kbd_queue_has_data = true;
                    let mut buffer = std::mem::take(&mut self.keyboard.kbd_buffer);
                    for sp in buffer.iter_mut() {
                        if self.event_to_kboard(sp) == Some(kb) {
                            sp.kind = EventKind::NoEvent;
                            sp.frame_or_window = Value::NIL;
                            sp.arg = Value::NIL;
                        }
                    }
                    self.keyboard.kbd_buffer = buffer;
                    return Ok(());
                }
                if let Some(hold) = hold_quit {
                    *hold = event;
                    return Ok(());
                }
                let mut focus = self.frame(event.frame_or_window).focus_frame;
                if focus.is_nil() {
                    focus = event.frame_or_window;
                }
                self.keyboard.internal_last_event_frame = focus;
                self.set_plain_value(sym::LAST_EVENT_FRAME, focus);
                return self.handle_interrupt(false);
            }
        }
        if self.keyboard.kbd_buffer.len() < KBD_BUFFER_SIZE - 1 {
            self.keyboard.kbd_buffer.push_back(event);
        }
        let throw_on_input = self.plain_value(sym::THROW_ON_INPUT);
        if !throw_on_input.is_nil() && !self.is_ignored_event(&event) {
            self.set_plain_value(sym::QUIT_FLAG, throw_on_input);
        }
        Ok(())
    }

    pub fn discard_mouse_events(&mut self) {
        for sp in self.keyboard.kbd_buffer.iter_mut() {
            if matches!(
                sp.kind,
                EventKind::MouseClick
                    | EventKind::Wheel
                    | EventKind::HorizWheel
                    | EventKind::ScrollBarClick
                    | EventKind::HorizontalScrollBarClick
            ) {
                sp.kind = EventKind::NoEvent;
            }
        }
    }

    pub fn kbd_buffer_events_waiting(&mut self) -> bool {
        while let Some(front) = self.keyboard.kbd_buffer.front() {
            if front.kind != EventKind::NoEvent {
                return true;
            }
            self.keyboard.kbd_buffer.pop_front();
        }
        false
    }

    pub fn cancel_echoing(&mut self) {
        let kb = self.current_kboard;
        self.kboards[kb].immediate_echo = false;
        self.set_kvar(Kvar::EchoPrompt, Value::NIL);
        self.set_kvar(Kvar::EchoString, Value::NIL);
        self.keyboard.ok_to_echo_at_next_pause = None;
        self.keyboard.echo_kboard = None;
        self.keyboard.echo_message_buffer = Value::NIL;
    }

    pub fn handle_interrupt(&mut self, in_signal_handler: bool) -> LispResult<()> {
        self.cancel_echoing();
        let quit_flag = self.plain_value(sym::QUIT_FLAG);
        let tty = if quit_flag.is_nil() {
            None
        } else {
            self.get_named_terminal(crate::term::DEV_TTY)
        };
        match tty {
            Some(t) => match t {},
            None => {
                let count = if quit_flag.is_nil() {
                    1
                } else {
                    self.keyboard.force_quit_count + 1
                };
                self.keyboard.force_quit_count = count;
                if count == 3 {
                    self.set_plain_value(sym::INHIBIT_QUIT, Value::NIL);
                }
                self.set_plain_value(sym::QUIT_FLAG, sym::T);
            }
        }
        if self.keyboard.waiting_for_input && !self.keyboard.echoing {
            return self.quit_throw_to_read_char(in_signal_handler);
        }
        Ok(())
    }

    pub fn quit_throw_to_read_char(&mut self, from_signal: bool) -> LispResult<()> {
        if !from_signal && self.plain_value(sym::QUIT_FLAG) == sym::KILL_EMACS {
            crate::emacs::f_kill_emacs(self, &[Value::NIL, Value::NIL])?;
        }
        self.keyboard.waiting_for_input = false;
        self.keyboard.input_pending = false;
        self.set_plain_value(sym::UNREAD_COMMAND_EVENTS, Value::NIL);
        let last = self.keyboard.internal_last_event_frame;
        if self.is_frame(last) && last != self.frames.selected_frame {
            let switch = self.make_lispy_switch_frame(last);
            self.do_switch_frame(switch, false, Value::NIL)?;
        }
        let tag = self.keyboard.getcjmp;
        if tag.is_nil() {
            panic!("emacs_abort: quit_throw_to_read_char outside read_char");
        }
        Err(self.throw(tag, tag))
    }

    fn process_special_events(&mut self) {
        if self.keyboard.kbd_buffer.iter().any(|event| {
            matches!(
                event.kind,
                EventKind::SelectionRequest | EventKind::SelectionClear
            )
        }) {
            unreachable!("selection events need pgtk_handle_selection_event");
        }
    }

    pub fn swallow_events(&mut self, do_display: bool) -> LispResult<()> {
        self.process_special_events();
        let old_timers_run = self.keyboard.timers_run;
        self.get_input_pending(READABLE_EVENTS_DO_TIMERS_NOW)?;
        if !self.keyboard.input_pending && self.keyboard.timers_run != old_timers_run && do_display
        {
            self.redisplay_preserve_echo_area(7)?;
        }
        Ok(())
    }

    pub fn requeued_events_pending_p(&self) -> bool {
        self.requeued_command_events_pending_p()
            || !self
                .plain_value(sym::UNREAD_POST_INPUT_METHOD_EVENTS)
                .is_nil()
            || !self.plain_value(sym::UNREAD_INPUT_METHOD_EVENTS).is_nil()
    }

    fn kbd_buffer_get_event_1(&mut self, arg: Value) -> LispResult {
        let coding_system =
            crate::textprop::f_get_text_property(self, &[Value::fixnum(0), sym::CODING, arg])?;
        if coding_system == sym::T {
            return Ok(arg);
        }
        let coding_system = if coding_system.is_nil() {
            self.find_symbol_value(sym::LOCALE_CODING_SYSTEM)
        } else {
            coding_system
        };
        self.code_convert_string(arg, coding_system, Value::NIL, false, false, false)
    }

    pub fn kbd_buffer_get_event(
        &mut self,
        used_mouse_menu: Option<&mut bool>,
        end_time: Option<Timespec>,
    ) -> LispResult<(Value, usize, Value, EventKind)> {
        let _ = used_mouse_menu;
        let mut event_frame = Value::NIL;
        let mut event_kind = EventKind::NoEvent;
        let mut kbp = self.current_kboard;
        loop {
            if self.plain_value(sym::UNREAD_COMMAND_EVENTS).is_cons() {
                break;
            }
            if !self.keyboard.kbd_buffer.is_empty() {
                break;
            }
            if self.some_mouse_moved().is_some() {
                break;
            }
            if !self.plain_value(sym::QUIT_FLAG).is_nil() {
                self.quit_throw_to_read_char(false)?;
            }
            if !self.keyboard.kbd_buffer.is_empty() {
                break;
            }
            if self.some_mouse_moved().is_some() {
                break;
            }
            match end_time {
                Some(end_time) => {
                    let now = current_timespec();
                    if end_time <= now {
                        return Ok((Value::NIL, kbp, event_frame, event_kind));
                    }
                    let (sec, nsec) = timespec_sub(end_time, now);
                    self.wait_reading_process_output(sec, nsec, -1, true, Value::NIL)?;
                }
                None => {
                    self.wait_reading_process_output(0, 0, -1, true, Value::NIL)?;
                }
            }
            if kbp != self.current_kboard {
                kbp = self.current_kboard;
                return Ok((Value::fixnum(-2), kbp, event_frame, event_kind));
            }
            if !self.keyboard.interrupt_input && self.keyboard.kbd_buffer.is_empty() {
                self.gobble_input()?;
            }
        }
        let unread = self.plain_value(sym::UNREAD_COMMAND_EVENTS);
        let obj;
        if unread.is_cons() {
            let first = self.xcar(unread);
            let rest = self.xcdr(unread);
            self.set_plain_value(sym::UNREAD_COMMAND_EVENTS, rest);
            return Ok((first, self.current_kboard, event_frame, event_kind));
        } else if let Some(&front) = self.keyboard.kbd_buffer.front() {
            let mut event = front;
            event_kind = event.kind;
            kbp = self.event_to_kboard(&event).unwrap_or(self.current_kboard);
            if !matches!(
                event.kind,
                EventKind::SelectionRequest | EventKind::SelectionClear
            ) && self.is_frame(event.frame_or_window)
            {
                event_frame = event.frame_or_window;
            }
            obj = match event.kind {
                EventKind::SelectionRequest | EventKind::SelectionClear => {
                    unreachable!("selection events need pgtk_handle_selection_event")
                }
                EventKind::MonitorsChanged => {
                    self.keyboard.kbd_buffer.pop_front();
                    self.keyboard.input_pending = self.readable_events(0)?;
                    let hook = sym::DISPLAY_MONITORS_CHANGED_FUNCTIONS;
                    crate::eval::f_run_hook_with_args(self, &[hook, event.arg])?;
                    Value::NIL
                }
                EventKind::ToolkitThemeChanged => {
                    self.keyboard.kbd_buffer.pop_front();
                    self.keyboard.input_pending = self.readable_events(0)?;
                    self.set_plain_value(sym::TOOLKIT_THEME, event.arg);
                    let hook = sym::TOOLKIT_THEME_SET_FUNCTIONS;
                    crate::eval::f_run_hook_with_args(self, &[hook, event.arg])?;
                    Value::NIL
                }
                EventKind::PreeditText
                | EventKind::DeleteWindow
                | EventKind::Iconify
                | EventKind::Deiconify
                | EventKind::MoveFrame
                | EventKind::FileNotify
                | EventKind::Thread
                | EventKind::SaveSession
                | EventKind::NoEvent
                | EventKind::Help
                | EventKind::FocusIn
                | EventKind::ConfigChanged
                | EventKind::FocusOut
                | EventKind::SelectWindow
                | EventKind::Sleep => {
                    let obj = self.make_lispy_event(&mut event)?;
                    self.keyboard.kbd_buffer.pop_front();
                    obj
                }
                _ => self.kbd_buffer_get_keyboard_event(event)?,
            };
        } else if let Some(moved) = self.some_mouse_moved() {
            event_frame = moved;
            kbp = self.current_kboard;
            let mut f = moved;
            let mut position = None;
            let terminal = self.frame(f).terminal;
            if let Some(hook) = self.terminal(terminal).hooks.mouse_position
                && let Some(p) = hook(self, f, 0)
            {
                f = p.frame;
                position = Some(p);
            }
            let mut movement = Value::NIL;
            if let Some(p) = position.filter(|p| !p.x.is_nil()) {
                if !f.is_nil() {
                    let mut frame = self.frame(f).focus_frame;
                    if frame.is_nil() {
                        frame = f;
                    }
                    if frame != self.keyboard.internal_last_event_frame
                        && frame != self.frames.selected_frame
                    {
                        movement = self.make_lispy_switch_frame(frame);
                    }
                    self.keyboard.internal_last_event_frame = frame;
                }
                if movement.is_nil() {
                    movement = self.make_lispy_movement(f, p.bar_window, p.x, p.y, p.time)?;
                }
            }
            if !movement.is_nil() {
                let device = self.frame(event_frame).last_mouse_device;
                let device = if device.is_string() {
                    device
                } else {
                    self.keyboard.virtual_core_pointer_name
                };
                self.set_plain_value(sym::LAST_EVENT_DEVICE, device);
            }
            obj = movement;
        } else {
            panic!("emacs_abort: kbd_buffer_get_event found no input");
        }
        self.keyboard.input_pending = self.readable_events(0)?;
        let last = self.keyboard.internal_last_event_frame;
        self.set_plain_value(sym::LAST_EVENT_FRAME, last);
        Ok((obj, kbp, event_frame, event_kind))
    }

    fn kbd_buffer_get_keyboard_event(&mut self, mut event: KbdEvent) -> LispResult {
        let mut frame = event.frame_or_window;
        if frame.is_cons() {
            frame = self.xcar(frame);
        } else if self.is_window(frame) {
            frame = self.window(frame).frame;
        }
        if self.is_frame(frame) {
            let focus = self.frame(frame).focus_frame;
            if !focus.is_nil() {
                frame = focus;
            }
        }
        let mut obj = Value::NIL;
        if frame != self.keyboard.internal_last_event_frame && frame != self.frames.selected_frame {
            obj = self.make_lispy_switch_frame(frame);
        }
        self.keyboard.internal_last_event_frame = frame;
        let device = if event.device == sym::T {
            if matches!(
                event.kind,
                EventKind::AsciiKeystroke
                    | EventKind::MultibyteCharKeystroke
                    | EventKind::NonAsciiKeystroke
            ) {
                self.keyboard.virtual_core_keyboard_name
            } else {
                self.keyboard.virtual_core_pointer_name
            }
        } else {
            event.device
        };
        self.set_plain_value(sym::LAST_EVENT_DEVICE, device);
        if !obj.is_nil() {
            return Ok(obj);
        }
        if event.kind == EventKind::Pinch {
            unreachable!("pinch events need make_lispy_position");
        }
        if event.kind == EventKind::MultibyteCharKeystroke && event.arg.is_string() {
            let base = self.stack.len();
            self.stack.push(event.arg);
            let decoded = self.condition_case_internal(
                sym::T,
                |l| l.kbd_buffer_get_event_1(event.arg),
                |_, _| Ok(Value::NIL),
            );
            self.stack.truncate(base);
            let mut str = decoded?;
            if str.is_nil() {
                str = event.arg;
            }
            if self.string_chars(str) == 0 {
                self.keyboard.kbd_buffer.pop_front();
                return Ok(Value::NIL);
            }
            event.arg = self.cons(Value::fixnum(0), str);
        }
        if event.kind == EventKind::MultibyteCharKeystroke && event.arg.is_cons() {
            let index = self.xcar(event.arg);
            let str = self.xcdr(event.arg);
            let c = crate::data::f_aref(self, &[str, index])?;
            event.code = c.as_fixnum().unwrap_or(0) as u32;
            let next = Value::fixnum(index.as_fixnum().unwrap_or(0) + 1);
            self.heap.set_car(event.arg, next);
        }
        let obj = self.make_lispy_event(&mut event)?;
        let done = event.kind != EventKind::MultibyteCharKeystroke
            || !event.arg.is_cons()
            || self.xcar(event.arg).as_fixnum().unwrap_or(0) as usize
                >= self.string_chars(self.xcdr(event.arg));
        if done {
            self.keyboard.kbd_buffer.pop_front();
        } else if let Some(front) = self.keyboard.kbd_buffer.front_mut() {
            *front = event;
        }
        Ok(obj)
    }
}

fn f_input_pending_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if lisp.requeued_events_pending_p() {
        return Ok(sym::T);
    }
    lisp.process_special_events();
    let timers = if args.first().is_none_or(|v| v.is_nil()) {
        0
    } else {
        READABLE_EVENTS_DO_TIMERS_NOW
    };
    let pending = lisp.get_input_pending(timers | READABLE_EVENTS_FILTER_EVENTS)?;
    Ok(lisp.bool(pending))
}

fn f_insert_special_event(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let event = args[0];
    if !event.is_cons() {
        return Err(lisp.wrong_type(sym::CONSP, event));
    }
    let head = lisp.xcar(event);
    let kind = match head {
        h if h == sym::DELETE_FRAME => EventKind::DeleteWindow,
        h if h == sym::ICONIFY_FRAME => EventKind::Iconify,
        h if h == sym::MAKE_FRAME_VISIBLE => EventKind::Deiconify,
        h if h == sym::SAVE_SESSION => EventKind::SaveSession,
        h if h == sym::CONFIG_CHANGED_EVENT => EventKind::ConfigChanged,
        h if h == sym::FOCUS_IN => EventKind::FocusIn,
        h if h == sym::FOCUS_OUT => EventKind::FocusOut,
        h if h == sym::MOVE_FRAME => EventKind::MoveFrame,
        h if h == sym::SLEEP_EVENT => EventKind::Sleep,
        _ => return Err(lisp.signal_error("Invalid special event kind", head)),
    };
    let ie = KbdEvent {
        kind,
        frame_or_window: Value::NIL,
        arg: lisp.xcdr(event),
        ..KbdEvent::default()
    };
    lisp.kbd_buffer_store_event(ie)?;
    Ok(Value::NIL)
}
