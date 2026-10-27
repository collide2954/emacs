use crate::chartab::CHARTAB_SIZE;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::Weakness;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("make-category-set", 1, 1, f_make_category_set),
    Subr::fixed("define-category", 2, 3, f_define_category),
    Subr::fixed("category-docstring", 1, 2, f_category_docstring),
    Subr::fixed("get-unused-category", 0, 1, f_get_unused_category),
    Subr::fixed("category-table-p", 1, 1, f_category_table_p),
    Subr::fixed("category-table", 0, 0, f_category_table),
    Subr::fixed("standard-category-table", 0, 0, f_standard_category_table),
    Subr::fixed("copy-category-table", 0, 1, f_copy_category_table),
    Subr::fixed("make-category-table", 0, 0, f_make_category_table),
    Subr::fixed("set-category-table", 1, 1, f_set_category_table),
    Subr::fixed("char-category-set", 1, 1, f_char_category_set),
    Subr::fixed("category-set-mnemonics", 1, 1, f_category_set_mnemonics),
    Subr::fixed("modify-category-entry", 2, 4, f_modify_category_entry),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.put(
        sym::CATEGORY_TABLE,
        sym::CHAR_TABLE_EXTRA_SLOTS,
        Value::fixnum(2),
    );
    let table = lisp
        .make_char_table(sym::CATEGORY_TABLE, Value::NIL)
        .expect("standard category table");
    lisp.set_plain_value(sym::STANDARD_CATEGORY_TABLE_INTERNAL, table);
    let empty = lisp.make_category_set();
    lisp.char_table_mut(table).default = empty;
    let docs = lisp.vector(vec![Value::NIL; 95]);
    lisp.char_table_mut(table).extras[0] = docs;
    lisp.defvar(sym::WORD_COMBINING_CATEGORIES, Value::NIL);
    lisp.defvar(sym::WORD_SEPARATING_CATEGORIES, Value::NIL);
}

impl Lisp {
    pub fn make_category_set(&mut self) -> Value {
        self.make_bool_vector(vec![false; 128])
    }

    pub fn standard_category_table(&self) -> Value {
        self.plain_value(sym::STANDARD_CATEGORY_TABLE_INTERNAL)
    }

    pub fn is_category_table(&self, v: Value) -> bool {
        self.is_char_table(v) && self.char_table(v).purpose == sym::CATEGORY_TABLE
    }

    fn check_category_table(&mut self, table: Value) -> LispResult<Value> {
        if table.is_nil() {
            return Ok(self.cur().category_table);
        }
        if self.is_category_table(table) {
            Ok(table)
        } else {
            Err(self.wrong_type(sym::CATEGORY_TABLE_P, table))
        }
    }

    fn check_category(&mut self, v: Value) -> LispResult<u32> {
        match v.as_fixnum() {
            Some(n) if (0x20..=0x7e).contains(&n) => Ok(n as u32),
            _ => Err(self.wrong_type(sym::CATEGORYP, v)),
        }
    }

    fn category_docs(&self, table: Value) -> Value {
        self.char_table(table).extras[0]
    }

    fn category_docstring(&self, table: Value, category: u32) -> Value {
        let docs = self.category_docs(table);
        self.vector_items(docs)
            .get((category - b' ' as u32) as usize)
            .copied()
            .unwrap_or(Value::NIL)
    }

    pub fn bool_vector_ref(&self, v: Value, idx: usize) -> bool {
        if !v.is_vectorlike() {
            return false;
        }
        match self.heap.vectorlike_ref(v) {
            crate::object::Vectorlike::BoolVector(b) => b.bits.get(idx).copied().unwrap_or(false),
            _ => false,
        }
    }

    fn is_category_set(&self, v: Value) -> bool {
        v.is_vectorlike()
            && matches!(self.heap.vectorlike_ref(v), crate::object::Vectorlike::BoolVector(b) if b.bits.len() == 128)
    }

    pub fn char_category_set(&self, c: u32) -> Value {
        let table = self.cur().category_table;
        if self.is_char_table(table) {
            self.char_table_ref(table, c)
        } else {
            Value::NIL
        }
    }

    fn hash_get_category_set(&mut self, table: Value, set: Value) -> LispResult {
        if self.char_table(table).extras[1].is_nil() {
            let h = self.make_hash_table_with(sym::EQUAL, Weakness::None);
            self.char_table_mut(table).extras[1] = h;
        }
        let h = self.char_table(table).extras[1];
        let found = self.hash_lookup_key(h, set)?;
        if let Some(key) = found {
            return Ok(key);
        }
        self.puthash(set, Value::NIL, h)?;
        Ok(set)
    }

    fn copy_category_table(&mut self, table: Value) -> LispResult {
        let table = crate::fns::f_copy_sequence(self, &[table])?;
        self.stack.push(table);
        let default = self.char_table(table).default;
        if !default.is_nil() {
            let copy = crate::fns::f_copy_sequence(self, &[default])?;
            self.char_table_mut(table).default = copy;
        }
        let docs = self.char_table(table).extras[0];
        let copy = crate::fns::f_copy_sequence(self, &[docs])?;
        self.char_table_mut(table).extras[0] = copy;
        for (from, to, val) in self.map_char_table_collect(table) {
            let copy = crate::fns::f_copy_sequence(self, &[val])?;
            if from == to {
                self.char_table_set(table, from, copy);
            } else {
                self.char_table_set_range(table, from, to, copy);
            }
        }
        self.stack.pop();
        Ok(table)
    }
}

fn f_make_category_set(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let categories = lisp.check_string(args[0])?;
    if lisp.string_multibyte(categories) {
        return Err(lisp.error("Multibyte string in `make-category-set'"));
    }
    let val = lisp.make_category_set();
    let bytes = lisp.string_bytes(categories).to_vec();
    for &cat in bytes.iter().rev() {
        lisp.check_category(Value::fixnum(cat as i64))?;
        if let crate::object::Vectorlike::BoolVector(b) = lisp.heap.vectorlike_mut(val) {
            b.bits[cat as usize] = true;
        }
    }
    Ok(val)
}

fn f_define_category(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let category = lisp.check_category(args[0])?;
    let docstring = lisp.check_string(args[1])?;
    let table = lisp.check_category_table(args.get(2).copied().unwrap_or(Value::NIL))?;
    if !lisp.category_docstring(table, category).is_nil() {
        let ch = char::from_u32(category).unwrap_or('?');
        return Err(lisp.error(&format!("Category `{ch}' is already defined")));
    }
    let docs = lisp.category_docs(table);
    lisp.aset_value(
        docs,
        Value::fixnum((category - b' ' as u32) as i64),
        docstring,
    )?;
    Ok(Value::NIL)
}

fn f_category_docstring(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let category = lisp.check_category(args[0])?;
    let table = lisp.check_category_table(args.get(1).copied().unwrap_or(Value::NIL))?;
    Ok(lisp.category_docstring(table, category))
}

fn f_get_unused_category(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = lisp.check_category_table(args.first().copied().unwrap_or(Value::NIL))?;
    for i in b' '..=b'~' {
        if lisp.category_docstring(table, i as u32).is_nil() {
            return Ok(Value::fixnum(i as i64));
        }
    }
    Ok(Value::NIL)
}

fn f_category_table_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.is_category_table(args[0])))
}

fn f_category_table(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.cur().category_table)
}

fn f_standard_category_table(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.standard_category_table())
}

fn f_copy_category_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = args.first().copied().unwrap_or(Value::NIL);
    let table = if table.is_nil() {
        lisp.standard_category_table()
    } else {
        lisp.check_category_table(table)?
    };
    lisp.copy_category_table(table)
}

fn f_make_category_table(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let val = lisp.make_char_table(sym::CATEGORY_TABLE, Value::NIL)?;
    lisp.stack.push(val);
    let default = lisp.make_category_set();
    lisp.char_table_mut(val).default = default;
    for i in 0..CHARTAB_SIZE[0] {
        let set = lisp.make_category_set();
        lisp.char_table_mut(val).contents[i] = set;
    }
    let docs = lisp.vector(vec![Value::NIL; 95]);
    lisp.char_table_mut(val).extras[0] = docs;
    lisp.stack.pop();
    Ok(val)
}

fn f_set_category_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let table = lisp.check_category_table(args[0])?;
    lisp.cur_mut().category_table = table;
    Ok(table)
}

fn f_char_category_set(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    Ok(lisp.char_category_set(c))
}

fn f_category_set_mnemonics(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let set = args[0];
    if !lisp.is_category_set(set) {
        return Err(lisp.wrong_type(sym::CATEGORYSETP, set));
    }
    let mut out = String::new();
    for i in 32..127 {
        if lisp.bool_vector_ref(set, i) {
            out.push(i as u8 as char);
        }
    }
    Ok(lisp.string(&out))
}

fn f_modify_category_entry(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let character = args[0];
    let (mut start, end) = if character.is_fixnum() {
        let c = lisp.check_character(character)?;
        (c, c)
    } else {
        lisp.check_cons(character)?;
        let s = lisp.check_character(lisp.xcar(character))?;
        let e = lisp.check_character(lisp.xcdr(character))?;
        (s, e)
    };
    let category = lisp.check_category(args[1])?;
    let table = lisp.check_category_table(args.get(2).copied().unwrap_or(Value::NIL))?;
    let reset = args.get(3).copied().unwrap_or(Value::NIL);
    if lisp.category_docstring(table, category).is_nil() {
        let ch = char::from_u32(category).unwrap_or('?');
        return Err(lisp.error(&format!("Undefined category: {ch}")));
    }
    let set_value = reset.is_nil();
    while start <= end {
        let mut from = start;
        let mut to = end;
        let set = lisp.char_table_ref_and_range(table, start, &mut from, &mut to);
        if lisp.bool_vector_ref(set, category as usize) != reset.is_nil() {
            let copy = crate::fns::f_copy_sequence(lisp, &[set])?;
            if let crate::object::Vectorlike::BoolVector(b) = lisp.heap.vectorlike_mut(copy) {
                b.bits[category as usize] = set_value;
            }
            lisp.stack.push(copy);
            let shared = lisp.hash_get_category_set(table, copy);
            lisp.stack.pop();
            let shared = shared?;
            lisp.char_table_set_range(table, start, to, shared);
        }
        start = to + 1;
    }
    Ok(Value::NIL)
}
