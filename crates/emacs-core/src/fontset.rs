use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

const FONTSET_ID: usize = 0;
const FONTSET_NAME: usize = 1;
const FONTSET_ASCII: usize = 2;
const FONTSET_BASE: usize = 3;

pub struct Fontsets {
    pub table: Vec<Value>,
    pub next_fontset_id: usize,
    pub default_fontset: Value,
    pub cached_fontset_data: Value,
    pub auto_fontset_alist: Value,
}

impl Default for Fontsets {
    fn default() -> Self {
        Fontsets {
            table: Vec::new(),
            next_fontset_id: 0,
            default_fontset: Value::NIL,
            cached_fontset_data: Value::NIL,
            auto_fontset_alist: Value::NIL,
        }
    }
}

impl Fontsets {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&self.table);
        out.push(self.default_fontset);
        out.push(self.cached_fontset_data);
        out.push(self.auto_fontset_alist);
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("query-fontset", 1, 2, f_query_fontset),
    Subr::fixed("fontset-list", 0, 0, f_fontset_list),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.put(sym::FONTSET, sym::CHAR_TABLE_EXTRA_SLOTS, Value::fixnum(8));
    lisp.put(
        sym::FONTSET_INFO,
        sym::CHAR_TABLE_EXTRA_SLOTS,
        Value::fixnum(1),
    );
    lisp.fontsets.cached_fontset_data = Value::NIL;
    lisp.fontsets.table = vec![Value::NIL; 32];
    let default = lisp
        .make_char_table(sym::FONTSET, Value::NIL)
        .expect("default fontset");
    lisp.fontsets.default_fontset = default;
    lisp.char_table_mut(default).extras[FONTSET_ID] = Value::fixnum(0);
    let name = lisp.string("-*-*-*-*-*-*-*-*-*-*-*-*-fontset-default");
    lisp.char_table_mut(default).extras[FONTSET_NAME] = name;
    lisp.fontsets.table[0] = default;
    lisp.fontsets.next_fontset_id = 1;
    lisp.fontsets.auto_fontset_alist = Value::NIL;
    lisp.defvar(sym::FONT_ENCODING_CHARSET_ALIST, Value::NIL);
    lisp.defvar(sym::USE_DEFAULT_ASCENT, Value::NIL);
    lisp.defvar_bool(sym::USE_DEFAULT_FONT_FOR_SYMBOLS, true);
    lisp.defvar(sym::IGNORE_RELATIVE_COMPOSITION, Value::NIL);
    lisp.defvar(sym::ALTERNATE_FONTNAME_ALIST, Value::NIL);
    let alias = lisp.string("fontset-default");
    let cell = lisp.cons(name, alias);
    let alist = lisp.list(&[cell]);
    lisp.defvar(sym::FONTSET_ALIAS_ALIST, alist);
    lisp.defvar(sym::VERTICAL_CENTERING_FONT_REGEXP, Value::NIL);
    lisp.defvar(sym::OTF_SCRIPT_ALIST, Value::NIL);
}

impl Lisp {
    fn fontset_extra(&self, fontset: Value, idx: usize) -> Value {
        self.char_table(fontset).extras[idx]
    }

    fn base_fontset_p(&self, fontset: Value) -> bool {
        self.fontset_extra(fontset, FONTSET_BASE).is_nil()
    }

    pub fn fontset_name(&self, id: usize) -> Value {
        self.fontset_extra(self.fontsets.table[id], FONTSET_NAME)
    }

    pub fn fontset_ascii(&self, id: usize) -> Value {
        let elt = self.fontset_extra(self.fontsets.table[id], FONTSET_ASCII);
        if elt.is_cons() { self.xcar(elt) } else { elt }
    }

    fn fontset_pattern_regexp(&mut self, pattern: Value) -> Value {
        let bytes = c_bytes(self.string_bytes(pattern)).to_vec();
        if !bytes.contains(&b'*') && !bytes.contains(&b'?') {
            return Value::NIL;
        }
        let cached = self.fontsets.cached_fontset_data;
        if !cached.is_cons() || c_bytes(self.string_bytes(self.xcar(cached))) != bytes.as_slice() {
            let ndashes = bytes.iter().filter(|&&c| c == b'-').count();
            let mut regex = vec![b'^'];
            for &c in &bytes {
                match c {
                    b'*' => {
                        if ndashes < 14 {
                            regex.push(b'.');
                        } else {
                            regex.extend_from_slice(b"[^-]");
                        }
                        regex.push(b'*');
                    }
                    b'?' => regex.push(b'.'),
                    b'[' | b'.' | b'\\' | b'+' | b'^' | b'$' => {
                        regex.push(b'\\');
                        regex.push(c);
                    }
                    _ => regex.push(c),
                }
            }
            regex.push(b'$');
            let name = self.make_string(&bytes);
            self.stack.push(name);
            let regex = self.make_string(&regex);
            self.stack.pop();
            self.fontsets.cached_fontset_data = self.cons(name, regex);
        }
        self.xcdr(self.fontsets.cached_fontset_data)
    }

    pub fn fs_query_fontset(&mut self, name: Value, name_pattern: i32) -> LispResult<i64> {
        let mut name = crate::casefiddle::f_downcase(self, &[name])?;
        let mut name_pattern = name_pattern;
        let base = self.stack.len();
        self.stack.push(name);
        if name_pattern != 1 {
            let alist = self.find_symbol_value(sym::FONTSET_ALIAS_ALIST);
            let mut tem = crate::fns::f_rassoc(self, &[name, alist])?;
            if tem.is_nil() {
                let alist = self.find_symbol_value(sym::FONTSET_ALIAS_ALIST);
                tem = crate::fns::f_assoc(self, &[name, alist, Value::NIL])?;
            }
            if tem.is_cons() && self.xcar(tem).is_string() {
                name = self.xcar(tem);
            } else if name_pattern == 0 {
                let tem = self.fontset_pattern_regexp(name);
                if tem.is_string() {
                    name = tem;
                    name_pattern = 1;
                }
            }
            self.stack.push(name);
        }
        let mut found = -1;
        for i in 0..self.fontsets.table.len() {
            let fontset = self.fontsets.table[i];
            if fontset.is_nil() || !self.base_fontset_p(fontset) {
                continue;
            }
            let this_name = self.fontset_extra(fontset, FONTSET_NAME);
            let matched = if name_pattern == 1 {
                let canon = self.ascii_canon_table;
                match self.fast_string_match(name, this_name, canon) {
                    Ok(pos) => pos >= 0,
                    Err(e) => {
                        self.stack.truncate(base);
                        return Err(e);
                    }
                }
            } else {
                c_bytes(self.string_bytes(name))
                    .eq_ignore_ascii_case(c_bytes(self.string_bytes(this_name)))
            };
            if matched {
                found = i as i64;
                break;
            }
        }
        self.stack.truncate(base);
        Ok(found)
    }
}

pub fn c_bytes(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(n) => &bytes[..n],
        None => bytes,
    }
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

pub(crate) fn f_query_fontset(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (pattern, regexpp) = (args[0], arg(args, 1));
    lisp.check_window_system(None)?;
    lisp.check_string(pattern)?;
    if lisp.string_chars(pattern) == 0 {
        return Ok(Value::NIL);
    }
    let id = lisp.fs_query_fontset(pattern, i32::from(!regexpp.is_nil()))?;
    if id < 0 {
        return Ok(Value::NIL);
    }
    Ok(lisp.fontset_name(id as usize))
}

fn f_fontset_list(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let mut list = Value::NIL;
    for i in 0..lisp.fontsets.table.len() {
        let fontset = lisp.fontsets.table[i];
        if !fontset.is_nil() && lisp.base_fontset_p(fontset) {
            let name = lisp.fontset_extra(fontset, FONTSET_NAME);
            list = lisp.cons(name, list);
        }
    }
    Ok(list)
}
