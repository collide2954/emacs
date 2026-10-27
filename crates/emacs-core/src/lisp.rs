use std::io::Write;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::character;
use crate::error::{BUILTIN_ERRORS, Flow, Handler, HandlerKind, LispResult, UNCAUGHT};
use crate::heap::Heap;
use crate::object::{
    Forward, Interned, LispString, Obarray, Symbol, SymbolValue, TrappedWrite, Vectorlike,
};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::{Tag, Value};

#[derive(Clone, Copy, Debug)]
pub enum BacktraceArgs {
    Unevalled(Value),
    Evaluated { start: usize, len: usize },
}

pub type UnwindFn = fn(&mut Lisp, Value) -> LispResult<()>;

#[derive(Clone, Copy)]
pub enum SpecBinding {
    Let {
        symbol: Value,
        old: Value,
    },
    LetLocal {
        symbol: Value,
        old: Value,
        buffer: Value,
    },
    LetDefault {
        symbol: Value,
        old: Value,
    },
    UnwindForms(Value),
    Unwind(UnwindFn, Value),
    Backtrace {
        function: Value,
        args: BacktraceArgs,
        debug_on_exit: bool,
    },
    Nop,
}

pub struct Lisp {
    pub heap: Heap,
    pub obarray: Value,
    pub specpdl: Vec<SpecBinding>,
    pub handlers: Vec<Handler>,
    pub stack: Vec<Value>,
    pub lisp_eval_depth: i64,
    pub gc_inhibited: usize,
    pub gc_stress: bool,
    pub empty_unibyte: Value,
    pub empty_multibyte: Value,
    pub list_of_t: Value,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
    pub rng_state: u64,
    pub current_buffer: Value,
    pub buffers: Vec<Value>,
    pub point_before_last_command_or_undo: usize,
    pub buffer_before_last_command_or_undo: Value,
    pub prin1_to_string_buffer: Value,
    pub pending_boundary: Value,
    pub current_global_map: Value,
    pub search_regs: crate::regex::Registers,
    pub last_thing_searched: Value,
    pub regexp_cache: Vec<crate::search::RegexpCacheEntry>,
    pub syntax_code_objects: Value,
    pub ascii_downcase_table: Value,
    pub ascii_upcase_table: Value,
    pub ascii_canon_table: Value,
    pub ascii_eqv_table: Value,
    pub local_tz: Option<crate::timefns::Zone>,
    pub system_tz: Option<jiff::tz::TimeZone>,
    pub start_instant: std::time::Instant,
    pub charsets: crate::charset::Charsets,
    pub codings: crate::coding::Codings,
    pub ccl: crate::ccl::CclState,
    pub infile: Option<crate::lread::Source>,
    pub kboards: Vec<crate::keyboard::Kboard>,
    pub current_kboard: usize,
    pub initial_kboard: usize,
    pub terminals: crate::terminal::Terminals,
    pub frames: crate::frame::Frames,
    pub windows: crate::window::Windows,
    pub minibuf: crate::minibuf::Minibuf,
    pub faces: crate::xfaces::Faces,
    pub fonts: crate::font::Fonts,
    pub pending_funcalls: Value,
}

impl Default for Lisp {
    fn default() -> Self {
        Self::new()
    }
}

pub const ALL_SUBRS: &[&[Subr]] = &[
    crate::eval::SUBRS,
    crate::data::SUBRS,
    crate::alloc::SUBRS,
    crate::fns::SUBRS,
    crate::lread::SUBRS,
    crate::print::SUBRS,
    crate::floatfns::SUBRS,
    crate::chartab::SUBRS,
    crate::casefiddle::SUBRS,
    crate::charfns::SUBRS,
    crate::buffer::SUBRS,
    crate::marker::SUBRS,
    crate::insdel::SUBRS,
    crate::editfns::SUBRS,
    crate::textprop::SUBRS,
    crate::overlay::SUBRS,
    crate::fileio::SUBRS,
    crate::emacs::SUBRS,
    crate::keyboard::SUBRS,
    crate::keymap::SUBRS,
    crate::syntax::SUBRS,
    crate::category::SUBRS,
    crate::casetab::SUBRS,
    crate::search::SUBRS,
    crate::timefns::SUBRS,
    crate::charset::SUBRS,
    crate::ccl::SUBRS,
    crate::coding::SUBRS,
    crate::terminal::SUBRS,
    crate::frame::SUBRS,
    crate::window::SUBRS,
    crate::minibuf::SUBRS,
    crate::font::SUBRS,
];

type InitFn = fn(&mut Lisp);

pub const ALL_INITS: &[InitFn] = &[
    crate::eval::init_vars,
    crate::data::init_vars,
    crate::alloc::init_vars,
    crate::fns::init_vars,
    crate::lread::init_vars,
    crate::print::init_vars,
    crate::floatfns::init_vars,
    crate::chartab::init_vars,
    crate::charfns::init_vars,
    init_emacs_vars,
    crate::buffer::init_vars,
    crate::editfns::init_vars,
    crate::fileio::init_vars,
    crate::emacs::init_vars,
    crate::keyboard::init_vars,
    crate::window::init_vars,
    crate::keymap::init_vars,
    crate::syntax::init_vars,
    crate::category::init_vars,
    crate::casetab::init_vars,
    crate::search::init_vars,
    crate::timefns::init_vars,
    crate::charset::init_vars,
    crate::ccl::init_vars,
    crate::coding::init_vars,
    crate::terminal::init_vars,
    crate::frame::init_vars,
    crate::minibuf::init_vars,
    crate::font::init_vars,
    crate::xfaces::init_vars,
    init_first_buffers,
    crate::minibuf::init_minibuf_once,
    crate::window::init_window_once,
];

fn init_emacs_vars(lisp: &mut Lisp) {
    lisp.defvar_bool(sym::NONINTERACTIVE, true);
    let version = lisp.string("31.0.50");
    lisp.defvar(sym::EMACS_VERSION, version);
    lisp.defvar(sym::STANDARD_SYNTAX_TABLE_INTERNAL, Value::NIL);
    lisp.defvar(sym::STANDARD_CATEGORY_TABLE_INTERNAL, Value::NIL);
}

fn init_first_buffers(lisp: &mut Lisp) {
    let prin1 = lisp.string(" prin1");
    let prin1 = lisp.get_buffer_create(prin1, true).expect("prin1 buffer");
    lisp.prin1_to_string_buffer = prin1;
    let scratch = lisp.string("*scratch*");
    let scratch = lisp
        .get_buffer_create(scratch, false)
        .expect("scratch buffer");
    lisp.set_buffer_internal(scratch);
}

impl Lisp {
    pub fn new() -> Lisp {
        let mut lisp = Lisp {
            heap: Heap::new(),
            obarray: Value::NIL,
            specpdl: Vec::new(),
            handlers: Vec::new(),
            stack: Vec::new(),
            lisp_eval_depth: 0,
            gc_inhibited: 1,
            gc_stress: std::env::var_os("EMACS_GC_STRESS").is_some(),
            empty_unibyte: Value::NIL,
            empty_multibyte: Value::NIL,
            list_of_t: Value::NIL,
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            rng_state: 0x2545_F491_4F6C_DD1D,
            current_buffer: Value::NIL,
            buffers: Vec::new(),
            point_before_last_command_or_undo: 1,
            buffer_before_last_command_or_undo: Value::NIL,
            prin1_to_string_buffer: Value::NIL,
            pending_boundary: Value::NIL,
            current_global_map: Value::NIL,
            search_regs: crate::regex::Registers::default(),
            last_thing_searched: Value::NIL,
            regexp_cache: Vec::new(),
            syntax_code_objects: Value::NIL,
            ascii_downcase_table: Value::NIL,
            ascii_upcase_table: Value::NIL,
            ascii_canon_table: Value::NIL,
            ascii_eqv_table: Value::NIL,
            local_tz: None,
            system_tz: None,
            start_instant: std::time::Instant::now(),
            charsets: crate::charset::Charsets::default(),
            codings: crate::coding::Codings::default(),
            ccl: crate::ccl::CclState::default(),
            infile: None,
            kboards: Vec::new(),
            current_kboard: 0,
            initial_kboard: 0,
            terminals: crate::terminal::Terminals::default(),
            frames: crate::frame::Frames::default(),
            windows: crate::window::Windows::default(),
            minibuf: crate::minibuf::Minibuf::default(),
            faces: crate::xfaces::Faces::default(),
            fonts: crate::font::Fonts::default(),
            pending_funcalls: Value::NIL,
        };
        lisp.init_symbols();
        lisp.empty_unibyte = lisp.heap.string(LispString::unibyte(Vec::new()));
        lisp.empty_multibyte = lisp.heap.string(LispString::multibyte(Vec::new(), 0));
        lisp.list_of_t = lisp.list(&[sym::T]);
        lisp.init_errors();
        for subrs in ALL_SUBRS {
            for subr in *subrs {
                lisp.defsubr(subr);
            }
        }
        for init in ALL_INITS {
            init(&mut lisp);
        }
        lisp.gc_inhibited = 0;
        lisp
    }

    fn init_symbols(&mut self) {
        let mut obarray = Obarray::default();
        for (index, name) in sym::NAMES.iter().enumerate() {
            let (bytes, chars, multibyte) = character::encode_utf8_str(name);
            let name_value = if multibyte {
                self.heap
                    .string(LispString::multibyte(bytes.clone(), chars))
            } else {
                self.heap.string(LispString::unibyte(bytes.clone()))
            };
            let symbol = self.heap.symbol(Symbol {
                name: name_value,
                value: SymbolValue::Plain(sym::UNBOUND),
                function: Value::NIL,
                plist: Value::NIL,
                interned: Interned::InitialObarray,
                special: false,
                trapped_write: TrappedWrite::Untrapped,
            });
            assert_eq!(symbol.index() as usize, index);
            assert_eq!(symbol.generation(), 0);
            if symbol == sym::UNBOUND || index >= sym::FIRST_INTERNAL {
                self.heap.symbol_mut(symbol).interned = Interned::Uninterned;
            } else {
                obarray.symbols.insert(bytes, symbol);
            }
            if name.starts_with(':') {
                let s = self.heap.symbol_mut(symbol);
                s.value = SymbolValue::Plain(symbol);
                s.trapped_write = TrappedWrite::NoWrite;
                s.special = true;
            }
        }
        self.heap.pin_symbols();
        {
            let unbound = self.heap.symbol_mut(sym::UNBOUND);
            unbound.interned = Interned::Uninterned;
        }
        for (symbol, value) in [(sym::NIL, sym::NIL), (sym::T, sym::T)] {
            let s = self.heap.symbol_mut(symbol);
            s.value = SymbolValue::Plain(value);
            s.trapped_write = TrappedWrite::NoWrite;
            s.special = true;
        }
        self.obarray = self.heap.vectorlike(Vectorlike::Obarray(obarray));
    }

    fn init_errors(&mut self) {
        let message = self.string("error");
        let conditions = self.list(&[sym::ERROR]);
        self.put(sym::ERROR, sym::ERROR_CONDITIONS, conditions);
        self.put(sym::ERROR, sym::ERROR_MESSAGE, message);
        for (symbol, parents, text) in BUILTIN_ERRORS {
            let mut conditions = self.list(parents);
            conditions = self.cons(*symbol, conditions);
            let message = self.string(text);
            self.put(*symbol, sym::ERROR_CONDITIONS, conditions);
            self.put(*symbol, sym::ERROR_MESSAGE, message);
        }
    }

    pub fn defsubr(&mut self, subr: &'static Subr) {
        let function = self.heap.vectorlike(Vectorlike::Subr(subr));
        let symbol = self.intern(subr.name);
        self.heap.symbol_mut(symbol).function = function;
    }

    pub fn defvar(&mut self, symbol: Value, value: Value) {
        let s = self.heap.symbol_mut(symbol);
        s.value = SymbolValue::Plain(value);
        s.special = true;
    }

    pub fn defvar_int(&mut self, symbol: Value, value: i64) {
        let s = self.heap.symbol_mut(symbol);
        s.value = SymbolValue::Forwarded(Forward::Int(value));
        s.special = true;
    }

    pub fn defvar_bool(&mut self, symbol: Value, value: bool) {
        let s = self.heap.symbol_mut(symbol);
        s.value = SymbolValue::Forwarded(Forward::Bool(value));
        s.special = true;
    }

    pub fn defvar_kboard(&mut self, symbol: Value, slot: crate::keyboard::Kvar) {
        let s = self.heap.symbol_mut(symbol);
        s.value = SymbolValue::Forwarded(Forward::Kboard(slot as usize));
        s.special = true;
    }

    pub fn make_constant(&mut self, symbol: Value) {
        self.heap.symbol_mut(symbol).trapped_write = TrappedWrite::NoWrite;
    }

    pub fn cons(&mut self, car: Value, cdr: Value) -> Value {
        self.heap.cons(car, cdr)
    }

    pub fn list(&mut self, items: &[Value]) -> Value {
        let mut tail = Value::NIL;
        for &item in items.iter().rev() {
            tail = self.heap.cons(item, tail);
        }
        tail
    }

    pub fn list_with_tail(&mut self, items: &[Value], mut tail: Value) -> Value {
        for &item in items.iter().rev() {
            tail = self.heap.cons(item, tail);
        }
        tail
    }

    pub fn float(&mut self, value: f64) -> Value {
        self.heap.float(value)
    }

    pub fn vector(&mut self, items: Vec<Value>) -> Value {
        self.heap.vectorlike(Vectorlike::Vector(items))
    }

    pub fn string(&mut self, text: &str) -> Value {
        let (bytes, chars, multibyte) = character::encode_utf8_str(text);
        if multibyte {
            self.heap.string(LispString::multibyte(bytes, chars))
        } else {
            self.heap.string(LispString::unibyte(bytes))
        }
    }

    pub fn multibyte_string(&mut self, text: &str) -> Value {
        let (bytes, chars, _) = character::encode_utf8_str(text);
        self.heap.string(LispString::multibyte(bytes, chars))
    }

    pub fn unibyte_string(&mut self, bytes: &[u8]) -> Value {
        self.heap.string(LispString::unibyte(bytes.to_vec()))
    }

    pub fn make_string_from_bytes(&mut self, bytes: Vec<u8>, multibyte: bool) -> Value {
        if multibyte {
            let chars = character::chars_in_multibyte(&bytes);
            self.heap.string(LispString::multibyte(bytes, chars))
        } else {
            self.heap.string(LispString::unibyte(bytes))
        }
    }

    pub fn make_specified_string(
        &mut self,
        bytes: Vec<u8>,
        chars: usize,
        multibyte: bool,
    ) -> Value {
        if multibyte {
            self.heap.string(LispString::multibyte(bytes, chars))
        } else {
            self.heap.string(LispString::unibyte(bytes))
        }
    }

    pub fn make_symbol(&mut self, name: Value) -> Value {
        self.heap.symbol(Symbol {
            name,
            value: SymbolValue::Plain(sym::UNBOUND),
            function: Value::NIL,
            plist: Value::NIL,
            interned: Interned::Uninterned,
            special: false,
            trapped_write: TrappedWrite::Untrapped,
        })
    }

    pub fn intern(&mut self, name: &str) -> Value {
        let obarray = self.obarray;
        let (bytes, _, _) = character::encode_utf8_str(name);
        if let Some(found) = self.obarray_lookup(obarray, &bytes) {
            return found;
        }
        let name = self.string(name);
        self.intern_new(obarray, name)
    }

    pub fn obarray_lookup(&self, obarray: Value, bytes: &[u8]) -> Option<Value> {
        match self.heap.vectorlike_ref(obarray) {
            Vectorlike::Obarray(o) => o.symbols.get(bytes).copied(),
            _ => None,
        }
    }

    pub fn intern_new(&mut self, obarray: Value, name: Value) -> Value {
        let bytes = self.string_bytes(name).to_vec();
        let symbol = self.make_symbol(name);
        let initial = obarray == self.obarray;
        {
            let s = self.heap.symbol_mut(symbol);
            s.interned = if initial {
                Interned::InitialObarray
            } else {
                Interned::Interned
            };
            if initial && bytes.first() == Some(&b':') {
                s.value = SymbolValue::Plain(symbol);
                s.trapped_write = TrappedWrite::NoWrite;
                s.special = true;
            }
        }
        if let Vectorlike::Obarray(o) = self.heap.vectorlike_mut(obarray) {
            o.symbols.insert(bytes, symbol);
        }
        symbol
    }

    pub fn integer(&mut self, n: i64) -> Value {
        match Value::try_fixnum(n) {
            Some(v) => v,
            None => self.heap.vectorlike(Vectorlike::Bignum(BigInt::from(n))),
        }
    }

    pub fn bignum(&mut self, n: BigInt) -> Value {
        match n.to_i64().and_then(Value::try_fixnum) {
            Some(v) => v,
            None => self.heap.vectorlike(Vectorlike::Bignum(n)),
        }
    }

    pub fn xcar(&self, cell: Value) -> Value {
        self.heap.car(cell)
    }

    pub fn xcdr(&self, cell: Value) -> Value {
        self.heap.cdr(cell)
    }

    pub fn car(&mut self, list: Value) -> LispResult {
        if list.is_cons() {
            Ok(self.heap.car(list))
        } else if list.is_nil() {
            Ok(Value::NIL)
        } else {
            Err(self.wrong_type(sym::LISTP, list))
        }
    }

    pub fn cdr(&mut self, list: Value) -> LispResult {
        if list.is_cons() {
            Ok(self.heap.cdr(list))
        } else if list.is_nil() {
            Ok(Value::NIL)
        } else {
            Err(self.wrong_type(sym::LISTP, list))
        }
    }

    pub fn car_safe(&self, v: Value) -> Value {
        if v.is_cons() {
            self.heap.car(v)
        } else {
            Value::NIL
        }
    }

    pub fn cdr_safe(&self, v: Value) -> Value {
        if v.is_cons() {
            self.heap.cdr(v)
        } else {
            Value::NIL
        }
    }

    pub fn list_to_vec(&mut self, list: Value) -> LispResult<Vec<Value>> {
        let mut out = Vec::new();
        let mut tail = list;
        while tail.is_cons() {
            out.push(self.heap.car(tail));
            tail = self.heap.cdr(tail);
            if out.len() > 1_000_000 && self.list_is_circular(list) {
                return Err(self.signal_list(sym::CIRCULAR_LIST, &[list]));
            }
        }
        if !tail.is_nil() {
            return Err(self.wrong_type(sym::LISTP, list));
        }
        Ok(out)
    }

    pub fn list_is_circular(&self, list: Value) -> bool {
        let mut slow = list;
        let mut fast = list;
        loop {
            for _ in 0..2 {
                if !fast.is_cons() {
                    return false;
                }
                fast = self.heap.cdr(fast);
            }
            slow = self.heap.cdr(slow);
            if slow == fast {
                return true;
            }
        }
    }

    pub fn proper_list_p(&self, list: Value) -> bool {
        let mut tail = list;
        let mut count = 0usize;
        while tail.is_cons() {
            tail = self.heap.cdr(tail);
            count += 1;
            if count.is_power_of_two() && count > 1024 && self.list_is_circular(list) {
                return false;
            }
        }
        tail.is_nil()
    }

    pub fn is_string(&self, v: Value) -> bool {
        v.is_string()
    }

    pub fn string_bytes(&self, v: Value) -> &[u8] {
        &self.heap.string_ref(v).data
    }

    pub fn string_multibyte(&self, v: Value) -> bool {
        self.heap.string_ref(v).multibyte
    }

    pub fn string_chars(&self, v: Value) -> usize {
        self.heap.string_ref(v).chars
    }

    pub fn string_to_rust(&self, v: Value) -> String {
        let s = self.heap.string_ref(v);
        character::decode_to_string(&s.data, s.multibyte)
    }

    pub fn symbol_name(&self, symbol: Value) -> Value {
        self.heap.symbol_ref(symbol).name
    }

    pub fn symbol_name_string(&self, symbol: Value) -> String {
        let name = self.symbol_name(symbol);
        self.string_to_rust(name)
    }

    pub fn subr(&self, v: Value) -> Option<&'static Subr> {
        if !v.is_vectorlike() {
            return None;
        }
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Subr(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_bignum(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Bignum(_))
    }

    pub fn bignum_ref(&self, v: Value) -> Option<&BigInt> {
        if !v.is_vectorlike() {
            return None;
        }
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Bignum(b) => Some(b),
            _ => None,
        }
    }

    pub fn is_integer(&self, v: Value) -> bool {
        v.is_fixnum() || self.is_bignum(v)
    }

    pub fn is_number(&self, v: Value) -> bool {
        v.is_fixnum() || v.is_float() || self.is_bignum(v)
    }

    pub fn is_vector(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Vector(_))
    }

    pub fn is_closure(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Closure(_))
    }

    pub fn is_record(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Record(_))
    }

    pub fn is_hash_table(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::HashTable(_))
    }

    pub fn is_char_table(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::CharTable(_))
    }

    pub fn is_bool_vector(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::BoolVector(_))
    }

    pub fn is_obarray(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Obarray(_))
    }

    pub fn vector_items(&self, v: Value) -> &[Value] {
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Vector(items) | Vectorlike::Record(items) | Vectorlike::Closure(items) => {
                items
            }
            _ => &[],
        }
    }

    pub fn check_symbol(&mut self, v: Value) -> LispResult<Value> {
        if v.is_symbol() {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::SYMBOLP, v))
        }
    }

    pub fn check_string(&mut self, v: Value) -> LispResult<Value> {
        if v.is_string() {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::STRINGP, v))
        }
    }

    pub fn check_cons(&mut self, v: Value) -> LispResult<Value> {
        if v.is_cons() {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::CONSP, v))
        }
    }

    pub fn check_list(&mut self, v: Value) -> LispResult<Value> {
        if v.is_cons() || v.is_nil() {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::LISTP, v))
        }
    }

    pub fn check_fixnum(&mut self, v: Value) -> LispResult<i64> {
        match v.as_fixnum() {
            Some(n) => Ok(n),
            None => Err(self.wrong_type(sym::FIXNUMP, v)),
        }
    }

    pub fn check_fixnat(&mut self, v: Value) -> LispResult<i64> {
        match v.as_fixnum() {
            Some(n) if n >= 0 => Ok(n),
            _ => Err(self.wrong_type(sym::WHOLENUMP, v)),
        }
    }

    pub fn check_character(&mut self, v: Value) -> LispResult<u32> {
        match v.as_fixnum() {
            Some(n) if (0..=character::MAX_CHAR as i64).contains(&n) => Ok(n as u32),
            _ => Err(self.wrong_type(sym::CHARACTERP, v)),
        }
    }

    pub fn get(&self, symbol: Value, property: Value) -> Value {
        let mut tail = self.heap.symbol_ref(symbol).plist;
        while tail.is_cons() {
            let next = self.heap.cdr(tail);
            if !next.is_cons() {
                break;
            }
            if self.heap.car(tail) == property {
                return self.heap.car(next);
            }
            tail = self.heap.cdr(next);
        }
        Value::NIL
    }

    pub fn put(&mut self, symbol: Value, property: Value, value: Value) {
        let plist = self.heap.symbol_ref(symbol).plist;
        let mut tail = plist;
        while tail.is_cons() {
            let next = self.heap.cdr(tail);
            if !next.is_cons() {
                break;
            }
            if self.heap.car(tail) == property {
                self.heap.set_car(next, value);
                return;
            }
            tail = self.heap.cdr(next);
        }
        let new_plist = self.list_with_tail(&[property, value], Value::NIL);
        if plist.is_nil() {
            self.heap.symbol_mut(symbol).plist = new_plist;
        } else {
            let mut last = plist;
            while self.heap.cdr(last).is_cons() {
                last = self.heap.cdr(last);
            }
            self.heap.set_cdr(last, new_plist);
        }
    }

    pub fn plain_value(&self, symbol: Value) -> Value {
        match &self.heap.symbol_ref(symbol).value {
            SymbolValue::Plain(v) => *v,
            SymbolValue::Forwarded(Forward::Int(n)) => Value::fixnum(*n),
            SymbolValue::Forwarded(Forward::Bool(b)) => {
                if *b {
                    sym::T
                } else {
                    Value::NIL
                }
            }
            SymbolValue::Forwarded(Forward::Kboard(slot)) => self.kboard_slot(*slot),
            SymbolValue::Localized(l) => l.default,
            SymbolValue::Alias(target) => self.plain_value(*target),
        }
    }

    pub fn set_plain_value(&mut self, symbol: Value, value: Value) {
        if let SymbolValue::Plain(v) = &mut self.heap.symbol_mut(symbol).value {
            *v = value;
        }
    }

    pub fn var_true(&self, symbol: Value) -> bool {
        !self.plain_value(symbol).is_nil()
    }

    pub fn int_var(&self, symbol: Value) -> i64 {
        self.plain_value(symbol).as_fixnum().unwrap_or(0)
    }

    pub fn push_handler(&mut self, kind: HandlerKind) -> usize {
        self.handlers.push(Handler {
            kind,
            specpdl: self.specpdl.len(),
            stack: self.stack.len(),
            eval_depth: self.lisp_eval_depth,
        });
        self.handlers.len() - 1
    }

    pub fn pop_handler(&mut self, index: usize) {
        self.handlers.truncate(index);
    }

    pub fn catch_all<F>(&mut self, body: F) -> LispResult
    where
        F: FnOnce(&mut Lisp) -> LispResult,
    {
        let index = self.push_handler(HandlerKind::CatchAll);
        let result = body(self);
        match result {
            Ok(v) => {
                self.pop_handler(index);
                Ok(v)
            }
            Err(flow) if flow.target() == index || flow.target() == UNCAUGHT => {
                let flow = self.unwind_to_handler(index, flow)?;
                self.pop_handler(index);
                Err(flow)
            }
            Err(flow) => Err(flow),
        }
    }

    pub fn unwind_to_handler(&mut self, index: usize, flow: Flow) -> LispResult<Flow> {
        let [a, b] = flow.values();
        let mark = self.stack.len();
        self.stack.push(a);
        self.stack.push(b);
        while self.handlers.len() > index + 1 {
            let top = *self.handlers.last().expect("handler");
            self.unbind_to(top.specpdl)?;
            self.handlers.pop();
        }
        let handler = self.handlers[index];
        let result = self.unbind_to(handler.specpdl);
        self.stack.truncate(mark);
        result?;
        self.stack.truncate(handler.stack);
        self.lisp_eval_depth = handler.eval_depth;
        Ok(flow)
    }

    pub fn roots(&self) -> Vec<Value> {
        let mut roots = Vec::with_capacity(self.stack.len() + 3 * self.specpdl.len() + 16);
        roots.push(self.obarray);
        roots.push(self.empty_unibyte);
        roots.push(self.empty_multibyte);
        roots.push(self.list_of_t);
        roots.push(self.current_buffer);
        roots.extend_from_slice(&self.buffers);
        roots.push(self.buffer_before_last_command_or_undo);
        roots.push(self.prin1_to_string_buffer);
        roots.push(self.pending_boundary);
        roots.push(self.current_global_map);
        roots.push(self.last_thing_searched);
        roots.push(self.syntax_code_objects);
        roots.push(self.ascii_downcase_table);
        roots.push(self.ascii_upcase_table);
        roots.push(self.ascii_canon_table);
        roots.push(self.ascii_eqv_table);
        self.charsets.roots(&mut roots);
        self.codings.roots(&mut roots);
        self.ccl.roots(&mut roots);
        self.terminals.roots(&mut roots);
        self.frames.roots(&mut roots);
        self.windows.roots(&mut roots);
        self.minibuf.roots(&mut roots);
        self.faces.roots(&mut roots);
        self.fonts.roots(&mut roots);
        roots.push(self.pending_funcalls);
        for kboard in &self.kboards {
            roots.extend_from_slice(&kboard.vars);
        }
        for entry in &self.regexp_cache {
            roots.push(entry.regexp);
            roots.push(entry.whitespace);
            roots.push(entry.syntax_table);
            roots.push(entry.pattern.translate);
        }
        roots.extend_from_slice(&self.stack);
        for binding in &self.specpdl {
            match *binding {
                SpecBinding::Let { symbol, old } | SpecBinding::LetDefault { symbol, old } => {
                    roots.push(symbol);
                    roots.push(old);
                }
                SpecBinding::LetLocal {
                    symbol,
                    old,
                    buffer,
                } => {
                    roots.push(symbol);
                    roots.push(old);
                    roots.push(buffer);
                }
                SpecBinding::UnwindForms(forms) => roots.push(forms),
                SpecBinding::Unwind(_, arg) => roots.push(arg),
                SpecBinding::Backtrace { function, args, .. } => {
                    roots.push(function);
                    if let BacktraceArgs::Unevalled(list) = args {
                        roots.push(list);
                    }
                }
                SpecBinding::Nop => {}
            }
        }
        for handler in &self.handlers {
            match handler.kind {
                HandlerKind::Catch(v) | HandlerKind::ConditionCase(v) => roots.push(v),
                HandlerKind::HandlerBind {
                    conditions,
                    handler,
                    ..
                } => {
                    roots.push(conditions);
                    roots.push(handler);
                }
                HandlerKind::CatchAll
                | HandlerKind::CatchAllDebuggable
                | HandlerKind::SkipConditions(_) => {}
            }
        }
        roots
    }

    pub fn garbage_collect(&mut self) -> usize {
        let roots = self.roots();
        self.heap.collect(&roots)
    }

    pub fn maybe_gc(&mut self) {
        if self.gc_inhibited > 0 {
            return;
        }
        if self.gc_stress {
            self.garbage_collect();
            return;
        }
        let threshold = self.int_var(sym::GC_CONS_THRESHOLD).max(100_000) as usize;
        if self.heap.bytes_since_gc > threshold {
            self.garbage_collect();
        }
    }

    pub fn type_of_tag(v: Value) -> Tag {
        v.tag()
    }
}
