use crate::charfns::f_string_width;
use crate::composite::CompositionIt;
use crate::editfns::f_insert_char;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::textprop::{f_next_property_change, f_next_single_property_change};
use crate::value::{MOST_POSITIVE_FIXNUM, Value};
use crate::window::WindowLoop;

const MAX_FACE_ID: i64 = (1 << 20) - 1;
const DISP_TABLE_EXTRA_SLOTS: usize = 6;
const INT_MAX: i64 = i32::MAX as i64;

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
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_bool(sym::INDENT_TABS_MODE, true);
}

fn sanitize_char_width(width: i64) -> i64 {
    if (0..=1000).contains(&width) {
        width
    } else {
        1000
    }
}

fn leading_code_p(byte: u8) -> bool {
    byte & 0xC0 == 0xC0
}

fn bytes_by_char_head(byte: u8) -> usize {
    if byte & 0x80 == 0 {
        1
    } else if byte & 0x20 == 0 {
        2
    } else if byte & 0x10 == 0 {
        3
    } else if byte & 0x08 == 0 {
        4
    } else {
        5
    }
}

pub struct ColumnScan {
    pub endpos: i64,
    pub goalcol: i64,
    pub prev_pos: i64,
    pub prev_bpos: i64,
    pub prev_col: i64,
}

impl Lisp {
    fn disp_table_p(&self, obj: Value) -> bool {
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

    fn glyph_code_p(&self, gc: Value) -> bool {
        if gc.is_cons() {
            matches!(self.xcar(gc).as_fixnum(), Some(c) if (0..=crate::character::MAX_CHAR as i64).contains(&c))
                && matches!(self.xcdr(gc).as_fixnum(), Some(f) if (0..=MAX_FACE_ID).contains(&f))
        } else {
            let max = (MAX_FACE_ID << 22) | crate::character::MAX_CHAR as i64;
            matches!(gc.as_fixnum(), Some(n) if (0..=max).contains(&n))
        }
    }

    fn glyph_code_char(&self, gc: Value) -> i64 {
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

    pub fn skip_invisible(&mut self, pos: i64, to: i64) -> LispResult<(i64, i64)> {
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
        let prop = self.get_char_property(position, sym::INVISIBLE, buffer)?;
        if self.text_prop_means_invisible(prop) == 1 {
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
                    (scan, next_boundary) = self.skip_invisible(scan, end)?;
                    if scan != old_scan {
                        scan_byte = self.cur().text.char_to_byte(scan as usize) as i64;
                    }
                    if scan >= end {
                        break 'endloop;
                    }
                    if scan > cmp_it.stop_pos && cmp_it.id < 0 {
                        self.composition_reseat_it(&mut cmp_it, scan, end, window, -1, Value::NIL)?;
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

    fn position_indentation(&mut self, pos_byte: i64) -> LispResult<i64> {
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
                let (pos, next_boundary) = self.skip_invisible(pos, zv)?;
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

fn f_move_to_column(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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
