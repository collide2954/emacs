use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub const REDISPLAY_SOME: i64 = 2;

#[derive(Default)]
pub struct Xdisp {
    pub message_log_need_newline: bool,
}

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_bool(sym::SCROLL_MINIBUFFER_CONSERVATIVELY, true);
    lisp.defvar_bool(sym::INHIBIT_MESSAGE, false);
    let name = lisp.string("*Messages*");
    lisp.defvar(sym::MESSAGES_BUFFER_NAME, name);
    lisp.defvar_bool(sym::X_STRETCH_CURSOR, false);
    lisp.defvar(sym::SHOW_TRAILING_WHITESPACE, Value::NIL);
    lisp.defvar(sym::MODE_LINE_COMPACT, Value::NIL);
    lisp.defvar(sym::NOBREAK_CHAR_DISPLAY, sym::T);
    lisp.defvar_bool(sym::NOBREAK_CHAR_ASCII_DISPLAY, false);
    lisp.defvar(sym::VOID_TEXT_AREA_POINTER, sym::ARROW);
    lisp.defvar(sym::INHIBIT_REDISPLAY, Value::NIL);
    lisp.defvar(sym::GLOBAL_MODE_STRING, Value::NIL);
    lisp.defvar(sym::OVERLAY_ARROW_POSITION, Value::NIL);
    let arrow = lisp.string("=>");
    lisp.defvar(sym::OVERLAY_ARROW_STRING, arrow);
    let arrows = lisp.list(&[sym::OVERLAY_ARROW_POSITION]);
    lisp.defvar(sym::OVERLAY_ARROW_VARIABLE_LIST, arrows);
    lisp.defvar_int(sym::SCROLL_STEP, 0);
    lisp.defvar_int(sym::SCROLL_CONSERVATIVELY, 0);
    lisp.defvar_int(sym::SCROLL_MARGIN, 0);
    let margin = lisp.float(0.25);
    lisp.defvar(sym::MAXIMUM_SCROLL_MARGIN, margin);
    let ppi = lisp.float(72.0);
    lisp.defvar(sym::DISPLAY_PIXELS_PER_INCH, ppi);
    lisp.defvar_int(sym::DEBUG_END_POS, 0);
    lisp.defvar(sym::TRUNCATE_PARTIAL_WIDTH_WINDOWS, Value::fixnum(50));
    lisp.defvar_bool(sym::WORD_WRAP_BY_CATEGORY, false);
    lisp.defvar(sym::LINE_NUMBER_DISPLAY_LIMIT, Value::NIL);
    lisp.defvar_int(sym::LINE_NUMBER_DISPLAY_LIMIT_WIDTH, 200);
    lisp.defvar_bool(sym::HIGHLIGHT_NONSELECTED_WINDOWS, false);
    lisp.defvar_bool(sym::MULTIPLE_FRAMES, false);
    let empty = lisp.empty_unibyte;
    let at = lisp.string("%b - GNU Emacs at ");
    lisp.stack.push(at);
    let icon_title_name_format = lisp.list(&[empty, at, sym::SYSTEM_NAME]);
    lisp.stack.push(icon_title_name_format);
    let b = lisp.string("%b");
    let format = lisp.list(&[sym::MULTIPLE_FRAMES, b, icon_title_name_format]);
    lisp.stack.truncate(lisp.stack.len() - 2);
    lisp.defvar(sym::FRAME_TITLE_FORMAT, format);
    lisp.defvar(sym::ICON_TITLE_FORMAT, format);
    lisp.defvar(sym::MESSAGE_LOG_MAX, Value::fixnum(1000));
    lisp.defvar(sym::WINDOW_SCROLL_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::MOUSE_AUTOSELECT_WINDOW, Value::NIL);
    lisp.defvar(sym::AUTO_RESIZE_TAB_BARS, sym::T);
    lisp.defvar_bool(sym::AUTO_RAISE_TAB_BAR_BUTTONS, true);
    lisp.defvar(sym::AUTO_RESIZE_TOOL_BARS, sym::T);
    lisp.defvar_bool(sym::AUTO_RAISE_TOOL_BAR_BUTTONS, true);
    lisp.defvar(sym::MAKE_CURSOR_LINE_FULLY_VISIBLE, sym::T);
    lisp.defvar_bool(sym::MAKE_WINDOW_START_VISIBLE, false);
    lisp.make_variable_buffer_local_builtin(sym::MAKE_WINDOW_START_VISIBLE);
    lisp.defvar_bool(sym::TAB_BAR_TRUNCATE, false);
    lisp.defvar(sym::TAB_BAR_BORDER, sym::INTERNAL_BORDER_WIDTH);
    lisp.defvar(sym::TAB_BAR_BUTTON_MARGIN, Value::fixnum(1));
    lisp.defvar_int(sym::TAB_BAR_BUTTON_RELIEF, 1);
    lisp.defvar(sym::TOOL_BAR_BORDER, sym::INTERNAL_BORDER_WIDTH);
    lisp.defvar(sym::TOOL_BAR_BUTTON_MARGIN, Value::fixnum(4));
    lisp.defvar_int(sym::TOOL_BAR_BUTTON_RELIEF, 1);
    lisp.defvar(sym::TOOL_BAR_STYLE, Value::NIL);
    lisp.defvar_int(sym::TOOL_BAR_MAX_LABEL_SIZE, 14);
    lisp.defvar(sym::FONTIFICATION_FUNCTIONS, Value::NIL);
    lisp.make_variable_buffer_local_builtin(sym::FONTIFICATION_FUNCTIONS);
    lisp.defvar_bool(sym::UNIBYTE_DISPLAY_VIA_LANGUAGE_ENVIRONMENT, false);
    let height = lisp.float(0.25);
    lisp.defvar(sym::MAX_MINI_WINDOW_HEIGHT, height);
    lisp.defvar(sym::RESIZE_MINI_WINDOWS, Value::NIL);
    lisp.defvar(sym::BLINK_CURSOR_ALIST, Value::NIL);
    lisp.defvar(sym::AUTO_HSCROLL_MODE, sym::T);
    lisp.defvar_int(sym::HSCROLL_MARGIN, 5);
    lisp.defvar(sym::HSCROLL_STEP, Value::fixnum(0));
    lisp.defvar_bool(sym::MESSAGE_TRUNCATE_LINES, false);
    lisp.defvar(sym::MENU_BAR_UPDATE_HOOK, Value::NIL);
    lisp.defvar(sym::MENU_UPDATING_FRAME, Value::NIL);
    lisp.defvar_bool(sym::INHIBIT_MENUBAR_UPDATE, false);
    for symbol in [
        sym::WRAP_PREFIX,
        sym::LINE_PREFIX,
        sym::DISPLAY_LINE_NUMBERS,
        sym::DISPLAY_LINE_NUMBERS_WIDTH,
    ] {
        lisp.defvar(symbol, Value::NIL);
        lisp.make_variable_buffer_local_builtin(symbol);
    }
    lisp.defvar(sym::DISPLAY_LINE_NUMBERS_CURRENT_ABSOLUTE, sym::T);
    lisp.defvar_bool(sym::DISPLAY_LINE_NUMBERS_WIDEN, false);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_LINE_NUMBERS_WIDEN);
    lisp.defvar_int(sym::DISPLAY_LINE_NUMBERS_OFFSET, 0);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_LINE_NUMBERS_OFFSET);
    lisp.defvar_bool(sym::DISPLAY_FILL_COLUMN_INDICATOR, false);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_FILL_COLUMN_INDICATOR);
    lisp.defvar(sym::DISPLAY_FILL_COLUMN_INDICATOR_COLUMN, sym::T);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_FILL_COLUMN_INDICATOR_COLUMN);
    lisp.defvar(sym::DISPLAY_FILL_COLUMN_INDICATOR_CHARACTER, Value::NIL);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_FILL_COLUMN_INDICATOR_CHARACTER);
    lisp.defvar_int(sym::DISPLAY_LINE_NUMBERS_MAJOR_TICK, 0);
    lisp.defvar_int(sym::DISPLAY_LINE_NUMBERS_MINOR_TICK, 0);
    for symbol in [
        sym::INHIBIT_EVAL_DURING_REDISPLAY,
        sym::INHIBIT_FREE_REALIZED_FACES,
        sym::INHIBIT_BIDI_MIRRORING,
        sym::BIDI_INHIBIT_BPA,
        sym::INHIBIT_TRY_WINDOW_ID,
        sym::INHIBIT_TRY_WINDOW_REUSING,
        sym::INHIBIT_TRY_CURSOR_MOVEMENT,
    ] {
        lisp.defvar_bool(symbol, false);
    }
    lisp.defvar_int(sym::OVERLINE_MARGIN, 2);
    lisp.defvar_int(sym::UNDERLINE_MINIMUM_OFFSET, 1);
    lisp.defvar_bool(sym::UNDERLINE_LINE_SCALING_FLAG, false);
    lisp.defvar_bool(sym::OVERLINE_LINE_SCALING_FLAG, false);
    lisp.defvar_bool(sym::STRIKE_THROUGH_LINE_SCALING_FLAG, false);
    lisp.defvar_bool(sym::DISPLAY_HOURGLASS, true);
    lisp.defvar(sym::HOURGLASS_DELAY, Value::fixnum(1));
    let ignore = lisp.intern("ignore");
    lisp.defvar(sym::PRE_REDISPLAY_FUNCTION, ignore);
    lisp.put(
        sym::GLYPHLESS_CHAR_DISPLAY,
        sym::CHAR_TABLE_EXTRA_SLOTS,
        Value::fixnum(1),
    );
    let glyphless = lisp
        .make_char_table(sym::GLYPHLESS_CHAR_DISPLAY, Value::NIL)
        .expect("glyphless-char-display");
    lisp.char_table_mut(glyphless).extras[0] = sym::EMPTY_BOX;
    lisp.defvar(sym::GLYPHLESS_CHAR_DISPLAY, glyphless);
    let mirror = lisp
        .make_char_table(Value::NIL, Value::NIL)
        .expect("special-mirror-table");
    lisp.defvar(sym::SPECIAL_MIRROR_TABLE, mirror);
    lisp.defvar(sym::DEBUG_ON_MESSAGE, Value::NIL);
    lisp.defvar(sym::SET_MESSAGE_FUNCTION, Value::NIL);
    lisp.defvar(sym::CLEAR_MESSAGE_FUNCTION, Value::NIL);
    let all_windows = lisp.make_hash_table_with(sym::EQL, crate::object::Weakness::None);
    lisp.defvar(sym::REDISPLAY__ALL_WINDOWS_CAUSE, all_windows);
    let mode_lines = lisp.make_hash_table_with(sym::EQL, crate::object::Weakness::None);
    lisp.defvar(sym::REDISPLAY__MODE_LINES_CAUSE, mode_lines);
    lisp.defvar_bool(sym::REDISPLAY__INHIBIT_BIDI, true);
    lisp.defvar_bool(sym::DISPLAY_RAW_BYTES_AS_HEX, false);
    lisp.defvar_bool(sym::MOUSE_FINE_GRAINED_TRACKING, false);
    lisp.defvar_bool(sym::TAB_BAR__DRAGGING_IN_PROGRESS, false);
    lisp.defvar_bool(sym::REDISPLAY_SKIP_INITIAL_FRAME, true);
    lisp.defvar_bool(sym::REDISPLAY_SKIP_FONTIFICATION_ON_INPUT, false);
    lisp.defvar_bool(sym::REDISPLAY_ADHOC_SCROLL_IN_RESIZE_MINI_WINDOWS, true);
    lisp.defvar_bool(sym::COMPOSITION_BREAK_AT_POINT, false);
    lisp.defvar_int(sym::MAX_REDISPLAY_TICKS, 0);
}

impl Lisp {
    pub fn redisplay_other_windows(&mut self) {
        if self.windows.windows_or_buffers_changed == 0 {
            self.windows.windows_or_buffers_changed = REDISPLAY_SOME;
        }
    }

    pub fn wset_redisplay(&mut self, w: Value) {
        if w != self.windows.selected_window {
            self.redisplay_other_windows();
        }
        self.window_mut(w).flags.redisplay = true;
    }

    pub fn fset_redisplay(&mut self, f: Value) {
        self.redisplay_other_windows();
        self.frame_mut(f).flags.redisplay = true;
    }

    pub fn bset_redisplay(&mut self, b: Value) {
        let count = self.buffer(b).window_count;
        if count > 0 {
            let sw = self.windows.selected_window;
            if count > 1 || !self.is_window(sw) || b != self.window(sw).contents {
                self.redisplay_other_windows();
            }
            self.buffer_mut(b).text.redisplay = true;
        }
    }

    pub fn bset_update_mode_line(&mut self, b: Value) {
        if self.windows.update_mode_lines == 0 {
            self.windows.update_mode_lines = REDISPLAY_SOME;
        }
        self.buffer_mut(b).text.redisplay = true;
    }

    pub fn wset_update_mode_line(&mut self, w: Value) {
        self.window_mut(w).flags.update_mode_line = true;
        self.wset_redisplay(w);
    }

    pub fn add_to_log(&mut self, format: &str, args: &[Value]) -> LispResult<()> {
        let base = self.stack.len();
        let fmt = self.string(format);
        self.stack.push(fmt);
        self.stack.extend_from_slice(args);
        let all = self.stack[base..].to_vec();
        let r = self.format_string(&all, true);
        self.stack.truncate(base);
        let msg = r?;
        let bytes = self.string_bytes(msg).to_vec();
        let multibyte = self.string_multibyte(msg);
        self.message_dolog(&bytes, true, multibyte)
    }

    fn message_log_insert(&mut self, bytes: &[u8], nchars: usize, before_markers: bool) {
        let (pt, pt_byte) = (self.cur().pt, self.cur().pt_byte);
        let multibyte = self.cur().text.multibyte;
        self.insert_from_gap(pt, pt_byte, bytes, nchars, before_markers);
        let n = if multibyte { nchars } else { bytes.len() };
        let b = self.cur_mut();
        b.pt = pt + n;
        b.pt_byte = pt_byte + bytes.len();
    }

    fn temp_set_pt(&mut self, pos: usize) {
        let b = self.cur_mut();
        b.pt = pos;
        b.pt_byte = b.text.char_to_byte(pos);
    }

    pub fn message_dolog(&mut self, msg: &[u8], nlflag: bool, multibyte: bool) -> LispResult<()> {
        if !self.find_symbol_value(sym::MEMORY_FULL).is_nil() {
            return Ok(());
        }
        if self.find_symbol_value(sym::MESSAGE_LOG_MAX).is_nil() {
            return Ok(());
        }
        let old_windows_or_buffers_changed = self.windows.windows_or_buffers_changed;
        let old_deactivate_mark = self.bound_var(sym::DEACTIVATE_MARK);
        let oldbuf = self.current_buffer;
        let base = self.stack.len();
        self.stack.push(old_deactivate_mark);
        self.stack.push(oldbuf);
        let r = self.message_dolog_1(msg, nlflag, multibyte, old_windows_or_buffers_changed);
        self.stack.truncate(base);
        r?;
        self.set_buffer_internal(oldbuf);
        self.xdisp.message_log_need_newline = !nlflag;
        self.set_plain_value(sym::DEACTIVATE_MARK, old_deactivate_mark);
        Ok(())
    }

    fn message_dolog_1(
        &mut self,
        msg: &[u8],
        nlflag: bool,
        multibyte: bool,
        old_windows_or_buffers_changed: i64,
    ) -> LispResult<()> {
        if !self
            .find_symbol_value(sym::MESSAGES_BUFFER_NAME)
            .is_string()
        {
            let name = self.string("*Messages*");
            self.set_plain_value(sym::MESSAGES_BUFFER_NAME, name);
        }
        let name = self.find_symbol_value(sym::MESSAGES_BUFFER_NAME);
        let newbuffer = crate::buffer::f_get_buffer(self, &[name])?.is_nil();
        let name = self.find_symbol_value(sym::MESSAGES_BUFFER_NAME);
        let buf = self.get_buffer_create(name, false)?;
        crate::buffer::f_set_buffer(self, &[buf])?;
        if newbuffer
            && !self
                .heap
                .symbol_ref(sym::MESSAGES_BUFFER_MODE)
                .function
                .is_nil()
        {
            self.funcall(sym::MESSAGES_BUFFER_MODE, &[])?;
        }
        self.set_undo_list(sym::T);
        let cb = self.current_buffer;
        self.buffer_local_set(cb, sym::CACHE_LONG_SCANS, Value::NIL);
        let (pt, begv, zv) = (self.cur().pt, self.cur().begv, self.cur().zv);
        let oldpoint = self.make_marker_at(pt, cb, false);
        let oldbegv = self.make_marker_at(begv, cb, false);
        let oldzv = self.make_marker_at(zv, cb, false);
        let z = self.cur().z();
        let point_at_end = pt == z;
        let zv_at_end = zv == z;
        {
            let b = self.cur_mut();
            let (z, z_byte) = (b.z(), b.z_byte());
            b.begv = 1;
            b.begv_byte = 1;
            b.zv = z;
            b.zv_byte = z_byte;
            b.pt = z;
            b.pt_byte = z_byte;
        }
        let buffer_multibyte = self.cur().text.multibyte;
        if multibyte && !buffer_multibyte {
            let bytes: Vec<u8> = character::multibyte_chars(msg)
                .map(character::char_to_byte8)
                .collect();
            for b in bytes {
                self.message_log_insert(&[b], 1, false);
            }
        } else if !multibyte && buffer_multibyte {
            for &b in msg {
                let s = character::str_to_multibyte(&[b]);
                self.message_log_insert(&s, 1, false);
            }
        } else if !msg.is_empty() {
            let nchars = if buffer_multibyte {
                character::chars_in_multibyte(msg)
            } else {
                msg.len()
            };
            self.message_log_insert(msg, nchars, false);
        }
        if nlflag {
            let count = self.specpdl.len();
            self.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
            let r = self.message_dolog_newline();
            let u = self.unbind_to(count);
            r?;
            u?;
        }
        let begv = self.marker(oldbegv).charpos;
        let zv = self.marker(oldzv).charpos;
        let point = self.marker(oldpoint).charpos;
        {
            let b = self.cur_mut();
            b.begv = begv;
            b.begv_byte = b.text.char_to_byte(begv);
            if zv_at_end {
                b.zv = b.z();
                b.zv_byte = b.z_byte();
            } else {
                b.zv = zv;
                b.zv_byte = b.text.char_to_byte(zv);
            }
        }
        if point_at_end {
            let z = self.cur().z();
            self.temp_set_pt(z);
        } else {
            self.temp_set_pt(point);
        }
        self.unchain_marker(oldpoint);
        self.unchain_marker(oldbegv);
        self.unchain_marker(oldzv);
        self.windows.windows_or_buffers_changed = old_windows_or_buffers_changed;
        let cb = self.current_buffer;
        self.bset_redisplay(cb);
        Ok(())
    }

    fn message_dolog_newline(&mut self) -> LispResult<()> {
        self.message_log_insert(b"\n", 1, false);
        let z = self.cur().z();
        let (this_bol, _) = self.find_newline(z, 1, -2);
        self.temp_set_pt(this_bol);
        let this_bol_byte = self.cur().pt_byte;
        if this_bol > 1 {
            let (prev_bol, _) = self.find_newline(this_bol, 1, -2);
            self.temp_set_pt(prev_bol);
            let prev_bol_byte = self.cur().pt_byte;
            let dups = self.message_log_check_duplicate(prev_bol_byte, this_bol_byte);
            if dups != 0 {
                self.del_range_1(prev_bol, this_bol, false)?;
                if dups > 1 {
                    let dupstr = format!(" [{dups} times]");
                    let z = self.cur().z();
                    self.temp_set_pt(z - 1);
                    self.message_log_insert(dupstr.as_bytes(), dupstr.len(), true);
                }
            }
        }
        let max = self.find_symbol_value(sym::MESSAGE_LOG_MAX);
        if let Some(n) = max.as_fixnum().filter(|&n| n >= 0) {
            let z = self.cur().z();
            let (pos, _) = self.find_newline(z, 1, -n - 1);
            self.temp_set_pt(pos);
            self.del_range_1(1, pos, false)?;
        }
        Ok(())
    }

    fn message_log_check_duplicate(&self, prev_bol_byte: usize, this_bol_byte: usize) -> i64 {
        let b = self.cur();
        let z_byte = b.z_byte();
        let tail = b.text.slice(prev_bol_byte - 1, z_byte - 1);
        let len = z_byte - 1 - this_bol_byte;
        let offset = this_bol_byte - prev_bol_byte;
        let p1 = &tail[..];
        let p2 = &tail[offset..];
        let mut seen_dots = false;
        for i in 0..len {
            if i >= 3 && p1[i - 3] == b'.' && p1[i - 2] == b'.' && p1[i - 1] == b'.' {
                seen_dots = true;
            }
            if p1[i] != p2[i] {
                return i64::from(seen_dots);
            }
        }
        let rest = &p1[len..];
        if rest.first() == Some(&b'\n') {
            return 2;
        }
        if rest.first() == Some(&b' ') && rest.get(1) == Some(&b'[') {
            let digits = &rest[2..];
            let mut i = 0;
            while digits.get(i).is_some_and(|c| c.is_ascii_whitespace()) {
                i += 1;
            }
            let start = i;
            if matches!(digits.get(i), Some(b'+') | Some(b'-')) {
                i += 1;
            }
            let num_start = i;
            while digits.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i > num_start {
                let n = std::str::from_utf8(&digits[start..i])
                    .ok()
                    .and_then(|s| s.trim_start().parse::<i64>().ok())
                    .unwrap_or(i64::MAX);
                if 0 < n && n < i64::MAX && digits[i..].starts_with(b" times]\n") {
                    return n + 1;
                }
            }
        }
        0
    }
}
