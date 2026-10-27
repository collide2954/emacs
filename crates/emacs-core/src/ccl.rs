use std::rc::Rc;

use crate::character::MAX_CHAR;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const STAT_SUCCESS: i32 = 0;
pub const STAT_SUSPEND_BY_SRC: i32 = 1;
pub const STAT_SUSPEND_BY_DST: i32 = 2;
pub const STAT_INVALID_CMD: i32 = 3;
pub const STAT_QUIT: i32 = 4;

const HEADER_BUF_MAG: usize = 0;
const HEADER_EOF: usize = 1;
const HEADER_MAIN: i32 = 2;

const CODE_MAX: i64 = (1 << (28 - 1)) - 1;
const CODE_MIN: i64 = -1 - CODE_MAX;

const SET_REGISTER: i32 = 0x00;
const SET_SHORT_CONST: i32 = 0x01;
const SET_CONST: i32 = 0x02;
const SET_ARRAY: i32 = 0x03;
const JUMP: i32 = 0x04;
const JUMP_COND: i32 = 0x05;
const WRITE_REGISTER_JUMP: i32 = 0x06;
const WRITE_REGISTER_READ_JUMP: i32 = 0x07;
const WRITE_CONST_JUMP: i32 = 0x08;
const WRITE_CONST_READ_JUMP: i32 = 0x09;
const WRITE_STRING_JUMP: i32 = 0x0A;
const WRITE_ARRAY_READ_JUMP: i32 = 0x0B;
const READ_JUMP: i32 = 0x0C;
const BRANCH: i32 = 0x0D;
const READ_REGISTER: i32 = 0x0E;
const WRITE_EXPR_CONST: i32 = 0x0F;
const READ_BRANCH: i32 = 0x10;
const WRITE_REGISTER: i32 = 0x11;
const WRITE_EXPR_REGISTER: i32 = 0x12;
const CALL: i32 = 0x13;
const WRITE_CONST_STRING: i32 = 0x14;
const WRITE_ARRAY: i32 = 0x15;
const END: i32 = 0x16;
const EXPR_SELF_CONST: i32 = 0x17;
const EXPR_SELF_REG: i32 = 0x18;
const SET_EXPR_CONST: i32 = 0x19;
const SET_EXPR_REG: i32 = 0x1A;
const JUMP_COND_EXPR_CONST: i32 = 0x1B;
const JUMP_COND_EXPR_REG: i32 = 0x1C;
const READ_JUMP_COND_EXPR_CONST: i32 = 0x1D;
const READ_JUMP_COND_EXPR_REG: i32 = 0x1E;
const EXTENSION: i32 = 0x1F;

const READ_MULTIBYTE_CHAR2: i32 = 0x00;
const WRITE_MULTIBYTE_CHAR2: i32 = 0x01;
const TRANSLATE_CHARACTER: i32 = 0x02;
const TRANSLATE_CHARACTER_CONST_TBL: i32 = 0x03;
const ITERATE_MULTIPLE_MAP: i32 = 0x10;
const MAP_MULTIPLE: i32 = 0x11;
const MAP_SINGLE: i32 = 0x12;
const LOOKUP_INT_CONST_TBL: i32 = 0x13;
const LOOKUP_CHAR_CONST_TBL: i32 = 0x14;

const OP_PLUS: i32 = 0x00;
const OP_MINUS: i32 = 0x01;
const OP_MUL: i32 = 0x02;
const OP_DIV: i32 = 0x03;
const OP_MOD: i32 = 0x04;
const OP_AND: i32 = 0x05;
const OP_OR: i32 = 0x06;
const OP_XOR: i32 = 0x07;
const OP_LSH: i32 = 0x08;
const OP_RSH: i32 = 0x09;
const OP_LSH8: i32 = 0x0A;
const OP_RSH8: i32 = 0x0B;
const OP_DIVMOD: i32 = 0x0C;
const OP_LS: i32 = 0x10;
const OP_GT: i32 = 0x11;
const OP_EQ: i32 = 0x12;
const OP_LE: i32 = 0x13;
const OP_GE: i32 = 0x14;
const OP_NE: i32 = 0x15;
const OP_DECODE_SJIS: i32 = 0x16;
const OP_ENCODE_SJIS: i32 = 0x17;

const MAX_MAP_SET_LEVEL: usize = 30;
const PROG_STACK_DEPTH: i32 = 256;
const EXECUTE_BUF_SIZE: usize = 1024;

#[derive(Clone, Debug)]
pub struct CclProgram {
    pub idx: i64,
    pub size: i32,
    pub prog: Rc<[i64]>,
    pub ic: i32,
    pub eof_ic: i32,
    pub reg: [i32; 8],
    pub status: i32,
    pub buf_magnification: i32,
    pub stack_idx: i32,
    pub consumed: i32,
    pub produced: i32,
    pub last_block: bool,
    pub quit_silently: bool,
}

impl Default for CclProgram {
    fn default() -> Self {
        CclProgram {
            idx: -1,
            size: 0,
            prog: Rc::from(Vec::new()),
            ic: 0,
            eof_ic: 0,
            reg: [0; 8],
            status: 0,
            buf_magnification: 0,
            stack_idx: 0,
            consumed: 0,
            produced: 0,
            last_block: false,
            quit_silently: false,
        }
    }
}

#[derive(Clone)]
struct Frame {
    prog: Rc<[i64]>,
    ic: i32,
    eof_ic: i32,
}

pub struct CclState {
    pub program_table: Value,
    prog_stack: Vec<Frame>,
    stack_idx_of_map_multiple: i32,
}

impl Default for CclState {
    fn default() -> Self {
        let empty = Frame {
            prog: Rc::from(Vec::new()),
            ic: 0,
            eof_ic: 0,
        };
        CclState {
            program_table: Value::NIL,
            prog_stack: vec![empty; PROG_STACK_DEPTH as usize],
            stack_idx_of_map_multiple: 0,
        }
    }
}

impl CclState {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.push(self.program_table);
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("ccl-program-p", 1, 1, f_ccl_program_p),
    Subr::fixed("ccl-execute", 2, 2, f_ccl_execute),
    Subr::fixed("ccl-execute-on-string", 3, 5, f_ccl_execute_on_string),
    Subr::fixed("register-ccl-program", 2, 2, f_register_ccl_program),
    Subr::fixed(
        "register-code-conversion-map",
        2,
        2,
        f_register_code_conversion_map,
    ),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.ccl.program_table = lisp.vector(vec![Value::NIL; 32]);
    let maps = lisp.vector(vec![Value::NIL; 16]);
    lisp.defvar(sym::CODE_CONVERSION_MAP_VECTOR, maps);
    lisp.defvar(sym::TRANSLATION_HASH_TABLE_VECTOR, Value::NIL);
}

fn int_fixnum(v: Value) -> Option<i32> {
    v.as_fixnum().and_then(|n| i32::try_from(n).ok())
}

fn sjis_to_jis(code: i32) -> i32 {
    let s1 = code >> 8;
    let s2 = code & 0xFF;
    let (j1, j2) = if s2 >= 0x9F {
        (s1 * 2 - if s1 >= 0xE0 { 0x160 } else { 0xE0 }, s2 - 0x7E)
    } else {
        (
            s1 * 2 - if s1 >= 0xE0 { 0x161 } else { 0xE1 },
            s2 - if s2 >= 0x7F { 0x20 } else { 0x1F },
        )
    };
    (j1 << 8) | j2
}

fn jis_to_sjis(code: i32) -> i32 {
    let j1 = code >> 8;
    let j2 = code & 0xFF;
    let (s1, s2) = if j1 & 1 != 0 {
        (
            j1 / 2 + if j1 < 0x5F { 0x71 } else { 0xB1 },
            j2 + if j2 >= 0x60 { 0x20 } else { 0x1F },
        )
    } else {
        (j1 / 2 + if j1 < 0x5F { 0x70 } else { 0xB0 }, j2 + 0x7E)
    };
    (s1 << 8) | s2
}

fn larger_vector(lisp: &mut Lisp, vec: Value, incr_min: usize) -> Value {
    let mut items = lisp.vector_items(vec).to_vec();
    let incr = incr_min.max(items.len() >> 1);
    items.resize(items.len() + incr, Value::NIL);
    lisp.vector(items)
}

enum Flow {
    Next,
    Finish,
    Error,
}

struct Driver<'a> {
    reg: [i32; 8],
    ic: i32,
    code: i32,
    field1: i32,
    field2: i32,
    prog: Rc<[i64]>,
    source: &'a [i32],
    src: Option<usize>,
    out: Option<&'a mut [i32]>,
    dst: Option<usize>,
    stack_idx: i32,
    this_ic: i32,
    eof_ic: i32,
    top_eof_ic: i32,
    last_block: bool,
    status: i32,
    mapping: [(i32, i32); MAX_MAP_SET_LEVEL],
    msp: usize,
    charset_list: Value,
}

impl Driver<'_> {
    fn word(&self, ic: i32) -> i64 {
        usize::try_from(ic)
            .ok()
            .and_then(|i| self.prog.get(i).copied())
            .unwrap_or(i64::MIN)
    }

    fn int(&self, ic: i32) -> i32 {
        self.word(ic) as i32
    }

    fn rrr(&self) -> usize {
        (self.field2 & 7) as usize
    }

    fn big_rrr(&self) -> usize {
        (self.field1 & 7) as usize
    }

    fn mid_rrr(&self) -> usize {
        ((self.field1 >> 3) & 7) as usize
    }

    fn push_mapping(&mut self, rest: i32, orig: i32) {
        if self.msp < MAX_MAP_SET_LEVEL {
            self.mapping[self.msp] = (rest, orig);
            self.msp += 1;
        }
    }

    fn pop_mapping(&mut self) -> (i32, i32) {
        self.msp = self.msp.saturating_sub(1);
        self.mapping[self.msp]
    }

    fn dst_len(&self) -> usize {
        self.out.as_ref().map_or(0, |o| o.len())
    }
}

macro_rules! invalid {
    ($st:ident) => {{
        $st.status = STAT_INVALID_CMD;
        return Ok(Flow::Error);
    }};
}

macro_rules! suspend {
    ($st:ident, $stat:expr) => {{
        $st.ic -= 1;
        $st.status = $stat;
        return Ok(Flow::Finish);
    }};
}

macro_rules! get_range {
    ($st:ident, $ic:expr, $lo:expr, $hi:expr) => {{
        let word = $st.word($ic);
        let lo: i64 = $lo;
        let hi: i64 = $hi;
        if !(lo <= word && word <= hi) {
            invalid!($st);
        }
        word
    }};
}

macro_rules! write_char {
    ($st:ident, $ch:expr) => {{
        let ch: i32 = $ch;
        match $st.dst {
            None => invalid!($st),
            Some(d) if d < $st.dst_len() => {
                if let Some(o) = $st.out.as_deref_mut() {
                    o[d] = ch;
                }
                $st.dst = Some(d + 1);
            }
            Some(_) => suspend!($st, STAT_SUSPEND_BY_DST),
        }
    }};
}

macro_rules! read_char {
    ($st:ident, $r:expr) => {{
        match $st.src {
            None => invalid!($st),
            Some(p) if p < $st.source.len() => {
                $r = $st.source[p];
                $st.src = Some(p + 1);
            }
            Some(_) if $st.last_block => {
                $r = -1;
                $st.ic = $st.top_eof_ic;
                return Ok(Flow::Next);
            }
            Some(_) => suspend!($st, STAT_SUSPEND_BY_SRC),
        }
    }};
}

macro_rules! read_value {
    ($st:ident) => {{
        match $st.src {
            None => invalid!($st),
            Some(p) if p < $st.source.len() => {
                $st.src = Some(p + 1);
                $st.source[p]
            }
            Some(_) if $st.last_block => {
                $st.ic = $st.top_eof_ic;
                return Ok(Flow::Next);
            }
            Some(_) => suspend!($st, STAT_SUSPEND_BY_SRC),
        }
    }};
}

macro_rules! write_string {
    ($st:ident, $len:expr) => {{
        let len: i32 = $len;
        match $st.dst {
            None => invalid!($st),
            Some(d) if d as i64 + len as i64 <= $st.dst_len() as i64 => {
                let ic = $st.ic;
                let multibyte = $st.word(ic) & 0x1000000 != 0;
                let mut d = d;
                for k in 0..len.max(0) {
                    let ch = if multibyte {
                        ($st.word(ic + k) & 0xFFFFFF) as i32
                    } else {
                        (($st.word(ic + k / 3) >> ((2 - (k % 3)) * 8)) & 0xFF) as i32
                    };
                    if let Some(o) = $st.out.as_deref_mut() {
                        o[d] = ch;
                    }
                    d += 1;
                }
                $st.dst = Some(d);
            }
            Some(_) => suspend!($st, STAT_SUSPEND_BY_DST),
        }
    }};
}

impl Lisp {
    fn ccl_code_of(&self, vector: Value) -> Rc<[i64]> {
        self.vector_items(vector)
            .iter()
            .map(|v| v.as_fixnum().unwrap_or(i64::MIN))
            .collect()
    }

    fn resolve_symbol_ccl_program(&mut self, ccl: Value) -> LispResult {
        let size = self.vector_items(ccl).len();
        if size <= HEADER_MAIN as usize || size > i32::MAX as usize {
            return Ok(Value::NIL);
        }
        let result = crate::fns::f_copy_sequence(self, &[ccl])?;
        let mut unresolved = false;
        for i in 0..size {
            let contents = self.vector_items(result)[i];
            if int_fixnum(contents).is_some() {
                continue;
            }
            if contents.is_cons()
                && self.xcar(contents).is_symbol()
                && self.xcdr(contents).is_symbol()
            {
                let val = self.get(self.xcar(contents), self.xcdr(contents));
                if int_fixnum(val).is_some_and(|n| n >= 0) {
                    self.aset_value(result, Value::fixnum(i as i64), val)?;
                } else {
                    unresolved = true;
                }
                continue;
            }
            if contents.is_symbol() {
                let mut resolved = None;
                for prop in [
                    sym::TRANSLATION_TABLE_ID,
                    sym::CODE_CONVERSION_MAP_ID,
                    sym::CCL_PROGRAM_IDX,
                ] {
                    let val = self.get(contents, prop);
                    if int_fixnum(val).is_some_and(|n| n >= 0) {
                        resolved = Some(val);
                        break;
                    }
                }
                match resolved {
                    Some(val) => {
                        self.aset_value(result, Value::fixnum(i as i64), val)?;
                    }
                    None => unresolved = true,
                }
                continue;
            }
            return Ok(Value::NIL);
        }
        let items = self.vector_items(result);
        let buf_mag = items[HEADER_BUF_MAG].as_fixnum().unwrap_or(-1);
        let eof = items[HEADER_EOF].as_fixnum().unwrap_or(-1);
        if !(0 <= buf_mag && 0 <= eof && eof <= size as i64) {
            return Ok(Value::NIL);
        }
        Ok(if unresolved { sym::T } else { result })
    }

    fn ccl_get_compiled_code(&mut self, ccl_prog: Value) -> LispResult<(Value, i64)> {
        if self.is_vector(ccl_prog) {
            let val = self.resolve_symbol_ccl_program(ccl_prog)?;
            return Ok((if self.is_vector(val) { val } else { Value::NIL }, -1));
        }
        if !ccl_prog.is_symbol() {
            return Ok((Value::NIL, -1));
        }
        let table = self.ccl.program_table;
        let len = self.vector_items(table).len() as i64;
        let idx = match self.get(ccl_prog, sym::CCL_PROGRAM_IDX).as_fixnum() {
            Some(n) if 0 <= n && n < len => n,
            _ => return Ok((Value::NIL, -1)),
        };
        let slot = self.vector_items(table)[idx as usize];
        if !self.is_vector(slot)
            || self.vector_items(slot).len() != 4
            || !self.is_vector(self.vector_items(slot)[1])
        {
            return Ok((Value::NIL, -1));
        }
        if self.vector_items(slot)[2].is_nil() {
            let code = self.vector_items(slot)[1];
            let val = self.resolve_symbol_ccl_program(code)?;
            if !self.is_vector(val) {
                return Ok((Value::NIL, idx));
            }
            self.aset_value(slot, Value::fixnum(1), val)?;
            self.aset_value(slot, Value::fixnum(2), sym::T)?;
        }
        Ok((self.vector_items(slot)[1], idx))
    }

    pub fn setup_ccl_program(&mut self, ccl: &mut CclProgram, ccl_prog: Value) -> LispResult<bool> {
        if !ccl_prog.is_nil() {
            let (code, idx) = self.ccl_get_compiled_code(ccl_prog)?;
            ccl.idx = idx;
            if !self.is_vector(code) {
                return Ok(false);
            }
            let items = self.vector_items(code);
            ccl.size = items.len() as i32;
            ccl.eof_ic = items[HEADER_EOF].as_fixnum().unwrap_or(0) as i32;
            ccl.buf_magnification = items[HEADER_BUF_MAG].as_fixnum().unwrap_or(0) as i32;
            ccl.prog = self.ccl_code_of(code);
            if ccl.idx >= 0 {
                let slot = self.vector_items(self.ccl.program_table)[ccl.idx as usize];
                self.aset_value(slot, Value::fixnum(3), Value::NIL)?;
            }
        }
        ccl.ic = HEADER_MAIN;
        ccl.reg = [0; 8];
        ccl.last_block = false;
        ccl.status = 0;
        ccl.stack_idx = 0;
        ccl.quit_silently = false;
        Ok(true)
    }

    pub fn ccl_program_p(&mut self, object: Value) -> LispResult<bool> {
        if self.is_vector(object) {
            let val = self.resolve_symbol_ccl_program(object)?;
            return Ok(self.is_vector(val));
        }
        if !object.is_symbol() {
            return Ok(false);
        }
        let len = self.vector_items(self.ccl.program_table).len() as i64;
        Ok(
            matches!(self.get(object, sym::CCL_PROGRAM_IDX).as_fixnum(), Some(n) if 0 <= n && n < len),
        )
    }

    pub fn check_ccl_program(&mut self, x: Value) -> LispResult<()> {
        if self.ccl_program_p(x)? {
            Ok(())
        } else {
            Err(self.wrong_type(sym::CCLP, x))
        }
    }

    pub fn ccl_driver(
        &mut self,
        ccl: &mut CclProgram,
        source: Option<&[i32]>,
        destination: Option<&mut [i32]>,
        charset_list: Value,
    ) -> LispResult<()> {
        let has_destination = destination.is_some();
        let mut st = Driver {
            reg: ccl.reg,
            ic: ccl.ic,
            code: 0,
            field1: 0,
            field2: 0,
            prog: ccl.prog.clone(),
            source: source.unwrap_or(&[]),
            src: source.map(|_| 0),
            dst: destination.as_ref().map(|_| 0),
            out: destination,
            stack_idx: ccl.stack_idx,
            this_ic: 0,
            eof_ic: ccl.eof_ic,
            top_eof_ic: ccl.eof_ic,
            last_block: ccl.last_block,
            status: ccl.status,
            mapping: [(0, 0); MAX_MAP_SET_LEVEL],
            msp: 0,
            charset_list,
        };
        if ccl.buf_magnification == 0 {
            st.dst = None;
        }
        let mut errored = false;
        loop {
            if self.var_true(sym::QUIT_FLAG) && !self.var_true(sym::INHIBIT_QUIT) {
                if st.src.is_some() {
                    st.src = Some(st.source.len());
                }
                st.status = STAT_QUIT;
                errored = true;
                break;
            }
            match self.ccl_step(&mut st)? {
                Flow::Next => {}
                Flow::Finish => break,
                Flow::Error => {
                    errored = true;
                    break;
                }
            }
        }
        if errored && has_destination {
            let mut d = st.dst.unwrap_or(0);
            let msg = match st.status {
                STAT_INVALID_CMD => format!(
                    "\nCCL: Invalid command {:x} (ccl_code = {:x}) at {}.",
                    st.code & 0x1F,
                    st.code as u32,
                    st.this_ic
                ),
                STAT_QUIT if ccl.quit_silently => String::new(),
                STAT_QUIT => "\nCCL: Quitted.".to_string(),
                s => format!("\nCCL: Unknown error type ({s})"),
            };
            if msg.len() <= st.dst_len() - d
                && let Some(o) = st.out.as_deref_mut()
            {
                for b in msg.bytes() {
                    o[d] = b as i32;
                    d += 1;
                }
            }
            if st.status == STAT_INVALID_CMD {
                st.src = Some(st.source.len());
            }
            st.dst = Some(d);
        }
        ccl.reg = st.reg;
        ccl.ic = st.ic;
        ccl.stack_idx = st.stack_idx;
        ccl.prog = st.prog;
        ccl.status = st.status;
        ccl.consumed = st.src.unwrap_or(0) as i32;
        ccl.produced = st.dst.unwrap_or(0) as i32;
        Ok(())
    }

    fn ccl_decode_char(&mut self, id: i32, code: i32) -> LispResult<i32> {
        if id == 0 {
            return Ok(code);
        }
        if id < 0 || id as usize >= self.charsets.table.len() {
            return Ok(-1);
        }
        self.decode_char(id, code as u32)
    }

    fn ccl_encode_char(
        &mut self,
        st: &mut Driver,
        c: i32,
        id_reg: usize,
        code_reg: usize,
    ) -> LispResult<()> {
        if !(0..=MAX_CHAR as i32).contains(&c) {
            return Ok(());
        }
        let mut found = self.char_charset(c, st.charset_list)?;
        if found.is_none() && !st.charset_list.is_nil() {
            found = self.char_charset(c, Value::NIL)?;
        }
        if let Some((id, code)) = found {
            st.reg[id_reg] = id;
            st.reg[code_reg] = code as i32;
        }
        Ok(())
    }

    fn ccl_translation_table(&self, id: i64) -> Value {
        let vector = self.bound_var(sym::TRANSLATION_TABLE_VECTOR);
        if !self.is_vector(vector) {
            return Value::NIL;
        }
        match usize::try_from(id)
            .ok()
            .and_then(|i| self.vector_items(vector).get(i).copied())
        {
            Some(entry) if entry.is_cons() => self.xcdr(entry),
            _ => Value::NIL,
        }
    }

    fn ccl_hash_table(&self, id: i64) -> Option<Value> {
        let vector = self.bound_var(sym::TRANSLATION_HASH_TABLE_VECTOR);
        if !self.is_vector(vector) {
            return None;
        }
        let entry = usize::try_from(id)
            .ok()
            .and_then(|i| self.vector_items(vector).get(i).copied())?;
        if !entry.is_cons() {
            return None;
        }
        let table = self.xcdr(entry);
        self.is_hash_table(table).then_some(table)
    }

    fn vector_size_or_minus_one(&self, symbol: Value) -> i64 {
        let v = self.bound_var(symbol);
        if self.is_vector(v) {
            self.vector_items(v).len() as i64
        } else {
            -1
        }
    }

    fn ccl_map_lookup(&self, maps: Value, map_size: i64, point: i64, op: i32) -> Option<Value> {
        if !(0 <= point && point < map_size) {
            return None;
        }
        let map = self.vector_items(maps)[point as usize];
        if !map.is_cons() {
            return None;
        }
        let map = self.xcdr(map);
        if !self.is_vector(map) {
            return None;
        }
        let items = self.vector_items(map);
        let size = items.len() as i64;
        if size <= 1 {
            return None;
        }
        let content = items[0];
        let op = op as i64;
        if let Some(start) = content.as_fixnum() {
            if !(start <= op && op - start + 1 < size) {
                return None;
            }
            Some(items[(op - start + 1) as usize])
        } else if content == sym::T {
            if size != 4 {
                return None;
            }
            match (items[2].as_fixnum(), items[3].as_fixnum()) {
                (Some(lo), Some(hi)) if lo <= op && op < hi => Some(items[1]),
                _ => None,
            }
        } else {
            None
        }
    }

    fn ccl_call_for_map(
        &mut self,
        st: &mut Driver,
        symbol: Value,
        ret_ic: i32,
    ) -> LispResult<Flow> {
        let mut called = CclProgram::default();
        if st.stack_idx >= PROG_STACK_DEPTH || !self.setup_ccl_program(&mut called, symbol)? {
            if st.stack_idx > 0 {
                let frame = self.ccl.prog_stack[0].clone();
                st.prog = frame.prog;
                st.ic = frame.ic;
                st.eof_ic = frame.eof_ic;
            }
            invalid!(st);
        }
        self.ccl.prog_stack[st.stack_idx as usize] = Frame {
            prog: st.prog.clone(),
            ic: ret_ic,
            eof_ic: st.eof_ic,
        };
        st.stack_idx += 1;
        st.prog = called.prog;
        st.ic = HEADER_MAIN;
        st.eof_ic = st.int(HEADER_EOF as i32);
        Ok(Flow::Next)
    }

    fn ccl_step(&mut self, st: &mut Driver) -> LispResult<Flow> {
        st.this_ic = st.ic;
        let code = get_range!(st, st.ic, CODE_MIN, CODE_MAX) as i32;
        st.ic += 1;
        st.code = code;
        st.field1 = code >> 8;
        st.field2 = (code & 0xFF) >> 5;
        let mut i: i32;
        let j: i32;
        let op: i32;
        let jump_address: i32;
        match code & 0x1F {
            SET_REGISTER => {
                st.reg[st.rrr()] = st.reg[st.big_rrr()];
                return Ok(Flow::Next);
            }
            SET_SHORT_CONST => {
                st.reg[st.rrr()] = st.field1;
                return Ok(Flow::Next);
            }
            SET_CONST => {
                st.reg[st.rrr()] = st.int(st.ic);
                st.ic += 1;
                return Ok(Flow::Next);
            }
            SET_ARRAY => {
                i = st.reg[st.big_rrr()];
                j = st.field1 >> 3;
                if 0 <= i && i < j {
                    st.reg[st.rrr()] = st.int(st.ic + i);
                }
                st.ic += j;
                return Ok(Flow::Next);
            }
            JUMP => {
                st.ic += st.field1;
                return Ok(Flow::Next);
            }
            JUMP_COND => {
                if st.reg[st.rrr()] == 0 {
                    st.ic += st.field1;
                }
                return Ok(Flow::Next);
            }
            WRITE_REGISTER_JUMP => {
                write_char!(st, st.reg[st.rrr()]);
                st.ic += st.field1;
                return Ok(Flow::Next);
            }
            WRITE_REGISTER_READ_JUMP => {
                write_char!(st, st.reg[st.rrr()]);
                st.ic += 1;
                let r = st.rrr();
                read_char!(st, st.reg[r]);
                st.ic += st.field1 - 1;
                return Ok(Flow::Next);
            }
            WRITE_CONST_JUMP => {
                write_char!(st, st.int(st.ic));
                st.ic += st.field1;
                return Ok(Flow::Next);
            }
            WRITE_CONST_READ_JUMP => {
                write_char!(st, st.int(st.ic));
                st.ic += 1;
                let r = st.rrr();
                read_char!(st, st.reg[r]);
                st.ic += st.field1 - 1;
                return Ok(Flow::Next);
            }
            WRITE_STRING_JUMP => {
                j = st.int(st.ic);
                st.ic += 1;
                write_string!(st, j);
                st.ic += st.field1 - 1;
                return Ok(Flow::Next);
            }
            WRITE_ARRAY_READ_JUMP => {
                i = st.reg[st.rrr()];
                j = st.int(st.ic);
                if 0 <= i && i < j {
                    write_char!(st, st.int(st.ic + 1 + i));
                }
                st.ic += j + 2;
                let r = st.rrr();
                read_char!(st, st.reg[r]);
                st.ic += st.field1 - (j + 2);
                return Ok(Flow::Next);
            }
            READ_JUMP => {
                let r = st.rrr();
                read_char!(st, st.reg[r]);
                st.ic += st.field1;
                return Ok(Flow::Next);
            }
            READ_BRANCH | BRANCH => {
                let r = st.rrr();
                if code & 0x1F == READ_BRANCH {
                    read_char!(st, st.reg[r]);
                }
                let v = st.reg[r];
                let ioff = if 0 <= v && v < st.field1 {
                    v
                } else {
                    st.field1
                };
                st.ic += st.int(st.ic + ioff);
                return Ok(Flow::Next);
            }
            READ_REGISTER => {
                loop {
                    let r = st.rrr();
                    read_char!(st, st.reg[r]);
                    if st.field1 == 0 {
                        break;
                    }
                    let c = get_range!(st, st.ic, CODE_MIN, CODE_MAX) as i32;
                    st.ic += 1;
                    st.code = c;
                    st.field1 = c >> 8;
                    st.field2 = (c & 0xFF) >> 5;
                }
                return Ok(Flow::Next);
            }
            WRITE_EXPR_CONST => {
                st.field2 = 7;
                i = st.reg[st.big_rrr()];
                j = st.int(st.ic);
                op = st.field1 >> 6;
                jump_address = st.ic + 1;
            }
            WRITE_REGISTER => {
                loop {
                    write_char!(st, st.reg[st.rrr()]);
                    if st.field1 == 0 {
                        break;
                    }
                    let c = get_range!(st, st.ic, CODE_MIN, CODE_MAX) as i32;
                    st.ic += 1;
                    st.code = c;
                    st.field1 = c >> 8;
                    st.field2 = (c & 0xFF) >> 5;
                }
                return Ok(Flow::Next);
            }
            WRITE_EXPR_REGISTER => {
                st.field2 = 7;
                i = st.reg[st.big_rrr()];
                j = st.reg[st.mid_rrr()];
                op = st.field1 >> 6;
                jump_address = st.ic;
            }
            CALL => {
                let prog_id = if st.rrr() != 0 {
                    let id = st.int(st.ic);
                    st.ic += 1;
                    id
                } else {
                    st.field1
                };
                let table = self.ccl.program_table;
                let slot = usize::try_from(prog_id)
                    .ok()
                    .and_then(|p| self.vector_items(table).get(p).copied())
                    .filter(|s| self.is_vector(*s));
                let code_vec = slot
                    .and_then(|s| self.vector_items(s).get(1).copied())
                    .filter(|c| self.is_vector(*c));
                let code_vec = match code_vec {
                    Some(c) if st.stack_idx < PROG_STACK_DEPTH => c,
                    _ => {
                        if st.stack_idx > 0 {
                            let frame = self.ccl.prog_stack[0].clone();
                            st.prog = frame.prog;
                            st.ic = frame.ic;
                            st.eof_ic = frame.eof_ic;
                        }
                        invalid!(st);
                    }
                };
                self.ccl.prog_stack[st.stack_idx as usize] = Frame {
                    prog: st.prog.clone(),
                    ic: st.ic,
                    eof_ic: st.eof_ic,
                };
                st.stack_idx += 1;
                st.prog = self.ccl_code_of(code_vec);
                st.ic = HEADER_MAIN;
                st.eof_ic = st.int(HEADER_EOF as i32);
                return Ok(Flow::Next);
            }
            WRITE_CONST_STRING => {
                if st.rrr() == 0 {
                    write_char!(st, st.field1);
                } else {
                    write_string!(st, st.field1);
                    st.ic += (st.field1 + 2) / 3;
                }
                return Ok(Flow::Next);
            }
            WRITE_ARRAY => {
                i = st.reg[st.rrr()];
                if 0 <= i && i < st.field1 {
                    write_char!(st, st.int(st.ic + i));
                }
                st.ic += st.field1;
                return Ok(Flow::Next);
            }
            END => {
                if st.stack_idx > 0 {
                    st.stack_idx -= 1;
                    let frame = self.ccl.prog_stack[st.stack_idx as usize].clone();
                    st.prog = frame.prog;
                    st.ic = frame.ic;
                    st.eof_ic = frame.eof_ic;
                    return Ok(Flow::Next);
                }
                if st.src.is_some() {
                    st.src = Some(st.source.len());
                }
                st.ic -= 1;
                st.status = STAT_SUCCESS;
                return Ok(Flow::Finish);
            }
            EXPR_SELF_CONST | EXPR_SELF_REG => {
                if code & 0x1F == EXPR_SELF_CONST {
                    i = st.int(st.ic);
                    st.ic += 1;
                } else {
                    i = st.reg[st.big_rrr()];
                }
                let r = st.rrr();
                match st.field1 >> 6 {
                    OP_PLUS => st.reg[r] = st.reg[r].wrapping_add(i),
                    OP_MINUS => st.reg[r] = st.reg[r].wrapping_sub(i),
                    OP_MUL => st.reg[r] = st.reg[r].wrapping_mul(i),
                    OP_DIV => {
                        if i == 0 {
                            invalid!(st);
                        }
                        if !(st.reg[r] == i32::MIN && i == -1) {
                            st.reg[r] /= i;
                        }
                    }
                    OP_MOD => {
                        if i == 0 {
                            invalid!(st);
                        }
                        st.reg[r] = if i == -1 { 0 } else { st.reg[r] % i };
                    }
                    OP_AND => st.reg[r] &= i,
                    OP_OR => st.reg[r] |= i,
                    OP_XOR => st.reg[r] ^= i,
                    OP_LSH => {
                        if i < 0 {
                            invalid!(st);
                        }
                        st.reg[r] = if i < 32 {
                            ((st.reg[r] as u32) << i) as i32
                        } else {
                            0
                        };
                    }
                    OP_RSH => {
                        if i < 0 {
                            invalid!(st);
                        }
                        st.reg[r] >>= i.min(31);
                    }
                    OP_LSH8 => {
                        st.reg[r] = ((st.reg[r] as u32) << 8) as i32;
                        st.reg[r] |= i;
                    }
                    OP_RSH8 => {
                        st.reg[7] = st.reg[r] & 0xFF;
                        st.reg[r] >>= 8;
                    }
                    OP_DIVMOD => {
                        if i == 0 {
                            invalid!(st);
                        }
                        if i == -1 {
                            st.reg[7] = 0;
                            st.reg[r] = 0i32.wrapping_sub(st.reg[r]);
                        } else {
                            st.reg[7] = st.reg[r] % i;
                            st.reg[r] /= i;
                        }
                    }
                    OP_LS => st.reg[r] = (st.reg[r] < i) as i32,
                    OP_GT => st.reg[r] = (st.reg[r] > i) as i32,
                    OP_EQ => st.reg[r] = (st.reg[r] == i) as i32,
                    OP_LE => st.reg[r] = (st.reg[r] <= i) as i32,
                    OP_GE => st.reg[r] = (st.reg[r] >= i) as i32,
                    OP_NE => st.reg[r] = (st.reg[r] != i) as i32,
                    _ => invalid!(st),
                }
                return Ok(Flow::Next);
            }
            SET_EXPR_CONST => {
                i = st.reg[st.big_rrr()];
                j = st.int(st.ic);
                st.ic += 1;
                op = st.field1 >> 6;
                jump_address = st.ic;
            }
            SET_EXPR_REG => {
                i = st.reg[st.big_rrr()];
                j = st.reg[st.mid_rrr()];
                op = st.field1 >> 6;
                jump_address = st.ic;
            }
            READ_JUMP_COND_EXPR_CONST | JUMP_COND_EXPR_CONST => {
                let r = st.rrr();
                if code & 0x1F == READ_JUMP_COND_EXPR_CONST {
                    read_char!(st, st.reg[r]);
                }
                i = st.reg[r];
                jump_address = st.ic + st.field1;
                op = st.int(st.ic);
                st.ic += 1;
                j = st.int(st.ic);
                st.ic += 1;
                st.field2 = 7;
            }
            READ_JUMP_COND_EXPR_REG | JUMP_COND_EXPR_REG => {
                let r = st.rrr();
                if code & 0x1F == READ_JUMP_COND_EXPR_REG {
                    read_char!(st, st.reg[r]);
                }
                i = st.reg[r];
                jump_address = st.ic + st.field1;
                op = st.int(st.ic);
                st.ic += 1;
                let k = get_range!(st, st.ic, 0, 7);
                st.ic += 1;
                j = st.reg[k as usize];
                st.field2 = 7;
            }
            EXTENSION => return self.ccl_extension(st),
            _ => invalid!(st),
        }
        let r = st.rrr();
        match op {
            OP_PLUS => st.reg[r] = i.wrapping_add(j),
            OP_MINUS => st.reg[r] = i.wrapping_sub(j),
            OP_MUL => st.reg[r] = i.wrapping_mul(j),
            OP_DIV => {
                if j == 0 {
                    invalid!(st);
                }
                if !(i == i32::MIN && j == -1) {
                    i /= j;
                }
                st.reg[r] = i;
            }
            OP_MOD => {
                if j == 0 {
                    invalid!(st);
                }
                st.reg[r] = if j == -1 { 0 } else { i % j };
            }
            OP_AND => st.reg[r] = i & j,
            OP_OR => st.reg[r] = i | j,
            OP_XOR => st.reg[r] = i ^ j,
            OP_LSH => {
                if j < 0 {
                    invalid!(st);
                }
                st.reg[r] = if j < 32 { ((i as u32) << j) as i32 } else { 0 };
            }
            OP_RSH => {
                if j < 0 {
                    invalid!(st);
                }
                st.reg[r] = i >> j.min(31);
            }
            OP_LSH8 => st.reg[r] = (((i as u32) << 8) as i32) | j,
            OP_RSH8 => {
                st.reg[r] = i >> 8;
                st.reg[7] = i & 0xFF;
            }
            OP_DIVMOD => {
                if j == 0 {
                    invalid!(st);
                }
                if j == -1 {
                    st.reg[r] = 0i32.wrapping_sub(st.reg[r]);
                    st.reg[7] = 0;
                } else {
                    st.reg[r] = i / j;
                    st.reg[7] = i % j;
                }
            }
            OP_LS => st.reg[r] = (i < j) as i32,
            OP_GT => st.reg[r] = (i > j) as i32,
            OP_EQ => st.reg[r] = (i == j) as i32,
            OP_LE => st.reg[r] = (i <= j) as i32,
            OP_GE => st.reg[r] = (i >= j) as i32,
            OP_NE => st.reg[r] = (i != j) as i32,
            OP_DECODE_SJIS => {
                let v = sjis_to_jis((((i as u32) << 8) as i32) | j);
                st.reg[r] = v >> 8;
                st.reg[7] = v & 0xFF;
            }
            OP_ENCODE_SJIS => {
                let v = jis_to_sjis((((i as u32) << 8) as i32) | j);
                st.reg[r] = v >> 8;
                st.reg[7] = v & 0xFF;
            }
            _ => invalid!(st),
        }
        let c = st.code & 0x1F;
        if c == WRITE_EXPR_CONST || c == WRITE_EXPR_REGISTER {
            write_char!(st, st.reg[r]);
            st.ic = jump_address;
        } else if st.reg[r] == 0 {
            st.ic = jump_address;
        }
        Ok(Flow::Next)
    }

    fn ccl_extension(&mut self, st: &mut Driver) -> LispResult<Flow> {
        let (r, big, mid) = (st.rrr(), st.big_rrr(), st.mid_rrr());
        match st.field1 >> 6 {
            READ_MULTIBYTE_CHAR2 => {
                if st.src.is_none() {
                    invalid!(st);
                }
                let c = read_value!(st);
                self.ccl_encode_char(st, c, big, r)?;
            }
            WRITE_MULTIBYTE_CHAR2 => {
                if st.dst.is_none() {
                    invalid!(st);
                }
                let c = self.ccl_decode_char(st.reg[big], st.reg[r])?;
                write_char!(st, c);
            }
            TRANSLATE_CHARACTER => {
                let c = self.ccl_decode_char(st.reg[big], st.reg[r])?;
                let table = self.ccl_translation_table(st.reg[mid] as i64);
                let t = self.translate_char(table, c as u32) as i32;
                self.ccl_encode_char(st, t, big, r)?;
            }
            TRANSLATE_CHARACTER_CONST_TBL => {
                let hi = self.vector_size_or_minus_one(sym::TRANSLATION_TABLE_VECTOR);
                let eop = get_range!(st, st.ic, 0, hi);
                st.ic += 1;
                let c = self.ccl_decode_char(st.reg[big], st.reg[r])?;
                let table = self.ccl_translation_table(eop);
                let t = self.translate_char(table, c as u32) as i32;
                self.ccl_encode_char(st, t, big, r)?;
            }
            LOOKUP_INT_CONST_TBL => {
                let hi = self.vector_size_or_minus_one(sym::TRANSLATION_HASH_TABLE_VECTOR);
                let eop = get_range!(st, st.ic, 0, hi);
                st.ic += 1;
                let Some(h) = self.ccl_hash_table(eop) else {
                    invalid!(st);
                };
                let key = Value::fixnum(st.reg[big] as i64);
                match self.hash_lookup_key(h, key)? {
                    Some(opl) => {
                        let Some(c) = opl
                            .as_fixnum()
                            .filter(|c| (0..=MAX_CHAR as i64).contains(c))
                        else {
                            invalid!(st);
                        };
                        st.reg[big] = self.charsets.unicode;
                        st.reg[r] = c as i32;
                        st.reg[7] = 1;
                    }
                    None => st.reg[7] = 0,
                }
            }
            LOOKUP_CHAR_CONST_TBL => {
                let hi = self.vector_size_or_minus_one(sym::TRANSLATION_HASH_TABLE_VECTOR);
                let eop = get_range!(st, st.ic, 0, hi);
                st.ic += 1;
                let c = self.ccl_decode_char(st.reg[big], st.reg[r])?;
                let Some(h) = self.ccl_hash_table(eop) else {
                    invalid!(st);
                };
                match self.hash_lookup_key(h, Value::fixnum(c as i64))? {
                    Some(opl) => {
                        let Some(n) = int_fixnum(opl) else {
                            invalid!(st);
                        };
                        st.reg[big] = n;
                        st.reg[7] = 1;
                    }
                    None => st.reg[7] = 0,
                }
            }
            ITERATE_MULTIPLE_MAP => return self.ccl_iterate_multiple_map(st),
            MAP_MULTIPLE => return self.ccl_map_multiple(st),
            MAP_SINGLE => return self.ccl_map_single(st),
            _ => invalid!(st),
        }
        Ok(Flow::Next)
    }

    fn ccl_iterate_multiple_map(&mut self, st: &mut Driver) -> LispResult<Flow> {
        let (r, big) = (st.rrr(), st.big_rrr());
        let j = st.int(st.ic);
        st.ic += 1;
        let fin_ic = st.ic + j;
        let op = st.reg[r];
        let mut i;
        if j > st.reg[big] && j >= 0 {
            st.ic += st.reg[big];
            i = st.reg[big];
        } else {
            st.reg[big] = -1;
            st.ic = fin_ic;
            return Ok(Flow::Next);
        }
        while i < j {
            let maps = self.bound_var(sym::CODE_CONVERSION_MAP_VECTOR);
            if !self.is_vector(maps) {
                i += 1;
                continue;
            }
            let size = self.vector_items(maps).len() as i64;
            let point = st.word(st.ic);
            st.ic += 1;
            let Some(content) = self.ccl_map_lookup(maps, size, point, op) else {
                i += 1;
                continue;
            };
            if content.is_nil() {
                i += 1;
                continue;
            }
            if let Some(n) = int_fixnum(content) {
                st.reg[big] = i;
                st.reg[r] = n;
                break;
            }
            if content == sym::T || content == sym::LAMBDA {
                st.reg[big] = i;
                break;
            }
            if content.is_cons() {
                let attrib = self.xcar(content);
                let value = self.xcdr(content);
                match (attrib.as_fixnum(), int_fixnum(value)) {
                    (Some(_), Some(v)) => {
                        st.reg[big] = i;
                        st.reg[r] = v;
                        break;
                    }
                    _ => {
                        i += 1;
                        continue;
                    }
                }
            }
            if content.is_symbol() {
                return self.ccl_call_for_map(st, content, fin_ic);
            }
            invalid!(st);
        }
        if i == j {
            st.reg[big] = -1;
        }
        st.ic = fin_ic;
        Ok(Flow::Next)
    }

    fn ccl_map_multiple(&mut self, st: &mut Driver) -> LispResult<Flow> {
        let (r, big) = (st.rrr(), st.big_rrr());
        let current_ic = st.this_ic;
        if self.ccl.stack_idx_of_map_multiple > 0 {
            if self.ccl.stack_idx_of_map_multiple <= st.stack_idx {
                self.ccl.stack_idx_of_map_multiple = 0;
                st.msp = 0;
                invalid!(st);
            }
        } else {
            st.msp = 0;
        }
        self.ccl.stack_idx_of_map_multiple = 0;
        let mut rest = st.int(st.ic);
        st.ic += 1;
        let fin_ic = st.ic + rest;
        let mut op = st.reg[r];
        let mut i;
        if rest > st.reg[big] && st.reg[big] >= 0 {
            st.ic += st.reg[big];
            i = st.reg[big];
            rest -= i;
        } else {
            st.ic = fin_ic;
            st.reg[big] = -1;
            st.msp = 0;
            return Ok(Flow::Next);
        }
        if st.msp <= 1 {
            st.msp = 0;
            st.push_mapping(0, op);
            st.reg[big] = -1;
        } else {
            let (_, orig_op) = st.pop_mapping();
            let (l, v) = st.pop_mapping();
            rest = l;
            st.reg[r] = v;
            match op {
                -1 => {
                    op = orig_op;
                    i += 1;
                    st.ic += 1;
                    rest -= 1;
                }
                -2 => {
                    op = st.reg[r];
                    i += 1;
                    st.ic += 1;
                    rest -= 1;
                }
                -3 => {
                    op = orig_op;
                    i += rest;
                    st.ic += rest;
                    rest = 0;
                }
                _ => {
                    i += rest;
                    st.ic += rest;
                    let (l, v) = st.pop_mapping();
                    rest = l;
                    st.reg[r] = v;
                }
            }
        }
        let maps = self.bound_var(sym::CODE_CONVERSION_MAP_VECTOR);
        if !self.is_vector(maps) {
            invalid!(st);
        }
        let map_vector_size = self.vector_items(maps).len() as i64;
        loop {
            while rest > 0 {
                let mut point = st.word(st.ic);
                if point < 0 {
                    point = -point + 1;
                    if st.msp >= MAX_MAP_SET_LEVEL {
                        invalid!(st);
                    }
                    st.push_mapping(rest - point as i32, st.reg[r]);
                    rest = point as i32;
                    st.reg[r] = op;
                    i += 1;
                    st.ic += 1;
                    rest -= 1;
                    continue;
                }
                let content = if point >= map_vector_size {
                    None
                } else {
                    self.ccl_map_lookup(maps, map_vector_size, point, op)
                };
                let content = match content {
                    Some(c) if !c.is_nil() => c,
                    _ => {
                        i += 1;
                        st.ic += 1;
                        rest -= 1;
                        continue;
                    }
                };
                st.reg[big] = i;
                if let Some(n) = int_fixnum(content) {
                    op = n;
                    i += rest - 1;
                    st.ic += rest - 1;
                    let (l, v) = st.pop_mapping();
                    rest = l;
                    st.reg[r] = v;
                    rest += 1;
                } else if content.is_cons() {
                    let attrib = self.xcar(content);
                    let value = self.xcdr(content);
                    match (attrib.as_fixnum(), int_fixnum(value)) {
                        (Some(_), Some(v)) => {
                            op = v;
                            i += rest - 1;
                            st.ic += rest - 1;
                            let (l, v) = st.pop_mapping();
                            rest = l;
                            st.reg[r] = v;
                            rest += 1;
                        }
                        _ => {
                            i += 1;
                            st.ic += 1;
                            rest -= 1;
                            continue;
                        }
                    }
                } else if content == sym::T {
                    op = st.reg[r];
                } else if content == sym::LAMBDA {
                    i += rest;
                    st.ic += rest;
                    break;
                } else if content.is_symbol() {
                    if st.msp >= MAX_MAP_SET_LEVEL {
                        invalid!(st);
                    }
                    st.push_mapping(rest, st.reg[r]);
                    st.push_mapping(rest, op);
                    self.ccl.stack_idx_of_map_multiple = st.stack_idx + 1;
                    return self.ccl_call_for_map(st, content, current_ic);
                } else {
                    invalid!(st);
                }
                i += 1;
                st.ic += 1;
                rest -= 1;
            }
            if st.msp <= 1 {
                break;
            }
            let (l, v) = st.pop_mapping();
            rest = l;
            st.reg[r] = v;
            i += rest;
            st.ic += rest;
            let (l, v) = st.pop_mapping();
            rest = l;
            st.reg[r] = v;
        }
        st.ic = fin_ic;
        st.reg[r] = op;
        Ok(Flow::Next)
    }

    fn ccl_map_single(&mut self, st: &mut Driver) -> LispResult<Flow> {
        let (r, big) = (st.rrr(), st.big_rrr());
        let j = st.word(st.ic);
        st.ic += 1;
        let op = st.reg[r] as i64;
        let maps = self.bound_var(sym::CODE_CONVERSION_MAP_VECTOR);
        if !(self.is_vector(maps) && 0 <= j && j < self.vector_items(maps).len() as i64) {
            st.reg[big] = -1;
            return Ok(Flow::Next);
        }
        let map = self.vector_items(maps)[j as usize];
        if !map.is_cons() {
            st.reg[big] = -1;
            return Ok(Flow::Next);
        }
        let map = self.xcdr(map);
        let start = if self.is_vector(map) {
            let items = self.vector_items(map);
            items
                .first()
                .and_then(|v| v.as_fixnum())
                .filter(|&s| s <= op && op - s + 1 < items.len() as i64)
        } else {
            None
        };
        let Some(start) = start else {
            st.reg[big] = -1;
            return Ok(Flow::Next);
        };
        let point = (op - start + 1) as usize;
        st.reg[big] = 0;
        let content = self.vector_items(map)[point];
        if content.is_nil() {
            st.reg[big] = -1;
        } else if let Some(n) = int_fixnum(content) {
            st.reg[r] = n;
        } else if content == sym::T {
        } else if content.is_cons() {
            let attrib = self.xcar(content);
            let value = self.xcdr(content);
            if let (Some(_), Some(v)) = (attrib.as_fixnum(), int_fixnum(value)) {
                st.reg[r] = v;
            }
        } else if content.is_symbol() {
            let ic = st.ic;
            return self.ccl_call_for_map(st, content, ic);
        } else {
            st.reg[big] = -1;
        }
        Ok(Flow::Next)
    }
}

fn check_vector(lisp: &mut Lisp, v: Value) -> LispResult<()> {
    if lisp.is_vector(v) {
        Ok(())
    } else {
        Err(lisp.wrong_type(sym::VECTORP, v))
    }
}

fn f_ccl_program_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = lisp.ccl_program_p(args[0])?;
    Ok(lisp.bool(v))
}

fn f_ccl_execute(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (ccl_prog, reg) = (args[0], args[1]);
    let mut ccl = CclProgram::default();
    if !lisp.setup_ccl_program(&mut ccl, ccl_prog)? {
        return Err(lisp.error("Invalid CCL program"));
    }
    check_vector(lisp, reg)?;
    if lisp.vector_items(reg).len() != 8 {
        return Err(lisp.error("Length of vector REGISTERS is not 8"));
    }
    for i in 0..8 {
        ccl.reg[i] = int_fixnum(lisp.vector_items(reg)[i]).unwrap_or(0);
    }
    lisp.ccl_driver(&mut ccl, None, None, Value::NIL)?;
    if ccl.status != STAT_SUCCESS {
        return Err(lisp.error(&format!("Error in CCL program at {}th code", ccl.ic)));
    }
    for i in 0..8 {
        let v = lisp.integer(ccl.reg[i] as i64);
        lisp.aset_value(reg, Value::fixnum(i as i64), v)?;
    }
    Ok(Value::NIL)
}

fn f_ccl_execute_on_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let ccl_prog = args[0];
    let status = args[1];
    let s = args[2];
    let contin = args.get(3).copied().unwrap_or(Value::NIL);
    let unibyte_p = args.get(4).copied().unwrap_or(Value::NIL);
    let mut ccl = CclProgram::default();
    if !lisp.setup_ccl_program(&mut ccl, ccl_prog)? {
        return Err(lisp.error("Invalid CCL program"));
    }
    check_vector(lisp, status)?;
    if lisp.vector_items(status).len() != 9 {
        return Err(lisp.error("Length of vector STATUS is not 9"));
    }
    lisp.check_string(s)?;
    for i in 0..8 {
        if lisp.vector_items(status)[i].is_nil() {
            lisp.aset_value(status, Value::fixnum(i as i64), Value::fixnum(0))?;
        }
        if let Some(n) = int_fixnum(lisp.vector_items(status)[i]) {
            ccl.reg[i] = n;
        }
    }
    if let Some(ic) = lisp.vector_items(status)[8].as_fixnum()
        && (ccl.ic as i64) < ic
        && ic < ccl.size as i64
    {
        ccl.ic = ic as i32;
    }
    let chars: Vec<i32> = lisp
        .string_char_vec(s)
        .into_iter()
        .map(|c| c as i32)
        .collect();
    let mut output: Vec<i32> = Vec::new();
    let mut consumed = 0;
    loop {
        let end = (consumed + EXECUTE_BUF_SIZE).min(chars.len());
        let chunk = &chars[consumed..end];
        consumed = end;
        if consumed == chars.len() {
            ccl.last_block = contin.is_nil();
        }
        let mut offset = 0;
        loop {
            let mut destination = [0i32; EXECUTE_BUF_SIZE];
            lisp.ccl_driver(
                &mut ccl,
                Some(&chunk[offset..]),
                Some(&mut destination),
                Value::NIL,
            )?;
            output.extend_from_slice(&destination[..ccl.produced as usize]);
            offset += ccl.consumed as usize;
            if ccl.status != STAT_SUSPEND_BY_DST {
                break;
            }
        }
        if ccl.status != STAT_SUSPEND_BY_SRC || consumed == chars.len() {
            break;
        }
    }
    if ccl.status == STAT_INVALID_CMD {
        return Err(lisp.error(&format!("Error in CCL program at {}th code", ccl.ic)));
    }
    if ccl.status == STAT_QUIT {
        return Err(lisp.error(&format!("CCL program interrupted at {}th code", ccl.ic)));
    }
    for i in 0..8 {
        let v = lisp.integer(ccl.reg[i] as i64);
        lisp.aset_value(status, Value::fixnum(i as i64), v)?;
    }
    let v = lisp.integer(ccl.ic as i64);
    lisp.aset_value(status, Value::fixnum(8), v)?;
    if unibyte_p.is_nil() {
        let chars: Vec<u32> = output.iter().map(|&c| (c as u32) & MAX_CHAR).collect();
        Ok(lisp.string_from_chars(&chars, true))
    } else {
        let bytes: Vec<u8> = output.iter().map(|&c| c as u8).collect();
        Ok(lisp.unibyte_string(&bytes))
    }
}

fn f_register_ccl_program(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let name = lisp.check_symbol(args[0])?;
    let mut ccl_prog = args[1];
    let mut resolved = Value::NIL;
    if !ccl_prog.is_nil() {
        check_vector(lisp, ccl_prog)?;
        resolved = lisp.resolve_symbol_ccl_program(ccl_prog)?;
        if resolved.is_nil() {
            return Err(lisp.error("Error in CCL program"));
        }
        if lisp.is_vector(resolved) {
            ccl_prog = resolved;
            resolved = sym::T;
        } else {
            resolved = Value::NIL;
        }
    }
    let table = lisp.ccl.program_table;
    let len = lisp.vector_items(table).len();
    let mut idx = 0;
    while idx < len {
        let slot = lisp.vector_items(table)[idx];
        if !lisp.is_vector(slot) {
            break;
        }
        if name == lisp.vector_items(slot)[0] {
            lisp.aset_value(slot, Value::fixnum(1), ccl_prog)?;
            lisp.aset_value(slot, Value::fixnum(2), resolved)?;
            lisp.aset_value(slot, Value::fixnum(3), sym::T)?;
            return Ok(Value::fixnum(idx as i64));
        }
        idx += 1;
    }
    if idx == len {
        lisp.ccl.program_table = larger_vector(lisp, table, 1);
    }
    let entry = lisp.vector(vec![name, ccl_prog, resolved, sym::T]);
    let table = lisp.ccl.program_table;
    lisp.aset_value(table, Value::fixnum(idx as i64), entry)?;
    lisp.put(name, sym::CCL_PROGRAM_IDX, Value::fixnum(idx as i64));
    Ok(Value::fixnum(idx as i64))
}

fn f_register_code_conversion_map(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.check_symbol(args[0])?;
    let map = args[1];
    check_vector(lisp, map)?;
    let mut maps = lisp.bound_var(sym::CODE_CONVERSION_MAP_VECTOR);
    if !lisp.is_vector(maps) {
        return Err(lisp.error("Invalid code-conversion-map-vector"));
    }
    let len = lisp.vector_items(maps).len();
    let mut i = 0;
    while i < len {
        let slot = lisp.vector_items(maps)[i];
        if !slot.is_cons() {
            break;
        }
        if symbol == lisp.xcar(slot) {
            let idx = Value::fixnum(i as i64);
            lisp.heap.set_cdr(slot, map);
            lisp.put(symbol, sym::CODE_CONVERSION_MAP, map);
            lisp.put(symbol, sym::CODE_CONVERSION_MAP_ID, idx);
            return Ok(idx);
        }
        i += 1;
    }
    if i == len {
        maps = larger_vector(lisp, maps, 1);
        lisp.set(sym::CODE_CONVERSION_MAP_VECTOR, maps)?;
    }
    let idx = Value::fixnum(i as i64);
    lisp.put(symbol, sym::CODE_CONVERSION_MAP, map);
    lisp.put(symbol, sym::CODE_CONVERSION_MAP_ID, idx);
    let cell = lisp.cons(symbol, map);
    lisp.aset_value(maps, idx, cell)?;
    Ok(idx)
}
