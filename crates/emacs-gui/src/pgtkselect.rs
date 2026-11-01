use emacs_core::error::LispResult;
use emacs_core::lisp::Lisp;
use emacs_core::subr::Subr;
use emacs_core::value::Value;

use crate::pgtkterm::with_output;

static SUBRS: &[Subr] = &[Subr::fixed(
    "pgtk-register-dnd-targets",
    2,
    2,
    f_pgtk_register_dnd_targets,
)];

pub fn syms_of_pgtkselect(lisp: &mut Lisp) {
    for subr in SUBRS {
        lisp.defsubr(subr);
    }
    for name in [
        "selection-converter-alist",
        "pgtk-lost-selection-functions",
        "pgtk-sent-selection-functions",
        "pgtk-sent-selection-hooks",
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar(symbol, Value::NIL);
    }
    let symbol = lisp.intern("pgtk-selection-timeout");
    lisp.defvar_int(symbol, 0);
    let symbol = lisp.intern("pgtk-selection-alias-alist");
    lisp.defvar(symbol, Value::NIL);
}

fn f_pgtk_register_dnd_targets(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (frame, targets) = (args[0], args[1]);
    let f = lisp.decode_window_system_frame(frame)?;
    lisp.check_list(targets)?;
    let length = lisp.list_length(targets)?;
    let mut entries = Vec::with_capacity(length);
    let base = lisp.stack.len();
    let mut tem = lisp.for_each_tail(targets);
    while tem.tail.is_cons() {
        let t = lisp.xcar(tem.tail);
        let t = match lisp.check_string(t) {
            Ok(t) => t,
            Err(e) => {
                lisp.stack.truncate(base);
                return Err(e);
            }
        };
        entries.push(lisp.string_bytes(t).to_vec());
        if let Err(e) = lisp.tail_step(&mut tem) {
            lisp.stack.truncate(base);
            return Err(e);
        }
    }
    lisp.stack.truncate(base);
    lisp.check_list_end(tem.tail, targets)?;
    if entries.len() != length {
        unreachable!("dnd target list changed while it was read");
    }
    with_output(lisp, f, |output| output.drag_dest_targets = Some(entries));
    Ok(Value::NIL)
}
