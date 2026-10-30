use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::character;
use crate::error::LispResult;
use crate::eval::SetMode;
use crate::lisp::{Lisp, SpecBinding};
use crate::object::{BoolVector, Forward, Localized, SymbolValue, TrappedWrite, Vectorlike};
use crate::subr::{Subr, SubrKind};
use crate::symbols as sym;
use crate::value::{MOST_NEGATIVE_FIXNUM, MOST_POSITIVE_FIXNUM, Tag, Value};

pub const SUBRS: &[Subr] = &[
    Subr::fixed("eq", 2, 2, f_eq),
    Subr::fixed("null", 1, 1, f_null),
    Subr::fixed("type-of", 1, 1, f_type_of),
    Subr::fixed("cl-type-of", 1, 1, f_cl_type_of),
    Subr::fixed("consp", 1, 1, f_consp),
    Subr::fixed("atom", 1, 1, f_atom),
    Subr::fixed("listp", 1, 1, f_listp),
    Subr::fixed("nlistp", 1, 1, f_nlistp),
    Subr::fixed("bare-symbol-p", 1, 1, f_bare_symbol_p),
    Subr::fixed("symbol-with-pos-p", 1, 1, f_symbol_with_pos_p),
    Subr::fixed("symbolp", 1, 1, f_symbolp),
    Subr::fixed("keywordp", 1, 1, f_keywordp),
    Subr::fixed("vectorp", 1, 1, f_vectorp),
    Subr::fixed("recordp", 1, 1, f_recordp),
    Subr::fixed("stringp", 1, 1, f_stringp),
    Subr::fixed("multibyte-string-p", 1, 1, f_multibyte_string_p),
    Subr::fixed("char-table-p", 1, 1, f_char_table_p),
    Subr::fixed("vector-or-char-table-p", 1, 1, f_vector_or_char_table_p),
    Subr::fixed("closurep", 1, 1, f_closurep),
    Subr::fixed("byte-code-function-p", 1, 1, f_byte_code_function_p),
    Subr::fixed("interpreted-function-p", 1, 1, f_interpreted_function_p),
    Subr::fixed("bool-vector-p", 1, 1, f_bool_vector_p),
    Subr::fixed("arrayp", 1, 1, f_arrayp),
    Subr::fixed("sequencep", 1, 1, f_sequencep),
    Subr::fixed("bufferp", 1, 1, f_bufferp),
    Subr::fixed("markerp", 1, 1, f_markerp),
    Subr::fixed("user-ptrp", 1, 1, f_false),
    Subr::fixed("subrp", 1, 1, f_subrp),
    Subr::fixed("module-function-p", 1, 1, f_false),
    Subr::fixed("char-or-string-p", 1, 1, f_char_or_string_p),
    Subr::fixed("integerp", 1, 1, f_integerp),
    Subr::fixed("integer-or-marker-p", 1, 1, f_integer_or_marker_p),
    Subr::fixed("natnump", 1, 1, f_natnump),
    Subr::fixed("numberp", 1, 1, f_numberp),
    Subr::fixed("number-or-marker-p", 1, 1, f_number_or_marker_p),
    Subr::fixed("floatp", 1, 1, f_floatp),
    Subr::fixed("threadp", 1, 1, f_false),
    Subr::fixed("mutexp", 1, 1, f_false),
    Subr::fixed("condition-variable-p", 1, 1, f_false),
    Subr::fixed("car", 1, 1, f_car),
    Subr::fixed("car-safe", 1, 1, f_car_safe),
    Subr::fixed("cdr", 1, 1, f_cdr),
    Subr::fixed("cdr-safe", 1, 1, f_cdr_safe),
    Subr::fixed("setcar", 2, 2, f_setcar),
    Subr::fixed("setcdr", 2, 2, f_setcdr),
    Subr::fixed("boundp", 1, 1, f_boundp),
    Subr::fixed("fboundp", 1, 1, f_fboundp),
    Subr::fixed("makunbound", 1, 1, f_makunbound),
    Subr::fixed("fmakunbound", 1, 1, f_fmakunbound),
    Subr::fixed("symbol-function", 1, 1, f_symbol_function),
    Subr::fixed("symbol-plist", 1, 1, f_symbol_plist),
    Subr::fixed("symbol-name", 1, 1, f_symbol_name),
    Subr::fixed("bare-symbol", 1, 1, f_bare_symbol),
    Subr::fixed("symbol-with-pos-pos", 1, 1, f_symbol_with_pos_pos),
    Subr::fixed("remove-pos-from-symbol", 1, 1, f_remove_pos_from_symbol),
    Subr::fixed("position-symbol", 2, 2, f_position_symbol),
    Subr::fixed("fset", 2, 2, f_fset),
    Subr::fixed("defalias", 2, 3, f_defalias),
    Subr::fixed("setplist", 2, 2, f_setplist),
    Subr::fixed("subr-arity", 1, 1, f_subr_arity),
    Subr::fixed("subr-name", 1, 1, f_subr_name),
    Subr::fixed("native-comp-function-p", 1, 1, f_false),
    Subr::fixed("subr-native-lambda-list", 1, 1, f_false),
    Subr::fixed("subr-type", 1, 1, f_false),
    Subr::fixed("interactive-form", 1, 1, f_interactive_form),
    Subr::fixed("command-modes", 1, 1, f_command_modes),
    Subr::fixed("indirect-variable", 1, 1, f_indirect_variable),
    Subr::fixed("symbol-value", 1, 1, f_symbol_value),
    Subr::fixed("set", 2, 2, f_set),
    Subr::fixed("add-variable-watcher", 2, 2, f_add_variable_watcher),
    Subr::fixed("remove-variable-watcher", 2, 2, f_remove_variable_watcher),
    Subr::fixed("get-variable-watchers", 1, 1, f_get_variable_watchers),
    Subr::fixed("default-boundp", 1, 1, f_default_boundp),
    Subr::fixed("default-value", 1, 1, f_default_value),
    Subr::fixed("set-default", 2, 2, f_set_default),
    Subr::fixed(
        "make-variable-buffer-local",
        1,
        1,
        f_make_variable_buffer_local,
    )
    .interactive("vMake Variable Buffer Local: "),
    Subr::fixed("make-local-variable", 1, 1, f_make_local_variable)
        .interactive("vMake Local Variable: "),
    Subr::fixed("kill-local-variable", 1, 1, f_kill_local_variable)
        .interactive("vKill Local Variable: "),
    Subr::fixed("local-variable-p", 1, 2, f_local_variable_p),
    Subr::fixed("local-variable-if-set-p", 1, 2, f_local_variable_if_set_p),
    Subr::fixed("variable-binding-locus", 1, 1, f_variable_binding_locus),
    Subr::fixed("indirect-function", 1, 2, f_indirect_function),
    Subr::fixed("aref", 2, 2, f_aref),
    Subr::fixed("aset", 3, 3, f_aset),
    Subr::many("=", 1, f_eqlsign),
    Subr::many("<", 1, f_lss),
    Subr::many(">", 1, f_gtr),
    Subr::many("<=", 1, f_leq),
    Subr::many(">=", 1, f_geq),
    Subr::fixed("/=", 2, 2, f_neq),
    Subr::fixed("number-to-string", 1, 1, f_number_to_string),
    Subr::fixed("string-to-number", 1, 2, f_string_to_number),
    Subr::many("+", 0, f_plus),
    Subr::many("-", 0, f_minus),
    Subr::many("*", 0, f_times),
    Subr::many("/", 1, f_quo),
    Subr::fixed("%", 2, 2, f_rem),
    Subr::fixed("mod", 2, 2, f_mod),
    Subr::many("max", 1, f_max),
    Subr::many("min", 1, f_min),
    Subr::many("logand", 0, f_logand),
    Subr::many("logior", 0, f_logior),
    Subr::many("logxor", 0, f_logxor),
    Subr::fixed("logcount", 1, 1, f_logcount),
    Subr::fixed("ash", 2, 2, f_ash),
    Subr::fixed("1+", 1, 1, f_add1),
    Subr::fixed("1-", 1, 1, f_sub1),
    Subr::fixed("lognot", 1, 1, f_lognot),
    Subr::fixed("byteorder", 0, 0, f_byteorder),
    Subr::fixed("bool-vector-exclusive-or", 2, 3, f_bool_vector_exclusive_or),
    Subr::fixed("bool-vector-union", 2, 3, f_bool_vector_union),
    Subr::fixed("bool-vector-intersection", 2, 3, f_bool_vector_intersection),
    Subr::fixed(
        "bool-vector-set-difference",
        2,
        3,
        f_bool_vector_set_difference,
    ),
    Subr::fixed("bool-vector-subsetp", 2, 2, f_bool_vector_subsetp),
    Subr::fixed("bool-vector-not", 1, 2, f_bool_vector_not),
    Subr::fixed(
        "bool-vector-count-population",
        1,
        1,
        f_bool_vector_count_population,
    ),
    Subr::fixed(
        "bool-vector-count-consecutive",
        3,
        3,
        f_bool_vector_count_consecutive,
    ),
];

pub fn init_vars(lisp: &mut Lisp) {
    let most_positive = Value::fixnum(MOST_POSITIVE_FIXNUM);
    let most_negative = Value::fixnum(MOST_NEGATIVE_FIXNUM);
    lisp.defvar(sym::MOST_POSITIVE_FIXNUM, most_positive);
    lisp.make_constant(sym::MOST_POSITIVE_FIXNUM);
    lisp.defvar(sym::MOST_NEGATIVE_FIXNUM, most_negative);
    lisp.make_constant(sym::MOST_NEGATIVE_FIXNUM);
    lisp.defvar(sym::SYMBOLS_WITH_POS_ENABLED, Value::NIL);
    lisp.defvar(sym::PRINT_SYMBOLS_BARE, Value::NIL);
    lisp.defvar_int(sym::INTEGER_WIDTH, 65536);
}

#[derive(Clone, Debug)]
pub enum Num {
    Fix(i64),
    Big(BigInt),
    Float(f64),
}

impl Num {
    pub fn to_f64(&self) -> f64 {
        match self {
            Num::Fix(n) => *n as f64,
            Num::Big(b) => b.to_f64().unwrap_or(f64::NAN),
            Num::Float(f) => *f,
        }
    }

    pub fn to_bigint(&self) -> Option<BigInt> {
        match self {
            Num::Fix(n) => Some(BigInt::from(*n)),
            Num::Big(b) => Some(b.clone()),
            Num::Float(_) => None,
        }
    }
}

fn bool_value(b: bool) -> Value {
    if b { sym::T } else { Value::NIL }
}

impl Lisp {
    pub fn bool(&self, b: bool) -> Value {
        bool_value(b)
    }

    pub fn coerce_marker(&mut self, v: Value) -> Value {
        if let Some(pos) = self.marker_position_value(v) {
            return pos;
        }
        v
    }

    pub fn number(&mut self, v: Value) -> LispResult<Num> {
        let v = self.coerce_marker(v);
        if let Some(n) = v.as_fixnum() {
            return Ok(Num::Fix(n));
        }
        if v.is_float() {
            return Ok(Num::Float(self.heap.float_value(v)));
        }
        if let Some(b) = self.bignum_ref(v) {
            return Ok(Num::Big(b.clone()));
        }
        Err(self.wrong_type(sym::NUMBER_OR_MARKER_P, v))
    }

    pub fn integer_num(&mut self, v: Value) -> LispResult<Num> {
        let v = self.coerce_marker(v);
        if let Some(n) = v.as_fixnum() {
            return Ok(Num::Fix(n));
        }
        if let Some(b) = self.bignum_ref(v) {
            return Ok(Num::Big(b.clone()));
        }
        Err(self.wrong_type(sym::INTEGER_OR_MARKER_P, v))
    }

    pub fn make_num(&mut self, n: Num) -> Value {
        match n {
            Num::Fix(n) => self.integer(n),
            Num::Big(b) => self.bignum(b),
            Num::Float(f) => self.float(f),
        }
    }

    pub fn float_value_of(&mut self, v: Value) -> LispResult<f64> {
        Ok(self.number(v)?.to_f64())
    }

    pub fn check_integer(&mut self, v: Value) -> LispResult<Num> {
        if let Some(n) = v.as_fixnum() {
            return Ok(Num::Fix(n));
        }
        if let Some(b) = self.bignum_ref(v) {
            return Ok(Num::Big(b.clone()));
        }
        Err(self.wrong_type(sym::INTEGERP, v))
    }

    pub fn arithcompare(&mut self, a: Value, b: Value) -> LispResult<(bool, bool, bool)> {
        let a = self.number(a)?;
        let b = self.number(b)?;
        Ok(compare_nums(&a, &b))
    }

    pub fn find_symbol_value(&self, symbol: Value) -> Value {
        let mut symbol = symbol;
        loop {
            match &self.heap.symbol_ref(symbol).value {
                SymbolValue::Alias(target) => symbol = *target,
                SymbolValue::Plain(v) => return *v,
                SymbolValue::Forwarded(Forward::Int(n)) => return Value::fixnum(*n),
                SymbolValue::Forwarded(Forward::Bool(b)) => return bool_value(*b),
                SymbolValue::Forwarded(Forward::Kboard(slot)) => return self.kboard_slot(*slot),
                SymbolValue::Localized(l) => {
                    let default = l.default;
                    let buffer = self.current_buffer_value();
                    return self.buffer_local_get(buffer, symbol).unwrap_or(default);
                }
            }
        }
    }

    pub fn symbol_value(&mut self, symbol: Value) -> LispResult {
        let value = self.find_symbol_value(symbol);
        if value == sym::UNBOUND {
            return Err(self.signal_list(sym::VOID_VARIABLE, &[symbol]));
        }
        Ok(value)
    }

    pub fn set(&mut self, symbol: Value, value: Value) -> LispResult<()> {
        self.set_internal(symbol, value, Value::NIL, SetMode::Set)
    }

    pub fn indirect_variable(&self, mut symbol: Value) -> Value {
        while let SymbolValue::Alias(target) = self.heap.symbol_ref(symbol).value {
            symbol = target;
        }
        symbol
    }

    pub fn set_internal(
        &mut self,
        symbol: Value,
        newval: Value,
        where_: Value,
        mode: SetMode,
    ) -> LispResult<()> {
        let symbol = self.check_symbol(symbol)?;
        let voide = newval == sym::UNBOUND;
        match self.heap.symbol_ref(symbol).trapped_write {
            TrappedWrite::NoWrite => {
                let keyword = self.keywordp(symbol);
                if !keyword || newval != self.find_symbol_value(symbol) {
                    return Err(self.signal_list(sym::SETTING_CONSTANT, &[symbol]));
                }
                return Ok(());
            }
            TrappedWrite::Trapped => {
                let operation = match mode {
                    SetMode::Bind => sym::LET,
                    SetMode::Unbind => sym::UNLET,
                    SetMode::Makunbound => sym::MAKUNBOUND,
                    SetMode::Set if voide => sym::MAKUNBOUND,
                    SetMode::Set => sym::SET,
                };
                let shown = if voide { Value::NIL } else { newval };
                self.notify_variable_watchers(symbol, shown, operation, where_)?;
            }
            TrappedWrite::Untrapped => {}
        }
        let target = self.indirect_variable(symbol);
        let current = self.heap.symbol_ref(target).value.clone();
        match current {
            SymbolValue::Plain(_) => {
                self.set_plain_value(target, newval);
                Ok(())
            }
            SymbolValue::Alias(_) => unreachable!(),
            SymbolValue::Localized(l) => {
                if let Some(per_buffer) = l.per_buffer
                    && !voide
                {
                    self.check_per_buffer_value(per_buffer.predicate, newval)?;
                }
                let buffer = if where_.is_nil() {
                    self.current_buffer_value()
                } else {
                    where_
                };
                if self.buffer_local_get(buffer, target).is_some()
                    || (l.local_if_set
                        && mode != SetMode::Unbind
                        && !buffer.is_nil()
                        && !self.let_shadows_global_binding_p(target))
                {
                    self.buffer_local_set(buffer, target, newval);
                } else if let SymbolValue::Localized(l) = &mut self.heap.symbol_mut(target).value {
                    l.default = newval;
                }
                Ok(())
            }
            SymbolValue::Forwarded(Forward::Int(_)) => {
                if voide {
                    let s = self.heap.symbol_mut(target);
                    s.value = SymbolValue::Plain(sym::UNBOUND);
                    return Ok(());
                }
                match newval.as_fixnum() {
                    Some(n) => {
                        self.set_forwarded_int(target, n);
                        Ok(())
                    }
                    None => Err(self.wrong_type(sym::INTEGERP, newval)),
                }
            }
            SymbolValue::Forwarded(Forward::Bool(_)) => {
                if let SymbolValue::Forwarded(Forward::Bool(b)) =
                    &mut self.heap.symbol_mut(target).value
                {
                    *b = !newval.is_nil();
                }
                Ok(())
            }
            SymbolValue::Forwarded(Forward::Kboard(slot)) => {
                self.set_kboard_slot(slot, newval);
                Ok(())
            }
        }
    }

    fn check_per_buffer_value(&mut self, predicate: Value, newval: Value) -> LispResult<()> {
        if newval.is_nil() || predicate.is_nil() {
            return Ok(());
        }
        let choice_key = self.intern("choice");
        let choice = self.get(predicate, choice_key);
        if !choice.is_nil() {
            if self.memq(newval, choice).is_nil() {
                let names = self.list_to_vec(choice)?;
                let mut text = String::from("One of ");
                for (i, name) in names.iter().enumerate() {
                    text.push_str(&self.symbol_name_string(*name));
                    text.push_str(if i + 1 == names.len() {
                        " should be specified"
                    } else if i + 2 == names.len() {
                        " or "
                    } else {
                        ", "
                    });
                }
                let message = self.string(&text);
                return Err(self.signal_list(sym::ERROR, &[message, newval]));
            }
            return Ok(());
        }
        let range_key = self.intern("range");
        let range = self.get(predicate, range_key);
        if range.is_cons() {
            let (min, max) = (self.xcar(range), self.xcdr(range));
            let ok = self.is_number(newval) && {
                let (lt1, eq1, _) = self.arithcompare(min, newval)?;
                let (lt2, eq2, _) = self.arithcompare(newval, max)?;
                (lt1 || eq1) && (lt2 || eq2)
            };
            if !ok {
                let lo = self.prin1_to_string(min);
                let hi = self.prin1_to_string(max);
                let message = self.string(&format!("Value should be from {lo} to {hi}"));
                return Err(self.signal_list(sym::ERROR, &[message, newval]));
            }
            return Ok(());
        }
        if self.functionp(predicate) && self.funcall(predicate, &[newval])?.is_nil() {
            return Err(self.wrong_type(predicate, newval));
        }
        Ok(())
    }

    fn let_shadows_global_binding_p(&self, symbol: Value) -> bool {
        self.specpdl.iter().any(|b| {
            matches!(*b, SpecBinding::Let { symbol: s, .. } | SpecBinding::LetDefault { symbol: s, .. } if s == symbol)
        })
    }

    pub fn notify_variable_watchers(
        &mut self,
        symbol: Value,
        newval: Value,
        operation: Value,
        where_: Value,
    ) -> LispResult<()> {
        let watchers = self.get(symbol, sym::WATCHERS);
        if watchers.is_nil() {
            return Ok(());
        }
        let count = self.specpdl.len();
        self.stack.push(watchers);
        let saved = self.heap.symbol_ref(symbol).trapped_write;
        self.heap.symbol_mut(symbol).trapped_write = TrappedWrite::Untrapped;
        self.record_unwind(restore_trapped, symbol);
        let mut tail = watchers;
        let mut result = Ok(());
        while tail.is_cons() {
            let watcher = self.xcar(tail);
            tail = self.xcdr(tail);
            let where_value = if where_.is_nil() { Value::NIL } else { where_ };
            if let Err(e) = self.funcall(watcher, &[symbol, newval, operation, where_value]) {
                result = Err(e);
                break;
            }
        }
        let _ = saved;
        self.stack.pop();
        let r = self.unbind_to(count);
        result?;
        r
    }

    pub fn default_value_unchecked(&self, symbol: Value) -> Value {
        let symbol = self.indirect_variable(symbol);
        match &self.heap.symbol_ref(symbol).value {
            SymbolValue::Plain(v) => *v,
            SymbolValue::Localized(l) => l.default,
            SymbolValue::Forwarded(Forward::Int(n)) => Value::fixnum(*n),
            SymbolValue::Forwarded(Forward::Bool(b)) => bool_value(*b),
            SymbolValue::Forwarded(Forward::Kboard(slot)) => self.kboard_slot(*slot),
            SymbolValue::Alias(_) => unreachable!(),
        }
    }

    pub fn default_value_raw(&mut self, symbol: Value) -> LispResult {
        let symbol = self.check_symbol(symbol)?;
        Ok(self.default_value_unchecked(symbol))
    }

    pub fn default_value(&mut self, symbol: Value) -> LispResult {
        let value = self.default_value_raw(symbol)?;
        if value == sym::UNBOUND {
            return Err(self.signal_list(sym::VOID_VARIABLE, &[symbol]));
        }
        Ok(value)
    }

    pub fn set_default(&mut self, symbol: Value, value: Value) -> LispResult<()> {
        self.set_default_internal(symbol, value, SetMode::Set)
    }

    pub fn set_default_internal(
        &mut self,
        symbol: Value,
        value: Value,
        mode: SetMode,
    ) -> LispResult<()> {
        let symbol = self.check_symbol(symbol)?;
        match self.heap.symbol_ref(symbol).trapped_write {
            TrappedWrite::NoWrite => {
                if self.keywordp(symbol) && value == self.find_symbol_value(symbol) {
                    return Ok(());
                }
                return Err(self.signal_list(sym::SETTING_CONSTANT, &[symbol]));
            }
            TrappedWrite::Trapped => {
                let operation = match mode {
                    SetMode::Bind => sym::LET,
                    SetMode::Unbind => sym::UNLET,
                    _ => sym::SET,
                };
                self.notify_variable_watchers(symbol, value, operation, Value::NIL)?;
            }
            TrappedWrite::Untrapped => {}
        }
        let target = self.indirect_variable(symbol);
        let is_localized = matches!(
            self.heap.symbol_ref(target).value,
            SymbolValue::Localized(_)
        );
        if is_localized {
            if let SymbolValue::Localized(l) = &mut self.heap.symbol_mut(target).value {
                l.default = value;
            }
            Ok(())
        } else {
            self.set_internal(target, value, Value::NIL, mode)
        }
    }

    pub fn keywordp(&self, v: Value) -> bool {
        if !v.is_symbol() {
            return false;
        }
        let s = self.heap.symbol_ref(v);
        let name = self.string_bytes(s.name);
        name.first() == Some(&b':') && s.interned == crate::object::Interned::InitialObarray
    }

    pub fn fset(&mut self, symbol: Value, definition: Value) -> LispResult {
        let symbol = self.check_symbol(symbol)?;
        if symbol.is_nil() && !definition.is_nil() {
            return Err(self.signal_list(sym::SETTING_CONSTANT, &[symbol]));
        }
        let function = self.heap.symbol_ref(symbol).function;
        let queue = self.plain_value(sym::AUTOLOAD_QUEUE);
        if !queue.is_nil() && !function.is_nil() {
            let pair = self.cons(symbol, function);
            let queue = self.cons(pair, queue);
            self.set_plain_value(sym::AUTOLOAD_QUEUE, queue);
        }
        if function.is_cons() && self.xcar(function) == sym::AUTOLOAD {
            let history = self.get(symbol, sym::FUNCTION_HISTORY);
            let history = self.cons(function, history);
            self.put(symbol, sym::FUNCTION_HISTORY, history);
        }
        if definition.is_symbol() && !definition.is_nil() {
            let mut s = definition;
            while s.is_symbol() && !s.is_nil() {
                if s == symbol {
                    return Err(self.signal_list(sym::CYCLIC_FUNCTION_INDIRECTION, &[symbol]));
                }
                s = self.heap.symbol_ref(s).function;
            }
        }
        self.heap.symbol_mut(symbol).function = definition;
        Ok(definition)
    }

    pub fn defalias(&mut self, symbol: Value, definition: Value, docstring: Value) -> LispResult {
        let symbol = self.check_symbol(symbol)?;
        let file = self.plain_value(sym::LOAD_FILE_NAME);
        let purify = self.plain_value(sym::PURIFY_FLAG);
        let _ = (file, purify);
        let hook = self.get(symbol, sym::DEFALIAS_FSET_FUNCTION);
        self.stack.push(definition);
        if !hook.is_nil() {
            self.funcall(hook, &[symbol, definition])?;
        } else {
            self.fset(symbol, definition)?;
        }
        self.stack.pop();
        if !docstring.is_nil() {
            self.put(symbol, sym::FUNCTION_DOCUMENTATION, docstring);
        }
        self.load_history_function(symbol)?;
        Ok(symbol)
    }

    fn load_history_function(&mut self, symbol: Value) -> LispResult<()> {
        let file = self.plain_value(sym::LOAD_FILE_NAME);
        if file.is_nil() || file == sym::UNBOUND {
            return Ok(());
        }
        let entry = self.cons(sym::DEFUN, symbol);
        let list = self.plain_value(sym::CURRENT_LOAD_LIST);
        let list = self.cons(entry, list);
        self.set_plain_value(sym::CURRENT_LOAD_LIST, list);
        Ok(())
    }

    pub fn make_localized(&mut self, symbol: Value, local_if_set: bool) -> LispResult<Value> {
        let target = self.indirect_variable(symbol);
        let current = self.heap.symbol_ref(target).value.clone();
        match current {
            SymbolValue::Plain(v) => {
                let default = if v == sym::UNBOUND && local_if_set {
                    Value::NIL
                } else {
                    v
                };
                self.heap.symbol_mut(target).value = SymbolValue::Localized(Box::new(Localized {
                    local_if_set,
                    default,
                    per_buffer: None,
                }));
            }
            SymbolValue::Localized(_) => {
                if local_if_set
                    && let SymbolValue::Localized(l) = &mut self.heap.symbol_mut(target).value
                {
                    l.local_if_set = true;
                }
            }
            SymbolValue::Forwarded(_) => {
                let v = self.find_symbol_value(target);
                self.heap.symbol_mut(target).value = SymbolValue::Localized(Box::new(Localized {
                    local_if_set,
                    default: v,
                    per_buffer: None,
                }));
            }
            SymbolValue::Alias(_) => unreachable!(),
        }
        Ok(target)
    }

    pub fn aref_value(&mut self, array: Value, idx: Value) -> LispResult {
        let i = self.check_fixnum(idx)?;
        if array.is_string() {
            let s = self.heap.string_ref(array);
            if i < 0 || i as usize >= s.chars {
                return Err(self.args_out_of_range(array, idx));
            }
            if !s.multibyte {
                return Ok(Value::fixnum(s.data[i as usize] as i64));
            }
            let offset = character::char_offset_to_byte(&s.data, i as usize);
            let (c, _) = character::string_char(&s.data[offset..]);
            return Ok(Value::fixnum(c as i64));
        }
        if !array.is_vectorlike() {
            return Err(self.wrong_type(sym::ARRAYP, array));
        }
        match self.heap.vectorlike_ref(array) {
            Vectorlike::Vector(items) | Vectorlike::Record(items) | Vectorlike::Closure(items) => {
                match usize::try_from(i).ok().and_then(|i| items.get(i)) {
                    Some(v) => Ok(*v),
                    None => Err(self.args_out_of_range(array, idx)),
                }
            }
            Vectorlike::BoolVector(b) => {
                match usize::try_from(i).ok().and_then(|i| b.bits.get(i)) {
                    Some(bit) => Ok(bool_value(*bit)),
                    None => Err(self.args_out_of_range(array, idx)),
                }
            }
            Vectorlike::CharTable(_) => {
                let c = self.check_character(idx)?;
                Ok(self.char_table_ref(array, c))
            }
            _ => Err(self.wrong_type(sym::ARRAYP, array)),
        }
    }

    pub fn aset_value(&mut self, array: Value, idx: Value, newelt: Value) -> LispResult {
        let i = self.check_fixnum(idx)?;
        if array.is_string() {
            let chars = self.heap.string_ref(array).chars;
            if i < 0 || i as usize >= chars {
                return Err(self.args_out_of_range(array, idx));
            }
            let c = self.check_character(newelt)?;
            let i = i as usize;
            if self.string_multibyte(array) {
                if c > 0x7f {
                    return Err(self.error("Attempt to store non-ASCII char into multibyte string"));
                }
                let offset = character::char_offset_to_byte(self.string_bytes(array), i);
                if self.string_bytes(array)[offset] > 0x7f {
                    return Err(self.error("Attempt to replace non-ASCII char in multibyte string"));
                }
                self.heap.string_mut(array).data[offset] = c as u8;
            } else {
                if c > 0xff {
                    return Err(self.error("Attempt to store non-byte value into unibyte string"));
                }
                self.heap.string_mut(array).data[i] = c as u8;
            }
            return Ok(newelt);
        }
        if !array.is_vectorlike() {
            return Err(self.wrong_type(sym::ARRAYP, array));
        }
        let is_char_table = self.is_char_table(array);
        if is_char_table {
            let c = self.check_character(idx)?;
            self.char_table_set(array, c, newelt);
            return Ok(newelt);
        }
        match self.heap.vectorlike_mut(array) {
            Vectorlike::Vector(items) | Vectorlike::Record(items) | Vectorlike::Closure(items) => {
                match usize::try_from(i).ok().and_then(|i| items.get_mut(i)) {
                    Some(slot) => {
                        *slot = newelt;
                        Ok(newelt)
                    }
                    None => Err(self.args_out_of_range(array, idx)),
                }
            }
            Vectorlike::BoolVector(b) => {
                match usize::try_from(i).ok().and_then(|i| b.bits.get_mut(i)) {
                    Some(bit) => {
                        *bit = !newelt.is_nil();
                        Ok(newelt)
                    }
                    None => Err(self.args_out_of_range(array, idx)),
                }
            }
            _ => Err(self.wrong_type(sym::ARRAYP, array)),
        }
    }
}

fn restore_trapped(lisp: &mut Lisp, symbol: Value) -> LispResult<()> {
    if !lisp.get(symbol, sym::WATCHERS).is_nil() {
        lisp.heap.symbol_mut(symbol).trapped_write = TrappedWrite::Trapped;
    }
    Ok(())
}

pub fn compare_nums(a: &Num, b: &Num) -> (bool, bool, bool) {
    use std::cmp::Ordering;
    let ord: Option<Ordering> = match (a, b) {
        (Num::Fix(x), Num::Fix(y)) => Some(x.cmp(y)),
        (Num::Float(x), Num::Float(y)) => x.partial_cmp(y),
        (Num::Big(x), Num::Big(y)) => Some(x.cmp(y)),
        (Num::Fix(x), Num::Big(y)) => Some(BigInt::from(*x).cmp(y)),
        (Num::Big(x), Num::Fix(y)) => Some(x.cmp(&BigInt::from(*y))),
        (Num::Float(f), other) => float_vs_integer(*f, other),
        (other, Num::Float(f)) => float_vs_integer(*f, other).map(Ordering::reverse),
    };
    match ord {
        Some(Ordering::Less) => (true, false, false),
        Some(Ordering::Equal) => (false, true, false),
        Some(Ordering::Greater) => (false, false, true),
        None => (false, false, false),
    }
}

fn float_vs_integer(f: f64, i: &Num) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;
    if f.is_nan() {
        return None;
    }
    if f.is_infinite() {
        return Some(if f > 0.0 {
            Ordering::Greater
        } else {
            Ordering::Less
        });
    }
    let int = i.to_bigint()?;
    let trunc = f.trunc();
    let fint = BigInt::from(trunc as i128);
    let fint = if trunc.abs() >= 1.0e38 {
        num_bigint_from_f64(trunc)
    } else {
        fint
    };
    match fint.cmp(&int) {
        Ordering::Equal => {
            let frac = f - trunc;
            Some(if frac > 0.0 {
                Ordering::Greater
            } else if frac < 0.0 {
                Ordering::Less
            } else {
                Ordering::Equal
            })
        }
        other => Some(other),
    }
}

pub fn num_bigint_from_f64(f: f64) -> BigInt {
    use num_traits::FromPrimitive;
    BigInt::from_f64(f).unwrap_or_default()
}

fn f_false(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_eq(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.eq(args[0], args[1])))
}

fn f_null(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(args[0].is_nil()))
}

pub fn cl_type_of(lisp: &Lisp, object: Value) -> Value {
    match object.tag() {
        Tag::Fixnum => sym::FIXNUM,
        Tag::Symbol => {
            if object.is_nil() {
                sym::NULL
            } else if object == sym::T {
                sym::BOOLEAN
            } else {
                sym::SYMBOL
            }
        }
        Tag::String => sym::STRING,
        Tag::Cons => sym::CONS,
        Tag::Float => sym::FLOAT,
        Tag::Vectorlike => match lisp.heap.vectorlike_ref(object) {
            Vectorlike::Vector(_) => sym::VECTOR,
            Vectorlike::Bignum(_) => sym::BIGNUM,
            Vectorlike::SymbolWithPos { .. } => sym::SYMBOL_WITH_POS,
            Vectorlike::Finalizer { .. } => sym::FINALIZER,
            Vectorlike::Subr(s) => {
                if s.is_special_form() {
                    sym::SPECIAL_FORM
                } else {
                    sym::PRIMITIVE_FUNCTION
                }
            }
            Vectorlike::Closure(slots) => {
                if slots.get(1).is_some_and(|c| c.is_cons()) {
                    sym::INTERPRETED_FUNCTION
                } else {
                    sym::BYTE_CODE_FUNCTION
                }
            }
            Vectorlike::CharTable(_) => sym::CHAR_TABLE,
            Vectorlike::SubCharTable(_) => sym::SUB_CHAR_TABLE,
            Vectorlike::BoolVector(_) => sym::BOOL_VECTOR,
            Vectorlike::HashTable(_) => sym::HASH_TABLE,
            Vectorlike::Obarray(_) => sym::OBARRAY,
            Vectorlike::Buffer(_) => sym::BUFFER,
            Vectorlike::Marker(_) => sym::MARKER,
            Vectorlike::Overlay(_) => sym::OVERLAY,
            Vectorlike::Terminal(_) => sym::TERMINAL,
            Vectorlike::Frame(_) => sym::FRAME,
            Vectorlike::Window(_) => sym::WINDOW,
            Vectorlike::WindowConfiguration(_) => sym::WINDOW_CONFIGURATION,
            Vectorlike::Font(f) => match f.kind {
                crate::font::FontKind::Spec => sym::FONT_SPEC,
                crate::font::FontKind::Entity => sym::FONT_ENTITY,
                crate::font::FontKind::Object => sym::FONT_OBJECT,
            },
            Vectorlike::Record(slots) => {
                let t = slots.first().copied().unwrap_or(Value::NIL);
                if t.is_vectorlike()
                    && let Vectorlike::Record(inner) = lisp.heap.vectorlike_ref(t)
                    && inner.len() > 1
                {
                    return inner[1];
                }
                t
            }
        },
    }
}

fn f_cl_type_of(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(cl_type_of(lisp, args[0]))
}

fn f_type_of(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let object = args[0];
    if object.is_symbol() {
        return Ok(sym::SYMBOL);
    }
    if lisp.is_integer(object) {
        return Ok(sym::INTEGER);
    }
    if lisp.subr(object).is_some() {
        return Ok(sym::SUBR);
    }
    Ok(cl_type_of(lisp, object))
}

fn f_consp(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(args[0].is_cons()))
}

fn f_atom(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(!args[0].is_cons()))
}

fn f_listp(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(args[0].is_cons() || args[0].is_nil()))
}

fn f_nlistp(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(!(args[0].is_cons() || args[0].is_nil())))
}

fn f_bare_symbol_p(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(args[0].is_symbol()))
}

impl Lisp {
    pub fn is_symbol_with_pos(&self, v: Value) -> bool {
        v.is_vectorlike()
            && matches!(
                self.heap.vectorlike_ref(v),
                Vectorlike::SymbolWithPos { .. }
            )
    }

    pub fn symbolp(&self, v: Value) -> bool {
        v.is_symbol()
            || (self.is_symbol_with_pos(v) && self.var_true(sym::SYMBOLS_WITH_POS_ENABLED))
    }
}

fn f_symbol_with_pos_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_symbol_with_pos(args[0])))
}

fn f_symbolp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.symbolp(args[0])))
}

fn f_keywordp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = lisp.maybe_remove_pos_from_symbol(args[0]);
    Ok(bool_value(lisp.keywordp(v)))
}

fn f_vectorp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_vector(args[0])))
}

fn f_recordp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_record(args[0])))
}

fn f_stringp(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(args[0].is_string()))
}

fn f_multibyte_string_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        args[0].is_string() && lisp.string_multibyte(args[0]),
    ))
}

fn f_char_table_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_char_table(args[0])))
}

fn f_bool_vector_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_bool_vector(args[0])))
}

fn f_vector_or_char_table_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        lisp.is_vector(args[0]) || lisp.is_char_table(args[0]),
    ))
}

fn closure_code(lisp: &Lisp, object: Value) -> Option<Value> {
    if !lisp.is_closure(object) {
        return None;
    }
    match lisp.heap.vectorlike_ref(object) {
        Vectorlike::Closure(items) => items.get(1).copied(),
        _ => None,
    }
}

fn f_closurep(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_closure(args[0])))
}

fn f_byte_code_function_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        closure_code(lisp, args[0]).is_some_and(|c| c.is_string()),
    ))
}

fn f_interpreted_function_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        closure_code(lisp, args[0]).is_some_and(|c| c.is_cons()),
    ))
}

impl Lisp {
    pub fn arrayp(&self, v: Value) -> bool {
        v.is_string()
            || (v.is_vectorlike()
                && matches!(
                    self.heap.vectorlike_ref(v),
                    Vectorlike::Vector(_) | Vectorlike::BoolVector(_) | Vectorlike::CharTable(_)
                ))
    }
}

fn f_arrayp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.arrayp(args[0])))
}

fn f_sequencep(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = args[0];
    Ok(bool_value(v.is_cons() || v.is_nil() || lisp.arrayp(v)))
}

fn f_bufferp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_buffer(args[0])))
}

fn f_markerp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.marker_position_value(args[0]).is_some()))
}

fn f_subrp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.subr(args[0]).is_some()))
}

fn f_char_or_string_p(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = args[0];
    Ok(bool_value(
        v.is_string()
            || v.as_fixnum()
                .is_some_and(|n| (0..=character::MAX_CHAR as i64).contains(&n)),
    ))
}

fn f_integerp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_integer(args[0])))
}

fn f_integer_or_marker_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        lisp.is_integer(args[0]) || lisp.marker_position_value(args[0]).is_some(),
    ))
}

fn f_natnump(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = args[0];
    Ok(bool_value(match v.as_fixnum() {
        Some(n) => n >= 0,
        None => lisp.bignum_ref(v).is_some_and(|b| b.is_positive()),
    }))
}

fn f_numberp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_number(args[0])))
}

fn f_number_or_marker_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        lisp.is_number(args[0]) || lisp.marker_position_value(args[0]).is_some(),
    ))
}

fn f_floatp(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(args[0].is_float()))
}

fn f_car(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.car(args[0])
}

fn f_car_safe(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.car_safe(args[0]))
}

fn f_cdr(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.cdr(args[0])
}

fn f_cdr_safe(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.cdr_safe(args[0]))
}

fn f_setcar(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let cell = lisp.check_cons(args[0])?;
    lisp.heap.set_car(cell, args[1]);
    Ok(args[1])
}

fn f_setcdr(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let cell = lisp.check_cons(args[0])?;
    lisp.heap.set_cdr(cell, args[1]);
    Ok(args[1])
}

fn f_boundp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.maybe_remove_pos_from_symbol(args[0]);
    let symbol = lisp.check_symbol(symbol)?;
    Ok(bool_value(lisp.find_symbol_value(symbol) != sym::UNBOUND))
}

pub(crate) fn f_fboundp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.maybe_remove_pos_from_symbol(args[0]);
    let symbol = lisp.check_symbol(symbol)?;
    Ok(bool_value(!lisp.heap.symbol_ref(symbol).function.is_nil()))
}

fn f_makunbound(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.check_symbol(args[0])?;
    if lisp.heap.symbol_ref(symbol).trapped_write == TrappedWrite::NoWrite {
        return Err(lisp.signal_list(sym::SETTING_CONSTANT, &[symbol]));
    }
    lisp.set_internal(symbol, sym::UNBOUND, Value::NIL, SetMode::Makunbound)?;
    Ok(symbol)
}

fn f_fmakunbound(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.check_symbol(args[0])?;
    if symbol.is_nil() || symbol == sym::T {
        return Err(lisp.signal_list(sym::SETTING_CONSTANT, &[symbol]));
    }
    lisp.heap.symbol_mut(symbol).function = Value::NIL;
    Ok(symbol)
}

fn f_symbol_function(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.maybe_remove_pos_from_symbol(args[0]);
    let symbol = lisp.check_symbol(symbol)?;
    Ok(lisp.heap.symbol_ref(symbol).function)
}

fn f_symbol_plist(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.maybe_remove_pos_from_symbol(args[0]);
    let symbol = lisp.check_symbol(symbol)?;
    Ok(lisp.heap.symbol_ref(symbol).plist)
}

fn f_symbol_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.maybe_remove_pos_from_symbol(args[0]);
    let symbol = lisp.check_symbol(symbol)?;
    Ok(lisp.symbol_name(symbol))
}

fn f_bare_symbol(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = args[0];
    if v.is_symbol() {
        return Ok(v);
    }
    if v.is_vectorlike()
        && let Vectorlike::SymbolWithPos { symbol, .. } = lisp.heap.vectorlike_ref(v)
    {
        return Ok(*symbol);
    }
    Err(lisp.wrong_type(sym::SYMBOLP, v))
}

fn f_symbol_with_pos_pos(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = args[0];
    if v.is_vectorlike()
        && let Vectorlike::SymbolWithPos { position, .. } = lisp.heap.vectorlike_ref(v)
    {
        return Ok(*position);
    }
    Err(lisp.wrong_type(sym::SYMBOL_WITH_POS_P, v))
}

fn f_remove_pos_from_symbol(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = args[0];
    if v.is_vectorlike()
        && let Vectorlike::SymbolWithPos { symbol, .. } = lisp.heap.vectorlike_ref(v)
    {
        return Ok(*symbol);
    }
    Ok(v)
}

fn f_position_symbol(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, pos) = (args[0], args[1]);
    let bare = if symbol.is_symbol() {
        symbol
    } else if let Vectorlike::SymbolWithPos { symbol: s, .. } = lisp.heap.vectorlike_ref(symbol) {
        *s
    } else {
        return Err(lisp.wrong_type(sym::SYMBOLP, symbol));
    };
    let position = if pos.is_fixnum() {
        pos
    } else if lisp.is_symbol_with_pos(pos) {
        match lisp.heap.vectorlike_ref(pos) {
            Vectorlike::SymbolWithPos { position, .. } => *position,
            _ => unreachable!(),
        }
    } else {
        return Err(lisp.wrong_type(sym::FIXNUM_OR_SYMBOL_WITH_POS_P, pos));
    };
    Ok(lisp.heap.vectorlike(Vectorlike::SymbolWithPos {
        symbol: bare,
        position,
    }))
}

fn f_fset(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.fset(args[0], args[1])
}

fn f_defalias(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.defalias(args[0], args[1], args[2])
}

fn f_setplist(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.check_symbol(args[0])?;
    lisp.heap.symbol_mut(symbol).plist = args[1];
    Ok(args[1])
}

fn f_subr_arity(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let Some(subr) = lisp.subr(args[0]) else {
        return Err(lisp.wrong_type(sym::SUBRP, args[0]));
    };
    let min = Value::fixnum(subr.min_args as i64);
    let max = match subr.kind {
        SubrKind::Fixed(m, _) => Value::fixnum(m as i64),
        SubrKind::Many(_) => sym::MANY,
        SubrKind::Special(_) => sym::UNEVALLED,
    };
    Ok(lisp.cons(min, max))
}

fn f_subr_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let Some(subr) = lisp.subr(args[0]) else {
        return Err(lisp.wrong_type(sym::SUBRP, args[0]));
    };
    Ok(lisp.string(subr.name))
}

fn f_interactive_form(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let cmd = args[0];
    let mut fun = lisp.indirect_function(cmd);
    let genfun = false;
    if fun.is_symbol() && !fun.is_nil() {
        let spec = lisp.get(fun, sym::INTERACTIVE_FORM);
        if !spec.is_nil() {
            return Ok(spec);
        }
    }
    let _ = genfun;
    if fun.is_symbol() {
        let spec = lisp.get(cmd, sym::INTERACTIVE_FORM);
        if !spec.is_nil() {
            return Ok(spec);
        }
    }
    if let Some(subr) = lisp.subr(fun) {
        if let Some(spec) = subr.interactive {
            let s = lisp.string(spec);
            return Ok(lisp.list(&[sym::INTERACTIVE, s]));
        }
        return Ok(Value::NIL);
    }
    if fun.is_vectorlike() && lisp.is_closure(fun) {
        let slots = lisp.vector_items(fun);
        if slots.len() > 5 {
            let form = slots[5];
            if lisp.is_vector(form) {
                let items = lisp.vector_items(form).to_vec();
                let spec = items.first().copied().unwrap_or(Value::NIL);
                let modes = items.get(1).copied().unwrap_or(Value::NIL);
                let tail = lisp.list(&[spec]);
                let _ = modes;
                return Ok(lisp.cons(sym::INTERACTIVE, tail));
            }
            return Ok(lisp.list(&[sym::INTERACTIVE, form]));
        }
        return Ok(Value::NIL);
    }
    if fun.is_cons() {
        let funcar = lisp.xcar(fun);
        if funcar == sym::AUTOLOAD {
            return Ok(Value::NIL);
        }
        if funcar == sym::LAMBDA {
            fun = lisp.xcdr(fun);
            let mut body = lisp.cdr_safe(fun);
            if body.is_cons() && lisp.xcar(body).is_string() {
                body = lisp.xcdr(body);
            }
            while body.is_cons() {
                let form = lisp.xcar(body);
                if form.is_cons() && lisp.xcar(form) == sym::INTERACTIVE {
                    return Ok(form);
                }
                if !(form.is_cons() && lisp.xcar(form) == sym::DECLARE) {
                    break;
                }
                body = lisp.xcdr(body);
            }
        }
    }
    Ok(Value::NIL)
}

fn f_command_modes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let command = args[0];
    let fun = lisp.indirect_function(command);
    if command.is_symbol() {
        let modes = lisp.get(command, sym::COMMAND_MODES);
        if !modes.is_nil() {
            return Ok(modes);
        }
    }
    if fun.is_vectorlike() && lisp.is_closure(fun) {
        let slots = lisp.vector_items(fun);
        if slots.len() > 5 {
            let form = slots[5];
            if lisp.is_vector(form) {
                return Ok(lisp
                    .vector_items(form)
                    .get(1)
                    .copied()
                    .unwrap_or(Value::NIL));
            }
        }
    }
    Ok(Value::NIL)
}

fn f_indirect_variable(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args[0].is_symbol() {
        return Ok(lisp.indirect_variable(args[0]));
    }
    Ok(args[0])
}

fn f_symbol_value(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.check_symbol(args[0])?;
    lisp.symbol_value(symbol)
}

fn f_set(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.set(args[0], args[1])?;
    Ok(args[1])
}

fn f_add_variable_watcher(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, watch_function) = (args[0], args[1]);
    let symbol = lisp.check_symbol(symbol)?;
    let symbol = lisp.indirect_variable(symbol);
    if lisp.heap.symbol_ref(symbol).trapped_write == TrappedWrite::NoWrite {
        return Err(lisp.signal_list(sym::TRAPPING_CONSTANT, &[symbol]));
    }
    lisp.heap.symbol_mut(symbol).trapped_write = TrappedWrite::Trapped;
    let watchers = lisp.get(symbol, sym::WATCHERS);
    if lisp.member(watch_function, watchers)?.is_nil() {
        let watchers = lisp.cons(watch_function, watchers);
        lisp.put(symbol, sym::WATCHERS, watchers);
    }
    Ok(Value::NIL)
}

fn f_remove_variable_watcher(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, watch_function) = (args[0], args[1]);
    let symbol = lisp.check_symbol(symbol)?;
    let symbol = lisp.indirect_variable(symbol);
    let watchers = lisp.get(symbol, sym::WATCHERS);
    let watchers = lisp.delete(watch_function, watchers)?;
    if watchers.is_nil() {
        let s = lisp.heap.symbol_mut(symbol);
        if s.trapped_write == TrappedWrite::Trapped {
            s.trapped_write = TrappedWrite::Untrapped;
        }
    }
    lisp.put(symbol, sym::WATCHERS, watchers);
    Ok(Value::NIL)
}

fn f_get_variable_watchers(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.check_symbol(args[0])?;
    let symbol = lisp.indirect_variable(symbol);
    if lisp.heap.symbol_ref(symbol).trapped_write == TrappedWrite::Trapped {
        Ok(lisp.get(symbol, sym::WATCHERS))
    } else {
        Ok(Value::NIL)
    }
}

fn f_default_boundp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let value = lisp.default_value_raw(args[0])?;
    Ok(bool_value(value != sym::UNBOUND))
}

fn f_default_value(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.default_value(args[0])
}

fn f_set_default(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.set_default(args[0], args[1])?;
    Ok(args[1])
}

fn f_make_variable_buffer_local(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = lisp.check_symbol(args[0])?;
    let target = lisp.indirect_variable(variable);
    if lisp.heap.symbol_ref(target).trapped_write == TrappedWrite::NoWrite {
        let name = lisp.symbol_name_string(variable);
        return Err(lisp.error(&format!("Symbol {name} may not be buffer-local")));
    }
    lisp.make_localized(target, true)?;
    lisp.heap.symbol_mut(target).special = true;
    Ok(variable)
}

fn f_make_local_variable(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = lisp.check_symbol(args[0])?;
    let target = lisp.indirect_variable(variable);
    if lisp.heap.symbol_ref(target).trapped_write == TrappedWrite::NoWrite {
        let name = lisp.symbol_name_string(variable);
        return Err(lisp.error(&format!("Symbol {name} may not be buffer-local")));
    }
    let buffer = lisp.current_buffer_value();
    let value = lisp.find_symbol_value(target);
    lisp.make_localized(target, false)?;
    if !buffer.is_nil() && lisp.buffer_local_get(buffer, target).is_none() {
        let initial = if value == sym::UNBOUND {
            lisp.default_value_unchecked(target)
        } else {
            value
        };
        lisp.buffer_local_set(buffer, target, initial);
    }
    Ok(variable)
}

fn f_kill_local_variable(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = lisp.check_symbol(args[0])?;
    let target = lisp.indirect_variable(variable);
    let buffer = lisp.current_buffer_value();
    if lisp.heap.symbol_ref(target).trapped_write == TrappedWrite::Trapped {
        lisp.notify_variable_watchers(variable, Value::NIL, sym::MAKUNBOUND, buffer)?;
    }
    lisp.buffer_local_remove(buffer, target);
    Ok(variable)
}

fn f_local_variable_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = lisp.check_symbol(args[0])?;
    let target = lisp.indirect_variable(variable);
    let buffer = if args[1].is_nil() {
        lisp.current_buffer_value()
    } else {
        args[1]
    };
    Ok(bool_value(lisp.local_variable_in(target, buffer)))
}

fn f_local_variable_if_set_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = lisp.check_symbol(args[0])?;
    let target = lisp.indirect_variable(variable);
    match &lisp.heap.symbol_ref(target).value {
        SymbolValue::Localized(l) if l.local_if_set => Ok(sym::T),
        _ => f_local_variable_p(lisp, args),
    }
}

fn f_variable_binding_locus(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = lisp.check_symbol(args[0])?;
    let target = lisp.indirect_variable(variable);
    let buffer = lisp.current_buffer_value();
    if lisp.local_variable_in(target, buffer) {
        return Ok(buffer);
    }
    Ok(Value::NIL)
}

fn f_indirect_function(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.indirect_function(args[0]))
}

fn f_aref(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.aref_value(args[0], args[1])
}

fn f_aset(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.aset_value(args[0], args[1], args[2])
}

fn compare_chain(
    lisp: &mut Lisp,
    args: &[Value],
    test: fn((bool, bool, bool)) -> bool,
) -> LispResult {
    if args.len() == 1 {
        lisp.number(args[0])?;
        return Ok(sym::T);
    }
    for pair in args.windows(2) {
        let r = lisp.arithcompare(pair[0], pair[1])?;
        if !test(r) {
            for &rest in args {
                lisp.number(rest)?;
            }
            return Ok(Value::NIL);
        }
    }
    Ok(sym::T)
}

fn f_eqlsign(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    compare_chain(lisp, args, |(_, eq, _)| eq)
}

fn f_lss(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    compare_chain(lisp, args, |(lt, _, _)| lt)
}

fn f_gtr(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    compare_chain(lisp, args, |(_, _, gt)| gt)
}

fn f_leq(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    compare_chain(lisp, args, |(lt, eq, _)| lt || eq)
}

fn f_geq(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    compare_chain(lisp, args, |(_, eq, gt)| gt || eq)
}

fn f_neq(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (_, eq, _) = lisp.arithcompare(args[0], args[1])?;
    Ok(bool_value(!eq))
}

pub(crate) fn f_number_to_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = args[0];
    if n.is_float() {
        let f = lisp.heap.float_value(n);
        let s = lisp.float_to_string(f);
        return Ok(lisp.string(&s));
    }
    if let Some(i) = n.as_fixnum() {
        return Ok(lisp.string(&i.to_string()));
    }
    if let Some(b) = lisp.bignum_ref(n) {
        let s = b.to_string();
        return Ok(lisp.string(&s));
    }
    Err(lisp.wrong_type(sym::NUMBERP, n))
}

pub(crate) fn f_string_to_number(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let string = lisp.check_string(args[0])?;
    let base = if args[1].is_nil() {
        10
    } else {
        let b = lisp.check_fixnum(args[1])?;
        if !(2..=16).contains(&b) {
            return Err(lisp.signal_list(sym::ARGS_OUT_OF_RANGE, &[args[1]]));
        }
        b as u32
    };
    let bytes = lisp.string_bytes(string).to_vec();
    let start = bytes
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(bytes.len());
    match crate::lread::string_to_number(lisp, &bytes[start..], base) {
        Some((value, _)) => Ok(value),
        None => Ok(Value::fixnum(0)),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ArithOp {
    Add,
    Sub,
    Mult,
    Div,
    Logand,
    Logior,
    Logxor,
}

fn arith_driver(lisp: &mut Lisp, code: ArithOp, args: &[Value], first: Num) -> LispResult {
    let float_op = matches!(
        code,
        ArithOp::Add | ArithOp::Sub | ArithOp::Mult | ArithOp::Div
    );
    let mut accum = first;
    for &arg in &args[1..] {
        let next = lisp.number(arg)?;
        accum = match (accum, next) {
            (Num::Float(a), n) => {
                if !float_op {
                    return Err(lisp.wrong_type(sym::INTEGER_OR_MARKER_P, arg));
                }
                Num::Float(float_op_apply(code, a, n.to_f64()))
            }
            (a, Num::Float(f)) => {
                if !float_op {
                    return Err(lisp.wrong_type(sym::INTEGER_OR_MARKER_P, arg));
                }
                Num::Float(float_op_apply(code, a.to_f64(), f))
            }
            (Num::Fix(a), Num::Fix(b)) => {
                let fast = match code {
                    ArithOp::Add => a.checked_add(b),
                    ArithOp::Sub => a.checked_sub(b),
                    ArithOp::Mult => a.checked_mul(b),
                    ArithOp::Div => {
                        if b == 0 {
                            return Err(lisp.signal(sym::ARITH_ERROR, Value::NIL));
                        }
                        a.checked_div(b)
                    }
                    ArithOp::Logand => Some(a & b),
                    ArithOp::Logior => Some(a | b),
                    ArithOp::Logxor => Some(a ^ b),
                };
                match fast {
                    Some(r) => Num::Fix(r),
                    None => big_op(lisp, code, BigInt::from(a), BigInt::from(b))?,
                }
            }
            (a, b) => {
                let a = a.to_bigint().expect("integer");
                let b = b.to_bigint().expect("integer");
                big_op(lisp, code, a, b)?
            }
        };
    }
    Ok(lisp.make_num(normalize(accum)))
}

fn normalize(n: Num) -> Num {
    match n {
        Num::Big(b) => match b.to_i64() {
            Some(i) if Value::try_fixnum(i).is_some() => Num::Fix(i),
            _ => Num::Big(b),
        },
        other => other,
    }
}

fn big_op(lisp: &mut Lisp, code: ArithOp, a: BigInt, b: BigInt) -> LispResult<Num> {
    Ok(Num::Big(match code {
        ArithOp::Add => a + b,
        ArithOp::Sub => a - b,
        ArithOp::Mult => {
            let bits = a.bits() + b.bits();
            if bits > lisp.int_var(sym::INTEGER_WIDTH).max(0) as u64 + 1 {
                return Err(lisp.signal(sym::OVERFLOW_ERROR, Value::NIL));
            }
            a * b
        }
        ArithOp::Div => {
            if b.is_zero() {
                return Err(lisp.signal(sym::ARITH_ERROR, Value::NIL));
            }
            a / b
        }
        ArithOp::Logand => a & b,
        ArithOp::Logior => a | b,
        ArithOp::Logxor => a ^ b,
    }))
}

fn float_op_apply(code: ArithOp, a: f64, b: f64) -> f64 {
    match code {
        ArithOp::Add => a + b,
        ArithOp::Sub => a - b,
        ArithOp::Mult => a * b,
        ArithOp::Div => a / b,
        _ => unreachable!(),
    }
}

fn f_plus(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args.is_empty() {
        return Ok(Value::fixnum(0));
    }
    let a = lisp.number(args[0])?;
    if args.len() == 1 {
        return Ok(lisp.make_num(a));
    }
    arith_driver(lisp, ArithOp::Add, args, a)
}

fn f_minus(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args.is_empty() {
        return Ok(Value::fixnum(0));
    }
    let a = lisp.number(args[0])?;
    if args.len() == 1 {
        let negated = match a {
            Num::Fix(n) => match n.checked_neg() {
                Some(m) => Num::Fix(m),
                None => Num::Big(-BigInt::from(n)),
            },
            Num::Float(f) => Num::Float(-f),
            Num::Big(b) => Num::Big(-b),
        };
        return Ok(lisp.make_num(normalize(negated)));
    }
    arith_driver(lisp, ArithOp::Sub, args, a)
}

fn f_times(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args.is_empty() {
        return Ok(Value::fixnum(1));
    }
    let a = lisp.number(args[0])?;
    if args.len() == 1 {
        return Ok(lisp.make_num(a));
    }
    arith_driver(lisp, ArithOp::Mult, args, a)
}

fn f_quo(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.number(args[0])?;
    if args.len() == 1 {
        return match a {
            Num::Fix(0) => Err(lisp.signal(sym::ARITH_ERROR, Value::NIL)),
            Num::Fix(n) => Ok(Value::fixnum(1 / n)),
            Num::Float(f) => Ok(lisp.float(1.0 / f)),
            Num::Big(_) => Ok(Value::fixnum(0)),
        };
    }
    if args[2..].iter().any(|v| v.is_float()) {
        let mut accum = a.to_f64();
        for &arg in &args[1..] {
            let next = lisp.number(arg)?.to_f64();
            accum /= next;
        }
        return Ok(lisp.float(accum));
    }
    arith_driver(lisp, ArithOp::Div, args, a)
}

fn integer_remainder(lisp: &mut Lisp, num: Num, den: Num, modulo: bool) -> LispResult {
    if let (Num::Fix(n), Num::Fix(d)) = (&num, &den) {
        let (n, d) = (*n, *d);
        if d == 0 {
            return Err(lisp.signal(sym::ARITH_ERROR, Value::NIL));
        }
        let mut r = n.wrapping_rem(d);
        if modulo && (if d < 0 { r > 0 } else { r < 0 }) {
            r += d;
        }
        return Ok(Value::fixnum(r));
    }
    let n = num.to_bigint().expect("integer");
    let d = den.to_bigint().expect("integer");
    if d.is_zero() {
        return Err(lisp.signal(sym::ARITH_ERROR, Value::NIL));
    }
    let mut r = &n % &d;
    if modulo
        && (if d.is_negative() {
            r.is_positive()
        } else {
            r.is_negative()
        })
    {
        r += &d;
    }
    Ok(lisp.bignum(r))
}

fn f_rem(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let x = lisp.integer_num(args[0])?;
    let y = lisp.integer_num(args[1])?;
    integer_remainder(lisp, x, y, false)
}

fn f_mod(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let x = lisp.number(args[0])?;
    let y = lisp.number(args[1])?;
    if matches!(x, Num::Float(_)) || matches!(y, Num::Float(_)) {
        let f1 = x.to_f64();
        let f2 = y.to_f64();
        let mut r = f1 % f2;
        if f2 != 0.0 && (if f2 < 0.0 { r > 0.0 } else { r < 0.0 }) {
            r += f2;
        }
        return Ok(lisp.float(r));
    }
    integer_remainder(lisp, x, y, true)
}

fn minmax(lisp: &mut Lisp, args: &[Value], want_greater: bool) -> LispResult {
    let mut accum_value = lisp.coerce_marker(args[0]);
    let mut accum = lisp.number(accum_value)?;
    for &arg in &args[1..] {
        let value = lisp.coerce_marker(arg);
        let n = lisp.number(value)?;
        let (lt, _, gt) = compare_nums(&n, &accum);
        if (want_greater && gt) || (!want_greater && lt) {
            accum = n;
            accum_value = value;
        } else if let Num::Float(f) = n
            && f.is_nan()
        {
            return Ok(value);
        }
    }
    let _ = accum;
    Ok(accum_value)
}

fn f_max(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    minmax(lisp, args, true)
}

fn f_min(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    minmax(lisp, args, false)
}

fn logical(lisp: &mut Lisp, args: &[Value], code: ArithOp, empty: i64) -> LispResult {
    if args.is_empty() {
        return Ok(Value::fixnum(empty));
    }
    let a = lisp.integer_num(args[0])?;
    if args.len() == 1 {
        return Ok(lisp.make_num(a));
    }
    arith_driver(lisp, code, args, a)
}

fn f_logand(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    logical(lisp, args, ArithOp::Logand, -1)
}

fn f_logior(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    logical(lisp, args, ArithOp::Logior, 0)
}

fn f_logxor(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    logical(lisp, args, ArithOp::Logxor, 0)
}

fn f_logcount(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let value = lisp.check_integer(args[0])?;
    let count = match value {
        Num::Fix(n) => {
            let v = if n < 0 { !n } else { n };
            v.count_ones() as i64
        }
        Num::Big(b) => {
            let v = if b.is_negative() { !b } else { b };
            v.to_biguint().map_or(0, |u| u.count_ones() as i64)
        }
        Num::Float(_) => unreachable!(),
    };
    Ok(Value::fixnum(count))
}

fn f_ash(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let value = lisp.check_integer(args[0])?;
    let count = lisp.check_integer(args[1])?;
    let c = match count {
        Num::Fix(c) => c,
        Num::Big(b) => {
            if args[0] == Value::fixnum(0) {
                return Ok(args[0]);
            }
            if b.is_negative() {
                let negative = match &value {
                    Num::Fix(v) => *v < 0,
                    Num::Big(v) => v.is_negative(),
                    Num::Float(_) => false,
                };
                return Ok(Value::fixnum(if negative { -1 } else { 0 }));
            }
            return Err(lisp.signal(sym::OVERFLOW_ERROR, Value::NIL));
        }
        Num::Float(_) => unreachable!(),
    };
    if c == 0 {
        return Ok(args[0]);
    }
    let v = value.to_bigint().expect("integer");
    let result = if c < 0 {
        let shift = (-c).min(u32::MAX as i64) as u32;
        v.div_floor(&(BigInt::from(1) << shift))
    } else {
        let width = lisp.int_var(sym::INTEGER_WIDTH).max(0) as u64;
        if v.bits() + c as u64 > width + 1 && !v.is_zero() {
            return Err(lisp.signal(sym::OVERFLOW_ERROR, Value::NIL));
        }
        v << (c as u32)
    };
    Ok(lisp.bignum(result))
}

pub(crate) fn f_add1(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = lisp.number(args[0])?;
    let r = match n {
        Num::Fix(i) => match i.checked_add(1) {
            Some(r) => Num::Fix(r),
            None => Num::Big(BigInt::from(i) + 1),
        },
        Num::Float(f) => Num::Float(f + 1.0),
        Num::Big(b) => Num::Big(b + 1),
    };
    Ok(lisp.make_num(normalize(r)))
}

fn f_sub1(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = lisp.number(args[0])?;
    let r = match n {
        Num::Fix(i) => match i.checked_sub(1) {
            Some(r) => Num::Fix(r),
            None => Num::Big(BigInt::from(i) - 1),
        },
        Num::Float(f) => Num::Float(f - 1.0),
        Num::Big(b) => Num::Big(b - 1),
    };
    Ok(lisp.make_num(normalize(r)))
}

fn f_lognot(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = lisp.check_integer(args[0])?;
    let r = match n {
        Num::Fix(i) => Num::Fix(!i),
        Num::Big(b) => Num::Big(!b),
        Num::Float(_) => unreachable!(),
    };
    Ok(lisp.make_num(normalize(r)))
}

fn f_byteorder(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(if cfg!(target_endian = "big") {
        b'B' as i64
    } else {
        b'l' as i64
    }))
}

impl Lisp {
    pub fn bool_vector_bits(&mut self, v: Value) -> LispResult<Vec<bool>> {
        if v.is_vectorlike()
            && let Vectorlike::BoolVector(b) = self.heap.vectorlike_ref(v)
        {
            return Ok(b.bits.clone());
        }
        Err(self.wrong_type(sym::BOOL_VECTOR_P, v))
    }

    pub fn make_bool_vector(&mut self, bits: Vec<bool>) -> Value {
        self.heap
            .vectorlike(Vectorlike::BoolVector(BoolVector { bits }))
    }
}

fn bool_vector_binop(lisp: &mut Lisp, args: &[Value], op: fn(bool, bool) -> bool) -> LispResult {
    let a = lisp.bool_vector_bits(args[0])?;
    let b = lisp.bool_vector_bits(args[1])?;
    if a.len() != b.len() {
        return Err(lisp.signal_list(sym::WRONG_LENGTH_ARGUMENT, &[args[0], args[1]]));
    }
    let result: Vec<bool> = a.iter().zip(&b).map(|(&x, &y)| op(x, y)).collect();
    if args[2].is_nil() {
        return Ok(lisp.make_bool_vector(result));
    }
    let dest = args[2];
    let d = lisp.bool_vector_bits(dest)?;
    if d.len() != a.len() {
        return Err(lisp.signal_list(sym::WRONG_LENGTH_ARGUMENT, &[args[0], args[1], dest]));
    }
    let changed = d != result;
    if let Vectorlike::BoolVector(bv) = lisp.heap.vectorlike_mut(dest) {
        bv.bits = result;
    }
    Ok(if changed { dest } else { Value::NIL })
}

fn f_bool_vector_exclusive_or(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    bool_vector_binop(lisp, args, |a, b| a ^ b)
}

fn f_bool_vector_union(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    bool_vector_binop(lisp, args, |a, b| a | b)
}

fn f_bool_vector_intersection(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    bool_vector_binop(lisp, args, |a, b| a & b)
}

fn f_bool_vector_set_difference(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    bool_vector_binop(lisp, args, |a, b| a & !b)
}

fn f_bool_vector_subsetp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.bool_vector_bits(args[0])?;
    let b = lisp.bool_vector_bits(args[1])?;
    if a.len() != b.len() {
        return Err(lisp.signal_list(sym::WRONG_LENGTH_ARGUMENT, &[args[0], args[1]]));
    }
    Ok(bool_value(a.iter().zip(&b).all(|(&x, &y)| !x || y)))
}

fn f_bool_vector_not(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.bool_vector_bits(args[0])?;
    let result: Vec<bool> = a.iter().map(|&x| !x).collect();
    if args[1].is_nil() {
        return Ok(lisp.make_bool_vector(result));
    }
    let dest = args[1];
    let d = lisp.bool_vector_bits(dest)?;
    if d.len() != a.len() {
        return Err(lisp.signal_list(sym::WRONG_LENGTH_ARGUMENT, &[args[0], dest]));
    }
    if let Vectorlike::BoolVector(bv) = lisp.heap.vectorlike_mut(dest) {
        bv.bits = result;
    }
    Ok(dest)
}

fn f_bool_vector_count_population(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.bool_vector_bits(args[0])?;
    Ok(Value::fixnum(a.iter().filter(|&&b| b).count() as i64))
}

fn f_bool_vector_count_consecutive(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.bool_vector_bits(args[0])?;
    let i = lisp.check_fixnat(args[2])? as usize;
    if i > a.len() {
        return Err(lisp.args_out_of_range(args[0], args[2]));
    }
    let target = !args[1].is_nil();
    Ok(Value::fixnum(
        a[i..].iter().take_while(|&&b| b == target).count() as i64,
    ))
}
