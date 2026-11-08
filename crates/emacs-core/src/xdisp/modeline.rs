use std::rc::Rc;

use crate::dispextern::{It, TEXT_AREA};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::xfaces::{
    DEFAULT_FACE_ID, HEADER_LINE_ACTIVE_FACE_ID, HEADER_LINE_INACTIVE_FACE_ID,
    MODE_LINE_ACTIVE_FACE_ID, MODE_LINE_INACTIVE_FACE_ID, TAB_BAR_FACE_ID, TAB_LINE_ACTIVE_FACE_ID,
    TAB_LINE_INACTIVE_FACE_ID, TOOL_BAR_FACE_ID,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ModeLineTarget {
    #[default]
    Display,
    Title,
    Noprop,
    String,
}

impl ModeLineTarget {
    fn code(self) -> i64 {
        match self {
            ModeLineTarget::Display => 0,
            ModeLineTarget::Title => 1,
            ModeLineTarget::Noprop => 2,
            ModeLineTarget::String => 3,
        }
    }

    fn from_code(n: i64) -> ModeLineTarget {
        match n {
            1 => ModeLineTarget::Title,
            2 => ModeLineTarget::Noprop,
            3 => ModeLineTarget::String,
            _ => ModeLineTarget::Display,
        }
    }
}

const LOTS_OF_DASHES: usize = 140;

fn pint2str(width: i32, d: i64) -> Vec<u8> {
    let digits = d.max(0).to_string().into_bytes();
    let mut buf = vec![b' '; (width.max(0) as usize).saturating_sub(digits.len())];
    buf.extend_from_slice(&digits);
    buf
}

const POWER_LETTER: [u8; 11] = [
    0, b'k', b'M', b'G', b'T', b'P', b'E', b'Z', b'Y', b'R', b'Q',
];

fn pint2hrstr(width: i32, d: i64) -> Vec<u8> {
    let mut quotient = d;
    let mut remainder;
    let mut tenths: i64 = -1;
    let mut exponent = 0usize;
    if quotient >= 1000 {
        loop {
            remainder = quotient % 1000;
            quotient /= 1000;
            exponent += 1;
            if quotient < 1000 {
                break;
            }
        }
        if quotient <= 9 {
            tenths = remainder / 100;
            if remainder % 100 >= 50 {
                if tenths < 9 {
                    tenths += 1;
                } else {
                    quotient += 1;
                    tenths = if quotient == 10 { -1 } else { 0 };
                }
            }
        } else if remainder >= 500 {
            if quotient < 999 {
                quotient += 1;
            } else {
                quotient = 1;
                exponent += 1;
                tenths = 0;
            }
        }
    }
    let length = if tenths == -1 && quotient <= 99 {
        if quotient <= 9 { 1 } else { 2 }
    } else {
        3
    };
    let size = (width.max(length) as usize).max(length as usize);
    let mut body = Vec::new();
    let mut q = quotient;
    loop {
        body.push(b'0' + (q % 10) as u8);
        q /= 10;
        if q == 0 {
            break;
        }
    }
    body.reverse();
    if tenths >= 0 {
        body.push(b'.');
        body.push(b'0' + tenths as u8);
    }
    let mut buf = vec![b' '; size.saturating_sub(body.len())];
    buf.extend_from_slice(&body);
    if POWER_LETTER[exponent] != 0 {
        buf.push(POWER_LETTER[exponent]);
    }
    buf
}

fn percent99(n: i64, d: i64) -> i32 {
    let percent = ((d - 1) as f64 + 100.0 * n as f64) / d as f64;
    (percent as i32).min(99)
}

impl Lisp {
    pub fn c_string_width(&self, s: &[u8], precision: i32) -> (i64, usize, usize) {
        let dp = self.buffer_display_table();
        let mut i = 0usize;
        let mut i_byte = 0usize;
        let mut width: i64 = 0;
        while i_byte < s.len() {
            let (c, bytes) = crate::character::string_char(&s[i_byte..]);
            let val = dp.map(|dp| self.disp_char_vector(dp, c));
            let thiswidth = match val {
                Some(val) if self.is_vector(val) => {
                    crate::indent::sanitize_char_width(self.vector_items(val).len() as i64)
                }
                _ => self.char_width(c, dp) as i64,
            };
            if 0 < precision && i64::from(precision) - width < thiswidth {
                return (width, i, i_byte);
            }
            width += thiswidth;
            i += 1;
            i_byte += bytes;
        }
        (width, i, i_byte)
    }

    fn format_mode_line_unwind_data(
        &mut self,
        target_frame: Option<Value>,
        obuf: Option<Value>,
        owin: Value,
        save_proptrans: bool,
    ) -> Value {
        let mut vector = self.xdisp.mode_line_unwind_vector;
        self.xdisp.mode_line_unwind_vector = Value::NIL;
        if vector.is_nil() {
            vector = self.vector(vec![Value::NIL; 12]);
        }
        let x = &self.xdisp;
        let (target, len, list, proptrans, face, face_prop) = (
            x.mode_line_target.code(),
            x.mode_line_noprop_buf.len() as i64,
            x.mode_line_string_list,
            if save_proptrans {
                x.mode_line_proptrans_alist
            } else {
                sym::T
            },
            x.mode_line_string_face,
            x.mode_line_string_face_prop,
        );
        self.vector_set(vector, 0, Value::fixnum(target));
        self.vector_set(vector, 1, Value::fixnum(len));
        self.vector_set(vector, 2, list);
        self.vector_set(vector, 3, proptrans);
        self.vector_set(vector, 4, face);
        self.vector_set(vector, 5, face_prop);
        self.vector_set(vector, 6, obuf.unwrap_or(Value::NIL));
        self.vector_set(vector, 7, owin);
        if let Some(tf) = target_frame {
            let sw = self.frame(tf).selected_window;
            let buffer = self.window(sw).contents;
            self.vector_set(vector, 8, sw);
            self.vector_set(vector, 10, buffer);
            let (pt, _) = {
                let b = self.buffer(buffer);
                (b.pt, b.pt_byte)
            };
            let marker = self.make_marker_at(pt, buffer, false);
            self.vector_set(vector, 11, marker);
        }
        vector
    }

    #[allow(clippy::too_many_arguments)]
    fn display_string(
        &mut self,
        string: Option<Rc<[u8]>>,
        lisp_string: Value,
        face_string: Value,
        face_string_pos: i64,
        start: i64,
        it: &mut It,
        field_width: i32,
        precision: i32,
        max_x: i32,
        multibyte: i32,
    ) -> LispResult<i32> {
        let hpos_at_start = it.hpos;
        let saved_face_id = it.face_id;
        let row = it.glyph_row.expect("display_string needs a glyph row");
        let has_string = string.is_some();
        let s = if lisp_string.is_nil() { string } else { None };
        self.reseat_to_string(
            it,
            s,
            lisp_string,
            start,
            i64::from(precision),
            i64::from(field_width),
            multibyte,
        )?;
        if has_string && lisp_string.is_string() {
            it.stop_charpos = it.end_charpos;
        }
        if face_string.is_string() {
            it.face_id = self
                .face_at_string_position(
                    it.window,
                    face_string,
                    face_string_pos,
                    0,
                    it.base_face_id as usize,
                    false,
                    0,
                )?
                .0 as i32;
            let f = it.f;
            it.face_box_p = self
                .face_from_id(f, it.face_id as usize)
                .expect("realized face")
                .box_type
                != crate::xfaces::FaceBoxType::NoBox;
            if lisp_string.is_nil() {
                let display = crate::textprop::f_get_text_property(
                    self,
                    &[Value::fixnum(0), sym::DISPLAY, face_string],
                )?;
                if !display.is_nil() {
                    let min_width = self.plist_get(display, sym::MIN_WIDTH);
                    if !min_width.is_nil() {
                        self.display_min_width(it, 0, face_string, min_width)?;
                    }
                }
            }
        }
        let max_x = if max_x <= 0 {
            it.last_visible_x
        } else {
            max_x.min(it.last_visible_x)
        };
        if it.current_x < it.first_visible_x {
            let first = it.first_visible_x;
            self.move_it_in_display_line_to(
                it,
                100000,
                first,
                crate::dispextern::MOVE_TO_POS | crate::dispextern::MOVE_TO_X,
            )?;
        }
        {
            let r = self.glyph_row_mut(row);
            r.ascent = it.max_ascent;
            r.height = it.max_ascent + it.max_descent;
            r.phys_ascent = it.max_phys_ascent;
            r.phys_height = it.max_phys_ascent + it.max_phys_descent;
            r.extra_line_spacing = it.max_extra_line_spacing;
        }
        let mut it_charpos = if it.string.is_string() {
            it.string_charpos()
        } else {
            it.charpos()
        };
        while it.current_x < max_x {
            if !self.get_next_display_element(it)? {
                break;
            }
            let x_before = it.current_x;
            let n_glyphs_before = self.glyph_row(row).used[TEXT_AREA] as usize;
            self.produce_glyphs_macro(it)?;
            let nglyphs = self.glyph_row(row).used[TEXT_AREA] as usize - n_glyphs_before;
            let mut i = 0;
            let mut x = x_before;
            while i < nglyphs {
                let glyph = self.glyph_row(row).area_glyphs(TEXT_AREA)[n_glyphs_before + i];
                if it.line_wrap != crate::dispextern::LineWrap::Truncate
                    && x + i32::from(glyph.pixel_width) > max_x
                {
                    let reversed_p = self.glyph_row(row).reversed_p;
                    if glyph.padding_p {
                        if reversed_p {
                            let used = self.glyph_row(row).used[TEXT_AREA] as usize;
                            self.unproduce_glyphs(it, (used - n_glyphs_before) as i32);
                        }
                        self.glyph_row_mut(row).used[TEXT_AREA] = n_glyphs_before as i16;
                        it.current_x = x_before;
                    } else {
                        if reversed_p {
                            let used = self.glyph_row(row).used[TEXT_AREA] as usize;
                            self.unproduce_glyphs(it, (used - (n_glyphs_before + i)) as i32);
                        }
                        self.glyph_row_mut(row).used[TEXT_AREA] = (n_glyphs_before + i) as i16;
                        it.current_x = x;
                    }
                    break;
                } else if x + i32::from(glyph.pixel_width) >= it.first_visible_x {
                    it.hpos += 1;
                    if x < it.first_visible_x {
                        self.glyph_row_mut(row).x = x - it.first_visible_x;
                    }
                } else {
                    unreachable!("emacs_abort: glyph off the left margin in display_string");
                }
                let r = self.glyph_row_mut(row);
                r.ascent = r.ascent.max(it.max_ascent);
                r.height = r.height.max(it.max_ascent + it.max_descent);
                r.phys_ascent = r.phys_ascent.max(it.max_phys_ascent);
                r.phys_height = r.phys_height.max(it.max_phys_ascent + it.max_phys_descent);
                r.extra_line_spacing = r.extra_line_spacing.max(it.max_extra_line_spacing);
                x += i32::from(glyph.pixel_width);
                i += 1;
            }
            if i < nglyphs {
                break;
            }
            if it.at_end_of_line_p() {
                it.continuation_lines_width = 0;
                break;
            }
            self.set_iterator_to_next(it, true)?;
            it_charpos = if it.string.is_string() {
                it.string_charpos()
            } else {
                it.charpos()
            };
            if it.line_wrap == crate::dispextern::LineWrap::Truncate
                && it.current_x >= it.last_visible_x
            {
                if it_charpos <= it.string_nchars {
                    if !self.frame_window_p(it.f) {
                        unreachable!("tty frames have no glyph matrices");
                    }
                    self.glyph_row_mut(row).truncated_on_right_p = true;
                }
                break;
            }
        }
        if it.first_visible_x != 0 && it_charpos > 0 {
            let reversed_p = self.glyph_row(row).reversed_p;
            let w = it.window;
            let fringe = if reversed_p {
                self.window_right_fringe_width(w)
            } else {
                self.window_left_fringe_width(w)
            };
            if !self.frame_window_p(it.f) || fringe == 0 {
                self.insert_left_trunc_glyphs(it)?;
            }
            self.glyph_row_mut(row).truncated_on_left_p = true;
        }
        it.face_id = saved_face_id;
        Ok(it.hpos - hpos_at_start)
    }

    fn store_mode_line_noprop(&mut self, s: &[u8], field_width: i32, precision: i32) -> i32 {
        let (width, _, nbytes) = self.c_string_width(s, precision);
        let mut n = width as i32;
        self.xdisp
            .mode_line_noprop_buf
            .extend_from_slice(&s[..nbytes]);
        while field_width > 0 && n < field_width {
            self.xdisp.mode_line_noprop_buf.push(b' ');
            n += 1;
        }
        n
    }

    fn store_mode_line_string(
        &mut self,
        string: Option<&[u8]>,
        lisp_string: Value,
        copy_string: bool,
        field_width: i32,
        precision: i32,
        props: Value,
    ) -> LispResult<i32> {
        let mut n = 0;
        let mut props = props;
        let mut lisp_string = lisp_string;
        let len;
        let base = self.stack.len();
        if let Some(string) = string {
            let limit = if precision <= 0 {
                string.len()
            } else {
                (precision as usize).min(string.len())
            };
            let bytes = &string[..string[..limit]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(limit)];
            len = bytes.len() as i64;
            lisp_string = self.make_string(bytes);
            self.stack.push(lisp_string);
            let face = self.xdisp.mode_line_string_face;
            if props.is_nil() {
                props = self.xdisp.mode_line_string_face_prop;
            } else if !face.is_nil() {
                let mut pface = self.plist_get(props, sym::FACE);
                props = crate::fns::f_copy_sequence(self, &[props])?;
                self.stack.push(props);
                if pface.is_nil() {
                    pface = face;
                } else {
                    pface = self.list(&[pface, face]);
                }
                props = self.plist_put_eq(props, sym::FACE, pface)?;
            }
            self.stack.push(props);
            crate::textprop::f_add_text_properties(
                self,
                &[Value::fixnum(0), Value::fixnum(len), props, lisp_string],
            )?;
        } else {
            let mut l = self.string_chars(lisp_string) as i64;
            let mut precision = precision;
            if precision > 0 && l > i64::from(precision) {
                l = i64::from(precision);
                lisp_string = crate::fns::f_substring(
                    self,
                    &[lisp_string, Value::fixnum(0), Value::fixnum(l)],
                )?;
                precision = -1;
            }
            let _ = precision;
            self.stack.push(lisp_string);
            len = l;
            let face_base = self.xdisp.mode_line_string_face;
            if !face_base.is_nil() {
                let tp =
                    crate::textprop::f_text_properties_at(self, &[Value::fixnum(0), lisp_string])?;
                let string_face = self.plist_get(tp, sym::FACE);
                let mut face = string_face;
                if !props.is_nil() {
                    let propface = self.plist_get(props, sym::FACE);
                    if !propface.is_nil() {
                        face = propface;
                    }
                }
                face = if face.is_nil() {
                    face_base
                } else {
                    self.list(&[face, face_base])
                };
                self.stack.push(face);
                props = crate::fns::f_copy_sequence(self, &[props])?;
                self.stack.push(props);
                props = self.plist_put_eq(props, sym::FACE, face)?;
                self.stack.push(props);
                if copy_string {
                    lisp_string = crate::fns::f_copy_sequence(self, &[lisp_string])?;
                    self.stack.push(lisp_string);
                }
            }
            if !props.is_nil() {
                crate::textprop::f_add_text_properties(
                    self,
                    &[Value::fixnum(0), Value::fixnum(len), props, lisp_string],
                )?;
            }
        }
        if len > 0 {
            self.xdisp.mode_line_string_list =
                self.cons(lisp_string, self.xdisp.mode_line_string_list);
            n += len as i32;
        }
        let mut field_width = i64::from(field_width);
        if field_width > len {
            field_width -= len;
            let spaces = crate::alloc::f_make_string(
                self,
                &[
                    Value::fixnum(field_width),
                    Value::fixnum(i64::from(b' ')),
                    Value::NIL,
                ],
            )?;
            self.stack.push(spaces);
            if !props.is_nil() {
                crate::textprop::f_add_text_properties(
                    self,
                    &[Value::fixnum(0), Value::fixnum(field_width), props, spaces],
                )?;
            }
            self.xdisp.mode_line_string_list = self.cons(spaces, self.xdisp.mode_line_string_list);
            n += field_width as i32;
        }
        self.stack.truncate(base);
        Ok(n)
    }

    fn move_elt_to_front(&mut self, elt: Value, list: Value) -> LispResult<Value> {
        let mut list = list;
        let mut tail = list;
        let mut prev = Value::NIL;
        while tail.is_cons() {
            let tem = self.xcar(tail);
            if elt == tem {
                if prev.is_nil() {
                    list = self.xcdr(tail);
                } else {
                    let rest = self.xcdr(tail);
                    self.heap.set_cdr(prev, rest);
                }
                self.heap.set_cdr(tail, list);
                return Ok(tail);
            }
            prev = tail;
            tail = self.xcdr(tail);
            self.maybe_quit()?;
        }
        Ok(list)
    }

    fn mode_line_elt_no_props(&mut self) -> Value {
        let n = Value::fixnum(self.xdisp.mode_line_elt_no);
        self.list(&[sym::MODE_LINE_ELT_NO, n])
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn display_mode_element(
        &mut self,
        it: &mut It,
        depth: i32,
        field_width: i32,
        precision: i32,
        elt: Value,
        props: Value,
        risky: bool,
    ) -> LispResult<i32> {
        let base = self.stack.len();
        let r = self.display_mode_element_1(it, depth, field_width, precision, elt, props, risky);
        self.stack.truncate(base);
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn display_mode_element_1(
        &mut self,
        it: &mut It,
        depth: i32,
        field_width: i32,
        precision: i32,
        elt: Value,
        props: Value,
        risky: bool,
    ) -> LispResult<i32> {
        let mut n = 0;
        let mut literal = false;
        let mut depth = depth;
        let mut field_width = field_width;
        let mut precision = precision;
        let mut elt = elt;
        let mut props = props;
        let mut risky = risky;
        self.stack.push(elt);
        self.stack.push(props);
        'tail_recurse: loop {
            if depth > 100 {
                elt = self.string("*too-deep*");
                self.stack.push(elt);
            }
            if self.xdisp.mode_line_target == ModeLineTarget::String {
                self.xdisp.mode_line_elt_no += 1;
            }
            depth += 1;
            if elt.is_string() {
                if self.string_chars(elt) > 0 && (!props.is_nil() || risky) {
                    let mut oprops =
                        crate::textprop::f_text_properties_at(self, &[Value::fixnum(0), elt])?;
                    self.stack.push(oprops);
                    let differ = crate::fns::f_equal(self, &[props, oprops])?.is_nil();
                    if differ || risky {
                        if !oprops.is_nil() && !risky {
                            oprops = crate::fns::f_copy_sequence(self, &[oprops])?;
                            self.stack.push(oprops);
                            let mut tem = props;
                            while tem.is_cons() && self.xcdr(tem).is_cons() {
                                let k = self.xcar(tem);
                                let v = self.xcar(self.xcdr(tem));
                                oprops = self.plist_put_eq(oprops, k, v)?;
                                self.stack.push(oprops);
                                tem = self.xcdr(self.xcdr(tem));
                            }
                            props = oprops;
                        }
                        let alist = self.xdisp.mode_line_proptrans_alist;
                        let aelt = crate::fns::f_assoc(self, &[elt, alist, Value::NIL])?;
                        self.stack.push(aelt);
                        let same = !aelt.is_nil() && {
                            let cdr = self.xcdr(aelt);
                            !crate::fns::f_equal(self, &[props, cdr])?.is_nil()
                        };
                        if same {
                            elt = self.xcar(aelt);
                            self.stack.push(elt);
                            let alist = self.xdisp.mode_line_proptrans_alist;
                            self.xdisp.mode_line_proptrans_alist =
                                self.move_elt_to_front(aelt, alist)?;
                        } else {
                            if !aelt.is_nil() {
                                let alist = self.xdisp.mode_line_proptrans_alist;
                                self.xdisp.mode_line_proptrans_alist =
                                    crate::fns::f_delq(self, &[aelt, alist])?;
                            }
                            elt = crate::fns::f_copy_sequence(self, &[elt])?;
                            self.stack.push(elt);
                            let length = crate::fns::f_length(self, &[elt])?;
                            let args = [Value::fixnum(0), length, props, elt];
                            self.condition_case_internal(
                                sym::T,
                                |l| crate::textprop::f_set_text_properties(l, &args),
                                |l, error| l.safe_eval_handler(error, &args),
                            )?;
                            let pair = self.cons(elt, props);
                            self.xdisp.mode_line_proptrans_alist =
                                self.cons(pair, self.xdisp.mode_line_proptrans_alist);
                            let tem = crate::fns::f_nthcdr(
                                self,
                                &[Value::fixnum(50), self.xdisp.mode_line_proptrans_alist],
                            )?;
                            if !tem.is_nil() {
                                self.heap.set_cdr(tem, Value::NIL);
                            }
                        }
                    }
                }
                if literal {
                    let prec = precision - n;
                    match self.xdisp.mode_line_target {
                        ModeLineTarget::Noprop | ModeLineTarget::Title => {
                            let bytes = self.string_bytes(elt).to_vec();
                            let bytes =
                                &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())];
                            n += self.store_mode_line_noprop(bytes, -1, prec);
                        }
                        ModeLineTarget::String => {
                            let src = self.mode_line_elt_no_props();
                            self.stack.push(src);
                            n += self.store_mode_line_string(None, elt, true, 0, prec, src)?;
                        }
                        ModeLineTarget::Display => {
                            let mb = i32::from(self.string_multibyte(elt));
                            n += self.display_string(
                                None,
                                elt,
                                Value::NIL,
                                0,
                                0,
                                it,
                                0,
                                prec,
                                0,
                                mb,
                            )?;
                        }
                    }
                    break 'tail_recurse;
                }
                let mut offset = 0usize;
                loop {
                    let bytes = self.string_bytes(elt);
                    if !((precision <= 0 || n < precision)
                        && bytes.get(offset).copied().unwrap_or(0) != 0
                        && (self.xdisp.mode_line_target != ModeLineTarget::Display
                            || it.current_x < it.last_visible_x))
                    {
                        break;
                    }
                    let last_offset = offset;
                    let mut c;
                    loop {
                        c = self.string_bytes(elt).get(offset).copied().unwrap_or(0);
                        offset += 1;
                        if c == 0 || c == b'%' {
                            break;
                        }
                    }
                    if offset - 1 != last_offset {
                        offset -= 1;
                        let seg = self.string_bytes(elt)[last_offset..offset].to_vec();
                        let (prec, nchars, _) = self.c_string_width(&seg, precision - n);
                        let prec = prec as i32;
                        match self.xdisp.mode_line_target {
                            ModeLineTarget::Noprop | ModeLineTarget::Title => {
                                let tail = self.string_bytes(elt)[last_offset..].to_vec();
                                let tail = &tail
                                    [..tail.iter().position(|&b| b == 0).unwrap_or(tail.len())];
                                n += self.store_mode_line_noprop(tail, 0, prec);
                            }
                            ModeLineTarget::String => {
                                let charpos =
                                    self.string_byte_to_char_index(elt, last_offset) as i64;
                                let endpos = if precision <= 0 {
                                    self.string_byte_to_char_index(elt, offset) as i64
                                } else {
                                    charpos + nchars as i64
                                };
                                let mode_string = crate::fns::f_substring(
                                    self,
                                    &[elt, Value::fixnum(charpos), Value::fixnum(endpos)],
                                )?;
                                self.stack.push(mode_string);
                                let src = self.mode_line_elt_no_props();
                                self.stack.push(src);
                                n += self.store_mode_line_string(
                                    None,
                                    mode_string,
                                    false,
                                    0,
                                    0,
                                    src,
                                )?;
                            }
                            ModeLineTarget::Display => {
                                let charpos =
                                    self.string_byte_to_char_index(elt, last_offset) as i64;
                                let nchars = if precision <= 0 {
                                    self.string_byte_to_char_index(elt, offset) as i64 - charpos
                                } else {
                                    nchars as i64
                                };
                                let mb = i32::from(self.string_multibyte(elt));
                                n += self.display_string(
                                    None,
                                    elt,
                                    Value::NIL,
                                    0,
                                    charpos,
                                    it,
                                    0,
                                    nchars as i32,
                                    0,
                                    mb,
                                )?;
                            }
                        }
                    } else {
                        let percent_position = offset;
                        let mut field = 0;
                        let mut c;
                        loop {
                            c = u32::from(self.string_bytes(elt).get(offset).copied().unwrap_or(0));
                            offset += 1;
                            if !(u32::from(b'0')..=u32::from(b'9')).contains(&c) {
                                break;
                            }
                            field = field * 10 + (c - u32::from(b'0')) as i32;
                        }
                        if self.string_multibyte(elt) {
                            offset -= 1;
                            let bytes = self.string_bytes(elt);
                            let (ch, length) = crate::character::string_char(&bytes[offset..]);
                            c = ch;
                            offset += length;
                        }
                        if field_width - n > 0 && field > field_width - n {
                            field = field_width - n;
                        }
                        let prec = precision - n;
                        if c == u32::from(b'M') {
                            let gms = self.find_symbol_value(sym::GLOBAL_MODE_STRING);
                            n += self
                                .display_mode_element(it, depth, field, prec, gms, props, risky)?;
                        } else if c != 0 {
                            let bytepos = percent_position;
                            let charpos = if self.string_multibyte(elt) {
                                self.string_byte_to_char_index(elt, bytepos) as i64
                            } else {
                                bytepos as i64
                            };
                            let (spec, string) = self.decode_mode_spec(it.window, c, field)?;
                            self.stack.push(string);
                            let mut multibyte = !string.is_nil() && self.string_multibyte(string);
                            let nbytes = spec.len();
                            let (nchars, mb_nbytes) =
                                crate::character::parse_str_as_multibyte(&spec);
                            if !(nbytes == nchars || nbytes != mb_nbytes) {
                                multibyte = true;
                            }
                            match self.xdisp.mode_line_target {
                                ModeLineTarget::Noprop | ModeLineTarget::Title => {
                                    n += self.store_mode_line_noprop(&spec, field, prec);
                                }
                                ModeLineTarget::String => {
                                    let tem = self.make_string(&spec);
                                    self.stack.push(tem);
                                    let p = crate::textprop::f_text_properties_at(
                                        self,
                                        &[Value::fixnum(charpos), elt],
                                    )?;
                                    self.stack.push(p);
                                    let no = Value::fixnum(self.xdisp.mode_line_elt_no);
                                    props = self.plist_put_eq(p, sym::MODE_LINE_ELT_NO, no)?;
                                    self.stack.push(props);
                                    n += self.store_mode_line_string(
                                        None, tem, false, field, prec, props,
                                    )?;
                                }
                                ModeLineTarget::Display => {
                                    let row = it.glyph_row.expect("glyph row");
                                    let nglyphs_before =
                                        self.glyph_row(row).used[TEXT_AREA] as usize;
                                    let s: Rc<[u8]> = spec.into();
                                    let nwritten = self.display_string(
                                        Some(s),
                                        string,
                                        elt,
                                        charpos,
                                        0,
                                        it,
                                        field,
                                        prec,
                                        0,
                                        i32::from(multibyte),
                                    )?;
                                    if nwritten > 0 {
                                        let r = self.glyph_row_mut(row);
                                        let start = r.area[TEXT_AREA] + nglyphs_before;
                                        for g in &mut r.glyphs[start..start + nwritten as usize] {
                                            g.object = elt;
                                            g.charpos = charpos;
                                        }
                                        n += nwritten;
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
                break 'tail_recurse;
            } else if elt.is_symbol() {
                if self.get(elt, sym::RISKY_LOCAL_VARIABLE).is_nil() {
                    risky = true;
                }
                let tem = crate::data::f_boundp(self, &[elt])?;
                if !tem.is_nil() {
                    let tem = crate::data::f_symbol_value(self, &[elt])?;
                    if tem.is_string() {
                        literal = true;
                    }
                    if tem != elt {
                        elt = tem;
                        self.stack.push(elt);
                        continue 'tail_recurse;
                    }
                }
                break 'tail_recurse;
            } else if elt.is_cons() {
                let car = self.xcar(elt);
                if car == sym::K_EVAL {
                    if risky {
                        break 'tail_recurse;
                    }
                    let rest = self.xcdr(elt);
                    if rest.is_cons() {
                        let spec = self.dsafe_eval(self.xcar(rest))?;
                        self.stack.push(spec);
                        if !self.frame_live_p(it.f) {
                            return Err(
                                self.signal_error(":eval deleted the frame being displayed", elt)
                            );
                        }
                        n += self.display_mode_element(
                            it,
                            depth,
                            field_width - n,
                            precision - n,
                            spec,
                            props,
                            risky,
                        )?;
                    }
                } else if car == sym::K_PROPERTIZE {
                    if risky {
                        break 'tail_recurse;
                    }
                    let rest = self.xcdr(elt);
                    if rest.is_cons() {
                        let inner = self.xcar(rest);
                        let p = self.xcdr(rest);
                        n += self.display_mode_element(
                            it,
                            depth,
                            field_width - n,
                            precision - n,
                            inner,
                            p,
                            risky,
                        )?;
                    }
                } else if car.is_symbol() {
                    let tem = crate::data::f_boundp(self, &[car])?;
                    elt = self.xcdr(elt);
                    if !elt.is_cons() {
                        elt = self.string("*invalid*");
                        self.stack.push(elt);
                        continue 'tail_recurse;
                    }
                    if !tem.is_nil() {
                        let tem = crate::data::f_symbol_value(self, &[car])?;
                        if !tem.is_nil() {
                            elt = self.xcar(elt);
                            continue 'tail_recurse;
                        }
                    }
                    elt = self.xcdr(elt);
                    if elt.is_nil() {
                        break 'tail_recurse;
                    } else if !elt.is_cons() {
                        elt = self.string("*invalid*");
                        self.stack.push(elt);
                        continue 'tail_recurse;
                    }
                    elt = self.xcar(elt);
                    continue 'tail_recurse;
                } else if let Some(lim) = car.as_fixnum() {
                    let lim = lim as i32;
                    elt = self.xcdr(elt);
                    if lim < 0 {
                        precision = if precision <= 0 {
                            -lim
                        } else {
                            precision.min(-lim)
                        };
                    } else if lim > 0 {
                        let lim = if precision > 0 {
                            precision.min(lim)
                        } else {
                            lim
                        };
                        field_width = lim.max(field_width);
                    }
                    continue 'tail_recurse;
                } else if car.is_string() || car.is_cons() {
                    let mut tail = crate::lisp::ForEachTail::unrooted(elt);
                    while tail.tail.is_cons() {
                        if 0 < precision && precision <= n {
                            break;
                        }
                        let rest = self.xcdr(tail.tail);
                        let fw = if !rest.is_cons() { field_width - n } else { 0 };
                        let item = self.xcar(tail.tail);
                        n += self.display_mode_element(
                            it,
                            depth,
                            fw,
                            precision - n,
                            item,
                            props,
                            risky,
                        )?;
                        tail.step_safe(self);
                    }
                }
                break 'tail_recurse;
            } else {
                elt = self.string("*invalid*");
                self.stack.push(elt);
                continue 'tail_recurse;
            }
        }
        if field_width > 0 && n < field_width {
            match self.xdisp.mode_line_target {
                ModeLineTarget::Noprop | ModeLineTarget::Title => {
                    n += self.store_mode_line_noprop(b"", field_width - n, 0);
                }
                ModeLineTarget::String => {
                    n += self.store_mode_line_string(
                        Some(b""),
                        Value::NIL,
                        false,
                        field_width - n,
                        0,
                        Value::NIL,
                    )?;
                }
                ModeLineTarget::Display => {
                    let empty: Rc<[u8]> = Rc::from(&b""[..]);
                    n += self.display_string(
                        Some(empty),
                        Value::NIL,
                        Value::NIL,
                        0,
                        0,
                        it,
                        field_width - n,
                        0,
                        0,
                        0,
                    )?;
                }
            }
        }
        Ok(n)
    }

    fn decode_mode_spec_coding(&mut self, coding_system: Value, buf: &mut Vec<u8>, eol_flag: bool) {
        let multibyte = !self.plain_value(sym::ENABLE_MULTIBYTE_CHARACTERS).is_nil();
        let val = self.coding_system_spec(coding_system).unwrap_or(Value::NIL);
        let mut eoltype = Value::NIL;
        if !self.is_vector(val) {
            buf.push(if multibyte { b'-' } else { b' ' });
            if eol_flag {
                eoltype = self.find_symbol_value(sym::EOL_MNEMONIC_UNDECIDED);
            }
        } else {
            let items = self.vector_items(val).to_vec();
            let (attrs, eolvalue) = (items[0], items[2]);
            if multibyte {
                let mnemonic = self
                    .attr(attrs, crate::coding::ATTR_MNEMONIC)
                    .as_fixnum()
                    .unwrap_or(0);
                buf.extend_from_slice(&crate::character::char_string(mnemonic as u32));
            } else {
                buf.push(b' ');
            }
            if eol_flag {
                eoltype = if eolvalue.is_nil() || self.is_vector(eolvalue) {
                    self.find_symbol_value(sym::EOL_MNEMONIC_UNDECIDED)
                } else if eolvalue == sym::UNIX {
                    self.find_symbol_value(sym::EOL_MNEMONIC_UNIX)
                } else if eolvalue == sym::DOS {
                    self.find_symbol_value(sym::EOL_MNEMONIC_DOS)
                } else {
                    self.find_symbol_value(sym::EOL_MNEMONIC_MAC)
                };
            }
        }
        if eol_flag {
            if eoltype.is_string() {
                buf.extend_from_slice(self.string_bytes(eoltype));
            } else if let Some(c) = eoltype
                .as_fixnum()
                .filter(|&c| (0..=i64::from(crate::character::MAX_CHAR)).contains(&c))
            {
                buf.extend_from_slice(&crate::character::char_string(c as u32));
            } else {
                buf.extend_from_slice(b"(*invalid*)");
            }
        }
    }

    pub fn decode_mode_spec(
        &mut self,
        w: Value,
        c: u32,
        field_width: i32,
    ) -> LispResult<(Vec<u8>, Value)> {
        let f = self.window(w).frame;
        let message_buf_size = (4 * self.frame(f).geometry.text_cols) as i32;
        let width = field_width.min(message_buf_size);
        let b = self.current_buffer;
        let mut obj = Value::NIL;
        let read_only = !self.buffer_local_value(sym::BUFFER_READ_ONLY, b).is_nil();
        let (modiff, save_modiff) = (self.cur().text.modiff, self.cur().text.save_modiff);
        let lit = |s: &str| Ok((s.as_bytes().to_vec(), Value::NIL));
        let Some(ch) = char::from_u32(c) else {
            return lit("");
        };
        match ch {
            '*' => {
                if read_only {
                    return lit("%");
                }
                if modiff > save_modiff {
                    return lit("*");
                }
                return lit("-");
            }
            '+' => {
                if modiff > save_modiff {
                    return lit("*");
                }
                if read_only {
                    return lit("%");
                }
                return lit("-");
            }
            '&' => {
                return if modiff > save_modiff {
                    lit("*")
                } else {
                    lit("-")
                };
            }
            '%' => return lit("%"),
            '[' => {
                let level = self.minibuf.command_loop_level;
                if level > 5 {
                    return lit("[[[... ");
                }
                return Ok((vec![b'['; level.max(0) as usize], Value::NIL));
            }
            ']' => {
                let level = self.minibuf.command_loop_level;
                if level > 5 {
                    return lit(" ...]]]");
                }
                return Ok((vec![b']'; level.max(0) as usize], Value::NIL));
            }
            '-' => {
                if matches!(
                    self.xdisp.mode_line_target,
                    ModeLineTarget::Noprop | ModeLineTarget::String
                ) {
                    return lit("--");
                }
                if field_width <= 0 || field_width as usize > LOTS_OF_DASHES + 1 {
                    return Ok((
                        vec![b'-'; (message_buf_size - 1).max(0) as usize],
                        Value::NIL,
                    ));
                }
                return Ok((vec![b'-'; LOTS_OF_DASHES], Value::NIL));
            }
            'b' => obj = self.buffer(b).name,
            'c' | 'C' => {
                if self.xdisp.mode_line_target == ModeLineTarget::Title {
                    return lit("");
                }
                let col = self.current_column()?;
                let disp_col = if ch == 'C' { col + 1 } else { col };
                self.window_mut(w).column_number_displayed = col;
                return Ok((pint2str(width, disp_col), Value::NIL));
            }
            'e' => {
                return if self.find_symbol_value(sym::MEMORY_FULL).is_nil() {
                    lit("")
                } else {
                    lit("!MEM FULL! ")
                };
            }
            'F' => {
                let fr = self.frame(f);
                if !fr.title.is_nil() {
                    return Ok((self.string_bytes(fr.title).to_vec(), Value::NIL));
                }
                if fr.flags.explicit_name || !self.frame_window_p(f) {
                    return Ok((self.string_bytes(fr.name).to_vec(), Value::NIL));
                }
                return lit("Emacs");
            }
            'f' => obj = self.buffer_local_value(sym::BUFFER_FILE_NAME, b),
            'i' => {
                let size = (self.cur().zv - self.cur().begv) as i64;
                return Ok((pint2str(width, size), Value::NIL));
            }
            'I' => {
                let size = (self.cur().zv - self.cur().begv) as i64;
                return Ok((pint2hrstr(width, size), Value::NIL));
            }
            'l' => return self.decode_mode_spec_line(w, width),
            'm' => obj = self.buffer_local_value(sym::MODE_NAME, b),
            'n' => {
                let buf = self.cur();
                if buf.begv > 1 || buf.zv < buf.z() {
                    return lit(" Narrow");
                }
            }
            'o' => {
                let toppos = self.marker(self.window(w).start).charpos as i64;
                let buf = self.cur();
                let botpos = buf.z() as i64 - self.window(w).window_end_pos;
                let (begv, zv) = (buf.begv as i64, buf.zv as i64);
                if zv <= botpos {
                    return if toppos <= begv {
                        lit("All")
                    } else {
                        lit("Bottom")
                    };
                } else if toppos <= begv {
                    return lit("Top");
                }
                let p = percent99(toppos - begv, (toppos - begv) + (zv - botpos));
                return Ok((format!("{p:2}%").into_bytes(), Value::NIL));
            }
            'p' => {
                let pos = self.marker(self.window(w).start).charpos as i64;
                let buf = self.cur();
                let (begv, zv, z) = (buf.begv as i64, buf.zv as i64, buf.z() as i64);
                if self.window(w).window_end_pos <= z - zv {
                    return if pos <= begv {
                        lit("All")
                    } else {
                        lit("Bottom")
                    };
                } else if pos <= begv {
                    return lit("Top");
                }
                let p = percent99(pos - begv, zv - begv);
                return Ok((format!("{p:2}%").into_bytes(), Value::NIL));
            }
            'P' => {
                let toppos = self.marker(self.window(w).start).charpos as i64;
                let buf = self.cur();
                let botpos = buf.z() as i64 - self.window(w).window_end_pos;
                let (begv, zv) = (buf.begv as i64, buf.zv as i64);
                if zv <= botpos {
                    return if toppos <= begv {
                        lit("All")
                    } else {
                        lit("Bottom")
                    };
                }
                let p = percent99(botpos - begv, zv - begv);
                let s = if begv < toppos {
                    format!("{p:2}%")
                } else {
                    format!("Top{p:2}%")
                };
                return Ok((s.into_bytes(), Value::NIL));
            }
            'q' => {
                let toppos = self.marker(self.window(w).start).charpos as i64;
                let buf = self.cur();
                let botpos = buf.z() as i64 - self.window(w).window_end_pos;
                let (begv, zv) = (buf.begv as i64, buf.zv as i64);
                if toppos <= begv && zv <= botpos {
                    return lit("All   ");
                }
                let top_perc = if toppos <= begv {
                    0
                } else {
                    percent99(toppos - begv, zv - begv)
                };
                let bot_perc = if zv <= botpos {
                    100
                } else {
                    percent99(botpos - begv, zv - begv)
                };
                let s = if top_perc == bot_perc {
                    format!("{top_perc}%")
                } else {
                    format!("{top_perc}-{bot_perc}%")
                };
                return Ok((s.into_bytes(), Value::NIL));
            }
            's' => {
                let cb = self.current_buffer;
                obj = crate::process::f_get_buffer_process(self, &[cb])?;
                if obj.is_nil() {
                    return lit("no process");
                }
                unreachable!("process-status needs process objects");
            }
            '@' => {
                let curdir = self.find_symbol_value(sym::DEFAULT_DIRECTORY);
                let mut val = Value::NIL;
                if curdir.is_string() {
                    self.gc_inhibited += 1;
                    let r = self.safe_call(true, &[sym::FILE_REMOTE_P, curdir]);
                    self.gc_inhibited -= 1;
                    val = r?;
                }
                return if val.is_nil() { lit("-") } else { lit("@") };
            }
            'z' | 'Z' => {
                let eol_flag = ch == 'Z';
                let mut buf = Vec::new();
                if !self.frame_window_p(f) {
                    let t = self.frame(f).terminal;
                    let (kb, tm) = {
                        let term = self.terminal(t);
                        (term.keyboard_coding.id, term.terminal_coding.id)
                    };
                    let kb = self.coding_id_name(kb);
                    self.decode_mode_spec_coding(kb, &mut buf, false);
                    let tm = self.coding_id_name(tm);
                    self.decode_mode_spec_coding(tm, &mut buf, false);
                }
                let coding = self.buffer_local_value(sym::BUFFER_FILE_CODING_SYSTEM, b);
                self.decode_mode_spec_coding(coding, &mut buf, eol_flag);
                return Ok((buf, Value::NIL));
            }
            _ => {}
        }
        if obj.is_string() {
            let bytes = self.string_bytes(obj).to_vec();
            let bytes = bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())].to_vec();
            return Ok((bytes, obj));
        }
        lit("")
    }

    fn decode_mode_spec_line(&mut self, w: Value, width: i32) -> LispResult<(Vec<u8>, Value)> {
        let no_value = |width: i32| {
            let mut buf = vec![b' '; (width - 2).max(0) as usize];
            buf.extend_from_slice(b"??");
            Ok((buf, Value::NIL))
        };
        if self.xdisp.mode_line_target == ModeLineTarget::Title {
            return Ok((Vec::new(), Value::NIL));
        }
        let start = self.window(w).start;
        let (mut startpos, mut startpos_byte) = {
            let m = self.marker(start);
            (m.charpos as i64, m.bytepos as i64)
        };
        let height = self.window(w).total_lines;
        let b = self.current_buffer;
        let same_buffer = b == self.window(w).contents;
        let (begv, begv_byte, zv, zv_byte) = {
            let buf = self.cur();
            (
                buf.begv as i64,
                buf.begv_byte as i64,
                buf.zv as i64,
                buf.zv_byte as i64,
            )
        };
        if !same_buffer || !(begv_byte <= startpos_byte && startpos_byte <= zv_byte) {
            startpos = begv;
            startpos_byte = begv_byte;
        }
        if self.window(w).base_line_pos == -1 {
            return no_value(width);
        }
        let limit_var = self.find_symbol_value(sym::LINE_NUMBER_DISPLAY_LIMIT);
        if let Some(limit) = limit_var.as_fixnum()
            && zv - begv > limit
        {
            return no_value(width);
        }
        let win = self.window(w);
        let (line, linepos, linepos_byte) = if same_buffer
            && win.base_line_number > 0
            && win.base_line_pos > 0
            && win.base_line_pos <= startpos
        {
            let lp = win.base_line_pos;
            (
                win.base_line_number,
                lp,
                self.cur().text.char_to_byte(lp as usize) as i64,
            )
        } else {
            (1, begv, begv_byte)
        };
        let (mut nlines, _) =
            self.display_count_lines(linepos_byte as usize, startpos_byte as usize, startpos);
        let topline = nlines + line;
        if !same_buffer {
        } else if startpos == begv {
            let win = self.window_mut(w);
            win.base_line_number = topline;
            win.base_line_pos = begv;
        } else if nlines < height + 25 || nlines > height * 3 + 50 || linepos == begv {
            let mut limit = begv;
            let mut limit_byte = begv_byte;
            let width_var = self.int_var(sym::LINE_NUMBER_DISPLAY_LIMIT_WIDTH);
            let distance = if width_var < 0 {
                0
            } else {
                width_var.checked_mul(height * 2 + 30).unwrap_or(i64::MAX)
            };
            if startpos.saturating_sub(distance) > limit {
                limit = startpos - distance;
                limit_byte = self.cur().text.char_to_byte(limit as usize) as i64;
            }
            let (n, position) = self.display_count_lines(
                startpos_byte as usize,
                limit_byte as usize,
                -(height * 2 + 30),
            );
            nlines = n;
            if position as i64 == limit_byte && limit == startpos.saturating_sub(distance) {
                let win = self.window_mut(w);
                win.base_line_pos = -1;
                win.base_line_number = 0;
                return no_value(width);
            }
            let base_pos = self.cur().text.byte_to_char(position) as i64;
            let win = self.window_mut(w);
            win.base_line_number = topline - nlines;
            win.base_line_pos = base_pos;
        }
        let (pt, pt_byte) = (self.cur().pt as i64, self.cur().pt_byte as i64);
        let (nlines, _) = self.display_count_lines(startpos_byte as usize, pt_byte as usize, pt);
        self.xdisp.line_number_displayed = true;
        Ok((pint2str(width, topline + nlines), Value::NIL))
    }

    pub fn display_mode_lines(&mut self, w: Value) -> LispResult<i32> {
        let old_selected_window = self.windows.selected_window;
        let new_frame = self.window(w).frame;
        let count = self.specpdl_index();
        self.record_unwind(restore_selected_window, old_selected_window);
        let fsw = self.frame(new_frame).selected_window;
        self.record_unwind(restore_frame_selected_window, fsw);
        let r = self.display_mode_lines_1(w, old_selected_window, new_frame);
        let n = self.unbind_to_value(count, r.map(|n| Value::fixnum(i64::from(n))))?;
        let n = n.as_fixnum().unwrap_or(0) as i32;
        if n > 0 {
            self.window_mut(w).flags.must_be_updated_p = true;
        }
        Ok(n)
    }

    fn display_mode_lines_1(
        &mut self,
        w: Value,
        old_selected_window: Value,
        new_frame: Value,
    ) -> LispResult<i32> {
        let mut n = 0;
        if self.window_wants_mode_line(w) {
            let contents = self.window(w).contents;
            let default_help = self.buffer_local_value(sym::MODE_LINE_DEFAULT_HELP_ECHO, contents);
            let help = if self.functionp(default_help) {
                self.safe_call(true, &[default_help, w])?
            } else if default_help.is_string() {
                default_help
            } else {
                Value::NIL
            };
            self.window_mut(w).mode_line_help_echo = help;
        }
        self.frames.selected_frame = new_frame;
        self.windows.selected_window = w;
        self.frame_mut(new_frame).selected_window = w;
        self.xdisp.line_number_displayed = false;
        self.window_mut(w).column_number_displayed = -1;
        let sel_w = old_selected_window;
        if self.window_wants_mode_line(w) {
            let format = self.window_parameter(w, sym::MODE_LINE_FORMAT);
            let face = self.line_active_face_id_3(
                sel_w,
                sel_w,
                w,
                MODE_LINE_ACTIVE_FACE_ID,
                MODE_LINE_INACTIVE_FACE_ID,
            );
            let format = if format.is_nil() {
                self.plain_value(sym::MODE_LINE_FORMAT)
            } else {
                format
            };
            self.display_mode_line(w, face, format)?;
            n += 1;
        }
        if self.window_wants_tab_line(w) {
            let format = self.window_parameter(w, sym::TAB_LINE_FORMAT);
            let face = self.line_active_face_id_3(
                sel_w,
                sel_w,
                w,
                TAB_LINE_ACTIVE_FACE_ID,
                TAB_LINE_INACTIVE_FACE_ID,
            );
            let format = if format.is_nil() {
                self.plain_value(sym::TAB_LINE_FORMAT)
            } else {
                format
            };
            self.display_mode_line(w, face, format)?;
            n += 1;
        }
        if self.window_wants_header_line(w) {
            let format = self.window_parameter(w, sym::HEADER_LINE_FORMAT);
            let face = self.line_active_face_id_3(
                sel_w,
                sel_w,
                w,
                HEADER_LINE_ACTIVE_FACE_ID,
                HEADER_LINE_INACTIVE_FACE_ID,
            );
            let format = if format.is_nil() {
                self.plain_value(sym::HEADER_LINE_FORMAT)
            } else {
                format
            };
            self.display_mode_line(w, face, format)?;
            n += 1;
        }
        Ok(n)
    }

    pub(crate) fn line_active_face_id_3(
        &self,
        selw: Value,
        mbw: Value,
        scrw: Value,
        active: usize,
        inactive: usize,
    ) -> usize {
        let minibuf_selected = self.windows.minibuf_selected_window;
        if !self.var_true(sym::MODE_LINE_IN_NON_SELECTED_WINDOWS)
            || selw == self.windows.selected_window
            || (self.minibuf.minibuf_level > 0
                && !minibuf_selected.is_nil()
                && mbw == self.windows.minibuf_window
                && scrw == minibuf_selected)
        {
            active
        } else {
            inactive
        }
    }

    fn display_mode_line(&mut self, w: Value, face_id: usize, format: Value) -> LispResult<i32> {
        self.with_gc_deferred(|lisp| lisp.display_mode_line_1(w, face_id, format))
    }

    fn display_mode_line_1(&mut self, w: Value, face_id: usize, format: Value) -> LispResult<i32> {
        let count = self.specpdl_index();
        let base = self.stack.len();
        self.stack.push(format);
        let mut it = It::default();
        self.init_iterator(&mut it, w, -1, -1, None, face_id)?;
        let row = it.glyph_row.expect("mode line row");
        self.glyph_row_mut(row).enabled_p = false;
        let vpos = match row {
            crate::dispextern::GlyphRowRef::Window { vpos, .. } => vpos,
            crate::dispextern::GlyphRowRef::Scratch => 0,
        };
        self.prepare_desired_row(w, vpos, true);
        self.glyph_row_mut(row).mode_line_p = true;
        if face_id == TAB_LINE_ACTIVE_FACE_ID || face_id == TAB_LINE_INACTIVE_FACE_ID {
            self.glyph_row_mut(row).tab_line_p = true;
            if let Some(m) = self.window_matrix_mut(w, crate::dispnew::matrix::WhichMatrix::Desired)
            {
                m.tab_line_p = true;
            }
        } else if (face_id == HEADER_LINE_ACTIVE_FACE_ID || face_id == HEADER_LINE_INACTIVE_FACE_ID)
            && let Some(m) = self.window_matrix_mut(w, crate::dispnew::matrix::WhichMatrix::Desired)
        {
            m.header_line_p = true;
        }
        it.paragraph_embedding = crate::bidi::BidiDir::L2R;
        let data = self.format_mode_line_unwind_data(None, None, Value::NIL, false);
        self.record_unwind(unwind_format_mode_line, data);
        self.xdisp.dont_resize_frames += 1;
        let kb = self.frame_kboard(it.f).unwrap_or(self.current_kboard);
        self.push_kboard(kb);
        self.record_unwind_save_match_data()?;
        let compact = self.find_symbol_value(sym::MODE_LINE_COMPACT);
        let r = (|| -> LispResult<()> {
            if compact.is_nil()
                || face_id == HEADER_LINE_ACTIVE_FACE_ID
                || face_id == HEADER_LINE_INACTIVE_FACE_ID
                || face_id == TAB_LINE_ACTIVE_FACE_ID
                || face_id == TAB_LINE_INACTIVE_FACE_ID
            {
                self.xdisp.mode_line_target = ModeLineTarget::Display;
                self.display_mode_element(&mut it, 0, 0, 0, format, Value::NIL, false)?;
            } else {
                self.display_compact_mode_line(&mut it, format, compact)?;
            }
            Ok(())
        })();
        self.pop_kboard();
        r?;
        let space: Rc<[u8]> = Rc::from(&b" "[..]);
        self.display_string(
            Some(space),
            Value::NIL,
            Value::NIL,
            0,
            0,
            &mut it,
            10000,
            -1,
            -1,
            0,
        )?;
        self.compute_line_metrics(&mut it);
        {
            let r = self.glyph_row_mut(row);
            r.full_width_p = true;
            r.continued_p = false;
            r.truncated_on_left_p = false;
            r.truncated_on_right_p = false;
        }
        let (box_type, box_thickness) = {
            let face = self.face_from_id(it.f, face_id).expect("realized face");
            (face.box_type, face.box_vertical_line_width)
        };
        self.extend_face_to_end_of_line(&mut it)?;
        if box_type != crate::xfaces::FaceBoxType::NoBox {
            let extra = (box_thickness - (it.current_x - it.last_visible_x)).max(0);
            let r = self.glyph_row_mut(row);
            let used = r.used[TEXT_AREA] as usize;
            let start = r.area[TEXT_AREA];
            if used > 0 {
                let last = &mut r.glyphs[start + used - 1];
                last.right_box_line_p = true;
                if box_thickness > 0 {
                    last.pixel_width += extra as i16;
                }
            }
        }
        let height = self.glyph_row(row).height;
        self.stack.truncate(base);
        self.unbind_to(count)?;
        Ok(height)
    }

    fn display_compact_mode_line(
        &mut self,
        it: &mut It,
        format: Value,
        compact: Value,
    ) -> LispResult<()> {
        let mode_string = f_format_mode_line(self, &[format, Value::NIL, Value::NIL, Value::NIL])?;
        let base = self.stack.len();
        self.stack.push(mode_string);
        let w = it.window;
        let nchars = self.string_chars(mode_string) as i64;
        if compact == sym::LONG && self.window(w).total_cols >= nchars {
            let mut start = Value::fixnum(0);
            loop {
                let end = crate::textprop::f_next_single_property_change(
                    self,
                    &[start, sym::MODE_LINE_ELT_NO, mode_string, Value::NIL],
                )?;
                let elt = crate::fns::f_substring(self, &[mode_string, start, end])?;
                self.stack.push(elt);
                let mb = i32::from(self.string_multibyte(elt));
                self.display_string(None, elt, Value::NIL, 0, 0, it, 0, 0, 0, mb)?;
                start = end;
                if end.is_nil() {
                    break;
                }
            }
        } else {
            let mut i = 0usize;
            let mut i_byte = 0usize;
            let mut prev = 0u32;
            let mut start = Value::fixnum(0);
            let mut elt = self.unibyte_string(b"");
            loop {
                let end = crate::textprop::f_next_single_property_change(
                    self,
                    &[
                        start,
                        sym::MODE_LINE_ELT_NO,
                        mode_string,
                        Value::fixnum(nchars),
                    ],
                )?;
                let end_n = end.as_fixnum().unwrap_or(nchars) as usize;
                while i < end_n {
                    let (mut c, len) =
                        crate::character::string_char(&self.string_bytes(mode_string)[i_byte..]);
                    i += 1;
                    i_byte += len;
                    if c == u32::from(b' ') && prev == u32::from(b' ') {
                        let prev_pos = Value::fixnum(i as i64 - 1);
                        let display = crate::textprop::f_get_text_property(
                            self,
                            &[prev_pos, sym::DISPLAY, mode_string],
                        )?;
                        if display.is_nil() {
                            let piece =
                                crate::fns::f_substring(self, &[mode_string, start, prev_pos])?;
                            elt = crate::fns::f_concat(self, &[elt, piece])?;
                            self.stack.push(elt);
                            let mut display = crate::textprop::f_get_text_property(
                                self,
                                &[Value::fixnum(i as i64), sym::DISPLAY, mode_string],
                            )?;
                            while c == u32::from(b' ') && i < end_n && display.is_nil() {
                                let (cc, len) = crate::character::string_char(
                                    &self.string_bytes(mode_string)[i_byte..],
                                );
                                c = cc;
                                i += 1;
                                i_byte += len;
                                display = crate::textprop::f_get_text_property(
                                    self,
                                    &[Value::fixnum(i as i64), sym::DISPLAY, mode_string],
                                )?;
                            }
                            start = if c == u32::from(b' ') && display.is_nil() {
                                end
                            } else {
                                Value::fixnum(i as i64 - 1)
                            };
                        }
                    }
                    prev = c;
                }
                if start.as_fixnum().unwrap_or(0) < end_n as i64 {
                    let piece = crate::fns::f_substring(self, &[mode_string, start, end])?;
                    elt = crate::fns::f_concat(self, &[elt, piece])?;
                    self.stack.push(elt);
                }
                let mb = i32::from(self.string_multibyte(elt));
                self.display_string(None, elt, Value::NIL, 0, 0, it, 0, 0, 0, mb)?;
                elt = self.unibyte_string(b"");
                start = end;
                if (end_n as i64) >= nchars {
                    break;
                }
            }
        }
        self.stack.truncate(base);
        Ok(())
    }
}

pub fn restore_selected_window(lisp: &mut Lisp, window: Value) -> LispResult<()> {
    if lisp.window_live_p(window) {
        lisp.windows.selected_window = window;
        let f = lisp.window(window).frame;
        lisp.frames.selected_frame = f;
        lisp.frame_mut(f).selected_window = window;
    } else if lisp.is_frame(lisp.frames.selected_frame)
        && lisp.frame_live_p(lisp.frames.selected_frame)
    {
        lisp.windows.selected_window = lisp.frame(lisp.frames.selected_frame).selected_window;
    } else {
        let frames = lisp.frames.list.clone();
        for frame in frames {
            if lisp.frame(frame).parent_frame.is_nil() && !lisp.frame(frame).flags.tooltip {
                lisp.frames.selected_frame = frame;
                lisp.windows.selected_window = lisp.frame(frame).selected_window;
                return Ok(());
            }
        }
        std::process::abort();
    }
    Ok(())
}

pub fn restore_frame_selected_window(lisp: &mut Lisp, window: Value) -> LispResult<()> {
    if lisp.window_live_p(window) {
        let frame = lisp.window(window).frame;
        lisp.frame_mut(frame).selected_window = window;
        if frame == lisp.frames.selected_frame {
            lisp.windows.selected_window = window;
        }
    }
    Ok(())
}

pub fn unwind_format_mode_line(lisp: &mut Lisp, vector: Value) -> LispResult<()> {
    let items = lisp.aref_items(vector).to_vec();
    let old_window = items[7];
    let target_frame_window = items[8];
    let old_top_frame = items[9];
    lisp.xdisp.mode_line_target = ModeLineTarget::from_code(items[0].as_fixnum().unwrap_or(0));
    let len = items[1].as_fixnum().unwrap_or(0).max(0) as usize;
    lisp.xdisp.mode_line_noprop_buf.truncate(len);
    lisp.xdisp.mode_line_string_list = items[2];
    if items[3] != sym::T {
        lisp.xdisp.mode_line_proptrans_alist = items[3];
    }
    lisp.xdisp.mode_line_string_face = items[4];
    lisp.xdisp.mode_line_string_face_prop = items[5];
    if lisp.window_live_p(old_window) {
        if lisp.window_live_p(target_frame_window) {
            let frame = lisp.window(target_frame_window).frame;
            if frame != lisp.window(old_window).frame {
                crate::window::f_select_window(lisp, &[target_frame_window, sym::T])?;
            }
            if !old_top_frame.is_nil() && old_top_frame != frame && lisp.frame_live_p(old_top_frame)
            {
                crate::frame::f_select_frame(lisp, &[old_top_frame, sym::T])?;
            }
        }
        crate::window::f_select_window(lisp, &[old_window, sym::T])?;
        if lisp.window_live_p(target_frame_window) {
            let buffer = items[10];
            if lisp.buffer_live_p(buffer) {
                let cb = lisp.current_buffer;
                lisp.current_buffer = buffer;
                lisp.set_point_from_marker(items[11]);
                lisp.vector_set(vector, 11, Value::NIL);
                lisp.current_buffer = cb;
            }
        }
    }
    if !items[6].is_nil() {
        lisp.set_buffer_internal(items[6]);
        lisp.vector_set(vector, 6, Value::NIL);
    }
    lisp.xdisp.mode_line_unwind_vector = vector;
    if lisp.xdisp.dont_resize_frames > 0 {
        lisp.xdisp.dont_resize_frames -= 1;
    }
    Ok(())
}

impl Lisp {
    pub(crate) fn base_line_number_valid_p(&self, w: Value) -> bool {
        !self.cur().clip_changed && self.cur().text.beg_unchanged >= self.window(w).base_line_pos
    }

    pub(crate) fn set_point_from_marker(&mut self, marker: Value) {
        let (begv, zv, begv_byte, zv_byte) = {
            let b = self.cur();
            (b.begv, b.zv, b.begv_byte, b.zv_byte)
        };
        let m = self.marker(marker);
        let (mpos, mbyte, mbuf) = (m.charpos, m.bytepos, m.buffer);
        let charpos = mpos.clamp(begv, zv);
        let bytepos = if mbuf != self.current_buffer {
            self.cur().text.char_to_byte(charpos)
        } else {
            mbyte.clamp(begv_byte, zv_byte)
        };
        self.set_pt_both(charpos, bytepos);
    }
}

pub fn f_format_mode_line(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.with_gc_deferred(|lisp| format_mode_line_1(lisp, args))
}

fn format_mode_line_1(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let arg = |i: usize| args.get(i).copied().unwrap_or(Value::NIL);
    let (format, mut face, window, mut buffer) = (arg(0), arg(1), arg(2), arg(3));
    let no_props = face.is_fixnum() || lisp.is_bignum(face);
    let count = lisp.specpdl_index();
    let w = lisp.decode_any_window(window)?;
    let window = w;
    if buffer.is_nil() {
        buffer = lisp.window(w).contents;
    }
    let buffer = lisp.check_buffer(buffer)?;
    if !lisp.buffer_live_p(buffer) {
        return Err(lisp.error("Attempt to format a mode line for a dead buffer"));
    }
    if format.is_nil() || lisp.noninteractive {
        return Ok(lisp.unibyte_string(b""));
    }
    if no_props {
        face = Value::NIL;
    }
    let selected = lisp.windows.selected_window;
    let face_id = if face.is_nil() || face == sym::DEFAULT {
        DEFAULT_FACE_ID
    } else if face == sym::T {
        if window == selected {
            MODE_LINE_ACTIVE_FACE_ID
        } else {
            MODE_LINE_INACTIVE_FACE_ID
        }
    } else if face == sym::MODE_LINE_ACTIVE {
        MODE_LINE_ACTIVE_FACE_ID
    } else if face == sym::MODE_LINE_INACTIVE {
        MODE_LINE_INACTIVE_FACE_ID
    } else if face == sym::HEADER_LINE_ACTIVE {
        HEADER_LINE_ACTIVE_FACE_ID
    } else if face == sym::HEADER_LINE_INACTIVE {
        HEADER_LINE_INACTIVE_FACE_ID
    } else if face == sym::TAB_LINE_ACTIVE {
        TAB_LINE_ACTIVE_FACE_ID
    } else if face == sym::TAB_LINE_INACTIVE {
        TAB_LINE_INACTIVE_FACE_ID
    } else if face == sym::TAB_BAR {
        TAB_BAR_FACE_ID
    } else if face == sym::TOOL_BAR {
        TOOL_BAR_FACE_ID
    } else {
        DEFAULT_FACE_ID
    };
    if face == sym::T {
        face = if window == selected {
            sym::MODE_LINE_ACTIVE
        } else {
            sym::MODE_LINE_INACTIVE
        };
    } else if face == sym::DEFAULT {
        face = Value::NIL;
    }
    let old_buffer = lisp.current_buffer;
    let f = lisp.window(w).frame;
    let data = lisp.format_mode_line_unwind_data(Some(f), Some(old_buffer), selected, true);
    lisp.record_unwind(unwind_format_mode_line, data);
    lisp.xdisp.mode_line_proptrans_alist = Value::NIL;
    let r = (|| -> LispResult {
        crate::window::f_select_window(lisp, &[window, sym::T])?;
        lisp.set_buffer_internal(buffer);
        let mut it = It::default();
        lisp.init_iterator(&mut it, w, -1, -1, None, face_id)?;
        lisp.xdisp.dont_resize_frames += 1;
        if lisp.current_buffer == lisp.window(w).contents && !lisp.base_line_number_valid_p(w) {
            lisp.window_mut(w).base_line_number = 0;
        }
        let mut string_start = 0;
        if no_props {
            lisp.xdisp.mode_line_target = ModeLineTarget::Noprop;
            lisp.xdisp.mode_line_string_face_prop = Value::NIL;
            lisp.xdisp.mode_line_string_list = Value::NIL;
            string_start = lisp.xdisp.mode_line_noprop_buf.len();
        } else {
            lisp.xdisp.mode_line_target = ModeLineTarget::String;
            lisp.xdisp.mode_line_string_list = Value::NIL;
            lisp.xdisp.mode_line_string_face = face;
            lisp.xdisp.mode_line_string_face_prop = if face.is_nil() {
                Value::NIL
            } else {
                lisp.list(&[sym::FACE, face])
            };
        }
        let kb = lisp.frame_kboard(it.f).unwrap_or(lisp.current_kboard);
        lisp.push_kboard(kb);
        lisp.xdisp.mode_line_elt_no = 0;
        let r = lisp.display_mode_element(&mut it, 0, 0, 0, format, Value::NIL, false);
        lisp.pop_kboard();
        r?;
        if no_props {
            let bytes = lisp.xdisp.mode_line_noprop_buf[string_start..].to_vec();
            Ok(lisp.make_string(&bytes))
        } else {
            let list = lisp.xdisp.mode_line_string_list;
            let list = crate::fns::f_nreverse(lisp, &[list])?;
            lisp.xdisp.mode_line_string_list = list;
            let empty = lisp.unibyte_string(b"");
            crate::fns::f_mapconcat(lisp, &[sym::IDENTITY, list, empty])
        }
    })();
    lisp.unbind_to_value(count, r)
}
