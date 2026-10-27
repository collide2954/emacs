use std::io::Write;

use num_bigint::BigInt;
use num_traits::Signed;

use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::{LispString, PropRun};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("char-to-string", 1, 1, f_char_to_string),
    Subr::fixed("byte-to-string", 1, 1, f_byte_to_string),
    Subr::fixed("string-to-char", 1, 1, f_string_to_char),
    Subr::fixed("point", 0, 0, f_point),
    Subr::fixed("point-marker", 0, 0, f_point_marker),
    Subr::fixed("goto-char", 1, 1, f_goto_char).interactive("NGoto char: "),
    Subr::fixed("region-beginning", 0, 0, f_region_beginning),
    Subr::fixed("region-end", 0, 0, f_region_end),
    Subr::fixed("mark-marker", 0, 0, f_mark_marker),
    Subr::fixed("pos-bol", 0, 1, f_pos_bol),
    Subr::fixed("line-beginning-position", 0, 1, f_line_beginning_position),
    Subr::fixed("pos-eol", 0, 1, f_pos_eol),
    Subr::fixed("line-end-position", 0, 1, f_line_end_position),
    Subr::special("save-excursion", 0, f_save_excursion),
    Subr::special("save-current-buffer", 0, f_save_current_buffer),
    Subr::fixed("buffer-size", 0, 1, f_buffer_size),
    Subr::fixed("point-min", 0, 0, f_point_min),
    Subr::fixed("point-min-marker", 0, 0, f_point_min_marker),
    Subr::fixed("point-max", 0, 0, f_point_max),
    Subr::fixed("point-max-marker", 0, 0, f_point_max_marker),
    Subr::fixed("gap-position", 0, 0, f_gap_position),
    Subr::fixed("gap-size", 0, 0, f_gap_size),
    Subr::fixed("position-bytes", 1, 1, f_position_bytes),
    Subr::fixed("byte-to-position", 1, 1, f_byte_to_position),
    Subr::fixed("following-char", 0, 0, f_following_char),
    Subr::fixed("preceding-char", 0, 0, f_preceding_char),
    Subr::fixed("bobp", 0, 0, f_bobp),
    Subr::fixed("eobp", 0, 0, f_eobp),
    Subr::fixed("bolp", 0, 0, f_bolp),
    Subr::fixed("eolp", 0, 0, f_eolp),
    Subr::fixed("char-after", 0, 1, f_char_after),
    Subr::fixed("char-before", 0, 1, f_char_before),
    Subr::fixed("user-login-name", 0, 1, f_user_login_name),
    Subr::fixed("user-real-login-name", 0, 0, f_user_real_login_name),
    Subr::fixed("user-uid", 0, 0, f_user_uid),
    Subr::fixed("user-real-uid", 0, 0, f_user_uid),
    Subr::fixed("group-name", 1, 1, f_group_name),
    Subr::fixed("group-gid", 0, 0, f_user_uid),
    Subr::fixed("group-real-gid", 0, 0, f_user_uid),
    Subr::fixed("user-full-name", 0, 1, f_user_full_name),
    Subr::fixed("system-name", 0, 0, f_system_name),
    Subr::fixed("emacs-pid", 0, 0, f_emacs_pid),
    Subr::many("insert", 0, f_insert),
    Subr::many("insert-and-inherit", 0, f_insert_and_inherit),
    Subr::many("insert-before-markers", 0, f_insert_before_markers),
    Subr::many(
        "insert-before-markers-and-inherit",
        0,
        f_insert_before_markers_and_inherit,
    ),
    Subr::fixed("insert-char", 1, 3, f_insert_char)
        .interactive("(list (read-char-by-name \"Insert character (Unicode name or hex): \") (prefix-numeric-value current-prefix-arg) t))"),
    Subr::fixed("insert-byte", 2, 3, f_insert_byte),
    Subr::fixed("buffer-substring", 2, 2, f_buffer_substring),
    Subr::fixed(
        "buffer-substring-no-properties",
        2,
        2,
        f_buffer_substring_no_properties,
    ),
    Subr::fixed("buffer-string", 0, 0, f_buffer_string),
    Subr::fixed("insert-buffer-substring", 1, 3, f_insert_buffer_substring),
    Subr::fixed("compare-buffer-substrings", 6, 6, f_compare_buffer_substrings),
    Subr::fixed("subst-char-in-region", 4, 5, f_subst_char_in_region),
    Subr::fixed("delete-region", 2, 2, f_delete_region).interactive("r"),
    Subr::fixed("delete-and-extract-region", 2, 2, f_delete_and_extract_region),
    Subr::fixed("widen", 0, 0, f_widen).interactive(""),
    Subr::fixed("narrow-to-region", 2, 2, f_narrow_to_region).interactive("r"),
    Subr::fixed(
        "internal--labeled-narrow-to-region",
        3,
        3,
        f_labeled_narrow_to_region,
    ),
    Subr::fixed("internal--labeled-widen", 1, 1, f_labeled_widen),
    Subr::special("save-restriction", 0, f_save_restriction),
    Subr::fixed("ngettext", 3, 3, f_ngettext),
    Subr::many("message", 1, f_message),
    Subr::many("message-box", 1, f_message),
    Subr::many("message-or-box", 1, f_message),
    Subr::fixed("current-message", 0, 0, f_current_message),
    Subr::many("propertize", 1, f_propertize),
    Subr::many("format", 1, f_format),
    Subr::many("format-message", 1, f_format_message),
    Subr::fixed("char-equal", 2, 2, f_char_equal),
    Subr::fixed("transpose-regions", 4, 5, f_transpose_regions),
    Subr::fixed("forward-char", 0, 1, f_forward_char).interactive("^p"),
    Subr::fixed("backward-char", 0, 1, f_backward_char).interactive("^p"),
    Subr::fixed("forward-line", 0, 1, f_forward_line).interactive("^p"),
    Subr::fixed("beginning-of-line", 0, 1, f_beginning_of_line).interactive("^p"),
    Subr::fixed("end-of-line", 0, 1, f_end_of_line).interactive("^p"),
    Subr::fixed("delete-char", 1, 2, f_delete_char).interactive("p\nP"),
    Subr::fixed("text-quoting-style", 0, 0, f_text_quoting_style),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::DEACTIVATE_MARK, Value::NIL);
    lisp.defvar(sym::TEXT_QUOTING_STYLE, Value::NIL);
    lisp.defvar(sym::TEXT_PROPERTY_DEFAULT_NONSTICKY, Value::NIL);
    let alist = lisp.list(&[]);
    let pair1 = lisp.cons(sym::SYNTAX_TABLE, sym::T);
    let pair2 = lisp.cons(sym::DISPLAY, sym::T);
    let pair3 = lisp.cons(sym::COMPOSITION, sym::T);
    let pair4 = lisp.cons(sym::CURSOR, sym::T);
    let _ = alist;
    let nonsticky = lisp.list(&[pair1, pair2, pair3, pair4]);
    lisp.set_plain_value(sym::TEXT_PROPERTY_DEFAULT_NONSTICKY, nonsticky);
    lisp.defvar(sym::CHAR_PROPERTY_ALIAS_ALIST, Value::NIL);
    lisp.defvar(sym::DEFAULT_TEXT_PROPERTIES, Value::NIL);
    lisp.defvar(sym::INHIBIT_POINT_MOTION_HOOKS, sym::T);
    lisp.defvar(sym::INHIBIT_FIELD_TEXT_MOTION, Value::NIL);
    lisp.defvar(sym::BUFFER_ACCESS_FONTIFY_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::BUFFER_ACCESS_FONTIFIED_PROPERTY, Value::NIL);
    lisp.defvar(sym::SYSTEM_NAME, Value::NIL);
    lisp.defvar(sym::USER_FULL_NAME, Value::NIL);
    lisp.defvar(sym::USER_LOGIN_NAME, Value::NIL);
    lisp.defvar(sym::USER_REAL_LOGIN_NAME, Value::NIL);
    lisp.defvar(sym::OPERATING_SYSTEM_RELEASE, Value::NIL);
    lisp.defvar(sym::UNDO_AUTO__LAST_BOUNDARY_CAUSE, Value::NIL);
    lisp.defvar(sym::MESSAGE_LOG_MAX, Value::fixnum(1000));
}

impl Lisp {
    pub fn char_at_pos(&self, pos: usize) -> u32 {
        let b = self.cur();
        let byte = b.text.char_to_byte(pos);
        let c = b.text.char_at_byte(byte);
        if !b.text.multibyte && c >= 0x80 {
            return character::byte8_to_char(c as u8);
        }
        c
    }

    pub fn goto_char_internal(&mut self, pos: usize) {
        let b = self.cur_mut();
        let pos = pos.clamp(b.begv, b.zv);
        b.pt = pos;
        b.pt_byte = b.text.char_to_byte(pos);
    }

    pub fn widen(&mut self) {
        let b = self.cur_mut();
        b.begv = 1;
        b.begv_byte = 1;
        b.zv = b.z();
        b.zv_byte = b.z_byte();
    }

    pub fn narrow(&mut self, start: usize, end: usize) {
        let b = self.cur_mut();
        b.begv = start;
        b.begv_byte = b.text.char_to_byte(start);
        b.zv = end;
        b.zv_byte = b.text.char_to_byte(end);
        let pt = b.pt.clamp(start, end);
        b.pt = pt;
        b.pt_byte = b.text.char_to_byte(pt);
    }

    pub fn validate_region(&mut self, start: Value, end: Value) -> LispResult<(usize, usize)> {
        let s = self.position_arg(start)?;
        let e = self.position_arg(end)?;
        let (s, e) = if s <= e { (s, e) } else { (e, s) };
        let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
        if s < begv || e > zv {
            return Err(self.args_out_of_range(start, end));
        }
        Ok((s as usize, e as usize))
    }

    pub fn find_newline(&self, start: usize, end: usize, count: i64) -> (usize, i64) {
        let original = count;
        let mut count = count;
        if count > 0 {
            let mut p = start;
            while p < end {
                if self.char_at_pos(p) == 10 {
                    count -= 1;
                    if count == 0 {
                        return (p + 1, original);
                    }
                }
                p += 1;
            }
            return (end, original - count);
        }
        if count < 0 {
            let mut p = start;
            while p > end {
                p -= 1;
                if self.char_at_pos(p) == 10 {
                    count += 1;
                    if count >= 0 {
                        return (p + 1, original);
                    }
                }
            }
            return (end, original - count);
        }
        (start, 0)
    }

    pub fn scan_newline_from_point(&self, count: i64) -> (usize, i64) {
        let b = self.cur();
        if count <= 0 {
            self.find_newline(b.pt, b.begv, count - 1)
        } else {
            self.find_newline(b.pt, b.zv, count)
        }
    }

    pub fn line_beginning(&self, n: i64) -> usize {
        self.scan_newline_from_point(n - 1).0
    }

    pub fn line_end(&self, n: i64) -> usize {
        let b = self.cur();
        let cnt = n - i64::from(n <= 0);
        let end = if cnt > 0 { b.zv } else { b.begv };
        let (pos, counted) = self.find_newline(b.pt, end, cnt);
        if counted == cnt { pos - 1 } else { pos }
    }
    pub fn save_excursion_save(&mut self) -> Value {
        let buffer = self.current_buffer;
        let pt = self.cur().pt;
        self.make_marker_at(pt, buffer, false)
    }

    pub fn current_message_value(&self) -> Value {
        self.plain_value(sym::CURRENT_MESSAGE_INTERNAL)
    }

    pub fn message_string(&mut self, s: Value) -> LispResult<()> {
        if self.noninteractive() {
            let text = self.string_to_rust(s);
            let _ = self.stderr.write_all(text.as_bytes());
            let _ = self.stderr.write_all(b"\n");
            let _ = self.stderr.flush();
        }
        self.set_plain_value(sym::CURRENT_MESSAGE_INTERNAL, s);
        Ok(())
    }

    pub fn text_quoting_style(&self) -> Value {
        let v = self.plain_value(sym::TEXT_QUOTING_STYLE);
        if v.is_nil() || v == sym::UNBOUND {
            return sym::CURVE;
        }
        v
    }

    pub fn format_string(&mut self, args: &[Value], message: bool) -> LispResult {
        styled_format(self, args, message)
    }
}

pub fn restore_excursion(lisp: &mut Lisp, marker: Value) -> LispResult<()> {
    let buffer = lisp.marker(marker).buffer;
    if !lisp.buffer_live_p(buffer) {
        return Ok(());
    }
    lisp.set_buffer_internal(buffer);
    let pos = lisp.marker(marker).charpos;
    lisp.goto_char_internal(pos);
    lisp.unchain_marker(marker);
    Ok(())
}

fn restore_restriction(lisp: &mut Lisp, data: Value) -> LispResult<()> {
    if lisp.is_buffer(data) {
        if lisp.buffer_live_p(data) {
            let count = lisp.specpdl.len();
            lisp.record_unwind_current_buffer();
            lisp.set_buffer_internal(data);
            lisp.widen();
            lisp.unbind_to(count)?;
        }
        return Ok(());
    }
    let beg = lisp.xcar(data);
    let end = lisp.xcdr(data);
    let buffer = lisp.marker(beg).buffer;
    if !lisp.buffer_live_p(buffer) {
        return Ok(());
    }
    let (b, e) = (lisp.marker(beg).charpos, lisp.marker(end).charpos);
    let count = lisp.specpdl.len();
    lisp.record_unwind_current_buffer();
    lisp.set_buffer_internal(buffer);
    let z = lisp.cur().z();
    lisp.narrow(b.min(z), e.min(z).max(b.min(z)));
    lisp.unchain_marker(beg);
    lisp.unchain_marker(end);
    lisp.unbind_to(count)
}

fn f_char_to_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    if c < 0x80 {
        return Ok(lisp.unibyte_string(&[c as u8]));
    }
    Ok(lisp.string_from_chars(&[c], true))
}

fn f_byte_to_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.check_fixnum(args[0])?;
    if !(0..=255).contains(&b) {
        return Err(lisp.error("Invalid byte"));
    }
    Ok(lisp.unibyte_string(&[b as u8]))
}

fn f_string_to_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.check_string(args[0])?;
    let chars = lisp.string_char_vec(s);
    Ok(Value::fixnum(chars.first().copied().unwrap_or(0) as i64))
}

fn f_point(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(lisp.cur().pt as i64))
}

fn f_point_marker(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let (pt, b) = (lisp.cur().pt, lisp.current_buffer);
    Ok(lisp.make_marker_at(pt, b, false))
}

fn f_goto_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let position = args[0];
    if lisp.is_marker(position) {
        let m = *lisp.marker(position);
        if m.buffer == lisp.current_buffer {
            lisp.goto_char_internal(m.charpos);
            return Ok(position);
        }
        let pos = lisp.marker_position_checked(position)?;
        lisp.goto_char_internal(pos);
        return Ok(position);
    }
    let pos = match position.as_fixnum() {
        Some(n) => n,
        None if lisp.is_bignum(position) => {
            if lisp.bignum_ref(position).is_some_and(|b| b.is_negative()) {
                0
            } else {
                i64::MAX
            }
        }
        None => return Err(lisp.wrong_type(sym::INTEGER_OR_MARKER_P, position)),
    };
    let (begv, zv) = (lisp.cur().begv as i64, lisp.cur().zv as i64);
    lisp.goto_char_internal(pos.clamp(begv, zv) as usize);
    Ok(position)
}

fn region_limit(lisp: &mut Lisp, beginning: bool) -> LispResult {
    let transient = lisp.find_symbol_value(sym::TRANSIENT_MARK_MODE);
    let active = lisp.find_symbol_value(sym::MARK_ACTIVE);
    let even_if_inactive = lisp.find_symbol_value(sym::MARK_EVEN_IF_INACTIVE);
    if !transient.is_nil() && even_if_inactive.is_nil() && active.is_nil() {
        return Err(lisp.signal(sym::MARK_INACTIVE, Value::NIL));
    }
    let mark = lisp.cur().mark;
    let m = *lisp.marker(mark);
    if m.buffer.is_nil() {
        return Err(lisp.error("The mark is not set now, so there is no region"));
    }
    let pt = lisp.cur().pt;
    let mpos = m.charpos.clamp(lisp.cur().begv, lisp.cur().zv);
    Ok(Value::fixnum(if beginning {
        pt.min(mpos)
    } else {
        pt.max(mpos)
    } as i64))
}

fn f_region_beginning(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    region_limit(lisp, true)
}

fn f_region_end(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    region_limit(lisp, false)
}

fn f_mark_marker(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.cur().mark)
}

fn line_count_arg(lisp: &mut Lisp, n: Value) -> LispResult<i64> {
    if n.is_nil() {
        Ok(1)
    } else {
        lisp.check_fixnum(n)
    }
}

fn f_pos_bol(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = line_count_arg(lisp, args[0])?;
    Ok(Value::fixnum(lisp.line_beginning(n) as i64))
}

fn f_line_beginning_position(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    f_pos_bol(lisp, args)
}

fn f_pos_eol(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = line_count_arg(lisp, args[0])?;
    Ok(Value::fixnum(lisp.line_end(n) as i64))
}

fn f_line_end_position(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    f_pos_eol(lisp, args)
}

fn f_save_excursion(lisp: &mut Lisp, args: Value) -> LispResult {
    let count = lisp.specpdl.len();
    let marker = lisp.save_excursion_save();
    lisp.record_unwind(restore_excursion, marker);
    let r = lisp.progn(args);
    lisp.unbind_to_value(count, r)
}

fn f_save_current_buffer(lisp: &mut Lisp, args: Value) -> LispResult {
    let count = lisp.specpdl.len();
    lisp.record_unwind_current_buffer();
    let r = lisp.progn(args);
    lisp.unbind_to_value(count, r)
}

fn f_buffer_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.decode_buffer(args[0])?;
    Ok(Value::fixnum(lisp.buffer(b).text.chars as i64))
}

fn f_point_min(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(lisp.cur().begv as i64))
}

fn f_point_min_marker(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let (p, b) = (lisp.cur().begv, lisp.current_buffer);
    Ok(lisp.make_marker_at(p, b, false))
}

fn f_point_max(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(lisp.cur().zv as i64))
}

fn f_point_max_marker(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let (p, b) = (lisp.cur().zv, lisp.current_buffer);
    Ok(lisp.make_marker_at(p, b, false))
}

fn f_gap_position(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(lisp.cur().z() as i64))
}

fn f_gap_size(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(0))
}

fn f_position_bytes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = lisp.position_arg(args[0])?;
    let b = lisp.cur();
    if pos < 1 || pos as usize > b.z() {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(b.text.char_to_byte(pos as usize) as i64))
}

fn f_byte_to_position(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let byte = lisp.check_fixnum(args[0])?;
    let b = lisp.cur();
    if byte < 1 || byte as usize > b.z_byte() {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(b.text.byte_to_char(byte as usize) as i64))
}

fn f_following_char(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let b = lisp.cur();
    if b.pt >= b.zv {
        return Ok(Value::fixnum(0));
    }
    let pt = b.pt;
    Ok(Value::fixnum(lisp.char_at_pos(pt) as i64))
}

fn f_preceding_char(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let b = lisp.cur();
    if b.pt <= b.begv {
        return Ok(Value::fixnum(0));
    }
    let pt = b.pt;
    Ok(Value::fixnum(lisp.char_at_pos(pt - 1) as i64))
}

fn f_bobp(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let b = lisp.cur();
    Ok(lisp.bool(b.pt == b.begv))
}

fn f_eobp(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let b = lisp.cur();
    Ok(lisp.bool(b.pt == b.zv))
}

fn f_bolp(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let b = lisp.cur();
    let (pt, begv) = (b.pt, b.begv);
    Ok(lisp.bool(pt == begv || lisp.char_at_pos(pt - 1) == 10))
}

fn f_eolp(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let b = lisp.cur();
    let (pt, zv) = (b.pt, b.zv);
    Ok(lisp.bool(pt == zv || lisp.char_at_pos(pt) == 10))
}

fn f_char_after(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = if args[0].is_nil() {
        lisp.cur().pt as i64
    } else if lisp.is_marker(args[0]) {
        lisp.marker_position_checked(args[0])? as i64
    } else if let Some(n) = args[0].as_fixnum() {
        n
    } else if lisp.is_bignum(args[0]) {
        return Ok(Value::NIL);
    } else {
        return Err(lisp.wrong_type(sym::INTEGER_OR_MARKER_P, args[0]));
    };
    let b = lisp.cur();
    if pos < b.begv as i64 || pos >= b.zv as i64 {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(lisp.char_at_pos(pos as usize) as i64))
}

fn f_char_before(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = if args[0].is_nil() {
        lisp.cur().pt as i64
    } else if lisp.is_marker(args[0]) {
        lisp.marker_position_checked(args[0])? as i64
    } else if let Some(n) = args[0].as_fixnum() {
        n
    } else if lisp.is_bignum(args[0]) {
        return Ok(Value::NIL);
    } else {
        return Err(lisp.wrong_type(sym::INTEGER_OR_MARKER_P, args[0]));
    };
    let b = lisp.cur();
    if pos <= b.begv as i64 || pos > b.zv as i64 {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(lisp.char_at_pos(pos as usize - 1) as i64))
}

fn login_name() -> String {
    std::env::var("LOGNAME")
        .or_else(|_| std::env::var("USER"))
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn f_user_login_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if !args[0].is_nil() {
        return Ok(Value::NIL);
    }
    let v = lisp.plain_value(sym::USER_LOGIN_NAME);
    if v.is_string() {
        return Ok(v);
    }
    let name = login_name();
    Ok(lisp.string(&name))
}

fn f_user_real_login_name(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let name = login_name();
    Ok(lisp.string(&name))
}

fn f_user_uid(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(1000))
}

fn f_group_name(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_user_full_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if !args[0].is_nil() {
        return Ok(Value::NIL);
    }
    let v = lisp.plain_value(sym::USER_FULL_NAME);
    if v.is_string() {
        return Ok(v);
    }
    let name = std::env::var("NAME").unwrap_or_else(|_| login_name());
    Ok(lisp.string(&name))
}

fn f_system_name(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let v = lisp.plain_value(sym::SYSTEM_NAME);
    if v.is_string() {
        return Ok(v);
    }
    let name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "localhost".to_string());
    Ok(lisp.string(&name))
}

fn f_emacs_pid(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(std::process::id() as i64))
}

fn general_insert(
    lisp: &mut Lisp,
    args: &[Value],
    inherit: bool,
    before_markers: bool,
) -> LispResult {
    for &arg in args {
        if let Some(c) = arg.as_fixnum() {
            let c = lisp.check_character(Value::fixnum(c))?;
            lisp.insert_char_codes(&[c], inherit, before_markers)?;
        } else if arg.is_string() {
            lisp.insert_string(arg, inherit, before_markers)?;
        } else {
            return Err(lisp.wrong_type(sym::CHAR_OR_STRING_P, arg));
        }
    }
    Ok(Value::NIL)
}

fn f_insert(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    general_insert(lisp, args, false, false)
}

fn f_insert_and_inherit(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    general_insert(lisp, args, true, false)
}

fn f_insert_before_markers(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    general_insert(lisp, args, false, true)
}

fn f_insert_before_markers_and_inherit(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    general_insert(lisp, args, true, true)
}

fn f_insert_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    let count = if args[1].is_nil() {
        1
    } else {
        lisp.check_fixnum(args[1])?
    };
    if count <= 0 {
        return Ok(Value::NIL);
    }
    let chars = vec![c; count as usize];
    lisp.insert_char_codes(&chars, !args[2].is_nil(), false)?;
    Ok(Value::NIL)
}

fn f_insert_byte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.check_fixnum(args[0])?;
    if !(0..=255).contains(&b) {
        return Err(lisp.args_out_of_range3(args[0], Value::fixnum(0), Value::fixnum(255)));
    }
    let c = if b >= 0x80 && lisp.cur().text.multibyte {
        character::byte8_to_char(b as u8)
    } else {
        b as u32
    };
    let count = lisp.check_fixnum(args[1])?.max(0) as usize;
    let chars = vec![c; count];
    lisp.insert_char_codes(&chars, !args[2].is_nil(), false)?;
    Ok(Value::NIL)
}

fn f_buffer_substring(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (s, e) = lisp.validate_region(args[0], args[1])?;
    Ok(lisp.buffer_substring_value(s, e, true))
}

fn f_buffer_substring_no_properties(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (s, e) = lisp.validate_region(args[0], args[1])?;
    Ok(lisp.buffer_substring_value(s, e, false))
}

fn f_buffer_string(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let (s, e) = (lisp.cur().begv, lisp.cur().zv);
    Ok(lisp.buffer_substring_value(s, e, true))
}

fn f_insert_buffer_substring(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = f_get_buffer_arg(lisp, args[0])?;
    let count = lisp.specpdl.len();
    let current = lisp.current_buffer;
    lisp.record_unwind_current_buffer();
    lisp.set_buffer_internal(buffer);
    let start = if args[1].is_nil() {
        Value::fixnum(lisp.cur().begv as i64)
    } else {
        args[1]
    };
    let end = if args[2].is_nil() {
        Value::fixnum(lisp.cur().zv as i64)
    } else {
        args[2]
    };
    let r = lisp.validate_region(start, end);
    let (s, e) = match r {
        Ok(v) => v,
        Err(err) => {
            lisp.unbind_to(count)?;
            return Err(err);
        }
    };
    let text = lisp.buffer_substring_value(s, e, true);
    lisp.stack.push(text);
    lisp.set_buffer_internal(current);
    let r = lisp.insert_string(text, false, false);
    lisp.stack.pop();
    let done = lisp.unbind_to(count);
    r?;
    done?;
    Ok(Value::NIL)
}

fn f_get_buffer_arg(lisp: &mut Lisp, v: Value) -> LispResult<Value> {
    if lisp.is_buffer(v) {
        if !lisp.buffer_live_p(v) {
            return Err(lisp.error("Selecting deleted buffer"));
        }
        return Ok(v);
    }
    lisp.check_string(v)?;
    let b = lisp.get_buffer(v);
    if b.is_nil() {
        let name = lisp.string_to_rust(v);
        return Err(lisp.error(&format!("No such buffer {name}")));
    }
    Ok(b)
}

fn f_compare_buffer_substrings(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let get = |lisp: &mut Lisp, buf: Value, start: Value, end: Value| -> LispResult<Vec<u32>> {
        let buffer = if buf.is_nil() {
            lisp.current_buffer
        } else {
            f_get_buffer_arg(lisp, buf)?
        };
        let count = lisp.specpdl.len();
        lisp.record_unwind_current_buffer();
        lisp.set_buffer_internal(buffer);
        let s = if start.is_nil() {
            Value::fixnum(lisp.cur().begv as i64)
        } else {
            start
        };
        let e = if end.is_nil() {
            Value::fixnum(lisp.cur().zv as i64)
        } else {
            end
        };
        let r = lisp.validate_region(s, e);
        let chars = r.map(|(s, e)| (s..e).map(|p| lisp.char_at_pos(p)).collect());
        lisp.unbind_to(count)?;
        chars
    };
    let a = get(lisp, args[0], args[1], args[2])?;
    let b = get(lisp, args[3], args[4], args[5])?;
    let fold = !lisp.find_symbol_value(sym::CASE_FOLD_SEARCH).is_nil();
    for (i, (&x, &y)) in a.iter().zip(&b).enumerate() {
        let (x, y) = if fold {
            (
                crate::casefiddle::downcase_char(x),
                crate::casefiddle::downcase_char(y),
            )
        } else {
            (x, y)
        };
        if x != y {
            let n = (i + 1) as i64;
            return Ok(Value::fixnum(if x < y { -n } else { n }));
        }
    }
    let n = a.len().min(b.len()) as i64 + 1;
    Ok(Value::fixnum(match a.len().cmp(&b.len()) {
        std::cmp::Ordering::Less => -n,
        std::cmp::Ordering::Greater => n,
        std::cmp::Ordering::Equal => 0,
    }))
}

fn f_subst_char_in_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (start, end, fromchar, tochar, noundo) = (args[0], args[1], args[2], args[3], args[4]);
    let (s, e) = lisp.validate_region(start, end)?;
    let from = lisp.check_character(fromchar)?;
    let to = lisp.check_character(tochar)?;
    let positions: Vec<usize> = (s..e).filter(|&p| lisp.char_at_pos(p) == from).collect();
    if positions.is_empty() {
        return Ok(Value::NIL);
    }
    let saved_undo = lisp.undo_list();
    let pt = lisp.cur().pt;
    for p in positions {
        let replacement = lisp.string_from_chars(&[to], to >= 0x80);
        lisp.replace_range_chars(p, p + 1, replacement)?;
    }
    lisp.goto_char_internal(pt);
    if !noundo.is_nil() {
        let b = lisp.current_buffer;
        lisp.buffer_local_set(b, sym::BUFFER_UNDO_LIST, saved_undo);
    }
    Ok(Value::NIL)
}

fn f_delete_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (s, e) = lisp.validate_region(args[0], args[1])?;
    lisp.del_range(s, e)?;
    Ok(Value::NIL)
}

fn f_delete_and_extract_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (s, e) = lisp.validate_region(args[0], args[1])?;
    if s == e {
        return Ok(lisp.empty_unibyte);
    }
    lisp.del_range_1(s, e, true)
}

fn f_widen(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    lisp.widen();
    Ok(Value::NIL)
}

fn f_narrow_to_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let s = lisp.position_arg(args[0])?;
    let e = lisp.position_arg(args[1])?;
    let (s, e) = if s <= e { (s, e) } else { (e, s) };
    let z = lisp.cur().z() as i64;
    if s < 1 || e > z {
        return Err(lisp.args_out_of_range(args[0], args[1]));
    }
    lisp.narrow(s as usize, e as usize);
    Ok(Value::NIL)
}

fn f_labeled_narrow_to_region(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    f_narrow_to_region(lisp, &args[..2])
}

fn f_labeled_widen(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    lisp.widen();
    Ok(Value::NIL)
}

fn f_save_restriction(lisp: &mut Lisp, body: Value) -> LispResult {
    let count = lisp.specpdl.len();
    let b = lisp.cur();
    let buffer = lisp.current_buffer;
    let data = if b.begv == 1 && b.zv == b.z() {
        buffer
    } else {
        let (begv, zv) = (b.begv, b.zv);
        let beg = lisp.make_marker_at(begv, buffer, false);
        let end = lisp.make_marker_at(zv, buffer, true);
        lisp.cons(beg, end)
    };
    lisp.record_unwind(restore_restriction, data);
    let r = lisp.progn(body);
    lisp.unbind_to_value(count, r)
}

fn f_ngettext(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(if args[2] == Value::fixnum(1) {
        args[0]
    } else {
        args[1]
    })
}

fn f_message(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args[0].is_nil()
        || (args[0].is_string() && lisp.string_chars(args[0]) == 0 && args.len() == 1)
    {
        lisp.set_plain_value(sym::CURRENT_MESSAGE_INTERNAL, Value::NIL);
        return Ok(args[0]);
    }
    let s = lisp.format_string(args, true)?;
    lisp.stack.push(s);
    let r = lisp.message_string(s);
    lisp.stack.pop();
    r?;
    Ok(s)
}

fn f_current_message(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.current_message_value())
}

fn f_propertize(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args.len().is_multiple_of(2) {
        let n = Value::fixnum(args.len() as i64);
        return Err(lisp.signal_list(sym::WRONG_NUMBER_OF_ARGUMENTS, &[sym::PROPERTIZE, n]));
    }
    let s = lisp.check_string(args[0])?;
    let copy = lisp.heap.string_ref(s).clone();
    let string = lisp.heap.string(copy);
    lisp.stack.push(string);
    let mut props = Value::NIL;
    let mut i = 1;
    while i < args.len() {
        let tail = lisp.cons(args[i + 1], props);
        props = lisp.cons(args[i], tail);
        i += 2;
    }
    let len = Value::fixnum(lisp.string_chars(string) as i64);
    let r = lisp.add_text_properties(Value::fixnum(0), len, props, string, false);
    lisp.stack.pop();
    r?;
    Ok(string)
}

fn f_format(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.format_string(args, false)
}

fn f_format_message(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.format_string(args, true)
}

fn f_text_quoting_style(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.text_quoting_style())
}

fn f_char_equal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.check_character(args[0])?;
    let b = lisp.check_character(args[1])?;
    if a == b {
        return Ok(sym::T);
    }
    if lisp.find_symbol_value(sym::CASE_FOLD_SEARCH).is_nil() {
        return Ok(Value::NIL);
    }
    let fold = |c: u32| crate::casefiddle::downcase_char(c);
    Ok(lisp.bool(fold(a) == fold(b)))
}

fn f_transpose_regions(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (s1, e1) = lisp.validate_region(args[0], args[1])?;
    let (s2, e2) = lisp.validate_region(args[2], args[3])?;
    let ((s1, e1), (s2, e2)) = if s1 <= s2 {
        ((s1, e1), (s2, e2))
    } else {
        ((s2, e2), (s1, e1))
    };
    if e1 > s2 {
        return Err(lisp.error("Transposed regions overlap"));
    }
    let r1 = lisp.buffer_substring_value(s1, e1, true);
    let mid = lisp.buffer_substring_value(e1, s2, true);
    let r2 = lisp.buffer_substring_value(s2, e2, true);
    lisp.stack.extend_from_slice(&[r1, mid, r2]);
    let pt = lisp.cur().pt;
    let combined = lisp.concat_to_string(&[r2, mid, r1])?;
    lisp.stack.push(combined);
    lisp.del_range(s1, e2)?;
    lisp.goto_char_internal(s1);
    lisp.insert_string(combined, false, false)?;
    lisp.goto_char_internal(pt);
    let len = lisp.stack.len();
    lisp.stack.truncate(len - 4);
    Ok(Value::NIL)
}

fn f_forward_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = if args[0].is_nil() {
        1
    } else {
        lisp.check_fixnum(args[0])?
    };
    let b = lisp.cur();
    let target = b.pt as i64 + n;
    if target < b.begv as i64 {
        let begv = b.begv;
        lisp.goto_char_internal(begv);
        return Err(lisp.signal(sym::BEGINNING_OF_BUFFER, Value::NIL));
    }
    if target > b.zv as i64 {
        let zv = b.zv;
        lisp.goto_char_internal(zv);
        return Err(lisp.signal(sym::END_OF_BUFFER, Value::NIL));
    }
    lisp.goto_char_internal(target as usize);
    Ok(Value::NIL)
}

fn f_backward_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = if args[0].is_nil() {
        1
    } else {
        lisp.check_fixnum(args[0])?
    };
    f_forward_char(lisp, &[Value::fixnum(-n)])
}

fn f_forward_line(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let count = if args[0].is_nil() {
        1
    } else {
        lisp.check_fixnum(args[0])?
    };
    let opoint = lisp.cur().pt;
    let (pos, counted) = lisp.scan_newline_from_point(count);
    lisp.goto_char_internal(pos);
    let mut shortage = count - i64::from(count <= 0) - counted;
    if shortage != 0 {
        let b = lisp.cur();
        let adjust = if count <= 0 {
            -1
        } else {
            i64::from(b.begv < b.zv && b.pt != opoint && lisp.char_at_pos(b.pt - 1) != 10)
        };
        shortage -= adjust;
    }
    Ok(Value::fixnum(shortage))
}
fn f_beginning_of_line(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = line_count_arg(lisp, args[0])?;
    let pos = lisp.line_beginning(n);
    lisp.goto_char_internal(pos);
    Ok(Value::NIL)
}

fn f_end_of_line(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = line_count_arg(lisp, args[0])?;
    let pos = lisp.line_end(n);
    lisp.goto_char_internal(pos);
    Ok(Value::NIL)
}

fn f_delete_char(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = lisp.check_fixnum(args[0])?;
    let b = lisp.cur();
    let pt = b.pt as i64;
    let (from, to) = if n < 0 { (pt + n, pt) } else { (pt, pt + n) };
    if from < b.begv as i64 {
        return Err(lisp.signal(sym::BEGINNING_OF_BUFFER, Value::NIL));
    }
    if to > b.zv as i64 {
        return Err(lisp.signal(sym::END_OF_BUFFER, Value::NIL));
    }
    lisp.del_range(from as usize, to as usize)?;
    Ok(Value::NIL)
}

struct Spec {
    start: usize,
    end: usize,
    fbeg: usize,
    fend: usize,
    arg: Value,
    props: bool,
}

fn styled_format(lisp: &mut Lisp, args: &[Value], message: bool) -> LispResult {
    let fmt = lisp.check_string(args[0])?;
    let base = lisp.stack.len();
    lisp.stack.extend_from_slice(args);
    let r = styled_format_inner(lisp, args, message);
    lisp.stack.truncate(base);
    let _ = fmt;
    r
}

fn styled_format_inner(lisp: &mut Lisp, args: &[Value], message: bool) -> LispResult {
    let fmt = args[0];
    let fchars = {
        let multibyte = lisp.string_multibyte(fmt);
        let chars = lisp.string_char_vec(fmt);
        if multibyte {
            chars
        } else {
            chars
                .into_iter()
                .map(|c| {
                    if c >= 0x80 {
                        character::byte8_to_char(c as u8)
                    } else {
                        c
                    }
                })
                .collect()
        }
    };
    let quoting = if message {
        lisp.text_quoting_style()
    } else {
        Value::NIL
    };
    let mut out: Vec<u32> = Vec::with_capacity(fchars.len() + 16);
    let mut specs: Vec<Spec> = Vec::new();
    let mut new_result = false;
    let mut n = 0usize;
    let mut i = 0usize;
    let mut multibyte = lisp.string_multibyte(fmt)
        || args[1..]
            .iter()
            .any(|&a| a.is_string() && lisp.string_multibyte(a));
    while i < fchars.len() {
        let c = fchars[i];
        if c != '%' as u32 {
            if (c == '`' as u32 || c == '\'' as u32) && quoting == sym::CURVE {
                out.push(if c == '`' as u32 { 0x2018 } else { 0x2019 });
                multibyte = true;
                new_result = true;
            } else if c == '`' as u32 && quoting == sym::STRAIGHT {
                out.push('\'' as u32);
                new_result = true;
            } else {
                out.push(c);
            }
            i += 1;
            continue;
        }
        let fbeg = i;
        i += 1;
        let digits = |i: &mut usize| -> Option<usize> {
            let start = *i;
            let mut v: usize = 0;
            while *i < fchars.len() && (0x30..=0x39).contains(&fchars[*i]) {
                v = v
                    .saturating_mul(10)
                    .saturating_add((fchars[*i] - 0x30) as usize);
                *i += 1;
            }
            (*i > start).then_some(v)
        };
        let save = i;
        if let Some(num) = digits(&mut i) {
            if i < fchars.len() && fchars[i] == '$' as u32 {
                n = num.saturating_sub(1);
                i += 1;
            } else {
                i = save;
            }
        }
        let (mut minus, mut plus, mut space, mut sharp, mut zero) =
            (false, false, false, false, false);
        while i < fchars.len() {
            match char::from_u32(fchars[i]) {
                Some('-') => minus = true,
                Some('+') => plus = true,
                Some(' ') => space = true,
                Some('#') => sharp = true,
                Some('0') => zero = true,
                _ => break,
            }
            i += 1;
        }
        space &= !plus;
        zero &= !minus;
        let width = digits(&mut i).unwrap_or(0);
        let precision = if i < fchars.len() && fchars[i] == '.' as u32 {
            i += 1;
            Some(digits(&mut i).unwrap_or(0))
        } else {
            None
        };
        if i >= fchars.len() {
            return Err(lisp.error("Format string ends in middle of format specifier"));
        }
        let conversion = char::from_u32(fchars[i]).unwrap_or('\0');
        i += 1;
        if conversion == '%' {
            out.push('%' as u32);
            new_result = true;
            continue;
        }
        n += 1;
        if n >= args.len() {
            return Err(lisp.error("Not enough arguments for format string"));
        }
        let mut arg = args[n];
        let mut conversion = conversion;
        if conversion == 'S' || (conversion == 's' && !arg.is_string() && !arg.is_symbol()) {
            let chars = lisp.print_to_chars(arg, conversion == 'S')?;
            if chars.iter().any(|&c| c >= 0x80) {
                multibyte = true;
            }
            arg = lisp.string_from_chars(&chars, chars.iter().any(|&c| c >= 0x80));
            lisp.stack.push(arg);
            conversion = 's';
        } else if conversion == 'c' {
            if let Some(code) = arg.as_fixnum()
                && code >= 0x80
            {
                multibyte = true;
            }
            zero = false;
        }
        if arg.is_symbol() {
            arg = lisp.symbol_name(arg);
        }
        let start = out.len();
        match conversion {
            's' => {
                if fchars.len() == 2 && i == 2 && lisp.heap.string_ref(fmt).props.is_empty() {
                    return Ok(arg);
                }
                let chars: Vec<u32> = {
                    let m = lisp.string_multibyte(arg);
                    lisp.string_char_vec(arg)
                        .into_iter()
                        .map(|c| {
                            if !m && c >= 0x80 {
                                character::byte8_to_char(c as u8)
                            } else {
                                c
                            }
                        })
                        .collect()
                };
                let mut taken = Vec::new();
                let mut w = 0usize;
                for &c in &chars {
                    let cw = lisp.char_width_of(c);
                    if let Some(p) = precision
                        && w + cw > p
                    {
                        break;
                    }
                    w += cw;
                    taken.push(c);
                }
                let padding = width.saturating_sub(w);
                if !minus {
                    out.extend(std::iter::repeat_n(' ' as u32, padding));
                }
                let text_start = out.len();
                out.extend_from_slice(&taken);
                let text_end = out.len();
                if minus {
                    out.extend(std::iter::repeat_n(' ' as u32, padding));
                }
                let has_props = !lisp.heap.string_ref(arg).props.is_empty();
                specs.push(Spec {
                    start: text_start,
                    end: text_end,
                    fbeg,
                    fend: i,
                    arg,
                    props: has_props,
                });
                new_result = true;
                continue;
            }
            'c' | 'd' | 'i' | 'o' | 'x' | 'X' | 'b' | 'B' | 'e' | 'f' | 'g' => {}
            other => {
                return Err(lisp.error(&format!("Invalid format operation %{other}")));
            }
        }
        let is_float = matches!(conversion, 'e' | 'f' | 'g');
        if !(arg.is_fixnum() || ((lisp.is_bignum(arg) || arg.is_float()) && conversion != 'c')) {
            return Err(lisp.error("Format specifier doesn't match argument type"));
        }
        let text = if conversion == 'c' {
            let code = arg.as_fixnum().unwrap_or(0) as u32;
            if precision == Some(0) {
                String::new()
            } else {
                char::from_u32(code).map(String::from).unwrap_or_default()
            }
        } else if is_float {
            let value = lisp.number(arg)?.to_f64();
            format_float(value, conversion, precision, sharp, plus, space)
        } else {
            let value: BigInt = if arg.is_float() {
                let f = lisp.heap.float_value(arg).trunc();
                crate::data::num_bigint_from_f64(f)
            } else {
                lisp.number(arg)?.to_bigint().unwrap_or_default()
            };
            format_integer(&value, conversion, precision, sharp, plus, space)
        };
        let mut text: Vec<u32> = text.chars().map(|c| c as u32).collect();
        if conversion == 'c' && text.is_empty() && precision != Some(0) {
            text.push(arg.as_fixnum().unwrap_or(0) as u32);
        }
        let padding = width.saturating_sub(text.len());
        let numeric = conversion != 'c';
        if zero
            && numeric
            && padding > 0
            && text.iter().any(|&c| (0x30..=0x39).contains(&c))
            && !(precision.is_some() && !is_float)
        {
            let sign_len = text
                .iter()
                .take_while(|&&c| c == '-' as u32 || c == '+' as u32 || c == ' ' as u32)
                .count();
            let prefix_len = if !is_float
                && text.len() > sign_len + 1
                && text[sign_len] == '0' as u32
                && matches!(
                    char::from_u32(text[sign_len + 1]),
                    Some('x' | 'X' | 'b' | 'B')
                ) {
                2
            } else {
                0
            };
            let insert_at = sign_len + prefix_len;
            for _ in 0..padding {
                text.insert(insert_at, '0' as u32);
            }
            out.extend_from_slice(&text);
        } else if minus {
            out.extend_from_slice(&text);
            out.extend(std::iter::repeat_n(' ' as u32, padding));
        } else {
            out.extend(std::iter::repeat_n(' ' as u32, padding));
            out.extend_from_slice(&text);
        }
        specs.push(Spec {
            start,
            end: out.len(),
            fbeg,
            fend: i,
            arg,
            props: false,
        });
        new_result = true;
    }
    if !new_result {
        return Ok(fmt);
    }
    let multibyte = multibyte || out.iter().any(|&c| c >= 0x80);
    let result = lisp.string_from_chars(&out, multibyte);
    let fmt_props = lisp.heap.string_ref(fmt).props.clone();
    let mut props: Vec<PropRun> = Vec::new();
    if !fmt_props.is_empty() {
        let map = |pos: usize, end_side: bool| -> usize {
            let mut delta: isize = 0;
            for s in &specs {
                if pos <= s.fbeg {
                    break;
                }
                if pos < s.fend {
                    return if end_side { s.end } else { s.start };
                }
                delta += (s.end - s.start) as isize - (s.fend - s.fbeg) as isize;
            }
            (pos as isize + delta).max(0) as usize
        };
        for run in fmt_props {
            let s = map(run.start, false);
            let e = map(run.end, true);
            if s < e {
                props.push(PropRun {
                    start: s,
                    end: e,
                    plist: run.plist,
                });
            }
        }
    }
    for s in specs.iter().filter(|s| s.props) {
        let arg_props = lisp.heap.string_ref(s.arg).props.clone();
        for run in arg_props {
            let start = s.start + run.start;
            let end = (s.start + run.end).min(s.end);
            if start < end {
                props.push(PropRun {
                    start,
                    end,
                    plist: run.plist,
                });
            }
        }
    }
    if !props.is_empty() {
        props.sort_by_key(|r| r.start);
        let mut merged: Vec<PropRun> = Vec::new();
        for run in props {
            match merged.last_mut() {
                Some(last) if run.start < last.end => {
                    let combined = merge_plists(lisp, last.plist, run.plist);
                    if run.start > last.start {
                        let tail_end = last.end;
                        last.end = run.start;
                        merged.push(PropRun {
                            start: run.start,
                            end: run.end.min(tail_end),
                            plist: combined,
                        });
                    } else {
                        last.plist = combined;
                    }
                }
                _ => merged.push(run),
            }
        }
        let s: &mut LispString = lisp.heap.string_mut(result);
        s.props = merged;
    }
    Ok(result)
}

fn merge_plists(lisp: &mut Lisp, a: Value, b: Value) -> Value {
    let mut items = lisp.list_to_vec(a).unwrap_or_default();
    let extra = lisp.list_to_vec(b).unwrap_or_default();
    for pair in extra.chunks(2) {
        if let [k, v] = pair {
            match items.chunks(2).position(|c| c[0] == *k) {
                Some(idx) => items[idx * 2 + 1] = *v,
                None => {
                    items.push(*k);
                    items.push(*v);
                }
            }
        }
    }
    lisp.list(&items)
}

pub fn format_float(
    value: f64,
    conversion: char,
    precision: Option<usize>,
    sharp: bool,
    plus: bool,
    space: bool,
) -> String {
    let sign = if value.is_sign_negative() && !(value == 0.0 && !value.is_sign_negative()) {
        "-"
    } else if plus {
        "+"
    } else if space {
        " "
    } else {
        ""
    };
    let a = value.abs();
    if a.is_nan() {
        return format!("{sign}nan");
    }
    if a.is_infinite() {
        return format!("{sign}inf");
    }
    let prec = precision.unwrap_or(6);
    let body = match conversion {
        'f' => {
            let s = format!("{a:.prec$}");
            if sharp && prec == 0 {
                format!("{s}.")
            } else {
                s
            }
        }
        'e' => {
            let s = format!("{a:.prec$e}");
            let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
            let exp: i32 = e.parse().unwrap_or(0);
            let m = if sharp && prec == 0 {
                format!("{m}.")
            } else {
                m.to_string()
            };
            format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
        }
        _ => {
            let p = if prec == 0 { 1 } else { prec };
            if sharp {
                format_g_sharp(a, p)
            } else {
                crate::print::format_g(a, p)
            }
        }
    };
    format!("{sign}{body}")
}

fn format_g_sharp(a: f64, p: usize) -> String {
    if a == 0.0 {
        return format!("{:.*}", p - 1, 0.0);
    }
    let sci = format!("{:.*e}", p - 1, a);
    let exp: i32 = sci[sci.find('e').map_or(0, |i| i + 1)..]
        .parse()
        .unwrap_or(0);
    if exp < -4 || exp >= p as i32 {
        let (m, _) = sci.split_once('e').unwrap_or((&sci, ""));
        let m = if m.contains('.') {
            m.to_string()
        } else {
            format!("{m}.")
        };
        format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    } else {
        let decimals = (p as i32 - 1 - exp).max(0) as usize;
        let s = format!("{a:.decimals$}");
        if s.contains('.') { s } else { format!("{s}.") }
    }
}

pub fn format_integer(
    value: &BigInt,
    conversion: char,
    precision: Option<usize>,
    sharp: bool,
    plus: bool,
    space: bool,
) -> String {
    let negative = value.is_negative();
    let magnitude = value.abs();
    let mut digits = match conversion {
        'o' => magnitude.to_str_radix(8),
        'x' => magnitude.to_str_radix(16),
        'X' => magnitude.to_str_radix(16).to_uppercase(),
        'b' | 'B' => magnitude.to_str_radix(2),
        _ => magnitude.to_string(),
    };
    if let Some(p) = precision {
        if p == 0 && magnitude == BigInt::from(0) {
            digits.clear();
        }
        while digits.len() < p {
            digits.insert(0, '0');
        }
    }
    let nonzero = magnitude != BigInt::from(0);
    let prefix = if sharp {
        match conversion {
            'x' if nonzero => "0x",
            'X' if nonzero => "0X",
            'b' if nonzero => "0b",
            'B' if nonzero => "0B",
            'o' if !digits.starts_with('0') => "0",
            _ => "",
        }
    } else {
        ""
    };
    let sign = if negative {
        "-"
    } else if plus {
        "+"
    } else if space {
        " "
    } else {
        ""
    };
    format!("{sign}{prefix}{digits}")
}
