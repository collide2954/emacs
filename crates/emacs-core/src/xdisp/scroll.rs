use crate::bidi::ShelvedBidiCache;
use crate::dispextern::{
    GlyphRow, GlyphRowRef, It, ItMethod, LineWrap, MOVE_TO_POS, MOVE_TO_VPOS, MOVE_TO_X, MOVE_TO_Y,
    MoveItResult, TEXT_AREA, TextPos,
};
use crate::dispnew::matrix::{WhichMatrix, clear_glyph_matrix};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::window::MarginUnit;
use crate::xfaces::DEFAULT_FACE_ID;

use super::DisplaySpecContext;
use super::reseat::DISP_INFINITY;

pub const SCROLLING_SUCCESS: i32 = 1;
pub const SCROLLING_FAILED: i32 = 0;
pub const SCROLLING_NEED_LARGER_MATRICES: i32 = -1;

pub const CURSOR_MOVEMENT_SUCCESS: i32 = 0;
pub const CURSOR_MOVEMENT_CANNOT_BE_USED: i32 = 1;
pub const CURSOR_MOVEMENT_MUST_SCROLL: i32 = 2;
pub const CURSOR_MOVEMENT_NEED_LARGER_MATRICES: i32 = 3;

pub const SCROLL_LIMIT: i64 = 100;

fn mr_partially_visible(row: &GlyphRow) -> bool {
    row.height != row.visible_height
}

fn ends_in_middle_of_char_p(row: &GlyphRow) -> bool {
    row.end.dpvec_index > 0 || row.end.overlay_string_index >= 0 || row.ends_in_middle_of_char_p
}

fn starts_in_middle_of_char_p(row: &GlyphRow) -> bool {
    row.start.dpvec_index > 0
        || row.starts_in_middle_of_char_p
        || (row.start.overlay_string_index >= 0 && row.start.string_pos.charpos > 0)
}

impl Lisp {
    fn mr_partially_visible_at_top(&mut self, w: Value, row: &GlyphRow) -> bool {
        i64::from(row.y) < self.window_tab_line_height(w) + self.window_header_line_height(w)
    }

    fn mr_partially_visible_at_bottom(&mut self, w: Value, row: &GlyphRow) -> bool {
        i64::from(row.y + row.height - row.extra_line_spacing)
            > self.window_box_height_no_mode_line(w)
    }

    pub(super) fn matrix_row_partially_visible_p(&mut self, w: Value, row: &GlyphRow) -> bool {
        mr_partially_visible(row)
            && (self.mr_partially_visible_at_top(w, row)
                || self.mr_partially_visible_at_bottom(w, row))
    }

    pub(crate) fn matrix_row_partially_visible_at_bottom_p(
        &mut self,
        w: Value,
        row: &GlyphRow,
    ) -> bool {
        mr_partially_visible(row) && self.mr_partially_visible_at_bottom(w, row)
    }

    pub(crate) fn matrix_row_partially_visible_at_top_p(
        &mut self,
        w: Value,
        row: &GlyphRow,
    ) -> bool {
        mr_partially_visible(row) && self.mr_partially_visible_at_top(w, row)
    }

    pub(super) fn window_box_text_height(&mut self, w: Value) -> i64 {
        let mode_line = if self.window_wants_mode_line(w) {
            self.current_mode_line_height(w)
        } else {
            0
        };
        self.window(w).pixel_height
            - self.window_bottom_divider_width(w)
            - self.window_scroll_bar_area_height(w)
            - mode_line
            - self.window_header_line_height(w)
            - self.window_tab_line_height(w)
    }

    fn matrix_row(&self, w: Value, which: WhichMatrix, vpos: usize) -> &GlyphRow {
        &self
            .window_matrix(w, which)
            .expect("window has no glyph matrix")
            .rows[vpos]
    }

    pub fn cursor_row_fully_visible_p(
        &mut self,
        w: Value,
        force_p: bool,
        current_matrix_p: bool,
        just_test_user_preference_p: bool,
    ) -> LispResult<bool> {
        let contents = self.window(w).contents;
        let mclfv_p = self.buffer_local_value(sym::MAKE_CURSOR_LINE_FULLY_VISIBLE, contents);
        if self.functionp(mclfv_p) {
            let val = self.safe_call(false, &[mclfv_p, w])?;
            if val.is_nil() {
                return Ok(true);
            } else if just_test_user_preference_p {
                return Ok(false);
            }
        } else if mclfv_p.is_nil() {
            return Ok(true);
        } else if just_test_user_preference_p {
            return Ok(false);
        }
        let vpos = self.window(w).cursor.vpos;
        if vpos < 0 {
            return Ok(true);
        }
        let which = if current_matrix_p {
            WhichMatrix::Current
        } else {
            WhichMatrix::Desired
        };
        let row = self.matrix_row(w, which, vpos as usize).clone();
        if !self.matrix_row_partially_visible_p(w, &row) {
            return Ok(true);
        }
        let window_height = self.window_box_height(w);
        if i64::from(row.height) >= window_height {
            let win = self.window(w);
            if !force_p || win.mini || win.vscroll != 0 || win.cursor.vpos == 0 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn row_containing_pos(
        &mut self,
        w: Value,
        charpos: i64,
        which: WhichMatrix,
        start: usize,
        end: Option<usize>,
        dy: i32,
    ) -> LispResult<Option<usize>> {
        let contents = self.window(w).contents;
        let mut row = start;
        let mut best_row = None;
        let mut mindif = self.buffer(contents).zv as i64 + 1;
        if self.matrix_row(w, which, row).tab_line_p {
            row += 1;
        }
        if self.matrix_row(w, which, row).mode_line_p {
            row += 1;
        }
        let past_end = |row: usize| end.is_some_and(|e| row >= e);
        if past_end(row) || !self.matrix_row(w, which, row).enabled_p {
            return Ok(None);
        }
        let last_y = self.window_text_bottom_y(w) - i64::from(dy);
        let reordering = !self
            .buffer_local_value(sym::BIDI_DISPLAY_REORDERING, contents)
            .is_nil();
        loop {
            let r = self.matrix_row(w, which, row).clone();
            if past_end(row) || !r.enabled_p {
                return Ok(None);
            }
            if i64::from(r.bottom_y()) > last_y {
                return Ok(None);
            }
            let end_charpos = r.end_charpos();
            if !(end_charpos < charpos
                || (end_charpos == charpos && !self.row_for_charpos_p(&r, charpos)?))
                && charpos >= r.start_charpos()
            {
                if !reordering || (best_row.is_none() && !r.continued_p) {
                    return Ok(Some(row));
                }
                for g in r.area_glyphs(TEXT_AREA) {
                    if !g.object.is_string()
                        && g.charpos > 0
                        && (g.charpos - charpos).abs() < mindif
                    {
                        mindif = (g.charpos - charpos).abs();
                        best_row = Some(row);
                        if mindif == 0 {
                            return Ok(best_row);
                        }
                    }
                }
            } else if best_row.is_some() && !r.continued_p {
                return Ok(best_row);
            }
            row += 1;
        }
    }

    pub fn run_window_scroll_functions(
        &mut self,
        window: Value,
        startp: TextPos,
    ) -> LispResult<TextPos> {
        let start = self.window(window).start;
        let buffer = self.current_buffer;
        self.set_marker_position(start, startp.charpos as usize, buffer);
        let mut startp = startp;
        if !self
            .find_symbol_value(sym::WINDOW_SCROLL_FUNCTIONS)
            .is_nil()
        {
            self.safe_run_hooks_2(
                sym::WINDOW_SCROLL_FUNCTIONS,
                window,
                Value::fixnum(startp.charpos),
            )?;
            let m = *self.marker(self.window(window).start);
            startp = TextPos::new(m.charpos as i64, m.bytepos as i64);
            let contents = self.window(window).contents;
            self.set_buffer_internal(contents);
        }
        Ok(startp)
    }

    pub fn window_start_acceptable_p(&mut self, window: Value, startp: i64) -> LispResult<bool> {
        if !self.var_true(sym::MAKE_WINDOW_START_VISIBLE) {
            return Ok(true);
        }
        let f = self.window(window).frame;
        let startpos = Value::fixnum(startp);
        let invprop = self.get_char_property(startpos, sym::INVISIBLE, window)?;
        if self.text_prop_means_invisible(invprop) != 0 {
            return Ok(false);
        }
        let disp_spec = self.get_char_property(startpos, sym::DISPLAY, window)?;
        if !disp_spec.is_nil() {
            let ctx = DisplaySpecContext {
                object: Value::NIL,
                position: startp as usize,
                bufpos: startp as usize,
                frame_window_p: self.frame_window_p(f),
                enable_eval: true,
            };
            if self.handle_display_spec_without_iterator(disp_spec, ctx)? > 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(super) fn overlay_arrow_in_current_buffer_p(&self) -> bool {
        let mut vlist = self.find_symbol_value(sym::OVERLAY_ARROW_VARIABLE_LIST);
        while vlist.is_cons() {
            let var = self.xcar(vlist);
            vlist = self.xcdr(vlist);
            if !var.is_symbol() {
                continue;
            }
            let val = self.find_symbol_value(var);
            if self.is_marker(val) && self.marker(val).buffer == self.current_buffer {
                return true;
            }
        }
        false
    }

    pub fn compute_window_start_on_continuation_line(&mut self, w: Value) -> LispResult<bool> {
        self.with_gc_deferred(|lisp| lisp.compute_window_start_on_continuation_line_1(w))
    }

    fn compute_window_start_on_continuation_line_1(&mut self, w: Value) -> LispResult<bool> {
        let m = *self.marker(self.window(w).start);
        let mut start_pos = TextPos::new(m.charpos as i64, m.bytepos as i64);
        let mut window_start_changed_p = false;
        let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
        if start_pos.charpos > begv
            && self.cur().text.byte_at((start_pos.bytepos - 1) as usize) != b'\n'
        {
            if start_pos.charpos < begv {
                start_pos = TextPos::new(begv, self.cur().begv_byte as i64);
            } else if start_pos.charpos > zv {
                start_pos = TextPos::new(zv, self.cur().zv_byte as i64);
            }
            let vpos = usize::from(self.window_wants_tab_line(w))
                + usize::from(self.window_wants_header_line(w));
            let row = GlyphRowRef::Window {
                window: w,
                desired: true,
                vpos,
            };
            let mut it = It::default();
            self.init_iterator(
                &mut it,
                w,
                start_pos.charpos,
                start_pos.bytepos,
                Some(row),
                DEFAULT_FACE_ID,
            )?;
            self.reseat_at_previous_visible_line_start(&mut it)?;
            let pt = self.cur().pt as i64;
            let (total_lines, total_cols) = {
                let win = self.window(w);
                (win.total_lines, win.total_cols)
            };
            if it.charpos() <= pt && start_pos.charpos - it.charpos() < total_lines * total_cols {
                let mut pos = it.current.pos;
                let mut pos_before_pt = pos;
                let mut min_distance = DISP_INFINITY;
                loop {
                    let distance = (start_pos.charpos - it.charpos()).abs();
                    if distance >= min_distance {
                        break;
                    }
                    min_distance = distance;
                    if pos.charpos <= pt {
                        pos_before_pt = pos;
                    }
                    pos = it.current.pos;
                    if it.line_wrap == LineWrap::WordWrap {
                        let (to_x, to_y) = (it.last_visible_x, it.current_y);
                        self.move_it_to(
                            &mut it,
                            zv,
                            to_x,
                            to_y,
                            -1,
                            MOVE_TO_POS | MOVE_TO_X | MOVE_TO_Y,
                        )?;
                        let to_y = it.current_y + it.max_ascent + it.max_descent;
                        self.move_it_to(
                            &mut it,
                            zv,
                            0,
                            to_y,
                            -1,
                            MOVE_TO_POS | MOVE_TO_X | MOVE_TO_Y,
                        )?;
                    } else {
                        self.move_it_by_lines(&mut it, 1)?;
                    }
                }
                if pos.charpos > pt {
                    pos = pos_before_pt;
                }
                let start = self.window(w).start;
                let buffer = self.current_buffer;
                self.set_marker_position(start, pos.charpos as usize, buffer);
                window_start_changed_p = true;
            }
        }
        Ok(window_start_changed_p)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_scrolling(
        &mut self,
        window: Value,
        just_this_one_p: bool,
        arg_scroll_conservatively: i64,
        scroll_step: i64,
        temp_scroll_step: bool,
        last_line_misfit: bool,
    ) -> LispResult<i32> {
        self.with_gc_deferred(|lisp| {
            lisp.try_scrolling_1(
                window,
                just_this_one_p,
                arg_scroll_conservatively,
                scroll_step,
                temp_scroll_step,
                last_line_misfit,
            )
        })
    }

    fn aggressive_scroll(
        &mut self,
        w: Value,
        aggressive: Value,
        this_scroll_margin: i32,
        dy: i32,
    ) -> i32 {
        let height = self.window_box_text_height(w) as i32;
        let float_amount = self.xfloatint(aggressive) * f64::from(height);
        let mut aggressive_scroll = float_amount as i32;
        if aggressive_scroll == 0 && float_amount > 0.0 {
            aggressive_scroll = 1;
        }
        if aggressive_scroll + 2 * this_scroll_margin > height {
            aggressive_scroll = height - 2 * this_scroll_margin;
        }
        dy + aggressive_scroll
    }

    #[allow(clippy::too_many_arguments)]
    fn try_scrolling_1(
        &mut self,
        window: Value,
        _just_this_one_p: bool,
        arg_scroll_conservatively: i64,
        scroll_step: i64,
        temp_scroll_step: bool,
        last_line_misfit: bool,
    ) -> LispResult<i32> {
        let w = window;
        let mut arg_scroll_conservatively = arg_scroll_conservatively;
        let mut dy = 0;
        let mut amount_to_scroll = 0;
        let mut extra_scroll_margin_lines = i32::from(last_line_misfit);
        let scroll_limit = SCROLL_LIMIT;
        let frame_line_height = self.default_line_pixel_height(w);
        let m = *self.marker(self.window(w).start);
        let mut startp = TextPos::new(m.charpos as i64, m.bytepos as i64);
        let this_scroll_margin = self.window_scroll_margin(w, MarginUnit::InPixels);
        let contents = self.current_buffer;
        let scroll_max = if arg_scroll_conservatively > scroll_limit {
            arg_scroll_conservatively = scroll_limit + 1;
            scroll_limit as i32 * frame_line_height
        } else if 0 < scroll_step || 0 < arg_scroll_conservatively || temp_scroll_step {
            let scroll_lines_max =
                scroll_step.max(arg_scroll_conservatively.max(i64::from(temp_scroll_step)));
            let scroll_lines = scroll_lines_max.clamp(0, 1_000_000) as i32;
            scroll_lines * frame_line_height
        } else if self.is_number(self.buffer_local_value(sym::SCROLL_DOWN_AGGRESSIVELY, contents))
            || self.is_number(self.buffer_local_value(sym::SCROLL_UP_AGGRESSIVELY, contents))
        {
            10 * frame_line_height
        } else {
            0
        };
        loop {
            let mut it = It::default();
            let mut scroll_down_p = false;
            let pt = self.cur().pt as i64;
            let zv = self.cur().zv as i64;
            if pt > startp.charpos {
                self.start_display(&mut it, w, startp)?;
                let scroll_margin_y = it.last_visible_y
                    - self.partial_line_height(&it)?
                    - this_scroll_margin
                    - frame_line_height * extra_scroll_margin_lines;
                self.move_it_to(
                    &mut it,
                    pt,
                    -1,
                    scroll_margin_y - 1,
                    -1,
                    MOVE_TO_POS | MOVE_TO_Y,
                )?;
                if pt > it.current.pos.charpos {
                    let y0 = self.line_bottom_y(&it)?;
                    let slack = scroll_max.max(10 * frame_line_height);
                    let y_to_move = it.last_visible_y + slack;
                    self.move_it_to(&mut it, pt, -1, y_to_move, -1, MOVE_TO_POS | MOVE_TO_Y)?;
                    dy = self.line_bottom_y(&it)? - y0;
                    if dy > scroll_max {
                        return Ok(SCROLLING_FAILED);
                    }
                    if dy > 0 {
                        scroll_down_p = true;
                    }
                } else if pt == it.charpos()
                    && it.charpos() < zv
                    && it.method == ItMethod::FromString
                    && arg_scroll_conservatively > scroll_limit
                    && it.current_x == 0
                {
                    let y1 = it.current_y;
                    loop {
                        let skip = self.move_it_in_display_line_to(&mut it, zv, -1, MOVE_TO_POS)?;
                        if skip != MoveItResult::NewlineOrCr
                            || it.charpos() != pt
                            || it.method == ItMethod::FromBuffer
                        {
                            break;
                        }
                        let vpos = it.vpos;
                        self.move_it_to(&mut it, -1, -1, -1, vpos + 1, MOVE_TO_VPOS)?;
                        if it.vpos <= vpos {
                            break;
                        }
                    }
                    dy = it.current_y - y1;
                    if dy > scroll_max {
                        return Ok(SCROLLING_FAILED);
                    }
                    if dy > 0 {
                        scroll_down_p = true;
                    }
                } else if scroll_margin_y < 0 {
                    scroll_down_p = true;
                }
            }
            if scroll_down_p {
                if arg_scroll_conservatively != 0 {
                    amount_to_scroll = dy
                        .max(frame_line_height)
                        .min((i64::from(frame_line_height) * arg_scroll_conservatively) as i32);
                } else if scroll_step != 0 || temp_scroll_step {
                    amount_to_scroll = scroll_max;
                } else {
                    let aggressive = self.buffer_local_value(sym::SCROLL_UP_AGGRESSIVELY, contents);
                    if self.is_number(aggressive) {
                        amount_to_scroll =
                            self.aggressive_scroll(w, aggressive, this_scroll_margin, dy);
                    }
                }
                if amount_to_scroll <= 0 {
                    return Ok(SCROLLING_FAILED);
                }
                self.start_display(&mut it, w, startp)?;
                if arg_scroll_conservatively <= scroll_limit {
                    self.move_it_vertically(&mut it, amount_to_scroll)?;
                } else {
                    let mut it1data: ShelvedBidiCache = None;
                    let mut it1 = self.save_it(&it, &mut it1data);
                    let start_y = self.line_bottom_y(&it1)?;
                    loop {
                        self.restore_it(&mut it, None, &mut it1data);
                        self.move_it_by_lines(&mut it, 1)?;
                        it1 = self.save_it(&it, &mut it1data);
                        let zv = self.cur().zv as i64;
                        if !(it.charpos() < zv
                            && self.line_bottom_y(&it1)? - start_y < amount_to_scroll)
                        {
                            break;
                        }
                    }
                    self.bidi_unshelve_cache(it1data.take(), true);
                }
                if it.charpos() == startp.charpos {
                    self.move_it_by_lines(&mut it, 1)?;
                }
                startp = it.current.pos;
            } else {
                let mut scroll_margin_pos = startp;
                let mut y_offset = 0;
                if this_scroll_margin != 0 {
                    self.start_display(&mut it, w, startp)?;
                    let y_start = it.current_y;
                    self.move_it_vertically(&mut it, this_scroll_margin)?;
                    scroll_margin_pos = it.current.pos;
                    if it.charpos() == zv && it.current_y - y_start < this_scroll_margin {
                        y_offset = this_scroll_margin - (it.current_y - y_start);
                    }
                }
                if pt < scroll_margin_pos.charpos {
                    let pos = TextPos::new(pt, self.cur().pt_byte as i64);
                    self.start_display(&mut it, w, pos)?;
                    let y0 = it.current_y;
                    let y_to_move = it
                        .last_visible_y
                        .max(scroll_max.max(10 * frame_line_height));
                    self.move_it_to(
                        &mut it,
                        scroll_margin_pos.charpos,
                        0,
                        y_to_move,
                        -1,
                        MOVE_TO_POS | MOVE_TO_X | MOVE_TO_Y,
                    )?;
                    dy = it.current_y - y0;
                    if dy > scroll_max || it.charpos() < scroll_margin_pos.charpos {
                        return Ok(SCROLLING_FAILED);
                    }
                    dy += y_offset;
                    self.start_display(&mut it, w, startp)?;
                    if arg_scroll_conservatively != 0 {
                        amount_to_scroll = dy
                            .max(frame_line_height)
                            .min((i64::from(frame_line_height) * arg_scroll_conservatively) as i32);
                    } else if scroll_step != 0 || temp_scroll_step {
                        amount_to_scroll = scroll_max;
                    } else {
                        let aggressive =
                            self.buffer_local_value(sym::SCROLL_DOWN_AGGRESSIVELY, contents);
                        if self.is_number(aggressive) {
                            amount_to_scroll =
                                self.aggressive_scroll(w, aggressive, this_scroll_margin, dy);
                        }
                    }
                    if amount_to_scroll <= 0 {
                        return Ok(SCROLLING_FAILED);
                    }
                    self.move_it_vertically_backward(&mut it, amount_to_scroll)?;
                    startp = it.current.pos;
                }
            }
            startp = self.run_window_scroll_functions(window, startp)?;
            if self.try_window(window, startp, 0)? == 0 {
                return Ok(SCROLLING_NEED_LARGER_MATRICES);
            }
            if self.window(w).cursor.vpos < 0 {
                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                    clear_glyph_matrix(m);
                }
                return Ok(SCROLLING_FAILED);
            }
            let base_line_pos = self.window(w).base_line_pos;
            if !(!self.cur().clip_changed && self.cur().text.beg_unchanged >= base_line_pos) {
                self.window_mut(w).base_line_number = 0;
            }
            if !self.cursor_row_fully_visible_p(w, extra_scroll_margin_lines <= 1, false, false)? {
                let vpos = self.window(w).cursor.vpos as usize;
                let row = self.matrix_row(w, WhichMatrix::Desired, vpos).clone();
                if !self.matrix_row_partially_visible_at_top_p(w, &row) {
                    if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                        clear_glyph_matrix(m);
                    }
                    extra_scroll_margin_lines += 1;
                    continue;
                }
            }
            return Ok(SCROLLING_SUCCESS);
        }
    }

    pub fn try_cursor_movement(
        &mut self,
        window: Value,
        startp: TextPos,
        scroll_step: &mut bool,
    ) -> LispResult<i32> {
        let w = window;
        let f = self.window(w).frame;
        let mut rc = CURSOR_MOVEMENT_CANNOT_BE_USED;
        let pt = self.cur().pt as i64;
        let display_line_numbers = self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS);
        if !(pt >= startp.charpos
            && !self.cur().clip_changed
            && self.windows.update_mode_lines == 0
            && self.windows.windows_or_buffers_changed == 0
            && !self.frame(f).flags.cursor_type_changed
            && self
                .find_symbol_value(sym::SHOW_TRAILING_WHITESPACE)
                .is_nil()
            && display_line_numbers != sym::RELATIVE
            && display_line_numbers != sym::VISUAL
            && !(!display_line_numbers.is_nil()
                && crate::xfaces::f_internal_lisp_face_equal_p(
                    self,
                    &[sym::LINE_NUMBER, sym::LINE_NUMBER_CURRENT_LINE, f, sym::T],
                )?
                .is_nil())
            && window != self.windows.minibuf_window
            && !self.overlay_arrow_in_current_buffer_p())
        {
            return Ok(rc);
        }
        let this_scroll_margin = self.window_scroll_margin(w, MarginUnit::InPixels);
        let mut top_scroll_margin = this_scroll_margin;
        if self.window_wants_tab_line(w) {
            top_scroll_margin += self.current_tab_line_height(w) as i32;
        }
        if self.window_wants_header_line(w) {
            top_scroll_margin += self.current_header_line_height(w) as i32;
        }
        let cur = WhichMatrix::Current;
        let (nrows, first_text_row, mode_line_row) = {
            let m = self
                .window_matrix(w, cur)
                .expect("window has no current matrix");
            (
                m.nrows as isize,
                m.first_text_row() as isize,
                m.mode_line_row() as isize,
            )
        };
        let row_at = |lisp: &Lisp, row: isize| lisp.matrix_row(w, cur, row as usize).clone();
        let last_cursor_vpos = self.window(w).last_cursor_vpos as isize;
        let mut row: isize = 0;
        if last_cursor_vpos < 0 || last_cursor_vpos >= nrows {
            rc = CURSOR_MOVEMENT_MUST_SCROLL;
        } else {
            row = last_cursor_vpos;
            if row_at(self, row).tab_line_p {
                row += 1;
            }
            if row_at(self, row).mode_line_p {
                row += 1;
            }
            if !row_at(self, row).enabled_p {
                rc = CURSOR_MOVEMENT_MUST_SCROLL;
            }
        }
        if rc != CURSOR_MOVEMENT_CANNOT_BE_USED {
            return Ok(rc);
        }
        let mut scroll_p = false;
        let mut must_scroll = false;
        let last_y = (self.window_text_bottom_y(w) - i64::from(this_scroll_margin)) as i32;
        let last_point = self.window(w).last_point;
        let begv = self.cur().begv as i64;
        if pt > last_point {
            while row_at(self, row).end_charpos() < pt && row_at(self, row).bottom_y() < last_y {
                row += 1;
            }
            loop {
                let r = row_at(self, row);
                if !(r.bottom_y() < last_y
                    && r.end_charpos() == pt
                    && row < mode_line_row
                    && row_at(self, row + 1).start_charpos() >= pt
                    && !self.cursor_row_p(&r)?)
                {
                    break;
                }
                row += 1;
            }
            let r = row_at(self, row);
            if r.bottom_y() > last_y
                || pt > r.end_charpos()
                || (r.bottom_y() == last_y
                    && pt == r.end_charpos()
                    && !r.ends_at_zv_p
                    && !ends_in_middle_of_char_p(&r))
            {
                scroll_p = true;
            }
        } else if pt < last_point {
            let row0 = row;
            while row >= 0 {
                let r = row_at(self, row);
                if !(!r.mode_line_p
                    && (r.start_charpos() > pt
                        || (r.start_charpos() == pt
                            && (starts_in_middle_of_char_p(&r)
                                || (row > 0
                                    && row_at(self, row - 1).ends_in_newline_from_string_p))))
                    && (r.y > top_scroll_margin || startp.charpos == begv))
                {
                    break;
                }
                row -= 1;
            }
            let outside = row < 0 || {
                let r = row_at(self, row);
                !(r.start_charpos() <= pt && pt <= r.end_charpos())
            };
            if outside && row_at(self, row0).continued_p {
                row = row0;
                while row_at(self, row).start_charpos() > pt
                    && row_at(self, row).bottom_y() < last_y
                {
                    row += 1;
                }
            }
            if row < 0 || row_at(self, row).mode_line_p {
                row = 0;
                if row_at(self, row).tab_line_p {
                    row += 1;
                }
                if row_at(self, row).mode_line_p {
                    row += 1;
                }
            }
            loop {
                let r = row_at(self, row);
                if !(r.bottom_y() < last_y && r.end_charpos() == pt && !self.cursor_row_p(&r)?) {
                    break;
                }
                row += 1;
            }
            let r = row_at(self, row);
            if (r.y < top_scroll_margin && startp.charpos != begv)
                || r.bottom_y() > last_y
                || pt > r.end_charpos()
                || (r.bottom_y() == last_y
                    && pt == r.end_charpos()
                    && !r.ends_at_zv_p
                    && !ends_in_middle_of_char_p(&r))
            {
                scroll_p = true;
            }
        } else {
            rc = CURSOR_MOVEMENT_SUCCESS;
        }
        let contents = self.window(w).contents;
        let reordering = !self
            .buffer_local_value(sym::BIDI_DISPLAY_REORDERING, contents)
            .is_nil();
        let r = row_at(self, row);
        if (pt < r.start_charpos()
            && (row == first_text_row || row_at(self, row - 1).end_charpos() == r.start_charpos()))
            || pt > r.end_charpos()
        {
            rc = CURSOR_MOVEMENT_MUST_SCROLL;
            must_scroll = true;
        } else if rc != CURSOR_MOVEMENT_SUCCESS && reordering {
            let row1 = first_text_row;
            while row_at(self, row).continuation_line_p() {
                if row <= row1 {
                    rc = CURSOR_MOVEMENT_MUST_SCROLL;
                    break;
                }
                row -= 1;
            }
        }
        let r = row_at(self, row);
        if must_scroll {
        } else if rc != CURSOR_MOVEMENT_SUCCESS
            && self.matrix_row_partially_visible_p(w, &r)
            && !r.mode_line_p
            && !self.cursor_row_fully_visible_p(w, true, true, true)?
        {
            if pt == r.end_charpos() && !r.ends_at_zv_p && !ends_in_middle_of_char_p(&r) {
                rc = CURSOR_MOVEMENT_MUST_SCROLL;
            } else if i64::from(r.height) > self.window_box_height(w) {
                *scroll_step = true;
                rc = CURSOR_MOVEMENT_MUST_SCROLL;
            } else {
                self.set_cursor_from_row(w, row as usize, cur, 0, 0, 0, 0)?;
                rc = if !self.cursor_row_fully_visible_p(w, false, true, false)? {
                    CURSOR_MOVEMENT_MUST_SCROLL
                } else {
                    CURSOR_MOVEMENT_SUCCESS
                };
            }
        } else if scroll_p {
            rc = CURSOR_MOVEMENT_MUST_SCROLL;
        } else if rc != CURSOR_MOVEMENT_SUCCESS && reordering {
            let mut rv = false;
            let mut pt_invis = false;
            let (val, _) =
                self.get_char_property_and_overlay(Value::fixnum(pt), sym::INVISIBLE, Value::NIL)?;
            if self.text_prop_means_invisible(val) != 0 {
                pt_invis = true;
            }
            loop {
                let r = row_at(self, row);
                if (pt_invis || r.start_charpos() <= pt)
                    && pt <= r.end_charpos()
                    && self.cursor_row_p(&r)?
                {
                    rv |= self.set_cursor_from_row(w, row as usize, cur, 0, 0, 0, 0)?;
                }
                if rv {
                    let cursor = self.window(w).cursor;
                    let candidate = row_at(self, cursor.vpos as isize);
                    let at_zv_p = candidate.ends_at_zv_p;
                    let mut exact_match_p = false;
                    if !at_zv_p
                        && cursor.hpos >= 0
                        && cursor.hpos < i64::from(candidate.used[TEXT_AREA])
                    {
                        let g = candidate.glyphs[candidate.area[TEXT_AREA] + cursor.hpos as usize];
                        let endpos = candidate.end_charpos();
                        exact_match_p = (self.is_buffer(g.object) && g.charpos == pt)
                            || (g.object.is_nil()
                                && (g.charpos == pt || (g.charpos == 0 && endpos - 1 == pt)));
                        if !exact_match_p && pt_invis {
                            exact_match_p = true;
                        }
                    }
                    if at_zv_p || exact_match_p {
                        rc = CURSOR_MOVEMENT_SUCCESS;
                        break;
                    }
                }
                if r.bottom_y() == last_y {
                    break;
                }
                row += 1;
                let r = row_at(self, row);
                if !(((r.continuation_line_p() || r.continued_p) && r.bottom_y() <= last_y)
                    || (r.start_charpos() == pt && r.bottom_y() < last_y))
                {
                    break;
                }
            }
            let r = row_at(self, row);
            if rc != CURSOR_MOVEMENT_SUCCESS && !(rv && !r.continuation_line_p() && !r.continued_p)
            {
                rc = CURSOR_MOVEMENT_MUST_SCROLL;
            } else if rv {
                rc = CURSOR_MOVEMENT_SUCCESS;
            }
        } else {
            loop {
                if self.set_cursor_from_row(w, row as usize, cur, 0, 0, 0, 0)? {
                    rc = CURSOR_MOVEMENT_SUCCESS;
                    break;
                }
                row += 1;
                let r = row_at(self, row);
                if !(r.bottom_y() < last_y && r.start_charpos() == pt && self.cursor_row_p(&r)?) {
                    break;
                }
            }
        }
        Ok(rc)
    }
}
