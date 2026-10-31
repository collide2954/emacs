use std::cell::Cell;

use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::lread::{Source, SourceKind};
use crate::object::{Localized, PerBuffer, PropRun, SymbolValue, TrappedWrite, Vectorlike};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

#[derive(Clone, Debug)]
pub struct BufferText {
    data: Vec<u8>,
    gap_start: usize,
    gap_len: usize,
    pub chars: usize,
    pub multibyte: bool,
    pub props: Vec<PropRun>,
    pub markers: Vec<Value>,
    pub modiff: i64,
    pub chars_modiff: i64,
    pub save_modiff: i64,
    pub overlay_modiff: i64,
    pub autosave_modiff: i64,
    pub redisplay: bool,
    cache: Cell<(usize, usize)>,
}

impl Default for BufferText {
    fn default() -> Self {
        BufferText {
            data: vec![0; 64],
            gap_start: 0,
            gap_len: 64,
            chars: 0,
            multibyte: true,
            props: Vec::new(),
            markers: Vec::new(),
            modiff: 1,
            chars_modiff: 1,
            save_modiff: 1,
            overlay_modiff: 1,
            autosave_modiff: 1,
            redisplay: false,
            cache: Cell::new((1, 1)),
        }
    }
}

impl BufferText {
    pub fn bytes(&self) -> usize {
        self.data.len() - self.gap_len
    }

    fn byte_at_offset(&self, offset: usize) -> u8 {
        if offset < self.gap_start {
            self.data[offset]
        } else {
            self.data[offset + self.gap_len]
        }
    }

    pub fn byte_at(&self, byte_pos: usize) -> u8 {
        self.byte_at_offset(byte_pos - 1)
    }

    pub fn set_byte(&mut self, byte_pos: usize, value: u8) {
        let offset = byte_pos - 1;
        if offset < self.gap_start {
            self.data[offset] = value;
        } else {
            self.data[offset + self.gap_len] = value;
        }
    }

    pub fn slice(&self, from: usize, to: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(to - from);
        if to <= self.gap_start {
            out.extend_from_slice(&self.data[from..to]);
        } else if from >= self.gap_start {
            out.extend_from_slice(&self.data[from + self.gap_len..to + self.gap_len]);
        } else {
            out.extend_from_slice(&self.data[from..self.gap_start]);
            out.extend_from_slice(&self.data[self.gap_start + self.gap_len..to + self.gap_len]);
        }
        out
    }

    fn contiguous_from(&self, offset: usize, len: usize) -> Vec<u8> {
        let end = (offset + len).min(self.bytes());
        self.slice(offset, end)
    }

    pub fn char_at_byte(&self, byte_pos: usize) -> u32 {
        let offset = byte_pos - 1;
        let first = self.byte_at_offset(offset);
        if !self.multibyte {
            return first as u32;
        }
        if first < 0x80 {
            return first as u32;
        }
        let bytes = self.contiguous_from(offset, character::MAX_MULTIBYTE_LENGTH);
        character::string_char(&bytes).0
    }

    pub fn char_len_at_byte(&self, byte_pos: usize) -> usize {
        if !self.multibyte {
            return 1;
        }
        let offset = byte_pos - 1;
        let first = self.byte_at_offset(offset);
        if first < 0x80 {
            return 1;
        }
        let bytes = self.contiguous_from(offset, character::MAX_MULTIBYTE_LENGTH);
        character::multibyte_length(&bytes).unwrap_or(1)
    }

    pub fn prev_char_len(&self, byte_pos: usize) -> usize {
        if !self.multibyte || byte_pos <= 1 {
            return 1;
        }
        let mut start = byte_pos - 1;
        let limit = byte_pos
            .saturating_sub(character::MAX_MULTIBYTE_LENGTH)
            .max(1);
        while start > limit && self.byte_at_offset(start - 1) & 0xC0 == 0x80 {
            start -= 1;
        }
        let len = byte_pos - start;
        if self.char_len_at_byte(start) == len {
            len
        } else {
            1
        }
    }

    pub fn char_to_byte(&self, pos: usize) -> usize {
        if !self.multibyte || self.chars == self.bytes() {
            return pos;
        }
        let (cached_char, cached_byte) = self.cache.get();
        let candidates = [
            (1usize, 1usize),
            (self.chars + 1, self.bytes() + 1),
            (cached_char, cached_byte),
        ];
        let (mut c, mut b) = candidates
            .iter()
            .copied()
            .filter(|&(cc, _)| cc >= 1 && cc <= self.chars + 1)
            .min_by_key(|&(cc, _)| cc.abs_diff(pos))
            .unwrap_or((1, 1));
        while c < pos {
            b += self.char_len_at_byte(b);
            c += 1;
        }
        while c > pos {
            b -= self.prev_char_len(b);
            c -= 1;
        }
        self.cache.set((c, b));
        b
    }

    pub fn byte_to_char(&self, byte_pos: usize) -> usize {
        if !self.multibyte || self.chars == self.bytes() {
            return byte_pos;
        }
        let (cached_char, cached_byte) = self.cache.get();
        let candidates = [
            (1usize, 1usize),
            (self.chars + 1, self.bytes() + 1),
            (cached_char, cached_byte),
        ];
        let (mut c, mut b) = candidates
            .iter()
            .copied()
            .filter(|&(_, bb)| bb >= 1 && bb <= self.bytes() + 1)
            .min_by_key(|&(_, bb)| bb.abs_diff(byte_pos))
            .unwrap_or((1, 1));
        while b < byte_pos {
            b += self.char_len_at_byte(b);
            c += 1;
        }
        while b > byte_pos {
            b -= self.prev_char_len(b);
            c -= 1;
        }
        self.cache.set((c, b));
        c
    }

    fn move_gap(&mut self, offset: usize) {
        if offset < self.gap_start {
            let n = self.gap_start - offset;
            self.data
                .copy_within(offset..self.gap_start, offset + self.gap_len);
            self.gap_start -= n;
        } else if offset > self.gap_start {
            let n = offset - self.gap_start;
            let src = self.gap_start + self.gap_len;
            self.data.copy_within(src..src + n, self.gap_start);
            self.gap_start += n;
        }
    }

    fn ensure_gap(&mut self, needed: usize) {
        if self.gap_len >= needed {
            return;
        }
        let extra = needed.max(self.data.len() / 2).max(64);
        let tail_start = self.gap_start + self.gap_len;
        let tail: Vec<u8> = self.data[tail_start..].to_vec();
        self.data.truncate(self.gap_start);
        self.data.resize(self.gap_start + self.gap_len + extra, 0);
        self.gap_len += extra;
        self.data.extend_from_slice(&tail);
    }

    pub fn insert_bytes(&mut self, byte_pos: usize, bytes: &[u8], nchars: usize) {
        let offset = byte_pos - 1;
        self.move_gap(offset);
        self.ensure_gap(bytes.len());
        self.data[self.gap_start..self.gap_start + bytes.len()].copy_from_slice(bytes);
        self.gap_start += bytes.len();
        self.gap_len -= bytes.len();
        self.chars += nchars;
        self.cache.set((1, 1));
    }

    pub fn delete_bytes(&mut self, from_byte: usize, to_byte: usize, nchars: usize) {
        let from = from_byte - 1;
        let to = to_byte - 1;
        self.move_gap(to);
        self.gap_start = from;
        self.gap_len += to - from;
        self.chars -= nchars;
        self.cache.set((1, 1));
    }

    pub fn set_contents(&mut self, bytes: Vec<u8>, chars: usize, multibyte: bool) {
        let len = bytes.len();
        self.data = bytes;
        self.data.resize(len + 64, 0);
        self.gap_start = len;
        self.gap_len = 64;
        self.chars = chars;
        self.multibyte = multibyte;
        self.cache.set((1, 1));
    }

    pub fn all_bytes(&self) -> Vec<u8> {
        self.slice(0, self.bytes())
    }

    pub fn parts(&self) -> (&[u8], &[u8]) {
        (
            &self.data[..self.gap_start],
            &self.data[self.gap_start + self.gap_len..],
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Marker {
    pub buffer: Value,
    pub charpos: usize,
    pub bytepos: usize,
    pub insertion_type: bool,
}

#[derive(Clone, Debug)]
pub struct Overlay {
    pub buffer: Value,
    pub start: usize,
    pub end: usize,
    pub plist: Value,
    pub front_advance: bool,
    pub rear_advance: bool,
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub name: Value,
    pub last_name: Value,
    pub text: BufferText,
    pub pt: usize,
    pub pt_byte: usize,
    pub begv: usize,
    pub begv_byte: usize,
    pub zv: usize,
    pub zv_byte: usize,
    pub local_vars: Vec<(Value, Value)>,
    pub overlays: Vec<Value>,
    pub mark: Value,
    pub keymap: Value,
    pub syntax_table: Value,
    pub category_table: Value,
    pub downcase_table: Value,
    pub upcase_table: Value,
    pub case_canon_table: Value,
    pub case_eqv_table: Value,
    pub base_buffer: Value,
    pub inhibit_hooks: bool,
    pub prevent_redisplay_optimizations_p: bool,
    pub long_line_optimizations_p: bool,
    pub last_window_start: usize,
    pub window_count: usize,
    pub point_marker: Value,
    pub modtime: (i64, i64),
    pub modtime_size: i64,
    pub auto_save_failure_time: i64,
    pub last_selected_window: Value,
}

impl Buffer {
    pub fn trace(&self, out: &mut Vec<Value>) {
        out.push(self.name);
        out.push(self.last_name);
        for (k, v) in &self.local_vars {
            out.push(*k);
            out.push(*v);
        }
        out.extend_from_slice(&self.overlays);
        out.push(self.mark);
        out.push(self.keymap);
        out.push(self.syntax_table);
        out.push(self.category_table);
        out.push(self.downcase_table);
        out.push(self.upcase_table);
        out.push(self.case_canon_table);
        out.push(self.case_eqv_table);
        out.push(self.base_buffer);
        out.push(self.point_marker);
        out.push(self.last_selected_window);
        out.extend(self.text.props.iter().map(|r| r.plist));
    }

    pub fn z(&self) -> usize {
        self.text.chars + 1
    }

    pub fn z_byte(&self) -> usize {
        self.text.bytes() + 1
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("buffer-live-p", 1, 1, f_buffer_live_p),
    Subr::fixed("buffer-list", 0, 1, f_buffer_list),
    Subr::fixed("force-mode-line-update", 0, 1, f_force_mode_line_update),
    Subr::fixed("get-buffer", 1, 1, f_get_buffer),
    Subr::fixed("get-file-buffer", 1, 1, f_get_file_buffer),
    Subr::fixed("get-truename-buffer", 1, 1, f_get_truename_buffer),
    Subr::fixed("find-buffer", 2, 2, f_find_buffer),
    Subr::fixed("get-buffer-create", 1, 2, f_get_buffer_create),
    Subr::fixed("generate-new-buffer-name", 1, 2, f_generate_new_buffer_name),
    Subr::fixed("buffer-name", 0, 1, f_buffer_name),
    Subr::fixed("buffer-last-name", 0, 1, f_buffer_last_name),
    Subr::fixed("buffer-file-name", 0, 1, f_buffer_file_name),
    Subr::fixed("buffer-base-buffer", 0, 1, f_buffer_base_buffer),
    Subr::fixed("buffer-local-value", 2, 2, f_buffer_local_value),
    Subr::fixed("buffer-local-variables", 0, 1, f_buffer_local_variables),
    Subr::fixed("buffer-modified-p", 0, 1, f_buffer_modified_p),
    Subr::fixed("set-buffer-modified-p", 1, 1, f_set_buffer_modified_p),
    Subr::fixed(
        "restore-buffer-modified-p",
        1,
        1,
        f_restore_buffer_modified_p,
    ),
    Subr::fixed("buffer-modified-tick", 0, 1, f_buffer_modified_tick),
    Subr::fixed(
        "internal--set-buffer-modified-tick",
        1,
        2,
        f_set_buffer_modified_tick,
    ),
    Subr::fixed(
        "buffer-chars-modified-tick",
        0,
        1,
        f_buffer_chars_modified_tick,
    ),
    Subr::fixed("rename-buffer", 1, 2, f_rename_buffer).interactive(
        "(list (read-string (format-prompt \"Rename buffer\" nil)) current-prefix-arg)",
    ),
    Subr::fixed("other-buffer", 0, 3, f_other_buffer),
    Subr::fixed("buffer-enable-undo", 0, 1, f_buffer_enable_undo),
    Subr::fixed("kill-buffer", 0, 1, f_kill_buffer).interactive("bKill buffer: "),
    Subr::fixed("bury-buffer-internal", 1, 1, f_bury_buffer_internal),
    Subr::fixed("set-buffer-major-mode", 1, 1, f_set_buffer_major_mode),
    Subr::fixed("current-buffer", 0, 0, f_current_buffer),
    Subr::fixed("set-buffer", 1, 1, f_set_buffer),
    Subr::fixed("barf-if-buffer-read-only", 0, 1, f_barf_if_buffer_read_only),
    Subr::fixed("erase-buffer", 0, 0, f_erase_buffer).interactive("*"),
    Subr::fixed("set-buffer-multibyte", 1, 1, f_set_buffer_multibyte),
    Subr::fixed("kill-all-local-variables", 0, 1, f_kill_all_local_variables),
    Subr::fixed(
        "internal--set-buffer-modified-tick",
        1,
        2,
        f_set_buffer_modified_tick,
    ),
];

pub struct PerBufferVar {
    pub symbol: Value,
    pub default: fn(&mut Lisp) -> Value,
    pub predicate: Value,
    pub always_local: bool,
    pub permanent: bool,
}

fn nil(_: &mut Lisp) -> Value {
    Value::NIL
}

fn t(_: &mut Lisp) -> Value {
    sym::T
}

fn fundamental_mode(_: &mut Lisp) -> Value {
    sym::FUNDAMENTAL_MODE
}

fn mode_name(lisp: &mut Lisp) -> Value {
    lisp.string("Fundamental")
}

fn mode_line_format(lisp: &mut Lisp) -> Value {
    lisp.string("%-")
}

fn eight(_: &mut Lisp) -> Value {
    Value::fixnum(8)
}

fn seventy(_: &mut Lisp) -> Value {
    Value::fixnum(70)
}

fn zero(_: &mut Lisp) -> Value {
    Value::fixnum(0)
}

macro_rules! per_buffer {
    ($sym:expr, $default:expr, $pred:expr) => {
        (
            $sym,
            $default as fn(&mut Lisp) -> Value,
            $pred,
            false,
            false,
        )
    };
    ($sym:expr, $default:expr, $pred:expr, always) => {
        ($sym, $default as fn(&mut Lisp) -> Value, $pred, true, false)
    };
    ($sym:expr, $default:expr, $pred:expr, permanent) => {
        ($sym, $default as fn(&mut Lisp) -> Value, $pred, false, true)
    };
}

type PerBufferSpec = (Value, fn(&mut Lisp) -> Value, Value, bool, bool);

pub const PER_BUFFER_VARS: &[PerBufferSpec] = &[
    per_buffer!(sym::TAB_LINE_FORMAT, nil, Value::NIL),
    per_buffer!(sym::HEADER_LINE_FORMAT, nil, Value::NIL),
    per_buffer!(sym::MODE_LINE_FORMAT, mode_line_format, Value::NIL),
    per_buffer!(sym::MAJOR_MODE, fundamental_mode, sym::SYMBOLP, always),
    per_buffer!(sym::LOCAL_MINOR_MODES, nil, Value::NIL, always),
    per_buffer!(sym::MODE_NAME, mode_name, Value::NIL, always),
    per_buffer!(sym::LOCAL_ABBREV_TABLE, nil, Value::NIL),
    per_buffer!(sym::ABBREV_MODE, nil, Value::NIL),
    per_buffer!(sym::FILL_COLUMN, seventy, sym::INTEGERP),
    per_buffer!(sym::LEFT_MARGIN, zero, sym::INTEGERP),
    per_buffer!(sym::TAB_WIDTH, eight, sym::INTEGERP),
    per_buffer!(sym::CTL_ARROW, t, Value::NIL),
    per_buffer!(sym::ENABLE_MULTIBYTE_CHARACTERS, t, Value::NIL, always),
    per_buffer!(sym::BUFFER_FILE_CODING_SYSTEM, nil, Value::NIL, permanent),
    per_buffer!(sym::BIDI_DISPLAY_REORDERING, t, Value::NIL),
    per_buffer!(sym::BIDI_PARAGRAPH_START_RE, nil, Value::NIL),
    per_buffer!(sym::BIDI_PARAGRAPH_SEPARATE_RE, nil, Value::NIL),
    per_buffer!(sym::BIDI_PARAGRAPH_DIRECTION, nil, Value::NIL),
    per_buffer!(sym::TRUNCATE_LINES, nil, Value::NIL, permanent),
    per_buffer!(sym::WORD_WRAP, nil, Value::NIL),
    per_buffer!(sym::DEFAULT_DIRECTORY, nil, sym::STRINGP, always),
    per_buffer!(sym::AUTO_FILL_FUNCTION, nil, Value::NIL),
    per_buffer!(sym::BUFFER_FILE_NAME, nil, sym::STRINGP, always),
    per_buffer!(sym::BUFFER_FILE_TRUENAME, nil, sym::STRINGP, always),
    per_buffer!(sym::BUFFER_AUTO_SAVE_FILE_NAME, nil, sym::STRINGP, always),
    per_buffer!(sym::BUFFER_READ_ONLY, nil, Value::NIL, always),
    per_buffer!(sym::BUFFER_BACKED_UP, nil, Value::NIL, always),
    per_buffer!(sym::BUFFER_SAVED_SIZE, zero, sym::INTEGERP, always),
    per_buffer!(sym::SELECTIVE_DISPLAY, nil, Value::NIL),
    per_buffer!(sym::SELECTIVE_DISPLAY_ELLIPSES, t, Value::NIL),
    per_buffer!(sym::OVERWRITE_MODE, nil, sym::OVERWRITE_MODE),
    per_buffer!(sym::BUFFER_DISPLAY_TABLE, nil, Value::NIL),
    per_buffer!(sym::LEFT_MARGIN_WIDTH, zero, sym::INTEGERP),
    per_buffer!(sym::RIGHT_MARGIN_WIDTH, zero, sym::INTEGERP),
    per_buffer!(sym::LEFT_FRINGE_WIDTH, nil, sym::INTEGERP),
    per_buffer!(sym::RIGHT_FRINGE_WIDTH, nil, sym::INTEGERP),
    per_buffer!(sym::FRINGES_OUTSIDE_MARGINS, nil, Value::NIL),
    per_buffer!(sym::SCROLL_BAR_WIDTH, nil, sym::INTEGERP),
    per_buffer!(sym::SCROLL_BAR_HEIGHT, nil, sym::INTEGERP),
    per_buffer!(sym::VERTICAL_SCROLL_BAR, t, sym::VERTICAL_SCROLL_BAR),
    per_buffer!(sym::HORIZONTAL_SCROLL_BAR, t, Value::NIL),
    per_buffer!(sym::INDICATE_EMPTY_LINES, nil, Value::NIL),
    per_buffer!(sym::INDICATE_BUFFER_BOUNDARIES, nil, Value::NIL),
    per_buffer!(sym::FRINGE_INDICATOR_ALIST, nil, Value::NIL),
    per_buffer!(sym::FRINGE_CURSOR_ALIST, nil, Value::NIL),
    per_buffer!(sym::SCROLL_UP_AGGRESSIVELY, nil, sym::FRACTION),
    per_buffer!(sym::SCROLL_DOWN_AGGRESSIVELY, nil, sym::FRACTION),
    per_buffer!(sym::BUFFER_UNDO_LIST, nil, Value::NIL, always),
    per_buffer!(sym::MARK_ACTIVE, nil, Value::NIL, always),
    per_buffer!(sym::CACHE_LONG_SCANS, t, Value::NIL),
    per_buffer!(sym::POINT_BEFORE_SCROLL, nil, Value::NIL, always),
    per_buffer!(sym::BUFFER_FILE_FORMAT, nil, Value::NIL, always),
    per_buffer!(sym::BUFFER_AUTO_SAVE_FILE_FORMAT, t, Value::NIL, always),
    per_buffer!(sym::BUFFER_INVISIBILITY_SPEC, t, Value::NIL, always),
    per_buffer!(sym::BUFFER_DISPLAY_COUNT, zero, sym::INTEGERP, always),
    per_buffer!(sym::BUFFER_DISPLAY_TIME, nil, Value::NIL, always),
    per_buffer!(sym::CURSOR_TYPE, t, Value::NIL),
    per_buffer!(sym::LINE_SPACING, nil, Value::NIL),
    per_buffer!(sym::CURSOR_IN_NON_SELECTED_WINDOWS, t, Value::NIL),
    per_buffer!(sym::TEXT_CONVERSION_STYLE, nil, Value::NIL),
];

pub fn emacs_wd() -> std::io::Result<Vec<u8>> {
    let dir = std::env::current_dir()?;
    let bytes = crate::emacs::os_bytes(dir.as_os_str());
    #[cfg(unix)]
    if bytes.first() == Some(&b'(') {
        return Err(std::io::Error::from_raw_os_error(2));
    }
    Ok(bytes)
}

fn is_directory_sep(c: u8) -> bool {
    c == b'/' || (cfg!(windows) && c == b'\\')
}

pub fn init_buffer(lisp: &mut Lisp) {
    use std::io::Write;
    let scratch = lisp.string("*scratch*");
    let scratch = lisp
        .get_buffer_create(scratch, false)
        .expect("scratch buffer");
    f_set_buffer(lisp, &[scratch]).expect("select scratch buffer");
    if lisp
        .default_value_unchecked(sym::ENABLE_MULTIBYTE_CHARACTERS)
        .is_nil()
    {
        f_set_buffer_multibyte(lisp, &[Value::NIL]).expect("unibyte scratch buffer");
    }
    let current = lisp.current_buffer;
    match emacs_wd() {
        Err(e) => {
            let _ = writeln!(lisp.stderr, "Error getting directory: {e}");
            lisp.buffer_local_set(current, sym::DEFAULT_DIRECTORY, Value::NIL);
        }
        Ok(mut pwd) => {
            if !pwd.last().is_some_and(|&c| is_directory_sep(c)) {
                pwd.push(b'/');
            }
            let dirname = lisp.make_string_from_bytes(pwd, false);
            lisp.buffer_local_set(current, sym::DEFAULT_DIRECTORY, dirname);
            let temp = crate::fileio::f_find_file_name_handler(lisp, &[dirname, sym::T])
                .expect("find-file-name-handler");
            if !temp.is_nil() && lisp.string_bytes(dirname) != b"/" {
                let slash_colon = lisp.string("/:");
                let dir = lisp
                    .concat_to_string(&[slash_colon, dirname])
                    .expect("concat directory");
                lisp.buffer_local_set(current, sym::DEFAULT_DIRECTORY, dir);
            }
        }
    }
    let temp = lisp.get_minibuffer(0).expect("minibuffer");
    let dir = lisp
        .buffer_local_get(current, sym::DEFAULT_DIRECTORY)
        .unwrap_or(Value::NIL);
    lisp.buffer_local_set(temp, sym::DEFAULT_DIRECTORY, dir);
}

pub fn init_vars(lisp: &mut Lisp) {
    for &(symbol, default, predicate, always_local, permanent) in PER_BUFFER_VARS {
        let value = default(lisp);
        let s = lisp.heap.symbol_mut(symbol);
        s.special = true;
        s.value = SymbolValue::Localized(Box::new(Localized {
            local_if_set: true,
            default: value,
            per_buffer: Some(PerBuffer {
                always_local,
                permanent,
                predicate,
            }),
            fwd: None,
        }));
    }
    for symbol in [
        sym::TRUNCATE_LINES,
        sym::BUFFER_FILE_CODING_SYSTEM,
        sym::KILL_BUFFER_HOOK,
    ] {
        lisp.put(symbol, sym::PERMANENT_LOCAL, sym::T);
    }
    let choice = lisp.list(&[Value::NIL, sym::T, sym::LEFT, sym::RIGHT]);
    lisp.put(sym::VERTICAL_SCROLL_BAR, sym::CHOICE, choice);
    let (min, max) = (lisp.float(0.0), lisp.float(1.0));
    let range = lisp.cons(min, max);
    lisp.put(sym::FRACTION, sym::RANGE, range);
    let textual = lisp.intern("overwrite-mode-textual");
    let choice = lisp.list(&[Value::NIL, textual, sym::OVERWRITE_MODE_BINARY]);
    lisp.put(sym::OVERWRITE_MODE, sym::CHOICE, choice);
    lisp.defvar(sym::BEFORE_CHANGE_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::AFTER_CHANGE_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::FIRST_CHANGE_HOOK, Value::NIL);
    lisp.defvar(sym::KILL_BUFFER_HOOK, Value::NIL);
    lisp.defvar(sym::KILL_BUFFER_QUERY_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::BUFFER_LIST_UPDATE_HOOK, Value::NIL);
    lisp.defvar(sym::CHANGE_MAJOR_MODE_HOOK, Value::NIL);
    lisp.defvar(sym::INHIBIT_READ_ONLY, Value::NIL);
    lisp.defvar_bool(sym::INHIBIT_MODIFICATION_HOOKS, false);
    lisp.defvar(sym::TRANSIENT_MARK_MODE, Value::NIL);
    lisp.defvar(sym::CASE_FOLD_SEARCH, sym::T);
    lisp.make_variable_buffer_local_builtin(sym::CASE_FOLD_SEARCH);
    lisp.defvar_bool(sym::KILL_BUFFER_DELETE_AUTO_SAVE_FILES, false);
    lisp.defvar(sym::DELETE_AUTO_SAVE_FILES, sym::T);
    lisp.defvar_int(sym::LARGE_HSCROLL_THRESHOLD, 10000);
    lisp.defvar(sym::LONG_LINE_THRESHOLD, Value::fixnum(50000));
    lisp.defvar(sym::UNDO_OUTER_LIMIT, Value::fixnum(24_000_000));
    lisp.defvar_int(sym::UNDO_LIMIT, 160_000);
    lisp.defvar_int(sym::UNDO_STRONG_LIMIT, 240_000);
    lisp.defvar(sym::UNDO_INHIBIT_RECORD_POINT, Value::NIL);
}

impl Lisp {
    pub fn make_variable_buffer_local_builtin(&mut self, symbol: Value) {
        let value = self.plain_value(symbol);
        let fwd = match self.heap.symbol_ref(symbol).value {
            SymbolValue::Forwarded(fwd) => Some(fwd),
            _ => None,
        };
        self.heap.symbol_mut(symbol).value = SymbolValue::Localized(Box::new(Localized {
            local_if_set: true,
            default: value,
            per_buffer: None,
            fwd,
        }));
    }

    pub fn buffer(&self, v: Value) -> &Buffer {
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Buffer(b) => b,
            _ => panic!("not a buffer"),
        }
    }

    pub fn buffer_mut(&mut self, v: Value) -> &mut Buffer {
        match self.heap.vectorlike_mut(v) {
            Vectorlike::Buffer(b) => b,
            _ => panic!("not a buffer"),
        }
    }

    pub fn cur(&self) -> &Buffer {
        self.buffer(self.current_buffer)
    }

    pub fn cur_mut(&mut self) -> &mut Buffer {
        let b = self.current_buffer;
        self.buffer_mut(b)
    }

    pub fn current_buffer_value(&self) -> Value {
        self.current_buffer
    }

    pub fn is_buffer(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Buffer(_))
    }

    pub fn is_marker(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Marker(_))
    }

    pub fn is_overlay(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Overlay(_))
    }

    pub fn buffer_live_p(&self, buffer: Value) -> bool {
        self.is_buffer(buffer) && !self.buffer(buffer).name.is_nil()
    }

    pub fn check_buffer(&mut self, v: Value) -> LispResult<Value> {
        if self.is_buffer(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::BUFFERP, v))
        }
    }

    pub(crate) fn nsberror(&mut self, spec: Value) -> crate::error::Flow {
        if spec.is_string() {
            let name = self.string_to_rust(spec);
            return self.error(&format!("No buffer named {name}"));
        }
        self.error("Invalid buffer argument")
    }

    pub fn decode_buffer(&mut self, v: Value) -> LispResult<Value> {
        if v.is_nil() {
            Ok(self.current_buffer)
        } else {
            self.check_buffer(v)
        }
    }

    pub fn buffer_local_get(&self, buffer: Value, symbol: Value) -> Option<Value> {
        if buffer.is_nil() || !self.is_buffer(buffer) {
            return None;
        }
        self.buffer(buffer)
            .local_vars
            .iter()
            .find(|(k, _)| *k == symbol)
            .map(|(_, v)| *v)
    }

    pub fn buffer_local_set(&mut self, buffer: Value, symbol: Value, value: Value) {
        if buffer.is_nil() || !self.is_buffer(buffer) {
            return;
        }
        let b = self.buffer_mut(buffer);
        match b.local_vars.iter_mut().find(|(k, _)| *k == symbol) {
            Some(entry) => entry.1 = value,
            None => b.local_vars.push((symbol, value)),
        }
    }

    pub fn buffer_local_remove(&mut self, buffer: Value, symbol: Value) {
        if buffer.is_nil() || !self.is_buffer(buffer) {
            return;
        }
        self.buffer_mut(buffer)
            .local_vars
            .retain(|(k, _)| *k != symbol);
    }

    pub fn per_buffer_info(&self, symbol: Value) -> Option<PerBuffer> {
        match &self.heap.symbol_ref(symbol).value {
            SymbolValue::Localized(l) => l.per_buffer,
            _ => None,
        }
    }

    pub fn local_binding_exists(&self, symbol: Value) -> bool {
        let buffer = self.current_buffer;
        self.buffer_local_get(buffer, symbol).is_some()
    }

    pub fn local_variable_in(&self, symbol: Value, buffer: Value) -> bool {
        self.buffer_local_get(buffer, symbol).is_some()
    }

    pub fn noninteractive(&self) -> bool {
        self.noninteractive
    }

    pub fn echo_area_append(&mut self, text: &str) -> LispResult<()> {
        use std::io::Write;
        let _ = self.stdout.write_all(text.as_bytes());
        Ok(())
    }

    pub fn make_buffer(&mut self, name: Value, inhibit_hooks: bool) -> Value {
        let directory = if self.current_buffer.is_nil() {
            Value::NIL
        } else {
            self.buffer_local_get(self.current_buffer, sym::DEFAULT_DIRECTORY)
                .unwrap_or(Value::NIL)
        };
        let buffer = Buffer {
            name,
            last_name: name,
            text: BufferText::default(),
            pt: 1,
            pt_byte: 1,
            begv: 1,
            begv_byte: 1,
            zv: 1,
            zv_byte: 1,
            local_vars: Vec::new(),
            overlays: Vec::new(),
            mark: Value::NIL,
            keymap: Value::NIL,
            syntax_table: self.plain_value(sym::STANDARD_SYNTAX_TABLE_INTERNAL),
            category_table: self.plain_value(sym::STANDARD_CATEGORY_TABLE_INTERNAL),
            downcase_table: self.ascii_downcase_table,
            upcase_table: self.ascii_upcase_table,
            case_canon_table: self.ascii_canon_table,
            case_eqv_table: self.ascii_eqv_table,
            base_buffer: Value::NIL,
            inhibit_hooks,
            prevent_redisplay_optimizations_p: false,
            long_line_optimizations_p: false,
            last_window_start: 1,
            window_count: 0,
            point_marker: Value::NIL,
            modtime: (0, -2),
            modtime_size: -1,
            auto_save_failure_time: 0,
            last_selected_window: Value::NIL,
        };
        let value = self.heap.vectorlike(Vectorlike::Buffer(Box::new(buffer)));
        self.stack.push(value);
        for &(symbol, default, _, always_local, _) in PER_BUFFER_VARS {
            if always_local {
                let v = if symbol == sym::DEFAULT_DIRECTORY {
                    directory
                } else if symbol == sym::MAJOR_MODE || symbol == sym::MODE_NAME {
                    self.default_value_unchecked(symbol)
                } else {
                    default(self)
                };
                self.buffer_mut(value).local_vars.push((symbol, v));
            }
        }
        let multibyte = !self
            .default_value_unchecked(sym::ENABLE_MULTIBYTE_CHARACTERS)
            .is_nil();
        self.buffer_mut(value).text.multibyte = multibyte;
        self.buffer_local_set(
            value,
            sym::ENABLE_MULTIBYTE_CHARACTERS,
            self.bool(multibyte),
        );
        let mark = self.make_marker_object();
        self.buffer_mut(value).mark = mark;
        let name_str = self.string_bytes(name).first().copied();
        if name_str == Some(b' ') {
            self.buffer_local_set(value, sym::BUFFER_UNDO_LIST, sym::T);
        }
        self.stack.pop();
        value
    }

    pub fn get_buffer(&self, name: Value) -> Value {
        for &b in &self.buffers {
            let bname = self.buffer(b).name;
            if !bname.is_nil() && self.string_bytes(bname) == self.string_bytes(name) {
                return b;
            }
        }
        Value::NIL
    }

    pub fn get_buffer_create(&mut self, name: Value, inhibit_hooks: bool) -> LispResult<Value> {
        let existing = self.get_buffer(name);
        if !existing.is_nil() {
            return Ok(existing);
        }
        if self.string_chars(name) == 0 {
            return Err(self.error("Empty string for buffer name is not allowed"));
        }
        let s = self.heap.string_ref(name).clone();
        let mut copy = s;
        copy.props.clear();
        let name = self.heap.string(copy);
        let buffer = self.make_buffer(name, inhibit_hooks);
        self.buffers.push(buffer);
        if !inhibit_hooks {
            self.stack.push(buffer);
            let r = self.run_hook(sym::BUFFER_LIST_UPDATE_HOOK);
            self.stack.pop();
            r?;
        }
        Ok(buffer)
    }

    pub fn set_buffer_internal(&mut self, buffer: Value) {
        if buffer == self.current_buffer {
            return;
        }
        self.current_buffer = buffer;
        self.indent.last_known_column_point = -1;
    }

    pub fn set_buffer_checked(&mut self, buffer: Value) -> LispResult<()> {
        let buffer = self.check_buffer(buffer)?;
        if !self.buffer_live_p(buffer) {
            return Err(self.error("Selecting deleted buffer"));
        }
        self.set_buffer_internal(buffer);
        Ok(())
    }

    pub fn record_unwind_current_buffer(&mut self) {
        let b = self.current_buffer;
        self.record_unwind(restore_current_buffer, b);
    }

    pub fn generate_new_buffer_name(&mut self, name: Value, ignore: Value) -> LispResult<Value> {
        let name = self.check_string(name)?;
        if (!ignore.is_nil() && !crate::fns::f_string_equal(self, &[name, ignore])?.is_nil())
            || self.get_buffer(name).is_nil()
        {
            return Ok(name);
        }
        let base = self.stack.len();
        let genbase = if self.string_bytes(name).first() != Some(&b' ') {
            name
        } else {
            let i = self.get_random() % 1_000_000;
            let lnumber = self.string(&format!("-{i}"));
            let genbase = crate::fns::f_concat(self, &[name, lnumber])?;
            if self.get_buffer(genbase).is_nil() {
                return Ok(genbase);
            }
            genbase
        };
        self.stack.push(genbase);
        let mut count = 2;
        loop {
            let lnumber = self.string(&format!("<{count}>"));
            let gentemp = crate::fns::f_concat(self, &[genbase, lnumber])?;
            if !crate::fns::f_string_equal(self, &[gentemp, ignore])?.is_nil()
                || self.get_buffer(gentemp).is_nil()
            {
                self.stack.truncate(base);
                return Ok(gentemp);
            }
            count += 1;
        }
    }

    pub fn buffer_hidden_p(&self, buffer: Value) -> bool {
        let name = self.buffer(buffer).name;
        self.is_string(name) && self.string_bytes(name).first() == Some(&b' ')
    }

    fn candidate_buffer(&self, b: Value, buffer: Value) -> bool {
        self.is_buffer(b) && b != buffer && self.buffer_live_p(b) && !self.buffer_hidden_p(b)
    }

    pub fn other_buffer_safely(&mut self, buffer: Value) -> Value {
        let live = self.buffers.clone();
        for b in live {
            if self.candidate_buffer(b, buffer) {
                return b;
            }
        }
        let buf = self
            .safe_calln(sym::GET_SCRATCH_BUFFER_CREATE, &[])
            .unwrap_or(Value::NIL);
        if !buf.is_nil() {
            return buf;
        }
        let scratch = self.string("*scratch*");
        let buf = self
            .get_buffer_create(scratch, false)
            .expect("scratch buffer");
        let _ = f_set_buffer_major_mode(self, &[buf]);
        buf
    }

    pub fn reset_buffer(&mut self, b: Value) {
        let directory = if self.current_buffer.is_nil() {
            Value::NIL
        } else {
            self.buffer_local_get(self.current_buffer, sym::DEFAULT_DIRECTORY)
                .unwrap_or(Value::NIL)
        };
        let multibyte = self.default_value_unchecked(sym::ENABLE_MULTIBYTE_CHARACTERS);
        let cursor_type = self.default_value_unchecked(sym::CURSOR_TYPE);
        let line_spacing = self.default_value_unchecked(sym::LINE_SPACING);
        for (symbol, value) in [
            (sym::BUFFER_FILE_NAME, Value::NIL),
            (sym::BUFFER_FILE_TRUENAME, Value::NIL),
            (sym::DEFAULT_DIRECTORY, directory),
            (sym::BUFFER_SAVED_SIZE, Value::fixnum(0)),
            (sym::BUFFER_BACKED_UP, Value::NIL),
            (sym::LOCAL_MINOR_MODES, Value::NIL),
            (sym::BUFFER_AUTO_SAVE_FILE_NAME, Value::NIL),
            (sym::BUFFER_READ_ONLY, Value::NIL),
            (sym::MARK_ACTIVE, Value::NIL),
            (sym::POINT_BEFORE_SCROLL, Value::NIL),
            (sym::BUFFER_FILE_FORMAT, Value::NIL),
            (sym::BUFFER_AUTO_SAVE_FILE_FORMAT, sym::T),
            (sym::BUFFER_DISPLAY_COUNT, Value::fixnum(0)),
            (sym::BUFFER_DISPLAY_TIME, Value::NIL),
            (sym::ENABLE_MULTIBYTE_CHARACTERS, multibyte),
            (sym::CURSOR_TYPE, cursor_type),
            (sym::LINE_SPACING, line_spacing),
        ] {
            self.buffer_local_set(b, symbol, value);
        }
        let buf = self.buffer_mut(b);
        buf.modtime = (0, -2);
        buf.modtime_size = -1;
        buf.last_window_start = 1;
        buf.text.autosave_modiff = 0;
        buf.auto_save_failure_time = 0;
        buf.overlays.clear();
        buf.last_selected_window = Value::NIL;
    }

    pub fn record_buffer(&mut self, buffer: Value) -> LispResult<()> {
        let buffer = self.check_buffer(buffer)?;
        if let Some(i) = self.buffers.iter().position(|&b| b == buffer) {
            self.buffers.remove(i);
            self.buffers.insert(0, buffer);
        }
        let f = self.frames.selected_frame;
        let list = self.frame(f).buffer_list;
        let list = crate::fns::f_delq(self, &[buffer, list])?;
        let list = self.cons(buffer, list);
        self.frame_mut(f).buffer_list = list;
        let buried = self.frame(f).buried_buffer_list;
        let buried = crate::fns::f_delq(self, &[buffer, buried])?;
        self.frame_mut(f).buried_buffer_list = buried;
        if !self.buffer(buffer).inhibit_hooks {
            self.run_hook(sym::BUFFER_LIST_UPDATE_HOOK)?;
        }
        Ok(())
    }

    pub fn buffer_read_only_p(&self) -> bool {
        let ro = self.find_symbol_value(sym::BUFFER_READ_ONLY);
        !ro.is_nil() && self.plain_value(sym::INHIBIT_READ_ONLY).is_nil()
    }

    pub fn barf_if_buffer_read_only(&mut self, position: usize) -> LispResult<()> {
        let ro = self.find_symbol_value(sym::BUFFER_READ_ONLY);
        if ro.is_nil() {
            return Ok(());
        }
        let inhibit = self.plain_value(sym::INHIBIT_READ_ONLY);
        if !inhibit.is_nil() {
            return Ok(());
        }
        let prop = self.text_property_at(position, sym::INHIBIT_READ_ONLY);
        if !prop.is_nil() {
            return Ok(());
        }
        let buffer = self.current_buffer;
        Err(self.signal_list(sym::BUFFER_READ_ONLY, &[buffer]))
    }

    pub fn buffer_read_char(&mut self, source: Value) -> i64 {
        let (buffer, marker) = if self.is_marker(source) {
            (self.marker(source).buffer, Some(source))
        } else {
            (source, None)
        };
        if !self.buffer_live_p(buffer) {
            return -1;
        }
        let (pos, limit) = match marker {
            Some(m) => (self.marker(m).charpos, self.buffer(buffer).zv),
            None => (self.buffer(buffer).pt, self.buffer(buffer).zv),
        };
        if pos >= limit {
            return -1;
        }
        let b = self.buffer(buffer);
        let byte = b.text.char_to_byte(pos);
        let c = b.text.char_at_byte(byte);
        let len = b.text.char_len_at_byte(byte);
        match marker {
            Some(m) => {
                if let Vectorlike::Marker(mk) = self.heap.vectorlike_mut(m) {
                    mk.charpos = pos + 1;
                    mk.bytepos = byte + len;
                }
            }
            None => {
                let b = self.buffer_mut(buffer);
                b.pt = pos + 1;
                b.pt_byte = byte + len;
            }
        }
        let multibyte = self.buffer(buffer).text.multibyte;
        if !multibyte && c >= 0x80 {
            return character::byte8_to_char(c as u8) as i64;
        }
        c as i64
    }

    pub fn buffer_unread_char(&mut self, source: Value) {
        if self.is_marker(source) {
            let buffer = self.marker(source).buffer;
            if !self.buffer_live_p(buffer) {
                return;
            }
            let pos = self.marker(source).charpos;
            if pos > 1 {
                let byte = self.buffer(buffer).text.char_to_byte(pos - 1);
                if let Vectorlike::Marker(mk) = self.heap.vectorlike_mut(source) {
                    mk.charpos = pos - 1;
                    mk.bytepos = byte;
                }
            }
            return;
        }
        if !self.buffer_live_p(source) {
            return;
        }
        let pos = self.buffer(source).pt;
        if pos > self.buffer(source).begv {
            let byte = self.buffer(source).text.char_to_byte(pos - 1);
            let b = self.buffer_mut(source);
            b.pt = pos - 1;
            b.pt_byte = byte;
        }
    }

    pub fn buffer_line_column(&self, source: Value) -> (i64, i64) {
        let (buffer, pos) = if self.is_marker(source) {
            let m = self.marker(source);
            (m.buffer, m.charpos)
        } else {
            (source, self.buffer(source).pt)
        };
        if !self.buffer_live_p(buffer) {
            return (1, 0);
        }
        let b = self.buffer(buffer);
        let mut line = 1;
        let mut column = 0;
        let mut p = b.begv;
        while p < pos {
            let c = b.text.char_at_byte(b.text.char_to_byte(p));
            if c == 10 {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
            p += 1;
        }
        (line, column)
    }

    pub fn buffer_source(&mut self, stream: Value) -> LispResult<Source> {
        let buffer = if self.is_marker(stream) {
            self.marker(stream).buffer
        } else {
            stream
        };
        if !self.buffer_live_p(buffer) {
            return Err(self.error("Reading from killed buffer"));
        }
        Ok(Source {
            kind: SourceKind::Buffer(stream),
            pos: 0,
            chars_read: 0,
            unread: Vec::new(),
            from_file: false,
        })
    }

    pub fn buffer_source_finish(&mut self, _stream: Value, _source: &Source) {}

    pub fn insert_chars_into(&mut self, dest: Value, chars: &[u32]) -> LispResult<()> {
        let count = self.specpdl.len();
        self.record_unwind_current_buffer();
        let (buffer, marker) = if self.is_marker(dest) {
            (self.marker(dest).buffer, Some(dest))
        } else {
            (dest, None)
        };
        if !self.buffer_live_p(buffer) {
            self.unbind_to(count)?;
            return Err(self.error("Marker does not point anywhere"));
        }
        self.set_buffer_internal(buffer);
        if let Some(m) = marker {
            let pos = self.marker(m).charpos;
            self.goto_char_internal(pos);
        }
        let r = self.insert_char_codes(chars, false, false);
        if let (Some(m), Ok(())) = (marker, &r) {
            let pt = self.cur().pt;
            self.set_marker_position(m, pt, buffer);
        }
        let done = self.unbind_to(count);
        r?;
        done
    }

    pub fn buffer_at_line_start(&self, buffer: Value) -> bool {
        let b = self.buffer(buffer);
        if b.pt <= b.begv {
            return true;
        }
        let byte = b.text.char_to_byte(b.pt - 1);
        b.text.char_at_byte(byte) == 10
    }

    pub fn buffer_name_string(&self, buffer: Value) -> String {
        let name = self.buffer(buffer).name;
        if name.is_nil() {
            String::new()
        } else {
            self.string_to_rust(name)
        }
    }

    pub fn kill_buffer_internal(&mut self, buffer: Value) -> LispResult<bool> {
        if !self.buffer_live_p(buffer) {
            return Ok(false);
        }
        let count = self.specpdl.len();
        let inhibit = self.buffer(buffer).inhibit_hooks;
        if !inhibit {
            self.record_unwind_current_buffer();
            self.set_buffer_internal(buffer);
            let query = self.run_hook_with_args(&[sym::KILL_BUFFER_QUERY_FUNCTIONS], |l, a| {
                let v = l.funcall(a[0], &a[1..])?;
                Ok(if v.is_nil() { sym::T } else { Value::NIL })
            })?;
            if !query.is_nil() {
                self.unbind_to(count)?;
                return Ok(false);
            }
            self.run_hook(sym::KILL_BUFFER_HOOK)?;
            self.unbind_to(count)?;
        }
        if !self.buffer_live_p(buffer) {
            return Ok(true);
        }
        let display_count = self
            .buffer_local_get(buffer, sym::BUFFER_DISPLAY_COUNT)
            .and_then(|v| v.as_fixnum())
            .unwrap_or(0);
        if display_count > 0 {
            self.window_discard_buffer_from_dead_windows(buffer)?;
        }
        if !self.buffer_live_p(buffer) {
            return Ok(true);
        }
        if buffer == self.current_buffer {
            let other = self
                .buffers
                .iter()
                .copied()
                .find(|&b| {
                    b != buffer
                        && self.buffer_live_p(b)
                        && !self.buffer_name_string(b).starts_with(' ')
                })
                .or_else(|| {
                    self.buffers
                        .iter()
                        .copied()
                        .find(|&b| b != buffer && self.buffer_live_p(b))
                });
            match other {
                Some(o) => self.set_buffer_internal(o),
                None => {
                    let name = self.string("*scratch*");
                    let scratch = self.get_buffer_create(name, false)?;
                    self.set_buffer_internal(scratch);
                }
            }
        }
        self.unlock_buffer(buffer)?;
        let markers = std::mem::take(&mut self.buffer_mut(buffer).text.markers);
        for m in markers {
            if let Vectorlike::Marker(mk) = self.heap.vectorlike_mut(m) {
                mk.buffer = Value::NIL;
            }
        }
        let overlays = std::mem::take(&mut self.buffer_mut(buffer).overlays);
        for o in overlays {
            if let Vectorlike::Overlay(ov) = self.heap.vectorlike_mut(o) {
                ov.buffer = Value::NIL;
            }
        }
        self.buffers.retain(|&b| b != buffer);
        {
            let b = self.buffer_mut(buffer);
            b.name = Value::NIL;
            b.local_vars.clear();
            b.text.set_contents(Vec::new(), 0, true);
            b.text.props.clear();
            b.pt = 1;
            b.pt_byte = 1;
            b.begv = 1;
            b.begv_byte = 1;
            b.zv = 1;
            b.zv_byte = 1;
        }
        if !inhibit {
            self.run_hook(sym::BUFFER_LIST_UPDATE_HOOK)?;
        }
        Ok(true)
    }

    pub fn kill_all_local_variables(&mut self, kill_permanent: Value) -> LispResult<()> {
        self.run_hook(sym::CHANGE_MAJOR_MODE_HOOK)?;
        let mut permanent_too = 0;
        if !kill_permanent.is_nil() {
            permanent_too = 2;
            if kill_permanent == sym::PERMANENT_LOCAL {
                permanent_too = 1;
            } else if kill_permanent == sym::RESET {
                permanent_too = 1;
                self.reset_buffer(self.current_buffer);
            }
        }
        let buffer = self.current_buffer;
        self.reset_buffer_local_variables(buffer, permanent_too)
    }

    fn reset_buffer_local_variables(&mut self, buffer: Value, permanent_too: u8) -> LispResult<()> {
        let locals = self.buffer(buffer).local_vars.clone();
        let base = self.stack.len();
        for &(symbol, value) in &locals {
            self.stack.push(symbol);
            self.stack.push(value);
        }
        let mut kept = Vec::with_capacity(locals.len());
        for &(symbol, value) in locals.iter().rev() {
            match self.per_buffer_info(symbol) {
                Some(PerBuffer {
                    always_local: true, ..
                }) => {
                    if symbol == sym::MAJOR_MODE {
                        kept.push((symbol, sym::FUNDAMENTAL_MODE));
                    } else if symbol == sym::MODE_NAME {
                        let name = self.string("Fundamental");
                        self.stack.push(name);
                        kept.push((symbol, name));
                    } else if symbol == sym::BUFFER_INVISIBILITY_SPEC {
                        kept.push((symbol, sym::T));
                    } else {
                        kept.push((symbol, value));
                    }
                }
                Some(PerBuffer { permanent, .. }) => {
                    if permanent_too == 0 && permanent {
                        kept.push((symbol, value));
                    }
                }
                None if permanent_too == 1 => {}
                None => {
                    let trapped =
                        |l: &Lisp| l.heap.symbol_ref(symbol).trapped_write == TrappedWrite::Trapped;
                    if trapped(self) {
                        let current = self.current_buffer;
                        self.notify_variable_watchers(
                            symbol,
                            Value::NIL,
                            sym::MAKUNBOUND,
                            current,
                        )?;
                    }
                    let prop = self.get(symbol, sym::PERMANENT_LOCAL);
                    if prop.is_nil() || permanent_too != 0 {
                        continue;
                    }
                    let value = self.buffer_local_get(buffer, symbol).unwrap_or(value);
                    if prop != sym::PERMANENT_LOCAL_HOOK {
                        kept.push((symbol, value));
                        continue;
                    }
                    let newlist = if value.is_cons() {
                        let mut new = Vec::new();
                        let mut tail = value;
                        while tail.is_cons() {
                            let elt = self.xcar(tail);
                            if elt == sym::T
                                || (elt.is_symbol()
                                    && !self.get(elt, sym::PERMANENT_LOCAL_HOOK).is_nil())
                            {
                                new.push(elt);
                            }
                            tail = self.xcdr(tail);
                        }
                        self.list(&new)
                    } else {
                        value
                    };
                    self.stack.push(newlist);
                    if trapped(self) {
                        let current = self.current_buffer;
                        self.notify_variable_watchers(symbol, newlist, sym::MAKUNBOUND, current)?;
                    }
                    kept.push((symbol, newlist));
                }
            }
        }
        kept.reverse();
        self.stack.truncate(base);
        let down = self.ascii_downcase_table;
        let extras_ok = self.is_char_table(down)
            && self.char_table(down).extras[..3]
                .iter()
                .all(|&e| self.is_char_table(e));
        if !extras_ok {
            self.set_case_table(down, true)?;
        }
        let (up, canon, eqv) = {
            let e = &self.char_table(down).extras;
            (e[0], e[1], e[2])
        };
        let syntax = self.standard_syntax_table();
        let category = self.standard_category_table();
        let b = self.buffer_mut(buffer);
        b.local_vars = kept;
        b.keymap = Value::NIL;
        b.syntax_table = syntax;
        b.category_table = category;
        b.downcase_table = down;
        b.upcase_table = up;
        b.case_canon_table = canon;
        b.case_eqv_table = eqv;
        Ok(())
    }

    pub fn buffer_list_value(&mut self) -> Value {
        let live: Vec<Value> = self
            .buffers
            .iter()
            .copied()
            .filter(|&b| self.buffer_live_p(b))
            .collect();
        self.list(&live)
    }
}

pub fn restore_current_buffer(lisp: &mut Lisp, buffer: Value) -> LispResult<()> {
    if lisp.buffer_live_p(buffer) {
        lisp.set_buffer_internal(buffer);
    }
    Ok(())
}

fn f_buffer_live_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.buffer_live_p(args[0])))
}

fn f_buffer_list(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.buffer_list_value())
}

fn f_force_mode_line_update(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let all = args.first().copied().unwrap_or(Value::NIL);
    let cb = lisp.current_buffer;
    if !all.is_nil() {
        lisp.windows.update_mode_lines = 10;
        lisp.buffer_mut(cb).prevent_redisplay_optimizations_p = true;
    } else if lisp.buffer(cb).window_count > 0 {
        lisp.bset_update_mode_line(cb);
        lisp.buffer_mut(cb).prevent_redisplay_optimizations_p = true;
    }
    Ok(all)
}

pub(crate) fn f_get_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let name = args[0];
    if lisp.is_buffer(name) {
        return Ok(name);
    }
    lisp.check_string(name)?;
    Ok(lisp.get_buffer(name))
}

fn find_buffer_by_local(lisp: &mut Lisp, variable: Value, value: Value) -> LispResult {
    let buffers = lisp.buffers.clone();
    for b in buffers {
        if !lisp.buffer_live_p(b) {
            continue;
        }
        if let Some(v) = lisp.buffer_local_get(b, variable)
            && lisp.equal(v, value)?
        {
            return Ok(b);
        }
    }
    Ok(Value::NIL)
}

fn f_get_file_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let filename = lisp.check_string(args[0])?;
    let expanded = lisp.expand_file_name(filename, Value::NIL)?;
    find_buffer_by_local(lisp, sym::BUFFER_FILE_NAME, expanded)
}

pub(crate) fn f_get_truename_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    find_buffer_by_local(lisp, sym::BUFFER_FILE_TRUENAME, args[0])
}

fn f_find_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let variable = lisp.check_symbol(args[0])?;
    let buffers = lisp.buffers.clone();
    for b in buffers {
        if !lisp.buffer_live_p(b) {
            continue;
        }
        let v = lisp
            .buffer_local_get(b, variable)
            .unwrap_or_else(|| lisp.default_value_unchecked(variable));
        if lisp.equal(v, args[1])? {
            return Ok(b);
        }
    }
    Ok(Value::NIL)
}

fn f_get_buffer_create(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let name = args[0];
    if lisp.is_buffer(name) {
        return Ok(name);
    }
    lisp.check_string(name)?;
    lisp.get_buffer_create(name, !args[1].is_nil())
}

fn f_generate_new_buffer_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.generate_new_buffer_name(args[0], args[1])
}

fn f_buffer_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.decode_buffer(args[0])?;
    Ok(lisp.buffer(b).name)
}

fn f_buffer_last_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.decode_buffer(args[0])?;
    Ok(lisp.buffer(b).last_name)
}

fn f_buffer_file_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.decode_buffer(args[0])?;
    Ok(lisp
        .buffer_local_get(b, sym::BUFFER_FILE_NAME)
        .unwrap_or(Value::NIL))
}

fn f_buffer_base_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.decode_buffer(args[0])?;
    Ok(lisp.buffer(b).base_buffer)
}

pub(crate) fn f_buffer_local_value(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (variable, buffer) = (args[0], args[1]);
    let variable = lisp.check_symbol(variable)?;
    let buffer = lisp.check_buffer(buffer)?;
    let target = lisp.indirect_variable(variable);
    let value = match lisp.buffer_local_get(buffer, target) {
        Some(v) => v,
        None => lisp.default_value_unchecked(target),
    };
    if value == sym::UNBOUND {
        return Err(lisp.signal_list(sym::VOID_VARIABLE, &[variable]));
    }
    Ok(value)
}

fn f_buffer_local_variables(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.decode_buffer(args[0])?;
    let locals = lisp.buffer(b).local_vars.clone();
    let mut items = Vec::with_capacity(locals.len());
    for (k, v) in locals {
        if v == sym::UNBOUND {
            items.push(k);
        } else {
            items.push(lisp.cons(k, v));
        }
    }
    Ok(lisp.list(&items))
}

pub(crate) fn f_buffer_modified_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.decode_buffer(args[0])?;
    let t = &lisp.buffer(b).text;
    if t.save_modiff < t.modiff {
        if t.autosave_modiff == t.modiff {
            return Ok(sym::AUTOSAVED);
        }
        return Ok(sym::T);
    }
    Ok(Value::NIL)
}

fn f_set_buffer_modified_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    f_restore_buffer_modified_p(lisp, args)
}

fn f_restore_buffer_modified_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let flag = args[0];
    let b = if lisp.cur().base_buffer.is_nil() {
        lisp.current_buffer
    } else {
        lisp.cur().base_buffer
    };
    if !lisp.var_true(sym::INHIBIT_MODIFICATION_HOOKS) {
        let fn_ = lisp
            .buffer_local_get(b, sym::BUFFER_FILE_TRUENAME)
            .unwrap_or(Value::NIL);
        let filename = lisp
            .buffer_local_get(b, sym::BUFFER_FILE_NAME)
            .unwrap_or(Value::NIL);
        if !fn_.is_nil() && !filename.is_nil() {
            let t = &lisp.cur().text;
            let already = t.save_modiff < t.modiff;
            if !already && !flag.is_nil() {
                crate::filelock::f_lock_file(lisp, &[fn_])?;
            } else if already && flag.is_nil() {
                crate::filelock::f_unlock_file(lisp, &[fn_])?;
            }
        }
    }
    let t = &mut lisp.cur_mut().text;
    if flag.is_nil() {
        t.save_modiff = t.modiff;
    } else {
        if t.save_modiff >= t.modiff {
            t.save_modiff = t.modiff;
            t.modiff += 1;
        }
        if flag == sym::AUTOSAVED {
            t.autosave_modiff = t.modiff;
        }
    }
    Ok(flag)
}

fn f_buffer_modified_tick(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.decode_buffer(args[0])?;
    Ok(Value::fixnum(lisp.buffer(b).text.modiff))
}

fn f_set_buffer_modified_tick(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let tick = lisp.check_fixnum(args[0])?;
    let b = lisp.decode_buffer(args[1])?;
    lisp.buffer_mut(b).text.modiff = tick;
    Ok(Value::NIL)
}

fn f_buffer_chars_modified_tick(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = lisp.decode_buffer(args[0])?;
    Ok(Value::fixnum(lisp.buffer(b).text.chars_modiff))
}

fn f_rename_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (newname, unique) = (args[0], args[1]);
    let newname = lisp.check_string(newname)?;
    if lisp.string_chars(newname) == 0 {
        return Err(lisp.error("Empty string is invalid as a buffer name"));
    }
    let current = lisp.current_buffer;
    let existing = lisp.get_buffer(newname);
    let newname = if !existing.is_nil() && existing != current {
        if unique.is_nil() {
            let name = lisp.string_to_rust(newname);
            return Err(lisp.error(&format!("Buffer name `{name}' is in use")));
        }
        let own = lisp.cur().name;
        lisp.generate_new_buffer_name(newname, own)?
    } else {
        newname
    };
    let b = lisp.cur_mut();
    b.last_name = b.name;
    b.name = newname;
    lisp.run_hook(sym::BUFFER_LIST_UPDATE_HOOK)?;
    Ok(newname)
}

fn f_other_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = args[0];
    let buffers = lisp.buffers.clone();
    for b in buffers {
        if b != buffer && lisp.buffer_live_p(b) && !lisp.buffer_name_string(b).starts_with(' ') {
            return Ok(b);
        }
    }
    let name = lisp.string("*scratch*");
    let existing = lisp.get_buffer(name);
    if !existing.is_nil() {
        return Ok(existing);
    }
    lisp.get_buffer_create(name, false)
}

pub(crate) fn f_buffer_enable_undo(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let b = if args[0].is_nil() {
        lisp.current_buffer
    } else {
        let v = f_get_buffer(lisp, &[args[0]])?;
        if v.is_nil() {
            return Err(lisp.signal_list(sym::ERROR, &[args[0]]));
        }
        v
    };
    if lisp.buffer_local_get(b, sym::BUFFER_UNDO_LIST) == Some(sym::T) {
        lisp.buffer_local_set(b, sym::BUFFER_UNDO_LIST, Value::NIL);
    }
    Ok(Value::NIL)
}

fn f_kill_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = if args[0].is_nil() {
        lisp.current_buffer
    } else {
        let b = f_get_buffer(lisp, &[args[0]])?;
        if b.is_nil() {
            let name = lisp.princ_to_string(args[0]);
            return Err(lisp.error(&format!("No such buffer {name}")));
        }
        b
    };
    let killed = lisp.kill_buffer_internal(buffer)?;
    Ok(lisp.bool(killed))
}

fn f_bury_buffer_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = lisp.check_buffer(args[0])?;
    lisp.buffers.retain(|&b| b != buffer);
    lisp.buffers.push(buffer);
    Ok(Value::NIL)
}

fn f_set_buffer_major_mode(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = lisp.check_buffer(args[0])?;
    if lisp.buffer_name_string(buffer).starts_with(' ') {
        return Ok(Value::NIL);
    }
    let function = lisp.default_value_unchecked(sym::MAJOR_MODE);
    let function = if function.is_nil() {
        sym::FUNDAMENTAL_MODE
    } else {
        function
    };
    let count = lisp.specpdl.len();
    lisp.record_unwind_current_buffer();
    lisp.set_buffer_internal(buffer);
    let r = lisp.call0(function);
    lisp.unbind_to_value(count, r)
}

fn f_current_buffer(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.current_buffer)
}

pub(crate) fn f_set_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = f_get_buffer(lisp, &[args[0]])?;
    if buffer.is_nil() {
        return Err(lisp.signal_error("No such buffer", args[0]));
    }
    if !lisp.buffer_live_p(buffer) {
        return Err(lisp.error("Selecting deleted buffer"));
    }
    lisp.set_buffer_internal(buffer);
    Ok(buffer)
}

fn f_barf_if_buffer_read_only(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let pos = if args[0].is_nil() {
        lisp.cur().pt
    } else {
        lisp.check_fixnum(args[0])? as usize
    };
    lisp.barf_if_buffer_read_only(pos)?;
    Ok(Value::NIL)
}

pub(crate) fn f_erase_buffer(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    lisp.widen();
    let (beg, end) = (1, lisp.cur().z());
    lisp.del_range(beg, end)?;
    Ok(Value::NIL)
}

pub(crate) fn f_set_buffer_multibyte(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let flag = !args[0].is_nil();
    let buffer = lisp.current_buffer;
    if lisp.buffer(buffer).text.multibyte == flag {
        return Ok(args[0]);
    }
    let b = lisp.buffer(buffer);
    let bytes = b.text.all_bytes();
    let (data, chars) = if flag {
        let mut res = Vec::with_capacity(bytes.len());
        let mut n = 0;
        let mut i = 0;
        while i < bytes.len() {
            if let Some(len) = character::multibyte_length(&bytes[i..])
                && bytes[i] >= 0xC0
            {
                res.extend_from_slice(&bytes[i..i + len]);
                i += len;
            } else if bytes[i] < 0x80 {
                res.push(bytes[i]);
                i += 1;
            } else {
                character::push_char(&mut res, character::byte8_to_char(bytes[i]));
                i += 1;
            }
            n += 1;
        }
        (res, n)
    } else {
        let unibyte = character::str_as_unibyte(&bytes);
        let n = unibyte.len();
        (unibyte, n)
    };
    let (pt, begv, zv) = {
        let b = lisp.buffer(buffer);
        (b.pt, b.begv, b.zv)
    };
    let old_chars = lisp.buffer(buffer).text.chars;
    let b = lisp.buffer_mut(buffer);
    b.text.set_contents(data, chars, flag);
    let clamp = |p: usize| p.min(chars + 1);
    b.pt = clamp(pt);
    b.begv = clamp(begv);
    b.zv = if zv == old_chars + 1 {
        chars + 1
    } else {
        clamp(zv)
    };
    b.pt_byte = b.text.char_to_byte(b.pt);
    b.begv_byte = b.text.char_to_byte(b.begv);
    b.zv_byte = b.text.char_to_byte(b.zv);
    let markers = b.text.markers.clone();
    for m in markers {
        let pos = lisp.marker(m).charpos.min(chars + 1);
        lisp.set_marker_position(m, pos, buffer);
    }
    let v = lisp.bool(flag);
    lisp.buffer_local_set(buffer, sym::ENABLE_MULTIBYTE_CHARACTERS, v);
    Ok(args[0])
}

pub(crate) fn f_kill_all_local_variables(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.kill_all_local_variables(args[0])?;
    Ok(Value::NIL)
}
