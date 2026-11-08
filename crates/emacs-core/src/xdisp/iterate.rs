use crate::dispextern::{DisplayElementType, DpVec, It, ItMethod};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

impl Lisp {
    pub fn set_iterator_to_next(&mut self, it: &mut It, reseat_p: bool) -> LispResult<()> {
        if self.int_var(sym::MAX_REDISPLAY_TICKS) > 0 {
            self.update_redisplay_ticks(1, it.window)?;
        }
        match it.method {
            ItMethod::FromBuffer => {
                if it.at_end_of_line_p() && reseat_p {
                    self.reseat_at_next_visible_line_start(it, false)?;
                } else if it.cmp_it.id >= 0 {
                    if !it.bidi_p {
                        it.current.pos.charpos += it.cmp_it.nchars;
                        it.current.pos.bytepos += it.cmp_it.nbytes;
                    } else {
                        for _ in 0..it.cmp_it.nchars {
                            self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                        }
                        it.current.pos.bytepos = it.bidi_it.bytepos;
                        it.current.pos.charpos = it.bidi_it.charpos;
                    }
                    if (!it.bidi_p || !it.cmp_it.reversed_p) && it.cmp_it.to < it.cmp_it.nglyphs {
                        it.cmp_it.from = it.cmp_it.to;
                    } else if (it.bidi_p && it.cmp_it.reversed_p) && it.cmp_it.from > 0 {
                        it.cmp_it.to = it.cmp_it.from;
                    } else {
                        let mut stop = it.end_charpos;
                        if it.bidi_it.scan_dir < 0 {
                            stop = self.bidi_level_start(i32::from(it.bidi_it.resolved_level)) - 1;
                        }
                        let charpos = it.charpos();
                        self.composition_compute_stop_pos(
                            &mut it.cmp_it,
                            charpos,
                            stop,
                            Value::NIL,
                            true,
                        )?;
                    }
                } else if !it.bidi_p {
                    it.current.pos.bytepos += i64::from(it.len);
                    it.current.pos.charpos += 1;
                } else {
                    let prev_scan_dir = it.bidi_it.scan_dir;
                    if it.bidi_it.new_paragraph {
                        let pe = it.paragraph_embedding;
                        self.bidi_paragraph_init(pe, &mut it.bidi_it, false)?;
                    }
                    self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                    it.current.pos.bytepos = it.bidi_it.bytepos;
                    it.current.pos.charpos = it.bidi_it.charpos;
                    if prev_scan_dir != it.bidi_it.scan_dir {
                        let mut stop = it.end_charpos;
                        if it.bidi_it.scan_dir < 0 {
                            stop = self.bidi_level_start(i32::from(it.bidi_it.resolved_level)) - 1;
                        }
                        let charpos = it.charpos();
                        self.composition_compute_stop_pos(
                            &mut it.cmp_it,
                            charpos,
                            stop,
                            Value::NIL,
                            true,
                        )?;
                    }
                }
            }
            ItMethod::FromCString => {
                if !it.bidi_p || it.charpos() >= it.bidi_it.string.schars {
                    it.current.pos.bytepos += i64::from(it.len);
                    it.current.pos.charpos += 1;
                } else {
                    self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                    it.current.pos.bytepos = it.bidi_it.bytepos;
                    it.current.pos.charpos = it.bidi_it.charpos;
                }
            }
            ItMethod::FromDisplayVector => {
                it.current.dpvec_index += 1;
                it.face_id = it.saved_face_id;
                if i64::from(it.current.dpvec_index) >= it.dpend {
                    let recheck_faces = it.ellipsis_p;
                    if it.s.is_some() {
                        it.method = ItMethod::FromCString;
                    } else if it.string.is_string() {
                        it.method = ItMethod::FromString;
                    } else {
                        it.method = ItMethod::FromBuffer;
                        it.object = self.window(it.window).contents;
                    }
                    it.dpvec = DpVec::None;
                    it.current.dpvec_index = -1;
                    if it.dpvec_char_len < 0 {
                        self.reseat_at_next_visible_line_start(it, true)?;
                    } else if it.dpvec_char_len > 0 {
                        it.len = it.dpvec_char_len;
                        self.set_iterator_to_next(it, reseat_p)?;
                    }
                    if recheck_faces {
                        it.stop_charpos = if it.method == ItMethod::FromString {
                            it.string_charpos()
                        } else {
                            it.charpos()
                        };
                    }
                }
            }
            ItMethod::FromString => {
                let at_end = if it.current.overlay_string_index >= 0 {
                    it.string_charpos() >= self.string_chars(it.string) as i64
                } else {
                    it.string_charpos() >= it.end_charpos
                };
                if !at_end {
                    self.advance_in_string(it)?;
                }
                self.consider_string_end(it)?;
            }
            ItMethod::FromImage | ItMethod::FromStretch | ItMethod::FromXwidget => {
                self.pop_it(it)?;
                if it.method == ItMethod::FromString {
                    self.consider_string_end(it)?;
                }
            }
        }
        Ok(())
    }

    fn string_stop_limit(&self, it: &It) -> i64 {
        let mut stop = self.string_chars(it.string) as i64;
        if it.bidi_p && it.bidi_it.scan_dir < 0 {
            stop = -1;
        } else if it.end_charpos < stop {
            stop = it.end_charpos;
        }
        stop
    }

    fn advance_in_string(&mut self, it: &mut It) -> LispResult<()> {
        if it.cmp_it.id >= 0 {
            if !it.bidi_p {
                it.current.string_pos.charpos += it.cmp_it.nchars;
                it.current.string_pos.bytepos += it.cmp_it.nbytes;
            } else {
                for _ in 0..it.cmp_it.nchars {
                    self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                }
                it.current.string_pos.bytepos = it.bidi_it.bytepos;
                it.current.string_pos.charpos = it.bidi_it.charpos;
            }
            if (!it.bidi_p || !it.cmp_it.reversed_p) && it.cmp_it.to < it.cmp_it.nglyphs {
                it.cmp_it.from = it.cmp_it.to;
            } else if (it.bidi_p && it.cmp_it.reversed_p) && it.cmp_it.from > 0 {
                it.cmp_it.to = it.cmp_it.from;
            } else {
                let stop = self.string_stop_limit(it);
                let (charpos, string) = (it.string_charpos(), it.string);
                self.composition_compute_stop_pos(&mut it.cmp_it, charpos, stop, string, true)?;
            }
        } else if !it.bidi_p || it.string_charpos() >= it.bidi_it.string.schars {
            it.current.string_pos.bytepos += i64::from(it.len);
            it.current.string_pos.charpos += 1;
        } else {
            let prev_scan_dir = it.bidi_it.scan_dir;
            self.bidi_move_to_visually_next(&mut it.bidi_it)?;
            it.current.string_pos.bytepos = it.bidi_it.bytepos;
            it.current.string_pos.charpos = it.bidi_it.charpos;
            if prev_scan_dir != it.bidi_it.scan_dir {
                let stop = self.string_stop_limit(it);
                let (charpos, string) = (it.string_charpos(), it.string);
                self.composition_compute_stop_pos(&mut it.cmp_it, charpos, stop, string, true)?;
            }
        }
        Ok(())
    }

    fn consider_string_end(&mut self, it: &mut It) -> LispResult<()> {
        loop {
            if it.current.overlay_string_index >= 0 {
                if it.string_charpos() >= self.string_chars(it.string) as i64 {
                    let (pos, string) = (it.string_charpos(), it.string);
                    self.display_min_width(it, pos, string, Value::NIL)?;
                    it.ellipsis_p = false;
                    self.next_overlay_string(it)?;
                    if it.ellipsis_p {
                        self.setup_for_ellipsis(it, 0);
                    }
                }
                return Ok(());
            }
            if it.string_charpos() == self.string_chars(it.string) as i64 && it.sp > 0 {
                let (pos, string) = (it.string_charpos(), it.string);
                self.display_min_width(it, pos, string, Value::NIL)?;
                self.pop_it(it)?;
                if it.method == ItMethod::FromString {
                    continue;
                }
            }
            return Ok(());
        }
    }

    pub(super) fn next_element_from_display_vector(&mut self, it: &mut It) -> LispResult<bool> {
        let prev_face_id = it.face_id;
        it.face_id = it.saved_face_id;
        let gc = if it.dpend > 0 {
            self.dpvec_ref(it, i64::from(it.current.dpvec_index))
        } else {
            Value::NIL
        };
        if it.dpend > 0 && self.glyph_code_p(gc) {
            it.c = self.glyph_code_char(gc) as i32;
            it.len = crate::character::char_bytes(it.c as u32) as i32;
            if !crate::character::ascii_char_p(it.c as u32)
                && !it.multibyte_p
                && !(((it.sp == 0 && self.is_buffer(it.object))
                    || (it.sp > 1 && !it.stack[0].string.is_nil()))
                    && self.plain_value(sym::ENABLE_MULTIBYTE_CHARACTERS).is_nil())
            {
                it.multibyte_p = true;
            }
            if it.dpvec_face_id >= 0 {
                it.face_id = it.dpvec_face_id;
            } else {
                let lface_id = self.glyph_code_face(gc);
                if lface_id > 0 {
                    it.face_id = self.merge_faces(
                        it.window,
                        sym::T,
                        i64::from(lface_id),
                        it.saved_face_id as usize,
                    )? as i32;
                }
            }
            let this_box = self.face_box_opt(it.f, it.face_id);
            let prev_box = self.face_box_opt(it.f, prev_face_id);
            it.start_of_box_run_p = this_box == Some(true) && prev_box != Some(true);
            let mut next_face_id = it.saved_face_id;
            if i64::from(it.current.dpvec_index) < it.dpend - 1 {
                if it.dpvec_face_id >= 0 {
                    next_face_id = it.dpvec_face_id;
                } else {
                    let gc = self.dpvec_ref(it, i64::from(it.current.dpvec_index) + 1);
                    let lface_id = if self.glyph_code_p(gc) {
                        self.glyph_code_face(gc)
                    } else {
                        0
                    };
                    if lface_id > 0 {
                        next_face_id = self.merge_faces(
                            it.window,
                            sym::T,
                            i64::from(lface_id),
                            it.saved_face_id as usize,
                        )? as i32;
                    }
                }
            }
            let next_box = self.face_box_opt(it.f, next_face_id);
            if this_box == Some(true) && next_box != Some(true) {
                it.end_of_box_run_p = true;
            }
            it.face_box_p = this_box == Some(true);
        } else {
            it.c = 32;
            it.len = 1;
        }
        it.what = DisplayElementType::Character;
        Ok(true)
    }

    pub(super) fn face_box_opt(&self, f: Value, face_id: i32) -> Option<bool> {
        if face_id < 0 {
            return None;
        }
        self.face_from_id(f, face_id as usize)
            .map(|face| face.box_type != crate::xfaces::FaceBoxType::NoBox)
    }
}
