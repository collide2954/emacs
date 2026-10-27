use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use num_traits::ToPrimitive;

use crate::character;
use crate::data::{Num, compare_nums};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::{HashTable, LispString, Vectorlike, Weakness};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("identity", 1, 1, f_identity),
    Subr::fixed("random", 0, 1, f_random),
    Subr::fixed("length", 1, 1, f_length),
    Subr::fixed("safe-length", 1, 1, f_safe_length),
    Subr::fixed("length<", 2, 2, f_length_less),
    Subr::fixed("length>", 2, 2, f_length_greater),
    Subr::fixed("length=", 2, 2, f_length_equal),
    Subr::fixed("proper-list-p", 1, 1, f_proper_list_p),
    Subr::fixed("string-bytes", 1, 1, f_string_bytes),
    Subr::fixed("string-distance", 2, 3, f_string_distance),
    Subr::fixed("string-equal", 2, 2, f_string_equal),
    Subr::fixed("compare-strings", 6, 7, f_compare_strings),
    Subr::fixed("string-lessp", 2, 2, f_string_lessp),
    Subr::fixed("string-version-lessp", 2, 2, f_string_version_lessp),
    Subr::fixed("string-collate-lessp", 2, 4, f_string_collate_lessp),
    Subr::fixed("string-collate-equalp", 2, 4, f_string_collate_equalp),
    Subr::many("append", 0, f_append),
    Subr::many("concat", 0, f_concat),
    Subr::many("vconcat", 0, f_vconcat),
    Subr::fixed("copy-sequence", 1, 1, f_copy_sequence),
    Subr::fixed("string-make-multibyte", 1, 1, f_string_make_multibyte),
    Subr::fixed("string-make-unibyte", 1, 1, f_string_make_unibyte),
    Subr::fixed("string-as-unibyte", 1, 1, f_string_as_unibyte),
    Subr::fixed("string-as-multibyte", 1, 1, f_string_as_multibyte),
    Subr::fixed("string-to-multibyte", 1, 1, f_string_to_multibyte),
    Subr::fixed("string-to-unibyte", 1, 1, f_string_to_unibyte),
    Subr::fixed("copy-alist", 1, 1, f_copy_alist),
    Subr::fixed("substring", 1, 3, f_substring),
    Subr::fixed("substring-no-properties", 1, 3, f_substring_no_properties),
    Subr::fixed("take", 2, 2, f_take),
    Subr::fixed("ntake", 2, 2, f_ntake),
    Subr::fixed("nthcdr", 2, 2, f_nthcdr),
    Subr::fixed("nth", 2, 2, f_nth),
    Subr::fixed("elt", 2, 2, f_elt),
    Subr::fixed("member", 2, 2, f_member),
    Subr::fixed("memq", 2, 2, f_memq),
    Subr::fixed("memql", 2, 2, f_memql),
    Subr::fixed("assq", 2, 2, f_assq),
    Subr::fixed("assoc", 2, 3, f_assoc),
    Subr::fixed("rassq", 2, 2, f_rassq),
    Subr::fixed("rassoc", 2, 2, f_rassoc),
    Subr::fixed("delq", 2, 2, f_delq),
    Subr::fixed("delete", 2, 2, f_delete),
    Subr::fixed("nreverse", 1, 1, f_nreverse),
    Subr::fixed("reverse", 1, 1, f_reverse),
    Subr::many("sort", 1, f_sort),
    Subr::fixed("plist-get", 2, 3, f_plist_get),
    Subr::fixed("get", 2, 2, f_get),
    Subr::fixed("plist-put", 3, 4, f_plist_put),
    Subr::fixed("put", 3, 3, f_put),
    Subr::fixed("plist-member", 2, 3, f_plist_member),
    Subr::fixed("eql", 2, 2, f_eql),
    Subr::fixed("equal", 2, 2, f_equal),
    Subr::fixed(
        "equal-including-properties",
        2,
        2,
        f_equal_including_properties,
    ),
    Subr::fixed("value<", 2, 2, f_value_lt),
    Subr::fixed("fillarray", 2, 2, f_fillarray),
    Subr::fixed("clear-string", 1, 1, f_clear_string),
    Subr::many("nconc", 0, f_nconc),
    Subr::fixed("mapconcat", 2, 3, f_mapconcat),
    Subr::fixed("mapcar", 2, 2, f_mapcar),
    Subr::fixed("mapc", 2, 2, f_mapc),
    Subr::fixed("mapcan", 2, 2, f_mapcan),
    Subr::fixed("featurep", 1, 2, f_featurep),
    Subr::fixed("provide", 1, 2, f_provide),
    Subr::fixed("require", 1, 3, f_require),
    Subr::fixed("load-average", 0, 1, f_load_average),
    Subr::fixed("locale-info", 1, 1, f_locale_info),
    Subr::fixed("base64-encode-string", 1, 2, f_base64_encode_string),
    Subr::fixed("base64url-encode-string", 1, 2, f_base64url_encode_string),
    Subr::fixed("base64-decode-string", 1, 3, f_base64_decode_string),
    Subr::fixed("sxhash-eq", 1, 1, f_sxhash_eq),
    Subr::fixed("sxhash-eql", 1, 1, f_sxhash_eql),
    Subr::fixed("sxhash-equal", 1, 1, f_sxhash_equal),
    Subr::fixed("sxhash-equal-including-properties", 1, 1, f_sxhash_equal),
    Subr::many("make-hash-table", 0, f_make_hash_table),
    Subr::fixed("copy-hash-table", 1, 1, f_copy_hash_table),
    Subr::fixed("hash-table-count", 1, 1, f_hash_table_count),
    Subr::fixed("hash-table-rehash-size", 1, 1, f_hash_table_rehash_size),
    Subr::fixed(
        "hash-table-rehash-threshold",
        1,
        1,
        f_hash_table_rehash_threshold,
    ),
    Subr::fixed("hash-table-size", 1, 1, f_hash_table_size),
    Subr::fixed("hash-table-test", 1, 1, f_hash_table_test),
    Subr::fixed("hash-table-weakness", 1, 1, f_hash_table_weakness),
    Subr::fixed("hash-table-p", 1, 1, f_hash_table_p),
    Subr::fixed("clrhash", 1, 1, f_clrhash),
    Subr::fixed("gethash", 2, 3, f_gethash),
    Subr::fixed("puthash", 3, 3, f_puthash),
    Subr::fixed("remhash", 2, 2, f_remhash),
    Subr::fixed("maphash", 2, 2, f_maphash),
    Subr::fixed("define-hash-table-test", 3, 3, f_define_hash_table_test),
    Subr::fixed("secure-hash-algorithms", 0, 0, f_secure_hash_algorithms),
    Subr::fixed("string-search", 2, 3, f_string_search),
    Subr::fixed("object-intervals", 1, 1, f_object_intervals),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::FEATURES, Value::NIL);
    lisp.defvar_bool(sym::USE_DIALOG_BOX, true);
    lisp.defvar_bool(sym::USE_FILE_DIALOG, true);
    lisp.defvar_bool(sym::USE_SHORT_ANSWERS, false);
    lisp.defvar(sym::YES_OR_NO_PROMPT, Value::NIL);
    let prompt = lisp.string("(yes or no) ");
    lisp.set_plain_value(sym::YES_OR_NO_PROMPT, prompt);
    lisp.defvar(sym::REQUIRE_NESTING_LIST, Value::NIL);
}

fn bool_value(b: bool) -> Value {
    if b { sym::T } else { Value::NIL }
}

impl Lisp {
    pub fn assq(&self, key: Value, mut list: Value) -> Value {
        while list.is_cons() {
            let elt = self.xcar(list);
            if elt.is_cons() && self.xcar(elt) == key {
                return elt;
            }
            list = self.xcdr(list);
        }
        Value::NIL
    }

    pub fn memq(&self, elt: Value, mut list: Value) -> Value {
        while list.is_cons() {
            if self.xcar(list) == elt {
                return list;
            }
            list = self.xcdr(list);
        }
        Value::NIL
    }

    pub fn member(&mut self, elt: Value, mut list: Value) -> LispResult {
        while list.is_cons() {
            let item = self.xcar(list);
            if self.equal(elt, item)? {
                return Ok(list);
            }
            list = self.xcdr(list);
        }
        if !list.is_nil() {
            return Err(self.wrong_type(sym::LISTP, list));
        }
        Ok(Value::NIL)
    }

    pub fn nth(&self, n: usize, mut list: Value) -> Value {
        for _ in 0..n {
            if !list.is_cons() {
                return Value::NIL;
            }
            list = self.xcdr(list);
        }
        self.car_safe(list)
    }

    pub fn length(&mut self, seq: Value) -> LispResult {
        let n = self.length_of(seq)?;
        Ok(Value::fixnum(n as i64))
    }

    pub fn length_of(&mut self, seq: Value) -> LispResult<usize> {
        if seq.is_cons() {
            let mut n = 0usize;
            let mut tail = seq;
            while tail.is_cons() {
                n += 1;
                tail = self.xcdr(tail);
                if n.is_multiple_of(1_000_000) && self.list_is_circular(seq) {
                    return Err(self.signal_list(sym::CIRCULAR_LIST, &[seq]));
                }
            }
            if !tail.is_nil() {
                return Err(self.wrong_type(sym::LISTP, seq));
            }
            return Ok(n);
        }
        if seq.is_nil() {
            return Ok(0);
        }
        if seq.is_string() {
            return Ok(self.string_chars(seq));
        }
        if seq.is_vectorlike() {
            match self.heap.vectorlike_ref(seq) {
                Vectorlike::Vector(v) | Vectorlike::Record(v) | Vectorlike::Closure(v) => {
                    return Ok(v.len());
                }
                Vectorlike::BoolVector(b) => return Ok(b.bits.len()),
                Vectorlike::CharTable(_) => return Ok(crate::character::MAX_CHAR as usize + 1),
                _ => {}
            }
        }
        Err(self.wrong_type(sym::SEQUENCEP, seq))
    }

    pub fn equal(&mut self, a: Value, b: Value) -> LispResult<bool> {
        self.internal_equal(a, b, false, 0)
    }

    pub fn eql(&self, a: Value, b: Value) -> bool {
        if a == b {
            return true;
        }
        if a.is_float() && b.is_float() {
            return self.heap.float_value(a).to_bits() == self.heap.float_value(b).to_bits();
        }
        if let (Some(x), Some(y)) = (self.bignum_ref(a), self.bignum_ref(b)) {
            return x == y;
        }
        if self.var_true(sym::SYMBOLS_WITH_POS_ENABLED) {
            return self.eq(a, b);
        }
        false
    }

    fn internal_equal(
        &mut self,
        a: Value,
        b: Value,
        props: bool,
        depth: usize,
    ) -> LispResult<bool> {
        if depth > 200 {
            return Err(self.error("Stack overflow in equal"));
        }
        let mut a = a;
        let mut b = b;
        loop {
            if a == b {
                return Ok(true);
            }
            if self.var_true(sym::SYMBOLS_WITH_POS_ENABLED) {
                a = self.maybe_remove_pos_from_symbol(a);
                b = self.maybe_remove_pos_from_symbol(b);
                if a == b {
                    return Ok(true);
                }
            }
            if a.tag() != b.tag() {
                return Ok(false);
            }
            if a.is_float() {
                return Ok(self.heap.float_value(a).to_bits() == self.heap.float_value(b).to_bits());
            }
            if a.is_cons() {
                let (ca, cb) = (self.xcar(a), self.xcar(b));
                if !self.internal_equal(ca, cb, props, depth + 1)? {
                    return Ok(false);
                }
                a = self.xcdr(a);
                b = self.xcdr(b);
                continue;
            }
            if a.is_string() {
                let sa = self.heap.string_ref(a);
                let sb = self.heap.string_ref(b);
                if sa.chars != sb.chars || sa.data != sb.data {
                    return Ok(false);
                }
                if props {
                    let pa = sa.props.clone();
                    let pb = sb.props.clone();
                    return self.props_equal(&pa, &pb);
                }
                return Ok(true);
            }
            if a.is_vectorlike() {
                let pair = (self.heap.vectorlike_ref(a), self.heap.vectorlike_ref(b));
                let (ia, ib): (Vec<Value>, Vec<Value>) = match pair {
                    (Vectorlike::Vector(x), Vectorlike::Vector(y))
                    | (Vectorlike::Record(x), Vectorlike::Record(y))
                    | (Vectorlike::Closure(x), Vectorlike::Closure(y)) => {
                        if x.len() != y.len() {
                            return Ok(false);
                        }
                        (x.clone(), y.clone())
                    }
                    (Vectorlike::Bignum(x), Vectorlike::Bignum(y)) => return Ok(x == y),
                    (Vectorlike::BoolVector(x), Vectorlike::BoolVector(y)) => {
                        return Ok(x.bits == y.bits);
                    }
                    (Vectorlike::CharTable(x), Vectorlike::CharTable(y)) => {
                        if x.extras.len() != y.extras.len() {
                            return Ok(false);
                        }
                        let mut xa = vec![x.default, x.parent, x.purpose, x.ascii];
                        xa.extend_from_slice(&x.contents);
                        xa.extend_from_slice(&x.extras);
                        let mut ya = vec![y.default, y.parent, y.purpose, y.ascii];
                        ya.extend_from_slice(&y.contents);
                        ya.extend_from_slice(&y.extras);
                        (xa, ya)
                    }
                    (Vectorlike::SubCharTable(x), Vectorlike::SubCharTable(y)) => {
                        if x.depth != y.depth || x.min_char != y.min_char {
                            return Ok(false);
                        }
                        (x.contents.clone(), y.contents.clone())
                    }
                    (Vectorlike::SymbolWithPos { .. }, Vectorlike::SymbolWithPos { .. }) => {
                        let (
                            Vectorlike::SymbolWithPos {
                                symbol: s1,
                                position: p1,
                            },
                            Vectorlike::SymbolWithPos {
                                symbol: s2,
                                position: p2,
                            },
                        ) = pair
                        else {
                            unreachable!()
                        };
                        return Ok(s1 == s2 && p1 == p2);
                    }
                    _ => {
                        return Ok(self.vectorlike_equal_other(a, b));
                    }
                };
                for (x, y) in ia.into_iter().zip(ib) {
                    if !self.internal_equal(x, y, props, depth + 1)? {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
            return Ok(false);
        }
    }

    fn props_equal(
        &mut self,
        a: &[crate::object::PropRun],
        b: &[crate::object::PropRun],
    ) -> LispResult<bool> {
        let a: Vec<_> = a.iter().filter(|r| !r.plist.is_nil()).copied().collect();
        let b: Vec<_> = b.iter().filter(|r| !r.plist.is_nil()).copied().collect();
        if a.len() != b.len() {
            return Ok(false);
        }
        for (x, y) in a.iter().zip(&b) {
            if x.start != y.start || x.end != y.end {
                return Ok(false);
            }
            if !self.plists_equal(x.plist, y.plist)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn plists_equal(&mut self, a: Value, b: Value) -> LispResult<bool> {
        let la = self.list_length_unchecked(a);
        let lb = self.list_length_unchecked(b);
        if la != lb {
            return Ok(false);
        }
        let mut tail = a;
        while tail.is_cons() {
            let key = self.xcar(tail);
            let rest = self.xcdr(tail);
            let value = self.car_safe(rest);
            let other = self.plist_get_eq(b, key);
            if !self.equal(value, other)? {
                return Ok(false);
            }
            tail = self.cdr_safe(rest);
        }
        Ok(true)
    }

    pub fn plist_get_eq(&self, plist: Value, prop: Value) -> Value {
        let mut tail = plist;
        while tail.is_cons() {
            let rest = self.xcdr(tail);
            if !rest.is_cons() {
                break;
            }
            if self.xcar(tail) == prop {
                return self.xcar(rest);
            }
            tail = self.xcdr(rest);
        }
        Value::NIL
    }

    pub fn vectorlike_equal_other(&self, _a: Value, _b: Value) -> bool {
        false
    }

    pub fn delete(&mut self, elt: Value, seq: Value) -> LispResult {
        if seq.is_nil() || seq.is_cons() {
            let mut prev = Value::NIL;
            let mut head = seq;
            let mut tail = seq;
            while tail.is_cons() {
                let item = self.xcar(tail);
                if self.equal(elt, item)? {
                    if prev.is_nil() {
                        head = self.xcdr(tail);
                    } else {
                        let next = self.xcdr(tail);
                        self.heap.set_cdr(prev, next);
                    }
                } else {
                    prev = tail;
                }
                tail = self.xcdr(tail);
            }
            if !tail.is_nil() {
                return Err(self.wrong_type(sym::LISTP, seq));
            }
            return Ok(head);
        }
        if self.is_vector(seq) {
            let items = self.vector_items(seq).to_vec();
            let mut kept = Vec::with_capacity(items.len());
            for item in items.iter().copied() {
                if !self.equal(elt, item)? {
                    kept.push(item);
                }
            }
            if kept.len() == items.len() {
                return Ok(seq);
            }
            return Ok(self.vector(kept));
        }
        if seq.is_string() {
            let chars = self.string_char_vec(seq);
            let multibyte = self.string_multibyte(seq);
            let mut kept = Vec::new();
            for c in &chars {
                let cv = Value::fixnum(*c as i64);
                if !self.equal(elt, cv)? {
                    kept.push(*c);
                }
            }
            if kept.len() == chars.len() {
                return Ok(seq);
            }
            return Ok(self.string_from_chars(&kept, multibyte));
        }
        Err(self.wrong_type(sym::SEQUENCEP, seq))
    }

    pub fn string_char_vec(&self, s: Value) -> Vec<u32> {
        let ls = self.heap.string_ref(s);
        if ls.multibyte {
            character::multibyte_chars(&ls.data).collect()
        } else {
            ls.data.iter().map(|&b| b as u32).collect()
        }
    }

    pub fn string_from_chars(&mut self, chars: &[u32], multibyte: bool) -> Value {
        if multibyte {
            let mut data = Vec::with_capacity(chars.len());
            for &c in chars {
                character::push_char(&mut data, c);
            }
            self.heap.string(LispString::multibyte(data, chars.len()))
        } else {
            let data = chars.iter().map(|&c| character::char_to_byte8(c)).collect();
            self.heap.string(LispString::unibyte(data))
        }
    }

    pub fn sequence_elements(&mut self, seq: Value) -> LispResult<Vec<Value>> {
        if seq.is_nil() {
            return Ok(Vec::new());
        }
        if seq.is_cons() {
            return self.list_to_vec(seq);
        }
        if seq.is_string() {
            let chars = self.string_char_vec(seq);
            return Ok(chars.iter().map(|&c| Value::fixnum(c as i64)).collect());
        }
        if seq.is_vectorlike() {
            match self.heap.vectorlike_ref(seq) {
                Vectorlike::Vector(v) | Vectorlike::Record(v) | Vectorlike::Closure(v) => {
                    return Ok(v.clone());
                }
                Vectorlike::BoolVector(b) => {
                    return Ok(b.bits.iter().map(|&x| bool_value(x)).collect());
                }
                _ => {}
            }
        }
        Err(self.wrong_type(sym::SEQUENCEP, seq))
    }

    pub fn plist_put_eq(&mut self, plist: Value, prop: Value, val: Value) -> LispResult {
        let mut prev = Value::NIL;
        let mut tail = plist;
        while tail.is_cons() {
            let rest = self.xcdr(tail);
            if !rest.is_cons() {
                return Err(self.wrong_type(sym::PLISTP, plist));
            }
            if self.xcar(tail) == prop {
                self.heap.set_car(rest, val);
                return Ok(plist);
            }
            prev = tail;
            tail = self.xcdr(rest);
        }
        if !tail.is_nil() {
            return Err(self.wrong_type(sym::PLISTP, plist));
        }
        let new = self.list(&[prop, val]);
        if prev.is_nil() {
            return Ok(new);
        }
        let last = self.xcdr(prev);
        self.heap.set_cdr(last, new);
        Ok(plist)
    }

    pub fn nconc2(&mut self, a: Value, b: Value) -> Value {
        if a.is_nil() {
            return b;
        }
        let mut tail = a;
        while self.xcdr(tail).is_cons() {
            tail = self.xcdr(tail);
        }
        self.heap.set_cdr(tail, b);
        a
    }

    pub fn reverse_list(&mut self, list: Value) -> LispResult {
        let mut result = Value::NIL;
        let mut tail = list;
        while tail.is_cons() {
            let item = self.xcar(tail);
            result = self.cons(item, result);
            tail = self.xcdr(tail);
        }
        if !tail.is_nil() {
            return Err(self.wrong_type(sym::LISTP, list));
        }
        Ok(result)
    }

    pub fn mapcar_vec(&mut self, function: Value, seq: Value) -> LispResult<Vec<Value>> {
        let items = self.sequence_elements(seq)?;
        let base = self.stack.len();
        self.stack.push(function);
        self.stack.push(seq);
        self.stack.extend_from_slice(&items);
        let results_start = self.stack.len();
        for item in items {
            match self.funcall(function, &[item]) {
                Ok(v) => self.stack.push(v),
                Err(e) => {
                    self.stack.truncate(base);
                    return Err(e);
                }
            }
        }
        let results = self.stack[results_start..].to_vec();
        self.stack.truncate(base);
        Ok(results)
    }

    pub fn concat_to_string(&mut self, args: &[Value]) -> LispResult {
        let mut multibyte = false;
        for &arg in args {
            if arg.is_string() {
                if self.string_multibyte(arg) {
                    multibyte = true;
                }
            } else {
                for elt in self.sequence_elements(arg)? {
                    let c = self.check_character(elt)?;
                    if !character::ascii_char_p(c) && !character::char_byte8_p(c) {
                        multibyte = true;
                    }
                }
            }
        }
        let mut data = Vec::new();
        let mut chars = 0;
        let mut props = Vec::new();
        for &arg in args {
            if arg.is_string() {
                let s = self.heap.string_ref(arg);
                for run in &s.props {
                    props.push(crate::object::PropRun {
                        start: run.start + chars,
                        end: run.end + chars,
                        plist: run.plist,
                    });
                }
                if multibyte && !s.multibyte {
                    data.extend(character::str_to_multibyte(&s.data));
                } else {
                    data.extend_from_slice(&s.data);
                }
                chars += s.chars;
            } else {
                for elt in self.sequence_elements(arg)? {
                    let c = self.check_character(elt)?;
                    if multibyte {
                        character::push_char(&mut data, c);
                    } else {
                        data.push(character::char_to_byte8(c));
                    }
                    chars += 1;
                }
            }
        }
        let mut s = if multibyte {
            LispString::multibyte(data, chars)
        } else {
            LispString::unibyte(data)
        };
        s.props = props;
        Ok(self.heap.string(s))
    }

    pub fn substring_chars(&mut self, string: Value, from: usize, to: usize) -> Value {
        let s = self.heap.string_ref(string);
        let multibyte = s.multibyte;
        let (start, end) = if multibyte {
            let start = character::char_offset_to_byte(&s.data, from);
            let end = start + character::char_offset_to_byte(&s.data[start..], to - from);
            (start, end)
        } else {
            (from, to)
        };
        let data = s.data[start..end].to_vec();
        let props: Vec<_> = s
            .props
            .iter()
            .filter(|r| r.end > from && r.start < to)
            .map(|r| crate::object::PropRun {
                start: r.start.max(from) - from,
                end: r.end.min(to) - from,
                plist: r.plist,
            })
            .collect();
        let mut new = if multibyte {
            LispString::multibyte(data, to - from)
        } else {
            LispString::unibyte(data)
        };
        new.props = props;
        self.heap.string(new)
    }

    pub fn validate_subarray(
        &mut self,
        array: Value,
        from: Value,
        to: Value,
        size: usize,
    ) -> LispResult<(usize, usize)> {
        let size = size as i64;
        let f = if from.is_nil() {
            0
        } else {
            self.check_fixnum(from)?
        };
        let t = if to.is_nil() {
            size
        } else {
            self.check_fixnum(to)?
        };
        let f2 = if f < 0 { f + size } else { f };
        let t2 = if t < 0 { t + size } else { t };
        if !(0 <= f2 && f2 <= t2 && t2 <= size) {
            return Err(self.args_out_of_range3(array, from, to));
        }
        Ok((f2 as usize, t2 as usize))
    }

    pub fn hash_table_ref(&mut self, table: Value) -> LispResult<()> {
        if self.is_hash_table(table) {
            Ok(())
        } else {
            Err(self.wrong_type(sym::HASH_TABLE_P, table))
        }
    }

    pub fn hash_table(&self, table: Value) -> &HashTable {
        match self.heap.vectorlike_ref(table) {
            Vectorlike::HashTable(h) => h,
            _ => unreachable!(),
        }
    }

    fn hash_table_mut(&mut self, table: Value) -> &mut HashTable {
        match self.heap.vectorlike_mut(table) {
            Vectorlike::HashTable(h) => h,
            _ => unreachable!(),
        }
    }

    pub fn hash_key(&mut self, test: Value, key: Value) -> LispResult<u64> {
        if test == sym::EQ {
            return Ok(hash_u64(self.maybe_remove_pos_from_symbol(key).bits()));
        }
        if test == sym::EQL {
            return Ok(self.sxhash_eql(key));
        }
        if test == sym::EQUAL {
            return Ok(self.sxhash_equal(key, 0));
        }
        let spec = self.get(test, sym::HASH_TABLE_TEST);
        let hash_fn = self.nth(1, spec);
        let h = self.funcall(hash_fn, &[key])?;
        Ok(match h.as_fixnum() {
            Some(n) => n as u64,
            None => self.sxhash_equal(h, 0),
        })
    }

    pub fn hash_test_equal(&mut self, test: Value, a: Value, b: Value) -> LispResult<bool> {
        if test == sym::EQ {
            return Ok(self.eq(a, b));
        }
        if test == sym::EQL {
            return Ok(self.eql(a, b));
        }
        if test == sym::EQUAL {
            return self.equal(a, b);
        }
        let spec = self.get(test, sym::HASH_TABLE_TEST);
        let test_fn = self.car_safe(spec);
        Ok(!self.funcall(test_fn, &[a, b])?.is_nil())
    }

    pub fn hash_lookup(&mut self, table: Value, key: Value) -> LispResult<Option<usize>> {
        let test = self.hash_table(table).test;
        let hash = self.hash_key(test, key)?;
        let candidates = self.hash_table(table).candidates(hash).to_vec();
        for slot in candidates {
            let Some((k, _)) = self.hash_table(table).entries[slot] else {
                continue;
            };
            if self.hash_test_equal(test, key, k)? {
                return Ok(Some(slot));
            }
        }
        Ok(None)
    }

    pub fn hash_lookup_key(&mut self, table: Value, key: Value) -> LispResult<Option<Value>> {
        Ok(self
            .hash_lookup(table, key)?
            .and_then(|slot| self.hash_table(table).entries[slot].map(|(k, _)| k)))
    }

    pub fn gethash(&mut self, key: Value, table: Value, default: Value) -> LispResult {
        self.hash_table_ref(table)?;
        match self.hash_lookup(table, key)? {
            Some(slot) => Ok(self.hash_table(table).entries[slot].map_or(default, |(_, v)| v)),
            None => Ok(default),
        }
    }

    pub fn puthash(&mut self, key: Value, value: Value, table: Value) -> LispResult {
        self.hash_table_ref(table)?;
        if !self.hash_table(table).mutable {
            return Err(self.signal_list(sym::ERROR, &[]));
        }
        match self.hash_lookup(table, key)? {
            Some(slot) => {
                if let Some(entry) = &mut self.hash_table_mut(table).entries[slot] {
                    entry.1 = value;
                }
            }
            None => {
                let test = self.hash_table(table).test;
                let hash = self.hash_key(test, key)?;
                self.hash_table_mut(table).insert_new(hash, key, value);
            }
        }
        Ok(value)
    }

    pub fn make_hash_table_with(&mut self, test: Value, weakness: Weakness) -> Value {
        self.heap
            .vectorlike(Vectorlike::HashTable(Box::new(HashTable::new(
                test, weakness,
            ))))
    }

    pub fn sxhash_eql(&self, key: Value) -> u64 {
        if key.is_float() {
            return hash_u64(self.heap.float_value(key).to_bits());
        }
        if let Some(b) = self.bignum_ref(key) {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            b.hash(&mut h);
            return h.finish();
        }
        hash_u64(self.maybe_remove_pos_from_symbol(key).bits())
    }

    pub fn sxhash_equal(&self, key: Value, depth: usize) -> u64 {
        const MAX_DEPTH: usize = 3;
        const MAX_LEN: usize = 7;
        if depth > MAX_DEPTH {
            return 0;
        }
        if key.is_string() {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.string_bytes(key).hash(&mut h);
            return h.finish();
        }
        if key.is_float() {
            return hash_u64(self.heap.float_value(key).to_bits());
        }
        if key.is_cons() {
            let mut hash = 0u64;
            let mut tail = key;
            let mut i = 0;
            while tail.is_cons() && i < MAX_LEN {
                let h = self.sxhash_equal(self.xcar(tail), depth + 1);
                hash = combine(hash, h);
                tail = self.xcdr(tail);
                i += 1;
            }
            if !tail.is_nil() && i < MAX_LEN {
                hash = combine(hash, self.sxhash_equal(tail, depth + 1));
            }
            return hash;
        }
        if key.is_vectorlike() {
            match self.heap.vectorlike_ref(key) {
                Vectorlike::Bignum(_) => return self.sxhash_eql(key),
                Vectorlike::Vector(v) | Vectorlike::Record(v) | Vectorlike::Closure(v) => {
                    let mut hash = v.len() as u64;
                    for item in v.iter().take(MAX_LEN) {
                        hash = combine(hash, self.sxhash_equal(*item, depth + 1));
                    }
                    return hash;
                }
                Vectorlike::BoolVector(b) => {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    b.bits.hash(&mut h);
                    return h.finish();
                }
                Vectorlike::SymbolWithPos { symbol, .. } => {
                    return hash_u64(symbol.bits());
                }
                _ => {}
            }
        }
        hash_u64(key.bits())
    }

    pub fn string_compare(&self, a: Value, b: Value) -> Ordering {
        let ca = self.string_char_vec(a);
        let cb = self.string_char_vec(b);
        ca.cmp(&cb)
    }

    pub fn string_or_symbol_name(&mut self, v: Value) -> LispResult<Value> {
        let v = self.maybe_remove_pos_from_symbol(v);
        if v.is_symbol() {
            return Ok(self.symbol_name(v));
        }
        self.check_string(v)
    }

    pub fn value_cmp(&mut self, a: Value, b: Value, maxdepth: i32) -> LispResult<Ordering> {
        if maxdepth < 0 {
            return Err(self.error("Maximum depth exceeded in comparison"));
        }
        let mut a = a;
        let mut b = b;
        loop {
            if self.is_number(a) && self.is_number(b) {
                let na = self.number(a)?;
                let nb = self.number(b)?;
                let (lt, _, gt) = compare_nums(&na, &nb);
                return Ok(if lt {
                    Ordering::Less
                } else if gt {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                });
            }
            if a.is_symbol() {
                if b.is_symbol() {
                    let (na, nb) = (self.symbol_name(a), self.symbol_name(b));
                    return Ok(self.string_compare(na, nb));
                }
                if a.is_nil() && b.is_cons() {
                    return Ok(Ordering::Less);
                }
                return Err(self.signal_list(sym::TYPE_MISMATCH, &[a, b]));
            }
            if a.is_string() {
                if b.is_string() {
                    return Ok(self.string_compare(a, b));
                }
                return Err(self.signal_list(sym::TYPE_MISMATCH, &[a, b]));
            }
            if a.is_cons() {
                if b.is_nil() {
                    return Ok(Ordering::Greater);
                }
                if !b.is_cons() {
                    return Err(self.signal_list(sym::TYPE_MISMATCH, &[a, b]));
                }
                let (ca, cb) = (self.xcar(a), self.xcar(b));
                let c = self.value_cmp(ca, cb, maxdepth - 1)?;
                if c != Ordering::Equal {
                    return Ok(c);
                }
                a = self.xcdr(a);
                b = self.xcdr(b);
                continue;
            }
            if a.is_vectorlike() && b.is_vectorlike() {
                let ia = match self.heap.vectorlike_ref(a) {
                    Vectorlike::Vector(v) | Vectorlike::Record(v) => Some(v.clone()),
                    _ => None,
                };
                let ib = match self.heap.vectorlike_ref(b) {
                    Vectorlike::Vector(v) | Vectorlike::Record(v) => Some(v.clone()),
                    _ => None,
                };
                if let (Some(ia), Some(ib)) = (ia, ib) {
                    for (x, y) in ia.iter().zip(&ib) {
                        let c = self.value_cmp(*x, *y, maxdepth - 1)?;
                        if c != Ordering::Equal {
                            return Ok(c);
                        }
                    }
                    return Ok(ia.len().cmp(&ib.len()));
                }
                if let (Ok(x), Ok(y)) = (self.bool_vector_bits(a), self.bool_vector_bits(b)) {
                    return Ok(x.cmp(&y));
                }
                return Ok(Ordering::Equal);
            }
            return Err(self.signal_list(sym::TYPE_MISMATCH, &[a, b]));
        }
    }
}

fn hash_u64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn combine(a: u64, b: u64) -> u64 {
    a.rotate_left(4).wrapping_add(b)
}

fn sxhash_value(h: u64) -> Value {
    Value::fixnum((h >> 3) as i64 & crate::value::MOST_POSITIVE_FIXNUM)
}

fn f_identity(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(args[0])
}

impl Lisp {
    pub fn next_random(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }
}

fn f_random(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let limit = args[0];
    if limit == sym::T {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x1234_5678, |d| d.as_nanos() as u64);
        lisp.rng_state = seed | 1;
    } else if limit.is_string() {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        lisp.string_bytes(limit).hash(&mut h);
        lisp.rng_state = h.finish() | 1;
    }
    if let Some(n) = limit.as_fixnum()
        && n > 0
    {
        let r = lisp.next_random() % n as u64;
        return Ok(Value::fixnum(r as i64));
    }
    if let Some(b) = lisp.bignum_ref(limit).cloned()
        && b > num_bigint::BigInt::from(0)
    {
        let bits = b.bits();
        let mut acc = num_bigint::BigInt::from(0);
        let mut produced = 0;
        while produced < bits + 64 {
            acc = (acc << 64) + num_bigint::BigInt::from(lisp.next_random());
            produced += 64;
        }
        let r = acc % b;
        return Ok(lisp.bignum(r));
    }
    let r = (lisp.next_random() >> 3) as i64 & crate::value::MOST_POSITIVE_FIXNUM;
    let r = if lisp.next_random() & 1 == 1 {
        r
    } else {
        r - crate::value::MOST_POSITIVE_FIXNUM - 1
    };
    Ok(Value::fixnum(r))
}

fn f_length(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.length(args[0])
}

fn f_safe_length(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let list = args[0];
    let mut n = 0u64;
    let mut slow = list;
    let mut fast = list;
    while fast.is_cons() {
        fast = lisp.xcdr(fast);
        n += 1;
        if n.is_multiple_of(2) {
            slow = lisp.xcdr(slow);
            if slow == fast && fast.is_cons() {
                break;
            }
        }
    }
    Ok(Value::fixnum(n as i64))
}

fn length_compare(lisp: &mut Lisp, seq: Value, n: Value) -> LispResult<Ordering> {
    let n = lisp.check_fixnum(n)?;
    if seq.is_cons() {
        let mut len = 0i64;
        let mut tail = seq;
        while tail.is_cons() && len <= n {
            len += 1;
            tail = lisp.xcdr(tail);
        }
        return Ok(len.cmp(&n));
    }
    let len = lisp.length_of(seq)? as i64;
    Ok(len.cmp(&n))
}

fn f_length_less(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        length_compare(lisp, args[0], args[1])? == Ordering::Less,
    ))
}

fn f_length_greater(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        length_compare(lisp, args[0], args[1])? == Ordering::Greater,
    ))
}

fn f_length_equal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        length_compare(lisp, args[0], args[1])? == Ordering::Equal,
    ))
}

fn f_proper_list_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let list = args[0];
    if !lisp.proper_list_p(list) {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(lisp.list_length_unchecked(list) as i64))
}

fn f_string_bytes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    Ok(Value::fixnum(lisp.string_bytes(s).len() as i64))
}

fn f_string_distance(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.check_string(args[0])?;
    let b = lisp.check_string(args[1])?;
    let (x, y): (Vec<u32>, Vec<u32>) = if !args[2].is_nil() {
        (
            lisp.string_bytes(a).iter().map(|&c| c as u32).collect(),
            lisp.string_bytes(b).iter().map(|&c| c as u32).collect(),
        )
    } else {
        (lisp.string_char_vec(a), lisp.string_char_vec(b))
    };
    let mut prev: Vec<usize> = (0..=y.len()).collect();
    for (i, cx) in x.iter().enumerate() {
        let mut cur = vec![i + 1; y.len() + 1];
        for (j, cy) in y.iter().enumerate() {
            let cost = usize::from(cx != cy);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    Ok(Value::fixnum(prev[y.len()] as i64))
}

pub(crate) fn f_string_equal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.string_or_symbol_name(args[0])?;
    let b = lisp.string_or_symbol_name(args[1])?;
    let sa = lisp.heap.string_ref(a);
    let sb = lisp.heap.string_ref(b);
    Ok(bool_value(sa.chars == sb.chars && sa.data == sb.data))
}

fn f_compare_strings(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (str1, start1, end1, str2, start2, end2, ignore_case) = (
        args[0], args[1], args[2], args[3], args[4], args[5], args[6],
    );
    lisp.check_string(str1)?;
    lisp.check_string(str2)?;
    let c1 = lisp.string_char_vec(str1);
    let c2 = lisp.string_char_vec(str2);
    let end1 = if end1.is_nil() {
        Value::NIL
    } else if end1.as_fixnum().is_some_and(|n| n > c1.len() as i64) {
        Value::fixnum(c1.len() as i64)
    } else {
        end1
    };
    let end2 = if end2.is_nil() {
        Value::NIL
    } else if end2.as_fixnum().is_some_and(|n| n > c2.len() as i64) {
        Value::fixnum(c2.len() as i64)
    } else {
        end2
    };
    let (f1, t1) = lisp.validate_subarray(str1, start1, end1, c1.len())?;
    let (f2, t2) = lisp.validate_subarray(str2, start2, end2, c2.len())?;
    let mut i1 = f1;
    let mut i2 = f2;
    while i1 < t1 && i2 < t2 {
        let mut a = c1[i1];
        let mut b = c2[i2];
        if !ignore_case.is_nil() {
            a = crate::casefiddle::upcase_char(a);
            b = crate::casefiddle::upcase_char(b);
        }
        if a != b {
            let pos = (i1 - f1 + 1) as i64;
            return Ok(Value::fixnum(if a < b { -pos } else { pos }));
        }
        i1 += 1;
        i2 += 1;
    }
    if i1 < t1 {
        return Ok(Value::fixnum((i1 - f1 + 1) as i64));
    }
    if i2 < t2 {
        return Ok(Value::fixnum(-((i1 - f1 + 1) as i64)));
    }
    Ok(sym::T)
}

pub(crate) fn f_string_lessp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.string_or_symbol_name(args[0])?;
    let b = lisp.string_or_symbol_name(args[1])?;
    Ok(bool_value(lisp.string_compare(a, b) == Ordering::Less))
}

fn version_cmp(a: &[u32], b: &[u32]) -> Ordering {
    let digit = |c: u32| (0x30..=0x39).contains(&c);
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if digit(a[i]) && digit(b[j]) {
            let si = i;
            while i < a.len() && digit(a[i]) {
                i += 1;
            }
            let sj = j;
            while j < b.len() && digit(b[j]) {
                j += 1;
            }
            let na: String = a[si..i].iter().map(|&c| c as u8 as char).collect();
            let nb: String = b[sj..j].iter().map(|&c| c as u8 as char).collect();
            let ta = na.trim_start_matches('0');
            let tb = nb.trim_start_matches('0');
            let c = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
            if c != Ordering::Equal {
                return c;
            }
        } else {
            if a[i] != b[j] {
                return a[i].cmp(&b[j]);
            }
            i += 1;
            j += 1;
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

fn f_string_version_lessp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.string_or_symbol_name(args[0])?;
    let b = lisp.string_or_symbol_name(args[1])?;
    let ca = lisp.string_char_vec(a);
    let cb = lisp.string_char_vec(b);
    Ok(bool_value(version_cmp(&ca, &cb) == Ordering::Less))
}

fn f_string_collate_lessp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.string_or_symbol_name(args[0])?;
    let b = lisp.string_or_symbol_name(args[1])?;
    let mut ca = lisp.string_char_vec(a);
    let mut cb = lisp.string_char_vec(b);
    if !args[3].is_nil() {
        ca = ca
            .into_iter()
            .map(crate::casefiddle::downcase_char)
            .collect();
        cb = cb
            .into_iter()
            .map(crate::casefiddle::downcase_char)
            .collect();
    }
    Ok(bool_value(ca < cb))
}

fn f_string_collate_equalp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.string_or_symbol_name(args[0])?;
    let b = lisp.string_or_symbol_name(args[1])?;
    let mut ca = lisp.string_char_vec(a);
    let mut cb = lisp.string_char_vec(b);
    if !args[3].is_nil() {
        ca = ca
            .into_iter()
            .map(crate::casefiddle::downcase_char)
            .collect();
        cb = cb
            .into_iter()
            .map(crate::casefiddle::downcase_char)
            .collect();
    }
    Ok(bool_value(ca == cb))
}

fn f_append(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args.is_empty() {
        return Ok(Value::NIL);
    }
    let mut items = Vec::new();
    for &arg in &args[..args.len() - 1] {
        items.extend(lisp.sequence_elements(arg)?);
    }
    let last = args[args.len() - 1];
    Ok(lisp.list_with_tail(&items, last))
}

pub(crate) fn f_concat(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.concat_to_string(args)
}

fn f_vconcat(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut items = Vec::new();
    for &arg in args {
        items.extend(lisp.sequence_elements(arg)?);
    }
    Ok(lisp.vector(items))
}

pub fn f_copy_sequence(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let arg = args[0];
    if arg.is_nil() {
        return Ok(Value::NIL);
    }
    if arg.is_cons() {
        let items = lisp.list_to_vec(arg)?;
        return Ok(lisp.list(&items));
    }
    if arg.is_string() {
        let s = lisp.heap.string_ref(arg).clone();
        return Ok(lisp.heap.string(s));
    }
    if arg.is_vectorlike() {
        let copy = match lisp.heap.vectorlike_ref(arg) {
            Vectorlike::Vector(v) => Some(Vectorlike::Vector(v.clone())),
            Vectorlike::Record(v) => Some(Vectorlike::Record(v.clone())),
            Vectorlike::Closure(v) => Some(Vectorlike::Closure(v.clone())),
            Vectorlike::BoolVector(b) => Some(Vectorlike::BoolVector(b.clone())),
            Vectorlike::CharTable(t) => Some(Vectorlike::CharTable(t.clone())),
            _ => None,
        };
        if let Some(copy) = copy {
            if let Vectorlike::CharTable(t) = &copy {
                let contents = t.contents.clone();
                let mut copy = copy;
                let mut new_contents = Vec::with_capacity(contents.len());
                for c in contents {
                    new_contents.push(lisp.copy_sub_char_table(c));
                }
                if let Vectorlike::CharTable(t) = &mut copy {
                    t.contents = new_contents;
                }
                return Ok(lisp.heap.vectorlike(copy));
            }
            return Ok(lisp.heap.vectorlike(copy));
        }
    }
    Err(lisp.wrong_type(sym::SEQUENCEP, arg))
}

fn f_string_make_multibyte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    if lisp.string_multibyte(s) {
        return Ok(s);
    }
    let bytes = lisp.string_bytes(s).to_vec();
    if bytes.iter().all(|&b| b < 0x80) {
        return Ok(s);
    }
    let chars: Vec<u32> = bytes.iter().map(|&b| b as u32).collect();
    Ok(lisp.string_from_chars(&chars, true))
}

fn f_string_make_unibyte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    if !lisp.string_multibyte(s) {
        return Ok(s);
    }
    let chars = lisp.string_char_vec(s);
    let bytes: Vec<u8> = chars.iter().map(|&c| (c & 0xFF) as u8).collect();
    Ok(lisp.unibyte_string(&bytes))
}

fn f_string_as_unibyte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    if !lisp.string_multibyte(s) {
        return Ok(s);
    }
    let bytes = character::str_as_unibyte(lisp.string_bytes(s));
    Ok(lisp.unibyte_string(&bytes))
}

fn f_string_as_multibyte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    if lisp.string_multibyte(s) {
        return Ok(s);
    }
    let bytes = lisp.string_bytes(s).to_vec();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match character::multibyte_length(&bytes[i..]) {
            Some(len) if bytes[i] >= 0x80 => {
                out.extend_from_slice(&bytes[i..i + len]);
                i += len;
            }
            _ => {
                if bytes[i] < 0x80 {
                    out.push(bytes[i]);
                } else {
                    character::push_char(&mut out, character::byte8_to_char(bytes[i]));
                }
                i += 1;
            }
        }
    }
    Ok(lisp.make_string_from_bytes(out, true))
}

fn f_string_to_multibyte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    if lisp.string_multibyte(s) {
        return Ok(s);
    }
    let bytes = lisp.string_bytes(s).to_vec();
    if bytes.iter().all(|&b| b < 0x80) {
        let chars = bytes.len();
        return Ok(lisp.make_specified_string(bytes, chars, true));
    }
    let chars = bytes.len();
    let data = character::str_to_multibyte(&bytes);
    Ok(lisp.make_specified_string(data, chars, true))
}

fn f_string_to_unibyte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    if !lisp.string_multibyte(s) {
        return Ok(s);
    }
    let chars = lisp.string_char_vec(s);
    let mut bytes = Vec::with_capacity(chars.len());
    for (i, &c) in chars.iter().enumerate() {
        if c < 0x80 {
            bytes.push(c as u8);
        } else if character::char_byte8_p(c) {
            bytes.push(character::char_to_byte8(c));
        } else {
            return Err(lisp.error(&format!("Cannot convert character at index {i} to unibyte")));
        }
    }
    Ok(lisp.unibyte_string(&bytes))
}

pub(crate) fn f_copy_alist(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let alist = lisp.check_list(args[0])?;
    let items = lisp.list_to_vec(alist)?;
    let mut copied = Vec::with_capacity(items.len());
    for item in items {
        if item.is_cons() {
            let (a, d) = (lisp.xcar(item), lisp.xcdr(item));
            copied.push(lisp.cons(a, d));
        } else {
            copied.push(item);
        }
    }
    Ok(lisp.list(&copied))
}

pub fn f_substring(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (string, from, to) = (args[0], args[1], args[2]);
    if string.is_string() {
        let size = lisp.string_chars(string);
        let (f, t) = lisp.validate_subarray(string, from, to, size)?;
        return Ok(lisp.substring_chars(string, f, t));
    }
    if lisp.is_vector(string) {
        let items = lisp.vector_items(string).to_vec();
        let (f, t) = lisp.validate_subarray(string, from, to, items.len())?;
        return Ok(lisp.vector(items[f..t].to_vec()));
    }
    Err(lisp.wrong_type(sym::ARRAYP, string))
}

fn f_substring_no_properties(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (string, from, to) = (args[0], args[1], args[2]);
    lisp.check_string(string)?;
    let size = lisp.string_chars(string);
    let (f, t) = lisp.validate_subarray(string, from, to, size)?;
    let v = lisp.substring_chars(string, f, t);
    lisp.heap.string_mut(v).props.clear();
    Ok(v)
}

fn f_take(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (n, list) = (args[0], args[1]);
    let mut m = match n.as_fixnum() {
        Some(m) if m <= 0 => return Ok(Value::NIL),
        Some(m) => m,
        None => match lisp.bignum_ref(n) {
            Some(b) if b.sign() == num_bigint::Sign::Minus => return Ok(Value::NIL),
            Some(_) => crate::value::MOST_POSITIVE_FIXNUM,
            None => return Err(lisp.wrong_type(sym::INTEGERP, n)),
        },
    };
    lisp.check_list(list)?;
    let mut items = Vec::new();
    let mut tail = list;
    while m > 0 && tail.is_cons() {
        items.push(lisp.xcar(tail));
        tail = lisp.xcdr(tail);
        m -= 1;
    }
    if m > 0 && !tail.is_nil() {
        return Err(lisp.wrong_type(sym::LISTP, tail));
    }
    Ok(lisp.list(&items))
}

fn f_ntake(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (n, list) = (args[0], args[1]);
    let n = match n.as_fixnum() {
        Some(n) => n,
        None if lisp.is_bignum(n) => return Ok(list),
        None => return Err(lisp.wrong_type(sym::INTEGERP, n)),
    };
    if n <= 0 {
        return Ok(Value::NIL);
    }
    let mut tail = list;
    for _ in 1..n {
        if !tail.is_cons() {
            return Ok(list);
        }
        tail = lisp.xcdr(tail);
    }
    if tail.is_cons() {
        lisp.heap.set_cdr(tail, Value::NIL);
    }
    Ok(list)
}

pub(crate) fn f_nthcdr(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (n, list) = (args[0], args[1]);
    let n = match n.as_fixnum() {
        Some(n) => n,
        None if lisp.bignum_ref(n).is_some() => {
            let big = lisp.bignum_ref(n).cloned().unwrap_or_default();
            if big < num_bigint::BigInt::from(0) {
                return Ok(list);
            }
            let mut tail = list;
            while tail.is_cons() {
                tail = lisp.xcdr(tail);
            }
            return Ok(tail);
        }
        None => return Err(lisp.wrong_type(sym::INTEGERP, n)),
    };
    let mut tail = list;
    for _ in 0..n.max(0) {
        if tail.is_nil() {
            break;
        }
        if !tail.is_cons() {
            return Err(lisp.wrong_type(sym::LISTP, tail));
        }
        tail = lisp.xcdr(tail);
    }
    Ok(tail)
}

fn f_nth(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let tail = f_nthcdr(lisp, args)?;
    lisp.car(tail)
}

fn f_elt(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (seq, n) = (args[0], args[1]);
    if seq.is_cons() || seq.is_nil() {
        return f_nth(lisp, &[n, seq]);
    }
    if !lisp.arrayp(seq) {
        return Err(lisp.wrong_type(sym::SEQUENCEP, seq));
    }
    lisp.aref_value(seq, n)
}

fn f_member(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.member(args[0], args[1])
}

fn f_memq(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (elt, list) = (args[0], args[1]);
    let mut tail = list;
    while tail.is_cons() {
        let item = lisp.xcar(tail);
        if lisp.eq(item, elt) {
            return Ok(tail);
        }
        tail = lisp.xcdr(tail);
    }
    if !tail.is_nil() {
        return Err(lisp.wrong_type(sym::LISTP, list));
    }
    Ok(Value::NIL)
}

fn f_memql(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (elt, list) = (args[0], args[1]);
    let mut tail = list;
    while tail.is_cons() {
        if lisp.eql(lisp.xcar(tail), elt) {
            return Ok(tail);
        }
        tail = lisp.xcdr(tail);
    }
    if !tail.is_nil() {
        return Err(lisp.wrong_type(sym::LISTP, list));
    }
    Ok(Value::NIL)
}

fn f_assq(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (key, alist) = (args[0], args[1]);
    let mut tail = alist;
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        if elt.is_cons() && lisp.eq(lisp.xcar(elt), key) {
            return Ok(elt);
        }
        tail = lisp.xcdr(tail);
    }
    if !tail.is_nil() {
        return Err(lisp.wrong_type(sym::LISTP, alist));
    }
    Ok(Value::NIL)
}

fn f_assoc(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (key, alist, testfn) = (args[0], args[1], args[2]);
    if testfn.is_nil() && (key.is_symbol() || key.is_fixnum()) {
        return f_assq(lisp, &[key, alist]);
    }
    let base = lisp.stack.len();
    lisp.stack.push(alist);
    let mut tail = alist;
    let result = (|| {
        while tail.is_cons() {
            let elt = lisp.xcar(tail);
            if elt.is_cons() {
                let k = lisp.xcar(elt);
                let matched = if testfn.is_nil() {
                    lisp.equal(k, key)?
                } else {
                    !lisp.funcall(testfn, &[k, key])?.is_nil()
                };
                if matched {
                    return Ok(elt);
                }
            }
            tail = lisp.xcdr(tail);
        }
        if !tail.is_nil() {
            return Err(lisp.wrong_type(sym::LISTP, alist));
        }
        Ok(Value::NIL)
    })();
    lisp.stack.truncate(base);
    result
}

fn f_rassq(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (key, alist) = (args[0], args[1]);
    let mut tail = alist;
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        if elt.is_cons() && lisp.eq(lisp.xcdr(elt), key) {
            return Ok(elt);
        }
        tail = lisp.xcdr(tail);
    }
    Ok(Value::NIL)
}

fn f_rassoc(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (key, alist) = (args[0], args[1]);
    let mut tail = alist;
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        if elt.is_cons() {
            let v = lisp.xcdr(elt);
            if lisp.equal(v, key)? {
                return Ok(elt);
            }
        }
        tail = lisp.xcdr(tail);
    }
    Ok(Value::NIL)
}

pub fn f_delq(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (elt, list) = (args[0], args[1]);
    let mut prev = Value::NIL;
    let mut head = list;
    let mut tail = list;
    while tail.is_cons() {
        let item = lisp.xcar(tail);
        if lisp.eq(item, elt) {
            let next = lisp.xcdr(tail);
            if prev.is_nil() {
                head = next;
            } else {
                lisp.heap.set_cdr(prev, next);
            }
        } else {
            prev = tail;
        }
        tail = lisp.xcdr(tail);
    }
    if !tail.is_nil() {
        return Err(lisp.wrong_type(sym::LISTP, list));
    }
    Ok(head)
}

fn f_delete(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.delete(args[0], args[1])
}

fn f_nreverse(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let seq = args[0];
    if seq.is_nil() {
        return Ok(seq);
    }
    if seq.is_string() {
        let chars: Vec<u32> = lisp.string_char_vec(seq).into_iter().rev().collect();
        let multibyte = lisp.string_multibyte(seq);
        let new = lisp.string_from_chars(&chars, multibyte);
        let data = lisp.heap.string_ref(new).data.clone();
        lisp.heap.string_mut(seq).data = data;
        return Ok(seq);
    }
    if seq.is_cons() {
        let mut prev = Value::NIL;
        let mut tail = seq;
        while tail.is_cons() {
            let next = lisp.xcdr(tail);
            lisp.heap.set_cdr(tail, prev);
            prev = tail;
            tail = next;
        }
        if !tail.is_nil() {
            return Err(lisp.wrong_type(sym::LISTP, seq));
        }
        return Ok(prev);
    }
    if seq.is_vectorlike() {
        match lisp.heap.vectorlike_mut(seq) {
            Vectorlike::Vector(v) => {
                v.reverse();
                return Ok(seq);
            }
            Vectorlike::BoolVector(b) => {
                b.bits.reverse();
                return Ok(seq);
            }
            _ => {}
        }
    }
    Err(lisp.wrong_type(sym::ARRAYP, seq))
}

fn f_reverse(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let seq = args[0];
    if seq.is_nil() {
        return Ok(seq);
    }
    if seq.is_cons() {
        return lisp.reverse_list(seq);
    }
    if seq.is_string() {
        let chars: Vec<u32> = lisp.string_char_vec(seq).into_iter().rev().collect();
        let multibyte = lisp.string_multibyte(seq);
        return Ok(lisp.string_from_chars(&chars, multibyte));
    }
    if seq.is_vectorlike() {
        match lisp.heap.vectorlike_ref(seq) {
            Vectorlike::Vector(v) => {
                let r: Vec<Value> = v.iter().rev().copied().collect();
                return Ok(lisp.vector(r));
            }
            Vectorlike::BoolVector(b) => {
                let r: Vec<bool> = b.bits.iter().rev().copied().collect();
                return Ok(lisp.make_bool_vector(r));
            }
            _ => {}
        }
    }
    Err(lisp.wrong_type(sym::SEQUENCEP, seq))
}

impl Lisp {
    pub fn sort_values(
        &mut self,
        items: &mut [Value],
        lessp: Value,
        key: Value,
        reverse: bool,
    ) -> LispResult<()> {
        let base = self.stack.len();
        self.stack.extend_from_slice(items);
        self.stack.push(lessp);
        self.stack.push(key);
        let keys: Vec<Value> = if key.is_nil() {
            items.to_vec()
        } else {
            let mut ks = Vec::with_capacity(items.len());
            for &item in items.iter() {
                match self.funcall(key, &[item]) {
                    Ok(k) => {
                        self.stack.push(k);
                        ks.push(k);
                    }
                    Err(e) => {
                        self.stack.truncate(base);
                        return Err(e);
                    }
                }
            }
            ks
        };
        let mut order: Vec<usize> = (0..items.len()).collect();
        let mut error = None;
        let mut less = |lisp: &mut Lisp, a: Value, b: Value| -> bool {
            if error.is_some() {
                return false;
            }
            let r = if lessp.is_nil() {
                lisp.value_cmp(a, b, 200).map(|o| o == Ordering::Less)
            } else {
                lisp.funcall(lessp, &[a, b]).map(|v| !v.is_nil())
            };
            match r {
                Ok(v) => v,
                Err(e) => {
                    error = Some(e);
                    false
                }
            }
        };
        order.sort_by(|&i, &j| {
            let (a, b) = if reverse {
                (keys[j], keys[i])
            } else {
                (keys[i], keys[j])
            };
            if less(self, a, b) {
                Ordering::Less
            } else if less(self, b, a) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        self.stack.truncate(base);
        if let Some(e) = error {
            return Err(e);
        }
        let sorted: Vec<Value> = order.iter().map(|&i| items[i]).collect();
        items.copy_from_slice(&sorted);
        Ok(())
    }
}

fn f_sort(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let seq = args[0];
    let mut key = Value::NIL;
    let mut lessp = Value::NIL;
    let mut inplace = false;
    let mut reverse = false;
    if args.len() == 2 {
        lessp = args[1];
        inplace = true;
    } else if args.len().is_multiple_of(2) {
        return Err(lisp.error("Invalid argument list"));
    } else {
        let mut i = 1;
        while i < args.len() - 1 {
            let (k, v) = (args[i], args[i + 1]);
            if k == sym::K_KEY {
                key = v;
            } else if k == sym::K_LESSP {
                lessp = v;
            } else if k == sym::K_IN_PLACE {
                inplace = !v.is_nil();
            } else if k == sym::K_REVERSE {
                reverse = !v.is_nil();
            } else {
                return Err(lisp.signal_error("Invalid keyword argument", k));
            }
            i += 2;
        }
    }
    if seq.is_cons() {
        let mut items = lisp.list_to_vec(seq)?;
        if items.len() < 2 {
            return Ok(if inplace { seq } else { lisp.list(&items) });
        }
        lisp.sort_values(&mut items, lessp, key, reverse)?;
        if inplace {
            let mut tail = seq;
            for item in items {
                lisp.heap.set_car(tail, item);
                tail = lisp.xcdr(tail);
            }
            return Ok(seq);
        }
        return Ok(lisp.list(&items));
    }
    if seq.is_nil() {
        return Ok(seq);
    }
    if lisp.is_vector(seq) {
        let target = if inplace {
            seq
        } else {
            f_copy_sequence(lisp, &[seq])?
        };
        let mut items = lisp.vector_items(target).to_vec();
        lisp.stack.push(target);
        let r = lisp.sort_values(&mut items, lessp, key, reverse);
        lisp.stack.pop();
        r?;
        if let Vectorlike::Vector(v) = lisp.heap.vectorlike_mut(target) {
            *v = items;
        }
        return Ok(target);
    }
    Err(lisp.wrong_type(sym::LIST_OR_VECTOR_P, seq))
}

fn plist_get_with(lisp: &mut Lisp, plist: Value, prop: Value, predicate: Value) -> LispResult {
    if predicate.is_nil() || predicate == sym::EQ {
        let mut tail = plist;
        while tail.is_cons() {
            let rest = lisp.xcdr(tail);
            if !rest.is_cons() {
                break;
            }
            if lisp.eq(lisp.xcar(tail), prop) {
                return Ok(lisp.xcar(rest));
            }
            tail = lisp.xcdr(rest);
        }
        return Ok(Value::NIL);
    }
    let mut tail = plist;
    while tail.is_cons() {
        let rest = lisp.xcdr(tail);
        if !rest.is_cons() {
            break;
        }
        let k = lisp.xcar(tail);
        if !lisp.funcall(predicate, &[k, prop])?.is_nil() {
            return Ok(lisp.xcar(rest));
        }
        tail = lisp.xcdr(rest);
    }
    Ok(Value::NIL)
}

fn f_plist_get(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    plist_get_with(lisp, args[0], args[1], args[2])
}

fn f_get(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.maybe_remove_pos_from_symbol(args[0]);
    let symbol = lisp.check_symbol(symbol)?;
    let overriding = lisp.plain_value(sym::OVERRIDING_PLIST_ENVIRONMENT);
    if !overriding.is_nil() {
        let entry = lisp.assq(symbol, overriding);
        if !entry.is_nil() {
            let plist = lisp.xcdr(entry);
            let mut tail = plist;
            while tail.is_cons() {
                let rest = lisp.xcdr(tail);
                if !rest.is_cons() {
                    break;
                }
                if lisp.xcar(tail) == args[1] {
                    return Ok(lisp.xcar(rest));
                }
                tail = lisp.xcdr(rest);
            }
        }
    }
    Ok(lisp.get(symbol, args[1]))
}

fn f_plist_put(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (plist, prop, val, predicate) = (args[0], args[1], args[2], args[3]);
    if predicate.is_nil() || predicate == sym::EQ {
        return lisp.plist_put_eq(plist, prop, val);
    }
    let mut prev = Value::NIL;
    let mut tail = plist;
    while tail.is_cons() {
        let rest = lisp.xcdr(tail);
        if !rest.is_cons() {
            return Err(lisp.wrong_type(sym::PLISTP, plist));
        }
        let k = lisp.xcar(tail);
        if !lisp.funcall(predicate, &[k, prop])?.is_nil() {
            lisp.heap.set_car(rest, val);
            return Ok(plist);
        }
        prev = tail;
        tail = lisp.xcdr(rest);
    }
    let new = lisp.list(&[prop, val]);
    if prev.is_nil() {
        return Ok(new);
    }
    let last = lisp.xcdr(prev);
    lisp.heap.set_cdr(last, new);
    Ok(plist)
}

fn f_put(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let symbol = lisp.maybe_remove_pos_from_symbol(args[0]);
    let symbol = lisp.check_symbol(symbol)?;
    let plist = lisp.heap.symbol_ref(symbol).plist;
    let new = lisp.plist_put_eq(plist, args[1], args[2])?;
    lisp.heap.symbol_mut(symbol).plist = new;
    Ok(args[2])
}

fn f_plist_member(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (plist, prop, predicate) = (args[0], args[1], args[2]);
    let mut tail = plist;
    while tail.is_cons() {
        let k = lisp.xcar(tail);
        let matched = if predicate.is_nil() || predicate == sym::EQ {
            lisp.eq(k, prop)
        } else {
            !lisp.funcall(predicate, &[k, prop])?.is_nil()
        };
        if matched {
            return Ok(tail);
        }
        tail = lisp.xcdr(tail);
        if !tail.is_cons() {
            break;
        }
        tail = lisp.xcdr(tail);
    }
    Ok(Value::NIL)
}

fn f_eql(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.eql(args[0], args[1])))
}

fn f_equal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.equal(args[0], args[1])?))
}

fn f_equal_including_properties(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.internal_equal(args[0], args[1], true, 0)?))
}

fn f_value_lt(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(
        lisp.value_cmp(args[0], args[1], 200)? == Ordering::Less,
    ))
}

fn f_fillarray(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (array, item) = (args[0], args[1]);
    if array.is_string() {
        let c = lisp.check_character(item)?;
        let n = lisp.string_chars(array);
        let multibyte = lisp.string_multibyte(array) || (c >= 0x80 && !character::char_byte8_p(c));
        let new = lisp.string_from_chars(&vec![c; n], multibyte);
        let s = lisp.heap.string_ref(new).clone();
        let target = lisp.heap.string_mut(array);
        target.data = s.data;
        target.multibyte = s.multibyte;
        return Ok(array);
    }
    let is_char_table = lisp.is_char_table(array);
    if is_char_table {
        lisp.char_table_fill(array, item);
        return Ok(array);
    }
    if array.is_vectorlike() {
        match lisp.heap.vectorlike_mut(array) {
            Vectorlike::Vector(v) | Vectorlike::Record(v) => {
                v.fill(item);
                return Ok(array);
            }
            Vectorlike::BoolVector(b) => {
                b.bits.fill(!item.is_nil());
                return Ok(array);
            }
            _ => {}
        }
    }
    Err(lisp.wrong_type(sym::ARRAYP, array))
}

fn f_clear_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    let n = lisp.string_bytes(s).len();
    let ls = lisp.heap.string_mut(s);
    ls.data = vec![0; n];
    ls.chars = n;
    ls.multibyte = false;
    Ok(Value::NIL)
}

fn f_nconc(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut val = Value::NIL;
    let mut last_cons = Value::NIL;
    for (i, &arg) in args.iter().enumerate() {
        if arg.is_nil() {
            continue;
        }
        if val.is_nil() {
            val = arg;
        } else {
            lisp.heap.set_cdr(last_cons, arg);
        }
        if i == args.len() - 1 {
            break;
        }
        if !arg.is_cons() {
            return Err(lisp.wrong_type(sym::CONSP, arg));
        }
        let mut tail = arg;
        while lisp.xcdr(tail).is_cons() {
            tail = lisp.xcdr(tail);
        }
        last_cons = tail;
    }
    Ok(val)
}

fn f_mapconcat(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, sequence, separator) = (args[0], args[1], args[2]);
    let results = lisp.mapcar_vec(function, sequence)?;
    let base = lisp.stack.len();
    lisp.stack.extend_from_slice(&results);
    let separator = if separator.is_nil() {
        lisp.empty_unibyte
    } else {
        separator
    };
    let mut parts = Vec::with_capacity(results.len() * 2);
    for (i, r) in results.iter().enumerate() {
        if i > 0 {
            parts.push(separator);
        }
        parts.push(*r);
    }
    let r = lisp.concat_to_string(&parts);
    lisp.stack.truncate(base);
    r
}

fn f_mapcar(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let results = lisp.mapcar_vec(args[0], args[1])?;
    Ok(lisp.list(&results))
}

fn f_mapc(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, sequence) = (args[0], args[1]);
    let items = lisp.sequence_elements(sequence)?;
    let base = lisp.stack.len();
    lisp.stack.push(function);
    lisp.stack.push(sequence);
    lisp.stack.extend_from_slice(&items);
    for item in items {
        if let Err(e) = lisp.funcall(function, &[item]) {
            lisp.stack.truncate(base);
            return Err(e);
        }
    }
    lisp.stack.truncate(base);
    Ok(sequence)
}

fn f_mapcan(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let results = lisp.mapcar_vec(args[0], args[1])?;
    let base = lisp.stack.len();
    lisp.stack.extend_from_slice(&results);
    let r = f_nconc(lisp, &results);
    lisp.stack.truncate(base);
    r
}

fn f_featurep(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let feature = lisp.check_symbol(args[0])?;
    let features = lisp.plain_value(sym::FEATURES);
    let mut tem = lisp.memq(feature, features);
    if !tem.is_nil() && !args[1].is_nil() {
        let subs = lisp.get(feature, sym::SUBFEATURES);
        tem = lisp.member(args[1], subs)?;
    }
    Ok(bool_value(!tem.is_nil()))
}

pub(crate) fn f_provide(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let feature = lisp.check_symbol(args[0])?;
    let subfeatures = lisp.check_list(args[1])?;
    let features = lisp.plain_value(sym::FEATURES);
    let queue = lisp.plain_value(sym::AUTOLOAD_QUEUE);
    if !queue.is_nil() {
        let entry = lisp.cons(Value::fixnum(0), features);
        let queue = lisp.cons(entry, queue);
        lisp.set_plain_value(sym::AUTOLOAD_QUEUE, queue);
    }
    if lisp.memq(feature, features).is_nil() {
        let features = lisp.cons(feature, features);
        lisp.set_plain_value(sym::FEATURES, features);
    }
    if !subfeatures.is_nil() {
        lisp.put(feature, sym::SUBFEATURES, subfeatures);
    }
    let entry = lisp.cons(sym::PROVIDE, feature);
    lisp.loadhist_attach(entry);
    let alist = lisp.plain_value(sym::AFTER_LOAD_ALIST);
    let tem = lisp.assq(feature, alist);
    if tem.is_cons() {
        let funcs = lisp.xcdr(tem);
        f_mapc(lisp, &[sym::FUNCALL, funcs])?;
    }
    Ok(feature)
}

fn f_require(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (feature, filename, noerror) = (args[0], args[1], args[2]);
    let feature = lisp.check_symbol(feature)?;
    let mut from_file = lisp.load_in_progress();
    if !from_file {
        let mut tail = lisp.plain_value(sym::CURRENT_LOAD_LIST);
        while tail.is_cons() {
            if lisp.xcdr(tail).is_nil() && lisp.xcar(tail).is_string() {
                from_file = true;
            }
            tail = lisp.xcdr(tail);
        }
    }
    if from_file {
        let tem = lisp.cons(sym::REQUIRE, feature);
        let current = lisp.plain_value(sym::CURRENT_LOAD_LIST);
        if lisp.member(tem, current)?.is_nil() {
            lisp.loadhist_attach(tem);
        }
    }
    let features = lisp.plain_value(sym::FEATURES);
    if !lisp.memq(feature, features).is_nil() {
        return Ok(feature);
    }
    let count = lisp.specpdl.len();
    let nesting_list = lisp.plain_value(sym::REQUIRE_NESTING_LIST);
    let mut nesting = 0;
    let mut tem = nesting_list;
    while tem.is_cons() {
        if lisp.xcar(tem) == feature {
            nesting += 1;
        }
        tem = lisp.xcdr(tem);
    }
    if nesting > 3 {
        let name = lisp.symbol_name_string(feature);
        return Err(lisp.error(&format!("Recursive `require' for feature `{name}'")));
    }
    let list = lisp.cons(feature, nesting_list);
    lisp.specbind(sym::REQUIRE_NESTING_LIST, list)?;
    let file = if filename.is_nil() {
        lisp.symbol_name(feature)
    } else {
        filename
    };
    let must_suffix = if filename.is_nil() {
        sym::T
    } else {
        Value::NIL
    };
    let loaded = lisp.load_for_require(file, noerror, must_suffix);
    let tem = lisp.unbind_to_value(count, loaded)?;
    if tem.is_nil() {
        return Ok(Value::NIL);
    }
    let features = lisp.plain_value(sym::FEATURES);
    if lisp.memq(feature, features).is_nil() {
        if !noerror.is_nil() {
            return Ok(Value::NIL);
        }
        let file = lisp.string_to_rust(file);
        let name = lisp.symbol_name_string(feature);
        return Err(lisp.error(&format!(
            "Loading file {file} failed to provide feature `{name}'"
        )));
    }
    Ok(feature)
}

fn f_load_average(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Err(lisp.error("load-average not implemented for this operating system"))
}

fn f_locale_info(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64_encode(input: &[u8], line_break: bool, url: bool, pad: bool) -> Vec<u8> {
    let table = if url { B64URL } else { B64 };
    let mut out = Vec::with_capacity(input.len() * 4 / 3 + 4);
    let mut counter = 0;
    for chunk in input.chunks(3) {
        if line_break && counter == 19 {
            out.push(b'\n');
            counter = 0;
        }
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        out.push(table[(b[0] >> 2) as usize]);
        out.push(table[(((b[0] & 3) << 4) | (b[1] >> 4)) as usize]);
        if chunk.len() > 1 {
            out.push(table[(((b[1] & 0xF) << 2) | (b[2] >> 6)) as usize]);
        } else if pad {
            out.push(b'=');
        }
        if chunk.len() > 2 {
            out.push(table[(b[2] & 0x3F) as usize]);
        } else if pad {
            out.push(b'=');
        }
        counter += 1;
    }
    out
}

fn base64_encode_string(lisp: &mut Lisp, args: &[Value], url: bool) -> LispResult {
    let s = lisp.check_string(args[0])?;
    let bytes = if lisp.string_multibyte(s) {
        let chars = lisp.string_char_vec(s);
        let mut out = Vec::with_capacity(chars.len());
        for c in chars {
            if c < 0x80 || character::char_byte8_p(c) {
                out.push(character::char_to_byte8(c));
            } else if c < 0x100 {
                out.push(c as u8);
            } else {
                return Err(lisp.error("Multibyte character in data for base64 encoding"));
            }
        }
        out
    } else {
        lisp.string_bytes(s).to_vec()
    };
    let line_break = !url && args[1].is_nil();
    let pad = !url || args[1].is_nil();
    let encoded = base64_encode(&bytes, line_break, url, pad);
    Ok(lisp.unibyte_string(&encoded))
}

fn f_base64_encode_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    base64_encode_string(lisp, args, false)
}

fn f_base64url_encode_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    base64_encode_string(lisp, args, true)
}

fn f_base64_decode_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    let url = !args[1].is_nil();
    let ignore_invalid = !args[2].is_nil();
    let input = lisp.string_bytes(s).to_vec();
    let decode = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' if !url => Some(62),
            b'/' if !url => Some(63),
            b'-' if url => Some(62),
            b'_' if url => Some(63),
            _ => None,
        }
    };
    let mut bits = 0u32;
    let mut nbits = 0;
    let mut out = Vec::new();
    for &c in &input {
        if c == b'=' {
            break;
        }
        if c.is_ascii_whitespace() {
            continue;
        }
        match decode(c) {
            Some(v) => {
                bits = (bits << 6) | v as u32;
                nbits += 6;
                if nbits >= 8 {
                    nbits -= 8;
                    out.push((bits >> nbits) as u8);
                }
            }
            None if ignore_invalid => {}
            None => return Err(lisp.error("Invalid base64 data")),
        }
    }
    Ok(lisp.unibyte_string(&out))
}

fn f_sxhash_eq(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(sxhash_value(hash_u64(
        lisp.maybe_remove_pos_from_symbol(args[0]).bits(),
    )))
}

fn f_sxhash_eql(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(sxhash_value(lisp.sxhash_eql(args[0])))
}

fn f_sxhash_equal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(sxhash_value(lisp.sxhash_equal(args[0], 0)))
}

fn f_make_hash_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut test = sym::EQL;
    let mut weakness = Weakness::None;
    let mut i = 0;
    if args.len() == 1 && args[0].is_fixnum() {
        return Ok(lisp.make_hash_table_with(test, weakness));
    }
    while i < args.len() {
        let key = args[i];
        if i + 1 >= args.len() {
            return Err(lisp.signal_error("Invalid argument list", key));
        }
        let value = args[i + 1];
        if key == sym::K_TEST {
            test = if value.is_nil() { sym::EQL } else { value };
            if test != sym::EQ && test != sym::EQL && test != sym::EQUAL {
                let spec = lisp.get(test, sym::HASH_TABLE_TEST);
                if !spec.is_cons() {
                    return Err(lisp.signal_error("Invalid hash table test", test));
                }
            }
        } else if key == sym::K_WEAKNESS {
            weakness = if value.is_nil() {
                Weakness::None
            } else if value == sym::KEY {
                Weakness::Key
            } else if value == sym::VALUE {
                Weakness::Value
            } else if value == sym::KEY_OR_VALUE {
                Weakness::KeyOrValue
            } else if value == sym::KEY_AND_VALUE || value == sym::T {
                Weakness::KeyAndValue
            } else {
                return Err(lisp.signal_error("Invalid hash table weakness", value));
            };
        } else if key == sym::K_SIZE {
            if !value.is_nil() && !value.as_fixnum().is_some_and(|n| n >= 0) {
                return Err(lisp.signal_error("Invalid hash table size", value));
            }
        } else if key == sym::K_REHASH_SIZE
            || key == sym::K_REHASH_THRESHOLD
            || key == sym::K_PURECOPY
        {
        } else {
            return Err(lisp.signal_error("Invalid argument list", key));
        }
        i += 2;
    }
    Ok(lisp.make_hash_table_with(test, weakness))
}

fn f_copy_hash_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.hash_table_ref(args[0])?;
    let mut copy = lisp.hash_table(args[0]).clone();
    copy.mutable = true;
    Ok(lisp.heap.vectorlike(Vectorlike::HashTable(Box::new(copy))))
}

fn f_hash_table_count(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.hash_table_ref(args[0])?;
    Ok(Value::fixnum(lisp.hash_table(args[0]).count as i64))
}

fn f_hash_table_rehash_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.hash_table_ref(args[0])?;
    Ok(lisp.float(1.5))
}

fn f_hash_table_rehash_threshold(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.hash_table_ref(args[0])?;
    Ok(lisp.float(0.8125))
}

fn f_hash_table_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.hash_table_ref(args[0])?;
    Ok(Value::fixnum(lisp.hash_table(args[0]).count as i64))
}

fn f_hash_table_test(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.hash_table_ref(args[0])?;
    Ok(lisp.hash_table(args[0]).test)
}

pub fn weakness_symbol(w: Weakness) -> Value {
    match w {
        Weakness::None => Value::NIL,
        Weakness::Key => sym::KEY,
        Weakness::Value => sym::VALUE,
        Weakness::KeyOrValue => sym::KEY_OR_VALUE,
        Weakness::KeyAndValue => sym::KEY_AND_VALUE,
    }
}

fn f_hash_table_weakness(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.hash_table_ref(args[0])?;
    Ok(weakness_symbol(lisp.hash_table(args[0]).weakness))
}

fn f_hash_table_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(bool_value(lisp.is_hash_table(args[0])))
}

fn f_clrhash(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.hash_table_ref(args[0])?;
    lisp.hash_table_mut(args[0]).clear();
    Ok(args[0])
}

fn f_gethash(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.gethash(args[0], args[1], args[2])
}

fn f_puthash(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.puthash(args[0], args[1], args[2])
}

fn f_remhash(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.hash_table_ref(args[1])?;
    if let Some(slot) = lisp.hash_lookup(args[1], args[0])? {
        lisp.hash_table_mut(args[1]).remove_slot(slot);
    }
    Ok(Value::NIL)
}

fn f_maphash(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, table) = (args[0], args[1]);
    lisp.hash_table_ref(table)?;
    let base = lisp.stack.len();
    lisp.stack.push(table);
    lisp.stack.push(function);
    let mut slot = 0;
    loop {
        let entry = {
            let h = lisp.hash_table(table);
            if slot >= h.entries.len() {
                break;
            }
            h.entries[slot]
        };
        slot += 1;
        if let Some((k, v)) = entry
            && let Err(e) = lisp.funcall(function, &[k, v])
        {
            lisp.stack.truncate(base);
            return Err(e);
        }
    }
    lisp.stack.truncate(base);
    Ok(Value::NIL)
}

fn f_define_hash_table_test(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (name, test, hash) = (args[0], args[1], args[2]);
    let spec = lisp.list(&[test, hash]);
    lisp.put(name, sym::HASH_TABLE_TEST, spec);
    Ok(spec)
}

fn f_secure_hash_algorithms(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let names = ["md5", "sha1", "sha224", "sha256", "sha384", "sha512"];
    let symbols: Vec<Value> = names.iter().map(|n| lisp.intern(n)).collect();
    Ok(lisp.list(&symbols))
}

fn f_string_search(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (needle, haystack, start) = (args[0], args[1], args[2]);
    lisp.check_string(needle)?;
    lisp.check_string(haystack)?;
    let n = lisp.string_char_vec(needle);
    let h = lisp.string_char_vec(haystack);
    let start_pos = if start.is_nil() {
        0
    } else {
        let s = lisp.check_fixnum(start)?;
        if s < 0 || s as usize > h.len() {
            return Err(lisp.signal_list(sym::ARGS_OUT_OF_RANGE, &[start]));
        }
        s as usize
    };
    let n_multi = lisp.string_multibyte(needle);
    let h_multi = lisp.string_multibyte(haystack);
    let normalize = |chars: Vec<u32>, multi: bool| -> Vec<u32> {
        if multi {
            chars
        } else {
            chars
                .into_iter()
                .map(|c| {
                    if c >= 0x80 {
                        character::byte8_to_char(c as u8)
                    } else {
                        c
                    }
                })
                .collect()
        }
    };
    let n = normalize(n, n_multi);
    let h = normalize(h, h_multi);
    if n.is_empty() {
        return Ok(Value::fixnum(start_pos as i64));
    }
    if n.len() > h.len() {
        return Ok(Value::NIL);
    }
    for i in start_pos..=h.len() - n.len() {
        if h[i..i + n.len()] == n[..] {
            return Ok(Value::fixnum(i as i64));
        }
    }
    Ok(Value::NIL)
}

fn f_object_intervals(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let object = args[0];
    if !object.is_string() {
        return Ok(Value::NIL);
    }
    let runs = lisp.heap.string_ref(object).props.clone();
    let mut items = Vec::new();
    for run in runs {
        let start = Value::fixnum(run.start as i64);
        let end = Value::fixnum(run.end as i64);
        items.push(lisp.list(&[start, end, run.plist]));
    }
    Ok(lisp.list(&items))
}

pub fn integer_to_usize(lisp: &mut Lisp, v: Value) -> LispResult<usize> {
    match v.as_fixnum() {
        Some(n) if n >= 0 => Ok(n as usize),
        _ => {
            if let Some(b) = lisp.bignum_ref(v)
                && let Some(n) = b.to_usize()
            {
                return Ok(n);
            }
            Err(lisp.wrong_type(sym::WHOLENUMP, v))
        }
    }
}

pub fn num_as_value(lisp: &mut Lisp, n: Num) -> Value {
    lisp.make_num(n)
}
