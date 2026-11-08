use crate::dispextern::{It, LineWrap, MOVE_TO_POS, TextPos};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::terminal::OutputMethod;
use crate::value::Value;
use crate::xfaces::DEFAULT_FACE_ID;

#[derive(Clone, Copy)]
enum EchoFn {
    DisplayEchoArea(Value),
    ResizeMiniWindow(Value, bool),
    CurrentMessage,
    TruncateMessage(usize),
    SetMessage(Value),
}

impl Lisp {
    pub fn ensure_echo_area_buffers(&mut self) -> LispResult<()> {
        for i in 0..2 {
            let old_buffer = self.xdisp.echo_buffer[i];
            if !self.is_buffer(old_buffer) || !self.buffer_live_p(old_buffer) {
                let name = self.string(&format!(" *Echo Area {i}*"));
                let buffer = self.get_buffer_create(name, false)?;
                self.xdisp.echo_buffer[i] = buffer;
                self.buffer_local_set(buffer, sym::TRUNCATE_LINES, Value::NIL);
                for j in 0..2 {
                    if old_buffer == self.xdisp.echo_area_buffer[j] {
                        self.xdisp.echo_area_buffer[j] = buffer;
                    }
                }
            }
        }
        Ok(())
    }

    fn with_echo_area_buffer(
        &mut self,
        w: Option<Value>,
        which: i32,
        f: EchoFn,
    ) -> LispResult<(bool, Value)> {
        let count = self.specpdl_index();
        self.ensure_echo_area_buffers()?;
        let mut clear_buffer_p = false;
        let (this_one, the_other) = if which == 0 {
            (0, 1)
        } else if which > 0 {
            (1, 0)
        } else {
            clear_buffer_p = true;
            let ea = self.xdisp.echo_area_buffer;
            if !ea[0].is_nil() && ea[0] == ea[1] {
                self.xdisp.echo_area_buffer[0] = Value::NIL;
            }
            (0, 1)
        };
        if self.xdisp.echo_area_buffer[this_one].is_nil() {
            self.xdisp.echo_area_buffer[this_one] =
                if self.xdisp.echo_area_buffer[the_other] == self.xdisp.echo_buffer[this_one] {
                    self.xdisp.echo_buffer[the_other]
                } else {
                    self.xdisp.echo_buffer[this_one]
                };
            clear_buffer_p = true;
        }
        let buffer = self.xdisp.echo_area_buffer[this_one];
        if self.keyboard.echo_kboard.is_none() && buffer == self.keyboard.echo_message_buffer {
            self.cancel_echoing();
        }
        let data = self.with_echo_area_buffer_unwind_data(w);
        self.record_unwind(unwind_with_echo_area_buffer, data);
        self.set_buffer_internal(buffer);
        if let Some(w) = w {
            self.window_mut(w).contents = buffer;
            let (pointm, old_pointm) = (self.window(w).pointm, self.window(w).old_pointm);
            self.set_marker_position(pointm, 1, buffer);
            self.set_marker_position(old_pointm, 1, buffer);
        }
        self.buffer_local_set(buffer, sym::BUFFER_UNDO_LIST, sym::T);
        self.buffer_local_set(buffer, sym::BUFFER_READ_ONLY, Value::NIL);
        self.specbind(sym::INHIBIT_READ_ONLY, sym::T)?;
        self.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
        let z = self.cur().z();
        if clear_buffer_p && z > 1 {
            self.del_range(1, z)?;
        }
        let r = match f {
            EchoFn::DisplayEchoArea(w) => self.display_echo_area_1(w).map(|b| (b, Value::NIL)),
            EchoFn::ResizeMiniWindow(w, exactly) => {
                self.resize_mini_window(w, exactly).map(|b| (b, Value::NIL))
            }
            EchoFn::CurrentMessage => Ok((false, self.current_message_1())),
            EchoFn::TruncateMessage(nchars) => self
                .truncate_message_1(nchars)
                .map(|()| (false, Value::NIL)),
            EchoFn::SetMessage(string) => self.set_message_1(string).map(|()| (false, Value::NIL)),
        };
        let (rc, value) = r?;
        let base = self.stack.len();
        self.stack.push(value);
        self.unbind_to(count)?;
        self.stack.truncate(base);
        Ok((rc, value))
    }

    fn with_echo_area_buffer_unwind_data(&mut self, w: Option<Value>) -> Value {
        let mut vector = self.xdisp.with_echo_area_save_vector;
        self.xdisp.with_echo_area_save_vector = Value::NIL;
        if vector.is_nil() {
            vector = self.vector(vec![Value::NIL; 11]);
        }
        let items = {
            let mut items = vec![
                self.current_buffer,
                self.find_symbol_value(sym::DEACTIVATE_MARK),
                Value::fixnum(self.windows.windows_or_buffers_changed),
            ];
            match w {
                Some(w) => {
                    let win = self.window(w);
                    let (pointm, old_pointm, start) = (
                        *self.marker(win.pointm),
                        *self.marker(win.old_pointm),
                        *self.marker(win.start),
                    );
                    items.extend_from_slice(&[
                        w,
                        win.contents,
                        Value::fixnum(pointm.charpos as i64),
                        Value::fixnum(pointm.bytepos as i64),
                        Value::fixnum(old_pointm.charpos as i64),
                        Value::fixnum(old_pointm.bytepos as i64),
                        Value::fixnum(start.charpos as i64),
                        Value::fixnum(start.bytepos as i64),
                    ]);
                }
                None => items.extend_from_slice(&[Value::NIL; 8]),
            }
            items
        };
        for (i, item) in items.into_iter().enumerate() {
            self.vector_set(vector, i, item);
        }
        vector
    }

    pub fn setup_echo_area_for_printing(&mut self, multibyte_p: bool) -> LispResult<()> {
        let sf = self.frames.selected_frame;
        if !self.frame_live_p(sf) {
            crate::emacs::f_kill_emacs(self, &[Value::NIL, Value::NIL])?;
        }
        self.ensure_echo_area_buffers()?;
        if !self.xdisp.message_buf_print {
            self.xdisp.echo_area_buffer[0] =
                if self.xdisp.echo_area_buffer[1] == self.xdisp.echo_buffer[0] {
                    self.xdisp.echo_buffer[1]
                } else {
                    self.xdisp.echo_buffer[0]
                };
            let buffer = self.xdisp.echo_area_buffer[0];
            self.set_buffer_internal(buffer);
            self.buffer_local_set(buffer, sym::TRUNCATE_LINES, Value::NIL);
            let z = self.cur().z();
            if z > 1 {
                let count = self.specpdl_index();
                self.specbind(sym::INHIBIT_READ_ONLY, sym::T)?;
                self.del_range(1, z)?;
                self.unbind_to(count)?;
            }
            self.set_pt_both(1, 1);
            let multibyte = !self
                .find_symbol_value(sym::ENABLE_MULTIBYTE_CHARACTERS)
                .is_nil();
            if self.var_true(sym::UNIBYTE_DISPLAY_VIA_LANGUAGE_ENVIRONMENT)
                && !multibyte_p
                && multibyte
            {
                crate::buffer::f_set_buffer_multibyte(self, &[Value::NIL])?;
            } else if !multibyte {
                crate::buffer::f_set_buffer_multibyte(self, &[sym::T])?;
            }
            if self.var_true(sym::MINIBUFFER_AUTO_RAISE) {
                let mini_window = self.frame(sf).minibuffer_window;
                let frame = self.window(mini_window).frame;
                crate::frame::f_raise_frame(self, &[frame])?;
            }
            self.message_log_maybe_newline()?;
            self.xdisp.message_buf_print = true;
        } else {
            if self.xdisp.echo_area_buffer[0].is_nil() {
                self.xdisp.echo_area_buffer[0] =
                    if self.xdisp.echo_area_buffer[1] == self.xdisp.echo_buffer[0] {
                        self.xdisp.echo_buffer[1]
                    } else {
                        self.xdisp.echo_buffer[0]
                    };
            }
            let buffer = self.xdisp.echo_area_buffer[0];
            if self.current_buffer != buffer {
                self.set_buffer_internal(buffer);
                self.buffer_local_set(buffer, sym::TRUNCATE_LINES, Value::NIL);
            }
        }
        Ok(())
    }

    fn display_echo_area(&mut self, w: Value) -> LispResult<bool> {
        self.gc_inhibited += 1;
        let i = usize::from(self.xdisp.display_last_displayed_message_p);
        let no_message_p = self.xdisp.echo_area_buffer[i].is_nil();
        let r = self.with_echo_area_buffer(Some(w), i as i32, EchoFn::DisplayEchoArea(w));
        if r.is_ok() && no_message_p {
            self.xdisp.echo_area_buffer[i] = Value::NIL;
        }
        self.gc_inhibited -= 1;
        Ok(r?.0)
    }

    fn display_echo_area_1(&mut self, w: Value) -> LispResult<bool> {
        self.forget_escape_and_glyphless_faces();
        let window_height_changed_p = self.resize_mini_window(w, false)?;
        let start = {
            let m = self.marker(self.window(w).start);
            TextPos::new(m.charpos as i64, m.bytepos as i64)
        };
        if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
            crate::dispnew::matrix::clear_glyph_matrix(m);
        }
        let itdata = self.bidi_shelve_cache();
        self.try_window(w, start, 0)?;
        self.bidi_unshelve_cache(itdata, false);
        Ok(window_height_changed_p)
    }

    pub fn resize_echo_area_exactly(&mut self) -> LispResult<()> {
        let window = self.xdisp.echo_area_window;
        if self.is_buffer(self.xdisp.echo_area_buffer[0]) && self.is_window(window) {
            let exactly = self.minibuf.minibuf_level == 0;
            let (resized_p, _) = self.with_echo_area_buffer(
                Some(window),
                0,
                EchoFn::ResizeMiniWindow(window, exactly),
            )?;
            if resized_p {
                self.windows.windows_or_buffers_changed = 42;
                self.windows.update_mode_lines = 30;
                self.redisplay_internal()?;
            }
        }
        Ok(())
    }

    pub fn resize_mini_window(&mut self, w: Value, exact_p: bool) -> LispResult<bool> {
        self.with_gc_deferred(|lisp| lisp.resize_mini_window_1(w, exact_p))
    }

    fn resize_mini_window_1(&mut self, w: Value, exact_p: bool) -> LispResult<bool> {
        let f = self.window(w).frame;
        let old_height = self.window_box_text_height(w);
        if !self.find_symbol_value(sym::INHIBIT_REDISPLAY).is_nil() {
            return Ok(false);
        }
        if self.var_true(sym::REDISPLAY_ADHOC_SCROLL_IN_RESIZE_MINI_WINDOWS) {
            let (start, contents) = (self.window(w).start, self.window(w).contents);
            let begv = self.buffer(contents).begv;
            self.set_marker_position(start, begv, contents);
        }
        let minibuf_only = self.frame_minibuf_only_p(f);
        let resize_mini_frames = self.find_symbol_value(sym::RESIZE_MINI_FRAMES);
        let resize_mini_windows = self.find_symbol_value(sym::RESIZE_MINI_WINDOWS);
        if resize_mini_windows.is_nil() && (resize_mini_frames.is_nil() || !minibuf_only) {
            return Ok(false);
        }
        if minibuf_only {
            if !resize_mini_frames.is_nil() {
                self.safe_call(true, &[sym::WINDOW__RESIZE_MINI_FRAME, f])?;
            }
        } else {
            let windows_height = self.frame_inner_height(f) as i32;
            let contents = self.window(w).contents;
            let mut old_current_buffer = None;
            if self.current_buffer != contents {
                old_current_buffer = Some(self.current_buffer);
                self.set_buffer_internal(contents);
            }
            let (begv, begv_byte) = (self.cur().begv as i64, self.cur().begv_byte as i64);
            let mut it = It::default();
            self.init_iterator(&mut it, w, begv, begv_byte, None, DEFAULT_FACE_ID)?;
            let unit = self.frame(f).geometry.line_height as i32
                + if it.extra_line_spacing_above != 0 {
                    it.extra_line_spacing
                } else {
                    0
                };
            let max_mini = self.find_symbol_value(sym::MAX_MINI_WINDOW_HEIGHT);
            let mut max_height = if max_mini.is_float() {
                (self.heap.float_value(max_mini) * f64::from(windows_height)) as i32
            } else if let Some(n) = max_mini.as_fixnum() {
                (n as i32).saturating_mul(unit)
            } else {
                windows_height / 4
            };
            max_height = max_height.clamp(unit.min(windows_height), windows_height.max(unit));
            self.xdisp.last_height = 0;
            let zv = self.cur().zv as i64;
            self.move_it_to(&mut it, zv, -1, -1, -1, MOVE_TO_POS)?;
            let mut height = if it.max_ascent == 0 && it.max_descent == 0 {
                let mut height = it.current_y;
                let zv_byte = self.cur().zv_byte;
                if !(it.line_wrap == LineWrap::Truncate
                    && it.current_x <= it.first_visible_x
                    && zv_byte > 1
                    && self.cur().text.byte_at(zv_byte - 1) != b'\n')
                {
                    height += self.xdisp.last_height;
                }
                height
            } else {
                it.current_y + it.max_ascent + it.max_descent
            };
            if it.extra_line_spacing_above == 0 {
                height -= it.extra_line_spacing.min(it.max_extra_line_spacing);
            }
            if height > max_height {
                height = (max_height / unit) * unit;
                if self.var_true(sym::REDISPLAY_ADHOC_SCROLL_IN_RESIZE_MINI_WINDOWS) {
                    let zv_byte = self.cur().zv_byte as i64;
                    it = It::default();
                    self.init_iterator(&mut it, w, zv, zv_byte, None, DEFAULT_FACE_ID)?;
                    self.move_it_vertically_backward(&mut it, height - unit)?;
                    self.move_it_by_lines(&mut it, 0)?;
                    let start = it.current.pos;
                    self.window_mut(w).flags.start_at_line_beg = false;
                    let marker = self.window(w).start;
                    let cb = self.current_buffer;
                    self.set_marker_position(marker, start.charpos as usize, cb);
                }
            } else {
                let marker = self.window(w).start;
                let cb = self.current_buffer;
                self.set_marker_position(marker, begv as usize, cb);
            }
            if resize_mini_windows == sym::GROW_ONLY {
                if i64::from(height) > old_height {
                    self.grow_mini_window(w, height - old_height as i32, unit)?;
                } else if i64::from(height) < old_height && (exact_p || begv == zv) {
                    self.shrink_mini_window(w, unit)?;
                }
            } else if i64::from(height) != old_height {
                self.grow_mini_window(w, height - old_height as i32, unit)?;
            }
            if let Some(old) = old_current_buffer {
                self.set_buffer_internal(old);
            }
        }
        Ok(self.window_box_text_height(w) != old_height)
    }

    pub fn current_message(&mut self) -> LispResult {
        if !self.is_buffer(self.xdisp.echo_area_buffer[0]) {
            return Ok(Value::NIL);
        }
        let (_, msg) = self.with_echo_area_buffer(None, 0, EchoFn::CurrentMessage)?;
        if msg.is_nil() {
            self.xdisp.echo_area_buffer[0] = Value::NIL;
        }
        Ok(msg)
    }

    fn current_message_1(&mut self) -> Value {
        let z = self.cur().z();
        if z > 1 {
            self.buffer_substring_value(1, z, true)
        } else {
            Value::NIL
        }
    }

    pub fn truncate_echo_area(&mut self, nchars: usize) -> LispResult<()> {
        if nchars == 0 {
            self.xdisp.echo_area_buffer[0] = Value::NIL;
        } else if !self.noninteractive
            && self.find_symbol_value(sym::EXECUTING_KBD_MACRO).is_nil()
            && !self.xdisp.echo_area_buffer[0].is_nil()
        {
            let sf = self.frames.selected_frame;
            if self.frame(sf).flags.glyphs_initialized_p {
                self.with_echo_area_buffer(None, 0, EchoFn::TruncateMessage(nchars))?;
            }
        }
        Ok(())
    }

    fn truncate_message_1(&mut self, nchars: usize) -> LispResult<()> {
        let z = self.cur().z();
        if 1 + nchars < z {
            self.del_range(1 + nchars, z)?;
        }
        if self.cur().z() == 1 {
            self.xdisp.echo_area_buffer[0] = Value::NIL;
        }
        Ok(())
    }

    pub(crate) fn set_message(&mut self, string: Value) -> LispResult<()> {
        let mut string = string;
        let mut message = Value::NIL;
        let function = self.find_symbol_value(sym::SET_MESSAGE_FUNCTION);
        if self.functionp(function) && self.gc_inhibited == 0 {
            let count = self.specpdl_index();
            self.specbind(sym::INHIBIT_QUIT, sym::T)?;
            message = self.safe_call(true, &[function, string])?;
            self.unbind_to(count)?;
            if message.is_string() {
                string = message;
                message = Value::NIL;
            }
        }
        let base = self.stack.len();
        self.stack.push(string);
        if message.is_nil() {
            self.xdisp.message_enable_multibyte = self.string_multibyte(string);
            self.with_echo_area_buffer(None, -1, EchoFn::SetMessage(string))?;
            self.xdisp.message_buf_print = false;
            self.xdisp.help_echo_showing_p = false;
        }
        let debug_on_message = self.find_symbol_value(sym::DEBUG_ON_MESSAGE);
        if debug_on_message.is_string()
            && string.is_string()
            && self.fast_string_match(debug_on_message, string, Value::NIL)? >= 0
        {
            let args = self.list(&[sym::ERROR, string]);
            self.call_debugger(args)?;
        }
        self.stack.truncate(base);
        Ok(())
    }

    fn set_message_1(&mut self, string: Value) -> LispResult<()> {
        let multibyte = !self
            .find_symbol_value(sym::ENABLE_MULTIBYTE_CHARACTERS)
            .is_nil();
        if !self.xdisp.message_enable_multibyte
            && self.var_true(sym::UNIBYTE_DISPLAY_VIA_LANGUAGE_ENVIRONMENT)
            && multibyte
        {
            crate::buffer::f_set_buffer_multibyte(self, &[Value::NIL])?;
        } else if !multibyte {
            crate::buffer::f_set_buffer_multibyte(self, &[sym::T])?;
        }
        let cb = self.current_buffer;
        let truncate = if self.var_true(sym::MESSAGE_TRUNCATE_LINES) {
            sym::T
        } else {
            Value::NIL
        };
        self.buffer_local_set(cb, sym::TRUNCATE_LINES, truncate);
        if !self
            .find_symbol_value(sym::BIDI_DISPLAY_REORDERING)
            .is_nil()
        {
            self.buffer_local_set(cb, sym::BIDI_PARAGRAPH_DIRECTION, sym::LEFT_TO_RIGHT);
        }
        self.set_pt_both(1, 1);
        self.insert_string(string, true, false)
    }

    pub fn clear_garbaged_frames(&mut self) -> LispResult<bool> {
        let current_matrices_cleared = false;
        if self.frames.frame_garbaged {
            let frames = self.frames.list.clone();
            for f in frames {
                if self.frame_redisplay_p(f) && self.frame(f).flags.garbaged {
                    if self.frame(f).flags.resized_p {
                        self.redraw_frame(f);
                    } else {
                        self.clear_current_matrices(f);
                    }
                    if self.frame_window_p(f)
                        && let Some(hook) = self.frame_rif(f).clear_under_internal_border
                    {
                        hook(self, f);
                    }
                    self.fset_redisplay(f);
                    let fr = self.frame_mut(f);
                    fr.flags.garbaged = false;
                    fr.flags.resized_p = false;
                }
            }
            self.frames.frame_garbaged = false;
        }
        Ok(current_matrices_cleared)
    }

    pub fn echo_area_display(&mut self, update_frame_p: bool) -> LispResult<()> {
        let sf = self.frames.selected_frame;
        let mini_window = self.frame(sf).minibuffer_window;
        if mini_window.is_nil() {
            return Ok(());
        }
        let w = mini_window;
        let f = self.window(w).frame;
        if !self.frame_redisplay_p(f)
            || !self.frame(f).flags.glyphs_initialized_p
            || !self.find_symbol_value(sym::INHIBIT_REDISPLAY).is_nil()
        {
            return Ok(());
        }
        if self.frame(sf).output_method == OutputMethod::Initial {
            return Ok(());
        }
        self.clear_garbaged_frames()?;
        let mut window_height_changed_p = false;
        if !self.xdisp.echo_area_buffer[0].is_nil() || self.minibuf.minibuf_level == 0 {
            self.xdisp.echo_area_window = mini_window;
            window_height_changed_p = self.display_echo_area(w)?;
            self.window_mut(w).flags.must_be_updated_p = true;
            if update_frame_p && !self.xdisp.redisplaying_p {
                if window_height_changed_p && !self.find_symbol_value(sym::RUN_HOOKS).is_nil() {
                    self.fset_redisplay(f);
                    self.redisplay_internal()?;
                } else if self.frame_window_p(f) {
                    self.update_single_window(w)?;
                    self.flush_frame(f);
                } else {
                    unreachable!("tty frames have no glyph matrices");
                }
                if self.var_true(sym::CURSOR_IN_ECHO_AREA) {
                    self.wset_redisplay(mini_window);
                }
            }
        } else if mini_window != self.windows.selected_window {
            self.wset_redisplay(mini_window);
        }
        self.xdisp.echo_area_buffer[1] = self.xdisp.echo_area_buffer[0];
        self.keyboard.echo_message_buffer = Value::NIL;
        if mini_window == self.windows.selected_window {
            self.xdisp.this_line_start_pos.charpos = 0;
        }
        if window_height_changed_p {
            self.fset_redisplay(f);
            self.clear_garbaged_frames()?;
        }
        Ok(())
    }
}

pub fn unwind_with_echo_area_buffer(lisp: &mut Lisp, vector: Value) -> LispResult<()> {
    let items = lisp.vector_items(vector).to_vec();
    lisp.set_buffer_internal(items[0]);
    lisp.set_plain_value(sym::DEACTIVATE_MARK, items[1]);
    lisp.windows.windows_or_buffers_changed = items[2].as_fixnum().unwrap_or(0);
    if lisp.is_window(items[3]) {
        let w = items[3];
        let buffer = items[4];
        lisp.window_mut(w).contents = buffer;
        let (pointm, old_pointm, start) = {
            let win = lisp.window(w);
            (win.pointm, win.old_pointm, win.start)
        };
        for (marker, i) in [(pointm, 5), (old_pointm, 7), (start, 9)] {
            let charpos = items[i].as_fixnum().unwrap_or(1) as usize;
            let bytepos = items[i + 1].as_fixnum().unwrap_or(1) as usize;
            let _ = bytepos;
            let (begv, zv) = {
                let b = lisp.buffer(buffer);
                (b.begv, b.zv)
            };
            lisp.set_marker_position(marker, charpos.clamp(begv, zv), buffer);
        }
    }
    lisp.xdisp.with_echo_area_save_vector = vector;
    Ok(())
}
