use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
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
