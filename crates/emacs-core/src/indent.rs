use crate::character::bytes_by_char_head;
use crate::charfns::f_string_width;
use crate::composite::CompositionIt;
use crate::dispextern::{
    GlyphRowArea, It, ItMethod, LineWrap, MOVE_TO_POS, MOVE_TO_X, OVERLAY_STRING_CHUNK_SIZE,
    TextPos,
};
use crate::editfns::f_insert_char;
use crate::error::LispResult;
use crate::fns::rarely_quit;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::textprop::{f_next_property_change, f_next_single_property_change};
use crate::value::{MOST_POSITIVE_FIXNUM, Value};
use crate::window::{WindowBodyUnit, WindowLoop};

const MAX_FACE_ID: i64 = (1 << 20) - 1;
const DISP_TABLE_EXTRA_SLOTS: usize = 6;
const INT_MAX: i64 = i32::MAX as i64;
const DISP_INVIS_VECTOR_SLOT: usize = 4;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Position {
    pub bufpos: i64,
    pub bytepos: i64,
    pub hpos: i64,
    pub vpos: i64,
    pub prevhpos: i64,
    pub contin: bool,
}

#[derive(Default)]
pub struct Indent {
    pub last_known_column: i64,
    pub last_known_column_point: i64,
    pub last_known_column_modified: i64,
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("current-indentation", 0, 0, f_current_indentation),
    Subr::fixed("indent-to", 1, 2, f_indent_to).interactive("NIndent to column: "),
    Subr::fixed("current-column", 0, 0, f_current_column),
    Subr::fixed("move-to-column", 1, 2, f_move_to_column).interactive("NMove to column: "),
    Subr::fixed("compute-motion", 7, 7, f_compute_motion),
    Subr::fixed("vertical-motion", 1, 3, f_vertical_motion),
    Subr::fixed(
        "line-number-display-width",
        0,
        1,
        f_line_number_display_width,
    ),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_bool(sym::INDENT_TABS_MODE, true);
}

pub(crate) fn sanitize_char_width(width: i64) -> i64 {
    if (0..=1000).contains(&width) {
        width
    } else {
        1000
    }
}

fn leading_code_p(byte: u8) -> bool {
    byte & 0xC0 == 0xC0
}

pub struct ColumnScan {
    pub endpos: i64,
    pub goalcol: i64,
    pub prev_pos: i64,
    pub prev_bpos: i64,
    pub prev_col: i64,
}

impl Lisp {
    pub(crate) fn disp_table_p(&self, obj: Value) -> bool {
        self.is_char_table(obj)
            && self.char_table(obj).purpose == sym::DISPLAY_TABLE
            && self.char_table(obj).extras.len() == DISP_TABLE_EXTRA_SLOTS
    }

    pub fn buffer_display_table(&self) -> Option<Value> {
        let thisbuf = self.find_symbol_value(sym::BUFFER_DISPLAY_TABLE);
        if self.disp_table_p(thisbuf) {
            return Some(thisbuf);
        }
        let standard = self.find_symbol_value(sym::STANDARD_DISPLAY_TABLE);
        if self.disp_table_p(standard) {
            return Some(standard);
        }
        None
    }

    fn disp_vector(&self, dp: Option<Value>, c: u32) -> Option<Value> {
        let v = self.disp_char_vector(dp?, c);
        self.is_vector(v).then_some(v)
    }

    pub(crate) fn glyph_code_p(&self, gc: Value) -> bool {
        if gc.is_cons() {
            matches!(self.xcar(gc).as_fixnum(), Some(c) if (0..=crate::character::MAX_CHAR as i64).contains(&c))
                && matches!(self.xcdr(gc).as_fixnum(), Some(f) if (0..=MAX_FACE_ID).contains(&f))
        } else {
            let max = (MAX_FACE_ID << 22) | crate::character::MAX_CHAR as i64;
            matches!(gc.as_fixnum(), Some(n) if (0..=max).contains(&n))
        }
    }

    pub(crate) fn glyph_code_char(&self, gc: Value) -> i64 {
        if gc.is_cons() {
            self.xcar(gc).as_fixnum().unwrap_or(0)
        } else {
            gc.as_fixnum().unwrap_or(0) & crate::character::MAX_CHAR as i64
        }
    }

    fn display_vector_char(&self, entry: Value) -> i64 {
        if self.glyph_code_p(entry) {
            self.glyph_code_char(entry)
        } else {
            b' ' as i64
        }
    }

    fn multibyte_bytes_width(&self, pos_byte: usize, dp: Option<Value>) -> (usize, i64) {
        let text = &self.cur().text;
        let ch = text.char_at_byte(pos_byte);
        let bytes = text.char_len_at_byte(pos_byte);
        let width = if bytes_by_char_head(text.byte_at(pos_byte)) != bytes {
            bytes as i64 * 4
        } else if let Some(v) = self.disp_vector(dp, ch) {
            sanitize_char_width(self.vector_items(v).len() as i64)
        } else {
            self.char_width_of(ch) as i64
        };
        (bytes, width)
    }

    fn selective_display_t(&self) -> bool {
        self.find_symbol_value(sym::SELECTIVE_DISPLAY) == sym::T
    }

    fn record_known_column(&mut self, col: i64) {
        self.indent.last_known_column = col;
        self.indent.last_known_column_point = self.cur().pt as i64;
        self.indent.last_known_column_modified = self.cur().text.modiff;
    }

    pub fn invalidate_current_column(&mut self) {
        self.indent.last_known_column_point = 0;
    }

    pub fn current_column(&mut self) -> LispResult<i64> {
        let pt = self.cur().pt as i64;
        if pt == self.indent.last_known_column_point
            && self.cur().text.modiff == self.indent.last_known_column_modified
        {
            return Ok(self.indent.last_known_column);
        }
        let begv = self.cur().begv as i64;
        let line_beg = self.find_newline(pt as usize, begv as usize, -1).0 as i64;
        if self.cur().long_line_optimizations_p
            && let Some(threshold) = self.find_symbol_value(sym::LONG_LINE_THRESHOLD).as_fixnum()
            && pt - line_beg > threshold
        {
            return Ok(pt - line_beg);
        }
        let b = self.cur();
        if !b.text.props.is_empty() || !b.overlays.is_empty() || b.z() != b.z_byte() {
            return self.current_column_1();
        }
        let tab_width = self.sane_tab_width() as i64;
        let ctl_arrow = !self.find_symbol_value(sym::CTL_ARROW).is_nil();
        let dp = self.buffer_display_table();
        let selective = self.selective_display_t();
        let mut col = 0;
        let mut tab_seen = false;
        let mut post_tab = 0;
        let mut pos = pt;
        'scan: while pos > begv {
            pos -= 1;
            let byte = self.cur().text.byte_at(pos as usize) as u32;
            let charvec = self.disp_vector(dp, byte);
            let n = charvec.map_or(1, |v| self.vector_items(v).len());
            for i in (0..n).rev() {
                let c = match charvec {
                    Some(v) => self.display_vector_char(self.vector_items(v)[i]),
                    None => byte as i64,
                };
                if (0o40..0o177).contains(&c) {
                    col += 1;
                } else if c == b'\n' as i64 || (c == b'\r' as i64 && selective) {
                    break 'scan;
                } else if c == b'\t' as i64 {
                    if tab_seen {
                        col = ((col + tab_width) / tab_width) * tab_width;
                    }
                    post_tab += col;
                    col = 0;
                    tab_seen = true;
                } else if charvec.is_some() {
                    col += 1;
                } else {
                    col += if ctl_arrow && c < 0o200 { 2 } else { 4 };
                }
            }
        }
        if tab_seen {
            col = ((col + tab_width) / tab_width) * tab_width;
            col += post_tab;
        }
        self.record_known_column(col);
        Ok(col)
    }

    fn current_column_1(&mut self) -> LispResult<i64> {
        let pt = self.cur().pt as i64;
        Ok(self
            .scan_for_column(Some(pt), Some(MOST_POSITIVE_FIXNUM))?
            .goalcol)
    }

    pub fn skip_invisible(&mut self, pos: i64, to: i64, window: Value) -> LispResult<(i64, i64)> {
        let position = Value::fixnum(pos);
        let buffer = self.current_buffer;
        let overlay_limit = self.next_overlay_change(buffer, pos as usize, true) as i64;
        let mut proplimit = f_next_property_change(self, &[position, buffer, sym::T])?
            .as_fixnum()
            .unwrap_or(0);
        if overlay_limit < proplimit {
            proplimit = overlay_limit;
        }
        let next_boundary = if proplimit > pos + 100 || proplimit >= to {
            proplimit
        } else {
            let mut limit = (pos + 100).min(to);
            if overlay_limit < limit {
                limit = overlay_limit;
            }
            f_next_single_property_change(
                self,
                &[position, sym::INVISIBLE, buffer, Value::fixnum(limit)],
            )?
            .as_fixnum()
            .unwrap_or(limit)
        };
        let object = if !window.is_nil() && self.window(window).contents == buffer {
            window
        } else {
            buffer
        };
        let prop = self.get_char_property(position, sym::INVISIBLE, object)?;
        let inv_p = self.text_prop_means_invisible(prop);
        if if window.is_nil() {
            inv_p == 1
        } else {
            inv_p != 0
        } {
            return Ok((next_boundary, next_boundary));
        }
        Ok((pos, next_boundary))
    }

    fn check_display_width(&mut self, pos: i64, col: i64) -> LispResult<Option<(i64, i64)>> {
        let (val, overlay) =
            self.get_char_property_and_overlay(Value::fixnum(pos), sym::DISPLAY, Value::NIL)?;
        if val.is_nil() {
            return Ok(None);
        }
        let mut width = -1;
        let mut plist = Value::NIL;
        if val.is_cons() {
            let xcar = self.xcar(val);
            if xcar == sym::SPACE {
                let align_to_max = if col < MOST_POSITIVE_FIXNUM - INT_MAX {
                    INT_MAX + col
                } else {
                    MOST_POSITIVE_FIXNUM
                };
                plist = self.xcdr(val);
                let ranged = |v: Value, lo: i64, hi: i64| matches!(v.as_fixnum(), Some(n) if lo <= n && n <= hi);
                let mut prop = self.plist_get_eq(plist, sym::K_WIDTH);
                let fixnum_width = ranged(prop, 0, INT_MAX) || {
                    prop = self.plist_get_eq(plist, sym::K_RELATIVE_WIDTH);
                    ranged(prop, 0, INT_MAX)
                };
                if fixnum_width {
                    width = prop.as_fixnum().unwrap_or(0);
                } else if prop.is_float()
                    && (0.0..=INT_MAX as f64).contains(&self.heap.float_value(prop))
                {
                    width = (self.heap.float_value(prop) + 0.5) as i32 as i64;
                } else {
                    prop = self.plist_get_eq(plist, sym::K_ALIGN_TO);
                    if ranged(prop, col, align_to_max) {
                        width = prop.as_fixnum().unwrap_or(0) - col;
                    } else if prop.is_float()
                        && col as f64 <= self.heap.float_value(prop)
                        && self.heap.float_value(prop) <= align_to_max as f64
                    {
                        width = (self.heap.float_value(prop) + 0.5) as i32 as i64 - col;
                    }
                }
            } else if xcar == sym::IMAGE
                || xcar == sym::SLICE
                || (xcar.is_cons() && self.xcar(xcar) == sym::SLICE)
            {
                return Err(
                    self.error("Measuring an image's display width needs the display iterator")
                );
            }
        } else if val.is_string() {
            width = f_string_width(self, &[val, Value::NIL, Value::NIL])?
                .as_fixnum()
                .unwrap_or(0);
        }
        if width < 0 {
            return Ok(None);
        }
        let endpos = if self.is_overlay(overlay) {
            self.overlay(overlay).end as i64
        } else {
            self.get_property_and_range(pos, sym::DISPLAY, Value::NIL)?
                .map_or(pos, |(_, _, end)| end as i64)
        };
        if !plist.is_nil()
            && !self.plist_get_eq(plist, sym::K_RELATIVE_WIDTH).is_nil()
            && self.cur().text.multibyte
        {
            let dp = self.buffer_display_table();
            let pos_byte = self.cur().text.char_to_byte(pos as usize);
            width *= self.multibyte_bytes_width(pos_byte, dp).1;
        }
        Ok(Some((width, endpos)))
    }

    pub fn scan_for_column(
        &mut self,
        endpos: Option<i64>,
        goalcol: Option<i64>,
    ) -> LispResult<ColumnScan> {
        let tab_width = self.sane_tab_width() as i64;
        let ctl_arrow = !self.find_symbol_value(sym::CTL_ARROW).is_nil();
        let dp = self.buffer_display_table();
        let multibyte = self.cur().text.multibyte;
        let selective = self.selective_display_t();
        let mut col = 0;
        let mut prev_col = 0;
        let goal = goalcol.unwrap_or(MOST_POSITIVE_FIXNUM);
        let (pt, begv, zv) = (
            self.cur().pt as i64,
            self.cur().begv as i64,
            self.cur().zv as i64,
        );
        let end = endpos.unwrap_or(pt);
        let mut scan = self.find_newline(pt as usize, begv as usize, -1).0 as i64;
        let mut scan_byte = self.cur().text.char_to_byte(scan as usize) as i64;
        let buffer = self.current_buffer;
        let window = self.window_loop(WindowLoop::GetBufferWindow, buffer, true, Value::NIL)?;
        let mut prev_pos;
        let mut prev_bpos;
        'endloop: {
            if self.cur().long_line_optimizations_p {
                let mut lines_truncated = false;
                if !self.find_symbol_value(sym::TRUNCATE_LINES).is_nil() {
                    lines_truncated = true;
                } else {
                    let tpww = self.find_symbol_value(sym::TRUNCATE_PARTIAL_WIDTH_WINDOWS);
                    if !tpww.is_nil()
                        && !window.is_nil()
                        && self.window(window).total_cols
                            < self.frame(self.window(window).frame).geometry.text_cols
                    {
                        lines_truncated = match tpww.as_fixnum() {
                            Some(n) => self.window(window).total_cols < n,
                            None => true,
                        };
                    }
                }
                if lines_truncated {
                    let bolpos = scan;
                    let eolpos = self.find_newline(pt as usize, zv as usize, 1).0 as i64;
                    scan = bolpos + goal;
                    if scan > end {
                        scan = end;
                    }
                    if scan > eolpos {
                        scan = if eolpos == zv { zv } else { eolpos - 1 };
                    }
                    col = scan - bolpos;
                    if col > self.int_var(sym::LARGE_HSCROLL_THRESHOLD) {
                        prev_col = col - 1;
                        prev_pos = scan - 1;
                        prev_bpos = self.cur().text.char_to_byte(scan as usize) as i64;
                        break 'endloop;
                    }
                    scan = bolpos;
                    col = 0;
                }
            }
            let mut next_boundary = scan;
            prev_pos = scan;
            prev_bpos = scan_byte;
            let mut cmp_it = CompositionIt {
                parent_w: window,
                parent_f: if window.is_nil() {
                    Value::NIL
                } else {
                    self.window(window).frame
                },
                id: -1,
                ..CompositionIt::default()
            };
            self.composition_compute_stop_pos(&mut cmp_it, scan, end, Value::NIL, true)?;
            while scan < end {
                while scan == next_boundary {
                    let old_scan = scan;
                    (scan, next_boundary) = self.skip_invisible(scan, end, Value::NIL)?;
                    if scan != old_scan {
                        scan_byte = self.cur().text.char_to_byte(scan as usize) as i64;
                    }
                    if scan >= end {
                        break 'endloop;
                    }
                    if scan > cmp_it.stop_pos && cmp_it.id < 0 {
                        self.composition_reseat_it(
                            &mut cmp_it,
                            scan,
                            end,
                            window,
                            -1,
                            None,
                            Value::NIL,
                        )?;
                    }
                }
                if col >= goal {
                    break;
                }
                prev_col = col;
                prev_pos = scan;
                prev_bpos = scan_byte;
                if let Some((width, endp)) = self.check_display_width(scan, col)? {
                    col += width;
                    if endp > scan {
                        scan = endp;
                        scan_byte = self.cur().text.char_to_byte(scan as usize) as i64;
                        continue;
                    }
                }
                if cmp_it.id >= 0
                    || (scan == cmp_it.stop_pos
                        && self.composition_reseat_it(
                            &mut cmp_it,
                            scan,
                            end,
                            window,
                            -1,
                            None,
                            Value::NIL,
                        )?)
                {
                    self.composition_update_it(&mut cmp_it, scan, scan_byte, Value::NIL);
                }
                if cmp_it.id >= 0 {
                    scan += cmp_it.nchars;
                    scan_byte += cmp_it.nbytes;
                    if scan <= end {
                        col += cmp_it.width;
                    }
                    if cmp_it.to == cmp_it.nglyphs {
                        cmp_it.id = -1;
                        self.composition_compute_stop_pos(
                            &mut cmp_it,
                            scan,
                            end,
                            Value::NIL,
                            true,
                        )?;
                    } else {
                        cmp_it.from = cmp_it.to;
                    }
                    continue;
                }
                let c = self.cur().text.byte_at(scan_byte as usize);
                let charvec = if dp.is_some() && !(multibyte && leading_code_p(c)) {
                    self.disp_vector(dp, c as u32)
                } else {
                    None
                };
                if let Some(charvec) = charvec {
                    for entry in self.vector_items(charvec).to_vec() {
                        let c = self.display_vector_char(entry);
                        if c == b'\n' as i64 || (c == b'\r' as i64 && selective) {
                            break 'endloop;
                        }
                        if c == b'\t' as i64 {
                            col += tab_width;
                            col = col / tab_width * tab_width;
                        } else {
                            col += 1;
                        }
                    }
                } else {
                    if c == b'\n' || (c == b'\r' && selective) {
                        break 'endloop;
                    }
                    if c == b'\t' {
                        col += tab_width;
                        col = col / tab_width * tab_width;
                    } else if multibyte && leading_code_p(c) {
                        let (bytes, width) = self.multibyte_bytes_width(scan_byte as usize, dp);
                        scan_byte += bytes as i64 - 1;
                        col += width;
                    } else if ctl_arrow && (c < 0o40 || c == 0o177) {
                        col += 2;
                    } else if !(0o40..0o177).contains(&c) {
                        col += 4;
                    } else {
                        col += 1;
                    }
                }
                scan += 1;
                scan_byte += 1;
            }
        }
        self.record_known_column(col);
        Ok(ColumnScan {
            endpos: scan,
            goalcol: col,
            prev_pos,
            prev_bpos,
            prev_col,
        })
    }

    pub(crate) fn position_indentation(&mut self, pos_byte: i64) -> LispResult<i64> {
        let tab_width = self.sane_tab_width() as i64;
        let mut column = 0;
        let mut pos_byte = pos_byte;
        let mut next_boundary_byte = pos_byte;
        loop {
            while pos_byte == self.cur().zv_byte as i64 || pos_byte == next_boundary_byte {
                if pos_byte == self.cur().zv_byte as i64 {
                    return Ok(column);
                }
                let pos = self.cur().text.byte_to_char(pos_byte as usize) as i64;
                let zv = self.cur().zv as i64;
                let (pos, next_boundary) = self.skip_invisible(pos, zv, Value::NIL)?;
                pos_byte = self.cur().text.char_to_byte(pos as usize) as i64;
                next_boundary_byte = self.cur().text.char_to_byte(next_boundary as usize) as i64;
            }
            let multibyte = self.cur().text.multibyte;
            let c = self.cur().text.byte_at(pos_byte as usize);
            pos_byte += 1;
            match c {
                0o240 if multibyte => return Ok(column),
                0o240 | b' ' => column += 1,
                b'\t' => column += tab_width - column % tab_width,
                _ => {
                    if c < 0x80 || !multibyte {
                        return Ok(column);
                    }
                    pos_byte -= 1;
                    let ch = self.cur().text.char_at_byte(pos_byte as usize);
                    if self.regex_env(None).char_has_category(ch, b' ' as u32) {
                        column += 1;
                        pos_byte += self.cur().text.char_len_at_byte(pos_byte as usize) as i64;
                    } else {
                        return Ok(column);
                    }
                }
            }
        }
    }

    pub fn indented_beyond_p(&mut self, pos: i64, column: i64) -> LispResult<bool> {
        let mut pos = pos;
        let begv = self.cur().begv as i64;
        while pos > begv && self.char_at_pos(pos as usize) == b'\n' as u32 {
            pos -= 1;
            pos = self.find_newline(pos as usize, begv as usize, -1).0 as i64;
        }
        let pos_byte = self.cur().text.char_to_byte(pos as usize) as i64;
        Ok(self.position_indentation(pos_byte)? >= column)
    }

    fn indent_to(&mut self, column: i64, minimum: i64) -> LispResult<i64> {
        let tab_width = self.sane_tab_width() as i64;
        let mut fromcol = self.current_column()?;
        let mut mincol = fromcol + minimum;
        if mincol < column {
            mincol = column;
        }
        if fromcol == mincol {
            return Ok(mincol);
        }
        if self.var_true(sym::INDENT_TABS_MODE) {
            let n = mincol / tab_width - fromcol / tab_width;
            if n != 0 {
                f_insert_char(
                    self,
                    &[Value::fixnum(b'\t' as i64), Value::fixnum(n), sym::T],
                )?;
                fromcol = (mincol / tab_width) * tab_width;
            }
        }
        f_insert_char(
            self,
            &[
                Value::fixnum(b' ' as i64),
                Value::fixnum(mincol - fromcol),
                sym::T,
            ],
        )?;
        self.record_known_column(mincol);
        Ok(mincol)
    }

    fn selective_display_value(&self) -> i64 {
        let selective = self.find_symbol_value(sym::SELECTIVE_DISPLAY);
        match selective.as_fixnum() {
            Some(n) => n,
            None if selective.is_nil() => 0,
            None => -1,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn compute_motion(
        &mut self,
        from: i64,
        frombyte: i64,
        fromvpos: i64,
        fromhpos: i64,
        did_motion: bool,
        to: i64,
        tovpos: i64,
        tohpos: i64,
        width: i64,
        hscroll: i64,
        tab_offset: i64,
        win: Value,
    ) -> LispResult<Position> {
        let mut hpos = fromhpos;
        let mut vpos = fromvpos;
        let mut did_motion = did_motion;
        let mut tab_offset = tab_offset;
        let mut width = width;
        let tab_width = self.sane_tab_width() as i64;
        let ctl_arrow = !self.find_symbol_value(sym::CTL_ARROW).is_nil();
        let table = self.window_display_table(win);
        let dp = (!table.is_nil()).then_some(table);
        let selective = self.selective_display_value();
        let selective_rlen = match dp {
            Some(dp) if selective != 0 => {
                let invis = self.char_table(dp).extras[DISP_INVIS_VECTOR_SLOT];
                if self.is_vector(invis) {
                    self.vector_items(invis).len() as i64
                } else {
                    0
                }
            }
            _ => 0,
        };
        let mut next_boundary = from;
        let multibyte = self.cur().text.multibyte;
        let mut wide_column_end_hpos = 0;
        let mut prev_hpos = 0;
        let mut prev_vpos = 0;
        let frame = self.window(win).frame;
        let window_p = self.frame_window_p(frame);
        let base = self.stack.len();
        self.stack.push(win);
        self.stack.push(table);
        if width < 0 {
            width = self.window_body_width(win, WindowBodyUnit::CanonicalChars)?;
            if !window_p {
                width -= 1;
            }
        }
        let continuation_glyph_width = if window_p { 0 } else { 1 };
        let mut pos = from;
        let mut prev_pos = from;
        let mut pos_byte = frombyte;
        let mut prev_pos_byte = frombyte;
        let mut contin_hpos = 0;
        let mut prev_tab_offset = tab_offset;
        let mut cmp_it = CompositionIt {
            id: -1,
            ..CompositionIt::default()
        };
        self.composition_compute_stop_pos(&mut cmp_it, pos, to, Value::NIL, true)?;
        let mut quit_count: u16 = 0;
        'after_loop: loop {
            quit_count = quit_count.wrapping_add(1);
            rarely_quit(self, i64::from(quit_count))?;
            while pos == next_boundary {
                let pos_here = pos;
                if vpos > tovpos || (vpos == tovpos && hpos >= tohpos) {
                    if contin_hpos != 0
                        && prev_hpos == 0
                        && hpos > tohpos
                        && (contin_hpos == width || wide_column_end_hpos > width)
                    {
                        pos = prev_pos;
                        pos_byte = prev_pos_byte;
                        hpos = prev_hpos;
                        vpos = prev_vpos;
                        tab_offset = prev_tab_offset;
                    }
                    break;
                }
                if !did_motion {
                    let ovstr = self.overlay_strings(pos, win);
                    let ovlen = ovstr.len() as i64;
                    hpos += if multibyte && ovlen > 0 {
                        self.strwidth(&ovstr) as i64
                    } else {
                        ovlen
                    };
                }
                did_motion = false;
                if pos >= to {
                    break;
                }
                let newpos;
                (newpos, next_boundary) = self.skip_invisible(pos, to, win)?;
                if newpos >= to {
                    pos = to.min(newpos);
                    pos_byte = self.cur().text.char_to_byte(pos as usize) as i64;
                    break 'after_loop;
                }
                if newpos != pos_here {
                    pos = newpos;
                    pos_byte = self.cur().text.char_to_byte(pos as usize) as i64;
                }
                if newpos > cmp_it.stop_pos && cmp_it.id < 0 {
                    self.composition_reseat_it(&mut cmp_it, pos, to, win, -1, None, Value::NIL)?;
                }
                quit_count = quit_count.wrapping_add(1);
                rarely_quit(self, i64::from(quit_count))?;
            }
            if hpos > width {
                let total_width = width + continuation_glyph_width;
                let mut truncate = false;
                let tpww = self.find_symbol_value(sym::TRUNCATE_PARTIAL_WIDTH_WINDOWS);
                if !tpww.is_nil() && total_width < self.frame(frame).geometry.text_cols {
                    truncate = match tpww.as_fixnum() {
                        Some(n) => total_width < n,
                        None => true,
                    };
                }
                if hscroll != 0 || truncate || !self.find_symbol_value(sym::TRUNCATE_LINES).is_nil()
                {
                    if pos <= to {
                        pos = self.find_before_next_newline(pos as usize, to as usize, 1) as i64;
                        pos_byte = self.cur().text.char_to_byte(pos as usize) as i64;
                        hpos = width;
                        if pos >= next_boundary {
                            next_boundary = pos + 1;
                        }
                        prev_hpos = width;
                        prev_vpos = vpos;
                        prev_tab_offset = tab_offset;
                    }
                } else {
                    prev_tab_offset = tab_offset;
                    if wide_column_end_hpos > width {
                        hpos -= prev_hpos;
                        tab_offset += prev_hpos;
                    } else {
                        tab_offset += width;
                        hpos -= width;
                    }
                    vpos += 1;
                    contin_hpos = prev_hpos;
                    prev_hpos = 0;
                    prev_vpos = vpos;
                }
            }
            if pos > to {
                pos = prev_pos;
                pos_byte = prev_pos_byte;
                hpos = prev_hpos;
                vpos = prev_vpos;
                if contin_hpos != 0
                    && prev_hpos == 0
                    && contin_hpos < width
                    && wide_column_end_hpos == 0
                {
                    hpos = contin_hpos;
                    vpos -= 1;
                }
                break;
            }
            if vpos > tovpos || (vpos == tovpos && hpos >= tohpos) {
                if contin_hpos != 0
                    && prev_hpos == 0
                    && hpos > tohpos
                    && (contin_hpos == width || wide_column_end_hpos > width)
                {
                    pos = prev_pos;
                    pos_byte = prev_pos_byte;
                    hpos = prev_hpos;
                    vpos = prev_vpos;
                }
                break;
            }
            if pos == self.cur().zv as i64 {
                break;
            }
            prev_hpos = hpos;
            prev_vpos = vpos;
            prev_pos = pos;
            prev_pos_byte = pos_byte;
            wide_column_end_hpos = 0;
            if cmp_it.id >= 0
                || (pos == cmp_it.stop_pos
                    && self.composition_reseat_it(
                        &mut cmp_it,
                        pos,
                        to,
                        win,
                        -1,
                        None,
                        Value::NIL,
                    )?)
            {
                self.composition_update_it(&mut cmp_it, pos, pos_byte, Value::NIL);
            }
            if cmp_it.id >= 0 {
                pos += cmp_it.nchars;
                pos_byte += cmp_it.nbytes;
                hpos += cmp_it.width;
                if cmp_it.to == cmp_it.nglyphs {
                    cmp_it.id = -1;
                    self.composition_compute_stop_pos(&mut cmp_it, pos, to, Value::NIL, true)?;
                } else {
                    cmp_it.from = cmp_it.to;
                }
                continue;
            }
            let mut c = i64::from(self.cur().text.byte_at(pos_byte as usize));
            pos += 1;
            pos_byte += 1;
            let charvec = if dp.is_some() && !(multibyte && leading_code_p(c as u8)) {
                self.disp_vector(dp, c as u32)
            } else {
                None
            };
            let n = charvec.map_or(1, |v| self.vector_items(v).len());
            for i in 0..n {
                if let Some(v) = charvec {
                    c = self.display_vector_char(self.vector_items(v)[i]);
                }
                if (0o40..0o177).contains(&c) {
                    hpos += 1;
                } else if c == i64::from(b'\t') {
                    let mut tem =
                        (hpos + tab_offset + hscroll - i64::from(hscroll > 0)) % tab_width;
                    if tem < 0 {
                        tem += tab_width;
                    }
                    hpos += tab_width - tem;
                } else if c == i64::from(b'\n') {
                    if selective > 0 && self.indented_beyond_p(pos, selective)? {
                        if pos < to {
                            loop {
                                pos = self.find_before_next_newline(pos as usize, to as usize, 1)
                                    as i64;
                                if pos < to {
                                    pos += 1;
                                }
                                quit_count = quit_count.wrapping_add(1);
                                rarely_quit(self, i64::from(quit_count))?;
                                if !(pos < to && self.indented_beyond_p(pos, selective)?) {
                                    break;
                                }
                            }
                            if selective_rlen != 0 {
                                hpos += selective_rlen;
                                if hpos >= width {
                                    hpos = width;
                                }
                            }
                            pos -= 1;
                            pos_byte = self.cur().text.char_to_byte(pos as usize) as i64;
                        }
                    } else {
                        vpos += 1;
                        hpos = 0;
                        hpos -= hscroll;
                        if hscroll > 0 {
                            hpos += continuation_glyph_width;
                        }
                        tab_offset = 0;
                    }
                    contin_hpos = 0;
                } else if c == i64::from(b'\r') && selective < 0 {
                    if pos < to {
                        pos = self.find_before_next_newline(pos as usize, to as usize, 1) as i64;
                        pos_byte = self.cur().text.char_to_byte(pos as usize) as i64;
                    }
                    if pos > next_boundary {
                        next_boundary = pos;
                    }
                    if selective_rlen != 0 {
                        hpos += selective_rlen;
                        if hpos >= width {
                            hpos = width;
                        }
                    }
                } else if multibyte && leading_code_p(c as u8) {
                    pos_byte -= 1;
                    let head = self.cur().text.byte_at(pos_byte as usize);
                    let (mb_bytes, mb_width) = self.multibyte_bytes_width(pos_byte as usize, dp);
                    pos_byte += mb_bytes as i64;
                    if mb_width > 1 && bytes_by_char_head(head) == mb_bytes {
                        wide_column_end_hpos = hpos + mb_width;
                    }
                    hpos += mb_width;
                } else if charvec.is_some() {
                    hpos += 1;
                } else {
                    hpos += if ctl_arrow && c < 0o200 { 2 } else { 4 };
                }
            }
        }
        self.stack.truncate(base);
        let contin = contin_hpos != 0 && prev_hpos == 0;
        Ok(Position {
            bufpos: pos,
            bytepos: pos_byte,
            hpos,
            vpos,
            prevhpos: if contin { contin_hpos } else { prev_hpos },
            contin,
        })
    }
}

impl Lisp {
    fn vmotion_line_hidden(
        &mut self,
        prevline: i64,
        selective: i64,
        prop_pos: i64,
        object: Value,
    ) -> LispResult<bool> {
        if selective > 0 && self.indented_beyond_p(prevline, selective)? {
            return Ok(true);
        }
        let propval = self.get_char_property(Value::fixnum(prop_pos), sym::INVISIBLE, object)?;
        Ok(self.text_prop_means_invisible(propval) != 0)
    }

    pub fn vmotion(
        &mut self,
        from: i64,
        from_byte: i64,
        vtarget: i64,
        w: Value,
    ) -> LispResult<Position> {
        let hscroll = self.window(w).hscroll;
        let mut vpos: i64 = 0;
        let lmargin = if hscroll > 0 { 1 - hscroll } else { 0 };
        let selective_display = self.find_symbol_value(sym::SELECTIVE_DISPLAY);
        let selective = match selective_display.as_fixnum() {
            Some(n) => n.max(-1),
            None if selective_display.is_nil() => 0,
            None => -1,
        };
        let text_prop_object = if self.window(w).contents == self.current_buffer {
            w
        } else {
            self.current_buffer
        };
        let begv = self.cur().begv as i64;
        let mut from = from;
        let mut from_byte = from_byte;
        if vpos >= vtarget {
            let mut first = true;
            while (vpos > vtarget || first) && from > begv {
                let mut prevline =
                    self.find_newline((from - 1) as usize, begv as usize, -1).0 as i64;
                while prevline > begv
                    && self.vmotion_line_hidden(
                        prevline,
                        selective,
                        prevline - 1,
                        text_prop_object,
                    )?
                {
                    prevline = self
                        .find_newline((prevline - 1) as usize, begv as usize, -1)
                        .0 as i64;
                }
                let bytepos = self.cur().text.char_to_byte(prevline as usize) as i64;
                let pos = self.compute_motion(
                    prevline,
                    bytepos,
                    0,
                    lmargin,
                    false,
                    from,
                    1 << 15,
                    1 << 15,
                    -1,
                    hscroll,
                    0,
                    w,
                )?;
                vpos -= pos.vpos;
                first = false;
                from = prevline;
                from_byte = bytepos;
            }
            if vpos >= vtarget {
                return Ok(Position {
                    bufpos: from,
                    bytepos: from_byte,
                    hpos: lmargin,
                    vpos,
                    prevhpos: 0,
                    contin: false,
                });
            }
        }
        let (hpos, did_motion) =
            if from > begv && self.cur().text.byte_at((from_byte - 1) as usize) != b'\n' {
                let mut prevline = self.find_newline(from as usize, begv as usize, -1).0 as i64;
                while prevline > begv
                    && self.vmotion_line_hidden(prevline, selective, prevline, text_prop_object)?
                {
                    prevline = self
                        .find_newline((prevline - 1) as usize, begv as usize, -1)
                        .0 as i64;
                }
                let bytepos = self.cur().text.char_to_byte(prevline as usize) as i64;
                let pos = self.compute_motion(
                    prevline,
                    bytepos,
                    0,
                    lmargin,
                    false,
                    from,
                    1 << 15,
                    1 << 15,
                    -1,
                    hscroll,
                    0,
                    w,
                )?;
                (pos.hpos, true)
            } else {
                (lmargin, false)
            };
        let zv = self.cur().zv as i64;
        self.compute_motion(
            from,
            from_byte,
            vpos,
            hpos,
            did_motion,
            zv,
            vtarget,
            -(1 << 15),
            -1,
            hscroll,
            0,
            w,
        )
    }

    pub fn line_number_display_width(&mut self, w: Value) -> LispResult<(i32, i32)> {
        if self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS).is_nil() {
            return Ok((0, 0));
        }
        let count = self.specpdl.len();
        let old_buf = self.current_buffer;
        let start = self.window(w).start;
        let marker = self.marker(start);
        let mut startpos = TextPos::new(marker.charpos as i64, marker.bytepos as i64);
        let start_buffer = marker.buffer;
        let itdata = self.bidi_shelve_cache();
        let contents = self.window(w).contents;
        self.set_buffer_internal(contents);
        let mut saved_restriction = false;
        if startpos.charpos > self.cur().z() as i64
            || !(self.is_buffer(contents) && contents == start_buffer)
        {
            startpos = TextPos::new(self.cur().pt as i64, self.cur().pt_byte as i64);
        }
        if startpos.charpos < self.cur().begv as i64 || startpos.charpos > self.cur().zv as i64 {
            self.record_unwind_restriction();
            self.widen();
            saved_restriction = true;
        }
        let r = self.with_gc_deferred(|lisp| -> LispResult<(i32, i32)> {
            let mut it = It::default();
            lisp.start_display(&mut it, w, startpos)?;
            it.first_visible_x = 0;
            lisp.move_it_by_lines(&mut it, 1)?;
            Ok((it.lnum_width, it.lnum_pixel_width))
        });
        if saved_restriction {
            self.unbind_to(count)?;
        }
        self.set_buffer_internal(old_buf);
        self.bidi_unshelve_cache(itdata, false);
        r
    }

    fn window_column_x(
        &mut self,
        w: Value,
        window: Value,
        col: f64,
        column: Value,
    ) -> LispResult<i32> {
        let f = self.window(w).frame;
        let x = col * self.frame(f).geometry.column_width as f64 + 0.5;
        if !(f64::from(i32::MIN) <= x && x <= f64::from(i32::MAX)) {
            return Err(self.args_out_of_range(window, column));
        }
        Ok(x as i32)
    }

    fn vertical_motion_with_iterator(
        &mut self,
        w: Value,
        window: Value,
        nlines: i64,
        lcols: Value,
        cur_col: Value,
    ) -> LispResult<i64> {
        let mut it_overshoot_count: i64;
        let mut overshoot_handled = false;
        let mut disp_string_at_start_p = false;
        let mut nlines = nlines;
        let mut vpos_init = 0;
        let mut start_col = 0.0;
        let mut start_x = 0;
        let mut to_x = -1;
        let mut ovl_start: i64 = -1;
        let start_x_given = !cur_col.is_nil();
        if start_x_given {
            start_col = crate::floatfns::float_arg(self, cur_col)?;
            start_x = self.window_column_x(w, window, start_col, cur_col)?;
        }
        let (lnum_width, lnum_pixel_width) =
            if self.find_symbol_value(sym::DISPLAY_LINE_NUMBERS).is_nil() {
                (0, 0)
            } else {
                self.line_number_display_width(w)?
            };
        let pt = TextPos::new(self.cur().pt as i64, self.cur().pt_byte as i64);
        let itdata = self.bidi_shelve_cache();
        self.record_unwind(unwind_display_working_on_window, Value::NIL);
        self.xdisp.display_working_on_window_p = true;
        let mut it = It::default();
        self.start_display(&mut it, w, pt)?;
        it.lnum_width = lnum_width;
        let first_x = it.first_visible_x;
        let it_start = it.charpos();
        let begv = self.cur().begv as i64;
        if it.cmp_it.id >= 0 {
            it_overshoot_count = 0;
        } else if it.method == ItMethod::FromString {
            let ovl_idx = if it.current.overlay_string_index >= 0 {
                it.current.overlay_string_index % OVERLAY_STRING_CHUNK_SIZE as i64
            } else {
                -1
            };
            if !it.string_from_display_prop_p && ovl_idx >= 0 {
                ovl_start = self.overlay(it.string_overlays[ovl_idx as usize]).start as i64;
            }
            disp_string_at_start_p = it.area == GlyphRowArea::Text
                && (it.string_from_display_prop_p
                    || (ovl_start >= begv && {
                        let prop = self.get_char_property(
                            Value::fixnum(ovl_start),
                            sym::INVISIBLE,
                            window,
                        )?;
                        self.text_prop_means_invisible(prop) != 0
                    }))
                && (it.sp > 0 && it.stack[(it.sp - 1) as usize].method == ItMethod::FromBuffer);
            it_overshoot_count = self
                .string_bytes(it.string)
                .iter()
                .filter(|&&b| b == b'\n')
                .count() as i64;
            if it_overshoot_count == 0 {
                it_overshoot_count = -1;
            }
        } else {
            it_overshoot_count = i64::from(
                !((it.method == ItMethod::FromImage && it.image_id >= 0)
                    || it.method == ItMethod::FromStretch),
            );
        }
        if start_x_given {
            it.hpos = start_col as i32;
            it.current_x = start_x;
        } else {
            self.reseat_at_previous_visible_line_start(&mut it)?;
            it.current_x = 0;
            it.hpos = 0;
        }
        if it.charpos() != pt.charpos {
            let target = if !disp_string_at_start_p
                || self.cur().text.byte_at(it.bytepos() as usize) == b'\n'
            {
                pt.charpos
            } else {
                pt.charpos - 1
            };
            self.move_it_to(&mut it, target, -1, -1, -1, MOVE_TO_POS)?;
        }
        if it.charpos() > it_start {
            if it_overshoot_count < 0 && it.method == ItMethod::FromBuffer && it.c == 10 {
                it_overshoot_count = 1;
            } else if it_overshoot_count == 1 && it.vpos == 0 && it.current_x < it.last_visible_x {
                it_overshoot_count = 0;
            } else if disp_string_at_start_p && it.vpos > 0 {
                it_overshoot_count = i64::from(it.vpos);
            }
            if it.line_wrap == LineWrap::Truncate
                && it.current_x >= it.last_visible_x
                && it_overshoot_count == 0
                && it.vpos > 0
            {
                it_overshoot_count = 1;
            }
            if it_overshoot_count > 0 {
                self.move_it_by_lines(&mut it, -it_overshoot_count)?;
            }
            overshoot_handled = true;
        } else if it.charpos() == pt.charpos - 1
            && self.cur().text.byte_at((pt.bytepos - 1) as usize) == b'\n'
            && nlines <= 0
        {
            nlines += 1;
            vpos_init = -1;
            overshoot_handled = true;
        }
        if !lcols.is_nil() {
            let col = self.xfloatint(lcols);
            to_x = self.window_column_x(w, window, col, lcols)? + lnum_pixel_width;
        }
        if nlines <= 0 {
            it.vpos = vpos_init;
            it.current_y = 0;
            if (nlines < 0 && it.charpos() > self.cur().begv as i64)
                || (nlines == 0 && !(start_x_given && start_x <= to_x))
            {
                self.move_it_by_lines(&mut it, nlines)?;
            }
            if it.charpos() == it_start
                && disp_string_at_start_p
                && ovl_start >= self.cur().begv as i64
                && it_overshoot_count > 0
            {
                self.move_it_by_lines(&mut it, -it_overshoot_count)?;
            }
        } else if overshoot_handled {
            it.vpos = vpos_init;
            it.current_y = 0;
            self.move_it_by_lines(&mut it, nlines)?;
        } else if it_start < self.cur().zv as i64 {
            if if it.bidi_it.scan_dir >= 0 || it.vpos == vpos_init {
                it.charpos() < it_start
            } else {
                it.charpos() > it_start
            } {
                it.vpos = 0;
                it.current_y = 0;
                self.move_it_by_lines(&mut it, 1)?;
            }
            while it.charpos() == it_start {
                it.vpos = 0;
                it.current_y = 0;
                self.move_it_by_lines(&mut it, 1)?;
            }
            if nlines > 1 {
                self.move_it_by_lines(&mut it, nlines - 1)?;
            }
        } else {
            it.vpos = 0;
            it.current_y = 0;
            self.move_it_by_lines(&mut it, nlines)?;
            if it.charpos() == pt.charpos && pt.charpos == it_start {
                it.vpos = 0;
            }
        }
        if !lcols.is_nil() {
            if it.continuation_lines_width <= 0
                && it.hpos > 0
                && it.method == ItMethod::FromString
                && !it.from_overlay.is_nil()
            {
                self.reseat_at_previous_visible_line_start(&mut it)?;
            }
            let zv = self.cur().zv as i64;
            self.move_it_in_display_line(&mut it, zv, first_x + to_x, MOVE_TO_X)?;
            if nlines >= 0 && it.area == GlyphRowArea::Text {
                while it.method == ItMethod::FromString
                    && !it.string_from_display_prop_p
                    && self.string_bytes(it.string)[it.string_bytepos() as usize..].contains(&b'\n')
                {
                    self.move_it_by_lines(&mut it, 1)?;
                    let zv = self.cur().zv as i64;
                    self.move_it_in_display_line(&mut it, zv, first_x + to_x, MOVE_TO_X)?;
                }
            }
        }
        self.goto_char_internal(it.charpos() as usize);
        self.bidi_unshelve_cache(itdata, false);
        Ok(i64::from(it.vpos))
    }
}

pub(crate) fn unwind_display_working_on_window(lisp: &mut Lisp, _arg: Value) -> LispResult<()> {
    lisp.xdisp.display_working_on_window_p = false;
    Ok(())
}

fn restore_window_buffer(lisp: &mut Lisp, list: Value) -> LispResult<()> {
    let w = lisp.decode_live_window(lisp.xcar(list))?;
    let list = lisp.xcdr(list);
    let buffer = lisp.xcar(list);
    lisp.wset_buffer(w, buffer);
    let list = lisp.xcdr(list);
    let charpos = lisp.xcar(list).as_fixnum().unwrap_or(1) as usize;
    let pointm = lisp.window(w).pointm;
    let contents = lisp.window(w).contents;
    lisp.set_marker_position(pointm, charpos, contents);
    Ok(())
}

fn f_line_number_display_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pixelwise = args[0];
    let w = lisp.windows.selected_window;
    let (width, pixel_width) = lisp.line_number_display_width(w)?;
    if pixelwise == sym::COLUMNS {
        let f = lisp.window(w).frame;
        let column_width = lisp.frame(f).geometry.column_width as f64;
        return Ok(lisp.float(f64::from(pixel_width) / column_width));
    }
    if !pixelwise.is_nil() {
        return Ok(Value::fixnum(i64::from(pixel_width)));
    }
    Ok(Value::fixnum(i64::from(width)))
}

pub(crate) fn f_vertical_motion(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (mut lines, window, cur_col) = (args[0], args[1], args[2]);
    let mut lcols = Value::NIL;
    let count = lisp.specpdl.len();
    if lines.is_cons() {
        lcols = lisp.xcar(lines);
        if !lisp.is_number(lcols) {
            return Err(lisp.wrong_type(sym::NUMBERP, lcols));
        }
        lines = lisp.xcdr(lines);
    }
    let nlines = lisp.check_fixnum(lines)?;
    let w = lisp.decode_live_window(window)?;
    let contents = lisp.window(w).contents;
    if contents != lisp.current_buffer {
        let pointm = lisp.window(w).pointm;
        let (charpos, bytepos) = (lisp.marker(pointm).charpos, lisp.marker(pointm).bytepos);
        let old = lisp.list(&[
            window,
            contents,
            Value::fixnum(charpos as i64),
            Value::fixnum(bytepos as i64),
        ]);
        lisp.record_unwind(restore_window_buffer, old);
        let buffer = lisp.current_buffer;
        lisp.wset_buffer(w, buffer);
        let pt = lisp.cur().pt;
        lisp.set_marker_position(pointm, pt, buffer);
    }
    let vpos = if lisp.noninteractive {
        let (pt, pt_byte) = (lisp.cur().pt as i64, lisp.cur().pt_byte as i64);
        let pos = lisp.vmotion(pt, pt_byte, nlines, w)?;
        lisp.goto_char_internal(pos.bufpos as usize);
        pos.vpos
    } else {
        lisp.with_gc_deferred(|lisp| {
            lisp.vertical_motion_with_iterator(w, window, nlines, lcols, cur_col)
        })?
    };
    lisp.unbind_to(count)?;
    Ok(Value::fixnum(vpos))
}

fn f_compute_motion(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (from, frompos, to, topos, width, offsets, window) = (
        args[0], args[1], args[2], args[3], args[4], args[5], args[6],
    );
    let from = lisp.fix_position(from)?;
    lisp.check_cons(frompos)?;
    let fromhpos = lisp.check_fixnum(lisp.xcar(frompos))?;
    let fromvpos = lisp.check_fixnum(lisp.xcdr(frompos))?;
    let to = lisp.fix_position(to)?;
    let mut target = None;
    if !topos.is_nil() {
        lisp.check_cons(topos)?;
        let tohpos = lisp.check_fixnum(lisp.xcar(topos))?;
        let tovpos = lisp.check_fixnum(lisp.xcdr(topos))?;
        target = Some((tohpos, tovpos));
    }
    let width = if width.is_nil() {
        -1
    } else {
        lisp.check_fixnum(width)?
    };
    let (hscroll, tab_offset) = if offsets.is_nil() {
        (0, 0)
    } else {
        lisp.check_cons(offsets)?;
        let hscroll = lisp.check_fixnum(lisp.xcar(offsets))?;
        let tab_offset = lisp.check_fixnum(lisp.xcdr(offsets))?;
        if !(0 <= hscroll && (0..=INT_MAX).contains(&tab_offset)) {
            let (a, b) = (lisp.xcar(offsets), lisp.xcdr(offsets));
            return Err(lisp.args_out_of_range(a, b));
        }
        (hscroll, tab_offset)
    };
    let w = lisp.decode_live_window(window)?;
    let (begv, zv) = (lisp.cur().begv as i64, lisp.cur().zv as i64);
    for pos in [from, to] {
        if pos < begv || pos > zv {
            return Err(lisp.args_out_of_range3(
                Value::fixnum(pos),
                Value::fixnum(begv),
                Value::fixnum(zv),
            ));
        }
    }
    let (tohpos, tovpos) = match target {
        Some(target) => target,
        None => {
            let body = lisp.window_body_width(w, WindowBodyUnit::CanonicalChars)?;
            let window_p = lisp.frame_window_p(lisp.window(w).frame);
            (
                body - if window_p { 0 } else { 1 },
                lisp.window_internal_height(w),
            )
        }
    };
    let frombyte = lisp.cur().text.char_to_byte(from as usize) as i64;
    let pos = lisp.compute_motion(
        from, frombyte, fromvpos, fromhpos, false, to, tovpos, tohpos, width, hscroll, tab_offset,
        w,
    )?;
    let contin = lisp.bool(pos.contin);
    Ok(lisp.list(&[
        Value::fixnum(pos.bufpos),
        Value::fixnum(pos.hpos),
        Value::fixnum(pos.vpos),
        Value::fixnum(pos.prevhpos),
        contin,
    ]))
}

fn f_current_column(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(lisp.current_column()?))
}

fn f_indent_to(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let column = lisp.check_fixnum(args[0])?;
    let minimum = if args[1].is_nil() {
        0
    } else {
        lisp.check_fixnum(args[1])?
    };
    Ok(Value::fixnum(lisp.indent_to(column, minimum)?))
}

fn f_current_indentation(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let (pt, begv) = (lisp.cur().pt, lisp.cur().begv);
    let pos = lisp.find_newline(pt, begv, -1).0;
    let pos_byte = lisp.cur().text.char_to_byte(pos) as i64;
    Ok(Value::fixnum(lisp.position_indentation(pos_byte)?))
}

pub(crate) fn f_move_to_column(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (column, force) = (args[0], args[1]);
    let goal = match column.as_fixnum() {
        Some(n) if n >= 0 => n,
        _ => return Err(lisp.wrong_type(sym::WHOLENUMP, column)),
    };
    let zv = lisp.cur().zv as i64;
    let scan = lisp.scan_for_column(Some(zv), Some(goal))?;
    let mut col = scan.goalcol;
    lisp.goto_char_internal(scan.endpos as usize);
    if !force.is_nil() && col > goal {
        let c = lisp.cur().text.char_at_byte(scan.prev_bpos as usize);
        if c == b'\t' as u32 && scan.prev_col < goal && scan.prev_bpos < lisp.cur().pt_byte as i64 {
            lisp.goto_char_internal(scan.prev_pos as usize);
            f_insert_char(
                lisp,
                &[
                    Value::fixnum(b' ' as i64),
                    Value::fixnum(goal - scan.prev_col),
                    sym::T,
                ],
            )?;
            let pt = lisp.cur().pt;
            lisp.del_range(pt, pt + 1)?;
            let goal_pt = lisp.cur().pt;
            lisp.indent_to(col, 0)?;
            lisp.goto_char_internal(goal_pt);
            col = goal;
        }
    }
    if col < goal && force == sym::T {
        col = goal;
        lisp.indent_to(col, 0)?;
    }
    lisp.record_known_column(col);
    Ok(Value::fixnum(col))
}
