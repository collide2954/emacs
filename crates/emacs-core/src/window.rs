use crate::lisp::Lisp;
use crate::object::Weakness;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    lisp.put(sym::SCROLL_UP, sym::SCROLL_COMMAND, sym::T);
    lisp.put(sym::SCROLL_DOWN, sym::SCROLL_COMMAND, sym::T);
    lisp.defvar(sym::TEMP_BUFFER_SHOW_FUNCTION, Value::NIL);
    lisp.defvar(sym::MINIBUFFER_SCROLL_WINDOW, Value::NIL);
    lisp.defvar_bool(sym::MODE_LINE_IN_NON_SELECTED_WINDOWS, true);
    lisp.defvar(sym::OTHER_WINDOW_SCROLL_BUFFER, Value::NIL);
    lisp.defvar(sym::OTHER_WINDOW_SCROLL_DEFAULT, Value::NIL);
    lisp.defvar_bool(sym::AUTO_WINDOW_VSCROLL, true);
    lisp.defvar_int(sym::NEXT_SCREEN_CONTEXT_LINES, 2);
    for symbol in [
        sym::SCROLL_PRESERVE_SCREEN_POSITION,
        sym::WINDOW_POINT_INSERTION_TYPE,
        sym::WINDOW_BUFFER_CHANGE_FUNCTIONS,
        sym::WINDOW_SIZE_CHANGE_FUNCTIONS,
        sym::WINDOW_SELECTION_CHANGE_FUNCTIONS,
        sym::WINDOW_STATE_CHANGE_FUNCTIONS,
        sym::WINDOW_STATE_CHANGE_HOOK,
        sym::WINDOW_CONFIGURATION_CHANGE_HOOK,
        sym::WINDOW_RESTORE_KILLED_BUFFER_WINDOWS,
    ] {
        lisp.defvar(symbol, Value::NIL);
    }
    lisp.defvar(sym::RECENTER_REDISPLAY, sym::TTY);
    lisp.defvar(sym::WINDOW_COMBINATION_RESIZE, Value::NIL);
    lisp.defvar(sym::WINDOW_COMBINATION_LIMIT, sym::WINDOW_SIZE);
    let clone = lisp.cons(sym::CLONE_OF, sym::T);
    let persistent = lisp.list(&[clone]);
    lisp.defvar(sym::WINDOW_PERSISTENT_PARAMETERS, persistent);
    lisp.defvar_bool(sym::WINDOW_RESIZE_PIXELWISE, false);
    lisp.defvar_bool(sym::FAST_BUT_IMPRECISE_SCROLLING, false);
    let dead = lisp.make_hash_table_with(sym::EQL, Weakness::Value);
    lisp.defvar(sym::WINDOW_DEAD_WINDOWS_TABLE, dead);
    lisp.defvar_bool(sym::WINDOW_AUTO_REDRAW_ON_PARAMETER_CHANGE, true);
}
