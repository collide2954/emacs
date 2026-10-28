use std::cmp::Ordering;

use crate::buffer::Overlay;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::Vectorlike;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

struct SortvecItem {
    overlay: Value,
    beg: usize,
    end: usize,
    priority: i64,
    spriority: i64,
}

fn compare_overlays(s1: &SortvecItem, s2: &SortvecItem) -> Ordering {
    if s1.priority != s2.priority {
        if s1.priority < s2.priority {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    } else if s1.beg < s2.beg {
        if s1.end < s2.end && s1.spriority > s2.spriority {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    } else if s1.beg > s2.beg {
        if s1.end > s2.end && s1.spriority < s2.spriority {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    } else if s1.end != s2.end {
        if s2.end < s1.end {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    } else if s1.spriority != s2.spriority {
        if s1.spriority < s2.spriority {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    } else if s1.overlay == s2.overlay {
        Ordering::Equal
    } else if s1.overlay.bits() < s2.overlay.bits() {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("overlayp", 1, 1, f_overlayp),
    Subr::fixed("make-overlay", 2, 5, f_make_overlay),
    Subr::fixed("move-overlay", 3, 4, f_move_overlay),
    Subr::fixed("delete-overlay", 1, 1, f_delete_overlay),
    Subr::fixed("delete-all-overlays", 0, 1, f_delete_all_overlays),
    Subr::fixed("overlay-start", 1, 1, f_overlay_start),
    Subr::fixed("overlay-end", 1, 1, f_overlay_end),
    Subr::fixed("overlay-buffer", 1, 1, f_overlay_buffer),
    Subr::fixed("overlay-properties", 1, 1, f_overlay_properties),
    Subr::fixed("overlays-at", 1, 2, f_overlays_at),
    Subr::fixed("overlays-in", 2, 2, f_overlays_in),
    Subr::fixed("next-overlay-change", 1, 1, f_next_overlay_change),
    Subr::fixed("previous-overlay-change", 1, 1, f_previous_overlay_change),
    Subr::fixed("overlay-lists", 0, 0, f_overlay_lists),
    Subr::fixed("overlay-recenter", 1, 1, f_overlay_recenter),
    Subr::fixed("overlay-get", 2, 2, f_overlay_get),
    Subr::fixed("overlay-put", 3, 3, f_overlay_put),
];

impl Lisp {
    pub fn overlay(&self, v: Value) -> &Overlay {
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Overlay(o) => o,
            _ => panic!("not an overlay"),
        }
    }

    pub fn overlay_mut(&mut self, v: Value) -> &mut Overlay {
        match self.heap.vectorlike_mut(v) {
            Vectorlike::Overlay(o) => o,
            _ => panic!("not an overlay"),
        }
    }

    fn check_overlay(&mut self, v: Value) -> LispResult<Value> {
        if self.is_overlay(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::OVERLAYP, v))
        }
    }

    fn overlay_priority(&self, ov: Value) -> i64 {
        let p = self.plist_get_eq(self.overlay(ov).plist, sym::PRIORITY);
        if let Some(n) = p.as_fixnum() {
            return n;
        }
        if p.is_cons() {
            return self
                .xcdr(p)
                .as_fixnum()
                .or(self.xcar(p).as_fixnum())
                .unwrap_or(0);
        }
        0
    }

    pub fn overlay_get(&self, overlay: Value, prop: Value) -> Value {
        let plist = self.overlay(overlay).plist;
        let mut tail = plist;
        while tail.is_cons() {
            if self.xcar(tail) == prop {
                return self.car_safe(self.xcdr(tail));
            }
            tail = self.cdr_safe(self.xcdr(tail));
        }
        let category = self.plist_get_eq(plist, sym::CATEGORY);
        if category.is_symbol() && !category.is_nil() {
            return self.get(category, prop);
        }
        Value::NIL
    }

    pub fn overlays_in(
        &self,
        beg: usize,
        end: usize,
        empty: bool,
        trailing: bool,
    ) -> (Vec<Value>, usize) {
        let b = self.cur();
        let z = b.z();
        let mut next = z;
        let mut search_end = z;
        if end >= z && (empty || trailing) {
            search_end += 1;
        }
        let mut nodes: Vec<Value> = b
            .overlays
            .iter()
            .copied()
            .filter(|&o| {
                let ov = self.overlay(o);
                (beg < ov.end && ov.start < search_end) || (ov.start == ov.end && beg == ov.start)
            })
            .collect();
        nodes.sort_by_key(|&o| self.overlay(o).start);
        let mut vec = Vec::new();
        for o in nodes {
            let ov = self.overlay(o);
            if ov.start > end {
                next = next.min(ov.start);
                break;
            } else if ov.start == end {
                next = ov.start;
                if (!empty || end < z) && beg < end {
                    break;
                }
                if empty && ov.start != ov.end {
                    continue;
                }
            }
            if !empty && ov.start == ov.end {
                continue;
            }
            vec.push(o);
        }
        (vec, if next != 0 { next } else { z })
    }

    pub fn overlays_at(&self, pos: usize) -> (Vec<Value>, usize) {
        self.overlays_in(pos, pos + 1, false, true)
    }

    fn overlay_matches_window(&self, w: Value, ov: Value) -> bool {
        let window = self.overlay_get(ov, sym::WINDOW);
        !self.is_window(window) || window == w
    }

    fn make_sortvec_item(&self, overlay: Value) -> SortvecItem {
        let ov = self.overlay(overlay);
        let tem = self.overlay_get(overlay, sym::PRIORITY);
        let (priority, spriority) = if let Some(n) = tem.as_fixnum() {
            (n, 0)
        } else if tem.is_cons() {
            (
                self.xcar(tem).as_fixnum().unwrap_or(0),
                self.xcdr(tem).as_fixnum().unwrap_or(0),
            )
        } else {
            (0, 0)
        };
        SortvecItem {
            overlay,
            beg: ov.start,
            end: ov.end,
            priority,
            spriority,
        }
    }

    pub fn sort_overlays(&self, overlay_vec: &[Value], w: Option<Value>) -> Vec<Value> {
        let mut sortvec = Vec::with_capacity(overlay_vec.len());
        for &overlay in overlay_vec {
            if self.is_overlay(overlay) && !self.overlay(overlay).buffer.is_nil() {
                if let Some(w) = w
                    && !self.overlay_matches_window(w, overlay)
                {
                    continue;
                }
                sortvec.push(self.make_sortvec_item(overlay));
            }
        }
        for i in 1..sortvec.len() {
            let mut j = i;
            while j > 0 && compare_overlays(&sortvec[j - 1], &sortvec[j]) == Ordering::Greater {
                sortvec.swap(j - 1, j);
                j -= 1;
            }
        }
        sortvec.into_iter().map(|s| s.overlay).collect()
    }

    pub fn sorted_overlays_at(&self, buffer: Value, pos: usize) -> Vec<Value> {
        let mut list: Vec<Value> = self
            .buffer(buffer)
            .overlays
            .iter()
            .copied()
            .filter(|&o| {
                let ov = self.overlay(o);
                ov.start <= pos && pos < ov.end
            })
            .collect();
        list.sort_by(|&a, &b| {
            let pa = self.overlay_priority(a);
            let pb = self.overlay_priority(b);
            pb.cmp(&pa).then_with(|| {
                let (sa, sb) = (self.overlay(a).start, self.overlay(b).start);
                sb.cmp(&sa)
            })
        });
        list
    }

    pub fn overlay_property_at(
        &self,
        buffer: Value,
        pos: usize,
        prop: Value,
    ) -> Option<(Value, Value)> {
        for ov in self.sorted_overlays_at(buffer, pos) {
            let plist = self.overlay(ov).plist;
            let mut tail = plist;
            while tail.is_cons() {
                if self.xcar(tail) == prop {
                    let v = self.car_safe(self.xcdr(tail));
                    return Some((v, ov));
                }
                tail = self.cdr_safe(self.xcdr(tail));
            }
            let category = self.plist_get_eq(plist, sym::CATEGORY);
            if category.is_symbol() && !category.is_nil() {
                let v = self.get(category, prop);
                if !v.is_nil() {
                    return Some((v, ov));
                }
            }
        }
        None
    }

    pub fn overlay_property_at_pos_stickiness(
        &self,
        buffer: Value,
        pos: usize,
        prop: Value,
    ) -> Option<(Value, Value)> {
        for &o in &self.buffer(buffer).overlays {
            let ov = self.overlay(o);
            let covers = (ov.start < pos && pos < ov.end)
                || (ov.start == pos && !ov.front_advance && ov.start < ov.end)
                || (ov.end == pos && ov.rear_advance && ov.start < ov.end);
            if covers {
                let v = self.plist_get_eq(ov.plist, prop);
                if !v.is_nil() {
                    return Some((v, o));
                }
            }
        }
        None
    }

    pub fn next_overlay_change(&self, buffer: Value, pos: usize, forward: bool) -> usize {
        let b = self.buffer(buffer);
        let mut best = if forward { b.zv } else { b.begv };
        for &o in &b.overlays {
            let ov = self.overlay(o);
            for p in [ov.start, ov.end] {
                if forward && p > pos && p < best {
                    best = p;
                }
                if !forward && p < pos && p > best {
                    best = p;
                }
            }
        }
        best
    }

    pub fn adjust_overlays_for_insert(
        &mut self,
        buffer: Value,
        pos: usize,
        n: usize,
        before_markers: bool,
    ) {
        let overlays = self.buffer(buffer).overlays.clone();
        for o in overlays {
            let ov = self.overlay_mut(o);
            let empty = ov.start == ov.end;
            if ov.end > pos || (ov.end == pos && (ov.rear_advance || before_markers)) {
                ov.end += n;
            }
            if (ov.start > pos || (ov.start == pos && (ov.front_advance || before_markers)))
                && !(empty && ov.start == pos && !ov.rear_advance && !before_markers)
            {
                ov.start += n;
            }
            if ov.start > ov.end {
                ov.start = ov.end;
            }
        }
        self.buffer_mut(buffer).text.overlay_modiff += 1;
    }

    pub fn adjust_overlays_for_delete(&mut self, buffer: Value, from: usize, to: usize) {
        let n = to - from;
        let overlays = self.buffer(buffer).overlays.clone();
        let mut evaporated = Vec::new();
        for o in overlays {
            let adjust = |p: usize| {
                if p >= to {
                    p - n
                } else if p > from {
                    from
                } else {
                    p
                }
            };
            let ov = self.overlay_mut(o);
            ov.start = adjust(ov.start);
            ov.end = adjust(ov.end);
            if ov.start == ov.end {
                evaporated.push(o);
            }
        }
        for o in evaporated {
            if !self
                .plist_get_eq(self.overlay(o).plist, sym::EVAPORATE)
                .is_nil()
            {
                self.drop_overlay(o);
            }
        }
        self.buffer_mut(buffer).text.overlay_modiff += 1;
    }

    fn drop_overlay(&mut self, o: Value) {
        let b = self.overlay(o).buffer;
        if self.is_buffer(b) {
            self.buffer_mut(b).overlays.retain(|&x| x != o);
        }
        self.overlay_mut(o).buffer = Value::NIL;
    }

    pub fn overlays_in_range(&self, buffer: Value, beg: usize, end: usize) -> Vec<Value> {
        let b = self.buffer(buffer);
        let zv = b.zv;
        b.overlays
            .iter()
            .copied()
            .filter(|&o| {
                let ov = self.overlay(o);
                (ov.start < end && ov.end > beg)
                    || (ov.start == ov.end
                        && (ov.start == beg
                            || (beg < ov.start && ov.start < end)
                            || (ov.start == end && end == zv)))
            })
            .collect()
    }
}

fn f_overlayp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.is_overlay(args[0])))
}

fn f_make_overlay(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (beg, end, buffer, front_advance, rear_advance) =
        (args[0], args[1], args[2], args[3], args[4]);
    let buffer = lisp.decode_buffer(buffer)?;
    if !lisp.buffer_live_p(buffer) {
        return Err(lisp.error("Attempt to create an overlay in a dead buffer"));
    }
    let b = lisp.position_arg(beg)?;
    let e = lisp.position_arg(end)?;
    let (b, e) = if b <= e { (b, e) } else { (e, b) };
    let z = lisp.buffer(buffer).z() as i64;
    let b = b.clamp(1, z) as usize;
    let e = e.clamp(1, z) as usize;
    let ov = lisp.heap.vectorlike(Vectorlike::Overlay(Box::new(Overlay {
        buffer,
        start: b,
        end: e,
        plist: Value::NIL,
        front_advance: !front_advance.is_nil(),
        rear_advance: !rear_advance.is_nil(),
    })));
    lisp.buffer_mut(buffer).overlays.push(ov);
    Ok(ov)
}

fn f_move_overlay(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (overlay, beg, end, buffer) = (args[0], args[1], args[2], args[3]);
    let overlay = lisp.check_overlay(overlay)?;
    let buffer = if buffer.is_nil() {
        let b = lisp.overlay(overlay).buffer;
        if b.is_nil() { lisp.current_buffer } else { b }
    } else {
        lisp.check_buffer(buffer)?
    };
    if !lisp.buffer_live_p(buffer) {
        return Err(lisp.error("Attempt to move overlay to a dead buffer"));
    }
    let b = lisp.position_arg(beg)?;
    let e = lisp.position_arg(end)?;
    let (b, e) = if b <= e { (b, e) } else { (e, b) };
    let z = lisp.buffer(buffer).z() as i64;
    let b = b.clamp(1, z) as usize;
    let e = e.clamp(1, z) as usize;
    let old = lisp.overlay(overlay).buffer;
    if old != buffer {
        if lisp.is_buffer(old) {
            lisp.buffer_mut(old).overlays.retain(|&x| x != overlay);
        }
        lisp.buffer_mut(buffer).overlays.push(overlay);
    }
    let ov = lisp.overlay_mut(overlay);
    ov.buffer = buffer;
    ov.start = b;
    ov.end = e;
    if b == e
        && !lisp
            .plist_get_eq(lisp.overlay(overlay).plist, sym::EVAPORATE)
            .is_nil()
    {
        lisp.drop_overlay(overlay);
    }
    Ok(overlay)
}

fn f_delete_overlay(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let overlay = lisp.check_overlay(args[0])?;
    lisp.drop_overlay(overlay);
    Ok(Value::NIL)
}

pub(crate) fn f_delete_all_overlays(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = lisp.decode_buffer(args[0])?;
    let overlays = std::mem::take(&mut lisp.buffer_mut(buffer).overlays);
    for o in overlays {
        lisp.overlay_mut(o).buffer = Value::NIL;
    }
    Ok(Value::NIL)
}

fn f_overlay_start(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let o = lisp.check_overlay(args[0])?;
    let ov = lisp.overlay(o);
    if ov.buffer.is_nil() {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(ov.start as i64))
}

fn f_overlay_end(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let o = lisp.check_overlay(args[0])?;
    let ov = lisp.overlay(o);
    if ov.buffer.is_nil() {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(ov.end as i64))
}

fn f_overlay_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let o = lisp.check_overlay(args[0])?;
    Ok(lisp.overlay(o).buffer)
}

fn f_overlay_properties(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let o = lisp.check_overlay(args[0])?;
    let plist = lisp.overlay(o).plist;
    let items = lisp.list_to_vec(plist)?;
    Ok(lisp.list(&items))
}

fn f_overlays_at(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = lisp.position_arg(args[0])? as usize;
    let buffer = lisp.current_buffer;
    let list = if args[1].is_nil() {
        lisp.buffer(buffer)
            .overlays
            .iter()
            .copied()
            .filter(|&o| {
                let ov = lisp.overlay(o);
                ov.start <= pos && pos < ov.end
            })
            .collect::<Vec<_>>()
    } else {
        let mut l = lisp.sorted_overlays_at(buffer, pos);
        l.reverse();
        l.reverse();
        l
    };
    Ok(lisp.list(&list))
}

fn f_overlays_in(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.position_arg(args[0])? as usize;
    let e = lisp.position_arg(args[1])? as usize;
    let (b, e) = if b <= e { (b, e) } else { (e, b) };
    let buffer = lisp.current_buffer;
    let list = lisp.overlays_in_range(buffer, b, e);
    Ok(lisp.list(&list))
}

fn f_next_overlay_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = lisp.position_arg(args[0])? as usize;
    let buffer = lisp.current_buffer;
    Ok(Value::fixnum(
        lisp.next_overlay_change(buffer, pos, true) as i64
    ))
}

fn f_previous_overlay_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = lisp.position_arg(args[0])? as usize;
    let buffer = lisp.current_buffer;
    Ok(Value::fixnum(
        lisp.next_overlay_change(buffer, pos, false) as i64
    ))
}

fn f_overlay_lists(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let overlays = lisp.cur().overlays.clone();
    let list = lisp.list(&overlays);
    Ok(lisp.cons(list, Value::NIL))
}

fn f_overlay_recenter(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_overlay_get(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let o = lisp.check_overlay(args[0])?;
    Ok(lisp.overlay_get(o, args[1]))
}

fn f_overlay_put(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (o, prop, value) = (args[0], args[1], args[2]);
    let o = lisp.check_overlay(o)?;
    let plist = lisp.overlay(o).plist;
    let new = lisp.plist_put_eq(plist, prop, value)?;
    lisp.overlay_mut(o).plist = new;
    if prop == sym::EVAPORATE && !value.is_nil() {
        let ov = lisp.overlay(o);
        if ov.start == ov.end && !ov.buffer.is_nil() {
            lisp.drop_overlay(o);
        }
    }
    let buffer = lisp.overlay(o).buffer;
    if lisp.is_buffer(buffer) {
        lisp.buffer_mut(buffer).text.overlay_modiff += 1;
    }
    Ok(value)
}
