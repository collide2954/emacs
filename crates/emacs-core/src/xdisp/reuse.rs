use crate::dispextern::{GlyphRowRef, It, ItMethod, Run, TEXT_AREA, TextPos};
use crate::dispnew::matrix::{
    WhichMatrix, clear_glyph_matrix, clear_glyph_matrix_rows, rotate_matrix,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::window::MarginUnit;

fn current(w: Value, vpos: usize) -> GlyphRowRef {
    GlyphRowRef::Window {
        window: w,
        desired: false,
        vpos,
    }
}

fn desired(w: Value, vpos: usize) -> GlyphRowRef {
    GlyphRowRef::Window {
        window: w,
        desired: true,
        vpos,
    }
}

fn ends_in_middle_of_char_p(row: &crate::dispextern::GlyphRow) -> bool {
    row.end.dpvec_index > 0 || row.end.overlay_string_index >= 0 || row.ends_in_middle_of_char_p
}

impl Lisp {
    pub(crate) fn init_to_row_start(
        &mut self,
        it: &mut It,
        w: Value,
        row: GlyphRowRef,
    ) -> LispResult<()> {
        let (start, width) = {
            let r = self.glyph_row(row);
            (r.start, r.continuation_lines_width)
        };
        self.init_from_display_pos(it, w, &start)?;
        it.start = start;
        it.continuation_lines_width = width;
        Ok(())
    }

    pub(crate) fn init_to_row_end(
        &mut self,
        it: &mut It,
        w: Value,
        row: GlyphRowRef,
    ) -> LispResult<bool> {
        let (end, continued_p, width, pixel_width) = {
            let r = self.glyph_row(row);
            (
                r.end,
                r.continued_p,
                r.continuation_lines_width,
                r.pixel_width,
            )
        };
        if self.init_from_display_pos(it, w, &end)? {
            if continued_p {
                it.continuation_lines_width = width + pixel_width;
            }
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn overlay_arrows_changed_p(&mut self, set_redisplay: bool) -> LispResult<bool> {
        let mut changed = false;
        let mut vlist = self.find_symbol_value(sym::OVERLAY_ARROW_VARIABLE_LIST);
        while vlist.is_cons() {
            let var = self.xcar(vlist);
            vlist = self.xcdr(vlist);
            if !var.is_symbol() {
                continue;
            }
            let val = self.find_symbol_value(var);
            if !self.is_marker(val) {
                continue;
            }
            let position = crate::marker::f_marker_position(self, &[val])?;
            let last_position = self.get(var, sym::LAST_ARROW_POSITION);
            let pstr = self.overlay_arrow_string_or_property(var);
            let last_string = self.get(var, sym::LAST_ARROW_STRING);
            if position != last_position || pstr != last_string {
                let buf = self.marker(val).buffer;
                if set_redisplay {
                    if !buf.is_nil() {
                        self.bset_redisplay(buf);
                    }
                    changed = true;
                } else {
                    return Ok(true);
                }
            }
        }
        Ok(changed)
    }

    fn find_last_row_displaying_text(
        &self,
        w: Value,
        last_visible_y: i32,
        start: Option<usize>,
    ) -> Option<usize> {
        let m = self.window_matrix(w, WhichMatrix::Current)?;
        let mut row = start.unwrap_or_else(|| m.first_text_row());
        let mut row_found = None;
        while m.rows[row].displays_text_p {
            row_found = Some(row);
            if m.rows[row].bottom_y() >= last_visible_y {
                break;
            }
            row += 1;
        }
        row_found
    }

    fn find_last_unchanged_at_beg_row(&mut self, w: Value) -> Option<usize> {
        let first_changed_pos = 1 + self.cur().text.beg_unchanged;
        let zv = self.cur().zv as i64;
        let yb = self.window_text_bottom_y(w) as i32;
        let m = self.window_matrix(w, WhichMatrix::Current)?;
        let mut row = m.first_text_row();
        let mut row_found = None;
        while m.rows[row].displays_text_p && m.rows[row].start_charpos() < first_changed_pos {
            let r = &m.rows[row];
            if r.end_charpos() <= first_changed_pos
                && !r.ends_at_zv_p
                && !(r.end_charpos() == first_changed_pos
                    && (r.continued_p || r.exact_window_width_line_p))
                && r.end.pos.charpos <= zv
            {
                row_found = Some(row);
            }
            if r.bottom_y() >= yb {
                break;
            }
            row += 1;
        }
        row_found
    }

    fn find_first_unchanged_at_end_row(
        &mut self,
        w: Value,
        delta: &mut i64,
        delta_bytes: &mut i64,
    ) -> Option<usize> {
        *delta = 0;
        *delta_bytes = 0;
        let end_unchanged = self.cur().text.end_unchanged;
        let (z, z_byte) = (self.cur().z() as i64, self.cur().z_byte() as i64);
        let win = self.window(w);
        let (window_end_pos, window_end_bytepos, window_end_vpos) = (
            win.window_end_pos,
            win.window_end_bytepos,
            win.window_end_vpos,
        );
        if window_end_pos >= end_unchanged {
            return None;
        }
        let m = self.window_matrix(w, WhichMatrix::Current)?;
        let mut row = window_end_vpos as usize;
        let mut row_found = None;
        if m.rows[row].displays_text_p {
            let z_old = m.rows[row].end_charpos() + window_end_pos;
            let z_byte_old = m.rows[row].maxpos.bytepos + window_end_bytepos;
            let first_text_row = m.first_text_row();
            *delta = z - z_old;
            *delta_bytes = z_byte - z_byte_old;
            let last_unchanged_pos = z - end_unchanged + 1;
            let last_unchanged_pos_old = last_unchanged_pos - *delta;
            while row > first_text_row {
                let r = &m.rows[row];
                if !r.enabled_p || !r.displays_text_p {
                    break;
                }
                if r.start_charpos() >= last_unchanged_pos_old {
                    row_found = Some(row);
                }
                row -= 1;
            }
        }
        row_found
    }

    fn bottom_text_row(&self, w: Value) -> usize {
        let m = self
            .window_matrix(w, WhichMatrix::Current)
            .expect("glyph matrix");
        m.nrows as usize - usize::from(self.window_wants_mode_line(w))
    }

    fn top_lines_height_of(&mut self, w: Value) -> i32 {
        (self.window_tab_line_height(w) + self.window_header_line_height(w)) as i32
    }

    fn scroll_window_display(&mut self, w: Value, run: &Run) -> LispResult<()> {
        let f = self.window(w).frame;
        self.update_begin(f);
        self.gui_update_window_begin(w);
        let rif = self.frame_rif(f);
        let clear = rif
            .clear_window_mouse_face
            .expect("redisplay interface clear_window_mouse_face");
        clear(self, w)?;
        let scroll = rif
            .scroll_run_hook
            .expect("redisplay interface scroll_run_hook");
        scroll(self, w, run)?;
        self.gui_update_window_end(w, false, false)?;
        self.update_end(f);
        Ok(())
    }

    pub fn try_window_reusing_current_matrix(&mut self, w: Value) -> LispResult<bool> {
        self.with_gc_deferred(|lisp| lisp.try_window_reusing_current_matrix_1(w))
    }

    fn try_window_reusing_current_matrix_1(&mut self, w: Value) -> LispResult<bool> {
        let f = self.window(w).frame;
        if !self.frame_window_p(f)
            || self.windows.windows_or_buffers_changed != 0
            || self.frame(f).flags.cursor_type_changed
            || self.overlay_arrow_in_current_buffer_p()
        {
            return Ok(false);
        }
        if !self
            .find_symbol_value(sym::SHOW_TRAILING_WHITESPACE)
            .is_nil()
        {
            return Ok(false);
        }
        let (header_line_p, tab_mode_line_p, header_mode_line_p) = {
            let m = self
                .window_matrix(w, WhichMatrix::Current)
                .expect("glyph matrix");
            (
                m.header_line_p,
                m.rows[0].mode_line_p,
                m.rows[m.header_line_row()].mode_line_p,
            )
        };
        if !header_line_p && self.window_wants_tab_line(w) != tab_mode_line_p {
            return Ok(false);
        }
        if self.window_wants_header_line(w) != header_mode_line_p {
            return Ok(false);
        }
        let mut start_row = self
            .window_matrix(w, WhichMatrix::Current)
            .expect("glyph matrix")
            .first_text_row();
        let start_row_row = self.glyph_row(current(w, start_row)).clone();
        if self.window(w).vscroll != 0 || self.matrix_row_partially_visible_p(w, &start_row_row) {
            return Ok(false);
        }
        if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
            clear_glyph_matrix(m);
        }
        if !self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS).is_nil() {
            return Ok(false);
        }
        let marker = self.window(w).start;
        let new_start = {
            let m = self.marker(marker);
            TextPos::new(m.charpos as i64, m.bytepos as i64)
        };
        let mut start = start_row_row.minpos;
        let mut start_vpos = start_row;
        let mode_line_row = self
            .window_matrix(w, WhichMatrix::Current)
            .expect("glyph matrix")
            .mode_line_row();
        if new_start.charpos <= start.charpos {
            if self.in_ellipses_for_invisible_text_p(&start_row_row.start, w)? {
                return Ok(false);
            }
            let mut it = It::default();
            self.start_display(&mut it, w, new_start)?;
            if it.charpos() > start.charpos && it.charpos() >= start_row_row.end.pos.charpos {
                return Ok(false);
            }
            self.window_mut(w).cursor.vpos = -1;
            let mut last_text_row: Option<usize> = None;
            let mut last_reused_text_row: Option<usize> = None;
            while it.current_y < it.last_visible_y && !self.frame(f).fonts_changed {
                while it.charpos() > start.charpos {
                    start_row += 1;
                    start = self.glyph_row(current(w, start_row)).minpos;
                    let row = self.glyph_row(current(w, start_row)).clone();
                    let zv = self.cur().zv as i64;
                    if start_row == mode_line_row - 1
                        || self.window(w).vscroll != 0
                        || self.matrix_row_partially_visible_p(w, &row)
                        || start.charpos == zv
                    {
                        if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                            clear_glyph_matrix(m);
                        }
                        return Ok(false);
                    }
                    start_vpos = start_row;
                }
                if it.charpos() == start.charpos && it.current.dpvec_index < 0 {
                    break;
                }
                self.it_row_mut(&it).reversed_p = false;
                if self.display_line(&mut it, -1)? {
                    last_text_row = Some(Self::row_vpos(&it) - 1);
                }
            }
            if it.current_y < it.last_visible_y {
                let first_text_row = self
                    .window_matrix(w, WhichMatrix::Current)
                    .expect("glyph matrix")
                    .first_text_row();
                let nrows_scrolled = it.vpos - (start_row - first_text_row) as i32;
                if self.window(w).cursor.vpos < 0 {
                    let dy = it.current_y - self.glyph_row(current(w, start_row)).y;
                    let pt = self.cur().pt as i64;
                    let row = self.row_containing_pos(
                        w,
                        pt,
                        WhichMatrix::Current,
                        first_text_row,
                        None,
                        dy,
                    )?;
                    match row {
                        Some(row) => {
                            self.set_cursor_from_row(
                                w,
                                row,
                                WhichMatrix::Current,
                                0,
                                0,
                                dy,
                                nrows_scrolled,
                            )?;
                        }
                        None => {
                            if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                                clear_glyph_matrix(m);
                            }
                            return Ok(false);
                        }
                    }
                }
                let run = Run {
                    current_y: self.glyph_row(current(w, start_row)).y,
                    desired_y: it.current_y,
                    height: it.last_visible_y - it.current_y,
                    ..Run::default()
                };
                if run.height > 0 && run.current_y != run.desired_y {
                    self.scroll_window_display(w, &run)?;
                }
                let bottom_row = self.bottom_text_row(w);
                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Current) {
                    rotate_matrix(m, start_vpos, bottom_row, nrows_scrolled);
                }
                for i in 0..nrows_scrolled.max(0) as usize {
                    self.glyph_row_mut(current(w, start_row + i)).enabled_p = false;
                }
                let min_y = self.top_lines_height_of(w);
                let max_y = it.last_visible_y;
                let mut row = start_row + nrows_scrolled.max(0) as usize;
                while row < bottom_row {
                    let r = self.glyph_row_mut(current(w, row));
                    r.y = it.current_y;
                    r.visible_height = r.height;
                    if r.y < min_y {
                        r.visible_height -= min_y - r.y;
                    }
                    if r.y + r.height > max_y {
                        r.visible_height -= r.y + r.height - max_y;
                    }
                    if r.fringe_bitmap_periodic_p {
                        r.redraw_fringe_bitmaps_p = true;
                    }
                    it.current_y += r.height;
                    let (displays_text, bottom) = (r.displays_text_p, r.bottom_y());
                    if displays_text {
                        last_reused_text_row = Some(row);
                    }
                    if bottom >= it.last_visible_y {
                        break;
                    }
                    row += 1;
                }
                row += 1;
                while row < bottom_row {
                    let r = self.glyph_row_mut(current(w, row));
                    r.enabled_p = false;
                    r.mode_line_p = false;
                    r.tab_line_p = false;
                    row += 1;
                }
            }
            if let Some(row) = last_reused_text_row {
                self.adjust_window_ends(w, row, true);
            } else if let Some(row) = last_text_row {
                self.adjust_window_ends(w, row, false);
            } else {
                let (z, z_byte) = (self.cur().z() as i64, self.cur().z_byte() as i64);
                let (zv, zv_byte) = (self.cur().zv as i64, self.cur().zv_byte as i64);
                let win = self.window_mut(w);
                win.window_end_bytepos = z_byte - zv_byte;
                win.window_end_pos = z - zv;
                win.window_end_vpos = 0;
            }
            self.window_mut(w).flags.window_end_valid = false;
            if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                m.no_scrolling_p = true;
            }
            return Ok(true);
        }
        if new_start.charpos > start.charpos {
            let yb = self.window_text_bottom_y(w) as i32;
            let mut first_reusable_row = start_row;
            loop {
                let r = self.glyph_row(current(w, first_reusable_row));
                if r.enabled_p && r.bottom_y() < yb && r.start_charpos() < new_start.charpos {
                    first_reusable_row += 1;
                } else {
                    break;
                }
            }
            {
                let r = self.glyph_row(current(w, first_reusable_row));
                if r.bottom_y() >= yb || !r.enabled_p || r.start_charpos() != new_start.charpos {
                    return Ok(false);
                }
            }
            let pt = self.cur().pt as i64;
            let mut pt_row: Option<usize> = None;
            let mut first_row_to_display = first_reusable_row;
            while self.glyph_row(current(w, first_row_to_display)).bottom_y() < yb {
                let r = self.glyph_row(current(w, first_row_to_display));
                if pt >= r.start_charpos()
                    && (pt < r.end_charpos()
                        || (pt == r.end_charpos() && r.ends_at_zv_p && pt_row.is_none()))
                {
                    pt_row = Some(first_row_to_display);
                }
                first_row_to_display += 1;
            }
            if self.glyph_row(current(w, first_row_to_display)).y >= yb {
                return Ok(false);
            }
            let mut it = It::default();
            self.init_to_row_start(&mut it, w, current(w, first_row_to_display))?;
            let nrows_scrolled = (first_reusable_row - start_vpos) as i32;
            it.vpos = first_row_to_display as i32 - nrows_scrolled;
            let top_lines = self.top_lines_height_of(w);
            it.current_y = self.glyph_row(current(w, first_row_to_display)).y
                - self.glyph_row(current(w, first_reusable_row)).y
                + top_lines;
            it.glyph_row = Some(desired(w, it.vpos as usize));
            if pt_row.is_none() {
                self.window_mut(w).cursor.vpos = -1;
            }
            let mut last_text_row: Option<usize> = None;
            while it.current_y < it.last_visible_y && !self.frame(f).fonts_changed {
                let cursor_vpos = self.window(w).cursor.vpos;
                if self.display_line(&mut it, cursor_vpos)? {
                    last_text_row = Some(Self::row_vpos(&it) - 1);
                }
            }
            if pt_row.is_some() {
                let dy = i64::from(
                    self.glyph_row(current(w, first_reusable_row)).y
                        - self.glyph_row(current(w, start_row)).y,
                );
                let win = self.window_mut(w);
                win.cursor.vpos -= i64::from(nrows_scrolled);
                win.cursor.y -= dy;
            }
            if self.window(w).cursor.vpos < 0 {
                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                    clear_glyph_matrix(m);
                }
                return Ok(false);
            }
            let current_y = self.glyph_row(current(w, first_reusable_row)).y;
            let desired_y = top_lines;
            let run = Run {
                current_y,
                desired_y,
                height: it.last_visible_y - current_y,
                ..Run::default()
            };
            let dy = run.current_y - run.desired_y;
            if run.height != 0 {
                self.scroll_window_display(w, &run)?;
            }
            let bottom_row = self.bottom_text_row(w);
            let min_y = top_lines;
            let max_y = it.last_visible_y;
            let mut row = first_reusable_row;
            while row < first_row_to_display {
                let r = self.glyph_row_mut(current(w, row));
                r.y -= dy;
                r.visible_height = r.height;
                if r.y < min_y {
                    r.visible_height -= min_y - r.y;
                }
                if r.y + r.height > max_y {
                    r.visible_height -= r.y + r.height - max_y;
                }
                if r.fringe_bitmap_periodic_p {
                    r.redraw_fringe_bitmaps_p = true;
                }
                row += 1;
            }
            if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Current) {
                rotate_matrix(m, start_vpos, bottom_row, -nrows_scrolled);
            }
            row -= nrows_scrolled as usize;
            while row < bottom_row {
                self.glyph_row_mut(current(w, row)).enabled_p = false;
                row += 1;
            }
            if pt_row.is_some() {
                let mut row = self.window(w).cursor.vpos as usize;
                loop {
                    let r = self.glyph_row(current(w, row));
                    if !(row < bottom_row && pt >= r.end_charpos() && !r.ends_at_zv_p) {
                        break;
                    }
                    let y = r.y;
                    let win = self.window_mut(w);
                    win.cursor.vpos += 1;
                    win.cursor.y = i64::from(y);
                    row += 1;
                }
                if row < bottom_row {
                    let buffer = self.window(w).contents;
                    if !self
                        .buffer_local_value(sym::BIDI_DISPLAY_REORDERING, buffer)
                        .is_nil()
                    {
                        if !self.set_cursor_from_row(w, row, WhichMatrix::Current, 0, 0, 0, 0)? {
                            if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                                clear_glyph_matrix(m);
                            }
                            return Ok(false);
                        }
                    } else {
                        let glyphs: Vec<_> = self
                            .glyph_row(current(w, row))
                            .area_glyphs(TEXT_AREA)
                            .to_vec();
                        let mut hpos = self.window(w).cursor.hpos.max(0) as usize;
                        while hpos < glyphs.len()
                            && (!self.is_buffer(glyphs[hpos].object) || glyphs[hpos].charpos < pt)
                        {
                            let win = self.window_mut(w);
                            win.cursor.hpos += 1;
                            win.cursor.x += i64::from(glyphs[hpos].pixel_width);
                            hpos += 1;
                        }
                    }
                }
            }
            if let Some(row) = last_text_row {
                self.adjust_window_ends(w, row, false);
            } else {
                self.window_mut(w).window_end_vpos -= i64::from(nrows_scrolled);
            }
            self.window_mut(w).flags.window_end_valid = false;
            if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                m.no_scrolling_p = true;
            }
            return Ok(true);
        }
        Ok(false)
    }

    pub fn try_window_id(&mut self, w: Value) -> LispResult<i32> {
        self.with_gc_deferred(|lisp| lisp.try_window_id_1(w))
    }

    fn try_window_id_1(&mut self, w: Value) -> LispResult<i32> {
        let f = self.window(w).frame;
        let marker = self.window(w).start;
        let start = {
            let m = self.marker(marker);
            TextPos::new(m.charpos as i64, m.bytepos as i64)
        };
        if self.window(w).mini {
            return Ok(0);
        }
        if self.windows.windows_or_buffers_changed != 0 || self.frame(f).flags.cursor_type_changed {
            return Ok(0);
        }
        if self.window(w).last_overlay_modified != self.cur().text.overlay_modiff {
            return Ok(0);
        }
        if self.cur().clip_changed || self.cur().prevent_redisplay_optimizations_p {
            return Ok(0);
        }
        let pt = self.cur().pt as i64;
        if pt < start.charpos {
            return Ok(0);
        }
        if self.window(w).last_modified == 0 {
            return Ok(0);
        }
        if self.window(w).hscroll != 0 {
            return Ok(0);
        }
        if !self.window(w).flags.window_end_valid {
            return Ok(0);
        }
        if !self
            .find_symbol_value(sym::SHOW_TRAILING_WHITESPACE)
            .is_nil()
        {
            return Ok(0);
        }
        if self.overlay_arrows_changed_p(false)? {
            return Ok(0);
        }
        let buffer = self.window(w).contents;
        if !self.buffer_local_value(sym::WORD_WRAP, buffer).is_nil() {
            return Ok(0);
        }
        if !self
            .buffer_local_value(sym::BIDI_DISPLAY_REORDERING, buffer)
            .is_nil()
            && self
                .buffer_local_value(sym::BIDI_PARAGRAPH_DIRECTION, buffer)
                .is_nil()
        {
            return Ok(0);
        }
        if !self.buffer_local_value(sym::LINE_SPACING, buffer).is_nil() {
            return Ok(0);
        }
        let dln = self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS);
        if dln == sym::RELATIVE
            || dln == sym::VISUAL
            || (!dln.is_nil()
                && crate::xfaces::f_internal_lisp_face_equal_p(
                    self,
                    &[sym::LINE_NUMBER, sym::LINE_NUMBER_CURRENT_LINE, f, sym::T],
                )?
                .is_nil())
        {
            return Ok(0);
        }
        if self.var_true(sym::COMPOSITION_BREAK_AT_POINT) {
            return Ok(0);
        }
        {
            let b = self.cur_mut();
            let z_byte = b.z_byte() as i64;
            if b.text.modiff > b.text.save_modiff
                || b.text.beg_unchanged + b.text.end_unchanged > z_byte
            {
                let gpt = b.text.gpt() as i64;
                let z = b.z() as i64;
                if gpt - 1 < b.text.beg_unchanged {
                    b.text.beg_unchanged = gpt - 1;
                }
                if z - gpt < b.text.end_unchanged {
                    b.text.end_unchanged = z - gpt;
                }
            }
        }
        let (z, z_byte) = (self.cur().z() as i64, self.cur().z_byte() as i64);
        let first_changed_charpos = 1 + self.cur().text.beg_unchanged;
        let last_changed_charpos = z - self.cur().text.end_unchanged;
        let window_end_vpos = self.window(w).window_end_vpos as usize;
        let row = self.glyph_row(current(w, window_end_vpos)).clone();
        let begv = self.cur().begv as i64;
        if row.displays_text_p
            && ((last_changed_charpos < start.charpos && start.charpos == begv)
                || (last_changed_charpos < start.charpos - 1
                    && self.cur().text.byte_at(start.bytepos as usize - 1) == b'\n'))
        {
            let win = self.window(w);
            let z_old = row.end_charpos() + win.window_end_pos;
            let z_byte_old = row.maxpos.bytepos + win.window_end_bytepos;
            let z_delta = z - z_old;
            let z_delta_bytes = z_byte - z_byte_old;
            if pt >= row.end_charpos() + z_delta {
                return Ok(0);
            }
            let r0 = self
                .window_matrix(w, WhichMatrix::Current)
                .expect("glyph matrix")
                .first_text_row();
            let text_bottom = self.window_text_bottom_y(w) as i32;
            let r0_row = self.glyph_row(current(w, r0));
            if start.charpos == r0_row.start_charpos() + z_delta
                && start.bytepos == r0_row.minpos.bytepos + z_delta_bytes
                && !(pt >= row.start_charpos() + z_delta && row.bottom_y() > text_bottom)
            {
                if z_delta != 0 || z_delta_bytes != 0 {
                    let r1 = self.bottom_text_row(w);
                    self.increment_matrix_positions(
                        w,
                        WhichMatrix::Current,
                        r0,
                        r1,
                        z_delta,
                        z_delta_bytes,
                    );
                }
                if let Some(row) =
                    self.row_containing_pos(w, pt, WhichMatrix::Current, r0, None, 0)?
                {
                    self.set_cursor_from_row(w, row, WhichMatrix::Current, 0, 0, 0, 0)?;
                }
                return Ok(1);
            }
        }
        let current_zv = self
            .window_matrix(w, WhichMatrix::Current)
            .expect("glyph matrix")
            .zv;
        if first_changed_charpos >= row.end_charpos()
            && (current_zv > row.end_charpos() || first_changed_charpos == last_changed_charpos)
        {
            if pt >= row.end_charpos() {
                return Ok(0);
            }
            let r0 = self
                .window_matrix(w, WhichMatrix::Current)
                .expect("glyph matrix")
                .first_text_row();
            let r0_minpos = self.glyph_row(current(w, r0)).minpos;
            let text_bottom = self.window_text_bottom_y(w) as i32;
            if start == r0_minpos && !(pt >= row.start_charpos() && row.bottom_y() > text_bottom) {
                let win = self.window_mut(w);
                win.window_end_pos = z - row.end_charpos();
                win.window_end_bytepos = z_byte - row.maxpos.bytepos;
                if let Some(row) =
                    self.row_containing_pos(w, pt, WhichMatrix::Current, r0, None, 0)?
                {
                    self.set_cursor_from_row(w, row, WhichMatrix::Current, 0, 0, 0, 0)?;
                }
                return Ok(2);
            }
        }
        if start.charpos >= first_changed_charpos && start.charpos <= last_changed_charpos {
            return Ok(0);
        }
        let first_text_row = self
            .window_matrix(w, WhichMatrix::Current)
            .expect("glyph matrix")
            .first_text_row();
        if start != self.glyph_row(current(w, first_text_row)).minpos {
            return Ok(0);
        }
        {
            let r = self.glyph_row(current(w, window_end_vpos));
            if r.start_charpos() == r.end_charpos() {
                return Ok(0);
            }
        }
        let mut it = It::default();
        let start_pos;
        let mut last_unchanged_at_beg_row = self.find_last_unchanged_at_beg_row(w);
        if let Some(mut row) = last_unchanged_at_beg_row {
            loop {
                let r = self.glyph_row(current(w, row));
                if (ends_in_middle_of_char_p(r) || r.ends_in_newline_from_string_p) && row > 0 {
                    row -= 1;
                } else {
                    break;
                }
            }
            last_unchanged_at_beg_row = Some(row);
            if ends_in_middle_of_char_p(self.glyph_row(current(w, row))) {
                return Ok(0);
            }
            if !self.init_to_row_end(&mut it, w, current(w, row))? {
                return Ok(0);
            }
            let prefix_string = it.sp == 1
                && it.method == ItMethod::FromString
                && !it.string_from_prefix_prop_p
                && {
                    let pos = Value::fixnum(it.charpos());
                    !self
                        .get_char_property(pos, sym::LINE_PREFIX, buffer)?
                        .is_nil()
                        || !self
                            .get_char_property(pos, sym::WRAP_PREFIX, buffer)?
                            .is_nil()
                };
            if (it.sp > 1 && it.method == ItMethod::FromImage && it.image_id == -1) || prefix_string
            {
                return Ok(0);
            }
            start_pos = it.current.pos;
            it.vpos = 1 + row as i32;
            it.glyph_row = Some(desired(w, it.vpos as usize));
            it.current_y = self.glyph_row(current(w, row)).bottom_y();
        } else {
            self.start_display(&mut it, w, start)?;
            it.vpos = it.first_vpos;
            start_pos = it.current.pos;
        }
        if !self.window(w).flags.window_end_valid {
            return Ok(0);
        }
        let mut delta = 0;
        let mut delta_bytes = 0;
        let mut first_unchanged_at_end_row =
            self.find_first_unchanged_at_end_row(w, &mut delta, &mut delta_bytes);
        let mut stop_pos = 0;
        let mut first_unchanged_at_end_vpos = 0usize;
        if let Some(mut row) = first_unchanged_at_end_row {
            loop {
                let r = self.glyph_row(current(w, row));
                if r.continuation_line_p() && r.displays_text_p && r.bottom_y() < it.last_visible_y
                {
                    row += 1;
                } else {
                    break;
                }
            }
            let r = self.glyph_row(current(w, row));
            if !r.displays_text_p || r.bottom_y() >= it.last_visible_y {
                first_unchanged_at_end_row = None;
            } else {
                stop_pos = r.start_charpos() + delta;
                first_unchanged_at_end_vpos = row;
                first_unchanged_at_end_row = Some(row);
            }
        } else if last_unchanged_at_beg_row.is_none() {
            return Ok(0);
        }
        self.window_mut(w).cursor.vpos = -1;
        let mut last_text_row: Option<usize> = None;
        self.xdisp.overlay_arrow_seen = false;
        if it.current_y < it.last_visible_y
            && !self.frame(f).fonts_changed
            && (first_unchanged_at_end_row.is_none() || it.charpos() < stop_pos)
        {
            self.it_row_mut(&it).reversed_p = false;
        }
        while it.current_y < it.last_visible_y
            && !self.frame(f).fonts_changed
            && (first_unchanged_at_end_row.is_none() || it.charpos() < stop_pos)
        {
            if self.display_line(&mut it, -1)? {
                last_text_row = Some(Self::row_vpos(&it) - 1);
            }
        }
        if self.frame(f).fonts_changed {
            return Ok(-1);
        }
        if !self.window(w).flags.window_end_valid {
            if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                clear_glyph_matrix(m);
            }
            return Ok(-1);
        }
        let mut run = Run::default();
        let dvpos;
        let dy;
        match first_unchanged_at_end_row {
            Some(row) if it.current_y < it.last_visible_y => {
                dvpos = it.vpos - row as i32;
                let row_y = self.glyph_row(current(w, row)).y;
                dy = it.current_y - row_y;
                run.current_y = row_y;
                run.desired_y = run.current_y + dy;
                run.height = it.last_visible_y - run.current_y.max(run.desired_y);
            }
            _ => {
                delta = 0;
                delta_bytes = 0;
                dvpos = 0;
                dy = 0;
                first_unchanged_at_end_row = None;
            }
        }
        if self.window(w).cursor.vpos < 0 {
            if pt < start_pos.charpos && last_unchanged_at_beg_row.is_some() {
                let end = last_unchanged_at_beg_row.map(|r| r + 1);
                if let Some(row) =
                    self.row_containing_pos(w, pt, WhichMatrix::Current, first_text_row, end, 0)?
                {
                    self.set_cursor_from_row(w, row, WhichMatrix::Current, 0, 0, 0, 0)?;
                }
            } else if let Some(fuer) = first_unchanged_at_end_row
                && let Some(row) =
                    self.row_containing_pos(w, pt - delta, WhichMatrix::Current, fuer, None, 0)?
            {
                self.set_cursor_from_row(
                    w,
                    row,
                    WhichMatrix::Current,
                    delta,
                    delta_bytes,
                    dy,
                    dvpos,
                )?;
            }
            if self.window(w).cursor.vpos < 0 {
                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                    clear_glyph_matrix(m);
                }
                return Ok(-1);
            }
        }
        {
            let mut top_scroll_margin = self.window_scroll_margin(w, MarginUnit::InPixels);
            let bot_scroll_margin = top_scroll_margin;
            let cursor_vpos = self.window(w).cursor.vpos as usize;
            let cursor_height = self.glyph_row(desired(w, cursor_vpos)).height;
            if self.window_wants_header_line(w) {
                top_scroll_margin += self.current_header_line_height(w) as i32;
            }
            if self.window_wants_tab_line(w) {
                top_scroll_margin += self.current_tab_line_height(w) as i32;
            }
            let cursor_y = self.window(w).cursor.y;
            let fully = self.cursor_row_fully_visible_p(w, false, true, true)?;
            if (cursor_y < i64::from(top_scroll_margin) && start.charpos > begv)
                || cursor_y
                    + if fully {
                        1
                    } else {
                        i64::from(cursor_height + bot_scroll_margin)
                    }
                    > i64::from(it.last_visible_y)
            {
                self.window_mut(w).cursor.vpos = -1;
                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                    clear_glyph_matrix(m);
                }
                return Ok(-1);
            }
        }
        if dy != 0 && run.height != 0 {
            self.scroll_window_display(w, &run)?;
        }
        let bottom_vpos = self.bottom_text_row(w);
        if dvpos < 0 {
            if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Current) {
                let first = (first_unchanged_at_end_vpos as i32 + dvpos) as usize;
                rotate_matrix(m, first, bottom_vpos, dvpos);
                clear_glyph_matrix_rows(m, (bottom_vpos as i32 + dvpos) as usize, bottom_vpos);
            }
        } else if dvpos > 0
            && let Some(m) = self.window_matrix_mut(w, WhichMatrix::Current)
        {
            rotate_matrix(m, first_unchanged_at_end_vpos, bottom_vpos, dvpos);
            clear_glyph_matrix_rows(
                m,
                first_unchanged_at_end_vpos,
                first_unchanged_at_end_vpos + dvpos as usize,
            );
        }
        let shifted = (first_unchanged_at_end_vpos as i32 + dvpos).max(0) as usize;
        if delta != 0 || delta_bytes != 0 {
            self.increment_matrix_positions(
                w,
                WhichMatrix::Current,
                shifted,
                bottom_vpos,
                delta,
                delta_bytes,
            );
        }
        if dy != 0 {
            self.shift_glyph_matrix(w, WhichMatrix::Current, shifted, bottom_vpos, dy);
        }
        if let Some(row) = first_unchanged_at_end_row {
            let row = (row as i32 + dvpos) as usize;
            let r = self.glyph_row(current(w, row));
            first_unchanged_at_end_row =
                (r.y < it.last_visible_y && r.displays_text_p).then_some(row);
        }
        let mut last_text_row_at_end: Option<usize> = None;
        if dy < 0 {
            let last_vpos = (self.window(w).window_end_vpos as i32 + dvpos) as usize;
            let mut last_row = last_vpos;
            let (bottom, row_y) = {
                let r = self.glyph_row(current(w, last_row));
                (r.bottom_y(), r.y)
            };
            if bottom - dy >= it.last_visible_y {
                self.init_to_row_start(&mut it, w, current(w, last_row))?;
                it.vpos = last_vpos as i32;
                it.current_y = row_y;
            } else {
                self.init_to_row_end(&mut it, w, current(w, last_row))?;
                it.vpos = 1 + last_vpos as i32;
                it.current_y = bottom;
                last_row += 1;
            }
            it.continuation_lines_width = self
                .glyph_row(current(w, last_row))
                .continuation_lines_width;
            it.hpos = 0;
            it.current_x = 0;
            it.wrap_prefix_width = 0;
            it.glyph_row = Some(desired(w, it.vpos as usize));
            while it.current_y < it.last_visible_y && !self.frame(f).fonts_changed {
                self.glyph_row_mut(current(w, it.vpos as usize)).enabled_p = false;
                let cursor_vpos = self.window(w).cursor.vpos;
                if self.display_line(&mut it, cursor_vpos)? {
                    last_text_row_at_end = Some(Self::row_vpos(&it) - 1);
                }
            }
        }
        if let (Some(fuer), None) = (first_unchanged_at_end_row, last_text_row_at_end) {
            let row = self
                .find_last_row_displaying_text(w, it.last_visible_y, Some(fuer))
                .expect("a row displaying text");
            self.adjust_window_ends(w, row, true);
        } else if let Some(row) = last_text_row_at_end {
            self.adjust_window_ends(w, row, false);
        } else if let Some(row) = last_text_row {
            self.adjust_window_ends(w, row, false);
        } else if first_unchanged_at_end_row.is_none() {
            let first_vpos = i64::from(self.window_wants_tab_line(w))
                + i64::from(self.window_wants_header_line(w));
            let mut vpos = self.window(w).window_end_vpos;
            let mut found = None;
            while found.is_none() {
                assert!(first_vpos <= vpos);
                let d = self.glyph_row(desired(w, vpos as usize));
                if d.enabled_p {
                    if d.displays_text_p {
                        found = Some((d.end_charpos(), d.maxpos.bytepos));
                    }
                } else {
                    let c = self.glyph_row(current(w, vpos as usize));
                    if c.displays_text_p {
                        found = Some((c.end_charpos(), c.maxpos.bytepos));
                    }
                }
                vpos -= 1;
            }
            let (end_charpos, end_bytepos) = found.expect("a row displaying text");
            let win = self.window_mut(w);
            win.window_end_vpos = vpos + 1;
            win.window_end_pos = z - end_charpos;
            win.window_end_bytepos = z_byte - end_bytepos;
        } else {
            unreachable!("emacs_abort in try_window_id");
        }
        self.window_mut(w).flags.window_end_valid = false;
        if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
            m.no_scrolling_p = true;
        }
        Ok(3)
    }
}
