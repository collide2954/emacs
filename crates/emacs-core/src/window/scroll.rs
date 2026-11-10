use crate::dispextern::{
    DisplayElementType, It, ItMethod, MOVE_TO_POS, MOVE_TO_X, MOVE_TO_Y, TextPos,
};
use crate::error::LispResult;
use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;
use crate::window::MarginUnit;
use crate::xfaces::DEFAULT_FACE_ID;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("scroll-up", 0, 1, f_scroll_up).interactive("^P"),
    Subr::fixed("scroll-down", 0, 1, f_scroll_down).interactive("^P"),
];

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn in_display_vector_p(it: &It) -> bool {
    it.method == ItMethod::FromDisplayVector
        && it.current.dpvec_index > 0
        && it.dpend > i64::from(it.current.dpvec_index)
}

impl Lisp {
    fn sanitize_next_screen_context_lines(&self) -> i32 {
        self.int_var(sym::NEXT_SCREEN_CONTEXT_LINES)
            .clamp(0, 1_000_000) as i32
    }

    fn pt_both(&self) -> (i64, i64) {
        let b = self.cur();
        (b.pt as i64, b.pt_byte as i64)
    }

    fn goto_both(&mut self, charpos: i64, bytepos: i64) {
        self.set_pt_both(charpos as usize, bytepos as usize);
    }

    fn scroll_signal(&mut self, noerror: bool, symbol: Value) -> LispResult<()> {
        if noerror {
            Ok(())
        } else {
            Err(self.signal(symbol, Value::NIL))
        }
    }

    fn adjust_old_pointm(&mut self, window: Value) -> LispResult<()> {
        let w = window;
        let contents = self.window(w).contents;
        let pos = if w == self.windows.selected_window {
            Value::fixnum(self.buffer(contents).pt as i64)
        } else {
            let pointm = self.window(w).pointm;
            crate::marker::f_marker_position(self, &[pointm])?
        };
        let old_pointm = self.window(w).old_pointm;
        crate::marker::f_set_marker(self, &[old_pointm, pos, contents])?;
        Ok(())
    }

    fn scroll_command_repeated_p(&mut self) -> LispResult<bool> {
        let last_command = self.kvar(Kvar::LastCommand);
        Ok(last_command.is_symbol()
            && !crate::fns::f_get(self, &[last_command, sym::SCROLL_COMMAND])?.is_nil())
    }

    pub fn window_scroll(
        &mut self,
        window: Value,
        n: i64,
        whole: bool,
        noerror: bool,
    ) -> LispResult<()> {
        let w = window;
        let b = self.window(w).contents;
        let mut long_lines_truncated = self.buffer(b).long_line_optimizations_p
            && !self.buffer_local_value(sym::TRUNCATE_LINES, b).is_nil();
        let count = self.specpdl_index();
        let n = n.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        self.wset_redisplay(w);
        let truncate_partial = self.find_symbol_value(sym::TRUNCATE_PARTIAL_WIDTH_WINDOWS);
        let f = self.window(w).frame;
        if self.buffer(b).long_line_optimizations_p
            && !long_lines_truncated
            && !truncate_partial.is_nil()
            && self.window(w).total_cols < self.frame(f).geometry.text_cols
        {
            long_lines_truncated = match truncate_partial.as_fixnum() {
                Some(limit) => self.window(w).total_cols < limit,
                None => true,
            };
        }
        if whole && (self.var_true(sym::FAST_BUT_IMPRECISE_SCROLLING) || long_lines_truncated) {
            self.specbind(sym::FONTIFICATION_FUNCTIONS, Value::NIL)?;
        }
        if whole && long_lines_truncated {
            self.window_scroll_for_long_lines(w, n, noerror)?;
        } else if self.frame_window_p(f) {
            self.record_unwind(crate::indent::unwind_display_working_on_window, Value::NIL);
            self.xdisp.display_working_on_window_p = true;
            self.window_scroll_pixel_based(window, n, whole, noerror)?;
        } else {
            self.window_scroll_line_based(window, n, whole, noerror)?;
        }
        self.unbind_to(count)?;
        self.window_mut(window).flags.window_end_valid = false;
        Ok(())
    }

    fn window_scroll_for_long_lines(&mut self, w: Value, n: i32, noerror: bool) -> LispResult<()> {
        let (mut startpos, mut startbyte) = {
            let m = self.marker(self.window(w).start);
            (m.charpos as i64, m.bytepos as i64)
        };
        let nscls = self.sanitize_next_screen_context_lines();
        let ht = self.window_internal_height(w) as i32;
        let n = n * 1.max(ht - nscls);
        let (pt, pt_byte) = self.pt_both();
        let zv = self.cur().zv as i64;
        let fully_visible = pt >= startpos
            && pt <= zv
            && startpos <= zv
            && self
                .pos_visible_p(w, pt)?
                .is_some_and(|v| v.rtop == 0 && v.rbot == 0);
        if !fully_visible {
            let pos = self.vmotion(pt, pt_byte, -i64::from(ht / 2), w)?;
            startpos = pos.bufpos;
            startbyte = pos.bytepos;
        }
        self.goto_both(startpos, startbyte);
        let (pt, pt_byte) = self.pt_both();
        let lose = n < 0 && pt == self.cur().begv as i64;
        let mut pos = self.vmotion(pt, pt_byte, i64::from(n), w)?;
        if lose {
            return self.scroll_signal(noerror, sym::BEGINNING_OF_BUFFER);
        }
        let bolp = pos.bufpos == self.cur().begv as i64
            || self.cur().text.byte_at(pos.bytepos as usize - 1) == b'\n';
        if pos.bufpos < self.cur().zv as i64 {
            let (start, contents) = (self.window(w).start, self.window(w).contents);
            self.set_marker(start, Value::fixnum(pos.bufpos), contents, true)?;
            self.window_mut(w).flags.start_at_line_beg = bolp;
            self.wset_update_mode_line(w);
            self.window_mut(w).flags.force_start = true;
            self.goto_both(pos.bufpos, pos.bytepos);
            let (pt, pt_byte) = self.pt_both();
            if n > 0 {
                pos = self.vmotion(pt, pt_byte, i64::from(ht / 2), w)?;
            } else if n < 0 {
                pos = self.vmotion(pt, pt_byte, -i64::from(ht / 2), w)?;
            }
            self.goto_both(pos.bufpos, pos.bytepos);
            self.window_mut(w).flags.preserve_vscroll_p = false;
            Ok(())
        } else {
            self.scroll_signal(noerror, sym::END_OF_BUFFER)
        }
    }

    fn window_scroll_pixel_based(
        &mut self,
        window: Value,
        n: i32,
        whole: bool,
        noerror: bool,
    ) -> LispResult<()> {
        self.with_gc_deferred(|lisp| lisp.window_scroll_pixel_based_1(window, n, whole, noerror))
    }

    fn window_scroll_pixel_based_1(
        &mut self,
        window: Value,
        n: i32,
        whole: bool,
        noerror: bool,
    ) -> LispResult<()> {
        let w = window;
        let contents = self.window(w).contents;
        let mut it = It::default();
        let mut vscrolled = false;
        let frame_line_height = self.default_line_pixel_height(w);
        let point = crate::window::f_window_point(self, &[window])?;
        let old_point = crate::window::f_window_old_point(self, &[window])?;
        let adjust_old_pointm = self.equal(point, old_point)?;
        let start_from_marker = |lisp: &Lisp| {
            let m = lisp.marker(lisp.window(w).start);
            TextPos::new(m.charpos as i64, m.bytepos as i64)
        };
        let mut start = start_from_marker(self);
        let (begv, begv_byte) = (self.cur().begv as i64, self.cur().begv_byte as i64);
        if start.charpos > self.cur().zv as i64 || start.charpos < begv {
            start = TextPos::new(begv, begv_byte);
        }
        let (pt, pt_byte) = self.pt_both();
        let visible = self.pos_visible_p(w, pt)?;
        match visible {
            None => {
                let itdata = self.bidi_shelve_cache();
                self.init_iterator(&mut it, w, pt, pt_byte, None, DEFAULT_FACE_ID)?;
                it.current_y = it.last_visible_y;
                let half = (self.window_box_height(w) / 2) as i32;
                self.move_it_vertically_backward(&mut it, half)?;
                if it.current_y <= 0 {
                    self.init_iterator(&mut it, w, pt, pt_byte, None, DEFAULT_FACE_ID)?;
                    self.move_it_vertically_backward(&mut it, 0)?;
                    it.current_y = 0;
                }
                start = it.current.pos;
                self.bidi_unshelve_cache(itdata, false);
            }
            Some(v) if self.var_true(sym::AUTO_WINDOW_VSCROLL) => {
                if v.rtop != 0 || v.rbot != 0 {
                    let mut dy = frame_line_height;
                    if whole {
                        let ht = self.window_box_height(w) as i32;
                        let nscls = self.sanitize_next_screen_context_lines();
                        dy = dy.max((ht / dy - nscls) * dy);
                    }
                    dy *= n;
                    let vscroll = self.window(w).vscroll as i32;
                    if n < 0 && vscroll < 0 && v.rtop > 0 {
                        let px = 0.max(-vscroll - v.rtop.min(-dy));
                        crate::window::f_set_window_vscroll(
                            self,
                            &[window, Value::fixnum(i64::from(px)), sym::T, Value::NIL],
                        )?;
                        return Ok(());
                    }
                    if n > 0 {
                        if v.rbot > 0 && (vscroll < 0 || v.vpos == 0) {
                            let px = 0.max(-vscroll + v.rbot.min(dy));
                            crate::window::f_set_window_vscroll(
                                self,
                                &[window, Value::fixnum(i64::from(px)), sym::T, Value::NIL],
                            )?;
                            return Ok(());
                        }
                        if v.rbot > 0 || vscroll < 0 {
                            crate::window::f_set_window_vscroll(
                                self,
                                &[window, Value::fixnum(0), sym::T, Value::NIL],
                            )?;
                            let spos = if v.rbot > 0 {
                                crate::editfns::f_line_beginning_position(self, &[])?
                                    .as_fixnum()
                                    .unwrap_or(0)
                            } else {
                                let eol = crate::editfns::f_line_end_position(self, &[])?
                                    .as_fixnum()
                                    .unwrap_or(0);
                                (eol + 1).min(self.cur().zv as i64)
                            };
                            let start_marker = self.window(w).start;
                            self.set_marker(start_marker, Value::fixnum(spos), contents, true)?;
                            self.window_mut(w).flags.start_at_line_beg = true;
                            self.wset_update_mode_line(w);
                            self.window_mut(w).flags.force_start = true;
                            return Ok(());
                        }
                    }
                }
                crate::window::f_set_window_vscroll(
                    self,
                    &[window, Value::fixnum(0), sym::T, Value::NIL],
                )?;
            }
            Some(_) => {}
        }
        let itdata = self.bidi_shelve_cache();
        let preserve = self.find_symbol_value(sym::SCROLL_PRESERVE_SCREEN_POSITION);
        if !preserve.is_nil() {
            if self.windows.window_scroll_pixel_based_preserve_y < 0
                || !self.scroll_command_repeated_p()?
            {
                self.start_display(&mut it, w, start)?;
                let pt = self.cur().pt as i64;
                self.move_it_to(&mut it, pt, -1, -1, -1, MOVE_TO_POS)?;
                self.windows.window_scroll_pixel_based_preserve_y = it.current_y;
                self.windows.window_scroll_pixel_based_preserve_x = it.current_x;
            }
        } else {
            self.windows.window_scroll_pixel_based_preserve_y = -1;
            self.windows.window_scroll_pixel_based_preserve_x = -1;
        }
        self.start_display(&mut it, w, start)?;
        let zv = self.cur().zv as i64;
        let begv = self.cur().begv as i64;
        if whole {
            let start_pos = it.charpos();
            let flh = frame_line_height;
            let ht = self.window_box_height(w) as i32;
            let nscls = self.sanitize_next_screen_context_lines();
            let dy = n * flh.max((ht / flh - nscls) * flh);
            if dy <= 0 {
                let goal_y = it.current_y + dy;
                self.move_it_vertically_backward(&mut it, -dy)?;
                if f64::from(goal_y - it.current_y) > 0.5 * f64::from(flh) {
                    let it_data = self.bidi_shelve_cache();
                    let it1 = it.clone();
                    if self.line_bottom_y(&it1)? - goal_y < goal_y - it.current_y {
                        self.move_it_by_lines(&mut it, 1)?;
                    }
                    self.bidi_unshelve_cache(it_data, true);
                }
                while start_pos == it.charpos() && start_pos > begv {
                    self.move_it_by_lines(&mut it, -1)?;
                }
            } else if dy > 0 {
                let goal_y = it.current_y + dy;
                self.move_it_to(&mut it, zv, -1, goal_y, -1, MOVE_TO_POS | MOVE_TO_Y)?;
                if !preserve.is_nil() && f64::from(goal_y - it.current_y) > 0.5 * f64::from(flh) {
                    let it_data = self.bidi_shelve_cache();
                    let it2 = it.clone();
                    self.move_it_by_lines(&mut it, 1)?;
                    if f64::from(it.current_y) > f64::from(goal_y) + 0.5 * f64::from(flh) {
                        it = it2;
                        self.bidi_unshelve_cache(it_data, false);
                    } else {
                        self.bidi_unshelve_cache(it_data, true);
                    }
                }
                while start_pos == it.charpos() && start_pos < zv {
                    self.move_it_by_lines(&mut it, 1)?;
                }
            }
        } else {
            self.move_it_by_lines(&mut it, i64::from(n))?;
        }
        if (n > 0 && it.charpos() == zv) || (n < 0 && it.charpos() == start.charpos) {
            if it.charpos() == zv {
                if it.current_y < it.last_visible_y
                    && it.current_y + it.max_ascent + it.max_descent > it.last_visible_y
                {
                    self.window_mut(w).vscroll = i64::from(
                        it.last_visible_y - (it.current_y + it.max_ascent + it.max_descent),
                    );
                    self.adjust_frame_glyphs(it.f);
                } else {
                    self.bidi_unshelve_cache(itdata, false);
                    return self.scroll_signal(
                        noerror,
                        if n < 0 {
                            sym::BEGINNING_OF_BUFFER
                        } else {
                            sym::END_OF_BUFFER
                        },
                    );
                }
            } else if self.window(w).vscroll != 0 {
                self.window_mut(w).vscroll = 0;
            } else {
                self.bidi_unshelve_cache(itdata, false);
                return self.scroll_signal(noerror, sym::BEGINNING_OF_BUFFER);
            }
            self.buffer_mut(contents).prevent_redisplay_optimizations_p = true;
            vscrolled = true;
        }
        if !vscrolled {
            let mut pos = it.charpos();
            if in_display_vector_p(&it) {
                pos += 1;
                self.move_it_to(&mut it, pos, -1, -1, -1, MOVE_TO_POS)?;
            }
            let start_marker = self.window(w).start;
            self.set_marker(start_marker, Value::fixnum(it.charpos()), contents, true)?;
            let bytepos = self.marker(start_marker).bytepos;
            self.window_mut(w).flags.start_at_line_beg =
                pos == begv || self.cur().text.byte_at(bytepos - 1) == b'\n';
            self.wset_update_mode_line(w);
            self.window_mut(w).flags.force_start = true;
        }
        it.current_y = 0;
        it.vpos = 0;
        let this_scroll_margin = self.window_scroll_margin(w, MarginUnit::InPixels);
        let tab_header =
            (self.window_tab_line_height(w) + self.window_header_line_height(w)) as i32;
        if n > 0 {
            let last_y = it.last_visible_y - this_scroll_margin - 1;
            let pt = self.cur().pt as i64;
            if it.charpos() < pt {
                self.move_it_to(&mut it, pt, -1, -1, -1, MOVE_TO_POS)?;
            }
            if it.charpos() == pt
                && it.current_y >= this_scroll_margin
                && it.current_y <= last_y - tab_header
                && (preserve.is_nil() || preserve == sym::T)
            {
            } else {
                let preserve_y = self.windows.window_scroll_pixel_based_preserve_y;
                if preserve_y >= 0 {
                    let goal_y = last_y.min(preserve_y);
                    let preserve_x = self.windows.window_scroll_pixel_based_preserve_x;
                    self.move_it_to(
                        &mut it,
                        -1,
                        preserve_x,
                        goal_y - tab_header,
                        -1,
                        MOVE_TO_Y | MOVE_TO_X,
                    )?;
                }
                while it.current_y < this_scroll_margin {
                    let prev = it.current_y;
                    self.move_it_by_lines(&mut it, 1)?;
                    if prev == it.current_y {
                        break;
                    }
                }
                self.goto_both(it.charpos(), it.bytepos());
                if preserve_y >= 0 && preserve_y < this_scroll_margin {
                    self.windows.window_scroll_pixel_based_preserve_y = this_scroll_margin;
                }
            }
        } else if n < 0 {
            let pt = self.cur().pt as i64;
            let plh = self.partial_line_height(&it)?;
            let goal_y = it.last_visible_y - tab_header - plh - this_scroll_margin - 1;
            self.move_it_to(&mut it, pt, -1, goal_y, -1, MOVE_TO_POS | MOVE_TO_Y)?;
            let mut charpos = it.charpos();
            let mut bytepos = it.bytepos();
            if charpos != pt {
                let it2 = it.clone();
                let it_data = self.bidi_shelve_cache();
                self.move_it_to(&mut it, pt, -1, -1, -1, MOVE_TO_POS)?;
                if it.charpos() == pt && it.current_y == it2.current_y {
                    charpos = it.charpos();
                    bytepos = it.bytepos();
                    self.bidi_unshelve_cache(it_data, true);
                } else {
                    it = it2;
                    self.bidi_unshelve_cache(it_data, false);
                }
            }
            let partial_p = if it.what == DisplayElementType::Eob {
                it.current_y + it.ascent + it.descent
                    > it.last_visible_y - this_scroll_margin - tab_header
            } else {
                self.move_it_by_lines(&mut it, 1)?;
                it.current_y > it.last_visible_y - this_scroll_margin - tab_header
            };
            let preserve_y = self.windows.window_scroll_pixel_based_preserve_y;
            if charpos == pt && !partial_p && (preserve.is_nil() || preserve == sym::T) {
            } else if preserve_y >= 0 {
                let mut goal_y = (it.last_visible_y - this_scroll_margin - 1).min(preserve_y);
                if goal_y < this_scroll_margin {
                    goal_y = this_scroll_margin;
                }
                let start = start_from_marker(self);
                self.start_display(&mut it, w, start)?;
                let preserve_x = self.windows.window_scroll_pixel_based_preserve_x;
                self.move_it_to(&mut it, -1, preserve_x, goal_y, -1, MOVE_TO_Y | MOVE_TO_X)?;
                self.goto_both(it.charpos(), it.bytepos());
            } else if partial_p {
                self.move_it_by_lines(&mut it, -2)?;
                self.goto_both(it.charpos(), it.bytepos());
            } else {
                self.goto_both(charpos, bytepos);
            }
        }
        self.bidi_unshelve_cache(itdata, false);
        if adjust_old_pointm {
            self.adjust_old_pointm(window)?;
        }
        Ok(())
    }

    fn window_scroll_line_based(
        &mut self,
        window: Value,
        n: i32,
        whole: bool,
        noerror: bool,
    ) -> LispResult<()> {
        self.with_gc_deferred(|lisp| lisp.window_scroll_line_based_1(window, n, whole, noerror))
    }

    fn window_scroll_line_based_1(
        &mut self,
        window: Value,
        n: i32,
        whole: bool,
        noerror: bool,
    ) -> LispResult<()> {
        let w = window;
        let opoint_marker = crate::editfns::f_point_marker(self, &[])?;
        let ht = self.window_internal_height(w) as i32;
        let (mut startpos, mut startbyte) = {
            let m = self.marker(self.window(w).start);
            (m.charpos as i64, m.bytepos as i64)
        };
        let mut original_pos = Value::NIL;
        let point = crate::window::f_window_point(self, &[window])?;
        let old_point = crate::window::f_window_old_point(self, &[window])?;
        let adjust_old_pointm = self.equal(point, old_point)?;
        let mut n = n;
        if whole {
            let nscls = self.sanitize_next_screen_context_lines();
            n *= 1.max(ht - nscls);
        }
        let preserve = self.find_symbol_value(sym::SCROLL_PRESERVE_SCREEN_POSITION);
        if !preserve.is_nil() {
            if self.windows.window_scroll_preserve_vpos <= 0 || !self.scroll_command_repeated_p()? {
                let (pt, hscroll) = (self.cur().pt as i64, self.window(w).hscroll);
                let posit = self.compute_motion(
                    startpos,
                    startbyte,
                    0,
                    0,
                    false,
                    pt,
                    i64::from(ht),
                    0,
                    -1,
                    hscroll,
                    0,
                    w,
                )?;
                self.windows.window_scroll_preserve_vpos = posit.vpos;
                self.windows.window_scroll_preserve_hpos = posit.hpos + hscroll;
            }
            original_pos = self.cons(
                Value::fixnum(self.windows.window_scroll_preserve_hpos),
                Value::fixnum(self.windows.window_scroll_preserve_vpos),
            );
        }
        let pt = Value::fixnum(self.cur().pt as i64);
        let tem = crate::window::f_pos_visible_in_window_p(self, &[pt, window, Value::NIL])?;
        if tem.is_nil() {
            crate::indent::f_vertical_motion(
                self,
                &[Value::fixnum(-i64::from(ht / 2)), window, Value::NIL],
            )?;
            (startpos, startbyte) = self.pt_both();
        }
        self.goto_both(startpos, startbyte);
        let lose = n < 0 && self.cur().pt == self.cur().begv;
        crate::indent::f_vertical_motion(self, &[Value::fixnum(i64::from(n)), window, Value::NIL])?;
        let (pos, pos_byte) = self.pt_both();
        let bolp = crate::editfns::f_bolp(self, &[])?;
        let opoint = *self.marker(opoint_marker);
        self.goto_both(opoint.charpos as i64, opoint.bytepos as i64);
        if lose {
            return self.scroll_signal(noerror, sym::BEGINNING_OF_BUFFER);
        }
        if pos < self.cur().zv as i64 {
            let this_scroll_margin = i64::from(self.window_scroll_margin(w, MarginUnit::InLines));
            let (start, contents) = (self.window(w).start, self.window(w).contents);
            self.set_marker(start, Value::fixnum(pos), contents, true)?;
            self.window_mut(w).flags.start_at_line_beg = !bolp.is_nil();
            self.wset_update_mode_line(w);
            self.window_mut(w).flags.force_start = true;
            let opoint_pos = self.marker(opoint_marker).charpos as i64;
            if !preserve.is_nil() && this_scroll_margin == 0 && (whole || preserve != sym::T) {
                self.goto_both(pos, pos_byte);
                crate::indent::f_vertical_motion(self, &[original_pos, window, Value::NIL])?;
            } else if n > 0 {
                let top_margin = if this_scroll_margin > 0 {
                    self.goto_both(pos, pos_byte);
                    crate::indent::f_vertical_motion(
                        self,
                        &[Value::fixnum(this_scroll_margin), window, Value::NIL],
                    )?;
                    self.cur().pt as i64
                } else {
                    pos
                };
                if top_margin <= opoint_pos {
                    let opoint = *self.marker(opoint_marker);
                    self.goto_both(opoint.charpos as i64, opoint.bytepos as i64);
                } else if !preserve.is_nil() {
                    let mut nlines = self.windows.window_scroll_preserve_vpos;
                    self.goto_both(pos, pos_byte);
                    let total_lines = self.window(w).total_lines;
                    if self.windows.window_scroll_preserve_vpos < this_scroll_margin {
                        nlines = this_scroll_margin;
                    } else if self.windows.window_scroll_preserve_vpos
                        >= total_lines - this_scroll_margin
                    {
                        nlines = total_lines - this_scroll_margin - 1;
                    }
                    let target = self.cons(
                        Value::fixnum(self.windows.window_scroll_preserve_hpos),
                        Value::fixnum(nlines),
                    );
                    crate::indent::f_vertical_motion(self, &[target, window, Value::NIL])?;
                } else {
                    let byte = self.cur().text.char_to_byte(top_margin as usize);
                    self.set_pt_both(top_margin as usize, byte);
                }
            } else if n < 0 {
                self.goto_both(pos, pos_byte);
                let moved = crate::indent::f_vertical_motion(
                    self,
                    &[
                        Value::fixnum(i64::from(ht) - this_scroll_margin),
                        window,
                        Value::NIL,
                    ],
                )?;
                let pt = self.cur().pt as i64;
                let bottom_margin = if moved.as_fixnum() == Some(i64::from(ht) - this_scroll_margin)
                {
                    pt
                } else {
                    pt + 1
                };
                if bottom_margin > opoint_pos {
                    let opoint = *self.marker(opoint_marker);
                    self.goto_both(opoint.charpos as i64, opoint.bytepos as i64);
                } else if !preserve.is_nil() {
                    let mut nlines = self.windows.window_scroll_preserve_vpos;
                    self.goto_both(pos, pos_byte);
                    if self.windows.window_scroll_preserve_vpos < this_scroll_margin {
                        nlines = this_scroll_margin;
                    } else if self.windows.window_scroll_preserve_vpos
                        >= i64::from(ht) - this_scroll_margin
                    {
                        nlines = i64::from(ht) - this_scroll_margin - 1;
                    }
                    let target = self.cons(
                        Value::fixnum(self.windows.window_scroll_preserve_hpos),
                        Value::fixnum(nlines),
                    );
                    crate::indent::f_vertical_motion(self, &[target, window, Value::NIL])?;
                } else {
                    crate::indent::f_vertical_motion(
                        self,
                        &[Value::fixnum(-1), window, Value::NIL],
                    )?;
                }
            }
        } else {
            return self.scroll_signal(noerror, sym::END_OF_BUFFER);
        }
        if adjust_old_pointm {
            self.adjust_old_pointm(window)?;
        }
        Ok(())
    }

    fn scroll_command(&mut self, window: Value, n: Value, direction: i64) -> LispResult<()> {
        let w = window;
        let other_window = window != self.windows.selected_window;
        let count = self.specpdl_index();
        let contents = self.window(w).contents;
        if other_window || contents != self.current_buffer {
            self.record_unwind_protect_excursion();
            if contents != self.current_buffer {
                crate::buffer::f_set_buffer(self, &[contents])?;
            }
        }
        if other_window {
            let pointm = *self.marker(self.window(w).pointm);
            self.goto_both(pointm.charpos as i64, pointm.bytepos as i64);
            let old_pointm = *self.marker(self.window(w).old_pointm);
            self.goto_both(old_pointm.charpos as i64, old_pointm.bytepos as i64);
        }
        if n.is_nil() {
            self.window_scroll(window, direction, true, false)?;
        } else if n == sym::MINUS {
            self.window_scroll(window, -direction, true, false)?;
        } else {
            let n = crate::callint::f_prefix_numeric_value(self, &[n])?;
            let n = n.as_fixnum().unwrap_or(0);
            self.window_scroll(window, n * direction, false, false)?;
        }
        if other_window {
            let pt = self.cur().pt;
            let (pointm, old_pointm) = (self.window(w).pointm, self.window(w).old_pointm);
            let buffer = self.current_buffer;
            self.set_marker_position(pointm, pt, buffer);
            self.set_marker_position(old_pointm, pt, buffer);
        }
        self.unbind_to(count)
    }
}

fn f_scroll_up(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let window = lisp.windows.selected_window;
    lisp.scroll_command(window, arg(args, 0), 1)?;
    Ok(Value::NIL)
}

fn f_scroll_down(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let window = lisp.windows.selected_window;
    lisp.scroll_command(window, arg(args, 0), -1)?;
    Ok(Value::NIL)
}
