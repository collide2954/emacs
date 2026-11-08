use crate::dispextern::{
    Glyph, GlyphRow, GlyphRowRef, GlyphSlice, GlyphType, GlyphU, GlyphlessMethod, It, TEXT_AREA,
};
use crate::dispnew::matrix::WhichMatrix;
use crate::lisp::Lisp;
use crate::value::Value;

pub const MAX_SCRATCH_GLYPHS: usize = 100;

pub fn scratch_glyph_row() -> GlyphRow {
    GlyphRow {
        glyphs: vec![Glyph::default(); MAX_SCRATCH_GLYPHS],
        area: [0, 0, MAX_SCRATCH_GLYPHS, MAX_SCRATCH_GLYPHS],
        ..GlyphRow::default()
    }
}

fn glyphless_method_code(method: GlyphlessMethod) -> u8 {
    match method {
        GlyphlessMethod::ThinSpace => 0,
        GlyphlessMethod::EmptyBox => 1,
        GlyphlessMethod::Acronym => 2,
        GlyphlessMethod::HexCode => 3,
    }
}

impl GlyphRowRef {
    pub fn then_valid(self, lisp: &Lisp) -> Option<GlyphRowRef> {
        match self {
            GlyphRowRef::Window {
                window, desired, ..
            } => {
                let which = if desired {
                    WhichMatrix::Desired
                } else {
                    WhichMatrix::Current
                };
                lisp.window_matrix(window, which).map(|_| self)
            }
            GlyphRowRef::Scratch => Some(self),
        }
    }
}

impl Lisp {
    pub fn glyph_row(&self, r: GlyphRowRef) -> &GlyphRow {
        match r {
            GlyphRowRef::Window {
                window,
                desired,
                vpos,
            } => {
                let which = if desired {
                    WhichMatrix::Desired
                } else {
                    WhichMatrix::Current
                };
                &self
                    .window_matrix(window, which)
                    .expect("window has no glyph matrix")
                    .rows[vpos]
            }
            GlyphRowRef::Scratch => &self.xdisp.scratch_glyph_row,
        }
    }

    pub fn glyph_row_mut(&mut self, r: GlyphRowRef) -> &mut GlyphRow {
        match r {
            GlyphRowRef::Window {
                window,
                desired,
                vpos,
            } => {
                let which = if desired {
                    WhichMatrix::Desired
                } else {
                    WhichMatrix::Current
                };
                &mut self
                    .window_matrix_mut(window, which)
                    .expect("window has no glyph matrix")
                    .rows[vpos]
            }
            GlyphRowRef::Scratch => &mut self.xdisp.scratch_glyph_row,
        }
    }

    fn it_expand_matrix_width(&mut self, it: &It, area: usize) {
        let row = it.glyph_row.expect("glyph row");
        let has_room = {
            let r = self.glyph_row(row);
            r.area[area] < r.area[area + 1]
        };
        if !self.frame(it.f).fonts_changed && has_room {
            self.window_mut(it.window).ncols_scale_factor += 1;
            self.frame_mut(it.f).fonts_changed = true;
        }
    }

    fn append_slot(&mut self, it: &It) -> Option<usize> {
        let area = it.area.index();
        let r = self.glyph_row_mut(it.glyph_row.expect("glyph row"));
        let index = r.area[area] + r.used[area] as usize;
        if index >= r.area[area + 1] {
            return None;
        }
        if r.reversed_p && area == TEXT_AREA {
            let start = r.area[area];
            r.glyphs.copy_within(start..index, start + 1);
            return Some(start);
        }
        Some(index)
    }

    fn finish_append(&mut self, it: &It, index: usize, fill: impl FnOnce(&mut Glyph, bool)) {
        let area = it.area.index();
        let r = self.glyph_row_mut(it.glyph_row.expect("glyph row"));
        let reversed = r.reversed_p && area == TEXT_AREA;
        let glyph = &mut r.glyphs[index];
        fill(glyph, reversed);
        if reversed {
            glyph.right_box_line_p = it.start_of_box_run_p;
            glyph.left_box_line_p = it.end_of_box_run_p;
        } else {
            glyph.left_box_line_p = it.start_of_box_run_p;
            glyph.right_box_line_p = it.end_of_box_run_p;
        }
        r.used[area] += 1;
    }

    pub(super) fn append_glyph(&mut self, it: &It) {
        let Some(index) = self.append_slot(it) else {
            let area = it.area.index();
            self.it_expand_matrix_width(it, area);
            return;
        };
        self.finish_append(it, index, |glyph, _| {
            glyph.charpos = it.position.charpos;
            glyph.object = it.object;
            if it.pixel_width > 0 {
                glyph.pixel_width = it.pixel_width as i16;
                glyph.padding_p = false;
            } else {
                glyph.pixel_width = 1;
                glyph.padding_p = true;
            }
            glyph.ascent = it.ascent as i16;
            glyph.descent = it.descent as i16;
            glyph.voffset = it.voffset;
            glyph.kind = GlyphType::Char;
            glyph.avoid_cursor_p = it.avoid_cursor_p;
            glyph.multibyte_p = it.multibyte_p;
            glyph.overlaps_vertically_p =
                it.phys_ascent > it.ascent || it.phys_descent > it.descent;
            glyph.glyph_not_available_p = it.glyph_not_available_p;
            glyph.face_id = it.face_id as u32;
            glyph.u = GlyphU::Ch(it.char_to_display as u32);
            glyph.slice = GlyphSlice::Img {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            };
            glyph.font_type = 0;
            if it.bidi_p {
                glyph.resolved_level = it.bidi_it.resolved_level as u8;
                glyph.bidi_type = it.bidi_it.type_ as u8;
            } else {
                glyph.resolved_level = 0;
                glyph.bidi_type = 0;
            }
        });
    }

    pub(super) fn append_composite_glyph(&mut self, it: &It) {
        let Some(index) = self.append_slot(it) else {
            let area = it.area.index();
            self.it_expand_matrix_width(it, area);
            return;
        };
        self.finish_append(it, index, |glyph, _| {
            glyph.charpos = it.cmp_it.charpos;
            glyph.object = it.object;
            glyph.pixel_width = it.pixel_width as i16;
            glyph.ascent = it.ascent as i16;
            glyph.descent = it.descent as i16;
            glyph.voffset = it.voffset;
            glyph.kind = GlyphType::Composite;
            if it.cmp_it.ch < 0 {
                glyph.u = GlyphU::Cmp {
                    automatic: false,
                    id: it.cmp_it.id as u32,
                };
                glyph.slice = GlyphSlice::Cmp { from: 0, to: 0 };
            } else {
                glyph.u = GlyphU::Cmp {
                    automatic: true,
                    id: it.cmp_it.id as u32,
                };
                glyph.slice = GlyphSlice::Cmp {
                    from: it.cmp_it.from as i32,
                    to: it.cmp_it.to as i32 - 1,
                };
            }
            glyph.avoid_cursor_p = it.avoid_cursor_p;
            glyph.multibyte_p = it.multibyte_p;
            glyph.overlaps_vertically_p =
                it.phys_ascent > it.ascent || it.phys_descent > it.descent;
            glyph.padding_p = false;
            glyph.glyph_not_available_p = it.glyph_not_available_p;
            glyph.face_id = it.face_id as u32;
            glyph.font_type = 0;
            if it.bidi_p {
                glyph.resolved_level = it.bidi_it.resolved_level as u8;
                glyph.bidi_type = it.bidi_it.type_ as u8;
            }
        });
    }

    pub(super) fn append_stretch_glyph(
        &mut self,
        it: &It,
        object: Value,
        width: i32,
        height: i32,
        ascent: i32,
    ) {
        let Some(index) = self.append_slot(it) else {
            let area = it.area.index();
            self.it_expand_matrix_width(it, area);
            return;
        };
        let reversed = {
            let r = self.glyph_row(it.glyph_row.expect("glyph row"));
            r.reversed_p && it.area.index() == TEXT_AREA
        };
        let mut width = width;
        if reversed && it.current_x < it.first_visible_x {
            width -= it.first_visible_x - it.current_x;
        }
        self.finish_append(it, index, |glyph, _| {
            glyph.charpos = it.position.charpos;
            glyph.object = object;
            glyph.pixel_width = width.clamp(-1, i32::from(i16::MAX)) as i16;
            glyph.ascent = ascent as i16;
            glyph.descent = (height - ascent) as i16;
            glyph.voffset = it.voffset;
            glyph.kind = GlyphType::Stretch;
            glyph.avoid_cursor_p = it.avoid_cursor_p;
            glyph.multibyte_p = it.multibyte_p;
            glyph.overlaps_vertically_p = false;
            glyph.padding_p = false;
            glyph.glyph_not_available_p = false;
            glyph.face_id = it.face_id as u32;
            glyph.u = GlyphU::Stretch {
                height: height as u16,
                ascent: ascent as u16,
            };
            glyph.slice = GlyphSlice::Img {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            };
            glyph.font_type = 0;
            if it.bidi_p {
                glyph.resolved_level = it.bidi_it.resolved_level as u8;
                glyph.bidi_type = it.bidi_it.type_ as u8;
            } else {
                glyph.resolved_level = 0;
                glyph.bidi_type = 0;
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn append_glyphless_glyph(
        &mut self,
        it: &It,
        face_id: i32,
        for_no_font: bool,
        len: i32,
        offsets: (i16, i16, i16, i16),
    ) {
        let Some(index) = self.append_slot(it) else {
            let area = it.area.index();
            self.it_expand_matrix_width(it, area);
            return;
        };
        self.finish_append(it, index, |glyph, _| {
            glyph.charpos = it.position.charpos;
            glyph.object = it.object;
            glyph.pixel_width = it.pixel_width as i16;
            glyph.ascent = it.ascent as i16;
            glyph.descent = it.descent as i16;
            glyph.voffset = it.voffset;
            glyph.kind = GlyphType::Glyphless;
            glyph.u = GlyphU::Glyphless {
                method: glyphless_method_code(it.glyphless_method),
                for_no_font,
                len: len as u8,
                ch: it.c as u32,
            };
            glyph.slice = GlyphSlice::Glyphless {
                upper_xoff: offsets.0,
                upper_yoff: offsets.1,
                lower_xoff: offsets.2,
                lower_yoff: offsets.3,
            };
            glyph.avoid_cursor_p = it.avoid_cursor_p;
            glyph.multibyte_p = it.multibyte_p;
            glyph.overlaps_vertically_p =
                it.phys_ascent > it.ascent || it.phys_descent > it.descent;
            glyph.padding_p = false;
            glyph.glyph_not_available_p = false;
            glyph.face_id = face_id as u32;
            glyph.font_type = 0;
            if it.bidi_p {
                glyph.resolved_level = it.bidi_it.resolved_level as u8;
                glyph.bidi_type = it.bidi_it.type_ as u8;
            }
        });
    }
}
