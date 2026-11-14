use crate::bidi::ShelvedBidiCache;
use crate::dispextern::{It, LineWrap, MOVE_TO_POS, MOVE_TO_X, MOVE_TO_Y, TextPos};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::xdisp::echo::unwind_with_echo_area_buffer;
use crate::xfaces::{
    HEADER_LINE_ACTIVE_FACE_ID, HEADER_LINE_INACTIVE_FACE_ID, MODE_LINE_ACTIVE_FACE_ID,
    MODE_LINE_INACTIVE_FACE_ID, TAB_LINE_ACTIVE_FACE_ID, TAB_LINE_INACTIVE_FACE_ID,
};

fn clip_to_bounds(lower: i64, num: i64, upper: i64) -> i64 {
    if num < lower {
        lower
    } else if num <= upper {
        num
    } else {
        upper
    }
}

fn ranged_fixnum(v: Value) -> Option<i32> {
    v.as_fixnum()
        .filter(|n| (0..=i64::from(i32::MAX)).contains(n))
        .map(|n| n as i32)
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

impl Lisp {
    fn text_byte(&self, bytepos: i64) -> u8 {
        self.cur().text.byte_at(bytepos as usize)
    }

    fn multibyte_buffer_p(&self) -> bool {
        !self.plain_value(sym::ENABLE_MULTIBYTE_CHARACTERS).is_nil()
    }

    fn inc_both(&self, charpos: &mut i64, bytepos: &mut i64) {
        let p = self.inc_text_pos(TextPos::new(*charpos, *bytepos), self.multibyte_buffer_p());
        (*charpos, *bytepos) = (p.charpos, p.bytepos);
    }

    fn dec_both(&self, charpos: &mut i64, bytepos: &mut i64) {
        let p = self.dec_text_pos(TextPos::new(*charpos, *bytepos), self.multibyte_buffer_p());
        (*charpos, *bytepos) = (p.charpos, p.bytepos);
    }

    fn fetch_char_advance(&self, charpos: &mut i64, bytepos: &mut i64) -> u32 {
        let c = if self.multibyte_buffer_p() {
            self.cur().text.char_at_byte(*bytepos as usize)
        } else {
            u32::from(self.text_byte(*bytepos))
        };
        self.inc_both(charpos, bytepos);
        c
    }

    fn mode_line_format_of(&self, w: Value, param: Value) -> Value {
        let format = self.window_parameter(w, param);
        if format.is_nil() {
            self.find_symbol_value(param)
        } else {
            format
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn window_text_pixel_size(
        &mut self,
        window: Value,
        from: Value,
        to: Value,
        x_limit: Value,
        y_limit: Value,
        mode_lines: Value,
        ignore_line_at_end: Value,
    ) -> LispResult {
        let w = self.decode_live_window(window)?;
        self.with_gc_deferred(|lisp| {
            lisp.window_text_pixel_size_1(
                w,
                from,
                to,
                x_limit,
                y_limit,
                mode_lines,
                ignore_line_at_end,
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn window_text_pixel_size_1(
        &mut self,
        w: Value,
        from: Value,
        to: Value,
        x_limit: Value,
        y_limit: Value,
        mode_lines: Value,
        ignore_line_at_end: Value,
    ) -> LispResult {
        let (begv, begv_byte, zv, zv_byte) = {
            let b = self.cur();
            (
                b.begv as i64,
                b.begv_byte as i64,
                b.zv as i64,
                b.zv_byte as i64,
            )
        };
        let mut start;
        let mut bpos;
        let mut vertical_offset = 0;
        if from.is_nil() {
            start = begv;
            bpos = begv_byte;
        } else if from == sym::T {
            start = begv;
            bpos = begv_byte;
            while bpos < zv_byte {
                let c = self.text_byte(bpos);
                if !(c == b' ' || c == b'\t' || c == b'\n' || c == b'\r') {
                    break;
                }
                self.inc_both(&mut start, &mut bpos);
            }
            while bpos > begv_byte {
                self.dec_both(&mut start, &mut bpos);
                let c = self.text_byte(bpos);
                if !(c == b' ' || c == b'\t') {
                    break;
                }
            }
        } else if from.is_cons() {
            let pos = self.fix_position(self.xcar(from))?;
            start = clip_to_bounds(begv, pos, zv);
            bpos = self.cur().text.char_to_byte(start as usize) as i64;
            vertical_offset = self.check_fixnum(self.xcdr(from))? as i32;
        } else {
            let pos = self.fix_position(from)?;
            start = clip_to_bounds(begv, pos, zv);
            bpos = self.cur().text.char_to_byte(start as usize) as i64;
        }
        let startp = TextPos::new(start, bpos);
        let mut end;
        if to.is_nil() {
            end = zv;
        } else if to == sym::T {
            end = zv;
            bpos = zv_byte;
            while bpos > begv_byte {
                self.dec_both(&mut end, &mut bpos);
                let c = self.text_byte(bpos);
                if !(c == b' ' || c == b'\t' || c == b'\n' || c == b'\r') {
                    self.inc_both(&mut end, &mut bpos);
                    break;
                }
            }
            while bpos < zv_byte {
                let c = self.fetch_char_advance(&mut end, &mut bpos);
                if !(c == u32::from(b' ') || c == u32::from(b'\t')) {
                    break;
                }
            }
        } else {
            let pos = self.fix_position(to)?;
            end = clip_to_bounds(start, pos, zv);
        }
        let mut max_x = 0;
        if let Some(n) = ranged_fixnum(x_limit) {
            max_x = n;
        } else if !x_limit.is_nil() {
            max_x = i32::MAX;
        }
        let mut max_y = 0;
        if y_limit.is_nil() {
            max_y = i32::MAX;
        } else if let Some(n) = ranged_fixnum(y_limit) {
            max_y = n;
        }
        let itdata = self.bidi_shelve_cache();
        let mut it = It::default();
        self.start_display(&mut it, w, startp)?;
        let mut start_y = it.current_y;
        it.bidi_p = false;
        let start_x;
        let mut start_bpos = startp.bytepos;
        if vertical_offset != 0 {
            it.current_y = 0;
            self.move_it_by_lines(&mut it, 0)?;
            if vertical_offset < 0 {
                while it.current_y > vertical_offset {
                    let last_y = it.current_y;
                    let dy = vertical_offset.abs() + it.current_y;
                    self.move_it_vertically_backward(&mut it, dy)?;
                    if it.current_y == last_y {
                        break;
                    }
                }
            } else {
                self.move_it_vertically(&mut it, vertical_offset)?;
            }
            it.current_y =
                (self.window_tab_line_height(w) + self.window_header_line_height(w)) as i32;
            start = clip_to_bounds(begv, it.charpos(), zv);
            start_bpos = self.cur().text.char_to_byte(start as usize) as i64;
            start_y = it.current_y;
            start_x = it.current_x;
        } else {
            self.move_it_by_lines(&mut it, 0)?;
            it.current_x = 0;
            it.hpos = 0;
            it.wrap_prefix_width = 0;
            if it.charpos() != start {
                let mut it1data: ShelvedBidiCache = None;
                let it1 = self.save_it(&it, &mut it1data);
                self.move_it_to(&mut it, start, -1, -1, -1, MOVE_TO_POS)?;
                if it.charpos() > start && start > begv {
                    let it1pos = it1.charpos();
                    let it1_x = it1.current_x;
                    self.restore_it(&mut it, Some(&it1), &mut it1data);
                    if start - 1 > it1pos {
                        self.move_it_to(&mut it, start - 1, -1, -1, -1, MOVE_TO_POS)?;
                    } else {
                        self.move_it_in_display_line(
                            &mut it,
                            start,
                            it1_x + 1,
                            MOVE_TO_POS | MOVE_TO_X,
                        )?;
                    }
                    let mut x = it.current_x;
                    if it.charpos() == start - 1 {
                        x += it.pixel_width;
                    }
                    start_x = x;
                } else {
                    start_x = it.current_x;
                    self.bidi_unshelve_cache(it1data, true);
                }
            } else {
                start_x = it.current_x;
            }
        }
        let mut start_x = start_x;
        let mut move_op = MOVE_TO_POS | MOVE_TO_Y;
        let mut to_x = -1;
        it.current_y = start_y;
        if self.text_byte(start_bpos) == b'\n' {
            it.current_x = 0;
            start_x = 0;
            it.wrap_prefix_width = 0;
        }
        if !x_limit.is_nil() {
            it.last_visible_x = max_x;
            move_op |= MOVE_TO_X;
            to_x = i32::MAX;
        }
        let mut it2data: ShelvedBidiCache = None;
        let it2 = self.save_it(&it, &mut it2data);
        let mut x = self.move_it_to(&mut it, end, to_x, max_y, -1, move_op)?;
        let mut doff = 0;
        if it.charpos() > end {
            let end_y = it.current_y;
            end -= 1;
            self.restore_it(&mut it, Some(&it2), &mut it2data);
            x = self.move_it_to(&mut it, end, to_x, max_y, -1, move_op)?;
            if it.charpos() == end {
                x += it.pixel_width;
                if !ignore_line_at_end.is_nil() {
                    if it.current_y < end_y {
                        doff = it.max_ascent.max(it.ascent) + it.max_descent.max(it.descent);
                    }
                } else {
                    it.max_ascent = it.max_ascent.max(it.ascent);
                    it.max_descent = it.max_descent.max(it.descent);
                }
            } else if it.charpos() > end
                && it.line_wrap == LineWrap::Truncate
                && it.current_x - it.first_visible_x >= it.last_visible_x
            {
                it.current_y += it.max_ascent.max(it.ascent) + it.max_descent.max(it.descent);
            }
        } else {
            self.bidi_unshelve_cache(it2data, true);
        }
        if !x_limit.is_nil() && x > max_x {
            x = max_x;
        }
        if it.current_y > start_y {
            start_x = 0;
        }
        let lines_above =
            (self.window_tab_line_height(w) + self.window_header_line_height(w)) as i32;
        let mut y = if !ignore_line_at_end.is_nil() {
            it.current_y + doff - lines_above
        } else {
            it.current_y + it.max_ascent + it.max_descent + doff - lines_above
        };
        if y > max_y {
            y = max_y;
        }
        if (mode_lines == sym::TAB_LINE || mode_lines == sym::T) && self.window_wants_tab_line(w) {
            let format = self.mode_line_format_of(w, sym::TAB_LINE_FORMAT);
            let face = self.current_line_active_face_id(
                w,
                TAB_LINE_ACTIVE_FACE_ID,
                TAB_LINE_INACTIVE_FACE_ID,
            );
            y += self.display_mode_line(w, face, format)?;
        }
        if (mode_lines == sym::HEADER_LINE || mode_lines == sym::T)
            && self.window_wants_header_line(w)
        {
            let format = self.mode_line_format_of(w, sym::HEADER_LINE_FORMAT);
            let face = self.current_line_active_face_id(
                w,
                HEADER_LINE_ACTIVE_FACE_ID,
                HEADER_LINE_INACTIVE_FACE_ID,
            );
            y += self.display_mode_line(w, face, format)?;
        }
        if (mode_lines == sym::MODE_LINE || mode_lines == sym::T) && self.window_wants_mode_line(w)
        {
            let format = self.mode_line_format_of(w, sym::MODE_LINE_FORMAT);
            let face = self.current_line_active_face_id(
                w,
                MODE_LINE_ACTIVE_FACE_ID,
                MODE_LINE_INACTIVE_FACE_ID,
            );
            y += self.display_mode_line(w, face, format)?;
        }
        self.bidi_unshelve_cache(itdata, false);
        let width = Value::fixnum(i64::from(x - start_x));
        let height = Value::fixnum(i64::from(y));
        Ok(if vertical_offset == 0 {
            self.cons(width, height)
        } else {
            self.list(&[width, height, Value::fixnum(start)])
        })
    }
}

pub fn f_window_text_pixel_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let window = arg(args, 0);
    let w = lisp.decode_live_window(window)?;
    let b = lisp.window(w).contents;
    let old_b = (b != lisp.current_buffer).then_some(lisp.current_buffer);
    if old_b.is_some() {
        lisp.set_buffer_internal(b);
    }
    let value = lisp.window_text_pixel_size(
        window,
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        arg(args, 4),
        arg(args, 5),
        arg(args, 6),
    );
    if let Some(old_b) = old_b {
        lisp.set_buffer_internal(old_b);
    }
    value
}

pub fn f_buffer_text_pixel_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (buffer_or_name, window, x_limit, y_limit, from, to) = (
        arg(args, 0),
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        arg(args, 4),
        arg(args, 5),
    );
    let w = lisp.decode_live_window(window)?;
    let buffer = if buffer_or_name.is_nil() {
        lisp.current_buffer
    } else {
        let b = crate::buffer::f_get_buffer(lisp, &[buffer_or_name])?;
        lisp.check_buffer(b)?
    };
    let count = lisp.specpdl_index();
    let data = lisp.with_echo_area_buffer_unwind_data(Some(w));
    lisp.record_unwind(unwind_with_echo_area_buffer, data);
    lisp.set_buffer_internal(buffer);
    let base_line_pos = lisp.window(w).base_line_pos;
    let end_valid = lisp.window(w).flags.window_end_valid;
    let r = (|| -> LispResult {
        if buffer != lisp.window(w).contents {
            lisp.wset_buffer(w, buffer);
            let (pointm, old_pointm) = {
                let win = lisp.window(w);
                (win.pointm, win.old_pointm)
            };
            lisp.set_marker(pointm, Value::fixnum(1), buffer, false)?;
            lisp.set_marker(old_pointm, Value::fixnum(1), buffer, false)?;
        }
        lisp.window_text_pixel_size(window, from, to, x_limit, y_limit, Value::NIL, Value::NIL)
    })();
    let u = lisp.unbind_to(count);
    let value = r?;
    u?;
    lisp.window_mut(w).base_line_pos = base_line_pos;
    lisp.window_mut(w).flags.window_end_valid = end_valid;
    Ok(value)
}
