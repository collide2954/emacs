use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("case-table-p", 1, 1, f_case_table_p),
    Subr::fixed("current-case-table", 0, 0, f_current_case_table),
    Subr::fixed("standard-case-table", 0, 0, f_standard_case_table),
    Subr::fixed("set-case-table", 1, 1, f_set_case_table),
    Subr::fixed("set-standard-case-table", 1, 1, f_set_standard_case_table),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.put(
        sym::CASE_TABLE,
        sym::CHAR_TABLE_EXTRA_SLOTS,
        Value::fixnum(3),
    );
    let down = lisp
        .make_char_table(sym::CASE_TABLE, Value::NIL)
        .expect("case table");
    lisp.ascii_downcase_table = down;
    for i in 0..128u32 {
        let c = if (b'A' as u32..=b'Z' as u32).contains(&i) {
            i + 32
        } else {
            i
        };
        lisp.char_table_set(down, i, Value::fixnum(c as i64));
    }
    let canon = crate::fns::f_copy_sequence(lisp, &[down]).expect("canon table");
    lisp.char_table_mut(down).extras[1] = canon;
    let up = lisp
        .make_char_table(sym::CASE_TABLE, Value::NIL)
        .expect("case table");
    lisp.char_table_mut(down).extras[0] = up;
    for i in 0..128u32 {
        let c = if (b'a' as u32..=b'z' as u32).contains(&i) {
            i - 32
        } else {
            i
        };
        lisp.char_table_set(up, i, Value::fixnum(c as i64));
    }
    let eqv = lisp
        .make_char_table(sym::CASE_TABLE, Value::NIL)
        .expect("case table");
    for i in 0..128u32 {
        let c = if (b'A' as u32..=b'Z' as u32).contains(&i) {
            i + 32
        } else if (b'a' as u32..=b'z' as u32).contains(&i) {
            i - 32
        } else {
            i
        };
        lisp.char_table_set(eqv, i, Value::fixnum(c as i64));
    }
    lisp.char_table_mut(down).extras[2] = eqv;
    lisp.set_case_table(down, true)
        .expect("standard case table");
}

impl Lisp {
    pub fn is_case_table(&self, object: Value) -> bool {
        if !self.is_char_table(object) || self.char_table(object).purpose != sym::CASE_TABLE {
            return false;
        }
        let extras = &self.char_table(object).extras;
        if extras.len() < 3 {
            return false;
        }
        let (up, canon, eqv) = (extras[0], extras[1], extras[2]);
        (up.is_nil() || self.is_char_table(up))
            && ((canon.is_nil() && eqv.is_nil())
                || (self.is_char_table(canon) && (eqv.is_nil() || self.is_char_table(eqv))))
    }

    fn set_identity(&mut self, table: Value, source: Value) {
        for (from, to, elt) in self.map_char_table_collect(source) {
            if matches!(elt.as_fixnum(), Some(n) if n >= 0) {
                for c in from..=to {
                    self.char_table_set(table, c, Value::fixnum(c as i64));
                }
            }
        }
    }

    fn shuffle(&mut self, table: Value, source: Value) -> LispResult<()> {
        for (from, to, elt) in self.map_char_table_collect(source) {
            if matches!(elt.as_fixnum(), Some(n) if n >= 0) {
                for c in from..=to {
                    let tem = self.aref_value(table, elt)?;
                    self.aset_value(table, elt, Value::fixnum(c as i64))?;
                    self.aset_value(table, Value::fixnum(c as i64), tem)?;
                }
            }
        }
        Ok(())
    }

    pub fn set_case_table(&mut self, table: Value, standard: bool) -> LispResult {
        if !self.is_case_table(table) {
            return Err(self.wrong_type(sym::CASE_TABLE_P, table));
        }
        let base = self.stack.len();
        self.stack.push(table);
        let r = self.set_case_table_1(table, standard);
        self.stack.truncate(base);
        r
    }

    fn set_case_table_1(&mut self, table: Value, standard: bool) -> LispResult {
        let mut up = self.char_table(table).extras[0];
        let mut canon = self.char_table(table).extras[1];
        let mut eqv = self.char_table(table).extras[2];
        if up.is_nil() {
            up = self.make_char_table(sym::CASE_TABLE, Value::NIL)?;
            self.stack.push(up);
            self.set_identity(up, table);
            self.shuffle(up, table)?;
            self.char_table_mut(table).extras[0] = up;
        }
        if canon.is_nil() {
            canon = self.make_char_table(sym::CASE_TABLE, Value::NIL)?;
            self.stack.push(canon);
            self.char_table_mut(table).extras[1] = canon;
            for (from, to, elt) in self.map_char_table_collect(table) {
                if matches!(elt.as_fixnum(), Some(n) if n >= 0) {
                    let upped = self.aref_value(up, elt)?;
                    let value = self.aref_value(table, upped)?;
                    self.char_table_set_range(canon, from, to, value);
                }
            }
        }
        if eqv.is_nil() {
            eqv = self.make_char_table(sym::CASE_TABLE, Value::NIL)?;
            self.stack.push(eqv);
            self.set_identity(eqv, canon);
            self.shuffle(eqv, canon)?;
            self.char_table_mut(table).extras[2] = eqv;
        }
        if self.char_table(canon).extras.len() > 2 {
            self.char_table_mut(canon).extras[2] = eqv;
        }
        if standard {
            self.ascii_downcase_table = table;
            self.ascii_upcase_table = up;
            self.ascii_canon_table = canon;
            self.ascii_eqv_table = eqv;
        } else {
            let b = self.cur_mut();
            b.downcase_table = table;
            b.upcase_table = up;
            b.case_canon_table = canon;
            b.case_eqv_table = eqv;
        }
        Ok(table)
    }

    pub fn downcase_char_buffer(&self, c: u32) -> u32 {
        let table = self.cur().downcase_table;
        if !self.is_char_table(table) {
            return c;
        }
        match self.char_table_ref(table, c).as_fixnum() {
            Some(n) if n >= 0 => n as u32,
            _ => c,
        }
    }

    pub fn upcase_char_buffer(&self, c: u32) -> u32 {
        let table = self.cur().upcase_table;
        if !self.is_char_table(table) {
            return c;
        }
        match self.char_table_ref(table, c).as_fixnum() {
            Some(n) if n >= 0 => n as u32,
            _ => c,
        }
    }

    pub fn uppercasep(&self, c: u32) -> bool {
        self.downcase_char_buffer(c) != c
    }

    pub fn lowercasep(&self, c: u32) -> bool {
        !self.uppercasep(c) && self.upcase_char_buffer(c) != c
    }
}

fn f_case_table_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.is_case_table(args[0])))
}

fn f_current_case_table(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.cur().downcase_table)
}

fn f_standard_case_table(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.ascii_downcase_table)
}

fn f_set_case_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.set_case_table(args[0], false)
}

fn f_set_standard_case_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.set_case_table(args[0], true)
}
