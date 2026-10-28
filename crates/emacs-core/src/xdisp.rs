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
    let name = lisp.string("*Messages*");
    lisp.defvar(sym::MESSAGES_BUFFER_NAME, name);
    lisp.defvar_bool(sym::INHIBIT_REDISPLAY, false);
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
