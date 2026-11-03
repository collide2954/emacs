use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub const BIDI_MAXDEPTH: i32 = 125;
pub const IT_STACK_SIZE: usize = 5;
const LEVEL_STACK_SIZE: usize = (BIDI_MAXDEPTH + 2 + 1) as usize;
pub const BIDI_EOB: i32 = -1;
const BIDI_CACHE_CHUNK: i64 = 200;
const BIDI_CACHE_MAX_ELTS_PER_SLOT: i64 = 50000;
const MAX_PARAGRAPH_SEARCH: i64 = 7500;
const MAX_STRONG_CHAR_SEARCH: i64 = 100000;
const MAX_BPA_STACK: i32 = 977;
const FLAG_EMBEDDING_INSIDE: u8 = 1;
const FLAG_OPPOSITE_INSIDE: u8 = 2;
const PARAGRAPH_SEPARATOR: i32 = 0x2029;
const LEFT_POINTING_ANGLE_BRACKET: i32 = 0x2329;
const RIGHT_POINTING_ANGLE_BRACKET: i32 = 0x232A;
const LEFT_ANGLE_BRACKET: i32 = 0x3008;
const RIGHT_ANGLE_BRACKET: i32 = 0x3009;
const OBJECT_REPLACEMENT_CHARACTER: i32 = 0xFFFC;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BidiType {
    #[default]
    UnknownBt,
    StrongL,
    StrongR,
    WeakEn,
    WeakAn,
    WeakBn,
    NeutralB,
    StrongAl,
    Lre,
    Lro,
    Rle,
    Rlo,
    Pdf,
    Lri,
    Rli,
    Fsi,
    Pdi,
    WeakEs,
    WeakEt,
    WeakCs,
    WeakNsm,
    NeutralS,
    NeutralWs,
    NeutralOn,
}

const BIDI_TYPES: [BidiType; 24] = [
    BidiType::UnknownBt,
    BidiType::StrongL,
    BidiType::StrongR,
    BidiType::WeakEn,
    BidiType::WeakAn,
    BidiType::WeakBn,
    BidiType::NeutralB,
    BidiType::StrongAl,
    BidiType::Lre,
    BidiType::Lro,
    BidiType::Rle,
    BidiType::Rlo,
    BidiType::Pdf,
    BidiType::Lri,
    BidiType::Rli,
    BidiType::Fsi,
    BidiType::Pdi,
    BidiType::WeakEs,
    BidiType::WeakEt,
    BidiType::WeakCs,
    BidiType::WeakNsm,
    BidiType::NeutralS,
    BidiType::NeutralWs,
    BidiType::NeutralOn,
];

impl BidiType {
    pub fn from_index(n: i64) -> BidiType {
        match usize::try_from(n).ok().and_then(|i| BIDI_TYPES.get(i)) {
            Some(&t) => t,
            None => std::process::abort(),
        }
    }

    fn bits3(self) -> u8 {
        self as u8 & 7
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BidiBracketType {
    None,
    Open,
    Close,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BidiDir {
    #[default]
    NeutralDir,
    L2R,
    R2L,
}

impl BidiDir {
    fn from_bits(n: u8) -> BidiDir {
        match n {
            0 => BidiDir::NeutralDir,
            1 => BidiDir::L2R,
            _ => BidiDir::R2L,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BidiCategory {
    UnknownBc,
    Neutral,
    Weak,
    Strong,
    ExplicitFormatting,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BidiSavedInfo {
    pub charpos: i64,
    pub type_: BidiType,
    pub orig_type: BidiType,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BidiStack {
    pub next_for_neutral_pos: i64,
    pub next_for_neutral_type: u8,
    pub last_strong_type: u8,
    pub prev_for_neutral_type: u8,
    pub level: u8,
    pub flags: u8,
}

#[derive(Clone, Debug)]
pub struct BidiStringData {
    pub lstring: Value,
    pub s: Option<Rc<[u8]>>,
    pub schars: i64,
    pub bufpos: i64,
    pub from_disp_str: bool,
    pub unibyte: bool,
}

impl Default for BidiStringData {
    fn default() -> Self {
        BidiStringData {
            lstring: Value::NIL,
            s: None,
            schars: 0,
            bufpos: 0,
            from_disp_str: false,
            unibyte: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BidiState {
    pub bytepos: i64,
    pub charpos: i64,
    pub ch: i32,
    pub nchars: i64,
    pub ch_len: i64,
    pub type_: BidiType,
    pub type_after_wn: BidiType,
    pub orig_type: BidiType,
    pub resolved_level: i8,
    pub isolate_level: i8,
    pub invalid_levels: i64,
    pub invalid_isolates: i64,
    pub prev: BidiSavedInfo,
    pub last_strong: BidiSavedInfo,
    pub next_for_neutral: BidiSavedInfo,
    pub prev_for_neutral: BidiSavedInfo,
    pub next_for_ws: BidiSavedInfo,
    pub bracket_pairing_pos: i64,
    pub bracket_enclosed_type: BidiType,
    pub next_en_pos: i64,
    pub next_en_type: BidiType,
    pub sos: BidiDir,
    pub scan_dir: i32,
    pub disp_pos: i64,
    pub disp_prop: i32,
    pub stack_idx: i32,
}

#[derive(Clone, Debug)]
pub struct BidiIt {
    pub state: BidiState,
    pub level_stack: [BidiStack; LEVEL_STACK_SIZE],
    pub string: BidiStringData,
    pub w: Value,
    pub paragraph_dir: BidiDir,
    pub separator_limit: i64,
    pub first_elt: bool,
    pub new_paragraph: bool,
    pub frame_window_p: bool,
}

impl Default for BidiIt {
    fn default() -> Self {
        BidiIt {
            state: BidiState::default(),
            level_stack: [BidiStack::default(); LEVEL_STACK_SIZE],
            string: BidiStringData::default(),
            w: Value::NIL,
            paragraph_dir: BidiDir::NeutralDir,
            separator_limit: 0,
            first_elt: false,
            new_paragraph: false,
            frame_window_p: false,
        }
    }
}

impl Deref for BidiIt {
    type Target = BidiState;

    fn deref(&self) -> &BidiState {
        &self.state
    }
}

impl DerefMut for BidiIt {
    fn deref_mut(&mut self) -> &mut BidiState {
        &mut self.state
    }
}

impl BidiIt {
    fn isolate_status(&self, idx: i32) -> bool {
        self.level_stack[idx as usize].flags & 1 != 0
    }

    fn override_dir(&self, idx: i32) -> BidiDir {
        BidiDir::from_bits((self.level_stack[idx as usize].flags >> 1) & 3)
    }

    fn stack_level(&self, idx: i32) -> i32 {
        i32::from(self.level_stack[idx as usize].level)
    }

    fn current_stack_level(&self) -> i32 {
        self.stack_level(self.stack_idx)
    }

    pub fn at_base_level(&self) -> bool {
        i32::from(self.resolved_level) == self.stack_level(0)
    }

    pub fn string_p(&self) -> bool {
        self.string.s.is_some() || self.string.lstring.is_string()
    }

    pub fn trace(&self, out: &mut Vec<Value>) {
        out.push(self.string.lstring);
        out.push(self.w);
    }
}

fn bidi_copy_it(to: &mut BidiIt, from: &BidiIt) {
    to.state = from.state;
    let n = from.stack_idx as usize;
    to.level_stack[..=n].copy_from_slice(&from.level_stack[..=n]);
}

pub struct BidiShelf {
    idx: i64,
    cache: Vec<BidiIt>,
    start_stack: [i64; IT_STACK_SIZE],
    sp: i32,
    start: i64,
    last_idx: i64,
    max_elts: i64,
}

pub type ShelvedBidiCache = Option<Box<BidiShelf>>;

pub struct Bidi {
    initialized: bool,
    type_table: Value,
    mirror_table: Value,
    brackets_table: Value,
    paragraph_start_re: Value,
    paragraph_separate_re: Value,
    cache_max_elts: i64,
    cache: Vec<BidiIt>,
    cache_idx: i64,
    cache_last_idx: i64,
    cache_start: i64,
    cache_start_stack: [i64; IT_STACK_SIZE],
    cache_sp: i32,
    nsearch_for_strong: i64,
}

impl Default for Bidi {
    fn default() -> Self {
        Bidi {
            initialized: false,
            type_table: Value::NIL,
            mirror_table: Value::NIL,
            brackets_table: Value::NIL,
            paragraph_start_re: Value::NIL,
            paragraph_separate_re: Value::NIL,
            cache_max_elts: BIDI_CACHE_MAX_ELTS_PER_SLOT,
            cache: Vec::new(),
            cache_idx: 0,
            cache_last_idx: 0,
            cache_start: 0,
            cache_start_stack: [0; IT_STACK_SIZE],
            cache_sp: 0,
            nsearch_for_strong: 0,
        }
    }
}

impl Bidi {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.type_table,
            self.mirror_table,
            self.brackets_table,
            self.paragraph_start_re,
            self.paragraph_separate_re,
        ]);
        for entry in &self.cache {
            entry.trace(out);
        }
    }

    fn cache_size(&self) -> i64 {
        self.cache.len() as i64
    }

    fn entry(&self, idx: i64) -> &BidiIt {
        &self.cache[idx as usize]
    }

    fn entry_mut(&mut self, idx: i64) -> &mut BidiIt {
        &mut self.cache[idx as usize]
    }

    fn cache_reset_to(&mut self, n: i64) {
        self.cache_idx = self.cache_start + n;
        self.cache_last_idx = -1;
    }

    fn cache_reset(&mut self) {
        self.cache_reset_to(0);
    }

    fn cache_shrink(&mut self) {
        if self.cache_size() > BIDI_CACHE_CHUNK {
            self.cache.truncate(BIDI_CACHE_CHUNK as usize);
        }
        self.cache_reset();
        self.cache_max_elts = BIDI_CACHE_MAX_ELTS_PER_SLOT;
    }

    fn cache_fetch_state(&mut self, idx: i64, bidi_it: &mut BidiIt) {
        let current_scan_dir = bidi_it.scan_dir;
        if idx < self.cache_start || idx >= self.cache_idx {
            std::process::abort();
        }
        bidi_copy_it(bidi_it, &self.cache[idx as usize]);
        bidi_it.scan_dir = current_scan_dir;
        self.cache_last_idx = idx;
    }

    fn cache_search(&mut self, charpos: i64, level: i32, mut dir: i32) -> i64 {
        if self.cache_idx > self.cache_start {
            if self.cache_last_idx == -1 {
                self.cache_last_idx = self.cache_idx - 1;
            }
            let last = self.entry(self.cache_last_idx);
            let i_start = if charpos < last.charpos {
                dir = -1;
                self.cache_last_idx - 1
            } else if charpos > last.charpos + last.nchars - 1 {
                dir = 1;
                self.cache_last_idx + 1
            } else if dir != 0 {
                self.cache_last_idx
            } else {
                dir = -1;
                self.cache_idx - 1
            };
            let matches = |e: &BidiIt| {
                e.charpos <= charpos
                    && charpos < e.charpos + e.nchars
                    && (level == -1 || i32::from(e.resolved_level) <= level)
            };
            if dir < 0 {
                let mut i = i_start;
                while i >= self.cache_start {
                    if matches(self.entry(i)) {
                        return i;
                    }
                    i -= 1;
                }
            } else {
                let mut i = i_start;
                while i < self.cache_idx {
                    if matches(self.entry(i)) {
                        return i;
                    }
                    i += 1;
                }
            }
        }
        -1
    }

    fn cache_find_level_change(&self, level: i32, mut dir: i32, before: bool) -> i64 {
        if self.cache_idx != 0 {
            let mut i = if dir != 0 {
                self.cache_last_idx
            } else {
                self.cache_idx - 1
            };
            let incr = i64::from(before);
            if i < 0 {
                i = 0;
            }
            if dir == 0 {
                dir = -1;
            } else if incr == 0 {
                i += i64::from(dir);
            }
            let lower = |e: &BidiIt| e.resolved_level >= 0 && i32::from(e.resolved_level) < level;
            if dir < 0 {
                while i >= self.cache_start + incr {
                    if lower(self.entry(i - incr)) {
                        return i;
                    }
                    i -= 1;
                }
            } else {
                while i < self.cache_idx - incr {
                    if lower(self.entry(i + incr)) {
                        return i;
                    }
                    i += 1;
                }
            }
        }
        -1
    }

    fn cache_ensure_space(&mut self, idx: i64) {
        let size = self.cache_size();
        if idx >= size {
            let mut chunk_size = BIDI_CACHE_CHUNK;
            if size > self.cache_max_elts - chunk_size {
                chunk_size = self.cache_max_elts - size;
            }
            if (idx + 1).max(size + chunk_size) <= self.cache_max_elts {
                let incr_min = chunk_size.max(idx - size + 1);
                let mut n = (size + (size >> 1)).min(self.cache_max_elts);
                if n - size < incr_min {
                    n = size + incr_min;
                }
                self.cache.resize(n as usize, BidiIt::default());
            }
        }
    }

    fn cache_iterator_state(
        &mut self,
        bidi_it: &BidiIt,
        resolved: bool,
        update_only: bool,
    ) -> bool {
        if bidi_it.scan_dir == -1 {
            std::process::abort();
        }
        let mut idx = self.cache_search(bidi_it.charpos, -1, 1);
        if idx < 0 && update_only {
            return false;
        }
        if idx < 0 {
            idx = self.cache_idx;
            self.cache_ensure_space(idx);
            if self.cache_start < idx
                && idx < self.cache_size()
                && (bidi_it.charpos > self.entry(idx - 1).charpos + self.entry(idx - 1).nchars
                    || bidi_it.charpos < self.entry(self.cache_start).charpos)
            {
                self.cache_reset();
                idx = self.cache_start;
            }
            if bidi_it.nchars <= 0 {
                std::process::abort();
            }
            if self.cache_size() > idx {
                let entry = self.entry_mut(idx);
                bidi_copy_it(entry, bidi_it);
                if !resolved {
                    entry.resolved_level = -1;
                }
            }
        } else {
            let entry = self.entry_mut(idx);
            entry.type_ = bidi_it.type_;
            entry.type_after_wn = bidi_it.type_after_wn;
            entry.resolved_level = if resolved { bidi_it.resolved_level } else { -1 };
            entry.invalid_levels = bidi_it.invalid_levels;
            entry.next_for_neutral = bidi_it.next_for_neutral;
            entry.next_for_ws = bidi_it.next_for_ws;
            entry.disp_pos = bidi_it.disp_pos;
            entry.disp_prop = bidi_it.disp_prop;
            entry.bracket_pairing_pos = bidi_it.bracket_pairing_pos;
            entry.bracket_enclosed_type = bidi_it.bracket_enclosed_type;
        }
        if self.cache_size() > idx {
            self.cache_last_idx = idx;
            if idx >= self.cache_idx {
                self.cache_idx = idx + 1;
            }
            true
        } else {
            self.cache_last_idx = -1;
            false
        }
    }

    fn cache_find(&mut self, charpos: i64, resolved_only: bool, bidi_it: &mut BidiIt) -> BidiType {
        let i = self.cache_search(charpos, -1, bidi_it.scan_dir);
        if i >= self.cache_start && (!resolved_only || self.entry(i).resolved_level >= 0) {
            let current_scan_dir = bidi_it.scan_dir;
            bidi_copy_it(bidi_it, &self.cache[i as usize]);
            self.cache_last_idx = i;
            bidi_it.scan_dir = current_scan_dir;
            return bidi_it.type_;
        }
        BidiType::UnknownBt
    }

    fn peek_at_next_level(&self, bidi_it: &BidiIt) -> i32 {
        if self.cache_idx == self.cache_start {
            std::process::abort();
        }
        if self.cache_last_idx == -1
            || (self.cache_last_idx >= self.cache_idx - 1 && bidi_it.scan_dir > 0)
        {
            return i32::from(self.entry(self.cache_idx - 1).resolved_level);
        }
        i32::from(
            self.entry(self.cache_last_idx + i64::from(bidi_it.scan_dir))
                .resolved_level,
        )
    }

    fn level_start(&self, level: i32) -> i64 {
        let slot = self.cache_find_level_change(level, -1, true);
        if slot >= 0 {
            return self.entry(slot).charpos;
        }
        -1
    }

    fn record_type_for_neutral(&mut self, info: &BidiSavedInfo, level: i32, nextp: bool) {
        let mut idx = self.cache_last_idx + 1;
        while idx < self.cache_idx {
            let entry = self.entry_mut(idx);
            let lev = entry.current_stack_level();
            if lev <= level {
                if nextp {
                    entry.next_for_neutral = *info;
                } else {
                    entry.prev_for_neutral = *info;
                }
                break;
            }
            idx += 1;
        }
    }
}

fn bidi_get_category(type_: BidiType) -> BidiCategory {
    use BidiType::*;
    match type_ {
        UnknownBt => BidiCategory::UnknownBc,
        StrongL | StrongR | StrongAl => BidiCategory::Strong,
        WeakEn | WeakEs | WeakEt | WeakAn | WeakCs | WeakNsm | WeakBn => BidiCategory::Weak,
        NeutralB | NeutralS | NeutralWs | NeutralOn => BidiCategory::Neutral,
        Lre | Lro | Rle | Rlo | Pdf | Lri | Rli | Fsi | Pdi => BidiCategory::ExplicitFormatting,
    }
}

fn bidi_isolate_fmt_char(ch_type: BidiType) -> bool {
    matches!(
        ch_type,
        BidiType::Lri | BidiType::Rli | BidiType::Pdi | BidiType::Fsi
    )
}

fn bidi_set_sos_type(bidi_it: &mut BidiIt, level_before: i32, level_after: i32) {
    let higher_level = level_before.max(level_after);
    bidi_it.sos = if higher_level & 1 != 0 {
        BidiDir::R2L
    } else {
        BidiDir::L2R
    };
    bidi_it.prev.type_ = BidiType::UnknownBt;
    bidi_it.last_strong.type_ = BidiType::UnknownBt;
    bidi_it.last_strong.orig_type = BidiType::UnknownBt;
    bidi_it.prev_for_neutral.type_ = if bidi_it.sos == BidiDir::R2L {
        BidiType::StrongR
    } else {
        BidiType::StrongL
    };
    bidi_it.prev_for_neutral.charpos = bidi_it.charpos;
    bidi_it.next_for_neutral.type_ = BidiType::UnknownBt;
    bidi_it.next_for_neutral.orig_type = BidiType::UnknownBt;
}

fn bidi_push_embedding_level(
    bidi_it: &mut BidiIt,
    level: i32,
    override_dir: BidiDir,
    isolate_status: bool,
) {
    let prev_level = bidi_it.current_stack_level();
    bidi_it.stack_idx += 1;
    let idx = bidi_it.stack_idx as usize;
    let (last_strong, prev_for_neutral, next_for_neutral, sos) = (
        bidi_it.last_strong.type_,
        bidi_it.prev_for_neutral.type_,
        bidi_it.next_for_neutral,
        bidi_it.sos,
    );
    let st = &mut bidi_it.level_stack[idx];
    st.level = level as u8;
    st.flags = ((override_dir as u8 & 3) << 1) | u8::from(isolate_status);
    if isolate_status {
        st.last_strong_type = last_strong.bits3();
        st.prev_for_neutral_type = prev_for_neutral.bits3();
        st.next_for_neutral_type = next_for_neutral.type_.bits3();
        st.next_for_neutral_pos = next_for_neutral.charpos;
        st.flags |= u8::from(sos != BidiDir::L2R) << 3;
    }
    bidi_set_sos_type(bidi_it, prev_level, level);
}

fn bidi_pop_embedding_level(bidi_it: &mut BidiIt) -> i32 {
    if bidi_it.stack_idx > 0 {
        let isolate_status = bidi_it.isolate_status(bidi_it.stack_idx);
        let old_level = bidi_it.current_stack_level();
        let st = bidi_it.level_stack[bidi_it.stack_idx as usize];
        if isolate_status {
            let sos = (st.flags >> 3) & 1;
            bidi_it.prev.orig_type = BidiType::UnknownBt;
            bidi_it.prev.type_ = BidiType::UnknownBt;
            bidi_it.last_strong.type_ = BidiType::from_index(i64::from(st.last_strong_type));
            bidi_it.prev_for_neutral.type_ =
                BidiType::from_index(i64::from(st.prev_for_neutral_type));
            bidi_it.next_for_neutral.type_ =
                BidiType::from_index(i64::from(st.next_for_neutral_type));
            bidi_it.next_for_neutral.charpos = st.next_for_neutral_pos;
            bidi_it.sos = if sos == 0 { BidiDir::L2R } else { BidiDir::R2L };
        } else {
            let below = bidi_it.stack_level(bidi_it.stack_idx - 1);
            bidi_set_sos_type(bidi_it, old_level, below);
        }
        bidi_it.stack_idx -= 1;
    }
    bidi_it.current_stack_level()
}

fn bidi_remember_char(bidi_it: &BidiIt, from_type: bool) -> BidiSavedInfo {
    BidiSavedInfo {
        charpos: bidi_it.charpos,
        type_: if from_type {
            bidi_it.type_
        } else {
            bidi_it.type_after_wn
        },
        orig_type: bidi_it.orig_type,
    }
}

fn bidi_set_paragraph_end(bidi_it: &mut BidiIt) {
    bidi_it.invalid_levels = 0;
    bidi_it.invalid_isolates = 0;
    bidi_it.stack_idx = 0;
    bidi_it.isolate_level = 0;
    bidi_it.resolved_level = bidi_it.level_stack[0].level as i8;
}

fn bidi_resolve_neutral_1(mut prev_type: BidiType, mut next_type: BidiType, lev: i32) -> BidiType {
    if next_type == BidiType::WeakEn || next_type == BidiType::WeakAn {
        next_type = BidiType::StrongR;
    }
    if prev_type == BidiType::WeakEn || prev_type == BidiType::WeakAn {
        prev_type = BidiType::StrongR;
    }
    if next_type == prev_type {
        next_type
    } else if lev & 1 == 0 {
        BidiType::StrongL
    } else {
        BidiType::StrongR
    }
}

fn canonical_equ(c: i32) -> i32 {
    if character::ascii_char_p(c as u32) {
        c
    } else if c == LEFT_POINTING_ANGLE_BRACKET {
        LEFT_ANGLE_BRACKET
    } else if c == RIGHT_POINTING_ANGLE_BRACKET {
        RIGHT_ANGLE_BRACKET
    } else {
        c
    }
}

fn bidi_count_bytes(s: &[u8], beg: i64, begbyte: i64, end: i64, unibyte: bool) -> i64 {
    let start = begbyte as usize;
    let mut p = start;
    if unibyte {
        p = end as usize;
    } else {
        if s[p] & 0xC0 == 0x80 {
            std::process::abort();
        }
        let mut pos = beg;
        while pos < end {
            p += character::bytes_by_char_head(s[p]);
            pos += 1;
        }
    }
    p as i64 - start as i64
}

#[derive(Clone, Copy, Default)]
struct BpaStackEntry {
    close_bracket_char: i32,
    open_bracket_idx: i32,
    flags: u8,
}

struct Fetched {
    ch: i32,
    ch_len: i64,
    nchars: i64,
}

impl Lisp {
    fn bidi_eob(&self, bidi_it: &BidiIt) -> i64 {
        if bidi_it.string_p() {
            bidi_it.string.schars
        } else {
            self.cur().zv as i64
        }
    }

    fn bidi_string_char(&self, string: &BidiStringData, bytepos: i64) -> (i32, i64) {
        let bytes: &[u8] = if string.lstring.is_string() {
            self.string_bytes(string.lstring)
        } else {
            string.s.as_deref().unwrap_or(&[])
        };
        let (c, len) = character::string_char(&bytes[bytepos as usize..]);
        (c as i32, len as i64)
    }

    fn bidi_buffer_char(&self, bytepos: i64) -> (i32, i64) {
        let text = &self.cur().text;
        let b = bytepos as usize;
        (
            text.char_at_byte(b) as i32,
            character::bytes_by_char_head(text.byte_at(b)) as i64,
        )
    }

    fn bidi_char_at_pos(&self, bytepos: i64, string: &BidiStringData, use_string: bool) -> i32 {
        if use_string {
            if string.unibyte {
                let bytes: &[u8] = if string.lstring.is_string() {
                    self.string_bytes(string.lstring)
                } else {
                    string.s.as_deref().unwrap_or(&[])
                };
                return i32::from(bytes[bytepos as usize]);
            }
            return self.bidi_string_char(string, bytepos).0;
        }
        self.bidi_buffer_char(bytepos).0
    }

    fn bidi_dec_both(&self, pos: &mut i64, pos_byte: &mut i64) {
        *pos -= 1;
        let text = &self.cur().text;
        let mut b = *pos_byte as usize - 1;
        if text.multibyte {
            while text.byte_at(b) & 0xC0 == 0x80 {
                b -= 1;
            }
        }
        *pos_byte = b as i64;
    }

    fn bidi_initialize(&mut self) -> LispResult<()> {
        let tables = [
            ("bidi-class", 0usize),
            ("mirroring", 1),
            ("bracket-type", 2),
        ];
        for (name, slot) in tables {
            let prop = self.intern(name);
            let table = self.uniprop_table(prop)?;
            if table.is_nil() {
                std::process::abort();
            }
            match slot {
                0 => self.bidi.type_table = table,
                1 => self.bidi.mirror_table = table,
                _ => self.bidi.brackets_table = table,
            }
        }
        self.bidi.paragraph_start_re = self.string("^\\(\x0c\\|[ \t]*\\)$");
        self.bidi.paragraph_separate_re = self.string("^[ \t\x0c]*$");
        self.bidi.cache_sp = 0;
        self.bidi.cache_max_elts = BIDI_CACHE_MAX_ELTS_PER_SLOT;
        self.bidi.initialized = true;
        Ok(())
    }

    fn bidi_get_type(&self, ch: i32, override_dir: BidiDir) -> BidiType {
        if ch == BIDI_EOB {
            return BidiType::NeutralB;
        }
        if ch < 0 || ch as u32 > character::MAX_CHAR {
            std::process::abort();
        }
        let value = self.char_table_ref(self.bidi.type_table, ch as u32);
        let default_type = BidiType::from_index(value.as_fixnum().unwrap_or(0));
        if default_type == BidiType::UnknownBt {
            std::process::abort();
        }
        use BidiType::*;
        match default_type {
            WeakBn | NeutralB | Lre | Lro | Rle | Rlo | Pdf | Lri | Rli | Fsi | Pdi => default_type,
            _ => match override_dir {
                BidiDir::L2R => StrongL,
                BidiDir::R2L => StrongR,
                BidiDir::NeutralDir => default_type,
            },
        }
    }

    pub fn bidi_mirror_char(&mut self, c: i32) -> LispResult<i32> {
        if c == BIDI_EOB {
            return Ok(c);
        }
        if c < 0 || c as u32 > character::MAX_CHAR {
            std::process::abort();
        }
        if !self.is_char_table(self.bidi.mirror_table) && !self.bidi.initialized {
            self.bidi_initialize()?;
        }
        let val = self.char_table_ref(self.bidi.mirror_table, c as u32);
        if let Some(v) = val.as_fixnum() {
            if v < 0 || v > i64::from(character::MAX_CHAR) {
                std::process::abort();
            }
            return Ok(v as i32);
        }
        Ok(c)
    }

    fn bidi_paired_bracket_type(&self, c: i32) -> BidiBracketType {
        if c == BIDI_EOB || self.var_true(sym::BIDI_INHIBIT_BPA) {
            return BidiBracketType::None;
        }
        if c < 0 || c as u32 > character::MAX_CHAR {
            std::process::abort();
        }
        match self
            .char_table_ref(self.bidi.brackets_table, c as u32)
            .as_fixnum()
        {
            Some(2) => BidiBracketType::Open,
            Some(3) => BidiBracketType::Close,
            _ => BidiBracketType::None,
        }
    }

    fn bidi_explicit_dir_char(&self, ch: i32) -> bool {
        if !self.bidi.initialized {
            std::process::abort();
        }
        if ch < 0 {
            return false;
        }
        let value = self.char_table_ref(self.bidi.type_table, ch as u32);
        let ch_type = BidiType::from_index(value.as_fixnum().unwrap_or(0));
        matches!(
            ch_type,
            BidiType::Lre | BidiType::Lro | BidiType::Rle | BidiType::Rlo | BidiType::Pdf
        )
    }

    pub fn bidi_level_start(&self, level: i32) -> i64 {
        self.bidi.level_start(level)
    }

    pub fn bidi_push_it(&mut self, bidi_it: &BidiIt) {
        let b = &mut self.bidi;
        b.cache_max_elts += BIDI_CACHE_MAX_ELTS_PER_SLOT;
        b.cache_ensure_space(b.cache_idx);
        let idx = b.cache_idx as usize;
        b.cache[idx] = bidi_it.clone();
        b.cache_idx += 1;
        b.cache_start_stack[b.cache_sp as usize] = b.cache_start;
        b.cache_sp += 1;
        b.cache_start = b.cache_idx;
        b.cache_last_idx = -1;
    }

    pub fn bidi_pop_it(&mut self, bidi_it: &mut BidiIt) {
        let b = &mut self.bidi;
        if b.cache_start <= 0 {
            std::process::abort();
        }
        b.cache_idx = b.cache_start - 1;
        *bidi_it = b.cache[b.cache_idx as usize].clone();
        if b.cache_sp <= 0 {
            std::process::abort();
        }
        b.cache_sp -= 1;
        b.cache_start = b.cache_start_stack[b.cache_sp as usize];
        b.cache_last_idx = -1;
        b.cache_max_elts -= BIDI_CACHE_MAX_ELTS_PER_SLOT;
    }

    pub fn bidi_shelve_cache(&mut self) -> ShelvedBidiCache {
        let b = &mut self.bidi;
        if b.cache_idx == 0 {
            return None;
        }
        Some(Box::new(BidiShelf {
            idx: b.cache_idx,
            cache: b.cache[..b.cache_idx as usize].to_vec(),
            start_stack: b.cache_start_stack,
            sp: b.cache_sp,
            start: b.cache_start,
            last_idx: b.cache_last_idx,
            max_elts: b.cache_max_elts,
        }))
    }

    pub fn bidi_unshelve_cache(&mut self, databuf: ShelvedBidiCache, just_free: bool) {
        let b = &mut self.bidi;
        match databuf {
            None => {
                if !just_free {
                    b.cache_start = 0;
                    b.cache_sp = 0;
                    b.cache_max_elts = BIDI_CACHE_MAX_ELTS_PER_SLOT;
                    b.cache_reset();
                }
            }
            Some(shelf) => {
                if !just_free {
                    b.cache_idx = shelf.idx;
                    b.cache_ensure_space(b.cache_idx);
                    for (i, entry) in shelf.cache.into_iter().enumerate() {
                        b.cache[i] = entry;
                    }
                    b.cache_start_stack = shelf.start_stack;
                    b.cache_sp = shelf.sp;
                    b.cache_start = shelf.start;
                    b.cache_last_idx = shelf.last_idx;
                    b.cache_max_elts = shelf.max_elts;
                }
            }
        }
    }

    pub fn bidi_init_it(
        &mut self,
        charpos: i64,
        bytepos: i64,
        frame_window_p: bool,
        bidi_it: &mut BidiIt,
    ) -> LispResult<()> {
        if !self.bidi.initialized {
            self.bidi_initialize()?;
        }
        if charpos >= 0 {
            bidi_it.charpos = charpos;
        }
        if bytepos >= 0 {
            bidi_it.bytepos = bytepos;
        }
        bidi_it.frame_window_p = frame_window_p;
        bidi_it.nchars = -1;
        bidi_it.first_elt = true;
        bidi_set_paragraph_end(bidi_it);
        bidi_it.new_paragraph = true;
        bidi_it.separator_limit = -1;
        bidi_it.type_ = BidiType::NeutralB;
        bidi_it.type_after_wn = BidiType::NeutralB;
        bidi_it.orig_type = BidiType::NeutralB;
        bidi_it.prev.type_ = BidiType::UnknownBt;
        bidi_it.prev.orig_type = BidiType::UnknownBt;
        bidi_it.last_strong.type_ = BidiType::UnknownBt;
        bidi_it.last_strong.orig_type = BidiType::UnknownBt;
        bidi_it.next_for_neutral.charpos = -1;
        bidi_it.next_for_neutral.type_ = BidiType::UnknownBt;
        bidi_it.next_for_neutral.orig_type = BidiType::UnknownBt;
        bidi_it.prev_for_neutral.charpos = -1;
        bidi_it.prev_for_neutral.type_ = BidiType::UnknownBt;
        bidi_it.prev_for_neutral.orig_type = BidiType::UnknownBt;
        bidi_it.bracket_pairing_pos = -1;
        bidi_it.sos = BidiDir::L2R;
        bidi_it.disp_pos = -1;
        bidi_it.disp_prop = 0;
        if self.bidi.cache_start == 0 {
            self.bidi.cache_shrink();
        } else {
            self.bidi.cache_reset();
        }
        Ok(())
    }

    fn bidi_line_init(&mut self, bidi_it: &mut BidiIt) {
        bidi_it.scan_dir = 1;
        bidi_it.stack_idx = 0;
        bidi_it.resolved_level = bidi_it.level_stack[0].level as i8;
        bidi_it.level_stack[0].flags = 0;
        bidi_it.invalid_levels = 0;
        bidi_it.isolate_level = 0;
        bidi_it.invalid_isolates = 0;
        bidi_it.next_en_pos = 0;
        bidi_it.next_en_type = BidiType::UnknownBt;
        bidi_it.next_for_ws.charpos = -1;
        bidi_it.next_for_ws.type_ = BidiType::UnknownBt;
        bidi_it.bracket_pairing_pos = -1;
        let before = i32::from(bidi_it.paragraph_dir == BidiDir::R2L);
        let after = bidi_it.stack_level(0);
        bidi_set_sos_type(bidi_it, before, after);
        self.bidi.cache_reset();
    }

    #[allow(clippy::too_many_arguments)]
    fn bidi_fetch_char(
        &mut self,
        charpos: i64,
        bytepos: i64,
        disp_pos: &mut i64,
        disp_prop: &mut i32,
        string: &BidiStringData,
        w: Value,
        frame_window_p: bool,
    ) -> LispResult<Fetched> {
        let string_p = string.s.is_some() || string.lstring.is_string();
        let endpos = if string_p {
            string.schars
        } else {
            self.cur().zv as i64
        };
        if charpos < endpos && charpos > *disp_pos {
            *disp_pos = self.compute_display_string_pos(
                (charpos, bytepos),
                string,
                w,
                frame_window_p,
                disp_prop,
            )?;
            if self.int_var(sym::MAX_REDISPLAY_TICKS) > 0 && *disp_pos > charpos {
                self.update_redisplay_ticks((*disp_pos - charpos) / 100 + 1, w)?;
            }
        }
        let mut fetched = Fetched {
            ch: 0,
            ch_len: 0,
            nchars: 0,
        };
        let mut normal = true;
        if charpos >= endpos {
            fetched = Fetched {
                ch: BIDI_EOB,
                ch_len: 1,
                nchars: 1,
            };
            *disp_pos = endpos;
            *disp_prop = 0;
            normal = false;
        } else if charpos >= *disp_pos && *disp_prop != 0 {
            if charpos > *disp_pos {
                std::process::abort();
            }
            let ch = if *disp_prop == 2 {
                PARAGRAPH_SEPARATOR
            } else {
                OBJECT_REPLACEMENT_CHARACTER
            };
            let disp_end_pos = self.compute_display_string_end(*disp_pos, string, w)?;
            if disp_end_pos < 0 {
                *disp_prop = 0;
            } else {
                let nchars = disp_end_pos - *disp_pos;
                if nchars <= 0 {
                    std::process::abort();
                }
                let ch_len = if let Some(s) = &string.s {
                    bidi_count_bytes(s, *disp_pos, bytepos, disp_end_pos, string.unibyte)
                } else if string.lstring.is_string() {
                    let data = self.string_bytes(string.lstring);
                    bidi_count_bytes(data, *disp_pos, bytepos, disp_end_pos, string.unibyte)
                } else {
                    self.cur().text.char_to_byte(disp_end_pos as usize) as i64 - bytepos
                };
                fetched = Fetched { ch, ch_len, nchars };
                normal = false;
            }
        }
        if normal {
            let (ch, ch_len) = if string_p {
                if !string.unibyte {
                    self.bidi_string_char(string, bytepos)
                } else {
                    let bytes: &[u8] = if let Some(s) = &string.s {
                        s
                    } else {
                        self.string_bytes(string.lstring)
                    };
                    let b = bytes[bytepos as usize];
                    let c = if b < 0x80 {
                        u32::from(b)
                    } else {
                        character::byte8_to_char(b)
                    };
                    (c as i32, 1)
                }
            } else {
                self.bidi_buffer_char(bytepos)
            };
            fetched = Fetched {
                ch,
                ch_len,
                nchars: 1,
            };
        }
        if charpos + fetched.nchars <= endpos
            && charpos + fetched.nchars > *disp_pos
            && *disp_prop != 0
        {
            *disp_pos = self.compute_display_string_pos(
                (charpos + fetched.nchars, bytepos + fetched.ch_len),
                string,
                w,
                frame_window_p,
                disp_prop,
            )?;
            if self.int_var(sym::MAX_REDISPLAY_TICKS) > 0 && *disp_pos > charpos + fetched.nchars {
                self.update_redisplay_ticks((*disp_pos - charpos - fetched.nchars) / 100 + 1, w)?;
            }
        }
        Ok(fetched)
    }

    #[allow(clippy::too_many_arguments)]
    fn bidi_fetch_char_skip_isolates(
        &mut self,
        mut charpos: i64,
        mut bytepos: i64,
        disp_pos: &mut i64,
        disp_prop: &mut i32,
        string: &BidiStringData,
        w: Value,
        frame_window_p: bool,
    ) -> LispResult<Fetched> {
        let (orig_charpos, orig_bytepos) = (charpos, bytepos);
        let mut fetched = self.bidi_fetch_char(
            charpos,
            bytepos,
            disp_pos,
            disp_prop,
            string,
            w,
            frame_window_p,
        )?;
        let mut ch_type = self.bidi_get_type(fetched.ch, BidiDir::NeutralDir);
        let mut level = 0i64;
        if matches!(ch_type, BidiType::Lri | BidiType::Rli | BidiType::Fsi) {
            level += 1;
            while level > 0 && ch_type != BidiType::NeutralB {
                charpos += fetched.nchars;
                bytepos += fetched.ch_len;
                fetched = self.bidi_fetch_char(
                    charpos,
                    bytepos,
                    disp_pos,
                    disp_prop,
                    string,
                    w,
                    frame_window_p,
                )?;
                ch_type = self.bidi_get_type(fetched.ch, BidiDir::NeutralDir);
                if matches!(ch_type, BidiType::Lri | BidiType::Rli | BidiType::Fsi) {
                    level += 1;
                } else if ch_type == BidiType::Pdi {
                    level -= 1;
                }
            }
        }
        fetched.nchars += charpos - orig_charpos;
        fetched.ch_len += bytepos - orig_bytepos;
        Ok(fetched)
    }

    fn bidi_paragraph_re(&self, symbol: Value, fallback: Value) -> Value {
        let re = self.plain_value(symbol);
        if re.is_string() { re } else { fallback }
    }

    fn bidi_at_paragraph_end(&mut self, charpos: i64, _bytepos: i64) -> LispResult<i64> {
        let sep_re = self.bidi_paragraph_re(
            sym::BIDI_PARAGRAPH_SEPARATE_RE,
            self.bidi.paragraph_separate_re,
        );
        let start_re =
            self.bidi_paragraph_re(sym::BIDI_PARAGRAPH_START_RE, self.bidi.paragraph_start_re);
        let count = self.specpdl.len();
        let r = (|| -> LispResult {
            self.specbind(sym::INHIBIT_QUIT, sym::T)?;
            let zv = self.cur().zv;
            let mut val = self.fast_looking_at(sep_re, charpos as usize, zv, Value::NIL)? as i64;
            if val < 0 {
                val = if self.fast_looking_at(start_re, charpos as usize, zv, Value::NIL)? >= 0 {
                    -1
                } else {
                    -2
                };
            }
            Ok(Value::fixnum(val))
        })();
        let val = self.unbind_to_value(count, r)?;
        Ok(val.as_fixnum().unwrap_or(-2))
    }

    fn bidi_find_paragraph_start(&mut self, mut pos: i64, mut pos_byte: i64) -> LispResult<i64> {
        let re = self.bidi_paragraph_re(sym::BIDI_PARAGRAPH_START_RE, self.bidi.paragraph_start_re);
        let limit = self.cur().zv;
        let mut n = 0i64;
        let count = self.specpdl.len();
        let r = (|| -> LispResult {
            self.specbind(sym::INHIBIT_QUIT, sym::T)?;
            loop {
                if pos_byte <= self.cur().begv_byte as i64 {
                    break;
                }
                let more = n < MAX_PARAGRAPH_SEARCH;
                n += 1;
                if !more || self.fast_looking_at(re, pos as usize, limit, Value::NIL)? >= 0 {
                    break;
                }
                self.bidi_dec_both(&mut pos, &mut pos_byte);
                let begv = self.cur().begv;
                pos = self.find_newline(pos as usize, begv, -1).0 as i64;
                pos_byte = self.cur().text.char_to_byte(pos as usize) as i64;
            }
            Ok(Value::NIL)
        })();
        self.unbind_to_value(count, r)?;
        if n >= MAX_PARAGRAPH_SEARCH {
            pos_byte = self.cur().begv_byte as i64;
        }
        let b = self.cur();
        Ok(pos_byte.clamp(b.begv_byte as i64, b.zv_byte as i64))
    }

    #[allow(clippy::too_many_arguments)]
    fn find_first_strong_char(
        &mut self,
        mut pos: i64,
        mut bytepos: i64,
        end: i64,
        disp_pos: &mut i64,
        disp_prop: &mut i32,
        string: &BidiStringData,
        w: Value,
        string_p: bool,
        frame_window_p: bool,
        lens: &mut (i64, i64),
        stop_at_pdi: bool,
    ) -> LispResult<BidiType> {
        if stop_at_pdi {
            pos += lens.1;
            bytepos += lens.0;
        }
        let fetched = self.bidi_fetch_char_skip_isolates(
            pos,
            bytepos,
            disp_pos,
            disp_prop,
            string,
            w,
            frame_window_p,
        )?;
        *lens = (fetched.ch_len, fetched.nchars);
        let mut type_ = self.bidi_get_type(fetched.ch, BidiDir::NeutralDir);
        let pos1 = pos;
        pos += lens.1;
        bytepos += lens.0;
        while bidi_get_category(type_) != BidiCategory::Strong
            && !(stop_at_pdi && type_ == BidiType::Pdi)
            && pos - pos1 < MAX_STRONG_CHAR_SEARCH
        {
            if pos >= end {
                type_ = BidiType::NeutralB;
                break;
            }
            if !string_p
                && type_ == BidiType::NeutralB
                && self.bidi_at_paragraph_end(pos, bytepos)? >= -1
            {
                break;
            }
            let fetched = self.bidi_fetch_char_skip_isolates(
                pos,
                bytepos,
                disp_pos,
                disp_prop,
                string,
                w,
                frame_window_p,
            )?;
            *lens = (fetched.ch_len, fetched.nchars);
            pos += lens.1;
            bytepos += lens.0;
            type_ = self.bidi_get_type(fetched.ch, BidiDir::NeutralDir);
        }
        self.bidi.nsearch_for_strong += pos - pos1;
        Ok(type_)
    }

    pub fn bidi_paragraph_init(
        &mut self,
        mut dir: BidiDir,
        bidi_it: &mut BidiIt,
        no_default_p: bool,
    ) -> LispResult<()> {
        let mut bytepos = bidi_it.bytepos;
        let string_p = bidi_it.string_p();
        let begbyte = if string_p {
            0
        } else {
            self.cur().begv_byte as i64
        };
        let end = if string_p {
            bidi_it.string.schars
        } else {
            self.cur().zv as i64
        };
        let mut pos = bidi_it.charpos;
        self.bidi.nsearch_for_strong = 0;
        if bytepos == begbyte && bidi_it.charpos == end {
            dir = BidiDir::L2R;
        } else if bidi_it.charpos >= end || bytepos < begbyte {
            std::process::abort();
        }
        match dir {
            BidiDir::L2R => {
                bidi_it.paragraph_dir = BidiDir::L2R;
                bidi_it.new_paragraph = false;
            }
            BidiDir::R2L => {
                bidi_it.paragraph_dir = BidiDir::R2L;
                bidi_it.new_paragraph = false;
            }
            BidiDir::NeutralDir => {
                let mut disp_pos = -1i64;
                let mut disp_prop = 0i32;
                if !self.bidi.initialized {
                    self.bidi_initialize()?;
                }
                if !bidi_it.first_elt && bidi_it.charpos < bidi_it.separator_limit {
                    return Ok(());
                }
                pos = bidi_it.charpos;
                if bytepos > begbyte
                    && self.bidi_char_at_pos(bytepos, &bidi_it.string, string_p) == 10
                {
                    bytepos += 1;
                    pos += 1;
                }
                let mut pstartbyte = if string_p {
                    0
                } else {
                    self.bidi_find_paragraph_start(pos, bytepos)?
                };
                bidi_it.separator_limit = -1;
                bidi_it.new_paragraph = false;
                let string = bidi_it.string.clone();
                let w = bidi_it.w;
                loop {
                    bytepos = pstartbyte;
                    if !string_p {
                        pos = self.cur().text.byte_to_char(bytepos as usize) as i64;
                    }
                    let mut lens = (0, 0);
                    let type_ = self.find_first_strong_char(
                        pos,
                        bytepos,
                        end,
                        &mut disp_pos,
                        &mut disp_prop,
                        &string,
                        w,
                        string_p,
                        bidi_it.frame_window_p,
                        &mut lens,
                        false,
                    )?;
                    if type_ == BidiType::StrongR || type_ == BidiType::StrongAl {
                        bidi_it.paragraph_dir = BidiDir::R2L;
                    } else if type_ == BidiType::StrongL {
                        bidi_it.paragraph_dir = BidiDir::L2R;
                    }
                    if !string_p && no_default_p && bidi_it.paragraph_dir == BidiDir::NeutralDir {
                        let begv_byte = self.cur().begv_byte as i64;
                        if pstartbyte == begv_byte {
                            bidi_it.paragraph_dir = BidiDir::L2R;
                        } else {
                            let mut prevpbyte = pstartbyte;
                            let mut p = self.cur().text.byte_to_char(pstartbyte as usize) as i64;
                            let mut pbyte = pstartbyte;
                            while pbyte > begv_byte && prevpbyte >= pstartbyte {
                                self.bidi_dec_both(&mut p, &mut pbyte);
                                prevpbyte = self.bidi_find_paragraph_start(p, pbyte)?;
                            }
                            pstartbyte = prevpbyte;
                        }
                    }
                    if !(!string_p && no_default_p && bidi_it.paragraph_dir == BidiDir::NeutralDir)
                    {
                        break;
                    }
                }
            }
        }
        if bidi_it.paragraph_dir != BidiDir::L2R && bidi_it.paragraph_dir != BidiDir::R2L {
            bidi_it.paragraph_dir = BidiDir::L2R;
        }
        bidi_it.level_stack[0].level = u8::from(bidi_it.paragraph_dir == BidiDir::R2L);
        self.bidi_line_init(bidi_it);
        let nexamined = bidi_it.charpos - pos + self.bidi.nsearch_for_strong;
        if self.int_var(sym::MAX_REDISPLAY_TICKS) > 0 && nexamined > 0 {
            self.update_redisplay_ticks(nexamined / 50, bidi_it.w)?;
        }
        Ok(())
    }

    fn bidi_resolve_explicit(&mut self, bidi_it: &mut BidiIt) -> LispResult<i32> {
        let mut prev_type = BidiType::UnknownBt;
        let string_p = bidi_it.string_p();
        let eob = self.bidi_eob(bidi_it);
        if bidi_it.type_after_wn != BidiType::WeakBn && bidi_it.type_ != BidiType::WeakBn {
            let from_type = bidi_it.type_after_wn == BidiType::NeutralOn
                && bidi_get_category(bidi_it.type_) == BidiCategory::Strong
                && self.bidi_paired_bracket_type(bidi_it.ch) == BidiBracketType::Close;
            bidi_it.prev = bidi_remember_char(bidi_it, from_type);
        }
        if matches!(
            bidi_it.type_after_wn,
            BidiType::StrongR | BidiType::StrongL | BidiType::StrongAl
        ) {
            bidi_it.last_strong = bidi_remember_char(bidi_it, false);
        }
        if matches!(
            bidi_it.type_,
            BidiType::StrongR | BidiType::StrongL | BidiType::WeakEn | BidiType::WeakAn
        ) {
            bidi_it.prev_for_neutral = bidi_remember_char(bidi_it, true);
        }
        if bidi_it.charpos >= bidi_it.next_for_neutral.charpos {
            bidi_it.next_for_neutral.type_ = BidiType::UnknownBt;
            if bidi_it.bracket_pairing_pos == eob {
                bidi_it.bracket_pairing_pos = -1;
            }
        }
        if bidi_it.next_en_pos >= 0 && bidi_it.charpos >= bidi_it.next_en_pos {
            bidi_it.next_en_pos = 0;
            bidi_it.next_en_type = BidiType::UnknownBt;
        }
        if bidi_it.bracket_pairing_pos != eob {
            bidi_it.bracket_pairing_pos = -1;
            bidi_it.bracket_enclosed_type = BidiType::UnknownBt;
        }
        let begin = if string_p {
            0
        } else {
            self.cur().begv_byte as i64
        };
        if bidi_it.bytepos < begin || bidi_it.first_elt {
            bidi_it.first_elt = false;
            if string_p {
                if bidi_it.charpos < 0 {
                    bidi_it.charpos = 0;
                    bidi_it.bytepos = 0;
                }
            } else if bidi_it.charpos < self.cur().begv as i64 {
                bidi_it.charpos = self.cur().begv as i64;
                bidi_it.bytepos = self.cur().begv_byte as i64;
            }
            if bidi_it.first_elt && bidi_it.prev.type_ != BidiType::UnknownBt {
                prev_type = bidi_it.prev.orig_type;
            }
        } else if bidi_it.charpos < eob {
            if bidi_it.nchars <= 0 {
                std::process::abort();
            }
            bidi_it.charpos += bidi_it.nchars;
            if bidi_it.ch_len == 0 {
                std::process::abort();
            }
            bidi_it.bytepos += bidi_it.ch_len;
            prev_type = bidi_it.orig_type;
        } else {
            prev_type = BidiType::NeutralB;
        }
        let current_level = bidi_it.current_stack_level();
        let isolate_status = bidi_it.isolate_status(bidi_it.stack_idx);
        let mut override_dir = bidi_it.override_dir(bidi_it.stack_idx);
        let mut new_level = current_level;
        let curchar;
        if bidi_it.charpos >= eob {
            curchar = BIDI_EOB;
            bidi_it.ch_len = 1;
            bidi_it.nchars = 1;
            bidi_it.disp_pos = eob;
            bidi_it.disp_prop = 0;
        } else {
            match prev_type {
                BidiType::Rli => {
                    if current_level < BIDI_MAXDEPTH
                        && bidi_it.invalid_levels == 0
                        && bidi_it.invalid_isolates == 0
                    {
                        new_level = ((current_level + 1) & !1) + 1;
                        bidi_it.isolate_level += 1;
                        bidi_push_embedding_level(bidi_it, new_level, BidiDir::NeutralDir, true);
                    } else {
                        bidi_it.invalid_isolates += 1;
                    }
                }
                BidiType::Lri => {
                    if current_level < BIDI_MAXDEPTH - 1
                        && bidi_it.invalid_levels == 0
                        && bidi_it.invalid_isolates == 0
                    {
                        new_level = (current_level + 2) & !1;
                        bidi_it.isolate_level += 1;
                        bidi_push_embedding_level(bidi_it, new_level, BidiDir::NeutralDir, true);
                    } else {
                        bidi_it.invalid_isolates += 1;
                    }
                }
                BidiType::Pdf if bidi_it.invalid_isolates == 0 => {
                    if bidi_it.invalid_levels != 0 {
                        bidi_it.invalid_levels -= 1;
                    } else if !isolate_status && bidi_it.stack_idx >= 1 {
                        new_level = bidi_pop_embedding_level(bidi_it);
                    }
                }
                _ => {}
            }
            let (mut disp_pos, mut disp_prop) = (bidi_it.disp_pos, bidi_it.disp_prop);
            let string = bidi_it.string.clone();
            let fetched = self.bidi_fetch_char(
                bidi_it.charpos,
                bidi_it.bytepos,
                &mut disp_pos,
                &mut disp_prop,
                &string,
                bidi_it.w,
                bidi_it.frame_window_p,
            )?;
            bidi_it.disp_pos = disp_pos;
            bidi_it.disp_prop = disp_prop;
            bidi_it.ch_len = fetched.ch_len;
            bidi_it.nchars = fetched.nchars;
            curchar = fetched.ch;
        }
        bidi_it.ch = curchar;
        bidi_it.resolved_level = new_level as i8;
        let mut type_ = self.bidi_get_type(curchar, BidiDir::NeutralDir);
        bidi_it.orig_type = type_;
        bidi_it.type_after_wn = BidiType::UnknownBt;
        match type_ {
            BidiType::Rle | BidiType::Rlo => {
                bidi_it.type_after_wn = type_;
                type_ = BidiType::WeakBn;
                if new_level < BIDI_MAXDEPTH
                    && bidi_it.invalid_levels == 0
                    && bidi_it.invalid_isolates == 0
                {
                    new_level = ((new_level + 1) & !1) + 1;
                    override_dir = if bidi_it.type_after_wn == BidiType::Rle {
                        BidiDir::NeutralDir
                    } else {
                        BidiDir::R2L
                    };
                    bidi_push_embedding_level(bidi_it, new_level, override_dir, false);
                    bidi_it.resolved_level = new_level as i8;
                } else if bidi_it.invalid_isolates == 0 {
                    bidi_it.invalid_levels += 1;
                }
            }
            BidiType::Lre | BidiType::Lro => {
                bidi_it.type_after_wn = type_;
                type_ = BidiType::WeakBn;
                if new_level < BIDI_MAXDEPTH - 1
                    && bidi_it.invalid_levels == 0
                    && bidi_it.invalid_isolates == 0
                {
                    new_level = (new_level + 2) & !1;
                    override_dir = if bidi_it.type_after_wn == BidiType::Lre {
                        BidiDir::NeutralDir
                    } else {
                        BidiDir::L2R
                    };
                    bidi_push_embedding_level(bidi_it, new_level, override_dir, false);
                    bidi_it.resolved_level = new_level as i8;
                } else if bidi_it.invalid_isolates == 0 {
                    bidi_it.invalid_levels += 1;
                }
            }
            BidiType::Fsi | BidiType::Rli | BidiType::Lri => {
                if type_ == BidiType::Fsi {
                    let end = if string_p {
                        bidi_it.string.schars
                    } else {
                        self.cur().zv as i64
                    };
                    let (mut disp_pos, mut disp_prop) = (bidi_it.disp_pos, bidi_it.disp_prop);
                    let mut lens = (bidi_it.ch_len, bidi_it.nchars);
                    let string = bidi_it.string.clone();
                    let typ1 = self.find_first_strong_char(
                        bidi_it.charpos,
                        bidi_it.bytepos,
                        end,
                        &mut disp_pos,
                        &mut disp_prop,
                        &string,
                        bidi_it.w,
                        string_p,
                        bidi_it.frame_window_p,
                        &mut lens,
                        true,
                    )?;
                    type_ = if typ1 != BidiType::StrongR && typ1 != BidiType::StrongAl {
                        BidiType::Lri
                    } else {
                        BidiType::Rli
                    };
                    bidi_it.orig_type = type_;
                }
                bidi_it.type_after_wn = match override_dir {
                    BidiDir::NeutralDir => type_,
                    BidiDir::L2R => BidiType::StrongL,
                    BidiDir::R2L => BidiType::StrongR,
                };
            }
            BidiType::Pdi => {
                if bidi_it.invalid_isolates != 0 {
                    bidi_it.invalid_isolates -= 1;
                } else if bidi_it.isolate_level > 0 {
                    bidi_it.invalid_levels = 0;
                    while !bidi_it.isolate_status(bidi_it.stack_idx) {
                        bidi_pop_embedding_level(bidi_it);
                    }
                    new_level = bidi_pop_embedding_level(bidi_it);
                    bidi_it.isolate_level -= 1;
                }
                bidi_it.resolved_level = new_level as i8;
                bidi_it.type_after_wn = match bidi_it.override_dir(bidi_it.stack_idx) {
                    BidiDir::L2R => BidiType::StrongL,
                    BidiDir::R2L => BidiType::StrongR,
                    BidiDir::NeutralDir => type_,
                };
            }
            BidiType::Pdf => {
                bidi_it.type_after_wn = type_;
                type_ = BidiType::WeakBn;
            }
            _ => {}
        }
        bidi_it.type_ = type_;
        if bidi_it.type_ == BidiType::NeutralB {
            bidi_set_paragraph_end(bidi_it);
            bidi_it.type_after_wn = bidi_it.type_;
        }
        Ok(i32::from(bidi_it.resolved_level))
    }

    fn bidi_resolve_weak(&mut self, bidi_it: &mut BidiIt) -> LispResult<BidiType> {
        let prev_level = bidi_it.current_stack_level();
        let new_level = self.bidi_resolve_explicit(bidi_it)?;
        let eob = self.bidi_eob(bidi_it);
        let mut type_ = bidi_it.type_;
        let override_dir = bidi_it.override_dir(bidi_it.stack_idx);
        if bidi_it.type_ == BidiType::NeutralB {
            bidi_set_sos_type(bidi_it, prev_level, new_level);
        }
        if matches!(
            type_,
            BidiType::NeutralS | BidiType::NeutralWs | BidiType::WeakBn | BidiType::StrongAl
        ) {
            bidi_it.type_after_wn = type_;
        }
        if override_dir == BidiDir::R2L {
            type_ = BidiType::StrongR;
        } else if override_dir == BidiDir::L2R {
            type_ = BidiType::StrongL;
        } else {
            if type_ == BidiType::WeakNsm {
                if bidi_it.prev.type_ != BidiType::UnknownBt
                    && bidi_it.prev.type_ != BidiType::NeutralB
                {
                    type_ = if bidi_isolate_fmt_char(bidi_it.prev.type_) {
                        BidiType::NeutralOn
                    } else {
                        bidi_it.prev.type_
                    };
                } else if bidi_it.sos == BidiDir::R2L {
                    type_ = BidiType::StrongR;
                } else if bidi_it.sos == BidiDir::L2R {
                    type_ = BidiType::StrongL;
                } else {
                    std::process::abort();
                }
            }
            if type_ == BidiType::WeakEn && bidi_it.last_strong.type_ == BidiType::StrongAl {
                type_ = BidiType::WeakAn;
            } else if type_ == BidiType::StrongAl {
                type_ = BidiType::StrongR;
            } else if (type_ == BidiType::WeakEs
                && bidi_it.prev.type_ == BidiType::WeakEn
                && bidi_it.prev.orig_type == BidiType::WeakEn)
                || (type_ == BidiType::WeakCs
                    && ((bidi_it.prev.type_ == BidiType::WeakEn
                        && bidi_it.prev.orig_type == BidiType::WeakEn)
                        || bidi_it.prev.type_ == BidiType::WeakAn))
            {
                let next_char = if bidi_it.charpos + bidi_it.nchars >= eob {
                    BIDI_EOB
                } else {
                    self.bidi_char_at_pos(
                        bidi_it.bytepos + bidi_it.ch_len,
                        &bidi_it.string,
                        bidi_it.string_p(),
                    )
                };
                let mut type_of_next = self.bidi_get_type(next_char, override_dir);
                if type_of_next == BidiType::WeakBn || self.bidi_explicit_dir_char(next_char) {
                    let saved_it = bidi_it.clone();
                    while self.bidi_resolve_explicit(bidi_it)? == new_level
                        && bidi_it.type_ == BidiType::WeakBn
                    {
                        type_of_next = bidi_it.type_;
                    }
                    bidi_copy_it(bidi_it, &saved_it);
                }
                if type_ == BidiType::WeakEs
                    && type_of_next == BidiType::WeakEn
                    && bidi_it.last_strong.type_ != BidiType::StrongAl
                {
                    type_ = BidiType::WeakEn;
                } else if type_ == BidiType::WeakCs {
                    if bidi_it.prev.type_ == BidiType::WeakAn
                        && (type_of_next == BidiType::WeakAn
                            || (type_of_next == BidiType::WeakEn
                                && bidi_it.last_strong.type_ == BidiType::StrongAl))
                    {
                        type_ = BidiType::WeakAn;
                    } else if bidi_it.prev.type_ == BidiType::WeakEn
                        && type_of_next == BidiType::WeakEn
                        && bidi_it.last_strong.type_ != BidiType::StrongAl
                    {
                        type_ = BidiType::WeakEn;
                    }
                }
            } else if type_ == BidiType::WeakEt || type_ == BidiType::WeakBn {
                if bidi_it.prev.type_ == BidiType::WeakEn {
                    type_ = BidiType::WeakEn;
                } else if bidi_it.next_en_pos > bidi_it.charpos
                    && bidi_it.next_en_type != BidiType::WeakBn
                {
                    if bidi_it.next_en_type == BidiType::WeakEn {
                        type_ = BidiType::WeakEn;
                    }
                } else if type_ == BidiType::WeakBn
                    && new_level == 0
                    && !self.bidi_explicit_dir_char(bidi_it.ch)
                    && (bidi_it.last_strong.type_ == BidiType::StrongL
                        || (bidi_it.last_strong.type_ == BidiType::UnknownBt
                            && bidi_it.sos == BidiDir::L2R))
                {
                    type_ = BidiType::StrongL;
                } else if bidi_it.next_en_pos >= 0 {
                    let mut en_pos = bidi_it.charpos + bidi_it.nchars;
                    if bidi_it.nchars <= 0 {
                        std::process::abort();
                    }
                    let next_char = if bidi_it.charpos + bidi_it.nchars >= eob {
                        BIDI_EOB
                    } else {
                        self.bidi_char_at_pos(
                            bidi_it.bytepos + bidi_it.ch_len,
                            &bidi_it.string,
                            bidi_it.string_p(),
                        )
                    };
                    let mut type_of_next = self.bidi_get_type(next_char, override_dir);
                    if type_of_next == BidiType::WeakEt
                        || type_of_next == BidiType::WeakBn
                        || self.bidi_explicit_dir_char(next_char)
                    {
                        let saved_it = bidi_it.clone();
                        while self.bidi_resolve_explicit(bidi_it)? == new_level
                            && (bidi_it.type_ == BidiType::WeakBn
                                || bidi_it.type_ == BidiType::WeakEt)
                        {
                            type_of_next = bidi_it.type_;
                        }
                        if type_ == BidiType::WeakBn
                            && bidi_it.charpos == saved_it.charpos + saved_it.nchars
                        {
                            en_pos = saved_it.charpos;
                            type_of_next = type_;
                        } else {
                            en_pos = bidi_it.charpos;
                        }
                        bidi_copy_it(bidi_it, &saved_it);
                    }
                    bidi_it.next_en_pos = en_pos;
                    if type_of_next == BidiType::WeakEn {
                        if bidi_it.last_strong.type_ == BidiType::StrongAl {
                            type_of_next = BidiType::WeakAn;
                        } else if type_ == BidiType::WeakBn {
                            type_ = BidiType::NeutralOn;
                        } else {
                            type_ = BidiType::WeakEn;
                        }
                    } else if type_of_next == BidiType::NeutralB {
                        bidi_it.next_en_pos = -1;
                    }
                    bidi_it.next_en_type = type_of_next;
                }
            }
        }
        if matches!(
            type_,
            BidiType::WeakEs | BidiType::WeakEt | BidiType::WeakCs
        ) || (type_ == BidiType::WeakBn
            && matches!(
                bidi_it.prev.type_,
                BidiType::WeakCs | BidiType::WeakEs | BidiType::WeakEt
            ))
        {
            type_ = BidiType::NeutralOn;
        }
        if bidi_it.type_after_wn == BidiType::UnknownBt {
            bidi_it.type_after_wn = type_;
        }
        if type_ == BidiType::WeakEn
            && (bidi_it.last_strong.type_ == BidiType::StrongL
                || (bidi_it.last_strong.type_ == BidiType::UnknownBt
                    && bidi_it.sos == BidiDir::L2R))
        {
            type_ = BidiType::StrongL;
        }
        bidi_it.type_ = type_;
        Ok(type_)
    }

    fn push_bpa_stack(
        &mut self,
        bpa_stack: &mut [BpaStackEntry],
        bpa_sp: &mut i32,
        ch: i32,
    ) -> LispResult<()> {
        if *bpa_sp < MAX_BPA_STACK - 1 && self.bidi.cache_last_idx <= i64::from(i32::MAX) {
            *bpa_sp += 1;
            let ch = canonical_equ(ch);
            let close = self.bidi_mirror_char(ch)?;
            bpa_stack[*bpa_sp as usize] = BpaStackEntry {
                close_bracket_char: close,
                open_bracket_idx: self.bidi.cache_last_idx as i32,
                flags: 0,
            };
        }
        Ok(())
    }

    fn bidi_find_bracket_pairs(&mut self, bidi_it: &mut BidiIt) -> LispResult<bool> {
        let mut type_ = bidi_it.type_;
        let mut retval = false;
        let mut n = 0i64;
        if bidi_it.scan_dir != 1 {
            std::process::abort();
        }
        let mut btype = self.bidi_paired_bracket_type(bidi_it.ch);
        if btype == BidiBracketType::Open {
            let mut bpa_stack = vec![BpaStackEntry::default(); MAX_BPA_STACK as usize];
            let mut bpa_sp: i32 = -1;
            let base_level = bidi_it.stack_level(0);
            let embedding_level = bidi_it.current_stack_level();
            let mut maxlevel = embedding_level;
            let embedding_type = if embedding_level & 1 != 0 {
                BidiType::StrongR
            } else {
                BidiType::StrongL
            };
            let mut tem_it = BidiIt::default();
            let (mut l2r_seen, mut r2l_seen) = (false, false);
            let mut pairing_pos = 0i64;
            let idx_at_entry = self.bidi.cache_idx;
            let saved_it = bidi_it.clone();
            tem_it.scan_dir = 1;
            let mut gave_up = false;
            'outer: loop {
                let current_level = bidi_it.current_stack_level();
                if maxlevel < current_level {
                    maxlevel = current_level;
                }
                if btype == BidiBracketType::Open && bidi_it.bracket_pairing_pos == -1 {
                    bidi_it.bracket_pairing_pos = bidi_it.charpos;
                }
                if !self
                    .bidi
                    .cache_iterator_state(bidi_it, type_ == BidiType::NeutralB, false)
                {
                    let n = idx_at_entry - self.bidi.cache_start;
                    self.bidi.cache_reset_to(n);
                    bidi_copy_it(bidi_it, &saved_it);
                    gave_up = true;
                    break;
                }
                if btype == BidiBracketType::Open {
                    self.push_bpa_stack(&mut bpa_stack, &mut bpa_sp, bidi_it.ch)?;
                } else if btype == BidiBracketType::Close {
                    let mut sp = bpa_sp;
                    let curchar = canonical_equ(bidi_it.ch);
                    while sp >= 0 && bpa_stack[sp as usize].close_bracket_char != curchar {
                        sp -= 1;
                    }
                    if sp >= 0 {
                        let entry = bpa_stack[sp as usize];
                        self.bidi
                            .cache_fetch_state(i64::from(entry.open_bracket_idx), &mut tem_it);
                        tem_it.bracket_enclosed_type = if entry.flags & FLAG_EMBEDDING_INSIDE != 0 {
                            embedding_type
                        } else if entry.flags & FLAG_OPPOSITE_INSIDE != 0 {
                            if embedding_type == BidiType::StrongL {
                                BidiType::StrongR
                            } else {
                                BidiType::StrongL
                            }
                        } else {
                            BidiType::UnknownBt
                        };
                        tem_it.bracket_pairing_pos = bidi_it.charpos;
                        self.bidi.cache_iterator_state(&tem_it, false, true);
                        bpa_sp = sp - 1;
                    }
                    if bpa_sp < 0 {
                        retval = true;
                        break;
                    }
                } else if bidi_get_category(bidi_it.type_after_wn) != BidiCategory::Neutral {
                    let mut flag = 0u8;
                    match bidi_it.type_ {
                        BidiType::StrongL => {
                            flag = if embedding_level & 1 == 0 {
                                FLAG_EMBEDDING_INSIDE
                            } else {
                                FLAG_OPPOSITE_INSIDE
                            };
                            l2r_seen = true;
                        }
                        BidiType::StrongR | BidiType::WeakEn | BidiType::WeakAn => {
                            flag = if embedding_level & 1 == 1 {
                                FLAG_EMBEDDING_INSIDE
                            } else {
                                FLAG_OPPOSITE_INSIDE
                            };
                            r2l_seen = true;
                        }
                        _ => {}
                    }
                    if flag != 0 {
                        let mut sp = bpa_sp;
                        while sp >= 0 {
                            bpa_stack[sp as usize].flags |= flag;
                            sp -= 1;
                        }
                    }
                }
                let old_sidx = bidi_it.stack_idx;
                type_ = self.bidi_resolve_weak(bidi_it)?;
                n += 1;
                let new_sidx = bidi_it.stack_idx;
                if bidi_it.stack_level(new_sidx) > current_level
                    && (bidi_it.isolate_status(new_sidx)
                        || (new_sidx > old_sidx + 1 && bidi_it.isolate_status(new_sidx - 1)))
                {
                    while bidi_it.current_stack_level() > current_level {
                        if maxlevel < bidi_it.current_stack_level() {
                            maxlevel = bidi_it.current_stack_level();
                        }
                        if !self.bidi.cache_iterator_state(
                            bidi_it,
                            type_ == BidiType::NeutralB,
                            false,
                        ) {
                            let n = idx_at_entry - self.bidi.cache_start;
                            self.bidi.cache_reset_to(n);
                            bidi_copy_it(bidi_it, &saved_it);
                            gave_up = true;
                            break 'outer;
                        }
                        type_ = self.bidi_resolve_weak(bidi_it)?;
                        n += 1;
                    }
                }
                if type_ == BidiType::NeutralB || bidi_it.current_stack_level() != current_level {
                    pairing_pos = bidi_it.charpos;
                    break;
                }
                btype = if bidi_it.type_after_wn == BidiType::NeutralOn {
                    self.bidi_paired_bracket_type(bidi_it.ch)
                } else {
                    BidiBracketType::None
                };
            }
            if !gave_up {
                self.bidi.cache_find(saved_it.charpos, false, bidi_it);
                if maxlevel == base_level
                    && (l2r_seen || r2l_seen)
                    && ((base_level == 0 && !r2l_seen) || (base_level == 1 && !l2r_seen))
                {
                    let eob = self.bidi_eob(bidi_it);
                    if retval {
                        pairing_pos = bidi_it.bracket_pairing_pos;
                    }
                    bidi_it.bracket_pairing_pos = eob;
                    bidi_it.bracket_enclosed_type = embedding_type;
                    let n = self.bidi.cache_last_idx - self.bidi.cache_start;
                    self.bidi.cache_reset_to(n);
                    bidi_it.next_for_neutral.type_ = embedding_type;
                    bidi_it.next_for_neutral.charpos = pairing_pos;
                    retval = false;
                }
            }
        }
        if self.int_var(sym::MAX_REDISPLAY_TICKS) > 0 && n > 0 {
            self.update_redisplay_ticks(n / 20 + 1, bidi_it.w)?;
        }
        Ok(retval)
    }

    fn bidi_resolve_brackets(&mut self, bidi_it: &mut BidiIt) -> LispResult<BidiType> {
        let prev_level = bidi_it.current_stack_level();
        let mut resolve_bracket = false;
        let mut type_ = BidiType::UnknownBt;
        let mut ch = 0;
        let eob = self.bidi_eob(bidi_it);
        let prev_for_neutral = if matches!(
            bidi_it.type_,
            BidiType::StrongL | BidiType::StrongR | BidiType::WeakAn | BidiType::WeakEn
        ) {
            bidi_remember_char(bidi_it, true)
        } else {
            bidi_it.prev_for_neutral
        };
        let mut next_for_neutral = BidiSavedInfo::default();
        if bidi_it.next_for_neutral.charpos > bidi_it.charpos {
            next_for_neutral = bidi_it.next_for_neutral;
        } else {
            next_for_neutral.charpos = -1;
        }
        if !bidi_it.first_elt {
            type_ = self
                .bidi
                .cache_find(bidi_it.charpos + bidi_it.nchars, false, bidi_it);
            ch = bidi_it.ch;
        }
        if type_ == BidiType::UnknownBt {
            type_ = self.bidi_resolve_weak(bidi_it)?;
            if type_ == BidiType::NeutralOn {
                if bidi_it.bracket_pairing_pos == eob {
                    if bidi_it.next_for_neutral.charpos == bidi_it.charpos
                        && self.bidi_paired_bracket_type(bidi_it.ch) == BidiBracketType::Close
                    {
                        type_ = bidi_it.bracket_enclosed_type;
                    }
                } else if self.bidi_find_bracket_pairs(bidi_it)? {
                    resolve_bracket = true;
                }
            }
        } else if bidi_it.bracket_pairing_pos != eob {
            if bidi_it.current_stack_level() > prev_level
                && bidi_it.isolate_status(bidi_it.stack_idx)
            {
                self.bidi
                    .record_type_for_neutral(&prev_for_neutral, prev_level, false);
                self.bidi
                    .record_type_for_neutral(&next_for_neutral, prev_level, true);
            }
            if type_ == BidiType::NeutralOn
                && self.bidi_paired_bracket_type(ch) == BidiBracketType::Open
                && (bidi_it.bracket_pairing_pos > bidi_it.charpos
                    || (bidi_it.bracket_pairing_pos == -1
                        && self.bidi_find_bracket_pairs(bidi_it)?))
            {
                resolve_bracket = true;
            }
            if bidi_it.current_stack_level() == prev_level {
                bidi_it.prev_for_neutral = prev_for_neutral;
                if next_for_neutral.charpos > 0 {
                    bidi_it.next_for_neutral = next_for_neutral;
                }
            }
        }
        if resolve_bracket {
            let embedding_level = bidi_it.current_stack_level();
            let embedding_type = if embedding_level & 1 != 0 {
                BidiType::StrongR
            } else {
                BidiType::StrongL
            };
            if bidi_it.bracket_enclosed_type == embedding_type {
                type_ = embedding_type;
            } else if bidi_it.bracket_enclosed_type == BidiType::StrongL
                || bidi_it.bracket_enclosed_type == BidiType::StrongR
            {
                let mut prev_type_for_neutral = bidi_it.prev_for_neutral.type_;
                if prev_type_for_neutral == BidiType::UnknownBt {
                    prev_type_for_neutral = embedding_type;
                }
                match prev_type_for_neutral {
                    BidiType::StrongR | BidiType::WeakEn | BidiType::WeakAn => {
                        type_ = if bidi_it.bracket_enclosed_type == BidiType::StrongR {
                            BidiType::StrongR
                        } else {
                            embedding_type
                        };
                    }
                    BidiType::StrongL => {
                        type_ = if bidi_it.bracket_enclosed_type == BidiType::StrongL {
                            BidiType::StrongL
                        } else {
                            embedding_type
                        };
                    }
                    _ => {}
                }
            }
            if type_ != BidiType::NeutralOn {
                let idx = self.bidi.cache_search(bidi_it.bracket_pairing_pos, -1, 1);
                if idx < self.bidi.cache_start {
                    std::process::abort();
                }
                self.bidi.entry_mut(idx).type_ = type_;
            }
        }
        Ok(type_)
    }

    fn bidi_resolve_neutral(&mut self, bidi_it: &mut BidiIt) -> LispResult<BidiType> {
        let mut type_ = self.bidi_resolve_brackets(bidi_it)?;
        let current_level = bidi_it.current_stack_level();
        let is_neutral = bidi_get_category(type_) == BidiCategory::Neutral;
        if (type_ != BidiType::NeutralB && (is_neutral || bidi_isolate_fmt_char(type_)))
            || type_ == BidiType::WeakBn
        {
            if bidi_it.next_for_neutral.type_ != BidiType::UnknownBt
                && (bidi_it.next_for_neutral.charpos > bidi_it.charpos
                    || (bidi_it.next_for_neutral.charpos == bidi_it.charpos
                        && bidi_it.type_ == BidiType::Pdi))
            {
                type_ = bidi_resolve_neutral_1(
                    bidi_it.prev_for_neutral.type_,
                    bidi_it.next_for_neutral.type_,
                    current_level,
                );
            } else if current_level == 0
                && bidi_it.prev_for_neutral.type_ == BidiType::StrongL
                && (character::ascii_char_p(bidi_it.ch as u32)
                    || (type_ != BidiType::WeakBn
                        && !self.bidi_explicit_dir_char(bidi_it.ch)
                        && !bidi_isolate_fmt_char(type_)))
            {
                type_ = bidi_resolve_neutral_1(
                    bidi_it.prev_for_neutral.type_,
                    BidiType::StrongL,
                    current_level,
                );
            } else if current_level == 1
                && bidi_it.stack_level(0) == 1
                && matches!(
                    bidi_it.prev_for_neutral.type_,
                    BidiType::StrongR | BidiType::WeakEn | BidiType::WeakAn
                )
                && type_ != BidiType::WeakBn
                && !self.bidi_explicit_dir_char(bidi_it.ch)
                && !bidi_isolate_fmt_char(type_)
            {
                type_ = bidi_resolve_neutral_1(
                    bidi_it.prev_for_neutral.type_,
                    BidiType::StrongR,
                    current_level,
                );
            } else {
                let mut adjacent_to_neutrals = is_neutral;
                let mut saved_it = bidi_it.clone();
                loop {
                    self.bidi
                        .cache_iterator_state(bidi_it, type_ == BidiType::NeutralB, false);
                    let old_sidx = bidi_it.stack_idx;
                    type_ = self.bidi_resolve_brackets(bidi_it)?;
                    let new_sidx = bidi_it.stack_idx;
                    if bidi_it.stack_level(new_sidx) > current_level
                        && (bidi_it.isolate_status(new_sidx)
                            || (new_sidx > old_sidx + 1 && bidi_it.isolate_status(new_sidx - 1)))
                    {
                        while bidi_it.current_stack_level() > current_level {
                            self.bidi.cache_iterator_state(
                                bidi_it,
                                type_ == BidiType::NeutralB,
                                false,
                            );
                            type_ = self.bidi_resolve_brackets(bidi_it)?;
                        }
                    }
                    if !adjacent_to_neutrals
                        && (bidi_get_category(type_) == BidiCategory::Neutral
                            || bidi_isolate_fmt_char(type_))
                    {
                        adjacent_to_neutrals = true;
                    }
                    if type_ == BidiType::NeutralB
                        || (type_ != BidiType::WeakBn
                            && bidi_get_category(type_) != BidiCategory::Neutral
                            && !bidi_isolate_fmt_char(type_))
                        || bidi_it.current_stack_level() != current_level
                    {
                        break;
                    }
                }
                saved_it.next_for_neutral = bidi_remember_char(bidi_it, true);
                let next_type;
                if bidi_it.current_stack_level() != current_level || type_ == BidiType::NeutralB {
                    if adjacent_to_neutrals {
                        next_type = bidi_it.prev_for_neutral.type_;
                    } else {
                        bidi_copy_it(bidi_it, &saved_it);
                        return Ok(bidi_it.type_);
                    }
                } else {
                    next_type = match type_ {
                        BidiType::StrongL | BidiType::StrongR | BidiType::StrongAl => type_,
                        BidiType::WeakEn | BidiType::WeakAn => BidiType::StrongR,
                        _ => std::process::abort(),
                    };
                }
                type_ = bidi_resolve_neutral_1(
                    saved_it.prev_for_neutral.type_,
                    next_type,
                    current_level,
                );
                saved_it.next_for_neutral.type_ = next_type;
                saved_it.type_ = type_;
                bidi_copy_it(bidi_it, &saved_it);
            }
        }
        Ok(type_)
    }

    fn bidi_type_of_next_char(&mut self, bidi_it: &mut BidiIt) -> LispResult<BidiType> {
        if bidi_it.scan_dir != 1 {
            std::process::abort();
        }
        self.bidi_resolve_neutral(bidi_it)
    }

    fn bidi_level_of_next_char(&mut self, bidi_it: &mut BidiIt) -> LispResult<i32> {
        let mut type_ = BidiType::UnknownBt;
        if bidi_it.scan_dir == 1 {
            let eob = self.bidi_eob(bidi_it);
            if bidi_it.charpos >= eob {
                return Ok(i32::from(bidi_it.resolved_level));
            }
        }
        if self.bidi.cache_idx > self.bidi.cache_start && !bidi_it.first_elt {
            let bob = if bidi_it.string_p() { 0 } else { 1 };
            let mut next_char_pos = -2;
            if bidi_it.scan_dir > 0 {
                if bidi_it.nchars <= 0 {
                    std::process::abort();
                }
                next_char_pos = bidi_it.charpos + bidi_it.nchars;
            } else if bidi_it.charpos >= bob {
                next_char_pos = bidi_it.charpos - 1;
            }
            if next_char_pos >= bob - 1 {
                type_ = self.bidi.cache_find(next_char_pos, true, bidi_it);
            }
            if type_ != BidiType::UnknownBt {
                return Ok(i32::from(bidi_it.resolved_level));
            }
        }
        if bidi_it.scan_dir == -1 {
            std::process::abort();
        }
        if type_ == BidiType::UnknownBt {
            type_ = self.bidi_type_of_next_char(bidi_it)?;
        }
        if type_ == BidiType::NeutralB {
            return Ok(i32::from(bidi_it.resolved_level));
        }
        let mut level = bidi_it.current_stack_level();
        bidi_it.type_ = type_;
        if (bidi_it.orig_type == BidiType::NeutralWs
            || (bidi_it.orig_type == BidiType::WeakBn && level != bidi_it.stack_level(0))
            || bidi_isolate_fmt_char(bidi_it.orig_type))
            && bidi_it.next_for_ws.charpos < bidi_it.charpos
        {
            let mut clen = bidi_it.ch_len;
            let mut bpos = bidi_it.bytepos;
            let mut cpos = bidi_it.charpos;
            let mut disp_pos = bidi_it.disp_pos;
            let mut nc = bidi_it.nchars;
            let bs = bidi_it.string.clone();
            let fwp = bidi_it.frame_window_p;
            let mut dpp = bidi_it.disp_prop;
            if bidi_it.nchars <= 0 {
                std::process::abort();
            }
            let mut chtype;
            loop {
                cpos += nc;
                bpos += clen;
                let fetched =
                    self.bidi_fetch_char(cpos, bpos, &mut disp_pos, &mut dpp, &bs, bidi_it.w, fwp)?;
                clen = fetched.ch_len;
                nc = fetched.nchars;
                chtype = self.bidi_get_type(fetched.ch, BidiDir::NeutralDir);
                if !(chtype == BidiType::NeutralWs
                    || chtype == BidiType::WeakBn
                    || bidi_isolate_fmt_char(chtype)
                    || self.bidi_explicit_dir_char(fetched.ch))
                {
                    break;
                }
            }
            bidi_it.next_for_ws.type_ = chtype;
            bidi_it.next_for_ws.charpos = cpos;
        }
        self.bidi.cache_iterator_state(bidi_it, true, true);
        if bidi_it.orig_type == BidiType::NeutralB
            || bidi_it.orig_type == BidiType::NeutralS
            || bidi_it.ch == 10
            || bidi_it.ch == BIDI_EOB
            || ((bidi_it.orig_type == BidiType::NeutralWs
                || bidi_it.orig_type == BidiType::WeakBn
                || bidi_isolate_fmt_char(bidi_it.orig_type)
                || self.bidi_explicit_dir_char(bidi_it.ch))
                && (bidi_it.next_for_ws.type_ == BidiType::NeutralB
                    || bidi_it.next_for_ws.type_ == BidiType::NeutralS))
        {
            level = bidi_it.stack_level(0);
        } else if level & 1 == 0 {
            if type_ == BidiType::StrongR {
                level += 1;
            } else if type_ == BidiType::WeakEn || type_ == BidiType::WeakAn {
                level += 2;
            }
        } else if type_ == BidiType::StrongL
            || type_ == BidiType::WeakEn
            || type_ == BidiType::WeakAn
        {
            level += 1;
        }
        bidi_it.resolved_level = level as i8;
        Ok(level)
    }

    fn bidi_find_other_level_edge(
        &mut self,
        bidi_it: &mut BidiIt,
        level: i32,
        end_flag: bool,
    ) -> LispResult<()> {
        let dir = if end_flag {
            -bidi_it.scan_dir
        } else {
            bidi_it.scan_dir
        };
        let idx = self.bidi.cache_find_level_change(level, dir, end_flag);
        if idx >= self.bidi.cache_start {
            self.bidi.cache_fetch_state(idx, bidi_it);
        } else {
            let pos0 = bidi_it.charpos;
            if end_flag {
                std::process::abort();
            }
            if !self.bidi.cache_iterator_state(bidi_it, true, false) {
                std::process::abort();
            }
            loop {
                let mut new_level = self.bidi_level_of_next_char(bidi_it)?;
                if !self.bidi.cache_iterator_state(bidi_it, true, false) {
                    new_level = level - 1;
                    let last = self.bidi.cache_idx - 1;
                    self.bidi.cache_fetch_state(last, bidi_it);
                    bidi_it.resolved_level = new_level as i8;
                    self.bidi.cache_iterator_state(bidi_it, true, true);
                }
                if new_level < level {
                    break;
                }
            }
            if self.int_var(sym::MAX_REDISPLAY_TICKS) > 0 && bidi_it.charpos > pos0 {
                self.update_redisplay_ticks((bidi_it.charpos - pos0) / 50 + 1, bidi_it.w)?;
            }
        }
        Ok(())
    }

    pub fn bidi_move_to_visually_next(&mut self, bidi_it: &mut BidiIt) -> LispResult<()> {
        if bidi_it.charpos < 0 || bidi_it.bytepos < 0 {
            std::process::abort();
        }
        if bidi_it.scan_dir == 0 {
            bidi_it.scan_dir = 1;
        }
        if !bidi_it.first_elt && (bidi_it.ch == 10 || bidi_it.ch == BIDI_EOB) {
            self.bidi_line_init(bidi_it);
        }
        if self.bidi.cache_idx == self.bidi.cache_start {
            let mut sentinel = bidi_it.clone();
            if bidi_it.first_elt {
                sentinel.charpos -= 1;
                sentinel.bytepos -= 1;
                sentinel.ch = 10;
                sentinel.ch_len = 1;
                sentinel.nchars = 1;
            }
            self.bidi.cache_iterator_state(&sentinel, true, false);
        }
        let old_level = i32::from(bidi_it.resolved_level);
        let new_level = self.bidi_level_of_next_char(bidi_it)?;
        if new_level != old_level {
            let ascending = new_level > old_level;
            let mut level_to_search = if ascending { old_level + 1 } else { old_level };
            let incr = if ascending { 1 } else { -1 };
            let mut expected_next_level = old_level + incr;
            self.bidi_find_other_level_edge(bidi_it, level_to_search, !ascending)?;
            bidi_it.scan_dir = -bidi_it.scan_dir;
            let mut next_level = self.bidi.peek_at_next_level(bidi_it);
            while next_level != expected_next_level {
                expected_next_level += incr;
                level_to_search += incr;
                self.bidi_find_other_level_edge(bidi_it, level_to_search, !ascending)?;
                bidi_it.scan_dir = -bidi_it.scan_dir;
                next_level = self.bidi.peek_at_next_level(bidi_it);
            }
            self.bidi_level_of_next_char(bidi_it)?;
        }
        if bidi_it.scan_dir == 1 && (bidi_it.ch == 10 || bidi_it.ch == BIDI_EOB) {
            if bidi_it.string_p() {
                bidi_it.separator_limit = bidi_it.string.schars;
            } else if bidi_it.bytepos < self.cur().zv_byte as i64 {
                let sep_len = self.bidi_at_paragraph_end(
                    bidi_it.charpos + bidi_it.nchars,
                    bidi_it.bytepos + bidi_it.ch_len,
                )?;
                if bidi_it.nchars <= 0 {
                    std::process::abort();
                }
                if sep_len >= 0 {
                    bidi_it.new_paragraph = true;
                    bidi_it.separator_limit = if sep_len > 0 {
                        bidi_it.charpos + bidi_it.nchars + sep_len
                    } else {
                        bidi_it.charpos
                    };
                }
            }
        }
        if bidi_it.scan_dir == 1 && self.bidi.cache_idx > self.bidi.cache_start {
            let last = self.bidi.entry(self.bidi.cache_idx - 1);
            let (last_charpos, last_nchars) = (last.charpos, last.nchars);
            let base = i32::from(bidi_it.resolved_level) == bidi_it.stack_level(0);
            if base
                && (bidi_it.charpos > last_charpos + last_nchars - 1
                    || (self.bidi.cache_idx >= self.bidi.cache_size()
                        && bidi_it.charpos == last_charpos))
            {
                self.bidi.cache_reset();
            } else {
                self.bidi.cache_iterator_state(bidi_it, true, false);
            }
        }
        Ok(())
    }

    pub fn bidi_find_first_overridden(&mut self, bidi_it: &mut BidiIt) -> LispResult<i64> {
        let eob = if bidi_it.string.lstring.is_string() {
            bidi_it.string.schars
        } else {
            self.cur().zv as i64
        };
        let mut found_pos = eob;
        let max_l2r = if bidi_it.paragraph_dir == BidiDir::L2R {
            0
        } else {
            2
        };
        let max_r2l = 1;
        let max_weak = if bidi_it.paragraph_dir == BidiDir::L2R {
            1
        } else {
            2
        };
        loop {
            let type_ = self.bidi_resolve_weak(bidi_it)?;
            let level = bidi_it.current_stack_level();
            let category = bidi_get_category(bidi_it.orig_type);
            let orig = bidi_it.orig_type;
            if (type_ == BidiType::StrongR && orig == BidiType::StrongL)
                || (type_ == BidiType::StrongL
                    && (orig == BidiType::StrongR || orig == BidiType::StrongAl))
                || ((orig == BidiType::StrongL || orig == BidiType::WeakEn) && level > max_l2r)
                || ((orig == BidiType::StrongR || orig == BidiType::StrongAl) && level > max_r2l)
                || ((category == BidiCategory::Weak || orig == BidiType::NeutralOn)
                    && level > max_weak)
            {
                found_pos = bidi_it.charpos;
            }
            if !(found_pos == eob
                && bidi_it.charpos < eob
                && bidi_it.ch != BIDI_EOB
                && bidi_it.ch != 10)
            {
                break;
            }
        }
        Ok(found_pos)
    }
}
