use std::collections::HashMap;
use std::io::Write;

use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::{Interned, Vectorlike, Weakness};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::{Tag, Value};

pub const SUBRS: &[Subr] = &[
    Subr::fixed("write-char", 1, 2, f_write_char),
    Subr::fixed("terpri", 0, 2, f_terpri),
    Subr::fixed("prin1", 1, 3, f_prin1),
    Subr::fixed("prin1-to-string", 1, 3, f_prin1_to_string),
    Subr::fixed("error-message-string", 1, 1, f_error_message_string),
    Subr::fixed("princ", 1, 2, f_princ),
    Subr::fixed("print", 1, 2, f_print),
    Subr::fixed(
        "external-debugging-output",
        1,
        1,
        f_external_debugging_output,
    ),
    Subr::fixed(
        "redirect-debugging-output",
        1,
        2,
        f_redirect_debugging_output,
    ),
    Subr::fixed("print--preprocess", 1, 1, f_print_preprocess),
    Subr::fixed("flush-standard-output", 0, 0, f_flush_standard_output),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::STANDARD_OUTPUT, sym::T);
    lisp.defvar(sym::FLOAT_OUTPUT_FORMAT, Value::NIL);
    lisp.defvar(sym::PRINT_LENGTH, Value::NIL);
    lisp.defvar(sym::PRINT_LEVEL, Value::NIL);
    lisp.defvar_bool(sym::PRINT_ESCAPE_NEWLINES, false);
    lisp.defvar_bool(sym::PRINT_ESCAPE_CONTROL_CHARACTERS, false);
    lisp.defvar_bool(sym::PRINT_ESCAPE_NONASCII, false);
    lisp.defvar_bool(sym::PRINT_ESCAPE_MULTIBYTE, false);
    lisp.defvar_bool(sym::PRINT_QUOTED, true);
    lisp.defvar(sym::PRINT_GENSYM, Value::NIL);
    lisp.defvar(sym::PRINT_CIRCLE, Value::NIL);
    lisp.defvar(sym::PRINT_CONTINUOUS_NUMBERING, Value::NIL);
    lisp.defvar(sym::PRINT_NUMBER_TABLE, Value::NIL);
    lisp.defvar(sym::PRINT_CHARSET_TEXT_PROPERTY, Value::NIL);
    lisp.defvar_bool(sym::PRINT_INTEGERS_AS_CHARACTERS, false);
    lisp.defvar(sym::PRINT_UNREADABLE_FUNCTION, Value::NIL);
    lisp.defvar(sym::PRINT_ESCAPE_NEWLINES, Value::NIL);
    lisp.defvar_bool(sym::PRINT_ESCAPE_NEWLINES, false);
}

pub struct Printer {
    out: Vec<u32>,
    depth: usize,
    being_printed: Vec<Value>,
    circle: Option<HashMap<Value, i64>>,
    next_label: i64,
    new_backquote_output: usize,
}

impl Printer {
    fn push_str(&mut self, s: &str) {
        self.out.extend(s.chars().map(|c| c as u32));
    }

    fn push(&mut self, c: u32) {
        self.out.push(c);
    }
}

impl Lisp {
    pub fn float_to_string(&mut self, data: f64) -> String {
        if data.is_infinite() {
            return if data > 0.0 {
                "1.0e+INF".to_string()
            } else {
                "-1.0e+INF".to_string()
            };
        }
        if data.is_nan() {
            let bits = data.to_bits();
            let mantissa = bits & 0x0007_FFFF_FFFF_FFFF;
            let negative = bits >> 63 == 1;
            return format!("{}{}.0e+NaN", if negative { "-" } else { "" }, mantissa);
        }
        let format = self.plain_value(sym::FLOAT_OUTPUT_FORMAT);
        let mut s = None;
        let mut width = 1usize;
        if format.is_string() {
            let spec = self.string_to_rust(format);
            if let Some((w, conv)) = parse_float_format(&spec) {
                width = w.unwrap_or(1);
                s = Some(format_c_float(data, w, conv));
            }
        }
        let mut s = s.unwrap_or_else(|| shortest_float(data));
        if width != 0 {
            let first = s.find(|c: char| !(c.is_ascii_digit() || c == '-'));
            match first {
                Some(i) if s.as_bytes()[i] == b'.' && i == s.len() - 1 => s.push('0'),
                None => s.push_str(".0"),
                _ => {}
            }
        }
        s
    }

    pub fn prin1_to_string(&mut self, value: Value) -> String {
        let chars = self.print_to_chars(value, true).unwrap_or_default();
        chars_to_display_string(&chars)
    }

    pub fn princ_to_string(&mut self, value: Value) -> String {
        let chars = self.print_to_chars(value, false).unwrap_or_default();
        chars_to_display_string(&chars)
    }

    pub fn print_to_chars(&mut self, value: Value, escape: bool) -> LispResult<Vec<u32>> {
        let base = self.stack.len();
        self.stack.push(value);
        let circle = if !self.plain_value(sym::PRINT_CIRCLE).is_nil() {
            Some(self.print_preprocess(value))
        } else {
            None
        };
        let mut p = Printer {
            out: Vec::new(),
            depth: 0,
            being_printed: Vec::new(),
            circle,
            next_label: 1,
            new_backquote_output: 0,
        };
        let r = self.print_object(&mut p, value, escape);
        self.stack.truncate(base);
        r?;
        Ok(p.out)
    }

    fn print_preprocess(&self, root: Value) -> HashMap<Value, i64> {
        let mut seen: HashMap<Value, u32> = HashMap::new();
        let mut work = vec![root];
        while let Some(v) = work.pop() {
            let interesting = v.is_cons()
                || (v.is_string() && !self.heap.string_ref(v).props.is_empty())
                || (v.is_symbol()
                    && !v.is_nil()
                    && self.heap.symbol_ref(v).interned == Interned::Uninterned
                    && !self.plain_value(sym::PRINT_GENSYM).is_nil())
                || (v.is_vectorlike()
                    && matches!(
                        self.heap.vectorlike_ref(v),
                        Vectorlike::Vector(_)
                            | Vectorlike::Record(_)
                            | Vectorlike::Closure(_)
                            | Vectorlike::HashTable(_)
                    ));
            if !interesting {
                continue;
            }
            let count = seen.entry(v).or_insert(0);
            *count += 1;
            if *count > 1 {
                continue;
            }
            if v.is_cons() {
                work.push(self.xcdr(v));
                work.push(self.xcar(v));
            } else if v.is_vectorlike() {
                match self.heap.vectorlike_ref(v) {
                    Vectorlike::Vector(items)
                    | Vectorlike::Record(items)
                    | Vectorlike::Closure(items) => {
                        work.extend(items.iter().rev().copied());
                    }
                    Vectorlike::HashTable(h) => {
                        for (k, val) in h.entries.iter().flatten() {
                            work.push(*val);
                            work.push(*k);
                        }
                    }
                    _ => {}
                }
            } else if v.is_string() {
                for run in &self.heap.string_ref(v).props {
                    work.push(run.plist);
                }
            }
        }
        seen.into_iter()
            .filter(|&(_, c)| c > 1)
            .map(|(v, _)| (v, 0))
            .collect()
    }

    fn print_circle_label(&mut self, p: &mut Printer, obj: Value) -> bool {
        let Some(table) = &mut p.circle else {
            return false;
        };
        let Some(n) = table.get_mut(&obj) else {
            return false;
        };
        if *n > 0 {
            let label = *n;
            p.push_str(&format!("#{label}#"));
            return true;
        }
        *n = p.next_label;
        p.next_label += 1;
        let label = *n;
        p.push_str(&format!("#{label}="));
        false
    }

    fn print_object(&mut self, p: &mut Printer, obj: Value, escape: bool) -> LispResult<()> {
        if p.depth > 200 {
            return Err(self.error("Apparently circular structure being printed"));
        }
        let needs_cycle_check = obj.is_cons()
            || (obj.is_vectorlike()
                && matches!(
                    self.heap.vectorlike_ref(obj),
                    Vectorlike::Vector(_) | Vectorlike::Record(_) | Vectorlike::Closure(_)
                ));
        if needs_cycle_check {
            if p.circle.is_some() {
                if self.print_circle_label(p, obj) {
                    return Ok(());
                }
            } else if let Some(i) = p.being_printed.iter().position(|&v| v == obj) {
                p.push_str(&format!("#{i}"));
                return Ok(());
            }
        }
        match obj.tag() {
            Tag::Fixnum => {
                let i = obj.as_fixnum().unwrap_or(0);
                if self.var_true(sym::PRINT_INTEGERS_AS_CHARACTERS)
                    && (0..=character::MAX_UNICODE_CHAR as i64).contains(&i)
                    && let Some(text) = integer_as_character(i as u32, escape)
                {
                    p.push_str(&text);
                } else {
                    p.push_str(&i.to_string());
                }
            }
            Tag::Float => {
                let f = self.heap.float_value(obj);
                let s = self.float_to_string(f);
                p.push_str(&s);
            }
            Tag::String => {
                if escape {
                    if p.circle.is_some()
                        && !self.heap.string_ref(obj).props.is_empty()
                        && self.print_circle_label(p, obj)
                    {
                        return Ok(());
                    }
                    self.print_string_escaped(p, obj)?;
                } else {
                    let chars = self.string_char_vec(obj);
                    let multibyte = self.string_multibyte(obj);
                    for c in chars {
                        if !multibyte && c >= 0x80 {
                            p.push(character::byte8_to_char(c as u8));
                        } else {
                            p.push(c);
                        }
                    }
                }
            }
            Tag::Symbol => self.print_symbol(p, obj, escape),
            Tag::Cons => self.print_cons(p, obj, escape)?,
            Tag::Vectorlike => self.print_vectorlike(p, obj, escape)?,
        }
        Ok(())
    }

    fn print_string_escaped(&mut self, p: &mut Printer, obj: Value) -> LispResult<()> {
        let props = self.heap.string_ref(obj).props.clone();
        let props: Vec<_> = props.into_iter().filter(|r| !r.plist.is_nil()).collect();
        if !props.is_empty() {
            p.push_str("#(");
        }
        let multibyte = self.string_multibyte(obj);
        let data = self.string_bytes(obj).to_vec();
        let escape_newlines = self.var_true(sym::PRINT_ESCAPE_NEWLINES);
        let escape_control = self.var_true(sym::PRINT_ESCAPE_CONTROL_CHARACTERS);
        let escape_nonascii = self.var_true(sym::PRINT_ESCAPE_NONASCII);
        let escape_multibyte = self.var_true(sym::PRINT_ESCAPE_MULTIBYTE);
        p.push('"' as u32);
        let mut need_nonhex = false;
        let mut i = 0;
        while i < data.len() {
            let (c, len) = if multibyte {
                character::string_char(&data[i..])
            } else {
                (data[i] as u32, 1)
            };
            i += len;
            if multibyte && character::char_byte8_p(c) {
                let byte = character::char_to_byte8(c);
                octal_out(p, byte as u32, data.get(i).copied());
                need_nonhex = false;
            } else if !multibyte && c >= 0x80 && escape_nonascii {
                octal_out(p, c, data.get(i).copied());
                need_nonhex = false;
            } else if multibyte && c >= 0x80 && escape_multibyte {
                p.push_str(&format!("\\x{c:04x}"));
                need_nonhex = true;
            } else {
                if (c as u8 as char).is_ascii_hexdigit() && c < 0x80 {
                    if need_nonhex {
                        p.push_str("\\ ");
                    }
                    p.push(c);
                } else if c == 10 && escape_newlines {
                    p.push_str("\\n");
                } else if c == 12 && escape_newlines {
                    p.push_str("\\f");
                } else if c == '"' as u32 || c == '\\' as u32 {
                    p.push('\\' as u32);
                    p.push(c);
                } else if escape_control && (c < 32 || c == 127) {
                    octal_out(p, c, data.get(i).copied());
                } else if !multibyte && c >= 0x80 {
                    p.push(character::byte8_to_char(c as u8));
                } else {
                    p.push(c);
                }
                need_nonhex = false;
            }
        }
        p.push('"' as u32);
        if !props.is_empty() {
            for run in props {
                p.push(' ' as u32);
                p.push_str(&run.start.to_string());
                p.push(' ' as u32);
                p.push_str(&run.end.to_string());
                p.push(' ' as u32);
                self.print_object(p, run.plist, true)?;
            }
            p.push(')' as u32);
        }
        Ok(())
    }

    fn print_symbol(&mut self, p: &mut Printer, obj: Value, escape: bool) {
        let name = self.symbol_name(obj);
        let data = self.string_bytes(name).to_vec();
        let multibyte = self.string_multibyte(name);
        let size = data.len();
        let at = |i: usize| data.get(i).copied().unwrap_or(0);
        let signedp = at(0) == b'-' || at(0) == b'+';
        let s = usize::from(signedp);
        let mut confusing = ((at(s).is_ascii_digit() || at(s) == b'.')
            && crate::lread::string_to_number(self, &data, 10).is_some_and(|(_, len)| len == size))
            || at(0) == b'?'
            || (at(0) == b'.' && !(size > 1 && at(1).is_ascii_alphabetic()));
        let gensym = !self.plain_value(sym::PRINT_GENSYM).is_nil();
        if gensym && self.heap.symbol_ref(obj).interned != Interned::InitialObarray && escape {
            p.push_str("#:");
        } else if size == 0 {
            p.push_str("##");
            return;
        }
        let chars: Vec<u32> = if multibyte {
            character::multibyte_chars(&data).collect()
        } else {
            data.iter().map(|&b| b as u32).collect()
        };
        for c in chars {
            if escape
                && (matches!(
                    c,
                    0x22 | 0x5C | 0x27 | 0x3B | 0x23 | 0x28 | 0x29 | 0x2C | 0x60 | 0x5B | 0x5D
                ) || c <= 0x20
                    || c == character::NO_BREAK_SPACE
                    || confusing)
            {
                p.push('\\' as u32);
                confusing = false;
            }
            p.push(c);
        }
    }

    fn print_cons(&mut self, p: &mut Printer, obj: Value, escape: bool) -> LispResult<()> {
        let level = self.plain_value(sym::PRINT_LEVEL);
        if let Some(l) = level.as_fixnum()
            && p.depth as i64 >= l
        {
            p.push_str("...");
            return Ok(());
        }
        let quoted = self.var_true(sym::PRINT_QUOTED);
        let car = self.xcar(obj);
        let cdr = self.xcdr(obj);
        let two = cdr.is_cons() && self.xcdr(cdr).is_nil();
        if quoted && two {
            let arg = self.xcar(cdr);
            if car == sym::QUOTE {
                p.push('\'' as u32);
                return self.print_object(p, arg, escape);
            }
            if car == sym::FUNCTION {
                p.push_str("#'");
                return self.print_object(p, arg, escape);
            }
            if car == sym::BACKQUOTE {
                p.push('`' as u32);
                p.new_backquote_output += 1;
                let r = self.print_object(p, arg, escape);
                p.new_backquote_output -= 1;
                return r;
            }
            if (car == sym::COMMA || car == sym::COMMA_AT) && p.new_backquote_output > 0 {
                self.print_symbol(p, car, false);
                p.new_backquote_output -= 1;
                let r = self.print_object(p, arg, escape);
                p.new_backquote_output += 1;
                return r;
            }
        }
        p.push('(' as u32);
        let max_len = self
            .plain_value(sym::PRINT_LENGTH)
            .as_fixnum()
            .filter(|&n| n >= 0)
            .map_or(usize::MAX, |n| n as usize);
        if max_len == 0 {
            p.push_str("...)");
            return Ok(());
        }
        p.depth += 1;
        p.being_printed.push(obj);
        let mut tail = obj;
        let mut count = 0usize;
        let mut tortoise = obj;
        let mut n = 2usize;
        let mut m = 2usize;
        let mut tortoise_idx = 0usize;
        loop {
            let item = self.xcar(tail);
            self.print_object(p, item, escape)?;
            count += 1;
            let next = self.xcdr(tail);
            if next.is_nil() {
                break;
            }
            if !next.is_cons() {
                p.push_str(" . ");
                self.print_object(p, next, escape)?;
                break;
            }
            if let Some(table) = &p.circle
                && table.contains_key(&next)
            {
                p.push_str(" . ");
                self.print_object(p, next, escape)?;
                break;
            }
            p.push(' ' as u32);
            if count >= max_len {
                p.push_str("...");
                break;
            }
            tail = next;
            n -= 1;
            if n == 0 {
                m <<= 1;
                n = m;
                tortoise = tail;
                tortoise_idx = count;
            } else if tail == tortoise {
                p.push_str(&format!(". #{}", tortoise_idx / 2));
                break;
            }
        }
        p.being_printed.pop();
        p.depth -= 1;
        p.push(')' as u32);
        Ok(())
    }

    fn print_vector_items(
        &mut self,
        p: &mut Printer,
        obj: Value,
        open: &str,
        close: &str,
        items: &[Value],
        escape: bool,
    ) -> LispResult<()> {
        let level = self.plain_value(sym::PRINT_LEVEL);
        if let Some(l) = level.as_fixnum()
            && p.depth as i64 >= l
        {
            p.push_str("...");
            return Ok(());
        }
        p.push_str(open);
        let max_len = self
            .plain_value(sym::PRINT_LENGTH)
            .as_fixnum()
            .filter(|&n| n >= 0)
            .map_or(usize::MAX, |n| n as usize);
        p.depth += 1;
        p.being_printed.push(obj);
        for (i, &item) in items.iter().enumerate() {
            if i > 0 || (open.is_empty() && !p.out.ends_with(&['[' as u32])) {
                p.push(' ' as u32);
            }
            if i >= max_len {
                p.push_str("...");
                break;
            }
            self.print_object(p, item, escape)?;
        }
        p.being_printed.pop();
        p.depth -= 1;
        p.push_str(close);
        Ok(())
    }

    fn print_vectorlike(&mut self, p: &mut Printer, obj: Value, escape: bool) -> LispResult<()> {
        let v = self.heap.vectorlike_ref(obj).clone();
        match v {
            Vectorlike::Vector(items) => self.print_vector_items(p, obj, "[", "]", &items, escape),
            Vectorlike::Record(items) => {
                self.print_vector_items(p, obj, "#s(", ")", &items, escape)
            }
            Vectorlike::Closure(items) => {
                self.print_vector_items(p, obj, "#[", "]", &items, escape)
            }
            Vectorlike::CharTable(t) => {
                let mut items = vec![t.default, t.parent, t.purpose, t.ascii];
                items.extend(t.contents.iter().copied());
                items.extend(t.extras.iter().copied());
                self.print_vector_items(p, obj, "#^[", "]", &items, escape)
            }
            Vectorlike::SubCharTable(t) => {
                p.push_str(&format!("#^^[{} {}", t.depth, t.min_char));
                p.depth += 1;
                for item in t.contents.iter() {
                    p.push(' ' as u32);
                    self.print_object(p, *item, escape)?;
                }
                p.depth -= 1;
                p.push(']' as u32);
                Ok(())
            }
            Vectorlike::HashTable(h) => {
                p.push_str("#s(hash-table");
                if h.test != sym::EQL {
                    p.push_str(" test ");
                    self.print_object(p, h.test, escape)?;
                }
                if h.weakness != Weakness::None {
                    p.push_str(" weakness ");
                    let w = crate::fns::weakness_symbol(h.weakness);
                    self.print_object(p, w, escape)?;
                }
                let entries = h.live_entries();
                if !entries.is_empty() {
                    p.push_str(" data (");
                    let max_len = self
                        .plain_value(sym::PRINT_LENGTH)
                        .as_fixnum()
                        .filter(|&n| n >= 0)
                        .map_or(usize::MAX, |n| n as usize);
                    for (i, (k, val)) in entries.iter().enumerate() {
                        if i >= max_len {
                            p.push_str(" ...");
                            break;
                        }
                        if i > 0 {
                            p.push(' ' as u32);
                        }
                        self.print_object(p, *k, escape)?;
                        p.push(' ' as u32);
                        self.print_object(p, *val, escape)?;
                    }
                    p.push(')' as u32);
                }
                p.push(')' as u32);
                Ok(())
            }
            Vectorlike::Bignum(b) => {
                p.push_str(&b.to_string());
                Ok(())
            }
            Vectorlike::BoolVector(b) => {
                let n = b.bits.len();
                p.push_str(&format!("#&{n}\""));
                let bytes: Vec<u8> = b
                    .bits
                    .chunks(8)
                    .map(|chunk| {
                        chunk
                            .iter()
                            .enumerate()
                            .fold(0u8, |acc, (i, &bit)| acc | (u8::from(bit) << i))
                    })
                    .collect();
                let escape_newlines = self.var_true(sym::PRINT_ESCAPE_NEWLINES);
                let escape_control = self.var_true(sym::PRINT_ESCAPE_CONTROL_CHARACTERS);
                for (i, &c) in bytes.iter().enumerate() {
                    if c == b'\n' && escape_newlines {
                        p.push_str("\\n");
                    } else if c == 12 && escape_newlines {
                        p.push_str("\\f");
                    } else if c > 127 || (escape_control && (c < 32 || c == 127)) {
                        octal_out(p, c as u32, bytes.get(i + 1).copied());
                    } else {
                        if c == b'"' || c == b'\\' {
                            p.push('\\' as u32);
                        }
                        p.push(c as u32);
                    }
                }
                p.push('"' as u32);
                Ok(())
            }
            Vectorlike::Subr(s) => {
                p.push_str(&format!("#<subr {}>", s.name));
                Ok(())
            }
            Vectorlike::Obarray(o) => {
                p.push_str(&format!("#<obarray n={}>", o.symbols.len()));
                Ok(())
            }
            Vectorlike::SymbolWithPos { symbol, position } => {
                if self.var_true(sym::PRINT_SYMBOLS_BARE) {
                    self.print_symbol(p, symbol, escape);
                } else {
                    p.push_str("#<symbol ");
                    self.print_symbol(p, symbol, escape);
                    p.push_str(" at ");
                    p.push_str(&position.as_fixnum().unwrap_or(0).to_string());
                    p.push('>' as u32);
                }
                Ok(())
            }
            Vectorlike::Finalizer { .. } => {
                p.push_str("#<finalizer>");
                Ok(())
            }
            Vectorlike::Buffer(b) => {
                if b.name.is_nil() {
                    p.push_str("#<killed buffer>");
                } else if escape {
                    p.push_str("#<buffer ");
                    let chars = self.string_char_vec(b.name);
                    p.out.extend(chars);
                    p.push('>' as u32);
                } else {
                    let chars = self.string_char_vec(b.name);
                    p.out.extend(chars);
                }
                Ok(())
            }
            Vectorlike::Marker(m) => {
                if m.buffer.is_nil() {
                    p.push_str("#<marker in no buffer>");
                } else {
                    let name = self.buffer(m.buffer).name;
                    if name.is_nil() {
                        p.push_str("#<marker in killed buffer>");
                    } else {
                        let kind = if m.insertion_type {
                            "(moves after insertion) "
                        } else {
                            ""
                        };
                        p.push_str(&format!("#<marker {kind}at {} in ", m.charpos));
                        let chars = self.string_char_vec(name);
                        p.out.extend(chars);
                        p.push('>' as u32);
                    }
                }
                Ok(())
            }
            Vectorlike::Overlay(o) => {
                if o.buffer.is_nil() {
                    p.push_str("#<overlay in no buffer>");
                } else {
                    p.push_str(&format!("#<overlay from {} to {} in ", o.start, o.end));
                    let name = self.buffer(o.buffer).name;
                    let chars = self.string_char_vec(name);
                    p.out.extend(chars);
                    p.push('>' as u32);
                }
                Ok(())
            }
            Vectorlike::Terminal(t) => {
                p.push_str(&format!("#<terminal {}", t.id));
                if let Some(name) = &t.name {
                    p.push_str(" on ");
                    p.push_str(name);
                }
                p.push('>' as u32);
                Ok(())
            }
            Vectorlike::Frame(f) => {
                p.push_str(if f.terminal.is_nil() {
                    "#<dead frame "
                } else {
                    "#<frame "
                });
                let name = if self.is_string(f.name) {
                    self.string_char_vec(f.name)
                } else if f.name.is_symbol() {
                    let n = self.symbol_name(f.name);
                    self.string_char_vec(n)
                } else {
                    "*INVALID*FRAME*NAME*".chars().map(|c| c as u32).collect()
                };
                p.out.extend(name);
                p.push_str(&format!(" {:#x}>", obj.bits()));
                Ok(())
            }
            Vectorlike::Font(f) => {
                if f.kind == crate::font::FontKind::Object {
                    p.push_str("#<font-object ");
                    let name = f.props[crate::font::FONT_NAME_INDEX];
                    self.print_object(p, name, escape)?;
                } else {
                    let entity = f.kind == crate::font::FontKind::Entity;
                    p.push_str(if entity {
                        "#<font-entity"
                    } else {
                        "#<font-spec"
                    });
                    let props = f.props.clone();
                    for (i, &v) in props.iter().enumerate().take(crate::font::FONT_SPEC_MAX) {
                        if i == crate::font::FONT_EXTRA_INDEX && entity {
                            continue;
                        }
                        p.push(' ' as u32);
                        let v = if (crate::font::FONT_WEIGHT_INDEX..=crate::font::FONT_WIDTH_INDEX)
                            .contains(&i)
                        {
                            self.font_style_symbolic(obj, i, false)
                        } else {
                            v
                        };
                        self.print_object(p, v, escape)?;
                    }
                }
                p.push('>' as u32);
                Ok(())
            }
            Vectorlike::Window(w) => {
                p.push_str(&format!("#<window {}", w.sequence_number));
                if self.is_buffer(w.contents) {
                    p.push_str(" on ");
                    let name = self.buffer(w.contents).name;
                    let chars = self.string_char_vec(name);
                    p.out.extend(chars);
                }
                p.push('>' as u32);
                Ok(())
            }
        }
    }

    pub fn print_to_stream(&mut self, chars: &[u32], printcharfun: Value) -> LispResult<()> {
        let mut dest = if printcharfun.is_nil() {
            self.plain_value(sym::STANDARD_OUTPUT)
        } else {
            printcharfun
        };
        if dest.is_nil() {
            dest = sym::T;
        }
        if dest == sym::T || dest == sym::EXTERNAL_DEBUGGING_OUTPUT {
            let s: String = chars.iter().filter_map(|&c| char::from_u32(c)).collect();
            if dest == sym::T && !self.noninteractive() {
                self.echo_area_append(&s)?;
                return Ok(());
            }
            if dest == sym::EXTERNAL_DEBUGGING_OUTPUT {
                let _ = self.stderr.write_all(s.as_bytes());
            } else {
                let _ = self.stdout.write_all(s.as_bytes());
            }
            return Ok(());
        }
        if self.is_buffer(dest) || self.is_marker(dest) {
            return self.insert_chars_into(dest, chars);
        }
        let base = self.stack.len();
        self.stack.push(dest);
        for &c in chars {
            if let Err(e) = self.funcall(dest, &[Value::fixnum(c as i64)]) {
                self.stack.truncate(base);
                return Err(e);
            }
        }
        self.stack.truncate(base);
        Ok(())
    }

    pub fn message_raw(&mut self, text: &str) {
        let _ = self.stderr.write_all(text.as_bytes());
        let _ = self.stderr.write_all(b"\n");
    }

    pub fn error_message_string(&mut self, obj: Value) -> LispResult<String> {
        let errname = self.car_safe(obj);
        let data = self.cdr_safe(obj);
        if errname == sym::ERROR {
            let first = self.car_safe(data);
            if first.is_string() {
                let tail = self.cdr_safe(data);
                let mut out = self.princ_to_string(first);
                let items = self.list_to_vec(tail).unwrap_or_default();
                for (i, item) in items.iter().enumerate() {
                    out.push_str(if i == 0 { ": " } else { ", " });
                    out.push_str(&self.prin1_to_string(*item));
                }
                return Ok(out);
            }
        }
        let errmsg = if errname.is_symbol() {
            self.get(errname, sym::ERROR_MESSAGE)
        } else {
            Value::NIL
        };
        let file_error = if errname.is_symbol() {
            let conds = self.get(errname, sym::ERROR_CONDITIONS);
            !self.memq(sym::FILE_ERROR, conds).is_nil()
        } else {
            false
        };
        let (mut out, tail) = if file_error && data.is_cons() {
            (self.princ_to_string(self.xcar(data)), self.xcdr(data))
        } else if errmsg.is_string() {
            (self.string_to_rust(errmsg), data)
        } else {
            ("peculiar error".to_string(), data)
        };
        let items = self.list_to_vec(tail).unwrap_or_default();
        for (i, item) in items.iter().enumerate() {
            out.push_str(if i == 0 { ": " } else { ", " });
            if file_error || errname == sym::END_OF_FILE || errname == sym::USER_ERROR {
                out.push_str(&self.princ_to_string(*item));
            } else {
                out.push_str(&self.prin1_to_string(*item));
            }
        }
        Ok(out)
    }
}

pub fn chars_to_display_string(chars: &[u32]) -> String {
    let mut out = String::with_capacity(chars.len());
    for &c in chars {
        match char::from_u32(c) {
            Some(ch) if !character::char_byte8_p(c) => out.push(ch),
            _ => out.push_str(&format!("\\{:o}", character::char_to_byte8(c))),
        }
    }
    out
}

fn octal_out(p: &mut Printer, c: u32, next: Option<u8>) {
    let needs_full = next.is_some_and(|b| (b'0'..=b'7').contains(&b));
    if needs_full {
        p.push_str(&format!("\\{c:03o}"));
    } else {
        p.push_str(&format!("\\{c:o}"));
    }
}

fn integer_as_character(c: u32, escape: bool) -> Option<String> {
    let named = match c {
        7 => Some('a'),
        8 => Some('b'),
        9 => Some('t'),
        10 => Some('n'),
        11 => Some('v'),
        12 => Some('f'),
        13 => Some('r'),
        27 => Some('e'),
        127 => Some('d'),
        _ => None,
    };
    if let Some(n) = named {
        return Some(format!("?\\{n}"));
    }
    let ch = char::from_u32(c)?;
    if c < 32 || ch.is_control() {
        return None;
    }
    if escape
        && matches!(
            ch,
            ';' | '"' | '\'' | '\\' | '(' | ')' | '{' | '}' | '[' | ']'
        )
    {
        return Some(format!("?\\{ch}"));
    }
    Some(format!("?{ch}"))
}

fn parse_float_format(spec: &str) -> Option<(Option<usize>, char)> {
    let rest = spec.strip_prefix("%.")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let conv_part = &rest[digits.len()..];
    let mut chars = conv_part.chars();
    let conv = chars.next()?;
    if chars.next().is_some() || !matches!(conv, 'e' | 'f' | 'g') {
        return None;
    }
    let width = if digits.is_empty() {
        None
    } else {
        let w: usize = digits.parse().ok()?;
        if w > 15 || (w == 0 && conv != 'f') {
            return None;
        }
        Some(w)
    };
    Some((width, conv))
}

fn format_c_float(data: f64, width: Option<usize>, conv: char) -> String {
    let precision = width.unwrap_or(6);
    match conv {
        'f' => format!("{data:.precision$}"),
        'e' => c_exponent(format!("{data:.precision$e}")),
        _ => format_g(data, precision.max(1)),
    }
}

fn c_exponent(s: String) -> String {
    match s.find('e') {
        Some(i) => {
            let (mantissa, exp) = s.split_at(i);
            let exp = &exp[1..];
            let (sign, digits) = match exp.strip_prefix('-') {
                Some(d) => ('-', d),
                None => ('+', exp),
            };
            let digits = if digits.len() < 2 {
                format!("0{digits}")
            } else {
                digits.to_string()
            };
            format!("{mantissa}e{sign}{digits}")
        }
        None => s,
    }
}

pub fn format_g(data: f64, precision: usize) -> String {
    if data == 0.0 {
        return if data.is_sign_negative() {
            "-0".into()
        } else {
            "0".into()
        };
    }
    let sci = format!("{:.*e}", precision - 1, data);
    let exp: i32 = sci[sci.find('e').map_or(0, |i| i + 1)..]
        .parse()
        .unwrap_or(0);
    if exp < -4 || exp >= precision as i32 {
        let (mantissa, _) = sci.split_at(sci.find('e').unwrap_or(sci.len()));
        let mantissa = if mantissa.contains('.') {
            mantissa.trim_end_matches('0').trim_end_matches('.')
        } else {
            mantissa
        };
        c_exponent(format!("{mantissa}e{exp}"))
    } else {
        let decimals = (precision as i32 - 1 - exp).max(0) as usize;
        let fixed = format!("{data:.decimals$}");
        if fixed.contains('.') {
            fixed
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            fixed
        }
    }
}

pub fn shortest_float(data: f64) -> String {
    let start = if data.abs() < f64::MIN_POSITIVE {
        1
    } else {
        15
    };
    for precision in start..=17 {
        let s = format_g(data, precision);
        if s.parse::<f64>().ok() == Some(data) {
            return s;
        }
    }
    format_g(data, 17)
}

fn f_write_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    lisp.print_to_stream(&[c], args[1])?;
    Ok(args[0])
}

fn f_terpri(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (printcharfun, ensure) = (args[0], args[1]);
    if !ensure.is_nil() {
        let dest = if printcharfun.is_nil() {
            lisp.plain_value(sym::STANDARD_OUTPUT)
        } else {
            printcharfun
        };
        if lisp.is_buffer(dest) && lisp.buffer_at_line_start(dest) {
            return Ok(Value::NIL);
        }
    }
    lisp.print_to_stream(&[10], printcharfun)?;
    Ok(sym::T)
}

fn with_overrides<T>(
    lisp: &mut Lisp,
    overrides: Value,
    f: impl FnOnce(&mut Lisp) -> LispResult<T>,
) -> LispResult<T> {
    let count = lisp.specpdl.len();
    if !overrides.is_nil() {
        let pairs: Vec<(Value, Value)> = if overrides == sym::T {
            vec![
                (sym::PRINT_LENGTH, Value::NIL),
                (sym::PRINT_LEVEL, Value::NIL),
                (sym::PRINT_CIRCLE, Value::NIL),
                (sym::PRINT_QUOTED, sym::T),
                (sym::PRINT_ESCAPE_NEWLINES, Value::NIL),
                (sym::PRINT_ESCAPE_CONTROL_CHARACTERS, Value::NIL),
                (sym::PRINT_ESCAPE_NONASCII, Value::NIL),
                (sym::PRINT_ESCAPE_MULTIBYTE, Value::NIL),
                (sym::PRINT_GENSYM, Value::NIL),
                (sym::PRINT_CONTINUOUS_NUMBERING, Value::NIL),
                (sym::PRINT_NUMBER_TABLE, Value::NIL),
                (sym::FLOAT_OUTPUT_FORMAT, Value::NIL),
                (sym::PRINT_INTEGERS_AS_CHARACTERS, Value::NIL),
            ]
        } else {
            let mut out = Vec::new();
            let mut tail = overrides;
            while tail.is_cons() {
                let item = lisp.xcar(tail);
                if item == sym::T {
                    out.push((sym::PRINT_LENGTH, Value::NIL));
                    out.push((sym::PRINT_LEVEL, Value::NIL));
                } else if item.is_cons() {
                    let key = lisp.xcar(item);
                    let val = lisp.xcdr(item);
                    let var = match lisp.symbol_name_string(key).as_str() {
                        "length" => sym::PRINT_LENGTH,
                        "level" => sym::PRINT_LEVEL,
                        "circle" => sym::PRINT_CIRCLE,
                        "quoted" => sym::PRINT_QUOTED,
                        "escape-newlines" => sym::PRINT_ESCAPE_NEWLINES,
                        "escape-control-characters" => sym::PRINT_ESCAPE_CONTROL_CHARACTERS,
                        "escape-nonascii" => sym::PRINT_ESCAPE_NONASCII,
                        "escape-multibyte" => sym::PRINT_ESCAPE_MULTIBYTE,
                        "charset-text-property" => sym::PRINT_CHARSET_TEXT_PROPERTY,
                        "unreadable-function" => sym::PRINT_UNREADABLE_FUNCTION,
                        "gensym" => sym::PRINT_GENSYM,
                        "continuous-numbering" => sym::PRINT_CONTINUOUS_NUMBERING,
                        "number-table" => sym::PRINT_NUMBER_TABLE,
                        "float-format" => sym::FLOAT_OUTPUT_FORMAT,
                        "integers-as-characters" => sym::PRINT_INTEGERS_AS_CHARACTERS,
                        _ => return Err(lisp.signal_error("Unknown print override", key)),
                    };
                    out.push((var, val));
                } else {
                    return Err(lisp.signal_error("Invalid print override", item));
                }
                tail = lisp.xcdr(tail);
            }
            out
        };
        for (var, val) in pairs {
            lisp.specbind(var, val)?;
        }
    }
    let r = f(lisp);
    let ok = lisp.unbind_to(count);
    let r = r?;
    ok?;
    Ok(r)
}

fn f_prin1(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (object, printcharfun, overrides) = (args[0], args[1], args[2]);
    let chars = with_overrides(lisp, overrides, |l| l.print_to_chars(object, true))?;
    lisp.print_to_stream(&chars, printcharfun)?;
    Ok(object)
}

fn f_prin1_to_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (object, noescape, overrides) = (args[0], args[1], args[2]);
    let chars = with_overrides(lisp, overrides, |l| {
        l.print_to_chars(object, noescape.is_nil())
    })?;
    let multibyte = chars
        .iter()
        .any(|&c| c >= 0x80 && !character::char_byte8_p(c));
    if multibyte || chars.iter().any(|&c| c >= 0x80) {
        Ok(lisp.string_from_chars(&chars, true))
    } else {
        Ok(lisp.string_from_chars(&chars, false))
    }
}

fn f_error_message_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let obj = args[0];
    if !obj.is_cons() && !obj.is_nil() {
        return Err(lisp.wrong_type(sym::CONSP, obj));
    }
    let s = lisp.error_message_string(obj)?;
    Ok(lisp.string(&s))
}

fn f_princ(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let chars = lisp.print_to_chars(args[0], false)?;
    lisp.print_to_stream(&chars, args[1])?;
    Ok(args[0])
}

fn f_print(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut chars = vec![10];
    chars.extend(lisp.print_to_chars(args[0], true)?);
    chars.push(10);
    lisp.print_to_stream(&chars, args[1])?;
    Ok(args[0])
}

fn f_external_debugging_output(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    let mut buf = [0u8; 4];
    let s = char::from_u32(c).unwrap_or('?').encode_utf8(&mut buf);
    let _ = lisp.stderr.write_all(s.as_bytes());
    Ok(args[0])
}

fn f_redirect_debugging_output(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_print_preprocess(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_flush_standard_output(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let _ = lisp.stdout.flush();
    Ok(Value::NIL)
}
