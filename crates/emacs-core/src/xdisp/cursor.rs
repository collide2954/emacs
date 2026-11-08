use crate::dispextern::{
    DrawGlyphsFace, EmacsCursorKind, GlyphRowArea, GlyphRowRef, GlyphType, OVERLAPS_ERASED_CURSOR,
    TEXT_AREA, TextCursorKind,
};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

fn current_row(w: Value, vpos: usize) -> GlyphRowRef {
    GlyphRowRef::Window {
        window: w,
        desired: false,
        vpos,
    }
}

impl Lisp {
    pub fn get_specified_cursor_type(&self, arg: Value, width: &mut i32) -> TextCursorKind {
        if arg.is_nil() {
            return TextCursorKind::NoCursor;
        }
        if arg == sym::BOX {
            return TextCursorKind::FilledBox;
        }
        if arg == sym::HOLLOW {
            return TextCursorKind::HollowBox;
        }
        if arg == sym::BAR {
            *width = 2;
            return TextCursorKind::Bar;
        }
        if arg == sym::HBAR {
            *width = 2;
            return TextCursorKind::Hbar;
        }
        if arg.is_cons()
            && let Some(n) = self.xcdr(arg).as_fixnum()
            && (0..=i64::from(i32::MAX)).contains(&n)
        {
            *width = n as i32;
            let car = self.xcar(arg);
            if car == sym::BOX {
                return TextCursorKind::FilledBox;
            } else if car == sym::BAR {
                return TextCursorKind::Bar;
            } else if car == sym::HBAR {
                return TextCursorKind::Hbar;
            }
        }
        TextCursorKind::HollowBox
    }

    pub fn set_frame_cursor_types(&mut self, f: Value, arg: Value) -> LispResult<()> {
        let mut width = 1;
        let desired = self.get_specified_cursor_type(arg, &mut width);
        {
            let fr = self.frame_mut(f);
            fr.desired_cursor = desired;
            fr.cursor_width = width;
        }
        let alist = self.find_symbol_value(sym::BLINK_CURSOR_ALIST);
        let tem = crate::fns::f_assoc(self, &[arg, alist, Value::NIL])?;
        if !tem.is_nil() {
            let off = self.get_specified_cursor_type(self.xcdr(tem), &mut width);
            let fr = self.frame_mut(f);
            fr.blink_off_cursor = off;
            fr.blink_off_cursor_width = width;
        } else {
            self.frame_mut(f).blink_off_cursor = TextCursorKind::Default;
        }
        self.frame_mut(f).flags.cursor_type_changed = true;
        Ok(())
    }

    fn get_window_cursor_type(
        &mut self,
        w: Value,
        glyph: Option<GlyphType>,
        width: &mut i32,
        active_cursor: &mut bool,
    ) -> LispResult<TextCursorKind> {
        let f = self.window(w).frame;
        let b = self.window(w).contents;
        let mut cursor_type;
        let mut non_selected = false;
        *active_cursor = true;
        let echo_area_window = self.xdisp.echo_area_window;
        let buffer_cursor_type = self.buffer_local_value(sym::CURSOR_TYPE, b);
        if self.var_true(sym::CURSOR_IN_ECHO_AREA)
            && self.frame_has_minibuf_p(f)
            && self.frame(f).minibuffer_window == echo_area_window
        {
            if w == echo_area_window {
                let wct = self.window(w).cursor_type;
                if wct != sym::T {
                    return Ok(self.get_specified_cursor_type(wct, width));
                } else if buffer_cursor_type == sym::T || buffer_cursor_type.is_nil() {
                    *width = self.frame(f).cursor_width;
                    return Ok(self.frame(f).desired_cursor);
                } else {
                    return Ok(self.get_specified_cursor_type(buffer_cursor_type, width));
                }
            }
            *active_cursor = false;
            non_selected = true;
        } else if w != self.frame(f).selected_window
            || f != self.display_info(self.frame(f).terminal).highlight_frame
        {
            *active_cursor = false;
            if self.window(w).mini
                && (self.minibuf.minibuf_level == 0
                    || self.is_minibuffer(0, self.window(w).contents))
            {
                return Ok(TextCursorKind::NoCursor);
            }
            non_selected = true;
        }
        let wct = self.window(w).cursor_type;
        if wct != sym::T {
            cursor_type = self.get_specified_cursor_type(wct, width);
        } else {
            if buffer_cursor_type.is_nil() {
                return Ok(TextCursorKind::NoCursor);
            }
            if buffer_cursor_type == sym::T {
                cursor_type = self.frame(f).desired_cursor;
                *width = self.frame(f).cursor_width;
            } else {
                cursor_type = self.get_specified_cursor_type(buffer_cursor_type, width);
            }
        }
        if non_selected {
            let alt_cursor = self.buffer_local_value(sym::CURSOR_IN_NON_SELECTED_WINDOWS, b);
            if alt_cursor != sym::T {
                return Ok(self.get_specified_cursor_type(alt_cursor, width));
            }
            if cursor_type == TextCursorKind::FilledBox {
                cursor_type = TextCursorKind::HollowBox;
            } else if cursor_type == TextCursorKind::Bar && *width > 1 {
                *width -= 1;
            }
            return Ok(cursor_type);
        }
        if !self.window(w).flags.cursor_off_p {
            if glyph == Some(GlyphType::Xwidget) {
                return Ok(TextCursorKind::NoCursor);
            }
            if glyph == Some(GlyphType::Image) {
                unreachable!("image glyphs need image.c");
            }
            return Ok(cursor_type);
        }
        let alist = self.find_symbol_value(sym::BLINK_CURSOR_ALIST);
        let alt_cursor = crate::fns::f_assoc(self, &[buffer_cursor_type, alist, Value::NIL])?;
        if !alt_cursor.is_nil() {
            let cdr = self.xcdr(alt_cursor);
            return Ok(self.get_specified_cursor_type(cdr, width));
        }
        if self.frame(f).blink_off_cursor != TextCursorKind::Default {
            *width = self.frame(f).blink_off_cursor_width;
            return Ok(self.frame(f).blink_off_cursor);
        }
        Ok(TextCursorKind::NoCursor)
    }

    pub fn draw_phys_cursor_glyph(
        &mut self,
        w: Value,
        row: GlyphRowRef,
        hl: DrawGlyphsFace,
    ) -> LispResult<()> {
        let (reversed_p, used, overlapped_p, bottom, vpos) = {
            let r = self.glyph_row(row);
            let vpos = match row {
                GlyphRowRef::Window { vpos, .. } => vpos,
                GlyphRowRef::Scratch => 0,
            };
            (
                r.reversed_p,
                i64::from(r.used[TEXT_AREA]),
                r.overlapped_p,
                r.bottom_y(),
                vpos,
            )
        };
        let phys = self.window(w).phys_cursor;
        if !(if reversed_p {
            phys.hpos >= 0
        } else {
            phys.hpos < used
        }) {
            return Ok(());
        }
        let on_p = self.window(w).flags.phys_cursor_on_p;
        let mut hpos = phys.hpos;
        if !reversed_p && hpos < 0 {
            hpos = 0;
        }
        if reversed_p && hpos >= used {
            hpos = used - 1;
        }
        let x1 = self.draw_glyphs(w, phys.x as i32, row, TEXT_AREA, hpos, hpos + 1, hl, 0)?;
        self.window_mut(w).flags.phys_cursor_on_p = on_p;
        if hl == DrawGlyphsFace::Cursor {
            self.window_mut(w).phys_cursor_width = i64::from(x1) - phys.x;
        } else if hl == DrawGlyphsFace::NormalText && overlapped_p {
            self.window_mut(w).phys_cursor_width = i64::from(x1) - phys.x;
            let fix = self
                .frame_rif(self.window(w).frame)
                .fix_overlapping_area
                .expect("redisplay interface fix_overlapping_area");
            if vpos > 0 {
                let prev = self.glyph_row(current_row(w, vpos - 1));
                if prev.phys_height - prev.phys_ascent > prev.height - prev.ascent {
                    fix(
                        self,
                        w,
                        current_row(w, vpos - 1),
                        TEXT_AREA,
                        OVERLAPS_ERASED_CURSOR,
                    )?;
                }
            }
            if i64::from(bottom) < self.window_text_bottom_y(w) {
                let next = self.glyph_row(current_row(w, vpos + 1));
                if next.phys_ascent > next.ascent {
                    fix(
                        self,
                        w,
                        current_row(w, vpos + 1),
                        TEXT_AREA,
                        OVERLAPS_ERASED_CURSOR,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn erase_phys_cursor(&mut self, w: Value) -> LispResult<()> {
        let f = self.window(w).frame;
        let phys = self.window(w).phys_cursor;
        let mut hpos = phys.hpos;
        let vpos = phys.vpos;
        'mark: {
            if self.window(w).phys_cursor_type == TextCursorKind::NoCursor {
                break 'mark;
            }
            let nrows = self
                .window_matrix(w, WhichMatrix::Current)
                .map_or(0, |m| i64::from(m.nrows));
            if vpos >= nrows {
                break 'mark;
            }
            let cursor_row = current_row(w, vpos as usize);
            if !self.glyph_row(cursor_row).enabled_p {
                break 'mark;
            }
            let bottom = self.window_text_bottom_y(w) as i32;
            {
                let r = self.glyph_row_mut(cursor_row);
                r.visible_height = r.visible_height.min(bottom - r.y);
            }
            let (visible_height, cursor_in_fringe_p, reversed_p, used, row_y) = {
                let r = self.glyph_row(cursor_row);
                (
                    r.visible_height,
                    r.cursor_in_fringe_p,
                    r.reversed_p,
                    i64::from(r.used[TEXT_AREA]),
                    r.y,
                )
            };
            if visible_height <= 0 {
                break 'mark;
            }
            if cursor_in_fringe_p {
                self.glyph_row_mut(cursor_row).cursor_in_fringe_p = false;
                self.draw_fringe_bitmap(w, cursor_row, reversed_p)?;
                break 'mark;
            }
            if if reversed_p {
                phys.hpos < 0
            } else {
                phys.hpos >= used
            } {
                break 'mark;
            }
            if !reversed_p && hpos < 0 {
                hpos = 0;
            }
            if reversed_p && hpos >= used {
                hpos = used - 1;
            }
            let mouse_face_here_p = !self.mouse_hl_info(f).mouse_face_window.is_nil()
                && self.coords_in_mouse_face_p(w, hpos, vpos)
                && used > hpos
                && hpos >= 0;
            if self.frame_window_p(f) && mouse_face_here_p {
                let win = self.window_mut(w);
                win.flags.phys_cursor_on_p = false;
                win.phys_cursor_type = TextCursorKind::NoCursor;
                self.show_mouse_face(f, DrawGlyphsFace::MouseFace, false)?;
                return Ok(());
            }
            if self.window(w).phys_cursor_type == TextCursorKind::HollowBox {
                let Some(cursor_glyph) = self.get_phys_cursor_glyph(w) else {
                    break 'mark;
                };
                let tab_line_height = self.window_tab_line_height(w) as i32;
                let header_line_height = self.window_header_line_height(w) as i32;
                let mut width = i32::from(cursor_glyph.pixel_width);
                let mut x = phys.x as i32;
                if x < 0 {
                    width += x;
                    x = 0;
                }
                width = width.min(self.window_box_width(w, GlyphRowArea::Text) as i32 - x);
                let y = self.window_to_frame_pixel_y(
                    w,
                    i64::from(tab_line_height.max(header_line_height.max(row_y))),
                ) as i32;
                let x = self.window_text_to_frame_pixel_x(w, i64::from(x)) as i32;
                if width > 0 {
                    let clear = self
                        .frame_rif(f)
                        .clear_frame_area
                        .expect("redisplay interface clear_frame_area");
                    clear(self, f, x, y, width, visible_height);
                }
            }
            let hl = if mouse_face_here_p {
                DrawGlyphsFace::MouseFace
            } else {
                DrawGlyphsFace::NormalText
            };
            self.draw_phys_cursor_glyph(w, cursor_row, hl)?;
        }
        let win = self.window_mut(w);
        win.flags.phys_cursor_on_p = false;
        win.phys_cursor_type = TextCursorKind::NoCursor;
        Ok(())
    }

    pub fn display_and_set_cursor(
        &mut self,
        w: Value,
        on: bool,
        hpos: i64,
        vpos: i64,
        x: i64,
        y: i64,
    ) -> LispResult<()> {
        let f = self.window(w).frame;
        let Some((nrows, matrix_w)) = self
            .window_matrix(w, WhichMatrix::Current)
            .map(|m| (i64::from(m.nrows), i64::from(m.matrix_w)))
        else {
            return Ok(());
        };
        if !self.frame_redisplay_p(f) || vpos >= nrows || hpos >= matrix_w {
            return Ok(());
        }
        if !on && !self.window(w).flags.phys_cursor_on_p {
            return Ok(());
        }
        let glyph_row = current_row(w, vpos as usize);
        let (enabled_p, row_y, used, row_ascent, row_height, bottom) = {
            let r = self.glyph_row(glyph_row);
            (
                r.enabled_p,
                r.y,
                i64::from(r.used[TEXT_AREA]),
                r.ascent,
                r.height,
                r.bottom_y(),
            )
        };
        if !enabled_p {
            self.window_mut(w).flags.phys_cursor_on_p = false;
            return Ok(());
        }
        if self.frame(f).flags.garbaged {
            if on {
                let win = self.window_mut(w);
                win.phys_cursor.x = x;
                win.phys_cursor.y = i64::from(row_y);
                win.phys_cursor.hpos = hpos;
                win.phys_cursor.vpos = vpos;
            }
            return Ok(());
        }
        let glyph = (0 <= hpos && hpos < used)
            .then(|| self.glyph_row(glyph_row).area_glyphs(TEXT_AREA)[hpos as usize].kind);
        let mut new_cursor_width = 0;
        let mut active_cursor = false;
        let new_cursor_type =
            self.get_window_cursor_type(w, glyph, &mut new_cursor_width, &mut active_cursor)?;
        let win = self.window(w);
        if win.flags.phys_cursor_on_p
            && (!on
                || win.phys_cursor.x != x
                || win.phys_cursor.y != y
                || hpos < 0
                || new_cursor_type != win.phys_cursor_type
                || (matches!(new_cursor_type, TextCursorKind::Bar | TextCursorKind::Hbar)
                    && i64::from(new_cursor_width) != win.phys_cursor_width))
        {
            self.erase_phys_cursor(w)?;
        }
        if on {
            let win = self.window_mut(w);
            win.phys_cursor_ascent = i64::from(row_ascent);
            win.phys_cursor_height = i64::from(row_height);
            win.phys_cursor.x = x;
            win.phys_cursor.y = i64::from(row_y);
            win.phys_cursor.hpos = hpos;
            win.phys_cursor.vpos = vpos;
        }
        if bottom > 0 {
            let draw = self
                .frame_rif(f)
                .draw_window_cursor
                .expect("redisplay interface draw_window_cursor");
            draw(
                self,
                w,
                glyph_row,
                x as i32,
                y as i32,
                new_cursor_type,
                new_cursor_width,
                on,
                active_cursor,
            )?;
        }
        Ok(())
    }

    pub fn update_window_cursor(&mut self, w: Value, on: bool) -> LispResult<()> {
        let Some((nrows, matrix_w)) = self
            .window_matrix(w, WhichMatrix::Current)
            .map(|m| (i64::from(m.nrows), i64::from(m.matrix_w)))
        else {
            return Ok(());
        };
        let phys = self.window(w).phys_cursor;
        let mut hpos = phys.hpos;
        let vpos = phys.vpos;
        if vpos >= nrows || hpos >= matrix_w {
            return Ok(());
        }
        let (reversed_p, used) = {
            let r = self.glyph_row(current_row(w, vpos as usize));
            (r.reversed_p, i64::from(r.used[TEXT_AREA]))
        };
        if !reversed_p && hpos < 0 {
            hpos = 0;
        }
        if reversed_p && hpos >= used {
            hpos = used - 1;
        }
        self.display_and_set_cursor(w, on, hpos, vpos, phys.x, phys.y)
    }

    fn update_cursor_in_window_tree(&mut self, mut w: Value, on_p: bool) -> LispResult<()> {
        while !w.is_nil() {
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                self.update_cursor_in_window_tree(contents, on_p)?;
            } else {
                self.update_window_cursor(w, on_p)?;
            }
            w = self.window(w).next;
        }
        Ok(())
    }

    pub fn gui_update_cursor(&mut self, f: Value, on_p: bool) -> LispResult<()> {
        let root = self.frame(f).root_window;
        self.update_cursor_in_window_tree(root, on_p)
    }

    pub fn gui_clear_cursor(&mut self, w: Value) -> LispResult<()> {
        let f = self.window(w).frame;
        if self.frame_redisplay_p(f) && self.window(w).flags.phys_cursor_on_p {
            self.update_window_cursor(w, false)?;
        }
        Ok(())
    }

    pub fn frame_redisplay_p(&self, f: Value) -> bool {
        self.frame(f).flags.visible
    }

    fn draw_row_with_mouse_face(
        &mut self,
        w: Value,
        start_x: i32,
        row: GlyphRowRef,
        start_hpos: i64,
        end_hpos: i64,
        draw: DrawGlyphsFace,
    ) -> LispResult<()> {
        let f = self.window(w).frame;
        if self.frame_window_p(f) {
            self.draw_glyphs(w, start_x, row, TEXT_AREA, start_hpos, end_hpos, draw, 0)?;
            return Ok(());
        }
        unreachable!("tty frames have no glyph matrices")
    }

    fn get_cursor_offset_for_mouse_face(&mut self, w: Value, row: GlyphRowRef) -> i32 {
        let (mode_line_p, reversed_p, used, vpos) = {
            let r = self.glyph_row(row);
            let vpos = match row {
                GlyphRowRef::Window { vpos, .. } => vpos as i32,
                GlyphRowRef::Scratch => -1,
            };
            (r.mode_line_p, r.reversed_p, r.used[TEXT_AREA] as i32, vpos)
        };
        if mode_line_p {
            return 0;
        }
        let f = self.window(w).frame;
        let hlinfo = *self.mouse_hl_info(f);
        let mouse_face = hlinfo.mouse_face_face_id;
        let hpos = self.window(w).phys_cursor.hpos as i32;
        let end = hpos;
        let mut start = if !reversed_p {
            if vpos == hlinfo.mouse_face_beg_row {
                hlinfo.mouse_face_beg_col
            } else {
                0
            }
        } else if vpos == hlinfo.mouse_face_end_row {
            hlinfo.mouse_face_end_col
        } else {
            used - 1
        };
        let mut sum = 0;
        while if reversed_p { start > end } else { start < end } {
            let g = self.glyph_row(row).area_glyphs(TEXT_AREA)[start as usize];
            sum += self.mouse_face_glyph_width_adjustment(&g, mouse_face, f);
            if reversed_p {
                start -= 1;
            } else {
                start += 1;
            }
        }
        if reversed_p { -sum } else { sum }
    }

    fn mouse_face_glyph_width_adjustment(
        &self,
        g: &crate::dispextern::Glyph,
        mouse_face: usize,
        f: Value,
    ) -> i32 {
        if g.kind == GlyphType::Image {
            unreachable!("image glyphs need image.c");
        }
        let original = self
            .face_from_id(f, g.face_id as usize)
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

    pub fn show_mouse_face(
        &mut self,
        hl_frame: Value,
        draw: DrawGlyphsFace,
        define_mouse_cursor: bool,
    ) -> LispResult<()> {
        let hlinfo = *self.mouse_hl_info(hl_frame);
        if !self.is_window(hlinfo.mouse_face_window) {
            return Ok(());
        }
        let w = hlinfo.mouse_face_window;
        let f = self.window(w).frame;
        if f != hlinfo.mouse_face_mouse_frame {
            return Ok(());
        }
        let nrows = self.window_matrix(w, WhichMatrix::Current).map(|m| m.nrows);
        if let Some(nrows) = nrows
            && (draw != DrawGlyphsFace::MouseFace || !hlinfo.mouse_face_hidden)
            && hlinfo.mouse_face_end_row < nrows
        {
            let phys_cursor_on_p = self.window(w).flags.phys_cursor_on_p;
            let mut mouse_off = 0;
            let first = hlinfo.mouse_face_beg_row.max(0) as usize;
            let last = hlinfo.mouse_face_end_row.max(0) as usize;
            let mut row = first;
            while row <= last && self.glyph_row(current_row(w, row)).enabled_p {
                let r_ref = current_row(w, row);
                let (reversed_p, used) = {
                    let r = self.glyph_row(r_ref);
                    (r.reversed_p, i64::from(r.used[TEXT_AREA]))
                };
                let (start_hpos, start_x) = if row == first {
                    if !reversed_p {
                        (
                            i64::from(hlinfo.mouse_face_beg_col),
                            hlinfo.mouse_face_beg_x,
                        )
                    } else if row == last {
                        (
                            i64::from(hlinfo.mouse_face_end_col),
                            hlinfo.mouse_face_end_x,
                        )
                    } else {
                        (0, 0)
                    }
                } else if reversed_p && row == last {
                    (
                        i64::from(hlinfo.mouse_face_end_col),
                        hlinfo.mouse_face_end_x,
                    )
                } else {
                    (0, 0)
                };
                let end_hpos = if row == last {
                    if !reversed_p {
                        i64::from(hlinfo.mouse_face_end_col)
                    } else if row == first {
                        i64::from(hlinfo.mouse_face_beg_col)
                    } else {
                        if draw == DrawGlyphsFace::NormalText {
                            self.glyph_row_mut(r_ref).fill_line_p = true;
                        }
                        used
                    }
                } else if reversed_p && row == first {
                    i64::from(hlinfo.mouse_face_beg_col)
                } else {
                    if draw == DrawGlyphsFace::NormalText {
                        self.glyph_row_mut(r_ref).fill_line_p = true;
                    }
                    used
                };
                if end_hpos > start_hpos {
                    self.draw_row_with_mouse_face(w, start_x, r_ref, start_hpos, end_hpos, draw)?;
                    self.glyph_row_mut(r_ref).mouse_face_p =
                        draw == DrawGlyphsFace::MouseFace || draw == DrawGlyphsFace::ImageRaised;
                }
                if row as i64 == self.window(w).phys_cursor.vpos
                    && !self.window(w).flags.pseudo_window_p
                    && draw == DrawGlyphsFace::MouseFace
                {
                    mouse_off = self.get_cursor_offset_for_mouse_face(w, r_ref);
                }
                row += 1;
            }
            if self.frame_window_p(f) && phys_cursor_on_p && !self.window(w).flags.phys_cursor_on_p
            {
                let phys = self.window(w).phys_cursor;
                let mut hpos = phys.hpos;
                let exit_row = row.min(nrows.max(1) as usize - 1);
                let (reversed_p, used) = {
                    let r = self.glyph_row(current_row(w, exit_row));
                    (r.reversed_p, i64::from(r.used[TEXT_AREA]))
                };
                if !reversed_p && hpos < 0 {
                    hpos = 0;
                }
                if reversed_p && hpos >= used {
                    hpos = used - 1;
                }
                let old_phys_cursor_x = phys.x;
                self.display_and_set_cursor(
                    w,
                    true,
                    hpos,
                    phys.vpos,
                    phys.x + i64::from(mouse_off),
                    phys.y,
                )?;
                self.window_mut(w).phys_cursor.x = old_phys_cursor_x;
            }
        }
        if self.frame_window_p(f)
            && self.find_symbol_value(sym::TRACK_MOUSE).is_nil()
            && define_mouse_cursor
        {
            let fr = self.frame(f);
            let cursor = if draw == DrawGlyphsFace::NormalText
                && hlinfo.mouse_face_window != fr.tool_bar_window
                && hlinfo.mouse_face_window != fr.tab_bar_window
            {
                EmacsCursorKind::Text
            } else if draw == DrawGlyphsFace::MouseFace {
                EmacsCursorKind::Hand
            } else {
                EmacsCursorKind::Nontext
            };
            let define = self
                .frame_rif(f)
                .define_frame_cursor
                .expect("redisplay interface define_frame_cursor");
            define(self, f, cursor);
        }
        Ok(())
    }

    pub fn clear_mouse_face(&mut self, f: Value) -> LispResult<bool> {
        let hlinfo = *self.mouse_hl_info(f);
        let cleared = !hlinfo.mouse_face_hidden && !hlinfo.mouse_face_window.is_nil();
        if cleared {
            self.show_mouse_face(f, DrawGlyphsFace::NormalText, true)?;
        }
        let h = self.mouse_hl_info_mut(f);
        h.mouse_face_beg_row = -1;
        h.mouse_face_beg_col = -1;
        h.mouse_face_end_row = -1;
        h.mouse_face_end_col = -1;
        h.mouse_face_window = Value::NIL;
        h.mouse_face_overlay = Value::NIL;
        Ok(cleared)
    }

    pub fn gui_draw_vertical_border(&mut self, w: Value) {
        let f = self.window(w).frame;
        if self.frame_has_vertical_scroll_bars(f) || self.frame(f).geometry.right_divider_width != 0
        {
            return;
        }
        let draw = self
            .frame_rif(f)
            .draw_vertical_window_border
            .expect("redisplay interface draw_vertical_window_border");
        if !self.window_rightmost_p(w) && !self.window_has_vertical_scroll_bar_on_right(w) {
            let (x0, y0, mut x1, mut y1) = self.window_box_edges(w);
            y1 -= 1;
            if self.window_left_fringe_width(w) == 0 {
                x1 -= 1;
            }
            let _ = x0;
            draw(self, w, x1 as i32, y0 as i32, y1 as i32);
        }
        if self.window(w).pixel_left != 0 && !self.window_has_vertical_scroll_bar_on_left(w) {
            let (mut x0, y0, _, mut y1) = self.window_box_edges(w);
            y1 -= 1;
            if self.window_left_fringe_width(w) == 0 {
                x0 -= 1;
            }
            draw(self, w, x0 as i32, y0 as i32, y1 as i32);
        }
    }

    fn window_box_edges(&mut self, w: Value) -> (i64, i64, i64, i64) {
        let (x, y, width, height) = self.window_box(w, GlyphRowArea::Any);
        (x, y, x + width, y + height)
    }

    pub fn gui_draw_right_divider(&mut self, w: Value) {
        if self.window(w).mini || self.window(w).flags.pseudo_window_p {
            return;
        }
        let rdw = self.window_right_divider_width(w);
        if rdw == 0 {
            return;
        }
        let f = self.window(w).frame;
        let x0 = self.window_right_edge_x(w) - rdw;
        let x1 = self.window_right_edge_x(w);
        let y0 = self.window_top_edge_y(w);
        let mut y1 = self.window_top_edge_y(w) + self.window(w).pixel_height;
        let parent = self.window(w).parent;
        if self.window_bottom_divider_width(w) != 0
            && !parent.is_nil()
            && self.window(parent).horizontal
            && !self.window(w).next.is_nil()
        {
            y1 -= self.window_bottom_divider_width(w);
        }
        let draw = self
            .frame_rif(f)
            .draw_window_divider
            .expect("redisplay interface draw_window_divider");
        draw(self, w, x0 as i32, x1 as i32, y0 as i32, y1 as i32);
    }

    pub fn gui_draw_bottom_divider(&mut self, w: Value) {
        if self.window(w).mini || self.window(w).flags.pseudo_window_p {
            return;
        }
        let bdw = self.window_bottom_divider_width(w);
        if bdw == 0 {
            return;
        }
        let f = self.window(w).frame;
        let x0 = self.window_left_edge_x(w);
        let mut x1 = self.window_right_edge_x(w);
        let bottom = self.window_top_edge_y(w) + self.window(w).pixel_height;
        let y0 = bottom - bdw;
        let y1 = bottom;
        let p = self.window(w).parent;
        if self.window_right_divider_width(w) != 0 && !p.is_nil() {
            let pp = self.window(p).parent;
            let next = self.window(w).next;
            if (!self.window(p).horizontal && !next.is_nil())
                || (self.window(p).horizontal
                    && next.is_nil()
                    && !pp.is_nil()
                    && !self.window(pp).horizontal
                    && !self.window(pp).next.is_nil())
            {
                x1 -= self.window_right_divider_width(w);
            }
        }
        let draw = self
            .frame_rif(f)
            .draw_window_divider
            .expect("redisplay interface draw_window_divider");
        draw(self, w, x0 as i32, x1 as i32, y0 as i32, y1 as i32);
    }

    pub fn gui_clear_window_mouse_face(&mut self, w: Value) -> LispResult<()> {
        let f = self.window(w).frame;
        if self.mouse_hl_info(f).mouse_face_window == w {
            self.clear_mouse_face(f)?;
        }
        Ok(())
    }
}

pub fn gui_clear_window_mouse_face(lisp: &mut Lisp, w: Value) -> LispResult<()> {
    lisp.gui_clear_window_mouse_face(w)
}
