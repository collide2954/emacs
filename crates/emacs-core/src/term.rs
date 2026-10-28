use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::terminal::OutputMethod;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("tty-display-color-p", 0, 1, f_tty_display_color_p),
    Subr::fixed("tty-display-color-cells", 0, 1, f_tty_display_color_cells),
    Subr::fixed("tty-no-underline", 0, 1, f_tty_no_underline),
    Subr::fixed("tty-enable-overline", 1, 2, f_tty_enable_overline),
    Subr::fixed("tty-type", 0, 1, f_tty_type),
    Subr::fixed("controlling-tty-p", 0, 1, f_controlling_tty_p),
    Subr::fixed("tty-top-frame", 0, 1, f_tty_top_frame),
    Subr::fixed("suspend-tty", 0, 1, f_suspend_tty),
    Subr::fixed("resume-tty", 0, 1, f_resume_tty),
    Subr::fixed(
        "tty--set-output-buffer-size",
        1,
        2,
        f_tty_set_output_buffer_size,
    ),
    Subr::fixed("tty--output-buffer-size", 0, 1, f_tty_output_buffer_size),
    Subr::fixed("tty-frame-geometry", 0, 1, f_tty_frame_geometry),
    Subr::fixed("tty-frame-edges", 0, 2, f_tty_frame_edges),
    Subr::fixed("tty-frame-restack", 2, 3, f_tty_frame_restack),
    Subr::fixed("tty-display-pixel-width", 0, 1, f_tty_display_pixel_width),
    Subr::fixed("tty-display-pixel-height", 0, 1, f_tty_display_pixel_height),
];

pub enum TtyTerminal {}

impl Lisp {
    pub fn decode_tty_terminal(&mut self, terminal: Value) -> LispResult<Option<TtyTerminal>> {
        let t = self.decode_live_terminal(terminal)?;
        Ok(match self.terminal(t).kind {
            OutputMethod::Initial => None,
        })
    }
}

fn f_tty_display_color_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.decode_tty_terminal(args[0])? {
        None => Ok(Value::NIL),
        Some(t) => match t {},
    }
}

fn f_tty_display_color_cells(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.decode_tty_terminal(args[0])? {
        None => Ok(Value::fixnum(0)),
        Some(t) => match t {},
    }
}

fn f_tty_type(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.decode_tty_terminal(args[0])? {
        None => Ok(Value::NIL),
        Some(t) => match t {},
    }
}

fn f_controlling_tty_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.decode_tty_terminal(args[0])? {
        None => Ok(Value::NIL),
        Some(t) => match t {},
    }
}

fn f_tty_no_underline(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let t = lisp.decode_live_terminal(args[0])?;
    match lisp.terminal(t).kind {
        OutputMethod::Initial => Ok(Value::NIL),
    }
}

fn f_tty_enable_overline(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let t = lisp.decode_live_terminal(args[1])?;
    match lisp.terminal(t).kind {
        OutputMethod::Initial => Ok(Value::NIL),
    }
}

fn f_tty_top_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let t = lisp.decode_live_terminal(args[0])?;
    match lisp.terminal(t).kind {
        OutputMethod::Initial => Ok(Value::NIL),
    }
}

fn f_suspend_tty(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.decode_tty_terminal(args[0])? {
        None => Err(lisp.error("Attempt to suspend a non-text terminal device")),
        Some(t) => match t {},
    }
}

fn f_resume_tty(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.decode_tty_terminal(args[0])? {
        None => Err(lisp.error("Attempt to resume a non-text terminal device")),
        Some(t) => match t {},
    }
}

fn f_tty_set_output_buffer_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (size, tty) = (args[0], args[1]);
    if !size
        .as_fixnum()
        .is_some_and(|n| (0..=isize::MAX as i64).contains(&n))
    {
        return Err(lisp.error("Invalid output buffer size"));
    }
    f_suspend_tty(lisp, &[tty])?;
    match lisp.decode_tty_terminal(tty)? {
        None => unreachable!(),
        Some(t) => match t {},
    }
}

fn f_tty_output_buffer_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.decode_tty_terminal(args[0])? {
        None => Err(lisp.error("Not a tty terminal")),
        Some(t) => match t {},
    }
}

fn tty_frame_geometry(lisp: &mut Lisp, frame: Value, _attribute: Value) -> LispResult {
    let f = lisp.decode_live_frame(frame)?;
    match lisp.frame(f).output_method {
        OutputMethod::Initial => Ok(Value::NIL),
    }
}

fn f_tty_frame_geometry(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    tty_frame_geometry(lisp, args[0], Value::NIL)
}

fn f_tty_frame_edges(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (frame, mut kind) = (args[0], args[1]);
    let outer = lisp.intern("outer-edges");
    let inner = lisp.intern("inner-edges");
    if kind != outer && kind != inner {
        kind = lisp.intern("native-edges");
    }
    tty_frame_geometry(lisp, frame, kind)
}

fn f_tty_frame_restack(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Err(lisp.error("tty-frame-restack is not implemented"))
}

fn tty_display_dimension(lisp: &Lisp, frame: Value) -> (i64, i64) {
    let frame = if lisp.is_frame(frame) {
        frame
    } else {
        lisp.frames.selected_frame
    };
    match lisp.frame(frame).output_method {
        OutputMethod::Initial => (80, 25),
    }
}

fn f_tty_display_pixel_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(Value::fixnum(tty_display_dimension(lisp, args[0]).0))
}

fn f_tty_display_pixel_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(Value::fixnum(tty_display_dimension(lisp, args[0]).1))
}

pub fn init_vars(lisp: &mut Lisp) {
    for (name, value) in [
        ("system-uses-terminfo", false),
        ("visible-cursor", true),
        ("tty-menu-calls-mouse-position-function", false),
        ("tty-cursor-movement-use-TAB-BS", false),
        ("tty-cursor-movement-use-TAB", true),
        ("tty-display-emoji-force-wide", true),
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar_bool(symbol, value);
    }
    for name in [
        "suspend-tty-functions",
        "resume-tty-functions",
        "auto-composition-emoji-tty-eligible-codepoints",
        "auto-composition-emoji-unqualified-tail-initials",
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar(symbol, Value::NIL);
    }
}
