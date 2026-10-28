mod api;
mod cjk;
mod engine;
mod iso2022;
mod mule;
mod simple;
mod utf;

pub(crate) use api::f_find_operation_coding_system;
pub use engine::{DetectionInfo, EOL_SEEN_CR, EOL_SEEN_CRLF, EOL_SEEN_LF, EOL_SEEN_NONE};

use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::Weakness;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const ARG_NAME: usize = 0;
pub const ARG_MNEMONIC: usize = 1;
pub const ARG_CODING_TYPE: usize = 2;
pub const ARG_CHARSET_LIST: usize = 3;
pub const ARG_ASCII_COMPATIBLE_P: usize = 4;
pub const ARG_DECODE_TRANSLATION_TABLE: usize = 5;
pub const ARG_ENCODE_TRANSLATION_TABLE: usize = 6;
pub const ARG_POST_READ_CONVERSION: usize = 7;
pub const ARG_PRE_WRITE_CONVERSION: usize = 8;
pub const ARG_DEFAULT_CHAR: usize = 9;
pub const ARG_FOR_UNIBYTE: usize = 10;
pub const ARG_PLIST: usize = 11;
pub const ARG_EOL_TYPE: usize = 12;
pub const ARG_MAX: usize = 13;
const ARG_ISO2022_INITIAL: usize = ARG_MAX;
const ARG_ISO2022_REG_USAGE: usize = ARG_MAX + 1;
const ARG_ISO2022_REQUEST: usize = ARG_MAX + 2;
const ARG_ISO2022_FLAGS: usize = ARG_MAX + 3;
const ARG_ISO2022_MAX: usize = ARG_MAX + 4;
const ARG_UTF8_BOM: usize = ARG_MAX;
const ARG_UTF8_MAX: usize = ARG_MAX + 1;
const ARG_UTF16_BOM: usize = ARG_MAX;
const ARG_UTF16_ENDIAN: usize = ARG_MAX + 1;
const ARG_UTF16_MAX: usize = ARG_MAX + 2;
const ARG_CCL_DECODER: usize = ARG_MAX;
const ARG_CCL_ENCODER: usize = ARG_MAX + 1;
const ARG_CCL_VALIDS: usize = ARG_MAX + 2;
const ARG_CCL_MAX: usize = ARG_MAX + 3;
const ARG_UNDECIDED_INHIBIT_NBD: usize = ARG_MAX;
const ARG_UNDECIDED_INHIBIT_IED: usize = ARG_MAX + 1;
const ARG_UNDECIDED_PREFER_UTF_8: usize = ARG_MAX + 2;
const ARG_UNDECIDED_MAX: usize = ARG_MAX + 3;

pub const ATTR_BASE_NAME: usize = 0;
pub const ATTR_DOCSTRING: usize = 1;
pub const ATTR_MNEMONIC: usize = 2;
pub const ATTR_TYPE: usize = 3;
pub const ATTR_CHARSET_LIST: usize = 4;
pub const ATTR_ASCII_COMPAT: usize = 5;
pub const ATTR_DECODE_TBL: usize = 6;
pub const ATTR_ENCODE_TBL: usize = 7;
pub const ATTR_TRANS_TBL: usize = 8;
pub const ATTR_POST_READ: usize = 9;
pub const ATTR_PRE_WRITE: usize = 10;
pub const ATTR_DEFAULT_CHAR: usize = 11;
pub const ATTR_FOR_UNIBYTE: usize = 12;
pub const ATTR_PLIST: usize = 13;
pub const ATTR_CATEGORY: usize = 14;
pub const ATTR_SAFE_CHARSETS: usize = 15;
pub const ATTR_CHARSET_VALIDS: usize = 16;
pub const ATTR_CCL_DECODER: usize = 17;
pub const ATTR_CCL_ENCODER: usize = 18;
pub const ATTR_CCL_VALIDS: usize = 19;
pub const ATTR_ISO_INITIAL: usize = 20;
pub const ATTR_ISO_USAGE: usize = 21;
pub const ATTR_ISO_REQUEST: usize = 22;
pub const ATTR_ISO_FLAGS: usize = 23;
pub const ATTR_UTF_BOM: usize = 24;
pub const ATTR_UTF_16_ENDIAN: usize = 25;
pub const ATTR_EMACS_MULE_FULL: usize = 26;
pub const ATTR_UNDECIDED_INHIBIT_NBD: usize = 27;
pub const ATTR_UNDECIDED_INHIBIT_IED: usize = 28;
pub const ATTR_UNDECIDED_PREFER_UTF_8: usize = 29;
pub const ATTR_LAST: usize = 30;

pub const CAT_ISO_7: usize = 0;
pub const CAT_ISO_7_TIGHT: usize = 1;
pub const CAT_ISO_8_1: usize = 2;
pub const CAT_ISO_8_2: usize = 3;
pub const CAT_ISO_7_ELSE: usize = 4;
pub const CAT_ISO_8_ELSE: usize = 5;
pub const CAT_UTF_8_AUTO: usize = 6;
pub const CAT_UTF_8_NOSIG: usize = 7;
pub const CAT_UTF_8_SIG: usize = 8;
pub const CAT_UTF_16_AUTO: usize = 9;
pub const CAT_UTF_16_BE: usize = 10;
pub const CAT_UTF_16_LE: usize = 11;
pub const CAT_UTF_16_BE_NOSIG: usize = 12;
pub const CAT_UTF_16_LE_NOSIG: usize = 13;
pub const CAT_CHARSET: usize = 14;
pub const CAT_SJIS: usize = 15;
pub const CAT_BIG5: usize = 16;
pub const CAT_CCL: usize = 17;
pub const CAT_EMACS_MULE: usize = 18;
pub const CAT_RAW_TEXT: usize = 19;
pub const CAT_UNDECIDED: usize = 20;
pub const CAT_MAX: usize = 21;

const CATEGORY_NAMES: [&str; CAT_MAX] = [
    "coding-category-iso-7",
    "coding-category-iso-7-tight",
    "coding-category-iso-8-1",
    "coding-category-iso-8-2",
    "coding-category-iso-7-else",
    "coding-category-iso-8-else",
    "coding-category-utf-8-auto",
    "coding-category-utf-8",
    "coding-category-utf-8-sig",
    "coding-category-utf-16-auto",
    "coding-category-utf-16-be",
    "coding-category-utf-16-le",
    "coding-category-utf-16-be-nosig",
    "coding-category-utf-16-le-nosig",
    "coding-category-charset",
    "coding-category-sjis",
    "coding-category-big5",
    "coding-category-ccl",
    "coding-category-emacs-mule",
    "coding-category-raw-text",
    "coding-category-undecided",
];

pub const ISO_FLAG_LONG_FORM: i64 = 0x0001;
pub const ISO_FLAG_RESET_AT_EOL: i64 = 0x0002;
pub const ISO_FLAG_RESET_AT_CNTL: i64 = 0x0004;
pub const ISO_FLAG_SEVEN_BITS: i64 = 0x0008;
pub const ISO_FLAG_LOCKING_SHIFT: i64 = 0x0010;
pub const ISO_FLAG_SINGLE_SHIFT: i64 = 0x0020;
pub const ISO_FLAG_DESIGNATION: i64 = 0x0040;
pub const ISO_FLAG_REVISION: i64 = 0x0080;
pub const ISO_FLAG_DIRECTION: i64 = 0x0100;
pub const ISO_FLAG_INIT_AT_BOL: i64 = 0x0200;
pub const ISO_FLAG_DESIGNATE_AT_BOL: i64 = 0x0400;
pub const ISO_FLAG_SAFE: i64 = 0x0800;
pub const ISO_FLAG_LATIN_EXTRA: i64 = 0x1000;
pub const ISO_FLAG_COMPOSITION: i64 = 0x2000;
pub const ISO_FLAG_USE_ROMAN: i64 = 0x8000;
pub const ISO_FLAG_USE_OLDJIS: i64 = 0x10000;
pub const ISO_FLAG_LEVEL_4: i64 = 0x20000;
pub const ISO_FLAG_FULL_SUPPORT: i64 = 0x100000;

pub const ANNOTATE_COMPOSITION_MASK: u32 = 0x0001;
pub const ANNOTATE_CHARSET_MASK: u32 = 0x0003;
pub const FOR_UNIBYTE_MASK: u32 = 0x0100;
pub const REQUIRE_FLUSHING_MASK: u32 = 0x0200;
pub const REQUIRE_DECODING_MASK: u32 = 0x0400;
pub const REQUIRE_ENCODING_MASK: u32 = 0x0800;
pub const REQUIRE_DETECTION_MASK: u32 = 0x1000;

pub const MODE_LAST_BLOCK: u32 = 0x01;
pub const MODE_SELECTIVE_DISPLAY: u32 = 0x02;
pub const MODE_DIRECTION: u32 = 0x04;
pub const MODE_SAFE_ENCODING: u32 = 0x10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodingType {
    Undecided,
    Iso2022,
    Charset,
    Utf8,
    Utf16,
    Ccl,
    EmacsMule,
    ShiftJis,
    Big5,
    RawText,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bom {
    Detect,
    Without,
    With,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodingResult {
    Success,
    InsufficientSrc,
    InsufficientDst,
    InvalidSrc,
    Interrupt,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ComposingState {
    No,
    Char,
    Rule,
    ComponentChar,
    ComponentRule,
}

pub const COMPOSITION_RELATIVE: i32 = 0;
pub const COMPOSITION_WITH_RULE: i32 = 1;
pub const COMPOSITION_WITH_ALTCHARS: i32 = 2;
pub const COMPOSITION_WITH_RULE_ALTCHARS: i32 = 3;
pub const COMPOSITION_NO: i32 = 4;
pub const MAX_COMPOSITION_COMPONENTS: usize = 16;
pub const CMP_CARRYOVER_LEN: usize =
    4 + MAX_COMPOSITION_COMPONENTS * 3 - 2 + 2 + MAX_COMPOSITION_COMPONENTS;

#[derive(Clone, Debug)]
pub struct CompositionStatus {
    pub state: ComposingState,
    pub method: i32,
    pub old_form: bool,
    pub length: i32,
    pub nchars: i32,
    pub ncomps: i32,
    pub carryover: [i32; CMP_CARRYOVER_LEN],
}

impl Default for CompositionStatus {
    fn default() -> Self {
        CompositionStatus {
            state: ComposingState::No,
            method: COMPOSITION_NO,
            old_form: false,
            length: 0,
            nchars: 0,
            ncomps: 0,
            carryover: [0; CMP_CARRYOVER_LEN],
        }
    }
}

#[derive(Clone, Debug)]
pub struct CodingSystem {
    pub chars_at_source: bool,
    pub raw_destination: bool,
    pub annotated: bool,
    pub insert_before_markers: bool,
    pub result: CodingResult,
    pub detected_utf8_bytes: i64,
    pub detected_utf8_chars: i64,
    pub produced: usize,
    pub produced_char: usize,
    pub consumed: usize,
    pub consumed_char: usize,
    pub src_pos: i64,
    pub src_pos_byte: i64,
    pub src_chars: usize,
    pub src_bytes: usize,
    pub src_object: Value,
    pub source: Vec<u8>,
    pub dst_pos: usize,
    pub dst_pos_byte: usize,
    pub dst_object: Value,
    pub destination: Vec<u8>,
    pub charbuf: Vec<i32>,
    pub charbuf_size: usize,
    pub carryover: Vec<u8>,
    pub ccl: Option<Box<crate::ccl::CclProgram>>,
    pub cmp_status: CompositionStatus,
    pub id: i64,
    pub name: Value,
    pub kind: CodingType,
    pub common_flags: u32,
    pub mode: u32,
    pub eol_type: Value,
    pub max_charset_id: i64,
    pub safe_charsets: Vec<u8>,
    pub default_char: i64,
    pub iso_flags: i64,
    pub iso_invocation: [i32; 2],
    pub iso_designation: [i32; 4],
    pub iso_single_shifting: bool,
    pub iso_bol: bool,
    pub iso_extsegment_len: i64,
    pub iso_embedded_utf_8: bool,
    pub utf_8_bom: Bom,
    pub utf_16_bom: Bom,
    pub utf_16_big_endian: bool,
    pub utf_16_surrogate: i32,
    pub undecided_inhibit_nbd: i32,
    pub undecided_inhibit_ied: i32,
    pub undecided_prefer_utf_8: bool,
    pub src_multibyte: bool,
    pub dst_multibyte: bool,
    pub eol_seen: u32,
    pub head_ascii: i64,
}

impl CodingSystem {
    pub fn unset() -> CodingSystem {
        CodingSystem {
            chars_at_source: false,
            raw_destination: false,
            annotated: false,
            insert_before_markers: false,
            result: CodingResult::Success,
            detected_utf8_bytes: -1,
            detected_utf8_chars: -1,
            produced: 0,
            produced_char: 0,
            consumed: 0,
            consumed_char: 0,
            src_pos: 0,
            src_pos_byte: 0,
            src_chars: 0,
            src_bytes: 0,
            src_object: Value::NIL,
            source: Vec::new(),
            dst_pos: 0,
            dst_pos_byte: 0,
            dst_object: Value::NIL,
            destination: Vec::new(),
            charbuf: Vec::new(),
            charbuf_size: 0,
            carryover: Vec::new(),
            ccl: None,
            cmp_status: CompositionStatus::default(),
            id: -1,
            name: Value::NIL,
            kind: CodingType::RawText,
            common_flags: 0,
            mode: 0,
            eol_type: Value::NIL,
            max_charset_id: -1,
            safe_charsets: Vec::new(),
            default_char: b' ' as i64,
            iso_flags: 0,
            iso_invocation: [0, -1],
            iso_designation: [-1; 4],
            iso_single_shifting: false,
            iso_bol: true,
            iso_extsegment_len: 0,
            iso_embedded_utf_8: false,
            utf_8_bom: Bom::Without,
            utf_16_bom: Bom::Without,
            utf_16_big_endian: true,
            utf_16_surrogate: 0,
            undecided_inhibit_nbd: -1,
            undecided_inhibit_ied: -1,
            undecided_prefer_utf_8: false,
            src_multibyte: false,
            dst_multibyte: false,
            eol_seen: 0,
            head_ascii: -1,
        }
    }

    pub fn require_decoding(&self) -> bool {
        self.dst_multibyte || self.common_flags & REQUIRE_DECODING_MASK != 0
    }

    pub fn require_encoding(&self) -> bool {
        self.src_multibyte
            || self.common_flags & REQUIRE_ENCODING_MASK != 0
            || self.mode & MODE_SELECTIVE_DISPLAY != 0
    }

    pub fn require_detection(&self) -> bool {
        self.common_flags & REQUIRE_DETECTION_MASK != 0
    }
}

pub struct Codings {
    pub hash_table: Value,
    pub category_table: Value,
    pub categories: Vec<CodingSystem>,
    pub priorities: [usize; CAT_MAX],
    pub sjis_coding_system: Value,
    pub big5_coding_system: Value,
    pub safe_terminal: CodingSystem,
    pub terminal: CodingSystem,
    pub keyboard: CodingSystem,
    pub workbuf_name: Value,
    pub reused_workbuf: Value,
    pub reused_workbuf_in_use: bool,
}

impl Default for Codings {
    fn default() -> Self {
        let mut priorities = [0; CAT_MAX];
        for (i, p) in priorities.iter_mut().enumerate() {
            *p = i;
        }
        Codings {
            hash_table: Value::NIL,
            category_table: Value::NIL,
            categories: vec![CodingSystem::unset(); CAT_MAX],
            priorities,
            sjis_coding_system: Value::NIL,
            big5_coding_system: Value::NIL,
            safe_terminal: CodingSystem::unset(),
            terminal: CodingSystem::unset(),
            keyboard: CodingSystem::unset(),
            workbuf_name: Value::NIL,
            reused_workbuf: Value::NIL,
            reused_workbuf_in_use: false,
        }
    }
}

impl Codings {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.push(self.hash_table);
        out.push(self.category_table);
        out.push(self.sjis_coding_system);
        out.push(self.big5_coding_system);
        out.push(self.workbuf_name);
        out.push(self.reused_workbuf);
        for c in self
            .categories
            .iter()
            .chain([&self.safe_terminal, &self.terminal, &self.keyboard])
        {
            out.push(c.name);
            out.push(c.eol_type);
        }
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("coding-system-p", 1, 1, f_coding_system_p),
    Subr::fixed(
        "read-non-nil-coding-system",
        1,
        1,
        f_read_non_nil_coding_system,
    ),
    Subr::fixed("read-coding-system", 1, 2, f_read_coding_system),
    Subr::fixed("check-coding-system", 1, 1, f_check_coding_system),
    Subr::fixed(
        "set-terminal-coding-system-internal",
        1,
        2,
        f_set_terminal_coding_system_internal,
    ),
    Subr::fixed(
        "set-safe-terminal-coding-system-internal",
        1,
        1,
        f_set_safe_terminal_coding_system_internal,
    ),
    Subr::fixed("terminal-coding-system", 0, 1, f_terminal_coding_system),
    Subr::fixed(
        "set-keyboard-coding-system-internal",
        1,
        2,
        f_set_keyboard_coding_system_internal,
    ),
    Subr::fixed("keyboard-coding-system", 0, 1, f_keyboard_coding_system),
    Subr::many(
        "set-coding-system-priority",
        0,
        f_set_coding_system_priority,
    ),
    Subr::fixed(
        "coding-system-priority-list",
        0,
        1,
        f_coding_system_priority_list,
    ),
    Subr::many(
        "define-coding-system-internal",
        ARG_MAX as u16,
        f_define_coding_system_internal,
    ),
    Subr::fixed("coding-system-put", 3, 3, f_coding_system_put),
    Subr::fixed(
        "define-coding-system-alias",
        2,
        2,
        f_define_coding_system_alias,
    ),
    Subr::fixed("coding-system-base", 1, 1, f_coding_system_base),
    Subr::fixed("coding-system-plist", 1, 1, f_coding_system_plist),
    Subr::fixed("coding-system-aliases", 1, 1, f_coding_system_aliases),
    Subr::fixed("coding-system-eol-type", 1, 1, f_coding_system_eol_type),
    Subr::fixed("detect-coding-region", 2, 3, api::f_detect_coding_region),
    Subr::fixed("detect-coding-string", 1, 2, api::f_detect_coding_string),
    Subr::fixed(
        "find-coding-systems-region-internal",
        2,
        3,
        api::f_find_coding_systems_region_internal,
    ),
    Subr::fixed(
        "unencodable-char-position",
        3,
        5,
        api::f_unencodable_char_position,
    ),
    Subr::fixed(
        "check-coding-systems-region",
        3,
        3,
        api::f_check_coding_systems_region,
    ),
    Subr::fixed("decode-coding-region", 3, 4, api::f_decode_coding_region),
    Subr::fixed("encode-coding-region", 3, 4, api::f_encode_coding_region),
    Subr::fixed("decode-coding-string", 2, 4, api::f_decode_coding_string),
    Subr::fixed("encode-coding-string", 2, 4, api::f_encode_coding_string),
    Subr::fixed("decode-sjis-char", 1, 1, api::f_decode_sjis_char),
    Subr::fixed("encode-sjis-char", 1, 1, api::f_encode_sjis_char),
    Subr::fixed("decode-big5-char", 1, 1, api::f_decode_big5_char),
    Subr::fixed("encode-big5-char", 1, 1, api::f_encode_big5_char),
    Subr::many(
        "find-operation-coding-system",
        1,
        api::f_find_operation_coding_system,
    ),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.codings.hash_table = lisp.make_hash_table_with(sym::EQ, Weakness::None);
    lisp.codings.workbuf_name = lisp.string(" *code-conversion-work*");
    let names: Vec<Value> = CATEGORY_NAMES.iter().map(|n| lisp.intern(n)).collect();
    lisp.codings.category_table = lisp.vector(names);
    let message = lisp.string("Invalid coding system");
    let conditions = lisp.list(&[sym::CODING_SYSTEM_ERROR, sym::ERROR]);
    lisp.put(sym::CODING_SYSTEM_ERROR, sym::ERROR_CONDITIONS, conditions);
    lisp.put(sym::CODING_SYSTEM_ERROR, sym::ERROR_MESSAGE, message);
    lisp.put(
        sym::TRANSLATION_TABLE,
        sym::CHAR_TABLE_EXTRA_SLOTS,
        Value::fixnum(2),
    );
    for (operation, idx) in [
        (sym::INSERT_FILE_CONTENTS, 0),
        (sym::WRITE_REGION, 2),
        (sym::CALL_PROCESS, 0),
        (sym::CALL_PROCESS_REGION, 2),
        (sym::START_PROCESS, 2),
        (sym::OPEN_NETWORK_STREAM, 3),
    ] {
        lisp.put(operation, sym::TARGET_IDX, Value::fixnum(idx));
    }
    for symbol in [
        sym::CODING_SYSTEM_LIST,
        sym::CODING_SYSTEM_ALIST,
        sym::CODING_CATEGORY_LIST,
        sym::CODING_SYSTEM_FOR_READ,
        sym::CODING_SYSTEM_FOR_WRITE,
        sym::LAST_CODING_SYSTEM_USED,
        sym::LAST_CODE_CONVERSION_ERROR,
        sym::FILE_CODING_SYSTEM_ALIST,
        sym::PROCESS_CODING_SYSTEM_ALIST,
        sym::NETWORK_CODING_SYSTEM_ALIST,
        sym::LOCALE_CODING_SYSTEM,
        sym::STANDARD_TRANSLATION_TABLE_FOR_DECODE,
        sym::STANDARD_TRANSLATION_TABLE_FOR_ENCODE,
        sym::CHARSET_REVISION_TABLE,
        sym::DEFAULT_PROCESS_CODING_SYSTEM,
        sym::SELECT_SAFE_CODING_SYSTEM_FUNCTION,
        sym::TRANSLATION_TABLE_FOR_INPUT,
    ] {
        lisp.defvar(symbol, Value::NIL);
    }
    let mut list = Value::NIL;
    for name in CATEGORY_NAMES.iter().rev() {
        let s = lisp.intern(name);
        list = lisp.cons(s, list);
    }
    lisp.set_plain_value(sym::CODING_CATEGORY_LIST, list);
    lisp.defvar_bool(sym::INHIBIT_EOL_CONVERSION, false);
    lisp.defvar_bool(sym::INHERIT_PROCESS_CODING_SYSTEM, false);
    for (symbol, text) in [
        (sym::EOL_MNEMONIC_UNIX, ":"),
        (sym::EOL_MNEMONIC_DOS, "\\"),
        (sym::EOL_MNEMONIC_MAC, "/"),
        (sym::EOL_MNEMONIC_UNDECIDED, ":"),
    ] {
        let s = lisp.string(text);
        lisp.defvar(symbol, s);
    }
    lisp.defvar(sym::ENABLE_CHARACTER_TRANSLATION, sym::T);
    let table = lisp.vector(vec![Value::NIL; 256]);
    lisp.defvar(sym::LATIN_EXTRA_CODE_TABLE, table);
    lisp.defvar_bool(sym::CODING_SYSTEM_REQUIRE_WARNING, false);
    lisp.defvar_bool(sym::INHIBIT_ISO_ESCAPE_DETECTION, false);
    lisp.defvar_bool(sym::INHIBIT_NULL_BYTE_DETECTION, false);
    lisp.defvar_bool(sym::DISABLE_ASCII_OPTIMIZATION, false);
    let k_coding_type = lisp.intern(":coding-type");
    let k_for_unibyte = lisp.intern(":for-unibyte");
    let k_docstring = lisp.intern(":docstring");
    let k_eol_type = lisp.intern(":eol-type");
    let k_charset_list = lisp.intern(":charset-list");
    let doc = lisp.string(
        "Do no conversion.\n\nWhen you visit a file with this coding, the file is read into a\nunibyte buffer as is, thus each byte of a file is treated as a\ncharacter.",
    );
    let mut args = vec![Value::NIL; ARG_UNDECIDED_MAX];
    args[ARG_NAME] = sym::NO_CONVERSION;
    args[ARG_MNEMONIC] = Value::fixnum(b'=' as i64);
    args[ARG_CODING_TYPE] = sym::RAW_TEXT;
    args[ARG_ASCII_COMPATIBLE_P] = sym::T;
    args[ARG_DEFAULT_CHAR] = Value::fixnum(0);
    args[ARG_FOR_UNIBYTE] = sym::T;
    args[ARG_EOL_TYPE] = sym::UNIX;
    args[ARG_PLIST] = lisp.list(&[
        sym::K_NAME,
        sym::NO_CONVERSION,
        sym::K_MNEMONIC,
        Value::fixnum(b'=' as i64),
        k_coding_type,
        sym::RAW_TEXT,
        sym::K_ASCII_COMPATIBLE_P,
        sym::T,
        sym::K_DEFAULT_CHAR,
        Value::fixnum(0),
        k_for_unibyte,
        sym::T,
        k_docstring,
        doc,
        k_eol_type,
        sym::UNIX,
    ]);
    f_define_coding_system_internal(lisp, &args[..ARG_MAX]).expect("no-conversion");
    let doc = lisp.string("No conversion on encoding, automatic conversion on decoding.");
    let ascii_list = lisp.list(&[sym::ASCII]);
    args[ARG_NAME] = sym::UNDECIDED;
    args[ARG_MNEMONIC] = Value::fixnum(b'-' as i64);
    args[ARG_CODING_TYPE] = sym::UNDECIDED;
    args[ARG_CHARSET_LIST] = ascii_list;
    args[ARG_FOR_UNIBYTE] = Value::NIL;
    args[ARG_EOL_TYPE] = Value::NIL;
    args[ARG_PLIST] = lisp.list(&[
        sym::K_NAME,
        sym::UNDECIDED,
        sym::K_MNEMONIC,
        Value::fixnum(b'-' as i64),
        k_coding_type,
        sym::UNDECIDED,
        sym::K_ASCII_COMPATIBLE_P,
        sym::T,
        k_charset_list,
        ascii_list,
        k_for_unibyte,
        Value::NIL,
        k_docstring,
        doc,
        k_eol_type,
        Value::NIL,
    ]);
    args[ARG_UNDECIDED_INHIBIT_NBD] = Value::fixnum(0);
    args[ARG_UNDECIDED_INHIBIT_IED] = Value::fixnum(0);
    f_define_coding_system_internal(lisp, &args).expect("undecided");
    let safe = lisp
        .setup_coding_system(sym::NO_CONVERSION)
        .expect("safe terminal");
    lisp.codings.safe_terminal = safe;
    for i in 0..CAT_MAX {
        let s = lisp.vector_items(lisp.codings.category_table)[i];
        lisp.defvar(s, sym::NO_CONVERSION);
    }
}

fn encode_inhibit_flag(flag: Value) -> i32 {
    if flag.is_nil() {
        -1
    } else {
        i32::from(flag == sym::T)
    }
}

impl Lisp {
    pub fn coding_system_spec(&mut self, coding_system: Value) -> LispResult {
        let table = self.codings.hash_table;
        self.gethash(coding_system, table, Value::NIL)
    }

    pub fn coding_system_id(&mut self, coding_system: Value) -> LispResult<i64> {
        let table = self.codings.hash_table;
        Ok(match self.hash_lookup(table, coding_system)? {
            Some(slot) => slot as i64,
            None => -1,
        })
    }

    pub fn coding_id_spec(&self, id: i64) -> Value {
        self.hash_table(self.codings.hash_table).entries[id as usize].map_or(Value::NIL, |(_, v)| v)
    }

    pub fn coding_id_name(&self, id: i64) -> Value {
        self.hash_table(self.codings.hash_table).entries[id as usize].map_or(Value::NIL, |(k, _)| k)
    }

    pub fn coding_id_attrs(&self, id: i64) -> Value {
        self.vector_items(self.coding_id_spec(id))[0]
    }

    pub fn attr(&self, attrs: Value, idx: usize) -> Value {
        self.vector_items(attrs)[idx]
    }

    fn set_attr(&mut self, attrs: Value, idx: usize, val: Value) {
        let _ = self.aset_value(attrs, Value::fixnum(idx as i64), val);
    }

    pub fn coding_system_p(&mut self, object: Value) -> LispResult<bool> {
        if object.is_nil() || self.coding_system_id(object)? >= 0 {
            return Ok(true);
        }
        if !object.is_symbol() || self.get(object, sym::CODING_SYSTEM_DEFINE_FORM).is_nil() {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn check_coding_system(&mut self, coding_system: Value) -> LispResult {
        let define_form = if coding_system.is_symbol() {
            self.get(coding_system, sym::CODING_SYSTEM_DEFINE_FORM)
        } else {
            Value::NIL
        };
        if !define_form.is_nil() {
            self.put(coding_system, sym::CODING_SYSTEM_DEFINE_FORM, Value::NIL);
            self.stack.push(define_form);
            let r = self.condition_case_internal(
                sym::ERROR,
                |l| l.eval(define_form),
                |_, _| Ok(Value::NIL),
            );
            self.stack.pop();
            r?;
        }
        if self.coding_system_p(coding_system)? {
            return Ok(coding_system);
        }
        Err(self.signal_list(sym::CODING_SYSTEM_ERROR, &[coding_system]))
    }

    pub fn check_coding_system_get_spec(&mut self, x: Value) -> LispResult {
        let mut spec = self.coding_system_spec(x)?;
        if spec.is_nil() {
            self.check_coding_system(x)?;
            spec = self.coding_system_spec(x)?;
        }
        if spec.is_nil() {
            return Err(self.wrong_type(sym::CODING_SYSTEM_P, x));
        }
        Ok(spec)
    }

    pub fn check_coding_system_get_id(&mut self, x: Value) -> LispResult<i64> {
        let mut id = self.coding_system_id(x)?;
        if id < 0 {
            self.check_coding_system(x)?;
            id = self.coding_system_id(x)?;
        }
        if id < 0 {
            return Err(self.wrong_type(sym::CODING_SYSTEM_P, x));
        }
        Ok(id)
    }

    pub fn setup_iso_safe_charsets(&mut self, attrs: Value) -> LispResult<()> {
        let flags = self.attr(attrs, ATTR_ISO_FLAGS).as_fixnum().unwrap_or(0);
        let mut charset_list = self.attr(attrs, ATTR_CHARSET_LIST);
        if flags & ISO_FLAG_FULL_SUPPORT != 0 && charset_list != self.charsets.iso_2022_charset_list
        {
            charset_list = self.charsets.iso_2022_charset_list;
            self.set_attr(attrs, ATTR_CHARSET_LIST, charset_list);
            self.set_attr(attrs, ATTR_SAFE_CHARSETS, Value::NIL);
        }
        if self.attr(attrs, ATTR_SAFE_CHARSETS).is_string() {
            return Ok(());
        }
        let mut max_charset_id = 0i64;
        let mut tail = charset_list;
        while tail.is_cons() {
            let id = self.xcar(tail).as_fixnum().unwrap_or(0);
            if max_charset_id < id {
                max_charset_id = id;
            }
            tail = self.xcdr(tail);
        }
        let mut safe = vec![255u8; max_charset_id as usize + 1];
        let request = self.attr(attrs, ATTR_ISO_REQUEST);
        let reg_usage = self.attr(attrs, ATTR_ISO_USAGE);
        let reg94 = self.car_safe(reg_usage).as_fixnum().unwrap_or(0);
        let reg96 = self.cdr_safe(reg_usage).as_fixnum().unwrap_or(0);
        let mut tail = charset_list;
        while tail.is_cons() {
            let id = self.xcar(tail);
            let idn = id.as_fixnum().unwrap_or(0);
            let assoc = self.assq(id, request);
            if assoc.is_cons() {
                safe[idn as usize] = self.xcdr(assoc).as_fixnum().unwrap_or(0) as u8;
            } else if self.charset(idn as i32).iso_chars_96 {
                if reg96 < 4 {
                    safe[idn as usize] = reg96 as u8;
                }
            } else if reg94 < 4 {
                safe[idn as usize] = reg94 as u8;
            }
            tail = self.xcdr(tail);
        }
        let s = self.unibyte_string(&safe);
        self.set_attr(attrs, ATTR_SAFE_CHARSETS, s);
        Ok(())
    }

    pub fn setup_coding_system(&mut self, coding_system: Value) -> LispResult<CodingSystem> {
        let coding_system = if coding_system.is_nil() {
            sym::UNDECIDED
        } else {
            coding_system
        };
        let id = self.check_coding_system_get_id(coding_system)?;
        let attrs = self.coding_id_attrs(id);
        let eol_type = if self.var_true(sym::INHIBIT_EOL_CONVERSION) {
            sym::UNIX
        } else {
            self.vector_items(self.coding_id_spec(id))[2]
        };
        let mut c = CodingSystem::unset();
        c.id = id;
        c.name = self.coding_id_name(id);
        c.eol_type = eol_type;
        c.mode = 0;
        c.common_flags = if self.is_vector(eol_type) {
            REQUIRE_DECODING_MASK | REQUIRE_DETECTION_MASK
        } else if eol_type != sym::UNIX {
            REQUIRE_DECODING_MASK | REQUIRE_ENCODING_MASK
        } else {
            0
        };
        if !self.attr(attrs, ATTR_POST_READ).is_nil() {
            c.common_flags |= REQUIRE_DECODING_MASK;
        }
        if !self.attr(attrs, ATTR_PRE_WRITE).is_nil() {
            c.common_flags |= REQUIRE_ENCODING_MASK;
        }
        if !self.attr(attrs, ATTR_FOR_UNIBYTE).is_nil() {
            c.common_flags |= FOR_UNIBYTE_MASK;
        }
        let safe = self.attr(attrs, ATTR_SAFE_CHARSETS);
        c.safe_charsets = self.string_bytes(safe).to_vec();
        c.max_charset_id = c.safe_charsets.len() as i64 - 1;
        c.default_char = self
            .attr(attrs, ATTR_DEFAULT_CHAR)
            .as_fixnum()
            .unwrap_or(b' ' as i64);
        let coding_type = self.attr(attrs, ATTR_TYPE);
        if coding_type == sym::UNDECIDED {
            c.kind = CodingType::Undecided;
            c.common_flags |= REQUIRE_DETECTION_MASK;
            c.undecided_inhibit_nbd =
                encode_inhibit_flag(self.attr(attrs, ATTR_UNDECIDED_INHIBIT_NBD));
            c.undecided_inhibit_ied =
                encode_inhibit_flag(self.attr(attrs, ATTR_UNDECIDED_INHIBIT_IED));
            c.undecided_prefer_utf_8 = !self.attr(attrs, ATTR_UNDECIDED_PREFER_UTF_8).is_nil();
        } else if coding_type == sym::ISO_2022 {
            c.kind = CodingType::Iso2022;
            let flags = self.attr(attrs, ATTR_ISO_FLAGS).as_fixnum().unwrap_or(0);
            c.iso_invocation = [
                0,
                if flags & ISO_FLAG_SEVEN_BITS != 0 {
                    -1
                } else {
                    1
                },
            ];
            let initial = self.attr(attrs, ATTR_ISO_INITIAL);
            for i in 0..4 {
                c.iso_designation[i] =
                    self.vector_items(initial)[i].as_fixnum().unwrap_or(-1) as i32;
            }
            c.iso_single_shifting = false;
            c.iso_bol = true;
            if flags & ISO_FLAG_SAFE != 0 {
                c.mode |= MODE_SAFE_ENCODING;
            }
            c.common_flags |= REQUIRE_DECODING_MASK | REQUIRE_ENCODING_MASK | REQUIRE_FLUSHING_MASK;
            if flags & ISO_FLAG_COMPOSITION != 0 {
                c.common_flags |= ANNOTATE_COMPOSITION_MASK;
            }
            if flags & ISO_FLAG_DESIGNATION != 0 {
                c.common_flags |= ANNOTATE_CHARSET_MASK;
            }
            if flags & ISO_FLAG_FULL_SUPPORT != 0 {
                self.setup_iso_safe_charsets(attrs)?;
                let safe = self.attr(attrs, ATTR_SAFE_CHARSETS);
                c.safe_charsets = self.string_bytes(safe).to_vec();
                c.max_charset_id = c.safe_charsets.len() as i64 - 1;
            }
            c.iso_flags = flags;
            c.iso_extsegment_len = 0;
            c.iso_embedded_utf_8 = false;
        } else if coding_type == sym::CHARSET {
            c.kind = CodingType::Charset;
            c.common_flags |= REQUIRE_DECODING_MASK | REQUIRE_ENCODING_MASK;
        } else if coding_type == sym::UTF_8 {
            c.kind = CodingType::Utf8;
            let val = self.attr(attrs, ATTR_UTF_BOM);
            c.utf_8_bom = if val.is_cons() {
                Bom::Detect
            } else if val == sym::T {
                Bom::With
            } else {
                Bom::Without
            };
            c.common_flags |= REQUIRE_DECODING_MASK | REQUIRE_ENCODING_MASK;
            if c.utf_8_bom == Bom::Detect {
                c.common_flags |= REQUIRE_DETECTION_MASK;
            }
        } else if coding_type == sym::UTF_16 {
            c.kind = CodingType::Utf16;
            let val = self.attr(attrs, ATTR_UTF_BOM);
            c.utf_16_bom = if val.is_cons() {
                Bom::Detect
            } else if val == sym::T {
                Bom::With
            } else {
                Bom::Without
            };
            c.utf_16_big_endian = self.attr(attrs, ATTR_UTF_16_ENDIAN) == sym::BIG;
            c.utf_16_surrogate = 0;
            c.common_flags |= REQUIRE_DECODING_MASK | REQUIRE_ENCODING_MASK;
            if c.utf_16_bom == Bom::Detect {
                c.common_flags |= REQUIRE_DETECTION_MASK;
            }
        } else if coding_type == sym::CCL {
            c.kind = CodingType::Ccl;
            c.common_flags |= REQUIRE_DECODING_MASK | REQUIRE_ENCODING_MASK | REQUIRE_FLUSHING_MASK;
        } else if coding_type == sym::EMACS_MULE {
            c.kind = CodingType::EmacsMule;
            c.common_flags |= REQUIRE_DECODING_MASK | REQUIRE_ENCODING_MASK;
            let full = !self.attr(attrs, ATTR_EMACS_MULE_FULL).is_nil();
            if full && self.attr(attrs, ATTR_CHARSET_LIST) != self.charsets.emacs_mule_charset_list
            {
                let mut max_charset_id = 0i64;
                let mut tail = self.charsets.emacs_mule_charset_list;
                while tail.is_cons() {
                    max_charset_id = max_charset_id.max(self.xcar(tail).as_fixnum().unwrap_or(0));
                    tail = self.xcdr(tail);
                }
                let mut safe = vec![255u8; max_charset_id as usize + 1];
                let mut tail = self.charsets.emacs_mule_charset_list;
                while tail.is_cons() {
                    safe[self.xcar(tail).as_fixnum().unwrap_or(0) as usize] = 0;
                    tail = self.xcdr(tail);
                }
                c.max_charset_id = max_charset_id;
                c.safe_charsets = safe;
            }
        } else if coding_type == sym::SHIFT_JIS {
            c.kind = CodingType::ShiftJis;
            c.common_flags |= REQUIRE_DECODING_MASK | REQUIRE_ENCODING_MASK;
        } else if coding_type == sym::BIG5 {
            c.kind = CodingType::Big5;
            c.common_flags |= REQUIRE_DECODING_MASK | REQUIRE_ENCODING_MASK;
        } else {
            c.kind = CodingType::RawText;
            if eol_type != sym::UNIX {
                c.common_flags |= REQUIRE_DECODING_MASK;
                if !self.is_vector(eol_type) {
                    c.common_flags |= REQUIRE_ENCODING_MASK;
                }
            }
        }
        Ok(c)
    }

    pub fn coding_system_charset_list(&mut self, coding_system: Value) -> LispResult {
        let id = self.check_coding_system_get_id(coding_system)?;
        let attrs = self.coding_id_attrs(id);
        let kind = self.attr(attrs, ATTR_TYPE);
        if kind == sym::ISO_2022 {
            let flags = self.attr(attrs, ATTR_ISO_FLAGS).as_fixnum().unwrap_or(0);
            if flags & ISO_FLAG_FULL_SUPPORT != 0 {
                return Ok(self.charsets.iso_2022_charset_list);
            }
            return Ok(self.attr(attrs, ATTR_CHARSET_LIST));
        }
        if kind == sym::EMACS_MULE {
            return Ok(self.charsets.emacs_mule_charset_list);
        }
        Ok(self.attr(attrs, ATTR_CHARSET_LIST))
    }

    pub fn raw_text_coding_system(&mut self, coding_system: Value) -> LispResult {
        if coding_system.is_nil() {
            return Ok(sym::RAW_TEXT);
        }
        let spec = self.coding_system_spec(coding_system)?;
        let attrs = self.vector_items(spec)[0];
        if self.attr(attrs, ATTR_TYPE) == sym::RAW_TEXT {
            return Ok(coding_system);
        }
        let eol_type = self.vector_items(spec)[2];
        if self.is_vector(eol_type) {
            return Ok(sym::RAW_TEXT);
        }
        let raw_spec = self.coding_system_spec(sym::RAW_TEXT)?;
        let raw_eol = self.vector_items(raw_spec)[2];
        let idx = if eol_type == sym::UNIX {
            0
        } else if eol_type == sym::DOS {
            1
        } else {
            2
        };
        Ok(self.vector_items(raw_eol)[idx])
    }

    pub fn coding_inherit_eol_type(&mut self, coding_system: Value, parent: Value) -> LispResult {
        let coding_system = if coding_system.is_nil() {
            sym::RAW_TEXT
        } else {
            self.check_coding_system(coding_system)?;
            coding_system
        };
        let spec = self.coding_system_spec(coding_system)?;
        let eol_type = self.vector_items(spec)[2];
        if !self.is_vector(eol_type) {
            return Ok(coding_system);
        }
        let system_eol_type = if cfg!(windows) { sym::DOS } else { sym::UNIX };
        let mut parent_eol_type = system_eol_type;
        if !parent.is_nil() {
            self.check_coding_system(parent)?;
            let pspec = self.coding_system_spec(parent)?;
            let ptype = self.vector_items(pspec)[2];
            if !self.is_vector(ptype) {
                parent_eol_type = ptype;
            }
        }
        let idx = if parent_eol_type == sym::UNIX {
            Some(0)
        } else if parent_eol_type == sym::DOS {
            Some(1)
        } else if parent_eol_type == sym::MAC {
            Some(2)
        } else {
            None
        };
        Ok(match idx {
            Some(i) => self.vector_items(eol_type)[i],
            None => coding_system,
        })
    }

    fn make_subsidiaries(&mut self, base: Value) -> Value {
        let name = self.symbol_name_string(base);
        let items: Vec<Value> = ["-unix", "-dos", "-mac"]
            .iter()
            .map(|s| self.intern(&format!("{name}{s}")))
            .collect();
        self.vector(items)
    }

    fn register_coding_name(&mut self, name: Value, spec: Value) -> LispResult<()> {
        let table = self.codings.hash_table;
        self.puthash(name, spec, table)?;
        let list = self.bound_var(sym::CODING_SYSTEM_LIST);
        let list = self.cons(name, list);
        self.set_plain_value(sym::CODING_SYSTEM_LIST, list);
        let sname = self.symbol_name(name);
        let alist = self.bound_var(sym::CODING_SYSTEM_ALIST);
        let mut found = false;
        let mut tail = alist;
        while tail.is_cons() {
            let elt = self.xcar(tail);
            if elt.is_cons() && self.equal(self.xcar(elt), sname)? {
                found = true;
                break;
            }
            tail = self.xcdr(tail);
        }
        if !found {
            let s = crate::fns::f_copy_sequence(self, &[sname])?;
            let entry = self.cons(s, Value::NIL);
            let alist = self.cons(entry, alist);
            self.set_plain_value(sym::CODING_SYSTEM_ALIST, alist);
        }
        Ok(())
    }
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn f_coding_system_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = lisp.coding_system_p(args[0])?;
    Ok(lisp.bool(v))
}

fn f_read_non_nil_coding_system(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prompt = args[0];
    loop {
        let alist = lisp.bound_var(sym::CODING_SYSTEM_ALIST);
        let f = lisp.intern("completing-read");
        let val = lisp.funcall(
            f,
            &[
                prompt,
                alist,
                Value::NIL,
                Value::NIL,
                Value::NIL,
                sym::CODING_SYSTEM_HISTORY,
            ],
        )?;
        if lisp.string_chars(val) > 0 {
            let name = lisp.string_to_rust(val);
            return Ok(lisp.intern(&name));
        }
    }
}

fn f_read_coding_system(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prompt = args[0];
    let default = arg(args, 1);
    let default = if default.is_symbol() && !default.is_nil() {
        lisp.symbol_name(default)
    } else {
        default
    };
    let alist = lisp.bound_var(sym::CODING_SYSTEM_ALIST);
    let f = lisp.intern("completing-read");
    let val = lisp.funcall(
        f,
        &[
            prompt,
            alist,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            sym::CODING_SYSTEM_HISTORY,
            default,
            Value::NIL,
        ],
    )?;
    if lisp.string_chars(val) == 0 {
        return Ok(Value::NIL);
    }
    let name = lisp.string_to_rust(val);
    Ok(lisp.intern(&name))
}

fn f_check_coding_system(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.check_coding_system(args[0])
}

fn f_set_terminal_coding_system_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut c = lisp.setup_coding_system(args[0])?;
    c.common_flags |= REQUIRE_ENCODING_MASK;
    c.common_flags &= !ANNOTATE_COMPOSITION_MASK;
    lisp.codings.terminal = c;
    Ok(Value::NIL)
}

fn f_set_safe_terminal_coding_system_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.check_coding_system(args[0])?;
    let mut c = lisp.setup_coding_system(args[0])?;
    c.common_flags &= !ANNOTATE_COMPOSITION_MASK;
    lisp.codings.safe_terminal = c;
    Ok(Value::NIL)
}

fn f_terminal_coding_system(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let id = lisp.codings.terminal.id;
    if id < 0 {
        return Ok(Value::NIL);
    }
    let attrs = lisp.coding_id_attrs(id);
    Ok(lisp.attr(attrs, ATTR_BASE_NAME))
}

fn f_set_keyboard_coding_system_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.setup_coding_system(args[0])?;
    lisp.codings.keyboard = c;
    Ok(Value::NIL)
}

fn f_keyboard_coding_system(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let id = lisp.codings.keyboard.id;
    if id < 0 {
        return Ok(Value::NIL);
    }
    Ok(lisp.coding_id_name(id))
}

fn f_set_coding_system_priority(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut changed = [false; CAT_MAX];
    let mut priorities = [0usize; CAT_MAX];
    let mut j = 0;
    for &a in args {
        let spec = lisp.check_coding_system_get_spec(a)?;
        let attrs = lisp.vector_items(spec)[0];
        let category = lisp.attr(attrs, ATTR_CATEGORY).as_fixnum().unwrap_or(0) as usize;
        if changed[category] {
            continue;
        }
        changed[category] = true;
        priorities[j] = category;
        j += 1;
        let current = lisp.codings.categories[category].id;
        if current >= 0 && a != lisp.coding_id_name(current) {
            let c = lisp.setup_coding_system(a)?;
            lisp.codings.categories[category] = c;
        }
        let s = lisp.vector_items(lisp.codings.category_table)[category];
        lisp.set(s, a)?;
    }
    let mut k = 0;
    for slot in priorities.iter_mut().skip(j) {
        while k < CAT_MAX && changed[lisp.codings.priorities[k]] {
            k += 1;
        }
        *slot = lisp.codings.priorities[k];
        k += 1;
    }
    lisp.codings.priorities = priorities;
    let mut list = Value::NIL;
    for &p in priorities.iter().rev() {
        let s = lisp.vector_items(lisp.codings.category_table)[p];
        list = lisp.cons(s, list);
    }
    lisp.set_plain_value(sym::CODING_CATEGORY_LIST, list);
    Ok(Value::NIL)
}

fn f_coding_system_priority_list(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let highestp = arg(args, 0);
    let mut out = Vec::new();
    for i in 0..CAT_MAX {
        let category = lisp.codings.priorities[i];
        let id = lisp.codings.categories[category].id;
        if id < 0 {
            continue;
        }
        let attrs = lisp.coding_id_attrs(id);
        let base = lisp.attr(attrs, ATTR_BASE_NAME);
        if !highestp.is_nil() {
            return Ok(base);
        }
        out.push(base);
    }
    Ok(lisp.list(&out))
}

fn short_args(lisp: &mut Lisp, nargs: usize) -> crate::error::Flow {
    let n = Value::fixnum(nargs as i64);
    let data = lisp.cons(sym::DEFINE_CODING_SYSTEM_INTERNAL, n);
    lisp.signal(sym::WRONG_NUMBER_OF_ARGUMENTS, data)
}

fn f_define_coding_system_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args.len() < ARG_MAX {
        return Err(short_args(lisp, args.len()));
    }
    let attrs = lisp.vector(vec![Value::NIL; ATTR_LAST]);
    let base = lisp.stack.len();
    lisp.stack.push(attrs);
    let r = define_coding_system_internal_1(lisp, args, attrs);
    lisp.stack.truncate(base);
    r
}

fn define_coding_system_internal_1(lisp: &mut Lisp, args: &[Value], attrs: Value) -> LispResult {
    let name = lisp.check_symbol(args[ARG_NAME])?;
    lisp.set_attr(attrs, ATTR_BASE_NAME, name);
    let mut val = args[ARG_MNEMONIC];
    if val.is_string() {
        let c = lisp.string_char_vec(val).first().copied().unwrap_or(0);
        val = Value::fixnum(c as i64);
    } else {
        lisp.check_character(val)?;
    }
    lisp.set_attr(attrs, ATTR_MNEMONIC, val);
    let coding_type = lisp.check_symbol(args[ARG_CODING_TYPE])?;
    lisp.set_attr(attrs, ATTR_TYPE, coding_type);
    let mut charset_list = args[ARG_CHARSET_LIST];
    let mut max_charset_id: i64 = 0;
    if charset_list.is_symbol() {
        if charset_list == sym::ISO_2022 {
            if coding_type != sym::ISO_2022 {
                return Err(lisp.error("Invalid charset-list"));
            }
            charset_list = lisp.charsets.iso_2022_charset_list;
        } else if charset_list == sym::EMACS_MULE {
            if coding_type != sym::EMACS_MULE {
                return Err(lisp.error("Invalid charset-list"));
            }
            charset_list = lisp.charsets.emacs_mule_charset_list;
        }
        let mut tail = charset_list;
        while tail.is_cons() {
            match lisp.xcar(tail).as_fixnum() {
                Some(n) if (0..i32::MAX as i64 - 1).contains(&n) => {
                    max_charset_id = max_charset_id.max(n);
                }
                _ => return Err(lisp.error("Invalid charset-list")),
            }
            tail = lisp.xcdr(tail);
        }
    } else {
        charset_list = crate::fns::f_copy_sequence(lisp, &[charset_list])?;
        lisp.stack.push(charset_list);
        let mut tail = charset_list;
        while tail.is_cons() {
            let v = lisp.xcar(tail);
            let id = lisp.check_charset_get_id(v)?;
            let cs = lisp.charset(id);
            let bad = if coding_type == sym::ISO_2022 {
                cs.iso_final < 0
            } else if coding_type == sym::EMACS_MULE {
                cs.emacs_mule_id < 0
            } else {
                false
            };
            if bad {
                let cname = lisp.charset_name(id);
                let n = lisp.symbol_name_string(cname);
                return Err(lisp.error(&format!("Can't handle charset `{n}'")));
            }
            lisp.heap.set_car(tail, Value::fixnum(id as i64));
            max_charset_id = max_charset_id.max(id as i64);
            tail = lisp.xcdr(tail);
        }
    }
    lisp.set_attr(attrs, ATTR_CHARSET_LIST, charset_list);
    let mut safe = vec![255u8; max_charset_id as usize + 1];
    let mut tail = charset_list;
    while tail.is_cons() {
        safe[lisp.xcar(tail).as_fixnum().unwrap_or(0) as usize] = 0;
        tail = lisp.xcdr(tail);
    }
    let safe = lisp.unibyte_string(&safe);
    lisp.set_attr(attrs, ATTR_SAFE_CHARSETS, safe);
    lisp.set_attr(attrs, ATTR_ASCII_COMPAT, args[ARG_ASCII_COMPATIBLE_P]);
    for (idx, a) in [
        (ATTR_DECODE_TBL, ARG_DECODE_TRANSLATION_TABLE),
        (ATTR_ENCODE_TBL, ARG_ENCODE_TRANSLATION_TABLE),
    ] {
        let v = args[a];
        if !lisp.is_char_table(v) && !v.is_cons() {
            lisp.check_symbol(v)?;
        }
        lisp.set_attr(attrs, idx, v);
    }
    let v = lisp.check_symbol(args[ARG_POST_READ_CONVERSION])?;
    lisp.set_attr(attrs, ATTR_POST_READ, v);
    let v = lisp.check_symbol(args[ARG_PRE_WRITE_CONVERSION])?;
    lisp.set_attr(attrs, ATTR_PRE_WRITE, v);
    let v = args[ARG_DEFAULT_CHAR];
    if v.is_nil() {
        lisp.set_attr(attrs, ATTR_DEFAULT_CHAR, Value::fixnum(b' ' as i64));
    } else {
        lisp.check_character(v)?;
        lisp.set_attr(attrs, ATTR_DEFAULT_CHAR, v);
    }
    let v = if args[ARG_FOR_UNIBYTE].is_nil() {
        Value::NIL
    } else {
        sym::T
    };
    lisp.set_attr(attrs, ATTR_FOR_UNIBYTE, v);
    let plist = lisp.check_list(args[ARG_PLIST])?;
    lisp.set_attr(attrs, ATTR_PLIST, plist);
    let category;
    if coding_type == sym::CHARSET {
        let valids = lisp.vector(vec![Value::NIL; 256]);
        lisp.stack.push(valids);
        let mut tail = charset_list;
        while tail.is_cons() {
            let id_v = lisp.xcar(tail);
            let id = id_v.as_fixnum().unwrap_or(0) as i32;
            let cs = lisp.charset(id).clone();
            let dim = cs.dimension;
            let idx = ((dim - 1) * 4) as usize;
            if cs.ascii_compatible_p {
                lisp.set_attr(attrs, ATTR_ASCII_COMPAT, sym::T);
            }
            for i in cs.code_space[idx]..=cs.code_space[idx + 1] {
                let mut tmp = lisp.vector_items(valids)[i as usize];
                if tmp.is_nil() {
                    tmp = id_v;
                } else if let Some(n) = tmp.as_fixnum() {
                    let dim2 = lisp.charset(n as i32).dimension;
                    tmp = if dim < dim2 {
                        lisp.list(&[id_v, tmp])
                    } else {
                        lisp.list(&[tmp, id_v])
                    };
                } else {
                    let mut tmp2 = tmp;
                    while tmp2.is_cons() {
                        let dim2 = lisp
                            .charset(lisp.xcar(tmp2).as_fixnum().unwrap_or(0) as i32)
                            .dimension;
                        if dim < dim2 {
                            break;
                        }
                        tmp2 = lisp.xcdr(tmp2);
                    }
                    if tmp2.is_nil() {
                        let cell = lisp.list(&[id_v]);
                        tmp = lisp.nconc2(tmp, cell);
                    } else {
                        let rest = lisp.cons(lisp.xcar(tmp2), lisp.xcdr(tmp2));
                        lisp.heap.set_cdr(tmp2, rest);
                        lisp.heap.set_car(tmp2, id_v);
                    }
                }
                lisp.aset_value(valids, Value::fixnum(i as i64), tmp)?;
            }
            tail = lisp.xcdr(tail);
        }
        lisp.set_attr(attrs, ATTR_CHARSET_VALIDS, valids);
        category = CAT_CHARSET;
    } else if coding_type == sym::CCL {
        if args.len() < ARG_CCL_MAX {
            return Err(short_args(lisp, args.len()));
        }
        for (idx, a) in [
            (ATTR_CCL_DECODER, ARG_CCL_DECODER),
            (ATTR_CCL_ENCODER, ARG_CCL_ENCODER),
        ] {
            let mut v = args[a];
            lisp.check_ccl_program(v)?;
            if lisp.is_vector(v) {
                v = crate::fns::f_copy_sequence(lisp, &[v])?;
            }
            lisp.set_attr(attrs, idx, v);
        }
        let mut valids = vec![0u8; 256];
        let mut tail = args[ARG_CCL_VALIDS];
        while tail.is_cons() {
            let v = lisp.xcar(tail);
            let (from, to) = if let Some(n) = v.as_fixnum() {
                if !(0..=255).contains(&n) {
                    return Err(lisp.signal_list(
                        sym::ARGS_OUT_OF_RANGE,
                        &[v, Value::fixnum(0), Value::fixnum(255)],
                    ));
                }
                (n, n)
            } else {
                lisp.check_cons(v)?;
                let from = lisp.check_fixnum(lisp.xcar(v))?;
                if !(0..=255).contains(&from) {
                    return Err(lisp.signal_list(
                        sym::ARGS_OUT_OF_RANGE,
                        &[lisp.xcar(v), Value::fixnum(0), Value::fixnum(255)],
                    ));
                }
                let to = lisp.check_fixnum(lisp.xcdr(v))?;
                if !(from..=255).contains(&to) {
                    return Err(lisp.signal_list(
                        sym::ARGS_OUT_OF_RANGE,
                        &[lisp.xcdr(v), Value::fixnum(from), Value::fixnum(255)],
                    ));
                }
                (from, to)
            };
            for i in from..=to {
                valids[i as usize] = 1;
            }
            tail = lisp.xcdr(tail);
        }
        let valids = lisp.unibyte_string(&valids);
        lisp.set_attr(attrs, ATTR_CCL_VALIDS, valids);
        category = CAT_CCL;
    } else if coding_type == sym::UTF_16 {
        lisp.set_attr(attrs, ATTR_ASCII_COMPAT, Value::NIL);
        if args.len() < ARG_UTF16_MAX {
            return Err(short_args(lisp, args.len()));
        }
        let bom = args[ARG_UTF16_BOM];
        if !bom.is_nil() && bom != sym::T {
            lisp.check_cons(bom)?;
            lisp.check_coding_system(lisp.xcar(bom))?;
            lisp.check_coding_system(lisp.xcdr(bom))?;
        }
        lisp.set_attr(attrs, ATTR_UTF_BOM, bom);
        let mut endian = lisp.check_symbol(args[ARG_UTF16_ENDIAN])?;
        if endian.is_nil() {
            endian = sym::BIG;
        } else if endian != sym::BIG && endian != sym::LITTLE {
            let n = lisp.symbol_name_string(endian);
            return Err(lisp.error(&format!("Invalid endian: {n}")));
        }
        lisp.set_attr(attrs, ATTR_UTF_16_ENDIAN, endian);
        category = if bom.is_cons() {
            CAT_UTF_16_AUTO
        } else if bom.is_nil() {
            if endian == sym::BIG {
                CAT_UTF_16_BE_NOSIG
            } else {
                CAT_UTF_16_LE_NOSIG
            }
        } else if endian == sym::BIG {
            CAT_UTF_16_BE
        } else {
            CAT_UTF_16_LE
        };
    } else if coding_type == sym::ISO_2022 {
        if args.len() < ARG_ISO2022_MAX {
            return Err(short_args(lisp, args.len()));
        }
        let initial = crate::fns::f_copy_sequence(lisp, &[args[ARG_ISO2022_INITIAL]])?;
        if !lisp.is_vector(initial) {
            return Err(lisp.wrong_type(sym::VECTORP, initial));
        }
        lisp.stack.push(initial);
        for i in 0..4 {
            let v = lisp.vector_items(initial)[i];
            if !v.is_nil() {
                let id = lisp.check_charset_get_id(v)?;
                lisp.aset_value(initial, Value::fixnum(i as i64), Value::fixnum(id as i64))?;
                if i == 0 && lisp.charset(id).ascii_compatible_p {
                    lisp.set_attr(attrs, ATTR_ASCII_COMPAT, sym::T);
                }
            } else {
                lisp.aset_value(initial, Value::fixnum(i as i64), Value::fixnum(-1))?;
            }
        }
        let reg_usage = lisp.check_cons(args[ARG_ISO2022_REG_USAGE])?;
        lisp.check_fixnum(lisp.xcar(reg_usage))?;
        lisp.check_fixnum(lisp.xcdr(reg_usage))?;
        let request = crate::fns::f_copy_sequence(lisp, &[args[ARG_ISO2022_REQUEST]])?;
        lisp.stack.push(request);
        let mut tail = request;
        while tail.is_cons() {
            let v = lisp.check_cons(lisp.xcar(tail))?;
            let id = lisp.check_charset_get_id(lisp.xcar(v))?;
            let reg = lisp.check_fixnum(lisp.xcdr(v))?;
            if !(0..=3).contains(&reg) {
                return Err(lisp.signal_list(
                    sym::ARGS_OUT_OF_RANGE,
                    &[lisp.xcdr(v), Value::fixnum(0), Value::fixnum(3)],
                ));
            }
            lisp.heap.set_car(v, Value::fixnum(id as i64));
            tail = lisp.xcdr(tail);
        }
        let flags_v = lisp.check_fixnat(args[ARG_ISO2022_FLAGS])?;
        let mut i = flags_v & i32::MAX as i64;
        if args[ARG_CHARSET_LIST] == sym::ISO_2022 {
            i |= ISO_FLAG_FULL_SUPPORT;
        }
        lisp.set_attr(attrs, ATTR_ISO_INITIAL, initial);
        lisp.set_attr(attrs, ATTR_ISO_USAGE, reg_usage);
        lisp.set_attr(attrs, ATTR_ISO_REQUEST, request);
        lisp.set_attr(attrs, ATTR_ISO_FLAGS, Value::fixnum(i));
        lisp.setup_iso_safe_charsets(attrs)?;
        let full_list = args[ARG_CHARSET_LIST] == sym::ISO_2022;
        category = if i & ISO_FLAG_SEVEN_BITS != 0 {
            if i & (ISO_FLAG_LOCKING_SHIFT | ISO_FLAG_SINGLE_SHIFT) != 0 {
                CAT_ISO_7_ELSE
            } else if full_list {
                CAT_ISO_7
            } else {
                CAT_ISO_7_TIGHT
            }
        } else {
            let id = lisp.vector_items(initial)[1].as_fixnum().unwrap_or(-1);
            if i & ISO_FLAG_LOCKING_SHIFT != 0 || full_list || id < 0 {
                CAT_ISO_8_ELSE
            } else if lisp.charset(id as i32).dimension == 1 {
                CAT_ISO_8_1
            } else {
                CAT_ISO_8_2
            }
        };
        if category != CAT_ISO_8_1 && category != CAT_ISO_8_2 {
            lisp.set_attr(attrs, ATTR_ASCII_COMPAT, Value::NIL);
        }
    } else if coding_type == sym::EMACS_MULE {
        if args[ARG_CHARSET_LIST] == sym::EMACS_MULE {
            lisp.set_attr(attrs, ATTR_EMACS_MULE_FULL, sym::T);
        }
        lisp.set_attr(attrs, ATTR_ASCII_COMPAT, sym::T);
        category = CAT_EMACS_MULE;
    } else if coding_type == sym::SHIFT_JIS {
        let items = lisp.list_to_vec(charset_list)?;
        if items.len() != 3 && items.len() != 4 {
            return Err(lisp.error("There should be three or four charsets"));
        }
        let expect = [1, 1, 2, 2];
        for (k, item) in items.iter().enumerate() {
            let id = item.as_fixnum().unwrap_or(0) as i32;
            let cs = lisp.charset(id);
            if cs.dimension != expect[k] {
                let cname = lisp.charset_name(id);
                let n = lisp.symbol_name_string(cname);
                let word = if expect[k] == 1 { "one" } else { "two" };
                return Err(lisp.error(&format!("Dimension of charset {n} is not {word}")));
            }
            if k == 0 && cs.ascii_compatible_p {
                lisp.set_attr(attrs, ATTR_ASCII_COMPAT, sym::T);
            }
        }
        category = CAT_SJIS;
        lisp.codings.sjis_coding_system = name;
    } else if coding_type == sym::BIG5 {
        let items = lisp.list_to_vec(charset_list)?;
        if items.len() != 2 {
            return Err(lisp.error("There should be just two charsets"));
        }
        for (k, item) in items.iter().enumerate() {
            let id = item.as_fixnum().unwrap_or(0) as i32;
            let cs = lisp.charset(id);
            let want = k as i32 + 1;
            if cs.dimension != want {
                let cname = lisp.charset_name(id);
                let n = lisp.symbol_name_string(cname);
                let word = if want == 1 { "one" } else { "two" };
                return Err(lisp.error(&format!("Dimension of charset {n} is not {word}")));
            }
            if k == 0 && cs.ascii_compatible_p {
                lisp.set_attr(attrs, ATTR_ASCII_COMPAT, sym::T);
            }
        }
        category = CAT_BIG5;
        lisp.codings.big5_coding_system = name;
    } else if coding_type == sym::RAW_TEXT {
        category = CAT_RAW_TEXT;
        lisp.set_attr(attrs, ATTR_ASCII_COMPAT, sym::T);
    } else if coding_type == sym::UTF_8 {
        if args.len() < ARG_UTF8_MAX {
            return Err(short_args(lisp, args.len()));
        }
        let bom = args[ARG_UTF8_BOM];
        if !bom.is_nil() && bom != sym::T {
            lisp.check_cons(bom)?;
            lisp.check_coding_system(lisp.xcar(bom))?;
            lisp.check_coding_system(lisp.xcdr(bom))?;
        }
        lisp.set_attr(attrs, ATTR_UTF_BOM, bom);
        if bom.is_nil() {
            lisp.set_attr(attrs, ATTR_ASCII_COMPAT, sym::T);
        }
        category = if bom.is_cons() {
            CAT_UTF_8_AUTO
        } else if bom.is_nil() {
            CAT_UTF_8_NOSIG
        } else {
            CAT_UTF_8_SIG
        };
    } else if coding_type == sym::UNDECIDED {
        if args.len() < ARG_UNDECIDED_MAX {
            return Err(short_args(lisp, args.len()));
        }
        lisp.set_attr(
            attrs,
            ATTR_UNDECIDED_INHIBIT_NBD,
            args[ARG_UNDECIDED_INHIBIT_NBD],
        );
        lisp.set_attr(
            attrs,
            ATTR_UNDECIDED_INHIBIT_IED,
            args[ARG_UNDECIDED_INHIBIT_IED],
        );
        lisp.set_attr(
            attrs,
            ATTR_UNDECIDED_PREFER_UTF_8,
            args[ARG_UNDECIDED_PREFER_UTF_8],
        );
        category = CAT_UNDECIDED;
    } else {
        let n = lisp.symbol_name_string(coding_type);
        return Err(lisp.error(&format!("Invalid coding system type: {n}")));
    }
    lisp.set_attr(attrs, ATTR_CATEGORY, Value::fixnum(category as i64));
    let cat_sym = lisp.vector_items(lisp.codings.category_table)[category];
    let plist = lisp.attr(attrs, ATTR_PLIST);
    let plist = lisp.list_with_tail(&[sym::K_CATEGORY, cat_sym], plist);
    lisp.set_attr(attrs, ATTR_PLIST, plist);
    let compat = lisp.attr(attrs, ATTR_ASCII_COMPAT);
    let plist = lisp.list_with_tail(&[sym::K_ASCII_COMPATIBLE_P, compat], plist);
    lisp.set_attr(attrs, ATTR_PLIST, plist);
    let mut eol_type = args[ARG_EOL_TYPE];
    if !eol_type.is_nil() && eol_type != sym::UNIX && eol_type != sym::DOS && eol_type != sym::MAC {
        return Err(lisp.error("Invalid eol-type"));
    }
    let aliases = lisp.list(&[name]);
    lisp.stack.push(aliases);
    if eol_type.is_nil() {
        eol_type = lisp.make_subsidiaries(name);
        lisp.stack.push(eol_type);
        for i in 0..3 {
            let this_name = lisp.vector_items(eol_type)[i];
            let this_aliases = lisp.list(&[this_name]);
            let this_eol = [sym::UNIX, sym::DOS, sym::MAC][i];
            let spec = lisp.vector(vec![attrs, this_aliases, this_eol]);
            lisp.register_coding_name(this_name, spec)?;
        }
    }
    let spec = lisp.vector(vec![attrs, aliases, eol_type]);
    lisp.register_coding_name(name, spec)?;
    let id = lisp.codings.categories[category].id;
    if id < 0 || name == lisp.coding_id_name(id) {
        let c = lisp.setup_coding_system(name)?;
        lisp.codings.categories[category] = c;
    }
    Ok(Value::NIL)
}

fn f_coding_system_put(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (coding_system, prop, mut val) = (args[0], args[1], args[2]);
    let spec = lisp.check_coding_system_get_spec(coding_system)?;
    let attrs = lisp.vector_items(spec)[0];
    if prop == sym::K_MNEMONIC {
        if val.is_string() {
            let c = lisp.string_char_vec(val).first().copied().unwrap_or(0);
            val = Value::fixnum(c as i64);
        } else {
            lisp.check_character(val)?;
        }
        lisp.set_attr(attrs, ATTR_MNEMONIC, val);
    } else if prop == sym::K_DEFAULT_CHAR {
        if val.is_nil() {
            val = Value::fixnum(b' ' as i64);
        } else {
            lisp.check_character(val)?;
        }
        lisp.set_attr(attrs, ATTR_DEFAULT_CHAR, val);
    } else if prop == sym::K_DECODE_TRANSLATION_TABLE {
        if !lisp.is_char_table(val) && !val.is_cons() {
            lisp.check_symbol(val)?;
        }
        lisp.set_attr(attrs, ATTR_DECODE_TBL, val);
    } else if prop == sym::K_ENCODE_TRANSLATION_TABLE {
        if !lisp.is_char_table(val) && !val.is_cons() {
            lisp.check_symbol(val)?;
        }
        lisp.set_attr(attrs, ATTR_ENCODE_TBL, val);
    } else if prop == sym::K_POST_READ_CONVERSION {
        lisp.check_symbol(val)?;
        lisp.set_attr(attrs, ATTR_POST_READ, val);
    } else if prop == sym::K_PRE_WRITE_CONVERSION {
        lisp.check_symbol(val)?;
        lisp.set_attr(attrs, ATTR_PRE_WRITE, val);
    } else if prop == sym::K_ASCII_COMPATIBLE_P {
        lisp.set_attr(attrs, ATTR_ASCII_COMPAT, val);
    }
    let plist = lisp.attr(attrs, ATTR_PLIST);
    let plist = lisp.plist_put_eq(plist, prop, val)?;
    lisp.set_attr(attrs, ATTR_PLIST, plist);
    Ok(val)
}

fn f_define_coding_system_alias(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let alias = lisp.check_symbol(args[0])?;
    let spec = lisp.check_coding_system_get_spec(args[1])?;
    let mut aliases = lisp.vector_items(spec)[1];
    while !lisp.xcdr(aliases).is_nil() {
        aliases = lisp.xcdr(aliases);
    }
    let cell = lisp.list(&[alias]);
    lisp.heap.set_cdr(aliases, cell);
    let eol_type = lisp.vector_items(spec)[2];
    if lisp.is_vector(eol_type) {
        let subsidiaries = lisp.make_subsidiaries(alias);
        lisp.stack.push(subsidiaries);
        for i in 0..3 {
            let a = lisp.vector_items(subsidiaries)[i];
            let c = lisp.vector_items(eol_type)[i];
            f_define_coding_system_alias(lisp, &[a, c])?;
        }
        lisp.stack.pop();
    }
    lisp.register_coding_name(alias, spec)?;
    Ok(Value::NIL)
}

fn f_coding_system_base(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args[0].is_nil() {
        return Ok(sym::NO_CONVERSION);
    }
    let spec = lisp.check_coding_system_get_spec(args[0])?;
    let attrs = lisp.vector_items(spec)[0];
    Ok(lisp.attr(attrs, ATTR_BASE_NAME))
}

pub(crate) fn f_coding_system_plist(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let cs = if args[0].is_nil() {
        sym::NO_CONVERSION
    } else {
        args[0]
    };
    let spec = lisp.check_coding_system_get_spec(cs)?;
    let attrs = lisp.vector_items(spec)[0];
    Ok(lisp.attr(attrs, ATTR_PLIST))
}

fn f_coding_system_aliases(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let cs = if args[0].is_nil() {
        sym::NO_CONVERSION
    } else {
        args[0]
    };
    let spec = lisp.check_coding_system_get_spec(cs)?;
    Ok(lisp.vector_items(spec)[1])
}

fn f_coding_system_eol_type(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let cs = if args[0].is_nil() {
        sym::NO_CONVERSION
    } else {
        args[0]
    };
    if lisp.coding_system_id(cs)? < 0 && !(cs.is_symbol() && lisp.coding_system_p(cs)?) {
        return Ok(Value::NIL);
    }
    let spec = lisp.coding_system_spec(cs)?;
    if spec.is_nil() {
        return Ok(Value::NIL);
    }
    let eol_type = lisp.vector_items(spec)[2];
    if lisp.is_vector(eol_type) {
        return crate::fns::f_copy_sequence(lisp, &[eol_type]);
    }
    let n = if eol_type == sym::UNIX {
        0
    } else if eol_type == sym::DOS {
        1
    } else {
        2
    };
    Ok(Value::fixnum(n))
}
