use crate::bidi::BidiType;
use crate::dispextern::{DisplayElementType, It, MOVE_TO_POS, MOVE_TO_Y, TextPos};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::xfaces::DEFAULT_FACE_ID;

const SCROLL_LIMIT: i32 = 100;

fn pint2str(width: i64, d: i64) -> Vec<u8> {
    let digits = if d <= 0 {
        "0".to_string()
    } else {
        d.to_string()
    };
    let pad = (width - digits.len() as i64).max(0) as usize;
    let mut out = vec![b' '; pad];
    out.extend_from_slice(digits.as_bytes());
    out
}

impl Lisp {
    fn display_count_lines_logically(
        &mut self,
        start_byte: i64,
        limit_byte: i64,
        count: i64,
    ) -> LispResult<(i64, i64)> {
        let b = self.cur();
        if !self.var_true(sym::DISPLAY_LINE_NUMBERS_WIDEN) || (b.begv == 1 && b.zv == b.z()) {
            let (val, pos) =
                self.display_count_lines(start_byte as usize, limit_byte as usize, count);
            return Ok((val, pos as i64));
        }
        let pdl_count = self.specpdl.len();
        self.record_unwind_restriction();
        self.widen();
        let (val, pos) = self.display_count_lines(start_byte as usize, limit_byte as usize, count);
        self.unbind_to(pdl_count)?;
        Ok((val, pos as i64))
    }

    fn display_count_lines_visually(&mut self, it: &It) -> LispResult<i64> {
        if it.lnum_bytepos > 0 {
            return Ok(it.lnum + 1);
        }
        let count = self.specpdl.len();
        let (pt, pt_byte) = (self.cur().pt as i64, self.cur().pt_byte as i64);
        let (from, to) = if it.charpos() <= pt {
            (it.current.pos, pt)
        } else {
            (TextPos::new(pt, pt_byte), it.charpos())
        };
        self.specbind(sym::DISPLAY_LINE_NUMBERS, sym::RELATIVE)?;
        let mut tem_it = It::default();
        let r = (|| -> LispResult<()> {
            self.start_display(&mut tem_it, it.window, from)?;
            let line_height = self.frame(tem_it.f).geometry.line_height as i32;
            let to_y = tem_it.last_visible_y + (SCROLL_LIMIT + 10) * line_height;
            self.move_it_to(&mut tem_it, to, -1, to_y, -1, MOVE_TO_POS | MOVE_TO_Y)?;
            Ok(())
        })();
        self.unbind_to(count)?;
        r?;
        Ok(if it.charpos() <= pt {
            -i64::from(tem_it.vpos)
        } else {
            i64::from(tem_it.vpos)
        })
    }

    fn required_matrix_height(&mut self, w: Value) -> i64 {
        let f = self.window(w).frame;
        if self.frame_window_p(f) {
            let ch_height = i64::from(
                self.display_info(self.frame(f).terminal)
                    .smallest_font_height,
            );
            let window_pixel_height = self.window_box_height(w) + self.window(w).vscroll.abs();
            return ((window_pixel_height + ch_height - 1) / ch_height)
                * self.window(w).nrows_scale_factor
                + 2
                + 2;
        }
        self.window(w).total_lines
    }

    pub(super) fn maybe_produce_line_number(&mut self, it: &mut It) -> LispResult<()> {
        let itdata = self.bidi_shelve_cache();
        let r = self.maybe_produce_line_number_1(it);
        self.bidi_unshelve_cache(itdata, false);
        r
    }

    fn maybe_produce_line_number_1(&mut self, it: &mut It) -> LispResult<()> {
        let mut last_line = it.lnum;
        let mut first_time = false;
        let mode = self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS);
        let visual = mode == sym::VISUAL;
        let relative = mode == sym::RELATIVE;
        let offset = self.int_var(sym::DISPLAY_LINE_NUMBERS_OFFSET);
        let widen = self.var_true(sym::DISPLAY_LINE_NUMBERS_WIDEN);
        let line_numbers_wide = if offset != 0 && !widen && !visual && !relative {
            true
        } else {
            widen
        };
        let b = self.cur();
        let beg_byte: i64 = if line_numbers_wide {
            1
        } else {
            b.begv_byte as i64
        };
        let z_byte = if line_numbers_wide {
            b.z_byte() as i64
        } else {
            b.zv_byte as i64
        };
        let this_line = if visual {
            self.display_count_lines_visually(it)?
        } else {
            let mut start_from;
            if last_line == 0 {
                let win = self.window(it.window);
                let b = self.cur();
                if win.base_line_number > 0
                    && win.base_line_pos > 0
                    && win.base_line_pos <= it.charpos()
                    && !(line_numbers_wide && (b.begv_byte != 1 || b.z_byte() != b.zv_byte))
                    && !b.clip_changed
                {
                    start_from = b.text.char_to_byte(win.base_line_pos as usize) as i64;
                    last_line = win.base_line_number - 1;
                } else {
                    start_from = beg_byte;
                }
                if it.lnum_bytepos == 0 {
                    first_time = true;
                }
            } else {
                start_from = it.lnum_bytepos;
            }
            if !(beg_byte <= start_from && start_from <= z_byte) {
                last_line = 0;
                start_from = beg_byte;
            }
            let (count, _) =
                self.display_count_lines_logically(start_from, it.bytepos(), it.charpos())?;
            last_line + count
        };
        if this_line != last_line || it.lnum_bytepos == 0 {
            it.lnum = this_line;
            it.lnum_bytepos = it.bytepos();
        }
        let beyond_zv = it.bytepos() >= self.cur().zv_byte as i64;
        let mut lnum_offset: i64 = -1;
        let lnum_face_id = self.merge_faces(it.window, sym::LINE_NUMBER, 0, DEFAULT_FACE_ID)?;
        let current_lnum_face_id =
            self.merge_faces(it.window, sym::LINE_NUMBER_CURRENT_LINE, 0, DEFAULT_FACE_ID)?;
        let save_free_realized_faces = self.xdisp.inhibit_free_realized_faces;
        self.xdisp.inhibit_free_realized_faces = true;
        let r = self.produce_line_number_glyphs(
            it,
            this_line,
            last_line,
            first_time,
            beg_byte,
            beyond_zv,
            &mut lnum_offset,
            (lnum_face_id, current_lnum_face_id),
        );
        self.xdisp.inhibit_free_realized_faces = save_free_realized_faces;
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn produce_line_number_glyphs(
        &mut self,
        it: &mut It,
        this_line: i64,
        last_line: i64,
        first_time: bool,
        beg_byte: i64,
        beyond_zv: bool,
        lnum_offset: &mut i64,
        (lnum_face_id, current_lnum_face_id): (i64, i64),
    ) -> LispResult<()> {
        let mode = self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS);
        let visual = mode == sym::VISUAL;
        let relative = mode == sym::RELATIVE;
        let (pt, pt_byte) = (self.cur().pt as i64, self.cur().pt_byte as i64);
        if (relative || visual || lnum_face_id != current_lnum_face_id) && it.pt_lnum == 0 {
            if pt_byte > it.lnum_bytepos && !visual {
                let (count, _) =
                    self.display_count_lines_logically(it.lnum_bytepos, pt_byte, pt)?;
                it.pt_lnum = this_line + count;
            } else {
                it.pt_lnum = self.display_count_lines_logically(beg_byte, pt_byte, pt)?.0;
            }
        }
        if it.lnum_width == 0 {
            let width = self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS_WIDTH);
            if let Some(lnum_width) = width.as_fixnum().filter(|&n| n >= 0) {
                let column_width = self.frame(it.f).geometry.column_width as i32;
                let lnum_width_limit =
                    (it.last_visible_x - it.first_visible_x) / column_width - 5 - 2;
                let lnum_width = lnum_width as i32;
                it.lnum_width = if lnum_width < 1 {
                    1
                } else if lnum_width <= lnum_width_limit {
                    lnum_width
                } else {
                    lnum_width_limit
                };
            }
            let nrows = self.required_matrix_height(it.window);
            let current_absolute =
                self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS_CURRENT_ABSOLUTE);
            let max_lnum = if current_absolute.is_nil() && (relative || visual) {
                nrows - 2
            } else if visual {
                it.pt_lnum + nrows - 1
            } else {
                this_line + nrows - 1 - i64::from(it.vpos)
            }
            .max(1);
            it.lnum_width = f64::from(it.lnum_width).max((max_lnum as f64).log10() + 1.0) as i32;
        }
        let display_offset = self.int_var(sym::DISPLAY_LINE_NUMBERS_OFFSET);
        if relative {
            *lnum_offset = it.pt_lnum;
        } else if visual {
            *lnum_offset = 0;
        } else if display_offset != 0 {
            *lnum_offset -= display_offset;
        }
        let mut lnum_to_display = (this_line - *lnum_offset).abs();
        if (relative || visual)
            && lnum_to_display == 0
            && !self
                .find_symbol_value(sym::DISPLAY_LINE_NUMBERS_CURRENT_ABSOLUTE)
                .is_nil()
        {
            lnum_to_display = it.pt_lnum + 1;
        }
        let mut lnum_buf = pint2str(i64::from(it.lnum_width) + 1, lnum_to_display);
        lnum_buf.push(b' ');
        let mut tem_it = It::default();
        self.init_iterator(&mut tem_it, it.window, -1, -1, None, DEFAULT_FACE_ID)?;
        tem_it.position = TextPos::new(0, 0);
        tem_it.avoid_cursor_p = true;
        tem_it.bidi_p = true;
        tem_it.bidi_it.type_ = BidiType::WeakEn;
        tem_it.bidi_it.resolved_level = 2;
        let column_width = self.frame(it.f).geometry.column_width as i32;
        let width_limit = tem_it.last_visible_x - tem_it.first_visible_x - 3 * column_width;
        tem_it.face_id = lnum_face_id as i32;
        if lnum_face_id != current_lnum_face_id
            && (if visual {
                this_line == 0
            } else {
                this_line == it.pt_lnum
            })
            && it.what != DisplayElementType::Eob
        {
            tem_it.face_id = current_lnum_face_id as i32;
        } else if !beyond_zv {
            let major = self.int_var(sym::DISPLAY_LINE_NUMBERS_MAJOR_TICK);
            let minor = self.int_var(sym::DISPLAY_LINE_NUMBERS_MINOR_TICK);
            if major > 0 && lnum_to_display % major == 0 {
                tem_it.face_id =
                    self.merge_faces(it.window, sym::LINE_NUMBER_MAJOR_TICK, 0, DEFAULT_FACE_ID)?
                        as i32;
            } else if minor > 0 && lnum_to_display % minor == 0 {
                tem_it.face_id =
                    self.merge_faces(it.window, sym::LINE_NUMBER_MINOR_TICK, 0, DEFAULT_FACE_ID)?
                        as i32;
            }
        }
        let mut glyph_widths = Vec::new();
        for &p in &lnum_buf {
            let c = if beyond_zv
                || (!visual
                    && (it.continuation_lines_width > 0 || (this_line == last_line && !first_time)))
            {
                i32::from(b' ')
            } else {
                i32::from(p)
            };
            tem_it.c = c;
            tem_it.char_to_display = c;
            tem_it.len = 1;
            tem_it.position = TextPos::new(-1, -1);
            self.produce_glyphs_macro(&mut tem_it)?;
            if tem_it.nglyphs > 0 {
                let width = tem_it.pixel_width / tem_it.nglyphs;
                glyph_widths.extend(std::iter::repeat_n(width, tem_it.nglyphs as usize));
            }
            if tem_it.current_x >= width_limit {
                it.lnum_width = 0;
                it.lnum_pixel_width = 0;
                return Ok(());
            }
        }
        it.lnum_pixel_width = tem_it.current_x;
        for width in glyph_widths {
            it.current_x += width;
            if it.current_x > it.first_visible_x {
                it.hpos += 1;
            }
        }
        if it.glyph_row.is_some() {
            unreachable!("copying line-number glyphs needs glyph rows");
        }
        if !beyond_zv {
            it.max_ascent = it.max_ascent.max(tem_it.max_ascent);
            it.max_descent = it.max_descent.max(tem_it.max_descent);
            it.max_phys_ascent = it.max_phys_ascent.max(tem_it.max_phys_ascent);
            it.max_phys_descent = it.max_phys_descent.max(tem_it.max_phys_descent);
        }
        it.line_number_produced_p = true;
        Ok(())
    }
}
