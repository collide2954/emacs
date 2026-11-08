use crate::bidi::BidiDir;
use crate::dispextern::{
    DisplayElementType, GlyphRowArea, It, ItMethod, LineWrap, PropHandled, TextPos,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::xfaces::{DEFAULT_FACE_ID, FRINGE_FACE_ID, LFACE_HEIGHT_INDEX, MARGIN_FACE_ID};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpecPos {
    Buffer,
    String,
}

impl Lisp {
    pub fn compute_string_pos(&self, newpos: TextPos, _pos: TextPos, string: Value) -> TextPos {
        if self.string_multibyte(string) {
            let bytepos = self.string_char_to_byte_index(string, newpos.charpos as usize);
            TextPos::new(newpos.charpos, bytepos as i64)
        } else {
            TextPos::new(newpos.charpos, newpos.charpos)
        }
    }

    pub(super) fn handle_display_prop(&mut self, it: &mut It) -> LispResult<PropHandled> {
        let (mut object, which, bufpos) = if it.string.is_string() {
            (it.string, SpecPos::String, it.charpos())
        } else {
            (it.window, SpecPos::Buffer, it.charpos())
        };
        it.slice = crate::dispextern::ItSlice::default();
        it.space_width = Value::NIL;
        it.font_height = Value::NIL;
        it.voffset = 0;
        if !it.string_from_display_prop_p {
            it.area = GlyphRowArea::Text;
        }
        let position = match which {
            SpecPos::String => it.current.string_pos,
            SpecPos::Buffer => it.current.pos,
        };
        let (propval, overlay) = self.get_char_property_and_overlay(
            Value::fixnum(position.charpos),
            sym::DISPLAY,
            object,
        )?;
        let objwin = object;
        if !it.string.is_string() {
            object = self.window(it.window).contents;
        }
        if !it.min_width_property.is_nil()
            && self.find_display_property(propval, sym::MIN_WIDTH).is_nil()
        {
            let (pos, start) = if object.is_string() {
                (it.string_charpos(), 0)
            } else {
                (bufpos, self.cur().begv as i64)
            };
            if pos > start || (object.is_string() && it.sp == 0) {
                self.display_min_width(it, pos, objwin, Value::NIL)?;
            }
        }
        if propval.is_nil() {
            return Ok(PropHandled::Normally);
        }
        let fwp = self.frame_window_p(it.f);
        let base = self.stack.len();
        self.stack.extend_from_slice(&[propval, overlay, object]);
        let r = self.handle_display_spec_it(it, propval, object, overlay, which, bufpos, fwp);
        self.stack.truncate(base);
        let display_replaced = r?;
        Ok(if display_replaced != 0 {
            PropHandled::Return
        } else {
            PropHandled::Normally
        })
    }

    fn spec_position(it: &It, which: SpecPos) -> TextPos {
        match which {
            SpecPos::String => it.current.string_pos,
            SpecPos::Buffer => it.current.pos,
        }
    }

    fn store_spec_position(it: &mut It, which: SpecPos, position: TextPos) {
        match which {
            SpecPos::String => it.current.string_pos = position,
            SpecPos::Buffer => it.current.pos = position,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_display_spec_it(
        &mut self,
        it: &mut It,
        mut spec: Value,
        object: Value,
        overlay: Value,
        which: SpecPos,
        bufpos: i64,
        frame_window_p: bool,
    ) -> LispResult<i32> {
        let mut replacing = 0;
        let mut enable_eval = true;
        if spec.is_cons() && self.xcar(spec) == sym::DISABLE_EVAL {
            enable_eval = false;
            let rest = self.xcdr(spec);
            spec = if rest.is_cons() {
                self.xcar(rest)
            } else {
                Value::NIL
            };
        }
        let simple_list = spec.is_cons() && {
            let head = self.xcar(spec);
            ![
                sym::IMAGE,
                sym::SPACE,
                sym::WHEN,
                sym::SLICE,
                sym::SPACE_WIDTH,
                sym::HEIGHT,
                sym::RAISE,
                sym::LEFT_FRINGE,
                sym::RIGHT_FRINGE,
                sym::MIN_WIDTH,
            ]
            .contains(&head)
                && !(head.is_cons() && self.xcar(head) == sym::MARGIN)
                && !head.is_nil()
        };
        if simple_list {
            let mut tail = spec;
            while tail.is_cons() {
                let item = self.xcar(tail);
                let rv = self.handle_single_display_spec_it(
                    it,
                    item,
                    object,
                    overlay,
                    which,
                    bufpos,
                    replacing,
                    frame_window_p,
                    enable_eval,
                )?;
                if rv != 0 {
                    replacing = rv;
                    if object.is_string() {
                        break;
                    }
                }
                tail = self.xcdr(tail);
            }
        } else if self.is_vector(spec) {
            let mut i = 0;
            while i < self.vector_items(spec).len() {
                let item = self.vector_items(spec)[i];
                let rv = self.handle_single_display_spec_it(
                    it,
                    item,
                    object,
                    overlay,
                    which,
                    bufpos,
                    replacing,
                    frame_window_p,
                    enable_eval,
                )?;
                if rv != 0 {
                    replacing = rv;
                    if object.is_string() {
                        break;
                    }
                }
                i += 1;
            }
        } else {
            replacing = self.handle_single_display_spec_it(
                it,
                spec,
                object,
                overlay,
                which,
                bufpos,
                0,
                frame_window_p,
                enable_eval,
            )?;
        }
        Ok(replacing)
    }

    fn display_prop_end(
        &mut self,
        it: &It,
        object: Value,
        start_pos: TextPos,
    ) -> LispResult<TextPos> {
        let end = crate::textprop::f_next_single_char_property_change(
            self,
            &[
                Value::fixnum(start_pos.charpos),
                sym::DISPLAY,
                object,
                Value::NIL,
            ],
        )?;
        let charpos = end.as_fixnum().unwrap_or(0);
        Ok(if object.is_string() {
            self.compute_string_pos(TextPos::new(charpos, 0), start_pos, it.string)
        } else {
            TextPos::new(
                charpos,
                self.cur().text.char_to_byte(charpos as usize) as i64,
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_single_display_spec_it(
        &mut self,
        it: &mut It,
        mut spec: Value,
        object: Value,
        overlay: Value,
        which: SpecPos,
        bufpos: i64,
        display_replaced: i32,
        _frame_window_p: bool,
        enable_eval_p: bool,
    ) -> LispResult<i32> {
        let mut form = sym::T;
        if spec.is_cons() && self.xcar(spec) == sym::WHEN {
            spec = self.xcdr(spec);
            if !spec.is_cons() {
                return Ok(0);
            }
            form = self.xcar(spec);
            spec = self.xcdr(spec);
        }
        if !form.is_nil() && form != sym::T && !enable_eval_p {
            form = Value::NIL;
        }
        if !form.is_nil() && form != sym::T {
            let count = self.specpdl.len();
            let object = if object.is_nil() {
                self.current_buffer
            } else {
                object
            };
            self.specbind(sym::OBJECT, object)?;
            self.specbind(
                sym::POSITION,
                Value::fixnum(Self::spec_position(it, which).charpos),
            )?;
            self.specbind(sym::BUFFER_POSITION, Value::fixnum(bufpos))?;
            let itdata = self.bidi_shelve_cache();
            let base = self.stack.len();
            self.stack.push(spec);
            it.trace(&mut self.stack);
            let r = self.dsafe_eval(form);
            self.stack.truncate(base);
            self.bidi_unshelve_cache(itdata, false);
            form = self.unbind_to_value(count, r)?;
        }
        if form.is_nil() {
            return Ok(0);
        }
        let head = self.car_safe(spec);
        let rest = self.cdr_safe(spec);
        if spec.is_cons() && head == sym::HEIGHT && rest.is_cons() {
            if !self.frame_window_p(it.f) {
                return Ok(0);
            }
            it.font_height = self.xcar(rest);
            if !it.font_height.is_nil() {
                self.display_spec_height(it, enable_eval_p)?;
            }
            return Ok(0);
        }
        if spec.is_cons() && head == sym::SPACE_WIDTH && rest.is_cons() {
            if !self.frame_window_p(it.f) {
                return Ok(0);
            }
            let value = self.xcar(rest);
            if self.is_number(value) && self.xfloatint(value) > 0.0 {
                it.space_width = value;
            }
            return Ok(0);
        }
        if spec.is_cons() && head == sym::MIN_WIDTH && rest.is_cons() && self.xcar(rest).is_cons() {
            let pos = if object.is_string() {
                it.string_charpos()
            } else {
                bufpos
            };
            let width_spec = self.xcar(rest);
            self.display_min_width(it, pos, object, width_spec)?;
            return Ok(0);
        }
        if spec.is_cons() && head == sym::SLICE {
            if !self.frame_window_p(it.f) {
                return Ok(0);
            }
            let mut tem = rest;
            if tem.is_cons() {
                it.slice.x = self.xcar(tem);
                tem = self.xcdr(tem);
                if tem.is_cons() {
                    it.slice.y = self.xcar(tem);
                    tem = self.xcdr(tem);
                    if tem.is_cons() {
                        it.slice.width = self.xcar(tem);
                        tem = self.xcdr(tem);
                        if tem.is_cons() {
                            it.slice.height = self.xcar(tem);
                        }
                    }
                }
            }
            return Ok(0);
        }
        if spec.is_cons() && head == sym::RAISE && rest.is_cons() {
            if !self.frame_window_p(it.f) {
                return Ok(0);
            }
            let value = self.xcar(rest);
            if self.is_number(value) {
                let factor = self.xfloatint(value);
                let font = self
                    .face_from_id(it.f, it.face_id as usize)
                    .expect("face is not realized")
                    .font;
                let height = self.normal_char_height(font, -1);
                it.voffset = (-(factor * f64::from(height))) as i16;
            }
            return Ok(0);
        }
        if it.string_from_display_prop_p {
            return Ok(0);
        }
        let start_pos = Self::spec_position(it, which);
        if !overlay.is_nil() {
            let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
            let ovendpos = (self.overlay(overlay).end as i64).clamp(begv, zv);
            let end = TextPos::new(
                ovendpos,
                self.cur().text.char_to_byte(ovendpos as usize) as i64,
            );
            Self::store_spec_position(it, which, end);
        } else {
            let end = self.display_prop_end(it, object, start_pos)?;
            Self::store_spec_position(it, which, end);
        }
        it.stop_charpos = Self::spec_position(it, which).charpos;
        if spec.is_cons()
            && (head == sym::LEFT_FRINGE || head == sym::RIGHT_FRINGE)
            && rest.is_cons()
        {
            if !self.frame_window_p(it.f) {
                if it.bidi_p {
                    it.position = Self::spec_position(it, which);
                    self.iterate_out_of_display_property(it)?;
                    let p = it.position;
                    Self::store_spec_position(it, which, p);
                }
                return Ok(1);
            }
            let value = self.xcar(rest);
            let fringe_bitmap = if value.is_symbol() {
                self.lookup_fringe_bitmap(value)
            } else {
                0
            };
            if fringe_bitmap == 0 {
                if it.bidi_p {
                    it.position = Self::spec_position(it, which);
                    self.iterate_out_of_display_property(it)?;
                    let p = it.position;
                    Self::store_spec_position(it, which, p);
                }
                return Ok(1);
            }
            let mut face_id = self.lookup_basic_face(Some(it.window), it.f, DEFAULT_FACE_ID)?;
            let rest2 = self.xcdr(rest);
            if rest2.is_cons() {
                let face_name = self.xcar(rest2);
                let count1 = self.specpdl.len();
                self.specbind(sym::INHIBIT_QUIT, sym::T)?;
                let r = self.lookup_derived_face(
                    Some(it.window),
                    it.f,
                    face_name,
                    FRINGE_FACE_ID,
                    false,
                );
                self.unbind_to(count1)?;
                let face_id2 = r?;
                if face_id2 >= 0 {
                    face_id = face_id2 as usize;
                }
            }
            let p = Self::spec_position(it, which);
            self.push_it(it, Some(p))?;
            it.area = GlyphRowArea::Text;
            it.what = DisplayElementType::Image;
            it.image_id = -1;
            it.position = start_pos;
            it.object = if object.is_nil() {
                self.window(it.window).contents
            } else {
                object
            };
            it.method = ItMethod::FromImage;
            it.from_overlay = Value::NIL;
            it.face_id = face_id as i32;
            it.from_disp_prop_p = true;
            Self::store_spec_position(it, which, start_pos);
            if head == sym::LEFT_FRINGE {
                it.left_user_fringe_bitmap = fringe_bitmap as u32;
                it.left_user_fringe_face_id = face_id as u32;
            } else {
                it.right_user_fringe_bitmap = fringe_bitmap as u32;
                it.right_user_fringe_face_id = face_id as u32;
            }
            return Ok(1);
        }
        let mut location = sym::UNBOUND;
        let mut value = Value::NIL;
        if spec.is_cons() && head.is_cons() {
            value = rest;
            if value.is_cons() {
                value = self.xcar(value);
            }
            if self.xcar(head) == sym::MARGIN {
                let tem = self.xcdr(head);
                let tem = if tem.is_cons() {
                    self.xcar(tem)
                } else {
                    Value::NIL
                };
                if tem.is_nil() || tem == sym::LEFT_MARGIN || tem == sym::RIGHT_MARGIN {
                    location = tem;
                }
            }
        }
        if location == sym::UNBOUND {
            location = Value::NIL;
            value = spec;
        }
        let fwp = self.frame_window_p(it.f);
        let space = value.is_cons() && self.xcar(value) == sym::SPACE;
        let valid_p = value.is_string() || (fwp && self.valid_image_p(value)) || space;
        if valid_p && display_replaced == 0 {
            let mut retval = 1;
            let eob = if self.is_buffer(object) {
                self.cur().zv as i64
            } else {
                it.end_charpos
            };
            if it.bidi_it.first_elt && it.bidi_it.charpos < eob {
                let pe = it.paragraph_embedding;
                self.bidi_paragraph_init(pe, &mut it.bidi_it, true)?;
            }
            let p = Self::spec_position(it, which);
            self.push_it(it, Some(p))?;
            it.from_overlay = overlay;
            it.from_disp_prop_p = true;
            if location.is_nil() {
                it.area = GlyphRowArea::Text;
            } else {
                it.area = if location == sym::LEFT_MARGIN {
                    GlyphRowArea::LeftMargin
                } else {
                    GlyphRowArea::RightMargin
                };
                it.face_id = if self.find_symbol_value(sym::FACE_REMAPPING_ALIST).is_nil() {
                    MARGIN_FACE_ID as i32
                } else {
                    self.lookup_basic_face(Some(it.window), it.f, MARGIN_FACE_ID)? as i32
                };
            }
            if value.is_string() {
                it.string = value;
                it.multibyte_p = self.string_multibyte(value);
                it.current.overlay_string_index = -1;
                it.current.string_pos = TextPos::new(0, 0);
                it.end_charpos = self.string_chars(value) as i64;
                it.string_nchars = it.end_charpos;
                it.method = ItMethod::FromString;
                it.stop_charpos = 0;
                it.prev_stop = 0;
                it.base_level_stop = 0;
                it.string_from_display_prop_p = true;
                it.cmp_it.id = -1;
                if self.is_buffer(object) {
                    Self::store_spec_position(it, which, start_pos);
                }
                it.paragraph_embedding = if it.bidi_p && it.bidi_it.paragraph_dir == BidiDir::R2L {
                    it.bidi_it.paragraph_dir
                } else {
                    BidiDir::L2R
                };
                if it.bidi_p {
                    it.bidi_it.string.lstring = it.string;
                    it.bidi_it.string.s = None;
                    it.bidi_it.string.schars = it.end_charpos;
                    it.bidi_it.string.bufpos = bufpos;
                    it.bidi_it.string.from_disp_str = true;
                    it.bidi_it.string.unibyte = !it.multibyte_p;
                    it.bidi_it.w = it.window;
                    self.bidi_init_it(0, 0, fwp, &mut it.bidi_it)?;
                }
            } else if space {
                it.method = ItMethod::FromStretch;
                it.object = value;
                Self::store_spec_position(it, which, start_pos);
                it.position = start_pos;
                retval = 1 + i32::from(it.area == GlyphRowArea::Text);
            } else {
                unreachable!("image display specs need image.c's lookup_image");
            }
            return Ok(retval);
        }
        Self::store_spec_position(it, which, start_pos);
        Ok(0)
    }

    fn face_lface(&self, f: Value, face_id: usize, index: usize) -> Value {
        self.face_from_id(f, face_id)
            .expect("face is not realized")
            .lface[index]
    }

    fn display_spec_height(&mut self, it: &mut It, enable_eval_p: bool) -> LispResult<()> {
        let mut new_height = -1i64;
        let fh = it.font_height;
        let head = self.car_safe(fh);
        let rest = self.cdr_safe(fh);
        let steps_arg = self.car_safe(rest);
        if fh.is_cons()
            && (head == sym::PLUS || head == sym::MINUS)
            && rest.is_cons()
            && matches!(steps_arg.as_fixnum(), Some(n) if (0..=i64::from(i32::MAX)).contains(&n))
        {
            let mut steps = steps_arg.as_fixnum().unwrap_or(0);
            if head == sym::PLUS {
                steps = -steps;
            }
            it.face_id = self.smaller_face(it.f, it.face_id as usize, steps)? as i32;
        } else if self.functionp(fh) && enable_eval_p {
            let height_attr = self.face_lface(it.f, it.face_id as usize, LFACE_HEIGHT_INDEX);
            let itdata = self.bidi_shelve_cache();
            let base = self.stack.len();
            it.trace(&mut self.stack);
            let r = self.safe_call(false, &[fh, height_attr]);
            self.stack.truncate(base);
            self.bidi_unshelve_cache(itdata, false);
            let height = r?;
            if self.is_number(height) {
                new_height = self.xfloatint(height) as i64;
            }
        } else if self.is_number(fh) {
            let default_id = self.lookup_basic_face(Some(it.window), it.f, DEFAULT_FACE_ID)?;
            let base_height = self
                .face_lface(it.f, default_id, LFACE_HEIGHT_INDEX)
                .as_fixnum()
                .unwrap_or(0);
            new_height = (self.xfloatint(fh) * base_height as f64) as i64;
        } else if enable_eval_p {
            let count = self.specpdl.len();
            let height_attr = self.face_lface(it.f, it.face_id as usize, LFACE_HEIGHT_INDEX);
            self.specbind(sym::HEIGHT, height_attr)?;
            let itdata = self.bidi_shelve_cache();
            let base = self.stack.len();
            it.trace(&mut self.stack);
            let r = self.dsafe_eval(fh);
            self.stack.truncate(base);
            self.bidi_unshelve_cache(itdata, false);
            let value = self.unbind_to_value(count, r)?;
            if self.is_number(value) {
                new_height = self.xfloatint(value) as i64;
            }
        }
        if new_height > 0 {
            it.face_id = self.face_with_height(it.f, it.face_id as usize, new_height)? as i32;
        }
        Ok(())
    }

    fn single_display_spec_string_p(&self, mut prop: Value, string: Value) -> bool {
        if string == prop {
            return true;
        }
        if prop.is_cons() && self.xcar(prop) == sym::WHEN {
            prop = self.xcdr(prop);
            if !prop.is_cons() {
                return false;
            }
            prop = self.xcdr(prop);
        }
        if prop.is_cons() && self.xcar(prop) == sym::MARGIN {
            prop = self.xcdr(prop);
            if !prop.is_cons() {
                return false;
            }
            prop = self.xcdr(prop);
            if !prop.is_cons() {
                return false;
            }
        }
        prop == string || (prop.is_cons() && self.xcar(prop) == string)
    }

    pub fn display_prop_string_p(&self, prop: Value, string: Value) -> bool {
        if prop.is_cons()
            && self.xcar(prop) != sym::WHEN
            && !(self.xcar(prop).is_cons() && self.xcar(self.xcar(prop)) == sym::MARGIN)
        {
            let mut prop = prop;
            while prop.is_cons() {
                if self.single_display_spec_string_p(self.xcar(prop), string) {
                    return true;
                }
                prop = self.xcdr(prop);
            }
            false
        } else if self.is_vector(prop) {
            self.vector_items(prop)
                .iter()
                .any(|&p| self.single_display_spec_string_p(p, string))
        } else {
            self.single_display_spec_string_p(prop, string)
        }
    }

    pub(super) fn string_buffer_position_lim(
        &mut self,
        string: Value,
        from: i64,
        to: i64,
        back_p: bool,
    ) -> LispResult<i64> {
        let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
        let mut pos = Value::fixnum(from.max(begv));
        let mut found = false;
        let limit = if back_p {
            Value::fixnum(to.max(begv))
        } else {
            Value::fixnum(to.min(zv))
        };
        while !found && pos != limit {
            let prop = self.get_char_property(pos, sym::DISPLAY, Value::NIL)?;
            if !prop.is_nil() && self.display_prop_string_p(prop, string) {
                found = true;
            } else if back_p {
                pos = crate::textprop::f_previous_single_char_property_change(
                    self,
                    &[pos, sym::DISPLAY, Value::NIL, limit],
                )?;
            } else {
                pos = crate::textprop::f_next_single_char_property_change(
                    self,
                    &[pos, sym::DISPLAY, Value::NIL, limit],
                )?;
            }
        }
        Ok(if found {
            pos.as_fixnum().unwrap_or(0)
        } else {
            0
        })
    }

    pub fn string_buffer_position(
        &mut self,
        string: Value,
        around_charpos: i64,
    ) -> LispResult<i64> {
        const MAX_DISTANCE: i64 = 1000;
        let forward_limit = (around_charpos + MAX_DISTANCE).min(self.cur().zv as i64);
        let mut found =
            self.string_buffer_position_lim(string, around_charpos, forward_limit, false)?;
        if found == 0 {
            let backward_limit = (around_charpos - MAX_DISTANCE).max(self.cur().begv as i64);
            found =
                self.string_buffer_position_lim(string, around_charpos, backward_limit, true)?;
        }
        Ok(found)
    }

    pub(super) fn handle_composition_prop(&mut self, it: &mut It) -> LispResult<PropHandled> {
        let (pos, pos_byte, string);
        if it.string.is_string() {
            pos = it.string_charpos();
            pos_byte = it.string_bytepos();
            string = it.string;
            let bytes = self.string_bytes(string);
            it.c = if self.string_multibyte(string) {
                crate::character::string_char(&bytes[pos_byte as usize..]).0 as i32
            } else {
                i32::from(bytes[pos_byte as usize])
            };
        } else {
            pos = it.charpos();
            pos_byte = it.bytepos();
            string = Value::NIL;
            it.c = self.cur().text.char_at_byte(pos_byte as usize) as i32;
        }
        if let Some((start, end, prop)) = self.find_composition(pos, -1, string)?
            && self.composition_valid_p(start, end, prop)
            && (it.string.is_string() || {
                let pt = self.cur().pt;
                pt <= start || pt >= end
            })
        {
            let (start, end) = (start as i64, end as i64);
            if start < pos {
                return Ok(PropHandled::Normally);
            }
            it.cmp_it.id =
                self.get_composition_id(start as usize, (end - start) as usize, prop, string)?;
            if it.cmp_it.id >= 0 {
                it.cmp_it.ch = -1;
                it.cmp_it.nchars = self.composition_length(prop).unwrap_or(0);
                it.cmp_it.nglyphs = -1;
            }
        }
        Ok(PropHandled::Normally)
    }

    pub(super) fn handle_overlay_change(&mut self, it: &mut It) -> LispResult<PropHandled> {
        if !it.string.is_string() && self.get_overlay_strings(it, 0)? {
            Ok(PropHandled::RecomputeProps)
        } else {
            Ok(PropHandled::Normally)
        }
    }

    pub fn display_min_width(
        &mut self,
        it: &mut It,
        charpos: i64,
        object: Value,
        width_spec: Value,
    ) -> LispResult<()> {
        if !it.min_width_property.is_nil() && width_spec != it.min_width_property {
            let begv = self.cur().begv as i64;
            let at_end = if object.is_string() {
                (charpos == 0
                    && it.min_width_property
                        != self.get_display_property(0, sym::MIN_WIDTH, object)?)
                    || (charpos > 0
                        && it.min_width_property
                            == self.get_display_property(charpos - 1, sym::MIN_WIDTH, object)?)
            } else {
                charpos > begv
                    && it.min_width_property
                        == self.get_display_property(charpos - 1, sym::MIN_WIDTH, object)?
            };
            if at_end {
                let mut w = Value::NIL;
                let spec = self.xcar(it.min_width_property);
                if self.frame_window_p(it.f) {
                    let face_font = self
                        .face_from_id(it.f, it.face_id as usize)
                        .expect("face is not realized")
                        .font;
                    let font = if face_font.is_nil() {
                        self.frame(it.f).font
                    } else {
                        face_font
                    };
                    let mut width = self
                        .calc_pixel_width_or_height(it, spec, font, true, None, None)?
                        .unwrap_or(0.0);
                    width -= f64::from(it.current_x - it.min_width_start);
                    if width > 0.0
                        && !(it.line_wrap == LineWrap::Truncate
                            && f64::from(it.current_x) + width > f64::from(it.last_visible_x))
                    {
                        w = self.list(&[Value::fixnum(width as i64)]);
                    }
                } else {
                    let mut width = self
                        .calc_pixel_width_or_height(it, spec, Value::NIL, true, None, None)?
                        .unwrap_or(0.0);
                    width -= f64::from(
                        (it.current_x - it.min_width_start)
                            / self.frame(it.f).geometry.column_width as i32,
                    );
                    if width > 0.0
                        && !(it.line_wrap == LineWrap::Truncate
                            && f64::from(it.current_x) + width > f64::from(it.last_visible_x))
                    {
                        w = Value::fixnum(width as i64);
                    }
                }
                if !w.is_nil() {
                    it.object = self.list(&[sym::SPACE, sym::K_WIDTH, w]);
                    self.produce_stretch_glyph(it)?;
                    if it.area == GlyphRowArea::Text {
                        it.current_x += it.pixel_width;
                        if it.continuation_lines_width != 0 && it.string_from_prefix_prop_p {
                            it.wrap_prefix_width = it.current_x;
                        }
                    }
                }
                it.min_width_property = Value::NIL;
            }
        }
        if width_spec.is_cons() {
            let begv = self.cur().begv as i64;
            let start = (!object.is_string() && charpos == begv)
                || (object.is_string()
                    && charpos == 0
                    && it.min_width_property
                        != self.get_display_property(0, sym::MIN_WIDTH, object)?)
                || (!object.is_string()
                    && charpos > begv
                    && width_spec
                        != self.get_display_property(charpos - 1, sym::MIN_WIDTH, object)?);
            if start {
                it.min_width_property = width_spec;
                it.min_width_start = it.current_x;
            }
        }
        Ok(())
    }
}
