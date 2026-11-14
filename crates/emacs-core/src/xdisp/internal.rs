use crate::dispextern::{GlyphRowRef, It, ItMethod, LineWrap, MOVE_TO_POS, TextPos};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::terminal::OutputMethod;
use crate::value::Value;
use crate::xdisp::REDISPLAY_SOME;
use crate::xfaces::DEFAULT_FACE_ID;

const CLEAR_FACE_CACHE_COUNT: i32 = 500;
const CLEAR_IMAGE_CACHE_COUNT: i32 = 101;
const MAX_HSCROLL_RETRIES: i32 = 16;
const MAX_GARBAGED_FRAME_RETRIES: i32 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum After {
    Proceed,
    Retry,
    Update,
    Cancel,
    EndOfRedisplay,
}

impl Lisp {
    pub(crate) fn redisplay_internal(&mut self) -> LispResult<()> {
        self.with_gc_deferred(|lisp| lisp.redisplay_internal_1())
    }

    pub fn redisplay_preserve_echo_area(&mut self, _from_where: i32) -> LispResult<()> {
        let count = self.specpdl_index();
        self.record_unwind(unwind_redisplay_preserve_echo_area, Value::NIL);
        self.block_buffer_flips();
        if !self.xdisp.echo_area_buffer[1].is_nil() {
            self.xdisp.display_last_displayed_message_p = true;
            self.redisplay_internal()?;
            self.xdisp.display_last_displayed_message_p = false;
        } else {
            self.redisplay_internal()?;
        }
        let sf = self.frames.selected_frame;
        self.flush_frame(sf);
        self.unbind_to(count)
    }

    pub(crate) fn block_buffer_flips(&mut self) {
        self.xdisp.buffer_flip_blocked_depth += 1;
    }

    pub(crate) fn unblock_buffer_flips(&mut self) {
        self.xdisp.buffer_flip_blocked_depth -= 1;
        if self.xdisp.buffer_flip_blocked_depth == 0 {
            for f in self.frames.list.clone() {
                let t = self.frame(f).terminal;
                if let Some(hook) = self.terminal(t).hooks.buffer_flipping_unblocked {
                    hook(self, f);
                }
            }
        }
    }

    pub fn buffer_flipping_blocked_p(&self) -> bool {
        self.xdisp.buffer_flip_blocked_depth > 0
    }

    fn propagate_buffer_redisplay(&mut self) -> LispResult<()> {
        let mut ws = self.window_list()?;
        while ws.is_cons() {
            let w = self.xcar(ws);
            let b = self.window(w).contents;
            if self.buffer(b).text.redisplay {
                self.window_mut(w).flags.redisplay = true;
            }
            ws = self.xcdr(ws);
        }
        Ok(())
    }

    fn text_outside_line_unchanged_p(&self, w: Value, start: i64, end: i64) -> bool {
        let mut unchanged_p = true;
        if self.window_outdated(w) {
            let b = self.cur();
            let (gpt, z) = (b.text.gpt() as i64, b.z() as i64);
            let (beg_unchanged, end_unchanged) = (b.text.beg_unchanged, b.text.end_unchanged);
            if gpt < start || z - gpt < end {
                unchanged_p = false;
            }
            if unchanged_p && (beg_unchanged < start - 1 || end_unchanged < end) {
                unchanged_p = false;
            }
            let selective = self.find_symbol_value(sym::SELECTIVE_DISPLAY);
            if unchanged_p
                && selective.as_fixnum().is_some_and(|n| n > 0)
                && (beg_unchanged < start || gpt <= start)
            {
                unchanged_p = false;
            }
            if unchanged_p {
                if 1 + beg_unchanged == start && self.overlay_touches_p(start) {
                    unchanged_p = false;
                }
                if end_unchanged == end && self.overlay_touches_p(z - end) {
                    unchanged_p = false;
                }
            }
            let contents = self.window(w).contents;
            if !self
                .buffer_local_value(sym::BIDI_DISPLAY_REORDERING, contents)
                .is_nil()
                && self
                    .buffer_local_value(sym::BIDI_PARAGRAPH_DIRECTION, contents)
                    .is_nil()
            {
                unchanged_p = false;
            }
        }
        unchanged_p
    }

    fn redisplay_windows(&mut self, window: Value) -> LispResult<()> {
        let mut window = window;
        while !window.is_nil() {
            let contents = self.window(window).contents;
            if self.is_window(contents) {
                self.redisplay_windows(contents)?;
            } else if self.is_buffer(contents) {
                self.xdisp.displayed_buffer = contents;
                self.redisplay_window_safely(window, false)?;
            }
            window = self.window(window).next;
        }
        Ok(())
    }

    fn redisplay_window_safely(&mut self, window: Value, just_this_one_p: bool) -> LispResult<()> {
        let list_of_error = self.list(&[sym::ERROR]);
        self.condition_case_internal(
            list_of_error,
            |l| {
                let b = l.xdisp.displayed_buffer;
                if l.buffer(b).display_error_modiff < l.buffer(b).text.modiff {
                    l.redisplay_window(window, just_this_one_p)?;
                }
                Ok(Value::NIL)
            },
            |l, error| l.redisplay_window_error(error),
        )?;
        Ok(())
    }

    fn redisplay_window_error(&mut self, error_data: Value) -> LispResult {
        let b = self.xdisp.displayed_buffer;
        let modiff = self.buffer(b).text.modiff;
        self.buffer_mut(b).display_error_modiff = modiff;
        if self.int_var(sym::MAX_REDISPLAY_TICKS) > 0
            && error_data.is_cons()
            && self.xcar(error_data) == sym::ERROR
            && self.xcdr(error_data).is_cons()
            && self.xcar(self.xcdr(error_data)).is_string()
        {
            let message = self.xcar(self.xcdr(error_data));
            let entry = self.list(&[sym::ERROR, message]);
            let list = self.find_symbol_value(sym::DELAYED_WARNINGS_LIST);
            let list = self.cons(entry, list);
            self.set_plain_value(sym::DELAYED_WARNINGS_LIST, list);
        }
        Ok(Value::NIL)
    }

    fn frame_up_to_date(&mut self, f: Value) -> LispResult<()> {
        let t = self.frame(f).terminal;
        if let Some(hook) = self.terminal(t).hooks.frame_up_to_date {
            hook(self, f)?;
        }
        Ok(())
    }

    fn ainc(&mut self, var: Value, i: i64) -> LispResult<()> {
        let table = self.find_symbol_value(var);
        let key = Value::fixnum(i);
        let entry = self.gethash(key, table, Value::fixnum(0))?;
        if let Some(n) = entry.as_fixnum() {
            self.puthash(key, Value::fixnum(1 + n), table)?;
        }
        Ok(())
    }

    fn try_one_line_optimizations(
        &mut self,
        w: &mut Value,
        sw: Value,
        (tlbufpos, tlendpos): (TextPos, TextPos),
        must_finish: bool,
    ) -> LispResult<After> {
        let win = *w;
        let begv = self.cur().begv as i64;
        let zv = self.cur().zv as i64;
        let fetch = |l: &Lisp, bytepos: i64| l.cur().text.byte_at(bytepos as usize);
        if tlbufpos.charpos > begv
            && fetch(self, tlbufpos.bytepos - 1) != b'\n'
            && (tlbufpos.charpos == zv || fetch(self, tlbufpos.bytepos) == b'\n')
        {
            return Ok(After::Cancel);
        }
        if self.window_outdated(win) || self.window(win).mini {
            let line_height_before = self.xdisp.this_line_pixel_height;
            let mut it = It::default();
            self.start_display(&mut it, win, tlbufpos)?;
            if it.current_x != self.xdisp.this_line_start_x {
                return Ok(After::Cancel);
            }
            if it.sp > 1 && it.method == ItMethod::FromImage && it.image_id == -1 {
                return Ok(After::Cancel);
            }
            self.window_mut(win).cursor.vpos = -1;
            self.xdisp.overlay_arrow_seen = false;
            let this_line_vpos = self.xdisp.this_line_vpos;
            it.vpos = this_line_vpos;
            it.current_y = self.xdisp.this_line_y;
            it.glyph_row = Some(GlyphRowRef::Window {
                window: win,
                desired: true,
                vpos: this_line_vpos as usize,
            });
            self.display_line(&mut it, -1)?;
            if self.window(win).cursor.vpos >= 0
                && self.xdisp.this_line_start_pos.charpos != 0
                && self.xdisp.this_line_end_pos.charpos == tlendpos.charpos
                && self.xdisp.this_line_pixel_height == line_height_before
                && !self.hscrolling_current_line_p(win)
            {
                let vpos = this_line_vpos as usize;
                if it.current_y < it.last_visible_y {
                    let (z, z_byte) = (self.cur().z() as i64, self.cur().z_byte() as i64);
                    let (minpos, nrows) = {
                        let m = self
                            .window_matrix(win, WhichMatrix::Current)
                            .expect("current matrix");
                        (m.rows[vpos + 1].minpos, m.nrows as usize)
                    };
                    let delta = z - tlendpos.charpos - minpos.charpos;
                    let delta_bytes = z_byte - tlendpos.bytepos - minpos.bytepos;
                    self.increment_matrix_positions(
                        win,
                        WhichMatrix::Current,
                        vpos + 1,
                        nrows,
                        delta,
                        delta_bytes,
                    );
                }
                let displays_text = self
                    .window_matrix(win, WhichMatrix::Desired)
                    .expect("desired matrix")
                    .rows[vpos]
                    .displays_text_p;
                let this_line_vpos = i64::from(this_line_vpos);
                let wn = self.window_mut(win);
                if displays_text {
                    if wn.window_end_vpos < this_line_vpos {
                        wn.window_end_vpos = this_line_vpos;
                    }
                } else if wn.window_end_vpos == this_line_vpos && this_line_vpos > 0 {
                    wn.window_end_vpos = this_line_vpos - 1;
                }
                wn.flags.window_end_valid = false;
                if let Some(m) = self.window_matrix_mut(win, WhichMatrix::Desired) {
                    m.no_scrolling_p = true;
                }
                self.update_window_fringes(win, false)?;
                return Ok(After::Update);
            }
            return Ok(After::Cancel);
        }
        let (last_point, cursor_vpos, total_lines) = {
            let wn = self.window(win);
            (wn.last_point, wn.cursor.vpos, wn.total_lines)
        };
        if self.cur().pt as i64 == last_point && 0 <= cursor_vpos && cursor_vpos < total_lines {
            if !must_finish {
                self.do_pending_window_change(true)?;
                let selected = self.windows.selected_window;
                if self.is_window(selected) {
                    *w = selected;
                    if selected != sw {
                        return Ok(After::Retry);
                    }
                }
                let wn = self.window(*w);
                if wn.flags.cursor_off_p == wn.flags.last_cursor_off_p {
                    return Ok(After::EndOfRedisplay);
                }
            }
            return Ok(After::Update);
        }
        if self
            .find_symbol_value(sym::SHOW_TRAILING_WHITESPACE)
            .is_nil()
            && !self.var_true(sym::CURSOR_IN_ECHO_AREA)
            && !self.var_true(sym::COMPOSITION_BREAK_AT_POINT)
        {
            let mut it = It::default();
            self.init_iterator(
                &mut it,
                win,
                tlbufpos.charpos,
                tlbufpos.bytepos,
                None,
                DEFAULT_FACE_ID,
            )?;
            it.current_x = self.xdisp.this_line_start_x;
            it.current_y = self.xdisp.this_line_y;
            it.wrap_prefix_width = 0;
            let this_line_vpos = self.xdisp.this_line_vpos;
            it.vpos = this_line_vpos;
            let pt = self.cur().pt as i64;
            if self.cur().long_line_optimizations_p
                && it.line_wrap == LineWrap::Truncate
                && pt - tlbufpos.charpos > self.int_var(sym::LARGE_HSCROLL_THRESHOLD)
            {
                self.reseat_at_next_visible_line_start(&mut it, false)?;
                if it.charpos() <= pt {
                    it.vpos = this_line_vpos + 1;
                }
            } else {
                self.move_it_to(&mut it, pt, -1, -1, -1, MOVE_TO_POS)?;
            }
            let vpos = this_line_vpos as usize;
            if it.vpos == this_line_vpos
                && self
                    .window_matrix(win, WhichMatrix::Current)
                    .is_some_and(|m| m.rows[vpos].enabled_p)
            {
                self.set_cursor_from_row(win, vpos, WhichMatrix::Current, 0, 0, 0, 0)?;
                if self.cursor_row_fully_visible_p(win, false, true, false)? {
                    return Ok(After::Update);
                }
            }
            return Ok(After::Cancel);
        }
        Ok(After::Cancel)
    }

    fn redisplay_internal_1(&mut self) -> LispResult<()> {
        let mut w = self.windows.selected_window;
        let mut must_finish = false;
        let mut hscroll_retries = 0;
        let mut garbaged_frame_retries = 0;
        let mut update_miniwindow_p = false;
        if self.xdisp.redisplaying_p {
            return Ok(());
        }
        let sf = self.frames.selected_frame;
        if (self.frame(sf).output_method == OutputMethod::Initial
            && !self.plain_value(sym::REDISPLAY_SKIP_INITIAL_FRAME).is_nil())
            || !self.plain_value(sym::INHIBIT_REDISPLAY).is_nil()
        {
            return Ok(());
        }
        let fr = self.window(w).frame;
        if !self.frame(fr).flags.glyphs_initialized_p {
            return Ok(());
        }
        self.xdisp.redisplay_counter += 1;
        let count = self.specpdl_index();
        self.record_unwind(unwind_redisplay, Value::NIL);
        self.xdisp.redisplaying_p = true;
        self.block_buffer_flips();
        self.specbind(sym::INHIBIT_FREE_REALIZED_FACES, Value::NIL)?;
        self.record_in_backtrace(sym::REDISPLAY_INTERNAL_XC_FUNCTIONX, 0);
        for f in self.frames.list.clone() {
            self.frame_mut(f).flags.already_hscrolled_p = false;
        }
        'retry: loop {
            let mut sw = w;
            let sf = self.frames.selected_frame;
            self.forget_escape_and_glyphless_faces();
            self.xdisp.inhibit_free_realized_faces = false;
            if self.faces.face_change {
                self.windows.windows_or_buffers_changed = 47;
            }
            let mut number_of_visible_frames = 0;
            for f in self.frames.list.clone() {
                if self.frame_redisplay_p(f) {
                    number_of_visible_frames += 1;
                    if self.frame(f).fonts_changed {
                        self.adjust_frame_glyphs(f);
                        self.set_frame_garbaged(f);
                        self.frame_mut(f).fonts_changed = false;
                    }
                    if f != sf && self.frame(f).flags.cursor_type_changed {
                        self.fset_redisplay(f);
                    }
                }
                self.clear_desired_matrices(f);
            }
            self.do_pending_window_change(true)?;
            let current_matrices_cleared = self.clear_garbaged_frames()?;
            if self.find_symbol_value(sym::MEMORY_FULL).is_nil() {
                self.prepare_menu_bars()?;
            }
            let selected = self.windows.selected_window;
            if self.is_window(selected) {
                w = selected;
                if w != sw {
                    sw = w;
                }
            }
            self.reconsider_clip_changes(w)?;
            let match_p = self.window(w).contents == self.current_buffer;
            if match_p {
                let (save_modiff, modiff) = (self.cur().text.save_modiff, self.cur().text.modiff);
                if (save_modiff < modiff) != self.window(w).flags.last_had_star {
                    self.window_mut(w).flags.update_mode_line = true;
                }
                if self.mode_line_update_needed(w)? {
                    self.window_mut(w).flags.update_mode_line = true;
                }
                if self.cur().clip_changed {
                    let cb = self.current_buffer;
                    self.bset_update_mode_line(cb);
                }
            }
            let echo_pending = {
                let x = &self.xdisp;
                (!x.echo_area_buffer[0].is_nil() && !x.display_last_displayed_message_p)
                    || (!x.echo_area_buffer[1].is_nil() && x.display_last_displayed_message_p)
                    || (x.message_cleared_p
                        && self.minibuf.minibuf_level == 0
                        && !self.window(self.windows.selected_window).mini)
            };
            if echo_pending {
                self.echo_area_display(false)?;
                if self.xdisp.message_cleared_p {
                    update_miniwindow_p = true;
                }
                must_finish = true;
                if !self.xdisp.display_last_displayed_message_p {
                    self.xdisp.message_cleared_p = false;
                }
            } else if self.windows.selected_window == self.windows.minibuf_window
                && (self.cur().clip_changed || self.window_outdated(w))
                && self.resize_mini_window(w, false)?
            {
                must_finish = true;
                self.clear_garbaged_frames()?;
            }
            if !self.find_symbol_value(sym::RUN_HOOKS).is_nil() {
                self.run_window_change_functions()?;
            }
            if self.windows.windows_or_buffers_changed != 0 && self.windows.update_mode_lines == 0 {
                self.windows.update_mode_lines =
                    if self.windows.windows_or_buffers_changed == REDISPLAY_SOME {
                        REDISPLAY_SOME
                    } else {
                        32
                    };
            }
            self.overlay_arrows_changed_p(true)?;
            let consider_all_windows_p = self.windows.update_mode_lines != 0
                || self.windows.windows_or_buffers_changed != 0
                || current_matrices_cleared;
            let all_windows_cause = self.windows.windows_or_buffers_changed;
            self.ainc(sym::REDISPLAY__ALL_WINDOWS_CAUSE, all_windows_cause)?;
            let mode_lines_cause = self.windows.update_mode_lines;
            self.ainc(sym::REDISPLAY__MODE_LINES_CAUSE, mode_lines_cause)?;
            let tlbufpos = self.xdisp.this_line_start_pos;
            let tlendpos = self.xdisp.this_line_end_pos;
            let mut after = After::Proceed;
            let display_line_numbers = self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS);
            let wf = self.window(w).frame;
            let pt = self.cur().pt as i64;
            let z = self.cur().z() as i64;
            if !consider_all_windows_p
                && tlbufpos.charpos > 0
                && !self.window(w).flags.update_mode_line
                && !self.cur().clip_changed
                && !self.cur().prevent_redisplay_optimizations_p
                && self.frame_redisplay_p(wf)
                && !self.frame(wf).flags.cursor_type_changed
                && !self.frame(wf).flags.face_change
                && self.xdisp.this_line_buffer == self.current_buffer
                && match_p
                && !self.window(w).flags.force_start
                && !self.window(w).flags.optional_new_start
                && pt >= tlbufpos.charpos
                && pt <= z - tlendpos.charpos
                && (display_line_numbers.is_nil() || display_line_numbers == sym::VISUAL)
                && self.text_outside_line_unchanged_p(w, tlbufpos.charpos, tlendpos.charpos)
            {
                after =
                    self.try_one_line_optimizations(&mut w, sw, (tlbufpos, tlendpos), must_finish)?;
                match after {
                    After::Retry => continue 'retry,
                    After::EndOfRedisplay => break 'retry,
                    After::Cancel => {
                        let vpos = self.xdisp.this_line_vpos as usize;
                        if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                            m.rows[vpos].enabled_p = false;
                        }
                    }
                    After::Proceed | After::Update => {}
                }
            }
            if after != After::Update {
                self.xdisp.this_line_start_pos.charpos = 0;
                self.xdisp.clear_face_cache_count += 1;
                self.xdisp.clear_image_cache_count += 1;
            }
            if after != After::Update && consider_all_windows_p {
                for f in self.frames.list.clone() {
                    self.frame_mut(f).flags.updated_p = false;
                }
                self.propagate_buffer_redisplay()?;
                for f in self.frames.list.clone() {
                    'retry_frame: loop {
                        if !(self.frame_window_p(f) || f == sf) {
                            break;
                        }
                        let gcscrollbars =
                            self.frame(f).flags.redisplay || !self.redisplay_some_p();
                        let f_redisplay_flag = self.frame(f).flags.redisplay;
                        if !self.frame_live_p(f) {
                            break;
                        }
                        let t = self.frame(f).terminal;
                        if gcscrollbars
                            && let Some(hook) = self.terminal(t).hooks.condemn_scroll_bars
                        {
                            hook(self, f);
                        }
                        if self.frame_redisplay_p(f) {
                            self.frame_mut(f).flags.inhibit_clear_image_cache = true;
                            let root = self.frame(f).root_window;
                            self.redisplay_windows(root)?;
                        } else if !self.redisplay_some_p() {
                            self.frame_mut(f).flags.redisplay = true;
                        }
                        if !self.frame_live_p(f) {
                            break;
                        }
                        if gcscrollbars && let Some(hook) = self.terminal(t).hooks.judge_scroll_bars
                        {
                            hook(self, f);
                        }
                        if self.frame_redisplay_p(f) {
                            if self.frame(f).fonts_changed {
                                self.adjust_frame_glyphs(f);
                                self.set_frame_garbaged(f);
                                self.frame_mut(f).fonts_changed = false;
                                continue 'retry_frame;
                            }
                            if !self.frame(f).flags.already_hscrolled_p {
                                self.frame_mut(f).flags.already_hscrolled_p = true;
                                let root = self.frame(f).root_window;
                                if hscroll_retries <= MAX_HSCROLL_RETRIES
                                    && self.hscroll_windows(root)?
                                {
                                    hscroll_retries += 1;
                                    continue 'retry_frame;
                                }
                            }
                            if !f_redisplay_flag && self.frame(f).flags.redisplay {
                                continue 'retry_frame;
                            }
                            if self.frame(f).flags.garbaged {
                                let retry = garbaged_frame_retries < MAX_GARBAGED_FRAME_RETRIES;
                                garbaged_frame_retries += 1;
                                if retry {
                                    continue 'retry;
                                }
                            }
                            if self.frame_window_p(f)
                                && let Some(hook) = self.frame_rif(f).clear_under_internal_border
                            {
                                hook(self, f);
                            }
                            self.update_frame(f, false)?;
                            if self.frame(f).flags.garbaged {
                                self.fset_redisplay(f);
                                self.frame_mut(f).flags.garbaged = false;
                                continue 'retry_frame;
                            }
                            let fl = &mut self.frame_mut(f).flags;
                            fl.cursor_type_changed = false;
                            fl.updated_p = true;
                            fl.inhibit_clear_image_cache = false;
                        }
                        break;
                    }
                }
                for f in self.frames.list.clone() {
                    if self.frame(f).flags.updated_p {
                        let fl = &mut self.frame_mut(f).flags;
                        fl.redisplay = false;
                        fl.garbaged = false;
                        let root = self.frame(f).root_window;
                        self.mark_window_display_accurate(root, true);
                        self.frame_up_to_date(f)?;
                    }
                }
            } else if after != After::Update && self.frame_redisplay_p(sf) {
                self.frame_mut(sf).flags.inhibit_clear_image_cache = true;
                let selected = self.windows.selected_window;
                self.xdisp.displayed_buffer = self.window(selected).contents;
                self.redisplay_window_safely(selected, true)?;
                if update_miniwindow_p {
                    let mini_window = self.frame(sf).minibuffer_window;
                    self.xdisp.displayed_buffer = self.window(mini_window).contents;
                    self.redisplay_window_safely(mini_window, true)?;
                }
                after = After::Update;
            }
            if after == After::Update {
                if self.frame(sf).fonts_changed || self.frame(sf).flags.redisplay {
                    if self.frame(sf).flags.redisplay {
                        self.windows.windows_or_buffers_changed = 50;
                    }
                    continue 'retry;
                }
                self.xdisp.inhibit_free_realized_faces = true;
                if self.frame_redisplay_p(sf) {
                    let selected = self.windows.selected_window;
                    if hscroll_retries <= MAX_HSCROLL_RETRIES && self.hscroll_windows(selected)? {
                        hscroll_retries += 1;
                        continue 'retry;
                    }
                    self.window_mut(selected).flags.must_be_updated_p = true;
                    self.update_frame(sf, false)?;
                    let fl = &mut self.frame_mut(sf).flags;
                    fl.cursor_type_changed = false;
                    fl.inhibit_clear_image_cache = false;
                }
                let mini_window = self.frame(sf).minibuffer_window;
                let mini_frame = self.window(mini_window).frame;
                if mini_frame != sf {
                    self.window_mut(mini_window).flags.must_be_updated_p = true;
                    self.update_frame(mini_frame, false)?;
                    self.frame_mut(mini_frame).flags.cursor_type_changed = false;
                    if hscroll_retries <= MAX_HSCROLL_RETRIES
                        && self.hscroll_windows(mini_window)?
                    {
                        hscroll_retries += 1;
                        continue 'retry;
                    }
                }
            }
            if !consider_all_windows_p {
                let contents = self.window(w).contents;
                if self.buffer(contents).text.redisplay && self.buffer(contents).window_count > 1 {
                    self.propagate_buffer_redisplay()?;
                }
                self.mark_window_display_accurate_1(w, true);
                self.update_overlay_arrows(1);
                self.frame_up_to_date(sf)?;
            }
            self.windows.update_mode_lines = 0;
            self.windows.windows_or_buffers_changed = 0;
            let mut new_count = 0;
            for f in self.frames.list.clone() {
                if self.frame_redisplay_p(f) {
                    new_count += 1;
                }
            }
            if new_count != number_of_visible_frames {
                self.windows.windows_or_buffers_changed = 52;
            }
            self.do_pending_window_change(true)?;
            if self.windows.windows_or_buffers_changed != 0 {
                continue 'retry;
            }
            let selected = self.windows.selected_window;
            if self.is_window(selected) {
                w = selected;
                if w != sw {
                    continue 'retry;
                }
            }
            if self.xdisp.clear_face_cache_count > CLEAR_FACE_CACHE_COUNT {
                self.clear_face_cache(false);
                self.xdisp.clear_face_cache_count = 0;
            }
            if self.xdisp.clear_image_cache_count > CLEAR_IMAGE_CACHE_COUNT {
                self.xdisp.clear_image_cache_count = 0;
            }
            break 'retry;
        }
        if self.int_var(sym::MAX_REDISPLAY_TICKS) > 0 {
            self.update_redisplay_ticks(0, Value::NIL)?;
        }
        self.unbind_to(count)
    }
}

fn unwind_redisplay(lisp: &mut Lisp, _: Value) -> LispResult<()> {
    lisp.xdisp.redisplaying_p = false;
    lisp.unblock_buffer_flips();
    Ok(())
}

fn unwind_redisplay_preserve_echo_area(lisp: &mut Lisp, _: Value) -> LispResult<()> {
    lisp.unblock_buffer_flips();
    Ok(())
}
