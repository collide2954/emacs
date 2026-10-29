use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::{LispString, PropRun};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("undo-boundary", 0, 0, f_undo_boundary),
    Subr::fixed(
        "combine-after-change-execute",
        0,
        0,
        f_combine_after_change_execute,
    ),
];

impl Lisp {
    pub fn undo_list(&self) -> Value {
        self.buffer_local_get(self.current_buffer, sym::BUFFER_UNDO_LIST)
            .unwrap_or(Value::NIL)
    }

    pub fn set_undo_list(&mut self, list: Value) {
        let b = self.current_buffer;
        self.buffer_local_set(b, sym::BUFFER_UNDO_LIST, list);
    }

    fn prepare_record(&mut self) {
        if self.pending_boundary.is_nil() {
            self.pending_boundary = self.cons(Value::NIL, Value::NIL);
        }
    }

    fn record_point(&mut self, beg: usize) {
        if !self.plain_value(sym::UNDO_INHIBIT_RECORD_POINT).is_nil() {
            return;
        }
        let list = self.undo_list();
        let at_boundary = !list.is_cons() || self.xcar(list).is_nil();
        if self.cur().text.modiff <= self.cur().text.save_modiff {
            self.record_first_change();
        }
        if at_boundary
            && self.point_before_last_command_or_undo != beg
            && self.buffer_before_last_command_or_undo == self.current_buffer
        {
            let list = self.undo_list();
            let p = Value::fixnum(self.point_before_last_command_or_undo as i64);
            let list = self.cons(p, list);
            self.set_undo_list(list);
        }
    }

    pub fn record_insert(&mut self, beg: usize, length: usize) {
        if self.undo_list() == sym::T {
            return;
        }
        self.prepare_record();
        self.record_point(beg);
        let list = self.undo_list();
        if list.is_cons() {
            let elt = self.xcar(list);
            if elt.is_cons()
                && self.xcar(elt).is_fixnum()
                && self.xcdr(elt).as_fixnum() == Some(beg as i64)
            {
                self.heap.set_cdr(elt, Value::fixnum((beg + length) as i64));
                return;
            }
        }
        let pair = self.cons(
            Value::fixnum(beg as i64),
            Value::fixnum((beg + length) as i64),
        );
        let list = self.cons(pair, list);
        self.set_undo_list(list);
    }

    fn record_marker_adjustments(&mut self, from: usize, to: usize) {
        self.prepare_record();
        let buffer = self.current_buffer;
        let markers = self.buffer(buffer).text.markers.clone();
        for m in markers {
            let mk = *self.marker(m);
            if from <= mk.charpos && mk.charpos <= to {
                let target = if mk.insertion_type { to } else { from };
                let adjustment = target as i64 - mk.charpos as i64;
                if adjustment != 0 {
                    let pair = self.cons(m, Value::fixnum(adjustment));
                    let list = self.undo_list();
                    let list = self.cons(pair, list);
                    self.set_undo_list(list);
                }
            }
        }
    }

    pub fn record_delete(&mut self, beg: usize, string: Value, record_markers: bool) {
        if self.undo_list() == sym::T {
            return;
        }
        self.prepare_record();
        self.record_point(beg);
        let chars = self.string_chars(string);
        let sbeg = if self.cur().pt == beg + chars {
            Value::fixnum(-(beg as i64))
        } else {
            Value::fixnum(beg as i64)
        };
        if record_markers {
            self.record_marker_adjustments(beg, beg + chars);
        }
        let pair = self.cons(string, sbeg);
        let list = self.undo_list();
        let list = self.cons(pair, list);
        self.set_undo_list(list);
    }

    pub fn record_first_change(&mut self) {
        if self.undo_list() == sym::T {
            return;
        }
        let modtime = Value::fixnum(0);
        let pair = self.cons(sym::T, modtime);
        let list = self.undo_list();
        let list = self.cons(pair, list);
        self.set_undo_list(list);
    }

    pub fn record_property_change(&mut self, beg: usize, length: usize, prop: Value, value: Value) {
        if self.undo_list() == sym::T {
            return;
        }
        self.prepare_record();
        if self.cur().text.modiff <= self.cur().text.save_modiff {
            self.record_first_change();
        }
        let lbeg = Value::fixnum(beg as i64);
        let lend = Value::fixnum((beg + length) as i64);
        let tail = self.cons(lbeg, lend);
        let entry = self.list_with_tail(&[Value::NIL, prop, value], tail);
        let list = self.undo_list();
        let list = self.cons(entry, list);
        self.set_undo_list(list);
    }

    pub fn modify_text(&mut self) {
        let t = &mut self.cur_mut().text;
        t.modiff += 1;
        t.chars_modiff = t.modiff;
    }

    pub fn prepare_to_modify_buffer(&mut self, start: usize, end: usize) -> LispResult<()> {
        if self.buffer_read_only_p() {
            self.barf_if_buffer_read_only(start)?;
        }
        self.verify_interval_modification(start, end)?;
        let base_buffer = if self.cur().base_buffer.is_nil() {
            self.current_buffer
        } else {
            self.cur().base_buffer
        };
        if self.var_true(sym::INHIBIT_MODIFICATION_HOOKS) {
            return Ok(());
        }
        let truename = self
            .buffer_local_get(base_buffer, sym::BUFFER_FILE_TRUENAME)
            .unwrap_or(Value::NIL);
        let filename = self
            .buffer_local_get(base_buffer, sym::BUFFER_FILE_NAME)
            .unwrap_or(Value::NIL);
        let text = &self.cur().text;
        if !truename.is_nil() && !filename.is_nil() && text.save_modiff >= text.modiff {
            crate::filelock::f_lock_file(self, &[truename])?;
        }
        self.signal_before_change(start, end)?;
        self.set(sym::DEACTIVATE_MARK, sym::T)?;
        Ok(())
    }

    pub fn signal_before_change(&mut self, start: usize, end: usize) -> LispResult<()> {
        if self.var_true(sym::INHIBIT_MODIFICATION_HOOKS) {
            return Ok(());
        }
        let count = self.specpdl.len();
        self.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
        let buffer = self.current_buffer;
        if self.cur().text.modiff <= self.cur().text.save_modiff {
            self.run_hook(sym::FIRST_CHANGE_HOOK)?;
        }
        if self.buffer_live_p(buffer) {
            let s = Value::fixnum(start as i64);
            let e = Value::fixnum(end as i64);
            let r = self.run_hook_with_args(&[sym::BEFORE_CHANGE_FUNCTIONS, s, e], |l, a| {
                l.funcall(a[0], &a[1..])?;
                Ok(Value::NIL)
            });
            if r.is_err() {
                let _ = self.set(sym::BEFORE_CHANGE_FUNCTIONS, Value::NIL);
            }
            r?;
        }
        self.unbind_to(count)
    }

    pub fn signal_after_change(
        &mut self,
        charpos: usize,
        lendel: usize,
        lenins: usize,
    ) -> LispResult<()> {
        if self.var_true(sym::INHIBIT_MODIFICATION_HOOKS) {
            return Ok(());
        }
        let count = self.specpdl.len();
        self.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
        let s = Value::fixnum(charpos as i64);
        let e = Value::fixnum((charpos + lenins) as i64);
        let d = Value::fixnum(lendel as i64);
        let r = self.run_hook_with_args(&[sym::AFTER_CHANGE_FUNCTIONS, s, e, d], |l, a| {
            l.funcall(a[0], &a[1..])?;
            Ok(Value::NIL)
        });
        if r.is_err() {
            let _ = self.set(sym::AFTER_CHANGE_FUNCTIONS, Value::NIL);
        }
        r?;
        self.unbind_to(count)
    }

    pub fn encode_for_buffer(&self, string: Value) -> (Vec<u8>, usize) {
        let s = self.heap.string_ref(string);
        let multibyte_buffer = self.cur().text.multibyte;
        match (s.multibyte, multibyte_buffer) {
            (true, true) | (false, false) => (s.data.clone(), s.chars),
            (false, true) => (character::str_to_multibyte(&s.data), s.chars),
            (true, false) => {
                let bytes: Vec<u8> = character::multibyte_chars(&s.data)
                    .map(character::char_to_byte8)
                    .collect();
                let n = bytes.len();
                (bytes, n)
            }
        }
    }

    pub fn insert_string(
        &mut self,
        string: Value,
        inherit: bool,
        before_markers: bool,
    ) -> LispResult<()> {
        let (bytes, nchars) = self.encode_for_buffer(string);
        let props = self.heap.string_ref(string).props.clone();
        self.stack.push(string);
        let r = self.insert_internal(&bytes, nchars, &props, inherit, before_markers);
        self.stack.pop();
        r
    }

    pub fn insert_char_codes(
        &mut self,
        chars: &[u32],
        inherit: bool,
        before_markers: bool,
    ) -> LispResult<()> {
        let multibyte = self.cur().text.multibyte;
        let mut bytes = Vec::with_capacity(chars.len());
        for &c in chars {
            if multibyte {
                character::push_char(&mut bytes, c);
            } else {
                bytes.push(character::char_to_byte8(c));
            }
        }
        self.insert_internal(&bytes, chars.len(), &[], inherit, before_markers)
    }

    pub fn insert_str(&mut self, text: &str) -> LispResult<()> {
        let s = self.string(text);
        self.insert_string(s, false, false)
    }

    pub fn insert_internal(
        &mut self,
        bytes: &[u8],
        nchars: usize,
        props: &[PropRun],
        inherit: bool,
        before_markers: bool,
    ) -> LispResult<()> {
        if nchars == 0 {
            return Ok(());
        }
        let pt = self.cur().pt;
        self.prepare_to_modify_buffer(pt, pt)?;
        let buffer = self.current_buffer;
        let (pt, pt_byte) = {
            let b = self.cur();
            (b.pt, b.pt_byte)
        };
        self.record_insert(pt, nchars);
        self.modify_text();
        {
            let b = self.buffer_mut(buffer);
            b.text.insert_bytes(pt_byte, bytes, nchars);
            b.zv += nchars;
            b.zv_byte += bytes.len();
        }
        self.adjust_markers_for_insert(buffer, pt, nchars, before_markers);
        self.adjust_overlays_for_insert(buffer, pt, nchars, before_markers);
        self.props_adjust_for_insert(buffer, pt, nchars, props, inherit);
        {
            let b = self.buffer_mut(buffer);
            b.pt = pt + nchars;
            b.pt_byte = pt_byte + bytes.len();
            if b.begv > pt {
                b.begv += nchars;
                b.begv_byte += bytes.len();
            }
        }
        self.signal_after_change(pt, 0, nchars)?;
        let new_pt = self.cur().pt;
        self.update_compositions(pt, new_pt, crate::composite::CHECK_BORDER)
    }

    pub fn insert_from_gap(
        &mut self,
        pos: usize,
        pos_byte: usize,
        bytes: &[u8],
        nchars: usize,
        before_markers: bool,
    ) {
        let buffer = self.current_buffer;
        let nchars = if self.cur().text.multibyte {
            nchars
        } else {
            bytes.len()
        };
        self.record_insert(pos, nchars);
        {
            let t = &mut self.cur_mut().text;
            t.modiff += nchars as i64;
            t.chars_modiff = t.modiff;
        }
        {
            let b = self.buffer_mut(buffer);
            b.text.insert_bytes(pos_byte, bytes, nchars);
            b.zv += nchars;
            b.zv_byte += bytes.len();
        }
        self.adjust_overlays_for_insert(buffer, pos, nchars, before_markers);
        self.adjust_markers_for_insert(buffer, pos, nchars, before_markers);
        self.props_adjust_for_insert(buffer, pos, nchars, &[], false);
        let b = self.buffer_mut(buffer);
        if pos < b.pt {
            b.pt += nchars;
            b.pt_byte += bytes.len();
        }
    }

    pub fn del_range_2(&mut self, from: usize, to: usize) -> Value {
        let buffer = self.current_buffer;
        let deleted = self.buffer_substring_value(from, to, true);
        self.stack.push(deleted);
        self.record_delete(from, deleted, true);
        self.modify_text();
        self.adjust_markers_for_delete(buffer, from, to);
        self.adjust_overlays_for_delete(buffer, from, to);
        self.props_adjust_for_delete(buffer, from, to);
        let n = to - from;
        let b = self.buffer_mut(buffer);
        let from_byte = b.text.char_to_byte(from);
        let to_byte = b.text.char_to_byte(to);
        let nbytes = to_byte - from_byte;
        b.text.delete_bytes(from_byte, to_byte, n);
        if b.pt > to {
            b.pt -= n;
        } else if b.pt > from {
            b.pt = from;
        }
        b.zv -= n;
        b.zv_byte -= nbytes;
        b.pt_byte = b.text.char_to_byte(b.pt);
        b.begv_byte = b.text.char_to_byte(b.begv);
        self.stack.pop();
        deleted
    }

    pub fn buffer_substring_value(&mut self, from: usize, to: usize, props: bool) -> Value {
        let b = self.cur();
        let from_byte = b.text.char_to_byte(from);
        let to_byte = b.text.char_to_byte(to);
        let data = b.text.slice(from_byte - 1, to_byte - 1);
        let multibyte = b.text.multibyte;
        let mut s = if multibyte {
            LispString::multibyte(data, to - from)
        } else {
            LispString::unibyte(data)
        };
        if props {
            s.props = b
                .text
                .props
                .iter()
                .filter(|r| r.end > from && r.start < to)
                .map(|r| PropRun {
                    start: r.start.max(from) - from,
                    end: r.end.min(to) - from,
                    plist: r.plist,
                })
                .collect();
        }
        self.heap.string(s)
    }

    pub fn del_range(&mut self, from: usize, to: usize) -> LispResult<()> {
        self.del_range_1(from, to, true).map(|_| ())
    }

    pub fn del_range_1(&mut self, from: usize, to: usize, prepare: bool) -> LispResult<Value> {
        let (begv, zv) = (self.cur().begv, self.cur().zv);
        let from = from.max(begv);
        let to = to.min(zv);
        if from >= to {
            return Ok(self.empty_multibyte);
        }
        if prepare {
            self.prepare_to_modify_buffer(from, to)?;
        }
        let (begv, zv) = (self.cur().begv, self.cur().zv);
        let from = from.max(begv);
        let to = to.min(zv);
        if from >= to {
            return Ok(self.empty_multibyte);
        }
        let buffer = self.current_buffer;
        let deleted = self.buffer_substring_value(from, to, true);
        self.stack.push(deleted);
        self.record_delete(from, deleted, true);
        self.modify_text();
        self.adjust_markers_for_delete(buffer, from, to);
        self.adjust_overlays_for_delete(buffer, from, to);
        self.props_adjust_for_delete(buffer, from, to);
        let n = to - from;
        {
            let b = self.buffer_mut(buffer);
            let from_byte = b.text.char_to_byte(from);
            let to_byte = b.text.char_to_byte(to);
            let nbytes = to_byte - from_byte;
            b.text.delete_bytes(from_byte, to_byte, n);
            if b.pt > to {
                b.pt -= n;
            } else if b.pt > from {
                b.pt = from;
            }
            b.zv -= n;
            b.zv_byte -= nbytes;
            b.pt_byte = b.text.char_to_byte(b.pt);
            b.begv_byte = b.text.char_to_byte(b.begv);
        }
        let r = self.signal_after_change(from, n, 0);
        let r = r.and_then(|()| self.update_compositions(from, from, crate::composite::CHECK_HEAD));
        self.stack.pop();
        r?;
        Ok(deleted)
    }

    pub fn replace_range_chars(
        &mut self,
        from: usize,
        to: usize,
        replacement: Value,
    ) -> LispResult<()> {
        let pt = self.cur().pt;
        self.stack.push(replacement);
        self.del_range(from, to)?;
        self.goto_char_internal(from);
        self.insert_string(replacement, false, false)?;
        self.stack.pop();
        let new_pt = if pt < from {
            pt
        } else if pt >= to {
            pt + self.string_chars(replacement) - (to - from)
        } else {
            from
        };
        let zv = self.cur().zv;
        self.goto_char_internal(new_pt.min(zv));
        Ok(())
    }
}

fn f_undo_boundary(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let list = lisp.undo_list();
    if list == sym::T {
        return Ok(Value::NIL);
    }
    let tem = lisp.car(list)?;
    if !tem.is_nil() {
        let boundary = if lisp.pending_boundary.is_nil() {
            lisp.cons(Value::NIL, list)
        } else {
            let b = lisp.pending_boundary;
            lisp.heap.set_cdr(b, list);
            lisp.pending_boundary = Value::NIL;
            b
        };
        lisp.set_undo_list(boundary);
    }
    lisp.set(sym::UNDO_AUTO__LAST_BOUNDARY_CAUSE, sym::EXPLICIT)?;
    lisp.point_before_last_command_or_undo = lisp.cur().pt;
    lisp.buffer_before_last_command_or_undo = lisp.current_buffer;
    Ok(Value::NIL)
}

fn f_combine_after_change_execute(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}
