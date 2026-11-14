use crate::dispextern::{
    DrawGlyphsFace, EmacsCursorKind, Glyph, GlyphMatrix, GlyphRow, GlyphRowArea, GlyphType,
    LAST_AREA, LEFT_MARGIN_AREA, RIGHT_MARGIN_AREA, TEXT_AREA, WindowPart,
};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::textprop::{
    f_get_text_property, f_next_single_property_change, f_previous_single_property_change,
};
use crate::value::Value;
use crate::xdisp::scroll::ends_in_middle_of_char_p;

#[derive(Clone, Copy, Debug)]
pub struct XyHit {
    pub glyph: Option<Glyph>,
    pub hpos: usize,
    pub vpos: usize,
    pub area: usize,
    pub dx: i64,
    pub dy: i64,
}

fn past_row_end(row: &GlyphRow, charpos: i64) -> bool {
    charpos > row.end_charpos()
        || (charpos == row.end_charpos() && !row.ends_at_zv_p && !ends_in_middle_of_char_p(row))
}

fn row_enabled(m: &GlyphMatrix, i: usize) -> bool {
    m.rows.get(i).is_some_and(|r| r.enabled_p)
}

impl Lisp {
    fn hl_matrix(&self, w: Value) -> &GlyphMatrix {
        self.window_matrix(w, WhichMatrix::Current)
            .expect("glyph matrix")
    }

    fn row_glyphs(&self, w: Value, vpos: usize) -> Vec<Glyph> {
        self.hl_matrix(w).rows[vpos].area_glyphs(TEXT_AREA).to_vec()
    }

    pub fn x_y_to_hpos_vpos(&self, w: Value, x: i64, y: i64) -> XyHit {
        let mut hit = XyHit {
            glyph: None,
            hpos: 0,
            vpos: 0,
            area: LAST_AREA,
            dx: 0,
            dy: 0,
        };
        let m = self.hl_matrix(w);
        let nrows = m.nrows.max(0) as usize;
        let mut i = 0;
        while i < nrows {
            let row = &m.rows[i];
            if !row.enabled_p {
                return hit;
            }
            if y >= i64::from(row.y) && y < i64::from(row.bottom_y()) {
                break;
            }
            i += 1;
        }
        hit.vpos = i;
        if i == nrows {
            return hit;
        }
        let row = &m.rows[i];
        let x0 = if self.window(w).flags.pseudo_window_p {
            hit.area = TEXT_AREA;
            0
        } else if x < self.window_box_left_offset(w, GlyphRowArea::Text) {
            hit.area = LEFT_MARGIN_AREA;
            self.window_box_left_offset(w, GlyphRowArea::LeftMargin)
        } else if x < self.window_box_right_offset(w, GlyphRowArea::Text) {
            hit.area = TEXT_AREA;
            self.window_box_left_offset(w, GlyphRowArea::Text) + i64::from(row.x.min(0))
        } else {
            hit.area = RIGHT_MARGIN_AREA;
            self.window_box_left_offset(w, GlyphRowArea::RightMargin)
        };
        let glyphs = row.area_glyphs(hit.area);
        let mut x = x - x0;
        let mut g = 0;
        while g < glyphs.len() && x >= i64::from(glyphs[g].pixel_width) {
            x -= i64::from(glyphs[g].pixel_width);
            g += 1;
        }
        if g == glyphs.len() {
            return hit;
        }
        let glyph = glyphs[g];
        hit.dx = x;
        hit.dy = y - i64::from(row.y + row.ascent - i32::from(glyph.ascent));
        hit.hpos = g;
        hit.glyph = Some(glyph);
        hit
    }

    fn rows_from_pos_range(
        &mut self,
        w: Value,
        start_charpos: i64,
        end_charpos: i64,
        disp_string: Value,
    ) -> (Option<usize>, Option<usize>) {
        let last_y = self.window_text_bottom_y(w);
        let bottom = self.matrix_bottom_text_row(w, WhichMatrix::Current);
        let m = self.hl_matrix(w);
        let mut first = m.first_text_row();
        while !row_enabled(m, first) && first < bottom {
            first += 1;
        }
        let in_range = |lisp: &Lisp, g: &Glyph| {
            ((lisp.is_buffer(g.object) || g.object.is_nil())
                && start_charpos <= g.charpos
                && g.charpos < end_charpos)
                || g.object == disp_string
        };
        let mut start = None;
        let mut end = None;
        let mut row = first;
        while row_enabled(m, row) && i64::from(m.rows[row].bottom_y()) <= last_y {
            let r = &m.rows[row];
            if !((start_charpos < r.start_charpos() && end_charpos < r.start_charpos())
                || (past_row_end(r, start_charpos) && past_row_end(r, end_charpos)))
            {
                for g in r.area_glyphs(TEXT_AREA) {
                    if in_range(self, g) {
                        start = Some(row);
                    }
                }
                if start.is_some() {
                    break;
                }
            }
            row += 1;
        }
        if start.is_none()
            && !(row_enabled(m, row)
                && i64::from(m.rows[row].y) < last_y
                && i64::from(m.rows[row].bottom_y()) > last_y)
        {
            row = first;
        }
        while row_enabled(m, row) && i64::from(m.rows[row].bottom_y()) <= last_y {
            let next = row + 1;
            let Some(nr) = m.rows.get(next).filter(|r| r.enabled_p) else {
                end = Some(row);
                break;
            };
            let next_start = nr.start_charpos();
            if next >= bottom
                || (start_charpos < next_start && end_charpos < next_start)
                || (past_row_end(nr, start_charpos) && past_row_end(nr, end_charpos))
            {
                end = Some(row);
                break;
            }
            let glyphs = nr.area_glyphs(TEXT_AREA);
            let n = glyphs.len();
            let row_ends_at_zv = m.rows[row].ends_at_zv_p;
            let found = glyphs.iter().enumerate().any(|(i, g)| {
                ((self.is_buffer(g.object) || g.object.is_nil())
                    && ((start_charpos <= g.charpos && g.charpos < end_charpos)
                        || (((!nr.reversed_p && i == 0) || (nr.reversed_p && i + 1 == n))
                            && (g.charpos == end_charpos
                                || (g.charpos == -1
                                    && !row_ends_at_zv
                                    && next_start == end_charpos)))))
                    || g.object == disp_string
            });
            if !found {
                end = Some(row);
                break;
            } else if nr.ends_at_zv_p {
                end = Some(next);
                break;
            }
            row += 1;
        }
        (start, end)
    }

    fn string_outside_range(
        &mut self,
        object: Value,
        before_string: Value,
        after_string: Value,
        start_charpos: i64,
        end_charpos: i64,
    ) -> LispResult<bool> {
        if object == before_string {
            let pos = self.string_buffer_position(before_string, start_charpos)?;
            return Ok(pos == 0 || (pos >= start_charpos && pos < end_charpos));
        }
        if object == after_string {
            let pos = self.string_buffer_position(after_string, end_charpos)?;
            return Ok(pos == 0 || (pos >= start_charpos && pos < end_charpos));
        }
        Ok(false)
    }

    #[allow(clippy::too_many_arguments)]
    fn mouse_face_from_buffer_pos(
        &mut self,
        window: Value,
        mouse_charpos: i64,
        start_charpos: i64,
        end_charpos: i64,
        before_string: Value,
        after_string: Value,
        disp_string: Value,
    ) -> LispResult<()> {
        let w = window;
        let f = self.window(w).frame;
        let first = self.hl_matrix(w).first_text_row();
        let (r1, r2) = self.rows_from_pos_range(w, start_charpos, end_charpos, disp_string);
        let window_end_vpos = self.window(w).window_end_vpos.max(0) as usize;
        let mut r1 = r1.unwrap_or(window_end_vpos);
        if !before_string.is_nil() || !disp_string.is_nil() {
            while r1 > first {
                let prev = r1 - 1;
                let (end_cp, glyphs) = {
                    let r = &self.hl_matrix(w).rows[prev];
                    (r.end_charpos(), r.area_glyphs(TEXT_AREA).to_vec())
                };
                if !(end_cp == start_charpos && !glyphs.is_empty()) {
                    break;
                }
                match glyphs.iter().rev().find(|g| !g.object.is_nil()) {
                    Some(g) if g.object == before_string || g.object == disp_string => r1 = prev,
                    _ => break,
                }
            }
        }
        let mut r2 = match r2 {
            None => {
                self.mouse_hl_info_mut(f).mouse_face_past_end = true;
                window_end_vpos
            }
            Some(mut r2) => {
                if !after_string.is_nil() {
                    let m = self.hl_matrix(w);
                    let mut next = r2 + 1;
                    while next <= window_end_vpos
                        && m.rows[next].used[TEXT_AREA] > 0
                        && m.rows[next].area_glyphs(TEXT_AREA)[0].object == after_string
                    {
                        r2 = next;
                        next += 1;
                    }
                }
                r2
            }
        };
        {
            let m = self.hl_matrix(w);
            if m.rows[r1].y > m.rows[r2].y {
                std::mem::swap(&mut r1, &mut r2);
            }
        }
        {
            let h = self.mouse_hl_info_mut(f);
            h.mouse_face_beg_row = r1 as i32;
            h.mouse_face_end_row = r2 as i32;
        }
        let in_range = |lisp: &Lisp, g: &Glyph| {
            lisp.is_buffer(g.object) && g.charpos >= start_charpos && g.charpos < end_charpos
        };
        let (r1_reversed, r1_x, r1_displays_text) = {
            let r = &self.hl_matrix(w).rows[r1];
            (r.reversed_p, i64::from(r.x), r.displays_text_p)
        };
        let glyphs1 = self.row_glyphs(w, r1);
        let mut glyph: i64;
        let mut end: i64;
        let mut x: i64;
        if !r1_reversed {
            glyph = 0;
            end = glyphs1.len() as i64;
            x = r1_x;
            if r1_displays_text {
                while glyph < end
                    && glyphs1[glyph as usize].object.is_nil()
                    && glyphs1[glyph as usize].charpos < 0
                {
                    x += i64::from(glyphs1[glyph as usize].pixel_width);
                    glyph += 1;
                }
            }
            while glyph < end {
                let g = glyphs1[glyph as usize];
                if g.object.is_nil() || g.object == disp_string || in_range(self, &g) {
                    break;
                }
                if self.string_outside_range(
                    g.object,
                    before_string,
                    after_string,
                    start_charpos,
                    end_charpos,
                )? {
                    break;
                }
                x += i64::from(g.pixel_width);
                glyph += 1;
            }
            let h = self.mouse_hl_info_mut(f);
            h.mouse_face_beg_x = x as i32;
            h.mouse_face_beg_col = glyph as i32;
        } else {
            end = -1;
            glyph = glyphs1.len() as i64 - 1;
            if r1_displays_text {
                while glyph > end
                    && glyphs1[glyph as usize].object.is_nil()
                    && glyphs1[glyph as usize].charpos < 0
                {
                    glyph -= 1;
                }
            }
            while glyph > end {
                let g = glyphs1[glyph as usize];
                if g.object.is_nil() || g.object == disp_string || in_range(self, &g) {
                    break;
                }
                if self.string_outside_range(
                    g.object,
                    before_string,
                    after_string,
                    start_charpos,
                    end_charpos,
                )? {
                    break;
                }
                glyph -= 1;
            }
            glyph += 1;
            x = r1_x
                + glyphs1[..glyph as usize]
                    .iter()
                    .map(|g| i64::from(g.pixel_width))
                    .sum::<i64>();
            let h = self.mouse_hl_info_mut(f);
            h.mouse_face_beg_x = x as i32;
            h.mouse_face_beg_col = glyph as i32;
        }
        let (r2_reversed, r2_x) = {
            let r = &self.hl_matrix(w).rows[r2];
            (r.reversed_p, i64::from(r.x))
        };
        let glyphs2 = if r2 != r1 {
            let glyphs2 = self.row_glyphs(w, r2);
            if !r2_reversed {
                glyph = 0;
                end = glyphs2.len() as i64;
                x = r2_x;
            } else {
                end = -1;
                glyph = glyphs2.len() as i64 - 1;
            }
            glyphs2
        } else {
            glyphs1
        };
        if !r2_reversed {
            while end > glyph && glyphs2[(end - 1) as usize].object.is_nil() {
                end -= 1;
            }
            end -= 1;
            while end > glyph {
                let g = glyphs2[end as usize];
                if g.object.is_nil() || g.object == disp_string || in_range(self, &g) {
                    break;
                }
                if self.string_outside_range(
                    g.object,
                    before_string,
                    after_string,
                    start_charpos,
                    end_charpos,
                )? {
                    break;
                }
                end -= 1;
            }
            while glyph <= end {
                x += i64::from(glyphs2[glyph as usize].pixel_width);
                glyph += 1;
            }
            let h = self.mouse_hl_info_mut(f);
            h.mouse_face_end_x = x as i32;
            h.mouse_face_end_col = glyph as i32;
        } else {
            x = r2_x;
            end += 1;
            while end < glyph && glyphs2[end as usize].object.is_nil() {
                x += i64::from(glyphs2[end as usize].pixel_width);
                end += 1;
            }
            while end < glyph {
                let g = glyphs2[end as usize];
                if g.object.is_nil() || g.object == disp_string || in_range(self, &g) {
                    break;
                }
                if self.string_outside_range(
                    g.object,
                    before_string,
                    after_string,
                    start_charpos,
                    end_charpos,
                )? {
                    break;
                }
                x += i64::from(g.pixel_width);
                end += 1;
            }
            if end == glyph
                && let Some(g) = glyphs2.get(end as usize)
                && self.is_buffer(g.object)
                && (g.charpos < start_charpos || g.charpos >= end_charpos)
            {
                x += i64::from(g.pixel_width);
                end += 1;
            }
            let h = self.mouse_hl_info_mut(f);
            h.mouse_face_end_x = x as i32;
            h.mouse_face_end_col = end as i32;
        }
        self.mouse_hl_info_mut(f).mouse_face_window = window;
        let hidden = self.mouse_hl_info(f).mouse_face_hidden;
        let pos = mouse_charpos.max(0) as usize;
        let (face_id, _) = self.face_at_buffer_position(w, pos, pos + 1, !hidden, None, 0)?;
        self.mouse_hl_info_mut(f).mouse_face_face_id = face_id;
        self.show_mouse_face(f, DrawGlyphsFace::MouseFace, true)
    }

    fn mouse_face_from_string_pos(&mut self, w: Value, object: Value, startpos: i64, endpos: i64) {
        let f = self.window(w).frame;
        let yb = self.window_text_bottom_y(w);
        let m = self.hl_matrix(w);
        let matches = |g: &Glyph| g.object == object && startpos <= g.charpos && g.charpos < endpos;
        let mut r = m.first_text_row();
        let mut beg = None;
        while row_enabled(m, r) && i64::from(m.rows[r].y) < yb {
            let row = &m.rows[r];
            let glyphs = row.area_glyphs(TEXT_AREA);
            if !row.reversed_p {
                let mut gx = i64::from(row.x);
                for (i, g) in glyphs.iter().enumerate() {
                    if matches(g) {
                        beg = Some((r, i, gx));
                        break;
                    }
                    gx += i64::from(g.pixel_width);
                }
            } else {
                for g in (1..=glyphs.len()).rev() {
                    if matches(&glyphs[g - 1]) {
                        let gx = i64::from(row.x)
                            + glyphs[..g]
                                .iter()
                                .map(|g| i64::from(g.pixel_width))
                                .sum::<i64>();
                        beg = Some((r, g, gx));
                        break;
                    }
                }
            }
            if beg.is_some() {
                break;
            }
            r += 1;
        }
        let Some((beg_row, beg_col, beg_x)) = beg else {
            return;
        };
        r = beg_row + 1;
        while row_enabled(m, r) && i64::from(m.rows[r].y) < yb {
            if !m.rows[r].area_glyphs(TEXT_AREA).iter().any(matches) {
                break;
            }
            r += 1;
        }
        r -= 1;
        let row = &m.rows[r];
        let glyphs = row.area_glyphs(TEXT_AREA);
        let (end_col, end_x) = if !row.reversed_p {
            let mut e = glyphs.len();
            while e > 0 && !matches(&glyphs[e - 1]) {
                e -= 1;
            }
            let gx = i64::from(row.x)
                + glyphs[..e]
                    .iter()
                    .map(|g| i64::from(g.pixel_width))
                    .sum::<i64>();
            (e, gx)
        } else {
            let mut gx = i64::from(row.x);
            let mut e = 0;
            while e < glyphs.len() {
                if matches(&glyphs[e]) {
                    break;
                }
                gx += i64::from(glyphs[e].pixel_width);
                e += 1;
            }
            (e, gx)
        };
        let h = self.mouse_hl_info_mut(f);
        h.mouse_face_beg_row = beg_row as i32;
        h.mouse_face_beg_col = beg_col as i32;
        h.mouse_face_beg_x = beg_x as i32;
        h.mouse_face_end_row = r as i32;
        h.mouse_face_end_col = end_col as i32;
        h.mouse_face_end_x = end_x as i32;
    }

    fn define_frame_cursor1(&mut self, f: Value, cursor: Option<EmacsCursorKind>, pointer: Value) {
        if !self.frame_window_p(f) {
            return;
        }
        let track_mouse = self.plain_value(sym::TRACK_MOUSE);
        if track_mouse == sym::DRAGGING
            || track_mouse == sym::DROPPING
            || track_mouse == sym::DRAG_SOURCE
        {
            return;
        }
        let mut cursor = cursor;
        if !pointer.is_nil() {
            cursor = Some(if pointer == sym::ARROW {
                EmacsCursorKind::Nontext
            } else if pointer == sym::HAND {
                EmacsCursorKind::Hand
            } else if pointer == sym::TEXT {
                EmacsCursorKind::Text
            } else if pointer == sym::HDRAG {
                EmacsCursorKind::HorizontalDrag
            } else if pointer == sym::NHDRAG {
                EmacsCursorKind::VerticalDrag
            } else if pointer == sym::HOURGLASS {
                EmacsCursorKind::Hourglass
            } else if pointer == sym::MODELINE {
                EmacsCursorKind::Modeline
            } else {
                EmacsCursorKind::Nontext
            });
        }
        if let Some(cursor) = cursor
            && let Some(define) = self.frame_rif(f).define_frame_cursor
        {
            define(self, f, cursor);
        }
    }

    fn window_topmost_p(&self, w: Value) -> bool {
        self.window(w).pixel_top == 0
    }

    fn set_help_echo(&mut self, help: Value, window: Value, object: Value, pos: i64) {
        let x = &mut self.xdisp;
        x.help_echo_string = help;
        x.help_echo_window = window;
        x.help_echo_object = object;
        x.help_echo_pos = pos;
    }

    fn note_line_or_margin_highlight(
        &mut self,
        window: Value,
        x: i64,
        y: i64,
        area: WindowPart,
    ) -> LispResult<()> {
        let w = window;
        let f = self.window(w).frame;
        let mut cursor = None;
        let mut pointer = Value::NIL;
        let original_x_pixel = x;
        let on_line = matches!(
            area,
            WindowPart::ModeLine | WindowPart::HeaderLine | WindowPart::TabLine
        );
        let mut glyph: Option<usize> = None;
        let mut row = 0;
        let hit = if on_line {
            let hit = self.mode_line_string(w, area, x, y);
            let m = self.hl_matrix(w);
            row = match area {
                WindowPart::ModeLine => m.mode_line_row(),
                WindowPart::TabLine => 0,
                _ => m.header_line_row(),
            };
            let r = &m.rows[row];
            if r.mode_line_p && r.enabled_p {
                let glyphs = r.area_glyphs(TEXT_AREA);
                let mut x0 = original_x_pixel;
                let mut g = 0;
                while g < glyphs.len() && x0 >= i64::from(glyphs[g].pixel_width) {
                    x0 -= i64::from(glyphs[g].pixel_width);
                    g += 1;
                }
                if g < glyphs.len() {
                    glyph = Some(g);
                }
            }
            hit
        } else {
            let x = x - self.window_left_scroll_bar_area_width(w);
            self.marginal_area_string(w, area, x, y)
        };
        let (string, charpos, x, dx) = (hit.string, hit.charpos, hit.x, hit.dx);
        let mut pos = Value::NIL;
        if string.is_string() {
            pos = Value::fixnum(charpos.min(self.string_chars(string) as i64 - 1));
        }
        if string.is_string() || on_line {
            let help = if string.is_string() {
                f_get_text_property(self, &[pos, sym::HELP_ECHO, string])?
            } else {
                Value::NIL
            };
            if !help.is_nil() {
                self.set_help_echo(help, w, string, charpos);
            } else if area == WindowPart::ModeLine && !self.window(w).mode_line_help_echo.is_nil() {
                let help = self.window(w).mode_line_help_echo;
                self.set_help_echo(help, w, Value::NIL, -1);
            }
            if self.frame_window_p(f) {
                let draggable_window_bottom_line = area == WindowPart::ModeLine
                    && (!self.window_bottommost_p(w)
                        || self.minibuf.minibuf_level != 0
                        || self.plain_value(sym::RESIZE_MINI_WINDOWS).is_nil());
                let draggable_window_top_line =
                    matches!(area, WindowPart::HeaderLine | WindowPart::TabLine)
                        && !self.window_topmost_p(w);
                if string.is_string() {
                    cursor = Some(EmacsCursorKind::Nontext);
                    if pointer.is_nil() {
                        pointer = f_get_text_property(self, &[pos, sym::POINTER, string])?;
                    }
                    if pointer.is_nil() && on_line {
                        let mut map = f_get_text_property(self, &[pos, sym::LOCAL_MAP, string])?;
                        if !self.keymapp(map) {
                            map = f_get_text_property(self, &[pos, sym::KEYMAP, string])?;
                        }
                        if !self.keymapp(map)
                            && (draggable_window_top_line || draggable_window_bottom_line)
                        {
                            cursor = Some(EmacsCursorKind::VerticalDrag);
                        }
                    }
                } else if draggable_window_top_line || draggable_window_bottom_line {
                    cursor = Some(EmacsCursorKind::VerticalDrag);
                } else if (area == WindowPart::ModeLine
                    && self.window_bottommost_p(w)
                    && !self.frame_has_minibuf_p(f)
                    && !self.get_frame_param(f, sym::DRAG_WITH_MODE_LINE).is_nil())
                    || (((area == WindowPart::HeaderLine
                        && !self.get_frame_param(f, sym::DRAG_WITH_HEADER_LINE).is_nil())
                        || (area == WindowPart::TabLine
                            && !self.get_frame_param(f, sym::DRAG_WITH_TAB_LINE).is_nil()))
                        && self.window_topmost_p(w))
                {
                    cursor = Some(EmacsCursorKind::Hand);
                } else {
                    cursor = Some(EmacsCursorKind::Nontext);
                }
            }
        }
        let mut mouse_face_shown = false;
        if string.is_string() {
            let mouse_face = f_get_text_property(self, &[pos, sym::MOUSE_FACE, string])?;
            if !self.plain_value(sym::MOUSE_HIGHLIGHT).is_nil()
                && !mouse_face.is_nil()
                && on_line
                && let Some(gi) = glyph
            {
                let b = f_previous_single_property_change(
                    self,
                    &[
                        Value::fixnum(charpos + 1),
                        sym::MOUSE_FACE,
                        string,
                        Value::NIL,
                    ],
                )?;
                let begpos = b.as_fixnum().unwrap_or(0);
                let e = f_next_single_property_change(
                    self,
                    &[pos, sym::MOUSE_FACE, string, Value::NIL],
                )?;
                let endpos = e
                    .as_fixnum()
                    .unwrap_or_else(|| self.string_chars(string) as i64);
                let (glyphs, reversed_p, nrows, tab_line_p) = {
                    let m = self.hl_matrix(w);
                    (
                        m.rows[row].area_glyphs(TEXT_AREA).to_vec(),
                        m.rows[row].reversed_p,
                        m.nrows,
                        m.tab_line_p,
                    )
                };
                let g = glyphs[gi];
                let same =
                    |t: &Glyph| t.object == g.object && begpos <= t.charpos && t.charpos < endpos;
                let mut tmp = 0;
                while tmp < gi && !same(&glyphs[tmp]) {
                    tmp += 1;
                }
                let gpos = gi - tmp;
                let mut tmp = glyphs.len() - 1;
                while tmp > gi && !same(&glyphs[tmp]) {
                    tmp -= 1;
                }
                let gseq_length = gpos + (tmp - gi) + 1;
                let total_pixel_width: i64 = glyphs[gi - gpos..gi]
                    .iter()
                    .map(|g| i64::from(g.pixel_width))
                    .sum();
                let hpos = x - gpos as i64;
                let vpos = match area {
                    WindowPart::ModeLine => i64::from(nrows) - 1,
                    WindowPart::TabLine => 0,
                    _ => i64::from(tab_line_p),
                };
                let h = *self.mouse_hl_info(f);
                if window == h.mouse_face_window
                    && (if !reversed_p {
                        i64::from(h.mouse_face_beg_col) <= hpos
                            && hpos < i64::from(h.mouse_face_end_col)
                    } else {
                        i64::from(h.mouse_face_end_col) <= hpos
                            && hpos < i64::from(h.mouse_face_beg_col)
                    })
                    && i64::from(h.mouse_face_beg_row) == vpos
                {
                    return Ok(());
                }
                if self.clear_mouse_face(f)? {
                    cursor = None;
                }
                let beg_x = original_x_pixel - (total_pixel_width + dx);
                {
                    let h = self.mouse_hl_info_mut(f);
                    if !reversed_p {
                        h.mouse_face_beg_col = hpos as i32;
                        h.mouse_face_beg_x = beg_x as i32;
                        h.mouse_face_end_col = (hpos + gseq_length as i64) as i32;
                        h.mouse_face_end_x = 0;
                    } else {
                        h.mouse_face_end_col = hpos as i32;
                        h.mouse_face_end_x = beg_x as i32;
                        h.mouse_face_beg_col = (hpos + gseq_length as i64) as i32;
                        h.mouse_face_beg_x = 0;
                    }
                    h.mouse_face_beg_row = vpos as i32;
                    h.mouse_face_end_row = vpos as i32;
                    h.mouse_face_past_end = false;
                    h.mouse_face_window = window;
                }
                let (face_id, _) = self.face_at_string_position(
                    w,
                    string,
                    charpos,
                    0,
                    g.face_id as usize,
                    true,
                    0,
                )?;
                self.mouse_hl_info_mut(f).mouse_face_face_id = face_id;
                self.show_mouse_face(f, DrawGlyphsFace::MouseFace, true)?;
                mouse_face_shown = true;
                if pointer.is_nil() {
                    pointer = sym::HAND;
                }
            }
        }
        if on_line && !mouse_face_shown {
            self.clear_mouse_face(f)?;
        }
        self.define_frame_cursor1(f, cursor, pointer);
        Ok(())
    }

    fn note_fringe_highlight(
        &mut self,
        f: Value,
        window: Value,
        x: i64,
        y: i64,
        part: WindowPart,
    ) -> LispResult<()> {
        if !self.xdisp.help_echo_string.is_nil() || !self.frame(f).flags.glyphs_initialized_p {
            return Ok(());
        }
        let prop = if part == WindowPart::LeftFringe {
            sym::LEFT_FRINGE_HELP
        } else {
            sym::RIGHT_FRINGE_HELP
        };
        let w = window;
        let hit = self.x_y_to_hpos_vpos(w, x, y);
        if hit.glyph.is_none() {
            return Ok(());
        }
        let vpos = hit.vpos;
        if !self.window(w).flags.window_end_valid
            || self.window_outdated(w)
            || vpos >= self.hl_matrix(w).nrows.max(0) as usize
            || !self.hl_matrix(w).rows[vpos].displays_text_p
        {
            return Ok(());
        }
        for g in self.row_glyphs(w, vpos) {
            let pos = Value::fixnum(g.charpos);
            let help_echo = if g.object.is_string() || self.is_buffer(g.object) {
                self.get_char_property_and_overlay(pos, prop, g.object)?.0
            } else {
                Value::NIL
            };
            if help_echo.is_string() {
                self.xdisp.help_echo_string = help_echo;
                break;
            }
        }
        Ok(())
    }

    pub fn note_mouse_highlight(&mut self, f: Value, x: i64, y: i64) -> LispResult<()> {
        self.with_gc_deferred(|lisp| lisp.note_mouse_highlight_1(f, x, y))
    }

    fn note_mouse_highlight_1(&mut self, f: Value, x: i64, y: i64) -> LispResult<()> {
        if !self.frame(f).flags.glyphs_initialized_p || self.frame(f).flags.pointer_invisible {
            return Ok(());
        }
        {
            let h = self.mouse_hl_info_mut(f);
            h.mouse_face_mouse_x = x as i32;
            h.mouse_face_mouse_y = y as i32;
            h.mouse_face_mouse_frame = f;
            if h.mouse_face_defer {
                return Ok(());
            }
        }
        let mut part = WindowPart::Nothing;
        let window = self.window_from_coordinates(f, x, y, &mut part, true);
        let hl_window = self.mouse_hl_info(f).mouse_face_window;
        if window != hl_window
            || (!hl_window.is_nil()
                && !window.is_nil()
                && !matches!(
                    part,
                    WindowPart::Text
                        | WindowPart::ModeLine
                        | WindowPart::HeaderLine
                        | WindowPart::TabLine
                ))
        {
            self.clear_mouse_face(f)?;
        }
        self.xdisp.help_echo_string = Value::NIL;
        if !self.frame_window_p(f) {
            let g = &self.frame(f).geometry;
            if y >= g.menu_bar_lines && y < g.menu_bar_lines + g.tab_bar_lines {
                unreachable!("tty tab bars need tty_get_tab_bar_item");
            }
        }
        let mut pointer = Value::NIL;
        if self.frame_internal_border_width(f) > 0
            && !self.get_frame_param(f, sym::DRAG_INTERNAL_BORDER).is_nil()
        {
            let border = self.frame_internal_border_part(f, x, y);
            let border_cursor = Some(if border == sym::LEFT_EDGE {
                EmacsCursorKind::LeftEdge
            } else if border == sym::TOP_LEFT_CORNER {
                EmacsCursorKind::TopLeftCorner
            } else if border == sym::TOP_EDGE {
                EmacsCursorKind::TopEdge
            } else if border == sym::TOP_RIGHT_CORNER {
                EmacsCursorKind::TopRightCorner
            } else if border == sym::RIGHT_EDGE {
                EmacsCursorKind::RightEdge
            } else if border == sym::BOTTOM_RIGHT_CORNER {
                EmacsCursorKind::BottomRightCorner
            } else if border == sym::BOTTOM_EDGE {
                EmacsCursorKind::BottomEdge
            } else if border == sym::BOTTOM_LEFT_CORNER {
                EmacsCursorKind::BottomLeftCorner
            } else {
                EmacsCursorKind::Nontext
            });
            if border_cursor != Some(EmacsCursorKind::Nontext) {
                self.xdisp.help_echo_string = self.make_string(b"drag-mouse-1: resize frame");
                self.define_frame_cursor1(f, border_cursor, pointer);
                return Ok(());
            }
        }
        if !self.is_window(window) {
            return Ok(());
        }
        let mut cursor: Option<EmacsCursorKind>;
        let w = window;
        let (x, y) = if self.window(w).flags.pseudo_window_p {
            (
                x - self.frame_internal_border_width(f),
                self.frame_to_window_pixel_y(w, y),
            )
        } else {
            (
                x - self.window_left_edge_x(w),
                self.frame_to_window_pixel_y(w, y),
            )
        };
        if window == self.frame(f).menu_bar_window {
            self.define_frame_cursor1(f, Some(EmacsCursorKind::Nontext), pointer);
            return Ok(());
        }
        if window == self.frame(f).tab_bar_window {
            unreachable!("note_tab_bar_highlight needs the tab bar");
        }
        self.frame_mut(f).last_tab_bar_item = -1;
        if window == self.frame(f).tool_bar_window {
            unreachable!("note_tool_bar_highlight needs the tool bar");
        }
        if matches!(
            part,
            WindowPart::ModeLine
                | WindowPart::HeaderLine
                | WindowPart::TabLine
                | WindowPart::LeftMargin
                | WindowPart::RightMargin
        ) {
            self.note_line_or_margin_highlight(window, x, y, part)?;
            if matches!(part, WindowPart::LeftMargin | WindowPart::RightMargin) {
                self.define_frame_cursor1(f, Some(EmacsCursorKind::Nontext), pointer);
            }
            return Ok(());
        }
        match part {
            WindowPart::VerticalBorder | WindowPart::RightDivider => {
                self.xdisp.help_echo_string = self.make_string(b"drag-mouse-1: resize");
                self.define_frame_cursor1(f, Some(EmacsCursorKind::HorizontalDrag), pointer);
                return Ok(());
            }
            WindowPart::BottomDivider => {
                if !self.window_bottommost_p(w)
                    || self.minibuf.minibuf_level != 0
                    || self.plain_value(sym::RESIZE_MINI_WINDOWS).is_nil()
                {
                    self.xdisp.help_echo_string = self.make_string(b"drag-mouse-1: resize");
                    self.define_frame_cursor1(f, Some(EmacsCursorKind::VerticalDrag), pointer);
                    return Ok(());
                }
                cursor = Some(EmacsCursorKind::Nontext);
            }
            WindowPart::LeftFringe | WindowPart::RightFringe => {
                cursor = Some(EmacsCursorKind::Nontext);
                self.note_fringe_highlight(f, window, x, y, part)?;
            }
            WindowPart::VerticalScrollBar | WindowPart::HorizontalScrollBar => {
                cursor = Some(EmacsCursorKind::Nontext);
            }
            _ => cursor = Some(EmacsCursorKind::Text),
        }
        let b = self.window(w).contents;
        if part == WindowPart::Text
            && self.window(w).flags.window_end_valid
            && !self.window_outdated(w)
        {
            let hit = self.x_y_to_hpos_vpos(w, x, y);
            if hit.glyph.is_some_and(|g| g.kind == GlyphType::Image) {
                unreachable!("image hot spots need image.c");
            }
            let off_text = match hit.glyph {
                None => true,
                Some(g) => {
                    let m = self.hl_matrix(w);
                    let r = &m.rows[hit.vpos];
                    hit.area != TEXT_AREA
                        || !r.displays_text_p
                        || g.object.is_nil()
                        || (r.reversed_p
                            && hit.hpos == 0
                            && g.kind == GlyphType::Stretch
                            && g.avoid_cursor_p)
                }
            };
            if off_text {
                if self.clear_mouse_face(f)? {
                    cursor = None;
                }
                if self.frame_window_p(f) && pointer.is_nil() {
                    if hit.area != TEXT_AREA {
                        cursor = Some(EmacsCursorKind::Nontext);
                    } else {
                        pointer = self.plain_value(sym::VOID_TEXT_AREA_POINTER);
                    }
                }
                self.define_frame_cursor1(f, cursor, pointer);
                return Ok(());
            }
            let glyph = hit.glyph.expect("glyph under the mouse");
            let pos = glyph.charpos;
            let object = glyph.object;
            if (!object.is_string() && !self.is_buffer(object))
                || (self.is_buffer(object) && pos > self.buffer(b).z() as i64)
            {
                self.define_frame_cursor1(f, cursor, pointer);
                return Ok(());
            }
            let obuf = self.current_buffer;
            self.current_buffer = b;
            let saved = {
                let bb = self.buffer(b);
                (bb.begv, bb.begv_byte, bb.zv, bb.zv_byte)
            };
            {
                let z = self.buffer(b).z();
                let z_byte = self.buffer(b).z_byte();
                let bb = self.buffer_mut(b);
                bb.begv = 1;
                bb.begv_byte = 1;
                bb.zv = z;
                bb.zv_byte = z_byte;
            }
            let r = self.note_text_highlight(f, window, &hit, glyph, &mut cursor, &mut pointer);
            {
                let bb = self.buffer_mut(b);
                (bb.begv, bb.begv_byte, bb.zv, bb.zv_byte) = saved;
            }
            self.current_buffer = obuf;
            r?;
        }
        self.define_frame_cursor1(f, cursor, pointer);
        Ok(())
    }

    fn note_text_highlight(
        &mut self,
        f: Value,
        window: Value,
        hit: &XyHit,
        glyph: Glyph,
        cursor: &mut Option<EmacsCursorKind>,
        pointer: &mut Value,
    ) -> LispResult<()> {
        let w = window;
        let mut pos = glyph.charpos;
        let object = glyph.object;
        let position = Value::fixnum(pos);
        let overlay_vec: Vec<Value> = if self.is_buffer(object) {
            let (overlays, _) = self.overlays_at(pos.max(0) as usize);
            self.sort_overlays(&overlays, Some(w))
        } else {
            Vec::new()
        };
        'check_help_echo: {
            if self.plain_value(sym::MOUSE_HIGHLIGHT).is_nil() {
                self.clear_mouse_face(f)?;
                break 'check_help_echo;
            }
            let same_region = self.coords_in_mouse_face_p(w, hit.hpos as i64, hit.vpos as i64);
            if same_region {
                *cursor = None;
            }
            let hl_overlay = self.mouse_hl_info(f).mouse_face_overlay;
            if !same_region
                || (!self.mouse_hl_info(f).mouse_face_hidden
                    && self.is_overlay(hl_overlay)
                    && !self.overlay(hl_overlay).buffer.is_nil()
                    && self.mouse_face_overlay_overlaps(hl_overlay))
            {
                let mut overlay = Value::NIL;
                let mut mouse_face = Value::NIL;
                for &o in overlay_vec.iter().rev() {
                    mouse_face = self.overlay_get(o, sym::MOUSE_FACE);
                    if !mouse_face.is_nil() {
                        overlay = o;
                        break;
                    }
                }
                if !overlay.is_nil() && overlay == self.mouse_hl_info(f).mouse_face_overlay {
                    break 'check_help_echo;
                }
                if self.clear_mouse_face(f)? {
                    *cursor = None;
                }
                self.mouse_hl_info_mut(f).mouse_face_overlay = overlay;
                if overlay.is_nil() {
                    mouse_face = f_get_text_property(self, &[position, sym::MOUSE_FACE, object])?;
                }
                if !mouse_face.is_nil() && object.is_string() {
                    let s = f_previous_single_property_change(
                        self,
                        &[Value::fixnum(pos + 1), sym::MOUSE_FACE, object, Value::NIL],
                    )?;
                    let e = f_next_single_property_change(
                        self,
                        &[position, sym::MOUSE_FACE, object, Value::NIL],
                    )?;
                    let s = s.as_fixnum().unwrap_or(0);
                    let e = e
                        .as_fixnum()
                        .unwrap_or_else(|| self.string_chars(object) as i64);
                    self.mouse_face_from_string_pos(w, object, s, e);
                    {
                        let h = self.mouse_hl_info_mut(f);
                        h.mouse_face_past_end = false;
                        h.mouse_face_window = window;
                    }
                    let (face_id, _) = self.face_at_string_position(
                        w,
                        object,
                        pos,
                        0,
                        glyph.face_id as usize,
                        true,
                        0,
                    )?;
                    self.mouse_hl_info_mut(f).mouse_face_face_id = face_id;
                    self.show_mouse_face(f, DrawGlyphsFace::MouseFace, true)?;
                    *cursor = None;
                } else {
                    let mut buffer = Value::NIL;
                    let mut disp_string = Value::NIL;
                    if object.is_string() {
                        let start = self.hl_matrix(w).rows[hit.vpos].start_charpos();
                        pos = self.string_buffer_position(object, start)?;
                        if pos > 0 {
                            let contents = self.window(w).contents;
                            let (mf, ov) = self.get_char_property_and_overlay(
                                Value::fixnum(pos),
                                sym::MOUSE_FACE,
                                contents,
                            )?;
                            mouse_face = mf;
                            overlay = ov;
                            buffer = contents;
                            disp_string = object;
                        }
                    } else {
                        buffer = object;
                    }
                    if !mouse_face.is_nil() {
                        let reordering = !self
                            .buffer_local_value(sym::BIDI_DISPLAY_REORDERING, buffer)
                            .is_nil();
                        let lim1 = if reordering {
                            Value::NIL
                        } else {
                            let start = self.window(w).start;
                            Value::fixnum(self.marker(start).charpos as i64)
                        };
                        let lim2 = if reordering {
                            Value::NIL
                        } else {
                            Value::fixnum(
                                self.buffer(buffer).z() as i64 - self.window(w).window_end_pos,
                            )
                        };
                        let (before, after, before_string, after_string) = if overlay.is_nil() {
                            let before = f_previous_single_property_change(
                                self,
                                &[Value::fixnum(pos + 1), sym::MOUSE_FACE, buffer, lim1],
                            )?;
                            let after = f_next_single_property_change(
                                self,
                                &[Value::fixnum(pos), sym::MOUSE_FACE, buffer, lim2],
                            )?;
                            (before, after, Value::NIL, Value::NIL)
                        } else {
                            let ov = self.overlay(overlay);
                            let (start, end) = (ov.start as i64, ov.end as i64);
                            let mut before_string = self.overlay_get(overlay, sym::BEFORE_STRING);
                            let mut after_string = self.overlay_get(overlay, sym::AFTER_STRING);
                            if !before_string.is_string() {
                                before_string = Value::NIL;
                            }
                            if !after_string.is_string() {
                                after_string = Value::NIL;
                            }
                            (
                                Value::fixnum(start),
                                Value::fixnum(end),
                                before_string,
                                after_string,
                            )
                        };
                        let z = self.buffer(buffer).z() as i64;
                        self.mouse_face_from_buffer_pos(
                            window,
                            pos,
                            before.as_fixnum().unwrap_or(1),
                            after.as_fixnum().unwrap_or(z),
                            before_string,
                            after_string,
                            disp_string,
                        )?;
                        *cursor = None;
                    }
                }
            }
        }
        if self.xdisp.help_echo_string.is_nil() {
            let mut help = Value::NIL;
            let mut overlay = Value::NIL;
            for &o in overlay_vec.iter().rev() {
                overlay = o;
                help = self.overlay_get(o, sym::HELP_ECHO);
                if !help.is_nil() {
                    break;
                }
            }
            if !help.is_nil() {
                self.set_help_echo(help, window, overlay, pos);
            } else {
                let mut obj = glyph.object;
                let mut charpos = glyph.charpos;
                if obj.is_string() && charpos >= 0 && charpos < self.string_chars(obj) as i64 {
                    help =
                        f_get_text_property(self, &[Value::fixnum(charpos), sym::HELP_ECHO, obj])?;
                    if help.is_nil() {
                        let start = self.hl_matrix(w).rows[hit.vpos].start_charpos();
                        let p = self.string_buffer_position(obj, start)?;
                        if p > 0 {
                            let contents = self.window(w).contents;
                            help =
                                self.get_char_property(Value::fixnum(p), sym::HELP_ECHO, contents)?;
                            if !help.is_nil() {
                                charpos = p;
                                obj = contents;
                            }
                        }
                    }
                } else if self.is_buffer(obj)
                    && charpos >= self.cur().begv as i64
                    && charpos < self.cur().zv as i64
                {
                    help =
                        f_get_text_property(self, &[Value::fixnum(charpos), sym::HELP_ECHO, obj])?;
                }
                if !help.is_nil() {
                    self.set_help_echo(help, window, obj, charpos);
                }
            }
        }
        if self.frame_window_p(f) && pointer.is_nil() {
            for &o in overlay_vec.iter().rev() {
                *pointer = self.overlay_get(o, sym::POINTER);
                if !pointer.is_nil() {
                    break;
                }
            }
            if pointer.is_nil() {
                let obj = glyph.object;
                let charpos = glyph.charpos;
                if obj.is_string() && charpos >= 0 && charpos < self.string_chars(obj) as i64 {
                    *pointer =
                        f_get_text_property(self, &[Value::fixnum(charpos), sym::POINTER, obj])?;
                    if pointer.is_nil() {
                        let start = self.hl_matrix(w).rows[hit.vpos].start_charpos();
                        let p = self.string_buffer_position(obj, start)?;
                        if p > 0 {
                            let contents = self.window(w).contents;
                            *pointer =
                                self.get_char_property(Value::fixnum(p), sym::POINTER, contents)?;
                        }
                    }
                } else if self.is_buffer(obj)
                    && charpos >= self.cur().begv as i64
                    && charpos < self.cur().zv as i64
                {
                    *pointer =
                        f_get_text_property(self, &[Value::fixnum(charpos), sym::POINTER, obj])?;
                }
            }
        }
        Ok(())
    }
}
