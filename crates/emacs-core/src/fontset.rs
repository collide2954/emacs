use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

const FONTSET_ID: usize = 0;
const FONTSET_NAME: usize = 1;
const FONTSET_ASCII: usize = 2;
const FONTSET_BASE: usize = 3;
const FONTSET_FRAME: usize = 4;
const FONTSET_NOFONT_FACE: usize = 5;
const FONTSET_DEFAULT: usize = 6;
const FONTSET_FALLBACK: usize = 7;

pub struct Fontsets {
    pub table: Vec<Value>,
    pub next_fontset_id: usize,
    pub default_fontset: Value,
    pub cached_fontset_data: Value,
    pub auto_fontset_alist: Value,
    pub num_auto_fontsets: usize,
}

impl Default for Fontsets {
    fn default() -> Self {
        Fontsets {
            table: Vec::new(),
            next_fontset_id: 0,
            default_fontset: Value::NIL,
            cached_fontset_data: Value::NIL,
            auto_fontset_alist: Value::NIL,
            num_auto_fontsets: 0,
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
    Subr::fixed("new-fontset", 2, 2, f_new_fontset),
    Subr::fixed("set-fontset-font", 3, 5, f_set_fontset_font),
    Subr::fixed("fontset-font", 2, 3, f_fontset_font),
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

impl Lisp {
    fn set_fontset_extra(&mut self, fontset: Value, idx: usize, val: Value) {
        self.char_table_mut(fontset).extras[idx] = val;
    }

    fn fontset_id_of(&self, fontset: Value) -> i64 {
        self.fontset_extra(fontset, FONTSET_ID)
            .as_fixnum()
            .unwrap_or(-1)
    }

    fn fontset_ref(&self, fontset: Value, c: u32) -> Value {
        let elt = self.char_table_ref(fontset, c);
        let default = self.fontsets.default_fontset;
        if elt.is_nil() && fontset != default && self.fontset_extra(fontset, FONTSET_BASE).is_nil()
        {
            return self.char_table_ref(default, c);
        }
        elt
    }

    fn fontset_ref_or_default(&self, fontset: Value, c: u32) -> Value {
        if fontset == self.fontsets.default_fontset {
            self.char_table_ref(fontset, c)
        } else {
            self.fontset_ref(fontset, c)
        }
    }

    fn fontset_add(
        &mut self,
        fontset: Value,
        range: Value,
        elt: Value,
        add: Value,
    ) -> LispResult<()> {
        let idx = if add == sym::APPEND { 0 } else { 1 };
        let new = self.vector(vec![elt]);
        let base = self.stack.len();
        self.stack.push(new);
        let combine = |lisp: &mut Lisp, existing: Value| -> LispResult {
            if existing.is_nil() {
                return Ok(new);
            }
            let mut args = [Value::NIL; 2];
            args[idx] = existing;
            args[1 - idx] = new;
            crate::fns::f_vconcat(lisp, &args)
        };
        let r = (|| -> LispResult<()> {
            if range.is_cons() {
                let mut from = self.xcar(range).as_fixnum().unwrap_or(0) as u32;
                let to = self.xcdr(range).as_fixnum().unwrap_or(0) as u32;
                loop {
                    let (mut from1, mut to1) = (from, to);
                    let existing =
                        self.char_table_ref_and_range(fontset, from, &mut from1, &mut to1);
                    let val = combine(self, existing)?;
                    self.char_table_set_range(fontset, from, to1, val);
                    from = to1 + 1;
                    if from > to {
                        break;
                    }
                }
            } else {
                let existing = self.fontset_extra(fontset, FONTSET_FALLBACK);
                let val = combine(self, existing)?;
                self.set_fontset_extra(fontset, FONTSET_FALLBACK, val);
            }
            Ok(())
        })();
        self.stack.truncate(base);
        r
    }

    fn fontset_add_def(
        &mut self,
        fontset: Value,
        range: Value,
        elt: Value,
        add: Value,
    ) -> LispResult<()> {
        if !add.is_nil() {
            return self.fontset_add(fontset, range, elt, add);
        }
        let val = self.vector(vec![elt]);
        if range.is_nil() {
            self.set_fontset_extra(fontset, FONTSET_FALLBACK, val);
        } else {
            crate::chartab::f_set_char_table_range(self, &[fontset, range, val])?;
        }
        Ok(())
    }

    fn make_fontset(&mut self, frame: Value, name: Value, base: Value) -> LispResult {
        let mut id = self.fontsets.next_fontset_id;
        while !self.fontsets.table[id].is_nil() {
            id += 1;
        }
        if id + 1 == self.fontsets.table.len() {
            self.fontsets.table.push(Value::NIL);
        }
        let fontset = self.make_char_table(sym::FONTSET, Value::NIL)?;
        self.set_fontset_extra(fontset, FONTSET_ID, Value::fixnum(id as i64));
        if base.is_nil() {
            self.set_fontset_extra(fontset, FONTSET_NAME, name);
        } else {
            self.set_fontset_extra(fontset, FONTSET_NAME, Value::NIL);
            self.set_fontset_extra(fontset, FONTSET_FRAME, frame);
            self.set_fontset_extra(fontset, FONTSET_BASE, base);
        }
        self.fontsets.table[id] = fontset;
        self.fontsets.next_fontset_id = id + 1;
        Ok(fontset)
    }

    fn fontset_vector_set(&mut self, v: Value, i: usize, x: Value) {
        if let crate::object::Vectorlike::Vector(items) = self.heap.vectorlike_mut(v) {
            items[i] = x;
        }
    }

    fn rfont_def_score(&self, rfont_def: Value) -> i64 {
        self.vector_items(rfont_def)[3].as_fixnum().unwrap_or(0)
    }

    fn fontset_set(&mut self, fontset: Value, c: u32, elt: Value) -> LispResult<()> {
        let c = Value::fixnum(i64::from(c));
        crate::chartab::f_set_char_table_range(self, &[fontset, c, elt])?;
        Ok(())
    }

    fn reorder_font_vector(&mut self, font_group: Value, font_object: Value) -> LispResult<()> {
        let vec = self.xcdr(font_group);
        let mut size = self.vector_items(vec).len();
        if self.vector_items(vec)[size - 1].is_nil() {
            size -= 1;
        }
        let mut score_changed = false;
        for i in 0..size {
            let rfont_def = self.vector_items(vec)[i];
            if rfont_def.is_nil() {
                continue;
            }
            let font_def = self.vector_items(rfont_def)[1];
            let font_spec = self.vector_items(font_def)[0];
            let mut score = self.rfont_def_score(rfont_def) & 0xFF;
            let otf_spec = crate::font::f_font_get(self, &[font_spec, sym::K_OTF])?;
            if !otf_spec.is_nil() {
            } else if !self.font_match_p(font_spec, font_object)? {
                let encoding = self.vector_items(font_def)[1];
                if !encoding.is_nil() {
                    let mut tail = self.charsets.ordered_list;
                    while tail != self.charsets.non_preferred_head && tail.is_cons() {
                        if encoding == self.xcar(tail) {
                            break;
                        } else if score
                            <= i64::from(i32::MAX).min(crate::value::MOST_POSITIVE_FIXNUM) - 0x100
                        {
                            score += 0x100;
                        }
                        tail = self.xcdr(tail);
                    }
                } else {
                    let lang = crate::font::f_font_get(self, &[font_spec, sym::K_LANG])?;
                    let current = self.find_symbol_value(sym::CURRENT_ISO639_LANGUAGE);
                    if !lang.is_nil()
                        && lang != current
                        && (!current.is_cons() || self.memq(lang, current).is_nil())
                    {
                        score |= 0x100;
                    }
                }
            }
            if self.rfont_def_score(rfont_def) != score {
                self.fontset_vector_set(rfont_def, 3, Value::fixnum(score));
                score_changed = true;
            }
        }
        if score_changed {
            let mut items = self.vector_items(vec)[..size].to_vec();
            items.sort_by(|&v1, &v2| match (v1.is_nil(), v2.is_nil()) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (false, false) => self.rfont_def_score(v1).cmp(&self.rfont_def_score(v2)),
            });
            for (i, item) in items.into_iter().enumerate() {
                self.fontset_vector_set(vec, i, item);
            }
        }
        let low_tick_bits =
            self.charsets.ordered_list_tick as i64 & crate::value::MOST_POSITIVE_FIXNUM;
        self.heap.set_car(font_group, Value::fixnum(low_tick_bits));
        Ok(())
    }

    fn fontset_get_font_group(&mut self, fontset: Value, c: i64) -> LispResult {
        let mut font_group = if c >= 0 {
            self.char_table_ref(fontset, c as u32)
        } else {
            self.fontset_extra(fontset, FONTSET_FALLBACK)
        };
        if !font_group.is_nil() {
            return Ok(font_group);
        }
        let base_fontset = self.fontset_extra(fontset, FONTSET_BASE);
        let (mut from, mut to) = (0, crate::character::MAX_CHAR);
        font_group = if base_fontset.is_nil() {
            Value::NIL
        } else if c >= 0 {
            self.char_table_ref_and_range(base_fontset, c as u32, &mut from, &mut to)
        } else {
            self.fontset_extra(base_fontset, FONTSET_FALLBACK)
        };
        if font_group.is_nil() {
            font_group = Value::fixnum(0);
            if c >= 0 {
                self.char_table_set_range(fontset, from, to, font_group);
            }
            return Ok(font_group);
        }
        if !self.is_vector(font_group) {
            return Ok(font_group);
        }
        font_group = crate::fns::f_copy_sequence(self, &[font_group])?;
        for i in 0..self.vector_items(font_group).len() {
            let font_def = self.vector_items(font_group)[i];
            if !font_def.is_nil() {
                let rfont_def = self.vector(vec![
                    Value::NIL,
                    font_def,
                    Value::NIL,
                    Value::fixnum(i as i64),
                ]);
                self.fontset_vector_set(font_group, i, rfont_def);
            }
        }
        font_group = self.cons(Value::fixnum(-1), font_group);
        if c >= 0 {
            self.char_table_set_range(fontset, from, to, font_group);
        } else {
            self.set_fontset_extra(fontset, FONTSET_FALLBACK, font_group);
        }
        Ok(font_group)
    }

    fn fontset_find_font(
        &mut self,
        fontset: Value,
        c: u32,
        face: Option<usize>,
        charset_id: i64,
        fallback: bool,
    ) -> LispResult {
        let frame = self.fontset_extra(fontset, FONTSET_FRAME);
        let f = if self.is_frame(frame) {
            frame
        } else {
            self.frames.selected_frame
        };
        let font_group =
            self.fontset_get_font_group(fontset, if fallback { -1 } else { i64::from(c) })?;
        if !font_group.is_cons() {
            return Ok(font_group);
        }
        let mut vec = self.xcdr(font_group);
        if self.vector_items(vec).is_empty() {
            return Ok(Value::NIL);
        }
        let face_info = |lisp: &Lisp| {
            let face = face.expect("fontset_find_font needs a face");
            let face = lisp.face_from_id(f, face).expect("realized face");
            let ascii = lisp
                .face_from_id(f, face.ascii_face)
                .expect("realized face");
            (face.lface, ascii.font)
        };
        let mut charset_matched: i64 = 0;
        if self.vector_items(vec).len() > 1 {
            if self.xcar(font_group).as_fixnum() != Some(self.charsets.ordered_list_tick as i64) {
                let (_, ascii_font) = face_info(self);
                self.reorder_font_vector(font_group, ascii_font)?;
            }
            if charset_id >= 0 {
                let lcsetid = Value::fixnum(charset_id);
                for i in 0..self.vector_items(vec).len() {
                    let rfont_def = self.vector_items(vec)[i];
                    if rfont_def.is_nil() {
                        break;
                    }
                    let font_def = self.vector_items(rfont_def)[1];
                    if self.vector_items(font_def)[2] == lcsetid {
                        charset_matched = i as i64;
                        break;
                    }
                }
            }
        }
        let item = |lisp: &Lisp, v: Value, i: i64| lisp.vector_items(v)[i as usize];
        let size = |lisp: &Lisp, v: Value| lisp.vector_items(v).len() as i64;
        let mut i: i64 = 0;
        let (found_index, rfont_def) = 'search: {
            while i < size(self, vec) {
                let mut found_index = i;
                if i == 0 {
                    if charset_matched > 0 {
                        found_index = charset_matched;
                        charset_matched = -charset_matched;
                        i = -1;
                    }
                } else if i == -charset_matched {
                    let rfont_def = item(self, vec, i);
                    let font_def = item(self, rfont_def, 1);
                    while i + 1 < size(self, vec) {
                        let next = item(self, vec, i + 1);
                        if next.is_nil() || item(self, next, 1) != font_def {
                            break;
                        }
                        i += 1;
                    }
                    i += 1;
                    continue;
                }
                let mut rfont_def = item(self, vec, found_index);
                if rfont_def.is_nil() {
                    if i < 0 {
                        i += 1;
                        continue;
                    }
                    return Ok(sym::T);
                }
                if item(self, rfont_def, 0)
                    .as_fixnum()
                    .is_some_and(|id| id < 0)
                {
                    i += 1;
                    continue;
                }
                let mut font_object = item(self, rfont_def, 2);
                if font_object.is_nil() {
                    let font_def = item(self, rfont_def, 1);
                    if face.is_none() {
                        return Ok(Value::NIL);
                    }
                    let (lface, _) = face_info(self);
                    let spec = item(self, font_def, 0);
                    let font_entity = self.font_find_for_lface(f, &lface, spec, -1)?;
                    if font_entity.is_nil() {
                        self.fontset_vector_set(rfont_def, 0, Value::fixnum(-1));
                        i += 1;
                        continue;
                    }
                    font_object = self.font_open_for_lface(f, font_entity, &lface, spec)?;
                    if font_object.is_nil() {
                        self.fontset_vector_set(rfont_def, 0, Value::fixnum(-1));
                        i += 1;
                        continue;
                    }
                    self.fontset_vector_set(rfont_def, 2, font_object);
                }
                if self.font_has_char(f, font_object, c as i32) != 0 {
                    break 'search (found_index, rfont_def);
                }
                let font_def = item(self, rfont_def, 1);
                while found_index + 1 < size(self, vec) {
                    rfont_def = item(self, vec, found_index + 1);
                    if rfont_def.is_nil() || item(self, rfont_def, 1) != font_def {
                        break;
                    }
                    font_object = item(self, rfont_def, 2);
                    if !font_object.is_nil() && self.font_has_char(f, font_object, c as i32) != 0 {
                        break 'search (found_index + 1, rfont_def);
                    }
                    found_index += 1;
                }
                let (lface, _) = face_info(self);
                let spec = item(self, font_def, 0);
                let font_entity = self.font_find_for_lface(f, &lface, spec, c as i32)?;
                if !font_entity.is_nil() {
                    let font_object =
                        self.font_open_for_lface(f, font_entity, &lface, Value::NIL)?;
                    if font_object.is_nil() {
                        i += 1;
                        continue;
                    }
                    let rfont_def =
                        self.vector(vec![Value::NIL, font_def, font_object, Value::fixnum(0)]);
                    let old = self.vector_items(vec).to_vec();
                    let found_index = found_index + 1;
                    let mut new = Vec::with_capacity(old.len() + 1);
                    new.extend_from_slice(&old[..found_index as usize]);
                    new.push(rfont_def);
                    new.extend_from_slice(&old[found_index as usize..]);
                    let new_vec = self.vector(new);
                    self.heap.set_cdr(font_group, new_vec);
                    vec = new_vec;
                    break 'search (found_index, rfont_def);
                }
                if i >= 0 {
                    i = found_index;
                }
                i += 1;
            }
            self.fontset_set(fontset, c, Value::fixnum(0))?;
            return Ok(Value::NIL);
        };
        if fallback && found_index > 0 {
            let mut items = self.vector_items(vec).to_vec();
            for i in (1..=found_index as usize).rev() {
                items[i] = items[i - 1];
            }
            items[0] = rfont_def;
            for (i, x) in items.into_iter().enumerate() {
                self.fontset_vector_set(vec, i, x);
            }
        }
        Ok(rfont_def)
    }

    fn fontset_font(&mut self, fontset: Value, c: u32, face: Option<usize>, id: i64) -> LispResult {
        let cv = Value::fixnum(i64::from(c));
        self.font_deferred_log("current fontset: font for", cv, Value::NIL);
        let mut rfont_def = self.fontset_find_font(fontset, c, face, id, false)?;
        if self.is_vector(rfont_def) {
            return Ok(rfont_def);
        }
        if rfont_def.is_nil() {
            self.fontset_set(fontset, c, Value::fixnum(0))?;
        }
        let base_fontset = self.fontset_extra(fontset, FONTSET_BASE);
        let default_fontset = self.fontsets.default_fontset;
        let mut default_rfont_def = Value::NIL;
        if base_fontset != default_fontset {
            if self.fontset_extra(fontset, FONTSET_DEFAULT).is_nil() {
                let frame = self.fontset_extra(fontset, FONTSET_FRAME);
                let default = self.make_fontset(frame, Value::NIL, default_fontset)?;
                self.set_fontset_extra(fontset, FONTSET_DEFAULT, default);
            }
            self.font_deferred_log("default fontset: font for", cv, Value::NIL);
            let default = self.fontset_extra(fontset, FONTSET_DEFAULT);
            default_rfont_def = self.fontset_find_font(default, c, face, id, false)?;
            if self.is_vector(default_rfont_def) {
                return Ok(default_rfont_def);
            }
            if default_rfont_def.is_nil() {
                self.fontset_set(default, c, Value::fixnum(0))?;
            }
        }
        if rfont_def != sym::T {
            self.font_deferred_log("current fallback: font for", cv, Value::NIL);
            rfont_def = self.fontset_find_font(fontset, c, face, id, true)?;
            if self.is_vector(rfont_def) {
                return Ok(rfont_def);
            }
            self.fontset_set(fontset, c, sym::T)?;
        }
        if base_fontset != default_fontset && default_rfont_def != sym::T {
            self.font_deferred_log("default fallback: font for", cv, Value::NIL);
            let default = self.fontset_extra(fontset, FONTSET_DEFAULT);
            rfont_def = self.fontset_find_font(default, c, face, id, true)?;
            if self.is_vector(rfont_def) {
                return Ok(rfont_def);
            }
            self.fontset_set(default, c, sym::T)?;
        }
        Ok(Value::NIL)
    }

    fn fontset_charset_id(&mut self, pos: i64, object: Value) -> LispResult<i64> {
        if pos < 0 {
            return Ok(-1);
        }
        let mut charset = self.get_char_property(Value::fixnum(pos), sym::CHARSET, object)?;
        if !self.charsetp(charset) {
            return Ok(-1);
        }
        let alist = self.find_symbol_value(sym::FONT_ENCODING_CHARSET_ALIST);
        let val = self.assq(charset, alist);
        if val.is_cons() && self.charsetp(self.xcdr(val)) {
            charset = self.xcdr(val);
        }
        Ok(i64::from(self.check_charset_get_id(charset)?))
    }

    pub fn face_for_char(
        &mut self,
        f: Value,
        face_id: usize,
        c: u32,
        pos: i64,
        object: Value,
    ) -> LispResult<usize> {
        let face = self.face_from_id(f, face_id).expect("realized face");
        let (ascii_face, fontset_id, font) = (face.ascii_face, face.fontset, face.font);
        if !(0x80..=crate::character::MAX_5_BYTE_CHAR).contains(&c) {
            return Ok(ascii_face);
        }
        if !self.plain_value(sym::USE_DEFAULT_FONT_FOR_SYMBOLS).is_nil() && c > 0 {
            let table = self.find_symbol_value(sym::CHAR_SCRIPT_TABLE);
            if self.char_table_ref(table, c) == sym::SYMBOL {
                let ascii_font = self
                    .face_from_id(f, ascii_face)
                    .expect("realized face")
                    .font;
                if !ascii_font.is_nil() && self.font_has_char(f, ascii_font, c as i32) != 0 {
                    return Ok(ascii_face);
                }
            }
        }
        if fontset_id < 0 && font.is_nil() {
            return Ok(face_id);
        }
        let fontset = self.fontsets.table[fontset_id as usize];
        let id = self.fontset_charset_id(pos, object)?;
        let rfont_def = self.fontset_font(fontset, c, Some(face_id), id)?;
        if self.is_vector(rfont_def) {
            let rfont_face = self.vector_items(rfont_def)[0];
            if let Some(id) = rfont_face.as_fixnum() {
                return Ok(id as usize);
            }
            let font_object = self.vector_items(rfont_def)[2];
            let id = self.face_for_font(f, font_object, face_id)?;
            self.fontset_vector_set(rfont_def, 0, Value::fixnum(id as i64));
            Ok(id)
        } else {
            let nofont = self.fontset_extra(fontset, FONTSET_NOFONT_FACE);
            if let Some(id) = nofont.as_fixnum() {
                return Ok(id as usize);
            }
            let id = self.face_for_font(f, Value::NIL, face_id)?;
            self.set_fontset_extra(fontset, FONTSET_NOFONT_FACE, Value::fixnum(id as i64));
            Ok(id)
        }
    }

    pub fn font_for_char(
        &mut self,
        f: Value,
        face_id: usize,
        c: u32,
        pos: i64,
        object: Value,
    ) -> LispResult {
        let face = self.face_from_id(f, face_id).expect("realized face");
        let (ascii_face, fontset_id) = (face.ascii_face, face.fontset);
        if c < 0x80 {
            return Ok(self
                .face_from_id(f, ascii_face)
                .expect("realized face")
                .font);
        }
        let fontset = self.fontsets.table[fontset_id as usize];
        let id = self.fontset_charset_id(pos, object)?;
        let rfont_def = self.fontset_font(fontset, c, Some(face_id), id)?;
        Ok(if self.is_vector(rfont_def) {
            self.vector_items(rfont_def)[2]
        } else {
            Value::NIL
        })
    }

    pub fn make_fontset_for_ascii_face(
        &mut self,
        f: Value,
        base_fontset_id: i64,
    ) -> LispResult<i64> {
        let base_fontset = if base_fontset_id >= 0 {
            let base = self.fontsets.table[base_fontset_id as usize];
            if self.base_fontset_p(base) {
                base
            } else {
                self.fontset_extra(base, FONTSET_BASE)
            }
        } else {
            self.fontsets.default_fontset
        };
        let fontset = self.make_fontset(f, Value::NIL, base_fontset)?;
        Ok(self.fontset_id_of(fontset))
    }

    pub fn free_face_fontset(&mut self, face_fontset: i32) {
        let id = face_fontset as usize;
        let fontset = self.fontsets.table[id];
        if fontset.is_nil() {
            return;
        }
        self.fontsets.table[id] = Value::NIL;
        if id < self.fontsets.next_fontset_id {
            self.fontsets.next_fontset_id = id;
        }
        let default = self.fontset_extra(fontset, FONTSET_DEFAULT);
        if !default.is_nil() {
            let default_id = self.fontset_id_of(default) as usize;
            self.fontsets.table[default_id] = Value::NIL;
            if default_id < self.fontsets.next_fontset_id {
                self.fontsets.next_fontset_id = id;
            }
        }
    }

    fn free_realized_fontsets(&mut self, base: Value) -> LispResult<()> {
        for id in 0..self.fontsets.table.len() {
            let this = self.fontsets.table[id];
            if self.is_char_table(this) && self.fontset_extra(this, FONTSET_BASE) == base {
                crate::xfaces::f_clear_face_cache(self, &[sym::T])?;
                let frame = self.fontset_extra(this, FONTSET_FRAME);
                if self.frame_live_p(frame) {
                    self.recompute_basic_faces(frame)?;
                }
                break;
            }
        }
        Ok(())
    }

    fn check_fontset_name(&mut self, name: Value, frame: &mut Value) -> LispResult {
        let f = self.decode_live_frame(*frame)?;
        *frame = f;
        if name == sym::T {
            return Ok(self.fontsets.default_fontset);
        }
        let id = if name.is_nil() {
            if !self.frame_window_p(f) {
                return Err(self.error("Can't use fontsets in non-GUI frames"));
            }
            self.frame(f).fontset
        } else {
            self.check_string(name)?;
            let mut id = self.fs_query_fontset(name, 2)?;
            if id < 0 {
                id = self.fs_query_fontset(name, 0)?;
            }
            if id < 0 {
                let name = self.string_bytes(name).to_vec();
                return Err(self.verror(
                    "Fontset `%s' does not exist",
                    &[crate::doprnt::Arg::Str(&name)],
                ));
            }
            id
        };
        Ok(self.fontsets.table[id as usize])
    }

    fn fontset_alias_push(&mut self, name: Value, alias: Value) {
        let cell = self.cons(name, alias);
        let alist = self.find_symbol_value(sym::FONTSET_ALIAS_ALIST);
        let alist = self.cons(cell, alist);
        self.set_plain_value(sym::FONTSET_ALIAS_ALIST, alist);
    }

    pub fn fontset_from_font(&mut self, font_object: Value) -> LispResult<i64> {
        let base = self.stack.len();
        let r = self.fontset_from_font_1(font_object);
        self.stack.truncate(base);
        r
    }

    fn fontset_from_font_1(&mut self, font_object: Value) -> LispResult<i64> {
        use crate::font::{FONT_NAME_INDEX, FONT_REGISTRY_INDEX};
        let font_name = self.font_get_name(font_object);
        let font_spec = self.copy_font_spec(font_object)?;
        self.stack.push(font_spec);
        let registry = self.font_aref(font_spec, FONT_REGISTRY_INDEX);
        let auto = self.fontsets.auto_fontset_alist;
        let val = self.font_assoc(font_spec, auto)?;
        if val.is_cons() {
            let fontset = self.xcdr(val);
            return Ok(self.fontset_id_of(fontset));
        }
        let n = self.fontsets.num_auto_fontsets;
        self.fontsets.num_auto_fontsets += 1;
        let alias = if n == 0 {
            sym::FONTSET_STARTUP
        } else {
            self.intern(&format!("fontset-auto{n}"))
        };
        let fontset_spec = self.copy_font_spec(font_spec)?;
        self.font_aset(fontset_spec, FONT_REGISTRY_INDEX, alias);
        let name = crate::font::f_font_xlfd_name(self, &[fontset_spec, Value::NIL, sym::T])?;
        self.stack.push(name);
        let fontset = self.make_fontset(Value::NIL, name, Value::NIL)?;
        let alias_name = self.symbol_name(alias);
        self.fontset_alias_push(name, alias_name);
        let xlfd = self.font_aref(font_object, FONT_NAME_INDEX);
        let alias = crate::casefiddle::f_downcase(self, &[xlfd])?;
        self.fontset_alias_push(name, alias);
        let entry = self.cons(font_spec, fontset);
        self.fontsets.auto_fontset_alist = self.cons(entry, self.fontsets.auto_fontset_alist);
        let font_spec = crate::font::f_font_spec(self, &[])?;
        self.stack.push(font_spec);
        self.font_aset(font_spec, FONT_REGISTRY_INDEX, registry);
        let registry_name = self.symbol_name(registry);
        let mut target = self.find_font_encoding(registry_name)?;
        if target.is_cons() {
            target = self.xcdr(target);
        }
        if !self.charsetp(target) {
            target = sym::LATIN;
        }
        f_set_fontset_font(self, &[name, target, font_spec, Value::NIL, Value::NIL])?;
        f_set_fontset_font(self, &[name, Value::NIL, font_spec, Value::NIL, Value::NIL])?;
        self.set_fontset_extra(fontset, FONTSET_ASCII, font_name);
        Ok(self.fontset_id_of(fontset))
    }

    fn update_auto_fontset_alist(&mut self, fontset: Value) {
        let mut prev = Value::NIL;
        let mut tail = self.fontsets.auto_fontset_alist;
        while tail.is_cons() {
            if self.xcdr(self.xcar(tail)) == fontset {
                let rest = self.xcdr(tail);
                if prev.is_nil() {
                    self.fontsets.auto_fontset_alist = rest;
                } else {
                    self.heap.set_cdr(prev, rest);
                }
                break;
            }
            prev = tail;
            tail = self.xcdr(tail);
        }
    }
}

fn set_fontset_font_callback(lisp: &mut Lisp, range: Value, arg: Value) -> LispResult<()> {
    let mut range = range;
    let mut from = lisp.xcar(range).as_fixnum().unwrap_or(0);
    let to = lisp.xcdr(range).as_fixnum().unwrap_or(0);
    let items = lisp.vector_items(arg).to_vec();
    let (fontset, font_def, add, ascii) = (items[0], items[1], items[2], items[3]);
    let mut script_range_list = items[4];
    if ascii.is_nil() && from < 0x80 {
        if to < 0x80 {
            return Ok(());
        }
        from = 0x80;
        let rest = lisp.xcdr(range);
        range = lisp.cons(Value::fixnum(0x80), rest);
    }
    let script_from =
        |lisp: &Lisp, list: Value| lisp.xcar(lisp.xcar(list)).as_fixnum().unwrap_or(0);
    let script_to = |lisp: &Lisp, list: Value| lisp.xcdr(lisp.xcar(list)).as_fixnum().unwrap_or(0);
    while script_range_list.is_cons() && script_to(lisp, script_range_list) < from {
        let script_range = lisp.xcar(script_range_list);
        lisp.fontset_add_def(fontset, script_range, font_def, add)?;
        script_range_list = lisp.xcdr(script_range_list);
    }
    if script_range_list.is_cons() {
        if script_from(lisp, script_range_list) < from {
            let start = script_from(lisp, script_range_list);
            let rest = lisp.xcdr(range);
            range = lisp.cons(Value::fixnum(start), rest);
        }
        while script_range_list.is_cons() && script_to(lisp, script_range_list) <= to {
            script_range_list = lisp.xcdr(script_range_list);
        }
        if script_range_list.is_cons() && script_from(lisp, script_range_list) <= to {
            let head = lisp.xcar(script_range_list);
            lisp.heap.set_car(head, Value::fixnum(to + 1));
        }
    }
    lisp.fontset_add_def(fontset, range, font_def, add)?;
    if let crate::object::Vectorlike::Vector(items) = lisp.heap.vectorlike_mut(arg) {
        items[4] = script_range_list;
    }
    Ok(())
}

fn f_set_fontset_font(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let base = lisp.stack.len();
    let r = set_fontset_font(lisp, args);
    lisp.stack.truncate(base);
    r
}

fn characterp(v: Value) -> bool {
    v.as_fixnum()
        .is_some_and(|c| (0..=i64::from(crate::character::MAX_CHAR)).contains(&c))
}

fn set_fontset_font(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    use crate::doprnt::Arg;
    use crate::font::{FONT_FAMILY_INDEX, FONT_REGISTRY_INDEX};
    let (fontset, characters) = (args[0], args[1]);
    let (mut font_spec, mut frame, add) = (args[2], arg(args, 3), arg(args, 4));
    let fontset_obj = lisp.check_fontset_name(fontset, &mut frame)?;
    lisp.stack.push(fontset_obj);
    let mut fontname = Value::NIL;
    if font_spec.is_cons() {
        let spec = crate::font::f_font_spec(lisp, &[])?;
        lisp.stack.push(spec);
        let (family, registry) = (lisp.xcar(font_spec), lisp.xcdr(font_spec));
        lisp.font_parse_family_registry(family, registry, spec)?;
        font_spec = spec;
        fontname = crate::font::f_font_xlfd_name(lisp, &[font_spec, Value::NIL, sym::T])?;
    } else if font_spec.is_string() {
        fontname = font_spec;
        font_spec = crate::font::f_font_spec(lisp, &[sym::K_NAME, fontname])?;
    } else if lisp.font_spec_p(font_spec) {
        fontname = crate::font::f_font_xlfd_name(lisp, &[font_spec, Value::NIL, sym::T])?;
    } else if !font_spec.is_nil() {
        let message = lisp.string("Invalid font-spec");
        let data = lisp.list(&[message, font_spec]);
        return Err(lisp.signal(sym::FONT, data));
    }
    lisp.stack.push(font_spec);
    lisp.stack.push(fontname);
    let font_def = if !font_spec.is_nil() {
        let mut family = lisp.font_aref(font_spec, FONT_FAMILY_INDEX);
        if !family.is_nil() {
            family = lisp.symbol_name(family);
        }
        let mut registry = lisp.font_aref(font_spec, FONT_REGISTRY_INDEX);
        if !registry.is_nil() {
            let name = lisp.symbol_name(registry);
            registry = crate::casefiddle::f_downcase(lisp, &[name])?;
        }
        let dash = lisp.string("-");
        let name = crate::fns::f_concat(lisp, &[family, dash, registry])?;
        let mut encoding = lisp.find_font_encoding(name)?;
        if encoding.is_nil() {
            encoding = sym::ASCII;
        }
        let (encoding, repertory) = if encoding.is_symbol() {
            let id = Value::fixnum(i64::from(lisp.check_charset_get_id(encoding)?));
            (id, id)
        } else {
            let mut repertory = lisp.xcdr(encoding);
            let id = lisp.xcar(encoding);
            let id = Value::fixnum(i64::from(lisp.check_charset_get_id(id)?));
            if !repertory.is_nil() && repertory.is_symbol() {
                repertory = Value::fixnum(i64::from(lisp.check_charset_get_id(repertory)?));
            }
            (id, repertory)
        };
        lisp.vector(vec![font_spec, encoding, repertory])
    } else {
        Value::NIL
    };
    lisp.stack.push(font_def);
    let mut ascii_changed = false;
    let mut charset = None;
    let mut range_list;
    if characterp(characters) {
        if characters.as_fixnum().unwrap_or(0) < 0x80 {
            return Err(lisp.error("Can't set a font for partial ASCII range"));
        }
        let range = lisp.cons(characters, characters);
        range_list = lisp.list(&[range]);
    } else if characters.is_cons() {
        let (from, to) = (lisp.xcar(characters), lisp.xcdr(characters));
        let from = lisp.check_character(from)?;
        let to = lisp.check_character(to)?;
        if from < 0x80 {
            if from != 0 || to < 0x7F {
                return Err(lisp.error("Can't set a font for partial ASCII range"));
            }
            ascii_changed = true;
        }
        range_list = lisp.list(&[characters]);
    } else if characters.is_symbol() && !characters.is_nil() {
        range_list = Value::NIL;
        let script_table = lisp.find_symbol_value(sym::CHAR_SCRIPT_TABLE);
        let script_list = if lisp.is_char_table(script_table) {
            lisp.char_table(script_table)
                .extras
                .first()
                .copied()
                .unwrap_or(Value::NIL)
        } else {
            Value::NIL
        };
        let mut tail = script_list;
        let mut member = false;
        while tail.is_cons() {
            if lisp.xcar(tail) == characters {
                member = true;
                break;
            }
            tail = lisp.xcdr(tail);
        }
        if member {
            if characters == sym::LATIN {
                ascii_changed = true;
            }
            let ranges = lisp.map_char_table_collect(script_table);
            let mut items = Vec::new();
            for (from, to, val) in ranges {
                if val == characters {
                    let range =
                        lisp.cons(Value::fixnum(i64::from(from)), Value::fixnum(i64::from(to)));
                    lisp.stack.push(range);
                    items.push(range);
                }
            }
            range_list = lisp.list(&items);
        }
        if lisp.charsetp(characters) {
            let id = lisp.check_charset_get_id(characters)?;
            if lisp.charset(id).ascii_compatible_p {
                ascii_changed = true;
            }
            charset = Some(id);
        } else if range_list.is_nil() {
            let name = lisp.string_bytes(lisp.symbol_name(characters)).to_vec();
            return Err(lisp.verror("Invalid script or charset name: %s", &[Arg::Str(&name)]));
        }
    } else if characters.is_nil() {
        range_list = lisp.list(&[Value::NIL]);
    } else {
        return Err(lisp.error("Invalid second argument for setting a font in a fontset"));
    }
    lisp.stack.push(range_list);
    if ascii_changed {
        if font_spec.is_nil() {
            return Err(lisp.error("Can't set ASCII font to nil"));
        }
        let val = lisp.char_table_ref(fontset_obj, 0);
        if !val.is_nil() && add == sym::APPEND {
            ascii_changed = false;
        }
    }
    if let Some(id) = charset {
        let ascii = lisp.bool(ascii_changed);
        let arg = lisp.vector(vec![fontset_obj, font_def, add, ascii, range_list]);
        lisp.stack.push(arg);
        let (min_code, max_code) = (lisp.charset(id).min_code, lisp.charset(id).max_code);
        lisp.map_charset_chars(
            crate::charset::CharsetMapFunction::Rust(set_fontset_font_callback),
            arg,
            id,
            min_code,
            max_code,
        )?;
        range_list = lisp.vector_items(arg)[4];
    }
    let mut tail = range_list;
    while tail.is_cons() {
        let range = lisp.xcar(tail);
        lisp.fontset_add_def(fontset_obj, range, font_def, add)?;
        tail = lisp.xcdr(tail);
    }
    if ascii_changed {
        let fontset_id = lisp.fontset_id_of(fontset_obj);
        lisp.set_fontset_extra(fontset_obj, FONTSET_ASCII, fontname);
        let fontset_name = lisp.fontset_extra(fontset_obj, FONTSET_NAME);
        for fr in lisp.frame_list_vec() {
            if lisp.frame(fr).output_method == crate::terminal::OutputMethod::Initial
                || lisp.frame(fr).termcap_p()
            {
                continue;
            }
            if fontset_id != lisp.frame(fr).fontset {
                continue;
            }
            let lface = lisp
                .face_from_id(fr, crate::xfaces::DEFAULT_FACE_ID)
                .map(|face| face.lface);
            let font_object = match lface {
                Some(lface) => lisp.font_load_for_lface(fr, &lface, font_spec)?,
                None => lisp.font_open_by_spec(fr, font_spec)?,
            };
            if !font_object.is_nil() {
                lisp.stack.push(font_object);
                lisp.update_auto_fontset_alist(fontset_obj);
                let val = lisp.cons(fontset_name, font_object);
                let pair = lisp.cons(sym::FONT, val);
                let arg = lisp.list(&[pair]);
                lisp.stack.push(arg);
                if lisp.frame_window_p(fr) {
                    lisp.gui_set_frame_parameters_1(fr, arg, true)?;
                } else {
                    crate::frame::f_modify_frame_parameters(lisp, &[fr, arg])?;
                }
            }
        }
    }
    lisp.free_realized_fontsets(fontset_obj)?;
    Ok(Value::NIL)
}

fn f_new_fontset(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let base = lisp.stack.len();
    let r = new_fontset(lisp, args[0], args[1]);
    lisp.stack.truncate(base);
    r
}

fn new_fontset(lisp: &mut Lisp, name: Value, fontlist: Value) -> LispResult {
    use crate::doprnt::Arg;
    use crate::font::FONT_REGISTRY_INDEX;
    lisp.check_string(name)?;
    let name = crate::casefiddle::f_downcase(lisp, &[name])?;
    lisp.stack.push(name);
    let id = lisp.fs_query_fontset(name, 0)?;
    if id < 0 {
        let font_spec = crate::font::f_font_spec(lisp, &[])?;
        lisp.stack.push(font_spec);
        let bytes = lisp.string_bytes(name).to_vec();
        if !lisp.font_parse_xlfd(&bytes, font_spec)? {
            return Err(lisp.error("Fontset name must be in XLFD format"));
        }
        let short_name = lisp.font_aref(font_spec, FONT_REGISTRY_INDEX);
        let short = lisp.symbol_name(short_name);
        let short_bytes = lisp.string_bytes(short);
        if !short_bytes.starts_with(b"fontset-") || short_bytes.len() < 9 {
            return Err(lisp.error("Registry field of fontset name must be \"fontset-*\""));
        }
        lisp.fontset_alias_push(name, short);
        lisp.font_aset(font_spec, FONT_REGISTRY_INDEX, sym::ISO8859_1);
        let fontset = lisp.make_fontset(Value::NIL, name, Value::NIL)?;
        let Some(ascii) = lisp.font_unparse_xlfd(font_spec, 0, usize::MAX) else {
            return Err(lisp.verror(
                "Invalid fontset name (perhaps too long): %s",
                &[Arg::Str(&bytes)],
            ));
        };
        let ascii = lisp.unibyte_string(ascii.as_bytes());
        lisp.set_fontset_extra(fontset, FONTSET_ASCII, ascii);
    } else {
        let fontset = lisp.fontsets.table[id as usize];
        lisp.free_realized_fontsets(fontset)?;
        crate::chartab::f_set_char_table_range(lisp, &[fontset, sym::T, Value::NIL])?;
    }
    let mut tail = fontlist;
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        let script = lisp.car(elt)?;
        let elt = lisp.cdr(elt)?;
        if elt.is_cons() && (lisp.xcdr(elt).is_nil() || lisp.xcdr(elt).is_cons()) {
            let mut elt = elt;
            while elt.is_cons() {
                let spec = lisp.xcar(elt);
                f_set_fontset_font(lisp, &[name, script, spec, Value::NIL, sym::APPEND])?;
                elt = lisp.xcdr(elt);
            }
        } else {
            f_set_fontset_font(lisp, &[name, script, elt, Value::NIL, sym::APPEND])?;
        }
        tail = lisp.xcdr(tail);
    }
    if !tail.is_nil() {
        return Err(lisp.wrong_type(sym::LISTP, fontlist));
    }
    Ok(name)
}

fn f_fontset_font(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    use crate::font::{FONT_FAMILY_INDEX, FONT_REGISTRY_INDEX};
    let (name, ch, all) = (args[0], args[1], arg(args, 2));
    let mut frame = Value::NIL;
    let mut fontset = lisp.check_fontset_name(name, &mut frame)?;
    let c = lisp.check_character(ch)?;
    let base = lisp.stack.len();
    let mut list = Vec::new();
    loop {
        for i in 0..2 {
            let elt = if i == 0 {
                lisp.fontset_ref_or_default(fontset, c)
            } else {
                lisp.fontset_extra(fontset, FONTSET_FALLBACK)
            };
            if !lisp.is_vector(elt) {
                continue;
            }
            for val in lisp.vector_items(elt).to_vec() {
                if val.is_nil() {
                    lisp.stack.truncate(base);
                    return Ok(Value::NIL);
                }
                let def = lisp.vector_items(val).to_vec();
                let repertory = def[1];
                if let Some(id) = repertory.as_fixnum() {
                    match lisp.char_charset_p(c as i32, id as i32) {
                        Ok(true) => {}
                        Ok(false) => continue,
                        Err(e) => {
                            lisp.stack.truncate(base);
                            return Err(e);
                        }
                    }
                } else if lisp.is_char_table(repertory)
                    && lisp.char_table_ref(repertory, c).is_nil()
                {
                    continue;
                }
                let spec = def[0];
                let mut family = lisp.font_aref(spec, FONT_FAMILY_INDEX);
                if !family.is_nil() {
                    family = lisp.symbol_name(family);
                }
                let mut registry = lisp.font_aref(spec, FONT_REGISTRY_INDEX);
                if !registry.is_nil() {
                    registry = lisp.symbol_name(registry);
                }
                let val = lisp.cons(family, registry);
                if all.is_nil() {
                    lisp.stack.truncate(base);
                    return Ok(val);
                }
                lisp.stack.push(val);
                list.push(val);
            }
        }
        if fontset == lisp.fontsets.default_fontset {
            break;
        }
        fontset = lisp.fontsets.default_fontset;
    }
    let list = lisp.list(&list);
    lisp.stack.truncate(base);
    Ok(list)
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
