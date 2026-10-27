use crate::error::LispResult;
use crate::frame::{FACE_TTY_DEFAULT_BG_COLOR, FACE_TTY_DEFAULT_FG_COLOR};
use crate::lisp::Lisp;
use crate::object::Weakness;
use crate::symbols as sym;
use crate::value::Value;

pub const LFACE_FAMILY_INDEX: usize = 1;
pub const LFACE_FOUNDRY_INDEX: usize = 2;
pub const LFACE_SWIDTH_INDEX: usize = 3;
pub const LFACE_HEIGHT_INDEX: usize = 4;
pub const LFACE_WEIGHT_INDEX: usize = 5;
pub const LFACE_SLANT_INDEX: usize = 6;
pub const LFACE_UNDERLINE_INDEX: usize = 7;
pub const LFACE_INVERSE_INDEX: usize = 8;
pub const LFACE_FOREGROUND_INDEX: usize = 9;
pub const LFACE_BACKGROUND_INDEX: usize = 10;
pub const LFACE_STIPPLE_INDEX: usize = 11;
pub const LFACE_OVERLINE_INDEX: usize = 12;
pub const LFACE_STRIKE_THROUGH_INDEX: usize = 13;
pub const LFACE_BOX_INDEX: usize = 14;
pub const LFACE_FONT_INDEX: usize = 15;
pub const LFACE_INHERIT_INDEX: usize = 16;
pub const LFACE_FONTSET_INDEX: usize = 17;
pub const LFACE_DISTANT_FOREGROUND_INDEX: usize = 18;
pub const LFACE_EXTEND_INDEX: usize = 19;
pub const LFACE_VECTOR_SIZE: usize = 20;

pub const FACE_ATTR_SYM: [Value; LFACE_VECTOR_SIZE] = [
    sym::FACE,
    sym::K_FAMILY,
    sym::K_FOUNDRY,
    sym::K_WIDTH,
    sym::K_HEIGHT,
    sym::K_WEIGHT,
    sym::K_SLANT,
    sym::K_UNDERLINE,
    sym::K_INVERSE_VIDEO,
    sym::K_FOREGROUND,
    sym::K_BACKGROUND,
    sym::K_STIPPLE,
    sym::K_OVERLINE,
    sym::K_STRIKE_THROUGH,
    sym::K_BOX,
    sym::K_FONT,
    sym::K_INHERIT,
    sym::K_FONTSET,
    sym::K_DISTANT_FOREGROUND,
    sym::K_EXTEND,
];

pub struct Faces {
    pub lface_id_to_name: Vec<Value>,
    pub face_change: bool,
    pub alternative_font_family_alist: Value,
    pub alternative_font_registry_alist: Value,
    pub font_sort_order: [i32; 4],
    pub tty_suppress_bold_inverse_default_colors_p: bool,
}

impl Default for Faces {
    fn default() -> Self {
        Faces {
            lface_id_to_name: Vec::new(),
            face_change: false,
            alternative_font_family_alist: Value::NIL,
            alternative_font_registry_alist: Value::NIL,
            font_sort_order: [0; 4],
            tty_suppress_bold_inverse_default_colors_p: false,
        }
    }
}

impl Faces {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&self.lface_id_to_name);
        out.push(self.alternative_font_family_alist);
        out.push(self.alternative_font_registry_alist);
    }
}

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_bool(sym::FACE_FILTERS_ALWAYS_MATCH, false);
    let defaults = lisp.make_hash_table_with(sym::EQ, Weakness::None);
    lisp.defvar(sym::FACE__NEW_FRAME_DEFAULTS, defaults);
    let stipple = lisp.string("gray3");
    lisp.defvar(sym::FACE_DEFAULT_STIPPLE, stipple);
    lisp.defvar(sym::TTY_DEFINED_COLOR_ALIST, Value::NIL);
    lisp.defvar(sym::SCALABLE_FONTS_ALLOWED, Value::NIL);
    lisp.defvar(sym::FACE_IGNORED_FONTS, Value::NIL);
    lisp.defvar(sym::FACE_REMAPPING_ALIST, Value::NIL);
    lisp.make_variable_buffer_local_builtin(sym::FACE_REMAPPING_ALIST);
    lisp.defvar(sym::FACE_FONT_RESCALE_ALIST, Value::NIL);
    lisp.defvar_int(sym::FACE_NEAR_SAME_COLOR_THRESHOLD, 30000);
    lisp.defvar(sym::FACE_FONT_LAX_MATCHED_ATTRIBUTES, sym::T);
}

impl Lisp {
    pub fn face_new_frame_defaults(&self) -> Value {
        self.find_symbol_value(sym::FACE__NEW_FRAME_DEFAULTS)
    }

    pub fn lfacep(&self, lface: Value) -> bool {
        self.is_vector(lface)
            && self.vector_items(lface).len() == LFACE_VECTOR_SIZE
            && self.vector_items(lface)[0] == sym::FACE
    }

    pub fn resolve_face_name(&mut self, face_name: Value, signal_p: bool) -> LispResult {
        let face_name = if self.is_string(face_name) {
            crate::lread::f_intern(self, &[face_name, Value::NIL])?
        } else {
            face_name
        };
        if face_name.is_nil() || !face_name.is_symbol() {
            return Ok(face_name);
        }
        let orig_face = face_name;
        let mut tortoise = face_name;
        let mut hare = face_name;
        let mut face_name;
        loop {
            face_name = hare;
            hare = self.get(hare, sym::FACE_ALIAS);
            if hare.is_nil() || !hare.is_symbol() {
                break;
            }
            face_name = hare;
            hare = self.get(hare, sym::FACE_ALIAS);
            if hare.is_nil() || !hare.is_symbol() {
                break;
            }
            tortoise = self.get(tortoise, sym::FACE_ALIAS);
            if hare == tortoise {
                if signal_p {
                    return Err(self.signal_list(sym::CIRCULAR_LIST, &[orig_face]));
                }
                return Ok(sym::DEFAULT);
            }
        }
        Ok(face_name)
    }

    pub fn lface_from_face_name_no_resolve(
        &mut self,
        f: Option<Value>,
        face_name: Value,
        signal_p: bool,
    ) -> LispResult {
        let lface = match f {
            Some(f) => {
                let table = self.frame(f).face_hash_table;
                self.gethash(face_name, table, Value::NIL)?
            }
            None => {
                let table = self.face_new_frame_defaults();
                let entry = self.gethash(face_name, table, Value::NIL)?;
                self.cdr_safe(entry)
            }
        };
        if signal_p && lface.is_nil() {
            let msg = self.string("Invalid face");
            return Err(self.signal_list(sym::ERROR, &[msg, face_name]));
        }
        Ok(lface)
    }

    pub fn lface_from_face_name(
        &mut self,
        f: Option<Value>,
        face_name: Value,
        signal_p: bool,
    ) -> LispResult {
        let face_name = self.resolve_face_name(face_name, signal_p)?;
        self.lface_from_face_name_no_resolve(f, face_name, signal_p)
    }

    pub fn tty_color_name(&mut self, f: Value, idx: i64) -> LispResult {
        if idx >= 0
            && !self
                .heap
                .symbol_ref(sym::TTY_COLOR_BY_INDEX)
                .function
                .is_nil()
        {
            let coldesc = self.funcall(sym::TTY_COLOR_BY_INDEX, &[Value::fixnum(idx), f])?;
            if !coldesc.is_nil() {
                return Ok(self.car_safe(coldesc));
            }
        }
        if idx == FACE_TTY_DEFAULT_FG_COLOR {
            return Ok(self.string("unspecified-fg"));
        }
        if idx == FACE_TTY_DEFAULT_BG_COLOR {
            return Ok(self.string("unspecified-bg"));
        }
        Ok(sym::UNSPECIFIED)
    }

    pub fn update_face_from_frame_parameter(
        &mut self,
        f: Value,
        param: Value,
        new_value: Value,
    ) -> LispResult<()> {
        let table = self.frame(f).face_hash_table;
        if self.hash_table(table).count == 0 {
            return Ok(());
        }
        let value = if self.is_string(new_value) {
            new_value
        } else {
            sym::UNSPECIFIED
        };
        let (face, index) = if param == sym::FOREGROUND_COLOR {
            (sym::DEFAULT, LFACE_FOREGROUND_INDEX)
        } else if param == sym::BACKGROUND_COLOR {
            self.funcall(sym::FRAME_SET_BACKGROUND_MODE, &[f])?;
            (sym::DEFAULT, LFACE_BACKGROUND_INDEX)
        } else if param == sym::BORDER_COLOR {
            (sym::BORDER, LFACE_BACKGROUND_INDEX)
        } else if param == sym::CURSOR_COLOR {
            (sym::CURSOR, LFACE_BACKGROUND_INDEX)
        } else if param == sym::MOUSE_COLOR {
            (sym::MOUSE, LFACE_BACKGROUND_INDEX)
        } else {
            return Ok(());
        };
        let lface = self.lface_from_face_name(Some(f), face, true)?;
        self.aset_value(lface, Value::fixnum(index as i64), value)?;
        if self.get(face, sym::FACE_NO_INHERIT).is_nil() {
            let fr = self.frame_mut(f);
            fr.flags.face_change = true;
            fr.flags.redisplay = true;
        }
        Ok(())
    }
}
