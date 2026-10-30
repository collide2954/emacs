use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::regex::{self, Env, Pattern, Registers, Text};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::syntax::{S_SYMBOL, S_WORD};
use crate::value::{MOST_NEGATIVE_FIXNUM, MOST_POSITIVE_FIXNUM, Value};

const REGEXP_CACHE_SIZE: usize = 20;

pub struct RegexpCacheEntry {
    pub regexp: Value,
    pub whitespace: Value,
    pub syntax_table: Value,
    pub posix: bool,
    pub pattern: Pattern,
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("looking-at", 1, 2, f_looking_at),
    Subr::fixed("posix-looking-at", 1, 2, f_posix_looking_at),
    Subr::fixed("string-match", 2, 4, f_string_match),
    Subr::fixed("posix-string-match", 2, 4, f_posix_string_match),
    Subr::fixed("search-forward", 1, 4, f_search_forward).interactive("MSearch: "),
    Subr::fixed("search-backward", 1, 4, f_search_backward).interactive("MSearch backward: "),
    Subr::fixed("re-search-forward", 1, 4, f_re_search_forward).interactive("sRE search: "),
    Subr::fixed("re-search-backward", 1, 4, f_re_search_backward)
        .interactive("sRE search backward: "),
    Subr::fixed("posix-search-forward", 1, 4, f_posix_search_forward)
        .interactive("sPosix search: "),
    Subr::fixed("posix-search-backward", 1, 4, f_posix_search_backward)
        .interactive("sPosix search backward: "),
    Subr::fixed("replace-match", 1, 5, f_replace_match),
    Subr::fixed("match-beginning", 1, 1, f_match_beginning),
    Subr::fixed("match-end", 1, 1, f_match_end),
    Subr::fixed("match-data", 0, 3, f_match_data),
    Subr::fixed("set-match-data", 1, 2, f_set_match_data),
    Subr::fixed("match-data--translate", 1, 1, f_match_data_translate),
    Subr::fixed("regexp-quote", 1, 1, f_regexp_quote),
    Subr::fixed("newline-cache-check", 0, 1, f_newline_cache_check),
    Subr::fixed("re--describe-compiled", 1, 2, f_re_describe_compiled),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::SEARCH_SPACES_REGEXP, Value::NIL);
    lisp.defvar(sym::INHIBIT_CHANGING_MATCH_DATA, Value::NIL);
}

fn trivial_regexp_p(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'.' | b'*' | b'+' | b'?' | b'[' | b'^' | b'$' => return false,
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    return false;
                }
                if matches!(
                    bytes[i],
                    b'|' | b'('
                        | b')'
                        | b'`'
                        | b'\''
                        | b'b'
                        | b'B'
                        | b'<'
                        | b'>'
                        | b'w'
                        | b'W'
                        | b's'
                        | b'S'
                        | b'='
                        | b'{'
                        | b'}'
                        | b'_'
                        | b'c'
                        | b'C'
                        | b'1'..=b'9'
                ) {
                    return false;
                }
            }
            _ => {}
        }
        i += 1;
    }
    true
}

impl Lisp {
    pub fn regex_env(&self, point: Option<usize>) -> Env<'_> {
        let b = self.cur();
        Env {
            lisp: self,
            syntax_table: b.syntax_table,
            category_table: b.category_table,
            downcase_table: b.downcase_table,
            upcase_table: b.upcase_table,
            unicode_category_table: self.bound_var(sym::UNICODE_CATEGORY_TABLE),
            char_script_table: self.bound_var(sym::CHAR_SCRIPT_TABLE),
            word_combining: self.bound_var(sym::WORD_COMBINING_CATEGORIES),
            word_separating: self.bound_var(sym::WORD_SEPARATING_CATEGORIES),
            point,
        }
    }

    pub fn record_unwind_save_match_data(&mut self) -> LispResult<()> {
        let data = self.match_data(Value::NIL, Value::NIL, Value::NIL)?;
        self.record_unwind(restore_match_data_unwind, data);
        Ok(())
    }

    pub fn fast_looking_at(
        &mut self,
        regexp: Value,
        pos: usize,
        limit: usize,
        string: Value,
    ) -> LispResult<isize> {
        let multibyte = if string.is_string() {
            self.string_multibyte(string)
        } else {
            self.cur().text.multibyte
        };
        let idx = self.compile_pattern(regexp, Value::NIL, false, multibyte)?;
        let mut pattern = self.take_pattern(idx);
        let len = if string.is_string() {
            let pos_byte = self.string_char_to_byte_index(string, pos);
            let limit_byte = self.string_char_to_byte_index(string, limit);
            let env = self.regex_env(None);
            let bytes = self.string_bytes(string);
            regex::re_match(
                &env,
                &mut pattern,
                Text::single(bytes),
                pos_byte,
                None,
                limit_byte,
            )
        } else {
            let b = self.cur();
            let pos_byte = b.text.char_to_byte(pos) - b.begv_byte;
            let limit_byte = b.text.char_to_byte(limit) - b.begv_byte;
            let point = b.pt_byte - b.begv_byte;
            let env = self.regex_env(Some(point));
            let text = self.visible_text();
            regex::re_match(&env, &mut pattern, text, pos_byte, None, limit_byte)
        };
        self.restore_pattern(idx, pattern);
        Ok(len)
    }

    pub fn bound_var(&self, symbol: Value) -> Value {
        let v = self.find_symbol_value(symbol);
        if v == sym::UNBOUND { Value::NIL } else { v }
    }

    pub fn visible_text(&self) -> Text<'_> {
        let b = self.cur();
        let (a, rest) = b.text.parts();
        let lo = b.begv_byte - 1;
        let hi = b.zv_byte - 1;
        let alen = a.len();
        let s1: &[u8] = if lo < alen { &a[lo..hi.min(alen)] } else { &[] };
        let s2: &[u8] = if hi > alen {
            &rest[lo.saturating_sub(alen)..hi - alen]
        } else {
            &[]
        };
        Text { s1, s2 }
    }

    pub fn clear_regexp_cache(&mut self) {
        self.regexp_cache.retain(|e| e.syntax_table == sym::T);
    }

    fn case_translate_table(&self) -> Value {
        if self.bound_var(sym::CASE_FOLD_SEARCH).is_nil() {
            Value::NIL
        } else {
            self.cur().case_canon_table
        }
    }

    fn case_inverse_table(&self) -> Value {
        if self.bound_var(sym::CASE_FOLD_SEARCH).is_nil() {
            Value::NIL
        } else {
            self.cur().case_eqv_table
        }
    }

    fn setup_canon_extras(&mut self) {
        let canon = self.cur().case_canon_table;
        let eqv = self.cur().case_eqv_table;
        if self.is_char_table(canon) && self.char_table(canon).extras.len() > 2 {
            self.char_table_mut(canon).extras[2] = eqv;
        }
    }

    fn modify_match_data_p(&self, modify: bool) -> bool {
        modify && self.bound_var(sym::INHIBIT_CHANGING_MATCH_DATA).is_nil()
    }

    pub fn compile_pattern(
        &mut self,
        pattern: Value,
        translate: Value,
        posix: bool,
        multibyte: bool,
    ) -> LispResult<usize> {
        let spaces = self.bound_var(sym::SEARCH_SPACES_REGEXP);
        let spaces = if spaces.is_string() {
            spaces
        } else {
            Value::NIL
        };
        let syntax_table = self.cur().syntax_table;
        let mut found = None;
        for i in 0..self.regexp_cache.len() {
            let e = &self.regexp_cache[i];
            if e.posix != posix
                || e.pattern.translate != translate
                || !(e.syntax_table == sym::T || e.syntax_table == syntax_table)
            {
                continue;
            }
            let (regexp, whitespace) = (e.regexp, e.whitespace);
            if self.string_chars(regexp) != self.string_chars(pattern)
                || self.string_multibyte(regexp) != self.string_multibyte(pattern)
                || self.string_bytes(regexp) != self.string_bytes(pattern)
            {
                continue;
            }
            if !self.equal(whitespace, spaces)? {
                continue;
            }
            found = Some(i);
            break;
        }
        match found {
            Some(i) => {
                let entry = self.regexp_cache.remove(i);
                self.regexp_cache.insert(0, entry);
            }
            None => {
                let bytes = self.string_bytes(pattern).to_vec();
                let ws = if spaces.is_string() {
                    Some(self.string_bytes(spaces).to_vec())
                } else {
                    None
                };
                let pat_multibyte = self.string_multibyte(pattern);
                let compiled = {
                    let env = self.regex_env(None);
                    regex::compile(&env, &bytes, pat_multibyte, posix, ws.as_deref(), translate)
                };
                let compiled = match compiled {
                    Ok(c) => c,
                    Err(msg) => {
                        let s = self.string(msg);
                        return Err(self.signal_list(sym::INVALID_REGEXP, &[s]));
                    }
                };
                let regexp = crate::fns::f_copy_sequence(self, &[pattern])?;
                let entry = RegexpCacheEntry {
                    regexp,
                    whitespace: spaces,
                    syntax_table: if compiled.used_syntax {
                        syntax_table
                    } else {
                        sym::T
                    },
                    posix,
                    pattern: compiled,
                };
                self.regexp_cache.insert(0, entry);
                self.regexp_cache.truncate(REGEXP_CACHE_SIZE);
            }
        }
        self.regexp_cache[0].pattern.target_multibyte = multibyte;
        Ok(0)
    }

    fn take_pattern(&mut self, idx: usize) -> Pattern {
        std::mem::take(&mut self.regexp_cache[idx].pattern)
    }

    fn restore_pattern(&mut self, idx: usize, pattern: Pattern) {
        self.regexp_cache[idx].pattern = pattern;
    }

    fn matcher_overflow(&mut self) -> crate::error::Flow {
        self.error("Stack overflow in regexp matcher")
    }

    pub fn string_byte_to_char_index(&self, string: Value, byte: usize) -> usize {
        if !self.string_multibyte(string) {
            return byte;
        }
        character::chars_in_multibyte(&self.string_bytes(string)[..byte])
    }

    pub fn string_char_to_byte_index(&self, string: Value, chars: usize) -> usize {
        if !self.string_multibyte(string) {
            return chars;
        }
        character::char_offset_to_byte(self.string_bytes(string), chars)
    }

    fn buffer_regs_to_chars(&mut self, regs: &mut Registers) {
        let begv_byte = self.cur().begv_byte;
        for i in 0..regs.start.len() {
            if regs.start[i] >= 0 {
                regs.start[i] = self
                    .cur()
                    .text
                    .byte_to_char(regs.start[i] as usize + begv_byte)
                    as isize;
                regs.end[i] = self
                    .cur()
                    .text
                    .byte_to_char(regs.end[i] as usize + begv_byte)
                    as isize;
            }
        }
    }

    pub fn looking_at_1(&mut self, regexp: Value, posix: bool, modify: bool) -> LispResult {
        self.setup_canon_extras();
        self.check_string(regexp)?;
        let modify = self.modify_match_data_p(modify);
        let translate = self.case_translate_table();
        let multibyte = self.cur().text.multibyte;
        let idx = self.compile_pattern(regexp, translate, posix, multibyte)?;
        let mut pattern = self.take_pattern(idx);
        let mut regs = std::mem::take(&mut self.search_regs);
        let i = {
            let b = self.cur();
            let pos = b.pt_byte - b.begv_byte;
            let stop = b.zv_byte - b.begv_byte;
            let env = self.regex_env(Some(pos));
            let text = self.visible_text();
            regex::re_match(
                &env,
                &mut pattern,
                text,
                pos,
                if modify { Some(&mut regs) } else { None },
                stop,
            )
        };
        self.restore_pattern(idx, pattern);
        if i == -2 {
            self.search_regs = regs;
            return Err(self.matcher_overflow());
        }
        if modify && i >= 0 {
            self.buffer_regs_to_chars(&mut regs);
            self.last_thing_searched = self.current_buffer;
        }
        self.search_regs = regs;
        Ok(self.bool(i >= 0))
    }

    pub fn string_match_1(
        &mut self,
        regexp: Value,
        string: Value,
        start: Value,
        posix: bool,
        modify: bool,
    ) -> LispResult {
        let modify = self.modify_match_data_p(modify);
        self.check_string(regexp)?;
        self.check_string(string)?;
        let len = self.string_chars(string) as i64;
        let pos_byte = if start.is_nil() {
            0
        } else {
            let mut pos = self.check_fixnum(start)?;
            if pos < 0 && -pos <= len {
                pos += len;
            } else if !(0..=len).contains(&pos) {
                return Err(self.args_out_of_range(string, start));
            }
            self.string_char_to_byte_index(string, pos as usize)
        };
        self.setup_canon_extras();
        let translate = self.case_translate_table();
        let multibyte = self.string_multibyte(string);
        let idx = self.compile_pattern(regexp, translate, posix, multibyte)?;
        let mut pattern = self.take_pattern(idx);
        let mut regs = std::mem::take(&mut self.search_regs);
        let val = {
            let env = self.regex_env(None);
            let bytes = self.string_bytes(string);
            let size = bytes.len();
            regex::re_search(
                &env,
                &mut pattern,
                Text::single(bytes),
                pos_byte,
                size as isize - pos_byte as isize,
                if modify { Some(&mut regs) } else { None },
                size,
            )
        };
        self.restore_pattern(idx, pattern);
        if modify {
            self.last_thing_searched = sym::T;
        }
        if val == -2 {
            self.search_regs = regs;
            return Err(self.matcher_overflow());
        }
        if val < 0 {
            self.search_regs = regs;
            return Ok(Value::NIL);
        }
        if modify {
            for i in 0..regs.start.len() {
                if regs.start[i] >= 0 {
                    regs.start[i] =
                        self.string_byte_to_char_index(string, regs.start[i] as usize) as isize;
                    regs.end[i] =
                        self.string_byte_to_char_index(string, regs.end[i] as usize) as isize;
                }
            }
        }
        self.search_regs = regs;
        Ok(Value::fixnum(
            self.string_byte_to_char_index(string, val as usize) as i64,
        ))
    }

    pub fn fast_string_match(
        &mut self,
        regexp: Value,
        string: Value,
        table: Value,
    ) -> LispResult<isize> {
        let multibyte = self.string_multibyte(string);
        let idx = self.compile_pattern(regexp, table, false, multibyte)?;
        let mut pattern = self.take_pattern(idx);
        let val = {
            let env = self.regex_env(None);
            let bytes = self.string_bytes(string);
            let size = bytes.len();
            regex::re_search(
                &env,
                &mut pattern,
                Text::single(bytes),
                0,
                size as isize,
                None,
                size,
            )
        };
        self.restore_pattern(idx, pattern);
        Ok(val)
    }

    pub fn fast_c_string_match_internal(
        &mut self,
        regexp: Value,
        string: &[u8],
        table: Value,
    ) -> LispResult<isize> {
        let regexp = crate::fns::f_string_make_unibyte(self, &[regexp])?;
        let idx = self.compile_pattern(regexp, table, false, false)?;
        let mut pattern = self.take_pattern(idx);
        let val = {
            let env = self.regex_env(None);
            let size = string.len();
            regex::re_search(
                &env,
                &mut pattern,
                Text::single(string),
                0,
                size as isize,
                None,
                size,
            )
        };
        self.restore_pattern(idx, pattern);
        Ok(val)
    }

    pub fn fast_c_string_match_ignore_case(
        &mut self,
        regexp: Value,
        string: &[u8],
    ) -> LispResult<isize> {
        let table = self.ascii_canon_table;
        self.fast_c_string_match_internal(regexp, string, table)
    }

    fn set_search_regs(&mut self, beg_byte: usize, nbytes: usize) {
        if !self.bound_var(sym::INHIBIT_CHANGING_MATCH_DATA).is_nil() {
            return;
        }
        if self.search_regs.start.is_empty() {
            self.search_regs.start = vec![-1; 2];
            self.search_regs.end = vec![-1; 2];
        }
        for i in 1..self.search_regs.start.len() {
            self.search_regs.start[i] = -1;
            self.search_regs.end[i] = -1;
        }
        let start = self.cur().text.byte_to_char(beg_byte) as isize;
        let end = self.cur().text.byte_to_char(beg_byte + nbytes) as isize;
        self.search_regs.start[0] = start;
        self.search_regs.end[0] = end;
        self.last_thing_searched = self.current_buffer;
    }

    fn char_table_translate(&self, trt: Value, c: u32) -> u32 {
        if trt.is_nil() || !self.is_char_table(trt) {
            return c;
        }
        match self.char_table_ref(trt, c).as_fixnum() {
            Some(n) => n as u32,
            None => c,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn search_buffer(
        &mut self,
        string: Value,
        pos: usize,
        pos_byte: usize,
        lim: usize,
        lim_byte: usize,
        n: i64,
        re: bool,
        trt: Value,
        posix: bool,
    ) -> LispResult<i64> {
        if n == 0 || self.string_chars(string) == 0 {
            self.set_search_regs(pos_byte, 0);
            return Ok(pos as i64);
        }
        let trivial = trivial_regexp_p(self.string_bytes(string))
            && self.bound_var(sym::SEARCH_SPACES_REGEXP).is_nil();
        if re && !trivial {
            self.search_buffer_re(string, pos, pos_byte, lim, lim_byte, n, trt, posix)
        } else {
            Ok(self.search_buffer_non_re(string, pos, pos_byte, lim, lim_byte, n, re, trt))
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn search_buffer_re(
        &mut self,
        string: Value,
        mut pos: usize,
        mut pos_byte: usize,
        _lim: usize,
        lim_byte: usize,
        mut n: i64,
        trt: Value,
        posix: bool,
    ) -> LispResult<i64> {
        let preserve = self.bound_var(sym::INHIBIT_CHANGING_MATCH_DATA).is_nil();
        let multibyte = self.cur().text.multibyte;
        let idx = self.compile_pattern(string, trt, posix, multibyte)?;
        let mut pattern = self.take_pattern(idx);
        let mut regs = if preserve {
            std::mem::take(&mut self.search_regs)
        } else {
            Registers::default()
        };
        let begv_byte = self.cur().begv_byte;
        let mut result = None;
        while n < 0 {
            let val = {
                let pt = self.cur().pt_byte - begv_byte;
                let env = self.regex_env(Some(pt));
                let text = self.visible_text();
                regex::re_search(
                    &env,
                    &mut pattern,
                    text,
                    pos_byte - begv_byte,
                    lim_byte as isize - pos_byte as isize,
                    Some(&mut regs),
                    pos_byte - begv_byte,
                )
            };
            if val == -2 {
                result = Some(Err(()));
                break;
            }
            if val >= 0 {
                pos_byte = regs.start[0] as usize + begv_byte;
                if preserve {
                    self.buffer_regs_to_chars(&mut regs);
                    self.last_thing_searched = self.current_buffer;
                    pos = regs.start[0] as usize;
                } else {
                    pos = self.cur().text.byte_to_char(pos_byte);
                }
            } else {
                result = Some(Ok(n));
                break;
            }
            n += 1;
        }
        while result.is_none() && n > 0 {
            let val = {
                let pt = self.cur().pt_byte - begv_byte;
                let env = self.regex_env(Some(pt));
                let text = self.visible_text();
                regex::re_search(
                    &env,
                    &mut pattern,
                    text,
                    pos_byte - begv_byte,
                    lim_byte as isize - pos_byte as isize,
                    Some(&mut regs),
                    lim_byte - begv_byte,
                )
            };
            if val == -2 {
                result = Some(Err(()));
                break;
            }
            if val >= 0 {
                pos_byte = regs.end[0] as usize + begv_byte;
                if preserve {
                    self.buffer_regs_to_chars(&mut regs);
                    self.last_thing_searched = self.current_buffer;
                    pos = regs.end[0] as usize;
                } else {
                    pos = self.cur().text.byte_to_char(pos_byte);
                }
            } else {
                result = Some(Ok(-n));
                break;
            }
            n -= 1;
        }
        self.restore_pattern(idx, pattern);
        if preserve {
            self.search_regs = regs;
        }
        match result {
            Some(Err(())) => Err(self.matcher_overflow()),
            Some(Ok(v)) => Ok(v),
            None => Ok(pos as i64),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn search_buffer_non_re(
        &mut self,
        string: Value,
        pos: usize,
        pos_byte: usize,
        lim: usize,
        lim_byte: usize,
        n: i64,
        re: bool,
        trt: Value,
    ) -> i64 {
        let multibyte = self.cur().text.multibyte;
        let str_multibyte = self.string_multibyte(string);
        let raw: Vec<u32> = if multibyte == str_multibyte {
            self.string_char_vec(string)
        } else if multibyte {
            self.string_bytes(string)
                .iter()
                .map(|&b| {
                    if b < 0x80 {
                        b as u32
                    } else {
                        character::byte8_to_char(b)
                    }
                })
                .collect()
        } else {
            self.string_char_vec(string)
                .into_iter()
                .map(|c| if c < 0x80 { c } else { c & 0xFF })
                .collect()
        };
        let mut pat: Vec<u8> = Vec::new();
        let mut len = 0usize;
        let mut i = 0;
        while i < raw.len() {
            if re && raw[i] == b'\\' as u32 {
                i += 1;
                if i >= raw.len() {
                    break;
                }
            }
            let translated = self.char_table_translate(trt, raw[i]);
            if multibyte {
                character::push_char(&mut pat, translated);
            } else {
                pat.push(translated as u8);
            }
            len += 1;
            i += 1;
        }
        self.simple_search(n, &pat, len, trt, pos, pos_byte, lim, lim_byte)
    }

    #[allow(clippy::too_many_arguments)]
    fn simple_search(
        &mut self,
        mut n: i64,
        pat: &[u8],
        len: usize,
        trt: Value,
        mut pos: usize,
        mut pos_byte: usize,
        lim: usize,
        lim_byte: usize,
    ) -> i64 {
        let multibyte = self.cur().text.multibyte;
        let forward = n > 0;
        let len_byte = pat.len();
        let mut match_byte = 0usize;
        let mut stopped = false;
        if lim > pos && multibyte {
            while n > 0 {
                loop {
                    if pos + len > lim || pos_byte + len_byte > lim_byte {
                        stopped = true;
                        break;
                    }
                    let mut this_pos_byte = pos_byte;
                    let mut this_len = len;
                    let mut p = 0;
                    while this_len > 0 {
                        let (pat_ch, charlen) = character::string_char(&pat[p..]);
                        let text = &self.cur().text;
                        let buf_ch = text.char_at_byte(this_pos_byte);
                        let buf_len = text.char_len_at_byte(this_pos_byte);
                        if self.char_table_translate(trt, buf_ch) != pat_ch {
                            break;
                        }
                        this_len -= 1;
                        p += charlen;
                        this_pos_byte += buf_len;
                    }
                    if this_len == 0 {
                        match_byte = this_pos_byte - pos_byte;
                        pos += len;
                        pos_byte += match_byte;
                        break;
                    }
                    pos_byte += self.cur().text.char_len_at_byte(pos_byte);
                    pos += 1;
                }
                if stopped {
                    break;
                }
                n -= 1;
            }
        } else if lim > pos {
            while n > 0 {
                loop {
                    if pos + len > lim {
                        stopped = true;
                        break;
                    }
                    let mut this_pos = pos;
                    let mut this_len = len;
                    let mut p = 0;
                    while this_len > 0 {
                        let buf_ch = self.cur().text.char_at_byte(this_pos);
                        if self.char_table_translate(trt, buf_ch) != pat[p] as u32 {
                            break;
                        }
                        p += 1;
                        this_len -= 1;
                        this_pos += 1;
                    }
                    if this_len == 0 {
                        match_byte = len;
                        pos += len;
                        break;
                    }
                    pos += 1;
                }
                if stopped {
                    break;
                }
                n -= 1;
            }
        } else if lim < pos && multibyte {
            while n < 0 {
                loop {
                    if (pos as i64 - len as i64) < lim as i64
                        || (pos_byte as i64 - len_byte as i64) < lim_byte as i64
                    {
                        stopped = true;
                        break;
                    }
                    let mut this_pos = pos;
                    let mut this_pos_byte = pos_byte;
                    let mut this_len = len;
                    let mut p = len_byte;
                    while this_len > 0 {
                        this_pos -= 1;
                        this_pos_byte -= self.cur().text.prev_char_len(this_pos_byte);
                        let mut q = p - 1;
                        while q > 0 && pat[q] & 0xC0 == 0x80 {
                            q -= 1;
                        }
                        p = q;
                        let pat_ch = character::string_char(&pat[p..]).0;
                        let buf_ch = self.cur().text.char_at_byte(this_pos_byte);
                        if self.char_table_translate(trt, buf_ch) != pat_ch {
                            break;
                        }
                        this_len -= 1;
                    }
                    if this_len == 0 {
                        match_byte = pos_byte - this_pos_byte;
                        pos = this_pos;
                        pos_byte = this_pos_byte;
                        break;
                    }
                    pos_byte -= self.cur().text.prev_char_len(pos_byte);
                    pos -= 1;
                }
                if stopped {
                    break;
                }
                n += 1;
            }
        } else if lim < pos {
            while n < 0 {
                loop {
                    if (pos as i64 - len as i64) < lim as i64 {
                        stopped = true;
                        break;
                    }
                    let mut this_pos = pos - len;
                    let mut this_len = len;
                    let mut p = 0;
                    while this_len > 0 {
                        let buf_ch = self.cur().text.char_at_byte(this_pos);
                        if self.char_table_translate(trt, buf_ch) != pat[p] as u32 {
                            break;
                        }
                        p += 1;
                        this_len -= 1;
                        this_pos += 1;
                    }
                    if this_len == 0 {
                        match_byte = len;
                        pos -= len;
                        break;
                    }
                    pos -= 1;
                }
                if stopped {
                    break;
                }
                n += 1;
            }
        }
        if n == 0 {
            let base = if multibyte { pos_byte } else { pos };
            if forward {
                self.set_search_regs(base - match_byte, match_byte);
            } else {
                self.set_search_regs(base, match_byte);
            }
            pos as i64
        } else if n > 0 {
            -n
        } else {
            n
        }
    }

    pub(crate) fn fix_position(&mut self, pos: Value) -> LispResult<i64> {
        if let Some(n) = pos.as_fixnum() {
            return Ok(n);
        }
        if self.is_marker(pos) {
            return Ok(self.marker_position_checked(pos)? as i64);
        }
        if !self.is_bignum(pos) {
            return Err(self.wrong_type(sym::INTEGER_OR_MARKER_P, pos));
        }
        let negative = self
            .bignum_ref(pos)
            .is_some_and(|b| b.sign() == num_bigint::Sign::Minus);
        Ok(if negative {
            MOST_NEGATIVE_FIXNUM
        } else {
            MOST_POSITIVE_FIXNUM
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn search_command(
        &mut self,
        string: Value,
        bound: Value,
        noerror: Value,
        count: Value,
        direction: i64,
        re: bool,
        posix: bool,
    ) -> LispResult {
        let mut n = direction;
        if !count.is_nil() {
            n *= self.check_fixnum(count)?;
        }
        self.check_string(string)?;
        let (pt, begv, zv) = {
            let b = self.cur();
            (b.pt, b.begv, b.zv)
        };
        let (lim, lim_byte) = if bound.is_nil() {
            if n > 0 {
                (zv, self.cur().zv_byte)
            } else {
                (begv, self.cur().begv_byte)
            }
        } else {
            let lim = self.fix_position(bound)?;
            if if n > 0 {
                lim < pt as i64
            } else {
                lim > pt as i64
            } {
                return Err(self.error("Invalid search bound (wrong side of point)"));
            }
            if lim > zv as i64 {
                (zv, self.cur().zv_byte)
            } else if lim < begv as i64 {
                (begv, self.cur().begv_byte)
            } else {
                let lim = lim as usize;
                (lim, self.cur().text.char_to_byte(lim))
            }
        };
        self.setup_canon_extras();
        let trt = self.case_translate_table();
        let _ = self.case_inverse_table();
        let pt_byte = self.cur().pt_byte;
        let np = self.search_buffer(string, pt, pt_byte, lim, lim_byte, n, re, trt, posix)?;
        if np <= 0 {
            if noerror.is_nil() {
                return Err(self.signal_list(sym::SEARCH_FAILED, &[string]));
            }
            if noerror != sym::T {
                self.goto_char_internal(lim);
            }
            return Ok(Value::NIL);
        }
        self.goto_char_internal(np as usize);
        Ok(Value::fixnum(np))
    }

    fn buffer_chars(&self, from: usize, to: usize) -> Vec<u32> {
        let text = &self.cur().text;
        let mut out = Vec::with_capacity(to.saturating_sub(from));
        let mut byte = text.char_to_byte(from);
        for _ in from..to {
            out.push(text.char_at_byte(byte));
            byte += text.char_len_at_byte(byte);
        }
        out
    }

    fn replace_match(
        &mut self,
        mut newtext: Value,
        fixedcase: Value,
        mut literal: Value,
        string: Value,
        subexp: Value,
    ) -> LispResult {
        self.check_string(newtext)?;
        if !string.is_nil() {
            self.check_string(string)?;
        }
        if literal.is_nil() && !self.string_bytes(newtext).contains(&b'\\') {
            literal = sym::T;
        }
        let num_regs = self.search_regs.start.len();
        if num_regs == 0 {
            return Err(self.error("`replace-match' called before any match found"));
        }
        let sub = if subexp.is_nil() {
            0
        } else {
            match subexp.as_fixnum() {
                Some(n) if (0..num_regs as i64).contains(&n) => n as usize,
                _ => {
                    let hi = Value::fixnum(num_regs as i64 - 1);
                    return Err(self.args_out_of_range(subexp, hi));
                }
            }
        };
        let sub_start = self.search_regs.start[sub];
        let sub_end = self.search_regs.end[sub];
        let in_range = if string.is_nil() {
            let b = self.cur();
            b.begv as isize <= sub_start && sub_end <= b.zv as isize
        } else {
            0 <= sub_start && sub_end <= self.string_chars(string) as isize
        };
        if !in_range {
            if sub_start < 0 {
                let msg = self.string("replace-match subexpression does not exist");
                return Err(self.signal_list(sym::ERROR, &[msg, subexp]));
            }
            return Err(self.args_out_of_range(
                Value::fixnum(sub_start as i64),
                Value::fixnum(sub_end as i64),
            ));
        }
        let (sub_start, sub_end) = (sub_start as usize, sub_end as usize);
        #[derive(PartialEq)]
        enum CaseAction {
            NoChange,
            AllCaps,
            CapInitial,
        }
        let mut case_action = CaseAction::NoChange;
        if fixedcase.is_nil() {
            let chars: Vec<u32> = if string.is_nil() {
                let multibyte = self.cur().text.multibyte;
                self.buffer_chars(sub_start, sub_end)
                    .into_iter()
                    .map(|c| {
                        if !multibyte && c >= 0x80 {
                            character::byte8_to_char(c as u8)
                        } else {
                            c
                        }
                    })
                    .collect()
            } else {
                let all = self.string_char_vec(string);
                let multibyte = self.string_multibyte(string);
                all[sub_start..sub_end]
                    .iter()
                    .map(|&c| {
                        if !multibyte && c >= 0x80 {
                            character::byte8_to_char(c as u8)
                        } else {
                            c
                        }
                    })
                    .collect()
            };
            let symbols_as_words = !self.bound_var(sym::CASE_SYMBOLS_AS_WORDS).is_nil();
            let mut prevc = b'\n' as u32;
            let mut some_multiletter_word = false;
            let mut some_lowercase = false;
            let mut some_nonuppercase_initial = false;
            let mut some_uppercase = false;
            for c in chars {
                let prev_syntax = self.syntax_class(prevc);
                let prev_is_word =
                    prev_syntax == S_WORD || (symbols_as_words && prev_syntax == S_SYMBOL);
                if self.lowercasep(c) {
                    some_lowercase = true;
                    if !prev_is_word {
                        some_nonuppercase_initial = true;
                    } else {
                        some_multiletter_word = true;
                    }
                } else if self.uppercasep(c) {
                    some_uppercase = true;
                    if prev_is_word {
                        some_multiletter_word = true;
                    }
                } else if !prev_is_word {
                    some_nonuppercase_initial = true;
                }
                prevc = c;
            }
            case_action = if !some_lowercase && some_multiletter_word {
                CaseAction::AllCaps
            } else if !some_nonuppercase_initial && some_multiletter_word {
                CaseAction::CapInitial
            } else if !some_nonuppercase_initial && some_uppercase {
                CaseAction::AllCaps
            } else {
                CaseAction::NoChange
            };
        }
        if !string.is_nil() {
            let mark = self.stack.len();
            self.stack.push(newtext);
            self.stack.push(string);
            let r = (|| -> LispResult {
                let before = self.substring_chars(string, 0, sub_start);
                self.stack.push(before);
                let total = self.string_chars(string);
                let after = self.substring_chars(string, sub_end, total);
                self.stack.push(after);
                if literal.is_nil() {
                    let text_chars = self.string_char_vec(newtext);
                    let mut pieces: Vec<Value> = Vec::new();
                    let mut lastpos = 0usize;
                    let mut pos = 0usize;
                    while pos < text_chars.len() {
                        let mut substart: Option<(usize, usize)> = None;
                        let mut delbackslash = false;
                        let c = text_chars[pos];
                        pos += 1;
                        if c == b'\\' as u32 {
                            let c = text_chars.get(pos).copied().unwrap_or(0);
                            pos += 1;
                            if c == b'&' as u32 {
                                substart = Some((sub_start, sub_end));
                            } else if (b'1' as u32..=b'9' as u32).contains(&c) {
                                let r = (c - b'0' as u32) as usize;
                                if r < num_regs && self.search_regs.start[r] >= 0 {
                                    substart = Some((
                                        self.search_regs.start[r] as usize,
                                        self.search_regs.end[r] as usize,
                                    ));
                                } else {
                                    substart = Some((0, 0));
                                }
                            } else if c == b'\\' as u32 {
                                delbackslash = true;
                            } else if c != b'?' as u32 {
                                return Err(self.error("Invalid use of `\\' in replacement text"));
                            }
                        }
                        if let Some((s, e)) = substart {
                            if pos - 2 != lastpos {
                                let middle = self.substring_chars(newtext, lastpos, pos - 2);
                                self.stack.push(middle);
                                pieces.push(middle);
                            }
                            let part = self.substring_chars(string, s, e);
                            self.stack.push(part);
                            pieces.push(part);
                            lastpos = pos;
                        } else if delbackslash {
                            let middle = self.substring_chars(newtext, lastpos, pos - 1);
                            self.stack.push(middle);
                            pieces.push(middle);
                            lastpos = pos;
                        }
                    }
                    if pos != lastpos {
                        let middle = self.substring_chars(newtext, lastpos, pos);
                        self.stack.push(middle);
                        pieces.push(middle);
                    }
                    newtext = self.concat_to_string(&pieces)?;
                    self.stack.push(newtext);
                }
                if case_action == CaseAction::AllCaps {
                    let f = self.intern("upcase");
                    newtext = self.funcall(f, &[newtext])?;
                } else if case_action == CaseAction::CapInitial {
                    let f = self.intern("upcase-initials");
                    newtext = self.funcall(f, &[newtext])?;
                }
                self.stack.push(newtext);
                self.concat_to_string(&[before, newtext, after])
            })();
            self.stack.truncate(mark);
            return r;
        }
        let (pt, zv) = (self.cur().pt, self.cur().zv);
        let opoint: i64 = if pt <= sub_start {
            pt as i64
        } else {
            pt.max(sub_end) as i64 - zv as i64
        };
        if literal.is_nil() {
            let buf_multibyte = self.cur().text.multibyte;
            let str_multibyte = self.string_multibyte(newtext);
            let raw: Vec<u32> = if str_multibyte {
                self.string_char_vec(newtext)
            } else {
                self.string_bytes(newtext)
                    .iter()
                    .map(|&b| b as u32)
                    .collect()
            };
            let convert = |c: u32| -> u32 {
                if str_multibyte {
                    if !buf_multibyte {
                        character::char_to_byte8(c) as u32
                    } else {
                        c
                    }
                } else if buf_multibyte && c >= 0x80 {
                    character::byte8_to_char(c as u8)
                } else {
                    c
                }
            };
            let mut out: Vec<u32> = Vec::new();
            let mut really_changed = false;
            let mut i = 0;
            while i < raw.len() {
                let c = convert(raw[i]);
                i += 1;
                if c == b'\\' as u32 {
                    really_changed = true;
                    let raw_c = raw.get(i).copied().unwrap_or(0);
                    i += 1;
                    let c = if str_multibyte {
                        if !buf_multibyte && raw_c >= 0x80 {
                            character::char_to_byte8(raw_c) as u32
                        } else {
                            raw_c
                        }
                    } else if buf_multibyte && raw_c >= 0x80 {
                        character::byte8_to_char(raw_c as u8)
                    } else {
                        raw_c
                    };
                    let mut idx: Option<usize> = None;
                    if c == b'&' as u32 {
                        idx = Some(sub);
                    } else if (b'1' as u32..=b'9' as u32).contains(&c)
                        && ((c - b'0' as u32) as usize) < num_regs
                    {
                        let r = (c - b'0' as u32) as usize;
                        if self.search_regs.start[r] >= 1 {
                            idx = Some(r);
                        }
                    } else if c == b'\\' as u32 {
                        out.push(b'\\' as u32);
                    } else {
                        return Err(self.error("Invalid use of `\\' in replacement text"));
                    }
                    if let Some(r) = idx {
                        let s = self.search_regs.start[r] as usize;
                        let e = self.search_regs.end[r] as usize;
                        out.extend(self.buffer_chars(s, e));
                    }
                } else {
                    out.push(c);
                }
            }
            if really_changed {
                newtext = self.string_from_chars(&out, buf_multibyte);
            }
        }
        let newlen = self.string_chars(newtext);
        let newpoint = sub_start + newlen;
        self.stack.push(newtext);
        let r = self.replace_range_chars(sub_start, sub_end, newtext);
        self.stack.pop();
        r?;
        self.update_search_regs(sub_start, sub_end, sub_start + newlen);
        if case_action == CaseAction::AllCaps {
            let start = Value::fixnum(self.search_regs.start[sub] as i64);
            let f = self.intern("upcase-region");
            self.funcall(f, &[start, Value::fixnum(newpoint as i64), Value::NIL])?;
        } else if case_action == CaseAction::CapInitial {
            let start = Value::fixnum(self.search_regs.start[sub] as i64);
            let f = self.intern("upcase-initials-region");
            self.funcall(f, &[start, Value::fixnum(newpoint as i64), Value::NIL])?;
        }
        let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
        let restore = (opoint + if opoint <= 0 { zv } else { 0 }).clamp(begv, zv);
        self.goto_char_internal(restore as usize);
        self.goto_char_internal(newpoint);
        Ok(Value::NIL)
    }

    pub fn update_search_regs(&mut self, oldstart: usize, oldend: usize, newend: usize) {
        let change = newend as isize - oldend as isize;
        let (oldstart, oldend) = (oldstart as isize, oldend as isize);
        for i in 0..self.search_regs.start.len() {
            let start = self.search_regs.start[i];
            if start <= oldstart {
            } else if start >= oldend {
                self.search_regs.start[i] += change;
            } else {
                self.search_regs.start[i] = oldstart;
            }
            let end = self.search_regs.end[i];
            if end >= oldend {
                self.search_regs.end[i] += change;
            } else if end > oldstart {
                self.search_regs.end[i] = oldstart;
            }
        }
    }

    fn match_limit(&mut self, num: Value, beginning: bool) -> LispResult {
        let n = self.check_fixnum(num)?;
        if n < 0 {
            return Err(self.args_out_of_range(num, Value::fixnum(0)));
        }
        if self.search_regs.start.is_empty() {
            return Err(self.error("No match data, because no search succeeded"));
        }
        let n = n as usize;
        if n >= self.search_regs.start.len() || self.search_regs.start[n] < 0 {
            return Ok(Value::NIL);
        }
        Ok(Value::fixnum(if beginning {
            self.search_regs.start[n] as i64
        } else {
            self.search_regs.end[n] as i64
        }))
    }

    pub fn match_data(&mut self, integers: Value, reuse: Value, reseat: Value) -> LispResult {
        if !reseat.is_nil() {
            let mut tail = reuse;
            while tail.is_cons() {
                let item = self.xcar(tail);
                if self.is_marker(item) {
                    self.unchain_marker(item);
                    self.heap.set_car(tail, Value::NIL);
                }
                tail = self.xcdr(tail);
            }
        }
        if self.last_thing_searched.is_nil() {
            return Ok(Value::NIL);
        }
        let last = self.last_thing_searched;
        let base = self.stack.len();
        let mut data: Vec<Value> = Vec::new();
        let mut len = 0usize;
        for i in 0..self.search_regs.start.len() {
            let start = self.search_regs.start[i];
            if start >= 0 {
                let end = self.search_regs.end[i];
                if last == sym::T || !integers.is_nil() {
                    data.push(Value::fixnum(start as i64));
                    data.push(Value::fixnum(end as i64));
                } else {
                    let m1 = self.make_marker_object();
                    self.stack.push(m1);
                    self.set_marker(m1, Value::fixnum(start as i64), last, false)?;
                    let m2 = self.make_marker_object();
                    self.stack.push(m2);
                    self.set_marker(m2, Value::fixnum(end as i64), last, false)?;
                    data.push(m1);
                    data.push(m2);
                }
                len = 2 * i + 2;
            } else {
                data.push(Value::NIL);
                data.push(Value::NIL);
            }
        }
        data.truncate(len);
        if self.is_buffer(last) && !integers.is_nil() {
            data.push(last);
            len += 1;
        }
        let result = if !reuse.is_cons() {
            self.list(&data[..len])
        } else {
            let mut i = 0;
            let mut tail = reuse;
            let mut prev = Value::NIL;
            while tail.is_cons() {
                self.heap
                    .set_car(tail, if i < len { data[i] } else { Value::NIL });
                prev = tail;
                tail = self.xcdr(tail);
                i += 1;
            }
            if i < len {
                let rest = self.list(&data[i..len]);
                self.heap.set_cdr(prev, rest);
            }
            reuse
        };
        self.stack.truncate(base);
        Ok(result)
    }

    pub fn set_match_data(&mut self, mut list: Value, reseat: Value) -> LispResult {
        self.check_list(list)?;
        self.last_thing_searched = sym::T;
        let length = self.list_length_unchecked(list) / 2;
        if length > self.search_regs.start.len() {
            self.search_regs.start.resize(length, -1);
            self.search_regs.end.resize(length, -1);
        }
        let mut i = 0;
        while list.is_cons() {
            let marker = self.xcar(list);
            if self.is_buffer(marker) {
                self.last_thing_searched = marker;
                break;
            }
            if i >= length {
                break;
            }
            if marker.is_nil() {
                self.search_regs.start[i] = -1;
                list = self.xcdr(list);
            } else {
                let m = marker;
                let from = self.coerce_match_marker(marker, true)?;
                if !reseat.is_nil() && self.is_marker(m) {
                    self.unchain_marker(m);
                    self.heap.set_car(list, Value::NIL);
                }
                list = self.xcdr(list);
                if !list.is_cons() {
                    break;
                }
                let m = self.xcar(list);
                let to = self.coerce_match_marker(m, false)?;
                self.search_regs.start[i] = from as isize;
                self.search_regs.end[i] = to as isize;
                if !reseat.is_nil() && self.is_marker(m) {
                    self.unchain_marker(m);
                    self.heap.set_car(list, Value::NIL);
                }
            }
            list = self.xcdr(list);
            i += 1;
        }
        while i < self.search_regs.start.len() {
            self.search_regs.start[i] = -1;
            i += 1;
        }
        Ok(Value::NIL)
    }

    fn coerce_match_marker(&mut self, marker: Value, set_last: bool) -> LispResult<i64> {
        if self.is_marker(marker) {
            let buffer = self.marker(marker).buffer;
            if buffer.is_nil() {
                return Ok(0);
            }
            if set_last {
                self.last_thing_searched = buffer;
            }
            return Ok(self.marker(marker).charpos as i64);
        }
        self.check_fixnum(marker)
    }
}

fn restore_match_data_unwind(lisp: &mut Lisp, data: Value) -> LispResult<()> {
    lisp.set_match_data(data, Value::NIL)?;
    Ok(())
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn f_looking_at(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.looking_at_1(args[0], false, arg(args, 1).is_nil())
}

fn f_posix_looking_at(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.looking_at_1(args[0], true, arg(args, 1).is_nil())
}

fn f_string_match(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.string_match_1(args[0], args[1], arg(args, 2), false, arg(args, 3).is_nil())
}

fn f_posix_string_match(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.string_match_1(args[0], args[1], arg(args, 2), true, arg(args, 3).is_nil())
}

fn f_search_forward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.search_command(
        args[0],
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        1,
        false,
        false,
    )
}

fn f_search_backward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.search_command(
        args[0],
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        -1,
        false,
        false,
    )
}

fn f_re_search_forward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.search_command(
        args[0],
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        1,
        true,
        false,
    )
}

fn f_re_search_backward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.search_command(
        args[0],
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        -1,
        true,
        false,
    )
}

fn f_posix_search_forward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.search_command(
        args[0],
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        1,
        true,
        true,
    )
}

fn f_posix_search_backward(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.search_command(
        args[0],
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        -1,
        true,
        true,
    )
}

fn f_replace_match(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.replace_match(
        args[0],
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        arg(args, 4),
    )
}

fn f_match_beginning(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.match_limit(args[0], true)
}

fn f_match_end(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.match_limit(args[0], false)
}

fn f_match_data(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.match_data(arg(args, 0), arg(args, 1), arg(args, 2))
}

fn f_set_match_data(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.set_match_data(args[0], arg(args, 1))
}

fn f_match_data_translate(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let delta = lisp.check_fixnum(args[0])? as isize;
    if !lisp.last_thing_searched.is_nil() {
        for i in 0..lisp.search_regs.start.len() {
            if lisp.search_regs.start[i] >= 0 {
                lisp.search_regs.start[i] = (lisp.search_regs.start[i] + delta).max(0);
                lisp.search_regs.end[i] = (lisp.search_regs.end[i] + delta).max(0);
            }
        }
    }
    Ok(Value::NIL)
}

fn f_regexp_quote(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let string = lisp.check_string(args[0])?;
    let bytes = lisp.string_bytes(string);
    let mut out = Vec::with_capacity(bytes.len() * 2);
    let mut added = 0;
    for &b in bytes {
        if matches!(b, b'[' | b'*' | b'.' | b'\\' | b'?' | b'+' | b'^' | b'$') {
            out.push(b'\\');
            added += 1;
        }
        out.push(b);
    }
    if added == 0 {
        return Ok(string);
    }
    let chars = lisp.string_chars(string) + added;
    let multibyte = lisp.string_multibyte(string);
    Ok(lisp.make_specified_string(out, chars, multibyte))
}

fn f_newline_cache_check(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = arg(args, 0);
    if !buffer.is_nil() {
        lisp.check_buffer(buffer)?;
    }
    Ok(Value::NIL)
}

fn f_re_describe_compiled(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let regexp = lisp.check_string(args[0])?;
    let translate = lisp.case_translate_table();
    let multibyte = lisp.cur().text.multibyte;
    let idx = lisp.compile_pattern(regexp, translate, false, multibyte)?;
    if !arg(args, 1).is_nil() {
        let bytes = lisp.regexp_cache[idx].pattern.buffer.clone();
        return Ok(lisp.unibyte_string(&bytes));
    }
    Err(lisp.error("Not available: rebuild with --enable-checking"))
}
