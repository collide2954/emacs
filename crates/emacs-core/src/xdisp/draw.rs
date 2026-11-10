use crate::character;
use crate::dispextern::{
    DrawGlyphsFace, EmacsRectangle, Glyph, GlyphRowArea, GlyphRowRef, GlyphSlice, GlyphString,
    GlyphType, GlyphU, LEFT_MARGIN_AREA, MouseHlInfo, OVERLAPS_BOTH, OVERLAPS_ERASED_CURSOR,
    OVERLAPS_PRED, OVERLAPS_SUCC, RIGHT_MARGIN_AREA, RedisplayInterface, TEXT_AREA,
    gui_intersect_rectangles,
};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::font::{FONT_INVALID_CODE, FontMetrics};
use crate::lisp::Lisp;
use crate::value::Value;
use crate::xfaces::{DEFAULT_FACE_ID, MOUSE_FACE_ID};

pub fn glyph_row_area(area: usize) -> GlyphRowArea {
    match area {
        LEFT_MARGIN_AREA => GlyphRowArea::LeftMargin,
        TEXT_AREA => GlyphRowArea::Text,
        RIGHT_MARGIN_AREA => GlyphRowArea::RightMargin,
        _ => GlyphRowArea::Any,
    }
}

fn cmp_union(u: GlyphU) -> (bool, u32) {
    let val = u.val();
    (val & 1 != 0, val >> 1)
}

fn slice_cmp(slice: GlyphSlice) -> (i32, i32) {
    match slice {
        GlyphSlice::None => (0, 0),
        GlyphSlice::Cmp { from, to } => (from, to),
        GlyphSlice::Img {
            x,
            y,
            width,
            height,
        } => (
            (u32::from(x) | (u32::from(y) << 16)) as i32,
            (u32::from(width) | (u32::from(height) << 16)) as i32,
        ),
        GlyphSlice::Glyphless {
            upper_xoff,
            upper_yoff,
            lower_xoff,
            lower_yoff,
        } => (
            (u32::from(upper_xoff as u16) | (u32::from(upper_yoff as u16) << 16)) as i32,
            (u32::from(lower_xoff as u16) | (u32::from(lower_yoff as u16) << 16)) as i32,
        ),
    }
}

fn append_glyph_string_lists(
    arena: &mut [GlyphString],
    head: &mut Option<usize>,
    tail: &mut Option<usize>,
    h: Option<usize>,
    t: Option<usize>,
) {
    if let Some(h) = h {
        match *head {
            Some(_) => arena[tail.expect("glyph string list tail")].next = Some(h),
            None => *head = Some(h),
        }
        arena[h].prev = *tail;
        *tail = t;
    }
}

fn prepend_glyph_string_lists(
    arena: &mut [GlyphString],
    head: &mut Option<usize>,
    tail: &mut Option<usize>,
    h: Option<usize>,
    t: Option<usize>,
) {
    if let Some(h) = h {
        let t = t.expect("glyph string list tail");
        match *head {
            Some(hd) => arena[hd].prev = Some(t),
            None => *tail = Some(t),
        }
        arena[t].next = *head;
        *head = Some(h);
    }
}

fn append_glyph_string(
    arena: &mut [GlyphString],
    head: &mut Option<usize>,
    tail: &mut Option<usize>,
    s: usize,
) {
    arena[s].next = None;
    arena[s].prev = None;
    append_glyph_string_lists(arena, head, tail, Some(s), Some(s));
}

impl Lisp {
    pub fn frame_rif(&self, f: Value) -> &'static RedisplayInterface {
        let t = self.frame(f).terminal;
        self.terminal(t)
            .rif
            .expect("window frame without a redisplay interface")
    }

    pub fn mouse_hl_info(&self, f: Value) -> &MouseHlInfo {
        &self.display_info(self.frame(f).terminal).mouse_highlight
    }

    pub fn mouse_hl_info_mut(&mut self, f: Value) -> &mut MouseHlInfo {
        let t = self.frame(f).terminal;
        &mut self.display_info_mut(t).mouse_highlight
    }

    pub(crate) fn frame_internal_border_width(&self, f: Value) -> i64 {
        self.frame(f).geometry.internal_border_width
    }

    pub(crate) fn window_bar_window_p(&self, w: Value) -> bool {
        let fr = self.frame(self.window(w).frame);
        fr.menu_bar_window == w || fr.tab_bar_window == w || fr.tool_bar_window == w
    }

    pub fn window_left_edge_x(&self, w: Value) -> i64 {
        self.frame_internal_border_width(self.window(w).frame) + self.window(w).pixel_left
    }

    pub fn window_box_left_edge_x(&self, w: Value) -> i64 {
        self.frame_internal_border_width(self.window(w).frame)
            + self.window(w).pixel_left
            + self.window_left_scroll_bar_area_width(w)
    }

    pub fn window_top_edge_y(&self, w: Value) -> i64 {
        let border = if self.window_bar_window_p(w) {
            0
        } else {
            self.frame_internal_border_width(self.window(w).frame)
        };
        border + self.window(w).pixel_top
    }

    pub fn window_to_frame_pixel_x(&self, w: Value, x: i64) -> i64 {
        x + self.window_box_left_edge_x(w)
    }

    pub fn window_to_frame_pixel_y(&self, w: Value, y: i64) -> i64 {
        y + self.window_top_edge_y(w)
    }

    pub fn frame_to_window_pixel_x(&self, w: Value, x: i64) -> i64 {
        x - self.window_box_left_edge_x(w)
    }

    pub fn frame_to_window_pixel_y(&self, w: Value, y: i64) -> i64 {
        y - self.window_top_edge_y(w)
    }

    pub fn window_box_left(&self, w: Value, area: GlyphRowArea) -> i64 {
        if self.window(w).flags.pseudo_window_p {
            return self.frame_internal_border_width(self.window(w).frame);
        }
        self.window_left_edge_x(w) + self.window_box_left_offset(w, area)
    }

    pub fn window_box_right(&self, w: Value, area: GlyphRowArea) -> i64 {
        self.window_box_left(w, area) + self.window_box_width(w, area)
    }

    pub fn window_text_to_frame_pixel_x(&self, w: Value, x: i64) -> i64 {
        self.window_box_left(w, GlyphRowArea::Text) + x
    }

    pub fn window_right_edge_x(&self, w: Value) -> i64 {
        self.frame_internal_border_width(self.window(w).frame)
            + self.window(w).pixel_left
            + self.window(w).pixel_width
    }

    pub fn window_box(&mut self, w: Value, area: GlyphRowArea) -> (i64, i64, i64, i64) {
        let width = self.window_box_width(w, area);
        let height = self.window_box_height(w);
        let x = self.window_box_left(w, area);
        let mut y = self.window_top_edge_y(w);
        if self.window_wants_tab_line(w) {
            y += self.current_tab_line_height(w);
        }
        if self.window_wants_header_line(w) {
            y += self.current_header_line_height(w);
        }
        (x, y, width, height)
    }

    fn top_lines_height(&mut self, w: Value) -> i64 {
        self.window_tab_line_height(w) + self.window_header_line_height(w)
    }

    fn row_glyph(&self, row: GlyphRowRef, area: usize, i: usize) -> Glyph {
        let r = self.glyph_row(row);
        r.glyphs[r.area[area] + i]
    }

    fn current_row_mouse_face_p(&self, w: Value, vpos: i64) -> bool {
        usize::try_from(vpos).ok().is_some_and(|vpos| {
            self.window_matrix(w, WhichMatrix::Current)
                .and_then(|m| m.rows.get(vpos))
                .is_some_and(|r| r.mouse_face_p)
        })
    }

    fn mouse_face_override_p(&self, s: &GlyphString) -> bool {
        s.hl == DrawGlyphsFace::MouseFace
            || (s.hl == DrawGlyphsFace::Cursor
                && self.current_row_mouse_face_p(s.w, self.window(s.w).phys_cursor.vpos)
                && self.cursor_in_mouse_face_p(s.w))
    }

    fn mouse_face_id(&self, f: Value) -> usize {
        let id = self.mouse_hl_info(f).mouse_face_face_id;
        if self.face_from_id(f, id).is_some() {
            id
        } else {
            MOUSE_FACE_ID
        }
    }

    fn face_font(&self, f: Value, face: usize) -> Value {
        self.face_from_id(f, face).expect("realized face").font
    }

    fn get_glyph_face_and_encoding(&mut self, f: Value, glyph: &Glyph) -> (usize, Option<u32>) {
        let face = glyph.face_id as usize;
        self.prepare_face_for_display(f, face);
        if glyph.kind != GlyphType::Char {
            return (face, None);
        }
        let font = self.face_font(f, face);
        let mut code = 0;
        if !font.is_nil() {
            let c = glyph.u.val();
            code = if character::char_byte8_p(c) {
                u32::from(character::char_to_byte8(c))
            } else {
                self.font_encode_char(font, c as i32)
            };
            if code == FONT_INVALID_CODE {
                code = 0;
            }
        }
        (face, Some(code & 0xFFFF))
    }

    fn init_glyph_string(
        &mut self,
        w: Value,
        row: GlyphRowRef,
        area: usize,
        start: usize,
        hl: DrawGlyphsFace,
    ) -> GlyphString {
        let f = self.window(w).frame;
        let (height, y, ascent) = {
            let r = self.glyph_row(row);
            (r.height, r.y, r.ascent)
        };
        let mut s = GlyphString::new(w, f, row, area, start);
        s.hl = hl;
        s.height = height;
        s.y = self.window_to_frame_pixel_y(w, i64::from(y)) as i32;
        s.ybase = s.y + ascent;
        s
    }

    fn fill_composite_glyph_string(
        &mut self,
        arena: &mut [GlyphString],
        si: usize,
        base_face: usize,
        overlaps: u8,
    ) -> LispResult<i32> {
        let f = arena[si].f;
        let cmp = arena[si].cmp.expect("static composition");
        let glyph_len = self.compositions.table[cmp].glyph_len;
        let base_ascii = self
            .face_from_id(f, base_face)
            .expect("realized face")
            .ascii_face;
        arena[si].for_overlaps = overlaps;
        arena[si].face = None;
        arena[si].font = Value::NIL;
        let mut i = arena[si].cmp_from.max(0) as usize;
        while i < glyph_len {
            let c = self.composition_glyph(cmp, i);
            if c != i32::from(b'\t') {
                let face_id = self.face_for_char(f, base_ascii, c as u32, -1, Value::NIL)?;
                let face = face_id;
                let code = self.get_char_face_and_encoding(f, c, face_id);
                arena[si].char2b[i] = code;
                match arena[si].face {
                    None => {
                        arena[si].face = Some(face);
                        arena[si].font = self.face_font(f, face);
                    }
                    Some(existing) if existing != face => break,
                    Some(_) => {}
                }
            }
            arena[si].nchars += 1;
            i += 1;
        }
        arena[si].cmp_to = i as i32;
        if arena[si].face.is_none() {
            arena[si].face = Some(base_ascii);
            arena[si].font = self.face_font(f, base_ascii);
        }
        if self.mouse_face_override_p(&arena[si]) {
            let c = self.composition_glyph(cmp, 0);
            let face = self.mouse_face_id(f);
            let face = self.face_for_char(f, face, c as u32, -1, Value::NIL)?;
            self.prepare_face_for_display(f, face);
            arena[si].face = Some(face);
        }
        let first = self.row_glyph(arena[si].row, arena[si].area, arena[si].first_glyph);
        arena[si].width = i32::from(first.pixel_width);
        if arena[si].font.is_nil() {
            arena[si].font_not_found_p = true;
            arena[si].font = self.frame(f).font;
        }
        arena[si].ybase += i32::from(first.voffset);
        Ok(arena[si].cmp_to)
    }

    fn fill_gstring_glyph_string(
        &mut self,
        arena: &mut [GlyphString],
        si: usize,
        face_id: usize,
        start: usize,
        end: usize,
        overlaps: u8,
    ) -> usize {
        let (f, row, area) = (arena[si].f, arena[si].row, arena[si].area);
        arena[si].for_overlaps = overlaps;
        let first = self.row_glyph(row, area, start);
        let voffset = first.voffset;
        let glyph_not_available_p = first.glyph_not_available_p;
        let (_, id) = cmp_union(first.u);
        let (from, to) = slice_cmp(first.slice);
        arena[si].cmp_id = i64::from(id);
        arena[si].cmp_from = from;
        arena[si].cmp_to = to + 1;
        if self.mouse_face_override_p(&arena[si]) {
            let face = self.mouse_face_id(f);
            arena[si].face = Some(face);
            self.prepare_face_for_display(f, face);
        } else {
            arena[si].face = Some(face_id);
        }
        let lgstring = self.composition_gstring_from_id(id as usize);
        arena[si].font = self.lgstring_font(lgstring);
        let anchor = self.row_glyph(row, area, arena[si].first_glyph);
        arena[si].width = i32::from(anchor.pixel_width);
        let mut glyph = start + 1;
        while glyph < end {
            let g = self.row_glyph(row, area, glyph);
            let (automatic, gid) = cmp_union(g.u);
            let (gfrom, gto) = slice_cmp(g.slice);
            if !(automatic
                && i64::from(gid) == arena[si].cmp_id
                && g.face_id as usize == face_id
                && arena[si].cmp_to == gfrom
                && g.glyph_not_available_p == glyph_not_available_p)
            {
                break;
            }
            arena[si].width += i32::from(g.pixel_width);
            arena[si].cmp_to = gto + 1;
            glyph += 1;
        }
        for i in arena[si].cmp_from..arena[si].cmp_to {
            let lglyph = self.lgstring_glyph(lgstring, i as usize);
            let code = self.lglyph_code(lglyph);
            arena[si].char2b[i as usize] = code & 0xFFFF;
        }
        if glyph_not_available_p {
            arena[si].font_not_found_p = true;
        }
        arena[si].ybase += i32::from(voffset);
        glyph
    }

    fn fill_glyphless_glyph_string(
        &mut self,
        arena: &mut [GlyphString],
        si: usize,
        face_id: usize,
        start: usize,
        end: usize,
        overlaps: u8,
    ) -> usize {
        let (f, row, area) = (arena[si].f, arena[si].row, arena[si].area);
        arena[si].for_overlaps = overlaps;
        let first = self.row_glyph(row, area, start);
        let voffset = first.voffset;
        arena[si].face = Some(face_id);
        let font = self.face_font(f, face_id);
        arena[si].font = if font.is_nil() {
            self.frame(f).font
        } else {
            font
        };
        if self.mouse_face_override_p(&arena[si]) {
            let face = self.mouse_face_id(f);
            arena[si].face = Some(face);
            self.prepare_face_for_display(f, face);
        }
        arena[si].nchars = 1;
        arena[si].width = i32::from(first.pixel_width);
        let mut glyph = start + 1;
        while glyph < end {
            let g = self.row_glyph(row, area, glyph);
            if !(g.kind == GlyphType::Glyphless
                && g.voffset == voffset
                && g.face_id as usize == face_id)
            {
                break;
            }
            arena[si].nchars += 1;
            arena[si].width += i32::from(g.pixel_width);
            glyph += 1;
        }
        arena[si].ybase += i32::from(voffset);
        glyph
    }

    fn fill_glyph_string(
        &mut self,
        arena: &mut [GlyphString],
        si: usize,
        face_id: usize,
        start: usize,
        end: usize,
        overlaps: u8,
    ) -> LispResult<usize> {
        let (f, row, area) = (arena[si].f, arena[si].row, arena[si].area);
        arena[si].for_overlaps = overlaps;
        let first = self.row_glyph(row, area, start);
        let voffset = first.voffset;
        arena[si].padding_p = first.padding_p;
        let glyph_not_available_p = first.glyph_not_available_p;
        let mut glyph = start;
        while glyph < end {
            let g = self.row_glyph(row, area, glyph);
            if !(g.kind == GlyphType::Char
                && g.voffset == voffset
                && g.face_id as usize == face_id
                && g.glyph_not_available_p == glyph_not_available_p)
            {
                break;
            }
            let (face, code) = self.get_glyph_face_and_encoding(f, &g);
            arena[si].face = Some(face);
            let n = arena[si].nchars;
            arena[si].char2b[n] = code.unwrap_or(0);
            arena[si].nchars += 1;
            arena[si].width += i32::from(g.pixel_width);
            glyph += 1;
            if g.padding_p != arena[si].padding_p {
                break;
            }
        }
        let face = arena[si].face.expect("glyph string face");
        arena[si].font = self.face_font(f, face);
        if self.mouse_face_override_p(&arena[si]) {
            let anchor = self.row_glyph(row, area, arena[si].first_glyph);
            let face = self.mouse_face_id(f);
            let face = self.face_for_char(f, face, anchor.u.val(), -1, Value::NIL)?;
            self.prepare_face_for_display(f, face);
            arena[si].face = Some(face);
        }
        if arena[si].font.is_nil() || glyph_not_available_p {
            arena[si].font_not_found_p = true;
            arena[si].font = self.frame(f).font;
        }
        arena[si].ybase += i32::from(voffset);
        Ok(glyph)
    }

    fn fill_stretch_glyph_string(
        &mut self,
        arena: &mut [GlyphString],
        si: usize,
        start: usize,
        end: usize,
    ) -> usize {
        let (f, row, area) = (arena[si].f, arena[si].row, arena[si].area);
        let first = self.row_glyph(row, area, start);
        let face_id = first.face_id as usize;
        arena[si].face = Some(face_id);
        arena[si].font = self.face_font(f, face_id);
        if self.mouse_face_override_p(&arena[si]) {
            let face = self.mouse_face_id(f);
            arena[si].face = Some(face);
            self.prepare_face_for_display(f, face);
        }
        arena[si].width = i32::from(first.pixel_width);
        arena[si].nchars = 1;
        let voffset = first.voffset;
        let mut glyph = start + 1;
        while glyph < end {
            let g = self.row_glyph(row, area, glyph);
            if !(g.kind == GlyphType::Stretch
                && g.voffset == voffset
                && g.face_id as usize == face_id)
            {
                break;
            }
            arena[si].width += i32::from(g.pixel_width);
            glyph += 1;
        }
        arena[si].ybase += i32::from(voffset);
        glyph
    }

    fn left_overwritten(&self, s: &GlyphString) -> i64 {
        if s.left_overhang == 0 {
            return -1;
        }
        let r = self.glyph_row(s.row);
        let glyphs = &r.glyphs[r.area[s.area]..];
        let mut x = 0;
        let mut i = s.first_glyph as i64 - 1;
        while i >= 0 && x > -s.left_overhang {
            x -= i32::from(glyphs[i as usize].pixel_width);
            i -= 1;
        }
        i + 1
    }

    fn left_overwriting(&mut self, s: &GlyphString) -> i64 {
        let mut k = -1;
        let mut x = 0;
        let mut i = s.first_glyph as i64 - 1;
        while i >= 0 {
            let g = self.row_glyph(s.row, s.area, i as usize);
            let (_, right) = gui_get_glyph_overhangs(self, &g, s.f);
            if x + right > 0 {
                k = i;
            }
            x -= i32::from(g.pixel_width);
            i -= 1;
        }
        k
    }

    fn first_glyph_after(&self, s: &GlyphString) -> usize {
        let first = self.row_glyph(s.row, s.area, s.first_glyph);
        s.first_glyph
            + if first.kind == GlyphType::Composite {
                1
            } else {
                s.nchars
            }
    }

    fn right_overwritten(&self, s: &GlyphString) -> i64 {
        if s.right_overhang == 0 {
            return -1;
        }
        let r = self.glyph_row(s.row);
        let glyphs = &r.glyphs[r.area[s.area]..];
        let end = r.used[s.area] as usize;
        let mut x = 0;
        let mut i = self.first_glyph_after(s);
        while i < end && s.right_overhang > x {
            x += i32::from(glyphs[i].pixel_width);
            i += 1;
        }
        i as i64
    }

    fn right_overwriting(&mut self, s: &GlyphString) -> i64 {
        let end = self.glyph_row(s.row).used[s.area] as usize;
        let mut k = -1;
        let mut x = 0;
        for i in self.first_glyph_after(s)..end {
            let g = self.row_glyph(s.row, s.area, i);
            let (left, _) = gui_get_glyph_overhangs(self, &g, s.f);
            if x - left < 0 {
                k = i as i64;
            }
            x += i32::from(g.pixel_width);
        }
        k
    }

    fn adjust_glyph_width_for_mouse_face(
        &self,
        g: &Glyph,
        original_face: usize,
        mouse_face: usize,
        f: Value,
    ) -> i32 {
        if g.kind == GlyphType::Image {
            unreachable!("image glyphs need image.c");
        }
        let original = self
            .face_from_id(f, original_face)
            .expect("realized face")
            .box_vertical_line_width;
        let mouse = self
            .face_from_id(f, mouse_face)
            .expect("realized face")
            .box_vertical_line_width;
        let mut sum = 0;
        if g.left_box_line_p {
            sum -= original.max(0);
        }
        if g.right_box_line_p {
            sum -= original.max(0);
        }
        if g.left_box_line_p {
            sum += mouse.max(0);
        }
        if g.right_box_line_p {
            sum += mouse.max(0);
        }
        sum
    }

    fn set_glyph_string_background_width(
        &mut self,
        arena: &mut [GlyphString],
        si: usize,
        start: usize,
        last_x: i32,
    ) {
        let (fill_line_p, used) = {
            let r = self.glyph_row(arena[si].row);
            (r.fill_line_p, r.used[arena[si].area] as usize)
        };
        let hl = arena[si].hl;
        if start == used
            && ((fill_line_p
                && matches!(
                    hl,
                    DrawGlyphsFace::NormalText
                        | DrawGlyphsFace::ImageRaised
                        | DrawGlyphsFace::ImageSunken
                ))
                || hl == DrawGlyphsFace::MouseFace)
        {
            arena[si].extends_to_end_of_line_p = true;
        }
        if arena[si].extends_to_end_of_line_p {
            arena[si].background_width = last_x - arena[si].x + 1;
        } else {
            arena[si].background_width = arena[si].width;
            let s = &arena[si];
            if self.frame_window_p(s.f)
                && s.hl == DrawGlyphsFace::Cursor
                && self.current_row_mouse_face_p(s.w, self.window(s.w).phys_cursor.vpos)
                && self.cursor_in_mouse_face_p(s.w)
            {
                let g = self.row_glyph(s.row, s.area, s.first_glyph);
                let adjust = self.adjust_glyph_width_for_mouse_face(
                    &g,
                    g.face_id as usize,
                    s.face.expect("glyph string face"),
                    s.f,
                );
                arena[si].background_width += adjust;
                arena[si].width = arena[si].background_width;
            }
        }
    }

    fn glyph_string_containing_background_width(&self, arena: &[GlyphString], s: usize) -> usize {
        let mut s = s;
        if arena[s].cmp.is_some() {
            while arena[s].cmp_from != 0 {
                s = arena[s].prev.expect("composition glyph string predecessor");
            }
        }
        s
    }

    fn compute_overhangs_and_x(
        &mut self,
        arena: &mut [GlyphString],
        s: Option<usize>,
        x: i32,
        backward_p: bool,
    ) {
        let mut s = s;
        let mut x = x;
        while let Some(i) = s {
            if let Some(hook) = self.frame_rif(arena[i].f).compute_glyph_string_overhangs {
                hook(self, arena, i);
            }
            let whole = arena[i]
                .cmp
                .is_none_or(|c| arena[i].cmp_to as usize == self.compositions.table[c].glyph_len);
            if backward_p {
                if whole {
                    x -= arena[i].width;
                }
                arena[i].x = x;
                s = arena[i].prev;
            } else {
                arena[i].x = x;
                if whole {
                    x += arena[i].width;
                }
                s = arena[i].next;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_glyph_strings(
        &mut self,
        arena: &mut Vec<GlyphString>,
        w: Value,
        row: GlyphRowRef,
        area: usize,
        start: &mut usize,
        end: usize,
        hl: DrawGlyphsFace,
        x: &mut i32,
        last_x: i32,
        overlaps: u8,
    ) -> LispResult<(Option<usize>, Option<usize>)> {
        let mut head = None;
        let mut tail = None;
        while *start < end {
            let first_glyph = self.row_glyph(row, area, *start);
            let s = match first_glyph.kind {
                GlyphType::Char => {
                    let face_id = first_glyph.face_id as usize;
                    let mut gs = self.init_glyph_string(w, row, area, *start, hl);
                    gs.char2b = vec![0; end - *start];
                    arena.push(gs);
                    let s = arena.len() - 1;
                    append_glyph_string(arena, &mut head, &mut tail, s);
                    arena[s].x = *x;
                    *start = self.fill_glyph_string(arena, s, face_id, *start, end, overlaps)?;
                    s
                }
                GlyphType::Composite => {
                    let (automatic, cmp_id) = cmp_union(first_glyph.u);
                    let face_id = first_glyph.face_id as usize;
                    if automatic {
                        let gstring = self.composition_gstring_from_id(cmp_id as usize);
                        let len = self.lgstring_glyph_len(gstring);
                        let mut gs = self.init_glyph_string(w, row, area, *start, hl);
                        gs.char2b = vec![0; len];
                        arena.push(gs);
                        let s = arena.len() - 1;
                        append_glyph_string(arena, &mut head, &mut tail, s);
                        arena[s].x = *x;
                        *start = self
                            .fill_gstring_glyph_string(arena, s, face_id, *start, end, overlaps);
                        s
                    } else {
                        let cmp = cmp_id as usize;
                        let glyph_len = self.compositions.table[cmp].glyph_len;
                        let mut first_s = None;
                        let mut n = 0;
                        while n < glyph_len {
                            let mut gs = self.init_glyph_string(w, row, area, *start, hl);
                            gs.char2b = vec![0; glyph_len];
                            arena.push(gs);
                            let s = arena.len() - 1;
                            append_glyph_string(arena, &mut head, &mut tail, s);
                            arena[s].cmp = Some(cmp);
                            arena[s].cmp_from = n as i32;
                            arena[s].x = *x;
                            if n == 0 {
                                first_s = Some(s);
                            }
                            n = self.fill_composite_glyph_string(arena, s, face_id, overlaps)?
                                as usize;
                        }
                        *start += 1;
                        first_s.expect("composition with glyphs")
                    }
                }
                GlyphType::Stretch => {
                    let gs = self.init_glyph_string(w, row, area, *start, hl);
                    arena.push(gs);
                    let s = arena.len() - 1;
                    *start = self.fill_stretch_glyph_string(arena, s, *start, end);
                    append_glyph_string(arena, &mut head, &mut tail, s);
                    arena[s].x = *x;
                    s
                }
                GlyphType::Image => unreachable!("image glyphs need image.c"),
                GlyphType::Xwidget => unreachable!("xwidgets are not supported"),
                GlyphType::Glyphless => {
                    let face_id = first_glyph.face_id as usize;
                    let gs = self.init_glyph_string(w, row, area, *start, hl);
                    arena.push(gs);
                    let s = arena.len() - 1;
                    append_glyph_string(arena, &mut head, &mut tail, s);
                    arena[s].x = *x;
                    *start =
                        self.fill_glyphless_glyph_string(arena, s, face_id, *start, end, overlaps);
                    s
                }
            };
            self.set_glyph_string_background_width(arena, s, *start, last_x);
            *x += arena[s].width;
        }
        Ok((head, tail))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_glyphs(
        &mut self,
        w: Value,
        x: i32,
        row: GlyphRowRef,
        area: usize,
        start: i64,
        end: i64,
        hl: DrawGlyphsFace,
        overlaps: u8,
    ) -> LispResult<i32> {
        let f = self.window(w).frame;
        let (
            used,
            full_width_p,
            mode_line_p,
            contains_overlapping,
            row_mouse_face_p,
            row_y,
            row_bottom,
        ) = {
            let r = self.glyph_row(row);
            (
                i64::from(r.used[area]),
                r.full_width_p,
                r.mode_line_p,
                r.contains_overlapping_glyphs_p,
                r.mouse_face_p,
                r.y,
                r.bottom_y(),
            )
        };
        let end = end.min(used);
        let mut start = start.clamp(0, end.max(0));
        let (area_left, last_x) = if full_width_p {
            let area_left = self.window_left_edge_x(w) as i32;
            let divider = if mode_line_p {
                self.window_right_divider_width(w)
            } else {
                0
            };
            (
                area_left,
                (self.window_left_edge_x(w) + self.window(w).pixel_width - divider) as i32,
            )
        } else {
            let area_left = self.window_box_left(w, glyph_row_area(area)) as i32;
            (
                area_left,
                area_left + self.window_box_width(w, glyph_row_area(area)) as i32,
            )
        };
        let mut x = x + area_left;
        let mut arena: Vec<GlyphString> = Vec::new();
        let mut i = start as usize;
        let (mut head, mut tail) = self.build_glyph_strings(
            &mut arena,
            w,
            row,
            area,
            &mut i,
            end as usize,
            hl,
            &mut x,
            last_x,
            overlaps,
        )?;
        let x_reached = match tail {
            Some(t) => {
                let s = self.glyph_string_containing_background_width(&arena, t);
                arena[s].x + arena[s].background_width
            }
            None => x,
        };
        let mut clip_head = None;
        let mut clip_tail = None;
        if let Some(first) = head
            && overlaps == 0
            && contains_overlapping
        {
            let mut head_s = first;
            let hlinfo = *self.mouse_hl_info(f);
            let mut mouse_beg_col = 0;
            let mut mouse_end_col = 0;
            let mut check_mouse_face = false;
            let mut dummy_x = 0;
            if area == TEXT_AREA
                && row_mouse_face_p
                && hlinfo.mouse_face_beg_row >= 0
                && hlinfo.mouse_face_end_row >= 0
            {
                let row_vpos = match row {
                    GlyphRowRef::Window { vpos, .. } => vpos as i32,
                    GlyphRowRef::Scratch => -1,
                };
                if row_vpos >= hlinfo.mouse_face_beg_row && row_vpos <= hlinfo.mouse_face_end_row {
                    check_mouse_face = true;
                    mouse_beg_col = if row_vpos == hlinfo.mouse_face_beg_row {
                        hlinfo.mouse_face_beg_col
                    } else {
                        0
                    };
                    mouse_end_col = if row_vpos == hlinfo.mouse_face_end_row {
                        hlinfo.mouse_face_end_col
                    } else {
                        self.glyph_row(row).used[TEXT_AREA] as i32
                    };
                }
            }
            if let Some(hook) = self.frame_rif(f).compute_glyph_string_overhangs {
                let mut s = head;
                while let Some(i) = s {
                    hook(self, &mut arena, i);
                    s = arena[i].next;
                }
            }
            let i = self.left_overwritten(&arena[head_s]);
            if i >= 0 {
                let overlap_hl = if check_mouse_face
                    && i64::from(mouse_beg_col) < start
                    && i64::from(mouse_end_col) > i
                {
                    DrawGlyphsFace::MouseFace
                } else {
                    DrawGlyphsFace::NormalText
                };
                if hl != overlap_hl {
                    clip_head = head;
                }
                let mut j = i as usize;
                let (h, t) = self.build_glyph_strings(
                    &mut arena,
                    w,
                    row,
                    area,
                    &mut j,
                    start as usize,
                    overlap_hl,
                    &mut dummy_x,
                    last_x,
                    overlaps,
                )?;
                start = i;
                let head_x = arena[head_s].x;
                self.compute_overhangs_and_x(&mut arena, t, head_x, true);
                prepend_glyph_string_lists(&mut arena, &mut head, &mut tail, h, t);
                if clip_head.is_none() {
                    clip_head = head;
                }
                head_s = head.expect("glyph string list head");
            }
            let i = self.left_overwriting(&arena[head_s].clone());
            if i >= 0 {
                let overlap_hl = if check_mouse_face
                    && i64::from(mouse_beg_col) < start
                    && i64::from(mouse_end_col) > i
                {
                    DrawGlyphsFace::MouseFace
                } else {
                    DrawGlyphsFace::NormalText
                };
                if hl == overlap_hl || clip_head.is_none() {
                    clip_head = head;
                }
                let mut j = i as usize;
                let (h, t) = self.build_glyph_strings(
                    &mut arena,
                    w,
                    row,
                    area,
                    &mut j,
                    start as usize,
                    overlap_hl,
                    &mut dummy_x,
                    last_x,
                    overlaps,
                )?;
                let mut s = h;
                while let Some(k) = s {
                    arena[k].background_filled_p = true;
                    s = arena[k].next;
                }
                let head_x = arena[head_s].x;
                self.compute_overhangs_and_x(&mut arena, t, head_x, true);
                prepend_glyph_string_lists(&mut arena, &mut head, &mut tail, h, t);
            }
            let tail_s = tail.expect("glyph string list tail");
            let i = self.right_overwritten(&arena[tail_s]);
            if i >= 0 {
                let overlap_hl = if check_mouse_face
                    && i64::from(mouse_beg_col) < i
                    && i64::from(mouse_end_col) > end
                {
                    DrawGlyphsFace::MouseFace
                } else {
                    DrawGlyphsFace::NormalText
                };
                if hl != overlap_hl {
                    clip_tail = tail;
                }
                let mut e = end as usize;
                let (h, t) = self.build_glyph_strings(
                    &mut arena, w, row, area, &mut e, i as usize, overlap_hl, &mut x, last_x,
                    overlaps,
                )?;
                let tx = arena[tail_s].x + arena[tail_s].width;
                self.compute_overhangs_and_x(&mut arena, h, tx, false);
                append_glyph_string_lists(&mut arena, &mut head, &mut tail, h, t);
                if clip_tail.is_none() {
                    clip_tail = tail;
                }
            }
            let tail_s = tail.expect("glyph string list tail");
            let i = self.right_overwriting(&arena[tail_s].clone());
            if i >= 0 {
                let overlap_hl = if check_mouse_face
                    && i64::from(mouse_beg_col) < i
                    && i64::from(mouse_end_col) > end
                {
                    DrawGlyphsFace::MouseFace
                } else {
                    DrawGlyphsFace::NormalText
                };
                if hl == overlap_hl || clip_tail.is_none() {
                    clip_tail = tail;
                }
                let mut e = end as usize;
                let (h, t) = self.build_glyph_strings(
                    &mut arena,
                    w,
                    row,
                    area,
                    &mut e,
                    i as usize + 1,
                    overlap_hl,
                    &mut x,
                    last_x,
                    overlaps,
                )?;
                let mut s = h;
                while let Some(k) = s {
                    arena[k].background_filled_p = true;
                    s = arena[k].next;
                }
                let tx = arena[tail_s].x + arena[tail_s].width;
                self.compute_overhangs_and_x(&mut arena, h, tx, false);
                append_glyph_string_lists(&mut arena, &mut head, &mut tail, h, t);
            }
            tail = tail.map(|t| self.glyph_string_containing_background_width(&arena, t));
            if let Some(ct) = clip_tail {
                clip_tail = Some(self.glyph_string_containing_background_width(&arena, ct));
            }
            if clip_head.is_some() || clip_tail.is_some() {
                let mut s = head;
                while let Some(k) = s {
                    arena[k].clip_head = clip_head;
                    arena[k].clip_tail = clip_tail;
                    s = arena[k].next;
                }
            }
        }
        let draw = self
            .frame_rif(f)
            .draw_glyph_string
            .expect("redisplay interface draw_glyph_string");
        let mut s = head;
        while let Some(k) = s {
            draw(self, &mut arena, k);
            s = arena[k].next;
        }
        if area == TEXT_AREA && !full_width_p && overlaps == 0 {
            let x0 = match (clip_head, head) {
                (Some(c), _) => arena[c].x,
                (None, Some(h)) => arena[h].x,
                (None, None) => x,
            };
            let x1 = match (clip_tail, tail) {
                (Some(c), _) => arena[c].x + arena[c].background_width,
                (None, Some(t)) => arena[t].x + arena[t].background_width,
                (None, None) => x,
            };
            self.notice_overwritten_cursor(
                w,
                TEXT_AREA,
                x0 - area_left,
                x1 - area_left,
                row_y,
                row_bottom,
            )?;
        }
        Ok(if full_width_p {
            self.frame_to_window_pixel_x(w, i64::from(x_reached)) as i32
        } else {
            x_reached - area_left
        })
    }

    pub fn font_for_underline_metrics(&self, s: &GlyphString) -> Value {
        let r = self.glyph_row(s.row);
        let glyphs = &r.glyphs[r.area[s.area]..];
        let mut g = s.first_glyph as i64 - 1;
        while g >= 0 {
            let underlined = self
                .face_from_id(s.f, glyphs[g as usize].face_id as usize)
                .is_some_and(|face| {
                    face.underline != crate::xfaces::FaceUnderlineType::NoUnderline
                });
            if !underlined {
                break;
            }
            g -= 1;
        }
        if g == s.first_glyph as i64 - 1 {
            s.font
        } else {
            self.face_font(s.f, glyphs[(g + 1) as usize].face_id as usize)
        }
    }

    pub fn get_glyph_string_clip_rects(
        &mut self,
        arena: &[GlyphString],
        si: usize,
        n: usize,
    ) -> Vec<EmacsRectangle> {
        if n == 0 {
            return Vec::new();
        }
        let s = &arena[si];
        let w = s.w;
        let (full_width_p, mode_line_p, visible_height, row_y, row_clip) = {
            let r = self.glyph_row(s.row);
            (r.full_width_p, r.mode_line_p, r.visible_height, r.y, r.clip)
        };
        let mut r = EmacsRectangle::default();
        if full_width_p {
            r.x = self.window_left_edge_x(w) as i32;
            r.width = if mode_line_p {
                (self.window(w).pixel_width - self.window_right_divider_width(w)) as i32
            } else {
                self.window(w).pixel_width as i32
            };
            r.height = if self.window(w).flags.pseudo_window_p {
                visible_height
            } else {
                s.height
            };
        } else {
            r.x = self.window_box_left(w, glyph_row_area(s.area)) as i32;
            r.width = self.window_box_width(w, glyph_row_area(s.area)) as i32;
            r.height = visible_height;
        }
        if let Some(ch) = s.clip_head
            && r.x < arena[ch].x
        {
            if r.width >= arena[ch].x - r.x {
                r.width -= arena[ch].x - r.x;
            } else {
                r.width = 0;
            }
            r.x = arena[ch].x;
        }
        if let Some(ct) = s.clip_tail
            && r.x + r.width > arena[ct].x + arena[ct].background_width
        {
            if arena[ct].x + arena[ct].background_width >= r.x {
                r.width = arena[ct].x + arena[ct].background_width - r.x;
            } else {
                r.width = 0;
            }
        }
        if s.for_overlaps != 0 {
            r.y = self.top_lines_height(w) as i32;
            r.height = self.window_text_bottom_y(w) as i32 - r.y;
            if s.for_overlaps & OVERLAPS_ERASED_CURSOR != 0 {
                let r_save = r;
                let win = self.window(w);
                let rc = EmacsRectangle {
                    x: self.window_text_to_frame_pixel_x(w, win.phys_cursor.x) as i32,
                    y: win.phys_cursor.y as i32,
                    width: win.phys_cursor_width as i32,
                    height: win.phys_cursor_height as i32,
                };
                if let Some(i) = gui_intersect_rectangles(&r_save, &rc) {
                    r = i;
                }
            }
        } else if !full_width_p && self.row_partially_visible_at_top_p(w, s.row) {
            r.y = self.top_lines_height(w) as i32;
        } else {
            r.y = row_y.max(0);
        }
        r.y = self.window_to_frame_pixel_y(w, i64::from(r.y)) as i32;
        if s.hl == DrawGlyphsFace::Cursor {
            let glyph = self.row_glyph(s.row, s.area, s.first_glyph);
            if s.x > r.x {
                if r.width >= s.x - r.x {
                    r.width -= s.x - r.x;
                } else {
                    r.width = 0;
                }
                r.x = s.x;
            }
            r.width = r.width.min(i32::from(glyph.pixel_width));
            let line_height = self.frame(s.f).geometry.line_height as i32;
            let glyph_height = i32::from(glyph.ascent) + i32::from(glyph.descent);
            let height = glyph_height.min(line_height.min(visible_height));
            let max_y = self.window_text_bottom_y(w) as i32 - height;
            let max_y = self.window_to_frame_pixel_y(w, i64::from(max_y)) as i32;
            if s.ybase - i32::from(glyph.ascent) > max_y {
                r.y = max_y;
                r.height = height;
            } else {
                let height = line_height.max(glyph_height);
                if height < r.height {
                    let max_y = r.y + r.height;
                    r.y = max_y.min(r.y.max(s.ybase + i32::from(glyph.descent) - height));
                    r.height = (max_y - r.y).min(height);
                }
            }
        }
        if let Some(clip) = row_clip {
            match gui_intersect_rectangles(&r, &clip) {
                Some(i) => r = i,
                None => r.width = 0,
            }
        }
        if (s.for_overlaps & OVERLAPS_BOTH) == 0
            || ((s.for_overlaps & OVERLAPS_BOTH) == OVERLAPS_BOTH && n == 1)
        {
            return vec![r];
        }
        let row_y = self.window_to_frame_pixel_y(w, i64::from(row_y)) as i32;
        let mut rs = Vec::new();
        if s.for_overlaps & OVERLAPS_PRED != 0 {
            let mut p = r;
            if r.y + r.height > row_y {
                p.height = if r.y < row_y { row_y - r.y } else { 0 };
            }
            rs.push(p);
        }
        if s.for_overlaps & OVERLAPS_SUCC != 0 {
            let mut p = r;
            if r.y < row_y + visible_height {
                if r.y + r.height > row_y + visible_height {
                    p.y = row_y + visible_height;
                    p.height = r.y + r.height - p.y;
                } else {
                    p.height = 0;
                }
            }
            rs.push(p);
        }
        rs
    }

    pub fn get_glyph_string_clip_rect(
        &mut self,
        arena: &[GlyphString],
        si: usize,
    ) -> EmacsRectangle {
        self.get_glyph_string_clip_rects(arena, si, 1)
            .first()
            .copied()
            .unwrap_or_default()
    }

    fn row_partially_visible_at_top_p(&mut self, w: Value, row: GlyphRowRef) -> bool {
        let (y, height, visible_height) = {
            let r = self.glyph_row(row);
            (r.y, r.height, r.visible_height)
        };
        height != visible_height && i64::from(y) < self.top_lines_height(w)
    }

    pub fn get_phys_cursor_geometry(
        &mut self,
        w: Value,
        row: GlyphRowRef,
        glyph: &Glyph,
    ) -> (i32, i32, i32) {
        let f = self.window(w).frame;
        let mut wd = i32::from(glyph.pixel_width);
        let mut x = self.window(w).phys_cursor.x as i32;
        if x < 0 {
            wd += x;
            x = 0;
        }
        if glyph.kind == GlyphType::Stretch && !self.var_true(crate::symbols::X_STRETCH_CURSOR) {
            wd = wd.min(self.frame(f).geometry.column_width as i32);
        }
        self.window_mut(w).phys_cursor_width = i64::from(wd);
        let (row_ascent, ends_at_zv_p, visible_height, row_height) = {
            let r = self.glyph_row(row);
            (r.ascent, r.ends_at_zv_p, r.visible_height, r.height)
        };
        let mut y = self.window(w).phys_cursor.y as i32;
        let mut ascent = row_ascent;
        if !ends_at_zv_p && row_ascent < i32::from(glyph.ascent) {
            y -= i32::from(glyph.ascent) - row_ascent;
            ascent = i32::from(glyph.ascent);
        }
        let line_height = self.frame(f).geometry.line_height as i32;
        let mut h0 = line_height.min(visible_height);
        let mut h = h0.max(ascent + i32::from(glyph.descent));
        h = h.min(row_height);
        h0 = h0.min(ascent + i32::from(glyph.descent));
        let mut y0 = self.top_lines_height(w) as i32;
        if y < y0 {
            h = (h - (y0 - y) + 1).max(h0);
            y = y0 - 1;
        } else {
            y0 = self.window_text_bottom_y(w) as i32 - h0;
            if y > y0 {
                h += y - y0;
                y = y0;
            }
        }
        (
            self.window_text_to_frame_pixel_x(w, i64::from(x)) as i32,
            self.window_to_frame_pixel_y(w, i64::from(y)) as i32,
            h,
        )
    }

    pub fn notice_overwritten_cursor(
        &mut self,
        w: Value,
        area: usize,
        x0: i32,
        x1: i32,
        y0: i32,
        y1: i32,
    ) -> LispResult<()> {
        if !self.window(w).flags.phys_cursor_on_p || area != TEXT_AREA {
            return Ok(());
        }
        let vpos = self.window(w).phys_cursor.vpos;
        let Some(matrix) = self.window_matrix(w, WhichMatrix::Current) else {
            return Ok(());
        };
        if vpos < 0 || vpos >= i64::from(matrix.nrows) {
            return Ok(());
        }
        let row = &matrix.rows[vpos as usize];
        if !(row.enabled_p && row.displays_text_p) {
            return Ok(());
        }
        let (cursor_in_fringe_p, reversed_p) = (row.cursor_in_fringe_p, row.reversed_p);
        if cursor_in_fringe_p {
            let r = GlyphRowRef::Window {
                window: w,
                desired: false,
                vpos: vpos as usize,
            };
            self.glyph_row_mut(r).cursor_in_fringe_p = false;
            self.draw_fringe_bitmap(w, r, reversed_p)?;
            self.window_mut(w).flags.phys_cursor_on_p = false;
            return Ok(());
        }
        let win = self.window(w);
        let cx0 = win.phys_cursor.x as i32;
        let cx1 = cx0 + win.phys_cursor_width as i32;
        if x0 > cx0 || (x1 >= 0 && x1 < cx1) {
            return Ok(());
        }
        let cy0 = win.phys_cursor.y as i32;
        let cy1 = cy0 + win.phys_cursor_height as i32;
        if (y0 < cy0 || y0 >= cy1) && (y1 <= cy0 || y1 >= cy1) {
            return Ok(());
        }
        self.window_mut(w).flags.phys_cursor_on_p = false;
        Ok(())
    }

    pub fn coords_in_mouse_face_p(&self, w: Value, hpos: i64, vpos: i64) -> bool {
        let f = self.window(w).frame;
        let hlinfo = self.mouse_hl_info(f);
        if !(self.is_window(hlinfo.mouse_face_window) && hlinfo.mouse_face_window == w) {
            return false;
        }
        let (beg_row, end_row) = (
            i64::from(hlinfo.mouse_face_beg_row),
            i64::from(hlinfo.mouse_face_end_row),
        );
        let (beg_col, end_col) = (
            i64::from(hlinfo.mouse_face_beg_col),
            i64::from(hlinfo.mouse_face_end_col),
        );
        if vpos < beg_row || vpos > end_row {
            return false;
        }
        if vpos > beg_row && vpos < end_row {
            return true;
        }
        let reversed_p = self
            .window_matrix(w, WhichMatrix::Current)
            .map(|m| m.rows[vpos as usize].reversed_p)
            .unwrap_or(false);
        if !reversed_p {
            if beg_row == end_row {
                if beg_col <= hpos && hpos < end_col {
                    return true;
                }
            } else if (vpos == beg_row && hpos >= beg_col) || (vpos == end_row && hpos < end_col) {
                return true;
            }
        } else if beg_row == end_row {
            if end_col < hpos && hpos <= beg_col {
                return true;
            }
        } else if (vpos == beg_row && hpos <= beg_col) || (vpos == end_row && hpos > end_col) {
            return true;
        }
        false
    }

    pub fn cursor_in_mouse_face_p(&self, w: Value) -> bool {
        let vpos = self.window(w).phys_cursor.vpos;
        let Some(matrix) = self.window_matrix(w, WhichMatrix::Current) else {
            return false;
        };
        if !(0 <= vpos && vpos < i64::from(matrix.nrows)) {
            return false;
        }
        let mut hpos = self.window(w).phys_cursor.hpos;
        let row = &matrix.rows[vpos as usize];
        if !row.reversed_p && hpos < 0 {
            hpos = 0;
        }
        if row.reversed_p && hpos >= i64::from(row.used[TEXT_AREA]) {
            hpos = i64::from(row.used[TEXT_AREA]) - 1;
        }
        self.coords_in_mouse_face_p(w, hpos, vpos)
    }

    pub fn get_phys_cursor_glyph(&self, w: Value) -> Option<Glyph> {
        let win = self.window(w);
        let matrix = win.current_matrix.as_deref()?;
        let vpos = win.phys_cursor.vpos;
        if !(vpos >= 0 && vpos < i64::from(matrix.nrows)) {
            return None;
        }
        let row = &matrix.rows[vpos as usize];
        if !row.enabled_p {
            return None;
        }
        let mut hpos = win.phys_cursor.hpos;
        if win.hscroll != 0 {
            if !row.reversed_p && hpos < 0 {
                hpos = 0;
            }
            if row.reversed_p && hpos >= i64::from(row.used[TEXT_AREA]) {
                hpos = i64::from(row.used[TEXT_AREA]) - 1;
            }
        }
        (0 <= hpos && hpos < i64::from(row.used[TEXT_AREA]))
            .then(|| row.area_glyphs(TEXT_AREA)[hpos as usize])
    }
}

pub fn gui_get_glyph_overhangs(lisp: &mut Lisp, glyph: &Glyph, f: Value) -> (i32, i32) {
    let mut left = 0;
    let mut right = 0;
    let (face, char2b) = lisp.get_glyph_face_and_encoding(f, glyph);
    let (font, overstrike) = {
        let face = lisp.face_from_id(f, face).expect("realized face");
        (face.font, face.overstrike)
    };
    if glyph.kind == GlyphType::Char {
        if !font.is_nil()
            && let Some(mut pcm) = lisp.get_per_char_metric(font, char2b.unwrap_or(0))
        {
            if overstrike {
                pcm.rbearing += 1;
            }
            if pcm.rbearing > pcm.width {
                right = pcm.rbearing - pcm.width;
            }
            if pcm.lbearing < 0 {
                left = -pcm.lbearing;
            }
        }
    } else if glyph.kind == GlyphType::Composite {
        let (automatic, id) = cmp_union(glyph.u);
        if !automatic {
            let cmp = &lisp.compositions.table[id as usize];
            let mut rbearing = cmp.rbearing;
            if overstrike {
                rbearing += 1;
            }
            if rbearing > cmp.pixel_width {
                right = cmp.rbearing - cmp.pixel_width;
            }
            if cmp.lbearing < 0 {
                left = -cmp.lbearing;
            }
        } else {
            let gstring = lisp.composition_gstring_from_id(id as usize);
            let (from, to) = slice_cmp(glyph.slice);
            let mut metrics = FontMetrics::default();
            lisp.composition_gstring_width(
                gstring,
                from as usize,
                (to + 1) as usize,
                Some(&mut metrics),
            );
            if overstrike {
                metrics.rbearing += 1;
            }
            if metrics.rbearing > metrics.width {
                right = metrics.rbearing - metrics.width;
            }
            if metrics.lbearing < 0 {
                left = -metrics.lbearing;
            }
        }
    }
    (left, right)
}

pub fn gui_write_glyphs(
    lisp: &mut Lisp,
    w: Value,
    updated_row: GlyphRowRef,
    updated_area: usize,
    start: usize,
    len: usize,
) -> LispResult<()> {
    let mut chpos = lisp.window(w).phys_cursor.hpos;
    let (reversed_p, used_text) = {
        let r = lisp.glyph_row(updated_row);
        (r.reversed_p, i64::from(r.used[TEXT_AREA]))
    };
    if !reversed_p && chpos < 0 {
        chpos = 0;
    }
    if reversed_p && chpos >= used_text {
        chpos = used_text - 1;
    }
    let hpos = start as i64;
    let output_x = lisp.window(w).output_cursor.x as i32;
    let x = lisp.draw_glyphs(
        w,
        output_x,
        updated_row,
        updated_area,
        hpos,
        hpos + len as i64,
        DrawGlyphsFace::NormalText,
        0,
    )?;
    let win = lisp.window_mut(w);
    if updated_area == TEXT_AREA
        && win.flags.phys_cursor_on_p
        && win.phys_cursor.vpos == win.output_cursor.vpos
        && chpos >= hpos
        && chpos < hpos + len as i64
    {
        win.flags.phys_cursor_on_p = false;
    }
    win.output_cursor.hpos += len as i64;
    win.output_cursor.x = i64::from(x);
    Ok(())
}

pub fn gui_insert_glyphs(
    lisp: &mut Lisp,
    w: Value,
    updated_row: GlyphRowRef,
    updated_area: usize,
    start: usize,
    len: usize,
) -> LispResult<()> {
    let f = lisp.window(w).frame;
    let (line_height, shift_by_width) = {
        let r = lisp.glyph_row(updated_row);
        let glyphs = &r.glyphs[r.area[updated_area] + start..];
        (
            r.height,
            glyphs[..len]
                .iter()
                .map(|g| i32::from(g.pixel_width))
                .sum::<i32>(),
        )
    };
    let area = glyph_row_area(updated_area);
    let output = lisp.window(w).output_cursor;
    let shifted_region_width =
        lisp.window_box_width(w, area) as i32 - output.x as i32 - shift_by_width;
    let frame_x = lisp.window_box_left(w, area) as i32 + output.x as i32;
    let frame_y = lisp.window_to_frame_pixel_y(w, output.y) as i32;
    let shift = lisp
        .frame_rif(f)
        .shift_glyphs_for_insert
        .expect("redisplay interface shift_glyphs_for_insert");
    shift(
        lisp,
        f,
        frame_x,
        frame_y,
        shifted_region_width,
        line_height,
        shift_by_width,
    );
    let hpos = start as i64;
    lisp.draw_glyphs(
        w,
        output.x as i32,
        updated_row,
        updated_area,
        hpos,
        hpos + len as i64,
        DrawGlyphsFace::NormalText,
        0,
    )?;
    let win = lisp.window_mut(w);
    win.output_cursor.hpos += len as i64;
    win.output_cursor.x += i64::from(shift_by_width);
    Ok(())
}

pub fn gui_clear_end_of_line(
    lisp: &mut Lisp,
    w: Value,
    updated_row: GlyphRowRef,
    updated_area: usize,
    to_x: i32,
) -> LispResult<()> {
    let f = lisp.window(w).frame;
    let face_stipple = lisp
        .face_from_id(f, DEFAULT_FACE_ID)
        .map(|face| face.stipple);
    let (full_width_p, mode_line_p, row_height, row_y, row_bottom) = {
        let r = lisp.glyph_row(updated_row);
        (r.full_width_p, r.mode_line_p, r.height, r.y, r.bottom_y())
    };
    let area = glyph_row_area(updated_area);
    let max_x = if full_width_p {
        let divider = if mode_line_p {
            lisp.window_right_divider_width(w)
        } else {
            0
        };
        (lisp.window(w).pixel_width - divider) as i32
    } else {
        lisp.window_box_width(w, area) as i32
    };
    let max_y = lisp.window_text_bottom_y(w) as i32;
    let mut to_x = match to_x {
        0 => return Ok(()),
        x if x < 0 => max_x,
        x => x.min(max_x),
    };
    let output = lisp.window(w).output_cursor;
    let to_y = max_y.min(output.y as i32 + row_height);
    if !full_width_p {
        lisp.notice_overwritten_cursor(w, updated_area, output.x as i32, -1, row_y, row_bottom)?;
    }
    let mut from_x = output.x as i32;
    if full_width_p {
        from_x = lisp.window_to_frame_pixel_x(w, i64::from(from_x)) as i32;
        to_x = lisp.window_to_frame_pixel_x(w, i64::from(to_x)) as i32;
    } else {
        let area_left = lisp.window_box_left(w, area) as i32;
        from_x += area_left;
        to_x += area_left;
    }
    let min_y = lisp.top_lines_height(w) as i32;
    let from_y = lisp.window_to_frame_pixel_y(w, i64::from(min_y.max(output.y as i32))) as i32;
    let to_y = lisp.window_to_frame_pixel_y(w, i64::from(to_y)) as i32;
    if to_x > from_x && to_y > from_y {
        let clear = lisp
            .frame_rif(f)
            .clear_frame_area
            .expect("redisplay interface clear_frame_area");
        clear(lisp, f, from_x, from_y, to_x - from_x, to_y - from_y);
        if let Some(stipple) = face_stipple {
            let row = lisp.glyph_row_mut(updated_row);
            if !row.stipple_p {
                row.stipple_p = stipple != 0;
            }
        }
    }
    Ok(())
}

pub fn gui_fix_overlapping_area(
    lisp: &mut Lisp,
    w: Value,
    row: GlyphRowRef,
    area: usize,
    overlaps: u8,
) -> LispResult<()> {
    let (mut x, used) = {
        let r = lisp.glyph_row(row);
        (r.x, r.used[area] as usize)
    };
    let mut i = 0;
    while i < used {
        let g = lisp.row_glyph(row, area, i);
        if g.overlaps_vertically_p {
            let start = i;
            let start_x = x;
            loop {
                x += i32::from(lisp.row_glyph(row, area, i).pixel_width);
                i += 1;
                if !(i < used && lisp.row_glyph(row, area, i).overlaps_vertically_p) {
                    break;
                }
            }
            lisp.draw_glyphs(
                w,
                start_x,
                row,
                area,
                start as i64,
                i as i64,
                DrawGlyphsFace::NormalText,
                overlaps,
            )?;
        } else {
            x += i32::from(g.pixel_width);
            i += 1;
        }
    }
    Ok(())
}
