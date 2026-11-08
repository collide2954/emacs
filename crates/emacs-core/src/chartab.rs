use crate::character::{self, MAX_CHAR};
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
    pub fn char_table(&self, table: Value) -> &CharTable {
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

    pub fn uniprop_table_p(&self, table: Value) -> bool {
        let t = self.char_table(table);
        t.purpose == sym::CHAR_CODE_PROPERTY_TABLE && t.extras.len() == 5
    }

    fn uniprop_compressed_form_p(&self, obj: Value) -> bool {
        if !obj.is_string() {
            return false;
        }
        let s = self.heap.string_ref(obj);
        s.chars > 0 && matches!(s.data[0], 1 | 2)
    }

    fn uniprop_uncompressed_contents(&self, val: Value) -> Vec<Value> {
        let mut contents = vec![Value::NIL; CHARTAB_SIZE[3]];
        let data = &self.heap.string_ref(val).data;
        let pend = data.len();
        if data[0] == 1 {
            let (start, len) = character::string_char(&data[1..]);
            let mut p = 1 + len;
            let mut idx = start as usize;
            while p < pend && idx < CHARTAB_CHARS[2] as usize {
                let (v, len) = character::string_char(&data[p..]);
                p += len;
                contents[idx] = if v > 0 {
                    Value::fixnum(i64::from(v))
                } else {
                    Value::NIL
                };
                idx += 1;
            }
        } else if data[0] == 2 {
            let mut p = 1;
            let mut idx = 0;
            while p < pend {
                let (v, len) = character::string_char(&data[p..]);
                p += len;
                let mut count = 1;
                if p < pend {
                    let (n, len) = character::string_char(&data[p..]);
                    if n >= 128 {
                        count = n - 128;
                        p += len;
                    }
                }
                while count > 0 && idx < CHARTAB_SIZE[3] {
                    contents[idx] = Value::fixnum(i64::from(v));
                    idx += 1;
                    count -= 1;
                }
            }
        }
        contents
    }

    fn uniprop_table_uncompress(&mut self, table: Value, idx: usize) -> Value {
        let (val, min_char) = {
            let t = self.sub_table(table);
            (t.contents[idx], t.min_char + CHARTAB_CHARS[2] * idx as u32)
        };
        let contents = self.uniprop_uncompressed_contents(val);
        let sub = self.make_sub_char_table(3, min_char, contents);
        self.sub_table_mut(table).contents[idx] = sub;
        sub
    }

    fn char_table_ascii(&mut self, table: Value) -> Value {
        let sub = self.char_table(table).contents[0];
        if !self.is_sub_char_table(sub) {
            return sub;
        }
        let sub = self.sub_table(sub).contents[0];
        if !self.is_sub_char_table(sub) {
            return sub;
        }
        let val = self.sub_table(sub).contents[0];
        if self.uniprop_table_p(table) && self.uniprop_compressed_form_p(val) {
            return self.uniprop_table_uncompress(sub, 0);
        }
        val
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

    pub fn copy_char_table(&mut self, table: Value) -> Value {
        let t = self.char_table(table).clone();
        let mut contents = Vec::with_capacity(t.contents.len());
        for v in t.contents {
            contents.push(self.copy_sub_char_table(v));
        }
        let copy = self
            .heap
            .vectorlike(Vectorlike::CharTable(Box::new(CharTable {
                purpose: t.purpose,
                default: t.default,
                parent: t.parent,
                ascii: Value::NIL,
                contents,
                extras: t.extras,
            })));
        let ascii = self.char_table_ascii(copy);
        self.char_table_mut(copy).ascii = ascii;
        copy
    }

    fn sub_char_table_ref(&self, table: Value, c: u32, is_uniprop: bool) -> Value {
        let t = self.sub_table(table);
        let idx = chartab_idx(c, t.depth as usize, t.min_char);
        let val = t.contents[idx];
        if is_uniprop && self.uniprop_compressed_form_p(val) {
            let min_char = t.min_char + CHARTAB_CHARS[2] * idx as u32;
            return self.uniprop_uncompressed_contents(val)[chartab_idx(c, 3, min_char)];
        }
        if self.is_sub_char_table(val) {
            self.sub_char_table_ref(val, c, is_uniprop)
        } else {
            val
        }
    }

    fn char_table_ref_1(&self, table: Value, c: u32) -> Value {
        let t = self.char_table(table);
        if c < 128 {
            let ascii = t.ascii;
            if self.is_sub_char_table(ascii) {
                self.sub_table(ascii).contents[c as usize]
            } else {
                ascii
            }
        } else {
            let v = t.contents[chartab_idx(c, 0, 0)];
            if self.is_sub_char_table(v) {
                self.sub_char_table_ref(v, c, self.uniprop_table_p(table))
            } else {
                v
            }
        }
    }

    pub fn char_table_ref(&self, table: Value, c: u32) -> Value {
        let mut val = self.char_table_ref_1(table, c);
        if val.is_nil() {
            let t = self.char_table(table);
            val = t.default;
            if val.is_nil() && t.parent.is_vectorlike() && self.is_char_table(t.parent) {
                val = self.char_table_ref(t.parent, c);
            }
        }
        val
    }

    fn sub_char_table_uncompress(&mut self, mut table: Value, c: u32) {
        loop {
            let (idx, mut val) = {
                let t = self.sub_table(table);
                let idx = chartab_idx(c, t.depth as usize, t.min_char);
                (idx, t.contents[idx])
            };
            if self.uniprop_compressed_form_p(val) {
                val = self.uniprop_table_uncompress(table, idx);
            }
            if !self.is_sub_char_table(val) {
                return;
            }
            table = val;
        }
    }

    pub fn char_table_ref_mut(&mut self, table: Value, c: u32) -> Value {
        if c >= 128 && self.uniprop_table_p(table) {
            let sub = self.char_table(table).contents[chartab_idx(c, 0, 0)];
            if self.is_sub_char_table(sub) {
                self.sub_char_table_uncompress(sub, c);
            }
        }
        let val = self.char_table_ref_1(table, c);
        if !val.is_nil() {
            return val;
        }
        let t = self.char_table(table);
        let (default, parent) = (t.default, t.parent);
        if !default.is_nil() {
            return default;
        }
        if parent.is_vectorlike() && self.is_char_table(parent) {
            return self.char_table_ref_mut(parent, c);
        }
        Value::NIL
    }

    pub fn disp_char_vector(&self, dp: Value, c: u32) -> Value {
        let t = self.char_table(dp);
        let val = if c < 128 {
            if self.is_sub_char_table(t.ascii) {
                self.sub_table(t.ascii).contents[c as usize]
            } else {
                t.ascii
            }
        } else {
            let v = t.contents[chartab_idx(c, 0, 0)];
            if self.is_sub_char_table(v) {
                self.sub_char_table_ref(v, c, self.uniprop_table_p(dp))
            } else {
                v
            }
        };
        if val.is_nil() { t.default } else { val }
    }

    fn sub_char_table_set(&mut self, table: Value, c: u32, val: Value, is_uniprop: bool) {
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
                if is_uniprop && self.uniprop_compressed_form_p(sub) {
                    sub = self.uniprop_table_uncompress(table, i);
                } else {
                    sub = self.make_sub_char_table_filled(
                        depth + 1,
                        min_char + i as u32 * CHARTAB_CHARS[depth],
                        sub,
                    );
                    self.sub_table_mut(table).contents[i] = sub;
                }
            }
            self.sub_char_table_set(sub, c, val, is_uniprop);
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
        let is_uniprop = self.uniprop_table_p(table);
        self.sub_char_table_set(sub, c, val, is_uniprop);
        if c < 128 {
            let a = self.char_table_ascii(table);
            self.char_table_mut(table).ascii = a;
        }
    }

    fn sub_char_table_set_range(
        &mut self,
        table: Value,
        from: u32,
        to: u32,
        val: Value,
        is_uniprop: bool,
    ) {
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
                    if is_uniprop && self.uniprop_compressed_form_p(sub) {
                        sub = self.uniprop_table_uncompress(table, i);
                    } else {
                        sub = self.make_sub_char_table_filled(depth + 1, c, sub);
                        self.sub_table_mut(table).contents[i] = sub;
                    }
                }
                self.sub_char_table_set_range(sub, from, to, val, is_uniprop);
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
        let is_uniprop = self.uniprop_table_p(table);
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
                self.sub_char_table_set_range(sub, from, to, val, is_uniprop);
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

    #[allow(clippy::too_many_arguments)]
    fn ref_simple(
        &mut self,
        table: Value,
        idx: usize,
        c: u32,
        range: &mut [u32; 2],
        default: Value,
        is_uniprop: bool,
        sub: bool,
    ) -> Value {
        let mut val = if sub {
            self.sub_table(table).contents[idx]
        } else {
            self.char_table(table).contents[idx]
        };
        if is_uniprop && sub && self.uniprop_compressed_form_p(val) {
            val = self.uniprop_table_uncompress(table, idx);
        }
        if self.is_sub_char_table(val) {
            self.sub_ref_and_range(val, c, range, default, is_uniprop)
        } else if val.is_nil() {
            default
        } else {
            val
        }
    }

    fn sub_ref_and_range(
        &mut self,
        table: Value,
        c: u32,
        range: &mut [u32; 2],
        default: Value,
        is_uniprop: bool,
    ) -> Value {
        let (depth, min_char) = {
            let t = self.sub_table(table);
            (t.depth as usize, t.min_char)
        };
        let mut chartab_idx_v = chartab_idx(c, depth, min_char);
        let val = self.ref_simple(table, chartab_idx_v, c, range, default, is_uniprop, true);
        let mut idx = chartab_idx_v;
        while idx > 0 && range[0] < min_char + idx as u32 * CHARTAB_CHARS[depth] {
            let cc = min_char + idx as u32 * CHARTAB_CHARS[depth] - 1;
            idx -= 1;
            let this_val = self.ref_simple(table, idx, cc, range, default, is_uniprop, true);
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
            let this_val =
                self.ref_simple(table, chartab_idx_v, cc, range, default, is_uniprop, true);
            if this_val != val {
                range[1] = cc - 1;
                break;
            }
        }
        val
    }

    pub fn char_table_ref_and_range(
        &mut self,
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
        &mut self,
        table: Value,
        c: u32,
        range: &mut [u32; 2],
        default: Value,
    ) -> Value {
        let is_uniprop = self.uniprop_table_p(table);
        let mut chartab_idx_v = chartab_idx(c, 0, 0);
        let val = self.ref_simple(table, chartab_idx_v, c, range, default, is_uniprop, false);
        let mut idx = chartab_idx_v;
        while range[0] < idx as u32 * CHARTAB_CHARS[0] {
            let cc = idx as u32 * CHARTAB_CHARS[0] - 1;
            idx -= 1;
            let this_val = self.ref_simple(table, idx, cc, range, default, is_uniprop, false);
            if this_val != val {
                range[0] = cc + 1;
                break;
            }
        }
        while range[1] >= (chartab_idx_v as u32 + 1) * CHARTAB_CHARS[0] {
            chartab_idx_v += 1;
            let cc = chartab_idx_v as u32 * CHARTAB_CHARS[0];
            let this_val =
                self.ref_simple(table, chartab_idx_v, cc, range, default, is_uniprop, false);
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

    fn uniprop_get_decoder(&self, table: Value) -> bool {
        matches!(self.char_table(table).extras[1].as_fixnum(), Some(0))
    }

    fn uniprop_decode_value_run_length(&self, table: Value, value: Value) -> Value {
        let valvec = self.char_table(table).extras[4];
        if self.is_vector(valvec) {
            let n = if value.is_nil() {
                Some(0)
            } else {
                value.as_fixnum()
            };
            if let Some(n) = n
                && let Some(&v) = usize::try_from(n)
                    .ok()
                    .and_then(|i| self.vector_items(valvec).get(i))
            {
                return v;
            }
        }
        value
    }

    fn uniprop_decode(&self, table: Value, value: Value) -> Value {
        if self.uniprop_table_p(table) && self.uniprop_get_decoder(table) {
            self.uniprop_decode_value_run_length(table, value)
        } else {
            value
        }
    }

    pub fn map_char_table(&mut self, function: Value, table: Value) -> LispResult<()> {
        let base = self.stack.len();
        let range = self.cons(Value::fixnum(0), Value::fixnum(i64::from(MAX_CHAR)));
        self.stack.push(range);
        self.stack.push(function);
        self.stack.push(table);
        let r = self.map_char_table_1(function, table, range);
        self.stack.truncate(base);
        r
    }

    fn map_char_table_1(
        &mut self,
        function: Value,
        mut table: Value,
        range: Value,
    ) -> LispResult<()> {
        let decoder = self.uniprop_table_p(table) && self.uniprop_get_decoder(table);
        let ascii = self.char_table(table).ascii;
        let mut val = if self.is_sub_char_table(ascii) {
            self.sub_table(ascii).contents[0]
        } else {
            ascii
        };
        val = self.map_sub_char_table(function, table, val, range, table)?;
        while val.is_nil() && !self.char_table(table).parent.is_nil() {
            let from = self.xcar(range).as_fixnum().unwrap_or(0);
            let parent = self.char_table(table).parent;
            let temp = self.char_table(parent).parent;
            self.char_table_mut(parent).parent = Value::NIL;
            val = self.char_table_ref_mut(parent, from as u32);
            self.char_table_mut(parent).parent = temp;
            val = self.map_sub_char_table(function, parent, val, range, parent)?;
            table = parent;
        }
        if !val.is_nil() {
            let (car, cdr) = (self.xcar(range), self.xcdr(range));
            let key = if self.eq(car, cdr) { car } else { range };
            if decoder {
                val = self.uniprop_decode_value_run_length(table, val);
            }
            self.funcall(function, &[key, val])?;
        }
        Ok(())
    }

    fn map_sub_char_table(
        &mut self,
        function: Value,
        table: Value,
        val: Value,
        range: Value,
        top: Value,
    ) -> LispResult<Value> {
        let base = self.stack.len();
        self.stack.push(table);
        self.stack.push(top);
        let r = self.map_sub_char_table_1(function, table, val, range, top);
        self.stack.truncate(base);
        r
    }

    fn map_sub_char_table_1(
        &mut self,
        function: Value,
        table: Value,
        mut val: Value,
        range: Value,
        top: Value,
    ) -> LispResult<Value> {
        let mut from = self.xcar(range).as_fixnum().unwrap_or(0);
        let to = self.xcdr(range).as_fixnum().unwrap_or(0);
        let is_uniprop = self.uniprop_table_p(top);
        let decoder = is_uniprop && self.uniprop_get_decoder(top);
        let is_sub = self.is_sub_char_table(table);
        let (depth, min_char, mut max_char) = if is_sub {
            let t = self.sub_table(table);
            let depth = t.depth as usize;
            let min_char = i64::from(t.min_char);
            (
                depth,
                min_char,
                min_char + i64::from(CHARTAB_CHARS[depth - 1]) - 1,
            )
        } else {
            (0, 0, i64::from(MAX_CHAR))
        };
        let chars_in_block = i64::from(CHARTAB_CHARS[depth]);
        if to < max_char {
            max_char = to;
        }
        let mut i = if from <= min_char {
            0
        } else {
            ((from - min_char) / chars_in_block) as usize
        };
        let mut c = min_char + chars_in_block * i as i64;
        while c <= max_char {
            let mut this = if is_sub {
                self.sub_table(table).contents[i]
            } else {
                self.char_table(table).contents[i]
            };
            let nextc = c + chars_in_block;
            if is_uniprop && is_sub && self.uniprop_compressed_form_p(this) {
                this = self.uniprop_table_uncompress(table, i);
            }
            if self.is_sub_char_table(this) {
                if to >= nextc {
                    self.heap.set_cdr(range, Value::fixnum(nextc - 1));
                }
                val = self.map_sub_char_table(function, this, val, range, top)?;
            } else {
                if this.is_nil() {
                    this = self.char_table(top).default;
                }
                if !self.eq(val, this) {
                    let mut different_value = true;
                    if val.is_nil() {
                        let parent = self.char_table(top).parent;
                        if !parent.is_nil() {
                            let temp = self.char_table(parent).parent;
                            self.char_table_mut(parent).parent = Value::NIL;
                            val = self.char_table_ref_mut(parent, from as u32);
                            self.char_table_mut(parent).parent = temp;
                            self.heap.set_cdr(range, Value::fixnum(c - 1));
                            let base = self.stack.len();
                            self.stack.push(this);
                            let r = self.map_sub_char_table(function, parent, val, range, parent);
                            self.stack.truncate(base);
                            val = r?;
                            if self.eq(val, this) {
                                different_value = false;
                            }
                        }
                    }
                    if !val.is_nil() && different_value {
                        self.heap.set_cdr(range, Value::fixnum(c - 1));
                        let (car, cdr) = (self.xcar(range), self.xcdr(range));
                        let key = if self.eq(car, cdr) { car } else { range };
                        if decoder {
                            val = self.uniprop_decode_value_run_length(top, val);
                        }
                        let base = self.stack.len();
                        self.stack.push(this);
                        self.stack.push(val);
                        let r = self.funcall(function, &[key, val]);
                        self.stack.truncate(base);
                        r?;
                    }
                    val = this;
                    from = c;
                    self.heap.set_car(range, Value::fixnum(c));
                }
            }
            self.heap.set_cdr(range, Value::fixnum(to));
            i += 1;
            c += chars_in_block;
        }
        Ok(val)
    }

    pub fn uniprop_table(&mut self, prop: Value) -> LispResult {
        let alist = self.plain_value(sym::CHAR_CODE_PROPERTY_ALIST);
        let val = self.assq(prop, alist);
        if !val.is_cons() {
            return Ok(Value::NIL);
        }
        let mut table = self.xcdr(val);
        if table.is_string() {
            let base = self.stack.len();
            self.stack.push(val);
            let count = self.specpdl.len();
            let coding = self.coding_inherit_eol_type(sym::UTF_8_EMACS, sym::UNIX);
            let result = coding.and_then(|coding| {
                self.specbind(sym::CODING_SYSTEM_FOR_READ, coding)?;
                let intl = self.string("international/");
                let file = crate::fns::f_concat(self, &[intl, table])?;
                self.save_match_data_load(file, sym::T, sym::T, sym::T, sym::T)
            });
            let result = self.unbind_to_value(count, result);
            self.stack.truncate(base);
            if result?.is_nil() {
                return Ok(Value::NIL);
            }
            table = self.xcdr(val);
        }
        if !self.is_char_table(table) || !self.uniprop_table_p(table) {
            return Ok(Value::NIL);
        }
        let decoder = self.char_table(table).extras[1];
        let invalid = match decoder.as_fixnum() {
            Some(n) => n != 0,
            None => !decoder.is_nil(),
        };
        if invalid {
            return Ok(Value::NIL);
        }
        let ascii = self.char_table_ascii(table);
        self.char_table_mut(table).ascii = ascii;
        Ok(table)
    }

    pub fn map_char_table_collect(&mut self, table: Value) -> Vec<(u32, u32, Value)> {
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
        return Ok(lisp.char_table_ref_mut(table, c as u32));
    }
    if range.is_cons() {
        let from = lisp.check_character(lisp.xcar(range))?;
        let to = lisp.check_character(lisp.xcdr(range))?;
        let (mut f, mut t) = (from, to);
        return Ok(lisp.char_table_ref_and_range(table, from, &mut f, &mut t));
    }
    Err(lisp.error("Invalid RANGE argument to `char-table-range'"))
}

pub(crate) fn f_set_char_table_range(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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
    lisp.map_char_table(function, table)?;
    Ok(Value::NIL)
}

fn f_unicode_property_table_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prop = args[0];
    let table = lisp.uniprop_table(prop)?;
    if lisp.is_char_table(table) {
        return Ok(table);
    }
    let alist = lisp.plain_value(sym::CHAR_CODE_PROPERTY_ALIST);
    let entry = crate::fns::f_assq(lisp, &[prop, alist])?;
    lisp.cdr(entry)
}

fn f_get_unicode_property_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = lisp.check_char_table(args[0])?;
    let c = lisp.check_character(args[1])?;
    if !lisp.uniprop_table_p(table) {
        return Err(lisp.error("Invalid Unicode property table"));
    }
    let val = lisp.char_table_ref_mut(table, c);
    Ok(lisp.uniprop_decode(table, val))
}

fn uniprop_encode_value_run_length(lisp: &mut Lisp, table: Value, value: Value) -> LispResult {
    let valvec = lisp.char_table(table).extras[4];
    match lisp
        .vector_items(valvec)
        .iter()
        .position(|&v| lisp.eq(v, value))
    {
        Some(i) => Ok(Value::fixnum(i as i64)),
        None => {
            let predicate = lisp.string("Unicode property value");
            Err(lisp.wrong_type(predicate, value))
        }
    }
}

fn uniprop_encode_value_numeric(lisp: &mut Lisp, table: Value, value: Value) -> LispResult {
    let valvec = lisp.char_table(table).extras[4];
    if !value.is_fixnum() {
        return Err(lisp.wrong_type(sym::FIXNUMP, value));
    }
    let items = lisp.vector_items(valvec).to_vec();
    let i = items
        .iter()
        .position(|&v| lisp.eq(v, value))
        .unwrap_or(items.len());
    let value = Value::fixnum(i as i64);
    if i == items.len() {
        let mut items = items;
        items.push(value);
        let extended = lisp.vector(items);
        lisp.char_table_mut(table).extras[4] = extended;
    }
    Ok(Value::fixnum(i as i64))
}

fn f_put_unicode_property_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = lisp.check_char_table(args[0])?;
    let c = lisp.check_character(args[1])?;
    if !lisp.uniprop_table_p(table) {
        return Err(lisp.error("Invalid Unicode property table"));
    }
    let mut value = args[2];
    match lisp.char_table(table).extras[2].as_fixnum() {
        Some(0) => {
            if !value.is_nil()
                && !matches!(value.as_fixnum(), Some(n) if (0..=i64::from(MAX_CHAR)).contains(&n))
            {
                return Err(lisp.wrong_type(sym::INTEGERP, value));
            }
        }
        Some(1) => value = uniprop_encode_value_run_length(lisp, table, value)?,
        Some(2) => value = uniprop_encode_value_numeric(lisp, table, value)?,
        _ => {}
    }
    lisp.char_table_set(table, c, value);
    Ok(Value::NIL)
}
