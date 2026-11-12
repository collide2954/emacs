use emacs_core::doprnt::Arg;
use emacs_core::error::LispResult;
use emacs_core::fns::f_provide;
use emacs_core::frame::ResourceType;
use emacs_core::lisp::Lisp;
use emacs_core::subr::Subr;
use emacs_core::symbols as sym;
use emacs_core::terminal::{FrameParmHandler, OutputMethod};
use emacs_core::value::Value;

use crate::pgtkterm::{
    allocate_output, fatal, frame_key, frame_title, pgtk_defined_color, pgtk_free_frame_resources,
    pgtk_make_frame_visible, pgtk_term_init, with_display_list, with_output,
};

const ICED_VERSION: &str = "0.14.0";
const TINY_SKIA_VERSION: &str = "0.11.4";

static SUBRS: &[Subr] = &[
    Subr::fixed("pgtk-set-resource", 2, 2, f_pgtk_set_resource),
    Subr::fixed("xw-display-color-p", 0, 1, f_xw_display_color_p),
    Subr::fixed("x-display-grayscale-p", 0, 1, f_x_display_grayscale_p),
    Subr::fixed("xw-color-defined-p", 1, 2, f_xw_color_defined_p),
    Subr::fixed("xw-color-values", 1, 2, f_xw_color_values),
    Subr::fixed("x-open-connection", 1, 3, f_x_open_connection),
    Subr::fixed("x-close-connection", 1, 1, f_x_close_connection),
    Subr::fixed(
        "x-server-max-request-size",
        0,
        1,
        f_x_server_max_request_size,
    ),
    Subr::fixed("x-display-screens", 0, 1, f_x_display_screens),
    Subr::fixed("x-display-backing-store", 0, 1, f_x_display_backing_store),
    Subr::fixed("x-display-visual-class", 0, 1, f_x_display_visual_class),
    Subr::fixed("x-display-save-under", 0, 1, f_x_display_save_under),
    Subr::fixed("x-display-planes", 0, 1, f_x_display_planes),
    Subr::fixed("x-display-color-cells", 0, 1, f_x_display_color_cells),
    Subr::fixed("x-display-list", 0, 0, f_x_display_list),
    Subr::fixed("x-create-frame", 1, 1, f_x_create_frame),
];

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

pub fn check_pgtk_display_info(lisp: &mut Lisp, object: Value) -> LispResult<Value> {
    if object.is_nil() {
        let sf = lisp.frames.selected_frame;
        if lisp.frame(sf).output_method == OutputMethod::Pgtk && lisp.frame_live_p(sf) {
            return Ok(lisp.frame(sf).terminal);
        }
        return match with_display_list(|displays| displays.first().map(|d| d.terminal)) {
            Some(terminal) => Ok(terminal),
            None => Err(lisp.error("Frames are not in use or not initialized")),
        };
    }
    if lisp.is_terminal(object) {
        let t = lisp.decode_live_terminal(object)?;
        if lisp.terminal(t).kind != OutputMethod::Pgtk {
            let id = lisp.terminal(t).id;
            return Err(lisp.verror("Terminal %d is not a display", &[Arg::Int(id)]));
        }
        return Ok(t);
    }
    if lisp.is_string(object) {
        return pgtk_display_info_for_name(lisp, object);
    }
    let f = lisp.decode_window_system_frame(object)?;
    Ok(lisp.frame(f).terminal)
}

fn is_wayland_display(name: &[u8]) -> bool {
    match name.strip_prefix(b"wayland-") {
        Some(rest) => !rest.is_empty() && rest.iter().all(u8::is_ascii_digit),
        None => false,
    }
}

fn pgtk_display_info_for_name(lisp: &mut Lisp, name: Value) -> LispResult<Value> {
    lisp.check_string(name)?;
    let entries: Vec<(Value, Value)> =
        with_display_list(|displays| displays.iter().map(|d| d.terminal).collect::<Vec<_>>())
            .into_iter()
            .map(|t| (t, lisp.display_info(t).name_list_element))
            .collect();
    let wayland = is_wayland_display(lisp.string_bytes(name));
    for (terminal, element) in entries {
        let display = lisp.xcar(element);
        let found = if wayland {
            is_wayland_display(lisp.string_bytes(display))
        } else {
            lisp.string_bytes(display) == lisp.string_bytes(name)
                && lisp.string_chars(display) == lisp.string_chars(name)
        };
        if found {
            return Ok(terminal);
        }
    }
    let invocation_name = lisp.plain_value(sym::INVOCATION_NAME);
    lisp.set_plain_value(sym::X_RESOURCE_NAME, invocation_name);
    lisp.validate_x_resource_name()?;
    let resource_name = lisp
        .string_bytes(lisp.plain_value(sym::X_RESOURCE_NAME))
        .to_vec();
    match pgtk_term_init(lisp, name, &resource_name)? {
        Some(terminal) => Ok(terminal),
        None => {
            let name = lisp.string_bytes(name).to_vec();
            Err(lisp.verror("Cannot connect to display server %s", &[Arg::Str(&name)]))
        }
    }
}

fn pgtk_get_defaults_value(_key: &[u8]) -> Option<Vec<u8>> {
    None
}

fn pgtk_set_defaults_value(lisp: &mut Lisp, _key: &[u8], _value: Option<&[u8]>) -> LispResult<()> {
    Err(lisp.error("gsettings not supported"))
}

fn f_pgtk_set_resource(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (attribute, value) = (args[0], args[1]);
    lisp.check_window_system(None)?;
    lisp.check_string(attribute)?;
    if !value.is_nil() {
        lisp.check_string(value)?;
    }
    let attr = lisp.string_bytes(attribute).to_vec();
    let res = if attr.first().is_some_and(u8::is_ascii_uppercase) {
        lisp.plain_value(sym::X_RESOURCE_CLASS)
    } else {
        lisp.plain_value(sym::X_RESOURCE_NAME)
    };
    let mut key = lisp.string_bytes(res).to_vec();
    key.push(b'.');
    key.extend_from_slice(&attr);
    let value = (!value.is_nil()).then(|| lisp.string_bytes(value).to_vec());
    pgtk_set_defaults_value(lisp, &key, value.as_deref())?;
    Ok(Value::NIL)
}

pub fn pgtk_get_string_resource(
    lisp: &mut Lisp,
    name: &[u8],
    class: &[u8],
) -> LispResult<Option<Vec<u8>>> {
    lisp.check_window_system(None)?;
    if !lisp.plain_value(sym::INHIBIT_X_RESOURCES).is_nil() {
        return Ok(None);
    }
    let Some(res) = pgtk_get_defaults_value(name).or_else(|| pgtk_get_defaults_value(class)) else {
        return Ok(None);
    };
    if res.len() >= 3 && res[..3].eq_ignore_ascii_case(b"YES") {
        return Ok(Some(b"true".to_vec()));
    }
    if res.len() >= 2 && res[..2].eq_ignore_ascii_case(b"NO") {
        return Ok(Some(b"false".to_vec()));
    }
    Ok(Some(res))
}

fn f_x_open_connection(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (mut display, must_succeed) = (arg(args, 0), arg(args, 2));
    if display.is_nil() {
        display = lisp.make_string(b"");
    }
    lisp.check_string(display)?;
    let resource_name = lisp.plain_value(sym::X_RESOURCE_NAME);
    let resource_name = if lisp.is_string(resource_name) {
        lisp.string_bytes(resource_name).to_vec()
    } else {
        Vec::new()
    };
    lisp.stack.push(display);
    let dpyinfo = pgtk_term_init(lisp, display, &resource_name);
    lisp.stack.pop();
    if dpyinfo?.is_none() {
        let name = String::from_utf8_lossy(lisp.string_bytes(display)).into_owned();
        let message = format!("Display on {name} not responding.\n");
        if !must_succeed.is_nil() {
            fatal(lisp, &message);
        }
        let name = lisp.string_bytes(display).to_vec();
        return Err(lisp.verror("Display on %s not responding.\n", &[Arg::Str(&name)]));
    }
    Ok(Value::NIL)
}

fn f_x_server_max_request_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    check_pgtk_display_info(lisp, arg(args, 0))?;
    Ok(Value::NIL)
}

fn f_x_display_screens(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    check_pgtk_display_info(lisp, arg(args, 0))?;
    Ok(Value::fixnum(1))
}

fn f_x_display_backing_store(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    check_pgtk_display_info(lisp, arg(args, 0))?;
    Ok(Value::NIL)
}

fn f_x_display_visual_class(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.intern("true-color"))
}

fn f_x_display_save_under(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    check_pgtk_display_info(lisp, arg(args, 0))?;
    Ok(Value::NIL)
}

fn f_x_display_planes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    check_pgtk_display_info(lisp, arg(args, 0))?;
    Ok(Value::fixnum(32))
}

fn f_x_display_color_cells(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let dpyinfo = check_pgtk_display_info(lisp, arg(args, 0))?;
    let n_planes = lisp.display_info(dpyinfo).n_planes;
    Ok(Value::fixnum(1 << n_planes.min(24)))
}

fn f_x_close_connection(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let dpyinfo = check_pgtk_display_info(lisp, args[0])?;
    if lisp.display_info(dpyinfo).reference_count > 0 {
        return Err(lisp.error("Display still has frames on it"));
    }
    crate::pgtkterm::pgtk_delete_terminal(lisp, dpyinfo)?;
    Ok(Value::NIL)
}

fn f_x_display_list(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let elements: Vec<Value> =
        with_display_list(|displays| displays.iter().map(|d| d.terminal).collect::<Vec<_>>())
            .into_iter()
            .map(|t| lisp.display_info(t).name_list_element)
            .collect();
    let mut result = Value::NIL;
    for element in elements {
        let name = lisp.xcar(element);
        result = lisp.cons(name, result);
    }
    Ok(result)
}

pub fn syms_of_pgtkfns(lisp: &mut Lisp) {
    let symbol = lisp.intern("x-cursor-fore-pixel");
    lisp.defvar(symbol, Value::NIL);
    let gtk = lisp.intern("gtk");
    f_provide(lisp, &[gtk, Value::NIL]).expect("provide gtk");
    let symbol = lisp.intern("gtk-version-string");
    let version = lisp.unibyte_string(ICED_VERSION.as_bytes());
    lisp.defvar(symbol, version);
    let cairo = lisp.intern("cairo");
    f_provide(lisp, &[cairo, Value::NIL]).expect("provide cairo");
    let symbol = lisp.intern("cairo-version-string");
    let version = lisp.unibyte_string(TINY_SKIA_VERSION.as_bytes());
    lisp.defvar(symbol, version);
    for subr in SUBRS {
        lisp.defsubr(subr);
    }
    for (name, value) in [
        ("x-gtk-use-old-file-dialog", false),
        ("x-gtk-show-hidden-files", false),
        ("x-gtk-file-dialog-help-text", true),
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar_bool(symbol, value);
    }
    let symbol = lisp.intern("x-max-tooltip-size");
    lisp.defvar(symbol, Value::NIL);
}

pub fn x_decode_color(
    lisp: &mut Lisp,
    f: Value,
    color_name: Value,
    mono_color: i64,
) -> LispResult<i64> {
    lisp.check_string(color_name)?;
    let terminal = lisp.frame(f).terminal;
    let n_planes = lisp
        .terminal(terminal)
        .display_info
        .as_ref()
        .map_or(0, |info| info.n_planes);
    if n_planes == 1 {
        return Ok(mono_color);
    }
    let name = lisp.string_bytes(color_name).to_vec();
    let (defined, color) = pgtk_defined_color(lisp, f, &name)?;
    if defined {
        return Ok(color.pixel);
    }
    Err(lisp.signal_error("Undefined color", color_name))
}

fn f_xw_color_defined_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_window_system_frame(arg(args, 1))?;
    let color = lisp.check_string(args[0])?;
    let name = lisp.string_bytes(color).to_vec();
    let (defined, _) = pgtk_defined_color(lisp, f, &name)?;
    Ok(lisp.bool(defined))
}

fn f_xw_color_values(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_window_system_frame(arg(args, 1))?;
    let color = lisp.check_string(args[0])?;
    let name = lisp.string_bytes(color).to_vec();
    let (defined, col) = pgtk_defined_color(lisp, f, &name)?;
    if !defined {
        return Ok(Value::NIL);
    }
    Ok(lisp.list(&[
        Value::fixnum(i64::from(col.red)),
        Value::fixnum(i64::from(col.green)),
        Value::fixnum(i64::from(col.blue)),
    ]))
}

fn f_xw_display_color_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    check_pgtk_display_info(lisp, arg(args, 0))?;
    Ok(sym::T)
}

fn f_x_display_grayscale_p(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(sym::T)
}

pub static PGTK_FRAME_PARM_HANDLERS: &[Option<FrameParmHandler>] = &[
    Some(Lisp::gui_set_autoraise),
    Some(Lisp::gui_set_autolower),
    Some(pgtk_set_background_color),
    None,
    Some(Lisp::gui_set_border_width),
    Some(pgtk_set_cursor_color),
    Some(Lisp::gui_set_cursor_type),
    Some(Lisp::gui_set_font),
    Some(pgtk_set_foreground_color),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(pgtk_set_mouse_color),
    Some(pgtk_explicitly_set_name),
    None,
    Some(Lisp::gui_set_scroll_bar_height),
    Some(pgtk_set_title),
    Some(Lisp::gui_set_unsplittable),
    Some(Lisp::gui_set_vertical_scroll_bars),
    Some(Lisp::gui_set_horizontal_scroll_bars),
    Some(Lisp::gui_set_visibility),
    None,
    None,
    None,
    None,
    Some(Lisp::gui_set_screen_gamma),
    Some(Lisp::gui_set_line_spacing),
    Some(Lisp::gui_set_left_fringe),
    Some(Lisp::gui_set_right_fringe),
    None,
    Some(Lisp::gui_set_fullscreen),
    Some(Lisp::gui_set_font_backend),
    Some(Lisp::gui_set_alpha),
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
    Some(Lisp::gui_set_no_special_glyphs),
    None,
    Some(Lisp::gui_set_borders_respect_alpha_background),
];

const DEFAULT_FONT_NAMES: &[&str] = &[
    "monospace-10",
    "-adobe-courier-medium-r-*-*-*-120-*-*-*-*-iso8859-1",
    "-misc-fixed-medium-r-normal-*-*-140-*-*-c-*-iso8859-1",
    "-*-*-medium-r-normal-*-*-140-*-*-c-*-iso8859-1",
    "-*-*-medium-r-*-*-*-*-*-*-c-*-iso8859-1",
    "-*-fixed-*-*-*-*-*-140-*-*-c-*-iso8859-1",
    "fixed",
];

pub fn pgtk_default_font_parameter(lisp: &mut Lisp, f: Value, parms: Value) -> LispResult<()> {
    let dpyinfo = lisp.frame_display_info(f);
    let mut font_param =
        lisp.gui_display_get_arg(dpyinfo, parms, sym::FONT, None, None, ResourceType::String)?;
    if font_param == sym::UNBOUND {
        font_param = Value::NIL;
    }
    let base = lisp.stack.len();
    lisp.stack.push(font_param);
    let r = (|| -> LispResult<()> {
        let mut font = if font_param.is_nil() {
            lisp.gui_display_get_arg(
                dpyinfo,
                parms,
                sym::FONT,
                Some("font"),
                Some("Font"),
                ResourceType::String,
            )?
        } else {
            font_param
        };
        lisp.stack.push(font);
        if !lisp.is_font(font) && !font.is_string() {
            font = Value::NIL;
            for name in DEFAULT_FONT_NAMES {
                let name = lisp.unibyte_string(name.as_bytes());
                font = lisp.font_open_by_name(f, name)?;
                if !font.is_nil() {
                    break;
                }
            }
            if font.is_nil() {
                return Err(lisp.error("No suitable font was found"));
            }
            lisp.stack.push(font);
        }
        lisp.gui_default_parameter(
            f,
            parms,
            sym::FONT,
            font,
            Some("font"),
            Some("Font"),
            ResourceType::String,
        )?;
        Ok(())
    })();
    lisp.stack.truncate(base);
    r
}

const BLACK_PIX_DEFAULT: i64 = 0x000000;
const WHITE_PIX_DEFAULT: i64 = 0xFFFFFF;
const PGTK_APP_NAME: &str = "Emacs";

fn pgtk_set_foreground_color(
    lisp: &mut Lisp,
    f: Value,
    arg: Value,
    _oldval: Value,
) -> LispResult<()> {
    let old_fg = with_output(lisp, f, |output| output.foreground_color);
    let fg = x_decode_color(lisp, f, arg, BLACK_PIX_DEFAULT)?;
    lisp.frame_mut(f).foreground_pixel = fg;
    let widget = with_output(lisp, f, |output| {
        output.foreground_color = fg;
        if output.widget && output.cursor_color == old_fg {
            output.cursor_color = fg;
            output.cursor_xgcv_background = fg;
        }
        output.widget
    });
    if widget {
        lisp.update_face_from_frame_parameter(f, sym::FOREGROUND_COLOR, arg)?;
        if lisp.frame(f).flags.visible {
            lisp.set_frame_garbaged(f);
        }
    }
    Ok(())
}

pub fn pgtk_clear_frame(lisp: &mut Lisp, f: Value) {
    let key = frame_key(lisp, f) as u64;
    let background = lisp.frame(f).background_pixel as u32;
    crate::with_display(|display| {
        if let Some(surface) = display.surface_mut(key) {
            let (width, height) = (surface.width() as i32, surface.height() as i32);
            surface.fill_rectangle(
                background,
                crate::surface::Rect::new(0, 0, width, height),
                false,
            );
        }
    });
}

fn pgtk_set_background_color(
    lisp: &mut Lisp,
    f: Value,
    arg: Value,
    _oldval: Value,
) -> LispResult<()> {
    let bg = x_decode_color(lisp, f, arg, WHITE_PIX_DEFAULT)?;
    lisp.frame_mut(f).background_pixel = bg;
    if lisp.frame(f).flags.visible {
        pgtk_clear_frame(lisp, f);
    }
    with_output(lisp, f, |output| {
        output.background_color = bg;
        output.cursor_xgcv_foreground = bg;
    });
    lisp.update_face_from_frame_parameter(f, sym::BACKGROUND_COLOR, arg)?;
    if lisp.frame(f).flags.visible {
        lisp.set_frame_garbaged(f);
    }
    Ok(())
}

fn pgtk_set_cursor_color(lisp: &mut Lisp, f: Value, arg: Value, _oldval: Value) -> LispResult<()> {
    let fore_pixel_var = lisp.intern("x-cursor-fore-pixel");
    let fore = lisp.find_symbol_value(fore_pixel_var);
    let mut fore_pixel = if fore.is_nil() {
        lisp.frame(f).background_pixel
    } else {
        x_decode_color(lisp, f, fore, WHITE_PIX_DEFAULT)?
    };
    let mut pixel = x_decode_color(lisp, f, arg, BLACK_PIX_DEFAULT)?;
    let background = lisp.frame(f).background_pixel;
    if pixel == background {
        pixel = with_output(lisp, f, |output| output.mouse_color);
        if pixel == fore_pixel {
            fore_pixel = background;
        }
    }
    with_output(lisp, f, |output| {
        output.cursor_foreground_color = fore_pixel;
        output.cursor_color = pixel;
        if output.widget {
            output.cursor_xgcv_background = pixel;
            output.cursor_xgcv_foreground = fore_pixel;
        }
    });
    lisp.update_face_from_frame_parameter(f, sym::CURSOR_COLOR, arg)
}

fn pgtk_set_mouse_color(
    _lisp: &mut Lisp,
    _f: Value,
    _arg: Value,
    _oldval: Value,
) -> LispResult<()> {
    Ok(())
}

fn pgtk_set_name_internal(lisp: &mut Lisp, f: Value, name: Value) {
    if with_output(lisp, f, |output| output.widget) {
        let key = frame_key(lisp, f) as u64;
        let title = String::from_utf8_lossy(lisp.string_bytes(name)).into_owned();
        crate::with_display(|display| display.set_title(key, &title));
    }
}

fn pgtk_set_name(lisp: &mut Lisp, f: Value, name: Value, explicit: bool) -> LispResult<()> {
    if explicit {
        if lisp.frame(f).flags.explicit_name && name.is_nil() {
            lisp.windows.update_mode_lines = 12;
        }
        lisp.frame_mut(f).flags.explicit_name = !name.is_nil();
    } else if lisp.frame(f).flags.explicit_name {
        return Ok(());
    }
    let mut name = if name.is_nil() {
        lisp.string(PGTK_APP_NAME)
    } else {
        lisp.check_string(name)?
    };
    let old = lisp.frame(f).name;
    if old.is_string() && lisp.string_bytes(name) == lisp.string_bytes(old) {
        return Ok(());
    }
    lisp.frame_mut(f).name = name;
    let title = lisp.frame(f).title;
    if !title.is_nil() {
        name = title;
    }
    pgtk_set_name_internal(lisp, f, name);
    Ok(())
}

fn pgtk_explicitly_set_name(
    lisp: &mut Lisp,
    f: Value,
    arg: Value,
    _oldval: Value,
) -> LispResult<()> {
    pgtk_set_name(lisp, f, arg, true)
}

pub fn pgtk_implicitly_set_name(
    lisp: &mut Lisp,
    f: Value,
    arg: Value,
    _oldval: Value,
) -> LispResult<()> {
    pgtk_set_name(lisp, f, arg, false)
}

fn pgtk_set_title(lisp: &mut Lisp, f: Value, name: Value, _old_name: Value) -> LispResult<()> {
    if name == lisp.frame(f).title {
        return Ok(());
    }
    lisp.windows.update_mode_lines = 22;
    lisp.frame_mut(f).title = name;
    let name = if name.is_nil() {
        lisp.frame(f).name
    } else {
        lisp.check_string(name)?
    };
    pgtk_set_name_internal(lisp, f, name);
    Ok(())
}

fn unwind_create_frame(lisp: &mut Lisp, frame: Value) -> LispResult<bool> {
    if !lisp.frame_live_p(frame) {
        return Ok(false);
    }
    if !lisp.frames.list.contains(&frame) {
        pgtk_free_frame_resources(lisp, frame)?;
        return Ok(true);
    }
    Ok(false)
}

fn f_x_create_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let parms = emacs_core::fns::f_copy_alist(lisp, &[args[0]])?;
    let base = lisp.stack.len();
    lisp.stack.push(parms);
    let count = lisp.specpdl.len();
    let mut frame = Value::NIL;
    let r = x_create_frame(lisp, parms, &mut frame);
    let unwound = if r.is_err() && !frame.is_nil() {
        unwind_create_frame(lisp, frame).map(|_| ())
    } else {
        Ok(())
    };
    let u = lisp.unbind_to(count);
    lisp.stack.truncate(base);
    let frame = r?;
    unwound?;
    u?;
    Ok(frame)
}

fn get_arg(
    lisp: &mut Lisp,
    dpyinfo: Value,
    parms: Value,
    param: Value,
    attribute: Option<&str>,
    class: Option<&str>,
    kind: ResourceType,
) -> LispResult {
    lisp.gui_display_get_arg(Some(dpyinfo), parms, param, attribute, class, kind)
}

fn x_create_frame(lisp: &mut Lisp, mut parms: Value, frame_out: &mut Value) -> LispResult {
    use ResourceType::{Boolean, Float, Number, String as Str, Symbol};
    let invocation_name = lisp.plain_value(sym::INVOCATION_NAME);
    lisp.set_plain_value(sym::X_RESOURCE_NAME, invocation_name);
    let mut display = lisp.gui_display_get_arg(None, parms, sym::TERMINAL, None, None, Number)?;
    if display == sym::UNBOUND {
        display = lisp.gui_display_get_arg(None, parms, sym::DISPLAY, None, None, Str)?;
    }
    if display == sym::UNBOUND {
        display = Value::NIL;
    }
    let dpyinfo = check_pgtk_display_info(lisp, display)?;
    let kb = lisp.terminal(dpyinfo).kboard.expect("display kboard");
    if lisp.terminal(dpyinfo).name.is_none() {
        return Err(lisp.error("Terminal is not live, can't create new frames on it"));
    }
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
    if name.is_string() {
        lisp.set_plain_value(sym::X_RESOURCE_NAME, name);
    }
    let mut parent = get_arg(lisp, dpyinfo, parms, sym::PARENT_ID, None, None, Number)?;
    if parent == sym::UNBOUND {
        parent = Value::NIL;
    }
    if !parent.is_nil() && !lisp.is_number(parent) {
        return Err(lisp.wrong_type(sym::NUMBERP, parent));
    }
    let tem = get_arg(
        lisp,
        dpyinfo,
        parms,
        sym::MINIBUFFER,
        Some("minibuffer"),
        Some("Minibuffer"),
        Symbol,
    )?;
    let mut minibuffer_only = false;
    let f = if tem == sym::NONE || tem.is_nil() {
        lisp.make_frame_without_minibuffer(Value::NIL, kb, display)?
    } else if tem == sym::ONLY {
        minibuffer_only = true;
        lisp.make_minibuffer_frame()?
    } else if lisp.is_window(tem) {
        lisp.make_frame_without_minibuffer(tem, kb, display)?
    } else {
        lisp.make_frame(true)?
    };
    lisp.stack.push(f);
    let mut parent_frame = get_arg(lisp, dpyinfo, parms, sym::PARENT_FRAME, None, None, Symbol)?;
    if !parent.is_nil()
        || parent_frame == sym::UNBOUND
        || parent_frame.is_nil()
        || !lisp.is_frame(parent_frame)
        || !lisp.frame_live_p(parent_frame)
        || lisp.frame(parent_frame).output_method != OutputMethod::Pgtk
    {
        parent_frame = Value::NIL;
    }
    lisp.frame_mut(f).parent_frame = parent_frame;
    lisp.store_frame_param(f, sym::PARENT_FRAME, parent_frame)?;
    let tem = get_arg(lisp, dpyinfo, parms, sym::UNDECORATED, None, None, Boolean)?;
    let undecorated = !tem.is_nil() && tem != sym::UNBOUND;
    lisp.frame_mut(f).flags.undecorated = undecorated;
    let value = lisp.bool(undecorated);
    lisp.store_frame_param(f, sym::UNDECORATED, value)?;
    let tem = get_arg(
        lisp,
        dpyinfo,
        parms,
        sym::OVERRIDE_REDIRECT,
        None,
        None,
        Boolean,
    )?;
    let override_redirect = !tem.is_nil() && tem != sym::UNBOUND;
    lisp.frame_mut(f).flags.override_redirect = override_redirect;
    let value = lisp.bool(override_redirect);
    lisp.store_frame_param(f, sym::OVERRIDE_REDIRECT, value)?;
    lisp.frame_set_id_from_params(f, parms)?;
    lisp.frame_mut(f).terminal = dpyinfo;
    lisp.frame_mut(f).output_method = OutputMethod::Pgtk;
    allocate_output(lisp, f);
    lisp.frame_mut(f).fontset = -1;
    with_output(lisp, f, |output| {
        output.white_relief_pixel = -1;
        output.black_relief_pixel = -1;
    });
    let icon_name = get_arg(
        lisp,
        dpyinfo,
        parms,
        sym::ICON_NAME,
        Some("iconName"),
        Some("Title"),
        Str,
    )?;
    lisp.frame_mut(f).icon_name = if icon_name.is_string() {
        icon_name
    } else {
        Value::NIL
    };
    *frame_out = f;
    lisp.frame_mut(f).foreground_pixel = -1;
    lisp.frame_mut(f).background_pixel = -1;
    with_output(lisp, f, |output| {
        output.cursor_color = -1;
        output.cursor_foreground_color = -1;
        output.border_pixel = -1;
        output.mouse_color = -1;
    });
    let black = lisp.string("black");
    lisp.stack.push(black);
    let pixel = x_decode_color(lisp, f, black, BLACK_PIX_DEFAULT)?;
    lisp.frame_mut(f).foreground_pixel = pixel;
    lisp.frame_mut(f).background_pixel = pixel;
    with_output(lisp, f, |output| {
        output.cursor_color = pixel;
        output.cursor_foreground_color = pixel;
        output.border_pixel = pixel;
        output.mouse_color = pixel;
    });
    let root_window = lisp.display_info(dpyinfo).root_window;
    with_output(lisp, f, |output| {
        if let Some(desc) = parent.as_fixnum() {
            output.parent_desc = desc;
            output.explicit_parent = true;
        } else {
            output.parent_desc = root_window;
            output.explicit_parent = false;
        }
    });
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
    if lisp.frame(f).font.is_nil() {
        return Err(lisp.error("Invalid frame font"));
    }
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
        Value::fixnum(0),
        Some("internalBorderWidth"),
        Some("internalBorderWidth"),
        Number,
    )?;
    if lisp.assq(sym::CHILD_FRAME_BORDER_WIDTH, parms).is_nil() {
        let value = get_arg(
            lisp,
            dpyinfo,
            parms,
            sym::CHILD_FRAME_BORDER_WIDTH,
            Some("childFrameBorder"),
            Some("childFrameBorder"),
            Number,
        )?;
        if value != sym::UNBOUND {
            let cell = lisp.cons(sym::CHILD_FRAME_BORDER_WIDTH, value);
            parms = lisp.cons(cell, parms);
            lisp.stack.push(parms);
        }
    }
    lisp.gui_default_parameter(
        f,
        parms,
        sym::CHILD_FRAME_BORDER_WIDTH,
        Value::NIL,
        Some("childFrameBorderWidth"),
        Some("childFrameBorderWidth"),
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
    lisp.gui_default_parameter(
        f,
        parms,
        sym::VERTICAL_SCROLL_BARS,
        sym::RIGHT,
        Some("verticalScrollBars"),
        Some("ScrollBars"),
        Symbol,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::HORIZONTAL_SCROLL_BARS,
        Value::NIL,
        Some("horizontalScrollBars"),
        Some("ScrollBars"),
        Symbol,
    )?;
    for (prop, color, xprop, xclass) in [
        (sym::FOREGROUND_COLOR, "black", "foreground", "Foreground"),
        (sym::BACKGROUND_COLOR, "white", "background", "Background"),
        (sym::MOUSE_COLOR, "black", "pointerColor", "Foreground"),
        (sym::BORDER_COLOR, "black", "borderColor", "BorderColor"),
    ] {
        let color = lisp.string(color);
        lisp.stack.push(color);
        lisp.gui_default_parameter(f, parms, prop, color, Some(xprop), Some(xclass), Str)?;
    }
    lisp.gui_default_parameter(
        f,
        parms,
        sym::SCREEN_GAMMA,
        Value::NIL,
        Some("screenGamma"),
        Some("ScreenGamma"),
        Float,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::LINE_SPACING,
        Value::NIL,
        Some("lineSpacing"),
        Some("LineSpacing"),
        Number,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::LEFT_FRINGE,
        Value::NIL,
        Some("leftFringe"),
        Some("LeftFringe"),
        Number,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::RIGHT_FRINGE,
        Value::NIL,
        Some("rightFringe"),
        Some("RightFringe"),
        Number,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::NO_SPECIAL_GLYPHS,
        Value::NIL,
        None,
        None,
        Boolean,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::SCROLL_BAR_FOREGROUND,
        Value::NIL,
        Some("scrollBarForeground"),
        Some("ScrollBarForeground"),
        Str,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::SCROLL_BAR_BACKGROUND,
        Value::NIL,
        Some("scrollBarBackground"),
        Some("ScrollBarBackground"),
        Str,
    )?;
    lisp.init_frame_faces(f)?;
    let tem = get_arg(lisp, dpyinfo, parms, sym::MIN_WIDTH, None, None, Number)?;
    if lisp.is_number(tem) {
        lisp.store_frame_param(f, sym::MIN_WIDTH, tem)?;
    }
    let tem = get_arg(lisp, dpyinfo, parms, sym::MIN_HEIGHT, None, None, Number)?;
    if lisp.is_number(tem) {
        lisp.store_frame_param(f, sym::MIN_HEIGHT, tem)?;
    }
    let geometry = &lisp.frame(f).geometry;
    let (width, height) = (
        geometry.text_cols * geometry.column_width,
        geometry.text_lines * geometry.line_height,
    );
    lisp.adjust_frame_size(f, width, height, 5, sym::X_CREATE_FRAME_1)?;
    for (prop, mode) in [
        (sym::MENU_BAR_LINES, sym::MENU_BAR_MODE),
        (sym::TAB_BAR_LINES, sym::TAB_BAR_MODE),
        (sym::TOOL_BAR_LINES, sym::TOOL_BAR_MODE),
    ] {
        let lines = Value::fixnum(i64::from(!lisp.find_symbol_value(mode).is_nil()));
        lisp.gui_default_parameter(f, parms, prop, lines, None, None, Number)?;
    }
    lisp.gui_default_parameter(
        f,
        parms,
        sym::BUFFER_PREDICATE,
        Value::NIL,
        Some("bufferPredicate"),
        Some("BufferPredicate"),
        Symbol,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::TITLE,
        Value::NIL,
        Some("title"),
        Some("Title"),
        Str,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::WAIT_FOR_WM,
        sym::T,
        Some("waitForWM"),
        Some("WaitForWM"),
        Boolean,
    )?;
    let position = lisp.frame(f).tool_bar_position;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::TOOL_BAR_POSITION,
        position,
        None,
        None,
        Symbol,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::INHIBIT_DOUBLE_BUFFERING,
        Value::NIL,
        Some("inhibitDoubleBuffering"),
        Some("InhibitDoubleBuffering"),
        Boolean,
    )?;
    let window_prompting = lisp.gui_figure_window_size(f, parms, true, true)?;
    let tem = get_arg(lisp, dpyinfo, parms, sym::UNSPLITTABLE, None, None, Boolean)?;
    lisp.frame_mut(f).flags.no_split = minibuffer_only || tem == sym::T;
    let key = frame_key(lisp, f) as u64;
    let (pixel_width, pixel_height) = (
        lisp.frame(f).geometry.pixel_width.max(1) as u32,
        lisp.frame(f).geometry.pixel_height.max(1) as u32,
    );
    let background = lisp.frame(f).background_pixel as u32;
    crate::with_display(|display| display.create_frame(key, pixel_width, pixel_height, background));
    with_output(lisp, f, |output| output.widget = true);
    let terminal = lisp.frame(f).terminal;
    lisp.terminal_mut(terminal).reference_count += 1;
    lisp.display_info_mut(terminal).reference_count += 1;
    lisp.frames.list.insert(0, f);
    lisp.gui_default_parameter(
        f,
        parms,
        sym::ICON_TYPE,
        sym::T,
        Some("bitmapIcon"),
        Some("BitmapIcon"),
        Boolean,
    )?;
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
        sym::SCROLL_BAR_WIDTH,
        Value::NIL,
        Some("scrollBarWidth"),
        Some("ScrollBarWidth"),
        Number,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::SCROLL_BAR_HEIGHT,
        Value::NIL,
        Some("scrollBarHeight"),
        Some("ScrollBarHeight"),
        Number,
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
    lisp.gui_default_parameter(
        f,
        parms,
        sym::NO_FOCUS_ON_MAP,
        Value::NIL,
        None,
        None,
        Boolean,
    )?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::NO_ACCEPT_FOCUS,
        Value::NIL,
        None,
        None,
        Boolean,
    )?;
    lisp.frame_mut(f).flags.can_set_window_size = true;
    let _ = window_prompting;
    let geometry = &lisp.frame(f).geometry;
    let (width, height) = (geometry.text_width, geometry.text_height);
    lisp.adjust_frame_size(f, width, height, 0, sym::X_CREATE_FRAME_2)?;
    lisp.gui_default_parameter(
        f,
        parms,
        sym::FULLSCREEN,
        Value::NIL,
        Some("fullscreen"),
        Some("Fullscreen"),
        Symbol,
    )?;
    if !with_output(lisp, f, |output| output.explicit_parent) {
        let mut visibility = get_arg(lisp, dpyinfo, parms, sym::VISIBILITY, None, None, Symbol)?;
        let height = get_arg(lisp, dpyinfo, parms, sym::HEIGHT, None, None, Number)?;
        let width = get_arg(lisp, dpyinfo, parms, sym::WIDTH, None, None, Number)?;
        if visibility == sym::ICON {
            lisp.frame_mut(f).flags.was_invisible = true;
        } else {
            if visibility == sym::UNBOUND {
                visibility = sym::T;
            }
            if !visibility.is_nil() {
                pgtk_make_frame_visible(lisp, f);
            } else {
                lisp.frame_mut(f).flags.was_invisible = true;
            }
        }
        let was_invisible =
            lisp.frame(f).flags.was_invisible && (height != sym::UNBOUND || width != sym::UNBOUND);
        lisp.frame_mut(f).flags.was_invisible = was_invisible;
        lisp.store_frame_param(f, sym::VISIBILITY, visibility)?;
    }
    lisp.gui_default_parameter(f, parms, sym::SKIP_TASKBAR, Value::NIL, None, None, Boolean)?;
    lisp.gui_default_parameter(f, parms, sym::Z_GROUP, Value::NIL, None, None, Symbol)?;
    let minibuffer_frame =
        lisp.kboards[kb].vars[emacs_core::keyboard::Kvar::DefaultMinibufferFrame as usize];
    if lisp.frame_has_minibuf_p(f)
        && (!lisp.is_frame(minibuffer_frame) || !lisp.frame_live_p(minibuffer_frame))
    {
        lisp.kboards[kb].vars[emacs_core::keyboard::Kvar::DefaultMinibufferFrame as usize] = f;
    }
    let mut tail = parms;
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        if elt.is_cons() && !lisp.xcar(elt).is_nil() {
            let alist = lisp.frame(f).param_alist;
            let alist = lisp.cons(elt, alist);
            lisp.frame_mut(f).param_alist = alist;
        }
        tail = lisp.xcdr(tail);
    }
    lisp.windows.window_list = Value::NIL;
    let _ = frame_title(lisp, f);
    Ok(f)
}
