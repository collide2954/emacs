use std::io::{Read, Seek, SeekFrom};

use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::Vectorlike;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

const CLOSURE_DOC_STRING: usize = 4;
const LEFT_SINGLE_QUOTATION_MARK: u32 = 0x2018;
const DISP_TABLE_EXTRA_SLOTS: usize = 18;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("documentation-stringp", 1, 1, f_documentation_stringp),
    Subr::fixed("documentation", 1, 2, f_documentation),
    Subr::fixed("internal-subr-documentation", 1, 1, f_subr_documentation),
    Subr::fixed("documentation-property", 2, 3, f_documentation_property),
    Subr::fixed("Snarf-documentation", 1, 1, f_snarf_documentation),
    Subr::fixed("text-quoting-style", 0, 0, f_text_quoting_style),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::INTERNAL_DOC_FILE_NAME, Value::NIL);
    lisp.defvar(sym::BUILD_FILES, Value::NIL);
    lisp.defvar(sym::TEXT_QUOTING_STYLE, Value::NIL);
    lisp.defvar_bool(sym::DOCUMENTATION_DYNAMIC_RELOAD, true);
    lisp.defvar_bool(sym::INTERNAL__TEXT_QUOTING_FLAG, true);
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn valid_docstring_p(lisp: &Lisp, doc: Value) -> bool {
    doc.is_fixnum()
        || doc.is_string()
        || (doc.is_cons() && lisp.xcar(doc).is_string() && lisp.xcdr(doc).is_fixnum())
}

fn doc_reference_p(lisp: &Lisp, doc: Value) -> bool {
    doc.is_fixnum() || (doc.is_cons() && lisp.xcdr(doc).is_fixnum())
}

impl Lisp {
    pub fn get_doc_string(&mut self, filepos: Value, unibyte: bool) -> LispResult {
        let (file, dir, pos) = if filepos.is_fixnum() {
            (
                self.find_symbol_value(sym::INTERNAL_DOC_FILE_NAME),
                self.find_symbol_value(sym::DOC_DIRECTORY),
                filepos,
            )
        } else if filepos.is_cons() {
            let dir = self.find_symbol_value(sym::LISP_DIRECTORY);
            if dir == sym::UNBOUND {
                return Err(self.signal_list(sym::VOID_VARIABLE, &[sym::LISP_DIRECTORY]));
            }
            (self.xcar(filepos), dir, self.xcdr(filepos))
        } else {
            return Ok(Value::NIL);
        };
        let Some(position) = pos.as_fixnum().map(i64::unsigned_abs) else {
            return Ok(Value::NIL);
        };
        if !dir.is_string() || !file.is_string() {
            return Ok(Value::NIL);
        }
        let file_text = self.string_to_rust(file);
        let mut name = if crate::fileio::file_name_absolute_p(&file_text) {
            String::new()
        } else {
            self.string_to_rust(dir)
        };
        name.push_str(&file_text);
        let mut fd = match std::fs::File::open(&name) {
            Ok(fd) => fd,
            Err(e) => {
                if !matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) {
                    return Err(self.file_error("Read error on documentation file", &e, file));
                }
                let prefix = self.string("Cannot open doc string file \"");
                self.stack.push(prefix);
                let suffix = self.string("\"\n");
                self.stack.pop();
                return crate::fns::f_concat(self, &[prefix, file, suffix]);
            }
        };
        let offset = position.min(1024.max(position % (8 * 1024))) as usize;
        if fd.seek(SeekFrom::Start(position - offset as u64)).is_err() {
            return Err(self.error(&format!(
                "Position {position} out of range in doc string file \"{name}\""
            )));
        }
        let mut buffer: Vec<u8> = Vec::new();
        let mut end;
        loop {
            let start = buffer.len();
            buffer.resize(start + 8 * 1024, 0);
            let nread = match fd.read(&mut buffer[start..]) {
                Ok(n) => n,
                Err(e) => {
                    return Err(self.file_error("Read error on documentation file", &e, file));
                }
            };
            buffer.truncate(start + nread);
            end = buffer.len();
            if nread == 0 {
                break;
            }
            let from = if start == 0 { offset } else { start };
            if from < buffer.len()
                && let Some(i) = buffer[from..].iter().position(|&c| c == 0o37 || c == 0)
                && buffer[from + i] == 0o37
            {
                end = from + i;
                break;
            }
        }
        let at =
            |i: usize| -> Option<u8> { offset.checked_sub(i).and_then(|j| buffer.get(j).copied()) };
        if filepos.is_cons() {
            let mut test = 1;
            if at(test) != Some(0o37) {
                if at(test) != Some(b' ') {
                    return Ok(Value::NIL);
                }
                test += 1;
                while at(test).is_some_and(|c| c.is_ascii_digit()) {
                    test += 1;
                }
                if at(test) != Some(b'@') || at(test + 1) != Some(b'#') {
                    return Ok(Value::NIL);
                }
            }
        } else {
            let mut test = 1;
            if at(test) != Some(b'\n') {
                return Ok(Value::NIL);
            }
            test += 1;
            while at(test).is_some_and(|c| c > b' ') {
                test += 1;
            }
            if at(test) != Some(0o37) {
                return Ok(Value::NIL);
            }
        }
        let data = buffer.get(offset..end).unwrap_or(&[]);
        let mut out = Vec::with_capacity(data.len());
        let mut i = 0;
        while i < data.len() {
            if data[i] == 1 {
                let c = data.get(i + 1).copied().unwrap_or(0);
                i += 2;
                match c {
                    1 => out.push(1),
                    b'0' => out.push(0),
                    b'_' => out.push(0o37),
                    _ => {
                        return Err(self.error(&format!(
                            "Invalid data in documentation file -- \u{1} followed by code {c:03o}"
                        )));
                    }
                }
            } else {
                out.push(data[i]);
                i += 1;
            }
        }
        if unibyte {
            return Ok(self.unibyte_string(&out));
        }
        let nchars = character::chars_in_multibyte(&out);
        let multibyte = nchars != out.len();
        Ok(self.make_specified_string(out, nchars, multibyte))
    }

    fn reread_doc_file(&mut self, file: Value) -> LispResult<()> {
        if file.is_nil() {
            let name = self.find_symbol_value(sym::INTERNAL_DOC_FILE_NAME);
            f_snarf_documentation(self, &[name])?;
        } else {
            let count = self.specpdl.len();
            self.record_unwind_save_match_data()?;
            let r = self.funcall(sym::LOAD, &[file, sym::T, sym::T, sym::T, Value::NIL]);
            self.unbind_to_value(count, r)?;
        }
        Ok(())
    }

    fn store_function_docstring(&mut self, obj: Value, offset: i64) -> LispResult<()> {
        let mut fun = if obj.is_symbol() {
            self.heap.symbol_ref(obj).function
        } else {
            obj
        };
        if fun.is_cons() && self.xcar(fun) == sym::MACRO {
            fun = self.xcdr(fun);
        }
        if let Some(subr) = self.subr(fun) {
            self.subr_docs.insert(subr as *const Subr as usize, offset);
        } else if self.is_closure(fun) {
            let items = self.vector_items(fun);
            if items.len() > CLOSURE_DOC_STRING
                && valid_docstring_p(self, items[CLOSURE_DOC_STRING])
            {
                if let Vectorlike::Closure(items) = self.heap.vectorlike_mut(fun) {
                    items[CLOSURE_DOC_STRING] = Value::fixnum(offset);
                }
            } else {
                let format = self.string("No doc string slot for compiled: %S");
                crate::editfns::f_message(self, &[format, obj])?;
            }
        } else {
            let format = self.string("Ignoring DOC string on non-compiled non-subr: %S");
            crate::editfns::f_message(self, &[format, obj])?;
        }
        Ok(())
    }

    fn default_to_grave_quoting_style(&self) -> bool {
        if !self.var_true(sym::INTERNAL__TEXT_QUOTING_FLAG) {
            return true;
        }
        let table = self.find_symbol_value(sym::STANDARD_DISPLAY_TABLE);
        if !(self.is_char_table(table)
            && self.char_table(table).purpose == sym::DISPLAY_TABLE
            && self.char_table(table).extras.len() == DISP_TABLE_EXTRA_SLOTS)
        {
            return false;
        }
        let dv = self.char_table_ref(table, LEFT_SINGLE_QUOTATION_MARK);
        self.is_vector(dv)
            && self.vector_items(dv).len() == 1
            && self.vector_items(dv)[0] == Value::fixnum(i64::from(b'`'))
    }

    pub fn text_quoting_style(&self) -> Value {
        let style = self.find_symbol_value(sym::TEXT_QUOTING_STYLE);
        let grave = if style.is_nil() {
            self.default_to_grave_quoting_style()
        } else {
            style == sym::GRAVE
        };
        if grave {
            sym::GRAVE
        } else if style == sym::STRAIGHT {
            sym::STRAIGHT
        } else {
            sym::CURVE
        }
    }
}

fn f_documentation_stringp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let object = args[0];
    let r = object.is_string()
        || object.is_fixnum()
        || (object.is_cons() && lisp.xcar(object).is_string() && lisp.xcdr(object).is_fixnum());
    Ok(lisp.bool(r))
}

fn f_documentation(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, raw) = (args[0], arg(args, 1));
    let mut try_reload = lisp.var_true(sym::DOCUMENTATION_DYNAMIC_RELOAD);
    loop {
        if function.is_symbol() {
            let tem = lisp.get(function, sym::FUNCTION_DOCUMENTATION);
            if !tem.is_nil() {
                return f_documentation_property(
                    lisp,
                    &[function, sym::FUNCTION_DOCUMENTATION, raw],
                );
            }
        }
        let mut fun = lisp.indirect_function(function);
        if fun.is_nil() {
            return Err(lisp.signal_list(sym::VOID_FUNCTION, &[function]));
        }
        if fun.is_cons() && lisp.xcar(fun) == sym::MACRO {
            fun = lisp.xcdr(fun);
        }
        let mut doc = lisp.funcall(sym::FUNCTION_DOCUMENTATION, &[fun])?;
        if doc == Value::fixnum(0) {
            doc = Value::NIL;
        }
        if doc_reference_p(lisp, doc) {
            lisp.stack.push(doc);
            let tem = lisp.get_doc_string(doc, false);
            lisp.stack.pop();
            let tem = tem?;
            if tem.is_nil() && try_reload {
                let file = lisp.car_safe(doc);
                lisp.reread_doc_file(file)?;
                try_reload = false;
                continue;
            }
            doc = tem;
        }
        if raw.is_nil() {
            doc = lisp.funcall(sym::SUBSTITUTE_COMMAND_KEYS, &[doc])?;
        }
        return Ok(doc);
    }
}

fn f_subr_documentation(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    match lisp.subr(args[0]) {
        Some(subr) => {
            let doc = lisp
                .subr_docs
                .get(&(subr as *const Subr as usize))
                .copied()
                .unwrap_or(0);
            Ok(Value::fixnum(doc))
        }
        None => Ok(sym::T),
    }
}

fn f_documentation_property(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, prop, raw) = (args[0], args[1], arg(args, 2));
    let mut try_reload = lisp.var_true(sym::DOCUMENTATION_DYNAMIC_RELOAD);
    loop {
        lisp.check_symbol(symbol)?;
        let mut tem = lisp.get(symbol, prop);
        if prop == sym::VARIABLE_DOCUMENTATION && tem.is_nil() {
            let indirect = lisp.indirect_variable(symbol);
            if !indirect.is_nil() {
                tem = lisp.get(indirect, prop);
            }
        }
        if tem == Value::fixnum(0) {
            tem = Value::NIL;
        }
        if doc_reference_p(lisp, tem) {
            let doc = tem;
            lisp.stack.push(doc);
            let r = lisp.get_doc_string(tem, false);
            lisp.stack.pop();
            tem = r?;
            if tem.is_nil() && try_reload {
                let file = lisp.car_safe(doc);
                lisp.reread_doc_file(file)?;
                try_reload = false;
                continue;
            }
        } else if !tem.is_string() {
            tem = lisp.eval_with_lexical(tem, Value::NIL)?;
        }
        if raw.is_nil() && tem.is_string() {
            tem = lisp.funcall(sym::SUBSTITUTE_COMMAND_KEYS, &[tem])?;
        }
        return Ok(tem);
    }
}

fn f_snarf_documentation(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let filename = args[0];
    let mut delayed_init = lisp.find_symbol_value(sym::CUSTOM_DELAYED_INIT_VARIABLES);
    if !delayed_init.is_cons() {
        delayed_init = Value::NIL;
    }
    lisp.check_string(filename)?;
    let dir = lisp.find_symbol_value(sym::DOC_DIRECTORY);
    lisp.check_string(dir)?;
    let base = lisp.stack.len();
    lisp.stack.push(delayed_init);
    let r = snarf_documentation(lisp, filename, dir, delayed_init);
    lisp.stack.truncate(base);
    r
}

fn snarf_documentation(
    lisp: &mut Lisp,
    filename: Value,
    dir: Value,
    delayed_init: Value,
) -> LispResult {
    let name = format!(
        "{}{}",
        lisp.string_to_rust(dir),
        lisp.string_to_rust(filename)
    );
    let mut fd = match std::fs::File::open(&name) {
        Ok(fd) => fd,
        Err(e) => {
            let file = lisp.string(&name);
            return Err(lisp.file_error("Opening doc string file", &e, file));
        }
    };
    lisp.set_plain_value(sym::INTERNAL_DOC_FILE_NAME, filename);
    let mut buf = [0u8; 1024 + 1];
    let mut filled = 0usize;
    let mut pos: i64 = 0;
    loop {
        if filled < 512 {
            match fd.read(&mut buf[filled..1024]) {
                Ok(n) => filled += n,
                Err(e) => {
                    let file = lisp.string(&name);
                    return Err(lisp.file_error("Read error on documentation file", &e, file));
                }
            }
        }
        if filled == 0 {
            break;
        }
        buf[filled] = 0;
        let mut end = if filled < 512 { filled } else { filled - 128 };
        if let Some(p) = buf[..end].iter().position(|&c| c == 0o37) {
            let nl = buf[p..=filled]
                .iter()
                .position(|&c| c == b'\n' || c == 0)
                .filter(|&i| buf[p + i] == b'\n');
            let Some(nl) = nl else {
                return Err(lisp.error(&format!("DOC file invalid at position {pos}")));
            };
            end = p + nl;
            let name_bytes = buf.get(p + 2..end).unwrap_or(&[]).to_vec();
            let obarray = lisp.obarray;
            if let Some(symbol) = lisp.obarray_lookup(obarray, &name_bytes) {
                let skip = buf[end..filled].starts_with(b"\nSKIP");
                let offset = pos + end as i64 + 1;
                match buf[p + 1] {
                    b'V' => {
                        let bound = lisp.find_symbol_value(symbol) != sym::UNBOUND;
                        if (bound || !lisp.memq(symbol, delayed_init).is_nil()) && !skip {
                            lisp.put(symbol, sym::VARIABLE_DOCUMENTATION, Value::fixnum(offset));
                        }
                    }
                    b'F' => {
                        if !lisp.heap.symbol_ref(symbol).function.is_nil() && !skip {
                            lisp.store_function_docstring(symbol, offset)?;
                        }
                    }
                    b'S' => {}
                    _ => {
                        return Err(lisp.error(&format!("DOC file invalid at position {pos}")));
                    }
                }
            }
        }
        pos += end as i64;
        filled -= end;
        buf.copy_within(end..end + filled, 0);
    }
    Ok(Value::NIL)
}

fn f_text_quoting_style(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.text_quoting_style())
}
