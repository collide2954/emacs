use crate::error::LispResult;
use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[Subr::fixed(
    "cancel-kbd-macro-events",
    0,
    0,
    f_cancel_kbd_macro_events,
)];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::KBD_MACRO_TERMINATION_HOOK, Value::NIL);
    lisp.defvar_kboard(sym::DEFINING_KBD_MACRO, Kvar::DefiningKbdMacro);
    lisp.defvar(sym::EXECUTING_KBD_MACRO, Value::NIL);
    lisp.defvar_int(sym::EXECUTING_KBD_MACRO_INDEX, 0);
    lisp.defvar_kboard(sym::LAST_KBD_MACRO, Kvar::LastKbdMacro);
}

impl Lisp {
    pub fn at_end_of_macro_p(&mut self) -> LispResult<bool> {
        let kbd_macro = self.plain_value(sym::EXECUTING_KBD_MACRO);
        if kbd_macro.is_nil() {
            return Ok(false);
        }
        let length = crate::fns::f_length(self, &[kbd_macro])?;
        let index = self
            .plain_value(sym::EXECUTING_KBD_MACRO_INDEX)
            .as_fixnum()
            .unwrap_or(0);
        Ok(index >= length.as_fixnum().unwrap_or(0))
    }

    pub fn store_kbd_macro_char(&mut self, c: Value) {
        if self.kvar(Kvar::DefiningKbdMacro).is_nil() {
            return;
        }
        let kb = &mut self.kboards[self.current_kboard];
        if kb.kbd_macro_ptr == kb.kbd_macro_buffer.len() {
            kb.kbd_macro_buffer.push(c);
        } else {
            kb.kbd_macro_buffer[kb.kbd_macro_ptr] = c;
        }
        kb.kbd_macro_ptr += 1;
    }

    pub fn end_kbd_macro(&mut self) {
        self.set_kvar(Kvar::DefiningKbdMacro, Value::NIL);
        self.windows.update_mode_lines = 20;
        let kb = &self.kboards[self.current_kboard];
        let events = kb.kbd_macro_buffer[..kb.kbd_macro_end].to_vec();
        let macro_vector = self.vector(events);
        self.set_kvar(Kvar::LastKbdMacro, macro_vector);
    }
}

pub(crate) fn f_cancel_kbd_macro_events(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let kb = &mut lisp.kboards[lisp.current_kboard];
    kb.kbd_macro_ptr = kb.kbd_macro_end;
    Ok(Value::NIL)
}
