use crate::character::MAX_CHAR;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::{CharTable, SubCharTable, Vectorlike};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const CHARTAB_SIZE_BITS: [u32; 4] = [6, 4, 5, 7];
pub const CHARTAB_SIZE: [usize; 4] = [64, 16, 32, 128];
pub const CHARTAB_CHARS: [u32; 4] = [1 << 16, 1 << 12, 1 << 7, 1];
pub const CHARTAB_BITS: [u32; 4] = [16, 12, 7, 0];
pub const CHAR_TABLE_STANDARD_SLOTS: usize = 4 + 64;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("make-char-table", 1, 2, f_make_char_table),
    Subr::fixed("char-table-subtype", 1, 1, f_char_table_subtype),
    Subr::fixed("char-table-parent", 1, 1, f_char_table_parent),
    Subr::fixed("set-char-table-parent", 2, 2, f_set_char_table_parent),
    Subr::fixed("char-table-extra-slot", 2, 2, f_char_table_extra_slot),
    Subr::fixed(
        "set-char-table-extra-slot",
        3,
        3,
        f_set_char_table_extra_slot,
    ),
    Subr::fixed("char-table-range", 2, 2, f_char_table_range),
    Subr::fixed("set-char-table-range", 3, 3, f_set_char_table_range),
    Subr::fixed("optimize-char-table", 1, 2, f_optimize_char_table),
    Subr::fixed("map-char-table", 2, 2, f_map_char_table),
    Subr::fixed(
        "unicode-property-table-internal",
        1,
        1,
        f_unicode_property_table_internal,
    ),
    Subr::fixed(
        "get-unicode-property-internal",
        2,
        2,
        f_get_unicode_property_internal,
    ),
    Subr::fixed(
        "put-unicode-property-internal",
        3,
        3,
        f_put_unicode_property_internal,
    ),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::CHAR_CODE_PROPERTY_ALIST, Value::NIL);
    lisp.put(
        sym::CHAR_CODE_PROPERTY_TABLE,
        sym::CHAR_TABLE_EXTRA_SLOTS,
        Value::fixnum(5),
    );
}

fn chartab_idx(c: u32, depth: usize, min_char: u32) -> usize {
    ((c - min_char) >> CHARTAB_BITS[depth]) as usize
}

impl Lisp {
    pub(crate) fn char_table(&self, table: Value) -> &CharTable {
        match self.heap.vectorlike_ref(table) {
            Vectorlike::CharTable(t) => t,
            _ => panic!("not a char-table"),
        }
    }

    pub(crate) fn char_table_mut(&mut self, table: Value) -> &mut CharTable {
        match self.heap.vectorlike_mut(table) {
            Vectorlike::CharTable(t) => t,
            _ => panic!("not a char-table"),
        }
    }

    pub fn char_table_default(&self, table: Value) -> Value {
        self.char_table(table).default
    }

    pub fn is_sub_char_table(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::SubCharTable(_))
    }

    fn sub_table(&self, v: Value) -> &SubCharTable {
        match self.heap.vectorlike_ref(v) {
            Vectorlike::SubCharTable(t) => t,
            _ => panic!("not a sub-char-table"),
        }
    }

    fn sub_table_mut(&mut self, v: Value) -> &mut SubCharTable {
        match self.heap.vectorlike_mut(v) {
            Vectorlike::SubCharTable(t) => t,
            _ => panic!("not a sub-char-table"),
        }
    }

    pub fn check_char_table(&mut self, v: Value) -> LispResult<Value> {
        if self.is_char_table(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::CHAR_TABLE_P, v))
        }
    }

    pub fn make_sub_char_table(&mut self, depth: u8, min_char: u32, contents: Vec<Value>) -> Value {
        self.heap
            .vectorlike(Vectorlike::SubCharTable(Box::new(SubCharTable {
                depth,
                min_char,
                contents,
            })))
    }

    fn make_sub_char_table_filled(&mut self, depth: usize, min_char: u32, init: Value) -> Value {
        self.make_sub_char_table(depth as u8, min_char, vec![init; CHARTAB_SIZE[depth]])
    }

    pub fn make_char_table(&mut self, purpose: Value, init: Value) -> LispResult<Value> {
        let n = self.get(purpose, sym::CHAR_TABLE_EXTRA_SLOTS);
        let extras = if n.is_nil() {
            0
        } else {
            self.check_fixnat(n)? as usize
        };
        Ok(self
            .heap
            .vectorlike(Vectorlike::CharTable(Box::new(CharTable {
                purpose,
                default: init,
                parent: Value::NIL,
                ascii: init,
                contents: vec![init; CHARTAB_SIZE[0]],
                extras: vec![init; extras],
            }))))
    }

    pub fn char_table_from_slots(&mut self, slots: &[Value]) -> Option<Value> {
        if slots.len() < CHAR_TABLE_STANDARD_SLOTS {
            return None;
        }
        Some(
            self.heap
                .vectorlike(Vectorlike::CharTable(Box::new(CharTable {
                    default: slots[0],
                    parent: slots[1],
                    purpose: slots[2],
                    ascii: slots[3],
                    contents: slots[4..CHAR_TABLE_STANDARD_SLOTS].to_vec(),
                    extras: slots[CHAR_TABLE_STANDARD_SLOTS..].to_vec(),
                }))),
        )
    }

    fn char_table_ascii(&self, table: Value) -> Value {
        let sub = self.char_table(table).contents[0];
        if !self.is_sub_char_table(sub) {
            return sub;
        }
        let sub = self.sub_table(sub).contents[0];
        if !self.is_sub_char_table(sub) {
            return sub;
        }
        self.sub_table(sub).contents[0]
    }

    pub fn copy_sub_char_table(&mut self, table: Value) -> Value {
        if !self.is_sub_char_table(table) {
            return table;
        }
        let t = self.sub_table(table).clone();
        let mut contents = Vec::with_capacity(t.contents.len());
        for v in t.contents {
            contents.push(self.copy_sub_char_table(v));
        }
        self.make_sub_char_table(t.depth, t.min_char, contents)
    }

    fn sub_char_table_ref(&self, table: Value, c: u32) -> Value {
        let t = self.sub_table(table);
        let idx = chartab_idx(c, t.depth as usize, t.min_char);
        let val = t.contents[idx];
        if self.is_sub_char_table(val) {
            self.sub_char_table_ref(val, c)
        } else {
            val
        }
    }

    pub fn char_table_ref(&self, table: Value, c: u32) -> Value {
        let t = self.char_table(table);
        let mut val = if c < 128 {
            let ascii = t.ascii;
            if self.is_sub_char_table(ascii) {
                self.sub_table(ascii).contents[c as usize]
            } else {
                ascii
            }
        } else {
            let v = t.contents[chartab_idx(c, 0, 0)];
            if self.is_sub_char_table(v) {
                self.sub_char_table_ref(v, c)
            } else {
                v
            }
        };
        if val.is_nil() {
            val = t.default;
            if val.is_nil() && t.parent.is_vectorlike() && self.is_char_table(t.parent) {
                val = self.char_table_ref(t.parent, c);
            }
        }
        val
    }

    fn sub_char_table_set(&mut self, table: Value, c: u32, val: Value) {
        let (depth, min_char) = {
            let t = self.sub_table(table);
            (t.depth as usize, t.min_char)
        };
        let i = chartab_idx(c, depth, min_char);
        if depth == 3 {
            self.sub_table_mut(table).contents[i] = val;
        } else {
            let mut sub = self.sub_table(table).contents[i];
            if !self.is_sub_char_table(sub) {
                sub = self.make_sub_char_table_filled(
                    depth + 1,
                    min_char + i as u32 * CHARTAB_CHARS[depth],
                    sub,
                );
                self.sub_table_mut(table).contents[i] = sub;
            }
            self.sub_char_table_set(sub, c, val);
        }
    }

    pub fn char_table_set(&mut self, table: Value, c: u32, val: Value) {
        let ascii = self.char_table(table).ascii;
        if c < 128 && self.is_sub_char_table(ascii) {
            self.sub_table_mut(ascii).contents[c as usize] = val;
            return;
        }
        let i = chartab_idx(c, 0, 0);
        let mut sub = self.char_table(table).contents[i];
        if !self.is_sub_char_table(sub) {
            sub = self.make_sub_char_table_filled(1, i as u32 * CHARTAB_CHARS[0], sub);
            self.char_table_mut(table).contents[i] = sub;
        }
        self.sub_char_table_set(sub, c, val);
        if c < 128 {
            let a = self.char_table_ascii(table);
            self.char_table_mut(table).ascii = a;
        }
    }

    fn sub_char_table_set_range(&mut self, table: Value, from: u32, to: u32, val: Value) {
        let (depth, min_char) = {
            let t = self.sub_table(table);
            (t.depth as usize, t.min_char)
        };
        let chars_in_block = CHARTAB_CHARS[depth];
        let from = from.max(min_char);
        let mut i = chartab_idx(from, depth, min_char);
        let mut c = min_char + chars_in_block * i as u32;
        while i < CHARTAB_SIZE[depth] {
            if c > to {
                break;
            }
            if from <= c && c + chars_in_block - 1 <= to {
                self.sub_table_mut(table).contents[i] = val;
            } else {
                let mut sub = self.sub_table(table).contents[i];
                if !self.is_sub_char_table(sub) {
                    sub = self.make_sub_char_table_filled(depth + 1, c, sub);
                    self.sub_table_mut(table).contents[i] = sub;
                }
                self.sub_char_table_set_range(sub, from, to, val);
            }
            i += 1;
            c += chars_in_block;
        }
    }

    pub fn char_table_set_range(&mut self, table: Value, from: u32, to: u32, val: Value) {
        if from == to {
            self.char_table_set(table, from, val);
            return;
        }
        let lim = chartab_idx(to, 0, 0);
        let mut i = chartab_idx(from, 0, 0);
        let mut c = i as u32 * CHARTAB_CHARS[0];
        while i <= lim {
            if c > to {
                break;
            }
            if from <= c && c + CHARTAB_CHARS[0] - 1 <= to {
                self.char_table_mut(table).contents[i] = val;
            } else {
                let mut sub = self.char_table(table).contents[i];
                if !self.is_sub_char_table(sub) {
                    sub = self.make_sub_char_table_filled(1, i as u32 * CHARTAB_CHARS[0], sub);
                    self.char_table_mut(table).contents[i] = sub;
                }
                self.sub_char_table_set_range(sub, from, to, val);
            }
            i += 1;
            c += CHARTAB_CHARS[0];
        }
        if from < 128 {
            let a = self.char_table_ascii(table);
            self.char_table_mut(table).ascii = a;
        }
    }

    pub fn char_table_fill(&mut self, table: Value, val: Value) {
        let t = self.char_table_mut(table);
        t.ascii = val;
        t.contents.fill(val);
        t.default = val;
    }

    fn ref_simple(
        &self,
        table: Value,
        idx: usize,
        c: u32,
        range: &mut [u32; 2],
        default: Value,
        sub: bool,
    ) -> Value {
        let val = if sub {
            self.sub_table(table).contents[idx]
        } else {
            self.char_table(table).contents[idx]
        };
        if self.is_sub_char_table(val) {
            self.sub_ref_and_range(val, c, range, default)
        } else if val.is_nil() {
            default
        } else {
            val
        }
    }

    fn sub_ref_and_range(
        &self,
        table: Value,
        c: u32,
        range: &mut [u32; 2],
        default: Value,
    ) -> Value {
        let (depth, min_char) = {
            let t = self.sub_table(table);
            (t.depth as usize, t.min_char)
        };
        let mut chartab_idx_v = chartab_idx(c, depth, min_char);
        let val = self.ref_simple(table, chartab_idx_v, c, range, default, true);
        let mut idx = chartab_idx_v;
        while idx > 0 && range[0] < min_char + idx as u32 * CHARTAB_CHARS[depth] {
            let cc = min_char + idx as u32 * CHARTAB_CHARS[depth] - 1;
            idx -= 1;
            let this_val = self.ref_simple(table, idx, cc, range, default, true);
            if this_val != val {
                range[0] = cc + 1;
                break;
            }
        }
        loop {
            let cc = (chartab_idx_v as u32 + 1) * CHARTAB_CHARS[depth];
            if cc >= CHARTAB_CHARS[depth - 1] {
                break;
            }
            let cc = cc + min_char;
            if cc > range[1] {
                break;
            }
            chartab_idx_v += 1;
            let this_val = self.ref_simple(table, chartab_idx_v, cc, range, default, true);
            if this_val != val {
                range[1] = cc - 1;
                break;
            }
        }
        val
    }

    pub fn char_table_ref_and_range(
        &self,
        table: Value,
        c: u32,
        from: &mut u32,
        to: &mut u32,
    ) -> Value {
        let default = self.char_table(table).default;
        let mut range = [*from, *to];
        let val = self.top_ref_and_range(table, c, &mut range, default);
        *from = range[0];
        *to = range[1];
        val
    }

    fn top_ref_and_range(
        &self,
        table: Value,
        c: u32,
        range: &mut [u32; 2],
        default: Value,
    ) -> Value {
        let mut chartab_idx_v = chartab_idx(c, 0, 0);
        let val = self.ref_simple(table, chartab_idx_v, c, range, default, false);
        let mut idx = chartab_idx_v;
        while range[0] < idx as u32 * CHARTAB_CHARS[0] {
            let cc = idx as u32 * CHARTAB_CHARS[0] - 1;
            idx -= 1;
            let this_val = self.ref_simple(table, idx, cc, range, default, false);
            if this_val != val {
                range[0] = cc + 1;
                break;
            }
        }
        while range[1] >= (chartab_idx_v as u32 + 1) * CHARTAB_CHARS[0] {
            chartab_idx_v += 1;
            let cc = chartab_idx_v as u32 * CHARTAB_CHARS[0];
            let this_val = self.ref_simple(table, chartab_idx_v, cc, range, default, false);
            if this_val != val {
                range[1] = cc - 1;
                break;
            }
        }
        val
    }

    fn optimize_sub(&mut self, table: Value, test: Value) -> LispResult<Value> {
        let depth = self.sub_table(table).depth as usize;
        let mut elt = self.sub_table(table).contents[0];
        if self.is_sub_char_table(elt) {
            elt = self.optimize_sub(elt, test)?;
            self.sub_table_mut(table).contents[0] = elt;
        }
        let mut optimizable = !self.is_sub_char_table(elt);
        for i in 1..CHARTAB_SIZE[depth] {
            let mut this = self.sub_table(table).contents[i];
            if self.is_sub_char_table(this) {
                this = self.optimize_sub(this, test)?;
                self.sub_table_mut(table).contents[i] = this;
            }
            if optimizable {
                let differ = if test.is_nil() {
                    !self.equal(this, elt)?
                } else if test == sym::EQ {
                    this != elt
                } else {
                    self.funcall(test, &[this, elt])?.is_nil()
                };
                if differ {
                    optimizable = false;
                }
            }
        }
        Ok(if optimizable { elt } else { table })
    }

    pub fn map_char_table_collect(&self, table: Value) -> Vec<(u32, u32, Value)> {
        let mut out = Vec::new();
        let mut c = 0u32;
        let mut current: Option<(u32, Value)> = None;
        while c <= MAX_CHAR {
            let mut from = c;
            let mut to = MAX_CHAR;
            let mut val = self.char_table_ref_and_range(table, c, &mut from, &mut to);
            if val.is_nil() {
                let parent = self.char_table(table).parent;
                if self.is_char_table(parent) {
                    let mut pfrom = c;
                    let mut pto = to;
                    val = self.char_table_ref_and_range(parent, c, &mut pfrom, &mut pto);
                    to = to.min(pto);
                }
            }
            match current {
                Some((start, v)) if v == val => current = Some((start, v)),
                Some((start, v)) => {
                    if !v.is_nil() {
                        out.push((start, c - 1, v));
                    }
                    current = Some((c, val));
                }
                None => current = Some((c, val)),
            }
            c = to + 1;
        }
        if let Some((start, v)) = current
            && !v.is_nil()
        {
            out.push((start, MAX_CHAR, v));
        }
        out
    }
}

fn f_make_char_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let purpose = lisp.check_symbol(args[0])?;
    lisp.make_char_table(purpose, args[1])
}

fn f_char_table_subtype(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let t = lisp.check_char_table(args[0])?;
    Ok(lisp.char_table(t).purpose)
}

fn f_char_table_parent(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let t = lisp.check_char_table(args[0])?;
    Ok(lisp.char_table(t).parent)
}

fn f_set_char_table_parent(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (table, parent) = (args[0], args[1]);
    lisp.check_char_table(table)?;
    if !parent.is_nil() {
        lisp.check_char_table(parent)?;
        let mut temp = parent;
        while !temp.is_nil() {
            if temp == table {
                return Err(lisp.error("Attempt to make a chartable be its own parent"));
            }
            temp = lisp.char_table(temp).parent;
        }
    }
    lisp.char_table_mut(table).parent = parent;
    Ok(parent)
}

fn f_char_table_extra_slot(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = lisp.check_char_table(args[0])?;
    let n = lisp.check_fixnum(args[1])?;
    let extras = &lisp.char_table(table).extras;
    match usize::try_from(n).ok().and_then(|i| extras.get(i)) {
        Some(v) => Ok(*v),
        None => Err(lisp.args_out_of_range(table, args[1])),
    }
}

fn f_set_char_table_extra_slot(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = lisp.check_char_table(args[0])?;
    let n = lisp.check_fixnum(args[1])?;
    let len = lisp.char_table(table).extras.len();
    match usize::try_from(n).ok().filter(|&i| i < len) {
        Some(i) => {
            lisp.char_table_mut(table).extras[i] = args[2];
            Ok(args[2])
        }
        None => Err(lisp.args_out_of_range(table, args[1])),
    }
}

fn f_char_table_range(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (table, range) = (args[0], args[1]);
    lisp.check_char_table(table)?;
    if range.is_nil() {
        return Ok(lisp.char_table(table).default);
    }
    if let Some(c) = range.as_fixnum()
        && (0..=MAX_CHAR as i64).contains(&c)
    {
        return Ok(lisp.char_table_ref(table, c as u32));
    }
    if range.is_cons() {
        let from = lisp.check_character(lisp.xcar(range))?;
        let to = lisp.check_character(lisp.xcdr(range))?;
        let (mut f, mut t) = (from, to);
        return Ok(lisp.char_table_ref_and_range(table, from, &mut f, &mut t));
    }
    Err(lisp.error("Invalid RANGE argument to `char-table-range'"))
}

fn f_set_char_table_range(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (table, range, value) = (args[0], args[1], args[2]);
    lisp.check_char_table(table)?;
    if range == sym::T {
        let t = lisp.char_table_mut(table);
        t.ascii = value;
        t.contents.fill(value);
    } else if range.is_nil() {
        lisp.char_table_mut(table).default = value;
    } else if let Some(c) = range.as_fixnum()
        && (0..=MAX_CHAR as i64).contains(&c)
    {
        lisp.char_table_set(table, c as u32, value);
    } else if range.is_cons() {
        let from = lisp.check_character(lisp.xcar(range))?;
        let to = lisp.check_character(lisp.xcdr(range))?;
        lisp.char_table_set_range(table, from, to, value);
    } else {
        return Err(lisp.error("Invalid RANGE argument to `set-char-table-range'"));
    }
    Ok(value)
}

fn f_optimize_char_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (table, test) = (args[0], args[1]);
    lisp.check_char_table(table)?;
    for i in 0..CHARTAB_SIZE[0] {
        let elt = lisp.char_table(table).contents[i];
        if lisp.is_sub_char_table(elt) {
            let optimized = lisp.optimize_sub(elt, test)?;
            lisp.char_table_mut(table).contents[i] = optimized;
        }
    }
    let a = lisp.char_table_ascii(table);
    lisp.char_table_mut(table).ascii = a;
    Ok(Value::NIL)
}

fn f_map_char_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, table) = (args[0], args[1]);
    lisp.check_char_table(table)?;
    let ranges = lisp.map_char_table_collect(table);
    let base = lisp.stack.len();
    lisp.stack.push(table);
    lisp.stack.extend(ranges.iter().map(|r| r.2));
    for (from, to, val) in ranges {
        let key = if from == to {
            Value::fixnum(from as i64)
        } else {
            lisp.cons(Value::fixnum(from as i64), Value::fixnum(to as i64))
        };
        if let Err(e) = lisp.funcall(function, &[key, val]) {
            lisp.stack.truncate(base);
            return Err(e);
        }
    }
    lisp.stack.truncate(base);
    Ok(Value::NIL)
}

fn f_unicode_property_table_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prop = args[0];
    let alist = lisp.plain_value(sym::CHAR_CODE_PROPERTY_ALIST);
    let entry = lisp.assq(prop, alist);
    let val = lisp.cdr_safe(entry);
    if lisp.is_char_table(val) {
        return Ok(val);
    }
    Ok(Value::NIL)
}

fn f_get_unicode_property_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = lisp.check_char_table(args[0])?;
    let c = lisp.check_character(args[1])?;
    Ok(lisp.char_table_ref(table, c))
}

fn f_put_unicode_property_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = lisp.check_char_table(args[0])?;
    let c = lisp.check_character(args[1])?;
    lisp.char_table_set(table, c, args[2]);
    Ok(Value::NIL)
}
