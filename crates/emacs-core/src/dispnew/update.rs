use crate::dispextern::{
    GlyphRowRef, LAST_AREA, LEFT_MARGIN_AREA, OVERLAPS_PRED, OVERLAPS_SUCC, RIGHT_MARGIN_AREA, Run,
    TEXT_AREA,
};
use crate::dispnew::matrix::{WhichMatrix, assign_row, clear_glyph_matrix, row_equal_p};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

fn desired(w: Value, vpos: usize) -> GlyphRowRef {
    GlyphRowRef::Window {
        window: w,
        desired: true,
        vpos,
    }
}

fn current(w: Value, vpos: usize) -> GlyphRowRef {
    GlyphRowRef::Window {
        window: w,
        desired: false,
        vpos,
    }
}

pub fn next_almost_prime(n: usize) -> usize {
    let mut n = n | 1;
    loop {
        if !n.is_multiple_of(3) && !n.is_multiple_of(5) && !n.is_multiple_of(7) {
            return n;
        }
        n += 2;
    }
}

#[derive(Clone, Copy)]
struct RowEntry {
    old_uses: i32,
    new_uses: i32,
    new_line_number: usize,
    which: WhichMatrix,
    vpos: usize,
    next: Option<usize>,
}

impl Lisp {
    pub fn update_begin(&mut self, f: Value) {
        let t = self.frame(f).terminal;
        if let Some(hook) = self.terminal(t).hooks.update_begin {
            hook(self, f);
        }
    }

    pub fn update_end(&mut self, f: Value) {
        let t = self.frame(f).terminal;
        if let Some(hook) = self.terminal(t).hooks.update_end {
            hook(self, f);
        }
    }

    pub fn output_cursor_to(&mut self, w: Value, vpos: i64, hpos: i64, y: i64, x: i64) {
        let c = &mut self.window_mut(w).output_cursor;
        c.hpos = hpos;
        c.vpos = vpos;
        c.x = x;
        c.y = y;
    }

    pub fn update_frame(&mut self, f: Value, _inhibit_scrolling: bool) -> LispResult<()> {
        if !self.frame(f).flags.glyphs_initialized_p {
            return Ok(());
        }
        if self.frame_window_p(f) {
            return self.update_window_frame(f);
        }
        unreachable!("tty and initial frames have no frame glyph matrices")
    }

    fn update_bar_window(&mut self, f: Value, tab: bool) -> LispResult<()> {
        let window = if tab {
            self.frame(f).tab_bar_window
        } else {
            self.frame(f).tool_bar_window
        };
        if self.is_window(window) && self.window(window).flags.must_be_updated_p {
            self.update_window(window)?;
            self.window_mut(window).flags.must_be_updated_p = false;
            let fr = self.frame_mut(f);
            if tab {
                std::mem::swap(
                    &mut fr.current_tab_bar_string,
                    &mut fr.desired_tab_bar_string,
                );
            } else {
                std::mem::swap(
                    &mut fr.current_tool_bar_string,
                    &mut fr.desired_tool_bar_string,
                );
            }
        }
        Ok(())
    }

    fn update_window_frame(&mut self, f: Value) -> LispResult<()> {
        self.update_begin(f);
        let menu_bar = self.frame(f).menu_bar_window;
        if self.is_window(menu_bar) {
            self.update_window(menu_bar)?;
        }
        self.update_bar_window(f, true)?;
        self.update_bar_window(f, false)?;
        let root = self.frame(f).root_window;
        self.update_window_tree(root)?;
        self.update_end(f);
        self.set_window_update_flags(root, false);
        Ok(())
    }

    fn update_window_tree(&mut self, w: Value) -> LispResult<()> {
        let mut w = w;
        while !w.is_nil() {
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                self.update_window_tree(contents)?;
            } else if self.window(w).flags.must_be_updated_p {
                self.update_window(w)?;
            }
            w = self.window(w).next;
        }
        Ok(())
    }

    pub fn update_single_window(&mut self, w: Value) -> LispResult<()> {
        if self.window(w).flags.must_be_updated_p {
            let f = self.window(w).frame;
            self.update_begin(f);
            self.update_window(w)?;
            self.update_end(f);
            self.window_mut(w).flags.must_be_updated_p = false;
        }
        Ok(())
    }

    fn redraw_overlapped_rows(&mut self, w: Value, yb: i32) -> LispResult<()> {
        let f = self.window(w).frame;
        let rif = self.frame_rif(f);
        let nrows = self
            .window_matrix(w, WhichMatrix::Current)
            .map_or(0, |m| m.nrows.max(0) as usize);
        for i in 0..nrows {
            let row = current(w, i);
            let (enabled_p, mode_line_p, overlapped_p, y, x, used, bottom) = {
                let r = self.glyph_row(row);
                (
                    r.enabled_p,
                    r.mode_line_p,
                    r.overlapped_p,
                    r.y,
                    r.x,
                    r.used,
                    r.bottom_y(),
                )
            };
            if !enabled_p {
                break;
            } else if mode_line_p {
                continue;
            }
            if overlapped_p {
                for (area, &n) in used.iter().enumerate().take(LAST_AREA) {
                    let x = if area == TEXT_AREA { x } else { 0 };
                    self.output_cursor_to(w, i as i64, 0, i64::from(y), i64::from(x));
                    if n != 0 {
                        let write = rif.write_glyphs.expect("redisplay interface write_glyphs");
                        write(self, w, row, area, 0, n as usize)?;
                    }
                    let clear = rif
                        .clear_end_of_line
                        .expect("redisplay interface clear_end_of_line");
                    clear(self, w, row, area, -1)?;
                }
                self.glyph_row_mut(row).overlapped_p = false;
            }
            if bottom >= yb {
                break;
            }
        }
        Ok(())
    }

    fn redraw_overlapping_rows(&mut self, w: Value, yb: i32) -> LispResult<()> {
        let f = self.window(w).frame;
        let rif = self.frame_rif(f);
        let nrows = self
            .window_matrix(w, WhichMatrix::Current)
            .map_or(0, |m| m.nrows.max(0) as usize);
        for i in 0..nrows {
            let row = current(w, i);
            let r = self.glyph_row(row);
            if !r.enabled_p {
                break;
            } else if r.mode_line_p {
                continue;
            }
            let bottom_y = r.bottom_y();
            if r.overlapping_p {
                let mut overlaps = 0;
                let overlaps_pred = r.phys_ascent > r.ascent;
                let overlaps_succ = r.phys_height - r.phys_ascent > r.height - r.ascent;
                let used = r.used;
                if overlaps_pred && i > 0 && !self.glyph_row(current(w, i - 1)).overlapped_p {
                    overlaps |= OVERLAPS_PRED;
                }
                if overlaps_succ && bottom_y < yb && !self.glyph_row(current(w, i + 1)).overlapped_p
                {
                    overlaps |= OVERLAPS_SUCC;
                }
                if overlaps != 0 {
                    let fix = rif
                        .fix_overlapping_area
                        .expect("redisplay interface fix_overlapping_area");
                    if used[LEFT_MARGIN_AREA] != 0 {
                        fix(self, w, row, LEFT_MARGIN_AREA, overlaps)?;
                    }
                    if used[TEXT_AREA] != 0 {
                        fix(self, w, row, TEXT_AREA, overlaps)?;
                    }
                    if used[RIGHT_MARGIN_AREA] != 0 {
                        fix(self, w, row, RIGHT_MARGIN_AREA, overlaps)?;
                    }
                    if overlaps & OVERLAPS_PRED != 0 {
                        self.glyph_row_mut(current(w, i - 1)).overlapped_p = true;
                    }
                    if overlaps & OVERLAPS_SUCC != 0 {
                        self.glyph_row_mut(current(w, i + 1)).overlapped_p = true;
                    }
                }
            }
            if bottom_y >= yb {
                break;
            }
        }
        Ok(())
    }

    fn set_current_row_enabled(&mut self, w: Value, i: usize, enabled: bool) {
        self.glyph_row_mut(current(w, i)).enabled_p = enabled;
    }

    fn update_window(&mut self, w: Value) -> LispResult<()> {
        let f = self.window(w).frame;
        let rif = self.frame_rif(f);
        self.gui_update_window_begin(w);
        let yb = self.window_text_bottom_y(w) as i32;
        let nrows = self
            .window_matrix(w, WhichMatrix::Desired)
            .map_or(0, |m| m.nrows.max(0) as usize);
        let cur_nrows = self
            .window_matrix(w, WhichMatrix::Current)
            .map_or(0, |m| m.nrows.max(0) as usize);
        let end = nrows - 1;
        let mut row = 0;
        let mut changed_p = false;
        let mut mouse_face_overwritten_p = false;
        let mut invisible_rows_marked = false;
        let tab_line_row = {
            let r = self.glyph_row(desired(w, 0));
            if r.mode_line_p && r.tab_line_p {
                row += 1;
                Some(0)
            } else {
                None
            }
        };
        let header_line_row = if self.glyph_row(desired(w, row)).mode_line_p {
            let h = row;
            row += 1;
            Some(h)
        } else {
            None
        };
        let (mode_line_p, mode_line_enabled) = {
            let r = self.glyph_row(desired(w, end));
            (r.mode_line_p, r.enabled_p)
        };
        if mode_line_p && mode_line_enabled {
            let y = yb + self.window_scroll_bar_area_height(w) as i32;
            self.glyph_row_mut(desired(w, end)).y = y;
            self.update_window_line(w, end, &mut mouse_face_overwritten_p)?;
        }
        while row < end && !self.glyph_row(desired(w, row)).enabled_p {
            row += 1;
        }
        let mut skip_to_set_cursor = false;
        let no_scrolling_p = self
            .window_matrix(w, WhichMatrix::Desired)
            .is_some_and(|m| m.no_scrolling_p);
        if row < end && !no_scrolling_p {
            let lines = i32::from(tab_line_row.is_some()) + i32::from(header_line_row.is_some());
            let rc = self.scrolling_window(w, lines)?;
            if rc < 0 {
                skip_to_set_cursor = true;
            } else if rc > 0 {
                changed_p = true;
            }
        }
        if !skip_to_set_cursor {
            while row < end {
                if self.glyph_row(desired(w, row)).enabled_p {
                    let vpos = row;
                    changed_p |= self.update_window_line(w, vpos, &mut mouse_face_overwritten_p)?;
                    if self.glyph_row(desired(w, row)).bottom_y() >= yb {
                        for i in vpos + 1..cur_nrows.saturating_sub(1) {
                            self.set_current_row_enabled(w, i, false);
                        }
                        invisible_rows_marked = true;
                    }
                }
                row += 1;
            }
            if !self.window_wants_mode_line(w) && cur_nrows > 0 {
                self.set_current_row_enabled(w, cur_nrows - 1, false);
            }
            if !invisible_rows_marked {
                let mut i = 0;
                while i < cur_nrows.saturating_sub(1) {
                    let r = self.glyph_row(current(w, i));
                    if r.enabled_p && r.bottom_y() >= yb {
                        i += 1;
                        while i < cur_nrows - 1 {
                            self.set_current_row_enabled(w, i, false);
                            i += 1;
                        }
                    }
                    i += 1;
                }
            }
        }
        if let Some(t) = tab_line_row
            && self.glyph_row(desired(w, t)).enabled_p
        {
            self.glyph_row_mut(desired(w, t)).y = 0;
            self.update_window_line(w, 0, &mut mouse_face_overwritten_p)?;
        }
        if let Some(h) = header_line_row
            && self.glyph_row(desired(w, h)).enabled_p
        {
            let y = if tab_line_row.is_some() {
                self.current_tab_line_height(w) as i32
            } else {
                0
            };
            self.glyph_row_mut(desired(w, h)).y = y;
            let vpos = usize::from(tab_line_row.is_some());
            self.update_window_line(w, vpos, &mut mouse_face_overwritten_p)?;
        }
        if !self.window(w).flags.pseudo_window_p {
            if changed_p && rif.fix_overlapping_area.is_some() {
                self.redraw_overlapped_rows(w, yb)?;
                self.redraw_overlapping_rows(w, yb)?;
            }
            self.set_window_cursor_after_update(w);
        }
        self.update_window_fringes(w, false)?;
        self.gui_update_window_end(w, true, mouse_face_overwritten_p)?;
        self.window_mut(w).flags.must_be_updated_p = false;
        if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
            clear_glyph_matrix(m);
        }
        Ok(())
    }

    pub fn gui_update_window_begin(&mut self, w: Value) {
        let f = self.window(w).frame;
        if let Some(hook) = self.frame_rif(f).update_window_begin_hook {
            hook(self, w);
        }
        let cursor = self.window(w).cursor;
        self.window_mut(w).output_cursor = cursor;
        if self.mouse_hl_info(f).mouse_face_mouse_frame == f {
            let garbaged = self.frame(f).flags.garbaged;
            let hlinfo = self.mouse_hl_info_mut(f);
            hlinfo.mouse_face_defer = true;
            if garbaged {
                hlinfo.mouse_face_window = Value::NIL;
            }
        }
    }

    pub fn gui_update_window_end(
        &mut self,
        w: Value,
        cursor_on_p: bool,
        mouse_face_overwritten_p: bool,
    ) -> LispResult<()> {
        let f = self.window(w).frame;
        if !self.window(w).flags.pseudo_window_p {
            if cursor_on_p {
                let c = self.window(w).output_cursor;
                self.display_and_set_cursor(w, true, c.hpos, c.vpos, c.x, c.y)?;
            }
            if self.draw_window_fringes(w, true)? {
                if self.window_right_divider_width(w) != 0 {
                    self.gui_draw_right_divider(w);
                } else {
                    self.gui_draw_vertical_border(w);
                }
            }
        }
        if mouse_face_overwritten_p {
            let hlinfo = self.mouse_hl_info_mut(f);
            hlinfo.mouse_face_beg_row = -1;
            hlinfo.mouse_face_beg_col = -1;
            hlinfo.mouse_face_end_row = -1;
            hlinfo.mouse_face_end_col = -1;
            hlinfo.mouse_face_window = Value::NIL;
        }
        if let Some(hook) = self.frame_rif(f).update_window_end_hook {
            hook(self, w, cursor_on_p, mouse_face_overwritten_p);
        }
        Ok(())
    }

    fn update_marginal_area(
        &mut self,
        w: Value,
        updated_row: GlyphRowRef,
        area: usize,
        vpos: usize,
    ) -> LispResult<()> {
        let f = self.window(w).frame;
        let rif = self.frame_rif(f);
        let (y, used) = {
            let r = self.glyph_row(desired(w, vpos));
            (r.y, r.used[area])
        };
        self.output_cursor_to(w, vpos as i64, 0, i64::from(y), 0);
        if used != 0 {
            let write = rif.write_glyphs.expect("redisplay interface write_glyphs");
            write(self, w, updated_row, area, 0, used as usize)?;
        }
        let clear = rif
            .clear_end_of_line
            .expect("redisplay interface clear_end_of_line");
        clear(self, w, updated_row, area, -1)
    }

    fn update_text_area(
        &mut self,
        w: Value,
        updated_row: GlyphRowRef,
        vpos: usize,
        partial_p: &mut bool,
    ) -> LispResult<bool> {
        let f = self.window(w).frame;
        let rif = self.frame_rif(f);
        let write = rif.write_glyphs.expect("redisplay interface write_glyphs");
        let clear = rif
            .clear_end_of_line
            .expect("redisplay interface clear_end_of_line");
        let current_row = self.glyph_row(current(w, vpos)).clone();
        let desired_row = self.glyph_row(desired(w, vpos)).clone();
        let mut changed_p = false;
        let current_matrix_tab_header = self
            .window_matrix(w, WhichMatrix::Current)
            .is_some_and(|m| m.tab_line_p && m.header_line_p);
        if !current_row.enabled_p
            || desired_row.y != current_row.y
            || desired_row.ascent != current_row.ascent
            || desired_row.phys_ascent != current_row.phys_ascent
            || desired_row.phys_height != current_row.phys_height
            || desired_row.visible_height != current_row.visible_height
            || current_row.overlapped_p
            || (current_row.mouse_face_p
                && !(current_row.mode_line_p && (vpos > usize::from(current_matrix_tab_header))))
            || current_row.x != desired_row.x
        {
            self.output_cursor_to(
                w,
                vpos as i64,
                0,
                i64::from(desired_row.y),
                i64::from(desired_row.x),
            );
            if desired_row.used[TEXT_AREA] != 0 {
                write(
                    self,
                    w,
                    updated_row,
                    TEXT_AREA,
                    0,
                    desired_row.used[TEXT_AREA] as usize,
                )?;
            }
            clear(self, w, updated_row, TEXT_AREA, -1)?;
            changed_p = true;
            if vpos as i64 == self.window(w).phys_cursor.vpos {
                self.window_mut(w).flags.phys_cursor_on_p = false;
            }
            return Ok(changed_p);
        }
        let current_glyphs = current_row.area_glyphs(TEXT_AREA);
        let desired_glyphs = desired_row.area_glyphs(TEXT_AREA);
        let overlapping_glyphs_p = current_row.contains_overlapping_glyphs_p;
        let mut desired_stop_pos = desired_row.used[TEXT_AREA] as usize;
        let cur_used = current_row.used[TEXT_AREA] as usize;
        let mut abort_skipping = false;
        if desired_row.fill_line_p
            && (desired_stop_pos < cur_used
                || (desired_stop_pos == cur_used && !current_row.fill_line_p))
        {
            desired_stop_pos = desired_stop_pos.saturating_sub(1);
        }
        let stop = cur_used.min(desired_stop_pos);
        let mut i = 0usize;
        let mut x = desired_row.x;
        let get_overhangs = rif
            .get_glyph_overhangs
            .expect("redisplay interface get_glyph_overhangs");
        while i < stop {
            let mut can_skip_p = !abort_skipping;
            if overlapping_glyphs_p && i > 0 {
                let (_, right) = get_overhangs(self, &current_glyphs[i - 1], f);
                can_skip_p = right == 0 && !abort_skipping;
            }
            if can_skip_p {
                let start_hpos = i;
                while i < stop && desired_glyphs[i].equal_p(&current_glyphs[i]) {
                    x += i32::from(desired_glyphs[i].pixel_width);
                    i += 1;
                    *partial_p = true;
                }
                if overlapping_glyphs_p && i > 0 && i < cur_used && cur_used != desired_glyphs.len()
                {
                    let (mut left, _) = get_overhangs(self, &current_glyphs[i], f);
                    while left > 0 && i > 0 {
                        i -= 1;
                        x -= i32::from(desired_glyphs[i].pixel_width);
                        left -= i32::from(desired_glyphs[i].pixel_width);
                    }
                    abort_skipping = i < start_hpos;
                }
            }
            if i < desired_glyphs.len() {
                let start_x = x;
                let start_hpos = i;
                let mut current_x = x;
                let mut skip_first_p = !can_skip_p;
                while i < stop
                    && (skip_first_p || !desired_glyphs[i].equal_p(&current_glyphs[i]))
                    && x == current_x
                {
                    x += i32::from(desired_glyphs[i].pixel_width);
                    current_x += i32::from(current_glyphs[i].pixel_width);
                    i += 1;
                    skip_first_p = false;
                }
                if i == start_hpos || x != current_x {
                    i = start_hpos;
                    x = start_x;
                    break;
                }
                self.output_cursor_to(
                    w,
                    vpos as i64,
                    start_hpos as i64,
                    i64::from(desired_row.y),
                    i64::from(start_x),
                );
                write(self, w, updated_row, TEXT_AREA, start_hpos, i - start_hpos)?;
                changed_p = true;
                *partial_p = true;
            }
        }
        if i == 0 {
            *partial_p = false;
        }
        if i < desired_glyphs.len() {
            self.output_cursor_to(
                w,
                vpos as i64,
                i as i64,
                i64::from(desired_row.y),
                i64::from(x),
            );
            write(self, w, updated_row, TEXT_AREA, i, desired_glyphs.len() - i)?;
            changed_p = true;
        }
        if desired_row.fill_line_p {
        } else if current_row.fill_line_p {
            if i >= desired_glyphs.len() {
                self.output_cursor_to(
                    w,
                    vpos as i64,
                    i as i64,
                    i64::from(desired_row.y),
                    i64::from(desired_row.pixel_width),
                );
            }
            clear(self, w, updated_row, TEXT_AREA, -1)?;
            changed_p = true;
        } else if desired_row.pixel_width < current_row.pixel_width {
            if i >= desired_glyphs.len() {
                self.output_cursor_to(
                    w,
                    vpos as i64,
                    i as i64,
                    i64::from(desired_row.y),
                    i64::from(desired_row.pixel_width),
                );
            }
            let phys = self.window(w).phys_cursor;
            let xlim = if vpos as i64 == phys.vpos
                && (if desired_row.reversed_p {
                    phys.hpos < 0
                } else {
                    phys.hpos >= desired_glyphs.len() as i64
                }) {
                self.window_mut(w).flags.phys_cursor_on_p = false;
                -1
            } else {
                current_row.pixel_width
            };
            clear(self, w, updated_row, TEXT_AREA, xlim)?;
            changed_p = true;
        }
        Ok(changed_p)
    }

    fn update_window_line(
        &mut self,
        w: Value,
        vpos: usize,
        mouse_face_overwritten_p: &mut bool,
    ) -> LispResult<bool> {
        let f = self.window(w).frame;
        let rif = self.frame_rif(f);
        let mut changed_p = false;
        let mut partial_p = false;
        let updated_row = desired(w, vpos);
        let (mode_line_p, visible_height, full_width_p) = {
            let r = self.glyph_row(updated_row);
            (r.mode_line_p, r.visible_height, r.full_width_p)
        };
        if mode_line_p || visible_height > 0 {
            if !full_width_p && self.window(w).left_margin_cols > 0 {
                changed_p = true;
                self.update_marginal_area(w, updated_row, LEFT_MARGIN_AREA, vpos)?;
                self.glyph_row_mut(current(w, vpos)).redraw_fringe_bitmaps_p = true;
            }
            if self.update_text_area(w, updated_row, vpos, &mut partial_p)? {
                changed_p = true;
                if self.glyph_row(current(w, vpos)).mouse_face_p {
                    *mouse_face_overwritten_p = true;
                }
            }
            if !full_width_p && self.window(w).right_margin_cols > 0 {
                changed_p = true;
                self.update_marginal_area(w, updated_row, RIGHT_MARGIN_AREA, vpos)?;
            }
            let c = self.glyph_row(current(w, vpos));
            let d = self.glyph_row(updated_row);
            if !c.enabled_p
                || d.y != c.y
                || d.visible_height != c.visible_height
                || d.cursor_in_fringe_p != c.cursor_in_fringe_p
                || d.overlay_arrow_bitmap != c.overlay_arrow_bitmap
                || c.redraw_fringe_bitmaps_p
                || d.mode_line_p != c.mode_line_p
                || d.exact_window_width_line_p != c.exact_window_width_line_p
                || d.continuation_line_p() != c.continuation_line_p()
            {
                let hook = rif
                    .after_update_window_line_hook
                    .expect("redisplay interface after_update_window_line_hook");
                hook(self, w, updated_row);
            }
        }
        let was_stipple = self.glyph_row(current(w, vpos)).stipple_p;
        self.make_current(w, vpos);
        if partial_p && was_stipple {
            self.glyph_row_mut(current(w, vpos)).stipple_p = true;
        }
        Ok(changed_p)
    }

    fn set_window_cursor_after_update(&mut self, w: Value) {
        let f = self.window(w).frame;
        let (mut cx, mut cy, mut vpos, mut hpos) = (0, 0, 0, 0);
        let echo_area_window = self.xdisp.echo_area_window;
        let minibuf_window = self.windows.minibuf_window;
        if self.var_true(sym::CURSOR_IN_ECHO_AREA)
            && !self.xdisp.echo_area_buffer[0].is_nil()
            && minibuf_window == w
            && minibuf_window == echo_area_window
            && self.frame_has_minibuf_p(f)
            && self.frame(f).minibuffer_window == echo_area_window
        {
            let yb = self.window_text_bottom_y(w) as i32;
            let mut last_row = None;
            let mut i = 0;
            loop {
                let r = self.glyph_row(current(w, i));
                if !(r.enabled_p && (last_row.is_none() || r.bottom_y() <= yb)) {
                    break;
                }
                if r.used[TEXT_AREA] != 0 && r.area_glyphs(TEXT_AREA)[0].charpos >= 0 {
                    last_row = Some(i);
                }
                i += 1;
            }
            if let Some(lr) = last_row {
                let r = self.glyph_row(current(w, lr));
                let glyphs = r.area_glyphs(TEXT_AREA);
                let mut last = glyphs.len() - 1;
                while last > 0 && glyphs[last].charpos < 0 {
                    last -= 1;
                }
                for g in &glyphs[..last] {
                    cx += i64::from(g.pixel_width);
                    hpos += 1;
                }
                cy = i64::from(r.y);
                vpos = lr as i64;
            }
        } else {
            let c = self.window(w).cursor;
            cx = c.x;
            cy = c.y;
            hpos = c.hpos;
            vpos = c.vpos;
        }
        let (matrix_w, nrows) = self
            .window_matrix(w, WhichMatrix::Current)
            .map_or((0, 0), |m| (i64::from(m.matrix_w), i64::from(m.nrows)));
        hpos = hpos.clamp(-1, (matrix_w - 1).max(-1));
        vpos = vpos.clamp(0, (nrows - 1).max(0));
        self.output_cursor_to(w, vpos, hpos, cy, cx);
    }

    fn add_row_entry(
        &self,
        w: Value,
        which: WhichMatrix,
        vpos: usize,
        row_table: &mut [Option<usize>],
        entries: &mut Vec<RowEntry>,
    ) -> usize {
        let row = &self.window_matrix(w, which).expect("glyph matrix").rows[vpos];
        let i = row.hash as usize % row_table.len();
        let mut entry = row_table[i];
        while let Some(e) = entry {
            let other = &self
                .window_matrix(w, entries[e].which)
                .expect("glyph matrix")
                .rows[entries[e].vpos];
            if row_equal_p(other, row, true) {
                break;
            }
            entry = entries[e].next;
        }
        match entry {
            Some(e) => e,
            None => {
                entries.push(RowEntry {
                    old_uses: 0,
                    new_uses: 0,
                    new_line_number: 0,
                    which,
                    vpos,
                    next: row_table[i],
                });
                let e = entries.len() - 1;
                row_table[i] = Some(e);
                e
            }
        }
    }

    fn scrolling_window(&mut self, w: Value, tab_line_p: i32) -> LispResult<i32> {
        let f = self.window(w).frame;
        let rif = self.frame_rif(f);
        let yb = self.window_text_bottom_y(w) as i32;
        let cur_nrows = self
            .window_matrix(w, WhichMatrix::Current)
            .map_or(0, |m| m.nrows.max(0) as usize);
        let des_nrows = self
            .window_matrix(w, WhichMatrix::Desired)
            .map_or(0, |m| m.nrows.max(0) as usize);
        let mut i = tab_line_p.max(0) as usize;
        while i + 1 < cur_nrows {
            let c = self.glyph_row(current(w, i));
            if c.stipple_p {
                return Ok(0);
            }
            let d = self.glyph_row(desired(w, i));
            if c.enabled_p
                && d.enabled_p
                && !d.redraw_fringe_bitmaps_p
                && c.y == d.y
                && c.bottom_y() <= yb
                && d.bottom_y() <= yb
                && row_equal_p(c, d, true)
            {
                self.assign_window_row(w, i, i);
                self.glyph_row_mut(desired(w, i)).enabled_p = false;
            } else {
                break;
            }
            i += 1;
        }
        if !self.glyph_row(desired(w, i)).enabled_p {
            return Ok(-1);
        }
        let first_old = i;
        let first_new = i;
        while i + 1 < cur_nrows {
            if self.glyph_row(current(w, i)).stipple_p {
                return Ok(0);
            }
            i += 1;
        }
        i = first_new + 1;
        while i + 1 < des_nrows {
            let d = self.glyph_row(desired(w, i));
            if !d.enabled_p {
                return Ok(0);
            }
            let bottom = d.bottom_y();
            if bottom <= yb {
                i += 1;
            }
            if bottom >= yb {
                break;
            }
        }
        let mut last_new = i;
        i = first_old + 1;
        while i + 1 < cur_nrows {
            let bottom = self.glyph_row(current(w, i)).bottom_y();
            if bottom <= yb {
                i += 1;
            }
            if bottom >= yb {
                break;
            }
        }
        let mut last_old = i;
        let mut i = last_new;
        let mut j = last_old;
        while i > first_new + 1 && j > first_old + 1 {
            let c = self.glyph_row(current(w, j - 1));
            let d = self.glyph_row(desired(w, i - 1));
            if c.enabled_p && c.y == d.y && !d.redraw_fringe_bitmaps_p && row_equal_p(d, c, true) {
                i -= 1;
                j -= 1;
            } else {
                break;
            }
        }
        last_new = i;
        last_old = j;
        if last_new == first_new {
            return Ok(0);
        }
        let n = des_nrows + cur_nrows;
        let mut row_table = vec![None; next_almost_prime(3 * n)];
        let mut entries: Vec<RowEntry> = Vec::with_capacity(n);
        let mut old_lines: Vec<Option<usize>> = vec![None; cur_nrows];
        let mut new_lines: Vec<Option<usize>> = vec![None; des_nrows];
        for (i, slot) in old_lines
            .iter_mut()
            .enumerate()
            .take(last_old)
            .skip(first_old)
        {
            if self.glyph_row(current(w, i)).enabled_p {
                let e =
                    self.add_row_entry(w, WhichMatrix::Current, i, &mut row_table, &mut entries);
                *slot = Some(e);
                entries[e].old_uses += 1;
            } else {
                *slot = None;
            }
        }
        for (i, slot) in new_lines
            .iter_mut()
            .enumerate()
            .take(last_new)
            .skip(first_new)
        {
            let e = self.add_row_entry(w, WhichMatrix::Desired, i, &mut row_table, &mut entries);
            entries[e].new_uses += 1;
            entries[e].new_line_number = i;
            *slot = Some(e);
        }
        let mut runs: Vec<Run> = Vec::new();
        let mut i = first_old;
        while i < last_old {
            match old_lines[i] {
                Some(e) if entries[e].old_uses == 1 && entries[e].new_uses == 1 => {
                    let new_line = entries[e].new_line_number;
                    let mut run = Run {
                        current_vpos: i as i32,
                        current_y: self.glyph_row(current(w, i)).y,
                        desired_vpos: new_line as i32,
                        desired_y: self.glyph_row(desired(w, new_line)).y,
                        nrows: 1,
                        height: self.glyph_row(current(w, i)).height,
                    };
                    let mut p = i as i64 - 1;
                    let mut q = new_line as i64 - 1;
                    while p > first_old as i64
                        && q > first_new as i64
                        && old_lines[p as usize] == new_lines[q as usize]
                    {
                        let h = self.glyph_row(current(w, p as usize)).height;
                        run.current_vpos -= 1;
                        run.desired_vpos -= 1;
                        run.nrows += 1;
                        run.height += h;
                        run.desired_y -= h;
                        run.current_y -= h;
                        p -= 1;
                        q -= 1;
                    }
                    let mut p = i + 1;
                    let mut q = new_line + 1;
                    while p < last_old && q < last_new && old_lines[p] == new_lines[q] {
                        let h = self.glyph_row(current(w, p)).height;
                        run.nrows += 1;
                        run.height += h;
                        p += 1;
                        q += 1;
                    }
                    let pos = runs
                        .iter()
                        .position(|r| r.height <= run.height)
                        .unwrap_or(runs.len());
                    runs.insert(pos, run);
                    i += run.nrows as usize;
                }
                _ => i += 1,
            }
        }
        let nruns = runs.len();
        for i in 0..nruns {
            if runs[i].nrows <= 0 {
                continue;
            }
            let r = runs[i];
            if r.current_y != r.desired_y {
                let clear = rif
                    .clear_window_mouse_face
                    .expect("redisplay interface clear_window_mouse_face");
                clear(self, w)?;
                let scroll = rif
                    .scroll_run_hook
                    .expect("redisplay interface scroll_run_hook");
                scroll(self, w, &r)?;
            }
            let mut j = nruns - 1;
            while j > i {
                let mut p = runs[j];
                let mut truncated_p = false;
                if p.nrows > 0
                    && p.desired_y < r.desired_y + r.height
                    && p.desired_y + p.height > r.desired_y
                {
                    if p.desired_y < r.desired_y {
                        p.nrows = r.desired_vpos - p.desired_vpos;
                        p.height = r.desired_y - p.desired_y;
                        truncated_p = true;
                    } else {
                        let nrows_copied = r.desired_vpos + r.nrows - p.desired_vpos;
                        if p.nrows <= nrows_copied {
                            p.nrows = 0;
                        } else {
                            let height_copied = r.desired_y + r.height - p.desired_y;
                            p.current_vpos += nrows_copied;
                            p.desired_vpos += nrows_copied;
                            p.nrows -= nrows_copied;
                            p.current_y += height_copied;
                            p.desired_y += height_copied;
                            p.height -= height_copied;
                            truncated_p = true;
                        }
                    }
                }
                if r.current_y != r.desired_y
                    && p.current_y < r.desired_y + r.height
                    && p.current_y + p.height > r.desired_y
                {
                    p.nrows = 0;
                }
                runs[j] = p;
                if truncated_p && p.nrows > 0 {
                    let mut k = nruns - 1;
                    while runs[k].nrows == 0 || runs[k].height < p.height {
                        k -= 1;
                    }
                    runs.copy_within(j + 1..k + 1, j);
                    runs[k] = p;
                }
                j -= 1;
            }
            for j in 0..r.nrows {
                let vpos = (r.desired_vpos + j) as usize;
                let to_overlapped_p = self.glyph_row(current(w, vpos)).overlapped_p;
                {
                    let from = self.glyph_row_mut(desired(w, vpos));
                    from.redraw_fringe_bitmaps_p = from.fringe_bitmap_periodic_p;
                }
                self.assign_window_row(w, vpos, vpos);
                self.glyph_row_mut(desired(w, vpos)).enabled_p = false;
                self.glyph_row_mut(current(w, vpos)).overlapped_p = to_overlapped_p;
            }
        }
        Ok(if nruns > 0 { 1 } else { 0 })
    }

    fn assign_window_row(&mut self, w: Value, to: usize, from: usize) {
        let win = self.window_mut(w);
        if let (Some(cur), Some(des)) = (
            win.current_matrix.as_deref_mut(),
            win.desired_matrix.as_deref_mut(),
        ) {
            assign_row(&mut cur.rows[to], &mut des.rows[from]);
        }
    }
}
