use crate::dispextern::{GlyphRowArea, It, ItMethod, MOVE_TO_POS, TEXT_AREA};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const DISP_INFINITY: i32 = 10_000_000;

impl Lisp {
    pub(crate) fn matrix_bottom_text_row(&self, w: Value, which: WhichMatrix) -> usize {
        let m = self.window_matrix(w, which).expect("glyph matrix");
        m.nrows as usize - usize::from(self.window_wants_mode_line(w))
    }

    pub fn hscroll_window_tree(&mut self, window: Value) -> LispResult<bool> {
        let mut hscrolled_p = false;
        let step = self.find_symbol_value(sym::HSCROLL_STEP);
        let mut hscroll_relative_p = step.is_float();
        let mut hscroll_step_abs: i64 = 0;
        let mut hscroll_step_rel = 0.0;
        if hscroll_relative_p {
            hscroll_step_rel = self.heap.float_value(step);
            if hscroll_step_rel < 0.0 {
                hscroll_relative_p = false;
                hscroll_step_abs = 0;
            }
        } else if let Some(n) = step.as_fixnum().filter(|n| i32::try_from(*n).is_ok()) {
            hscroll_step_abs = n.max(0);
        }
        let mut window = window;
        while self.is_window(window) {
            let w = window;
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                hscrolled_p |= self.hscroll_window_tree(contents)?;
            } else if self.window(w).cursor.vpos >= 0
                && !(w == self.xdisp.echo_area_window && !self.xdisp.echo_area_buffer[0].is_nil())
            {
                let vpos = self.window(w).cursor.vpos as usize;
                let mut which = WhichMatrix::Desired;
                let bottom_row = self.matrix_bottom_text_row(w, which);
                let mut cursor_row = if vpos < bottom_row {
                    vpos
                } else {
                    bottom_row - 1
                };
                if !self.window_matrix(w, which).expect("glyph matrix").rows[cursor_row].enabled_p {
                    which = WhichMatrix::Current;
                    let bottom_row = self.matrix_bottom_text_row(w, which);
                    cursor_row = if vpos < bottom_row {
                        vpos
                    } else {
                        bottom_row - 1
                    };
                }
                let row =
                    self.window_matrix(w, which).expect("glyph matrix").rows[cursor_row].clone();
                let row_r2l_p = row.reversed_p;
                let hscl = self.hscrolling_current_line_p(w);
                let mut x_offset = 0;
                if !self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS).is_nil() {
                    let glyphs = row.area_glyphs(TEXT_AREA);
                    if !row_r2l_p {
                        for g in glyphs {
                            if !(g.object.is_nil() && g.charpos < 0) {
                                break;
                            }
                            x_offset += i64::from(g.pixel_width);
                        }
                    } else {
                        for g in glyphs.iter().rev() {
                            if !(g.object.is_nil() && g.charpos < 0) {
                                break;
                            }
                            x_offset += i64::from(g.pixel_width);
                        }
                    }
                }
                let text_area_width = self.window_box_width(w, GlyphRowArea::Text);
                let h_margin = self.int_var(sym::HSCROLL_MARGIN).clamp(0, 1_000_000)
                    * self.window_frame_column_width(w);
                if self.window(w).flags.suspend_auto_hscroll {
                    let point = crate::window::f_window_point(self, &[window])?;
                    let old_point = crate::window::f_window_old_point(self, &[window])?;
                    if crate::fns::f_equal(self, &[point, old_point])?.is_nil() {
                        self.window_mut(w).flags.suspend_auto_hscroll = false;
                        let win = self.window(w);
                        if win.min_hscroll == 0
                            && win.hscroll > 0
                            && self.buffer_local_value(sym::AUTO_HSCROLL_MODE, contents)
                                == sym::CURRENT_LINE
                        {
                            let f = win.frame;
                            self.set_frame_garbaged(f);
                        }
                    }
                }
                let position = if w == self.windows.selected_window {
                    Value::fixnum(self.buffer(contents).pt as i64)
                } else {
                    Value::fixnum(self.marker(self.window(w).pointm).charpos as i64)
                };
                let old_pointm = self.window(w).old_pointm;
                self.set_marker(old_pointm, position, contents, false)?;
                let (hscroll, min_hscroll, cursor_x) = {
                    let win = self.window(w);
                    (win.hscroll, win.min_hscroll, win.cursor.x)
                };
                let auto = self.buffer_local_value(sym::AUTO_HSCROLL_MODE, contents);
                if !auto.is_nil()
                    && !self.window(w).flags.suspend_auto_hscroll
                    && row.start.pos.charpos >= 1
                    && ((!row_r2l_p
                        && ((hscroll != 0 && cursor_x <= h_margin + x_offset)
                            || (row.enabled_p
                                && row.truncated_on_right_p
                                && cursor_x >= text_area_width - h_margin)))
                        || (row_r2l_p
                            && ((row.enabled_p
                                && row.truncated_on_right_p
                                && cursor_x <= h_margin)
                                || (hscroll != 0
                                    && cursor_x >= text_area_width - h_margin - x_offset)))
                        || (hscl && hscroll != min_hscroll && !row.truncated_on_left_p))
                {
                    let saved_current_buffer = self.current_buffer;
                    self.current_buffer = contents;
                    let pt = if w == self.windows.selected_window {
                        self.cur().pt as i64
                    } else {
                        let b = self.cur();
                        let (begv, zv) = (b.begv as i64, b.zv as i64);
                        (self.marker(self.window(w).pointm).charpos as i64).clamp(begv, zv)
                    };
                    let row_ref = crate::dispextern::GlyphRowRef::Window {
                        window: w,
                        desired: which == WhichMatrix::Desired,
                        vpos: cursor_row,
                    };
                    let mut it = It::default();
                    let column_width =
                        self.frame(self.window(w).frame).geometry.column_width as i32;
                    self.init_to_row_start(&mut it, w, row_ref)?;
                    if hscl {
                        it.first_visible_x = self.window_hscroll_limited(w) as i32 * column_width;
                    }
                    it.last_visible_x = DISP_INFINITY;
                    let nchars = pt - it.charpos();
                    let threshold = self.int_var(sym::LARGE_HSCROLL_THRESHOLD);
                    if self.cur().long_line_optimizations_p && nchars > threshold {
                        self.fast_move_it_horizontally(&mut it, nchars)?;
                        it.current_x += (nchars * i64::from(column_width)) as i32;
                    } else {
                        self.move_it_in_display_line_to(&mut it, pt, -1, MOVE_TO_POS)?;
                    }
                    if it.method == ItMethod::FromString && pt > 1 {
                        self.init_to_row_start(&mut it, w, row_ref)?;
                        if hscl {
                            it.first_visible_x =
                                self.window_hscroll_limited(w) as i32 * column_width;
                        }
                        if self.cur().long_line_optimizations_p && nchars > threshold {
                            self.fast_move_it_horizontally(&mut it, nchars - 1)?;
                            it.current_x += ((nchars - 1) * i64::from(column_width)) as i32;
                        } else {
                            self.move_it_in_display_line_to(&mut it, pt - 1, -1, MOVE_TO_POS)?;
                        }
                    }
                    self.current_buffer = saved_current_buffer;
                    let column_width = i64::from(column_width);
                    let current_x = i64::from(it.current_x);
                    let mut new_hscroll = if !hscroll_relative_p && hscroll_step_abs == 0 {
                        (current_x
                            - if it.at_end_of_line_p() {
                                text_area_width - 4 * column_width
                            } else {
                                text_area_width / 2
                            })
                        .max(0)
                            / column_width
                    } else if (!row_r2l_p && cursor_x >= text_area_width - h_margin)
                        || (row_r2l_p && cursor_x <= h_margin)
                    {
                        let wanted_x = if hscroll_relative_p {
                            (text_area_width as f64 * (1.0 - hscroll_step_rel)) as i64 - h_margin
                        } else {
                            text_area_width - hscroll_step_abs * column_width - h_margin
                        };
                        (current_x - wanted_x).max(0) / column_width
                    } else {
                        let wanted_x = if hscroll_relative_p {
                            (text_area_width as f64 * hscroll_step_rel) as i64 + h_margin + x_offset
                        } else {
                            hscroll_step_abs * column_width + h_margin + x_offset
                        };
                        (current_x - wanted_x).max(0) / column_width
                    };
                    new_hscroll = new_hscroll.max(min_hscroll);
                    let (last_cursor_vpos, cursor_vpos) = {
                        let win = self.window(w);
                        (win.last_cursor_vpos, win.cursor.vpos)
                    };
                    if hscroll != new_hscroll || (hscl && last_cursor_vpos != cursor_vpos) {
                        self.buffer_mut(contents).prevent_redisplay_optimizations_p = true;
                        self.window_mut(w).hscroll = new_hscroll;
                        hscrolled_p = true;
                    }
                }
            }
            window = self.window(w).next;
        }
        Ok(hscrolled_p)
    }

    pub fn hscroll_windows(&mut self, window: Value) -> LispResult<bool> {
        let hscrolled_p = self.with_gc_deferred(|lisp| lisp.hscroll_window_tree(window))?;
        if hscrolled_p {
            let f = self.window(window).frame;
            self.clear_desired_matrices(f);
        }
        Ok(hscrolled_p)
    }
}
