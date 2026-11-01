use std::cell::RefCell;
use std::io::Write;

use emacs_core::character::{CHAR_ALT, CHAR_CTL, CHAR_HYPER, CHAR_META, CHAR_SUPER};
use emacs_core::editfns::f_system_name;
use emacs_core::error::LispResult;
use emacs_core::fns::f_provide;
use emacs_core::keyboard::f_set_input_interrupt_mode;
use emacs_core::lisp::Lisp;
use emacs_core::object::Weakness;
use emacs_core::symbols as sym;
use emacs_core::terminal::{DisplayInfo, OutputMethod};
use emacs_core::value::Value;
use emacs_core::xfaces::EmacsColor;

pub struct PgtkDisplayInfo {
    pub terminal: Value,
    pub x_id: u32,
    pub x_id_name: Vec<u8>,
    pub icon_bitmap_id: i32,
    pub scroll_x_per_char: i32,
    pub scroll_y_per_line: i32,
    pub connection: i32,
}

#[derive(Default)]
struct Pgtk {
    initialized: bool,
    display_id: u32,
    initial_display: Option<Vec<u8>>,
    displays: Vec<PgtkDisplayInfo>,
}

thread_local! {
    static PGTK: RefCell<Pgtk> = RefCell::new(Pgtk::default());
}

pub fn with_display_list<T>(f: impl FnOnce(&mut Vec<PgtkDisplayInfo>) -> T) -> T {
    PGTK.with(|p| f(&mut p.borrow_mut().displays))
}

pub fn mark_pgtkterm(roots: &mut Vec<Value>) {
    with_display_list(|displays| {
        for dpyinfo in displays.iter() {
            roots.push(dpyinfo.terminal);
        }
    });
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
    lisp.terminal_mut(terminal).hooks.get_string_resource =
        Some(crate::pgtkfns::pgtk_get_string_resource);
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
