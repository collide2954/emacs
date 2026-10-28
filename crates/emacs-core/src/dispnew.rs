use crate::buffer::f_buffer_modified_p;
use crate::error::LispResult;
use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[Subr::fixed(
    "frame-or-buffer-changed-p",
    0,
    1,
    f_frame_or_buffer_changed_p,
)];

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
        lisp.check_symbol(variable)?;
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
