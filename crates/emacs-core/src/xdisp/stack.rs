use crate::bidi::{BidiDir, BidiIt, ShelvedBidiCache};
use crate::dispextern::{
    DisplayElementType, It, ItMethod, LineWrap, OVERLAY_STRING_CHUNK_SIZE, PropHandled, TextPos,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

#[derive(Clone, Copy)]
struct OverlayEntry {
    overlay: Value,
    string: Value,
    priority: i64,
    after_string_p: bool,
}

fn compare_overlay_entries(e1: &OverlayEntry, e2: &OverlayEntry) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    if e1.after_string_p != e2.after_string_p {
        if e1.overlay == e2.overlay {
            if e1.after_string_p { Greater } else { Less }
        } else if e1.after_string_p {
            Less
        } else {
            Greater
        }
    } else if e1.priority != e2.priority {
        if e1.after_string_p {
            if e2.priority < e1.priority {
                Less
            } else {
                Greater
            }
        } else if e1.priority < e2.priority {
            Less
        } else {
            Greater
        }
    } else {
        Equal
    }
}

impl Lisp {
    pub(super) fn next_overlay_string(&mut self, it: &mut It) -> LispResult<()> {
        it.current.overlay_string_index += 1;
        if it.current.overlay_string_index == it.n_overlay_strings {
            it.ellipsis_p = it.stack[(it.sp - 1) as usize].display_ellipsis_p;
            self.pop_it(it)?;
            it.current.overlay_string_index = -1;
            it.n_overlay_strings = 0;
            if it.sp > 0 && it.string.is_string() && self.string_chars(it.string) == 0 {
                self.pop_it(it)?;
            }
            if it.overlay_strings_charpos == it.charpos() {
                it.ignore_overlay_strings_at_pos_p = true;
            }
            if it.string.is_nil()
                && it.charpos() >= it.end_charpos
                && it.overlay_strings_charpos >= it.end_charpos
            {
                it.overlay_strings_at_end_processed_p = true;
            }
            it.overlay_strings_charpos = -1;
        } else {
            let i = (it.current.overlay_string_index % OVERLAY_STRING_CHUNK_SIZE as i64) as usize;
            if it.current.overlay_string_index != 0 && i == 0 {
                let charpos = it.overlay_strings_charpos;
                self.load_overlay_strings(it, charpos)?;
            }
            it.string = it.overlay_strings[i];
            it.multibyte_p = self.string_multibyte(it.string);
            it.current.string_pos = TextPos::new(0, 0);
            it.method = ItMethod::FromString;
            it.stop_charpos = 0;
            it.end_charpos = self.string_chars(it.string) as i64;
            if it.cmp_it.stop_pos >= 0 {
                it.cmp_it.stop_pos = 0;
            }
            it.prev_stop = 0;
            it.base_level_stop = 0;
            if it.bidi_p {
                it.bidi_it.string.lstring = it.string;
                it.bidi_it.string.s = None;
                it.bidi_it.string.schars = it.end_charpos;
                it.bidi_it.string.bufpos = it.overlay_strings_charpos;
                it.bidi_it.string.from_disp_str = it.string_from_display_prop_p;
                it.bidi_it.string.unibyte = !it.multibyte_p;
                it.bidi_it.w = it.window;
                let fwp = self.frame_window_p(it.f);
                self.bidi_init_it(0, 0, fwp, &mut it.bidi_it)?;
            }
        }
        Ok(())
    }

    pub(super) fn load_overlay_strings(&mut self, it: &mut It, charpos: i64) -> LispResult<()> {
        let charpos = if charpos <= 0 { it.charpos() } else { charpos };
        let buffer = self.current_buffer;
        let mut candidates: Vec<(usize, usize, Value)> = self
            .buffer(buffer)
            .overlays
            .iter()
            .map(|&o| {
                let ov = self.overlay(o);
                (ov.start, ov.end, o)
            })
            .filter(|&(s, e, _)| s as i64 <= charpos + 1 && charpos - 1 <= e as i64)
            .collect();
        candidates.sort_by_key(|n| std::cmp::Reverse(n.0));
        let mut entries: Vec<OverlayEntry> = Vec::new();
        let base = self.stack.len();
        for (start, end, overlay) in candidates {
            let (start, end) = (start as i64, end as i64);
            if end != charpos && start != charpos {
                continue;
            }
            let window = crate::overlay::f_overlay_get(self, &[overlay, sym::WINDOW])?;
            if self.is_window(window) && window != it.window {
                continue;
            }
            let invisible = crate::overlay::f_overlay_get(self, &[overlay, sym::INVISIBLE])?;
            let invis = self.text_prop_means_invisible(invisible);
            let mut record = |lisp: &mut Lisp, string: Value, after: bool| -> LispResult<()> {
                let priority = crate::overlay::f_overlay_get(lisp, &[overlay, sym::PRIORITY])?;
                lisp.stack.push(string);
                entries.push(OverlayEntry {
                    overlay,
                    string,
                    priority: priority.as_fixnum().unwrap_or(0),
                    after_string_p: after,
                });
                Ok(())
            };
            if start == charpos || (end == charpos && invis != 0) {
                let s = crate::overlay::f_overlay_get(self, &[overlay, sym::BEFORE_STRING])?;
                if s.is_string() && self.string_chars(s) != 0 {
                    record(self, s, false)?;
                }
            }
            if end == charpos || (start == charpos && invis != 0) {
                let s = crate::overlay::f_overlay_get(self, &[overlay, sym::AFTER_STRING])?;
                if s.is_string() && self.string_chars(s) != 0 {
                    record(self, s, true)?;
                }
            }
        }
        self.stack.truncate(base);
        if entries.len() > 1 {
            entries.sort_by(compare_overlay_entries);
        }
        it.n_overlay_strings = entries.len() as i64;
        it.overlay_strings_charpos = charpos;
        let mut j = it.current.overlay_string_index;
        let mut i = 0;
        while i < OVERLAY_STRING_CHUNK_SIZE && j < entries.len() as i64 {
            it.overlay_strings[i] = entries[j as usize].string;
            it.string_overlays[i] = entries[j as usize].overlay;
            i += 1;
            j += 1;
        }
        Ok(())
    }

    pub(super) fn get_overlay_strings_1(
        &mut self,
        it: &mut It,
        charpos: i64,
        compute_stop_p: bool,
    ) -> LispResult<bool> {
        it.current.overlay_string_index = 0;
        self.load_overlay_strings(it, charpos)?;
        if it.n_overlay_strings != 0 {
            if compute_stop_p {
                self.compute_stop_pos(it)?;
            }
            if it.bidi_it.first_elt && it.bidi_it.charpos < self.cur().zv as i64 {
                let pe = it.paragraph_embedding;
                self.bidi_paragraph_init(pe, &mut it.bidi_it, true)?;
            }
            if !(!it.bidi_p && it.string.is_string() && self.string_chars(it.string) == 0) {
                self.push_it(it, None)?;
            }
            it.current.string_pos = TextPos::new(0, 0);
            it.string = it.overlay_strings[0];
            it.from_overlay = Value::NIL;
            it.stop_charpos = 0;
            it.end_charpos = self.string_chars(it.string) as i64;
            it.prev_stop = 0;
            it.base_level_stop = 0;
            it.multibyte_p = self.string_multibyte(it.string);
            it.method = ItMethod::FromString;
            it.from_disp_prop_p = false;
            it.cmp_it.id = -1;
            it.paragraph_embedding = if it.bidi_p && it.bidi_it.paragraph_dir == BidiDir::R2L {
                it.bidi_it.paragraph_dir
            } else {
                BidiDir::L2R
            };
            if it.bidi_p {
                let pos = if charpos > 0 { charpos } else { it.charpos() };
                it.bidi_it.string.lstring = it.string;
                it.bidi_it.string.s = None;
                it.bidi_it.string.schars = it.end_charpos;
                it.bidi_it.string.bufpos = pos;
                it.bidi_it.string.from_disp_str = it.string_from_display_prop_p;
                it.bidi_it.string.unibyte = !it.multibyte_p;
                it.bidi_it.w = it.window;
                let fwp = self.frame_window_p(it.f);
                self.bidi_init_it(0, 0, fwp, &mut it.bidi_it)?;
            }
            return Ok(true);
        }
        it.current.overlay_string_index = -1;
        Ok(false)
    }

    pub(super) fn get_overlay_strings(&mut self, it: &mut It, charpos: i64) -> LispResult<bool> {
        it.string = Value::NIL;
        it.method = ItMethod::FromBuffer;
        self.get_overlay_strings_1(it, charpos, true)?;
        Ok(it.string.is_string())
    }

    pub fn push_it(&mut self, it: &mut It, position: Option<TextPos>) -> LispResult<()> {
        let sp = it.sp as usize;
        let mut p = it.stack[sp].clone();
        p.stop_charpos = it.stop_charpos;
        p.prev_stop = it.prev_stop;
        p.base_level_stop = it.base_level_stop;
        p.cmp_it = it.cmp_it;
        p.face_id = it.face_id;
        p.string = it.string;
        p.method = it.method;
        p.from_overlay = it.from_overlay;
        match p.method {
            ItMethod::FromImage => {
                p.u_object = it.object;
                p.u_image_id = it.image_id;
                p.u_slice = it.slice;
            }
            ItMethod::FromStretch | ItMethod::FromXwidget => p.u_object = it.object,
            ItMethod::FromBuffer
            | ItMethod::FromDisplayVector
            | ItMethod::FromString
            | ItMethod::FromCString => {}
        }
        p.position = position.unwrap_or(it.position);
        p.current = it.current;
        p.end_charpos = it.end_charpos;
        p.string_nchars = it.string_nchars;
        p.area = it.area;
        p.multibyte_p = it.multibyte_p;
        p.avoid_cursor_p = it.avoid_cursor_p;
        p.space_width = it.space_width;
        p.font_height = it.font_height;
        p.voffset = it.voffset;
        p.string_from_display_prop_p = it.string_from_display_prop_p;
        p.string_from_prefix_prop_p = it.string_from_prefix_prop_p;
        p.display_ellipsis_p = false;
        p.line_wrap = it.line_wrap;
        p.bidi_p = it.bidi_p;
        p.paragraph_embedding = it.paragraph_embedding;
        p.from_disp_prop_p = it.from_disp_prop_p;
        it.stack[sp] = p;
        it.sp += 1;
        if it.bidi_p {
            self.bidi_push_it(&it.bidi_it);
        }
        Ok(())
    }

    pub fn iterate_out_of_display_property(&mut self, it: &mut It) -> LispResult<()> {
        let buffer_p = !it.string.is_string();
        let eob = if buffer_p {
            self.cur().zv as i64
        } else {
            it.end_charpos
        };
        let bob = if buffer_p { self.cur().begv as i64 } else { 0 };
        if it.bidi_it.first_elt && it.bidi_it.charpos < eob {
            let pe = it.paragraph_embedding;
            self.bidi_paragraph_init(pe, &mut it.bidi_it, true)?;
        }
        while it.bidi_it.charpos >= bob
            && it.prev_stop <= it.bidi_it.charpos
            && it.bidi_it.charpos < it.position.charpos
            && it.bidi_it.charpos < eob
        {
            self.bidi_move_to_visually_next(&mut it.bidi_it)?;
        }
        if it.bidi_it.charpos > it.position.charpos {
            it.prev_stop = it.position.charpos;
        }
        if it.bidi_it.charpos != it.position.charpos {
            it.position = TextPos::new(it.bidi_it.charpos, it.bidi_it.bytepos);
        }
        if buffer_p {
            it.current.pos = it.position;
        } else {
            it.current.string_pos = it.position;
        }
        Ok(())
    }

    fn restore_face_box_flags(&self, it: &mut It, prev_face_id: i32) {
        if it.face_id < 0 {
            return;
        }
        if let Some(face) = self.face_from_id(it.f, it.face_id as usize) {
            let face_box = face.box_type != crate::xfaces::FaceBoxType::NoBox;
            let prev_box = if prev_face_id < 0 {
                None
            } else {
                self.face_from_id(it.f, prev_face_id as usize)
                    .map(|f| f.box_type != crate::xfaces::FaceBoxType::NoBox)
            };
            if !(it.start_of_box_run_p && prev_box == Some(true)) {
                it.start_of_box_run_p = face_box && prev_box != Some(true);
            }
            it.face_box_p = face_box;
        }
    }

    pub fn pop_it(&mut self, it: &mut It) -> LispResult<()> {
        let from_display_prop = it.from_disp_prop_p;
        let prev_pos = it.charpos();
        let prev_face_id = it.face_id;
        it.sp -= 1;
        let p = it.stack[it.sp as usize].clone();
        it.stop_charpos = p.stop_charpos;
        it.prev_stop = p.prev_stop;
        it.base_level_stop = p.base_level_stop;
        it.cmp_it = p.cmp_it;
        it.face_id = p.face_id;
        it.current = p.current;
        it.position = p.position;
        it.string = p.string;
        it.from_overlay = p.from_overlay;
        if it.string.is_nil() {
            it.current.string_pos = TextPos::new(-1, -1);
        }
        it.method = p.method;
        match it.method {
            ItMethod::FromImage => {
                it.image_id = p.u_image_id;
                it.object = p.u_object;
                it.slice = p.u_slice;
            }
            ItMethod::FromXwidget | ItMethod::FromStretch => it.object = p.u_object,
            ItMethod::FromBuffer => {
                self.restore_face_box_flags(it, prev_face_id);
                it.object = self.window(it.window).contents;
            }
            ItMethod::FromString => {
                self.restore_face_box_flags(it, prev_face_id);
                it.object = it.string;
            }
            ItMethod::FromDisplayVector => {
                if it.s.is_some() {
                    it.method = ItMethod::FromCString;
                } else if it.string.is_string() {
                    it.method = ItMethod::FromString;
                } else {
                    it.method = ItMethod::FromBuffer;
                    it.object = self.window(it.window).contents;
                }
            }
            ItMethod::FromCString => {}
        }
        it.end_charpos = p.end_charpos;
        it.string_nchars = p.string_nchars;
        it.area = p.area;
        it.multibyte_p = p.multibyte_p;
        it.avoid_cursor_p = p.avoid_cursor_p;
        it.space_width = p.space_width;
        it.font_height = p.font_height;
        it.voffset = p.voffset;
        it.string_from_display_prop_p = p.string_from_display_prop_p;
        it.string_from_prefix_prop_p = p.string_from_prefix_prop_p;
        it.line_wrap = p.line_wrap;
        it.bidi_p = p.bidi_p;
        it.paragraph_embedding = p.paragraph_embedding;
        it.from_disp_prop_p = p.from_disp_prop_p;
        it.align_visually_p = false;
        if it.bidi_p {
            self.bidi_pop_it(&mut it.bidi_it);
            if from_display_prop
                && (it.method == ItMethod::FromBuffer || it.method == ItMethod::FromString)
            {
                self.iterate_out_of_display_property(it)?;
            }
        }
        if from_display_prop && it.sp == 0 && it.position.charpos != prev_pos {
            it.ignore_overlay_strings_at_pos_p = false;
        }
        Ok(())
    }

    fn with_narrowed_begv<T>(
        &mut self,
        it: &It,
        bv: impl FnOnce(&mut Lisp) -> LispResult<i64>,
        expr: impl FnOnce(&mut Lisp) -> T,
    ) -> LispResult<T> {
        if it.medium_narrowing_begv != 0 {
            let count = self.specpdl.len();
            let point_min = Value::fixnum(self.cur().begv as i64);
            self.record_unwind(unwind_narrowed_begv, point_min);
            let begv = bv(self)? as usize;
            self.set_buf_begv(begv);
            let r = expr(self);
            self.unbind_to(count)?;
            Ok(r)
        } else {
            Ok(expr(self))
        }
    }

    pub fn set_buf_begv(&mut self, begv: usize) {
        let b = self.cur_mut();
        b.begv = begv;
        b.begv_byte = b.text.char_to_byte(begv);
    }

    fn back_to_previous_line_start(&mut self, it: &mut It) -> LispResult<()> {
        let p = self.dec_text_pos(
            TextPos::new(it.charpos(), it.bytepos()),
            self.cur().text.multibyte,
        );
        let cp = p.charpos;
        let w = it.window;
        let pos = it.charpos();
        let truncate = it.line_wrap == LineWrap::Truncate;
        let found = self.with_narrowed_begv(
            it,
            |l| {
                if truncate {
                    Ok(l.cur().begv as i64)
                } else {
                    l.get_small_narrowing_begv(w, pos)
                }
            },
            |l| {
                let begv = l.cur().begv;
                l.find_newline(cp as usize, begv, -1).0
            },
        )?;
        it.current.pos = TextPos::new(found as i64, self.cur().text.char_to_byte(found) as i64);
        Ok(())
    }

    fn strings_with_newlines(&mut self, startpos: i64, endpos: i64, w: Value) -> LispResult<bool> {
        let buffer = self.current_buffer;
        let mut candidates: Vec<(usize, usize, Value)> = self
            .buffer(buffer)
            .overlays
            .iter()
            .map(|&o| {
                let ov = self.overlay(o);
                (ov.start, ov.end, o)
            })
            .filter(|&(s, e, _)| s as i64 <= endpos && startpos <= e as i64)
            .collect();
        candidates.sort_by_key(|n| std::cmp::Reverse(n.0));
        let zv = self.cur().zv as i64;
        for (ostart, oend, overlay) in candidates {
            let window = crate::overlay::f_overlay_get(self, &[overlay, sym::WINDOW])?;
            if self.is_window(window) && window != w {
                continue;
            }
            let (ostart, oend) = (ostart as i64, oend as i64);
            if !((startpos < oend && ostart < endpos)
                || (ostart == oend && (startpos == oend || (endpos == zv && oend == endpos))))
            {
                continue;
            }
            for which in [sym::BEFORE_STRING, sym::AFTER_STRING] {
                let s = crate::overlay::f_overlay_get(self, &[overlay, which])?;
                if s.is_string()
                    && self.string_chars(s) != 0
                    && self.string_bytes(s).contains(&b'\n')
                {
                    return Ok(true);
                }
            }
        }
        let mut cpos = Value::fixnum(startpos);
        let limpos = Value::fixnum(endpos);
        loop {
            cpos = crate::textprop::f_next_single_property_change(
                self,
                &[cpos, sym::DISPLAY, Value::NIL, limpos],
            )?;
            if cpos.is_nil() || cpos.as_fixnum().unwrap_or(0) >= endpos {
                break;
            }
            let spec = self.get_char_property(cpos, sym::DISPLAY, Value::NIL)?;
            let string = self.string_from_display_spec(spec);
            if string.is_string() && self.string_bytes(string).contains(&b'\n') {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn string_from_display_spec(&self, spec: Value) -> Value {
        let mut spec = spec;
        if self.is_vector(spec) {
            for &elt in self.vector_items(spec) {
                if elt.is_string() {
                    return elt;
                }
            }
        } else {
            while spec.is_cons() {
                let elt = self.xcar(spec);
                if elt.is_string() {
                    return elt;
                }
                spec = self.xcdr(spec);
            }
        }
        spec
    }

    pub(super) fn forward_to_next_line_start(
        &mut self,
        it: &mut It,
        skipped_p: &mut bool,
        bidi_it_prev: Option<&mut BidiIt>,
    ) -> LispResult<bool> {
        let mut bidi_it_prev = bidi_it_prev;
        let mut newline_found_p = false;
        const MAX_NEWLINE_DISTANCE: i32 = 500;
        if it.what == DisplayElementType::Character
            && it.c == 10
            && it.position.charpos == it.charpos()
        {
            if it.bidi_p
                && let Some(prev) = bidi_it_prev.as_deref_mut()
            {
                *prev = it.bidi_it.clone();
            }
            self.set_iterator_to_next(it, false)?;
            it.c = 0;
            return Ok(true);
        }
        let old_selective = it.selective;
        it.selective = 0;
        let mut n = 0;
        while !newline_found_p && n < MAX_NEWLINE_DISTANCE {
            if !self.get_next_display_element(it)? {
                return Ok(false);
            }
            newline_found_p = it.what == DisplayElementType::Character && it.c == 10;
            if newline_found_p
                && it.bidi_p
                && let Some(prev) = bidi_it_prev.as_deref_mut()
            {
                *prev = it.bidi_it.clone();
            }
            self.set_iterator_to_next(it, false)?;
            n += i32::from(!it.string.is_string());
        }
        if !newline_found_p {
            let start = it.charpos();
            let zv = self.cur().zv;
            let limit = self.find_newline(start as usize, zv, 1).0 as i64;
            let bytepos = self.cur().text.char_to_byte(limit as usize) as i64;
            let mut no_strings_with_newlines = it.stop_charpos >= limit;
            if !no_strings_with_newlines {
                if !(self.cur().long_line_optimizations_p && it.line_wrap == LineWrap::Truncate) {
                    let pos = crate::textprop::f_next_single_property_change(
                        self,
                        &[
                            Value::fixnum(start),
                            sym::DISPLAY,
                            Value::NIL,
                            Value::fixnum(limit),
                        ],
                    )?;
                    let buffer = self.current_buffer;
                    no_strings_with_newlines = (pos.is_nil() || pos.as_fixnum() == Some(limit))
                        && self.next_overlay_change(buffer, start as usize, true) as i64
                            == self.cur().zv as i64;
                } else {
                    no_strings_with_newlines =
                        !self.strings_with_newlines(start, limit, it.window)?;
                }
            }
            if no_strings_with_newlines {
                if !it.bidi_p || bidi_it_prev.is_none() {
                    it.current.pos = TextPos::new(limit, bytepos);
                } else {
                    if it.bidi_it.disp_pos < limit {
                        it.bidi_it.disp_pos = limit;
                        it.bidi_it.disp_prop = 0;
                    }
                    let mut bprev;
                    loop {
                        bprev = it.bidi_it.clone();
                        self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                        if it.bidi_it.charpos == limit {
                            break;
                        }
                    }
                    it.current.pos = TextPos::new(limit, it.bidi_it.bytepos);
                    if let Some(prev) = bidi_it_prev.as_deref_mut() {
                        *prev = bprev;
                    }
                }
                *skipped_p = true;
                newline_found_p = true;
            } else {
                while !newline_found_p {
                    if !self.get_next_display_element(it)? {
                        break;
                    }
                    newline_found_p = it.at_end_of_line_p();
                    if newline_found_p
                        && it.bidi_p
                        && let Some(prev) = bidi_it_prev.as_deref_mut()
                    {
                        *prev = it.bidi_it.clone();
                    }
                    self.set_iterator_to_next(it, false)?;
                }
            }
        }
        it.selective = old_selective;
        Ok(newline_found_p)
    }

    pub(super) fn back_to_previous_visible_line_start(&mut self, it: &mut It) -> LispResult<()> {
        while it.charpos() > self.cur().begv as i64 {
            self.back_to_previous_line_start(it)?;
            let begv = self.cur().begv as i64;
            if it.charpos() <= begv {
                break;
            }
            if it.selective > 0 && self.indented_beyond_p(it.charpos(), it.selective)? {
                continue;
            }
            let prop =
                self.get_char_property(Value::fixnum(it.charpos() - 1), sym::INVISIBLE, it.window)?;
            if self.text_prop_means_invisible(prop) != 0 {
                continue;
            }
            if it.charpos() <= begv {
                break;
            }
            let mut it2data: ShelvedBidiCache = None;
            let mut it2 = self.save_it(it, &mut it2data);
            let beg;
            let replaced = match self.find_composition(it.charpos(), -1, Value::NIL)? {
                Some((b, _, _)) if (b as i64) < it.charpos() => Some(b as i64),
                _ => None,
            };
            if let Some(b) = replaced {
                beg = b;
            } else {
                it2.current.pos.charpos -= 1;
                it2.current.pos.bytepos -= 1;
                let pos = it2.current.pos.charpos;
                it2.sp = 0;
                self.bidi_unshelve_cache(None, false);
                it2.string_from_display_prop_p = false;
                it2.from_disp_prop_p = false;
                let mut found = None;
                if self.handle_display_prop(&mut it2)? == PropHandled::Return {
                    let (val, overlay) = self.get_char_property_and_overlay(
                        Value::fixnum(pos),
                        sym::DISPLAY,
                        Value::NIL,
                    )?;
                    if !val.is_nil() {
                        if self.is_overlay(overlay) {
                            found = Some(self.overlay(overlay).start as i64);
                        } else if let Some((_, start, _)) =
                            self.get_property_and_range(pos, sym::DISPLAY, Value::NIL)?
                        {
                            found = Some(start as i64);
                        }
                    }
                }
                self.restore_it(it, None, &mut it2data);
                match found {
                    Some(b) => beg = b,
                    None => break,
                }
            }
            let beg = beg.max(self.cur().begv as i64);
            it.current.pos = TextPos::new(beg, self.cur().text.char_to_byte(beg as usize) as i64);
        }
        it.continuation_lines_width = 0;
        Ok(())
    }

    pub fn reseat_at_previous_visible_line_start(&mut self, it: &mut It) -> LispResult<()> {
        self.back_to_previous_visible_line_start(it)?;
        let pos = it.current.pos;
        self.reseat(it, pos, true)
    }
}

fn unwind_narrowed_begv(lisp: &mut Lisp, point_min: Value) -> LispResult<()> {
    let begv = point_min.as_fixnum().unwrap_or(1) as usize;
    lisp.set_buf_begv(begv);
    Ok(())
}
