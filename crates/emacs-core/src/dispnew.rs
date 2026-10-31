use std::io::Write;

use num_traits::{Signed, ToPrimitive};

use crate::buffer::f_buffer_modified_p;
use crate::error::LispResult;
use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::timefns::{current_timespec, dtotimespec, timespec_add, timespec_sub};
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed(
        "frame-or-buffer-changed-p",
        0,
        1,
        f_frame_or_buffer_changed_p,
    ),
    Subr::fixed("ding", 0, 1, f_ding),
    Subr::fixed("internal-show-cursor", 2, 2, f_internal_show_cursor),
    Subr::fixed("internal-show-cursor-p", 0, 1, f_internal_show_cursor_p),
    Subr::fixed("sleep-for", 1, 2, f_sleep_for),
    Subr::fixed("redraw-frame", 0, 1, f_redraw_frame),
];

fn f_redraw_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(args[0])?;
    lisp.redraw_frame(f);
    Ok(Value::NIL)
}

impl Lisp {
    pub fn redraw_frame(&mut self, f: Value) {
        if self.frame_window_p(f) {
            self.set_frame_garbaged(f);
        }
        self.fset_redisplay(f);
        let root = self.frame(f).root_window;
        self.mark_window_display_accurate(root, false);
        self.set_window_update_flags(root, true);
        self.frame_mut(f).flags.garbaged = false;
    }

    fn set_window_update_flags(&mut self, mut w: Value, on_p: bool) {
        loop {
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                self.set_window_update_flags(contents, on_p);
            } else {
                self.window_mut(w).flags.must_be_updated_p = on_p;
            }
            let next = self.window(w).next;
            if next.is_nil() {
                break;
            }
            w = next;
        }
    }
}

const WAIT_READING_MAX: i64 = i64::MAX;

pub(crate) fn f_sleep_for(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut duration = crate::floatfns::float_arg(lisp, args[0])?;
    if !args[1].is_nil() {
        let milliseconds = lisp.check_fixnum(args[1])?;
        duration += milliseconds as f64 / 1000.0;
    }
    if duration > 0.0 {
        let mut t = dtotimespec(duration);
        let tend = timespec_add(current_timespec(), t);
        loop {
            lisp.wait_reading_process_output(t.0, t.1, 0, Value::NIL)?;
            t = timespec_sub(tend, current_timespec());
            if t <= (0, 0) {
                break;
            }
        }
    }
    Ok(Value::NIL)
}

impl Lisp {
    pub fn sit_for(&mut self, timeout: Value, reading: bool) -> LispResult {
        let sw = self.windows.selected_window;
        let curbuf_eq_winbuf = self.current_buffer == self.window(sw).contents;
        self.swallow_events()?;
        if self.detect_input_pending_run_timers()?
            || !self.find_symbol_value(sym::EXECUTING_KBD_MACRO).is_nil()
        {
            return Ok(Value::NIL);
        }
        let (sec, nsec) = if let Some(n) = timeout.as_fixnum() {
            if n <= 0 {
                return Ok(sym::T);
            }
            (n, 0)
        } else if let Some(b) = self.bignum_ref(timeout) {
            match b.to_i64() {
                Some(n) if n <= 0 => return Ok(sym::T),
                Some(n) => (n, 0),
                None if b.is_negative() => return Ok(sym::T),
                None => (WAIT_READING_MAX, 0),
            }
        } else if timeout.is_float() {
            let seconds = self.heap.float_value(timeout);
            if seconds.is_nan() || seconds <= 0.0 {
                return Ok(sym::T);
            }
            dtotimespec(seconds)
        } else if timeout == sym::T {
            (0, 0)
        } else {
            return Err(self.wrong_type(sym::NUMBERP, timeout));
        };
        let nbytes =
            self.wait_reading_process_output(sec, nsec, if reading { -1 } else { 1 }, Value::NIL)?;
        if reading && curbuf_eq_winbuf {
            let sw = self.windows.selected_window;
            let buffer = self.window(sw).contents;
            self.set_buffer_internal(buffer);
        }
        Ok(if nbytes > 0 || self.detect_input_pending()? {
            Value::NIL
        } else {
            sym::T
        })
    }
}

fn f_internal_show_cursor(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if !lisp.xdisp.redisplaying_p {
        let w = lisp.decode_any_window(args[0])?;
        lisp.window_mut(w).flags.cursor_off_p = args[1].is_nil();
    }
    Ok(Value::NIL)
}

fn f_internal_show_cursor_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_any_window(args[0])?;
    Ok(if lisp.window(w).flags.cursor_off_p {
        Value::NIL
    } else {
        sym::T
    })
}

pub(crate) fn f_ding(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if !args[0].is_nil() {
        if lisp.noninteractive() {
            putchar_bell();
        } else {
            let f = lisp.frames.selected_frame;
            lisp.ring_bell(f)?;
        }
    } else {
        lisp.bitch_at_user()?;
    }
    Ok(Value::NIL)
}

fn putchar_bell() {
    let _ = std::io::stdout().write_all(b"\x07");
}

impl Lisp {
    pub fn bitch_at_user(&mut self) -> LispResult<()> {
        if self.noninteractive() {
            putchar_bell();
            Ok(())
        } else if !self.find_symbol_value(sym::EXECUTING_KBD_MACRO).is_nil() {
            let msg = self.string("Keyboard macro terminated by a command ringing the bell");
            Err(self.signal_list(sym::USER_ERROR, &[msg]))
        } else {
            let f = self.frames.selected_frame;
            self.ring_bell(f)
        }
    }

    pub fn ring_bell(&mut self, f: Value) -> LispResult<()> {
        let function = self.find_symbol_value(sym::RING_BELL_FUNCTION);
        if !function.is_nil() {
            self.set_plain_value(sym::RING_BELL_FUNCTION, Value::NIL);
            self.stack.push(function);
            let r = self.call0(function);
            self.stack.pop();
            r?;
            self.set_plain_value(sym::RING_BELL_FUNCTION, function);
            Ok(())
        } else {
            match self.terminal(self.frame(f).terminal).kind {
                crate::terminal::OutputMethod::Initial => Ok(()),
            }
        }
    }
}

fn listed_buffers(lisp: &Lisp) -> Vec<Value> {
    lisp.buffers
        .iter()
        .copied()
        .filter(|&b| lisp.buffer_live_p(b))
        .collect()
}

fn state_matches(lisp: &mut Lisp, state: Value) -> LispResult<bool> {
    let items = lisp.vector_items(state).to_vec();
    let mut idx = 0;
    for frame in lisp.frames.list.clone() {
        if idx == items.len() || items[idx] != frame {
            return Ok(false);
        }
        idx += 1;
        if idx == items.len() || !lisp.eq(items[idx], lisp.frame(frame).name) {
            return Ok(false);
        }
        idx += 1;
    }
    for buf in listed_buffers(lisp) {
        let name = lisp.buffer(buf).name;
        if lisp.string_bytes(name).first() == Some(&b' ') {
            continue;
        }
        if idx == items.len() || items[idx] != buf {
            return Ok(false);
        }
        idx += 1;
        let read_only = lisp
            .buffer_local_get(buf, sym::BUFFER_READ_ONLY)
            .unwrap_or(Value::NIL);
        if idx == items.len() || !lisp.eq(items[idx], read_only) {
            return Ok(false);
        }
        idx += 1;
        let modified = f_buffer_modified_p(lisp, &[buf])?;
        if idx == items.len() || !lisp.eq(items[idx], modified) {
            return Ok(false);
        }
        idx += 1;
    }
    Ok(idx != items.len() && lisp.eq(items[idx], sym::LAMBDA))
}

fn f_frame_or_buffer_changed_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = args[0];
    let mut state = if !variable.is_nil() {
        let variable = lisp.check_symbol(variable)?;
        lisp.symbol_value(variable)?
    } else {
        lisp.frame_and_buffer_state
    };
    if lisp.is_vector(state) && state_matches(lisp, state)? {
        return Ok(Value::NIL);
    }
    let buffers = listed_buffers(lisp);
    let n = 1 + 2 * lisp.frames.list.len() + 3 * buffers.len();
    if !lisp.is_vector(state)
        || n > lisp.vector_items(state).len()
        || n + 20 < lisp.vector_items(state).len() / 2
    {
        state = lisp.vector(vec![sym::LAMBDA; n + 20]);
        if !variable.is_nil() {
            lisp.set(variable, state)?;
        } else {
            lisp.frame_and_buffer_state = state;
        }
    }
    let mut data = Vec::with_capacity(n);
    for frame in lisp.frames.list.clone() {
        data.push(frame);
        data.push(lisp.frame(frame).name);
    }
    for buf in buffers {
        let name = lisp.buffer(buf).name;
        if lisp.string_bytes(name).first() == Some(&b' ') {
            continue;
        }
        data.push(buf);
        data.push(
            lisp.buffer_local_get(buf, sym::BUFFER_READ_ONLY)
                .unwrap_or(Value::NIL),
        );
        data.push(f_buffer_modified_p(lisp, &[buf])?);
    }
    let size = lisp.vector_items(state).len();
    data.resize(size, sym::LAMBDA);
    for (i, v) in data.into_iter().enumerate() {
        lisp.vector_set(state, i, v);
    }
    Ok(sym::T)
}

pub fn init_vars(lisp: &mut Lisp) {
    lisp.frame_and_buffer_state = lisp.vector(vec![sym::LAMBDA; 20]);
    lisp.defvar_int(sym::BAUD_RATE, 1200);
    lisp.defvar_bool(sym::INVERSE_VIDEO, false);
    lisp.defvar_bool(sym::VISIBLE_BELL, false);
    lisp.defvar_bool(sym::NO_REDRAW_ON_REENTER, false);
    lisp.defvar(sym::INITIAL_WINDOW_SYSTEM, Value::NIL);
    lisp.defvar_kboard(sym::WINDOW_SYSTEM, Kvar::WindowSystem);
    lisp.defvar_bool(sym::CURSOR_IN_ECHO_AREA, false);
    lisp.defvar_bool(sym::MOUSE_PREFER_CLOSEST_GLYPH, false);
    lisp.defvar(sym::GLYPH_TABLE, Value::NIL);
    lisp.defvar(sym::STANDARD_DISPLAY_TABLE, Value::NIL);
    lisp.defvar(sym::X_SHOW_TOOLTIP_TIMEOUT, Value::fixnum(5));
    lisp.defvar(sym::TAB_BAR_POSITION, Value::NIL);
    let feature = lisp.intern("tty-child-frames");
    crate::fns::f_provide(lisp, &[feature, Value::NIL]).expect("provide tty-child-frames");
}
