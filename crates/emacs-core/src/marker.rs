use crate::buffer::Marker;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::Vectorlike;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("make-marker", 0, 0, f_make_marker),
    Subr::fixed("marker-buffer", 1, 1, f_marker_buffer),
    Subr::fixed("marker-position", 1, 1, f_marker_position),
    Subr::fixed("marker-last-position", 1, 1, f_marker_position),
    Subr::fixed("set-marker", 2, 3, f_set_marker),
    Subr::fixed("copy-marker", 0, 2, f_copy_marker),
    Subr::fixed("marker-insertion-type", 1, 1, f_marker_insertion_type),
    Subr::fixed(
        "set-marker-insertion-type",
        2,
        2,
        f_set_marker_insertion_type,
    ),
    Subr::fixed("buffer-has-markers-at", 1, 1, f_buffer_has_markers_at),
];

impl Lisp {
    pub fn make_marker_object(&mut self) -> Value {
        self.heap.vectorlike(Vectorlike::Marker(Marker {
            buffer: Value::NIL,
            charpos: 0,
            bytepos: 0,
            insertion_type: false,
        }))
    }

    pub fn marker(&self, v: Value) -> &Marker {
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Marker(m) => m,
            _ => panic!("not a marker"),
        }
    }

    pub fn marker_mut(&mut self, v: Value) -> &mut Marker {
        match self.heap.vectorlike_mut(v) {
            Vectorlike::Marker(m) => m,
            _ => panic!("not a marker"),
        }
    }

    pub fn check_marker(&mut self, v: Value) -> LispResult<Value> {
        if self.is_marker(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::MARKERP, v))
        }
    }

    pub fn unchain_marker(&mut self, marker: Value) {
        let buffer = self.marker(marker).buffer;
        if self.is_buffer(buffer) {
            self.buffer_mut(buffer)
                .text
                .markers
                .retain(|&m| m != marker);
        }
        self.marker_mut(marker).buffer = Value::NIL;
    }

    pub fn set_marker_position(&mut self, marker: Value, pos: usize, buffer: Value) {
        let old = self.marker(marker).buffer;
        if old != buffer {
            if self.is_buffer(old) {
                self.buffer_mut(old).text.markers.retain(|&m| m != marker);
            }
            self.buffer_mut(buffer).text.markers.push(marker);
        }
        let byte = self.buffer(buffer).text.char_to_byte(pos);
        let m = self.marker_mut(marker);
        m.buffer = buffer;
        m.charpos = pos;
        m.bytepos = byte;
    }

    pub fn marker_position_value(&self, v: Value) -> Option<Value> {
        if !self.is_marker(v) {
            return None;
        }
        let m = self.marker(v);
        if m.buffer.is_nil() {
            return Some(Value::NIL);
        }
        Some(Value::fixnum(m.charpos as i64))
    }

    pub fn marker_position_checked(&mut self, marker: Value) -> LispResult<usize> {
        let m = *self.marker(marker);
        if m.buffer.is_nil() {
            return Err(self.error("Marker does not point anywhere"));
        }
        Ok(m.charpos)
    }

    pub fn marker_byte_position_checked(&mut self, marker: Value) -> LispResult<usize> {
        let m = *self.marker(marker);
        if m.buffer.is_nil() {
            return Err(self.error("Marker does not point anywhere"));
        }
        Ok(m.bytepos)
    }

    pub fn make_marker_at(&mut self, pos: usize, buffer: Value, insertion_type: bool) -> Value {
        let m = self.make_marker_object();
        self.set_marker_position(m, pos, buffer);
        self.marker_mut(m).insertion_type = insertion_type;
        m
    }

    pub fn set_marker(
        &mut self,
        marker: Value,
        position: Value,
        buffer: Value,
        restricted: bool,
    ) -> LispResult {
        let marker = self.check_marker(marker)?;
        if position.is_nil() {
            self.unchain_marker(marker);
            return Ok(marker);
        }
        let buffer = if buffer.is_nil() {
            if self.is_marker(position) && !self.marker(position).buffer.is_nil() {
                self.marker(position).buffer
            } else {
                self.current_buffer
            }
        } else {
            self.check_buffer(buffer)?
        };
        if !self.buffer_live_p(buffer) {
            self.unchain_marker(marker);
            return Ok(marker);
        }
        let pos = if self.is_marker(position) && self.marker(position).buffer == buffer {
            self.marker(position).charpos as i64
        } else {
            let p = self.coerce_marker(position);
            match p.as_fixnum() {
                Some(n) => n,
                None => {
                    if self.is_bignum(p) {
                        i64::MAX
                    } else {
                        return Err(self.wrong_type(sym::INTEGER_OR_MARKER_P, position));
                    }
                }
            }
        };
        let b = self.buffer(buffer);
        let (lo, hi) = if restricted {
            (b.begv, b.zv)
        } else {
            (1, b.z())
        };
        let pos = (pos.max(lo as i64) as usize).min(hi);
        self.set_marker_position(marker, pos, buffer);
        Ok(marker)
    }

    pub fn adjust_markers_for_insert(
        &mut self,
        buffer: Value,
        pos: usize,
        nchars: usize,
        before_markers: bool,
    ) {
        let markers = self.buffer(buffer).text.markers.clone();
        for m in markers {
            let (charpos, insertion_type) = {
                let mk = self.marker(m);
                (mk.charpos, mk.insertion_type)
            };
            if charpos > pos || (charpos == pos && (insertion_type || before_markers)) {
                self.marker_mut(m).charpos = charpos + nchars;
            }
        }
        self.refresh_marker_bytes(buffer);
    }

    pub fn adjust_markers_for_delete(&mut self, buffer: Value, from: usize, to: usize) {
        let markers = self.buffer(buffer).text.markers.clone();
        for m in markers {
            let charpos = self.marker(m).charpos;
            if charpos > to {
                self.marker_mut(m).charpos = charpos - (to - from);
            } else if charpos > from {
                self.marker_mut(m).charpos = from;
            }
        }
        self.refresh_marker_bytes(buffer);
    }

    fn refresh_marker_bytes(&mut self, buffer: Value) {
        let markers = self.buffer(buffer).text.markers.clone();
        for m in markers {
            let pos = self.marker(m).charpos;
            let byte = self.buffer(buffer).text.char_to_byte(pos);
            self.marker_mut(m).bytepos = byte;
        }
    }
}

fn f_make_marker(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.make_marker_object())
}

fn f_marker_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let m = lisp.check_marker(args[0])?;
    let b = lisp.marker(m).buffer;
    if lisp.buffer_live_p(b) {
        Ok(b)
    } else {
        Ok(Value::NIL)
    }
}

fn f_marker_position(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let m = lisp.check_marker(args[0])?;
    let mk = *lisp.marker(m);
    if mk.buffer.is_nil() {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(mk.charpos as i64))
}

fn f_set_marker(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.set_marker(args[0], args[1], args[2], false)
}

fn f_copy_marker(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (marker, kind) = (args[0], args[1]);
    let new = lisp.make_marker_object();
    lisp.marker_mut(new).insertion_type = !kind.is_nil();
    if marker.is_nil() {
        return Ok(new);
    }
    if !(lisp.is_marker(marker) || lisp.is_integer(marker)) {
        return Err(lisp.wrong_type(sym::INTEGER_OR_MARKER_P, marker));
    }
    lisp.set_marker(new, marker, Value::NIL, false)
}

fn f_marker_insertion_type(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let m = lisp.check_marker(args[0])?;
    Ok(lisp.bool(lisp.marker(m).insertion_type))
}

fn f_set_marker_insertion_type(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let m = lisp.check_marker(args[0])?;
    lisp.marker_mut(m).insertion_type = !args[1].is_nil();
    Ok(args[1])
}

fn f_buffer_has_markers_at(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = lisp.check_fixnum(args[0])? as usize;
    let buffer = lisp.current_buffer;
    let markers = lisp.buffer(buffer).text.markers.clone();
    Ok(lisp.bool(markers.iter().any(|&m| lisp.marker(m).charpos == pos)))
}
