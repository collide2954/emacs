use crate::bidi::{BidiDir, ShelvedBidiCache};
use crate::dispextern::{
    DisplayElementType, DisplayPos, DpVec, GlyphRowArea, GlyphRowRef, It, ItMethod, LineWrap,
    MOVE_TO_POS, MOVE_TO_X, OVERLAY_STRING_CHUNK_SIZE, PropHandled, TEXT_PROP_DISTANCE_LIMIT,
    TextPos,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::xfaces::{
    DEFAULT_FACE_ID, HEADER_LINE_ACTIVE_FACE_ID, HEADER_LINE_INACTIVE_FACE_ID,
    MODE_LINE_ACTIVE_FACE_ID, MODE_LINE_INACTIVE_FACE_ID, TAB_LINE_ACTIVE_FACE_ID,
    TAB_LINE_INACTIVE_FACE_ID,
};

const DISP_INVIS_VECTOR_SLOT: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PropKind {
    Fontified,
    Face,
    Display,
    Invisible,
    Composition,
}

const IT_PROPS: [PropKind; 5] = [
    PropKind::Fontified,
    PropKind::Face,
    PropKind::Display,
    PropKind::Invisible,
    PropKind::Composition,
];

fn prop_symbol(kind: PropKind) -> Value {
    match kind {
        PropKind::Fontified => sym::FONTIFIED,
        PropKind::Face => sym::FACE,
        PropKind::Display => sym::DISPLAY,
        PropKind::Invisible => sym::INVISIBLE,
        PropKind::Composition => sym::COMPOSITION,
    }
}

impl Lisp {
    pub fn save_it(&mut self, orig: &It, cache: &mut ShelvedBidiCache) -> It {
        if cache.is_some() {
            let old = cache.take();
            self.bidi_unshelve_cache(old, true);
        }
        let copy = orig.clone();
        *cache = self.bidi_shelve_cache();
        copy
    }

    pub fn restore_it(&mut self, it: &mut It, copy: Option<&It>, cache: &mut ShelvedBidiCache) {
        if let Some(copy) = copy {
            *it = copy.clone();
        }
        let data = cache.take();
        self.bidi_unshelve_cache(data, false);
    }

    fn disp_invis_vector(&self, dp: Value) -> Value {
        self.char_table(dp).extras[DISP_INVIS_VECTOR_SLOT]
    }

    #[allow(clippy::too_many_arguments)]
    pub fn init_iterator(
        &mut self,
        it: &mut It,
        w: Value,
        charpos: i64,
        bytepos: i64,
        row: Option<GlyphRowRef>,
        base_face_id: usize,
    ) -> LispResult<()> {
        let mut remapped_base_face_id = base_face_id;
        let f = self.window(w).frame;
        if !self.xdisp.inhibit_free_realized_faces {
            if self.faces.face_change {
                self.faces.face_change = false;
                self.frame_mut(f).flags.face_change = false;
                self.free_all_realized_faces(Value::NIL);
            } else if self.frame(f).flags.face_change {
                self.frame_mut(f).flags.face_change = false;
                self.free_all_realized_faces(f);
            }
        }
        if !self.find_symbol_value(sym::FACE_REMAPPING_ALIST).is_nil() {
            remapped_base_face_id = self.lookup_basic_face(Some(w), f, base_face_id)?;
        }
        if row.is_none()
            && [
                MODE_LINE_ACTIVE_FACE_ID,
                MODE_LINE_INACTIVE_FACE_ID,
                TAB_LINE_ACTIVE_FACE_ID,
                TAB_LINE_INACTIVE_FACE_ID,
                HEADER_LINE_ACTIVE_FACE_ID,
                HEADER_LINE_INACTIVE_FACE_ID,
            ]
            .contains(&base_face_id)
        {
            unreachable!("init_iterator for a mode line needs the window's desired matrix");
        }
        *it = It::default();
        it.current.overlay_string_index = -1;
        it.current.dpvec_index = -1;
        it.base_face_id = remapped_base_face_id as i32;
        it.current.string_pos = TextPos::new(-1, -1);
        it.paragraph_embedding = BidiDir::L2R;
        it.bidi_it.w = w;
        it.window = w;
        it.f = f;
        it.cmp_it.id = -1;
        it.cmp_it.parent_w = w;
        it.cmp_it.parent_f = f;
        if self.int_var(sym::MAX_REDISPLAY_TICKS) > 0 {
            self.update_redisplay_ticks(0, w)?;
        }
        if base_face_id == DEFAULT_FACE_ID && self.frame_window_p(f) {
            let line_height = i64::from(self.frame(f).geometry.line_height as i32);
            let spacing = self.plain_value(sym::LINE_SPACING);
            if let Some(n) = spacing.as_fixnum().filter(|&n| n >= 0) {
                it.extra_line_spacing = n as i32;
                it.extra_line_spacing_above = 0;
            } else if spacing.is_float() {
                it.extra_line_spacing =
                    (self.heap.float_value(spacing) * line_height as f64) as i32;
                it.extra_line_spacing_above = 0;
            } else if spacing.is_cons() {
                let above = self.xcar(spacing);
                let below = self.xcdr(spacing);
                let fixnat = |v: Value| v.as_fixnum().filter(|&n| n >= 0);
                if let (Some(a), Some(b)) = (fixnat(above), fixnat(below)) {
                    it.extra_line_spacing = (b + a) as i32;
                    it.extra_line_spacing_above = a as i32;
                } else if above.is_float() && below.is_float() {
                    let (a, b) = (self.heap.float_value(above), self.heap.float_value(below));
                    it.extra_line_spacing = ((a + b) * line_height as f64) as i32;
                    it.extra_line_spacing_above = (a * line_height as f64) as i32;
                } else {
                    it.extra_line_spacing = 0;
                    it.extra_line_spacing_above = 0;
                }
            } else if self.frame(f).geometry.extra_line_spacing > 0 {
                it.extra_line_spacing = self.frame(f).geometry.extra_line_spacing as i32;
                it.extra_line_spacing_above =
                    self.frame(f).geometry.extra_line_spacing_above as i32;
            }
        }
        if self.frame(f).face_cache.is_none() {
            self.init_frame_faces(f)?;
        }
        if self.frame(f).face_cache.as_ref().map(|c| c.used()) == Some(0) {
            self.recompute_basic_faces(f)?;
        }
        it.override_ascent = -1;
        it.ctl_arrow_p = !self.plain_value(sym::CTL_ARROW).is_nil();
        let selective = self.plain_value(sym::SELECTIVE_DISPLAY);
        it.selective = match selective.as_fixnum() {
            Some(n) => n.max(-1),
            None if !selective.is_nil() => -1,
            None => 0,
        };
        it.selective_display_ellipsis_p =
            !self.plain_value(sym::SELECTIVE_DISPLAY_ELLIPSES).is_nil();
        it.dp = self.window_display_table(w);
        it.multibyte_p = !self.plain_value(sym::ENABLE_MULTIBYTE_CHARACTERS).is_nil();
        it.tab_width = self.sane_tab_width() as i16;
        it.line_wrap = LineWrap::Truncate;
        let tpww = self.find_symbol_value(sym::TRUNCATE_PARTIAL_WIDTH_WINDOWS);
        if base_face_id == DEFAULT_FACE_ID
            && self.window(w).hscroll == 0
            && (self.window_full_width_p(w)
                || tpww.is_nil()
                || matches!(tpww.as_fixnum(), Some(n) if n <= self.window(w).total_cols))
            && self.plain_value(sym::TRUNCATE_LINES).is_nil()
        {
            it.line_wrap = if self.plain_value(sym::WORD_WRAP).is_nil() {
                LineWrap::WindowWrap
            } else {
                LineWrap::WordWrap
            };
        }
        if !self.frame(f).flags.no_special_glyphs {
            if it.line_wrap == LineWrap::Truncate {
                self.produce_special_glyphs(
                    it,
                    DisplayElementType::Truncation,
                    BidiDir::L2R,
                    false,
                )?;
                let width = it.pixel_width;
                self.produce_special_glyphs(
                    it,
                    DisplayElementType::Truncation,
                    BidiDir::R2L,
                    false,
                )?;
                it.truncation_pixel_width = width.max(it.pixel_width) as i16;
            } else {
                self.produce_special_glyphs(
                    it,
                    DisplayElementType::Continuation,
                    BidiDir::L2R,
                    false,
                )?;
                let width = it.pixel_width;
                self.produce_special_glyphs(
                    it,
                    DisplayElementType::Continuation,
                    BidiDir::R2L,
                    false,
                )?;
                it.continuation_pixel_width = width.max(it.pixel_width) as i16;
            }
        }
        it.pixel_width = 0;
        it.ascent = 0;
        it.descent = 0;
        it.phys_ascent = 0;
        it.phys_descent = 0;
        it.glyph_row = row;
        it.area = GlyphRowArea::Text;
        let mut body_height = 0i64;
        if base_face_id != DEFAULT_FACE_ID {
            it.first_visible_x = 0;
            it.last_visible_x =
                (self.window(w).pixel_width - self.window_right_divider_width(w)) as i32;
        } else {
            let column_width = self.frame(f).geometry.column_width as i32;
            if self.hscrolling_current_line_p(w) {
                let min_hscroll = self.window(w).min_hscroll;
                it.first_visible_x = if min_hscroll > 0 {
                    min_hscroll as i32 * column_width
                } else {
                    0
                };
            } else {
                it.first_visible_x = self.window_hscroll_limited(w) as i32 * column_width;
            }
            let body_width = self.window_box_width(w, GlyphRowArea::Text);
            if !self.window(w).flags.pseudo_window_p
                && !self.window(w).mini
                && body_width != self.window(w).old_body_pixel_width
            {
                self.frame_mut(f).flags.window_change = true;
            }
            it.last_visible_x = it.first_visible_x + body_width as i32;
            if self.window_right_fringe_width(w) == 0 {
                if it.line_wrap == LineWrap::Truncate {
                    it.last_visible_x -= i32::from(it.truncation_pixel_width);
                } else {
                    it.last_visible_x -= i32::from(it.continuation_pixel_width);
                }
            }
            it.tab_line_p = self.window_wants_tab_line(w);
            it.header_line_p = self.window_wants_header_line(w);
            body_height = self.window_tab_line_height(w) + self.window_header_line_height(w);
            it.current_y = (body_height + self.window(w).vscroll) as i32;
        }
        if !self.frame_window_p(f) && !self.window_rightmost_p(w) {
            it.last_visible_x -= 1;
        }
        it.last_visible_y = self.window_text_bottom_y(w) as i32;
        body_height += i64::from(it.last_visible_y);
        if !self.window(w).flags.pseudo_window_p
            && !self.window(w).mini
            && body_height != self.window(w).old_body_pixel_height
        {
            self.frame_mut(f).flags.window_change = true;
        }
        if base_face_id != DEFAULT_FACE_ID {
            it.face_id = remapped_base_face_id as i32;
            if let Some(face) = self.face_from_id(f, remapped_base_face_id)
                && face.box_type != crate::xfaces::FaceBoxType::NoBox
            {
                let box_thickness = face.box_vertical_line_width;
                it.face_box_p = true;
                it.start_of_box_run_p = true;
                if box_thickness > 0 {
                    it.last_visible_x -= box_thickness;
                }
            }
        }
        if charpos >= 1 {
            it.stop_charpos = charpos;
            it.end_charpos = self.cur().zv as i64;
            it.current.pos = TextPos::new(charpos, bytepos);
            it.face_id = it.base_face_id;
            it.start = it.current;
            it.bidi_p = !self.var_true(sym::REDISPLAY__INHIBIT_BIDI)
                && !self.plain_value(sym::BIDI_DISPLAY_REORDERING).is_nil()
                && it.multibyte_p;
            if it.bidi_p {
                if base_face_id == DEFAULT_FACE_ID
                    && self.window_left_fringe_width(w) == 0
                    && self.window_right_fringe_width(w) != 0
                {
                    if it.line_wrap == LineWrap::Truncate {
                        it.last_visible_x -= i32::from(it.truncation_pixel_width);
                    } else {
                        it.last_visible_x -= i32::from(it.continuation_pixel_width);
                    }
                }
                let direction = self.plain_value(sym::BIDI_PARAGRAPH_DIRECTION);
                it.paragraph_embedding = if direction == sym::LEFT_TO_RIGHT {
                    BidiDir::L2R
                } else if direction == sym::RIGHT_TO_LEFT {
                    BidiDir::R2L
                } else {
                    BidiDir::NeutralDir
                };
                self.bidi_unshelve_cache(None, false);
                let fwp = self.frame_window_p(f);
                self.bidi_init_it(charpos, bytepos, fwp, &mut it.bidi_it)?;
            }
            it.medium_narrowing_begv = 0;
            let pos = it.current.pos;
            self.reseat(it, pos, true)?;
        }
        Ok(())
    }

    pub fn start_display(&mut self, it: &mut It, w: Value, pos: TextPos) -> LispResult<()> {
        let first_vpos =
            i32::from(self.window_wants_tab_line(w)) + i32::from(self.window_wants_header_line(w));
        let f = self.window(w).frame;
        let row = self
            .frame(f)
            .flags
            .glyphs_initialized_p
            .then_some(GlyphRowRef::Window {
                window: w,
                desired: true,
                vpos: first_vpos as usize,
            });
        self.init_iterator(it, w, pos.charpos, pos.bytepos, row, DEFAULT_FACE_ID)?;
        it.first_vpos = first_vpos;
        if it.line_wrap != LineWrap::Truncate {
            let method = it.method;
            let start_at_line_beg_p = pos.charpos == self.cur().begv as i64
                || self.cur().text.byte_at(pos.bytepos as usize - 1) == b'\n';
            if !start_at_line_beg_p {
                let first_y = it.current_y;
                let mut itdata: ShelvedBidiCache = None;
                let mut it2 = None;
                if method != ItMethod::FromBuffer {
                    it2 = Some(self.save_it(it, &mut itdata));
                }
                self.reseat_at_previous_visible_line_start(it)?;
                self.move_it_to(it, pos.charpos, -1, -1, -1, MOVE_TO_POS)?;
                let new_x = it.current_x + it.pixel_width;
                let fringe = if it.bidi_p && it.bidi_it.paragraph_dir == BidiDir::R2L {
                    self.window_left_fringe_width(w)
                } else {
                    self.window_right_fringe_width(w)
                };
                if it.current_x > 0
                    && it.line_wrap != LineWrap::Truncate
                    && (new_x > it.last_visible_x
                        || (new_x == it.last_visible_x && self.frame_window_p(it.f) && fringe != 0))
                {
                    if (it.current.dpvec_index >= 0 || it.current.overlay_string_index >= 0)
                        && it.c != 10
                    {
                        self.set_iterator_to_next(it, true)?;
                        self.move_it_in_display_line_to(it, -1, -1, 0)?;
                    }
                    it.continuation_lines_width += it.current_x;
                } else if it.current.dpvec_index >= 0 {
                    it.current.dpvec_index = 0;
                }
                let continuation_width = it.continuation_lines_width;
                if method != ItMethod::FromBuffer {
                    let copy = it2.take().expect("saved iterator");
                    self.restore_it(it, Some(&copy), &mut itdata);
                    it.continuation_lines_width = continuation_width;
                }
                it.max_ascent = 0;
                it.max_descent = 0;
                it.max_phys_ascent = 0;
                it.max_phys_descent = 0;
                it.current_y = first_y;
                it.vpos = 0;
                it.current_x = 0;
                it.hpos = 0;
                it.wrap_prefix_width = 0;
            }
        }
        Ok(())
    }

    fn in_ellipses_for_invisible_text_p(&mut self, pos: &DisplayPos, w: Value) -> LispResult<bool> {
        let charpos = pos.pos.charpos;
        if pos.dpvec_index >= 0
            && pos.overlay_string_index < 0
            && pos.string_pos.charpos < 0
            && charpos > self.cur().begv as i64
        {
            let prop = self.get_char_property(Value::fixnum(charpos), sym::INVISIBLE, w)?;
            if self.text_prop_means_invisible(prop) == 0 {
                let prop = self.get_char_property(Value::fixnum(charpos - 1), sym::INVISIBLE, w)?;
                return Ok(self.text_prop_means_invisible(prop) == 2);
            }
        }
        Ok(false)
    }

    pub fn init_from_display_pos(
        &mut self,
        it: &mut It,
        w: Value,
        pos: &DisplayPos,
    ) -> LispResult<bool> {
        let mut charpos = pos.pos.charpos;
        let mut bytepos = pos.pos.bytepos;
        let mut overlay_strings_with_newlines = false;
        if self.in_ellipses_for_invisible_text_p(pos, w)? {
            charpos -= 1;
            bytepos = self.cur().text.byte_to_char(charpos as usize) as i64;
        }
        self.init_iterator(it, w, charpos, bytepos, None, DEFAULT_FACE_ID)?;
        let mut i = 0;
        while (i as i64) < it.n_overlay_strings && i < OVERLAY_STRING_CHUNK_SIZE {
            if self.string_bytes(it.overlay_strings[i]).contains(&b'\n') {
                overlay_strings_with_newlines = true;
                break;
            }
            i += 1;
        }
        if pos.overlay_string_index >= 0 {
            if it.method == ItMethod::FromImage {
                self.pop_it(it)?;
            }
            if pos.overlay_string_index >= OVERLAY_STRING_CHUNK_SIZE as i64 {
                let mut n = pos.overlay_string_index / OVERLAY_STRING_CHUNK_SIZE as i64;
                it.current.overlay_string_index = 0;
                while n > 0 {
                    n -= 1;
                    self.load_overlay_strings(it, 0)?;
                    it.current.overlay_string_index += OVERLAY_STRING_CHUNK_SIZE as i64;
                }
            }
            it.current.overlay_string_index = pos.overlay_string_index;
            let relative_index =
                (it.current.overlay_string_index % OVERLAY_STRING_CHUNK_SIZE as i64) as usize;
            it.string = it.overlay_strings[relative_index];
            it.current.string_pos = pos.string_pos;
            it.method = ItMethod::FromString;
            it.end_charpos = self.string_chars(it.string) as i64;
            if it.bidi_p {
                it.bidi_it.string.lstring = it.string;
                it.bidi_it.string.s = None;
                it.bidi_it.string.schars = self.string_chars(it.string) as i64;
                it.bidi_it.string.bufpos = it.overlay_strings_charpos;
                it.bidi_it.string.from_disp_str = it.string_from_display_prop_p;
                it.bidi_it.string.unibyte = !it.multibyte_p;
                it.bidi_it.w = it.window;
                let fwp = self.frame_window_p(it.f);
                self.bidi_init_it(
                    it.string_charpos(),
                    it.string_bytepos(),
                    fwp,
                    &mut it.bidi_it,
                )?;
                if pos.string_pos.charpos == 0 {
                    self.get_visually_first_element(it)?;
                    if it.string_charpos() != 0 {
                        loop {
                            self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                            if it.bidi_it.charpos == 0 {
                                break;
                            }
                        }
                    }
                }
            }
        }
        if pos.string_pos.charpos >= 0 {
            it.current.string_pos = pos.string_pos;
            if it.bidi_p {
                let fwp = self.frame_window_p(it.f);
                self.bidi_init_it(
                    it.string_charpos(),
                    it.string_bytepos(),
                    fwp,
                    &mut it.bidi_it,
                )?;
            }
        }
        if pos.dpvec_index >= 0 {
            if it.dpvec == DpVec::None {
                self.get_next_display_element(it)?;
            }
            it.current.dpvec_index = pos.dpvec_index;
        }
        Ok(!overlay_strings_with_newlines)
    }

    pub fn handle_stop(&mut self, it: &mut It) -> LispResult<()> {
        it.dpvec = DpVec::None;
        it.current.dpvec_index = -1;
        let mut handle_overlay_change_p = !it.ignore_overlay_strings_at_pos_p;
        it.ellipsis_p = false;
        if it.selective_display_ellipsis_p {
            it.saved_face_id = it.face_id;
        }
        let mut handled;
        loop {
            handled = PropHandled::Normally;
            for kind in IT_PROPS {
                handled = match kind {
                    PropKind::Fontified => self.handle_fontified_prop(it)?,
                    PropKind::Face => self.handle_face_prop(it)?,
                    PropKind::Display => self.handle_display_prop(it)?,
                    PropKind::Invisible => self.handle_invisible_prop(it)?,
                    PropKind::Composition => self.handle_composition_prop(it)?,
                };
                if handled == PropHandled::RecomputeProps {
                    break;
                } else if handled == PropHandled::Return {
                    if !handle_overlay_change_p
                        || it.sp > 1
                        || (it.current.overlay_string_index < 0
                            && !self.get_overlay_strings_1(it, 0, false)?)
                    {
                        if it.ellipsis_p {
                            self.setup_for_ellipsis(it, 0);
                        }
                        if it.string.is_string() && self.string_chars(it.string) == 0 {
                            self.pop_it(it)?;
                        }
                        return Ok(());
                    } else if it.string.is_string() && self.string_chars(it.string) == 0 {
                        self.pop_it(it)?;
                    } else {
                        it.string_from_display_prop_p = false;
                        it.from_disp_prop_p = false;
                        handle_overlay_change_p = false;
                    }
                    handled = PropHandled::RecomputeProps;
                    break;
                } else if handled == PropHandled::OverlayStringConsumed {
                    handle_overlay_change_p = false;
                }
            }
            if handled != PropHandled::RecomputeProps {
                if it.method == ItMethod::FromDisplayVector {
                    handle_overlay_change_p = false;
                }
                if handle_overlay_change_p {
                    handled = self.handle_overlay_change(it)?;
                }
            }
            if it.ellipsis_p {
                self.setup_for_ellipsis(it, 0);
                break;
            }
            if handled != PropHandled::RecomputeProps {
                break;
            }
        }
        if handled == PropHandled::Normally {
            self.compute_stop_pos(it)?;
        }
        Ok(())
    }

    pub fn compute_stop_pos(&mut self, it: &mut It) -> LispResult<()> {
        let mut cmp_limit_pos = -1i64;
        let (object, limit, charpos);
        if it.string.is_string() {
            it.stop_charpos = it.end_charpos;
            object = it.string;
            limit = None;
            charpos = it.string_charpos();
        } else {
            let zv = self.cur().zv as i64;
            if it.end_charpos > zv {
                it.end_charpos = zv;
            }
            it.stop_charpos = it.end_charpos;
            charpos = it.charpos();
            let buffer = self.current_buffer;
            let pos = self.next_overlay_change(buffer, charpos as usize, true) as i64;
            if pos < it.stop_charpos {
                it.stop_charpos = pos;
            }
            let multibyte = !self.plain_value(sym::ENABLE_MULTIBYTE_CHARACTERS).is_nil();
            let auto_composition = !self.find_symbol_value(sym::AUTO_COMPOSITION_MODE).is_nil();
            let break_at_point = self.var_true(sym::COMPOSITION_BREAK_AT_POINT);
            let pt = self.cur().pt as i64;
            if multibyte
                && auto_composition
                && break_at_point
                && charpos < pt
                && pt < it.stop_charpos
            {
                it.stop_charpos = pt;
            }
            object = self.current_buffer;
            let mut pos = charpos + TEXT_PROP_DISTANCE_LIMIT;
            if !break_at_point && multibyte && auto_composition {
                let mut endpos = charpos + 10 * TEXT_PROP_DISTANCE_LIMIT;
                let mut found = false;
                if pos > zv {
                    pos = zv;
                }
                if endpos > zv {
                    endpos = zv;
                }
                while pos < endpos {
                    let ch = self.fetch_char(pos as usize);
                    pos += 1;
                    if ch == 32 || ch == 9 || ch == 10 || ch == 12 {
                        found = true;
                        break;
                    }
                }
                if found {
                    pos -= 1;
                    cmp_limit_pos = pos;
                } else if it.stop_charpos < endpos {
                    pos = it.stop_charpos;
                } else {
                    pos = charpos + TEXT_PROP_DISTANCE_LIMIT;
                }
            }
            limit = Some(pos);
        }
        if let Some(change) = self.it_prop_change(object, charpos, limit) {
            match (limit, change) {
                (Some(limit), c) if c >= limit => {
                    it.stop_charpos = limit.min(it.stop_charpos);
                }
                (_, c) => {
                    it.stop_charpos = it.stop_charpos.min(c);
                }
            }
        }
        if it.cmp_it.id < 0 {
            let mut stoppos = it.end_charpos;
            if it.bidi_p && it.bidi_it.scan_dir < 0 {
                stoppos = self.bidi_level_start(i32::from(it.bidi_it.resolved_level)) - 1;
            } else if !it.string.is_string()
                && it.cmp_it.stop_pos <= it.charpos()
                && cmp_limit_pos > 0
            {
                stoppos = cmp_limit_pos;
            }
            let string = it.string;
            self.composition_compute_stop_pos(&mut it.cmp_it, charpos, stoppos, string, false)?;
        }
        Ok(())
    }

    fn it_prop_change(&self, object: Value, charpos: i64, limit: Option<i64>) -> Option<i64> {
        let (runs, beg, end): (&[crate::object::PropRun], i64, i64) = if object.is_string() {
            (
                &self.heap.string_ref(object).props,
                0,
                self.string_chars(object) as i64,
            )
        } else {
            let b = self.buffer(object);
            (&b.text.props, 1, b.z() as i64)
        };
        if runs.is_empty() || charpos < beg || charpos >= end {
            return None;
        }
        let segment = |pos: i64| -> (i64, i64, Value) {
            let i = runs.partition_point(|r| (r.end as i64) <= pos);
            match runs.get(i) {
                Some(r) if (r.start as i64) <= pos => (r.start as i64, r.end as i64, r.plist),
                Some(r) => (
                    if i == 0 { beg } else { runs[i - 1].end as i64 },
                    r.start as i64,
                    Value::NIL,
                ),
                None => (runs.last().map_or(beg, |r| r.end as i64), end, Value::NIL),
            }
        };
        let (_, mut next, plist) = segment(charpos);
        let values_here: Vec<Value> = IT_PROPS
            .iter()
            .map(|&k| self.lookup_char_property(plist, prop_symbol(k)))
            .collect();
        while next < end {
            if let Some(limit) = limit
                && limit <= next
            {
                return Some(next);
            }
            let (_, seg_end, plist) = segment(next);
            let differs = IT_PROPS
                .iter()
                .zip(&values_here)
                .any(|(&k, &v)| self.lookup_char_property(plist, prop_symbol(k)) != v);
            if differs {
                return Some(next);
            }
            next = seg_end;
        }
        None
    }

    fn handle_fontified_prop(&mut self, it: &mut It) -> LispResult<PropHandled> {
        let mut handled = PropHandled::Normally;
        if !self.find_symbol_value(sym::MEMORY_FULL).is_nil() {
            return Ok(handled);
        }
        let fontification_functions = self.find_symbol_value(sym::FONTIFICATION_FUNCTIONS);
        if it.string.is_string()
            || it.s.is_some()
            || fontification_functions.is_nil()
            || (self.keyboard.input_was_pending
                && self.var_true(sym::REDISPLAY_SKIP_FONTIFICATION_ON_INPUT))
            || self.find_symbol_value(sym::RUN_HOOKS).is_nil()
        {
            return Ok(handled);
        }
        let pos = Value::fixnum(it.charpos());
        let prop = self.get_char_property(pos, sym::FONTIFIED, Value::NIL)?;
        if !(prop.is_nil() && it.charpos() < self.cur().z() as i64) {
            return Ok(handled);
        }
        let count = self.specpdl.len();
        let obuf = self.current_buffer;
        let (begv, zv) = (self.cur().begv, self.cur().zv);
        let old_clip_changed = self.cur().clip_changed;
        let saved_inhibit_flag = self.frame(it.f).flags.inhibit_clear_image_cache;
        let mut val = fontification_functions;
        let base = self.stack.len();
        self.stack.push(val);
        it.trace(&mut self.stack);
        let r = (|| -> LispResult<()> {
            self.specbind(sym::FONTIFICATION_FUNCTIONS, Value::NIL)?;
            if self.cur().long_line_optimizations_p
                && self.int_var(sym::LONG_LINE_OPTIMIZATIONS_REGION_SIZE) > 0
            {
                let mut begv = it.large_narrowing_begv;
                let mut zv = it.large_narrowing_zv;
                let charpos = it.charpos();
                if charpos < begv || charpos > zv {
                    begv = self.get_large_narrowing_begv(charpos as usize) as i64;
                    zv = self.get_large_narrowing_zv(charpos as usize) as i64;
                }
                if begv != 1 || zv != self.cur().z() as i64 {
                    self.labeled_narrow_to_region(
                        Value::fixnum(begv),
                        Value::fixnum(zv),
                        sym::LONG_LINE_OPTIMIZATIONS_IN_FONTIFICATION_FUNCTIONS,
                    )?;
                }
            }
            self.frame_mut(it.f).flags.inhibit_clear_image_cache = true;
            if !val.is_cons() || self.xcar(val) == sym::LAMBDA {
                self.safe_call(false, &[val, pos])?;
            } else {
                while val.is_cons() {
                    let fun = self.xcar(val);
                    if fun == sym::T {
                        let mut fns =
                            crate::data::f_default_value(self, &[sym::FONTIFICATION_FUNCTIONS])?;
                        while fns.is_cons() {
                            let fun = self.xcar(fns);
                            if fun != sym::T {
                                self.safe_call(false, &[fun, pos])?;
                            }
                            fns = self.xcdr(fns);
                        }
                    } else {
                        self.safe_call(false, &[fun, pos])?;
                    }
                    val = self.xcdr(val);
                }
            }
            Ok(())
        })();
        self.stack.truncate(base);
        r?;
        self.frame_mut(it.f).flags.inhibit_clear_image_cache = saved_inhibit_flag;
        self.unbind_to(count)?;
        if obuf == self.current_buffer {
            if begv == self.cur().begv && zv == self.cur().zv {
                self.cur_mut().clip_changed = old_clip_changed;
            }
        } else if self.buffer_live_p(obuf) {
            self.set_buffer_internal(obuf);
        }
        it.end_charpos = self.cur().zv as i64;
        if !self
            .get_char_property(pos, sym::FONTIFIED, Value::NIL)?
            .is_nil()
        {
            handled = PropHandled::RecomputeProps;
        }
        Ok(handled)
    }

    fn face_at_pos(&mut self, it: &It, attr_filter: usize) -> LispResult<i32> {
        if !it.string.is_string() {
            let charpos = it.charpos();
            let (face_id, _) = self.face_at_buffer_position(
                it.window,
                charpos as usize,
                (charpos + TEXT_PROP_DISTANCE_LIMIT) as usize,
                false,
                Some(it.base_face_id as usize),
                attr_filter,
            )?;
            return Ok(face_id as i32);
        }
        let mut from_overlay = if it.current.overlay_string_index >= 0 {
            it.string_overlays
                [(it.current.overlay_string_index % OVERLAY_STRING_CHUNK_SIZE as i64) as usize]
        } else {
            Value::NIL
        };
        if !from_overlay.is_nil() {
            let mut i = it.sp - 1;
            while i >= 0 {
                let entry = &it.stack[i as usize];
                if entry.current.overlay_string_index >= 0 {
                    from_overlay = it.string_overlays[(entry.current.overlay_string_index
                        % OVERLAY_STRING_CHUNK_SIZE as i64)
                        as usize];
                } else if !entry.from_overlay.is_nil() {
                    from_overlay = entry.from_overlay;
                }
                if !from_overlay.is_nil() {
                    break;
                }
                i -= 1;
            }
        }
        let (mut base_face_id, bufpos);
        if !from_overlay.is_nil() {
            bufpos = it.charpos();
            base_face_id = self
                .face_for_overlay_string(
                    it.window,
                    it.charpos(),
                    it.charpos() + TEXT_PROP_DISTANCE_LIMIT,
                    false,
                    from_overlay,
                    attr_filter,
                )?
                .0;
        } else {
            bufpos = 0;
            base_face_id = if it.string_from_prefix_prop_p {
                if !self.find_symbol_value(sym::FACE_REMAPPING_ALIST).is_nil() {
                    self.lookup_basic_face(Some(it.window), it.f, DEFAULT_FACE_ID)?
                } else {
                    DEFAULT_FACE_ID
                }
            } else {
                underlying_face_id(it) as usize
            };
        }
        if it.string_from_display_prop_p && it.area != GlyphRowArea::Text && !it.window.is_nil() {
            base_face_id =
                self.lookup_basic_face(Some(it.window), it.f, crate::xfaces::MARGIN_FACE_ID)?;
        }
        let face_id = self
            .face_at_string_position(
                it.window,
                it.string,
                it.string_charpos(),
                bufpos,
                base_face_id,
                false,
                attr_filter,
            )?
            .0;
        Ok(face_id as i32)
    }

    fn handle_face_prop(&mut self, it: &mut It) -> LispResult<PropHandled> {
        let count = self.specpdl.len();
        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
        let r = self.face_at_pos(it, 0);
        self.unbind_to(count)?;
        let new_face_id = r?;
        if new_face_id != it.face_id {
            let new_box = self.face_box_p(it.f, new_face_id);
            let mut old_face = self.face_box_or_none(it.f, it.face_id);
            if !it.string.is_string() && old_face.is_none() && it.charpos() > 1 {
                let prev_face_id = self.face_before_or_after_it_pos(it, true)?;
                old_face = self.face_box_or_none(it.f, prev_face_id);
            }
            it.face_id = new_face_id;
            if !(it.start_of_box_run_p && old_face == Some(true)) {
                it.start_of_box_run_p = new_box && old_face != Some(true);
            }
            it.face_box_p = new_box;
        }
        Ok(PropHandled::Normally)
    }

    fn face_box_p(&self, f: Value, face_id: i32) -> bool {
        self.face_from_id(f, face_id as usize)
            .expect("realized face")
            .box_type
            != crate::xfaces::FaceBoxType::NoBox
    }

    fn face_box_or_none(&self, f: Value, face_id: i32) -> Option<bool> {
        if face_id < 0 {
            return None;
        }
        self.face_from_id(f, face_id as usize)
            .map(|face| face.box_type != crate::xfaces::FaceBoxType::NoBox)
    }

    pub fn face_before_or_after_it_pos(&mut self, it: &mut It, before_p: bool) -> LispResult<i32> {
        let mut it_copy_data: ShelvedBidiCache = None;
        if it.string.is_string() {
            let schars = self.string_chars(it.string) as i64;
            if it.string_charpos() >= schars
                || (it.string_charpos() == 0 && before_p)
                || it.current_x <= it.first_visible_x
            {
                return Ok(it.face_id);
            }
            let mut charpos;
            if !it.bidi_p {
                charpos = if before_p {
                    it.string_charpos() - 1
                } else if it.what == DisplayElementType::Composition {
                    it.string_charpos() + it.cmp_it.nchars
                } else {
                    it.string_charpos() + 1
                };
            } else {
                let mut it_copy = self.save_it(it, &mut it_copy_data);
                it_copy.current.string_pos.charpos = 0;
                let fwp = self.frame_window_p(it_copy.f);
                self.bidi_init_it(0, 0, fwp, &mut it_copy.bidi_it)?;
                it_copy.bidi_it.scan_dir = 0;
                loop {
                    charpos = it_copy.bidi_it.charpos;
                    if charpos >= schars {
                        break;
                    }
                    self.bidi_move_to_visually_next(&mut it_copy.bidi_it)?;
                    if it_copy.bidi_it.charpos == it.string_charpos() {
                        break;
                    }
                }
                if !before_p {
                    let mut n = if it.what == DisplayElementType::Composition {
                        it.cmp_it.nchars
                    } else {
                        1
                    };
                    if it_copy.bidi_it.first_elt {
                        n += 1;
                    }
                    while n > 0 {
                        n -= 1;
                        self.bidi_move_to_visually_next(&mut it_copy.bidi_it)?;
                    }
                    charpos = it_copy.bidi_it.charpos;
                }
                self.restore_it(it, None, &mut it_copy_data);
            }
            let bufpos = if it.current.overlay_string_index >= 0 {
                it.charpos()
            } else {
                0
            };
            let base_face_id = underlying_face_id(it);
            let mut face_id = self
                .face_at_string_position(
                    it.window,
                    it.string,
                    charpos,
                    bufpos,
                    base_face_id as usize,
                    false,
                    0,
                )?
                .0;
            if self.string_multibyte(it.string) {
                let bytepos = self.string_char_to_byte_index(it.string, charpos as usize);
                let bytes = self.string_bytes(it.string);
                let (c, _) = crate::character::string_char(&bytes[bytepos..]);
                face_id = self.face_for_char(it.f, face_id, c, charpos, it.string)?;
            }
            return Ok(face_id as i32);
        }
        let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
        if (it.charpos() >= zv && !before_p) || (it.charpos() <= begv && before_p) {
            return Ok(it.face_id);
        }
        let limit = it.charpos() + TEXT_PROP_DISTANCE_LIMIT;
        let mut pos = it.current.pos;
        if !it.bidi_p {
            if before_p {
                pos = self.dec_text_pos(pos, it.multibyte_p);
            } else if it.what == DisplayElementType::Composition {
                pos.charpos += it.cmp_it.nchars;
                pos.bytepos += it.cmp_it.nbytes;
            } else {
                pos = self.inc_text_pos(pos, it.multibyte_p);
            }
        } else if before_p {
            if it.current_x <= it.first_visible_x {
                return Ok(it.face_id);
            }
            let mut it_copy = self.save_it(it, &mut it_copy_data);
            let current_x = it_copy.current_x;
            self.move_it_vertically_backward(&mut it_copy, 0)?;
            self.move_it_in_display_line(&mut it_copy, zv, current_x - 1, MOVE_TO_X)?;
            pos = it_copy.current.pos;
            self.restore_it(it, None, &mut it_copy_data);
        } else {
            let mut n = if it.what == DisplayElementType::Composition {
                it.cmp_it.nchars
            } else {
                1
            };
            let mut it_copy = it.clone();
            if it.bidi_it.first_elt {
                n += 1;
            }
            while n > 0 {
                n -= 1;
                self.bidi_move_to_visually_next(&mut it_copy.bidi_it)?;
            }
            pos = TextPos::new(it_copy.bidi_it.charpos, it_copy.bidi_it.bytepos);
        }
        let (mut face_id, _) = self.face_at_buffer_position(
            it.window,
            pos.charpos as usize,
            limit as usize,
            false,
            None,
            0,
        )?;
        if it.multibyte_p {
            let c = self.cur().text.char_at_byte(pos.bytepos as usize);
            face_id = self.face_for_char(it.f, face_id, c, pos.charpos, Value::NIL)?;
        }
        Ok(face_id as i32)
    }

    pub fn dec_text_pos(&self, pos: TextPos, multibyte: bool) -> TextPos {
        let mut bytepos = pos.bytepos - 1;
        if multibyte {
            let text = &self.cur().text;
            while text.byte_at(bytepos as usize) & 0xC0 == 0x80 {
                bytepos -= 1;
            }
        }
        TextPos::new(pos.charpos - 1, bytepos)
    }

    pub fn inc_text_pos(&self, pos: TextPos, multibyte: bool) -> TextPos {
        let len = if multibyte {
            crate::character::bytes_by_char_head(self.cur().text.byte_at(pos.bytepos as usize))
        } else {
            1
        };
        TextPos::new(pos.charpos + 1, pos.bytepos + len as i64)
    }

    fn handle_invisible_prop(&mut self, it: &mut It) -> LispResult<PropHandled> {
        let mut handled = PropHandled::Normally;
        let (curpos, mut endpos, pos, mut prop, mut overlay);
        if it.string.is_string() {
            curpos = it.string_charpos();
            endpos = self.string_chars(it.string) as i64;
            pos = Value::fixnum(curpos);
            prop = crate::textprop::f_get_text_property(self, &[pos, sym::INVISIBLE, it.string])?;
            overlay = Value::NIL;
        } else {
            curpos = it.charpos();
            endpos = self.cur().zv as i64;
            pos = Value::fixnum(curpos);
            (prop, overlay) = self.get_char_property_and_overlay(pos, sym::INVISIBLE, it.window)?;
        }
        let mut invis = self.text_prop_means_invisible(prop);
        if invis == 0 || curpos >= it.end_charpos {
            return Ok(handled);
        }
        let slow = it.bidi_p && !it.bidi_it.at_base_level();
        let mut display_ellipsis_p = invis == 2;
        handled = PropHandled::RecomputeProps;
        if slow {
            if it.bidi_it.first_elt && it.bidi_it.charpos < endpos {
                let pe = it.paragraph_embedding;
                self.bidi_paragraph_init(pe, &mut it.bidi_it, true)?;
            }
            if it.string.is_string() {
                let mut done = false;
                loop {
                    self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                    if it.bidi_it.charpos < 0 || it.bidi_it.charpos >= endpos {
                        done = true;
                    } else {
                        let pos = Value::fixnum(it.bidi_it.charpos);
                        prop = crate::textprop::f_get_text_property(
                            self,
                            &[pos, sym::INVISIBLE, it.string],
                        )?;
                        invis = self.text_prop_means_invisible(prop);
                        if invis == 2 {
                            display_ellipsis_p = true;
                        }
                    }
                    if done || invis == 0 {
                        break;
                    }
                }
                if display_ellipsis_p {
                    it.ellipsis_p = true;
                }
                it.current.string_pos = TextPos::new(it.bidi_it.charpos, it.bidi_it.bytepos);
                if it.string_bytepos() >= endpos
                    && it.current.overlay_string_index >= 0
                    && !display_ellipsis_p
                {
                    self.next_overlay_string(it)?;
                    handled = PropHandled::OverlayStringConsumed;
                }
            } else {
                let mut done = false;
                let begv = self.cur().begv as i64;
                loop {
                    self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                    if it.bidi_it.charpos < begv || it.bidi_it.charpos >= endpos {
                        done = true;
                    } else {
                        let pos = Value::fixnum(it.bidi_it.charpos);
                        prop = self.get_char_property(pos, sym::INVISIBLE, it.window)?;
                        invis = self.text_prop_means_invisible(prop);
                        if invis == 2 {
                            display_ellipsis_p = true;
                        }
                    }
                    if done || invis == 0 {
                        break;
                    }
                }
                it.current.pos = TextPos::new(it.bidi_it.charpos, it.bidi_it.bytepos);
                if display_ellipsis_p {
                    it.position.charpos = it.charpos() - 1;
                    it.position.bytepos =
                        self.cur().text.char_to_byte(it.position.charpos as usize) as i64;
                }
                if overlay.is_nil() && self.get_overlay_strings(it, it.stop_charpos)? {
                    handled = PropHandled::RecomputeProps;
                    if it.sp > 0 {
                        let i = (it.sp - 1) as usize;
                        it.stack[i].display_ellipsis_p = display_ellipsis_p;
                        it.stack[i].stop_charpos = it.stack[i].current.pos.charpos;
                    }
                } else if display_ellipsis_p {
                    it.ellipsis_p = true;
                    handled = PropHandled::Return;
                }
            }
        } else if it.string.is_string() {
            let mut end_charpos = pos;
            if invis != 0 && it.string_charpos() < it.end_charpos {
                let len = endpos;
                let limit = Value::fixnum(len);
                loop {
                    end_charpos = crate::textprop::f_next_single_property_change(
                        self,
                        &[end_charpos, sym::INVISIBLE, it.string, limit],
                    )?;
                    if let Some(e) = end_charpos.as_fixnum() {
                        endpos = e;
                        prop = crate::textprop::f_get_text_property(
                            self,
                            &[end_charpos, sym::INVISIBLE, it.string],
                        )?;
                        invis = self.text_prop_means_invisible(prop);
                        if invis == 2 {
                            display_ellipsis_p = true;
                        }
                    } else {
                        endpos = len;
                    }
                    if !(invis != 0 && endpos < len) {
                        break;
                    }
                }
                if display_ellipsis_p {
                    it.ellipsis_p = true;
                }
                if endpos < len {
                    let old = it.current.string_pos;
                    let oldpos = old.charpos;
                    if it.bidi_p {
                        let schars = self.string_chars(it.string) as i64;
                        if it.bidi_it.first_elt && it.bidi_it.charpos < schars {
                            let pe = it.paragraph_embedding;
                            self.bidi_paragraph_init(pe, &mut it.bidi_it, true)?;
                        }
                        loop {
                            self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                            if !(oldpos <= it.bidi_it.charpos
                                && it.bidi_it.charpos < endpos
                                && it.bidi_it.charpos < it.bidi_it.string.schars)
                            {
                                break;
                            }
                        }
                        it.current.string_pos =
                            TextPos::new(it.bidi_it.charpos, it.bidi_it.bytepos);
                        if it.string_charpos() >= endpos {
                            it.prev_stop = endpos;
                        }
                    } else {
                        it.current.string_pos.charpos = endpos;
                        let string = it.string;
                        it.current.string_pos =
                            self.compute_string_pos(it.current.string_pos, old, string);
                    }
                } else if it.current.overlay_string_index >= 0 && !display_ellipsis_p {
                    self.next_overlay_string(it)?;
                    handled = PropHandled::OverlayStringConsumed;
                } else {
                    it.current.string_pos = TextPos::new(
                        self.string_chars(it.string) as i64,
                        self.string_bytes(it.string).len() as i64,
                    );
                }
            }
        } else {
            let mut tem = curpos;
            let mut newpos;
            if invis != 0 && curpos < it.end_charpos {
                let zv = self.cur().zv as i64;
                loop {
                    let (np, next_stop) = self.skip_invisible(tem, zv, it.window)?;
                    newpos = np;
                    if newpos == tem || newpos >= zv {
                        invis = 0;
                    } else {
                        let pos = Value::fixnum(newpos);
                        prop = self.get_char_property(pos, sym::INVISIBLE, it.window)?;
                        invis = self.text_prop_means_invisible(prop);
                    }
                    if invis != 0 {
                        tem = next_stop;
                    }
                    if invis == 2 {
                        display_ellipsis_p = true;
                    }
                    if invis == 0 {
                        break;
                    }
                }
                if it.bidi_p {
                    let bpos = self.cur().text.char_to_byte(newpos as usize) as i64;
                    let zv_byte = self.cur().zv_byte as i64;
                    let on_newline =
                        bpos == zv_byte || self.cur().text.byte_at(bpos as usize) == b'\n';
                    let after_newline = newpos <= self.cur().begv as i64
                        || self.cur().text.byte_at(bpos as usize - 1) == b'\n';
                    if on_newline || after_newline {
                        let pdir = it.bidi_it.paragraph_dir;
                        self.reseat_1(it, TextPos::new(newpos, bpos), false)?;
                        if on_newline {
                            it.bidi_it.first_elt = false;
                            it.bidi_it.paragraph_dir = pdir;
                            it.bidi_it.ch = if bpos == zv_byte { -1 } else { 10 };
                            it.bidi_it.nchars = 1;
                            it.bidi_it.ch_len = 1;
                        }
                    } else {
                        if it.bidi_it.first_elt && it.bidi_it.charpos < zv {
                            let pe = it.paragraph_embedding;
                            self.bidi_paragraph_init(pe, &mut it.bidi_it, true)?;
                        }
                        loop {
                            self.bidi_move_to_visually_next(&mut it.bidi_it)?;
                            if !(it.stop_charpos <= it.bidi_it.charpos
                                && it.bidi_it.charpos < newpos)
                            {
                                break;
                            }
                        }
                        it.current.pos = TextPos::new(it.bidi_it.charpos, it.bidi_it.bytepos);
                        if it.charpos() >= newpos {
                            it.prev_stop = newpos;
                        }
                    }
                } else {
                    it.current.pos =
                        TextPos::new(newpos, self.cur().text.char_to_byte(newpos as usize) as i64);
                }
                if display_ellipsis_p && !it.object.is_string() {
                    it.position.charpos = newpos - 1;
                    it.position.bytepos =
                        self.cur().text.char_to_byte(it.position.charpos as usize) as i64;
                }
                if overlay.is_nil() && self.get_overlay_strings(it, it.stop_charpos)? {
                    handled = PropHandled::RecomputeProps;
                    if it.sp > 0 {
                        let i = (it.sp - 1) as usize;
                        it.stack[i].display_ellipsis_p = display_ellipsis_p;
                        it.stack[i].stop_charpos = it.stack[i].current.pos.charpos;
                    }
                } else if display_ellipsis_p {
                    it.ellipsis_p = true;
                    handled = PropHandled::Return;
                }
            }
            overlay = Value::NIL;
            let _ = overlay;
        }
        Ok(handled)
    }

    pub fn setup_for_ellipsis(&mut self, it: &mut It, len: i32) {
        if !it.dp.is_nil() && self.is_vector(self.disp_invis_vector(it.dp)) {
            let v = self.disp_invis_vector(it.dp);
            it.dpvec = DpVec::Lisp(v);
            it.dpend = self.vector_items(v).len() as i64;
        } else {
            let v = self.xdisp.default_invis_vector;
            it.dpvec = DpVec::Lisp(v);
            it.dpend = 3;
        }
        it.dpvec_char_len = len;
        it.current.dpvec_index = 0;
        it.dpvec_face_id = -1;
        if it.saved_face_id >= 0 {
            it.face_id = it.saved_face_id;
        }
        if it.method == ItMethod::FromBuffer {
            it.ignore_overlay_strings_at_pos_p = false;
        }
        it.method = ItMethod::FromDisplayVector;
        it.ellipsis_p = true;
    }

    pub fn find_display_property(&self, disp: Value, spec: Value) -> Value {
        if disp.is_nil() {
            return Value::NIL;
        }
        let matches =
            |elem: Value| elem.is_cons() && self.xcdr(elem).is_cons() && self.xcar(elem) == spec;
        let found = if self.is_vector(disp) {
            self.vector_items(disp)
                .iter()
                .copied()
                .find(|&e| matches(e))
        } else if disp.is_cons() && self.xcar(disp).is_cons() {
            let mut disp = disp;
            let mut found = None;
            while !disp.is_nil() {
                let elem = self.xcar(disp);
                if matches(elem) {
                    found = Some(elem);
                    break;
                }
                disp = if self.xcdr(disp).is_cons() {
                    self.xcdr(disp)
                } else {
                    Value::NIL
                };
            }
            found
        } else if matches(disp) {
            Some(disp)
        } else {
            None
        };
        match found {
            Some(elem) => {
                let rest = self.xcdr(elem);
                if self.xcdr(rest).is_nil() {
                    self.xcar(rest)
                } else {
                    rest
                }
            }
            None => Value::NIL,
        }
    }

    pub fn get_display_property(&mut self, charpos: i64, prop: Value, object: Value) -> LispResult {
        let disp = self.get_char_property(Value::fixnum(charpos), sym::DISPLAY, object)?;
        Ok(self.find_display_property(disp, prop))
    }
}

fn underlying_face_id(it: &It) -> i32 {
    let mut face_id = it.base_face_id;
    let mut i = it.sp - 1;
    while i >= 0 {
        if it.stack[i as usize].string.is_nil() {
            face_id = it.stack[i as usize].face_id;
        }
        i -= 1;
    }
    face_id
}

pub(crate) fn f_get_display_property(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (position, spec) = (args[0], args[1]);
    let object = args.get(2).copied().unwrap_or(Value::NIL);
    let mut properties = args.get(3).copied().unwrap_or(Value::NIL);
    if properties.is_nil() {
        properties = crate::textprop::f_get_text_property(lisp, &[position, sym::DISPLAY, object])?;
    } else if !properties.is_cons() {
        return Err(lisp.wrong_type(sym::LISTP, properties));
    }
    Ok(lisp.find_display_property(properties, spec))
}
