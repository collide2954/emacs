use std::cell::RefCell;

use emacs_core::dispextern::{EmacsCursorKind, TextPos};
use emacs_core::dispnew::matrix::{WhichMatrix, clear_glyph_matrix};
use emacs_core::error::LispResult;
use emacs_core::frame::ResourceType;
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_core::terminal::OutputMethod;
use emacs_core::value::Value;
use emacs_core::xdisp::line::TRY_WINDOW_IGNORE_FONTS_CHANGE;

use super::{
    BLACK_PIX_DEFAULT, arg, f_pgtk_display_monitor_attributes_list, get_arg,
    pgtk_default_font_parameter, unwind_create_frame, x_decode_color,
};
use crate::pgtkterm::{
    allocate_output, frame_key, pgtk_cr_update_surface_desired_size, pgtk_make_frame_invisible,
    with_display_list, with_output,
};

#[derive(Clone, Copy)]
struct Tip {
    frame: Value,
    timer: Value,
    last_string: Value,
    last_frame: Value,
    last_parms: Value,
}

thread_local! {
    static TIP: RefCell<Tip> = const {
        RefCell::new(Tip {
            frame: Value::NIL,
            timer: Value::NIL,
            last_string: Value::NIL,
            last_frame: Value::NIL,
            last_parms: Value::NIL,
        })
    };
}

fn tip() -> Tip {
    TIP.with(|t| *t.borrow())
}

fn update(f: impl FnOnce(&mut Tip)) {
    TIP.with(|t| f(&mut t.borrow_mut()));
}

pub(super) fn mark_tip(roots: &mut Vec<Value>) {
    let t = tip();
    roots.extend_from_slice(&[t.frame, t.timer, t.last_string, t.last_frame, t.last_parms]);
}

fn unwind_create_tip_frame(lisp: &mut Lisp, frame: Value) -> LispResult<()> {
    if unwind_create_frame(lisp, frame)? {
        update(|t| t.frame = Value::NIL);
    }
    Ok(())
}

fn frame_parameter(lisp: &mut Lisp, f: Value, parameter: Value) -> LispResult {
    emacs_core::frame::f_frame_parameter(lisp, &[f, parameter])
}

fn modify_frame_parameter(lisp: &mut Lisp, f: Value, parameter: Value, value: Value) -> LispResult {
    let cell = lisp.cons(parameter, value);
    let alist = lisp.cons(cell, Value::NIL);
    emacs_core::frame::f_modify_frame_parameters(lisp, &[f, alist])
}

fn pgtk_create_tip_frame(lisp: &mut Lisp, dpyinfo: Value, parms: Value) -> LispResult<Value> {
    let count = lisp.specpdl_index();
    let base = lisp.stack.len();
    let face_change_before = lisp.faces.face_change;
    let r = pgtk_create_tip_frame_1(lisp, dpyinfo, parms, face_change_before);
    let u = lisp.unbind_to(count);
    lisp.stack.truncate(base);
    let frame = r?;
    u?;
    Ok(frame)
}

fn pgtk_create_tip_frame_1(
    lisp: &mut Lisp,
    dpyinfo: Value,
    parms: Value,
    face_change_before: bool,
) -> LispResult<Value> {
    use ResourceType::{Boolean, Number, String as Str, Symbol};
    if lisp.terminal(dpyinfo).name.is_none() {
        return Err(lisp.error("Terminal is not live, can't create new frames on it"));
    }
    let mut parms = emacs_core::fns::f_copy_alist(lisp, &[parms])?;
    lisp.stack.push(parms);
    let name = get_arg(
        lisp,
        dpyinfo,
        parms,
        sym::NAME,
        Some("name"),
        Some("Name"),
        Str,
    )?;
    if !name.is_string() && name != sym::UNBOUND && !name.is_nil() {
        return Err(lisp.error("Invalid frame name--not a string or nil"));
    }
    lisp.stack.push(name);
    let f = lisp.make_frame(false)?;
    lisp.stack.push(f);
    lisp.frame_mut(f).flags.wants_modeline = false;
    lisp.record_unwind(unwind_create_tip_frame, f);
    lisp.frame_mut(f).terminal = dpyinfo;
    lisp.frame_mut(f).output_method = OutputMethod::Pgtk;
    allocate_output(lisp, f);
    lisp.frame_mut(f).fontset = -1;
    with_output(lisp, f, |output| {
        output.white_relief_pixel = -1;
        output.black_relief_pixel = -1;
    });
    lisp.frame_mut(f).flags.tooltip = true;
    lisp.frame_mut(f).icon_name = Value::NIL;
    let root_window = lisp.display_info(dpyinfo).root_window;
    with_output(lisp, f, |output| {
        output.parent_desc = root_window;
        output.explicit_parent = false;
    });
    lisp.frame_mut(f).foreground_pixel = -1;
    lisp.frame_mut(f).background_pixel = -1;
    with_output(lisp, f, |output| output.border_pixel = -1);
    let black = lisp.string("black");
    lisp.stack.push(black);
    let pixel = x_decode_color(lisp, f, black, BLACK_PIX_DEFAULT)?;
    lisp.frame_mut(f).foreground_pixel = pixel;
    let pixel = x_decode_color(lisp, f, black, BLACK_PIX_DEFAULT)?;
    lisp.frame_mut(f).background_pixel = pixel;
    let pixel = x_decode_color(lisp, f, black, BLACK_PIX_DEFAULT)?;
    with_output(lisp, f, |output| output.border_pixel = pixel);
    if name == sym::UNBOUND || name.is_nil() {
        let id_name = with_display_list(|displays| {
            displays
                .iter()
                .find(|d| d.terminal == dpyinfo)
                .map(|d| d.x_id_name.clone())
                .unwrap_or_default()
        });
        let id_name = lisp.make_string(&id_name);
        lisp.frame_mut(f).name = id_name;
        lisp.frame_mut(f).flags.explicit_name = false;
    } else {
        lisp.frame_mut(f).name = name;
        lisp.frame_mut(f).flags.explicit_name = true;
        lisp.specbind(sym::X_RESOURCE_NAME, name)?;
    }
    let driver = crate::ftcrfont::ftcrfont_driver(lisp);
    lisp.register_font_driver(driver, Some(f))?;
    let driver = crate::ftcrfont::ftcrhbfont_driver(lisp);
    lisp.register_font_driver(driver, Some(f))?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::FONT_BACKEND,
        Value::NIL,
        Some("fontBackend"),
        Some("FontBackend"),
        Str,
    )?;
    pgtk_default_font_parameter(lisp, f, parms)?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::BORDER_WIDTH,
        Value::fixnum(0),
        Some("borderWidth"),
        Some("BorderWidth"),
        Number,
    )?;
    if lisp.assq(sym::INTERNAL_BORDER_WIDTH, parms).is_nil() {
        let value = get_arg(
            lisp,
            dpyinfo,
            parms,
            sym::INTERNAL_BORDER_WIDTH,
            Some("internalBorder"),
            Some("internalBorder"),
            Number,
        )?;
        if value != sym::UNBOUND {
            let cell = lisp.cons(sym::INTERNAL_BORDER_WIDTH, value);
            parms = lisp.cons(cell, parms);
            lisp.stack.push(parms);
        }
    }
    lisp.gui_default_parameter(
        f,
        parms,
        sym::INTERNAL_BORDER_WIDTH,
        Value::fixnum(1),
        Some("internalBorderWidth"),
        Some("internalBorderWidth"),
        Number,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::RIGHT_DIVIDER_WIDTH,
        Value::fixnum(0),
        None,
        None,
        Number,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::BOTTOM_DIVIDER_WIDTH,
        Value::fixnum(0),
        None,
        None,
        Number,
    )?;
    for (prop, color, xprop, xclass) in [
        (sym::FOREGROUND_COLOR, "black", "foreground", "Foreground"),
        (sym::BACKGROUND_COLOR, "white", "background", "Background"),
        (sym::MOUSE_COLOR, "black", "pointerColor", "Foreground"),
        (sym::CURSOR_COLOR, "black", "cursorColor", "Foreground"),
        (sym::BORDER_COLOR, "black", "borderColor", "BorderColor"),
    ] {
        let color = lisp.string(color);
        lisp.stack.push(color);
        lisp.gui_default_parameter(f, parms, prop, color, Some(xprop), Some(xclass), Str)?;
    }
    lisp.gui_default_parameter(
        f,
        parms,
        sym::NO_SPECIAL_GLYPHS,
        Value::NIL,
        None,
        None,
        Boolean,
    )?;
    lisp.init_frame_faces(f)?;
    with_output(lisp, f, |output| output.parent_desc = root_window);
    lisp.gui_default_parameter(
        f,
        parms,
        sym::INHIBIT_DOUBLE_BUFFERING,
        Value::NIL,
        Some("inhibitDoubleBuffering"),
        Some("InhibitDoubleBuffering"),
        Boolean,
    )?;
    lisp.gui_figure_window_size(f, parms, false, false)?;
    let key = frame_key(lisp, f) as u64;
    let (pixel_width, pixel_height) = (
        lisp.frame(f).geometry.pixel_width.max(1) as u32,
        lisp.frame(f).geometry.pixel_height.max(1) as u32,
    );
    let background = lisp.frame(f).background_pixel as u32;
    crate::with_display(|display| display.create_popup(key, pixel_width, pixel_height, background));
    with_output(lisp, f, |output| {
        output.widget = true;
        output.current_cursor = Some(EmacsCursorKind::Text);
    });
    lisp.gui_default_parameter(
        f,
        parms,
        sym::AUTO_RAISE,
        Value::NIL,
        Some("autoRaise"),
        Some("AutoRaiseLower"),
        Boolean,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::AUTO_LOWER,
        Value::NIL,
        Some("autoLower"),
        Some("AutoRaiseLower"),
        Boolean,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::CURSOR_TYPE,
        sym::BOX,
        Some("cursorType"),
        Some("CursorType"),
        Symbol,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::ALPHA,
        Value::NIL,
        Some("alpha"),
        Some("Alpha"),
        Number,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::ALPHA_BACKGROUND,
        Value::NIL,
        Some("alphaBackground"),
        Some("AlphaBackground"),
        Number,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::BORDERS_RESPECT_ALPHA_BACKGROUND,
        Value::NIL,
        Some("bordersRespectAlphaBackground"),
        Some("BordersRespectAlphaBackground"),
        Number,
    )?;
    if frame_parameter(lisp, f, sym::TOOLTIP)?.is_nil() {
        modify_frame_parameter(lisp, f, sym::TOOLTIP, sym::T)?;
    }
    if frame_parameter(lisp, f, sym::DISPLAY_TYPE)?.is_nil() {
        let disptype = lisp.intern("color");
        modify_frame_parameter(lisp, f, sym::DISPLAY_TYPE, disptype)?;
    }
    let bg = frame_parameter(lisp, f, sym::BACKGROUND_COLOR)?;
    lisp.stack.push(bg);
    lisp.funcall(sym::FACE_SET_AFTER_FRAME_DEFAULT, &[f, Value::NIL])?;
    if bg != frame_parameter(lisp, f, sym::BACKGROUND_COLOR)? {
        modify_frame_parameter(lisp, f, sym::BACKGROUND_COLOR, bg)?;
    }
    lisp.frame_mut(f).flags.no_split = true;
    lisp.display_info_mut(dpyinfo).reference_count += 1;
    lisp.terminal_mut(dpyinfo).reference_count += 1;
    lisp.frames.list.insert(0, f);
    lisp.frame_mut(f).flags.can_set_window_size = true;
    let (text_width, text_height) = (
        lisp.frame(f).geometry.text_width,
        lisp.frame(f).geometry.text_height,
    );
    lisp.adjust_frame_size(f, text_width, text_height, 0, sym::TIP_FRAME)?;
    lisp.faces.face_change = face_change_before;
    Ok(f)
}

fn nth_fixnum(lisp: &mut Lisp, n: i64, list: Value) -> LispResult<i64> {
    let v = emacs_core::fns::f_nth(lisp, &[Value::fixnum(n), list])?;
    lisp.check_fixnum(v)
}

fn pgtk_display_size() -> (i64, i64) {
    crate::with_display(|display| {
        display.monitors().iter().fold((0, 0), |(w, h), m| {
            (
                w.max(i64::from(m.geometry.x + m.geometry.width)),
                h.max(i64::from(m.geometry.y + m.geometry.height)),
            )
        })
    })
    .unwrap_or((0, 0))
}

#[allow(clippy::too_many_arguments)]
fn compute_tip_xy(
    lisp: &mut Lisp,
    f: Value,
    parms: Value,
    dx: i64,
    dy: i64,
    width: i64,
    height: i64,
) -> LispResult<(i64, i64)> {
    let base = lisp.stack.len();
    let r = (|| -> LispResult<(i64, i64)> {
        let edge = |lisp: &mut Lisp, key: Value| -> LispResult {
            let cell = emacs_core::fns::f_assq(lisp, &[key, parms])?;
            lisp.cdr(cell)
        };
        let left = edge(lisp, sym::LEFT)?;
        let top = edge(lisp, sym::TOP)?;
        let right = edge(lisp, sym::RIGHT)?;
        let bottom = edge(lisp, sym::BOTTOM)?;
        let (mut root_x, mut root_y) = (0, 0);
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (0, 0, 0, -1);
        if (!left.is_fixnum() && !right.is_fixnum()) || (!top.is_fixnum() && !bottom.is_fixnum()) {
            if let Some((x, y)) =
                crate::with_display(|display| display.pointer_position()).flatten()
            {
                root_x = i64::from(x);
                root_y = i64::from(y);
            }
            let mut attributes = f_pgtk_display_monitor_attributes_list(lisp, &[f])?;
            lisp.stack.push(attributes);
            while attributes.is_cons() {
                let monitor = lisp.xcar(attributes);
                let geometry = emacs_core::fns::f_assq(lisp, &[sym::GEOMETRY, monitor])?;
                if geometry.is_cons() {
                    min_x = nth_fixnum(lisp, 1, geometry)?;
                    min_y = nth_fixnum(lisp, 2, geometry)?;
                    max_x = min_x + nth_fixnum(lisp, 3, geometry)?;
                    max_y = min_y + nth_fixnum(lisp, 4, geometry)?;
                    if min_x <= root_x && root_x < max_x && min_y <= root_y && root_y < max_y {
                        break;
                    }
                    max_y = -1;
                }
                attributes = lisp.xcdr(attributes);
            }
        }
        if max_y < 0 {
            min_x = 0;
            min_y = 0;
            (max_x, max_y) = pgtk_display_size();
        }
        if let Some(top) = top.as_fixnum() {
            root_y = top;
        } else if let Some(bottom) = bottom.as_fixnum() {
            root_y = bottom - height;
        } else if root_y + dy <= min_y {
            root_y = min_y;
        } else if root_y + dy + height <= max_y {
            root_y += dy;
        } else if height + dy + min_y <= root_y {
            root_y -= height + dy;
        } else {
            root_y = min_y;
        }
        if let Some(left) = left.as_fixnum() {
            root_x = left;
        } else if let Some(right) = right.as_fixnum() {
            root_x = right - width;
        } else if root_x + dx <= min_x {
            root_x = 0;
        } else if root_x + dx + width <= max_x {
            root_x += dx;
        } else if width + dx + min_x <= root_x {
            root_x -= width + dx;
        } else {
            root_x = min_x;
        }
        Ok((root_x, root_y))
    })();
    lisp.stack.truncate(base);
    r
}

fn xg_show_tooltip(lisp: &Lisp, f: Value, string: Value) {
    let key = frame_key(lisp, f) as u64;
    let text = String::from_utf8_lossy(lisp.string_bytes(string)).into_owned();
    crate::with_display(|display| display.set_tooltip(key, Some(&text)));
}

fn xg_hide_tooltip(lisp: &Lisp, f: Value) -> bool {
    let key = frame_key(lisp, f) as u64;
    crate::with_display(|display| display.set_tooltip(key, None));
    true
}

fn cancel_tip_timer(lisp: &mut Lisp) -> LispResult<()> {
    let timer = tip().timer;
    if !timer.is_nil() {
        let cancel_timer = lisp.intern("cancel-timer");
        lisp.funcall(cancel_timer, &[timer])?;
        update(|t| t.timer = Value::NIL);
    }
    Ok(())
}

fn use_system_tooltips(lisp: &Lisp) -> bool {
    !lisp.plain_value(sym::USE_SYSTEM_TOOLTIPS).is_nil()
}

fn pgtk_hide_tip(lisp: &mut Lisp, delete: bool) -> LispResult {
    cancel_tip_timer(lisp)?;
    let t = tip();
    if (t.last_frame.is_nil() && t.frame.is_nil())
        || (!use_system_tooltips(lisp)
            && !delete
            && lisp.is_frame(t.frame)
            && lisp.frame_live_p(t.frame)
            && !lisp.frame(t.frame).flags.visible)
    {
        return Ok(Value::NIL);
    }
    let count = lisp.specpdl_index();
    lisp.specbind(sym::INHIBIT_REDISPLAY, sym::T)?;
    lisp.specbind(sym::INHIBIT_QUIT, sym::T)?;
    let r = (|| -> LispResult {
        let mut was_open = Value::NIL;
        let last_frame = tip().last_frame;
        if lisp.is_frame(last_frame)
            && lisp.frame_live_p(last_frame)
            && xg_hide_tooltip(lisp, last_frame)
        {
            was_open = sym::T;
        }
        if use_system_tooltips(lisp) {
            update(|t| t.last_frame = Value::NIL);
        }
        let frame = tip().frame;
        if lisp.is_frame(frame) && lisp.frame_live_p(frame) {
            if delete || use_system_tooltips(lisp) {
                lisp.delete_frame(frame, Value::NIL)?;
                update(|t| t.frame = Value::NIL);
            } else {
                pgtk_make_frame_invisible(lisp, frame);
            }
            was_open = sym::T;
        } else {
            update(|t| t.frame = Value::NIL);
        }
        Ok(was_open)
    })();
    let u = lisp.unbind_to(count);
    let was_open = r?;
    u?;
    Ok(was_open)
}

pub(super) fn f_x_hide_tip(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let reuse = !lisp.plain_value(sym::TOOLTIP_REUSE_HIDDEN_FRAME).is_nil();
    pgtk_hide_tip(lisp, !reuse)
}

pub(super) fn f_x_show_tip(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let count = lisp.specpdl_index();
    let base = lisp.stack.len();
    let r = lisp
        .specbind(sym::INHIBIT_REDISPLAY, sym::T)
        .and_then(|()| x_show_tip(lisp, args));
    let u = lisp.unbind_to(count);
    lisp.stack.truncate(base);
    let value = r?;
    u?;
    Ok(value)
}

fn is_edge(parm: Value) -> bool {
    parm == sym::LEFT || parm == sym::TOP || parm == sym::RIGHT || parm == sym::BOTTOM
}

fn assq_delete_all(lisp: &mut Lisp, parm: Value) -> LispResult<()> {
    let function = lisp.intern("assq-delete-all");
    let last_parms = tip().last_parms;
    let last_parms = lisp.funcall(function, &[parm, last_parms])?;
    update(|t| t.last_parms = last_parms);
    Ok(())
}

fn reused_tip_must_be_deleted(lisp: &mut Lisp, parms: Value) -> LispResult<bool> {
    let mut tail = parms;
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        let parm = lisp.car(elt)?;
        if !is_edge(parm) {
            let last = emacs_core::fns::f_assq(lisp, &[parm, tip().last_parms])?;
            let (now, before) = (lisp.cdr(elt)?, lisp.cdr(last)?);
            if !lisp.equal(now, before)? {
                return Ok(true);
            }
        }
        assq_delete_all(lisp, parm)?;
        tail = lisp.xcdr(tail);
    }
    let mut tail = tip().last_parms;
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        let parm = lisp.car(elt)?;
        if !is_edge(parm) && !lisp.cdr(elt)?.is_nil() {
            return Ok(true);
        }
        tail = lisp.xcdr(tail);
    }
    Ok(false)
}

fn start_timer(lisp: &mut Lisp, timeout: Value) -> LispResult {
    let run_at_time = lisp.intern("run-at-time");
    let x_hide_tip = lisp.intern("x-hide-tip");
    let timer = lisp.funcall(run_at_time, &[timeout, Value::NIL, x_hide_tip])?;
    update(|t| t.timer = timer);
    Ok(Value::NIL)
}

fn with_default(lisp: &mut Lisp, parms: Value, key: Value, value: Value) -> Value {
    if lisp.assq(key, parms).is_nil() {
        let cell = lisp.cons(key, value);
        let parms = lisp.cons(cell, parms);
        lisp.stack.push(parms);
        parms
    } else {
        parms
    }
}

fn x_show_tip(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (mut string, mut frame, parms) = (arg(args, 0), arg(args, 1), arg(args, 2));
    let (mut timeout, dx, dy) = (arg(args, 3), arg(args, 4), arg(args, 5));
    let old_windows_or_buffers_changed = lisp.windows.windows_or_buffers_changed;
    lisp.check_string(string)?;
    if lisp.string_chars(string) == 0 {
        string = lisp.unibyte_string(b" ");
        lisp.stack.push(string);
    }
    if frame.is_nil() {
        frame = lisp.frames.selected_frame;
    }
    let f = lisp.decode_window_system_frame(frame)?;
    if !with_output(lisp, f, |output| output.widget) {
        return Ok(Value::NIL);
    }
    if timeout.is_nil() {
        timeout = lisp.plain_value(sym::X_SHOW_TOOLTIP_TIMEOUT);
    }
    lisp.check_fixnat(timeout)?;
    let dx = if dx.is_nil() {
        5
    } else {
        lisp.check_fixnum(dx)?
    };
    let dy = if dy.is_nil() {
        -10
    } else {
        lisp.check_fixnum(dy)?
    };
    if use_system_tooltips(lisp) {
        f_x_hide_tip(lisp, &[])?;
        xg_show_tooltip(lisp, f, string);
        update(|t| t.last_frame = frame);
        return start_timer(lisp, timeout);
    }
    let tip_frame = tip().frame;
    if lisp.is_frame(tip_frame) && lisp.frame_live_p(tip_frame) {
        let t = tip();
        if lisp.frame(tip_frame).flags.visible
            && frame == t.last_frame
            && !emacs_core::fns::f_equal_including_properties(lisp, &[t.last_string, string])?
                .is_nil()
            && lisp.equal(t.last_parms, parms)?
        {
            cancel_tip_timer(lisp)?;
            let (width, height) = (
                lisp.frame(tip_frame).geometry.pixel_width,
                lisp.frame(tip_frame).geometry.pixel_height,
            );
            let (root_x, root_y) = compute_tip_xy(lisp, tip_frame, parms, dx, dy, width, height)?;
            let key = frame_key(lisp, tip_frame) as u64;
            crate::with_display(|display| display.move_frame(key, root_x as i32, root_y as i32));
            return start_timer(lisp, timeout);
        } else if !lisp.plain_value(sym::TOOLTIP_REUSE_HIDDEN_FRAME).is_nil()
            && frame == t.last_frame
        {
            let delete = reused_tip_must_be_deleted(lisp, parms)?;
            pgtk_hide_tip(lisp, delete)?;
        } else {
            pgtk_hide_tip(lisp, true)?;
        }
    } else {
        pgtk_hide_tip(lisp, true)?;
    }
    update(|t| {
        t.last_frame = frame;
        t.last_string = string;
        t.last_parms = parms;
    });
    let mut parms = parms;
    let tip_frame = tip().frame;
    if !lisp.is_frame(tip_frame) || !lisp.frame_live_p(tip_frame) {
        let tooltip = lisp.string("tooltip");
        parms = with_default(lisp, parms, sym::NAME, tooltip);
        parms = with_default(lisp, parms, sym::INTERNAL_BORDER_WIDTH, Value::fixnum(3));
        parms = with_default(lisp, parms, sym::BORDER_WIDTH, Value::fixnum(1));
        let lightyellow = lisp.string("lightyellow");
        parms = with_default(lisp, parms, sym::BORDER_COLOR, lightyellow);
        let lightyellow = lisp.string("lightyellow");
        parms = with_default(lisp, parms, sym::BACKGROUND_COLOR, lightyellow);
        let dpyinfo = lisp.frame(f).terminal;
        let created = pgtk_create_tip_frame(lisp, dpyinfo, parms)?;
        update(|t| t.frame = created);
    }
    let tip_f = tip().frame;
    let window = lisp.frame(tip_f).root_window;
    let name = lisp.string(" *tip*");
    let tip_buf = lisp.get_buffer_create(name, false)?;
    lisp.buffer_local_set(tip_buf, sym::LEFT_MARGIN_WIDTH, Value::fixnum(0));
    lisp.buffer_local_set(tip_buf, sym::RIGHT_MARGIN_WIDTH, Value::fixnum(0));
    lisp.set_window_buffer(window, tip_buf, false, false)?;
    {
        let w = lisp.window_mut(window);
        w.flags.pseudo_window_p = true;
        w.left_col = 0;
        w.top_line = 0;
        w.pixel_left = 0;
        w.pixel_top = 0;
    }
    let max_size_symbol = lisp.intern("x-max-tooltip-size");
    let max_size = lisp.plain_value(max_size_symbol);
    let ranged = |v: Value| {
        v.as_fixnum()
            .filter(|n| (1..=i64::from(i32::MAX)).contains(n))
    };
    let (total_cols, total_lines) = match max_size
        .is_cons()
        .then(|| (ranged(lisp.xcar(max_size)), ranged(lisp.xcdr(max_size))))
    {
        Some((Some(cols), Some(lines))) => (cols, lines),
        _ => (80, 40),
    };
    let (column_width, line_height) = (
        lisp.frame(tip_f).geometry.column_width,
        lisp.frame(tip_f).geometry.line_height,
    );
    {
        let w = lisp.window_mut(window);
        w.total_cols = total_cols;
        w.total_lines = total_lines;
        w.pixel_width = total_cols * column_width;
        w.pixel_height = total_lines * line_height;
    }
    lisp.frame_mut(tip_f).geometry.total_cols = total_cols;
    lisp.adjust_frame_glyphs(tip_f);
    let count_1 = lisp.specpdl_index();
    let old_buffer = lisp.current_buffer;
    let contents = lisp.window(window).contents;
    lisp.set_buffer_internal(contents);
    lisp.buffer_local_set(contents, sym::TRUNCATE_LINES, Value::NIL);
    lisp.specbind(sym::INHIBIT_READ_ONLY, sym::T)?;
    lisp.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
    lisp.specbind(sym::INHIBIT_POINT_MOTION_HOOKS, sym::T)?;
    emacs_core::buffer::f_erase_buffer(lisp, &[])?;
    emacs_core::editfns::f_insert(lisp, &[string])?;
    for which in [WhichMatrix::Desired, WhichMatrix::Current] {
        if let Some(m) = lisp.window_matrix_mut(window, which) {
            clear_glyph_matrix(m);
        }
    }
    let (begv, begv_byte) = (lisp.cur().begv as i64, lisp.cur().begv_byte as i64);
    lisp.try_window(
        window,
        TextPos::new(begv, begv_byte),
        TRY_WINDOW_IGNORE_FONTS_CHANGE,
    )?;
    let y_limit = Value::fixnum(lisp.window(window).pixel_height);
    let size = emacs_core::xdisp::text_size::f_window_text_pixel_size(
        lisp,
        &[
            window,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            y_limit,
            Value::NIL,
            Value::NIL,
        ],
    )?;
    let internal_border = lisp.frame_internal_border_width(tip_f);
    let size_width = lisp.car(size)?;
    let size_height = lisp.cdr(size)?;
    let mut width = lisp.check_fixnum(size_width)? + 2 * internal_border;
    let height = lisp.check_fixnum(size_height)? + 2 * internal_border;
    width += column_width;
    let (root_x, root_y) = compute_tip_xy(lisp, tip_f, parms, dx, dy, width, height)?;
    let key = frame_key(lisp, tip_f) as u64;
    crate::with_display(|display| {
        display.resize_window(key, width.max(1) as u32, height.max(1) as u32);
        display.move_frame(key, root_x as i32, root_y as i32);
        display.show_frame(key, "");
    });
    lisp.set_frame_visible(tip_f, true);
    pgtk_cr_update_surface_desired_size(lisp, tip_f, width, height, false);
    lisp.window_mut(window).flags.must_be_updated_p = true;
    lisp.update_single_window(window)?;
    lisp.flush_frame(tip_f);
    lisp.set_buffer_internal(old_buffer);
    lisp.unbind_to(count_1)?;
    lisp.windows.windows_or_buffers_changed = old_windows_or_buffers_changed;
    start_timer(lisp, timeout)
}
