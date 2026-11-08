use crate::bidi::{BidiDir, ShelvedBidiCache};
use crate::dispextern::{
    DisplayElementType, GlyphRowArea, It, ItMethod, LineWrap, MOVE_TO_POS, MOVE_TO_X, MOVE_TO_Y,
    MoveItResult, TextPos,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::terminal::OutputMethod;
use crate::value::Value;
use crate::xfaces::{
    HEADER_LINE_ACTIVE_FACE_ID, HEADER_LINE_INACTIVE_FACE_ID, MODE_LINE_ACTIVE_FACE_ID,
    MODE_LINE_INACTIVE_FACE_ID, TAB_LINE_ACTIVE_FACE_ID, TAB_LINE_INACTIVE_FACE_ID,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PosVisible {
    pub x: i32,
    pub y: i32,
    pub rtop: i32,
    pub rbot: i32,
    pub rowh: i32,
    pub vpos: i32,
}

impl Lisp {
    pub fn pos_visible_p(&mut self, w: Value, charpos: i64) -> LispResult<Option<PosVisible>> {
        self.with_gc_deferred(|lisp| lisp.pos_visible_p_1(w, charpos))
    }

    fn line_format(&self, w: Value, param: Value) -> Value {
        let format = self.window_parameter(w, param);
        if format.is_nil() {
            self.find_symbol_value(param)
        } else {
            format
        }
    }

    fn pos_visible_p_1(&mut self, w: Value, charpos: i64) -> LispResult<Option<PosVisible>> {
        let itdata = self.bidi_shelve_cache();
        let f = self.window(w).frame;
        if self.frame(f).output_method == OutputMethod::Initial {
            return Ok(None);
        }
        let contents = self.window(w).contents;
        let mut old_buffer = None;
        if contents != self.current_buffer {
            old_buffer = Some(self.current_buffer);
            self.set_buffer_internal(contents);
        }
        let mut top = {
            let m = self.marker(self.window(w).start);
            TextPos::new(m.charpos as i64, m.bytepos as i64)
        };
        let (begv, begv_byte, zv) = {
            let b = self.cur();
            (b.begv as i64, b.begv_byte as i64, b.zv as i64)
        };
        if top.charpos > zv || top.charpos < begv {
            top = TextPos::new(begv, begv_byte);
        }
        if charpos >= 0 && top.charpos > charpos {
            return Ok(None);
        }
        let (prev_mode_line_height, prev_header_line_height, prev_tab_line_height) = {
            let win = self.window(w);
            (
                win.mode_line_height,
                win.header_line_height,
                win.tab_line_height,
            )
        };
        if self.window_wants_mode_line(w) {
            let format = self.line_format(w, sym::MODE_LINE_FORMAT);
            let face = self.current_line_active_face_id(
                w,
                MODE_LINE_ACTIVE_FACE_ID,
                MODE_LINE_INACTIVE_FACE_ID,
            );
            let height = self.display_mode_line(w, face, format)?;
            self.window_mut(w).mode_line_height = i64::from(height);
        }
        if self.window_wants_tab_line(w) {
            let format = self.line_format(w, sym::TAB_LINE_FORMAT);
            let face = self.current_line_active_face_id(
                w,
                TAB_LINE_ACTIVE_FACE_ID,
                TAB_LINE_INACTIVE_FACE_ID,
            );
            let height = self.display_mode_line(w, face, format)?;
            self.window_mut(w).tab_line_height = i64::from(height);
        }
        if self.window_wants_header_line(w) {
            let format = self.line_format(w, sym::HEADER_LINE_FORMAT);
            let face = self.current_line_active_face_id(
                w,
                HEADER_LINE_ACTIVE_FACE_ID,
                HEADER_LINE_INACTIVE_FACE_ID,
            );
            let height = self.display_mode_line(w, face, format)?;
            self.window_mut(w).header_line_height = i64::from(height);
        }
        let mut it = It::default();
        self.start_display(&mut it, w, top)?;
        let op = if charpos >= 0 { MOVE_TO_POS } else { 0 } | MOVE_TO_Y;
        let last_visible_y = it.last_visible_y;
        self.move_it_to(&mut it, charpos, -1, last_visible_y - 1, -1, op)?;
        if !self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS).is_nil()
            && it.current_x >= it.first_visible_x
            && it.charpos() == charpos
            && !it.line_number_produced_p
        {
            if it.lnum_pixel_width == 0 {
                let mut it2data: ShelvedBidiCache = None;
                let mut it2 = self.save_it(&it, &mut it2data);
                self.move_it_by_lines(&mut it, 1)?;
                it2.lnum_pixel_width = it.lnum_pixel_width;
                self.restore_it(&mut it, Some(&it2), &mut it2data);
            }
            it.current_x += it.lnum_pixel_width;
        }
        let mut visible_p = false;
        let mut r2l = false;
        let mut out = PosVisible::default();
        let scanning_back = it.bidi_p && it.bidi_it.scan_dir == -1;
        if charpos >= 0
            && ((!scanning_back && it.charpos() >= charpos)
                || (scanning_back && it.charpos() <= charpos))
        {
            let mut top_x = it.current_x;
            let mut top_y = it.current_y;
            let window_top_y =
                (self.window_tab_line_height(w) + self.window_header_line_height(w)) as i32;
            let mut save_it_data: ShelvedBidiCache = None;
            let save_it = self.save_it(&it, &mut save_it_data);
            self.xdisp.last_height = 0;
            let bottom_y = self.line_bottom_y(&it)?;
            if top_y < window_top_y {
                visible_p = bottom_y > window_top_y;
            } else if top_y < it.last_visible_y {
                visible_p = true;
            }
            if bottom_y >= it.last_visible_y
                && it.bidi_p
                && it.bidi_it.scan_dir == -1
                && it.charpos() < charpos
            {
                let ten_more_lines = 10 * self.default_line_pixel_height(w);
                self.move_it_to(
                    &mut it,
                    charpos,
                    -1,
                    bottom_y + ten_more_lines,
                    -1,
                    MOVE_TO_POS | MOVE_TO_Y,
                )?;
                if it.current_y > top_y {
                    visible_p = false;
                }
            }
            self.restore_it(&mut it, Some(&save_it), &mut save_it_data);
            if visible_p {
                if it.method == ItMethod::FromDisplayVector {
                    if charpos < 2 || top.charpos >= charpos {
                        top_x = self.it_row_x(&it);
                    } else {
                        let mut it2 = It::default();
                        self.start_display(&mut it2, w, top)?;
                        it2.glyph_row = None;
                        self.move_it_to(&mut it2, charpos - 1, -1, -1, -1, MOVE_TO_POS)?;
                        let it2_prev = if it2.charpos() != charpos - 1 {
                            it2.clone()
                        } else {
                            loop {
                                self.get_next_display_element(&mut it2)?;
                                self.produce_glyphs_macro(&mut it2)?;
                                let prev = it2.clone();
                                self.set_iterator_to_next(&mut it2, true)?;
                                if !(it2.method == ItMethod::FromDisplayVector
                                    && it2.charpos() < charpos)
                                {
                                    break prev;
                                }
                            }
                        };
                        if it2_prev.at_end_of_line_p()
                            || it2_prev.current_x > it2_prev.last_visible_x
                        {
                            top_x = self.it_row_x(&it);
                        } else {
                            top_x = it2_prev.current_x;
                            top_y = it2_prev.current_y;
                        }
                    }
                } else if it.charpos() != charpos {
                    let (x, y) =
                        self.pos_visible_display_spec(w, top, charpos, &it, top_x, top_y)?;
                    top_x = x;
                    top_y = y;
                }
                out.x = top_x;
                out.y = if it.ascent == 0
                    && it.what == DisplayElementType::Image
                    && it.method != ItMethod::FromImage
                    && it.image_id < 0
                    && it.max_ascent > 0
                {
                    top_y.max(window_top_y)
                } else {
                    (top_y + (it.max_ascent - it.ascent).max(0)).max(window_top_y)
                };
                out.rtop = (window_top_y - top_y).max(0);
                out.rbot = (bottom_y - it.last_visible_y).max(0);
                out.rowh = (bottom_y.min(it.last_visible_y) - top_y.max(window_top_y)).max(0);
                out.vpos = it.vpos;
                if it.bidi_it.paragraph_dir == BidiDir::R2L {
                    r2l = true;
                }
            }
        } else {
            let mut it2data: ShelvedBidiCache = None;
            let mut it2 = self.save_it(&it, &mut it2data);
            let zv = self.cur().zv as i64;
            if it.charpos() < zv && self.cur().text.byte_at(it.bytepos() as usize) != b'\n' {
                self.move_it_by_lines(&mut it, 1)?;
            }
            if charpos < it.charpos()
                || (it.what == DisplayElementType::Eob && charpos == it.charpos())
            {
                visible_p = true;
                self.restore_it(&mut it2, None, &mut it2data);
                self.move_it_to(&mut it2, charpos, -1, -1, -1, MOVE_TO_POS)?;
                out.x = it2.current_x;
                out.y = if it2.ascent == 0
                    && it2.what == DisplayElementType::Image
                    && it2.method != ItMethod::FromImage
                    && it2.image_id < 0
                    && it2.max_ascent > 0
                {
                    it2.current_y
                } else {
                    it2.current_y + it2.max_ascent - it2.ascent
                };
                let bottom = it2.current_y + it2.max_ascent + it2.max_descent;
                out.rtop = (-it2.current_y).max(0);
                out.rbot = (bottom - it.last_visible_y).max(0);
                let tab_line = self.window_tab_line_height(w) as i32;
                let header_line = self.window_header_line_height(w) as i32;
                out.rowh = (bottom.min(it.last_visible_y)
                    - it2.current_y.max(tab_line).max(header_line))
                .max(0);
                out.vpos = it2.vpos;
                if it2.bidi_it.paragraph_dir == BidiDir::R2L {
                    r2l = true;
                }
            } else {
                self.bidi_unshelve_cache(it2data.take(), true);
            }
        }
        self.bidi_unshelve_cache(itdata, false);
        if let Some(old) = old_buffer {
            self.set_buffer_internal(old);
        }
        if visible_p {
            if self.window(w).hscroll > 0 {
                out.x -=
                    (self.window_hscroll_limited(w) * self.window_frame_column_width(w)) as i32;
            }
            if r2l {
                out.x = self.window_box_width(w, GlyphRowArea::Text) as i32 - out.x - 1;
            }
        }
        let win = self.window_mut(w);
        win.mode_line_height = prev_mode_line_height;
        win.header_line_height = prev_header_line_height;
        win.tab_line_height = prev_tab_line_height;
        Ok(visible_p.then_some(out))
    }

    fn it_row_x(&self, it: &It) -> i32 {
        it.glyph_row.map_or(0, |row| self.glyph_row(row).x)
    }

    fn pos_visible_display_spec(
        &mut self,
        w: Value,
        top: TextPos,
        charpos: i64,
        it: &It,
        top_x: i32,
        top_y: i32,
    ) -> LispResult<(i32, i32)> {
        let (mut top_x, mut top_y) = (top_x, top_y);
        let cpos = Value::fixnum(charpos);
        let spec = crate::textprop::f_get_char_property(self, &[cpos, sym::DISPLAY, Value::NIL])?;
        let string = self.string_from_display_spec(spec);
        let newline_in_string = string.is_string() && self.string_bytes(string).contains(&b'\n');
        let frame_window_p = self.frame_window_p(it.f);
        let replacing_spec_p = !spec.is_nil()
            && self.handle_display_spec_without_iterator(
                spec,
                crate::xdisp::DisplaySpecContext {
                    object: Value::NIL,
                    position: charpos as usize,
                    bufpos: charpos as usize,
                    frame_window_p,
                    enable_eval: true,
                },
            )? != 0;
        if replacing_spec_p {
            let endpos = crate::textprop::f_next_single_char_property_change(
                self,
                &[cpos, sym::DISPLAY, Value::NIL, Value::NIL],
            )?;
            let startpos = crate::textprop::f_previous_single_char_property_change(
                self,
                &[endpos, sym::DISPLAY, Value::NIL, Value::NIL],
            )?;
            let start = startpos.as_fixnum().unwrap_or(0);
            let end = endpos.as_fixnum().unwrap_or(0);
            let mut it3 = It::default();
            self.start_display(&mut it3, w, top)?;
            if start > top.charpos {
                self.move_it_to(&mut it3, start - 1, -1, -1, -1, MOVE_TO_POS)?;
            }
            if it3.method == ItMethod::FromBuffer
                && (it3.c == i32::from(b'\n')
                    || self.cur().text.byte_at(it3.bytepos() as usize) == b'\n')
            {
                self.move_it_by_lines(&mut it3, 1)?;
            } else {
                let x = it3.current_x + it3.pixel_width;
                if self.move_it_in_display_line_to(&mut it3, -1, x, MOVE_TO_X)?
                    == MoveItResult::LineContinued
                {
                    self.move_it_by_lines(&mut it3, 1)?;
                    if it3.line_wrap == LineWrap::WordWrap {
                        self.move_it_by_lines(&mut it3, -1)?;
                    }
                }
            }
            top_y = it3.current_y;
            if it3.bidi_p {
                self.start_display(&mut it3, w, top)?;
                self.move_it_to(&mut it3, end + 1, -1, -1, -1, MOVE_TO_POS)?;
                if it3.current_y < top_y {
                    top_y = it3.current_y;
                }
            }
            self.start_display(&mut it3, w, top)?;
            it3.glyph_row = None;
            self.move_it_to(&mut it3, -1, 0, top_y, -1, MOVE_TO_X | MOVE_TO_Y)?;
            let mut it3_moved = false;
            let mut top_x_before_string = it3.current_x;
            while self.get_next_display_element(&mut it3)? {
                if it3.object != string {
                    top_x_before_string = it3.current_x;
                }
                self.produce_glyphs_macro(&mut it3)?;
                if (it3.bidi_it.scan_dir == 1 && it3.charpos() >= charpos)
                    || (it3.bidi_it.scan_dir == -1 && it3.charpos() <= charpos)
                    || it3.at_end_of_line_p()
                {
                    break;
                }
                it3_moved = true;
                self.set_iterator_to_next(&mut it3, false)?;
            }
            top_x = it3.current_x - it3.pixel_width;
            if !it3.line_number_produced_p {
                if it3.lnum_pixel_width > 0 {
                    top_x += it3.lnum_pixel_width;
                    top_x_before_string += it3.lnum_pixel_width;
                } else if it.line_number_produced_p {
                    top_x += it.lnum_pixel_width;
                    top_x_before_string += it3.lnum_pixel_width;
                }
            }
            if it3_moved && newline_in_string && it3.charpos() != charpos && it3.object == string {
                top_x = top_x_before_string;
            }
        }
        Ok((top_x, top_y))
    }
}
