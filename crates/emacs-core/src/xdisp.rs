pub mod append;
pub mod cursor;
pub mod draw;
pub mod echo;
pub mod elements;
pub mod geom;
pub mod gui_produce;
pub mod hscroll;
pub mod internal;
pub mod it;
pub mod iterate;
pub mod line;
pub mod lnum;
pub mod menu_bar;
pub mod modeline;
pub mod move_it;
pub mod next;
pub mod produce;
pub mod props;
pub mod redisplay;
pub mod reseat;
pub mod reuse;
pub mod scroll;
pub mod stack;
pub mod visible;

use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("invisible-p", 1, 1, f_invisible_p),
    Subr::fixed("line-pixel-height", 0, 0, f_line_pixel_height),
    Subr::fixed(
        "long-line-optimizations-p",
        0,
        0,
        f_long_line_optimizations_p,
    ),
    Subr::fixed("get-display-property", 2, 4, it::f_get_display_property),
    Subr::fixed("format-mode-line", 1, 4, modeline::f_format_mode_line),
    Subr::fixed("tab-bar-height", 0, 2, f_tab_bar_height),
    Subr::fixed("tool-bar-height", 0, 2, f_tool_bar_height),
    Subr::fixed("set-buffer-redisplay", 4, 4, f_set_buffer_redisplay),
    Subr::fixed(
        "current-bidi-paragraph-direction",
        0,
        1,
        f_current_bidi_paragraph_direction,
    ),
    Subr::fixed(
        "bidi-find-overridden-directionality",
        3,
        4,
        f_bidi_find_overridden_directionality,
    ),
    Subr::fixed("remember-mouse-glyph", 3, 3, f_remember_mouse_glyph),
];

fn f_remember_mouse_glyph(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_window_system_frame(args[0])?;
    let x = lisp.check_fixnum(args[1])?;
    let y = lisp.check_fixnum(args[2])?;
    let rect = lisp.remember_mouse_glyph(f, i64::from(x as i32), i64::from(y as i32));
    Ok(lisp.list(&[
        Value::fixnum(rect.x.into()),
        Value::fixnum(rect.y.into()),
        Value::fixnum(rect.width.into()),
        Value::fixnum(rect.height.into()),
    ]))
}

pub(crate) fn f_current_bidi_paragraph_direction(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = args.first().copied().unwrap_or(Value::NIL);
    let old = lisp.current_buffer;
    let buf = if buffer.is_nil() {
        old
    } else {
        lisp.check_buffer(buffer)?
    };
    let local = |lisp: &Lisp, symbol| lisp.buffer_local_value(symbol, buf);
    if local(lisp, sym::BIDI_DISPLAY_REORDERING).is_nil()
        || local(lisp, sym::ENABLE_MULTIBYTE_CHARACTERS).is_nil()
        || lisp.var_true(sym::REDISPLAY__INHIBIT_BIDI)
    {
        return Ok(sym::LEFT_TO_RIGHT);
    }
    let direction = local(lisp, sym::BIDI_PARAGRAPH_DIRECTION);
    if !direction.is_nil() {
        return Ok(direction);
    }
    let mut pos = lisp.buffer(buf).pt as i64;
    let mut bytepos = lisp.buffer(buf).pt_byte as i64;
    let itb_data = lisp.bidi_shelve_cache();
    lisp.set_buffer_internal(buf);
    {
        let (begv, zv) = (lisp.cur().begv as i64, lisp.cur().zv as i64);
        if pos >= zv && pos > begv {
            pos -= 1;
            let text = &lisp.cur().text;
            bytepos -= 1;
            while text.multibyte && text.byte_at(bytepos as usize) & 0xC0 == 0x80 {
                bytepos -= 1;
            }
        }
        let trailing_white_space = lisp.string("[\x0c\t ]*\n");
        if lisp.fast_looking_at(trailing_white_space, pos as usize, zv as usize, Value::NIL)? > 0 {
            let begv_byte = lisp.cur().begv_byte as i64;
            loop {
                let c = lisp.cur().text.byte_at(bytepos as usize);
                if !(c == b'\n' || c == b' ' || c == b'\t' || c == 0x0c) {
                    break;
                }
                if bytepos <= begv_byte {
                    break;
                }
                bytepos -= 1;
                pos -= 1;
            }
            while lisp.cur().text.byte_at(bytepos as usize) & 0xC0 == 0x80 {
                bytepos -= 1;
            }
        }
        let mut itb = crate::bidi::BidiIt::default();
        let frame_window_p = lisp.frame_window_p(lisp.frames.selected_frame);
        lisp.bidi_init_it(pos, bytepos, frame_window_p, &mut itb)?;
        itb.paragraph_dir = crate::bidi::BidiDir::NeutralDir;
        itb.string = crate::bidi::BidiStringData::default();
        itb.w = Value::NIL;
        lisp.bidi_paragraph_init(crate::bidi::BidiDir::NeutralDir, &mut itb, true)?;
        lisp.bidi_unshelve_cache(itb_data, false);
        lisp.set_buffer_internal(old);
        match itb.paragraph_dir {
            crate::bidi::BidiDir::L2R => Ok(sym::LEFT_TO_RIGHT),
            crate::bidi::BidiDir::R2L => Ok(sym::RIGHT_TO_LEFT),
            crate::bidi::BidiDir::NeutralDir => std::process::abort(),
        }
    }
}

fn f_bidi_find_overridden_directionality(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    use crate::bidi::{BidiDir, BidiIt, BidiStringData};
    let (from, to, object) = (args[0], args[1], args[2]);
    let base_dir = args.get(3).copied().unwrap_or(Value::NIL);
    let old = lisp.current_buffer;
    let mut buf = old;
    let mut w = Value::NIL;
    let mut frame_window_p = lisp.frame_window_p(lisp.frames.selected_frame);
    if !object.is_nil() {
        if lisp.is_buffer(object) {
            buf = object;
        } else if lisp.is_window(object) {
            w = lisp.decode_live_window(object)?;
            buf = lisp.window(w).contents;
            frame_window_p = lisp.frame_window_p(lisp.window(w).frame);
        } else {
            lisp.check_string(object)?;
        }
    }
    let mut itb = BidiIt::default();
    let (from_pos, to_pos);
    let itb_data;
    if object.is_string() {
        if !lisp.string_multibyte(object) || lisp.var_true(sym::REDISPLAY__INHIBIT_BIDI) {
            return Ok(Value::NIL);
        }
        let schars = lisp.string_chars(object);
        let (f, t) = lisp.validate_subarray(object, from, to, schars)?;
        if f >= schars {
            return Ok(Value::NIL);
        }
        (from_pos, to_pos) = (f as i64, t as i64);
        itb_data = lisp.bidi_shelve_cache();
        itb.paragraph_dir = BidiDir::NeutralDir;
        itb.string = BidiStringData {
            lstring: object,
            schars: schars as i64,
            ..BidiStringData::default()
        };
        itb.w = w;
        lisp.bidi_init_it(0, 0, frame_window_p, &mut itb)?;
    } else {
        if lisp
            .buffer_local_value(sym::BIDI_DISPLAY_REORDERING, buf)
            .is_nil()
            || lisp
                .buffer_local_value(sym::ENABLE_MULTIBYTE_CHARACTERS, buf)
                .is_nil()
            || lisp.var_true(sym::REDISPLAY__INHIBIT_BIDI)
        {
            return Ok(Value::NIL);
        }
        lisp.set_buffer_internal(buf);
        let (f, t) = lisp.validate_region(from, to)?;
        (from_pos, to_pos) = (f as i64, t as i64);
        if from_pos >= lisp.cur().zv as i64 {
            return Ok(Value::NIL);
        }
        itb_data = lisp.bidi_shelve_cache();
        let from_bpos = lisp.cur().text.char_to_byte(from_pos as usize) as i64;
        if from_pos == lisp.cur().begv as i64 {
            itb.charpos = lisp.cur().begv as i64;
            itb.bytepos = lisp.cur().begv_byte as i64;
        } else if lisp.cur().text.byte_at(from_bpos as usize - 1) == b'\n' {
            itb.charpos = from_pos;
            itb.bytepos = from_bpos;
        } else {
            let begv = lisp.cur().begv;
            let charpos = lisp.find_newline(from_pos as usize, begv, -1).0;
            itb.charpos = charpos as i64;
            itb.bytepos = lisp.cur().text.char_to_byte(charpos) as i64;
        }
        itb.paragraph_dir = BidiDir::NeutralDir;
        itb.string = BidiStringData::default();
        itb.w = w;
        let (charpos, bytepos) = (itb.charpos, itb.bytepos);
        lisp.bidi_init_it(charpos, bytepos, frame_window_p, &mut itb)?;
    }
    let bdir = if base_dir == sym::RIGHT_TO_LEFT {
        BidiDir::R2L
    } else {
        BidiDir::L2R
    };
    let found = loop {
        lisp.bidi_paragraph_init(bdir, &mut itb, false)?;
        let found = loop {
            let found = lisp.bidi_find_first_overridden(&mut itb)?;
            if found >= from_pos {
                break found;
            }
        };
        if !(found == lisp.cur().zv as i64 && itb.ch == 10 && itb.charpos < to_pos) {
            break found;
        }
    };
    lisp.bidi_unshelve_cache(itb_data, false);
    lisp.set_buffer_internal(old);
    Ok(if from_pos <= found && found < to_pos {
        Value::fixnum(found)
    } else {
        Value::NIL
    })
}

fn f_long_line_optimizations_p(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.cur().long_line_optimizations_p))
}

fn f_line_pixel_height(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    lisp.with_gc_deferred(|lisp| {
        let w = lisp.windows.selected_window;
        let contents = lisp.window(w).contents;
        let old_buffer = (contents != lisp.current_buffer).then_some(lisp.current_buffer);
        if old_buffer.is_some() {
            lisp.set_buffer_internal(contents);
        }
        let pt = crate::dispextern::TextPos::new(lisp.cur().pt as i64, lisp.cur().pt_byte as i64);
        let itdata = lisp.bidi_shelve_cache();
        let mut it = crate::dispextern::It::default();
        lisp.start_display(&mut it, w, pt)?;
        lisp.move_it_by_lines(&mut it, 0)?;
        it.vpos = 0;
        it.current_y = 0;
        lisp.xdisp.last_height = 0;
        let result = Value::fixnum(i64::from(lisp.line_bottom_y(&it)?));
        if let Some(old) = old_buffer {
            lisp.set_buffer_internal(old);
        }
        lisp.bidi_unshelve_cache(itdata, false);
        Ok(result)
    })
}

fn f_set_buffer_redisplay(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let b = lisp.current_buffer;
    lisp.bset_update_mode_line(b);
    lisp.buffer_mut(b).prevent_redisplay_optimizations_p = true;
    Ok(Value::NIL)
}

pub const REDISPLAY_SOME: i64 = 2;

impl Lisp {
    pub fn invisible_prop(&self, propval: Value, list: Value) -> i64 {
        let mut tail = list;
        while tail.is_cons() {
            let tem = self.xcar(tail);
            if self.eq(propval, tem) {
                return 1;
            }
            if tem.is_cons() && self.eq(propval, self.xcar(tem)) {
                return if self.xcdr(tem).is_nil() { 1 } else { 2 };
            }
            tail = self.xcdr(tail);
        }
        if propval.is_cons() {
            let mut proptail = propval;
            while proptail.is_cons() {
                let propelt = self.xcar(proptail);
                let mut tail = list;
                while tail.is_cons() {
                    let tem = self.xcar(tail);
                    if self.eq(propelt, tem) {
                        return 1;
                    }
                    if tem.is_cons() && self.eq(propelt, self.xcar(tem)) {
                        return if self.xcdr(tem).is_nil() { 1 } else { 2 };
                    }
                    tail = self.xcdr(tail);
                }
                proptail = self.xcdr(proptail);
            }
        }
        0
    }

    pub fn text_prop_means_invisible(&self, prop: Value) -> i64 {
        let spec = self.find_symbol_value(sym::BUFFER_INVISIBILITY_SPEC);
        if spec == sym::T {
            i64::from(!prop.is_nil())
        } else {
            self.invisible_prop(prop, spec)
        }
    }
}

impl Lisp {
    pub fn redisplay(&mut self) -> LispResult<()> {
        self.redisplay_internal()
    }

    pub fn flush_frame(&mut self, f: Value) {
        let t = self.frame(f).terminal;
        if self.is_terminal(t)
            && let Some(rif) = self.terminal(t).rif
            && let Some(flush) = rif.flush_display
        {
            flush(self, f);
        }
    }

    fn estimated_line_height(
        &mut self,
        w: Value,
        (active, inactive): (usize, usize),
        row: fn(&crate::dispextern::GlyphMatrix) -> usize,
    ) -> i64 {
        let f = self.window(w).frame;
        if let Some(matrix) = self.window(w).current_matrix.as_deref()
            && !matrix.rows.is_empty()
            && matrix.nrows > 0
        {
            let r = &matrix.rows[row(matrix)];
            if r.mode_line_p {
                return i64::from(r.height);
            }
        }
        let face_id = self.current_line_active_face_id(w, active, inactive);
        self.estimate_mode_line_height(f, face_id)
    }

    pub fn window_box_height(&mut self, w: Value) -> i64 {
        let mut height = self.window(w).pixel_height;
        height -= self.window_bottom_divider_width(w);
        height -= self.window_scroll_bar_area_height(w);
        if self.window_wants_mode_line(w) {
            let line_height = self.window(w).mode_line_height;
            height -= if line_height >= 0 {
                line_height
            } else {
                self.estimated_line_height(
                    w,
                    (
                        crate::xfaces::MODE_LINE_ACTIVE_FACE_ID,
                        crate::xfaces::MODE_LINE_INACTIVE_FACE_ID,
                    ),
                    crate::dispextern::GlyphMatrix::mode_line_row,
                )
            };
        }
        if self.window_wants_tab_line(w) {
            let line_height = self.window(w).tab_line_height;
            height -= if line_height >= 0 {
                line_height
            } else {
                self.estimated_line_height(
                    w,
                    (
                        crate::xfaces::TAB_LINE_ACTIVE_FACE_ID,
                        crate::xfaces::TAB_LINE_INACTIVE_FACE_ID,
                    ),
                    |_| 0,
                )
            };
        }
        if self.window_wants_header_line(w) {
            let line_height = self.window(w).header_line_height;
            height -= if line_height >= 0 {
                line_height
            } else {
                self.estimated_line_height(
                    w,
                    (
                        crate::xfaces::HEADER_LINE_ACTIVE_FACE_ID,
                        crate::xfaces::HEADER_LINE_INACTIVE_FACE_ID,
                    ),
                    crate::dispextern::GlyphMatrix::header_line_row,
                )
            };
        }
        height.max(0)
    }
}

fn f_tab_bar_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(args.first().copied().unwrap_or(Value::NIL))?;
    let w = lisp.frame(f).tab_bar_window;
    if lisp.is_window(w) && lisp.window(w).pixel_height > 0 {
        unreachable!("tab-bar-height needs update_tab_bar and the display iterator");
    }
    Ok(Value::fixnum(0))
}

fn f_tool_bar_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(args.first().copied().unwrap_or(Value::NIL))?;
    let w = lisp.frame(f).tool_bar_window;
    if lisp.is_window(w) && lisp.window(w).pixel_height > 0 {
        unreachable!("tool-bar-height needs update_tool_bar and the display iterator");
    }
    Ok(Value::fixnum(0))
}

fn f_invisible_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = args[0];
    let prop = if matches!(pos.as_fixnum(), Some(n) if n >= 0) || lisp.is_marker(pos) {
        lisp.get_char_property(pos, sym::INVISIBLE, Value::NIL)?
    } else {
        pos
    };
    Ok(match lisp.text_prop_means_invisible(prop) {
        0 => Value::NIL,
        1 => sym::T,
        n => Value::fixnum(n),
    })
}

pub struct Xdisp {
    pub message_log_need_newline: bool,
    pub noninteractive_need_newline: bool,
    pub redisplaying_p: bool,
    pub message_stack: Value,
    pub echo_area_window: Value,
    pub echo_area_buffer: [Value; 2],
    pub help_echo_showing_p: bool,
    pub display_working_on_window_p: bool,
    pub redisplay_ticks_window: Value,
    pub window_ticks: i64,
    pub inhibit_free_realized_faces: bool,
    pub this_line_min_pos: crate::dispextern::TextPos,
    pub default_invis_vector: Value,
    pub last_height: i32,
    pub last_escape_glyph_frame: Value,
    pub last_escape_glyph_face_id: i64,
    pub last_escape_glyph_merged_face_id: i32,
    pub last_glyphless_glyph_frame: Value,
    pub last_glyphless_glyph_face_id: i64,
    pub last_glyphless_glyph_merged_face_id: i32,
    pub scratch_glyph_row: crate::dispextern::GlyphRow,
    pub overlay_arrow_seen: bool,
    pub this_line_buffer: Value,
    pub this_line_start_pos: crate::dispextern::TextPos,
    pub this_line_end_pos: crate::dispextern::TextPos,
    pub this_line_y: i32,
    pub this_line_pixel_height: i32,
    pub this_line_vpos: i32,
    pub this_line_start_x: i32,
    pub mode_line_target: modeline::ModeLineTarget,
    pub mode_line_noprop_buf: Vec<u8>,
    pub mode_line_proptrans_alist: Value,
    pub mode_line_string_list: Value,
    pub mode_line_elt_no: i64,
    pub mode_line_string_face: Value,
    pub mode_line_string_face_prop: Value,
    pub mode_line_unwind_vector: Value,
    pub line_number_displayed: bool,
    pub dont_resize_frames: i32,
    pub multiple_frames: bool,
    pub echo_buffer: [Value; 2],
    pub message_buf_print: bool,
    pub message_cleared_p: bool,
    pub display_last_displayed_message_p: bool,
    pub message_enable_multibyte: bool,
    pub with_echo_area_save_vector: Value,
    pub redisplay_counter: i64,
    pub clear_face_cache_count: i32,
    pub clear_image_cache_count: i32,
    pub displayed_buffer: Value,
    pub buffer_flip_blocked_depth: i32,
}

impl Default for Xdisp {
    fn default() -> Xdisp {
        Xdisp {
            message_log_need_newline: false,
            noninteractive_need_newline: false,
            redisplaying_p: false,
            message_stack: Value::NIL,
            echo_area_window: Value::NIL,
            echo_area_buffer: [Value::NIL; 2],
            help_echo_showing_p: false,
            display_working_on_window_p: false,
            redisplay_ticks_window: Value::NIL,
            window_ticks: 0,
            inhibit_free_realized_faces: false,
            this_line_min_pos: crate::dispextern::TextPos::default(),
            default_invis_vector: Value::NIL,
            last_height: 0,
            last_escape_glyph_frame: Value::NIL,
            last_escape_glyph_face_id: 1 << reseat::FACE_ID_BITS,
            last_escape_glyph_merged_face_id: 0,
            last_glyphless_glyph_frame: Value::NIL,
            last_glyphless_glyph_face_id: 1 << reseat::FACE_ID_BITS,
            last_glyphless_glyph_merged_face_id: 0,
            scratch_glyph_row: crate::dispextern::GlyphRow::default(),
            overlay_arrow_seen: false,
            this_line_buffer: Value::NIL,
            this_line_start_pos: crate::dispextern::TextPos::default(),
            this_line_end_pos: crate::dispextern::TextPos::default(),
            this_line_y: 0,
            this_line_pixel_height: 0,
            this_line_vpos: 0,
            this_line_start_x: 0,
            mode_line_target: modeline::ModeLineTarget::Display,
            mode_line_noprop_buf: Vec::new(),
            mode_line_proptrans_alist: Value::NIL,
            mode_line_string_list: Value::NIL,
            mode_line_elt_no: 0,
            mode_line_string_face: Value::NIL,
            mode_line_string_face_prop: Value::NIL,
            mode_line_unwind_vector: Value::NIL,
            line_number_displayed: false,
            dont_resize_frames: 0,
            multiple_frames: false,
            echo_buffer: [Value::NIL; 2],
            message_buf_print: false,
            message_cleared_p: false,
            display_last_displayed_message_p: false,
            message_enable_multibyte: false,
            with_echo_area_save_vector: Value::NIL,
            redisplay_counter: 0,
            clear_face_cache_count: 0,
            clear_image_cache_count: 0,
            displayed_buffer: Value::NIL,
            buffer_flip_blocked_depth: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct DisplaySpecContext {
    object: Value,
    position: usize,
    bufpos: usize,
    frame_window_p: bool,
    enable_eval: bool,
}

pub fn pop_message_unwind(lisp: &mut Lisp, _arg: Value) -> LispResult<()> {
    let stack = lisp.xdisp.message_stack;
    lisp.xdisp.message_stack = lisp.xcdr(stack);
    Ok(())
}

impl Lisp {
    pub fn push_message(&mut self) -> LispResult<bool> {
        let msg = self.current_message()?;
        self.xdisp.message_stack = self.cons(msg, self.xdisp.message_stack);
        Ok(msg.is_string())
    }

    pub fn restore_message(&mut self) -> LispResult<()> {
        let msg = self.xcar(self.xdisp.message_stack);
        self.message3_nolog(msg)
    }

    pub fn check_message_stack(&self) {
        if !self.xdisp.message_stack.is_nil() {
            std::process::abort();
        }
    }

    pub fn message1(&mut self, m: &str) -> LispResult<()> {
        let msg = self.unibyte_string(m.as_bytes());
        self.message3(msg)
    }

    pub fn message_log_maybe_newline(&mut self) -> LispResult<()> {
        if self.xdisp.message_log_need_newline {
            self.message_dolog(b"", true, false)?;
        }
        Ok(())
    }

    pub fn clear_message(&mut self, current_p: bool, last_displayed_p: bool) -> LispResult<()> {
        if current_p && !self.var_true(sym::INHIBIT_MESSAGE) {
            let mut preserve = Value::NIL;
            let function = self.find_symbol_value(sym::CLEAR_MESSAGE_FUNCTION);
            if self.functionp(function) && self.gc_inhibited == 0 {
                preserve = if self.var_true(sym::INHIBIT_EVAL_DURING_REDISPLAY) {
                    sym::UNBOUND
                } else {
                    let count = self.specpdl.len();
                    self.specbind(sym::INHIBIT_QUIT, sym::T)?;
                    let r = self.safe_calln(function, &[]);
                    self.unbind_to_value(count, r)?
                };
            }
            if preserve != sym::DONT_CLEAR_MESSAGE {
                self.xdisp.echo_area_buffer[0] = Value::NIL;
                self.xdisp.message_cleared_p = true;
            }
        }
        if last_displayed_p {
            self.xdisp.echo_area_buffer[1] = Value::NIL;
        }
        self.xdisp.message_buf_print = false;
        Ok(())
    }

    fn log_message(&mut self, m: Value) -> LispResult<()> {
        self.clear_message(true, false)?;
        self.cancel_echoing();
        self.message_log_maybe_newline()?;
        if m.is_string() {
            let bytes = self.string_bytes(m).to_vec();
            let multibyte = self.string_multibyte(m);
            self.message_dolog(&bytes, true, multibyte)?;
        }
        Ok(())
    }

    pub fn message3(&mut self, m: Value) -> LispResult<()> {
        let base = self.stack.len();
        self.stack.push(m);
        let r = self.log_message(m).and_then(|()| {
            if self.var_true(sym::INHIBIT_MESSAGE) {
                Ok(())
            } else {
                self.message3_nolog(m)
            }
        });
        self.stack.truncate(base);
        r
    }

    pub fn message3_nolog(&mut self, m: Value) -> LispResult<()> {
        let sf = self.frames.selected_frame;
        if self.frame(sf).output_method == crate::terminal::OutputMethod::Initial {
            return self.message_to_stderr(m);
        }
        let interactive =
            self.plain_value(sym::EXECUTING_KBD_MACRO).is_nil() && !self.noninteractive;
        if interactive && self.frame(sf).flags.glyphs_initialized_p {
            let count = self.specpdl_index();
            let mini_window = self.frame(sf).minibuffer_window;
            let frame = self.window(mini_window).frame;
            let mbf = frame;
            if self.frame(sf).flags.visible && !self.frame(mbf).flags.visible {
                crate::frame::f_make_frame_visible(self, &[frame])?;
            }
            if m.is_string() && self.string_chars(m) > 0 {
                self.set_message(m)?;
                if self.var_true(sym::MINIBUFFER_AUTO_RAISE) && !frame.is_nil() {
                    crate::frame::f_raise_frame(self, &[frame])?;
                }
                self.keyboard.echo_message_buffer = Value::NIL;
            } else {
                self.clear_message(true, true)?;
            }
            self.do_pending_window_change(false)?;
            self.echo_area_display(true)?;
            self.do_pending_window_change(false)?;
            let t = self.frame(mbf).terminal;
            if let Some(hook) = self.terminal(t).hooks.frame_up_to_date {
                hook(self, mbf);
            }
            self.unbind_to(count)?;
        }
        Ok(())
    }

    fn errwrite(&mut self, bytes: &[u8]) {
        use std::io::Write;
        let _ = self.stderr.write_all(bytes);
        let _ = self.stderr.flush();
    }

    pub fn message_to_stderr(&mut self, m: Value) -> LispResult<()> {
        if self.xdisp.noninteractive_need_newline {
            self.xdisp.noninteractive_need_newline = false;
            self.errwrite(b"\n");
        }
        if m.is_string() {
            let mut coding_system = self.find_symbol_value(sym::LOCALE_CODING_SYSTEM);
            let for_write = self.find_symbol_value(sym::CODING_SYSTEM_FOR_WRITE);
            if !for_write.is_nil() {
                coding_system = for_write;
            }
            let s = if coding_system.is_nil() {
                m
            } else {
                self.code_convert_string_norecord(m, coding_system, true)?
            };
            let bytes = self.string_bytes(s).to_vec();
            self.errwrite(&bytes);
        }
        if m.is_string() || !self.var_true(sym::CURSOR_IN_ECHO_AREA) {
            self.errwrite(b"\n");
        }
        Ok(())
    }

    pub fn estimate_mode_line_height(&mut self, f: Value, face_id: usize) -> i64 {
        if self.frame(f).window_p() {
            let frame_font = self.frame(f).font;
            let mut height = i64::from(self.font_object(frame_font).height);
            if self.frame(f).face_cache.is_some()
                && let Some(face) = self.face_from_id(f, face_id)
            {
                let (font, box_width) = (face.font, face.box_horizontal_line_width);
                if !font.is_nil() {
                    let (ascent, descent) = self.normal_char_ascent_descent(font, -1);
                    height = i64::from(ascent + descent);
                }
                if box_width > 0 {
                    height += 2 * i64::from(box_width);
                }
            }
            return height;
        }
        1
    }

    pub fn current_line_active_face_id(&self, w: Value, active: usize, inactive: usize) -> usize {
        let selected = self.windows.selected_window;
        self.line_active_face_id_3(w, selected, w, active, inactive)
    }

    pub fn message_with_string(&mut self, m: &str, string: Value, log: bool) -> LispResult<()> {
        self.check_string(string)?;
        let noninteractive = self.noninteractive();
        let interactive =
            !noninteractive && self.find_symbol_value(sym::EXECUTING_KBD_MACRO).is_nil();
        if !noninteractive && !interactive {
            return Ok(());
        }
        let fmt = self.string(m);
        let msg = crate::editfns::f_format_message(self, &[fmt, string])?;
        if noninteractive {
            self.message_to_stderr(msg)
        } else if log {
            self.message3(msg)
        } else {
            self.message3_nolog(msg)
        }
    }
}

pub fn init_xdisp(lisp: &mut Lisp) {
    let minibuf_window = lisp.windows.minibuf_window;
    lisp.xdisp.echo_area_window = minibuf_window;
    if !lisp.noninteractive {
        let m = minibuf_window;
        let f = lisp.window(m).frame;
        let root = lisp.frame(f).root_window;
        let top_margin = lisp.frame_top_margin(f);
        let g = lisp.frame(f).geometry.clone();
        let margins = g.menu_bar_lines + g.tab_bar_lines + g.tool_bar_lines;
        let r = lisp.window_mut(root);
        r.top_line = top_margin;
        r.pixel_top = r.top_line * g.line_height;
        r.total_cols = g.text_cols;
        r.pixel_width = r.total_cols * g.column_width;
        r.total_lines = g.total_lines - 1 - margins;
        r.pixel_height = r.total_lines * g.line_height;
        let mw = lisp.window_mut(m);
        mw.top_line = g.total_lines - 1;
        mw.pixel_top = mw.top_line * g.line_height;
        mw.total_cols = g.text_cols;
        mw.pixel_width = mw.total_cols * g.column_width;
        mw.total_lines = 1;
        mw.pixel_height = mw.total_lines * g.line_height;
        lisp.xdisp.scratch_glyph_row = append::scratch_glyph_row();
        let dot = Value::fixnum(i64::from(b'.'));
        lisp.xdisp.default_invis_vector = lisp.vector(vec![dot; 3]);
    }
    lisp.xdisp.help_echo_showing_p = false;
}

pub fn init_vars(lisp: &mut Lisp) {
    lisp.xdisp.default_invis_vector = lisp.vector(vec![Value::NIL; 3]);
    lisp.defvar_bool(sym::SCROLL_MINIBUFFER_CONSERVATIVELY, true);
    lisp.defvar_bool(sym::INHIBIT_MESSAGE, false);
    let name = lisp.string("*Messages*");
    lisp.defvar(sym::MESSAGES_BUFFER_NAME, name);
    lisp.defvar_bool(sym::X_STRETCH_CURSOR, false);
    lisp.defvar(sym::SHOW_TRAILING_WHITESPACE, Value::NIL);
    lisp.defvar(sym::MODE_LINE_COMPACT, Value::NIL);
    lisp.defvar(sym::NOBREAK_CHAR_DISPLAY, sym::T);
    lisp.defvar_bool(sym::NOBREAK_CHAR_ASCII_DISPLAY, false);
    lisp.defvar(sym::VOID_TEXT_AREA_POINTER, sym::ARROW);
    lisp.defvar(sym::INHIBIT_REDISPLAY, Value::NIL);
    lisp.defvar(sym::GLOBAL_MODE_STRING, Value::NIL);
    lisp.defvar(sym::OVERLAY_ARROW_POSITION, Value::NIL);
    let arrow = lisp.string("=>");
    lisp.defvar(sym::OVERLAY_ARROW_STRING, arrow);
    let arrows = lisp.list(&[sym::OVERLAY_ARROW_POSITION]);
    lisp.defvar(sym::OVERLAY_ARROW_VARIABLE_LIST, arrows);
    lisp.defvar_int(sym::SCROLL_STEP, 0);
    lisp.defvar_int(sym::SCROLL_CONSERVATIVELY, 0);
    lisp.defvar_int(sym::SCROLL_MARGIN, 0);
    let margin = lisp.float(0.25);
    lisp.defvar(sym::MAXIMUM_SCROLL_MARGIN, margin);
    let ppi = lisp.float(72.0);
    lisp.defvar(sym::DISPLAY_PIXELS_PER_INCH, ppi);
    lisp.defvar_int(sym::DEBUG_END_POS, 0);
    lisp.defvar(sym::TRUNCATE_PARTIAL_WIDTH_WINDOWS, Value::fixnum(50));
    lisp.defvar_bool(sym::WORD_WRAP_BY_CATEGORY, false);
    lisp.defvar(sym::LINE_NUMBER_DISPLAY_LIMIT, Value::NIL);
    lisp.defvar_int(sym::LINE_NUMBER_DISPLAY_LIMIT_WIDTH, 200);
    lisp.defvar_bool(sym::HIGHLIGHT_NONSELECTED_WINDOWS, false);
    lisp.defvar_bool(sym::MULTIPLE_FRAMES, false);
    let empty = lisp.empty_unibyte;
    let at = lisp.string("%b - GNU Emacs at ");
    lisp.stack.push(at);
    let icon_title_name_format = lisp.list(&[empty, at, sym::SYSTEM_NAME]);
    lisp.stack.push(icon_title_name_format);
    let b = lisp.string("%b");
    let format = lisp.list(&[sym::MULTIPLE_FRAMES, b, icon_title_name_format]);
    lisp.stack.truncate(lisp.stack.len() - 2);
    lisp.defvar(sym::FRAME_TITLE_FORMAT, format);
    lisp.defvar(sym::ICON_TITLE_FORMAT, format);
    lisp.defvar(sym::MESSAGE_LOG_MAX, Value::fixnum(1000));
    lisp.defvar(sym::WINDOW_SCROLL_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::MOUSE_AUTOSELECT_WINDOW, Value::NIL);
    lisp.defvar(sym::AUTO_RESIZE_TAB_BARS, sym::T);
    lisp.defvar_bool(sym::AUTO_RAISE_TAB_BAR_BUTTONS, true);
    lisp.defvar(sym::AUTO_RESIZE_TOOL_BARS, sym::T);
    lisp.defvar_bool(sym::AUTO_RAISE_TOOL_BAR_BUTTONS, true);
    lisp.defvar(sym::MAKE_CURSOR_LINE_FULLY_VISIBLE, sym::T);
    lisp.defvar_bool(sym::MAKE_WINDOW_START_VISIBLE, false);
    lisp.make_variable_buffer_local_builtin(sym::MAKE_WINDOW_START_VISIBLE);
    lisp.defvar_bool(sym::TAB_BAR_TRUNCATE, false);
    lisp.defvar(sym::TAB_BAR_BORDER, sym::INTERNAL_BORDER_WIDTH);
    lisp.defvar(sym::TAB_BAR_BUTTON_MARGIN, Value::fixnum(1));
    lisp.defvar_int(sym::TAB_BAR_BUTTON_RELIEF, 1);
    lisp.defvar(sym::TOOL_BAR_BORDER, sym::INTERNAL_BORDER_WIDTH);
    lisp.defvar(sym::TOOL_BAR_BUTTON_MARGIN, Value::fixnum(4));
    lisp.defvar_int(sym::TOOL_BAR_BUTTON_RELIEF, 1);
    lisp.defvar(sym::TOOL_BAR_STYLE, Value::NIL);
    lisp.defvar_int(sym::TOOL_BAR_MAX_LABEL_SIZE, 14);
    lisp.defvar(sym::FONTIFICATION_FUNCTIONS, Value::NIL);
    lisp.make_variable_buffer_local_builtin(sym::FONTIFICATION_FUNCTIONS);
    lisp.defvar_bool(sym::UNIBYTE_DISPLAY_VIA_LANGUAGE_ENVIRONMENT, false);
    let height = lisp.float(0.25);
    lisp.defvar(sym::MAX_MINI_WINDOW_HEIGHT, height);
    lisp.defvar(sym::RESIZE_MINI_WINDOWS, Value::NIL);
    lisp.defvar(sym::BLINK_CURSOR_ALIST, Value::NIL);
    lisp.defvar(sym::AUTO_HSCROLL_MODE, sym::T);
    lisp.defvar_int(sym::HSCROLL_MARGIN, 5);
    lisp.defvar(sym::HSCROLL_STEP, Value::fixnum(0));
    lisp.defvar_bool(sym::MESSAGE_TRUNCATE_LINES, false);
    lisp.defvar(sym::MENU_BAR_UPDATE_HOOK, Value::NIL);
    lisp.defvar(sym::MENU_UPDATING_FRAME, Value::NIL);
    lisp.defvar_bool(sym::INHIBIT_MENUBAR_UPDATE, false);
    for symbol in [
        sym::WRAP_PREFIX,
        sym::LINE_PREFIX,
        sym::DISPLAY_LINE_NUMBERS,
        sym::DISPLAY_LINE_NUMBERS_WIDTH,
    ] {
        lisp.defvar(symbol, Value::NIL);
        lisp.make_variable_buffer_local_builtin(symbol);
    }
    lisp.defvar(sym::DISPLAY_LINE_NUMBERS_CURRENT_ABSOLUTE, sym::T);
    lisp.defvar_bool(sym::DISPLAY_LINE_NUMBERS_WIDEN, false);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_LINE_NUMBERS_WIDEN);
    lisp.defvar_int(sym::DISPLAY_LINE_NUMBERS_OFFSET, 0);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_LINE_NUMBERS_OFFSET);
    lisp.defvar_bool(sym::DISPLAY_FILL_COLUMN_INDICATOR, false);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_FILL_COLUMN_INDICATOR);
    lisp.defvar(sym::DISPLAY_FILL_COLUMN_INDICATOR_COLUMN, sym::T);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_FILL_COLUMN_INDICATOR_COLUMN);
    lisp.defvar(sym::DISPLAY_FILL_COLUMN_INDICATOR_CHARACTER, Value::NIL);
    lisp.make_variable_buffer_local_builtin(sym::DISPLAY_FILL_COLUMN_INDICATOR_CHARACTER);
    lisp.defvar_int(sym::DISPLAY_LINE_NUMBERS_MAJOR_TICK, 0);
    lisp.defvar_int(sym::DISPLAY_LINE_NUMBERS_MINOR_TICK, 0);
    for symbol in [
        sym::INHIBIT_EVAL_DURING_REDISPLAY,
        sym::INHIBIT_FREE_REALIZED_FACES,
        sym::INHIBIT_BIDI_MIRRORING,
        sym::BIDI_INHIBIT_BPA,
        sym::INHIBIT_TRY_WINDOW_ID,
        sym::INHIBIT_TRY_WINDOW_REUSING,
        sym::INHIBIT_TRY_CURSOR_MOVEMENT,
    ] {
        lisp.defvar_bool(symbol, false);
    }
    lisp.defvar_int(sym::OVERLINE_MARGIN, 2);
    lisp.defvar_int(sym::UNDERLINE_MINIMUM_OFFSET, 1);
    lisp.defvar_bool(sym::UNDERLINE_LINE_SCALING_FLAG, false);
    lisp.defvar_bool(sym::OVERLINE_LINE_SCALING_FLAG, false);
    lisp.defvar_bool(sym::STRIKE_THROUGH_LINE_SCALING_FLAG, false);
    lisp.defvar_bool(sym::DISPLAY_HOURGLASS, true);
    lisp.defvar(sym::HOURGLASS_DELAY, Value::fixnum(1));
    let ignore = lisp.intern("ignore");
    lisp.defvar(sym::PRE_REDISPLAY_FUNCTION, ignore);
    lisp.put(
        sym::GLYPHLESS_CHAR_DISPLAY,
        sym::CHAR_TABLE_EXTRA_SLOTS,
        Value::fixnum(1),
    );
    let glyphless = lisp
        .make_char_table(sym::GLYPHLESS_CHAR_DISPLAY, Value::NIL)
        .expect("glyphless-char-display");
    lisp.char_table_mut(glyphless).extras[0] = sym::EMPTY_BOX;
    lisp.defvar(sym::GLYPHLESS_CHAR_DISPLAY, glyphless);
    let mirror = lisp
        .make_char_table(Value::NIL, Value::NIL)
        .expect("special-mirror-table");
    lisp.defvar(sym::SPECIAL_MIRROR_TABLE, mirror);
    lisp.defvar(sym::DEBUG_ON_MESSAGE, Value::NIL);
    lisp.defvar(sym::SET_MESSAGE_FUNCTION, Value::NIL);
    lisp.defvar(sym::CLEAR_MESSAGE_FUNCTION, Value::NIL);
    let all_windows = lisp.make_hash_table_with(sym::EQL, crate::object::Weakness::None);
    lisp.defvar(sym::REDISPLAY__ALL_WINDOWS_CAUSE, all_windows);
    let mode_lines = lisp.make_hash_table_with(sym::EQL, crate::object::Weakness::None);
    lisp.defvar(sym::REDISPLAY__MODE_LINES_CAUSE, mode_lines);
    lisp.defvar_bool(sym::REDISPLAY__INHIBIT_BIDI, true);
    lisp.defvar_bool(sym::DISPLAY_RAW_BYTES_AS_HEX, false);
    lisp.defvar_bool(sym::MOUSE_FINE_GRAINED_TRACKING, false);
    lisp.defvar_bool(sym::TAB_BAR__DRAGGING_IN_PROGRESS, false);
    lisp.defvar_bool(sym::REDISPLAY_SKIP_INITIAL_FRAME, true);
    lisp.defvar_bool(sym::REDISPLAY_SKIP_FONTIFICATION_ON_INPUT, false);
    lisp.defvar_bool(sym::REDISPLAY_ADHOC_SCROLL_IN_RESIZE_MINI_WINDOWS, true);
    lisp.defvar_bool(sym::COMPOSITION_BREAK_AT_POINT, false);
    lisp.defvar_int(sym::MAX_REDISPLAY_TICKS, 0);
}

impl Lisp {
    pub fn redisplay_other_windows(&mut self) {
        if self.windows.windows_or_buffers_changed == 0 {
            self.windows.windows_or_buffers_changed = REDISPLAY_SOME;
        }
    }

    pub fn wset_redisplay(&mut self, w: Value) {
        if w != self.windows.selected_window {
            self.redisplay_other_windows();
        }
        self.window_mut(w).flags.redisplay = true;
    }

    pub fn count_lines(&self, start_byte: usize, end_byte: usize) -> i64 {
        let zv = self.cur().zv as i64;
        self.display_count_lines(start_byte, end_byte, zv).0
    }

    pub fn display_count_lines(
        &self,
        mut start_byte: usize,
        limit_byte: usize,
        mut count: i64,
    ) -> (i64, usize) {
        let orig_count = count;
        let selective = self.find_symbol_value(sym::SELECTIVE_DISPLAY);
        let selective_display = !selective.is_nil() && !selective.is_fixnum();
        let text = &self.cur().text;
        let is_newline = |byte_pos: usize| {
            let c = text.byte_at(byte_pos);
            c == b'\n' || (selective_display && c == 0o15)
        };
        if count > 0 {
            while start_byte < limit_byte {
                if is_newline(start_byte) {
                    count -= 1;
                    if count == 0 {
                        return (orig_count, start_byte + 1);
                    }
                }
                start_byte += 1;
            }
        } else {
            while start_byte > limit_byte {
                start_byte -= 1;
                if is_newline(start_byte) {
                    count += 1;
                    if count == 0 {
                        return (-orig_count - 1, start_byte + 1);
                    }
                }
            }
        }
        if count < 0 {
            (-orig_count + count, limit_byte)
        } else {
            (orig_count - count, limit_byte)
        }
    }

    pub fn fset_redisplay(&mut self, f: Value) {
        self.redisplay_other_windows();
        self.frame_mut(f).flags.redisplay = true;
    }

    fn overlay_arrow_string_or_property(&self, var: Value) -> Value {
        let val = self.get(var, sym::OVERLAY_ARROW_STRING);
        if val.is_string() {
            return val;
        }
        self.plain_value(sym::OVERLAY_ARROW_STRING)
    }

    fn update_overlay_arrows(&mut self, up_to_date: i32) {
        let mut vlist = self.plain_value(sym::OVERLAY_ARROW_VARIABLE_LIST);
        while vlist.is_cons() {
            let var = self.xcar(vlist);
            vlist = self.xcdr(vlist);
            if !var.is_symbol() {
                continue;
            }
            if up_to_date > 0 {
                let val = self.find_symbol_value(var);
                if !self.is_marker(val) {
                    continue;
                }
                let position = self.coerce_marker(val);
                self.put(var, sym::LAST_ARROW_POSITION, position);
                let string = self.overlay_arrow_string_or_property(var);
                self.put(var, sym::LAST_ARROW_STRING, string);
            } else if up_to_date < 0 || !self.get(var, sym::LAST_ARROW_POSITION).is_nil() {
                self.put(var, sym::LAST_ARROW_POSITION, sym::T);
                self.put(var, sym::LAST_ARROW_STRING, sym::T);
            }
        }
    }

    fn mark_window_display_accurate_1(&mut self, w: Value, accurate_p: bool) {
        let b = self.window(w).contents;
        let text = &self.buffer(b).text;
        let (modiff, overlay_modiff, save_modiff) =
            (text.modiff, text.overlay_modiff, text.save_modiff);
        let window = self.window_mut(w);
        window.last_modified = if accurate_p { modiff } else { 0 };
        window.last_overlay_modified = if accurate_p { overlay_modiff } else { 0 };
        window.flags.last_had_star = modiff > save_modiff;
        if accurate_p {
            let header_line_p = self.window_wants_header_line(w);
            let tab_line_p = self.window_wants_tab_line(w);
            let last_point = if w == self.windows.selected_window {
                self.buffer(b).pt as i64
            } else {
                self.marker(self.window(w).pointm).charpos as i64
            };
            let mark = self.buffer(b).mark;
            let last_mark = if self.marker(mark).buffer == b
                && !self.buffer_local_value(sym::MARK_ACTIVE, b).is_nil()
            {
                self.marker(mark).charpos as i64
            } else {
                -1
            };
            let buffer = self.buffer_mut(b);
            buffer.clip_changed = false;
            buffer.prevent_redisplay_optimizations_p = false;
            let t = &mut buffer.text;
            t.redisplay = false;
            t.unchanged_modified = t.modiff;
            t.overlay_unchanged_modified = t.overlay_modiff;
            let gpt = t.gpt() as i64;
            t.beg_unchanged = gpt - 1;
            t.end_unchanged = t.chars as i64 + 1 - gpt;
            let (begv, zv) = (buffer.begv as i64, buffer.zv as i64);
            let window = self.window_mut(w);
            let m = window
                .current_matrix
                .as_mut()
                .expect("window has no current matrix");
            m.buffer = b;
            m.begv = begv;
            m.zv = zv;
            m.header_line_p = header_line_p;
            m.tab_line_p = tab_line_p;
            window.last_cursor_vpos = window.cursor.vpos;
            window.flags.last_cursor_off_p = window.flags.cursor_off_p;
            window.last_point = last_point;
            window.last_mark = last_mark;
            let end_vpos = window.window_end_vpos;
            let m = window.current_matrix.as_ref().expect("current matrix");
            if end_vpos < i64::from(m.nrows) {
                let row = &m.rows[end_vpos as usize];
                if !row.enabled_p || row.displays_text_p || end_vpos == 0 {
                    window.flags.window_end_valid = true;
                }
            }
            window.flags.update_mode_line = false;
            window.flags.preserve_vscroll_p = false;
        }
        self.window_mut(w).flags.redisplay = !accurate_p;
    }

    pub fn mark_window_display_accurate(&mut self, mut window: Value, accurate_p: bool) {
        while !window.is_nil() {
            let contents = self.window(window).contents;
            if self.is_window(contents) {
                self.mark_window_display_accurate(contents, accurate_p);
            } else {
                self.mark_window_display_accurate_1(window, accurate_p);
            }
            window = self.window(window).next;
        }
        self.update_overlay_arrows(if accurate_p { 1 } else { -1 });
    }

    pub fn bset_redisplay(&mut self, b: Value) {
        let count = self.buffer(b).window_count;
        if count > 0 {
            let sw = self.windows.selected_window;
            if count > 1 || !self.is_window(sw) || b != self.window(sw).contents {
                self.redisplay_other_windows();
            }
            self.buffer_mut(b).text.redisplay = true;
        }
    }

    pub fn bset_update_mode_line(&mut self, b: Value) {
        if self.windows.update_mode_lines == 0 {
            self.windows.update_mode_lines = REDISPLAY_SOME;
        }
        self.buffer_mut(b).text.redisplay = true;
    }

    pub fn wset_update_mode_line(&mut self, w: Value) {
        self.window_mut(w).flags.update_mode_line = true;
        self.wset_redisplay(w);
    }

    pub fn add_to_log(&mut self, format: &str, args: &[Value]) -> LispResult<()> {
        let base = self.stack.len();
        let fmt = self.string(format);
        self.stack.push(fmt);
        self.stack.extend_from_slice(args);
        let all = self.stack[base..].to_vec();
        let r = self.format_string(&all, true);
        self.stack.truncate(base);
        let msg = r?;
        let bytes = self.string_bytes(msg).to_vec();
        let multibyte = self.string_multibyte(msg);
        self.message_dolog(&bytes, true, multibyte)
    }

    fn message_log_insert(&mut self, bytes: &[u8], nchars: usize, before_markers: bool) {
        let (pt, pt_byte) = (self.cur().pt, self.cur().pt_byte);
        let multibyte = self.cur().text.multibyte;
        self.insert_from_gap(pt, pt_byte, bytes, nchars, before_markers);
        let n = if multibyte { nchars } else { bytes.len() };
        let b = self.cur_mut();
        b.pt = pt + n;
        b.pt_byte = pt_byte + bytes.len();
    }

    fn temp_set_pt(&mut self, pos: usize) {
        let b = self.cur_mut();
        b.pt = pos;
        b.pt_byte = b.text.char_to_byte(pos);
    }

    pub fn message_dolog(&mut self, msg: &[u8], nlflag: bool, multibyte: bool) -> LispResult<()> {
        if !self.find_symbol_value(sym::MEMORY_FULL).is_nil() {
            return Ok(());
        }
        if self.find_symbol_value(sym::MESSAGE_LOG_MAX).is_nil() {
            return Ok(());
        }
        let old_windows_or_buffers_changed = self.windows.windows_or_buffers_changed;
        let old_deactivate_mark = self.bound_var(sym::DEACTIVATE_MARK);
        let oldbuf = self.current_buffer;
        let base = self.stack.len();
        self.stack.push(old_deactivate_mark);
        self.stack.push(oldbuf);
        let r = self.message_dolog_1(msg, nlflag, multibyte, old_windows_or_buffers_changed);
        self.stack.truncate(base);
        r?;
        self.set_buffer_internal(oldbuf);
        self.xdisp.message_log_need_newline = !nlflag;
        self.set_plain_value(sym::DEACTIVATE_MARK, old_deactivate_mark);
        Ok(())
    }

    fn message_dolog_1(
        &mut self,
        msg: &[u8],
        nlflag: bool,
        multibyte: bool,
        old_windows_or_buffers_changed: i64,
    ) -> LispResult<()> {
        if !self
            .find_symbol_value(sym::MESSAGES_BUFFER_NAME)
            .is_string()
        {
            let name = self.string("*Messages*");
            self.set_plain_value(sym::MESSAGES_BUFFER_NAME, name);
        }
        let name = self.find_symbol_value(sym::MESSAGES_BUFFER_NAME);
        let newbuffer = crate::buffer::f_get_buffer(self, &[name])?.is_nil();
        let name = self.find_symbol_value(sym::MESSAGES_BUFFER_NAME);
        let buf = self.get_buffer_create(name, false)?;
        crate::buffer::f_set_buffer(self, &[buf])?;
        if newbuffer
            && !self
                .heap
                .symbol_ref(sym::MESSAGES_BUFFER_MODE)
                .function
                .is_nil()
        {
            self.funcall(sym::MESSAGES_BUFFER_MODE, &[])?;
        }
        self.set_undo_list(sym::T);
        let cb = self.current_buffer;
        self.buffer_local_set(cb, sym::CACHE_LONG_SCANS, Value::NIL);
        let (pt, begv, zv) = (self.cur().pt, self.cur().begv, self.cur().zv);
        let oldpoint = self.make_marker_at(pt, cb, false);
        let oldbegv = self.make_marker_at(begv, cb, false);
        let oldzv = self.make_marker_at(zv, cb, false);
        let z = self.cur().z();
        let point_at_end = pt == z;
        let zv_at_end = zv == z;
        {
            let b = self.cur_mut();
            let (z, z_byte) = (b.z(), b.z_byte());
            b.begv = 1;
            b.begv_byte = 1;
            b.zv = z;
            b.zv_byte = z_byte;
            b.pt = z;
            b.pt_byte = z_byte;
        }
        let buffer_multibyte = self.cur().text.multibyte;
        if multibyte && !buffer_multibyte {
            let bytes: Vec<u8> = character::multibyte_chars(msg)
                .map(character::char_to_byte8)
                .collect();
            for b in bytes {
                self.message_log_insert(&[b], 1, false);
            }
        } else if !multibyte && buffer_multibyte {
            for &b in msg {
                let s = character::str_to_multibyte(&[b]);
                self.message_log_insert(&s, 1, false);
            }
        } else if !msg.is_empty() {
            let nchars = if buffer_multibyte {
                character::chars_in_multibyte(msg)
            } else {
                msg.len()
            };
            self.message_log_insert(msg, nchars, false);
        }
        if nlflag {
            let count = self.specpdl.len();
            self.specbind(sym::INHIBIT_MODIFICATION_HOOKS, sym::T)?;
            let r = self.message_dolog_newline();
            let u = self.unbind_to(count);
            r?;
            u?;
        }
        let begv = self.marker(oldbegv).charpos;
        let zv = self.marker(oldzv).charpos;
        let point = self.marker(oldpoint).charpos;
        {
            let b = self.cur_mut();
            b.begv = begv;
            b.begv_byte = b.text.char_to_byte(begv);
            if zv_at_end {
                b.zv = b.z();
                b.zv_byte = b.z_byte();
            } else {
                b.zv = zv;
                b.zv_byte = b.text.char_to_byte(zv);
            }
        }
        if point_at_end {
            let z = self.cur().z();
            self.temp_set_pt(z);
        } else {
            self.temp_set_pt(point);
        }
        self.unchain_marker(oldpoint);
        self.unchain_marker(oldbegv);
        self.unchain_marker(oldzv);
        self.windows.windows_or_buffers_changed = old_windows_or_buffers_changed;
        let cb = self.current_buffer;
        self.bset_redisplay(cb);
        Ok(())
    }

    fn message_dolog_newline(&mut self) -> LispResult<()> {
        self.message_log_insert(b"\n", 1, false);
        let z = self.cur().z();
        let (this_bol, _) = self.find_newline(z, 1, -2);
        self.temp_set_pt(this_bol);
        let this_bol_byte = self.cur().pt_byte;
        if this_bol > 1 {
            let (prev_bol, _) = self.find_newline(this_bol, 1, -2);
            self.temp_set_pt(prev_bol);
            let prev_bol_byte = self.cur().pt_byte;
            let dups = self.message_log_check_duplicate(prev_bol_byte, this_bol_byte);
            if dups != 0 {
                self.del_range_1(prev_bol, this_bol, false)?;
                if dups > 1 {
                    let dupstr = format!(" [{dups} times]");
                    let z = self.cur().z();
                    self.temp_set_pt(z - 1);
                    self.message_log_insert(dupstr.as_bytes(), dupstr.len(), true);
                }
            }
        }
        let max = self.find_symbol_value(sym::MESSAGE_LOG_MAX);
        if let Some(n) = max.as_fixnum().filter(|&n| n >= 0) {
            let z = self.cur().z();
            let (pos, _) = self.find_newline(z, 1, -n - 1);
            self.temp_set_pt(pos);
            self.del_range_1(1, pos, false)?;
        }
        Ok(())
    }

    fn message_log_check_duplicate(&self, prev_bol_byte: usize, this_bol_byte: usize) -> i64 {
        let b = self.cur();
        let z_byte = b.z_byte();
        let tail = b.text.slice(prev_bol_byte - 1, z_byte - 1);
        let len = z_byte - 1 - this_bol_byte;
        let offset = this_bol_byte - prev_bol_byte;
        let p1 = &tail[..];
        let p2 = &tail[offset..];
        let mut seen_dots = false;
        for i in 0..len {
            if i >= 3 && p1[i - 3] == b'.' && p1[i - 2] == b'.' && p1[i - 1] == b'.' {
                seen_dots = true;
            }
            if p1[i] != p2[i] {
                return i64::from(seen_dots);
            }
        }
        let rest = &p1[len..];
        if rest.first() == Some(&b'\n') {
            return 2;
        }
        if rest.first() == Some(&b' ') && rest.get(1) == Some(&b'[') {
            let digits = &rest[2..];
            let mut i = 0;
            while digits.get(i).is_some_and(|c| c.is_ascii_whitespace()) {
                i += 1;
            }
            let start = i;
            if matches!(digits.get(i), Some(b'+') | Some(b'-')) {
                i += 1;
            }
            let num_start = i;
            while digits.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i > num_start {
                let n = std::str::from_utf8(&digits[start..i])
                    .ok()
                    .and_then(|s| s.trim_start().parse::<i64>().ok())
                    .unwrap_or(i64::MAX);
                if 0 < n && n < i64::MAX && digits[i..].starts_with(b" times]\n") {
                    return n + 1;
                }
            }
        }
        0
    }
}

impl Lisp {
    pub fn get_char_glyph_code(&mut self, c: u32, font: Value) -> Option<u32> {
        let code = if character::char_byte8_p(c) {
            u32::from(character::char_to_byte8(c))
        } else {
            let encode_char = self
                .font_object(font)
                .driver
                .expect("font driver")
                .encode_char;
            encode_char(self, font, c as i32)
        };
        (code != crate::font::FONT_INVALID_CODE).then_some(code & 0xFFFF)
    }

    pub fn get_per_char_metric(
        &mut self,
        font: Value,
        char2b: u32,
    ) -> Option<crate::font::FontMetrics> {
        if char2b == crate::font::FONT_INVALID_CODE {
            return None;
        }
        let text_extents = self
            .font_object(font)
            .driver
            .expect("font driver")
            .text_extents;
        Some(text_extents(self, font, &[char2b]))
    }

    pub fn normal_char_ascent_descent(&mut self, font: Value, c: i32) -> (i32, i32) {
        let f = self.font_object(font);
        let mut ascent = f.ascent;
        let mut descent = f.descent;
        if f.pixel_size > 0 && f.ascent + f.descent > 3 * f.pixel_size {
            let c = if c >= 0 { c as u32 } else { u32::from(b'{') };
            if let Some(char2b) = self.get_char_glyph_code(c, font)
                && let Some(pcm) = self.get_per_char_metric(font, char2b)
                && !(pcm.width == 0 && pcm.rbearing == 0 && pcm.lbearing == 0)
            {
                ascent = pcm.ascent + 1;
                descent = pcm.descent + 1;
            }
        }
        (ascent, descent)
    }

    pub fn get_font_ascent_descent(&mut self, font: Value) -> (i32, i32) {
        self.normal_char_ascent_descent(font, -1)
    }

    pub fn normal_char_height(&mut self, font: Value, c: i32) -> i32 {
        let (ascent, descent) = self.normal_char_ascent_descent(font, c);
        ascent + descent
    }
}

impl Lisp {
    pub fn get_large_narrowing_begv(&self, pos: usize) -> usize {
        let b = self.cur();
        let size = self.int_var(sym::LONG_LINE_OPTIMIZATIONS_REGION_SIZE);
        if size <= 0 {
            return b.begv;
        }
        let len = size / 2;
        let mut begv = (pos as i64 - len).max(b.begv as i64) as usize;
        let mut limit = self.int_var(sym::LONG_LINE_OPTIMIZATIONS_BOL_SEARCH_LIMIT);
        while limit > 0 {
            if begv == b.begv || b.text.byte_at(b.text.char_to_byte(begv) - 1) == b'\n' {
                return begv;
            }
            begv -= 1;
            limit -= 1;
        }
        begv
    }

    pub fn get_large_narrowing_zv(&self, pos: usize) -> usize {
        let zv = self.cur().zv;
        let size = self.int_var(sym::LONG_LINE_OPTIMIZATIONS_REGION_SIZE);
        if size <= 0 {
            return zv;
        }
        (pos as i64 + size / 2).min(zv as i64) as usize
    }

    pub fn update_redisplay_ticks(&mut self, ticks: i64, w: Value) -> LispResult<()> {
        if ticks == 0 && w != self.xdisp.redisplay_ticks_window {
            self.xdisp.redisplay_ticks_window = w;
            self.xdisp.window_ticks = 0;
        }
        if (w.is_nil() && !self.xdisp.redisplaying_p && !self.xdisp.display_working_on_window_p)
            || (!w.is_nil() && self.window(w).mini)
        {
            return Ok(());
        }
        if ticks > 0 {
            self.xdisp.window_ticks += ticks;
        }
        let max_redisplay_ticks = self.int_var(sym::MAX_REDISPLAY_TICKS);
        if max_redisplay_ticks > 0 && self.xdisp.window_ticks > max_redisplay_ticks {
            let contents = if w.is_nil() {
                Value::NIL
            } else {
                self.window(w).contents
            };
            let bufname = if contents.is_nil() {
                self.string_to_rust(self.cur().name)
            } else if self.is_buffer(contents) {
                self.string_to_rust(self.buffer(contents).name)
            } else {
                "<unknown>".to_string()
            };
            self.windows.windows_or_buffers_changed = 177;
            return Err(self.verror(
                "Window showing buffer %s takes too long to redisplay",
                &[crate::doprnt::Arg::Str(bufname.as_bytes())],
            ));
        }
        Ok(())
    }

    fn safe_eval_handler(&mut self, arg: Value, args: &[Value]) -> LispResult {
        let list = self.list(args);
        self.add_to_log("Error during redisplay: %S signaled %S", &[list, arg])?;
        Ok(Value::NIL)
    }

    pub fn safe_call(&mut self, inhibit_quit: bool, args: &[Value]) -> LispResult {
        if self.var_true(sym::INHIBIT_EVAL_DURING_REDISPLAY) {
            return Ok(Value::NIL);
        }
        let count = self.specpdl_index();
        self.specbind(sym::INHIBIT_REDISPLAY, sym::T)?;
        if inhibit_quit {
            self.specbind(sym::INHIBIT_QUIT, sym::T)?;
        }
        let base = self.stack.len();
        self.stack.extend_from_slice(args);
        let redisplay_counter_before = self.xdisp.redisplay_counter;
        let mut r = self.condition_case_internal(
            sym::T,
            |l| l.funcall(args[0], &args[1..]),
            |l, error| l.safe_eval_handler(error, args),
        );
        self.stack.truncate(base);
        if redisplay_counter_before != self.xdisp.redisplay_counter {
            r = crate::keyboard::cmdloop::f_top_level(self, &[]);
        }
        self.unbind_to_value(count, r)
    }

    pub fn dsafe_eval(&mut self, sexpr: Value) -> LispResult {
        self.safe_call(true, &[sym::EVAL, sexpr])
    }

    pub fn valid_image_p(&self, object: Value) -> bool {
        if object.is_cons() && self.xcar(object) == sym::IMAGE {
            unreachable!("valid_image_p needs image.c's image types");
        }
        false
    }

    pub fn compute_display_string_pos(
        &mut self,
        (charpos, bytepos): (i64, i64),
        string: &crate::bidi::BidiStringData,
        w: Value,
        frame_window_p: bool,
        disp_prop: &mut i32,
    ) -> LispResult<i64> {
        const MAX_DISP_SCAN: i64 = 250;
        let string_p = string.lstring.is_string() || string.s.is_some();
        let eob = if string_p {
            string.schars
        } else {
            self.cur().zv as i64
        };
        let begb = if string_p { 0 } else { self.cur().begv as i64 };
        let lim = if charpos < eob - MAX_DISP_SCAN {
            charpos + MAX_DISP_SCAN
        } else {
            eob
        };
        let (object, object1) = if string.lstring.is_string() {
            (string.lstring, string.lstring)
        } else if !w.is_nil() && !string_p {
            (w, Value::NIL)
        } else {
            (Value::NIL, Value::NIL)
        };
        *disp_prop = 1;
        if charpos >= eob || string.from_disp_str || (string.s.is_some() && !object.is_string()) {
            *disp_prop = 0;
            return Ok(eob);
        }
        let mut pos = Value::fixnum(charpos);
        let mut bufpos = if object.is_string() {
            string.bufpos
        } else {
            charpos
        };
        let mut tpos = (charpos, bytepos);
        let ctx = |position: i64, bufpos: i64| DisplaySpecContext {
            object: object1,
            position: position as usize,
            bufpos: bufpos as usize,
            frame_window_p,
            enable_eval: true,
        };
        let (spec, overlay) = self.get_char_property_and_overlay(pos, sym::DISPLAY, object)?;
        if !spec.is_nil() {
            let differs = charpos <= begb || {
                let before = Value::fixnum(charpos - 1);
                let (spec1, ov1) =
                    self.get_char_property_and_overlay(before, sym::DISPLAY, object)?;
                spec1 != spec || overlay != ov1
            };
            if differs {
                let rv = self.handle_display_spec_without_iterator(spec, ctx(tpos.0, bufpos))?;
                if rv != 0 {
                    if rv == 2 {
                        *disp_prop = 2;
                    }
                    return Ok(charpos);
                }
            }
        }
        let limpos = Value::fixnum(lim);
        let mut rv;
        loop {
            pos = crate::textprop::f_next_single_char_property_change(
                self,
                &[pos, sym::DISPLAY, object1, limpos],
            )?;
            tpos.0 = pos.as_fixnum().unwrap_or(0);
            if tpos.0 >= lim {
                *disp_prop = 0;
                rv = 0;
                break;
            }
            tpos.1 = if object.is_string() {
                self.string_char_to_byte_index(object, tpos.0 as usize) as i64
            } else {
                self.cur().text.char_to_byte(tpos.0 as usize) as i64
            };
            let (spec, _overlay) = self.get_char_property_and_overlay(pos, sym::DISPLAY, object)?;
            if !object.is_string() {
                bufpos = tpos.0;
            }
            if spec.is_nil() {
                continue;
            }
            rv = self.handle_display_spec_without_iterator(spec, ctx(tpos.0, bufpos))?;
            if rv != 0 {
                break;
            }
        }
        if rv == 2 {
            *disp_prop = 2;
        }
        Ok(tpos.0)
    }

    pub fn compute_display_string_end(
        &mut self,
        charpos: i64,
        string: &crate::bidi::BidiStringData,
        w: Value,
    ) -> LispResult<i64> {
        let mut object = if string.lstring.is_string() {
            string.lstring
        } else if self.is_window(w) {
            w
        } else {
            Value::NIL
        };
        let pos = Value::fixnum(charpos);
        let eob = if object.is_string() || string.s.is_some() {
            string.schars
        } else {
            self.cur().zv as i64
        };
        if charpos >= eob || (string.s.is_some() && !object.is_string()) {
            return Ok(eob);
        }
        let (spec, overlay) = self.get_char_property_and_overlay(pos, sym::DISPLAY, object)?;
        if spec.is_nil() {
            return Ok(-1);
        }
        if self.is_overlay(overlay) {
            return Ok(self.overlay(overlay).end as i64);
        }
        if self.is_window(object) {
            object = self.window(w).contents;
        }
        let pos = crate::textprop::f_next_single_char_property_change(
            self,
            &[pos, sym::DISPLAY, object, Value::NIL],
        )?;
        Ok(pos.as_fixnum().unwrap_or(0))
    }

    pub fn display_prop_intangible_p(&mut self, prop: Value, charpos: usize) -> LispResult<bool> {
        let frame_window_p = self.frame_window_p(self.frames.selected_frame);
        let ctx = DisplaySpecContext {
            object: Value::NIL,
            position: charpos,
            bufpos: charpos,
            frame_window_p,
            enable_eval: true,
        };
        Ok(self.handle_display_spec_without_iterator(prop, ctx)? != 0)
    }

    fn handle_display_spec_without_iterator(
        &mut self,
        mut spec: Value,
        mut ctx: DisplaySpecContext,
    ) -> LispResult<i32> {
        let mut replacing = 0;
        if spec.is_cons() && self.xcar(spec) == sym::DISABLE_EVAL {
            ctx.enable_eval = false;
            let rest = self.xcdr(spec);
            spec = if rest.is_cons() {
                self.xcar(rest)
            } else {
                Value::NIL
            };
        }
        let simple_list = spec.is_cons() && {
            let head = self.xcar(spec);
            ![
                sym::IMAGE,
                sym::SPACE,
                sym::WHEN,
                sym::SLICE,
                sym::SPACE_WIDTH,
                sym::HEIGHT,
                sym::RAISE,
                sym::LEFT_FRINGE,
                sym::RIGHT_FRINGE,
                sym::MIN_WIDTH,
            ]
            .contains(&head)
                && !(head.is_cons() && self.xcar(head) == sym::MARGIN)
                && !head.is_nil()
        };
        if simple_list {
            let mut tail = spec;
            while tail.is_cons() {
                let item = self.xcar(tail);
                let rv = self.handle_single_display_spec_without_iterator(item, ctx, replacing)?;
                if rv != 0 {
                    replacing = rv;
                    break;
                }
                tail = self.xcdr(tail);
            }
        } else if self.is_vector(spec) {
            let mut i = 0;
            while i < self.vector_items(spec).len() {
                let item = self.vector_items(spec)[i];
                let rv = self.handle_single_display_spec_without_iterator(item, ctx, replacing)?;
                if rv != 0 {
                    replacing = rv;
                    break;
                }
                i += 1;
            }
        } else {
            replacing = self.handle_single_display_spec_without_iterator(spec, ctx, 0)?;
        }
        Ok(replacing)
    }

    fn handle_single_display_spec_without_iterator(
        &mut self,
        mut spec: Value,
        ctx: DisplaySpecContext,
        display_replaced: i32,
    ) -> LispResult<i32> {
        let mut form = sym::T;
        if spec.is_cons() && self.xcar(spec) == sym::WHEN {
            spec = self.xcdr(spec);
            if !spec.is_cons() {
                return Ok(0);
            }
            form = self.xcar(spec);
            spec = self.xcdr(spec);
        }
        if !form.is_nil() && form != sym::T && !ctx.enable_eval {
            form = Value::NIL;
        }
        if !form.is_nil() && form != sym::T {
            let count = self.specpdl_index();
            let object = if ctx.object.is_nil() {
                self.current_buffer
            } else {
                ctx.object
            };
            self.specbind(sym::OBJECT, object)?;
            self.specbind(sym::POSITION, Value::fixnum(ctx.position as i64))?;
            self.specbind(sym::BUFFER_POSITION, Value::fixnum(ctx.bufpos as i64))?;
            self.stack.push(spec);
            let r = self.dsafe_eval(form);
            self.stack.pop();
            form = self.unbind_to_value(count, r)?;
        }
        if form.is_nil() {
            return Ok(0);
        }
        let head = self.car_safe(spec);
        let rest = self.cdr_safe(spec);
        if spec.is_cons()
            && (head == sym::HEIGHT || head == sym::SPACE_WIDTH || head == sym::RAISE)
            && rest.is_cons()
        {
            return Ok(0);
        }
        if spec.is_cons() && head == sym::MIN_WIDTH && rest.is_cons() && self.xcar(rest).is_cons() {
            return Ok(0);
        }
        if spec.is_cons() && head == sym::SLICE {
            return Ok(0);
        }
        if spec.is_cons()
            && (head == sym::LEFT_FRINGE || head == sym::RIGHT_FRINGE)
            && rest.is_cons()
        {
            return Ok(1);
        }
        let mut location = sym::UNBOUND;
        let mut value = Value::NIL;
        if spec.is_cons() && head.is_cons() {
            value = rest;
            if value.is_cons() {
                value = self.xcar(value);
            }
            if self.xcar(head) == sym::MARGIN {
                let tem = self.xcdr(head);
                let tem = if tem.is_cons() {
                    self.xcar(tem)
                } else {
                    Value::NIL
                };
                if tem.is_nil() || tem == sym::LEFT_MARGIN || tem == sym::RIGHT_MARGIN {
                    location = tem;
                }
            }
        }
        if location == sym::UNBOUND {
            location = Value::NIL;
            value = spec;
        }
        let space = value.is_cons() && self.xcar(value) == sym::SPACE;
        let valid_p =
            value.is_string() || (ctx.frame_window_p && self.valid_image_p(value)) || space;
        if valid_p && display_replaced == 0 {
            return Ok(if space && location.is_nil() { 2 } else { 1 });
        }
        Ok(0)
    }
}
