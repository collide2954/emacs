use crate::dispextern::{
    GlyphRowArea, It, ItMethod, LineWrap, MOVE_TO_POS, MOVE_TO_X, MOVE_TO_Y, TextPos,
};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::terminal::OutputMethod;
use crate::value::Value;
use crate::window::MarginUnit;
use crate::xdisp::line::TRY_WINDOW_CHECK_MARGINS;
use crate::xdisp::modeline::ModeLineTarget;
use crate::xdisp::scroll::{
    CURSOR_MOVEMENT_CANNOT_BE_USED, CURSOR_MOVEMENT_MUST_SCROLL, CURSOR_MOVEMENT_SUCCESS,
    SCROLL_LIMIT, SCROLLING_FAILED, SCROLLING_NEED_LARGER_MATRICES, SCROLLING_SUCCESS,
};
use crate::xfaces::DEFAULT_FACE_ID;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    Body,
    ForceStart,
    IgnoreStart,
    TryToScroll,
    Recenter,
    Done,
    FinishMenuBars,
    NeedLargerMatrices,
    FinishScrollBars,
}

impl Lisp {
    pub(crate) fn redisplay_some_p(&self) -> bool {
        let w = &self.windows;
        (w.windows_or_buffers_changed == 0
            || w.windows_or_buffers_changed == crate::xdisp::REDISPLAY_SOME)
            && (w.update_mode_lines == 0 || w.update_mode_lines == crate::xdisp::REDISPLAY_SOME)
    }

    pub(super) fn needs_no_redisplay(&self, w: Value) -> bool {
        let win = self.window(w);
        let buffer = self.buffer(win.contents);
        let f = self.frame(win.frame);
        self.redisplay_some_p()
            && !win.flags.redisplay
            && !win.flags.update_mode_line
            && !f.flags.face_change
            && !f.flags.redisplay
            && !buffer.text.redisplay
            && self.window_point(w) == win.last_point
    }

    pub(super) fn mode_line_update_needed(&mut self, w: Value) -> LispResult<bool> {
        let displayed = self.window(w).column_number_displayed;
        if displayed == -1 {
            return Ok(false);
        }
        if self.cur().pt as i64 == self.window(w).last_point && !self.window_outdated(w) {
            return Ok(false);
        }
        Ok(displayed != self.current_column()?)
    }

    pub(crate) fn window_frozen_p(&self, w: Value) -> bool {
        let f = self.window(w).frame;
        if self.frame(f).flags.frozen_window_starts {
            let selected = self.windows.selected_window;
            if self.window(w).mini || w == selected {
                return false;
            }
            if self.window(selected).mini && w == self.plain_value(sym::MINIBUFFER_SCROLL_WINDOW) {
                return false;
            }
            return true;
        }
        false
    }

    fn check_point_in_composition(
        &mut self,
        prev_buf: Value,
        prev_pt: i64,
        buf: Value,
        pt: i64,
    ) -> LispResult<bool> {
        let (begv, zv) = {
            let b = self.buffer(buf);
            (b.begv as i64, b.zv as i64)
        };
        if prev_buf == buf {
            if prev_pt == pt {
                return Ok(false);
            }
            if prev_pt > begv
                && prev_pt < zv
                && let Some((start, end, prop)) = self.find_composition(prev_pt, -1, buf)?
                && self.composition_valid_p(start, end, prop)
                && (start as i64) < prev_pt
                && (end as i64) > prev_pt
            {
                return Ok(pt <= start as i64 || pt >= end as i64);
            }
        }
        if pt > begv
            && pt < zv
            && let Some((start, end, prop)) = self.find_composition(pt, -1, buf)?
        {
            return Ok(self.composition_valid_p(start, end, prop)
                && (start as i64) < pt
                && (end as i64) > pt);
        }
        Ok(false)
    }

    pub(super) fn reconsider_clip_changes(&mut self, w: Value) -> LispResult<()> {
        let b = self.window(w).contents;
        let window_end_valid = self.window(w).flags.window_end_valid;
        let (matrix_buffer, matrix_zv, matrix_begv) = self
            .window_matrix(w, WhichMatrix::Current)
            .map_or((Value::NIL, -1, -1), |m| (m.buffer, m.zv, m.begv));
        let (zv, begv) = {
            let buf = self.buffer(b);
            (buf.zv as i64, buf.begv as i64)
        };
        if self.buffer(b).clip_changed
            && window_end_valid
            && matrix_buffer == b
            && matrix_zv == zv
            && matrix_begv == begv
        {
            self.buffer_mut(b).clip_changed = false;
        }
        if !self.buffer(b).clip_changed && window_end_valid {
            let pt = if w == self.windows.selected_window {
                self.cur().pt as i64
            } else {
                self.marker(self.window(w).pointm).charpos as i64
            };
            let last_point = self.window(w).last_point;
            if (matrix_buffer != b || pt != last_point)
                && self.check_point_in_composition(matrix_buffer, last_point, b, pt)?
            {
                self.buffer_mut(b).clip_changed = true;
            }
        }
        Ok(())
    }

    pub(crate) fn overlay_touches_p(&self, pos: i64) -> bool {
        let b = self.cur();
        b.overlays.iter().any(|&o| {
            let ov = self.overlay(o);
            ov.start as i64 == pos || ov.end as i64 == pos
        })
    }

    fn redisplay_tab_bar(&mut self, f: Value) -> bool {
        self.frame_mut(f).flags.tab_bar_redisplayed = true;
        let w = self.frame(f).tab_bar_window;
        if !self.is_window(w) || self.window(w).total_lines == 0 {
            self.frame_mut(f).flags.tab_bar_resized = true;
            return false;
        }
        unreachable!("tab bars need display_tab_bar_line");
    }

    fn display_menu_bar(&mut self, w: Value) {
        let f = self.window(w).frame;
        if self.frame(f).output_method == OutputMethod::Pgtk {
            return;
        }
        unreachable!("tty frames have no glyph matrices");
    }

    fn temp_set_pt_both(&mut self, charpos: i64, bytepos: i64) {
        self.set_pt_both(charpos as usize, bytepos as usize);
    }

    fn matrix_row_bottom_y(&self, w: Value, which: WhichMatrix, vpos: usize) -> i32 {
        let row = &self.window_matrix(w, which).expect("glyph matrix").rows[vpos];
        row.y + row.height
    }

    fn clear_desired_matrix(&mut self, w: Value) {
        if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
            crate::dispnew::matrix::clear_glyph_matrix(m);
        }
    }

    fn desired_line_height(
        &self,
        w: Value,
        row: fn(&crate::dispextern::GlyphMatrix) -> usize,
    ) -> i64 {
        self.window_matrix(w, WhichMatrix::Desired)
            .filter(|m| !m.rows.is_empty())
            .map_or(0, |m| i64::from(m.rows[row(m)].height))
    }

    fn invisible_alternative_pos(&mut self) -> LispResult<Option<i64>> {
        let pt = Value::fixnum(self.cur().pt as i64);
        let (invprop, _) = self.get_char_property_and_overlay(pt, sym::INVISIBLE, Value::NIL)?;
        if self.text_prop_means_invisible(invprop) == 0 {
            return Ok(None);
        }
        let end = crate::textprop::f_next_single_char_property_change(
            self,
            &[pt, sym::INVISIBLE, Value::NIL, Value::NIL],
        )?;
        Ok(Some(match end.as_fixnum() {
            Some(n) if n >= 0 => n,
            _ => self.cur().zv as i64,
        }))
    }

    pub fn redisplay_window(&mut self, window: Value, just_this_one_p: bool) -> LispResult<()> {
        self.with_gc_deferred(|lisp| lisp.redisplay_window_1(window, just_this_one_p))
    }

    fn redisplay_window_1(&mut self, window: Value, just_this_one_p: bool) -> LispResult<()> {
        let w = window;
        let f = self.window(w).frame;
        let buffer = self.window(w).contents;
        let old = self.current_buffer;
        let mut lpoint = TextPos::new(self.cur().pt as i64, self.cur().pt_byte as i64);
        let mut opoint = lpoint;
        let mut current_matrix_up_to_date_p = false;
        let mut used_current_matrix_p = false;
        let mut temp_scroll_step = false;
        let count = self.specpdl_index();
        let mut centering_position = -1;
        let mut last_line_misfit = false;
        let lchars_modiff = self.cur().text.chars_modiff;
        let mut ochars_modiff = lchars_modiff;
        let mut beg_unchanged = 0;
        let mut end_unchanged = 0;
        let mut startp = TextPos::default();
        let mut it = It::default();
        if !just_this_one_p && self.needs_no_redisplay(w) {
            return Ok(());
        }
        self.reconsider_clip_changes(w)?;
        let frame_line_height = self.default_line_pixel_height(w);
        let mut margin = self.window_scroll_margin(w, MarginUnit::InLines);
        let mut update_mode_line = self.window(w).flags.update_mode_line
            || self.windows.update_mode_lines != 0
            || self.buffer(buffer).clip_changed
            || self.buffer(buffer).prevent_redisplay_optimizations_p;
        if !just_this_one_p {
            self.window_mut(w).flags.must_be_updated_p = true;
        }
        let mut step = Step::Body;
        if self.window(w).mini {
            if w == self.xdisp.echo_area_window && !self.xdisp.echo_area_buffer[0].is_nil() {
                step = if update_mode_line {
                    Step::FinishMenuBars
                } else {
                    Step::FinishScrollBars
                };
            } else if (w != self.windows.minibuf_window || self.minibuf.minibuf_level == 0)
                && self.buffer(buffer).z() == 1
                && !self.memq(buffer, self.minibuf.minibuffer_list).is_nil()
            {
                let yb = self.window_text_bottom_y(w) as i32;
                let mut y = 0;
                let mut vpos = 0;
                while y < yb {
                    self.blank_row(w, WhichMatrix::Desired, vpos, y);
                    y += self
                        .window_matrix(w, WhichMatrix::Desired)
                        .expect("glyph matrix")
                        .rows[vpos]
                        .height;
                    vpos += 1;
                }
                step = Step::FinishScrollBars;
            } else if self.minibuf.minibuf_level >= 1
                && self.find_symbol_value(sym::RESIZE_MINI_FRAMES).is_nil()
            {
                self.resize_mini_window(w, false)?;
            }
            if step == Step::Body {
                self.clear_desired_matrix(w);
            }
        }
        loop {
            match step {
                Step::Body => {
                    self.set_buffer_internal(buffer);
                    current_matrix_up_to_date_p = self.window(w).flags.window_end_valid
                        && !self.cur().clip_changed
                        && !self.cur().prevent_redisplay_optimizations_p
                        && !self.window_outdated(w)
                        && !self.var_true(sym::COMPOSITION_BREAK_AT_POINT)
                        && !self.hscrolling_current_line_p(w);
                    beg_unchanged = self.cur().text.beg_unchanged;
                    end_unchanged = self.cur().text.end_unchanged;
                    opoint = TextPos::new(self.cur().pt as i64, self.cur().pt_byte as i64);
                    ochars_modiff = self.cur().text.chars_modiff;
                    self.specbind(sym::INHIBIT_POINT_MOTION_HOOKS, sym::T)?;
                    if self.windows.windows_or_buffers_changed != 0 {
                        let start = self.window(w).start;
                        if self.marker(start).buffer == self.current_buffer {
                            self.compute_window_start_on_continuation_line(w)?;
                        }
                        self.window_mut(w).flags.window_end_valid = false;
                        current_matrix_up_to_date_p = false;
                    }
                    let (z, z_byte) = (self.cur().z() as i64, self.cur().z_byte() as i64);
                    if (z == z_byte && opoint.charpos != opoint.bytepos)
                        || opoint.bytepos < opoint.charpos
                    {
                        unreachable!("emacs_abort in redisplay_window");
                    }
                    if self.mode_line_update_needed(w)? {
                        update_mode_line = true;
                    }
                    if window != self.windows.selected_window {
                        let pointm = self.window(w).pointm;
                        let (mut new_pt, mut new_pt_byte) = {
                            let m = self.marker(pointm);
                            (m.charpos as i64, m.bytepos as i64)
                        };
                        let b = self.cur();
                        let (begv, begv_byte, zv, zv_byte) = (
                            b.begv as i64,
                            b.begv_byte as i64,
                            b.zv as i64,
                            b.zv_byte as i64,
                        );
                        let cb = self.current_buffer;
                        if new_pt < begv {
                            new_pt = begv;
                            new_pt_byte = begv_byte;
                            self.set_marker_position(pointm, begv as usize, cb);
                        } else if new_pt > zv - 1 {
                            new_pt = zv;
                            new_pt_byte = zv_byte;
                            self.set_marker_position(pointm, zv as usize, cb);
                        }
                        self.temp_set_pt_both(new_pt, new_pt_byte);
                    }
                    self.check_long_lines()?;
                    let start = self.window(w).start;
                    if self.marker(start).buffer != self.current_buffer {
                        step = Step::Recenter;
                        continue;
                    }
                    startp = {
                        let m = self.marker(start);
                        TextPos::new(m.charpos as i64, m.bytepos as i64)
                    };
                    let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
                    if (self.window(w).flags.optional_new_start || self.window_frozen_p(w))
                        && startp.charpos >= begv
                        && startp.charpos <= zv
                    {
                        self.window_mut(w).flags.optional_new_start = false;
                        if !self.window(w).flags.force_start {
                            self.start_display(&mut it, w, startp)?;
                            let pt = self.cur().pt as i64;
                            let lvy = it.last_visible_y;
                            self.move_it_to(
                                &mut it,
                                pt,
                                0,
                                lvy,
                                -1,
                                MOVE_TO_POS | MOVE_TO_X | MOVE_TO_Y,
                            )?;
                            let it_charpos = it.charpos();
                            if (it.current_y == 0 || self.line_bottom_y(&it)? < it.last_visible_y)
                                && (it_charpos == pt || (it_charpos > pt && startp.charpos <= pt))
                            {
                                self.window_mut(w).flags.force_start = true;
                            }
                        }
                    }
                    if !self.base_line_number_valid_p(w) {
                        self.window_mut(w).base_line_number = 0;
                    }
                    step = Step::ForceStart;
                }
                Step::ForceStart => {
                    step = Step::IgnoreStart;
                    if !self.window(w).flags.force_start {
                        continue;
                    }
                    let mut new_vpos = -1;
                    self.window_mut(w).flags.force_start = false;
                    if !self.window(w).flags.preserve_vscroll_p && !self.window_frozen_p(w) {
                        self.window_mut(w).vscroll = 0;
                    }
                    self.window_mut(w).flags.preserve_vscroll_p = false;
                    self.window_mut(w).flags.window_end_valid = false;
                    if !update_mode_line
                        || !self
                            .find_symbol_value(sym::WINDOW_SCROLL_FUNCTIONS)
                            .is_nil()
                    {
                        update_mode_line = true;
                        self.window_mut(w).flags.update_mode_line = true;
                        startp = self.run_window_scroll_functions(window, startp)?;
                    }
                    let b = self.cur();
                    let (begv, begv_byte, zv, zv_byte) = (
                        b.begv as i64,
                        b.begv_byte as i64,
                        b.zv as i64,
                        b.zv_byte as i64,
                    );
                    if startp.charpos < begv {
                        startp = TextPos::new(begv, begv_byte);
                    } else if startp.charpos > zv {
                        startp = TextPos::new(zv, zv_byte);
                    }
                    if !self.window_start_acceptable_p(window, startp.charpos)? {
                        continue;
                    }
                    self.clear_desired_matrix(w);
                    if self.try_window(window, startp, 0)? == 0 {
                        self.window_mut(w).flags.force_start = true;
                        self.clear_desired_matrix(w);
                        step = Step::NeedLargerMatrices;
                        continue;
                    }
                    if self.window(w).cursor.vpos < 0 {
                        let mut r = None;
                        if let Some(alt_pt) = self.invisible_alternative_pos()? {
                            r = self.row_containing_pos(
                                w,
                                alt_pt,
                                WhichMatrix::Desired,
                                0,
                                None,
                                0,
                            )?;
                        }
                        new_vpos = match r {
                            Some(r) => self.matrix_row_bottom_y(w, WhichMatrix::Desired, r) as i64,
                            None => self.window_box_height(w) / 2,
                        };
                    }
                    if !self.cursor_row_fully_visible_p(w, false, false, false)? {
                        new_vpos = self.window_box_height(w);
                        if new_vpos >= self.window(w).cursor.y {
                            self.window_mut(w).cursor.vpos = -1;
                            self.clear_desired_matrix(w);
                            step = Step::TryToScroll;
                            continue;
                        }
                    } else if self.window(w).cursor.vpos >= 0 {
                        let pixel_margin = i64::from(margin * frame_line_height);
                        let tab_line = self.window_wants_tab_line(w);
                        let header_line = self.window_wants_header_line(w);
                        if self.window(w).cursor.vpos
                            < i64::from(margin) + i64::from(tab_line) + i64::from(header_line)
                        {
                            self.window_mut(w).cursor.vpos = -1;
                            self.clear_desired_matrix(w);
                            step = Step::TryToScroll;
                            continue;
                        }
                        let mut window_height = self.window_box_height(w);
                        if tab_line {
                            window_height += self.current_tab_line_height(w);
                        }
                        if header_line {
                            window_height += self.current_header_line_height(w);
                        }
                        if self.window(w).cursor.y >= window_height - pixel_margin {
                            self.window_mut(w).cursor.vpos = -1;
                            self.clear_desired_matrix(w);
                            step = Step::TryToScroll;
                            continue;
                        }
                    }
                    if new_vpos >= 0 {
                        let mut row = self
                            .window_matrix(w, WhichMatrix::Desired)
                            .expect("glyph matrix")
                            .first_text_row();
                        loop {
                            let m = self
                                .window_matrix(w, WhichMatrix::Desired)
                                .expect("glyph matrix");
                            let r = &m.rows[row];
                            if !(i64::from(r.y + r.height) < new_vpos && !r.ends_at_zv_p) {
                                break;
                            }
                            row += 1;
                        }
                        let minpos = self
                            .window_matrix(w, WhichMatrix::Desired)
                            .expect("glyph matrix")
                            .rows[row]
                            .minpos;
                        self.temp_set_pt_both(minpos.charpos, minpos.bytepos);
                        let (pt, cb) = (self.cur().pt, self.current_buffer);
                        if w != self.windows.selected_window {
                            let pointm = self.window(w).pointm;
                            self.set_marker_position(pointm, pt, cb);
                        } else if self.current_buffer == old {
                            lpoint = TextPos::new(pt as i64, self.cur().pt_byte as i64);
                        }
                        self.set_cursor_from_row(w, row, WhichMatrix::Desired, 0, 0, 0, 0)?;
                        self.window_mut(w).flags.redisplay = false;
                        let contents = self.window(w).contents;
                        self.buffer_mut(contents).text.redisplay = false;
                        let function = self.find_symbol_value(sym::PRE_REDISPLAY_FUNCTION);
                        let arg = self.list(&[window]);
                        self.safe_call(true, &[function, arg])?;
                        let lnum = self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS);
                        let first = self
                            .window_matrix(w, WhichMatrix::Desired)
                            .expect("glyph matrix")
                            .first_text_row();
                        if self.window(w).flags.redisplay
                            || self.buffer(contents).text.redisplay
                            || ((lnum == sym::RELATIVE || lnum == sym::VISUAL) && row != first)
                        {
                            self.clear_desired_matrix(w);
                            if self.try_window(window, startp, 0)? == 0 {
                                step = Step::NeedLargerMatrices;
                                continue;
                            }
                        }
                    }
                    if self.window(w).cursor.vpos < 0
                        || !self.cursor_row_fully_visible_p(w, false, false, false)?
                    {
                        self.clear_desired_matrix(w);
                        step = Step::TryToScroll;
                        continue;
                    }
                    step = Step::Done;
                }
                Step::IgnoreStart => {
                    step = Step::TryToScroll;
                    if current_matrix_up_to_date_p {
                        let rc = self.try_cursor_movement(window, startp, &mut temp_scroll_step)?;
                        if rc != CURSOR_MOVEMENT_CANNOT_BE_USED {
                            step = match rc {
                                CURSOR_MOVEMENT_SUCCESS => {
                                    used_current_matrix_p = true;
                                    Step::Done
                                }
                                CURSOR_MOVEMENT_MUST_SCROLL => Step::TryToScroll,
                                _ => unreachable!("emacs_abort in redisplay_window"),
                            };
                            continue;
                        }
                    }
                    let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
                    if self.window(w).flags.start_at_line_beg
                        && ((startp.charpos > begv
                            && self.cur().text.byte_at((startp.bytepos - 1) as usize) != b'\n')
                            || (startp.charpos >= begv
                                && startp.charpos <= zv
                                && !self.window_start_acceptable_p(window, startp.charpos)?))
                    {
                        step = Step::Recenter;
                        continue;
                    }
                    let tem = self.try_window_id(w)?;
                    if tem != 0 {
                        if self.frame(f).fonts_changed {
                            step = Step::NeedLargerMatrices;
                        } else if tem > 0 {
                            step = Step::Done;
                        }
                        continue;
                    }
                    let pt = self.cur().pt as i64;
                    if !(startp.charpos >= begv
                        && startp.charpos <= zv
                        && pt >= startp.charpos
                        && (startp.charpos < zv
                            || startp.charpos == begv
                            || !self.window_outdated(w)))
                    {
                        continue;
                    }
                    let z = self.cur().z() as i64;
                    let start = self.window(w).start;
                    if !self.window(w).flags.start_at_line_beg
                        && self.find_symbol_value(sym::TRACK_MOUSE).is_nil()
                        && startp.charpos > begv
                        && startp.charpos > 1 + beg_unchanged
                        && startp.charpos <= z - end_unchanged
                        && self.marker(start).buffer == self.current_buffer
                        && self.compute_window_start_on_continuation_line(w)?
                        && let Some(v) = self.pos_visible_p(w, pt)?
                        && (v.rtop != 0) == (v.rbot != 0)
                    {
                        self.window_mut(w).flags.force_start = true;
                        startp = {
                            let m = self.marker(start);
                            TextPos::new(m.charpos as i64, m.bytepos as i64)
                        };
                        step = Step::ForceStart;
                        continue;
                    }
                    if !self.window_start_acceptable_p(window, startp.charpos)? {
                        step = Step::Recenter;
                        continue;
                    }
                    let reuse = !(!current_matrix_up_to_date_p
                        || !self
                            .find_symbol_value(sym::WINDOW_SCROLL_FUNCTIONS)
                            .is_nil()
                        || self.window(w).mini)
                        && {
                            used_current_matrix_p = self.try_window_reusing_current_matrix(w)?;
                            used_current_matrix_p
                        };
                    if !reuse {
                        self.clear_desired_matrix(w);
                        if self.try_window(window, startp, TRY_WINDOW_CHECK_MARGINS)? < 0 {
                            continue;
                        }
                    }
                    if self.frame(f).fonts_changed {
                        step = Step::NeedLargerMatrices;
                        continue;
                    }
                    if self.window(w).cursor.vpos >= 0 {
                        if !self.cursor_row_fully_visible_p(w, true, false, false)? {
                            self.clear_desired_matrix(w);
                            last_line_misfit = true;
                        } else {
                            step = Step::Done;
                        }
                    } else {
                        self.clear_desired_matrix(w);
                    }
                }
                Step::TryToScroll => {
                    step = Step::Recenter;
                    if !update_mode_line {
                        update_mode_line = true;
                        self.window_mut(w).flags.update_mode_line = true;
                    }
                    let scroll_conservatively = self.int_var(sym::SCROLL_CONSERVATIVELY);
                    let scroll_minibuffer_conservatively =
                        self.var_true(sym::SCROLL_MINIBUFFER_CONSERVATIVELY);
                    let emacs_scroll_step = self.int_var(sym::SCROLL_STEP);
                    let mini = self.window(w).mini;
                    let cb = self.current_buffer;
                    let up = self.buffer_local_value(sym::SCROLL_UP_AGGRESSIVELY, cb);
                    let down = self.buffer_local_value(sym::SCROLL_DOWN_AGGRESSIVELY, cb);
                    let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
                    if (0 < scroll_conservatively
                        || (scroll_minibuffer_conservatively && mini)
                        || 0 < emacs_scroll_step
                        || temp_scroll_step
                        || self.is_number(up)
                        || self.is_number(down))
                        && startp.charpos >= begv
                        && startp.charpos <= zv
                    {
                        let conservatively = if scroll_minibuffer_conservatively && mini {
                            SCROLL_LIMIT + 1
                        } else {
                            scroll_conservatively
                        };
                        let ss = self.try_scrolling(
                            window,
                            just_this_one_p,
                            conservatively,
                            emacs_scroll_step,
                            temp_scroll_step,
                            last_line_misfit,
                        )?;
                        match ss {
                            SCROLLING_SUCCESS => step = Step::Done,
                            SCROLLING_NEED_LARGER_MATRICES => step = Step::NeedLargerMatrices,
                            SCROLLING_FAILED => {}
                            _ => unreachable!("emacs_abort in redisplay_window"),
                        }
                    }
                }
                Step::Recenter => {
                    step = Step::Done;
                    let (pt, pt_byte) = (self.cur().pt as i64, self.cur().pt_byte as i64);
                    it = It::default();
                    self.init_iterator(&mut it, w, pt, pt_byte, None, DEFAULT_FACE_ID)?;
                    it.current_y = it.last_visible_y;
                    let scroll_conservatively = self.int_var(sym::SCROLL_CONSERVATIVELY);
                    let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
                    if centering_position < 0 {
                        let mut margin_pos = startp.charpos;
                        if margin != 0 && begv <= startp.charpos && startp.charpos <= zv {
                            let mut it1data = None;
                            let mut it1 = self.save_it(&it, &mut it1data);
                            self.start_display(&mut it1, w, startp)?;
                            self.move_it_vertically(&mut it1, margin * frame_line_height)?;
                            margin_pos = it1.charpos();
                            self.restore_it(&mut it, None, &mut it1data);
                        }
                        let scrolling_up = pt > margin_pos;
                        let cb = self.current_buffer;
                        let aggressive = self.buffer_local_value(
                            if scrolling_up {
                                sym::SCROLL_UP_AGGRESSIVELY
                            } else {
                                sym::SCROLL_DOWN_AGGRESSIVELY
                            },
                            cb,
                        );
                        if !self.window(w).mini
                            && (scroll_conservatively > SCROLL_LIMIT || self.is_number(aggressive))
                        {
                            let mut pt_offset = 0;
                            if scroll_conservatively == 0 && self.is_number(aggressive) {
                                let float_amount = self.xfloatint(aggressive);
                                pt_offset =
                                    (float_amount * self.window_box_text_height(w) as f64) as i32;
                                if pt_offset == 0 && float_amount > 0.0 {
                                    pt_offset = 1;
                                }
                                if pt_offset != 0 && margin > 0 {
                                    margin -= 1;
                                }
                            }
                            if scrolling_up {
                                centering_position = it.last_visible_y;
                                if pt_offset != 0 {
                                    centering_position -= pt_offset;
                                }
                                centering_position -= frame_line_height
                                    * (1 + margin + i32::from(last_line_misfit))
                                    + self.window_tab_line_height(w) as i32
                                    + self.window_header_line_height(w) as i32;
                                if centering_position < margin * frame_line_height {
                                    centering_position = margin * frame_line_height;
                                }
                            } else {
                                centering_position = margin * frame_line_height + pt_offset;
                            }
                        } else {
                            centering_position = (self.window_box_height(w) / 2) as i32;
                        }
                    }
                    if self.cur().long_line_optimizations_p && it.line_wrap == LineWrap::Truncate {
                        let mut nlines = centering_position / frame_line_height;
                        while nlines != 0 && it.charpos() > begv {
                            nlines -= 1;
                            self.back_to_previous_visible_line_start(&mut it)?;
                        }
                        let pos = it.current.pos;
                        self.reseat_1(&mut it, pos, true)?;
                    } else {
                        self.move_it_vertically_backward(&mut it, centering_position)?;
                    }
                    if it.current_y <= 0 {
                        it = It::default();
                        self.init_iterator(&mut it, w, pt, pt_byte, None, DEFAULT_FACE_ID)?;
                        self.move_it_vertically_backward(&mut it, 0)?;
                        it.current_y = 0;
                    }
                    it.current_x = 0;
                    it.wrap_prefix_width = 0;
                    it.hpos = 0;
                    let start = self.window(w).start;
                    let cb = self.current_buffer;
                    self.set_marker_position(start, it.charpos() as usize, cb);
                    startp = self.run_window_scroll_functions(window, it.current.pos)?;
                    let itdata = self.bidi_shelve_cache();
                    let mut use_desired_matrix = false;
                    let reuse = !(!current_matrix_up_to_date_p
                        || self.windows.windows_or_buffers_changed != 0
                        || self.frame(f).flags.cursor_type_changed
                        || !self
                            .find_symbol_value(sym::WINDOW_SCROLL_FUNCTIONS)
                            .is_nil()
                        || !just_this_one_p
                        || self.window(w).mini)
                        && {
                            used_current_matrix_p = self.try_window_reusing_current_matrix(w)?;
                            used_current_matrix_p
                        };
                    if !reuse {
                        use_desired_matrix = self.try_window(window, startp, 0)? == 1;
                    }
                    self.bidi_unshelve_cache(itdata, false);
                    if self.frame(f).fonts_changed {
                        step = Step::NeedLargerMatrices;
                        continue;
                    }
                    if self.window(w).cursor.vpos < 0 {
                        let z = self.cur().z() as i64;
                        if self.window(w).flags.window_end_valid
                            && pt >= z - self.window(w).window_end_pos
                        {
                            self.clear_desired_matrix(w);
                            self.move_it_by_lines(&mut it, 1)?;
                            let pos = it.current.pos;
                            self.try_window(window, pos, 0)?;
                        } else if pt < it.charpos() {
                            self.clear_desired_matrix(w);
                            self.move_it_by_lines(&mut it, -1)?;
                            let pos = it.current.pos;
                            self.try_window(window, pos, 0)?;
                        } else if scroll_conservatively > SCROLL_LIMIT
                            && (it.method == ItMethod::FromString
                                || self.overlay_touches_p(it.charpos()))
                            && it.charpos() < zv
                        {
                            let pos0 = it.charpos();
                            self.clear_desired_matrix(w);
                            loop {
                                self.move_it_by_lines(&mut it, 1)?;
                                if it.charpos() != pos0 {
                                    break;
                                }
                            }
                            let pos = it.current.pos;
                            self.try_window(window, pos, 0)?;
                        }
                    }
                    if self.window(w).cursor.vpos < 0 {
                        let which = if use_desired_matrix {
                            WhichMatrix::Desired
                        } else {
                            WhichMatrix::Current
                        };
                        let mut row = self.row_containing_pos(w, pt, which, 0, None, 0)?;
                        if row.is_none()
                            && let Some(alt_pos) = self.invisible_alternative_pos()?
                        {
                            row = self.row_containing_pos(w, alt_pos, which, 0, None, 0)?;
                        }
                        let row = match row {
                            Some(row) => row,
                            None => {
                                let m = self.window_matrix(w, which).expect("glyph matrix");
                                let mut row = 0;
                                if m.rows[row].tab_line_p {
                                    row += 1;
                                }
                                if m.rows[row].mode_line_p {
                                    row += 1;
                                }
                                row
                            }
                        };
                        self.set_cursor_from_row(w, row, which, 0, 0, 0, 0)?;
                    }
                    if !self.cursor_row_fully_visible_p(w, false, false, false)? {
                        if self.window(w).vscroll != 0 {
                            self.window_mut(w).vscroll = 0;
                            self.clear_desired_matrix(w);
                            step = Step::Recenter;
                            continue;
                        }
                        if scroll_conservatively > SCROLL_LIMIT {
                            let window_total_lines = self.window(w).total_lines
                                * self.frame(f).geometry.line_height
                                / i64::from(frame_line_height);
                            let move_down = self.window(w).cursor.vpos >= window_total_lines / 2;
                            let by = if move_down { margin + 1 } else { -(margin + 1) };
                            self.move_it_by_lines(&mut it, i64::from(by))?;
                            self.clear_desired_matrix(w);
                            let pos = it.current.pos;
                            if self.try_window(window, pos, TRY_WINDOW_CHECK_MARGINS)? == 1 {
                                continue;
                            }
                        }
                        if centering_position == 0 {
                            continue;
                        }
                        self.clear_desired_matrix(w);
                        centering_position = 0;
                        step = Step::Recenter;
                    }
                }
                Step::Done => {
                    step = Step::FinishMenuBars;
                    let start = self.window(w).start;
                    startp = {
                        let m = self.marker(start);
                        TextPos::new(m.charpos as i64, m.bytepos as i64)
                    };
                    let begv = self.cur().begv as i64;
                    let at_line_beg = startp.charpos == begv
                        || self.cur().text.byte_at((startp.bytepos - 1) as usize) == b'\n';
                    self.window_mut(w).flags.start_at_line_beg = at_line_beg;
                    let wants = self.window_wants_mode_line(w)
                        || self.window_wants_header_line(w)
                        || self.window_wants_tab_line(w);
                    let needed = update_mode_line
                        || (!just_this_one_p
                            && !self.frame_window_p(f)
                            && !self.window_full_width_p(w))
                        || self.window(w).base_line_pos > 0
                        || (self.window(w).column_number_displayed != -1
                            && self.window(w).column_number_displayed != self.current_column()?);
                    if needed && wants {
                        let count1 = self.specpdl_index();
                        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
                        self.display_mode_lines(w)?;
                        self.unbind_to(count1)?;
                        if self.window_wants_mode_line(w) {
                            let desired = self.desired_line_height(
                                w,
                                crate::dispextern::GlyphMatrix::mode_line_row,
                            );
                            if self.current_mode_line_height(w) != desired {
                                self.frame_mut(f).fonts_changed = true;
                                self.window_mut(w).mode_line_height = -1;
                                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Current) {
                                    let r = m.mode_line_row();
                                    m.rows[r].height = desired as i32;
                                }
                            }
                        }
                        if self.window_wants_tab_line(w) {
                            let desired = self.desired_line_height(w, |_| 0);
                            if self.current_tab_line_height(w) != desired {
                                self.frame_mut(f).fonts_changed = true;
                                self.window_mut(w).tab_line_height = -1;
                                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Current) {
                                    m.rows[0].height = desired as i32;
                                }
                            }
                        }
                        if self.window_wants_header_line(w) {
                            let desired = self.desired_line_height(
                                w,
                                crate::dispextern::GlyphMatrix::header_line_row,
                            );
                            if self.current_header_line_height(w) != desired {
                                self.frame_mut(f).fonts_changed = true;
                                self.window_mut(w).header_line_height = -1;
                                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Current) {
                                    let r = m.header_line_row();
                                    m.rows[r].height = desired as i32;
                                }
                            }
                        }
                        if self.frame(f).fonts_changed {
                            step = Step::NeedLargerMatrices;
                            continue;
                        }
                    }
                    if !self.xdisp.line_number_displayed && self.window(w).base_line_pos != -1 {
                        let win = self.window_mut(w);
                        win.base_line_pos = 0;
                        win.base_line_number = 0;
                    }
                }
                Step::FinishMenuBars => {
                    step = Step::FinishScrollBars;
                    if update_mode_line && self.frame(f).selected_window == window {
                        if self.frame_window_p(f) {
                            if self.frame(f).flags.external_menu_bar {
                                self.display_menu_bar(w);
                            }
                        } else {
                            unreachable!("tty frames have no glyph matrices");
                        }
                        if self.frame_window_p(f) {
                            let tab_bar_window = self.frame(f).tab_bar_window;
                            if self.is_window(tab_bar_window)
                                && (self.frame(f).geometry.tab_bar_lines > 0
                                    || !self.find_symbol_value(sym::AUTO_RESIZE_TAB_BARS).is_nil())
                                && self.redisplay_tab_bar(f)
                            {
                                self.keyboard.ignore_mouse_drag_p = true;
                            }
                            if self.frame(f).flags.external_tool_bar {
                                unreachable!("external tool bars need update_frame_tool_bar");
                            }
                        } else {
                            unreachable!("tty frames have no glyph matrices");
                        }
                        self.gui_consider_frame_title(f)?;
                    }
                    if self.frame_window_p(f) {
                        let keep = just_this_one_p
                            || (!used_current_matrix_p && !self.xdisp.overlay_arrow_seen)
                            || self.window(w).flags.pseudo_window_p;
                        if self.update_window_fringes(w, keep)? {
                            self.update_begin(f);
                            if self.draw_window_fringes(w, true)? {
                                if self.window_right_divider_width(w) != 0 {
                                    self.gui_draw_right_divider(w);
                                } else {
                                    self.gui_draw_vertical_border(w);
                                }
                            }
                            self.update_end(f);
                        }
                    }
                    if self.window_bottom_divider_width(w) != 0 {
                        self.gui_draw_bottom_divider(w);
                    }
                }
                Step::NeedLargerMatrices => step = Step::FinishScrollBars,
                Step::FinishScrollBars => break,
            }
        }
        if self.window_has_vertical_scroll_bar(w) || self.window_has_horizontal_scroll_bar(w) {
            if self.window_has_vertical_scroll_bar(w) {
                self.set_vertical_scroll_bar(w)?;
            }
            if self.window_has_horizontal_scroll_bar(w) {
                self.set_horizontal_scroll_bar(w)?;
            }
            let t = self.frame(f).terminal;
            if let Some(hook) = self.terminal(t).hooks.redeem_scroll_bar {
                hook(self, w)?;
            }
        }
        let b = self.cur();
        let (begv, begv_byte, zv, z, z_byte) = (
            b.begv as i64,
            b.begv_byte as i64,
            b.zv as i64,
            b.z() as i64,
            b.z_byte() as i64,
        );
        if opoint.charpos < begv {
            self.temp_set_pt_both(begv, begv_byte);
        } else if opoint.charpos > zv {
            self.temp_set_pt_both(z, z_byte);
        } else if ochars_modiff == self.cur().text.chars_modiff {
            self.temp_set_pt_both(opoint.charpos, opoint.bytepos);
        } else {
            let byte = self.cur().text.char_to_byte(opoint.charpos as usize) as i64;
            self.temp_set_pt_both(opoint.charpos, byte);
        }
        self.set_buffer_internal(old);
        if lpoint.charpos <= self.cur().zv as i64 {
            if lchars_modiff == self.cur().text.chars_modiff {
                self.temp_set_pt_both(lpoint.charpos, lpoint.bytepos);
            } else {
                let byte = self.cur().text.char_to_byte(lpoint.charpos as usize) as i64;
                self.temp_set_pt_both(lpoint.charpos, byte);
            }
        }
        self.unbind_to(count)?;
        Ok(())
    }

    fn check_long_lines(&mut self) -> LispResult<()> {
        let threshold = self.find_symbol_value(sym::LONG_LINE_THRESHOLD);
        let b = self.cur();
        if threshold.is_nil()
            || b.long_line_optimizations_p
            || !(b.text.chars_modiff - b.text.unchanged_modified > 8 || b.clip_changed)
        {
            return Ok(());
        }
        let threshold = threshold.as_fixnum().unwrap_or(0);
        let (begv, zv) = (b.begv, b.zv);
        let mut max = 0;
        let mut cur = begv;
        while cur < zv {
            let (next, found) = self.find_newline1_forward(cur);
            if (next - cur) as i64 > max {
                max = (next - cur) as i64;
            }
            if !found || max > threshold {
                break;
            }
            cur = next;
        }
        if max > threshold {
            self.cur_mut().long_line_optimizations_p = true;
        }
        Ok(())
    }

    fn find_newline1_forward(&self, start: usize) -> (usize, bool) {
        let zv = self.cur().zv;
        let (pos, counted) = self.find_newline(start, zv, 1);
        (pos, counted > 0)
    }

    pub fn set_vertical_scroll_bar(&mut self, w: Value) -> LispResult<()> {
        self.with_gc_deferred(|lisp| lisp.set_vertical_scroll_bar_1(w))
    }

    fn set_vertical_scroll_bar_1(&mut self, w: Value) -> LispResult<()> {
        let (start, end, whole);
        if !self.window(w).mini
            || (w == self.windows.minibuf_window && self.xdisp.echo_area_buffer[0].is_nil())
        {
            let buf = self.window(w).contents;
            let (zv, begv, z, long_lines) = {
                let b = self.buffer(buf);
                (
                    b.zv as i64,
                    b.begv as i64,
                    b.z() as i64,
                    b.long_line_optimizations_p,
                )
            };
            let mut whole_ = zv - begv;
            let start_ = self.marker(self.window(w).start).charpos as i64 - begv;
            let mut end_ = z - self.window(w).window_end_pos - begv;
            if !self.window(w).flags.window_end_valid && !long_lines {
                let obuf = self.current_buffer;
                self.set_buffer_internal(buf);
                let start_pos = {
                    let m = self.marker(self.window(w).start);
                    TextPos::new(m.charpos as i64, m.bytepos as i64)
                };
                let mut it = It::default();
                self.start_display(&mut it, w, start_pos)?;
                let (lvx, height) = (it.last_visible_x, self.window_box_height(w) as i32);
                self.move_it_to(&mut it, -1, lvx, height, -1, MOVE_TO_X | MOVE_TO_Y)?;
                end_ -= (z - it.charpos()) - self.window(w).window_end_pos;
                self.set_buffer_internal(obuf);
            }
            if end_ < start_ {
                end_ = start_;
            }
            if whole_ < end_ - start_ {
                whole_ = end_ - start_;
            }
            start = start_;
            end = end_;
            whole = whole_;
        } else {
            start = 0;
            end = 0;
            whole = 0;
        }
        let f = self.window(w).frame;
        let t = self.frame(f).terminal;
        if let Some(hook) = self.terminal(t).hooks.set_vertical_scroll_bar {
            hook(self, w, end - start, whole, start)?;
        }
        Ok(())
    }

    pub fn set_horizontal_scroll_bar(&mut self, w: Value) -> LispResult<()> {
        self.with_gc_deferred(|lisp| lisp.set_horizontal_scroll_bar_1(w))
    }

    fn set_horizontal_scroll_bar_1(&mut self, w: Value) -> LispResult<()> {
        let (mut start, mut end, mut whole, portion);
        if !self.window(w).mini || self.window(w).horizontal_scroll_bar_type == sym::BOTTOM {
            let b = self.window(w).contents;
            let mut old_buffer = None;
            if b != self.current_buffer {
                old_buffer = Some(self.current_buffer);
                self.set_buffer_internal(b);
            }
            let startp = {
                let m = self.marker(self.window(w).start);
                TextPos::new(m.charpos as i64, m.bytepos as i64)
            };
            let mut it = It::default();
            self.start_display(&mut it, w, startp)?;
            it.last_visible_x = i32::MAX;
            let height = self.window_box_height(w) as i32;
            whole = i64::from(self.move_it_to(
                &mut it,
                -1,
                i32::MAX,
                height,
                -1,
                MOVE_TO_X | MOVE_TO_Y,
            )?);
            let f = self.window(w).frame;
            start = self.window(w).hscroll * self.frame(f).geometry.column_width;
            end = start + self.window_box_width(w, GlyphRowArea::Text);
            portion = end - start;
            whole = whole.max(end);
            if it.bidi_p {
                let pdir = crate::xdisp::f_current_bidi_paragraph_direction(self, &[Value::NIL])?;
                if pdir == sym::RIGHT_TO_LEFT {
                    start = whole - end;
                    end = start + portion;
                }
            }
            if let Some(old) = old_buffer {
                self.set_buffer_internal(old);
            }
        } else {
            start = 0;
            end = 0;
            whole = 0;
            portion = 0;
        }
        let f = self.window(w).frame;
        let t = self.frame(f).terminal;
        if let Some(hook) = self.terminal(t).hooks.set_horizontal_scroll_bar {
            hook(self, w, portion, whole, start)?;
        }
        let _ = end;
        Ok(())
    }

    pub fn gui_consider_frame_title(&mut self, frame: Value) -> LispResult<()> {
        self.with_gc_deferred(|lisp| lisp.gui_consider_frame_title_1(frame))
    }

    fn gui_consider_frame_title_1(&mut self, frame: Value) -> LispResult<()> {
        let f = frame;
        if !((self.frame_window_p(f)
            || self.frame_minibuf_only_p(f)
            || self.frame(f).flags.explicit_name)
            && !self.frame(f).flags.tooltip)
        {
            return Ok(());
        }
        let kb = self.frame_kboard(f);
        let frames = self.frames.list.clone();
        let mut found = false;
        for tf in frames {
            let fr = self.frame(tf);
            if tf != f
                && self.frame_kboard(tf) == kb
                && !self.frame_minibuf_only_p(tf)
                && fr.parent_frame.is_nil()
                && !fr.flags.tooltip
                && (fr.flags.visible || fr.flags.iconified)
            {
                found = true;
                break;
            }
        }
        self.xdisp.multiple_frames = found;
        let count = self.specpdl_index();
        self.specbind(sym::INHIBIT_REDISPLAY, sym::T)?;
        let old_buffer = self.current_buffer;
        let selected = self.windows.selected_window;
        let format_data =
            self.format_mode_line_unwind_data(Some(f), Some(old_buffer), selected, false);
        self.record_unwind(crate::xdisp::modeline::unwind_format_mode_line, format_data);
        let sw = self.frame(f).selected_window;
        crate::window::f_select_window(self, &[sw, sym::T])?;
        let contents = self.window(sw).contents;
        self.set_buffer_internal(contents);
        let icon_title_format = self.find_symbol_value(sym::ICON_TITLE_FORMAT);
        let fmt = if self.frame(f).flags.iconified && icon_title_format != sym::T {
            icon_title_format
        } else {
            self.find_symbol_value(sym::FRAME_TITLE_FORMAT)
        };
        self.xdisp.mode_line_target = ModeLineTarget::Title;
        let title_start = self.xdisp.mode_line_noprop_buf.len();
        let mut it = It::default();
        self.init_iterator(&mut it, sw, -1, -1, None, DEFAULT_FACE_ID)?;
        self.xdisp.dont_resize_frames += 1;
        self.display_mode_element(&mut it, 0, -1, -1, fmt, Value::NIL, false)?;
        let raw = self.xdisp.mode_line_noprop_buf[title_start..].to_vec();
        let (title, nchars) = crate::character::str_as_multibyte(&raw);
        self.unbind_to(count)?;
        let name = self.frame(f).name;
        let changed = !name.is_string() || self.string_bytes(name) != title.as_slice();
        let t = self.frame(f).terminal;
        if changed && let Some(hook) = self.terminal(t).hooks.implicit_set_name {
            let title_string = self.make_specified_string(title, nchars, true);
            hook(self, f, title_string, Value::NIL)?;
        }
        Ok(())
    }
}
