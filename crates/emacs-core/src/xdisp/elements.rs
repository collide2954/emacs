use crate::character;
use crate::dispextern::{DisplayElementType, It, ItMethod, TextPos};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

impl Lisp {
    pub(super) fn get_visually_first_element(&mut self, it: &mut It) -> LispResult<()> {
        let string_p = it.string.is_string() || it.s.is_some();
        let eob = if string_p {
            it.bidi_it.string.schars
        } else {
            self.cur().zv as i64
        };
        let obegv = self.cur().begv as i64;
        let bob = if string_p {
            0
        } else if it.medium_narrowing_begv != 0 {
            let charpos = it.charpos();
            let nb = it.medium_narrowing_begv;
            self.with_narrowed_begv_value(nb, |l| {
                if charpos < l.cur().begv as i64 {
                    obegv
                } else {
                    l.cur().begv as i64
                }
            })?
        } else if it.charpos() < self.cur().begv as i64 {
            obegv
        } else {
            self.cur().begv as i64
        };
        if it.string.is_string() {
            it.bidi_it.charpos = it.string_charpos();
            it.bidi_it.bytepos = it.string_bytepos();
        } else {
            it.bidi_it.charpos = it.charpos();
            it.bidi_it.bytepos = it.bytepos();
        }
        if it.bidi_it.charpos == eob {
            it.bidi_it.first_elt = false;
        } else if it.bidi_it.charpos == bob
            || (!string_p
                && (self.cur().text.byte_at(it.bidi_it.bytepos as usize - 1) == b'\n'
                    || self.cur().text.byte_at(it.bidi_it.bytepos as usize) == b'\n'))
        {
            let pe = it.paragraph_embedding;
            self.bidi_paragraph_init(pe, &mut it.bidi_it, true)?;
            self.bidi_move_to_visually_next(&mut it.bidi_it)?;
        } else {
            let orig_bytepos = it.bidi_it.bytepos;
            if string_p {
                it.bidi_it.charpos = 0;
                it.bidi_it.bytepos = 0;
            } else {
                let charpos = it.charpos();
                let compute = |l: &mut Lisp| {
                    let begv = l.cur().begv;
                    let p = l.find_newline(charpos as usize, begv, -1).0;
                    (p as i64, l.cur().text.char_to_byte(p) as i64)
                };
                let (p, b) = if it.medium_narrowing_begv != 0 {
                    let nb = it.medium_narrowing_begv;
                    self.with_narrowed_begv_value(nb, compute)?
                } else {
                    compute(self)
                };
                it.bidi_it.charpos = p;
                it.bidi_it.bytepos = b;
            }
            let pe = it.paragraph_embedding;
            self.bidi_paragraph_init(pe, &mut it.bidi_it, true)?;
            loop {
                self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                if !(it.bidi_it.bytepos != orig_bytepos && it.bidi_it.charpos < eob) {
                    break;
                }
            }
        }
        if it.string.is_string() {
            it.current.string_pos = TextPos::new(it.bidi_it.charpos, it.bidi_it.bytepos);
        } else {
            it.current.pos = TextPos::new(it.bidi_it.charpos, it.bidi_it.bytepos);
        }
        if it.string.is_string() || it.s.is_none() {
            let (mut stop, charpos) = if it.string.is_string() {
                let mut stop = self.string_chars(it.string) as i64;
                if stop > it.end_charpos {
                    stop = it.end_charpos;
                }
                (stop, it.string_charpos())
            } else {
                (it.end_charpos, it.charpos())
            };
            if it.bidi_it.scan_dir < 0 {
                stop = if it.string.is_string() {
                    -1
                } else {
                    self.bidi_level_start(i32::from(it.bidi_it.resolved_level)) - 1
                };
            }
            let string = it.string;
            self.composition_compute_stop_pos(&mut it.cmp_it, charpos, stop, string, true)?;
        }
        Ok(())
    }

    fn with_narrowed_begv_value<T>(
        &mut self,
        bv: i64,
        f: impl FnOnce(&mut Lisp) -> T,
    ) -> LispResult<T> {
        let count = self.specpdl.len();
        let point_min = Value::fixnum(self.cur().begv as i64);
        self.record_unwind(unwind_narrowed_begv_elements, point_min);
        self.set_buf_begv(bv as usize);
        let r = f(self);
        self.unbind_to(count)?;
        Ok(r)
    }

    pub(super) fn next_element_from_string(&mut self, it: &mut It) -> LispResult<bool> {
        let mut position = it.current.string_pos;
        if it.bidi_p && it.bidi_it.first_elt {
            self.get_visually_first_element(it)?;
            position = TextPos::new(it.string_charpos(), it.string_bytepos());
        }
        if it.string_charpos() < it.end_charpos {
            if it.string_charpos() >= it.stop_charpos {
                if !(!it.bidi_p
                    || it.bidi_it.at_base_level()
                    || it.string_charpos() == it.stop_charpos)
                {
                    let stop = it.stop_charpos;
                    self.handle_stop_backwards(it, stop)?;
                    return self.get_next_element_by_method(it);
                }
                if it.bidi_p {
                    it.prev_stop = it.stop_charpos;
                    if it.bidi_it.at_base_level() {
                        it.base_level_stop = it.stop_charpos;
                    }
                }
                self.handle_stop(it)?;
                return self.get_next_element_by_method(it);
            } else if it.bidi_p && it.string_charpos() < it.prev_stop && !it.bidi_it.at_base_level()
            {
                if it.base_level_stop <= 0 || it.string_charpos() < it.base_level_stop {
                    it.base_level_stop = 0;
                }
                let stop = it.base_level_stop;
                self.handle_stop_backwards(it, stop)?;
                return self.get_next_element_by_method(it);
            }
        }
        if it.current.overlay_string_index >= 0 {
            let schars = self.string_chars(it.string) as i64;
            if it.string_charpos() >= schars {
                it.what = DisplayElementType::Eob;
                return Ok(false);
            }
            let end = if it.bidi_it.scan_dir < 0 { -1 } else { schars };
            let charpos = it.string_charpos();
            if self.char_composed_p(it, charpos, end)? && self.next_element_from_composition(it)? {
                return Ok(true);
            }
            self.string_char_at_it(it);
        } else {
            if it.string_charpos() >= it.end_charpos {
                it.what = DisplayElementType::Eob;
                return Ok(false);
            } else if it.string_charpos() >= it.string_nchars {
                it.c = 32;
                it.len = 1;
                position = TextPos::new(-1, -1);
            } else {
                let end = if it.bidi_it.scan_dir < 0 {
                    -1
                } else {
                    it.string_nchars
                };
                let charpos = it.string_charpos();
                if self.char_composed_p(it, charpos, end)?
                    && self.next_element_from_composition(it)?
                {
                    return Ok(true);
                }
                self.string_char_at_it(it);
            }
        }
        it.what = DisplayElementType::Character;
        it.object = it.string;
        it.position = position;
        Ok(true)
    }

    fn string_char_at_it(&self, it: &mut It) {
        let bytes = self.string_bytes(it.string);
        let pos = it.string_bytepos() as usize;
        if self.string_multibyte(it.string) {
            let (c, len) = character::string_char(&bytes[pos..]);
            it.c = c as i32;
            it.len = len as i32;
        } else {
            it.c = i32::from(bytes[pos]);
            it.len = 1;
        }
    }

    pub(super) fn next_element_from_c_string(&mut self, it: &mut It) -> LispResult<bool> {
        let mut success_p = true;
        it.what = DisplayElementType::Character;
        it.position = TextPos::new(0, 0);
        it.object = Value::fixnum(0);
        if it.bidi_p && it.bidi_it.first_elt {
            self.get_visually_first_element(it)?;
        }
        if it.charpos() >= it.end_charpos {
            it.what = DisplayElementType::Eob;
            success_p = false;
        } else if it.charpos() >= it.string_nchars {
            it.c = 32;
            it.len = 1;
            it.position = TextPos::new(-1, -1);
        } else {
            let s = it.s.clone().expect("C string");
            let b = it.bytepos() as usize;
            if it.multibyte_p {
                let (c, len) = character::string_char(&s[b..]);
                it.c = c as i32;
                it.len = len as i32;
            } else {
                it.c = i32::from(s[b]);
                it.len = 1;
            }
        }
        Ok(success_p)
    }

    fn next_element_from_ellipsis(&mut self, it: &mut It) -> LispResult<bool> {
        if it.selective_display_ellipsis_p {
            let len = it.len;
            self.setup_for_ellipsis(it, len);
        } else {
            it.saved_face_id = it.face_id;
            it.method = ItMethod::FromBuffer;
            it.object = self.window(it.window).contents;
            self.reseat_at_next_visible_line_start(it, true)?;
            it.face_before_selective_p = true;
        }
        self.get_next_element_by_method(it)
    }

    pub(super) fn next_element_from_image(&mut self, it: &mut It) -> LispResult<bool> {
        it.what = DisplayElementType::Image;
        Ok(true)
    }

    pub(super) fn next_element_from_stretch(&mut self, it: &mut It) -> LispResult<bool> {
        it.what = DisplayElementType::Stretch;
        Ok(true)
    }

    fn compute_stop_pos_backwards(&mut self, it: &mut It) -> LispResult<()> {
        const SCAN_BACK_LIMIT: i64 = 1000;
        let save_current = it.current;
        let save_position = it.position;
        let mut charpos = it.charpos();
        let where_we_are = charpos;
        let save_stop_pos = it.stop_charpos;
        let save_end_pos = it.end_charpos;
        it.bidi_p = false;
        let begv = self.cur().begv as i64;
        loop {
            it.end_charpos = (charpos + 1).min(self.cur().zv as i64);
            charpos = (charpos - SCAN_BACK_LIMIT).max(begv);
            let pos = TextPos::new(
                charpos,
                self.cur().text.char_to_byte(charpos as usize) as i64,
            );
            self.reseat_1(it, pos, false)?;
            self.compute_stop_pos(it)?;
            if it.stop_charpos <= charpos {
                std::process::abort();
            }
            if !(charpos > begv && it.stop_charpos >= it.end_charpos) {
                break;
            }
        }
        it.prev_stop = if it.stop_charpos <= where_we_are {
            it.stop_charpos
        } else {
            begv
        };
        it.bidi_p = true;
        it.current = save_current;
        it.position = save_position;
        it.stop_charpos = save_stop_pos;
        it.end_charpos = save_end_pos;
        Ok(())
    }

    fn handle_stop_backwards(&mut self, it: &mut It, charpos: i64) -> LispResult<()> {
        let mut charpos = charpos;
        let bufp = !it.string.is_string();
        let where_we_are = if bufp {
            it.charpos()
        } else {
            it.string_charpos()
        };
        let save_current = it.current;
        let save_position = it.position;
        let save_cmp_it = it.cmp_it;
        let save_sp = it.sp;
        it.bidi_p = false;
        loop {
            it.prev_stop = charpos;
            if bufp {
                let pos1 = TextPos::new(
                    charpos,
                    self.cur().text.char_to_byte(charpos as usize) as i64,
                );
                self.reseat_1(it, pos1, false)?;
            } else {
                it.current.string_pos = self.string_pos(charpos, it.string);
            }
            self.compute_stop_pos(it)?;
            if it.stop_charpos <= it.prev_stop {
                std::process::abort();
            }
            charpos = it.stop_charpos;
            if charpos > where_we_are {
                break;
            }
        }
        it.bidi_p = true;
        it.current = save_current;
        it.position = save_position;
        it.cmp_it = save_cmp_it;
        let next_stop = it.stop_charpos;
        it.stop_charpos = it.prev_stop;
        self.handle_stop(it)?;
        if it.sp == save_sp {
            it.stop_charpos = next_stop;
        }
        Ok(())
    }

    pub(super) fn next_element_from_buffer(&mut self, it: &mut It) -> LispResult<bool> {
        let mut success_p = true;
        if it.bidi_p && it.bidi_it.first_elt {
            self.get_visually_first_element(it)?;
            it.position = TextPos::new(it.charpos(), it.bytepos());
        }
        if it.charpos() >= it.stop_charpos {
            if it.charpos() >= it.end_charpos {
                let overlay_strings_follow_p = if it.overlay_strings_at_end_processed_p {
                    false
                } else {
                    it.overlay_strings_at_end_processed_p = true;
                    self.get_overlay_strings(it, 0)?
                };
                if overlay_strings_follow_p {
                    success_p = self.get_next_element_by_method(it)?;
                } else {
                    it.what = DisplayElementType::Eob;
                    it.position = it.current.pos;
                    success_p = false;
                }
            } else if !(!it.bidi_p || it.bidi_it.at_base_level() || it.charpos() == it.stop_charpos)
            {
                let stop = it.stop_charpos;
                self.handle_stop_backwards(it, stop)?;
                it.ignore_overlay_strings_at_pos_p = false;
                return self.get_next_element_by_method(it);
            } else {
                if it.bidi_p {
                    it.prev_stop = it.stop_charpos;
                    if it.bidi_it.at_base_level() {
                        it.base_level_stop = it.stop_charpos;
                    }
                }
                self.handle_stop(it)?;
                it.ignore_overlay_strings_at_pos_p = false;
                return self.get_next_element_by_method(it);
            }
        } else if it.bidi_p && it.charpos() < it.prev_stop && !it.bidi_it.at_base_level() {
            if it.base_level_stop <= 0 || it.charpos() < it.base_level_stop {
                it.base_level_stop = self.cur().begv as i64;
                self.compute_stop_pos_backwards(it)?;
                let prev = it.prev_stop;
                self.handle_stop_backwards(it, prev)?;
            } else {
                let stop = it.base_level_stop;
                self.handle_stop_backwards(it, stop)?;
            }
            it.ignore_overlay_strings_at_pos_p = false;
            return self.get_next_element_by_method(it);
        } else {
            it.ignore_overlay_strings_at_pos_p = false;
            let pt = self.cur().pt as i64;
            let stop = if self.var_true(sym::COMPOSITION_BREAK_AT_POINT)
                && !self.plain_value(sym::ENABLE_MULTIBYTE_CHARACTERS).is_nil()
                && !self.find_symbol_value(sym::AUTO_COMPOSITION_MODE).is_nil()
            {
                if it.bidi_it.scan_dir < 0 {
                    if pt <= it.charpos() { pt } else { -1 }
                } else if it.charpos() < pt && pt < it.end_charpos {
                    pt
                } else {
                    it.end_charpos
                }
            } else if it.bidi_it.scan_dir < 0 {
                self.bidi_level_start(i32::from(it.bidi_it.resolved_level)) - 1
            } else {
                it.end_charpos
            };
            let charpos = it.charpos();
            if self.char_composed_p(it, charpos, stop)? && self.next_element_from_composition(it)? {
                return Ok(true);
            }
            let b = it.bytepos() as usize;
            let first = self.cur().text.byte_at(b);
            if it.multibyte_p && !character::ascii_char_p(u32::from(first)) {
                it.c = self.cur().text.char_at_byte(b) as i32;
                it.len = character::bytes_by_char_head(first) as i32;
            } else {
                it.c = i32::from(first);
                it.len = 1;
            }
            it.what = DisplayElementType::Character;
            it.object = self.window(it.window).contents;
            it.position = it.current.pos;
            if it.selective != 0 {
                if it.c == 10 {
                    let zv = self.cur().zv as i64;
                    if it.selective > 0
                        && it.charpos() + 1 < zv
                        && self.indented_beyond_p(it.charpos() + 1, it.selective)?
                    {
                        success_p = self.next_element_from_ellipsis(it)?;
                        it.dpvec_char_len = -1;
                    }
                } else if it.c == 13 && it.selective == -1 {
                    success_p = self.next_element_from_ellipsis(it)?;
                    it.dpvec_char_len = -1;
                }
            }
        }
        Ok(success_p)
    }

    fn next_element_from_composition(&mut self, it: &mut It) -> LispResult<bool> {
        it.what = DisplayElementType::Composition;
        it.len = it.cmp_it.nbytes as i32;
        if it.string.is_string() {
            if it.c < 0 {
                it.current.string_pos.charpos += it.cmp_it.nchars;
                it.current.string_pos.bytepos += it.cmp_it.nbytes;
                return Ok(false);
            }
            it.position = it.current.string_pos;
            it.object = it.string;
            let (charpos, bytepos, string) = (it.string_charpos(), it.string_bytepos(), it.string);
            it.c = self.composition_update_it(&mut it.cmp_it, charpos, bytepos, string) as i32;
        } else {
            if it.c < 0 {
                it.current.pos.charpos += it.cmp_it.nchars;
                it.current.pos.bytepos += it.cmp_it.nbytes;
                if it.bidi_p {
                    if it.bidi_it.new_paragraph {
                        let pe = it.paragraph_embedding;
                        self.bidi_paragraph_init(pe, &mut it.bidi_it, false)?;
                    }
                    while it.bidi_it.charpos < it.charpos() {
                        self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                    }
                }
                return Ok(false);
            }
            it.position = it.current.pos;
            it.object = self.window(it.window).contents;
            let (charpos, bytepos) = (it.charpos(), it.bytepos());
            it.c = self.composition_update_it(&mut it.cmp_it, charpos, bytepos, Value::NIL) as i32;
        }
        Ok(true)
    }
}

fn unwind_narrowed_begv_elements(lisp: &mut Lisp, point_min: Value) -> LispResult<()> {
    let begv = point_min.as_fixnum().unwrap_or(1) as usize;
    lisp.set_buf_begv(begv);
    Ok(())
}
