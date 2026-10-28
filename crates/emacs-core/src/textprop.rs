use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::PropRun;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("text-properties-at", 1, 2, f_text_properties_at),
    Subr::fixed("get-text-property", 2, 3, f_get_text_property),
    Subr::fixed("get-char-property", 2, 3, f_get_char_property),
    Subr::fixed("get-pos-property", 2, 3, f_get_pos_property),
    Subr::fixed(
        "get-char-property-and-overlay",
        2,
        3,
        f_get_char_property_and_overlay,
    ),
    Subr::fixed("next-property-change", 1, 3, f_next_property_change),
    Subr::fixed(
        "next-single-property-change",
        2,
        4,
        f_next_single_property_change,
    ),
    Subr::fixed("previous-property-change", 1, 3, f_previous_property_change),
    Subr::fixed(
        "previous-single-property-change",
        2,
        4,
        f_previous_single_property_change,
    ),
    Subr::fixed(
        "next-char-property-change",
        1,
        2,
        f_next_char_property_change,
    ),
    Subr::fixed(
        "previous-char-property-change",
        1,
        2,
        f_previous_char_property_change,
    ),
    Subr::fixed(
        "next-single-char-property-change",
        2,
        4,
        f_next_single_char_property_change,
    ),
    Subr::fixed(
        "previous-single-char-property-change",
        2,
        4,
        f_previous_single_char_property_change,
    ),
    Subr::fixed("add-text-properties", 3, 4, f_add_text_properties),
    Subr::fixed("put-text-property", 4, 5, f_put_text_property),
    Subr::fixed("set-text-properties", 3, 4, f_set_text_properties),
    Subr::fixed("add-face-text-property", 3, 5, f_add_face_text_property),
    Subr::fixed("remove-text-properties", 3, 4, f_remove_text_properties),
    Subr::fixed(
        "remove-list-of-text-properties",
        3,
        4,
        f_remove_list_of_text_properties,
    ),
    Subr::fixed("text-property-any", 4, 5, f_text_property_any),
    Subr::fixed("text-property-not-all", 4, 5, f_text_property_not_all),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::DEFAULT_TEXT_PROPERTIES, Value::NIL);
    lisp.defvar(sym::CHAR_PROPERTY_ALIAS_ALIST, Value::NIL);
    lisp.defvar(sym::INHIBIT_POINT_MOTION_HOOKS, sym::T);
    let syntax = lisp.cons(sym::SYNTAX_TABLE, sym::T);
    lisp.stack.push(syntax);
    let display = lisp.cons(sym::DISPLAY, sym::T);
    lisp.stack.pop();
    let nonsticky = lisp.list(&[syntax, display]);
    lisp.defvar(sym::TEXT_PROPERTY_DEFAULT_NONSTICKY, nonsticky);
    lisp.make_variable_buffer_local_builtin(sym::TEXT_PROPERTY_DEFAULT_NONSTICKY);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Obj {
    Buffer(Value),
    String(Value),
}

pub(crate) struct IntervalRun {
    pub value: Value,
    pub start: i64,
    pub end: i64,
    pub at_first: bool,
    pub at_last: bool,
}

impl Lisp {
    fn decode_obj(&mut self, object: Value) -> LispResult<Obj> {
        if object.is_nil() {
            return Ok(Obj::Buffer(self.current_buffer));
        }
        if object.is_string() {
            return Ok(Obj::String(object));
        }
        if self.is_buffer(object) {
            return Ok(Obj::Buffer(object));
        }
        Err(self.wrong_type(sym::BUFFER_OR_STRING_P, object))
    }

    fn obj_runs(&self, obj: Obj) -> Vec<PropRun> {
        match obj {
            Obj::Buffer(b) => self.buffer(b).text.props.clone(),
            Obj::String(s) => self.heap.string_ref(s).props.clone(),
        }
    }

    fn set_obj_runs(&mut self, obj: Obj, runs: Vec<PropRun>) {
        let runs = self.normalize_runs(runs);
        match obj {
            Obj::Buffer(b) => self.buffer_mut(b).text.props = runs,
            Obj::String(s) => self.heap.string_mut(s).props = runs,
        }
    }

    fn obj_limits(&self, obj: Obj) -> (usize, usize) {
        match obj {
            Obj::Buffer(b) => {
                let buf = self.buffer(b);
                (buf.begv, buf.zv)
            }
            Obj::String(s) => (0, self.string_chars(s)),
        }
    }

    fn validate_range(&mut self, obj: Obj, start: Value, end: Value) -> LispResult<(usize, usize)> {
        let s = self.position_arg(start)?;
        let e = self.position_arg(end)?;
        let (s, e) = if s <= e { (s, e) } else { (e, s) };
        let (lo, hi) = self.obj_limits(obj);
        if s < lo as i64 || e > hi as i64 {
            return Err(self.args_out_of_range(start, end));
        }
        Ok((s as usize, e as usize))
    }

    fn validate_position(&mut self, obj: Obj, pos: Value) -> LispResult<usize> {
        let p = self.position_arg(pos)?;
        let (lo, hi) = self.obj_limits(obj);
        if p < lo as i64 || p > hi as i64 {
            return Err(self.args_out_of_range(pos, pos));
        }
        Ok(p as usize)
    }

    pub fn position_arg(&mut self, v: Value) -> LispResult<i64> {
        if let Some(n) = v.as_fixnum() {
            return Ok(n);
        }
        if self.is_marker(v) {
            return Ok(self.marker_position_checked(v)? as i64);
        }
        Err(self.wrong_type(sym::INTEGER_OR_MARKER_P, v))
    }

    fn plist_equal_props(&self, a: Value, b: Value) -> bool {
        if a == b {
            return true;
        }
        let len = |mut l: Value| {
            let mut n = 0;
            while l.is_cons() {
                n += 1;
                l = self.xcdr(l);
            }
            n
        };
        if len(a) != len(b) {
            return false;
        }
        let mut tail = a;
        while tail.is_cons() {
            let key = self.xcar(tail);
            let rest = self.xcdr(tail);
            if !rest.is_cons() {
                break;
            }
            let val = self.xcar(rest);
            let mut found = false;
            let mut t2 = b;
            while t2.is_cons() {
                let r2 = self.xcdr(t2);
                if !r2.is_cons() {
                    break;
                }
                if self.xcar(t2) == key {
                    if self.xcar(r2) != val {
                        return false;
                    }
                    found = true;
                    break;
                }
                t2 = self.xcdr(r2);
            }
            if !found {
                return false;
            }
            tail = self.xcdr(rest);
        }
        true
    }

    fn normalize_runs(&self, mut runs: Vec<PropRun>) -> Vec<PropRun> {
        runs.retain(|r| r.start < r.end && !r.plist.is_nil());
        runs.sort_by_key(|r| r.start);
        let mut out: Vec<PropRun> = Vec::with_capacity(runs.len());
        for run in runs {
            if let Some(last) = out.last_mut()
                && last.end == run.start
                && self.plist_equal_props(last.plist, run.plist)
            {
                last.end = run.end;
                continue;
            }
            out.push(run);
        }
        out
    }

    fn props_at(&self, obj: Obj, pos: usize) -> Value {
        for run in self.obj_runs(obj) {
            if run.start <= pos && pos < run.end {
                return run.plist;
            }
        }
        Value::NIL
    }

    pub fn get_property_and_range(
        &mut self,
        pos: i64,
        prop: Value,
        object: Value,
    ) -> LispResult<Option<(Value, usize, usize)>> {
        let obj = self.decode_obj(object)?;
        if self.obj_runs(obj).is_empty() || pos < 0 {
            return Ok(None);
        }
        let pos = pos as usize;
        let (lo, hi) = match obj {
            Obj::Buffer(b) => (1, self.buffer(b).z()),
            Obj::String(s) => (0, self.string_chars(s)),
        };
        if pos < lo || pos >= hi {
            return Ok(None);
        }
        let segs = self.segments(obj, lo, hi);
        let Some(i) = segs.iter().position(|s| s.start <= pos && pos < s.end) else {
            return Ok(None);
        };
        let val = self.lookup_char_property(segs[i].plist, prop);
        if val.is_nil() {
            return Ok(None);
        }
        let mut first = i;
        while first > 0 && self.lookup_char_property(segs[first - 1].plist, prop) == val {
            first -= 1;
        }
        let mut last = first;
        while last + 1 < segs.len() && self.lookup_char_property(segs[last + 1].plist, prop) == val
        {
            last += 1;
        }
        Ok(Some((val, segs[first].start, segs[last].end)))
    }

    pub(crate) fn interval_run(
        &mut self,
        position: i64,
        object: Value,
        prop: Value,
    ) -> LispResult<Option<IntervalRun>> {
        if object == sym::T {
            return Ok(None);
        }
        let obj = self.decode_obj(object)?;
        let (beg, end) = self.obj_limits(obj);
        if position < beg as i64 || position > end as i64 {
            let p = Value::fixnum(position);
            return Err(self.args_out_of_range(p, p));
        }
        if beg == end || self.obj_runs(obj).is_empty() {
            return Ok(None);
        }
        let (lo, hi) = match obj {
            Obj::Buffer(b) => (1, self.buffer(b).z()),
            Obj::String(s) => (0, self.string_chars(s)),
        };
        let segs = self.segments(obj, lo, hi);
        let pos = position as usize;
        let i = segs
            .iter()
            .position(|s| s.start <= pos && pos < s.end)
            .unwrap_or(segs.len() - 1);
        let val = self.lookup_char_property(segs[i].plist, prop);
        let mut first = i;
        while first > 0 && self.lookup_char_property(segs[first - 1].plist, prop) == val {
            first -= 1;
        }
        let mut last = i;
        while last + 1 < segs.len() && self.lookup_char_property(segs[last + 1].plist, prop) == val
        {
            last += 1;
        }
        Ok(Some(IntervalRun {
            value: val,
            start: segs[first].start as i64,
            end: segs[last].end as i64,
            at_first: first == 0,
            at_last: last + 1 == segs.len(),
        }))
    }

    pub fn text_property_at(&self, pos: usize, prop: Value) -> Value {
        let plist = self.props_at(Obj::Buffer(self.current_buffer), pos);
        self.plist_get_eq(plist, prop)
    }

    pub fn string_property_at(&self, string: Value, pos: usize, prop: Value) -> Value {
        let plist = self.props_at(Obj::String(string), pos);
        self.plist_get_eq(plist, prop)
    }

    fn segments(&self, obj: Obj, start: usize, end: usize) -> Vec<PropRun> {
        let runs = self.obj_runs(obj);
        let mut out = Vec::new();
        let mut pos = start;
        for run in runs.iter().filter(|r| r.end > start && r.start < end) {
            if run.start > pos {
                out.push(PropRun {
                    start: pos,
                    end: run.start,
                    plist: Value::NIL,
                });
            }
            out.push(PropRun {
                start: run.start.max(start),
                end: run.end.min(end),
                plist: run.plist,
            });
            pos = run.end.min(end);
        }
        if pos < end {
            out.push(PropRun {
                start: pos,
                end,
                plist: Value::NIL,
            });
        }
        out
    }

    fn replace_segments(&mut self, obj: Obj, start: usize, end: usize, new: Vec<PropRun>) {
        let runs = self.obj_runs(obj);
        let mut out = Vec::with_capacity(runs.len() + new.len());
        for run in runs {
            if run.end <= start || run.start >= end {
                out.push(run);
                continue;
            }
            if run.start < start {
                out.push(PropRun {
                    start: run.start,
                    end: start,
                    plist: run.plist,
                });
            }
            if run.end > end {
                out.push(PropRun {
                    start: end,
                    end: run.end,
                    plist: run.plist,
                });
            }
        }
        out.extend(new);
        self.set_obj_runs(obj, out);
    }

    fn begin_property_change(
        &mut self,
        obj: Obj,
        start: usize,
        end: usize,
    ) -> LispResult<Option<usize>> {
        let Obj::Buffer(b) = obj else {
            return Ok(None);
        };
        let count = self.specpdl.len();
        self.record_unwind_current_buffer();
        self.set_buffer_internal(b);
        self.prepare_to_modify_buffer_for_props(start, end)?;
        Ok(Some(count))
    }

    fn prepare_to_modify_buffer_for_props(&mut self, start: usize, end: usize) -> LispResult<()> {
        if self.buffer_read_only_p() {
            self.barf_if_buffer_read_only(start)?;
        }
        self.signal_before_change(start, end)?;
        self.cur_mut().text.modiff += 1;
        Ok(())
    }

    fn end_property_change(
        &mut self,
        count: Option<usize>,
        start: usize,
        end: usize,
    ) -> LispResult<()> {
        if let Some(count) = count {
            self.signal_after_change(start, end - start, end - start)?;
            self.unbind_to(count)?;
        }
        Ok(())
    }

    fn modify_props(
        &mut self,
        obj: Obj,
        start: usize,
        end: usize,
        f: &mut dyn FnMut(&mut Lisp, Value) -> LispResult<Option<Value>>,
    ) -> LispResult<bool> {
        if start >= end {
            return Ok(false);
        }
        let segs = self.segments(obj, start, end);
        let base = self.stack.len();
        let mut changed = Vec::with_capacity(segs.len());
        let mut any = false;
        for seg in &segs {
            match f(self, seg.plist)? {
                Some(new) => {
                    self.stack.push(new);
                    any = true;
                    changed.push(PropRun {
                        start: seg.start,
                        end: seg.end,
                        plist: new,
                    });
                }
                None => changed.push(*seg),
            }
        }
        if !any {
            self.stack.truncate(base);
            return Ok(false);
        }
        let count = self.begin_property_change(obj, start, end)?;
        if let Obj::Buffer(_) = obj {
            for (old, new) in segs.iter().zip(&changed) {
                if old.plist != new.plist {
                    self.record_plist_change(old, new.plist);
                }
            }
        }
        self.replace_segments(obj, start, end, changed);
        self.stack.truncate(base);
        self.end_property_change(count, start, end)?;
        Ok(true)
    }

    fn record_plist_change(&mut self, old: &PropRun, new_plist: Value) {
        let mut keys = Vec::new();
        let mut tail = new_plist;
        while tail.is_cons() {
            keys.push(self.xcar(tail));
            tail = self.cdr_safe(self.xcdr(tail));
        }
        let mut tail = old.plist;
        while tail.is_cons() {
            let k = self.xcar(tail);
            if !keys.contains(&k) {
                keys.push(k);
            }
            tail = self.cdr_safe(self.xcdr(tail));
        }
        for key in keys {
            let before = self.plist_get_eq(old.plist, key);
            let after = self.plist_get_eq(new_plist, key);
            if before != after {
                self.record_property_change(old.start, old.end - old.start, key, before);
            }
        }
    }

    fn plist_with(&mut self, plist: Value, prop: Value, value: Value) -> Value {
        let mut items = Vec::new();
        let mut found = false;
        let mut tail = plist;
        while tail.is_cons() {
            let k = self.xcar(tail);
            let rest = self.xcdr(tail);
            let v = self.car_safe(rest);
            if k == prop {
                items.push(k);
                items.push(value);
                found = true;
            } else {
                items.push(k);
                items.push(v);
            }
            tail = self.cdr_safe(rest);
        }
        if !found {
            items.insert(0, value);
            items.insert(0, prop);
        }
        self.list(&items)
    }

    fn plist_without(&mut self, plist: Value, props: &[Value]) -> Value {
        let mut items = Vec::new();
        let mut tail = plist;
        while tail.is_cons() {
            let k = self.xcar(tail);
            let rest = self.xcdr(tail);
            if !props.contains(&k) {
                items.push(k);
                items.push(self.car_safe(rest));
            }
            tail = self.cdr_safe(rest);
        }
        self.list(&items)
    }

    fn plist_pairs(&self, plist: Value) -> Vec<(Value, Value)> {
        let mut out = Vec::new();
        let mut tail = plist;
        while tail.is_cons() {
            let k = self.xcar(tail);
            let rest = self.xcdr(tail);
            out.push((k, self.car_safe(rest)));
            tail = self.cdr_safe(rest);
        }
        out
    }

    pub fn add_text_properties(
        &mut self,
        start: Value,
        end: Value,
        properties: Value,
        object: Value,
        replace: bool,
    ) -> LispResult<bool> {
        let obj = self.decode_obj(object)?;
        let (s, e) = self.validate_range(obj, start, end)?;
        let pairs = self.plist_pairs(properties);
        self.stack.push(properties);
        let r = self.modify_props(obj, s, e, &mut |lisp, plist| {
            let mut new = plist;
            let mut changed = false;
            for &(k, v) in &pairs {
                let current = lisp.plist_member_eq(new, k);
                if current && lisp.plist_get_eq(new, k) == v && !replace {
                    continue;
                }
                if current && lisp.plist_get_eq(new, k) == v {
                    continue;
                }
                new = lisp.plist_with(new, k, v);
                changed = true;
            }
            Ok(changed.then_some(new))
        });
        self.stack.pop();
        r
    }

    fn plist_member_eq(&self, plist: Value, prop: Value) -> bool {
        let mut tail = plist;
        while tail.is_cons() {
            if self.xcar(tail) == prop {
                return true;
            }
            tail = self.cdr_safe(self.xcdr(tail));
        }
        false
    }

    pub fn set_text_properties(
        &mut self,
        start: Value,
        end: Value,
        properties: Value,
        object: Value,
    ) -> LispResult<()> {
        let obj = self.decode_obj(object)?;
        if let Obj::String(s) = obj {
            let len = self.string_chars(s);
            if start == Value::fixnum(0) && end == Value::fixnum(len as i64) && properties.is_nil()
            {
                self.heap.string_mut(s).props.clear();
                return Ok(());
            }
        }
        let (s, e) = self.validate_range(obj, start, end)?;
        if s == e {
            return Ok(());
        }
        let segs = self.segments(obj, s, e);
        if segs.iter().all(|seg| seg.plist.is_nil()) && properties.is_nil() {
            return Ok(());
        }
        let count = self.begin_property_change(obj, s, e)?;
        if let Obj::Buffer(_) = obj {
            for seg in &segs {
                self.record_plist_change(seg, properties);
            }
        }
        let copy = self.copy_plist(properties);
        self.replace_segments(
            obj,
            s,
            e,
            vec![PropRun {
                start: s,
                end: e,
                plist: copy,
            }],
        );
        self.end_property_change(count, s, e)
    }

    fn copy_plist(&mut self, plist: Value) -> Value {
        let items = self.list_to_vec(plist).unwrap_or_default();
        self.list(&items)
    }

    pub fn props_adjust_for_insert(
        &mut self,
        buffer: Value,
        pos: usize,
        n: usize,
        props: &[PropRun],
        inherit: bool,
    ) {
        let mut runs = std::mem::take(&mut self.buffer_mut(buffer).text.props);
        let mut inserted_plist = Value::NIL;
        if inherit {
            let left = runs
                .iter()
                .find(|r| r.start < pos && pos <= r.end)
                .map_or(Value::NIL, |r| r.plist);
            let right = runs
                .iter()
                .find(|r| r.start <= pos && pos < r.end)
                .map_or(Value::NIL, |r| r.plist);
            inserted_plist = self.sticky_plist(left, right);
        }
        let mut out = Vec::with_capacity(runs.len() + props.len() + 2);
        for run in runs.drain(..) {
            if run.end <= pos {
                out.push(run);
            } else if run.start >= pos {
                out.push(PropRun {
                    start: run.start + n,
                    end: run.end + n,
                    plist: run.plist,
                });
            } else {
                out.push(PropRun {
                    start: run.start,
                    end: pos,
                    plist: run.plist,
                });
                out.push(PropRun {
                    start: pos + n,
                    end: run.end + n,
                    plist: run.plist,
                });
            }
        }
        if !inserted_plist.is_nil() {
            out.push(PropRun {
                start: pos,
                end: pos + n,
                plist: inserted_plist,
            });
        }
        for p in props {
            if inherit && !inserted_plist.is_nil() {
                let merged = self.merge_plists(inserted_plist, p.plist);
                out.retain(|r| !(r.start == pos && r.end == pos + n && r.plist == inserted_plist));
                out.push(PropRun {
                    start: pos + p.start,
                    end: pos + p.end,
                    plist: merged,
                });
            } else {
                out.push(PropRun {
                    start: pos + p.start,
                    end: pos + p.end,
                    plist: p.plist,
                });
            }
        }
        let out = self.normalize_runs(out);
        self.buffer_mut(buffer).text.props = out;
    }

    fn merge_plists(&mut self, base: Value, over: Value) -> Value {
        let mut result = base;
        for (k, v) in self.plist_pairs(over) {
            result = self.plist_with(result, k, v);
        }
        result
    }

    fn sticky_plist(&mut self, left: Value, right: Value) -> Value {
        let default_nonsticky = self.find_symbol_value(sym::TEXT_PROPERTY_DEFAULT_NONSTICKY);
        let rear_nonsticky = self.plist_get_eq(left, sym::REAR_NONSTICKY);
        let front_sticky = self.plist_get_eq(right, sym::FRONT_STICKY);
        let mut items = Vec::new();
        for (k, v) in self.plist_pairs(left) {
            if k == sym::REAR_NONSTICKY || k == sym::FRONT_STICKY {
                continue;
            }
            let nonsticky = rear_nonsticky == sym::T
                || (rear_nonsticky.is_cons() && !self.memq(k, rear_nonsticky).is_nil())
                || {
                    let entry = self.assq(k, default_nonsticky);
                    !self.cdr_safe(entry).is_nil()
                };
            if !nonsticky {
                items.push(k);
                items.push(v);
            }
        }
        for (k, v) in self.plist_pairs(right) {
            let sticky = front_sticky == sym::T
                || (front_sticky.is_cons() && !self.memq(k, front_sticky).is_nil());
            if sticky && !items.chunks(2).any(|c| c[0] == k) {
                items.push(k);
                items.push(v);
            }
        }
        self.list(&items)
    }

    pub fn props_adjust_for_delete(&mut self, buffer: Value, from: usize, to: usize) {
        let n = to - from;
        let runs = std::mem::take(&mut self.buffer_mut(buffer).text.props);
        let mut out = Vec::with_capacity(runs.len());
        for run in runs {
            let start = if run.start >= to {
                run.start - n
            } else if run.start > from {
                from
            } else {
                run.start
            };
            let end = if run.end >= to {
                run.end - n
            } else if run.end > from {
                from
            } else {
                run.end
            };
            if start < end {
                out.push(PropRun {
                    start,
                    end,
                    plist: run.plist,
                });
            }
        }
        let out = self.normalize_runs(out);
        self.buffer_mut(buffer).text.props = out;
    }

    pub fn verify_interval_modification(&mut self, start: usize, end: usize) -> LispResult<()> {
        if !self.plain_value(sym::INHIBIT_READ_ONLY).is_nil() {
            return Ok(());
        }
        let buffer = self.current_buffer;
        let runs = self.buffer(buffer).text.props.clone();
        if runs.is_empty() {
            return Ok(());
        }
        let check = |lisp: &mut Lisp, plist: Value| -> LispResult<()> {
            let ro = lisp.plist_get_eq(plist, sym::READ_ONLY);
            if ro.is_nil() {
                return Ok(());
            }
            let inhibit = lisp.plain_value(sym::INHIBIT_READ_ONLY);
            if inhibit.is_cons() && !lisp.memq(ro, inhibit).is_nil() {
                return Ok(());
            }
            let data = if ro == sym::T {
                Value::NIL
            } else {
                lisp.list(&[ro])
            };
            Err(lisp.signal(sym::TEXT_READ_ONLY, data))
        };
        if start == end {
            let before = runs
                .iter()
                .find(|r| r.start < start && start <= r.end)
                .map_or(Value::NIL, |r| r.plist);
            let after = runs
                .iter()
                .find(|r| r.start <= start && start < r.end)
                .map_or(Value::NIL, |r| r.plist);
            let inherited = self.sticky_plist(before, after);
            let ro_after = self.plist_get_eq(after, sym::READ_ONLY);
            let ro_before = self.plist_get_eq(before, sym::READ_ONLY);
            if !ro_after.is_nil() && ro_after == ro_before {
                return check(self, after);
            }
            return check(self, inherited);
        }
        for run in runs.iter().filter(|r| r.end > start && r.start < end) {
            check(self, run.plist)?;
        }
        Ok(())
    }

    pub(crate) fn get_char_property_and_overlay(
        &mut self,
        position: Value,
        prop: Value,
        object: Value,
    ) -> LispResult<(Value, Value)> {
        let obj = self.decode_obj(object)?;
        if let Obj::Buffer(b) = obj {
            let pos = self.validate_position(obj, position)?;
            if let Some((val, ov)) = self.overlay_property_at(b, pos, prop) {
                return Ok((val, ov));
            }
            let plist = self.props_at(obj, pos);
            return Ok((self.lookup_char_property(plist, prop), Value::NIL));
        }
        let pos = self.validate_position(obj, position)?;
        let plist = self.props_at(obj, pos);
        Ok((self.lookup_char_property(plist, prop), Value::NIL))
    }

    pub fn lookup_char_property(&self, plist: Value, prop: Value) -> Value {
        let direct = self.plist_get_eq(plist, prop);
        if self.plist_member_eq(plist, prop) {
            return direct;
        }
        let category = self.plist_get_eq(plist, sym::CATEGORY);
        if category.is_symbol() && !category.is_nil() {
            let v = self.get(category, prop);
            if !v.is_nil() {
                return v;
            }
        }
        let alist = self.find_symbol_value(sym::CHAR_PROPERTY_ALIAS_ALIST);
        let entry = self.assq(prop, alist);
        let mut aliases = self.cdr_safe(entry);
        while aliases.is_cons() {
            let alias = self.xcar(aliases);
            if self.plist_member_eq(plist, alias) {
                return self.plist_get_eq(plist, alias);
            }
            aliases = self.xcdr(aliases);
        }
        let defaults = self.find_symbol_value(sym::DEFAULT_TEXT_PROPERTIES);
        if defaults.is_cons() {
            return self.plist_get_eq(defaults, prop);
        }
        Value::NIL
    }

    fn property_change(
        &mut self,
        position: Value,
        object: Value,
        limit: Value,
        prop: Option<Value>,
        forward: bool,
    ) -> LispResult {
        let obj = self.decode_obj(object)?;
        let pos = self.validate_position(obj, position)?;
        let (lo, hi) = self.obj_limits(obj);
        if forward && prop.is_none() && limit == sym::T {
            if self.obj_runs(obj).is_empty() {
                return Ok(Value::fixnum(hi as i64));
            }
            let (first, last) = match obj {
                Obj::Buffer(b) => (1, self.buffer(b).z()),
                Obj::String(s) => (0, self.string_chars(s)),
            };
            let segs = self.segments(obj, first, last);
            let next = segs
                .iter()
                .position(|s| s.start <= pos && pos < s.end)
                .and_then(|i| segs.get(i + 1));
            return Ok(Value::fixnum(next.map_or(hi, |s| s.start) as i64));
        }
        let limit_pos = if limit.is_nil() {
            None
        } else {
            Some(self.position_arg(limit)? as usize)
        };
        let value_at = |lisp: &Lisp, p: usize| -> Value {
            let plist = lisp.props_at(obj, p);
            match prop {
                Some(prop) => lisp.plist_get_eq(plist, prop),
                None => plist,
            }
        };
        let same = |lisp: &Lisp, a: Value, b: Value| -> bool {
            match prop {
                Some(_) => a == b,
                None => lisp.plist_equal_props(a, b),
            }
        };
        let mut boundaries: Vec<usize> = self
            .obj_runs(obj)
            .iter()
            .flat_map(|r| [r.start, r.end])
            .collect();
        boundaries.sort_unstable();
        boundaries.dedup();
        if forward {
            if pos >= hi {
                return Ok(limit_pos.map_or(Value::NIL, |l| Value::fixnum(l as i64)));
            }
            let here = value_at(self, pos);
            for &b in boundaries.iter().filter(|&&b| b > pos && b <= hi) {
                if let Some(l) = limit_pos
                    && b >= l
                {
                    return Ok(Value::fixnum(l as i64));
                }
                let there = if b < hi {
                    value_at(self, b)
                } else {
                    Value::NIL
                };
                if b == hi && prop.is_some() {
                    break;
                }
                if !same(self, here, there) || b == hi {
                    return Ok(Value::fixnum(b as i64));
                }
            }
            Ok(match limit_pos {
                Some(l) => Value::fixnum(l as i64),
                None => Value::NIL,
            })
        } else {
            if pos <= lo {
                return Ok(limit_pos.map_or(Value::NIL, |l| Value::fixnum(l as i64)));
            }
            let here = value_at(self, pos - 1);
            for &b in boundaries.iter().rev().filter(|&&b| b < pos && b >= lo) {
                if let Some(l) = limit_pos
                    && b <= l
                {
                    return Ok(Value::fixnum(l as i64));
                }
                let there = if b > lo {
                    value_at(self, b - 1)
                } else {
                    Value::NIL
                };
                if b == lo && prop.is_some() {
                    break;
                }
                if !same(self, here, there) || b == lo {
                    return Ok(Value::fixnum(b as i64));
                }
            }
            Ok(match limit_pos {
                Some(l) => Value::fixnum(l as i64),
                None => Value::NIL,
            })
        }
    }
}

fn f_text_properties_at(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let obj = lisp.decode_obj(args[1])?;
    let pos = lisp.validate_position(obj, args[0])?;
    let (_, hi) = lisp.obj_limits(obj);
    if pos == hi {
        return Ok(Value::NIL);
    }
    Ok(lisp.props_at(obj, pos))
}

pub(crate) fn f_get_text_property(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let plist = f_text_properties_at(lisp, &[args[0], args[2]])?;
    Ok(lisp.lookup_char_property(plist, args[1]))
}

fn f_get_char_property(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.get_char_property(args[0], args[1], args[2])
}

fn f_get_pos_property(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.get_pos_property(args[0], args[1], args[2])
}

impl Lisp {
    pub fn get_char_property(&mut self, position: Value, prop: Value, object: Value) -> LispResult {
        Ok(self
            .get_char_property_and_overlay(position, prop, object)?
            .0)
    }

    pub fn get_pos_property(&mut self, position: Value, prop: Value, object: Value) -> LispResult {
        let obj = self.decode_obj(object)?;
        let pos = self.validate_position(obj, position)?;
        if let Obj::Buffer(b) = obj
            && let Some((val, _)) = self.overlay_property_at_pos_stickiness(b, pos, prop)
        {
            return Ok(val);
        }
        let after = self.props_at(obj, pos);
        let before = if pos > self.obj_limits(obj).0 {
            self.props_at(obj, pos - 1)
        } else {
            Value::NIL
        };
        let sticky = self.sticky_plist(before, after);
        Ok(self.plist_get_eq(sticky, prop))
    }
}

fn f_get_char_property_and_overlay(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (v, o) = lisp.get_char_property_and_overlay(args[0], args[1], args[2])?;
    Ok(lisp.cons(v, o))
}

pub(crate) fn f_next_property_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.property_change(args[0], args[1], args[2], None, true)
}

pub(crate) fn f_next_single_property_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.property_change(args[0], args[2], args[3], Some(args[1]), true)
}

fn f_previous_property_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.property_change(args[0], args[1], args[2], None, false)
}

pub(crate) fn f_previous_single_property_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.property_change(args[0], args[2], args[3], Some(args[1]), false)
}

fn char_property_change(
    lisp: &mut Lisp,
    position: Value,
    limit: Value,
    forward: bool,
) -> LispResult {
    let r = lisp.property_change(position, Value::NIL, limit, None, forward)?;
    let buffer = lisp.current_buffer;
    let pos = lisp.position_arg(position)? as usize;
    let ov = lisp.next_overlay_change(buffer, pos, forward);
    let (begv, zv) = (lisp.cur().begv, lisp.cur().zv);
    let mut best = match r.as_fixnum() {
        Some(n) => n as usize,
        None => {
            if forward {
                zv
            } else {
                begv
            }
        }
    };
    if forward {
        best = best.min(ov);
    } else {
        best = best.max(ov);
    }
    if !limit.is_nil() {
        let l = lisp.position_arg(limit)? as usize;
        best = if forward { best.min(l) } else { best.max(l) };
    }
    Ok(Value::fixnum(best as i64))
}

fn f_next_char_property_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    char_property_change(lisp, args[0], args[1], true)
}

fn f_previous_char_property_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    char_property_change(lisp, args[0], args[1], false)
}

fn single_char_property_change(lisp: &mut Lisp, args: &[Value], forward: bool) -> LispResult {
    let (position, prop, object, limit) = (args[0], args[1], args[2], args[3]);
    if object.is_string() {
        let r = lisp.property_change(position, object, limit, Some(prop), forward)?;
        if r.is_nil() {
            return Ok(if limit.is_nil() {
                let len = lisp.string_chars(object);
                Value::fixnum(if forward { len as i64 } else { 0 })
            } else {
                limit
            });
        }
        return Ok(r);
    }
    let count = lisp.specpdl.len();
    if !object.is_nil() {
        lisp.record_unwind_current_buffer();
        let b = lisp.check_buffer(object)?;
        lisp.set_buffer_internal(b);
    }
    let mut pos = lisp.position_arg(position)?;
    let (begv, zv) = (lisp.cur().begv as i64, lisp.cur().zv as i64);
    let lim = if limit.is_nil() {
        if forward { zv } else { begv }
    } else {
        let l = lisp.position_arg(limit)?;
        if forward { l.min(zv) } else { l.max(begv) }
    };
    let result = (|| -> LispResult {
        if forward {
            if pos >= lim {
                return Ok(Value::fixnum(lim.max(pos.min(zv))));
            }
            let initial = lisp
                .get_char_property_and_overlay(Value::fixnum(pos), prop, Value::NIL)?
                .0;
            loop {
                pos = char_property_change(lisp, Value::fixnum(pos), Value::fixnum(lim), true)?
                    .as_fixnum()
                    .unwrap_or(lim);
                if pos >= lim {
                    return Ok(Value::fixnum(lim));
                }
                let v = lisp
                    .get_char_property_and_overlay(Value::fixnum(pos), prop, Value::NIL)?
                    .0;
                if v != initial {
                    return Ok(Value::fixnum(pos));
                }
            }
        } else {
            if pos <= lim {
                return Ok(Value::fixnum(lim.min(pos.max(begv))));
            }
            let initial = lisp
                .get_char_property_and_overlay(Value::fixnum(pos - 1), prop, Value::NIL)?
                .0;
            loop {
                pos = char_property_change(lisp, Value::fixnum(pos), Value::fixnum(lim), false)?
                    .as_fixnum()
                    .unwrap_or(lim);
                if pos <= lim {
                    return Ok(Value::fixnum(lim));
                }
                let v = lisp
                    .get_char_property_and_overlay(Value::fixnum(pos - 1), prop, Value::NIL)?
                    .0;
                if v != initial {
                    return Ok(Value::fixnum(pos));
                }
            }
        }
    })();
    lisp.unbind_to_value(count, result)
}

pub(crate) fn f_next_single_char_property_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    single_char_property_change(lisp, args, true)
}

pub(crate) fn f_previous_single_char_property_change(
    lisp: &mut Lisp,
    args: &[Value],
) -> LispResult {
    single_char_property_change(lisp, args, false)
}

fn f_add_text_properties(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let changed = lisp.add_text_properties(args[0], args[1], args[2], args[3], false)?;
    Ok(lisp.bool(changed))
}

pub(crate) fn f_put_text_property(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let plist = lisp.list(&[args[2], args[3]]);
    lisp.add_text_properties(args[0], args[1], plist, args[4], true)?;
    Ok(Value::NIL)
}

fn f_set_text_properties(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.set_text_properties(args[0], args[1], args[2], args[3])?;
    Ok(sym::T)
}

fn f_add_face_text_property(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (start, end, face, appendp, object) = (args[0], args[1], args[2], args[3], args[4]);
    let obj = lisp.decode_obj(object)?;
    let (s, e) = lisp.validate_range(obj, start, end)?;
    lisp.stack.push(face);
    let r = lisp.modify_props(obj, s, e, &mut |l, plist| {
        let old = l.plist_get_eq(plist, sym::FACE);
        let new = if old.is_nil() {
            face
        } else {
            let old_list = if old.is_cons() && !l.keywordp(l.xcar(old)) {
                old
            } else {
                l.list(&[old])
            };
            if appendp.is_nil() {
                l.cons(face, old_list)
            } else {
                let items = l.list_to_vec(old_list)?;
                let mut items = items;
                items.push(face);
                l.list(&items)
            }
        };
        Ok(Some(l.plist_with(plist, sym::FACE, new)))
    });
    lisp.stack.pop();
    r?;
    Ok(Value::NIL)
}

fn f_remove_text_properties(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pairs = lisp.plist_pairs(args[2]);
    let props: Vec<Value> = pairs.iter().map(|p| p.0).collect();
    remove_props(lisp, args[0], args[1], &props, args[3])
}

pub(crate) fn f_remove_list_of_text_properties(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let props = lisp.list_to_vec(args[2])?;
    remove_props(lisp, args[0], args[1], &props, args[3])
}

fn remove_props(
    lisp: &mut Lisp,
    start: Value,
    end: Value,
    props: &[Value],
    object: Value,
) -> LispResult {
    let obj = lisp.decode_obj(object)?;
    let (s, e) = lisp.validate_range(obj, start, end)?;
    let changed = lisp.modify_props(obj, s, e, &mut |l, plist| {
        if props.iter().any(|&p| l.plist_member_eq(plist, p)) {
            Ok(Some(l.plist_without(plist, props)))
        } else {
            Ok(None)
        }
    })?;
    Ok(lisp.bool(changed))
}

fn f_text_property_any(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (start, end, prop, value, object) = (args[0], args[1], args[2], args[3], args[4]);
    let obj = lisp.decode_obj(object)?;
    let (s, e) = lisp.validate_range(obj, start, end)?;
    for seg in lisp.segments(obj, s, e) {
        if lisp.plist_get_eq(seg.plist, prop) == value {
            return Ok(Value::fixnum(seg.start as i64));
        }
    }
    Ok(Value::NIL)
}

fn f_text_property_not_all(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (start, end, prop, value, object) = (args[0], args[1], args[2], args[3], args[4]);
    let obj = lisp.decode_obj(object)?;
    let (s, e) = lisp.validate_range(obj, start, end)?;
    for seg in lisp.segments(obj, s, e) {
        if lisp.plist_get_eq(seg.plist, prop) != value {
            return Ok(Value::fixnum(seg.start as i64));
        }
    }
    Ok(Value::NIL)
}
