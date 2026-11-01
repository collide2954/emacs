use emacs_core::doprnt::Arg;
use emacs_core::error::LispResult;
use emacs_core::fns::f_provide;
use emacs_core::lisp::Lisp;
use emacs_core::subr::Subr;
use emacs_core::symbols as sym;
use emacs_core::terminal::OutputMethod;
use emacs_core::value::Value;

use crate::pgtkterm::{fatal, pgtk_defined_color, pgtk_term_init, with_display_list};

const ICED_VERSION: &str = "0.14.0";
const TINY_SKIA_VERSION: &str = "0.11.4";

static SUBRS: &[Subr] = &[
    Subr::fixed("pgtk-set-resource", 2, 2, f_pgtk_set_resource),
    Subr::fixed("xw-display-color-p", 0, 1, f_xw_display_color_p),
    Subr::fixed("x-display-grayscale-p", 0, 1, f_x_display_grayscale_p),
    Subr::fixed("xw-color-defined-p", 1, 2, f_xw_color_defined_p),
    Subr::fixed("xw-color-values", 1, 2, f_xw_color_values),
    Subr::fixed("x-open-connection", 1, 3, f_x_open_connection),
    Subr::fixed("x-display-list", 0, 0, f_x_display_list),
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
