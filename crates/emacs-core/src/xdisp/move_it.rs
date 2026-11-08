use crate::bidi::{BidiDir, ShelvedBidiCache};
use crate::dispextern::{
    GlyphRowArea, It, ItMethod, LineWrap, MOVE_TO_POS, MOVE_TO_X, MoveItResult, TextPos,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;

struct Saved {
    it: Option<It>,
    data: ShelvedBidiCache,
}

impl Saved {
    fn none() -> Saved {
        Saved {
            it: None,
            data: None,
        }
    }

    fn valid(&self) -> bool {
        self.it.is_some()
    }

    fn current_x(&self) -> i32 {
        self.it.as_ref().map_or(0, |i| i.current_x)
    }
}

impl Lisp {
    fn save_into(&mut self, saved: &mut Saved, it: &It) {
        let copy = self.save_it(it, &mut saved.data);
        saved.it = Some(copy);
    }

    fn restore_from(&mut self, it: &mut It, saved: &mut Saved) {
        let copy = saved.it.take().expect("saved iterator");
        self.restore_it(it, Some(&copy), &mut saved.data);
    }

    pub(super) fn it_overflow_newline_into_fringe(&self, it: &It) -> bool {
        !self
            .find_symbol_value(sym::OVERFLOW_NEWLINE_INTO_FRINGE)
            .is_nil()
            && self.frame_window_p(it.f)
            && if it.bidi_it.paragraph_dir == BidiDir::R2L {
                self.window_left_fringe_width(it.window) > 0
            } else {
                self.window_right_fringe_width(it.window) > 0
            }
            && it.current_x == it.last_visible_x
    }

    fn fringe_width_in_direction(&self, it: &It) -> i64 {
        if it.bidi_p && it.bidi_it.paragraph_dir == BidiDir::R2L {
            self.window_left_fringe_width(it.window)
        } else {
            self.window_right_fringe_width(it.window)
        }
    }

    fn buffer_pos_reached_p(&self, it: &It, op: i32, to_charpos: i64) -> bool {
        (op & MOVE_TO_POS) != 0
            && self.is_buffer(it.object)
            && (it.charpos() == to_charpos
                || ((!it.bidi_p || it.bidi_it.at_base_level()) && it.charpos() > to_charpos)
                || (it.what == crate::dispextern::DisplayElementType::Composition
                    && ((it.charpos() > to_charpos && to_charpos >= it.cmp_it.charpos)
                        || (it.charpos() < to_charpos && to_charpos <= it.cmp_it.charpos))))
            && (it.method == ItMethod::FromBuffer
                || (it.method == ItMethod::FromDisplayVector
                    && i64::from(it.current.dpvec_index) + 1 >= it.dpend))
    }

    fn note_this_line_min_pos(&mut self, it: &It) {
        if it.charpos() < self.xdisp.this_line_min_pos.charpos {
            self.xdisp.this_line_min_pos = TextPos::new(it.charpos(), it.bytepos());
        }
    }

    pub fn move_it_in_display_line_to(
        &mut self,
        it: &mut It,
        to_charpos: i64,
        to_x: i32,
        op: i32,
    ) -> LispResult<MoveItResult> {
        let mut result = MoveItResult::Undefined;
        let mut wrap_it = Saved::none();
        let mut atpos_it = Saved::none();
        let mut atx_it = Saved::none();
        let mut ppos_it = Saved::none();
        let mut may_wrap = false;
        let mut prev_method = it.method;
        let mut closest_pos = 0i64;
        let mut prev_pos = it.charpos();
        let mut saw_smaller_pos = prev_pos < to_charpos;
        let mut line_number_pending = false;
        let mut this_line_subject_to_line_prefix = 0;
        let mut moved_to_next = false;
        let saved_glyph_row = it.glyph_row.take();
        let zv = self.cur().zv as i64;
        if it.bidi_p {
            if (op & MOVE_TO_POS) != 0 && it.charpos() >= to_charpos {
                self.save_into(&mut ppos_it, it);
                closest_pos = it.charpos();
            } else {
                closest_pos = zv;
            }
        }
        if it.hpos == 0 {
            if !(op != 0 && to_x == it.first_visible_x) && self.should_produce_line_number(it)? {
                if it.current_x == it.first_visible_x {
                    self.maybe_produce_line_number(it)?;
                } else {
                    line_number_pending = true;
                }
            }
            if it.area == GlyphRowArea::Text && !it.string_from_prefix_prop_p {
                self.handle_line_prefix(it)?;
            }
            this_line_subject_to_line_prefix = i32::from(it.string_from_prefix_prop_p);
        }
        self.note_this_line_min_pos(it);
        let mut saved_start_of_box_run = it.start_of_box_run_p;
        let mut skip_restore = false;
        'outer: loop {
            let mut x;
            if (op & MOVE_TO_POS) != 0
                && self.is_buffer(it.object)
                && it.method == ItMethod::FromBuffer
                && (((!it.bidi_p || it.bidi_it.at_base_level()) && it.charpos() > to_charpos)
                    || (it.bidi_p
                        && (prev_method == ItMethod::FromImage
                            || prev_method == ItMethod::FromStretch
                            || prev_method == ItMethod::FromString)
                        && ((prev_pos < to_charpos && it.charpos() >= to_charpos)
                            || (prev_pos > to_charpos && it.charpos() <= to_charpos))))
            {
                if it.line_wrap != LineWrap::WordWrap || !wrap_it.valid() {
                    result = MoveItResult::PosMatchOrZv;
                    break;
                } else if it.line_wrap == LineWrap::WordWrap && !atpos_it.valid() {
                    self.save_into(&mut atpos_it, it);
                }
            }
            if !self.get_next_display_element(it)? {
                result = MoveItResult::PosMatchOrZv;
                break;
            }
            if it.line_wrap == LineWrap::Truncate {
                if self.buffer_pos_reached_p(it, op, to_charpos)
                    && (it.pixel_width > 0
                        || it.charpos() > to_charpos
                        || it.area != GlyphRowArea::Text)
                {
                    result = MoveItResult::PosMatchOrZv;
                    break;
                }
            } else if it.line_wrap == LineWrap::WordWrap && it.area == GlyphRowArea::Text {
                let next_may_wrap = self.char_can_wrap_after(it)?;
                if may_wrap && self.char_can_wrap_before(it)? {
                    if atpos_it.valid() {
                        self.restore_from(it, &mut atpos_it);
                        result = MoveItResult::PosMatchOrZv;
                        skip_restore = true;
                        break;
                    }
                    if atx_it.valid() {
                        self.restore_from(it, &mut atx_it);
                        result = MoveItResult::XReached;
                        skip_restore = true;
                        break;
                    }
                    self.save_into(&mut wrap_it, it);
                }
                may_wrap = next_may_wrap;
            }
            let ascent = it.max_ascent;
            let descent = it.max_descent;
            x = it.current_x;
            saved_start_of_box_run = it.start_of_box_run_p;
            self.produce_glyphs_macro(it)?;
            moved_to_next = false;
            if it.area != GlyphRowArea::Text {
                prev_method = it.method;
                if it.method == ItMethod::FromBuffer {
                    prev_pos = it.charpos();
                }
                self.set_iterator_to_next(it, true)?;
                moved_to_next = true;
                self.note_this_line_min_pos(it);
                if it.bidi_p
                    && (op & MOVE_TO_POS) != 0
                    && it.charpos() > to_charpos
                    && it.charpos() < closest_pos
                {
                    closest_pos = it.charpos();
                }
                continue;
            }
            if it.nglyphs != 0 {
                let single_glyph_width = it.pixel_width / it.nglyphs;
                let x_before_this_char = x;
                let hpos_before_this_char = it.hpos;
                let mut i = 0;
                while i < it.nglyphs {
                    let new_x = x + single_glyph_width;
                    if (op & MOVE_TO_X) != 0 && new_x > to_x {
                        if self.buffer_pos_reached_p(it, op, to_charpos) {
                            if it.line_wrap != LineWrap::WordWrap || !wrap_it.valid() {
                                it.current_x = x;
                                it.max_ascent = ascent;
                                it.max_descent = descent;
                                result = MoveItResult::PosMatchOrZv;
                                break 'outer;
                            }
                            if !atpos_it.valid() {
                                self.save_into(&mut atpos_it, it);
                                if let Some(a) = atpos_it.it.as_mut() {
                                    a.current_x = x;
                                    a.max_ascent = ascent;
                                    a.max_descent = descent;
                                }
                            }
                        } else {
                            if it.line_wrap != LineWrap::WordWrap || !wrap_it.valid() {
                                it.current_x = x;
                                result = MoveItResult::XReached;
                                break;
                            }
                            if !atx_it.valid() {
                                self.save_into(&mut atx_it, it);
                                if let Some(a) = atx_it.it.as_mut() {
                                    a.current_x = x;
                                    a.max_ascent = ascent;
                                    a.max_descent = descent;
                                }
                            }
                        }
                    }
                    let fwp = self.frame_window_p(it.f);
                    let fringe = self.fringe_width_in_direction(it);
                    if it.line_wrap != LineWrap::Truncate
                        && (new_x > it.last_visible_x
                            || (new_x == it.last_visible_x && fwp && fringe != 0))
                        && (this_line_subject_to_line_prefix == 0
                            || (i != 0
                                && (x > it.last_visible_x
                                    || (x == it.last_visible_x && fwp && fringe != 0))))
                    {
                        let mut moved_forward = false;
                        let lnum_extra = if it.lnum_width != 0 {
                            it.lnum_width + 2
                        } else {
                            0
                        };
                        if it.hpos == lnum_extra || (new_x == it.last_visible_x && fwp) {
                            it.hpos += 1;
                            it.current_x = new_x;
                            if i == it.nglyphs - 1 {
                                if self.buffer_pos_reached_p(it, op, to_charpos) {
                                    let mut can_wrap = true;
                                    if it.line_wrap == LineWrap::WordWrap
                                        && wrap_it.valid()
                                        && may_wrap
                                        && self.it_overflow_newline_into_fringe(it)
                                    {
                                        let mut tem_data: ShelvedBidiCache = None;
                                        let tem_it = self.save_it(it, &mut tem_data);
                                        self.set_iterator_to_next(it, true)?;
                                        if self.get_next_display_element(it)?
                                            && !self.char_can_wrap_before(it)?
                                        {
                                            can_wrap = false;
                                        }
                                        self.restore_it(it, Some(&tem_it), &mut tem_data);
                                    }
                                    if it.line_wrap != LineWrap::WordWrap
                                        || !wrap_it.valid()
                                        || (may_wrap
                                            && can_wrap
                                            && self.it_overflow_newline_into_fringe(it))
                                    {
                                        it.hpos = hpos_before_this_char;
                                        it.current_x = x_before_this_char;
                                        result = MoveItResult::PosMatchOrZv;
                                        break;
                                    }
                                    if it.line_wrap == LineWrap::WordWrap && !atpos_it.valid() {
                                        self.save_into(&mut atpos_it, it);
                                        if let Some(a) = atpos_it.it.as_mut() {
                                            a.current_x = x_before_this_char;
                                            a.hpos = hpos_before_this_char;
                                        }
                                    }
                                }
                                prev_method = it.method;
                                if it.method == ItMethod::FromBuffer {
                                    prev_pos = it.charpos();
                                }
                                self.set_iterator_to_next(it, true)?;
                                moved_to_next = true;
                                self.note_this_line_min_pos(it);
                                if !fwp || fringe == 0 || self.it_overflow_newline_into_fringe(it) {
                                    if !self.get_next_display_element(it)? {
                                        result = MoveItResult::PosMatchOrZv;
                                        break;
                                    }
                                    moved_forward = true;
                                    if self.buffer_pos_reached_p(it, op, to_charpos) {
                                        if it.at_end_of_line_p() {
                                            result = MoveItResult::PosMatchOrZv;
                                            break;
                                        } else if (prev_method != ItMethod::FromStretch
                                            && prev_method != ItMethod::FromImage)
                                            || it.line_wrap != LineWrap::WordWrap
                                        {
                                            result = MoveItResult::LineContinued;
                                            break;
                                        }
                                    }
                                    if it.at_end_of_line_p()
                                        && (it.line_wrap != LineWrap::WordWrap
                                            || !wrap_it.valid()
                                            || self.it_overflow_newline_into_fringe(it))
                                    {
                                        result = MoveItResult::NewlineOrCr;
                                        break;
                                    }
                                }
                            }
                        } else {
                            it.current_x = x;
                            it.max_ascent = ascent;
                            it.max_descent = descent;
                        }
                        if may_wrap
                            && self.it_overflow_newline_into_fringe(it)
                            && (!moved_forward || self.char_can_wrap_before(it)?)
                        {
                            if (op & MOVE_TO_X) != 0 && new_x == it.last_visible_x && atx_it.valid()
                            {
                                self.restore_from(it, &mut atx_it);
                                atpos_it = Saved::none();
                                atx_it = Saved::none();
                                result = MoveItResult::XReached;
                                break;
                            }
                        } else if wrap_it.valid() {
                            self.restore_from(it, &mut wrap_it);
                            atpos_it = Saved::none();
                            atx_it = Saved::none();
                        }
                        result = MoveItResult::LineContinued;
                        break;
                    }
                    if self.buffer_pos_reached_p(it, op, to_charpos) {
                        if it.line_wrap != LineWrap::WordWrap || !wrap_it.valid() {
                            it.current_x = x;
                            it.max_ascent = ascent;
                            it.max_descent = descent;
                            result = MoveItResult::PosMatchOrZv;
                            break 'outer;
                        }
                        if it.line_wrap == LineWrap::WordWrap && !atpos_it.valid() {
                            self.save_into(&mut atpos_it, it);
                            if let Some(a) = atpos_it.it.as_mut() {
                                a.current_x = x;
                                a.max_ascent = ascent;
                                a.max_descent = descent;
                            }
                        }
                    }
                    if new_x > it.first_visible_x {
                        if line_number_pending {
                            line_number_pending = false;
                            it.current_x = it.first_visible_x;
                            self.maybe_produce_line_number(it)?;
                            it.current_x += new_x - it.first_visible_x;
                        }
                        it.hpos += 1;
                    }
                    i += 1;
                    x = new_x;
                }
                if result != MoveItResult::Undefined {
                    break;
                }
            } else if self.buffer_pos_reached_p(it, op, to_charpos) {
                it.current_x = x;
                it.max_ascent = ascent;
                it.max_descent = descent;
                result = MoveItResult::PosMatchOrZv;
                break;
            } else if (op & MOVE_TO_X) != 0 && it.current_x >= to_x {
                result = MoveItResult::XReached;
                break;
            }
            if it.at_end_of_line_p() {
                let mut reached = false;
                if it.bidi_p && (op & MOVE_TO_POS) != 0 {
                    if !saw_smaller_pos && it.charpos() > to_charpos {
                        if closest_pos < zv {
                            self.restore_from(it, &mut ppos_it);
                            if closest_pos != to_charpos {
                                self.move_it_in_display_line_to(it, closest_pos, -1, MOVE_TO_POS)?;
                            }
                            result = MoveItResult::PosMatchOrZv;
                        } else {
                            reached = true;
                        }
                    } else if it.line_wrap == LineWrap::WordWrap
                        && atpos_it.valid()
                        && it.charpos() > to_charpos
                    {
                        reached = true;
                    } else {
                        result = MoveItResult::NewlineOrCr;
                    }
                } else {
                    result = MoveItResult::NewlineOrCr;
                }
                if reached {
                    it.current_x = x;
                    it.max_ascent = ascent;
                    it.max_descent = descent;
                    result = MoveItResult::PosMatchOrZv;
                    break;
                }
                if it.line_wrap == LineWrap::Truncate
                    && it.current_x <= it.first_visible_x
                    && result == MoveItResult::NewlineOrCr
                    && it.char_to_display == 10
                {
                    it.max_ascent = it.ascent;
                    it.max_descent = it.descent;
                }
                if result == MoveItResult::NewlineOrCr {
                    it.constrain_row_ascent_descent_p = false;
                }
                break;
            }
            prev_method = it.method;
            if it.method == ItMethod::FromBuffer {
                prev_pos = it.charpos();
            }
            self.set_iterator_to_next(it, true)?;
            moved_to_next = true;
            if this_line_subject_to_line_prefix == 1 && !it.string_from_prefix_prop_p {
                this_line_subject_to_line_prefix = 2;
            } else if this_line_subject_to_line_prefix == 2 && !it.string_from_prefix_prop_p {
                this_line_subject_to_line_prefix = 0;
            }
            self.note_this_line_min_pos(it);
            if it.charpos() < to_charpos {
                saw_smaller_pos = true;
            }
            if it.bidi_p
                && (op & MOVE_TO_POS) != 0
                && it.charpos() >= to_charpos
                && it.charpos() < closest_pos
            {
                closest_pos = it.charpos();
            }
            if it.line_wrap == LineWrap::Truncate && it.current_x >= it.last_visible_x {
                if !self.frame_window_p(it.f)
                    || self.fringe_width_in_direction(it) == 0
                    || self.it_overflow_newline_into_fringe(it)
                {
                    let at_eob_p = !self.get_next_display_element(it)?;
                    if at_eob_p
                        || self.buffer_pos_reached_p(it, op, to_charpos)
                        || (it.bidi_p
                            && (op & MOVE_TO_POS) != 0
                            && !saw_smaller_pos
                            && it.charpos() > to_charpos)
                    {
                        if it.bidi_p
                            && !self.buffer_pos_reached_p(it, op, to_charpos)
                            && !at_eob_p
                            && closest_pos < zv
                        {
                            self.restore_from(it, &mut ppos_it);
                            if closest_pos != to_charpos {
                                self.move_it_in_display_line_to(it, closest_pos, -1, MOVE_TO_POS)?;
                            }
                        }
                        result = MoveItResult::PosMatchOrZv;
                        break;
                    }
                    if it.at_end_of_line_p() {
                        result = MoveItResult::NewlineOrCr;
                        break;
                    }
                } else if it.bidi_p
                    && (op & MOVE_TO_POS) != 0
                    && !saw_smaller_pos
                    && it.charpos() > to_charpos
                {
                    if closest_pos < zv {
                        self.restore_from(it, &mut ppos_it);
                        if closest_pos != to_charpos {
                            self.move_it_in_display_line_to(it, closest_pos, -1, MOVE_TO_POS)?;
                        }
                    }
                    result = MoveItResult::PosMatchOrZv;
                    break;
                }
                result = MoveItResult::LineTruncated;
                break;
            }
        }
        if !skip_restore {
            if result == MoveItResult::LineContinued
                && it.line_wrap == LineWrap::WordWrap
                && wrap_it.valid()
                && ((atpos_it.valid() && wrap_it.current_x() < atpos_it.current_x())
                    || (atx_it.valid() && wrap_it.current_x() < atx_it.current_x()))
            {
                self.restore_from(it, &mut wrap_it);
            } else if atpos_it.valid() {
                self.restore_from(it, &mut atpos_it);
            } else if atx_it.valid() {
                self.restore_from(it, &mut atx_it);
            }
        }
        if !moved_to_next {
            it.start_of_box_run_p = saved_start_of_box_run;
        }
        it.glyph_row = saved_glyph_row;
        Ok(result)
    }

    pub fn move_it_in_display_line(
        &mut self,
        it: &mut It,
        to_charpos: i64,
        to_x: i32,
        op: i32,
    ) -> LispResult<()> {
        if it.line_wrap == LineWrap::WordWrap && (op & MOVE_TO_X) != 0 {
            let mut save_data: ShelvedBidiCache = None;
            let save_it = self.save_it(it, &mut save_data);
            let skip = self.move_it_in_display_line_to(it, to_charpos, to_x, op)?;
            if skip == MoveItResult::LineContinued {
                let prev_x = (it.current_x - 1).max(0);
                self.restore_it(it, Some(&save_it), &mut save_data);
                self.move_it_in_display_line_to(it, -1, prev_x, MOVE_TO_X)?;
            }
        } else {
            self.move_it_in_display_line_to(it, to_charpos, to_x, op)?;
        }
        Ok(())
    }
}

impl Lisp {
    pub fn move_it_to(
        &mut self,
        it: &mut It,
        to_charpos: i64,
        to_x: i32,
        to_y: i32,
        to_vpos: i32,
        op: i32,
    ) -> LispResult<i32> {
        use crate::dispextern::{MOVE_TO_VPOS, MOVE_TO_Y};
        let mut skip;
        let mut skip2 = MoveItResult::XReached;
        let mut line_start_x = 0;
        let mut reached = 0;
        let mut max_current_x = 0;
        let mut backup_data: ShelvedBidiCache = None;
        'out: loop {
            if (op & MOVE_TO_VPOS) != 0 {
                if (op & (MOVE_TO_X | MOVE_TO_POS)) == 0 {
                    if it.vpos == to_vpos {
                        reached = 1;
                        break;
                    }
                    skip = self.move_it_in_display_line_to(it, -1, -1, 0)?;
                } else {
                    if it.vpos == to_vpos {
                        reached = 2;
                        break;
                    }
                    skip = self.move_it_in_display_line_to(it, to_charpos, to_x, op)?;
                    if skip == MoveItResult::PosMatchOrZv || it.vpos == to_vpos {
                        reached = 3;
                        break;
                    } else if skip == MoveItResult::XReached && it.vpos != to_vpos {
                        skip = self.move_it_in_display_line_to(it, to_charpos, -1, MOVE_TO_POS)?;
                        if skip == MoveItResult::PosMatchOrZv {
                            reached = 4;
                            break;
                        }
                    }
                }
            } else if (op & MOVE_TO_Y) != 0 {
                let mut it_backup: Option<It> = None;
                if it.line_wrap == LineWrap::WordWrap {
                    it_backup = Some(self.save_it(it, &mut backup_data));
                }
                skip = self.move_it_in_display_line_to(
                    it,
                    to_charpos,
                    if (op & MOVE_TO_X) != 0 { to_x } else { 0 },
                    MOVE_TO_X | (op & MOVE_TO_POS),
                )?;
                if skip == MoveItResult::PosMatchOrZv {
                    reached = 5;
                } else if skip == MoveItResult::XReached {
                    let line_height = it.max_ascent + it.max_descent;
                    if to_y >= it.current_y && to_y < it.current_y + line_height {
                        reached = 6;
                        break;
                    }
                    let backup = self.save_it(it, &mut backup_data);
                    skip2 =
                        self.move_it_in_display_line_to(it, to_charpos, -1, op & MOVE_TO_POS)?;
                    let line_height = it.max_ascent + it.max_descent;
                    if to_y >= it.current_y && to_y < it.current_y + line_height {
                        let (max_ascent, max_descent) = (it.max_ascent, it.max_descent);
                        self.restore_it(it, Some(&backup), &mut backup_data);
                        it.max_ascent = max_ascent;
                        it.max_descent = max_descent;
                        reached = 6;
                    } else {
                        skip = skip2;
                        if skip == MoveItResult::PosMatchOrZv {
                            reached = 7;
                            if to_charpos > 0
                                && it.charpos() != to_charpos
                                && ((backup.charpos() > to_charpos) == (it.charpos() > to_charpos))
                            {
                                let (max_ascent, max_descent) = (it.max_ascent, it.max_descent);
                                self.restore_it(it, Some(&backup), &mut backup_data);
                                it.max_ascent = max_ascent;
                                it.max_descent = max_descent;
                            }
                        }
                    }
                } else {
                    let line_height = it.max_ascent + it.max_descent;
                    if to_y >= it.current_y && to_y < it.current_y + line_height {
                        if to_y > it.current_y {
                            max_current_x = it.current_x.max(max_current_x);
                        }
                        if skip == MoveItResult::LineContinued && it.line_wrap == LineWrap::WordWrap
                        {
                            let prev_x = (it.current_x - 1).max(0);
                            let backup = it_backup.take().expect("word-wrap backup");
                            self.restore_it(it, Some(&backup), &mut backup_data);
                            skip = self.move_it_in_display_line_to(it, -1, prev_x, MOVE_TO_X)?;
                        }
                        reached = 6;
                    }
                }
                if reached != 0 {
                    max_current_x = it.current_x.max(max_current_x);
                    break;
                }
            } else if self.is_buffer(it.object)
                && (it.method == ItMethod::FromBuffer || it.method == ItMethod::FromStretch)
                && it.charpos() >= to_charpos
                && !(it.bidi_p && it.bidi_it.scan_dir == -1)
            {
                skip = MoveItResult::PosMatchOrZv;
            } else {
                skip = self.move_it_in_display_line_to(it, to_charpos, -1, MOVE_TO_POS)?;
            }
            match skip {
                MoveItResult::PosMatchOrZv => {
                    max_current_x = it.current_x.max(max_current_x);
                    reached = 8;
                    break 'out;
                }
                MoveItResult::NewlineOrCr => {
                    max_current_x = it.current_x.max(max_current_x);
                    if !self.it_overflow_newline_into_fringe(it) {
                        it.override_ascent = -1;
                    }
                    self.set_iterator_to_next(it, true)?;
                    it.continuation_lines_width = 0;
                }
                MoveItResult::LineTruncated => {
                    max_current_x = it.last_visible_x;
                    it.continuation_lines_width = 0;
                    self.reseat_at_next_visible_line_start(it, false)?;
                    let zv = self.cur().zv as i64;
                    let zv_byte = self.cur().zv_byte;
                    if (op & MOVE_TO_POS) != 0
                        && (it.charpos() > to_charpos
                            || (it.charpos() == to_charpos
                                && to_charpos == zv
                                && (zv_byte <= 1 || self.cur().text.byte_at(zv_byte - 1) != b'\n')
                                && (it.sp == 0
                                    || (it.string.is_string()
                                        && (it.current.overlay_string_index < 0
                                            || it.current.overlay_string_index
                                                >= it.n_overlay_strings - 1)
                                        && it.string_charpos() >= it.end_charpos))))
                    {
                        reached = 9;
                        break 'out;
                    }
                }
                MoveItResult::LineContinued => {
                    max_current_x = it.last_visible_x;
                    if it.c == 9 {
                        it.continuation_lines_width += it.last_visible_x;
                        if it.current_x != it.last_visible_x
                            && (op & MOVE_TO_VPOS) != 0
                            && (op & (MOVE_TO_X | MOVE_TO_POS)) == 0
                        {
                            line_start_x = it.current_x + it.pixel_width - it.last_visible_x;
                            if self.frame_window_p(it.f) {
                                let face_font = self
                                    .face_from_id(it.f, it.face_id as usize)
                                    .expect("face is not realized")
                                    .font;
                                if !face_font.is_nil() {
                                    let space_width = self.font_object(face_font).space_width;
                                    if line_start_x < space_width {
                                        line_start_x += i32::from(it.tab_width) * space_width;
                                    }
                                }
                            }
                            self.set_iterator_to_next(it, false)?;
                        }
                    } else {
                        it.continuation_lines_width += it.current_x;
                    }
                }
                MoveItResult::Undefined | MoveItResult::XReached => std::process::abort(),
            }
            it.current_x = line_start_x;
            it.wrap_prefix_width = 0;
            line_start_x = 0;
            it.hpos = 0;
            it.line_number_produced_p = false;
            it.current_y += it.max_ascent + it.max_descent;
            it.vpos += 1;
            self.xdisp.last_height = it.max_ascent + it.max_descent;
            it.max_ascent = 0;
            it.max_descent = 0;
        }
        let _ = (reached, skip2);
        if !self.frame_window_p(it.f)
            && (op & MOVE_TO_POS) != 0
            && it.charpos() == to_charpos
            && it.what == crate::dispextern::DisplayElementType::Character
            && it.nglyphs > 1
            && it.line_wrap == LineWrap::WindowWrap
            && it.current_x == it.last_visible_x - 1
            && it.c != 10
            && it.c != 9
            && self.window(it.window).flags.window_end_valid
            && i64::from(it.vpos) < self.window(it.window).window_end_vpos
        {
            it.continuation_lines_width += it.current_x;
            it.current_x = 0;
            it.hpos = 0;
            it.max_ascent = 0;
            it.max_descent = 0;
            it.wrap_prefix_width = 0;
            it.current_y += it.max_ascent + it.max_descent;
            it.vpos += 1;
            self.xdisp.last_height = it.max_ascent + it.max_descent;
        }
        Ok(max_current_x)
    }

    fn it_pos_valid_after_move_p(it: &It) -> bool {
        it.method != ItMethod::FromString || it.string_charpos() == 0
    }

    pub fn move_it_vertically_backward(&mut self, it: &mut It, dy: i32) -> LispResult<()> {
        use crate::dispextern::MOVE_TO_VPOS;
        let mut dy = dy;
        let column_width = self.frame(it.f).geometry.column_width as i32;
        let nchars_per_row = i64::from((it.last_visible_x - it.first_visible_x) / column_width);
        loop {
            let start_pos = it.charpos();
            let mut nlines = i64::from((dy / self.default_line_pixel_height(it.window)).max(1));
            let begv = self.cur().begv as i64;
            let pos_limit = if it.line_wrap == LineWrap::Truncate || nchars_per_row == 0 {
                begv
            } else {
                (start_pos - nlines * nchars_per_row).max(begv)
            };
            while nlines != 0 && it.charpos() > pos_limit {
                nlines -= 1;
                self.back_to_previous_visible_line_start(it)?;
            }
            let pos = it.current.pos;
            self.reseat_1(it, pos, true)?;
            it.wrap_prefix_width = 0;
            it.current_x = 0;
            it.hpos = 0;
            it.continuation_lines_width = 0;
            let mut it2data: ShelvedBidiCache = None;
            let mut it2 = self.save_it(it, &mut it2data);
            it2.max_ascent = 0;
            it2.max_descent = 0;
            let mut to_pos = start_pos;
            loop {
                let vpos = it2.vpos + 1;
                self.move_it_to(
                    &mut it2,
                    to_pos,
                    -1,
                    -1,
                    vpos,
                    if to_pos > 0 {
                        MOVE_TO_POS | MOVE_TO_VPOS
                    } else {
                        MOVE_TO_VPOS
                    },
                )?;
                if it2.charpos() >= start_pos {
                    to_pos = -1;
                }
                let in_string_after_newline = it2.method == ItMethod::FromString
                    && it2.charpos() == start_pos
                    && self.string_bytes(it2.string)[it2.string_bytepos() as usize - 1] == b'\n';
                if Self::it_pos_valid_after_move_p(&it2) || in_string_after_newline {
                    break;
                }
            }
            let mut it3data: ShelvedBidiCache = None;
            let it3 = self.save_it(&it2, &mut it3data);
            self.move_it_to(&mut it2, start_pos, -1, -1, -1, MOVE_TO_POS)?;
            let h = it2.current_y - it.current_y;
            let nlines = it2.vpos - it.vpos;
            it.vpos -= nlines;
            it.current_y -= h;
            if dy == 0 {
                self.restore_it(it, None, &mut it2data);
                if nlines > 0 {
                    self.move_it_by_lines(it, i64::from(nlines))?;
                }
                if it.bidi_p
                    && it.continuation_lines_width == 0
                    && !it.string.is_string()
                    && it.charpos() > self.cur().begv as i64
                    && self.cur().text.byte_at(it.bytepos() as usize - 1) != b'\n'
                {
                    let p = self.dec_text_pos(it.current.pos, self.cur().text.multibyte);
                    let w = it.window;
                    let charpos = it.charpos();
                    let cp = if it.medium_narrowing_begv != 0 {
                        let nb = self.get_small_narrowing_begv(w, charpos)?;
                        self.narrowed_find_newline_back(nb, p.charpos)?
                    } else {
                        let begv = self.cur().begv;
                        self.find_newline(p.charpos as usize, begv, -1).0 as i64
                    };
                    self.move_it_to(it, cp, -1, -1, -1, MOVE_TO_POS)?;
                }
                drop(it3data);
            } else {
                let target_y = it.current_y + h - dy;
                let y0 = it3.current_y;
                let mut it3 = it3;
                self.restore_it(&mut it3, None, &mut it3data);
                self.xdisp.last_height = 0;
                let y1 = self.line_bottom_y(&it3)?;
                let line_height = y1 - y0;
                self.restore_it(it, None, &mut it2data);
                if target_y < it.current_y
                    && i64::from(it.current_y - target_y)
                        > self
                            .window_box_height(it.window)
                            .min(i64::from(line_height * 2 / 3))
                    && it.charpos() > self.cur().begv as i64
                {
                    dy = it.current_y - target_y;
                    continue;
                } else if target_y >= it.current_y + line_height
                    && it.charpos() < self.cur().zv as i64
                {
                    if !self.frame_window_p(it.f) {
                        let d = target_y - it.current_y;
                        self.move_it_vertically(it, d)?;
                    } else {
                        let (mut last_pos, mut last_y, mut last_vpos);
                        loop {
                            last_pos = it.current.pos;
                            last_y = it.current_y;
                            last_vpos = it.vpos;
                            self.move_it_by_lines(it, 1)?;
                            if !(target_y > it.current_y && it.charpos() < self.cur().zv as i64) {
                                break;
                            }
                        }
                        if it.current_y > target_y {
                            self.reseat(it, last_pos, true)?;
                            it.current_y = last_y;
                            it.vpos = last_vpos;
                        }
                    }
                }
            }
            return Ok(());
        }
    }

    fn narrowed_find_newline_back(&mut self, begv: i64, from: i64) -> LispResult<i64> {
        let count = self.specpdl.len();
        let point_min = crate::value::Value::fixnum(self.cur().begv as i64);
        self.record_unwind(unwind_begv_move_it, point_min);
        self.set_buf_begv(begv as usize);
        let b = self.cur().begv;
        let r = self.find_newline(from as usize, b, -1).0 as i64;
        self.unbind_to(count)?;
        Ok(r)
    }

    pub fn move_it_vertically(&mut self, it: &mut It, dy: i32) -> LispResult<()> {
        use crate::dispextern::MOVE_TO_Y;
        if dy <= 0 {
            return self.move_it_vertically_backward(it, -dy);
        }
        let zv = self.cur().zv as i64;
        let y = it.current_y + dy;
        self.move_it_to(it, zv, -1, y, -1, MOVE_TO_POS | MOVE_TO_Y)?;
        if it.charpos() == zv
            && zv > self.cur().begv as i64
            && self.cur().text.byte_at(it.bytepos() as usize - 1) != b'\n'
        {
            self.move_it_by_lines(it, 0)?;
        }
        Ok(())
    }

    pub fn move_it_past_eol(&mut self, it: &mut It) -> LispResult<()> {
        let z = self.cur().z() as i64;
        let rc = self.move_it_in_display_line_to(it, z, 0, MOVE_TO_POS)?;
        if rc == MoveItResult::NewlineOrCr {
            self.set_iterator_to_next(it, false)?;
        }
        Ok(())
    }

    pub fn move_it_by_lines(&mut self, it: &mut It, dvpos: i64) -> LispResult<()> {
        use crate::dispextern::MOVE_TO_VPOS;
        let mut dvpos = dvpos;
        if dvpos == 0 {
            self.move_it_vertically_backward(it, 0)?;
            self.xdisp.last_height = 0;
        } else if dvpos > 0 {
            let vpos = it.vpos + dvpos as i32;
            self.move_it_to(it, -1, -1, -1, vpos, MOVE_TO_VPOS)?;
            if !Self::it_pos_valid_after_move_p(it) {
                let target = it.charpos() + i64::from(it.string_from_display_prop_p);
                self.move_it_to(it, target, -1, -1, -1, MOVE_TO_POS)?;
            }
        } else {
            let column_width = self.frame(it.f).geometry.column_width as i32;
            let nchars_per_row = i64::from((it.last_visible_x - it.first_visible_x) / column_width);
            let mut hit_pos_limit = false;
            dvpos += i64::from(it.vpos);
            let orig_charpos = it.charpos();
            self.move_it_vertically_backward(it, 0)?;
            dvpos -= i64::from(it.vpos);
            let start_charpos = it.charpos();
            let begv = self.cur().begv as i64;
            let pos_limit = if it.line_wrap == LineWrap::Truncate || nchars_per_row == 0 {
                begv
            } else {
                (start_charpos + dvpos * nchars_per_row).max(begv)
            };
            let mut i = -dvpos;
            while i > 0 && it.charpos() > pos_limit {
                self.back_to_previous_visible_line_start(it)?;
                i -= 1;
            }
            if i > 0 && it.charpos() <= pos_limit {
                hit_pos_limit = true;
            }
            let pos = it.current.pos;
            self.reseat(it, pos, true)?;
            while !Self::it_pos_valid_after_move_p(it) {
                dvpos += i64::from(it.vpos);
                self.move_it_vertically_backward(it, 0)?;
                dvpos -= i64::from(it.vpos);
                if Self::it_pos_valid_after_move_p(it) {
                    break;
                }
                self.back_to_previous_visible_line_start(it)?;
                let pos = it.current.pos;
                self.reseat(it, pos, true)?;
                dvpos -= 1;
            }
            it.current_x = 0;
            it.hpos = 0;
            it.wrap_prefix_width = 0;
            let mut it2data: ShelvedBidiCache = None;
            let mut it2 = self.save_it(it, &mut it2data);
            it2.vpos = 0;
            it2.current_y = 0;
            self.move_it_to(&mut it2, start_charpos, -1, -1, -1, MOVE_TO_POS)?;
            it.vpos -= it2.vpos;
            it.current_y -= it2.current_y;
            it.current_x = 0;
            it.hpos = 0;
            it.wrap_prefix_width = 0;
            if i64::from(it2.vpos) > -dvpos {
                let delta = i64::from(it2.vpos) + dvpos;
                self.restore_it(&mut it2, None, &mut it2data);
                let it2 = self.save_it(it, &mut it2data);
                let vpos = it.vpos + delta as i32;
                self.move_it_to(it, -1, -1, -1, vpos, MOVE_TO_VPOS)?;
                if i64::from(it.vpos - it2.vpos) > delta || it.charpos() == orig_charpos {
                    self.restore_it(it, Some(&it2), &mut it2data);
                }
            } else if hit_pos_limit && pos_limit > begv && dvpos < 0 && i64::from(it2.vpos) < -dvpos
            {
                dvpos += i64::from(it2.vpos);
                self.restore_it(it, None, &mut it2data);
                let mut i = -dvpos;
                while i > 0 {
                    self.back_to_previous_visible_line_start(it)?;
                    it.vpos -= 1;
                    i -= 1;
                }
                let pos = it.current.pos;
                self.reseat_1(it, pos, true)?;
            } else {
                self.restore_it(it, None, &mut it2data);
            }
        }
        Ok(())
    }
}

fn unwind_begv_move_it(lisp: &mut Lisp, point_min: crate::value::Value) -> LispResult<()> {
    let begv = point_min.as_fixnum().unwrap_or(1) as usize;
    lisp.set_buf_begv(begv);
    Ok(())
}
