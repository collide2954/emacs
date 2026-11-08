use crate::dispextern::{
    Glyph, GlyphMatrix, GlyphRow, GlyphRowArea, LAST_AREA, LEFT_MARGIN_AREA, RIGHT_MARGIN_AREA,
    TEXT_AREA,
};
use crate::lisp::Lisp;
use crate::value::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dim {
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhichMatrix {
    Current,
    Desired,
}

fn margin_glyphs_to_reserve(total_cols: i64, total_glyphs: i32, margin: i64) -> i32 {
    if margin > 0 {
        let width = total_cols as i32;
        let d = (margin.max(0) as f64).min(f64::from(width / 2 - 1));
        return ((f64::from(total_glyphs) / f64::from(width) * d) as i32).max(1);
    }
    0
}

fn xpalloc_len(n0: usize, incr_min: usize) -> usize {
    let n = n0 + (n0 >> 1);
    if n - n0 < incr_min { n0 + incr_min } else { n }
}

pub fn clear_glyph_row(row: &mut GlyphRow) {
    let glyphs = std::mem::take(&mut row.glyphs);
    let area = row.area;
    *row = GlyphRow {
        glyphs,
        area,
        ..GlyphRow::default()
    };
}

pub fn row_hash(row: &GlyphRow) -> u32 {
    let mut hashval: u32 = 0;
    for area in LEFT_MARGIN_AREA..LAST_AREA {
        for g in row.area_glyphs(area) {
            hashval = ((hashval << 4).wrapping_add(hashval >> 24) & 0x0fff_ffff)
                .wrapping_add(g.u.val())
                .wrapping_add(g.face_id)
                .wrapping_add(u32::from(g.padding_p))
                .wrapping_add((g.kind as u32) << 2);
        }
    }
    hashval
}

fn swap_glyph_pointers(a: &mut GlyphRow, b: &mut GlyphRow) {
    std::mem::swap(&mut a.glyphs, &mut b.glyphs);
    std::mem::swap(&mut a.area, &mut b.area);
    std::mem::swap(&mut a.used, &mut b.used);
    std::mem::swap(&mut a.hash, &mut b.hash);
}

fn copy_row_except_pointers(to: &mut GlyphRow, from: &mut GlyphRow) {
    let glyphs = std::mem::take(&mut from.glyphs);
    let mut copy = from.clone();
    from.glyphs = glyphs;
    std::mem::swap(&mut copy.glyphs, &mut to.glyphs);
    copy.area = to.area;
    copy.used = to.used;
    copy.hash = to.hash;
    *to = copy;
}

pub fn assign_row(to: &mut GlyphRow, from: &mut GlyphRow) {
    swap_glyph_pointers(to, from);
    copy_row_except_pointers(to, from);
}

pub fn row_equal_p(a: &GlyphRow, b: &GlyphRow, mouse_face_p: bool) -> bool {
    if std::ptr::eq(a, b) {
        return true;
    }
    if a.hash != b.hash {
        return false;
    }
    if mouse_face_p && a.mouse_face_p != b.mouse_face_p {
        return false;
    }
    for area in LEFT_MARGIN_AREA..LAST_AREA {
        if a.used[area] != b.used[area] {
            return false;
        }
        let (ga, gb) = (a.area_glyphs(area), b.area_glyphs(area));
        if !ga.iter().zip(gb).all(|(x, y)| x.equal_p(y)) {
            return false;
        }
    }
    !(a.fill_line_p != b.fill_line_p
        || a.cursor_in_fringe_p != b.cursor_in_fringe_p
        || a.left_fringe_bitmap != b.left_fringe_bitmap
        || a.left_fringe_face_id != b.left_fringe_face_id
        || a.left_fringe_offset != b.left_fringe_offset
        || a.right_fringe_bitmap != b.right_fringe_bitmap
        || a.right_fringe_face_id != b.right_fringe_face_id
        || a.right_fringe_offset != b.right_fringe_offset
        || a.fringe_bitmap_periodic_p != b.fringe_bitmap_periodic_p
        || a.overlay_arrow_bitmap != b.overlay_arrow_bitmap
        || a.exact_window_width_line_p != b.exact_window_width_line_p
        || a.overlapped_p != b.overlapped_p
        || a.continuation_line_p() != b.continuation_line_p()
        || a.reversed_p != b.reversed_p
        || a.x != b.x
        || a.ascent != b.ascent
        || a.phys_ascent != b.phys_ascent
        || a.phys_height != b.phys_height
        || a.visible_height != b.visible_height)
}

fn reverse_rows(matrix: &mut GlyphMatrix, start: usize, end: usize) {
    matrix.rows[start..end].reverse();
}

pub fn rotate_matrix(matrix: &mut GlyphMatrix, first: usize, last: usize, by: i32) {
    if by < 0 {
        let by = (-by) as usize;
        reverse_rows(matrix, first, first + by);
        reverse_rows(matrix, first + by, last);
        reverse_rows(matrix, first, last);
    } else if by > 0 {
        let by = by as usize;
        reverse_rows(matrix, last - by, last);
        reverse_rows(matrix, first, last - by);
        reverse_rows(matrix, first, last);
    }
}

pub fn clear_glyph_matrix_rows(matrix: &mut GlyphMatrix, start: usize, end: usize) {
    for row in &mut matrix.rows[start..end] {
        row.enabled_p = false;
    }
}

pub fn clear_glyph_matrix(matrix: &mut GlyphMatrix) {
    let nrows = matrix.nrows.max(0) as usize;
    clear_glyph_matrix_rows(matrix, 0, nrows);
    matrix.no_scrolling_p = false;
}

impl Lisp {
    pub fn window_matrix(&self, w: Value, which: WhichMatrix) -> Option<&GlyphMatrix> {
        let win = self.window(w);
        match which {
            WhichMatrix::Current => win.current_matrix.as_deref(),
            WhichMatrix::Desired => win.desired_matrix.as_deref(),
        }
    }

    pub fn window_matrix_mut(&mut self, w: Value, which: WhichMatrix) -> Option<&mut GlyphMatrix> {
        let win = self.window_mut(w);
        match which {
            WhichMatrix::Current => win.current_matrix.as_deref_mut(),
            WhichMatrix::Desired => win.desired_matrix.as_deref_mut(),
        }
    }

    pub fn with_window_matrix<R>(
        &mut self,
        w: Value,
        which: WhichMatrix,
        f: impl FnOnce(&mut Lisp, &mut GlyphMatrix) -> R,
    ) -> Option<R> {
        let slot = match which {
            WhichMatrix::Current => &mut self.window_mut(w).current_matrix,
            WhichMatrix::Desired => &mut self.window_mut(w).desired_matrix,
        };
        let mut matrix = slot.take()?;
        let r = f(self, &mut matrix);
        let slot = match which {
            WhichMatrix::Current => &mut self.window_mut(w).current_matrix,
            WhichMatrix::Desired => &mut self.window_mut(w).desired_matrix,
        };
        *slot = Some(matrix);
        Some(r)
    }

    pub fn shift_glyph_matrix(
        &mut self,
        w: Value,
        which: WhichMatrix,
        start: usize,
        end: usize,
        dy: i32,
    ) {
        let min_y = (self.window_tab_line_height(w) + self.window_header_line_height(w)) as i32;
        let max_y = self.window_box_height_no_mode_line(w) as i32;
        if let Some(matrix) = self.window_matrix_mut(w, which) {
            for row in &mut matrix.rows[start..end] {
                row.y += dy;
                row.visible_height = row.height;
                if row.y < min_y {
                    row.visible_height -= min_y - row.y;
                }
                if row.y + row.height > max_y {
                    row.visible_height -= row.y + row.height - max_y;
                }
                if row.fringe_bitmap_periodic_p {
                    row.redraw_fringe_bitmaps_p = true;
                }
            }
        }
    }

    pub fn window_box_height_no_mode_line(&mut self, w: Value) -> i64 {
        self.window(w).pixel_height
            - self.window_bottom_divider_width(w)
            - self.window_scroll_bar_area_height(w)
            - self.window_mode_line_height(w)
    }

    pub fn blank_row(&mut self, w: Value, which: WhichMatrix, vpos: usize, y: i32) {
        let min_y = (self.window_tab_line_height(w) + self.window_header_line_height(w)) as i32;
        let max_y = self.window_box_height_no_mode_line(w) as i32;
        let line_height = self.frame(self.window(w).frame).geometry.line_height as i32;
        if let Some(matrix) = self.window_matrix_mut(w, which) {
            let row = &mut matrix.rows[vpos];
            clear_glyph_row(row);
            row.y = y;
            row.ascent = 0;
            row.phys_ascent = 0;
            row.height = line_height;
            row.phys_height = line_height;
            row.visible_height = row.height;
            if row.y < min_y {
                row.visible_height -= min_y - row.y;
            }
            if row.y + row.height > max_y {
                row.visible_height -= row.y + row.height - max_y;
            }
            row.enabled_p = true;
        }
    }

    fn increment_row_positions(&self, row: &mut GlyphRow, delta: i64, delta_bytes: i64) {
        row.minpos.charpos += delta;
        row.minpos.bytepos += delta_bytes;
        row.maxpos.charpos += delta;
        row.maxpos.bytepos += delta_bytes;
        row.start.pos.charpos += delta;
        row.start.pos.bytepos += delta_bytes;
        row.end.pos.charpos += delta;
        row.end.pos.bytepos += delta_bytes;
        if !row.enabled_p {
            return;
        }
        for area in LEFT_MARGIN_AREA..LAST_AREA {
            let start = row.area[area];
            for i in 0..row.used[area] as usize {
                let g = &mut row.glyphs[start + i];
                if self.is_buffer(g.object) && g.charpos > 0 {
                    g.charpos += delta;
                }
            }
        }
        if row.used[TEXT_AREA] == 0 && row.displays_text_p {
            let start = row.area[TEXT_AREA];
            row.glyphs[start].charpos += delta;
        }
    }

    pub fn increment_matrix_positions(
        &mut self,
        w: Value,
        which: WhichMatrix,
        start: usize,
        end: usize,
        delta: i64,
        delta_bytes: i64,
    ) {
        self.with_window_matrix(w, which, |lisp, matrix| {
            for row in &mut matrix.rows[start..end] {
                lisp.increment_row_positions(row, delta, delta_bytes);
            }
        });
    }

    pub fn prepare_desired_row(&mut self, w: Value, vpos: usize, mode_line_p: bool) {
        let (left_cols, right_cols) = {
            let win = self.window(w);
            (win.left_margin_cols, win.right_margin_cols)
        };
        let window_p = self.frame_window_p(self.window(w).frame);
        let rightmost = self.window_rightmost_p(w);
        let Some(matrix) = self.window_matrix_mut(w, WhichMatrix::Desired) else {
            return;
        };
        let (left, right) = (matrix.left_margin_glyphs, matrix.right_margin_glyphs);
        let row = &mut matrix.rows[vpos];
        if !row.enabled_p {
            let rp = row.reversed_p;
            clear_glyph_row(row);
            row.enabled_p = true;
            row.reversed_p = rp;
        }
        if mode_line_p {
            if left_cols > 0 {
                row.area[TEXT_AREA] = row.area[LEFT_MARGIN_AREA];
            }
            if right_cols > 0 {
                row.area[RIGHT_MARGIN_AREA] = row.area[LAST_AREA];
            }
        } else {
            if left_cols > 0 && left as usize != row.area[TEXT_AREA] - row.area[LEFT_MARGIN_AREA] {
                row.area[TEXT_AREA] = row.area[LEFT_MARGIN_AREA] + left as usize;
            }
            if right_cols > 0 && right as usize != row.area[LAST_AREA] - row.area[RIGHT_MARGIN_AREA]
            {
                row.area[RIGHT_MARGIN_AREA] = row.area[LAST_AREA] - right as usize;
                if !window_p && !rightmost && right > 0 {
                    row.area[RIGHT_MARGIN_AREA] -= 1;
                }
            }
        }
    }

    pub fn make_current(&mut self, w: Value, row: usize) {
        let win = self.window_mut(w);
        let (Some(current), Some(desired)) = (
            win.current_matrix.as_deref_mut(),
            win.desired_matrix.as_deref_mut(),
        ) else {
            return;
        };
        let current_row = &mut current.rows[row];
        let desired_row = &mut desired.rows[row];
        let mouse_face_p = current_row.mouse_face_p;
        if desired_row.hash == 0
            && !(desired_row.used[0] == 0 && desired_row.used[1] == 0 && desired_row.used[2] == 0)
        {
            desired_row.hash = row_hash(desired_row);
        }
        assign_row(current_row, desired_row);
        current_row.enabled_p = true;
        current_row.mouse_face_p = mouse_face_p;
    }

    fn required_matrix_height(&mut self, w: Value) -> i32 {
        let f = self.window(w).frame;
        if self.frame_window_p(f) {
            let ch_height = self
                .display_info(self.frame(f).terminal)
                .smallest_font_height
                .max(1) as i64;
            let window_pixel_height = self.window_box_height(w) + self.window(w).vscroll.abs();
            return (((window_pixel_height + ch_height - 1) / ch_height)
                * self.window(w).nrows_scale_factor
                + 2
                + 3) as i32;
        }
        self.window(w).total_lines as i32
    }

    fn required_matrix_width(&self, w: Value) -> i32 {
        let f = self.window(w).frame;
        if self.frame_window_p(f) {
            let ch_width = self
                .display_info(self.frame(f).terminal)
                .smallest_char_width
                .max(1) as i64;
            let win = self.window(w);
            return (((win.pixel_width + ch_width - 1) / ch_width) * win.ncols_scale_factor
                + 2
                + 1
                + 1) as i32;
        }
        self.window(w).total_cols as i32
    }

    pub fn desired_matrix_nrows(&mut self, w: Value) -> i64 {
        match self.window_matrix(w, WhichMatrix::Desired) {
            Some(matrix) => i64::from(matrix.nrows),
            None => i64::from(self.required_matrix_height(w)),
        }
    }

    pub fn adjust_glyph_matrix(&mut self, w: Value, which: WhichMatrix, x: i32, y: i32, dim: Dim) {
        let window_width = self.window_box_width(w, GlyphRowArea::Any) as i32;
        let window_height = self.window_box_height(w) as i32;
        let tab_line_p = self.window_wants_tab_line(w);
        let header_line_p = self.window_wants_header_line(w);
        let wants_mode_line = self.window_wants_mode_line(w);
        let win = self.window(w);
        let (pixel_left, pixel_top, vscroll) = (
            win.pixel_left as i32,
            win.pixel_top as i32,
            win.vscroll as i32,
        );
        let left = margin_glyphs_to_reserve(win.total_cols, dim.width, win.left_margin_cols);
        let right = margin_glyphs_to_reserve(win.total_cols, dim.width, win.right_margin_cols);
        let window_end_vpos = win.window_end_vpos;
        let fonts_changed = self.frame(win.frame).fonts_changed;
        let current = which == WhichMatrix::Current;
        let mut invalidate_window_end = false;
        let changed = self.with_window_matrix(w, which, |_, matrix| {
            let tab_line_changed_p = tab_line_p != matrix.tab_line_p;
            let header_line_changed_p = header_line_p != matrix.header_line_p;
            matrix.tab_line_p = tab_line_p;
            matrix.header_line_p = header_line_p;
            let marginal_areas_changed_p =
                left != matrix.left_margin_glyphs || right != matrix.right_margin_glyphs;
            if !marginal_areas_changed_p
                && !fonts_changed
                && !tab_line_changed_p
                && !header_line_changed_p
                && matrix.window_pixel_left == pixel_left
                && matrix.window_pixel_top == pixel_top
                && matrix.window_height == window_height
                && matrix.window_vscroll == vscroll
                && matrix.window_width == window_width
            {
                return false;
            }
            let new_rows = if matrix.rows.len() < dim.height as usize {
                let old_alloc = matrix.rows.len();
                let new_rows = dim.height as usize - old_alloc;
                matrix
                    .rows
                    .resize_with(xpalloc_len(old_alloc, new_rows), GlyphRow::default);
                new_rows
            } else {
                0
            };
            let width = dim.width.max(0) as usize;
            if dim.width > matrix.matrix_w
                || new_rows != 0
                || tab_line_changed_p
                || header_line_changed_p
                || marginal_areas_changed_p
            {
                let realloc = dim.width > matrix.matrix_w || new_rows != 0;
                let last = dim.height as usize;
                for (i, row) in matrix.rows.iter_mut().enumerate() {
                    if realloc {
                        row.glyphs = vec![Glyph::default(); width];
                    }
                    let special = (i + 1 == last && !wants_mode_line)
                        || (i == 0 && matrix.tab_line_p)
                        || (i == 0 && !matrix.tab_line_p && matrix.header_line_p)
                        || (i == 1 && matrix.tab_line_p && matrix.header_line_p);
                    row.area = if special {
                        [0, 0, width, width]
                    } else {
                        [
                            0,
                            left as usize,
                            width.saturating_sub(right as usize),
                            width,
                        ]
                    };
                }
            }
            matrix.left_margin_glyphs = left;
            matrix.right_margin_glyphs = right;
            if current
                && matrix.nrows > 0
                && dim.height != matrix.nrows
                && matrix.nrows as usize <= matrix.rows.len()
            {
                let mode_line = matrix.mode_line_row();
                matrix.rows[mode_line].mode_line_p = false;
            }
            matrix.nrows = dim.height;
            let nrows = matrix.nrows.max(0) as usize;
            if current {
                if !marginal_areas_changed_p
                    && !tab_line_changed_p
                    && !header_line_changed_p
                    && new_rows == 0
                    && dim.width == matrix.matrix_w
                    && matrix.window_pixel_left == pixel_left
                    && matrix.window_pixel_top == pixel_top
                    && matrix.window_width == window_width
                {
                    let mut i = 0;
                    while i < nrows && matrix.rows[i].enabled_p {
                        if matrix.rows[i].bottom_y() >= window_height {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                    if window_end_vpos >= i as i64 {
                        invalidate_window_end = true;
                    }
                    clear_glyph_matrix_rows(matrix, i, nrows);
                } else {
                    clear_glyph_matrix_rows(matrix, 0, nrows);
                }
            } else {
                clear_glyph_matrix_rows(matrix, 0, nrows);
            }
            matrix.matrix_x = x;
            matrix.matrix_y = y;
            matrix.matrix_w = dim.width;
            matrix.matrix_h = dim.height;
            matrix.window_pixel_left = pixel_left;
            matrix.window_pixel_top = pixel_top;
            matrix.window_height = window_height;
            matrix.window_width = window_width;
            matrix.window_vscroll = vscroll;
            true
        });
        if changed == Some(true) && current {
            if invalidate_window_end {
                self.window_mut(w).flags.window_end_valid = false;
            }
            if wants_mode_line {
                self.window_mut(w).flags.update_mode_line = true;
            }
        }
    }

    pub fn allocate_matrices_for_window_redisplay(&mut self, w: Value) {
        let mut w = w;
        while !w.is_nil() {
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                self.allocate_matrices_for_window_redisplay(contents);
            } else {
                let win = self.window_mut(w);
                if win.desired_matrix.is_none() {
                    win.desired_matrix = Some(Box::default());
                }
                if win.current_matrix.is_none() {
                    win.current_matrix = Some(Box::default());
                }
                let dim = Dim {
                    width: self.required_matrix_width(w),
                    height: self.required_matrix_height(w),
                };
                self.adjust_glyph_matrix(w, WhichMatrix::Desired, 0, 0, dim);
                self.adjust_glyph_matrix(w, WhichMatrix::Current, 0, 0, dim);
            }
            w = self.window(w).next;
        }
    }

    fn adjust_frame_glyphs_for_window_redisplay(&mut self, f: Value) {
        let root = self.frame(f).root_window;
        self.allocate_matrices_for_window_redisplay(root);
        let mut w = self.frame(f).tab_bar_window;
        if w.is_nil() {
            w = self.make_window();
            self.frame_mut(f).tab_bar_window = w;
            let win = self.window_mut(w);
            win.frame = f;
            win.flags.pseudo_window_p = true;
        }
        let g = self.frame(f).geometry.clone();
        let tab_bar_position = !self
            .find_symbol_value(crate::symbols::TAB_BAR_POSITION)
            .is_nil();
        let tool_bar_top_height = self.frame_tool_bar_top_height(f);
        let tool_bar_top_lines = self.frame_tool_bar_top_lines(f);
        let win = self.window_mut(w);
        win.pixel_left = 0;
        win.left_col = 0;
        win.pixel_top = g.menu_bar_height
            + if tab_bar_position {
                tool_bar_top_height
            } else {
                0
            };
        win.top_line = g.menu_bar_lines
            + if tab_bar_position {
                tool_bar_top_lines
            } else {
                0
            };
        win.total_cols = g.total_cols;
        win.pixel_width = g.pixel_width - 2 * g.internal_border_width;
        win.total_lines = g.tab_bar_lines;
        win.pixel_height = g.tab_bar_height;
        self.allocate_matrices_for_window_redisplay(w);
    }

    pub fn adjust_frame_glyphs(&mut self, f: Value) {
        if !self.frame_window_p(f) {
            return;
        }
        self.adjust_frame_glyphs_for_window_redisplay(f);
        self.frame_mut(f).flags.glyphs_initialized_p = true;
    }

    pub fn clear_window_matrices(&mut self, w: Value, desired_p: bool) {
        let mut w = w;
        while !w.is_nil() {
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                self.clear_window_matrices(contents, desired_p);
            } else if desired_p {
                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                    clear_glyph_matrix(m);
                }
            } else {
                if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Current) {
                    clear_glyph_matrix(m);
                }
                self.window_mut(w).flags.window_end_valid = false;
            }
            w = self.window(w).next;
        }
    }

    fn clear_bar_window_matrix(&mut self, f: Value, which: WhichMatrix) {
        let w = self.frame(f).tab_bar_window;
        if self.is_window(w)
            && let Some(m) = self.window_matrix_mut(w, which)
        {
            clear_glyph_matrix(m);
        }
    }

    pub fn clear_current_matrices(&mut self, f: Value) {
        self.clear_bar_window_matrix(f, WhichMatrix::Current);
        let root = self.frame(f).root_window;
        self.clear_window_matrices(root, false);
    }

    pub fn clear_desired_matrices(&mut self, f: Value) {
        self.clear_bar_window_matrix(f, WhichMatrix::Desired);
        let root = self.frame(f).root_window;
        self.clear_window_matrices(root, true);
    }

    pub fn free_window_matrices(&mut self, w: Value) {
        let mut w = w;
        while !w.is_nil() {
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                self.free_window_matrices(contents);
            } else {
                let win = self.window_mut(w);
                win.current_matrix = None;
                win.desired_matrix = None;
            }
            w = self.window(w).next;
        }
    }

    pub fn free_glyphs(&mut self, f: Value) {
        if !self.frame(f).flags.glyphs_initialized_p {
            return;
        }
        self.frame_mut(f).flags.glyphs_initialized_p = false;
        let root = self.frame(f).root_window;
        if !root.is_nil() {
            self.free_window_matrices(root);
        }
        let w = self.frame(f).tab_bar_window;
        if !w.is_nil() {
            let win = self.window_mut(w);
            win.desired_matrix = None;
            win.current_matrix = None;
            self.frame_mut(f).tab_bar_window = Value::NIL;
        }
    }
}
