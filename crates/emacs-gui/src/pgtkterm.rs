use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;

use emacs_core::character::{CHAR_ALT, CHAR_CTL, CHAR_HYPER, CHAR_META, CHAR_SUPER};
use emacs_core::editfns::f_system_name;
use emacs_core::error::LispResult;
use emacs_core::fns::f_provide;
use emacs_core::keyboard::f_set_input_interrupt_mode;
use emacs_core::lisp::Lisp;
use emacs_core::object::Weakness;
use emacs_core::symbols as sym;
use emacs_core::termhooks::{EventKind, InputEvent, KbdEvent};
use emacs_core::terminal::{DisplayInfo, OutputMethod};
use emacs_core::value::Value;
use emacs_core::xfaces::EmacsColor;

use crate::protocol::Notice;

pub struct PgtkDisplayInfo {
    pub terminal: Value,
    pub x_id: u32,
    pub x_id_name: Vec<u8>,
    pub icon_bitmap_id: i32,
    pub scroll_x_per_char: i32,
    pub scroll_y_per_line: i32,
    pub connection: i32,
    pub last_user_time: u64,
    pub x_focus_frame: Value,
    pub x_focus_event_frame: Value,
}

const FOCUS_EXPLICIT: i32 = 2;

#[derive(Default)]
struct Pgtk {
    initialized: bool,
    display_id: u32,
    initial_display: Option<Vec<u8>>,
    displays: Vec<PgtkDisplayInfo>,
    evq: Vec<KbdEvent>,
}

#[derive(Clone, Debug, Default)]
pub struct PgtkOutput {
    pub foreground_color: i64,
    pub background_color: i64,
    pub cursor_color: i64,
    pub cursor_foreground_color: i64,
    pub border_pixel: i64,
    pub mouse_color: i64,
    pub cursor_xgcv_foreground: i64,
    pub cursor_xgcv_background: i64,
    pub white_relief_pixel: i64,
    pub black_relief_pixel: i64,
    pub relief_background: i64,
    pub relief_background_valid_p: bool,
    pub parent_desc: i64,
    pub explicit_parent: bool,
    pub preferred_width: i64,
    pub preferred_height: i64,
    pub widget: bool,
    pub drag_dest_targets: Option<Vec<Vec<u8>>>,
    pub focus_state: i32,
}

thread_local! {
    static PGTK: RefCell<Pgtk> = RefCell::new(Pgtk::default());
    static OUTPUTS: RefCell<HashMap<i64, PgtkOutput>> = RefCell::new(HashMap::new());
}

pub fn frame_key(lisp: &Lisp, f: Value) -> i64 {
    lisp.frame(f).id
}

pub fn with_output<T>(lisp: &Lisp, f: Value, g: impl FnOnce(&mut PgtkOutput) -> T) -> T {
    let key = frame_key(lisp, f);
    OUTPUTS.with(|outputs| g(outputs.borrow_mut().entry(key).or_default()))
}

pub fn has_output(lisp: &Lisp, f: Value) -> bool {
    let key = frame_key(lisp, f);
    OUTPUTS.with(|outputs| outputs.borrow().contains_key(&key))
}

pub fn allocate_output(lisp: &Lisp, f: Value) {
    let key = frame_key(lisp, f);
    OUTPUTS.with(|outputs| outputs.borrow_mut().insert(key, PgtkOutput::default()));
}

fn free_output(lisp: &Lisp, f: Value) {
    let key = frame_key(lisp, f);
    OUTPUTS.with(|outputs| outputs.borrow_mut().remove(&key));
}

pub fn pgtk_set_window_size(
    lisp: &mut Lisp,
    f: Value,
    _change_gravity: bool,
    pixelwidth: i64,
    pixelheight: i64,
) -> LispResult<()> {
    with_output(lisp, f, |output| {
        output.preferred_width = pixelwidth;
        output.preferred_height = pixelheight;
    });
    let key = frame_key(lisp, f) as u64;
    crate::with_display(|display| {
        display.resize_frame(key, pixelwidth.max(1) as u32, pixelheight.max(1) as u32)
    });
    Ok(())
}

pub fn frame_title(lisp: &Lisp, f: Value) -> String {
    let fr = lisp.frame(f);
    let name = if fr.title.is_nil() { fr.name } else { fr.title };
    if name.is_string() {
        String::from_utf8_lossy(lisp.string_bytes(name)).into_owned()
    } else {
        String::new()
    }
}

pub fn pgtk_make_frame_visible(lisp: &mut Lisp, f: Value) {
    if !lisp.frame(f).flags.visible {
        let key = frame_key(lisp, f) as u64;
        let title = frame_title(lisp, f);
        crate::with_display(|display| display.show_frame(key, &title));
    }
}

pub fn pgtk_make_frame_invisible(lisp: &mut Lisp, f: Value) {
    if lisp.frame(f).flags.visible {
        let fr = lisp.frame_mut(f);
        fr.flags.visible = false;
        fr.flags.iconified = false;
    }
}

pub fn pgtk_make_frame_visible_invisible(lisp: &mut Lisp, f: Value, visible: bool) {
    if visible {
        pgtk_make_frame_visible(lisp, f);
    } else {
        pgtk_make_frame_invisible(lisp, f);
    }
}

pub fn pgtk_free_frame_resources(lisp: &mut Lisp, f: Value) {
    lisp.free_frame_faces(f);
    let terminal = lisp.frame(f).terminal;
    if with_dpyinfo(lisp, f, |d| d.x_focus_frame) == f {
        let new_focus = lisp.frame_parent_frame(f);
        if lisp.display_info(terminal).highlight_frame == f {
            lisp.display_info_mut(terminal).highlight_frame = Value::NIL;
        }
        if !new_focus.is_nil() && lisp.frame_live_p(new_focus) && has_output(lisp, new_focus) {
            pgtk_new_focus_frame(lisp, terminal, new_focus);
        } else {
            with_dpyinfo(lisp, f, |d| d.x_focus_frame = Value::NIL);
            pgtk_frame_rehighlight(lisp, terminal);
        }
    }
    if lisp.display_info(terminal).highlight_frame == f {
        lisp.display_info_mut(terminal).highlight_frame = Value::NIL;
    }
    with_dpyinfo(lisp, f, |d| {
        if d.x_focus_event_frame == f {
            d.x_focus_event_frame = Value::NIL;
        }
    });
    let key = frame_key(lisp, f) as u64;
    crate::with_display(|display| display.close_frame(key));
    free_output(lisp, f);
}

pub fn pgtk_destroy_window(lisp: &mut Lisp, f: Value) {
    let terminal = lisp.frame(f).terminal;
    pgtk_free_frame_resources(lisp, f);
    lisp.display_info_mut(terminal).reference_count -= 1;
}

pub fn pgtk_delete_terminal(lisp: &mut Lisp, terminal: Value) -> LispResult<()> {
    if lisp.terminal(terminal).name.is_none() {
        return Ok(());
    }
    with_display_list(|displays| {
        if let Some(dpyinfo) = displays.iter_mut().find(|d| d.terminal == terminal) {
            dpyinfo.connection = -1;
        }
    });
    pgtk_delete_display(lisp, terminal)
}

fn pgtk_delete_display(lisp: &mut Lisp, dpyinfo: Value) -> LispResult<()> {
    let terminal = lisp
        .terminals
        .list
        .iter()
        .copied()
        .find(|&t| lisp.terminal(t).kind == OutputMethod::Pgtk && t == dpyinfo);
    if let Some(t) = terminal {
        lisp.delete_terminal(t)?;
    }
    with_display_list(|displays| displays.retain(|d| d.terminal != dpyinfo));
    Ok(())
}

pub fn with_display_list<T>(f: impl FnOnce(&mut Vec<PgtkDisplayInfo>) -> T) -> T {
    PGTK.with(|p| f(&mut p.borrow_mut().displays))
}

pub fn mark_pgtkterm(roots: &mut Vec<Value>) {
    PGTK.with(|p| {
        let p = p.borrow();
        for event in &p.evq {
            event.trace(roots);
        }
        for dpyinfo in &p.displays {
            roots.extend_from_slice(&[
                dpyinfo.terminal,
                dpyinfo.x_focus_frame,
                dpyinfo.x_focus_event_frame,
            ]);
        }
    });
}

fn with_dpyinfo<T>(lisp: &Lisp, f: Value, g: impl FnOnce(&mut PgtkDisplayInfo) -> T) -> T {
    let terminal = lisp.frame(f).terminal;
    with_display_list(|displays| {
        let dpyinfo = displays
            .iter_mut()
            .find(|d| d.terminal == terminal)
            .expect("frame display info");
        g(dpyinfo)
    })
}

pub fn focus_frame(lisp: &Lisp, f: Value) -> Option<Value> {
    let focus = with_dpyinfo(lisp, f, |d| d.x_focus_frame);
    (!focus.is_nil()).then_some(focus)
}

fn pgtk_any_window_to_frame(lisp: &Lisp, id: u64) -> Option<Value> {
    lisp.frames.list.iter().copied().find(|&f| {
        lisp.frame(f).output_method == OutputMethod::Pgtk
            && lisp.frame_live_p(f)
            && frame_key(lisp, f) as u64 == id
    })
}

fn evq_enqueue(lisp: &Lisp, ev: KbdEvent) {
    let mut frame = Value::NIL;
    if !matches!(
        ev.kind,
        EventKind::SelectionRequest | EventKind::SelectionClear
    ) {
        if lisp.is_window(ev.frame_or_window) {
            frame = lisp.window(ev.frame_or_window).frame;
        }
        if lisp.is_frame(ev.frame_or_window) {
            frame = ev.frame_or_window;
        }
    }
    PGTK.with(|p| p.borrow_mut().evq.push(ev));
    if !frame.is_nil() {
        with_dpyinfo(lisp, frame, |dpyinfo| {
            if dpyinfo.last_user_time < ev.timestamp {
                dpyinfo.last_user_time = ev.timestamp;
            }
        });
    }
}

fn evq_flush(lisp: &mut Lisp, hold_quit: &mut KbdEvent) -> LispResult<i32> {
    let mut n = 0;
    loop {
        let ev = PGTK.with(|p| {
            let mut p = p.borrow_mut();
            (!p.evq.is_empty()).then(|| p.evq.remove(0))
        });
        let Some(ev) = ev else {
            break;
        };
        lisp.kbd_buffer_store_buffered_event(ev, Some(hold_quit))?;
        n += 1;
    }
    Ok(n)
}

fn key_press_event(lisp: &mut Lisp, input: InputEvent) {
    let Some(f) = pgtk_any_window_to_frame(lisp, input.frame) else {
        return;
    };
    with_dpyinfo(lisp, f, |dpyinfo| dpyinfo.last_user_time = input.timestamp);
    let extra = lisp
        .plain_value(sym::EXTRA_KEYBOARD_MODIFIERS)
        .as_fixnum()
        .unwrap_or(0);
    let modifiers = input.modifiers
        | (extra
            & (emacs_core::keyboard::CTRL_MODIFIER
                | emacs_core::keyboard::META_MODIFIER
                | emacs_core::keyboard::SHIFT_MODIFIER
                | emacs_core::keyboard::SUPER_MODIFIER
                | emacs_core::keyboard::HYPER_MODIFIER
                | emacs_core::keyboard::ALT_MODIFIER));
    let arg = match &input.arg {
        Some(text) => lisp.unibyte_string(text.as_bytes()),
        None => Value::NIL,
    };
    evq_enqueue(
        lisp,
        KbdEvent {
            kind: input.kind,
            code: input.code,
            modifiers,
            timestamp: input.timestamp,
            frame_or_window: f,
            arg,
            ..KbdEvent::default()
        },
    );
}

fn delete_event(lisp: &mut Lisp, input: InputEvent) {
    if let Some(f) = pgtk_any_window_to_frame(lisp, input.frame) {
        evq_enqueue(
            lisp,
            KbdEvent {
                kind: EventKind::DeleteWindow,
                frame_or_window: f,
                ..KbdEvent::default()
            },
        );
    }
}

fn frame_highlight(lisp: &mut Lisp, f: Value) {
    lisp.gui_update_cursor(f, true);
}

fn frame_unhighlight(lisp: &mut Lisp, f: Value) {
    lisp.gui_update_cursor(f, true);
}

fn pgtk_frame_rehighlight(lisp: &mut Lisp, terminal: Value) {
    let old_highlight = lisp.display_info(terminal).highlight_frame;
    let focus = with_display_list(|displays| {
        displays
            .iter()
            .find(|d| d.terminal == terminal)
            .expect("display info")
            .x_focus_frame
    });
    let mut highlight = Value::NIL;
    if !focus.is_nil() {
        let redirect = lisp.frame(focus).focus_frame;
        highlight = if lisp.is_frame(redirect) {
            redirect
        } else {
            focus
        };
        if !lisp.frame_live_p(highlight) {
            lisp.frame_mut(focus).focus_frame = Value::NIL;
            highlight = focus;
        }
    }
    lisp.display_info_mut(terminal).highlight_frame = highlight;
    if !old_highlight.is_nil() {
        frame_unhighlight(lisp, old_highlight);
    }
    if !highlight.is_nil() {
        frame_highlight(lisp, highlight);
    }
}

fn pgtk_new_focus_frame(lisp: &mut Lisp, terminal: Value, frame: Value) {
    with_display_list(|displays| {
        if let Some(d) = displays.iter_mut().find(|d| d.terminal == terminal)
            && frame != d.x_focus_frame
        {
            d.x_focus_frame = frame;
        }
    });
    pgtk_frame_rehighlight(lisp, terminal);
}

fn pgtk_focus_changed(lisp: &mut Lisp, is_enter: bool, state: i32, frame: Value) -> KbdEvent {
    let terminal = lisp.frame(frame).terminal;
    let mut event = KbdEvent::default();
    if is_enter {
        if with_dpyinfo(lisp, frame, |d| d.x_focus_event_frame) != frame {
            pgtk_new_focus_frame(lisp, terminal, frame);
            with_dpyinfo(lisp, frame, |d| d.x_focus_event_frame = frame);
            let terminal_frame = lisp.plain_value(sym::TERMINAL_FRAME_2);
            let pgtk_terminal_frame = lisp.is_frame(terminal_frame)
                && lisp.frame(terminal_frame).output_method == OutputMethod::Pgtk;
            event.arg = lisp.bool(
                (terminal_frame.is_nil() || !pgtk_terminal_frame) && lisp.frames.list.len() > 1,
            );
            event.kind = EventKind::FocusIn;
            event.frame_or_window = frame;
        }
        with_output(lisp, frame, |output| output.focus_state |= state);
    } else {
        with_output(lisp, frame, |output| output.focus_state &= !state);
        if with_dpyinfo(lisp, frame, |d| d.x_focus_event_frame) == frame {
            with_dpyinfo(lisp, frame, |d| d.x_focus_event_frame = Value::NIL);
            pgtk_new_focus_frame(lisp, terminal, Value::NIL);
            event.kind = EventKind::FocusOut;
            event.frame_or_window = frame;
        }
        if lisp.frame(frame).flags.pointer_invisible {
            lisp.frame_mut(frame).flags.pointer_invisible = false;
        }
    }
    event
}

fn focus_event(lisp: &mut Lisp, input: InputEvent, is_enter: bool) {
    let Some(frame) = pgtk_any_window_to_frame(lisp, input.frame) else {
        return;
    };
    let event = pgtk_focus_changed(lisp, is_enter, FOCUS_EXPLICIT, frame);
    if event.kind != EventKind::NoEvent {
        evq_enqueue(lisp, event);
    }
}

fn pgtk_handle_notice(lisp: &mut Lisp, notice: Notice) {
    if let Notice::Input(input) = notice {
        match input.kind {
            EventKind::AsciiKeystroke
            | EventKind::MultibyteCharKeystroke
            | EventKind::NonAsciiKeystroke => key_press_event(lisp, input),
            EventKind::DeleteWindow => delete_event(lisp, input),
            EventKind::FocusIn => focus_event(lisp, input, true),
            EventKind::FocusOut => focus_event(lisp, input, false),
            _ => {}
        }
    }
}

pub fn pgtk_read_socket(
    lisp: &mut Lisp,
    _terminal: Value,
    hold_quit: &mut KbdEvent,
) -> LispResult<i32> {
    let count = evq_flush(lisp, hold_quit)?;
    if count > 0 {
        return Ok(count);
    }
    let notices = crate::with_display(|display| display.read_notices(None))
        .and_then(Result::ok)
        .unwrap_or_default();
    for notice in notices {
        pgtk_handle_notice(lisp, notice);
    }
    evq_flush(lisp, hold_quit)
}

pub fn pgtk_wait_for_input(_lisp: &mut Lisp, timeout: std::time::Duration) -> bool {
    match crate::with_display(|display| display.wait(timeout)) {
        Some(Ok(ready)) => ready,
        _ => {
            std::thread::sleep(timeout);
            false
        }
    }
}

fn initialize_display_info(terminal: Value) -> (PgtkDisplayInfo, DisplayInfo) {
    let mut common = DisplayInfo::new(Value::NIL);
    common.resx = 96.0;
    common.resy = 96.0;
    common.color_p = true;
    common.n_planes = 32;
    common.root_window = 42;
    common.smallest_font_height = 1;
    common.smallest_char_width = 1;
    let pgtk = PgtkDisplayInfo {
        terminal,
        x_id: 0,
        x_id_name: Vec::new(),
        icon_bitmap_id: 0,
        scroll_x_per_char: 0,
        scroll_y_per_line: 0,
        connection: 0,
        last_user_time: 0,
        x_focus_frame: Value::NIL,
        x_focus_event_frame: Value::NIL,
    };
    (pgtk, common)
}

fn default_display_name() -> Vec<u8> {
    ["WAYLAND_DISPLAY", "DISPLAY"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|name| name.to_string_lossy().into_owned())
        .find(|name| !name.is_empty())
        .unwrap_or_else(|| "iced".to_string())
        .into_bytes()
}

fn display_open(display_name: &[u8]) -> bool {
    display_name.is_empty() || display_name == default_display_name()
}

fn same_x_server(lisp: &mut Lisp, name1: &[u8], name2: &[u8]) -> LispResult<bool> {
    let sysname = f_system_name(lisp, &[])?;
    let system_name = lisp.string_bytes(sysname).to_vec();
    let length_until_period = system_name
        .iter()
        .position(|&b| b == b'.')
        .unwrap_or(system_name.len());
    let strip = |name: &[u8]| -> Vec<u8> {
        let mut name = name;
        if name.starts_with(b"unix:") {
            name = &name[4..];
        }
        if name.starts_with(&system_name) && name.get(system_name.len()) == Some(&b':') {
            name = &name[system_name.len()..];
        }
        if name.starts_with(&system_name[..length_until_period])
            && name.get(length_until_period) == Some(&b':')
        {
            name = &name[length_until_period..];
        }
        name.to_vec()
    };
    let (name1, name2) = (strip(name1), strip(name2));
    let mut seen_colon = false;
    let mut i = 0;
    while i < name1.len() && name2.get(i) == Some(&name1[i]) {
        if name1[i] == b':' {
            seen_colon = true;
        }
        if seen_colon && name1[i] == b'.' {
            return Ok(true);
        }
        i += 1;
    }
    let end1 = name1.get(i).copied().unwrap_or(0);
    let end2 = name2.get(i).copied().unwrap_or(0);
    Ok(seen_colon && (end1 == b'.' || end1 == 0) && (end2 == b'.' || end2 == 0))
}

pub fn pgtk_term_init(
    lisp: &mut Lisp,
    display_name: Value,
    _resource_name: &[u8],
) -> LispResult<Option<Value>> {
    let was_initialized = PGTK.with(|p| p.borrow().initialized);
    if !was_initialized {
        f_set_input_interrupt_mode(lisp, &[sym::T])?;
        lisp.set_plain_value(sym::BAUD_RATE, Value::fixnum(19200));
        PGTK.with(|p| p.borrow_mut().initialized = true);
    }
    let mut dpy_name = lisp.string_bytes(display_name).to_vec();
    let initial_display = PGTK.with(|p| p.borrow().initial_display.clone());
    if dpy_name.is_empty()
        && let Some(initial) = initial_display
    {
        dpy_name = initial;
    }
    if was_initialized {
        if !display_open(&dpy_name) {
            return Ok(None);
        }
    } else {
        let initial = default_display_name();
        PGTK.with(|p| p.borrow_mut().initial_display = Some(initial.clone()));
        dpy_name = initial;
    }
    let lisp_dpy_name = lisp.make_string(&dpy_name);
    lisp.stack.push(lisp_dpy_name);
    let terminal = lisp.create_terminal(OutputMethod::Pgtk)?;
    let hooks = &mut lisp.terminal_mut(terminal).hooks;
    hooks.get_string_resource = Some(crate::pgtkfns::pgtk_get_string_resource);
    hooks.defined_color = Some(pgtk_defined_color);
    hooks.set_new_font = Some(pgtk_new_font);
    hooks.set_window_size = Some(pgtk_set_window_size);
    hooks.frame_visible_invisible = Some(pgtk_make_frame_visible_invisible);
    hooks.default_font_parameter = Some(crate::pgtkfns::pgtk_default_font_parameter);
    hooks.frame_parm_handlers = Some(crate::pgtkfns::PGTK_FRAME_PARM_HANDLERS);
    hooks.delete_frame = Some(pgtk_destroy_window);
    hooks.delete_terminal = Some(pgtk_delete_terminal);
    hooks.read_socket = Some(pgtk_read_socket);
    hooks.update_begin = Some(crate::pgtkdraw::pgtk_update_begin);
    hooks.update_end = Some(crate::pgtkdraw::pgtk_update_end);
    lisp.terminal_mut(terminal).rif = Some(&crate::pgtkdraw::PGTK_REDISPLAY_INTERFACE);
    let (mut dpyinfo, mut common) = initialize_display_info(terminal);
    let names: Vec<(Value, Vec<u8>)> =
        with_display_list(|displays| displays.iter().map(|d| d.terminal).collect::<Vec<_>>())
            .into_iter()
            .map(|t| {
                let element = lisp.display_info(t).name_list_element;
                (t, lisp.string_bytes(lisp.xcar(element)).to_vec())
            })
            .collect();
    let mut share = None;
    for (t, name) in names {
        if same_x_server(lisp, &name, &dpy_name)? {
            share = Some(t);
            break;
        }
    }
    let kboard = match share.and_then(|t| lisp.terminal(t).kboard) {
        Some(kboard) => kboard,
        None => {
            let kboard = lisp.allocate_kboard(sym::PGTK);
            if lisp.current_kboard == lisp.initial_kboard {
                lisp.current_kboard = kboard;
            }
            kboard
        }
    };
    lisp.terminal_mut(terminal).kboard = Some(kboard);
    lisp.kboards[kboard].reference_count += 1;
    common.name_list_element = lisp.cons(lisp_dpy_name, Value::NIL);
    lisp.stack.pop();
    common.smallest_font_height = 1;
    common.smallest_char_width = 1;
    lisp.terminal_mut(terminal).name = Some(String::from_utf8_lossy(&dpy_name).into_owned());
    let system_name = f_system_name(lisp, &[])?;
    let invocation_name = lisp.plain_value(sym::INVOCATION_NAME);
    let mut x_id_name = lisp.string_bytes(invocation_name).to_vec();
    x_id_name.push(b'@');
    x_id_name.extend_from_slice(lisp.string_bytes(system_name));
    dpyinfo.x_id = PGTK.with(|p| {
        let mut p = p.borrow_mut();
        p.display_id += 1;
        p.display_id
    });
    dpyinfo.x_id_name = x_id_name;
    dpyinfo.icon_bitmap_id = -1;
    common.resx = 96.0;
    common.resy = 96.0;
    lisp.terminal_mut(terminal).display_info = Some(Box::new(common));
    dpyinfo.scroll_x_per_char = 1;
    dpyinfo.scroll_y_per_line = 1;
    dpyinfo.connection = -1;
    with_display_list(|displays| displays.insert(0, dpyinfo));
    lisp.gui_init_fringe(&crate::pgtkdraw::PGTK_REDISPLAY_INTERFACE);
    Ok(Some(terminal))
}

pub fn fatal(lisp: &mut Lisp, message: &str) -> ! {
    let _ = lisp.stderr.write_all(b"emacs: ");
    let _ = lisp.stderr.write_all(message.as_bytes());
    if !message.ends_with('\n') {
        let _ = lisp.stderr.write_all(b"\n");
    }
    let _ = lisp.stderr.flush();
    std::process::exit(1);
}

pub fn syms_of_pgtkterm(lisp: &mut Lisp) {
    let modifier_value = lisp.intern("modifier-value");
    for (name, modifier) in [
        ("alt", CHAR_ALT),
        ("hyper", CHAR_HYPER),
        ("meta", CHAR_META),
        ("super", CHAR_SUPER),
        ("control", CHAR_CTL),
    ] {
        let symbol = lisp.intern(name);
        lisp.put(symbol, modifier_value, Value::fixnum(modifier));
    }
    for name in [
        "x-ctrl-keysym",
        "x-alt-keysym",
        "x-hyper-keysym",
        "x-meta-keysym",
        "x-super-keysym",
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar(symbol, Value::NIL);
    }
    let symbol = lisp.intern("x-use-underline-position-properties");
    lisp.defvar_bool(symbol, true);
    let symbol = lisp.intern("x-underline-at-descent-line");
    lisp.defvar_bool(symbol, false);
    let symbol = lisp.intern("x-toolkit-scroll-bars");
    let gtk = lisp.intern("gtk");
    lisp.defvar(symbol, gtk);
    let symbol = lisp.intern("pgtk-wait-for-event-timeout");
    let timeout = lisp.float(0.1);
    lisp.defvar(symbol, timeout);
    let symbol = lisp.intern("pgtk-keysym-table");
    let table = lisp.make_hash_table_with(sym::EQL, Weakness::None);
    lisp.defvar(symbol, table);
    f_provide(lisp, &[sym::PGTK, Value::NIL]).expect("provide pgtk");
}

pub fn pgtk_parse_color(color_name: &[u8]) -> Option<EmacsColor> {
    let rgba = crate::gdkrgba::gdk_rgba_parse(color_name)?;
    let red = (rgba[0] * 65535.0) as u16;
    let green = (rgba[1] * 65535.0) as u16;
    let blue = (rgba[2] * 65535.0) as u16;
    Some(EmacsColor {
        pixel: (i64::from(red >> 8) << 16) | (i64::from(green >> 8) << 8) | i64::from(blue >> 8),
        red,
        green,
        blue,
    })
}

pub fn pgtk_defined_color(
    _lisp: &mut Lisp,
    _f: Value,
    name: &[u8],
) -> LispResult<(bool, EmacsColor)> {
    Ok(match pgtk_parse_color(name) {
        Some(color) => (true, color),
        None => (false, EmacsColor::default()),
    })
}

pub fn pgtk_new_font(lisp: &mut Lisp, f: Value, font_object: Value, fontset: i64) -> LispResult {
    let fontset = if fontset < 0 {
        lisp.fontset_from_font(font_object)?
    } else {
        fontset
    };
    lisp.frame_mut(f).fontset = fontset;
    if lisp.frame(f).font == font_object {
        return Ok(font_object);
    }
    lisp.frame_mut(f).font = font_object;
    let font = lisp.font_object(font_object);
    let (baseline_offset, average_width) = (font.baseline_offset, font.average_width);
    let (font_ascent, font_descent) = lisp.get_font_ascent_descent(font_object);
    let fr = lisp.frame_mut(f);
    fr.baseline_offset = i64::from(baseline_offset);
    let geometry = &mut fr.geometry;
    geometry.column_width = i64::from(average_width);
    geometry.line_height = i64::from(font_ascent + font_descent);
    geometry.tab_bar_height = geometry.tab_bar_lines * geometry.line_height;
    let wid = geometry.column_width;
    geometry.config_scroll_bar_cols = if geometry.config_scroll_bar_width > 0 {
        (geometry.config_scroll_bar_width + wid - 1) / wid
    } else {
        (14 + wid - 1) / wid
    };
    let height = geometry.line_height;
    geometry.config_scroll_bar_lines = if geometry.config_scroll_bar_height > 0 {
        (geometry.config_scroll_bar_height + height - 1) / height
    } else {
        (14 + height - 1) / height
    };
    let frame_id = lisp.frame(f).id as u64;
    let has_widget =
        crate::with_display(|display| display.surface(frame_id).is_some()) == Some(true);
    if has_widget {
        let geometry = &lisp.frame(f).geometry;
        let width = geometry.text_cols * geometry.column_width;
        let height = geometry.text_lines * geometry.line_height;
        lisp.adjust_frame_size(f, width, height, 3, sym::FONT)?;
    }
    Ok(font_object)
}
