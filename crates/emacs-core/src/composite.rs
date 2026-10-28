use crate::character;
use crate::error::LispResult;
use crate::font::FontMetrics;
use crate::lisp::Lisp;
use crate::object::{Vectorlike, Weakness};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::terminal::OutputMethod;
use crate::value::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompositionMethod {
    Relative,
    WithRule,
    WithAltchars,
    WithRuleAltchars,
}

pub const CHECK_HEAD: i32 = 1;
pub const CHECK_TAIL: i32 = 2;
pub const CHECK_INSIDE: i32 = 4;
pub const CHECK_BORDER: i32 = CHECK_HEAD | CHECK_TAIL;
pub const CHECK_ALL: i32 = CHECK_BORDER | CHECK_INSIDE;

const MAX_AUTO_COMPOSITION_LOOKBACK: i64 = 3;

pub const LGLYPH_IX_FROM: usize = 0;
pub const LGLYPH_IX_TO: usize = 1;
pub const LGLYPH_IX_CHAR: usize = 2;
pub const LGLYPH_IX_CODE: usize = 3;
pub const LGLYPH_IX_WIDTH: usize = 4;
pub const LGLYPH_IX_LBEARING: usize = 5;
pub const LGLYPH_IX_RBEARING: usize = 6;
pub const LGLYPH_IX_ASCENT: usize = 7;
pub const LGLYPH_IX_DESCENT: usize = 8;
pub const LGLYPH_IX_ADJUSTMENT: usize = 9;
pub const LGLYPH_SIZE: usize = 10;

const ZERO_WIDTH_NON_JOINER: i64 = 0x200C;
const ZERO_WIDTH_JOINER: i64 = 0x200D;
const TAG_SPACE: i64 = 0xE0020;
const CANCEL_TAG: i64 = 0xE007F;
const UNICODE_CATEGORY_ZS: i64 = 23;

#[derive(Clone, Copy, Debug)]
pub struct CompositionIt {
    pub stop_pos: i64,
    pub id: i64,
    pub ch: i64,
    pub rule_idx: i64,
    pub lookback: i64,
    pub nglyphs: i64,
    pub reversed_p: bool,
    pub parent_w: Value,
    pub parent_f: Value,
    pub charpos: i64,
    pub nchars: i64,
    pub nbytes: i64,
    pub from: i64,
    pub to: i64,
    pub width: i64,
}

impl Default for CompositionIt {
    fn default() -> Self {
        CompositionIt {
            stop_pos: 0,
            id: 0,
            ch: 0,
            rule_idx: 0,
            lookback: 0,
            nglyphs: 0,
            reversed_p: false,
            parent_w: Value::NIL,
            parent_f: Value::NIL,
            charpos: 0,
            nchars: 0,
            nbytes: 0,
            from: 0,
            to: 0,
            width: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Composition {
    pub glyph_len: usize,
    pub pixel_width: i32,
    pub ascent: i32,
    pub descent: i32,
    pub lbearing: i32,
    pub rbearing: i32,
    pub width: i32,
    pub method: CompositionMethod,
    pub key: Value,
    pub offsets: Vec<i16>,
}

pub struct Compositions {
    pub table: Vec<Composition>,
    pub hash_table: Value,
    pub gstring_hash_table: Value,
    pub gstring_work: Value,
    pub gstring_work_headers: Value,
}

impl Default for Compositions {
    fn default() -> Self {
        Compositions {
            table: Vec::new(),
            hash_table: Value::NIL,
            gstring_hash_table: Value::NIL,
            gstring_work: Value::NIL,
            gstring_work_headers: Value::NIL,
        }
    }
}

impl Compositions {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend([
            self.hash_table,
            self.gstring_hash_table,
            self.gstring_work,
            self.gstring_work_headers,
        ]);
        out.extend(self.table.iter().map(|c| c.key));
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("compose-region-internal", 2, 4, f_compose_region_internal),
    Subr::fixed("compose-string-internal", 3, 5, f_compose_string_internal),
    Subr::fixed(
        "find-composition-internal",
        4,
        4,
        f_find_composition_internal,
    ),
    Subr::fixed("composition-get-gstring", 4, 4, f_composition_get_gstring),
    Subr::fixed("clear-composition-cache", 0, 0, f_clear_composition_cache),
    Subr::fixed("composition-sort-rules", 1, 1, f_composition_sort_rules),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.compositions.hash_table = lisp.make_hash_table_with(sym::EQUAL, Weakness::None);
    lisp.compositions.gstring_hash_table = lisp.make_hash_table_with(sym::EQUAL, Weakness::None);
    let headers: Vec<Value> = (0..8)
        .map(|i| lisp.vector(vec![Value::NIL; i + 2]))
        .collect();
    lisp.compositions.gstring_work_headers = lisp.vector(headers);
    lisp.compositions.gstring_work = lisp.vector(vec![Value::NIL; 10]);
    let cell = lisp.cons(sym::COMPOSITION, sym::T);
    let old = lisp.find_symbol_value(sym::TEXT_PROPERTY_DEFAULT_NONSTICKY);
    let nonsticky = lisp.cons(cell, old);
    lisp.set_plain_value(sym::TEXT_PROPERTY_DEFAULT_NONSTICKY, nonsticky);
    let compose = lisp.intern("compose-chars-after");
    lisp.defvar(sym::COMPOSE_CHARS_AFTER_FUNCTION, compose);
    lisp.defvar(sym::AUTO_COMPOSITION_MODE, sym::T);
    lisp.defvar(sym::AUTO_COMPOSITION_FUNCTION, Value::NIL);
    let table = lisp
        .make_char_table(Value::NIL, Value::NIL)
        .expect("composition function table");
    lisp.defvar(sym::COMPOSITION_FUNCTION_TABLE, table);
    lisp.defvar(sym::AUTO_COMPOSITION_EMOJI_ELIGIBLE_CODEPOINTS, Value::NIL);
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

impl Lisp {
    fn aref_items(&self, v: Value) -> &[Value] {
        if self.is_vector(v) {
            self.vector_items(v)
        } else {
            &[]
        }
    }

    pub(crate) fn vector_set(&mut self, v: Value, i: usize, val: Value) {
        if let Vectorlike::Vector(items) = self.heap.vectorlike_mut(v) {
            items[i] = val;
        }
    }

    pub fn composition_registered_p(&self, prop: Value) -> bool {
        self.xcar(prop).is_fixnum()
    }

    fn composition_length(&self, prop: Value) -> Option<i64> {
        if self.composition_registered_p(prop) {
            self.car_safe(self.xcdr(prop)).as_fixnum()
        } else {
            self.car_safe(self.xcar(prop)).as_fixnum()
        }
    }

    pub fn composition_components(&self, prop: Value) -> Value {
        if self.composition_registered_p(prop) {
            self.car_safe(self.cdr_safe(self.xcdr(prop)))
        } else {
            self.cdr_safe(self.xcar(prop))
        }
    }

    pub fn composition_modification_func(&self, prop: Value) -> Value {
        if self.composition_registered_p(prop) {
            self.cdr_safe(self.cdr_safe(self.xcdr(prop)))
        } else if prop.is_cons() {
            self.xcdr(prop)
        } else {
            Value::NIL
        }
    }

    pub fn composition_method(&self, prop: Value) -> CompositionMethod {
        if self.composition_registered_p(prop) {
            let id = self.xcar(prop).as_fixnum().unwrap_or(0) as usize;
            return self.compositions.table[id].method;
        }
        let temp = self.cdr_safe(self.xcar(prop));
        if temp.is_nil() {
            CompositionMethod::Relative
        } else if temp.is_fixnum() || temp.is_string() {
            CompositionMethod::WithAltchars
        } else {
            CompositionMethod::WithRuleAltchars
        }
    }

    pub fn composition_valid_p(&self, start: usize, end: usize, prop: Value) -> bool {
        if !prop.is_cons() {
            return false;
        }
        let ok = if self.composition_registered_p(prop) {
            let id = self.xcar(prop).as_fixnum().unwrap_or(-1);
            id >= 0 && id <= self.compositions.table.len() as i64 && self.xcdr(prop).is_cons()
        } else {
            let car = self.xcar(prop);
            car.is_cons() && {
                let c = self.xcdr(car);
                c.is_nil() || c.is_string() || self.is_vector(c) || c.is_fixnum() || c.is_cons()
            }
        };
        ok && self.composition_length(prop) == Some(end as i64 - start as i64)
    }

    fn character_width(&self, ch: i64) -> i64 {
        if (0..=character::MAX_CHAR as i64).contains(&ch) {
            self.char_width_of(ch as u32) as i64
        } else {
            1
        }
    }

    pub fn get_composition_id(
        &mut self,
        charpos: usize,
        nchars: usize,
        prop: Value,
        string: Value,
    ) -> LispResult<i64> {
        if nchars == 0 || !prop.is_cons() {
            return Ok(-1);
        }
        let id = self.xcar(prop);
        if let Some(n) = id.as_fixnum() {
            if n < 0 || n >= self.compositions.table.len() as i64 {
                return Ok(-1);
            }
            return Ok(n);
        }
        if !id.is_cons() {
            return Ok(-1);
        }
        let length = self.xcar(id);
        if length.as_fixnum() != Some(nchars as i64) {
            return Ok(-1);
        }
        let components = self.xcdr(id);
        let key = if components.is_fixnum() {
            self.vector(vec![components])
        } else if components.is_string() || components.is_cons() {
            crate::fns::f_vconcat(self, &[components])?
        } else if self.is_vector(components) {
            components
        } else if components.is_nil() {
            let chars: Vec<Value> = if string.is_string() {
                let all = self.string_char_vec(string);
                all[charpos..charpos + nchars]
                    .iter()
                    .map(|&c| Value::fixnum(i64::from(c)))
                    .collect()
            } else {
                (charpos..charpos + nchars)
                    .map(|p| Value::fixnum(i64::from(self.char_at_pos(p))))
                    .collect()
            };
            self.vector(chars)
        } else {
            return Ok(-1);
        };
        let base = self.stack.len();
        self.stack.push(key);
        let r = self.register_composition(prop, nchars, components, key);
        self.stack.truncate(base);
        r
    }

    fn register_composition(
        &mut self,
        prop: Value,
        nchars: usize,
        components: Value,
        key: Value,
    ) -> LispResult<i64> {
        let table = self.compositions.hash_table;
        if let Some(slot) = self.hash_lookup(table, key)? {
            let (key, id) = self.hash_table(table).entries[slot].unwrap_or((key, Value::NIL));
            self.heap.set_car(prop, id);
            let rest = self.xcdr(prop);
            let tail = self.cons(key, rest);
            let cell = self.cons(Value::fixnum(nchars as i64), tail);
            self.heap.set_cdr(prop, cell);
            return Ok(id.as_fixnum().unwrap_or(-1));
        }
        let key_contents = self.vector_items(key).to_vec();
        if self.is_vector(components)
            && self.vector_items(components).len() >= 2
            && self.is_vector(self.vector_items(components)[0])
        {
            if key_contents.iter().skip(1).any(|&v| !self.is_vector(v)) {
                return Ok(-1);
            }
        } else if (self.is_vector(components) || components.is_cons())
            && (key_contents.len().is_multiple_of(2) || key_contents.iter().any(|v| !v.is_fixnum()))
        {
            return Ok(-1);
        }
        let n = self.compositions.table.len() as i64;
        let id = Value::fixnum(n);
        self.heap.set_car(prop, id);
        let rest = self.xcdr(prop);
        let tail = self.cons(key, rest);
        let cell = self.cons(Value::fixnum(nchars as i64), tail);
        self.heap.set_cdr(prop, cell);
        self.hash_put(table, key, id)?;
        let method = if components.is_nil() {
            CompositionMethod::Relative
        } else if components.is_fixnum() || components.is_string() {
            CompositionMethod::WithAltchars
        } else {
            CompositionMethod::WithRuleAltchars
        };
        let glyph_len = if method == CompositionMethod::WithRuleAltchars {
            key_contents.len().div_ceil(2)
        } else {
            key_contents.len()
        };
        let ch_at = |i: usize| key_contents.get(i).and_then(|v| v.as_fixnum()).unwrap_or(0);
        let width = if method != CompositionMethod::WithRuleAltchars {
            let mut width = 0;
            for i in 0..glyph_len {
                let ch = ch_at(i);
                let this_width = if ch == 9 { 1 } else { self.character_width(ch) };
                width = width.max(this_width);
            }
            width as i32
        } else {
            let mut leftmost = 0.0f64;
            let ch = ch_at(0);
            let mut rightmost = if ch != 9 { self.character_width(ch) } else { 1 } as f64;
            let mut i = 1;
            while i < glyph_len {
                let mut rule = ch_at(i);
                let ch = ch_at(i + 1);
                let this_width = if ch != 9 { self.character_width(ch) } else { 1 } as f64;
                rule &= 0xFF;
                let mut gref = rule / 12;
                if gref > 12 {
                    gref = 11;
                }
                let nref = rule % 12;
                let this_left = leftmost + (gref % 3) as f64 * (rightmost - leftmost) / 2.0
                    - (nref % 3) as f64 * this_width / 2.0;
                if this_left < leftmost {
                    leftmost = this_left;
                }
                if this_left + this_width > rightmost {
                    rightmost = this_left + this_width;
                }
                i += 2;
            }
            let span = rightmost - leftmost;
            let mut width = span as i32;
            if f64::from(width) < span {
                width += 1;
            }
            width
        };
        self.compositions.table.push(Composition {
            glyph_len,
            pixel_width: 0,
            ascent: 0,
            descent: 0,
            lbearing: 0,
            rbearing: 0,
            width,
            method,
            key,
            offsets: vec![0; glyph_len * 2],
        });
        Ok(n)
    }

    pub fn find_composition(
        &mut self,
        pos: i64,
        limit: i64,
        object: Value,
    ) -> LispResult<Option<(usize, usize, Value)>> {
        if let Some((prop, start, end)) =
            self.get_property_and_range(pos, sym::COMPOSITION, object)?
        {
            return Ok(Some((start, end, prop)));
        }
        if limit < 0 || limit == pos {
            return Ok(None);
        }
        let mut pos = pos;
        if limit > pos {
            let val = crate::textprop::f_next_single_property_change(
                self,
                &[
                    Value::fixnum(pos),
                    sym::COMPOSITION,
                    object,
                    Value::fixnum(limit),
                ],
            )?;
            pos = val.as_fixnum().unwrap_or(limit);
            if pos == limit {
                return Ok(None);
            }
        } else {
            if let Some((prop, start, end)) =
                self.get_property_and_range(pos - 1, sym::COMPOSITION, object)?
            {
                return Ok(Some((start, end, prop)));
            }
            let val = crate::textprop::f_previous_single_property_change(
                self,
                &[
                    Value::fixnum(pos),
                    sym::COMPOSITION,
                    object,
                    Value::fixnum(limit),
                ],
            )?;
            pos = val.as_fixnum().unwrap_or(limit);
            if pos == limit {
                return Ok(None);
            }
            pos -= 1;
        }
        let found = self.get_property_and_range(pos, sym::COMPOSITION, object)?;
        Ok(Some(match found {
            Some((prop, start, end)) => (start, end, prop),
            None => (pos as usize, pos as usize, Value::NIL),
        }))
    }

    fn run_composition_function(&mut self, from: usize, to: usize, prop: Value) -> LispResult<()> {
        let func = self.composition_modification_func(prop);
        let base = self.stack.len();
        self.stack.push(func);
        let r = self.run_composition_function_1(from, to, func);
        self.stack.truncate(base);
        r
    }

    fn run_composition_function_1(
        &mut self,
        from: usize,
        to: usize,
        func: Value,
    ) -> LispResult<()> {
        let mut from = from;
        let mut to = to;
        if from > self.cur().begv
            && let Some((start, end, prop)) =
                self.find_composition(from as i64 - 1, -1, Value::NIL)?
            && !self.composition_valid_p(start, end, prop)
        {
            from = start;
        }
        if to < self.cur().zv
            && let Some((start, end, prop)) = self.find_composition(to as i64, -1, Value::NIL)?
            && !self.composition_valid_p(start, end, prop)
        {
            to = end;
        }
        self.check_symbol(func)?;
        if !self.heap.symbol_ref(func).function.is_nil() {
            self.funcall(
                func,
                &[Value::fixnum(from as i64), Value::fixnum(to as i64)],
            )?;
        }
        Ok(())
    }

    fn put_composition_copy(&mut self, start: usize, end: usize, prop: Value) -> LispResult<()> {
        let (car, cdr) = (self.xcar(prop), self.xcdr(prop));
        let copy = self.cons(car, cdr);
        crate::textprop::f_put_text_property(
            self,
            &[
                Value::fixnum(start as i64),
                Value::fixnum(end as i64),
                sym::COMPOSITION,
                copy,
                Value::NIL,
            ],
        )?;
        Ok(())
    }

    pub fn update_compositions(
        &mut self,
        from: usize,
        to: usize,
        check_mask: i32,
    ) -> LispResult<()> {
        if self.var_true(sym::INHIBIT_MODIFICATION_HOOKS) {
            return Ok(());
        }
        let (begv, zv) = (self.cur().begv, self.cur().zv);
        if !(begv <= from && from <= to && to <= zv) {
            return Ok(());
        }
        let mut from = from;
        let mut min_pos = from;
        let mut max_pos = to;
        if check_mask & CHECK_HEAD != 0 {
            let before = if from > self.cur().begv {
                self.find_composition(from as i64 - 1, -1, Value::NIL)?
            } else {
                None
            };
            match before {
                Some((start, end, prop)) if self.composition_valid_p(start, end, prop) => {
                    min_pos = start;
                    if end > to {
                        max_pos = end;
                    }
                    if from < end {
                        self.put_composition_copy(from, end, prop)?;
                    }
                    self.run_composition_function(start, end, prop)?;
                    from = end;
                }
                _ => {
                    if from < self.cur().zv
                        && let Some((start, end, prop)) =
                            self.find_composition(from as i64, -1, Value::NIL)?
                    {
                        from = end;
                        if self.composition_valid_p(start, from, prop) {
                            if from > to {
                                max_pos = from;
                            }
                            self.run_composition_function(start, from, prop)?;
                        }
                    }
                }
            }
        }
        if check_mask & CHECK_INSIDE != 0 {
            while (from as i64) < to as i64 - 1 {
                let Some((start, end, prop)) =
                    self.find_composition(from as i64, to as i64, Value::NIL)?
                else {
                    break;
                };
                from = end;
                if !(self.composition_valid_p(start, from, prop) && (from as i64) < to as i64 - 1) {
                    break;
                }
                self.run_composition_function(start, from, prop)?;
            }
        }
        if check_mask & CHECK_TAIL != 0 {
            let before = if from < to {
                self.find_composition(to as i64 - 1, -1, Value::NIL)?
            } else {
                None
            };
            match before {
                Some((start, end, prop)) if self.composition_valid_p(start, end, prop) => {
                    if to < end {
                        self.put_composition_copy(start, to, prop)?;
                        max_pos = end;
                    }
                    self.run_composition_function(start, end, prop)?;
                }
                _ => {
                    if to < self.cur().zv
                        && let Some((start, end, prop)) =
                            self.find_composition(to as i64, -1, Value::NIL)?
                        && self.composition_valid_p(start, end, prop)
                    {
                        self.run_composition_function(start, end, prop)?;
                        max_pos = end;
                    }
                }
            }
        }
        if min_pos < max_pos {
            let count = self.specpdl.len();
            self.specbind(sym::INHIBIT_READ_ONLY, sym::T)?;
            self.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
            self.specbind(sym::INHIBIT_POINT_MOTION_HOOKS, sym::T)?;
            let props = self.list(&[sym::AUTO_COMPOSED]);
            let r = crate::textprop::f_remove_list_of_text_properties(
                self,
                &[
                    Value::fixnum(min_pos as i64),
                    Value::fixnum(max_pos as i64),
                    props,
                    Value::NIL,
                ],
            );
            self.unbind_to_value(count, r)?;
        }
        Ok(())
    }

    pub fn make_composition_value_copy(&mut self, list: Value) {
        let mut list = list;
        while list.is_cons() {
            let interval = self.xcar(list);
            let mut plist = self.car_safe(self.cdr_safe(self.cdr_safe(interval)));
            while plist.is_cons() && self.xcdr(plist).is_cons() {
                let next = self.xcdr(plist);
                let val = self.xcar(next);
                if self.xcar(plist) == sym::COMPOSITION && val.is_cons() {
                    let (car, cdr) = (self.xcar(val), self.xcdr(val));
                    let copy = self.cons(car, cdr);
                    self.heap.set_car(next, copy);
                }
                plist = self.xcdr(next);
            }
            list = self.xcdr(list);
        }
    }

    pub fn compose_text(
        &mut self,
        start: usize,
        end: usize,
        components: Value,
        modification_func: Value,
        string: Value,
    ) -> LispResult<()> {
        let head = self.cons(Value::fixnum((end - start) as i64), components);
        let prop = self.cons(head, modification_func);
        crate::textprop::f_put_text_property(
            self,
            &[
                Value::fixnum(start as i64),
                Value::fixnum(end as i64),
                sym::COMPOSITION,
                prop,
                string,
            ],
        )?;
        Ok(())
    }

    pub fn lgstring_header(&self, gstring: Value) -> Value {
        self.aref_items(gstring)
            .first()
            .copied()
            .unwrap_or(Value::NIL)
    }

    pub fn lgstring_font(&self, gstring: Value) -> Value {
        let header = self.lgstring_header(gstring);
        self.aref_items(header)
            .first()
            .copied()
            .unwrap_or(Value::NIL)
    }

    pub fn lgstring_char_len(&self, gstring: Value) -> usize {
        let header = self.lgstring_header(gstring);
        self.aref_items(header).len().saturating_sub(1)
    }

    pub fn lgstring_id(&self, gstring: Value) -> Value {
        self.aref_items(gstring)
            .get(1)
            .copied()
            .unwrap_or(Value::NIL)
    }

    pub fn lgstring_glyph_len(&self, gstring: Value) -> usize {
        self.aref_items(gstring).len().saturating_sub(2)
    }

    pub fn lgstring_glyph(&self, gstring: Value, idx: usize) -> Value {
        self.aref_items(gstring)
            .get(idx + 2)
            .copied()
            .unwrap_or(Value::NIL)
    }

    fn lglyph_int(&self, g: Value, idx: usize) -> i64 {
        self.aref_items(g)
            .get(idx)
            .and_then(|v| v.as_fixnum())
            .unwrap_or(0)
    }

    fn lglyph_adjustment(&self, g: Value) -> Value {
        self.aref_items(g)
            .get(LGLYPH_IX_ADJUSTMENT)
            .copied()
            .unwrap_or(Value::NIL)
    }

    fn lglyph_adjust_part(&self, g: Value, idx: usize) -> i64 {
        let adj = self.lglyph_adjustment(g);
        if self.is_vector(adj) {
            self.lglyph_int(adj, idx)
        } else {
            0
        }
    }

    pub fn composition_gstring_lookup_cache(&mut self, header: Value) -> LispResult {
        let table = self.compositions.gstring_hash_table;
        self.gethash(header, table, Value::NIL)
    }

    pub fn composition_gstring_put_cache(&mut self, gstring: Value, len: i64) -> LispResult {
        let header = self.lgstring_header(gstring);
        let len = if len < 0 {
            let glyph_len = self.lgstring_glyph_len(gstring);
            (0..glyph_len)
                .find(|&i| self.lgstring_glyph(gstring, i).is_nil())
                .unwrap_or(glyph_len)
        } else {
            len as usize
        };
        let base = self.stack.len();
        let copy = self.vector(vec![Value::NIL; len + 2]);
        self.stack.push(copy);
        let header_copy = crate::fns::f_copy_sequence(self, &[header])?;
        self.vector_set(copy, 0, header_copy);
        for i in 0..len {
            let g = self.lgstring_glyph(gstring, i);
            let g = crate::fns::f_copy_sequence(self, &[g])?;
            self.vector_set(copy, i + 2, g);
        }
        let table = self.compositions.gstring_hash_table;
        let id = self.hash_put(table, header_copy, copy)?;
        self.vector_set(copy, 1, Value::fixnum(id as i64));
        self.stack.truncate(base);
        Ok(copy)
    }

    pub fn composition_gstring_from_id(&self, id: usize) -> Value {
        let table = self.compositions.gstring_hash_table;
        self.hash_table(table)
            .entries
            .get(id)
            .copied()
            .flatten()
            .map_or(Value::NIL, |(_, v)| v)
    }

    pub fn composition_gstring_cache_clear_font(&mut self, font_object: Value) {
        let table = self.compositions.gstring_hash_table;
        let slots: Vec<usize> = self
            .hash_table(table)
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                e.filter(|&(_, g)| self.lgstring_font(g) == font_object)
                    .map(|_| i)
            })
            .collect();
        for slot in slots {
            self.hash_remove_slot(table, slot);
        }
    }

    pub fn composition_gstring_p(&mut self, gstring: Value) -> LispResult<bool> {
        if !self.is_vector(gstring) || self.vector_items(gstring).len() < 2 {
            return Ok(false);
        }
        let header = self.lgstring_header(gstring);
        if !self.is_vector(header) || self.vector_items(header).len() < 2 {
            return Ok(false);
        }
        let font = self.lgstring_font(gstring);
        if !font.is_nil() && !self.font_object_p(font) && !self.coding_system_p(font)? {
            return Ok(false);
        }
        if self
            .vector_items(header)
            .iter()
            .skip(1)
            .any(|v| !v.as_fixnum().is_some_and(|n| n >= 0))
        {
            return Ok(false);
        }
        let id = self.lgstring_id(gstring);
        if !id.is_nil() && !id.as_fixnum().is_some_and(|n| n >= 0) {
            return Ok(false);
        }
        for i in 0..self.lgstring_glyph_len(gstring) {
            let glyph = self.lgstring_glyph(gstring, i);
            if glyph.is_nil() {
                break;
            }
            if !self.is_vector(glyph) || self.vector_items(glyph).len() != LGLYPH_SIZE {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn composition_gstring_width(
        &self,
        gstring: Value,
        from: usize,
        to: usize,
        mut metrics: Option<&mut FontMetrics>,
    ) -> i32 {
        if let Some(m) = metrics.as_deref_mut() {
            let font_object = self.lgstring_font(gstring);
            if self.font_object_p(font_object) {
                unreachable!("font metrics need a font driver");
            }
            m.ascent = 1;
            m.descent = 0;
            m.width = 0;
            m.lbearing = 0;
            m.rbearing = 0;
        }
        let mut width = 0i64;
        for i in from..to {
            let glyph = self.lgstring_glyph(gstring, i);
            if self.lglyph_adjustment(glyph).is_nil() {
                width += self.lglyph_int(glyph, LGLYPH_IX_WIDTH);
            } else {
                width += self.lglyph_adjust_part(glyph, 2);
            }
            if let Some(m) = metrics.as_deref_mut() {
                let xoff = self.lglyph_adjust_part(glyph, 0);
                let yoff = self.lglyph_adjust_part(glyph, 1);
                let x = i64::from(m.width) + self.lglyph_int(glyph, LGLYPH_IX_LBEARING) + xoff;
                if i64::from(m.lbearing) > x {
                    m.lbearing = x as i32;
                }
                let x = i64::from(m.width) + self.lglyph_int(glyph, LGLYPH_IX_RBEARING) + xoff;
                if i64::from(m.rbearing) < x {
                    m.rbearing = x as i32;
                }
                m.width = width as i32;
                let x = self.lglyph_int(glyph, LGLYPH_IX_ASCENT) - yoff;
                if i64::from(m.ascent) < x {
                    m.ascent = x as i32;
                }
                let x = self.lglyph_int(glyph, LGLYPH_IX_DESCENT) + yoff;
                if i64::from(m.descent) < x {
                    m.descent = x as i32;
                }
            }
        }
        width as i32
    }

    pub fn composition_gstring_adjust_zero_width(&mut self, gstring: Value) {
        let mut from = 0i64;
        let mut width = 0i64;
        let mut i = 0usize;
        loop {
            let glyph = if i < self.lgstring_glyph_len(gstring) {
                self.lgstring_glyph(gstring, i)
            } else {
                Value::NIL
            };
            if glyph.is_nil() || from != self.lglyph_int(glyph, LGLYPH_IX_FROM) {
                let last = self.lgstring_glyph(gstring, i.wrapping_sub(1));
                if width == 0 {
                    let adj = self.lglyph_adjustment(last);
                    if adj.is_nil() {
                        let w = self.lglyph_int(last, LGLYPH_IX_WIDTH) + 1;
                        let v =
                            self.vector(vec![Value::fixnum(0), Value::fixnum(0), Value::fixnum(w)]);
                        self.vector_set(last, LGLYPH_IX_ADJUSTMENT, v);
                    } else {
                        let w = self.lglyph_adjust_part(last, 2) + 1;
                        self.vector_set(adj, 2, Value::fixnum(w));
                    }
                }
                if glyph.is_nil() {
                    break;
                }
                from = self.lglyph_int(glyph, LGLYPH_IX_FROM);
                width = 0;
            }
            width += if self.lglyph_adjustment(glyph).is_nil() {
                self.lglyph_int(glyph, LGLYPH_IX_WIDTH)
            } else {
                self.lglyph_adjust_part(glyph, 2)
            };
            i += 1;
        }
    }

    fn text_char(&self, pos: usize, string: Value) -> u32 {
        if string.is_string() {
            let chars = self.string_char_vec(string);
            chars.get(pos).copied().unwrap_or(0)
        } else if pos < self.cur().z() {
            self.char_at_pos(pos)
        } else {
            0
        }
    }

    fn fill_gstring_header(
        &mut self,
        from: usize,
        to: usize,
        font_object: Value,
        string: Value,
    ) -> LispResult {
        let len = to - from;
        if len == 0 {
            return Err(self.error("Attempt to shape zero-length text"));
        }
        let header = if len <= 8 {
            self.vector_items(self.compositions.gstring_work_headers)[len - 1]
        } else {
            self.vector(vec![Value::NIL; len + 1])
        };
        self.vector_set(header, 0, font_object);
        let chars: Vec<u32> = if string.is_string() {
            self.string_char_vec(string)[from..to].to_vec()
        } else {
            (from..to).map(|p| self.char_at_pos(p)).collect()
        };
        for (i, c) in chars.into_iter().enumerate() {
            self.vector_set(header, i + 1, Value::fixnum(i64::from(c)));
        }
        Ok(header)
    }

    fn fill_gstring_body(&mut self, gstring: Value) {
        let font_object = self.lgstring_font(gstring);
        if self.font_object_p(font_object) {
            unreachable!("shaping with a font object needs a font driver");
        }
        let header = self.lgstring_header(gstring);
        let len = self.lgstring_char_len(gstring);
        let width_table = self.bound_var(sym::CHAR_WIDTH_TABLE);
        for i in 0..len {
            let mut g = self.lgstring_glyph(gstring, i);
            let c = self.vector_items(header)[i + 1].as_fixnum().unwrap_or(0);
            if g.is_nil() {
                g = self.vector(vec![Value::NIL; LGLYPH_SIZE]);
                self.vector_set(gstring, i + 2, g);
            }
            let width = if self.is_char_table(width_table) {
                self.char_table_ref(width_table, c as u32)
                    .as_fixnum()
                    .unwrap_or(0)
            } else {
                0
            };
            for (idx, v) in [
                (LGLYPH_IX_FROM, i as i64),
                (LGLYPH_IX_TO, i as i64),
                (LGLYPH_IX_CHAR, c),
                (LGLYPH_IX_CODE, c),
                (LGLYPH_IX_LBEARING, 0),
                (LGLYPH_IX_RBEARING, width),
                (LGLYPH_IX_WIDTH, width),
                (LGLYPH_IX_ASCENT, 1),
                (LGLYPH_IX_DESCENT, 0),
            ] {
                self.vector_set(g, idx, Value::fixnum(v));
            }
            self.vector_set(g, LGLYPH_IX_ADJUSTMENT, Value::NIL);
        }
        let glyph_len = self.lgstring_glyph_len(gstring);
        for i in len..glyph_len {
            self.vector_set(gstring, i + 2, Value::NIL);
        }
    }

    pub fn composition_get_gstring(
        &mut self,
        from: Value,
        to: Value,
        font_object: Value,
        string: Value,
    ) -> LispResult {
        let mut font_object = font_object;
        if !self.font_object_p(font_object) {
            let terminal = self.decode_live_terminal(font_object)?;
            let id = {
                let coding = &self.terminal(terminal).terminal_coding;
                if coding.common_flags & crate::coding::REQUIRE_ENCODING_MASK != 0 {
                    coding.id
                } else {
                    self.codings.safe_terminal.id
                }
            };
            font_object = self.coding_id_name(id);
        }
        let mut string = string;
        let (frompos, topos) = if string.is_nil() {
            if !self.cur().text.multibyte {
                return Err(self.error("Attempt to shape unibyte text"));
            }
            self.validate_region(from, to)?
        } else {
            self.check_string(string)?;
            let chars = self.string_chars(string);
            let (f, t) = self.validate_subarray(string, from, to, chars)?;
            if !self.string_multibyte(string) {
                if self.string_bytes(string).iter().any(|&b| b >= 0x80) {
                    return Err(self.error("Attempt to shape unibyte text"));
                }
                let bytes = self.string_bytes(string).to_vec();
                string = self.make_specified_string(bytes, chars, true);
            }
            (f, t)
        };
        let base = self.stack.len();
        self.stack.push(string);
        self.stack.push(font_object);
        let r = self.composition_get_gstring_1(frompos, topos, font_object, string);
        self.stack.truncate(base);
        r
    }

    fn composition_get_gstring_1(
        &mut self,
        frompos: usize,
        topos: usize,
        font_object: Value,
        string: Value,
    ) -> LispResult {
        let header = self.fill_gstring_header(frompos, topos, font_object, string)?;
        let gstring = self.composition_gstring_lookup_cache(header)?;
        if !gstring.is_nil() {
            return Ok(gstring);
        }
        if self.lgstring_glyph_len(self.compositions.gstring_work) < topos - frompos {
            self.compositions.gstring_work = self.vector(vec![Value::NIL; topos - frompos + 2]);
        }
        let work = self.compositions.gstring_work;
        self.vector_set(work, 0, header);
        self.vector_set(work, 1, Value::NIL);
        self.fill_gstring_body(work);
        Ok(work)
    }

    fn char_composable_p(&self, c: i64) -> bool {
        if c < 32 {
            return false;
        }
        if c == ZERO_WIDTH_NON_JOINER
            || c == ZERO_WIDTH_JOINER
            || (TAG_SPACE..=CANCEL_TAG).contains(&c)
        {
            return true;
        }
        let table = self.bound_var(sym::UNICODE_CATEGORY_TABLE);
        self.is_char_table(table)
            && self
                .char_table_ref(table, c as u32)
                .as_fixnum()
                .is_some_and(|v| v <= UNICODE_CATEGORY_ZS)
    }

    fn tty_type_name(&self) -> Option<String> {
        let f = self.frames.selected_frame;
        match self.frame(f).output_method {
            OutputMethod::Initial => None,
        }
    }

    pub fn inhibit_auto_composition(&self) -> bool {
        let mode = self.bound_var(sym::AUTO_COMPOSITION_MODE);
        if mode.is_nil() {
            return true;
        }
        if mode.is_string()
            && let Some(name) = self.tty_type_name()
            && self.string_to_rust(mode) == name
        {
            return true;
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn autocmp_chars(
        &mut self,
        rule: Value,
        charpos: usize,
        limit: usize,
        win: Value,
        string: Value,
        direction: Value,
        ch: i64,
    ) -> LispResult {
        let count = self.specpdl.len();
        let r = self.autocmp_chars_1(rule, charpos, limit, win, string, direction, ch);
        self.unbind_to_value(count, r)
    }

    #[allow(clippy::too_many_arguments)]
    fn autocmp_chars_1(
        &mut self,
        rule: Value,
        charpos: usize,
        limit: usize,
        win: Value,
        string: Value,
        direction: Value,
        ch: i64,
    ) -> LispResult {
        let _ = ch;
        let pos = Value::fixnum(charpos as i64);
        let (pt, buffer) = (self.cur().pt, self.current_buffer);
        self.record_unwind_save_match_data()?;
        let re = self.aref_items(rule).first().copied().unwrap_or(Value::NIL);
        let len = if re.is_nil() {
            1
        } else if !re.is_string() {
            return Ok(Value::NIL);
        } else {
            let len = self.fast_looking_at(re, charpos, limit, string)?;
            if len > 0 {
                if string.is_nil() {
                    let start_byte = self.cur().text.char_to_byte(charpos);
                    self.cur().text.byte_to_char(start_byte + len as usize) as isize
                        - charpos as isize
                } else {
                    let start_byte = self.string_char_to_byte_index(string, charpos);
                    self.string_byte_to_char_index(string, start_byte + len as usize) as isize
                        - charpos as isize
                }
            } else {
                len
            }
        };
        if len <= 0 {
            return Ok(Value::NIL);
        }
        let to = charpos + len as usize;
        let font_object = self.window(win).frame;
        if self.frame(font_object).window_p() {
            unreachable!("window-system frames need font_range");
        }
        let mut lgstring =
            self.composition_get_gstring(pos, Value::fixnum(to as i64), font_object, string)?;
        if self.lgstring_id(lgstring).is_nil() {
            if string.is_nil() {
                let marker = self.make_marker_at(pt, buffer, false);
                self.record_unwind(crate::fileio::restore_point_unwind, marker);
            }
            let function = self.bound_var(sym::AUTO_COMPOSITION_FUNCTION);
            let func = self.aref_items(rule).get(2).copied().unwrap_or(Value::NIL);
            lgstring = self.safe_calln(
                function,
                &[
                    func,
                    pos,
                    Value::fixnum(to as i64),
                    font_object,
                    string,
                    direction,
                ],
            )?;
        }
        Ok(lgstring)
    }

    fn composition_function_table(&mut self) -> LispResult<Value> {
        let table = self.find_symbol_value(sym::COMPOSITION_FUNCTION_TABLE);
        if self.is_char_table(table) {
            return Ok(table);
        }
        let table = self.make_char_table(Value::NIL, Value::NIL)?;
        self.set_plain_value(sym::COMPOSITION_FUNCTION_TABLE, table);
        Ok(table)
    }

    fn text_char_to_byte(&self, pos: i64, string: Value) -> i64 {
        if string.is_nil() {
            self.cur().text.char_to_byte(pos as usize) as i64
        } else {
            self.string_char_to_byte_index(string, pos as usize) as i64
        }
    }

    fn text_byte_to_char(&self, pos_byte: i64, string: Value) -> i64 {
        if string.is_nil() {
            self.cur().text.byte_to_char(pos_byte as usize) as i64
        } else {
            self.string_byte_to_char_index(string, pos_byte as usize) as i64
        }
    }

    fn auto_rule(&self, elt: Value) -> Option<i64> {
        let items = self.aref_items(elt);
        if self.is_vector(elt) && items.len() == 3 {
            return items[1].as_fixnum().filter(|&n| n >= 0);
        }
        None
    }

    pub fn composition_compute_stop_pos(
        &mut self,
        cmp_it: &mut CompositionIt,
        charpos: i64,
        endpos: i64,
        string: Value,
        include_static: bool,
    ) -> LispResult<()> {
        const MAX_NEWLINE_DISTANCE: i64 = 500;
        let mut charpos = charpos;
        let mut endpos = endpos;
        if charpos < endpos {
            if endpos > charpos + MAX_NEWLINE_DISTANCE {
                endpos = charpos + MAX_NEWLINE_DISTANCE;
            }
        } else if endpos < charpos && endpos < 0 {
            endpos = if string.is_nil() {
                self.cur().begv as i64 - 1
            } else {
                -1
            };
            if self.cur().long_line_optimizations_p {
                unreachable!("long-line optimizations are enabled only by redisplay");
            }
        }
        cmp_it.id = -1;
        cmp_it.ch = -2;
        cmp_it.reversed_p = false;
        cmp_it.stop_pos = endpos;
        if charpos == endpos {
            return Ok(());
        }
        if include_static
            && charpos < endpos
            && let Some((start, end, prop)) = self.find_composition(charpos, endpos, string)?
            && start as i64 >= charpos
            && self.composition_valid_p(start, end, prop)
        {
            cmp_it.stop_pos = start as i64;
            endpos = start as i64;
            cmp_it.ch = -1;
        }
        if (string.is_nil() && !self.cur().text.multibyte)
            || (string.is_string() && !self.string_multibyte(string))
            || self.inhibit_auto_composition()
        {
            return Ok(());
        }
        let start = charpos;
        let table = self.composition_function_table()?;
        if charpos < endpos {
            while charpos < endpos {
                let c = self.text_char(charpos as usize, string) as i64;
                charpos += 1;
                if c == b'\n' as i64 {
                    cmp_it.ch = -2;
                    break;
                }
                let mut val = self.char_table_ref(table, c as u32);
                let mut ridx = 0;
                while val.is_cons() {
                    let elt = self.xcar(val);
                    if let Some(back) = self.auto_rule(elt)
                        && charpos - 1 - back >= start
                    {
                        cmp_it.rule_idx = ridx;
                        cmp_it.lookback = back;
                        cmp_it.stop_pos = charpos - 1 - back;
                        cmp_it.ch = c;
                        return Ok(());
                    }
                    val = self.xcdr(val);
                    ridx += 1;
                }
            }
            if charpos == endpos
                && !(string.is_string() && endpos == self.string_chars(string) as i64)
            {
                charpos = endpos - MAX_AUTO_COMPOSITION_LOOKBACK;
                if charpos < start {
                    charpos = start;
                }
            }
        } else if charpos > endpos {
            let mut c = self.text_char(charpos as usize, string) as i64;
            let base = self.stack.len();
            self.stack.push(table);
            let r = (|| -> LispResult<()> {
                while self.char_composable_p(c) {
                    let mut val = self.char_table_ref(table, c as u32);
                    let mut ridx = 0;
                    while val.is_cons() {
                        let elt = self.xcar(val);
                        if let Some(back) = self.auto_rule(elt)
                            && charpos - back > endpos
                        {
                            let mut cpos = charpos - back;
                            let re = self.aref_items(elt)[0];
                            let blen = if re.is_string() {
                                self.stack.push(val);
                                let r = self.fast_looking_at(
                                    re,
                                    cpos as usize,
                                    (start + 1) as usize,
                                    string,
                                );
                                self.stack.pop();
                                r? as i64
                            } else {
                                1
                            };
                            if blen > 0 {
                                if blen > 1 {
                                    let bpos = self.text_char_to_byte(cpos, string) + blen;
                                    cpos = self.text_byte_to_char(bpos, string) - 1;
                                }
                                let back = cpos - (charpos - back);
                                if cmp_it.stop_pos < cpos
                                    || (cmp_it.stop_pos == cpos && cmp_it.lookback < back)
                                {
                                    cmp_it.rule_idx = ridx;
                                    cmp_it.stop_pos = cpos;
                                    cmp_it.ch = c;
                                    cmp_it.lookback = back;
                                    cmp_it.nchars = back + 1;
                                }
                            }
                        }
                        val = self.xcdr(val);
                        ridx += 1;
                    }
                    if charpos - 1 == endpos {
                        break;
                    }
                    charpos -= 1;
                    c = self.text_char(charpos as usize, string) as i64;
                }
                Ok(())
            })();
            self.stack.truncate(base);
            r?;
            if cmp_it.ch >= 0 {
                return Ok(());
            }
            while charpos - 1 > endpos && !self.char_composable_p(c) {
                charpos -= 1;
                c = self.text_char(charpos as usize, string) as i64;
            }
        }
        cmp_it.stop_pos = charpos;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn composition_reseat_it(
        &mut self,
        cmp_it: &mut CompositionIt,
        charpos: i64,
        endpos: i64,
        w: Value,
        bidi_level: i32,
        string: Value,
    ) -> LispResult<bool> {
        if cmp_it.ch == -2 {
            self.composition_compute_stop_pos(cmp_it, charpos, endpos, string, true)?;
            if cmp_it.ch == -2 || cmp_it.stop_pos != charpos {
                return Ok(false);
            }
        }
        let mut endpos = endpos;
        if endpos < 0 {
            endpos = if string.is_nil() {
                self.cur().begv as i64
            } else {
                0
            };
        }
        let base = self.stack.len();
        let composed = self.composition_reseat_it_1(cmp_it, charpos, endpos, w, bidi_level, string);
        self.stack.truncate(base);
        if composed? {
            return Ok(true);
        }
        if charpos == endpos {
            return Ok(false);
        }
        let next = if charpos < endpos {
            charpos + 1
        } else {
            charpos - 1
        };
        if cmp_it.reversed_p {
            endpos = -1;
        }
        self.composition_compute_stop_pos(cmp_it, next, endpos, string, true)?;
        Ok(false)
    }

    fn composition_reseat_it_1(
        &mut self,
        cmp_it: &mut CompositionIt,
        charpos: i64,
        endpos: i64,
        w: Value,
        bidi_level: i32,
        string: Value,
    ) -> LispResult<bool> {
        if cmp_it.ch < 0 {
            let Some((start, end, prop)) = self.find_composition(charpos, -1, string)? else {
                return Ok(false);
            };
            cmp_it.id = self.get_composition_id(charpos as usize, end - start, prop, string)?;
            if cmp_it.id < 0 {
                return Ok(false);
            }
            cmp_it.nchars = (end - start) as i64;
            cmp_it.nglyphs = self.compositions.table[cmp_it.id as usize].glyph_len as i64;
            return Ok(true);
        }
        if w.is_nil() {
            return Ok(false);
        }
        let table = self.composition_function_table()?;
        self.stack.push(table);
        let mut val = self.char_table_ref(table, cmp_it.ch as u32);
        for _ in 0..cmp_it.rule_idx {
            val = self.xcdr(val);
        }
        self.stack.push(val);
        let direction = if bidi_level < 0 {
            Value::NIL
        } else if bidi_level & 1 == 0 {
            sym::L2R
        } else {
            sym::R2L
        };
        let mut lgstring = Value::NIL;
        if charpos < endpos {
            while val.is_cons() {
                let elt = self.xcar(val);
                let fixnum_rule = self.is_vector(elt)
                    && self.aref_items(elt).len() == 3
                    && self.aref_items(elt)[1].is_fixnum();
                if !fixnum_rule {
                    val = self.xcdr(val);
                    continue;
                }
                if self.aref_items(elt)[1].as_fixnum() != Some(cmp_it.lookback) {
                    return Ok(false);
                }
                lgstring = self.autocmp_chars(
                    elt,
                    charpos as usize,
                    endpos as usize,
                    w,
                    string,
                    direction,
                    cmp_it.ch,
                )?;
                if self.composition_gstring_p(lgstring)? {
                    break;
                }
                lgstring = Value::NIL;
                val = self.xcdr(val);
            }
            cmp_it.reversed_p = false;
        } else {
            let mut cpos = charpos;
            cmp_it.reversed_p = true;
            let elt = self.xcar(val);
            if cmp_it.lookback > 0 {
                cpos = charpos - cmp_it.lookback;
                if cpos < endpos {
                    return Ok(false);
                }
            }
            lgstring = self.autocmp_chars(
                elt,
                cpos as usize,
                (charpos + 1) as usize,
                w,
                string,
                direction,
                cmp_it.ch,
            )?;
            if !self.composition_gstring_p(lgstring)?
                || cpos + self.lgstring_char_len(lgstring) as i64 - 1 != charpos
            {
                return Ok(false);
            }
        }
        if lgstring.is_nil() {
            return Ok(false);
        }
        if self.lgstring_id(lgstring).is_nil() {
            lgstring = self.composition_gstring_put_cache(lgstring, -1)?;
        }
        cmp_it.id = self.lgstring_id(lgstring).as_fixnum().unwrap_or(-1);
        let glyph_len = self.lgstring_glyph_len(lgstring);
        let i = (0..glyph_len)
            .find(|&i| self.lgstring_glyph(lgstring, i).is_nil())
            .unwrap_or(glyph_len) as i64;
        cmp_it.nglyphs = i;
        cmp_it.from = 0;
        cmp_it.to = i;
        Ok(true)
    }

    pub fn composition_update_it(
        &mut self,
        cmp_it: &mut CompositionIt,
        charpos: i64,
        bytepos: i64,
        string: Value,
    ) -> i64 {
        let mut c: i64 = 0;
        if cmp_it.ch < 0 {
            let cmp = self.compositions.table[cmp_it.id as usize].clone();
            cmp_it.charpos = charpos;
            cmp_it.to = cmp_it.nglyphs;
            if cmp_it.nglyphs == 0 {
                c = -1;
            } else {
                for i in 0..cmp.glyph_len {
                    let idx = if cmp.method == CompositionMethod::WithRuleAltchars {
                        i * 2
                    } else {
                        i
                    };
                    c = self.aref_items(cmp.key)[idx].as_fixnum().unwrap_or(0);
                    if c != b'\t' as i64 {
                        break;
                    }
                }
                if c == b'\t' as i64 {
                    c = b' ' as i64;
                }
            }
            cmp_it.width = cmp.width as i64;
            let end = charpos + cmp_it.nchars;
            cmp_it.nbytes = self.text_char_to_byte(end, string) - bytepos;
        } else {
            let gstring = self.composition_gstring_from_id(cmp_it.id as usize);
            if cmp_it.nglyphs == 0 {
                cmp_it.nchars = self.lgstring_char_len(gstring) as i64;
                cmp_it.width = 0;
                cmp_it.from = 0;
                cmp_it.to = 0;
                return -1;
            }
            let from;
            if !cmp_it.reversed_p {
                let glyph = self.lgstring_glyph(gstring, cmp_it.from as usize);
                from = self.lglyph_int(glyph, LGLYPH_IX_FROM);
                cmp_it.to = cmp_it.from + 1;
                while cmp_it.to < cmp_it.nglyphs {
                    let glyph = self.lgstring_glyph(gstring, cmp_it.to as usize);
                    if self.lglyph_int(glyph, LGLYPH_IX_FROM) != from {
                        break;
                    }
                    cmp_it.to += 1;
                }
                cmp_it.charpos = charpos;
            } else {
                let glyph = self.lgstring_glyph(gstring, (cmp_it.to - 1) as usize);
                from = self.lglyph_int(glyph, LGLYPH_IX_FROM);
                cmp_it.charpos = charpos - (self.lglyph_int(glyph, LGLYPH_IX_TO) - from);
                cmp_it.from = cmp_it.to - 1;
                while cmp_it.from > 0 {
                    let glyph = self.lgstring_glyph(gstring, (cmp_it.from - 1) as usize);
                    if self.lglyph_int(glyph, LGLYPH_IX_FROM) != from {
                        break;
                    }
                    cmp_it.from -= 1;
                }
            }
            let glyph = self.lgstring_glyph(gstring, cmp_it.from as usize);
            cmp_it.nchars = self.lglyph_int(glyph, LGLYPH_IX_TO) + 1 - from;
            cmp_it.nbytes = 0;
            let mut column_width = 1;
            if self.font_object_p(self.lgstring_font(gstring)) && !cmp_it.parent_f.is_nil() {
                column_width = self.frame(cmp_it.parent_f).geometry.column_width;
            }
            cmp_it.width = self.composition_gstring_width(
                gstring,
                cmp_it.from as usize,
                cmp_it.to as usize,
                None,
            ) as i64;
            cmp_it.width /= column_width;
            let header = self.lgstring_header(gstring);
            for i in (0..cmp_it.nchars).rev() {
                c = self
                    .aref_items(header)
                    .get((from + i + 1) as usize)
                    .and_then(|v| v.as_fixnum())
                    .unwrap_or(0);
                cmp_it.nbytes += character::char_string(c as u32).len() as i64;
            }
        }
        c
    }

    pub fn find_automatic_composition(
        &mut self,
        pos: usize,
        limit: i64,
        backlim: i64,
        string: Value,
    ) -> LispResult<Option<(usize, usize, Value)>> {
        let cb = self.current_buffer;
        let window = self.window_loop(
            crate::window::WindowLoop::GetBufferWindow,
            cb,
            true,
            Value::NIL,
        )?;
        if window.is_nil() {
            return Ok(None);
        }
        let base = self.stack.len();
        self.stack.push(window);
        let r = self.find_automatic_composition_1(pos, limit, backlim, string, window);
        self.stack.truncate(base);
        r
    }

    fn find_automatic_composition_1(
        &mut self,
        pos: usize,
        limit: i64,
        backlim: i64,
        string: Value,
        window: Value,
    ) -> LispResult<Option<(usize, usize, Value)>> {
        let pos = pos as i64;
        let (head, tail) = if string.is_nil() {
            let head = if backlim < 0 {
                let begv = self.cur().begv;
                self.find_newline(pos as usize, begv, -1).0 as i64
            } else {
                backlim
            };
            (head, self.cur().zv as i64)
        } else {
            (backlim.max(0), self.string_chars(string) as i64)
        };
        let limit = if limit < 0 { pos } else { limit };
        let mut fore_check_limit = if limit <= pos {
            tail.min(pos + 1 + MAX_AUTO_COMPOSITION_LOOKBACK)
        } else {
            tail.min(limit + MAX_AUTO_COMPOSITION_LOOKBACK)
        };
        let ch_at = |l: &Lisp, p: i64| i64::from(l.text_char(p.max(0) as usize, string));
        let mut cur = pos;
        loop {
            let mut c = ch_at(self, cur);
            let mut rewind = true;
            if !self.char_composable_p(c) {
                if limit <= pos {
                    loop {
                        if cur <= limit {
                            return Ok(None);
                        }
                        cur -= 1;
                        c = ch_at(self, cur);
                        if self.char_composable_p(c) {
                            break;
                        }
                    }
                    fore_check_limit = cur + 1;
                } else {
                    rewind = false;
                }
            }
            if rewind {
                while head < cur {
                    let prev = cur;
                    cur -= 1;
                    c = ch_at(self, cur);
                    if !self.char_composable_p(c) {
                        cur = prev;
                        break;
                    }
                }
            }
            let mut found: Option<(usize, usize, Value)> = None;
            let prev = cur;
            while cur < fore_check_limit {
                c = ch_at(self, cur);
                let table = self.bound_var(sym::COMPOSITION_FUNCTION_TABLE);
                let mut val = if self.is_char_table(table) {
                    self.char_table_ref(table, c as u32)
                } else {
                    Value::NIL
                };
                while val.is_cons() {
                    let elt = self.xcar(val);
                    let lookback = self
                        .aref_items(elt)
                        .get(1)
                        .and_then(|v| v.as_fixnum())
                        .filter(|&n| n >= 0);
                    if let Some(back) = lookback
                        && self.vector_items(elt).len() == 3
                    {
                        let check_pos = cur - back;
                        if check_pos < head
                            || (if limit <= pos {
                                pos < check_pos
                            } else {
                                limit <= check_pos
                            })
                        {
                            val = self.xcdr(val);
                            continue;
                        }
                        self.stack.push(val);
                        let gstring = self.autocmp_chars(
                            elt,
                            check_pos as usize,
                            tail as usize,
                            window,
                            string,
                            Value::NIL,
                            c,
                        )?;
                        self.stack.pop();
                        if gstring.is_nil() {
                            found = None;
                        } else {
                            let start = check_pos as usize;
                            let end = start + self.lgstring_char_len(gstring);
                            let p = pos as usize;
                            let target = if pos < limit {
                                p < end
                            } else {
                                start <= p && p < end
                            };
                            if target {
                                return Ok(Some((start, end, gstring)));
                            }
                            found = Some((start, end, gstring));
                            self.stack.push(gstring);
                            cur = end as i64;
                            break;
                        }
                    }
                    val = self.xcdr(val);
                }
                if !val.is_cons() {
                    cur += 1;
                }
            }
            if pos < limit {
                return Ok(None);
            }
            if found.is_some() {
                return Ok(found);
            }
            if prev == head {
                return Ok(None);
            }
            cur = prev - 1;
        }
    }

    pub fn composition_adjust_point(&mut self, last_pt: usize, new_pt: usize) -> LispResult<usize> {
        let (begv, zv) = (self.cur().begv, self.cur().zv);
        if new_pt == begv || new_pt == zv {
            return Ok(new_pt);
        }
        if let Some((val, beg, end)) =
            self.get_property_and_range(new_pt as i64, sym::COMPOSITION, Value::NIL)?
            && self.composition_valid_p(beg, end, val)
        {
            if beg < new_pt && (last_pt <= beg || last_pt >= end) {
                return Ok(if new_pt < last_pt { beg } else { end });
            }
            return Ok(new_pt);
        }
        if !self.cur().text.multibyte || self.inhibit_auto_composition() {
            return Ok(new_pt);
        }
        let Some((beg, _end, val)) = self.find_automatic_composition(new_pt, -1, -1, Value::NIL)?
        else {
            return Ok(new_pt);
        };
        if beg == new_pt {
            return Ok(new_pt);
        }
        for i in 0..self.lgstring_glyph_len(val) {
            let glyph = self.lgstring_glyph(val, i);
            if glyph.is_nil() {
                break;
            }
            let from = beg as i64 + self.lglyph_int(glyph, LGLYPH_IX_FROM);
            let to = beg as i64 + self.lglyph_int(glyph, LGLYPH_IX_TO);
            if from == new_pt as i64 {
                return Ok(new_pt);
            }
            if to >= new_pt as i64 {
                return Ok(if new_pt < last_pt {
                    from as usize
                } else {
                    (to + 1) as usize
                });
            }
        }
        Ok(new_pt)
    }
}

fn f_compose_region_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (start, end, components, modification_func) =
        (args[0], args[1], arg(args, 2), arg(args, 3));
    let (start, end) = lisp.validate_region(start, end)?;
    if !components.is_nil()
        && !components.is_fixnum()
        && !components.is_cons()
        && !components.is_string()
        && !lisp.is_vector(components)
    {
        return Err(lisp.wrong_type(sym::VECTORP, components));
    }
    lisp.compose_text(start, end, components, modification_func, Value::NIL)?;
    Ok(Value::NIL)
}

fn f_compose_string_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (string, start, end, components, modification_func) =
        (args[0], args[1], args[2], arg(args, 3), arg(args, 4));
    lisp.check_string(string)?;
    let chars = lisp.string_chars(string);
    let (from, to) = lisp.validate_subarray(string, start, end, chars)?;
    lisp.compose_text(from, to, components, modification_func, string)?;
    Ok(string)
}

fn f_find_composition_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (pos, limit, string, detail_p) = (args[0], args[1], args[2], args[3]);
    let fixed_pos = lisp.position_arg(pos)?;
    let to = if !limit.is_nil() {
        lisp.position_arg(limit)?.min(lisp.cur().zv as i64)
    } else {
        -1
    };
    if !string.is_nil() {
        lisp.check_string(string)?;
        if !(0 <= fixed_pos && fixed_pos <= lisp.string_chars(string) as i64) {
            return Err(lisp.args_out_of_range(string, pos));
        }
    } else if !(lisp.cur().begv as i64 <= fixed_pos && fixed_pos <= lisp.cur().zv as i64) {
        let cb = lisp.current_buffer;
        return Err(lisp.args_out_of_range(cb, pos));
    }
    let from = fixed_pos;
    let auto_ok = |lisp: &Lisp| {
        ((string.is_nil() && lisp.cur().text.multibyte)
            || (!string.is_nil() && lisp.string_multibyte(string)))
            && !lisp.inhibit_auto_composition()
    };
    let Some((start, end, prop)) = lisp.find_composition(from, to, string)? else {
        if auto_ok(lisp)
            && let Some((s, e, gstring)) =
                lisp.find_automatic_composition(from as usize, to, -1, string)?
        {
            return Ok(lisp.list(&[Value::fixnum(s as i64), Value::fixnum(e as i64), gstring]));
        }
        return Ok(Value::NIL);
    };
    lisp.stack.push(prop);
    if !(start as i64 <= fixed_pos && fixed_pos < end as i64)
        && let Some((s, e, gstring)) =
            lisp.find_automatic_composition(from as usize, to, -1, string)?
        && (if e as i64 <= fixed_pos {
            e > end
        } else {
            s < start
        })
    {
        lisp.stack.pop();
        return Ok(lisp.list(&[Value::fixnum(s as i64), Value::fixnum(e as i64), gstring]));
    }
    lisp.stack.pop();
    let (start_v, end_v) = (Value::fixnum(start as i64), Value::fixnum(end as i64));
    if !lisp.composition_valid_p(start, end, prop) {
        return Ok(lisp.list(&[start_v, end_v, Value::NIL]));
    }
    if detail_p.is_nil() {
        return Ok(lisp.list(&[start_v, end_v, sym::T]));
    }
    let id = if lisp.composition_registered_p(prop) {
        lisp.xcar(prop).as_fixnum().unwrap_or(-1)
    } else {
        lisp.get_composition_id(start, end - start, prop, string)?
    };
    let tail = if id >= 0 {
        let method = lisp.composition_method(prop);
        let width = lisp.compositions.table[id as usize].width;
        let components = lisp.composition_components(prop);
        let components = crate::fns::f_copy_sequence(lisp, &[components])?;
        lisp.stack.push(components);
        let relative_p = if method == CompositionMethod::WithRuleAltchars {
            Value::NIL
        } else {
            sym::T
        };
        let mod_func = lisp.composition_modification_func(prop);
        let tail = lisp.list(&[
            components,
            relative_p,
            mod_func,
            Value::fixnum(i64::from(width)),
        ]);
        lisp.stack.pop();
        tail
    } else {
        Value::NIL
    };
    Ok(lisp.list_with_tail(&[start_v, end_v], tail))
}

fn f_composition_get_gstring(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.composition_get_gstring(args[0], args[1], args[2], args[3])
}

fn f_clear_composition_cache(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    lisp.compositions.gstring_hash_table = lisp.make_hash_table_with(sym::EQUAL, Weakness::None);
    crate::xfaces::f_clear_face_cache(lisp, &[sym::T])
}

fn f_composition_sort_rules(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let rules = args[0];
    lisp.check_list(rules)?;
    let items = lisp.list_to_vec(rules)?;
    if items.len() <= 1 {
        return Ok(rules);
    }
    let mut keyed = Vec::with_capacity(items.len());
    for elt in items {
        let lookback = lisp
            .aref_items(elt)
            .get(1)
            .and_then(|v| v.as_fixnum())
            .filter(|&n| n >= 0);
        match lookback {
            Some(n) if lisp.vector_items(elt).len() == 3 => keyed.push((n, elt)),
            _ => return Err(lisp.error("Invalid composition rule in RULES argument")),
        }
    }
    keyed.sort_by_key(|a| std::cmp::Reverse(a.0));
    let sorted: Vec<Value> = keyed.into_iter().map(|(_, e)| e).collect();
    Ok(lisp.list(&sorted))
}
