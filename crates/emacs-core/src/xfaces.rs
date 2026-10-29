use crate::error::LispResult;
use crate::font::{
    FONT_ADSTYLE_INDEX, FONT_FAMILY_INDEX, FONT_FOUNDRY_INDEX, FONT_SIZE_INDEX, FONT_SLANT_INDEX,
    FONT_WEIGHT_INDEX, FONT_WIDTH_INDEX,
};
use crate::frame::{FACE_TTY_DEFAULT_BG_COLOR, FACE_TTY_DEFAULT_FG_COLOR};
use crate::lisp::Lisp;
use crate::object::{Vectorlike, Weakness};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::terminal::OutputMethod;
use crate::value::Value;

pub const LFACE_FAMILY_INDEX: usize = 1;
pub const LFACE_FOUNDRY_INDEX: usize = 2;
pub const LFACE_SWIDTH_INDEX: usize = 3;
pub const LFACE_HEIGHT_INDEX: usize = 4;
pub const LFACE_WEIGHT_INDEX: usize = 5;
pub const LFACE_SLANT_INDEX: usize = 6;
pub const LFACE_UNDERLINE_INDEX: usize = 7;
pub const LFACE_INVERSE_INDEX: usize = 8;
pub const LFACE_FOREGROUND_INDEX: usize = 9;
pub const LFACE_BACKGROUND_INDEX: usize = 10;
pub const LFACE_STIPPLE_INDEX: usize = 11;
pub const LFACE_OVERLINE_INDEX: usize = 12;
pub const LFACE_STRIKE_THROUGH_INDEX: usize = 13;
pub const LFACE_BOX_INDEX: usize = 14;
pub const LFACE_FONT_INDEX: usize = 15;
pub const LFACE_INHERIT_INDEX: usize = 16;
pub const LFACE_FONTSET_INDEX: usize = 17;
pub const LFACE_DISTANT_FOREGROUND_INDEX: usize = 18;
pub const LFACE_EXTEND_INDEX: usize = 19;
pub const LFACE_VECTOR_SIZE: usize = 20;

const N: usize = LFACE_VECTOR_SIZE;

pub const FACE_ATTR_SYM: [Value; LFACE_VECTOR_SIZE] = [
    sym::FACE,
    sym::K_FAMILY,
    sym::K_FOUNDRY,
    sym::K_WIDTH,
    sym::K_HEIGHT,
    sym::K_WEIGHT,
    sym::K_SLANT,
    sym::K_UNDERLINE,
    sym::K_INVERSE_VIDEO,
    sym::K_FOREGROUND,
    sym::K_BACKGROUND,
    sym::K_STIPPLE,
    sym::K_OVERLINE,
    sym::K_STRIKE_THROUGH,
    sym::K_BOX,
    sym::K_FONT,
    sym::K_INHERIT,
    sym::K_FONTSET,
    sym::K_DISTANT_FOREGROUND,
    sym::K_EXTEND,
];

const XLFD_WEIGHT: i32 = 2;
const XLFD_SLANT: i32 = 3;
const XLFD_SWIDTH: i32 = 4;
const XLFD_POINT_SIZE: i32 = 7;

const FACE_TTY_DEFAULT_COLOR: i64 = -1;
const CLEAR_FONT_TABLE_COUNT: i32 = 100;

pub const DEFAULT_FACE_ID: usize = 0;
pub const MODE_LINE_ACTIVE_FACE_ID: usize = 1;
pub const MODE_LINE_INACTIVE_FACE_ID: usize = 2;
pub const TOOL_BAR_FACE_ID: usize = 3;
pub const FRINGE_FACE_ID: usize = 4;
pub const HEADER_LINE_ACTIVE_FACE_ID: usize = 5;
pub const HEADER_LINE_INACTIVE_FACE_ID: usize = 6;
pub const SCROLL_BAR_FACE_ID: usize = 7;
pub const BORDER_FACE_ID: usize = 8;
pub const CURSOR_FACE_ID: usize = 9;
pub const MOUSE_FACE_ID: usize = 10;
pub const MENU_FACE_ID: usize = 11;
pub const VERTICAL_BORDER_FACE_ID: usize = 12;
pub const WINDOW_DIVIDER_FACE_ID: usize = 13;
pub const WINDOW_DIVIDER_FIRST_PIXEL_FACE_ID: usize = 14;
pub const WINDOW_DIVIDER_LAST_PIXEL_FACE_ID: usize = 15;
pub const INTERNAL_BORDER_FACE_ID: usize = 16;
pub const CHILD_FRAME_BORDER_FACE_ID: usize = 17;
pub const TAB_BAR_FACE_ID: usize = 18;
pub const TAB_LINE_ACTIVE_FACE_ID: usize = 19;
pub const TAB_LINE_INACTIVE_FACE_ID: usize = 20;
pub const MARGIN_FACE_ID: usize = 21;
pub const BASIC_FACE_ID_SENTINEL: usize = 22;

const FACE_CACHE_BUCKETS_SIZE: usize = 1009;

const BASIC_FACES: [(Value, usize); 21] = [
    (sym::MODE_LINE_ACTIVE, MODE_LINE_ACTIVE_FACE_ID),
    (sym::MODE_LINE_INACTIVE, MODE_LINE_INACTIVE_FACE_ID),
    (sym::TOOL_BAR, TOOL_BAR_FACE_ID),
    (sym::FRINGE, FRINGE_FACE_ID),
    (sym::HEADER_LINE_ACTIVE, HEADER_LINE_ACTIVE_FACE_ID),
    (sym::HEADER_LINE_INACTIVE, HEADER_LINE_INACTIVE_FACE_ID),
    (sym::SCROLL_BAR, SCROLL_BAR_FACE_ID),
    (sym::BORDER, BORDER_FACE_ID),
    (sym::CURSOR, CURSOR_FACE_ID),
    (sym::MOUSE, MOUSE_FACE_ID),
    (sym::MENU, MENU_FACE_ID),
    (sym::VERTICAL_BORDER, VERTICAL_BORDER_FACE_ID),
    (sym::WINDOW_DIVIDER, WINDOW_DIVIDER_FACE_ID),
    (
        sym::WINDOW_DIVIDER_FIRST_PIXEL,
        WINDOW_DIVIDER_FIRST_PIXEL_FACE_ID,
    ),
    (
        sym::WINDOW_DIVIDER_LAST_PIXEL,
        WINDOW_DIVIDER_LAST_PIXEL_FACE_ID,
    ),
    (sym::INTERNAL_BORDER, INTERNAL_BORDER_FACE_ID),
    (sym::CHILD_FRAME_BORDER, CHILD_FRAME_BORDER_FACE_ID),
    (sym::TAB_BAR, TAB_BAR_FACE_ID),
    (sym::TAB_LINE_ACTIVE, TAB_LINE_ACTIVE_FACE_ID),
    (sym::TAB_LINE_INACTIVE, TAB_LINE_INACTIVE_FACE_ID),
    (sym::MARGIN, MARGIN_FACE_ID),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FaceBoxType {
    #[default]
    NoBox,
    SimpleBox,
    RaisedBox,
    SunkenBox,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FaceUnderlineType {
    #[default]
    NoUnderline,
    Single,
    DoubleLine,
    Wave,
    Dots,
    Dashes,
}

#[derive(Clone, Debug)]
pub struct Face {
    pub lface: [Value; LFACE_VECTOR_SIZE],
    pub id: usize,
    pub foreground: u64,
    pub background: u64,
    pub underline_color: u64,
    pub overline_color: u64,
    pub strike_through_color: u64,
    pub box_color: u64,
    pub font: Value,
    pub fontset: i32,
    pub box_vertical_line_width: i32,
    pub box_horizontal_line_width: i32,
    pub underline_pixels_above_descent_line: i32,
    pub box_type: FaceBoxType,
    pub underline: FaceUnderlineType,
    pub use_box_color_for_shadows_p: bool,
    pub overline_p: bool,
    pub strike_through_p: bool,
    pub foreground_defaulted_p: bool,
    pub background_defaulted_p: bool,
    pub underline_defaulted_p: bool,
    pub overline_color_defaulted_p: bool,
    pub strike_through_color_defaulted_p: bool,
    pub box_color_defaulted_p: bool,
    pub underline_at_descent_line_p: bool,
    pub tty_bold_p: bool,
    pub tty_italic_p: bool,
    pub tty_reverse_p: bool,
    pub tty_strike_through_p: bool,
    pub tty_overline_p: bool,
    pub colors_copied_bitwise_p: bool,
    pub overstrike: bool,
    pub hash: u64,
    pub ascii_face: usize,
}

impl Face {
    fn make_realized(attr: [Value; LFACE_VECTOR_SIZE]) -> Face {
        Face {
            lface: attr,
            id: 0,
            foreground: 0,
            background: 0,
            underline_color: 0,
            overline_color: 0,
            strike_through_color: 0,
            box_color: 0,
            font: Value::NIL,
            fontset: 0,
            box_vertical_line_width: 0,
            box_horizontal_line_width: 0,
            underline_pixels_above_descent_line: 0,
            box_type: FaceBoxType::NoBox,
            underline: FaceUnderlineType::NoUnderline,
            use_box_color_for_shadows_p: false,
            overline_p: false,
            strike_through_p: false,
            foreground_defaulted_p: false,
            background_defaulted_p: false,
            underline_defaulted_p: false,
            overline_color_defaulted_p: false,
            strike_through_color_defaulted_p: false,
            box_color_defaulted_p: false,
            underline_at_descent_line_p: false,
            tty_bold_p: false,
            tty_italic_p: false,
            tty_reverse_p: false,
            tty_strike_through_p: false,
            tty_overline_p: false,
            colors_copied_bitwise_p: false,
            overstrike: false,
            hash: 0,
            ascii_face: usize::MAX,
        }
    }

    fn is_ascii(&self) -> bool {
        self.ascii_face == self.id
    }
}

#[derive(Clone, Debug)]
pub struct FaceCache {
    pub buckets: Vec<Vec<usize>>,
    pub faces_by_id: Vec<Option<Box<Face>>>,
    pub menu_face_changed_p: bool,
}

impl FaceCache {
    pub fn used(&self) -> usize {
        self.faces_by_id.len()
    }

    pub fn face(&self, id: usize) -> Option<&Face> {
        self.faces_by_id.get(id).and_then(|f| f.as_deref())
    }

    pub fn trace(&self, out: &mut Vec<Value>) {
        for face in self.faces_by_id.iter().flatten() {
            out.extend_from_slice(&face.lface);
            out.push(face.font);
        }
    }

    fn cache_face(&mut self, mut face: Face, hash: u64) -> usize {
        let i = (hash % FACE_CACHE_BUCKETS_SIZE as u64) as usize;
        face.hash = hash;
        let id = self
            .faces_by_id
            .iter()
            .position(|f| f.is_none())
            .unwrap_or(self.faces_by_id.len());
        let ascii = face.ascii_face == usize::MAX;
        face.id = id;
        if ascii {
            face.ascii_face = id;
            self.buckets[i].insert(0, id);
        } else {
            self.buckets[i].push(id);
        }
        if id == self.faces_by_id.len() {
            self.faces_by_id.push(Some(Box::new(face)));
        } else {
            self.faces_by_id[id] = Some(Box::new(face));
        }
        id
    }

    fn uncache_face(&mut self, id: usize) {
        if let Some(face) = self.face(id) {
            let i = (face.hash % FACE_CACHE_BUCKETS_SIZE as u64) as usize;
            self.buckets[i].retain(|&x| x != id);
            self.faces_by_id[id] = None;
        }
    }
}

fn hash_string_case_insensitive(lisp: &Lisp, string: Value) -> u64 {
    if !string.is_string() {
        return 0;
    }
    let mut hash: u64 = 0;
    for &b in lisp.string_bytes(string) {
        if b == 0 {
            break;
        }
        hash = (hash << 1) ^ b.to_ascii_lowercase() as u64;
    }
    hash
}

fn lface_hash(lisp: &Lisp, v: &[Value; LFACE_VECTOR_SIZE]) -> u64 {
    hash_string_case_insensitive(lisp, v[LFACE_FAMILY_INDEX])
        ^ hash_string_case_insensitive(lisp, v[LFACE_FOUNDRY_INDEX])
        ^ hash_string_case_insensitive(lisp, v[LFACE_FOREGROUND_INDEX])
        ^ hash_string_case_insensitive(lisp, v[LFACE_BACKGROUND_INDEX])
        ^ v[LFACE_WEIGHT_INDEX].bits()
        ^ v[LFACE_SLANT_INDEX].bits()
        ^ v[LFACE_SWIDTH_INDEX].bits()
        ^ v[LFACE_HEIGHT_INDEX].bits()
}

pub struct Faces {
    pub lface_id_to_name: Vec<Value>,
    pub face_change: bool,
    pub menu_face_changed_default: bool,
    pub alternative_font_family_alist: Value,
    pub alternative_font_registry_alist: Value,
    pub font_sort_order: [i32; 4],
    pub tty_suppress_bold_inverse_default_colors_p: bool,
    pub clear_font_table_count: i32,
}

impl Default for Faces {
    fn default() -> Self {
        Faces {
            clear_font_table_count: 0,
            lface_id_to_name: Vec::new(),
            face_change: false,
            menu_face_changed_default: false,
            alternative_font_family_alist: Value::NIL,
            alternative_font_registry_alist: Value::NIL,
            font_sort_order: [0; 4],
            tty_suppress_bold_inverse_default_colors_p: false,
        }
    }
}

impl Faces {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&self.lface_id_to_name);
        out.push(self.alternative_font_family_alist);
        out.push(self.alternative_font_registry_alist);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NamedMergePointKind {
    Normal,
    Remap,
}

type MergePoints = Vec<(Value, NamedMergePointKind)>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EmacsColor {
    pub pixel: i64,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("internal-make-lisp-face", 1, 2, f_internal_make_lisp_face),
    Subr::fixed("internal-lisp-face-p", 1, 2, f_internal_lisp_face_p),
    Subr::fixed(
        "internal-set-lisp-face-attribute",
        3,
        4,
        f_internal_set_lisp_face_attribute,
    ),
    Subr::fixed(
        "internal-set-lisp-face-attribute-from-resource",
        3,
        4,
        f_internal_set_lisp_face_attribute_from_resource,
    ),
    Subr::fixed("color-gray-p", 1, 2, f_color_gray_p),
    Subr::fixed("color-supported-p", 1, 3, f_color_supported_p),
    Subr::fixed("x-load-color-file", 1, 1, f_x_load_color_file),
    Subr::fixed(
        "face-attribute-relative-p",
        2,
        2,
        f_face_attribute_relative_p,
    ),
    Subr::fixed("merge-face-attribute", 3, 3, f_merge_face_attribute),
    Subr::fixed(
        "internal-get-lisp-face-attribute",
        2,
        3,
        f_internal_get_lisp_face_attribute,
    ),
    Subr::fixed(
        "internal-lisp-face-attribute-values",
        1,
        1,
        f_internal_lisp_face_attribute_values,
    ),
    Subr::fixed(
        "internal-lisp-face-equal-p",
        2,
        4,
        f_internal_lisp_face_equal_p,
    ),
    Subr::fixed(
        "internal-lisp-face-empty-p",
        1,
        2,
        f_internal_lisp_face_empty_p,
    ),
    Subr::fixed("internal-copy-lisp-face", 4, 4, f_internal_copy_lisp_face),
    Subr::fixed(
        "internal-merge-in-global-face",
        2,
        2,
        f_internal_merge_in_global_face,
    ),
    Subr::fixed("frame--face-hash-table", 0, 1, f_frame_face_hash_table),
    Subr::fixed(
        "display-supports-face-attributes-p",
        1,
        2,
        f_display_supports_face_attributes_p,
    ),
    Subr::fixed("color-distance", 2, 4, f_color_distance),
    Subr::fixed(
        "internal-set-font-selection-order",
        1,
        1,
        f_internal_set_font_selection_order,
    ),
    Subr::fixed(
        "internal-set-alternative-font-family-alist",
        1,
        1,
        f_internal_set_alternative_font_family_alist,
    ),
    Subr::fixed(
        "internal-set-alternative-font-registry-alist",
        1,
        1,
        f_internal_set_alternative_font_registry_alist,
    ),
    Subr::fixed(
        "face-attributes-as-vector",
        1,
        1,
        f_face_attributes_as_vector,
    ),
    Subr::fixed(
        "tty-suppress-bold-inverse-default-colors",
        1,
        1,
        f_tty_suppress_bold_inverse_default_colors,
    ),
    Subr::fixed("bitmap-spec-p", 1, 1, f_bitmap_spec_p),
    Subr::fixed("clear-face-cache", 0, 1, f_clear_face_cache),
    Subr::fixed(
        "color-values-from-color-spec",
        1,
        1,
        f_color_values_from_color_spec,
    ),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_bool(sym::FACE_FILTERS_ALWAYS_MATCH, false);
    let defaults = lisp.make_hash_table_with(sym::EQ, Weakness::None);
    lisp.defvar(sym::FACE__NEW_FRAME_DEFAULTS, defaults);
    let stipple = lisp.string("gray3");
    lisp.defvar(sym::FACE_DEFAULT_STIPPLE, stipple);
    lisp.defvar(sym::TTY_DEFINED_COLOR_ALIST, Value::NIL);
    lisp.defvar(sym::SCALABLE_FONTS_ALLOWED, Value::NIL);
    lisp.defvar(sym::FACE_IGNORED_FONTS, Value::NIL);
    lisp.defvar(sym::FACE_REMAPPING_ALIST, Value::NIL);
    lisp.make_variable_buffer_local_builtin(sym::FACE_REMAPPING_ALIST);
    lisp.defvar(sym::FACE_FONT_RESCALE_ALIST, Value::NIL);
    lisp.defvar_int(sym::FACE_NEAR_SAME_COLOR_THRESHOLD, 30000);
    lisp.defvar(sym::FACE_FONT_LAX_MATCHED_ATTRIBUTES, sym::T);
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn unspecifiedp(v: Value) -> bool {
    v == sym::UNSPECIFIED
}

fn ignore_deffacep(v: Value) -> bool {
    v == sym::K_IGNORE_DEFFACE
}

fn resetp(v: Value) -> bool {
    v == sym::RESET
}

fn make_fixnum_wrapping(n: i64) -> Value {
    Value::fixnum((n << 2) >> 2)
}

fn push_named_merge_point(
    points: &mut MergePoints,
    face_name: Value,
    kind: NamedMergePointKind,
) -> bool {
    for &(name, prev_kind) in points.iter().rev() {
        if name == face_name {
            if prev_kind == kind {
                return false;
            } else if prev_kind == NamedMergePointKind::Remap {
                break;
            }
        }
    }
    points.push((face_name, kind));
    true
}

fn lface_font_prop(i: usize) -> usize {
    match i {
        LFACE_FAMILY_INDEX => FONT_FAMILY_INDEX,
        LFACE_FOUNDRY_INDEX => FONT_FOUNDRY_INDEX,
        LFACE_SWIDTH_INDEX => FONT_WIDTH_INDEX,
        LFACE_HEIGHT_INDEX => FONT_SIZE_INDEX,
        LFACE_WEIGHT_INDEX => FONT_WEIGHT_INDEX,
        _ => FONT_SLANT_INDEX,
    }
}

fn lface_fully_specified_p(attrs: &[Value]) -> bool {
    (1..N).all(|i| {
        i == LFACE_FONT_INDEX
            || i == LFACE_INHERIT_INDEX
            || i == LFACE_DISTANT_FOREGROUND_INDEX
            || !(unspecifiedp(attrs[i]) || ignore_deffacep(attrs[i]))
    })
}

fn parse_hex_color_comp(s: &[u8]) -> Option<u16> {
    let n = s.len();
    if n == 0 || n > 4 {
        return None;
    }
    let mut val: u32 = 0;
    for &c in s {
        let digit = match c {
            b'0'..=b'9' => c - b'0',
            b'A'..=b'F' => c - b'A' + 10,
            b'a'..=b'f' => c - b'a' + 10,
            _ => return None,
        };
        val = (val << 4) | u32::from(digit);
    }
    let maxval = (1u32 << (n * 4)) - 1;
    Some((val * 65535 / maxval) as u16)
}

fn parse_float_color_comp(s: &[u8]) -> Option<u16> {
    if !s
        .iter()
        .all(|&c| c.is_ascii_digit() || matches!(c, b'.' | b'+' | b'-' | b'e' | b'E'))
    {
        return None;
    }
    let x = if s.is_empty() {
        0.0
    } else {
        std::str::from_utf8(s).ok()?.parse::<f64>().ok()?
    };
    if (0.0..=1.0).contains(&x) {
        Some((x * 65535.0).round_ties_even() as u16)
    } else {
        None
    }
}

fn find_byte(s: &[u8], c: u8) -> Option<usize> {
    s.iter().position(|&b| b == c)
}

pub fn parse_color_spec(spec: &[u8]) -> Option<(u16, u16, u16)> {
    let spec = crate::fontset::c_bytes(spec);
    let len = spec.len();
    if spec.first() == Some(&b'#') {
        if (len - 1).is_multiple_of(3) {
            let n = (len - 1) / 3;
            let r = parse_hex_color_comp(&spec[1..1 + n])?;
            let g = parse_hex_color_comp(&spec[1 + n..1 + 2 * n])?;
            let b = parse_hex_color_comp(&spec[1 + 2 * n..1 + 3 * n])?;
            return Some((r, g, b));
        }
    } else if let Some(rest) = spec.strip_prefix(b"rgb:") {
        let sep1 = find_byte(rest, b'/')?;
        let sep2 = sep1 + 1 + find_byte(&rest[sep1 + 1..], b'/')?;
        let r = parse_hex_color_comp(&rest[..sep1])?;
        let g = parse_hex_color_comp(&rest[sep1 + 1..sep2])?;
        let b = parse_hex_color_comp(&rest[sep2 + 1..])?;
        return Some((r, g, b));
    } else if let Some(rest) = spec.strip_prefix(b"rgbi:") {
        let sep1 = find_byte(rest, b'/')?;
        let sep2 = sep1 + 1 + find_byte(&rest[sep1 + 1..], b'/')?;
        let r = parse_float_color_comp(&rest[..sep1])?;
        let g = parse_float_color_comp(&rest[sep1 + 1..sep2])?;
        let b = parse_float_color_comp(&rest[sep2 + 1..])?;
        return Some((r, g, b));
    }
    None
}

fn color_distance(x: &EmacsColor, y: &EmacsColor) -> i64 {
    let r = i64::from(x.red) - i64::from(y.red);
    let g = i64::from(x.green) - i64::from(y.green);
    let b = i64::from(x.blue) - i64::from(y.blue);
    let r_mean = (i64::from(x.red) + i64::from(y.red)) >> 1;
    ((((2 * 65536 + r_mean) * r * r) >> 16)
        + 4 * g * g
        + (((2 * 65536 + 65535 - r_mean) * b * b) >> 16))
        >> 16
}

impl Lisp {
    pub fn face_new_frame_defaults(&self) -> Value {
        self.find_symbol_value(sym::FACE__NEW_FRAME_DEFAULTS)
    }

    pub fn lfacep(&self, lface: Value) -> bool {
        self.is_vector(lface)
            && self.vector_items(lface).len() == LFACE_VECTOR_SIZE
            && self.vector_items(lface)[0] == sym::FACE
    }

    fn lface_aref(&self, lface: Value, i: usize) -> Value {
        self.vector_items(lface)[i]
    }

    fn lface_aset(&mut self, lface: Value, i: usize, v: Value) {
        if let Vectorlike::Vector(items) = self.heap.vectorlike_mut(lface) {
            items[i] = v;
        }
    }

    fn lface_contents(&self, lface: Value) -> [Value; N] {
        let mut out = [Value::NIL; N];
        out.copy_from_slice(&self.vector_items(lface)[..N]);
        out
    }

    fn make_lface_vector(&mut self) -> Value {
        let mut items = vec![sym::UNSPECIFIED; N];
        items[0] = sym::FACE;
        self.vector(items)
    }

    fn attrs_push(&mut self, fill: Value) -> usize {
        let base = self.stack.len();
        self.stack.extend(std::iter::repeat_n(fill, N));
        base
    }

    fn attrs_push_from(&mut self, src: &[Value]) -> usize {
        let base = self.stack.len();
        self.stack.extend_from_slice(&src[..N]);
        base
    }

    fn at(&self, a: usize, i: usize) -> Value {
        self.stack[a + i]
    }

    fn set_at(&mut self, a: usize, i: usize, v: Value) {
        self.stack[a + i] = v;
    }

    fn attrs_snapshot(&self, a: usize) -> [Value; N] {
        let mut out = [Value::NIL; N];
        out.copy_from_slice(&self.stack[a..a + N]);
        out
    }

    fn attrs_font_clear_prop(&mut self, a: usize, prop: usize) -> LispResult<()> {
        let mut attrs = self.attrs_snapshot(a);
        self.font_clear_prop(&mut attrs, prop)?;
        self.stack[a..a + N].copy_from_slice(&attrs);
        Ok(())
    }

    fn lface_font_clear_prop(&mut self, lface: Value, prop: usize) -> LispResult<()> {
        let mut attrs = self.lface_contents(lface);
        self.font_clear_prop(&mut attrs, prop)?;
        if let Vectorlike::Vector(items) = self.heap.vectorlike_mut(lface) {
            items[..N].copy_from_slice(&attrs);
        }
        Ok(())
    }

    pub fn resolve_face_name(&mut self, face_name: Value, signal_p: bool) -> LispResult {
        let face_name = if self.is_string(face_name) {
            crate::lread::f_intern(self, &[face_name, Value::NIL])?
        } else {
            face_name
        };
        if face_name.is_nil() || !face_name.is_symbol() {
            return Ok(face_name);
        }
        let orig_face = face_name;
        let mut tortoise = face_name;
        let mut hare = face_name;
        let mut face_name;
        loop {
            face_name = hare;
            hare = self.get(hare, sym::FACE_ALIAS);
            if hare.is_nil() || !hare.is_symbol() {
                break;
            }
            face_name = hare;
            hare = self.get(hare, sym::FACE_ALIAS);
            if hare.is_nil() || !hare.is_symbol() {
                break;
            }
            tortoise = self.get(tortoise, sym::FACE_ALIAS);
            if hare == tortoise {
                if signal_p {
                    return Err(self.signal_list(sym::CIRCULAR_LIST, &[orig_face]));
                }
                return Ok(sym::DEFAULT);
            }
        }
        Ok(face_name)
    }

    pub fn lface_from_face_name_no_resolve(
        &mut self,
        f: Option<Value>,
        face_name: Value,
        signal_p: bool,
    ) -> LispResult {
        let lface = match f {
            Some(f) => {
                let table = self.frame(f).face_hash_table;
                self.gethash(face_name, table, Value::NIL)?
            }
            None => {
                let table = self.face_new_frame_defaults();
                let entry = self.gethash(face_name, table, Value::NIL)?;
                self.cdr(entry)?
            }
        };
        if signal_p && lface.is_nil() {
            return Err(self.signal_error("Invalid face", face_name));
        }
        Ok(lface)
    }

    pub fn lface_from_face_name(
        &mut self,
        f: Option<Value>,
        face_name: Value,
        signal_p: bool,
    ) -> LispResult {
        let face_name = self.resolve_face_name(face_name, signal_p)?;
        self.lface_from_face_name_no_resolve(f, face_name, signal_p)
    }

    fn get_lface_attributes_no_remap(
        &mut self,
        f: Option<Value>,
        face_name: Value,
        attrs: usize,
        signal_p: bool,
    ) -> LispResult<bool> {
        let lface = self.lface_from_face_name_no_resolve(f, face_name, signal_p)?;
        if !lface.is_nil() {
            let contents = self.lface_contents(lface);
            self.stack[attrs..attrs + N].copy_from_slice(&contents);
        }
        Ok(!lface.is_nil())
    }

    fn get_lface_attributes(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        face_name: Value,
        attrs: usize,
        signal_p: bool,
        points: &mut MergePoints,
    ) -> LispResult<bool> {
        let face_name = self.resolve_face_name(face_name, signal_p)?;
        if let Some(fr) = f {
            let alist = self.find_symbol_value(sym::FACE_REMAPPING_ALIST);
            let face_remapping = self.assq(face_name, alist);
            if face_remapping.is_cons() {
                let depth = points.len();
                if push_named_merge_point(points, face_name, NamedMergePointKind::Remap) {
                    for i in 1..N {
                        self.set_at(attrs, i, sym::UNSPECIFIED);
                    }
                    let spec = self.xcdr(face_remapping);
                    let r = self.merge_face_ref(w, Some(fr), spec, attrs, signal_p, points, 0);
                    points.truncate(depth);
                    return r;
                }
            }
        }
        self.get_lface_attributes_no_remap(f, face_name, attrs, signal_p)
    }

    fn merge_face_heights(&mut self, from: Value, to: Value, invalid: Value) -> LispResult {
        let mut result = invalid;
        if from.is_fixnum() {
            result = from;
        } else if from.is_float() {
            let scale = self.heap.float_value(from);
            if let Some(t) = to.as_fixnum() {
                result = make_fixnum_wrapping((scale * t as f64) as i64);
            } else if to.is_float() {
                let t = self.heap.float_value(to);
                result = self.float(scale * t);
            } else if unspecifiedp(to) {
                result = from;
            }
        } else if self.functionp(from) {
            result = self.safe_calln(from, &[to])?;
            if to.is_fixnum() && !result.is_fixnum() {
                result = invalid;
            }
        }
        Ok(result)
    }

    fn merge_face_vectors(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        from: usize,
        to: usize,
        points: &mut MergePoints,
    ) -> LispResult<()> {
        let base = self.stack.len();
        let r = self.merge_face_vectors_1(w, f, from, to, points);
        self.stack.truncate(base);
        r
    }

    fn merge_face_vectors_1(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        from: usize,
        to: usize,
        points: &mut MergePoints,
    ) -> LispResult<()> {
        let inherit = self.at(from, LFACE_INHERIT_INDEX);
        if !unspecifiedp(inherit) && !inherit.is_nil() {
            self.merge_face_ref(w, f, inherit, to, false, points, 0)?;
        }
        let mut font = Value::NIL;
        let from_font = self.at(from, LFACE_FONT_INDEX);
        if self.font_spec_p(from_font) {
            let to_font = self.at(to, LFACE_FONT_INDEX);
            font = if !unspecifiedp(to_font) {
                self.merge_font_spec(from_font, to_font)?
            } else {
                self.copy_font_spec(from_font)?
            };
            self.set_at(to, LFACE_FONT_INDEX, font);
        }
        self.stack.push(font);
        for i in 1..N {
            let fv = self.at(from, i);
            if unspecifiedp(fv) {
                continue;
            }
            if i == LFACE_HEIGHT_INDEX && !fv.is_fixnum() {
                let tv = self.at(to, i);
                let h = self.merge_face_heights(fv, tv, tv)?;
                self.set_at(to, i, h);
                self.attrs_font_clear_prop(to, FONT_SIZE_INDEX)?;
            } else if i != LFACE_FONT_INDEX && self.at(to, i) != fv {
                self.set_at(to, i, fv);
                if (LFACE_FAMILY_INDEX..=LFACE_SLANT_INDEX).contains(&i) {
                    self.attrs_font_clear_prop(to, lface_font_prop(i))?;
                }
            }
        }
        if !font.is_nil() {
            let foundry = self.font_aref(font, FONT_FOUNDRY_INDEX);
            if !foundry.is_nil() {
                self.check_symbol(foundry)?;
                let name = self.symbol_name(foundry);
                self.set_at(to, LFACE_FOUNDRY_INDEX, name);
            }
            let family = self.font_aref(font, FONT_FAMILY_INDEX);
            if !family.is_nil() {
                self.check_symbol(family)?;
                let name = self.symbol_name(family);
                self.set_at(to, LFACE_FAMILY_INDEX, name);
            }
            for (fi, li) in [
                (FONT_WEIGHT_INDEX, LFACE_WEIGHT_INDEX),
                (FONT_SLANT_INDEX, LFACE_SLANT_INDEX),
                (FONT_WIDTH_INDEX, LFACE_SWIDTH_INDEX),
            ] {
                if !self.font_aref(font, fi).is_nil() {
                    let v = self.font_style_symbolic(font, fi, true);
                    self.set_at(to, li, v);
                }
            }
            let adstyle = self.font_aref(font, FONT_ADSTYLE_INDEX);
            if !adstyle.is_nil() {
                let tospec = self.at(to, LFACE_FONT_INDEX);
                let tospec = if !tospec.is_nil() {
                    self.copy_font_spec(tospec)?
                } else {
                    crate::font::f_font_spec(self, &[])?
                };
                self.set_at(to, LFACE_FONT_INDEX, tospec);
                self.font_aset(tospec, FONT_ADSTYLE_INDEX, adstyle);
            }
            self.font_aset(font, FONT_SIZE_INDEX, Value::NIL);
        }
        self.set_at(to, LFACE_INHERIT_INDEX, Value::NIL);
        Ok(())
    }

    fn face_inherited_attr(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        attrs: usize,
        attr_idx: usize,
        points: &mut MergePoints,
    ) -> LispResult {
        let base = self.stack.len();
        let r = self.face_inherited_attr_1(w, f, attrs, attr_idx, points);
        self.stack.truncate(base);
        r
    }

    fn face_inherited_attr_1(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        attrs: usize,
        attr_idx: usize,
        points: &mut MergePoints,
    ) -> LispResult {
        let snapshot = self.attrs_snapshot(attrs);
        let inherited = self.attrs_push_from(&snapshot);
        let mut attr_val = self.at(attrs, attr_idx);
        while unspecifiedp(attr_val)
            && !self.at(inherited, LFACE_INHERIT_INDEX).is_nil()
            && !unspecifiedp(self.at(inherited, LFACE_INHERIT_INDEX))
        {
            let parent_face = self.at(inherited, LFACE_INHERIT_INDEX);
            self.stack.push(parent_face);
            if parent_face.is_cons() {
                let mut ok = false;
                let mut tail = parent_face;
                while !tail.is_nil() {
                    let face = self.car_safe(tail);
                    ok = self.get_lface_attributes(w, f, face, inherited, false, points)?;
                    if !ok {
                        break;
                    }
                    attr_val = self.face_inherited_attr(w, f, inherited, attr_idx, points)?;
                    if !unspecifiedp(attr_val) {
                        break;
                    }
                    tail = self.cdr_safe(tail);
                }
                if !ok {
                    break;
                }
            } else {
                let ok = self.get_lface_attributes(w, f, parent_face, inherited, false, points)?;
                if !ok {
                    break;
                }
                attr_val = self.at(inherited, attr_idx);
            }
        }
        Ok(attr_val)
    }

    fn face_inheritance_cycle(
        &mut self,
        f: Option<Value>,
        face: Value,
        child: Value,
    ) -> LispResult<bool> {
        let base = self.stack.len();
        let r = self.face_inheritance_cycle_1(f, face, child);
        self.stack.truncate(base);
        r
    }

    fn face_inheritance_cycle_1(
        &mut self,
        f: Option<Value>,
        face: Value,
        child: Value,
    ) -> LispResult<bool> {
        let face_attrs = self.attrs_push(sym::UNSPECIFIED);
        let mut cycle_found = false;
        let continues =
            |p: Value| !p.is_nil() && !unspecifiedp(p) && !ignore_deffacep(p) && !resetp(p);
        if face.is_cons() {
            let mut tail = face;
            while tail.is_cons() {
                let member_face = self.xcar(tail);
                let mut points = MergePoints::new();
                let ok = self.get_lface_attributes(
                    None,
                    f,
                    member_face,
                    face_attrs,
                    false,
                    &mut points,
                )?;
                if !ok {
                    break;
                }
                let parent_face = self.at(face_attrs, LFACE_INHERIT_INDEX);
                if parent_face == member_face || parent_face == child {
                    cycle_found = true;
                } else if continues(parent_face) {
                    cycle_found = self.face_inheritance_cycle(f, parent_face, child)?;
                }
                if cycle_found {
                    break;
                }
                tail = self.xcdr(tail);
            }
        } else if face.is_symbol() {
            let mut points = MergePoints::new();
            let ok = self.get_lface_attributes(None, f, face, face_attrs, false, &mut points)?;
            if ok {
                let parent_face = self.at(face_attrs, LFACE_INHERIT_INDEX);
                if parent_face == face || parent_face == child {
                    cycle_found = true;
                } else if continues(parent_face) {
                    cycle_found = self.face_inheritance_cycle(f, parent_face, child)?;
                }
            }
        }
        Ok(cycle_found)
    }

    fn merge_named_face(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        face_name: Value,
        to: usize,
        points: &mut MergePoints,
        attr_filter: usize,
    ) -> LispResult<bool> {
        let depth = points.len();
        if !push_named_merge_point(points, face_name, NamedMergePointKind::Normal) {
            return Ok(false);
        }
        let base = self.stack.len();
        let r = self.merge_named_face_1(w, f, face_name, to, points, attr_filter);
        self.stack.truncate(base);
        points.truncate(depth);
        r
    }

    fn merge_named_face_1(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        face_name: Value,
        to: usize,
        points: &mut MergePoints,
        attr_filter: usize,
    ) -> LispResult<bool> {
        let from = self.attrs_push(sym::UNSPECIFIED);
        let ok = self.get_lface_attributes(w, f, face_name, from, false, points)?;
        if ok && face_name != sym::DEFAULT {
            let mut deflt: Option<[Value; N]> = None;
            for i in 1..N {
                if resetp(self.at(from, i)) {
                    let d = match deflt {
                        Some(d) => d,
                        None => {
                            let frame = f.expect("merging a reset attribute needs a frame");
                            let d = self
                                .face_from_id(frame, DEFAULT_FACE_ID)
                                .expect("default face is not realized")
                                .lface;
                            deflt = Some(d);
                            d
                        }
                    };
                    self.set_at(from, i, d[i]);
                }
            }
        }
        let merge = ok
            && (attr_filter == 0
                || (!self.at(from, attr_filter).is_nil()
                    && !unspecifiedp(self.at(from, attr_filter)))
                || (!self.at(from, LFACE_INHERIT_INDEX).is_nil()
                    && !unspecifiedp(self.at(from, LFACE_INHERIT_INDEX))
                    && {
                        let val = self.face_inherited_attr(w, f, from, attr_filter, points)?;
                        !val.is_nil() && !unspecifiedp(val)
                    }));
        if merge {
            self.merge_face_vectors(w, f, from, to, points)?;
        }
        Ok(ok)
    }

    fn evaluate_face_filter(
        &mut self,
        filter: Value,
        w: Option<Value>,
        ok: &mut bool,
        err_msgs: bool,
    ) -> LispResult<bool> {
        let orig_filter = filter;
        if filter.is_nil() {
            return Ok(true);
        }
        if self.var_true(sym::FACE_FILTERS_ALWAYS_MATCH) {
            return Ok(true);
        }
        'err: {
            if !filter.is_cons() || self.xcar(filter) != sym::K_WINDOW {
                break 'err;
            }
            let filter = self.xcdr(filter);
            let parameter = self.car_safe(filter);
            let filter = self.cdr_safe(filter);
            if !filter.is_cons() {
                break 'err;
            }
            let value = self.xcar(filter);
            if !self.xcdr(filter).is_nil() {
                break 'err;
            }
            self.check_symbol(parameter)?;
            if self.get(parameter, sym::K_FILTERED).is_nil() {
                self.put(parameter, sym::K_FILTERED, sym::T);
            }
            let mut matched = false;
            if let Some(w) = w {
                let params = self.window(w).window_parameters;
                let found = self.assq(parameter, params);
                if !found.is_nil() && self.xcdr(found) == value {
                    matched = true;
                }
            }
            return Ok(matched);
        }
        if err_msgs {
            self.add_to_log("Invalid face filter %S", &[orig_filter])?;
        }
        *ok = false;
        Ok(false)
    }

    fn filter_face_ref(
        &mut self,
        face_ref: Value,
        w: Option<Value>,
        ok: &mut bool,
        err_msgs: bool,
    ) -> LispResult {
        let orig_face_ref = face_ref;
        if !face_ref.is_cons() {
            return Ok(face_ref);
        }
        'err: {
            if self.xcar(face_ref) != sym::K_FILTERED {
                return Ok(face_ref);
            }
            let face_ref = self.xcdr(face_ref);
            if !face_ref.is_cons() {
                break 'err;
            }
            let filter = self.xcar(face_ref);
            let face_ref = self.xcdr(face_ref);
            if !face_ref.is_cons() {
                break 'err;
            }
            let filtered_face_ref = self.xcar(face_ref);
            if !self.xcdr(face_ref).is_nil() {
                break 'err;
            }
            return Ok(if self.evaluate_face_filter(filter, w, ok, err_msgs)? {
                filtered_face_ref
            } else {
                Value::NIL
            });
        }
        if err_msgs {
            self.add_to_log("Invalid face ref %S", &[orig_face_ref])?;
        }
        *ok = false;
        Ok(Value::NIL)
    }

    #[allow(clippy::too_many_arguments)]
    fn merge_face_ref(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        face_ref: Value,
        to: usize,
        err_msgs: bool,
        points: &mut MergePoints,
        attr_filter: usize,
    ) -> LispResult<bool> {
        let base = self.stack.len();
        self.stack.push(face_ref);
        let r = self.merge_face_ref_1(w, f, face_ref, to, err_msgs, points, attr_filter);
        self.stack.truncate(base);
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn merge_face_ref_1(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        face_ref: Value,
        to: usize,
        err_msgs: bool,
        points: &mut MergePoints,
        attr_filter: usize,
    ) -> LispResult<bool> {
        let mut ok = true;
        let mut attr_filter_passed = false;
        let mut filtered_face_ref = face_ref;
        let mut face_ref;
        loop {
            face_ref = filtered_face_ref;
            filtered_face_ref = self.filter_face_ref(face_ref, w, &mut ok, err_msgs)?;
            if !(ok && face_ref != filtered_face_ref) {
                break;
            }
        }
        if !ok {
            return Ok(false);
        }
        if face_ref.is_nil() {
            return Ok(true);
        }
        self.stack.push(face_ref);
        if face_ref.is_cons() {
            let first = self.xcar(face_ref);
            if first == sym::FOREGROUND_COLOR || first == sym::BACKGROUND_COLOR {
                let color_name = self.xcdr(face_ref);
                if color_name.is_string() {
                    if first == sym::FOREGROUND_COLOR {
                        self.set_at(to, LFACE_FOREGROUND_INDEX, color_name);
                    } else {
                        self.set_at(to, LFACE_BACKGROUND_INDEX, color_name);
                    }
                } else {
                    if err_msgs {
                        self.add_to_log("Invalid face color %S", &[color_name])?;
                    }
                    ok = false;
                }
            } else if first.is_symbol()
                && self.string_bytes(self.symbol_name(first)).first() == Some(&b':')
            {
                if attr_filter > 0 {
                    let mut parent_face = Value::NIL;
                    let mut attr_filter_seen = false;
                    let mut tem = face_ref;
                    while tem.is_cons() && self.xcdr(tem).is_cons() {
                        let keyword = self.xcar(tem);
                        let value = self.xcar(self.xcdr(tem));
                        if keyword == FACE_ATTR_SYM[attr_filter] {
                            attr_filter_seen = true;
                            if value.is_nil() {
                                return Ok(true);
                            }
                        } else if keyword == sym::K_INHERIT {
                            parent_face = value;
                        }
                        tem = self.xcdr(self.xcdr(tem));
                    }
                    if !attr_filter_seen {
                        if parent_face.is_nil() {
                            return Ok(true);
                        }
                        let scratch = self.attrs_push(sym::UNSPECIFIED);
                        self.set_at(scratch, 0, sym::FACE);
                        if !self.merge_face_ref(w, f, parent_face, scratch, err_msgs, points, 0)? {
                            self.add_to_log(
                                "Invalid face attribute %S %S",
                                &[sym::K_INHERIT, parent_face],
                            )?;
                            return Ok(false);
                        }
                        let v = self.at(scratch, attr_filter);
                        if v.is_nil() || unspecifiedp(v) {
                            return Ok(true);
                        }
                    }
                    attr_filter_passed = true;
                }
                let mut tail = face_ref;
                while tail.is_cons() && self.xcdr(tail).is_cons() {
                    let keyword = self.xcar(tail);
                    let value = self.xcar(self.xcdr(tail));
                    let err = !self.merge_face_plist_attr(
                        w,
                        f,
                        keyword,
                        value,
                        to,
                        err_msgs,
                        points,
                        attr_filter,
                        attr_filter_passed,
                    )?;
                    if err {
                        self.add_to_log("Invalid face attribute %S %S", &[keyword, value])?;
                        ok = false;
                    }
                    tail = self.xcdr(self.xcdr(tail));
                }
            } else {
                let next = self.xcdr(face_ref);
                if !next.is_nil() {
                    ok = self.merge_face_ref(w, f, next, to, err_msgs, points, attr_filter)?;
                }
                if !self.merge_face_ref(w, f, first, to, err_msgs, points, attr_filter)? {
                    ok = false;
                }
            }
        } else {
            ok = self.merge_named_face(w, f, face_ref, to, points, attr_filter)?;
            if !ok && err_msgs {
                self.add_to_log("Invalid face reference: %s", &[face_ref])?;
            }
        }
        Ok(ok)
    }

    #[allow(clippy::too_many_arguments)]
    fn merge_face_plist_attr(
        &mut self,
        w: Option<Value>,
        f: Option<Value>,
        keyword: Value,
        value: Value,
        to: usize,
        err_msgs: bool,
        points: &mut MergePoints,
        attr_filter: usize,
        attr_filter_passed: bool,
    ) -> LispResult<bool> {
        let mut value = value;
        if value == sym::UNSPECIFIED {
            return Ok(true);
        }
        if keyword == sym::K_FAMILY || keyword == sym::K_FOUNDRY {
            if !value.is_string() {
                return Ok(false);
            }
            let (li, fi) = if keyword == sym::K_FAMILY {
                (LFACE_FAMILY_INDEX, FONT_FAMILY_INDEX)
            } else {
                (LFACE_FOUNDRY_INDEX, FONT_FOUNDRY_INDEX)
            };
            self.set_at(to, li, value);
            self.attrs_font_clear_prop(to, fi)?;
        } else if keyword == sym::K_HEIGHT {
            let old = self.at(to, LFACE_HEIGHT_INDEX);
            let new_height = self.merge_face_heights(value, old, Value::NIL)?;
            if new_height.is_nil() {
                return Ok(false);
            }
            self.set_at(to, LFACE_HEIGHT_INDEX, new_height);
            self.attrs_font_clear_prop(to, FONT_SIZE_INDEX)?;
        } else if keyword == sym::K_WEIGHT || keyword == sym::K_SLANT || keyword == sym::K_WIDTH {
            let (li, fi) = if keyword == sym::K_WEIGHT {
                (LFACE_WEIGHT_INDEX, FONT_WEIGHT_INDEX)
            } else if keyword == sym::K_SLANT {
                (LFACE_SLANT_INDEX, FONT_SLANT_INDEX)
            } else {
                (LFACE_SWIDTH_INDEX, FONT_WIDTH_INDEX)
            };
            if !value.is_symbol() || self.font_style_to_value(fi, value, false)? >> 8 < 0 {
                return Ok(false);
            }
            self.set_at(to, li, value);
            self.attrs_font_clear_prop(to, fi)?;
        } else if keyword == sym::K_UNDERLINE {
            if !(value == sym::T || value.is_nil() || value.is_string() || value.is_cons()) {
                return Ok(false);
            }
            self.set_at(to, LFACE_UNDERLINE_INDEX, value);
        } else if keyword == sym::K_OVERLINE || keyword == sym::K_STRIKE_THROUGH {
            if !(value == sym::T || value.is_nil() || value.is_string()) {
                return Ok(false);
            }
            let li = if keyword == sym::K_OVERLINE {
                LFACE_OVERLINE_INDEX
            } else {
                LFACE_STRIKE_THROUGH_INDEX
            };
            self.set_at(to, li, value);
        } else if keyword == sym::K_BOX {
            if value == sym::T {
                value = Value::fixnum(1);
            }
            if !((value.as_fixnum().is_some_and(|n| n != 0))
                || value.is_string()
                || value.is_cons()
                || value.is_nil())
            {
                return Ok(false);
            }
            self.set_at(to, LFACE_BOX_INDEX, value);
        } else if keyword == sym::K_INVERSE_VIDEO || keyword == sym::K_EXTEND {
            if !(value == sym::T || value.is_nil()) {
                return Ok(false);
            }
            let li = if keyword == sym::K_INVERSE_VIDEO {
                LFACE_INVERSE_INDEX
            } else {
                LFACE_EXTEND_INDEX
            };
            self.set_at(to, li, value);
        } else if keyword == sym::K_FOREGROUND
            || keyword == sym::K_DISTANT_FOREGROUND
            || keyword == sym::K_BACKGROUND
        {
            if !value.is_string() {
                return Ok(false);
            }
            let li = if keyword == sym::K_FOREGROUND {
                LFACE_FOREGROUND_INDEX
            } else if keyword == sym::K_DISTANT_FOREGROUND {
                LFACE_DISTANT_FOREGROUND_INDEX
            } else {
                LFACE_BACKGROUND_INDEX
            };
            self.set_at(to, li, value);
        } else if keyword == sym::K_STIPPLE {
            if !(value.is_nil() || !f_bitmap_spec_p(self, &[value])?.is_nil()) {
                return Ok(false);
            }
            self.set_at(to, LFACE_STIPPLE_INDEX, value);
        } else if keyword == sym::K_FONT {
            if !self.is_font(value) {
                return Ok(false);
            }
            self.set_at(to, LFACE_FONT_INDEX, value);
        } else if keyword == sym::K_INHERIT {
            let filter = if attr_filter_passed { 0 } else { attr_filter };
            if !self.merge_face_ref(w, f, value, to, err_msgs, points, filter)? {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn tty_color_name(&mut self, f: Value, idx: i64) -> LispResult {
        if idx >= 0
            && !self
                .heap
                .symbol_ref(sym::TTY_COLOR_BY_INDEX)
                .function
                .is_nil()
        {
            let coldesc = self.funcall(sym::TTY_COLOR_BY_INDEX, &[Value::fixnum(idx), f])?;
            if !coldesc.is_nil() {
                return self.car(coldesc);
            }
        }
        if idx == FACE_TTY_DEFAULT_FG_COLOR {
            return Ok(self.string("unspecified-fg"));
        }
        if idx == FACE_TTY_DEFAULT_BG_COLOR {
            return Ok(self.string("unspecified-bg"));
        }
        Ok(sym::UNSPECIFIED)
    }

    fn parse_rgb_list(&self, rgb_list: Value, color: &mut EmacsColor) -> bool {
        let mut rgb_list = rgb_list;
        for field in [&mut color.red, &mut color.green, &mut color.blue] {
            if !rgb_list.is_cons() {
                return false;
            }
            match self.xcar(rgb_list).as_fixnum() {
                Some(n) => *field = n as u16,
                None => return false,
            }
            rgb_list = self.xcdr(rgb_list);
        }
        true
    }

    fn tty_lookup_color(
        &mut self,
        f: Value,
        color: Value,
        tty_color: &mut EmacsColor,
    ) -> LispResult<bool> {
        if !color.is_string() || self.heap.symbol_ref(sym::TTY_COLOR_DESC).function.is_nil() {
            return Ok(false);
        }
        let color_desc = self.funcall(sym::TTY_COLOR_DESC, &[color, f])?;
        if color_desc.is_cons() && self.xcdr(color_desc).is_cons() {
            let Some(pixel) = self.xcar(self.xcdr(color_desc)).as_fixnum() else {
                return Ok(false);
            };
            tty_color.pixel = pixel;
            let rgb = self.xcdr(self.xcdr(color_desc));
            Ok(self.parse_rgb_list(rgb, tty_color))
        } else if self
            .find_symbol_value(sym::TTY_DEFINED_COLOR_ALIST)
            .is_nil()
        {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn tty_defined_color(
        &mut self,
        f: Value,
        color_name: &[u8],
    ) -> LispResult<(bool, EmacsColor)> {
        let color_name = crate::fontset::c_bytes(color_name);
        let mut status = true;
        let mut color_def = EmacsColor {
            pixel: FACE_TTY_DEFAULT_COLOR,
            ..EmacsColor::default()
        };
        if !color_name.is_empty() {
            let lcolor = self.make_string(color_name);
            self.stack.push(lcolor);
            let r = self.tty_lookup_color(f, lcolor, &mut color_def);
            self.stack.pop();
            status = r?;
            if color_def.pixel == FACE_TTY_DEFAULT_COLOR {
                if color_name == b"unspecified-fg" {
                    color_def.pixel = FACE_TTY_DEFAULT_FG_COLOR;
                } else if color_name == b"unspecified-bg" {
                    color_def.pixel = FACE_TTY_DEFAULT_BG_COLOR;
                }
            }
        }
        if color_def.pixel != FACE_TTY_DEFAULT_COLOR {
            status = true;
        }
        Ok((status, color_def))
    }

    pub fn defined_color(&mut self, f: Value, color_name: &[u8]) -> LispResult<(bool, EmacsColor)> {
        match self.frame(f).output_method {
            OutputMethod::Initial => self.tty_defined_color(f, color_name),
        }
    }

    fn face_color_gray_p(&mut self, f: Value, color_name: &[u8]) -> LispResult<bool> {
        let (ok, color) = self.defined_color(f, color_name)?;
        if !ok {
            return Ok(false);
        }
        let (r, g, b) = (
            i32::from(color.red),
            i32::from(color.green),
            i32::from(color.blue),
        );
        Ok((r < 5000 && g < 5000 && b < 5000)
            || ((r - g).abs() < r.max(g) / 20
                && (g - b).abs() < g.max(b) / 20
                && (b - r).abs() < b.max(r) / 20))
    }

    fn face_color_supported_p(
        &mut self,
        f: Value,
        color_name: &[u8],
        _background_p: bool,
    ) -> LispResult<bool> {
        if self.frame(f).window_p() {
            unreachable!("window-system color support needs xw-display-color-p");
        }
        Ok(self.tty_defined_color(f, color_name)?.0)
    }

    pub fn update_face_from_frame_parameter(
        &mut self,
        f: Value,
        param: Value,
        new_value: Value,
    ) -> LispResult<()> {
        let table = self.frame(f).face_hash_table;
        if self.hash_table(table).count == 0 {
            return Ok(());
        }
        let value = if self.is_string(new_value) {
            new_value
        } else {
            sym::UNSPECIFIED
        };
        let (face, index) = if param == sym::FOREGROUND_COLOR {
            (sym::DEFAULT, LFACE_FOREGROUND_INDEX)
        } else if param == sym::BACKGROUND_COLOR {
            self.funcall(sym::FRAME_SET_BACKGROUND_MODE, &[f])?;
            (sym::DEFAULT, LFACE_BACKGROUND_INDEX)
        } else if param == sym::BORDER_COLOR {
            (sym::BORDER, LFACE_BACKGROUND_INDEX)
        } else if param == sym::CURSOR_COLOR {
            (sym::CURSOR, LFACE_BACKGROUND_INDEX)
        } else if param == sym::MOUSE_COLOR {
            (sym::MOUSE, LFACE_BACKGROUND_INDEX)
        } else {
            return Ok(());
        };
        let lface = self.lface_from_face_name(Some(f), face, true)?;
        self.lface_aset(lface, index, value);
        if self.get(face, sym::FACE_NO_INHERIT).is_nil() {
            self.frame_mut(f).flags.face_change = true;
            self.fset_redisplay(f);
        }
        Ok(())
    }

    fn set_font_frame_param(&mut self, frame: Value, lface: Value) {
        if self.frame(frame).window_p() && !unspecifiedp(self.lface_aref(lface, LFACE_FONT_INDEX)) {
            unreachable!("window-system frames need font_load_for_lface");
        }
    }

    fn make_face_cache(&self) -> crate::xfaces::FaceCache {
        FaceCache {
            buckets: vec![Vec::new(); FACE_CACHE_BUCKETS_SIZE],
            faces_by_id: Vec::new(),
            menu_face_changed_p: self.faces.menu_face_changed_default,
        }
    }

    pub fn face_from_id(&self, f: Value, id: usize) -> Option<&Face> {
        self.frame(f).face_cache.as_ref().and_then(|c| c.face(id))
    }

    fn face_cache_mut(&mut self, f: Value) -> &mut FaceCache {
        self.frame_mut(f)
            .face_cache
            .as_deref_mut()
            .expect("frame has no face cache")
    }

    pub(crate) fn set_frame_garbaged(&mut self, f: Value) {
        self.frames.frame_garbaged = true;
        self.fset_redisplay(f);
        self.frame_mut(f).flags.garbaged = true;
    }

    pub fn init_frame_faces(&mut self, f: Value) -> LispResult<()> {
        if self.frame(f).face_cache.is_none() {
            let cache = self.make_face_cache();
            self.frame_mut(f).face_cache = Some(Box::new(cache));
        }
        if !self.realize_basic_faces(f)? {
            panic!("cannot realize the basic faces of a new frame");
        }
        Ok(())
    }

    pub fn recompute_basic_faces(&mut self, f: Value) -> LispResult<()> {
        if let Some(used) = self.frame(f).face_cache.as_ref().map(|c| c.used()) {
            let non_basic_faces_cached = used > BASIC_FACE_ID_SENTINEL;
            self.clear_face_cache(false);
            if !self.realize_basic_faces(f)? {
                panic!("cannot realize the basic faces of a frame");
            }
            if non_basic_faces_cached {
                self.frame_mut(f).flags.face_change = true;
            }
        }
        Ok(())
    }

    fn free_realized_faces(&mut self, f: Value) {
        let used = self.frame(f).face_cache.as_ref().map_or(0, |c| c.used());
        if used == 0 {
            return;
        }
        let cache = self.face_cache_mut(f);
        cache.faces_by_id.clear();
        for bucket in &mut cache.buckets {
            bucket.clear();
        }
        if self.is_window(self.frame(f).root_window) {
            self.fset_redisplay(f);
        }
    }

    fn realize_face(
        &mut self,
        f: Value,
        attrs: [Value; N],
        former_face_id: Option<usize>,
    ) -> usize {
        if let Some(former) = former_face_id
            && self.face_cache_mut(f).used() > former
        {
            self.face_cache_mut(f).uncache_face(former);
            self.set_frame_garbaged(f);
        }
        let face = match self.frame(f).output_method {
            crate::terminal::OutputMethod::Initial => Face::make_realized(attrs),
        };
        let hash = lface_hash(self, &attrs);
        self.face_cache_mut(f).cache_face(face, hash)
    }

    fn lookup_face(&mut self, f: Value, attrs: &[Value; N]) -> LispResult<usize> {
        let hash = lface_hash(self, attrs);
        let i = (hash % FACE_CACHE_BUCKETS_SIZE as u64) as usize;
        let ids = self.face_cache_mut(f).buckets[i].clone();
        for id in ids {
            let Some(face) = self.face_from_id(f, id) else {
                continue;
            };
            if !face.is_ascii() {
                break;
            }
            let (face_hash, lface) = (face.hash, face.lface);
            if face_hash == hash && lface_equal_p(self, &lface, attrs)? {
                return Ok(id);
            }
        }
        Ok(self.realize_face(f, *attrs, None))
    }

    pub fn lookup_named_face(
        &mut self,
        w: Option<Value>,
        f: Value,
        symbol: Value,
        signal_p: bool,
    ) -> LispResult<i64> {
        if self.face_from_id(f, DEFAULT_FACE_ID).is_none() && !self.realize_basic_faces(f)? {
            return Ok(-1);
        }
        let default_lface = self
            .face_from_id(f, DEFAULT_FACE_ID)
            .expect("default face is not realized")
            .lface;
        let base = self.stack.len();
        let r = (|| -> LispResult<i64> {
            let symbol_attrs = self.attrs_push(sym::UNSPECIFIED);
            let mut points = MergePoints::new();
            if !self.get_lface_attributes(
                w,
                Some(f),
                symbol,
                symbol_attrs,
                signal_p,
                &mut points,
            )? {
                return Ok(-1);
            }
            let attrs = self.attrs_push_from(&default_lface);
            for i in 1..N {
                if resetp(self.at(symbol_attrs, i)) {
                    let v = self.at(attrs, i);
                    self.set_at(symbol_attrs, i, v);
                }
            }
            self.merge_face_vectors(w, Some(f), symbol_attrs, attrs, &mut points)?;
            let merged = self.attrs_snapshot(attrs);
            Ok(self.lookup_face(f, &merged)? as i64)
        })();
        self.stack.truncate(base);
        r
    }

    pub fn face_at_buffer_position(
        &mut self,
        w: Value,
        pos: usize,
        limit: usize,
        mouse: bool,
        base_face_id: Option<usize>,
        attr_filter: usize,
    ) -> LispResult<(usize, usize)> {
        let base = self.stack.len();
        let r = self.face_at_buffer_position_1(w, pos, limit, mouse, base_face_id, attr_filter);
        self.stack.truncate(base);
        r
    }

    fn face_at_buffer_position_1(
        &mut self,
        w: Value,
        pos: usize,
        limit: usize,
        mouse: bool,
        base_face_id: Option<usize>,
        attr_filter: usize,
    ) -> LispResult<(usize, usize)> {
        let f = self.window(w).frame;
        let contents = self.window(w).contents;
        let propname = if mouse { sym::MOUSE_FACE } else { sym::FACE };
        let position = Value::fixnum(pos as i64);
        let mut endpos = self.cur().zv;
        let prop = crate::textprop::f_get_text_property(self, &[position, propname, contents])?;
        self.stack.push(prop);
        let limit1 = Value::fixnum(limit.min(endpos) as i64);
        let end = crate::textprop::f_next_single_property_change(
            self,
            &[position, propname, contents, limit1],
        )?;
        if let Some(e) = end.as_fixnum() {
            endpos = e as usize;
        }
        let (overlay_vec, next_overlay) = self.overlays_at(pos);
        self.stack.extend_from_slice(&overlay_vec);
        let next_overlay = next_overlay.min(self.cur().zv);
        if next_overlay < endpos {
            endpos = next_overlay;
        }
        let face_id = match base_face_id {
            Some(id) => id,
            None if self.find_symbol_value(sym::FACE_REMAPPING_ALIST).is_nil() => DEFAULT_FACE_ID,
            None => self.lookup_basic_face(Some(w), f, DEFAULT_FACE_ID)?,
        };
        let default_face_id = if self.face_from_id(f, face_id).is_some() {
            face_id
        } else {
            let used = self
                .frame(f)
                .face_cache
                .as_ref()
                .expect("frame has no face cache")
                .used();
            if used == 0 {
                self.recompute_basic_faces(f)?;
            }
            let id = self.lookup_basic_face(Some(w), f, DEFAULT_FACE_ID)?;
            self.face_from_id(f, id).expect("default face").id
        };
        if overlay_vec.is_empty() && prop.is_nil() {
            return Ok((default_face_id, endpos));
        }
        let lface = self
            .face_from_id(f, default_face_id)
            .expect("default face")
            .lface;
        let attrs = self.attrs_push_from(&lface);
        if !prop.is_nil() {
            self.merge_face_ref(
                Some(w),
                Some(f),
                prop,
                attrs,
                true,
                &mut MergePoints::new(),
                attr_filter,
            )?;
        }
        let overlays = self.sort_overlays(&overlay_vec, Some(w));
        if mouse {
            let mut prop = Value::NIL;
            let mut i = overlays.len();
            while i > 0 && prop.is_nil() {
                i -= 1;
                prop = self.overlay_get(overlays[i], propname);
                if !prop.is_nil() {
                    self.stack[attrs..attrs + N].copy_from_slice(&lface);
                    self.merge_face_ref(
                        Some(w),
                        Some(f),
                        prop,
                        attrs,
                        true,
                        &mut MergePoints::new(),
                        attr_filter,
                    )?;
                }
                let oendpos = self.overlay(overlays[i]).end;
                if oendpos < endpos {
                    endpos = oendpos;
                }
            }
        } else {
            for &ov in &overlays {
                let prop = self.overlay_get(ov, propname);
                if !prop.is_nil() {
                    self.merge_face_ref(
                        Some(w),
                        Some(f),
                        prop,
                        attrs,
                        true,
                        &mut MergePoints::new(),
                        attr_filter,
                    )?;
                }
                let oendpos = self.overlay(ov).end;
                if oendpos < endpos {
                    endpos = oendpos;
                }
            }
        }
        let merged = self.attrs_snapshot(attrs);
        Ok((self.lookup_face(f, &merged)?, endpos))
    }

    pub fn lookup_basic_face(
        &mut self,
        w: Option<Value>,
        f: Value,
        face_id: usize,
    ) -> LispResult<usize> {
        let alist = self.find_symbol_value(sym::FACE_REMAPPING_ALIST);
        if alist.is_nil() {
            return Ok(face_id);
        }
        let name = if face_id == DEFAULT_FACE_ID {
            sym::DEFAULT
        } else {
            BASIC_FACES
                .iter()
                .find(|&&(_, id)| id == face_id)
                .map(|&(name, _)| name)
                .expect("not a basic face id")
        };
        let mapping = self.assq(name, alist);
        if mapping.is_nil() {
            let base = self.stack.len();
            let face_attrs = self.attrs_push(sym::UNSPECIFIED);
            let mut points = MergePoints::new();
            let ok = self.get_lface_attributes(w, Some(f), name, face_attrs, false, &mut points);
            let inherit = self.at(face_attrs, LFACE_INHERIT_INDEX);
            self.stack.truncate(base);
            if !ok? || inherit.is_nil() || unspecifiedp(inherit) {
                return Ok(face_id);
            }
        }
        let remapped_face_id = self.lookup_named_face(w, f, name, false)?;
        if remapped_face_id < 0 {
            return Ok(face_id);
        }
        Ok(remapped_face_id as usize)
    }

    pub fn realize_basic_faces(&mut self, f: Value) -> LispResult<bool> {
        if !self.realize_default_face(f)? {
            return Ok(false);
        }
        let count = self.specpdl.len();
        let r = (|| -> LispResult<()> {
            self.specbind(sym::FACE_REMAPPING_ALIST, Value::NIL)?;
            for (symbol, id) in BASIC_FACES {
                self.realize_named_face(f, symbol, id)?;
            }
            Ok(())
        })();
        let u = self.unbind_to(count);
        r?;
        u?;
        let cache = self.face_cache_mut(f);
        if cache.menu_face_changed_p {
            cache.menu_face_changed_p = false;
        }
        Ok(true)
    }

    fn realize_default_face(&mut self, f: Value) -> LispResult<bool> {
        let mut lface = self.lface_from_face_name(Some(f), sym::DEFAULT, false)?;
        if lface.is_nil() {
            lface = f_internal_make_lisp_face(self, &[sym::DEFAULT, f])?;
        }
        if self.frame(f).window_p() {
            unreachable!("window-system frames need set_lface_from_font");
        }
        let family = self.string("default");
        self.lface_aset(lface, LFACE_FAMILY_INDEX, family);
        self.lface_aset(lface, LFACE_FOUNDRY_INDEX, family);
        self.lface_aset(lface, LFACE_SWIDTH_INDEX, sym::NORMAL);
        self.lface_aset(lface, LFACE_HEIGHT_INDEX, Value::fixnum(1));
        for (i, value) in [
            (LFACE_WEIGHT_INDEX, sym::NORMAL),
            (LFACE_SLANT_INDEX, sym::NORMAL),
            (LFACE_FONTSET_INDEX, Value::NIL),
            (LFACE_EXTEND_INDEX, Value::NIL),
            (LFACE_UNDERLINE_INDEX, Value::NIL),
            (LFACE_OVERLINE_INDEX, Value::NIL),
            (LFACE_STRIKE_THROUGH_INDEX, Value::NIL),
            (LFACE_BOX_INDEX, Value::NIL),
            (LFACE_INVERSE_INDEX, Value::NIL),
        ] {
            if unspecifiedp(self.lface_aref(lface, i)) {
                self.lface_aset(lface, i, value);
            }
        }
        for (i, param, unspecified) in [
            (
                LFACE_FOREGROUND_INDEX,
                sym::FOREGROUND_COLOR,
                "unspecified-fg",
            ),
            (
                LFACE_BACKGROUND_INDEX,
                sym::BACKGROUND_COLOR,
                "unspecified-bg",
            ),
        ] {
            if unspecifiedp(self.lface_aref(lface, i)) {
                let params = self.frame(f).param_alist;
                let color = self.assq(param, params);
                if color.is_cons() && self.xcdr(color).is_string() {
                    let value = self.xcdr(color);
                    self.lface_aset(lface, i, value);
                } else if self.frame(f).window_p() {
                    return Ok(false);
                } else {
                    let value = self.string(unspecified);
                    self.lface_aset(lface, i, value);
                }
            }
        }
        if unspecifiedp(self.lface_aref(lface, LFACE_STIPPLE_INDEX)) {
            self.lface_aset(lface, LFACE_STIPPLE_INDEX, Value::NIL);
        }
        let attrs = self.lface_contents(lface);
        let count = self.specpdl.len();
        self.specbind(sym::INHIBIT_REDISPLAY, sym::T)?;
        self.realize_face(f, attrs, Some(DEFAULT_FACE_ID));
        self.unbind_to(count)?;
        Ok(true)
    }

    fn realize_named_face(&mut self, f: Value, symbol: Value, id: usize) -> LispResult<()> {
        let base = self.stack.len();
        let r = self.realize_named_face_1(f, symbol, id);
        self.stack.truncate(base);
        r
    }

    fn realize_named_face_1(&mut self, f: Value, symbol: Value, id: usize) -> LispResult<()> {
        let lface = self.lface_from_face_name(Some(f), symbol, false)?;
        let attrs = self.attrs_push(sym::UNSPECIFIED);
        self.get_lface_attributes_no_remap(Some(f), sym::DEFAULT, attrs, true)?;
        if lface.is_nil() {
            f_internal_make_lisp_face(self, &[symbol, f])?;
        }
        let symbol_attrs = self.attrs_push(sym::UNSPECIFIED);
        self.get_lface_attributes_no_remap(Some(f), symbol, symbol_attrs, true)?;
        for i in 1..N {
            if resetp(self.at(symbol_attrs, i)) {
                let v = self.at(attrs, i);
                self.set_at(symbol_attrs, i, v);
            }
        }
        let mut points = MergePoints::new();
        self.merge_face_vectors(None, Some(f), symbol_attrs, attrs, &mut points)?;
        let merged = self.attrs_snapshot(attrs);
        self.realize_face(f, merged, Some(id));
        Ok(())
    }

    fn gui_supports_face_attributes_p(
        &mut self,
        f: Value,
        attrs: &[Value; N],
        def_attrs: &[Value; N],
        def_font: Value,
    ) -> LispResult<bool> {
        let mut lattrs = *attrs;
        for i in 1..N {
            if resetp(attrs[i]) {
                lattrs[i] = def_attrs[i];
            }
        }
        for i in [
            LFACE_UNDERLINE_INDEX,
            LFACE_INVERSE_INDEX,
            LFACE_EXTEND_INDEX,
            LFACE_FOREGROUND_INDEX,
            LFACE_DISTANT_FOREGROUND_INDEX,
            LFACE_BACKGROUND_INDEX,
            LFACE_STIPPLE_INDEX,
            LFACE_OVERLINE_INDEX,
            LFACE_STRIKE_THROUGH_INDEX,
            LFACE_BOX_INDEX,
        ] {
            if !unspecifiedp(lattrs[i]) && face_attr_equal_p(self, lattrs[i], def_attrs[i])? {
                return Ok(false);
            }
        }
        if [
            LFACE_FAMILY_INDEX,
            LFACE_FOUNDRY_INDEX,
            LFACE_HEIGHT_INDEX,
            LFACE_WEIGHT_INDEX,
            LFACE_SLANT_INDEX,
            LFACE_SWIDTH_INDEX,
        ]
        .iter()
        .any(|&i| !unspecifiedp(lattrs[i]))
        {
            let base = self.stack.len();
            let r = (|| -> LispResult<usize> {
                let merged = self.attrs_push_from(def_attrs);
                let from = self.attrs_push_from(attrs);
                let mut points = MergePoints::new();
                self.merge_face_vectors(None, Some(f), from, merged, &mut points)?;
                let merged = self.attrs_snapshot(merged);
                self.lookup_face(f, &merged)
            })();
            self.stack.truncate(base);
            let face_id = r?;
            let Some(face) = self.face_from_id(f, face_id) else {
                return Err(self.error("Cannot make face"));
            };
            let font = face.font;
            if font == def_font || font.is_nil() {
                return Ok(false);
            }
            unreachable!("faces with fonts exist only on window-system frames");
        }
        let val = attrs[LFACE_UNDERLINE_INDEX];
        if !unspecifiedp(val) && self.car_safe(val) == sym::K_STYLE {
            let style = self.car_safe(self.cdr_safe(val));
            if !(style == sym::LINE || style == sym::WAVE) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn clear_face_cache(&mut self, clear_fonts_p: bool) {
        let thorough = clear_fonts_p || {
            self.faces.clear_font_table_count += 1;
            self.faces.clear_font_table_count == CLEAR_FONT_TABLE_COUNT
        };
        if thorough {
            self.faces.clear_font_table_count = 0;
        }
        for frame in self.frame_list_vec() {
            if self.frame(frame).window_p() {
                unreachable!("window-system frames need clear_font_cache and clear_face_gcs");
            }
        }
    }

    fn free_all_realized_faces(&mut self, frame: Value) {
        if frame.is_nil() {
            for f in self.frame_list_vec() {
                self.free_realized_faces(f);
            }
            self.windows.windows_or_buffers_changed = 58;
        } else {
            self.free_realized_faces(frame);
        }
    }

    fn face_change_for(&mut self, face: Value, f: Option<Value>, code: i64) {
        if self.get(face, sym::FACE_NO_INHERIT).is_nil() {
            match f {
                Some(f) => {
                    self.frame_mut(f).flags.face_change = true;
                    self.fset_redisplay(f);
                }
                None => {
                    self.faces.face_change = true;
                    self.windows.windows_or_buffers_changed = code;
                }
            }
        }
    }
}

pub(crate) fn f_internal_make_lisp_face(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (face, frame) = (args[0], arg(args, 1));
    lisp.check_symbol(face)?;
    let mut global_lface = lisp.lface_from_face_name(None, face, false)?;
    let (f, mut lface) = if !frame.is_nil() {
        lisp.check_live_frame(frame)?;
        (
            Some(frame),
            lisp.lface_from_face_name(Some(frame), face, false)?,
        )
    } else {
        (None, Value::NIL)
    };
    if global_lface.is_nil() {
        let face_id = Value::fixnum(lisp.faces.lface_id_to_name.len() as i64);
        lisp.faces.lface_id_to_name.push(face);
        lisp.put(face, sym::FACE, face_id);
        global_lface = lisp.make_lface_vector();
        let entry = lisp.cons(face_id, global_lface);
        let table = lisp.face_new_frame_defaults();
        lisp.puthash(face, entry, table)?;
    } else if f.is_none() {
        for i in 1..N {
            lisp.lface_aset(global_lface, i, sym::UNSPECIFIED);
        }
    }
    if let Some(fr) = f {
        if lface.is_nil() {
            lface = lisp.make_lface_vector();
            let table = lisp.frame(fr).face_hash_table;
            lisp.puthash(face, lface, table)?;
        } else {
            for i in 1..N {
                lisp.lface_aset(lface, i, sym::UNSPECIFIED);
            }
        }
    } else {
        lface = global_lface;
    }
    lisp.face_change_for(face, f, 54);
    Ok(lface)
}

fn f_internal_lisp_face_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (face, frame) = (args[0], arg(args, 1));
    let face = lisp.resolve_face_name(face, true)?;
    if !frame.is_nil() {
        lisp.check_live_frame(frame)?;
        lisp.lface_from_face_name(Some(frame), face, false)
    } else {
        lisp.lface_from_face_name(None, face, false)
    }
}

fn f_internal_copy_lisp_face(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (from, to, frame, mut new_frame) = (args[0], args[1], args[2], args[3]);
    lisp.check_symbol(from)?;
    lisp.check_symbol(to)?;
    let (lface, copy, f) = if frame == sym::T {
        let lface = lisp.lface_from_face_name(None, from, true)?;
        let copy = f_internal_make_lisp_face(lisp, &[to, Value::NIL])?;
        (lface, copy, None)
    } else {
        if new_frame.is_nil() {
            new_frame = frame;
        }
        lisp.check_live_frame(frame)?;
        lisp.check_live_frame(new_frame)?;
        let lface = lisp.lface_from_face_name(Some(frame), from, true)?;
        let copy = f_internal_make_lisp_face(lisp, &[to, new_frame])?;
        (lface, copy, Some(new_frame))
    };
    let contents = lisp.lface_contents(lface);
    for (i, v) in contents.into_iter().enumerate() {
        lisp.lface_aset(copy, i, v);
    }
    lisp.face_change_for(to, f, 55);
    Ok(to)
}

fn handle_invalid_nil_value(
    lisp: &mut Lisp,
    value: Value,
    attr: Value,
    face: Value,
) -> LispResult<Value> {
    if value.is_nil() {
        lisp.add_to_log(
            "Warning: setting attribute `%s' of face `%s': nil value is invalid, use `unspecified' instead.",
            &[attr, face],
        )?;
        return Ok(sym::UNSPECIFIED);
    }
    Ok(value)
}

fn valid_underline_p(lisp: &Lisp, value: Value) -> bool {
    if unspecifiedp(value) || ignore_deffacep(value) || resetp(value) {
        return true;
    }
    if value.is_nil() || value == sym::T {
        return true;
    }
    if value.is_string() {
        return lisp.string_chars(value) > 0;
    }
    if !value.is_cons() {
        return false;
    }
    let mut list = value;
    while !lisp.car_safe(list).is_nil() {
        let key = lisp.car_safe(list);
        list = lisp.cdr_safe(list);
        let val = lisp.car_safe(list);
        list = lisp.cdr_safe(list);
        let invalid_color = key == sym::K_COLOR
            && !(val == sym::FOREGROUND_COLOR || (val.is_string() && lisp.string_chars(val) > 0));
        let invalid_style = key == sym::K_STYLE
            && !(val == sym::LINE
                || val == sym::DOUBLE_LINE
                || val == sym::WAVE
                || val == sym::DOTS
                || val == sym::DASHES);
        if key.is_nil()
            || (val.is_nil() && key != sym::K_POSITION)
            || invalid_color
            || invalid_style
        {
            return false;
        }
    }
    true
}

fn valid_box_p(lisp: &Lisp, value: Value) -> bool {
    if unspecifiedp(value) || ignore_deffacep(value) || resetp(value) || value.is_nil() {
        return true;
    }
    if let Some(n) = value.as_fixnum() {
        return n != 0;
    }
    if value.is_string() {
        return lisp.string_chars(value) > 0;
    }
    if !value.is_cons() {
        return false;
    }
    if lisp.xcar(value).is_fixnum() && lisp.xcdr(value).is_fixnum() {
        return true;
    }
    let mut tem = value;
    while tem.is_cons() {
        let k = lisp.xcar(tem);
        tem = lisp.xcdr(tem);
        if !tem.is_cons() {
            break;
        }
        let v = lisp.xcar(tem);
        if k == sym::K_LINE_WIDTH {
            let pair_ok = v.is_cons()
                && lisp.xcar(v).as_fixnum().is_some_and(|n| n != 0)
                && lisp.xcdr(v).as_fixnum().is_some_and(|n| n != 0);
            let int_ok = v.as_fixnum().is_some_and(|n| n != 0);
            if !pair_ok && !int_ok {
                break;
            }
        } else if k == sym::K_COLOR {
            if !v.is_nil() && (!v.is_string() || lisp.string_chars(v) == 0) {
                break;
            }
        } else if k == sym::K_STYLE {
            if !v.is_nil()
                && v != sym::PRESSED_BUTTON
                && v != sym::RELEASED_BUTTON
                && v != sym::FLAT_BUTTON
            {
                break;
            }
        } else {
            break;
        }
        tem = lisp.xcdr(tem);
    }
    tem.is_nil()
}

pub(crate) fn f_internal_set_lisp_face_attribute(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let base = lisp.stack.len();
    lisp.stack.extend_from_slice(&args[..3]);
    lisp.stack.push(arg(args, 3));
    let r = internal_set_lisp_face_attribute(lisp, args[0], args[1], args[2], arg(args, 3));
    lisp.stack.truncate(base);
    r
}

fn internal_set_lisp_face_attribute(
    lisp: &mut Lisp,
    face: Value,
    attr: Value,
    value: Value,
    frame: Value,
) -> LispResult {
    let mut attr = attr;
    let mut value = value;
    let mut frame = frame;
    let mut old_value = Value::NIL;
    let mut prop_index = 0;
    lisp.check_symbol(face)?;
    lisp.check_symbol(attr)?;
    let face = lisp.resolve_face_name(face, true)?;
    if frame == Value::fixnum(0) {
        f_internal_set_lisp_face_attribute(lisp, &[face, attr, value, sym::T])?;
        for fr in lisp.frame_list_vec() {
            f_internal_set_lisp_face_attribute(lisp, &[face, attr, value, fr])?;
        }
        return Ok(face);
    }
    let (f, lface) = if frame == sym::T {
        let lface = lisp.lface_from_face_name(None, face, true)?;
        if unspecifiedp(value) {
            value = sym::K_IGNORE_DEFFACE;
        }
        (None, lface)
    } else {
        if frame.is_nil() {
            frame = lisp.frames.selected_frame;
        }
        lisp.check_live_frame(frame)?;
        let mut lface = lisp.lface_from_face_name(Some(frame), face, false)?;
        if lface.is_nil() {
            lface = f_internal_make_lisp_face(lisp, &[face, frame])?;
        }
        (Some(frame), lface)
    };
    lisp.stack.push(lface);
    let special = |v: Value| unspecifiedp(v) || ignore_deffacep(v) || resetp(v);
    if attr == sym::K_FAMILY || attr == sym::K_FOUNDRY {
        if !special(value) {
            lisp.check_string(value)?;
            if lisp.string_chars(value) == 0 {
                let msg = if attr == sym::K_FAMILY {
                    "Invalid face family"
                } else {
                    "Invalid face foundry"
                };
                return Err(lisp.signal_error(msg, value));
            }
        }
        let (li, fi) = if attr == sym::K_FAMILY {
            (LFACE_FAMILY_INDEX, FONT_FAMILY_INDEX)
        } else {
            (LFACE_FOUNDRY_INDEX, FONT_FOUNDRY_INDEX)
        };
        old_value = lisp.lface_aref(lface, li);
        lisp.lface_aset(lface, li, value);
        prop_index = fi;
    } else if attr == sym::K_HEIGHT {
        if !special(value) {
            if face == sym::DEFAULT {
                if !value.as_fixnum().is_some_and(|n| n > 0) {
                    return Err(
                        lisp.signal_error("Default face height not absolute and positive", value)
                    );
                }
            } else {
                let test = lisp.merge_face_heights(value, Value::fixnum(10), Value::NIL)?;
                if !test.as_fixnum().is_some_and(|n| n > 0) {
                    return Err(
                        lisp.signal_error("Face height does not produce a positive integer", value)
                    );
                }
            }
        }
        old_value = lisp.lface_aref(lface, LFACE_HEIGHT_INDEX);
        lisp.lface_aset(lface, LFACE_HEIGHT_INDEX, value);
        prop_index = FONT_SIZE_INDEX;
    } else if attr == sym::K_WEIGHT || attr == sym::K_SLANT || attr == sym::K_WIDTH {
        let (li, fi, msg) = if attr == sym::K_WEIGHT {
            (LFACE_WEIGHT_INDEX, FONT_WEIGHT_INDEX, "Invalid face weight")
        } else if attr == sym::K_SLANT {
            (LFACE_SLANT_INDEX, FONT_SLANT_INDEX, "Invalid face slant")
        } else {
            (LFACE_SWIDTH_INDEX, FONT_WIDTH_INDEX, "Invalid face width")
        };
        if !special(value) {
            lisp.check_symbol(value)?;
            if lisp.font_style_to_value(fi, value, false)? >> 8 < 0 {
                return Err(lisp.signal_error(msg, value));
            }
        }
        old_value = lisp.lface_aref(lface, li);
        lisp.lface_aset(lface, li, value);
        prop_index = fi;
    } else if attr == sym::K_UNDERLINE {
        if !valid_underline_p(lisp, value) {
            return Err(lisp.signal_error("Invalid face underline", value));
        }
        old_value = lisp.lface_aref(lface, LFACE_UNDERLINE_INDEX);
        lisp.lface_aset(lface, LFACE_UNDERLINE_INDEX, value);
    } else if attr == sym::K_OVERLINE || attr == sym::K_STRIKE_THROUGH {
        let (li, msg) = if attr == sym::K_OVERLINE {
            (LFACE_OVERLINE_INDEX, "Invalid face overline")
        } else {
            (LFACE_STRIKE_THROUGH_INDEX, "Invalid face strike-through")
        };
        if !special(value)
            && ((value.is_symbol() && value != sym::T && !value.is_nil())
                || (value.is_string() && lisp.string_chars(value) == 0))
        {
            return Err(lisp.signal_error(msg, value));
        }
        old_value = lisp.lface_aref(lface, li);
        lisp.lface_aset(lface, li, value);
    } else if attr == sym::K_BOX {
        if value == sym::T {
            value = Value::fixnum(1);
        }
        if !valid_box_p(lisp, value) {
            return Err(lisp.signal_error("Invalid face box", value));
        }
        old_value = lisp.lface_aref(lface, LFACE_BOX_INDEX);
        lisp.lface_aset(lface, LFACE_BOX_INDEX, value);
    } else if attr == sym::K_INVERSE_VIDEO || attr == sym::K_EXTEND {
        let (li, msg) = if attr == sym::K_INVERSE_VIDEO {
            (
                LFACE_INVERSE_INDEX,
                "Invalid inverse-video face attribute value",
            )
        } else {
            (LFACE_EXTEND_INDEX, "Invalid extend face attribute value")
        };
        if !special(value) {
            lisp.check_symbol(value)?;
            if value != sym::T && !value.is_nil() {
                return Err(lisp.signal_error(msg, value));
            }
        }
        old_value = lisp.lface_aref(lface, li);
        lisp.lface_aset(lface, li, value);
    } else if attr == sym::K_FOREGROUND
        || attr == sym::K_DISTANT_FOREGROUND
        || attr == sym::K_BACKGROUND
    {
        let (li, msg) = if attr == sym::K_FOREGROUND {
            (LFACE_FOREGROUND_INDEX, "Empty foreground color value")
        } else if attr == sym::K_DISTANT_FOREGROUND {
            (
                LFACE_DISTANT_FOREGROUND_INDEX,
                "Empty distant-foreground color value",
            )
        } else {
            (LFACE_BACKGROUND_INDEX, "Empty background color value")
        };
        value = handle_invalid_nil_value(lisp, value, attr, face)?;
        if !special(value) {
            lisp.check_string(value)?;
            if lisp.string_chars(value) == 0 {
                return Err(lisp.signal_error(msg, value));
            }
        }
        old_value = lisp.lface_aref(lface, li);
        lisp.lface_aset(lface, li, value);
    } else if attr == sym::K_STIPPLE {
        if !special(value) && !value.is_nil() && f_bitmap_spec_p(lisp, &[value])?.is_nil() {
            return Err(lisp.signal_error("Invalid stipple attribute", value));
        }
        old_value = lisp.lface_aref(lface, LFACE_STIPPLE_INDEX);
        lisp.lface_aset(lface, LFACE_STIPPLE_INDEX, value);
    } else if attr == sym::K_FONT {
        if frame == sym::T || lisp.frame(frame).window_p() {
            if !special(value) {
                old_value = lisp.lface_aref(lface, LFACE_FONT_INDEX);
                if !lisp.is_font(value) {
                    if value.is_string() {
                        let mut name = value;
                        let fontset = lisp.fs_query_fontset(name, 0)?;
                        if fontset >= 0 {
                            name = lisp.fontset_ascii(fontset as usize);
                        }
                        lisp.stack.push(name);
                        value = lisp.font_spec_from_name(name)?;
                        lisp.stack.push(value);
                        if !lisp.is_font(value) {
                            return Err(lisp.signal_error("Invalid font name", name));
                        }
                    } else {
                        return Err(lisp.signal_error("Invalid font or font-spec", value));
                    }
                }
                let f1 = if frame == sym::T {
                    lisp.frames.selected_frame
                } else {
                    frame
                };
                if lisp.frame(f1).window_p() {
                    unreachable!("window-system frames need font_load_for_lface");
                }
            } else {
                lisp.lface_aset(lface, LFACE_FONT_INDEX, value);
            }
        }
    } else if attr == sym::K_FONTSET {
        if frame == sym::T || lisp.frame(frame).window_p() {
            let mut tmp = value;
            old_value = lisp.lface_aref(lface, LFACE_FONTSET_INDEX);
            if !resetp(value) {
                tmp = crate::fontset::f_query_fontset(lisp, &[value, Value::NIL])?;
                if tmp.is_nil() {
                    return Err(lisp.signal_error("Invalid fontset name", value));
                }
            }
            value = tmp;
            lisp.stack.push(value);
            lisp.lface_aset(lface, LFACE_FONTSET_INDEX, value);
        }
    } else if attr == sym::K_INHERIT {
        let mut tail = Value::NIL;
        if !value.is_symbol() {
            tail = value;
            while tail.is_cons() {
                if !lisp.xcar(tail).is_symbol() {
                    break;
                }
                tail = lisp.xcdr(tail);
            }
        }
        if value == face || lisp.face_inheritance_cycle(f, value, face)? {
            return Err(lisp.signal_error("Face inheritance results in inheritance cycle", value));
        } else if tail.is_nil() {
            lisp.lface_aset(lface, LFACE_INHERIT_INDEX, value);
        } else {
            return Err(lisp.signal_error("Invalid face inheritance", value));
        }
    } else if attr == sym::K_BOLD || attr == sym::K_ITALIC {
        let (li, fi, on) = if attr == sym::K_BOLD {
            (LFACE_WEIGHT_INDEX, FONT_WEIGHT_INDEX, sym::BOLD)
        } else {
            attr = sym::K_SLANT;
            (LFACE_SLANT_INDEX, FONT_SLANT_INDEX, sym::ITALIC)
        };
        old_value = lisp.lface_aref(lface, li);
        if resetp(value) {
            lisp.lface_aset(lface, li, value);
        } else {
            lisp.lface_aset(lface, li, if value.is_nil() { sym::NORMAL } else { on });
        }
        prop_index = fi;
    } else {
        return Err(lisp.signal_error("Invalid face attribute name", attr));
    }
    if prop_index != 0 {
        lisp.lface_font_clear_prop(lface, prop_index)?;
    }
    if let Some(fr) = f
        && lisp.get(face, sym::FACE_NO_INHERIT).is_nil()
        && !lisp.equal(old_value, value)?
    {
        lisp.frame_mut(fr).flags.face_change = true;
        lisp.fset_redisplay(fr);
    }
    if !unspecifiedp(value) && !ignore_deffacep(value) && !lisp.equal(old_value, value)? {
        let mut param = Value::NIL;
        if face == sym::DEFAULT {
            let contents = lisp.lface_contents(lface);
            if lisp.is_frame(frame)
                && (prop_index != 0 || attr == sym::K_FONT)
                && lface_fully_specified_p(&contents)
            {
                lisp.set_font_frame_param(frame, lface);
            } else if attr == sym::K_FOREGROUND {
                param = sym::FOREGROUND_COLOR;
            } else if attr == sym::K_BACKGROUND {
                param = sym::BACKGROUND_COLOR;
            }
        } else if face == sym::SCROLL_BAR {
            if attr == sym::K_FOREGROUND {
                param = sym::SCROLL_BAR_FOREGROUND;
            } else if attr == sym::K_BACKGROUND {
                param = sym::SCROLL_BAR_BACKGROUND;
            }
        } else if face == sym::BORDER {
            if attr == sym::K_BACKGROUND {
                param = sym::BORDER_COLOR;
            }
        } else if face == sym::CURSOR {
            if attr == sym::K_BACKGROUND {
                param = sym::CURSOR_COLOR;
            }
        } else if face == sym::MOUSE {
            if attr == sym::K_BACKGROUND {
                param = sym::MOUSE_COLOR;
            }
        } else if face == sym::MENU {
            if lisp.is_frame(frame) {
                if lisp.frame(frame).face_cache.is_none() {
                    let cache = lisp.make_face_cache();
                    lisp.frame_mut(frame).face_cache = Some(Box::new(cache));
                }
                if let Some(c) = lisp.frame_mut(frame).face_cache.as_mut() {
                    c.menu_face_changed_p = true;
                }
            } else {
                lisp.faces.menu_face_changed_default = true;
            }
        }
        if !param.is_nil() {
            if frame == sym::T {
                let alist = lisp.find_symbol_value(sym::DEFAULT_FRAME_ALIST);
                let alist = lisp.store_in_alist(alist, param, value);
                lisp.set_plain_value(sym::DEFAULT_FRAME_ALIST, alist);
            } else {
                let cell = lisp.cons(param, value);
                let arg = lisp.list(&[cell]);
                crate::frame::f_modify_frame_parameters(lisp, &[frame, arg])?;
            }
        }
    }
    Ok(face)
}

fn face_boolean_x_resource_value(
    lisp: &mut Lisp,
    value: Value,
    signal_p: bool,
) -> LispResult<Value> {
    let s = crate::fontset::c_bytes(lisp.string_bytes(value)).to_vec();
    let is = |word: &[u8]| s.eq_ignore_ascii_case(word);
    if is(b"on") || is(b"true") {
        Ok(sym::T)
    } else if is(b"off") || is(b"false") {
        Ok(Value::NIL)
    } else if is(b"unspecified") {
        Ok(sym::UNSPECIFIED)
    } else if signal_p {
        Err(lisp.signal_error("Invalid face attribute value from X resource", value))
    } else {
        Ok(Value::fixnum(0))
    }
}

fn f_internal_set_lisp_face_attribute_from_resource(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (face, attr, mut value, frame) = (args[0], args[1], args[2], arg(args, 3));
    lisp.check_symbol(face)?;
    lisp.check_symbol(attr)?;
    lisp.check_string(value)?;
    if crate::fontset::c_bytes(lisp.string_bytes(value)).eq_ignore_ascii_case(b"unspecified") {
        value = sym::UNSPECIFIED;
    } else if attr == sym::K_HEIGHT {
        value = crate::data::f_string_to_number(lisp, &[value, Value::NIL])?;
        if !value.as_fixnum().is_some_and(|n| n > 0) {
            return Err(lisp.signal_error("Invalid face height from X resource", value));
        }
    } else if attr == sym::K_BOLD || attr == sym::K_ITALIC {
        value = face_boolean_x_resource_value(lisp, value, true)?;
    } else if attr == sym::K_WEIGHT || attr == sym::K_SLANT || attr == sym::K_WIDTH {
        let name = crate::fontset::c_bytes(lisp.string_bytes(value)).to_vec();
        let name = lisp.make_string(&name);
        value = crate::lread::f_intern(lisp, &[name, Value::NIL])?;
    } else if attr == sym::K_INVERSE_VIDEO || attr == sym::K_EXTEND {
        value = face_boolean_x_resource_value(lisp, value, true)?;
    } else if attr == sym::K_UNDERLINE || attr == sym::K_OVERLINE || attr == sym::K_STRIKE_THROUGH {
        let boolean_value = face_boolean_x_resource_value(lisp, value, false)?;
        if boolean_value.is_symbol() {
            value = boolean_value;
        }
    } else if attr == sym::K_BOX || attr == sym::K_INHERIT {
        let read = crate::lread::f_read_from_string(lisp, &[value, Value::NIL, Value::NIL])?;
        value = lisp.car(read)?;
    }
    f_internal_set_lisp_face_attribute(lisp, &[face, attr, value, frame])
}

fn f_face_attribute_relative_p(_lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (attribute, value) = (args[0], args[1]);
    if unspecifiedp(value) || ignore_deffacep(value) {
        Ok(sym::T)
    } else if attribute == sym::K_HEIGHT {
        Ok(if value.is_fixnum() {
            Value::NIL
        } else {
            sym::T
        })
    } else {
        Ok(Value::NIL)
    }
}

fn f_merge_face_attribute(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (attribute, value1, value2) = (args[0], args[1], args[2]);
    if unspecifiedp(value1) || ignore_deffacep(value1) {
        Ok(value2)
    } else if attribute == sym::K_HEIGHT {
        lisp.merge_face_heights(value1, value2, value1)
    } else {
        Ok(value1)
    }
}

fn f_internal_get_lisp_face_attribute(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (symbol, keyword, frame) = (args[0], args[1], arg(args, 2));
    let f = if frame == sym::T {
        None
    } else {
        Some(lisp.decode_live_frame(frame)?)
    };
    let lface = lisp.lface_from_face_name(f, symbol, true)?;
    lisp.check_symbol(symbol)?;
    lisp.check_symbol(keyword)?;
    let idx = FACE_ATTR_SYM[1..]
        .iter()
        .position(|&k| k == keyword)
        .map(|i| i + 1);
    let Some(idx) = idx else {
        return Err(lisp.signal_error("Invalid face attribute name", keyword));
    };
    let value = lisp.lface_aref(lface, idx);
    if ignore_deffacep(value) {
        return Ok(sym::UNSPECIFIED);
    }
    Ok(value)
}

fn f_internal_lisp_face_attribute_values(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let attr = args[0];
    lisp.check_symbol(attr)?;
    if attr == sym::K_UNDERLINE
        || attr == sym::K_OVERLINE
        || attr == sym::K_STRIKE_THROUGH
        || attr == sym::K_INVERSE_VIDEO
        || attr == sym::K_EXTEND
    {
        return Ok(lisp.list(&[sym::T, Value::NIL]));
    }
    Ok(Value::NIL)
}

fn f_internal_merge_in_global_face(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (face, frame) = (args[0], args[1]);
    lisp.check_live_frame(frame)?;
    let global_lface = lisp.lface_from_face_name(None, face, true)?;
    let mut local_lface = lisp.lface_from_face_name(Some(frame), face, false)?;
    if local_lface.is_nil() {
        local_lface = f_internal_make_lisp_face(lisp, &[face, frame])?;
    }
    let gvec = lisp.lface_contents(global_lface);
    for (i, &g) in gvec.iter().enumerate().skip(1) {
        if ignore_deffacep(g) {
            lisp.lface_aset(local_lface, i, sym::UNSPECIFIED);
        } else if !unspecifiedp(g) {
            lisp.lface_aset(local_lface, i, g);
        }
    }
    Ok(Value::NIL)
}

fn face_attr_equal_p(lisp: &mut Lisp, v1: Value, v2: Value) -> LispResult<bool> {
    if v1.tag() != v2.tag() {
        return Ok(false);
    }
    if v1 == v2 {
        return Ok(true);
    }
    match v1.tag() {
        crate::value::Tag::String => Ok(lisp.string_bytes(v1) == lisp.string_bytes(v2)),
        crate::value::Tag::Fixnum | crate::value::Tag::Symbol => Ok(false),
        _ => lisp.equal(v1, v2),
    }
}

fn lface_equal_p(lisp: &mut Lisp, v1: &[Value; N], v2: &[Value; N]) -> LispResult<bool> {
    for i in 1..N {
        if !face_attr_equal_p(lisp, v1[i], v2[i])? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn f_internal_lisp_face_equal_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (face1, face2, frame, inherit) = (args[0], args[1], arg(args, 2), arg(args, 3));
    let f = if frame == sym::T {
        None
    } else {
        Some(lisp.decode_live_frame(frame)?)
    };
    let lface1 = lisp.lface_from_face_name(f, face1, true)?;
    let lface2 = lisp.lface_from_face_name(f, face2, true)?;
    let attrs1 = lisp.lface_contents(lface1);
    let attrs2 = lisp.lface_contents(lface2);
    let mut equal_p = lface_equal_p(lisp, &attrs1, &attrs2)?;
    if !(inherit.is_nil() || equal_p) {
        let check = |lisp: &mut Lisp, child: &[Value; N]| -> LispResult<bool> {
            for i in 1..N {
                if i == LFACE_INHERIT_INDEX {
                    continue;
                }
                if !(face_attr_equal_p(lisp, attrs1[i], attrs2[i])? || unspecifiedp(child[i])) {
                    return Ok(false);
                }
            }
            Ok(true)
        };
        if attrs1[LFACE_INHERIT_INDEX] == face2 {
            equal_p = check(lisp, &attrs1)?;
        } else if attrs2[LFACE_INHERIT_INDEX] == face1 {
            equal_p = check(lisp, &attrs2)?;
        }
    }
    Ok(lisp.bool(equal_p))
}

fn f_internal_lisp_face_empty_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (face, frame) = (args[0], arg(args, 1));
    let f = if frame == sym::T {
        None
    } else {
        Some(lisp.decode_live_frame(frame)?)
    };
    let lface = lisp.lface_from_face_name(f, face, true)?;
    let contents = lisp.lface_contents(lface);
    Ok(lisp.bool(contents[1..].iter().all(|&v| unspecifiedp(v))))
}

fn f_frame_face_hash_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    Ok(lisp.frame(f).face_hash_table)
}

fn f_face_attributes_as_vector(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let plist = args[0];
    let base = lisp.stack.len();
    let to = lisp.attrs_push(sym::UNSPECIFIED);
    let f = lisp.frames.selected_frame;
    let mut points = MergePoints::new();
    let r = lisp.merge_face_ref(None, Some(f), plist, to, true, &mut points, 0);
    let attrs = lisp.attrs_snapshot(to);
    lisp.stack.truncate(base);
    r?;
    Ok(lisp.vector(attrs.to_vec()))
}

fn f_display_supports_face_attributes_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (attributes, display) = (args[0], arg(args, 1));
    if lisp.noninteractive() {
        return Ok(Value::NIL);
    }
    let frame = if display.is_nil() {
        lisp.frames.selected_frame
    } else if lisp.is_frame(display) {
        display
    } else {
        let mut found = Value::NIL;
        for fr in lisp.frame_list_vec() {
            let params = lisp.frame(fr).param_alist;
            let entry = lisp.assq(sym::DISPLAY, params);
            let d = lisp.cdr_safe(entry);
            found = fr;
            if lisp.equal(d, display)? {
                break;
            }
        }
        found
    };
    lisp.check_live_frame(frame)?;
    let base = lisp.stack.len();
    let r = (|| -> LispResult<bool> {
        let attrs = lisp.attrs_push(sym::UNSPECIFIED);
        let mut points = MergePoints::new();
        lisp.merge_face_ref(None, Some(frame), attributes, attrs, true, &mut points, 0)?;
        if lisp.face_from_id(frame, DEFAULT_FACE_ID).is_none()
            && !lisp.realize_basic_faces(frame)?
        {
            return Err(lisp.error("Cannot realize default face"));
        }
        let def_face = lisp
            .face_from_id(frame, DEFAULT_FACE_ID)
            .expect("default face is not realized");
        let (def_attrs, def_font) = (def_face.lface, def_face.font);
        if lisp.frame(frame).termcap_p() {
            unreachable!("tty frames need tty_supports_face_attributes_p");
        }
        let attrs = lisp.attrs_snapshot(attrs);
        lisp.gui_supports_face_attributes_p(frame, &attrs, &def_attrs, def_font)
    })();
    lisp.stack.truncate(base);
    Ok(lisp.bool(r?))
}

fn f_color_distance(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (color1, color2, frame, metric) = (args[0], args[1], arg(args, 2), arg(args, 3));
    let f = lisp.decode_live_frame(frame)?;
    let mut cdefs = [EmacsColor::default(); 2];
    for (cdef, color) in cdefs.iter_mut().zip([color1, color2]) {
        let mut ok = color.is_cons() && lisp.parse_rgb_list(color, cdef);
        if !ok && color.is_string() {
            let name = lisp.string_bytes(color).to_vec();
            let (defined, c) = lisp.defined_color(f, &name)?;
            if defined {
                *cdef = c;
                ok = true;
            }
        }
        if !ok {
            return Err(lisp.signal_error("Invalid color", color));
        }
    }
    let [cdef1, cdef2] = cdefs;
    if metric.is_nil() {
        return Ok(Value::fixnum(color_distance(&cdef1, &cdef2)));
    }
    let rgb = |c: &EmacsColor| {
        [
            Value::fixnum(i64::from(c.red)),
            Value::fixnum(i64::from(c.green)),
            Value::fixnum(i64::from(c.blue)),
        ]
    };
    let l1 = lisp.list(&rgb(&cdef1));
    lisp.stack.push(l1);
    let l2 = lisp.list(&rgb(&cdef2));
    lisp.stack.pop();
    lisp.funcall(metric, &[l1, l2])
}

fn f_internal_set_font_selection_order(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let order = args[0];
    lisp.check_list(order)?;
    let mut indices = [0i32; 4];
    let mut i = 0;
    let mut list = order;
    while list.is_cons() && i < indices.len() {
        let attr = lisp.xcar(list);
        let xlfd = if attr == sym::K_WIDTH {
            XLFD_SWIDTH
        } else if attr == sym::K_HEIGHT {
            XLFD_POINT_SIZE
        } else if attr == sym::K_WEIGHT {
            XLFD_WEIGHT
        } else if attr == sym::K_SLANT {
            XLFD_SLANT
        } else {
            break;
        };
        if indices[i] != 0 {
            break;
        }
        indices[i] = xlfd;
        list = lisp.xcdr(list);
        i += 1;
    }
    if !list.is_nil() || i != indices.len() || indices.contains(&0) {
        return Err(lisp.signal_error("Invalid font sort order", order));
    }
    if indices != lisp.faces.font_sort_order {
        lisp.faces.font_sort_order = indices;
        lisp.free_all_realized_faces(Value::NIL);
    }
    let order = lisp.faces.font_sort_order;
    lisp.fonts.font_update_sort_order(&order);
    Ok(Value::NIL)
}

fn set_alternative_alist(
    lisp: &mut Lisp,
    alist: Value,
    convert: fn(&mut Lisp, Value) -> LispResult,
) -> LispResult {
    lisp.check_list(alist)?;
    let alist = crate::fns::f_copy_sequence(lisp, &[alist])?;
    let base = lisp.stack.len();
    lisp.stack.push(alist);
    let r = convert_alternative_alist(lisp, alist, convert);
    lisp.stack.truncate(base);
    r?;
    Ok(alist)
}

fn convert_alternative_alist(
    lisp: &mut Lisp,
    alist: Value,
    convert: fn(&mut Lisp, Value) -> LispResult,
) -> LispResult<()> {
    let mut tail = alist;
    while tail.is_cons() {
        let entry = lisp.xcar(tail);
        lisp.check_list(entry)?;
        let entry = crate::fns::f_copy_sequence(lisp, &[entry])?;
        lisp.heap.set_car(tail, entry);
        let mut tail2 = entry;
        while tail2.is_cons() {
            let elt = lisp.xcar(tail2);
            let v = convert(lisp, elt)?;
            lisp.heap.set_car(tail2, v);
            tail2 = lisp.xcdr(tail2);
        }
        tail = lisp.xcdr(tail);
    }
    Ok(())
}

fn f_internal_set_alternative_font_family_alist(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let alist = set_alternative_alist(lisp, args[0], |lisp, v| {
        crate::lread::f_intern(lisp, &[v, Value::NIL])
    })?;
    lisp.faces.alternative_font_family_alist = alist;
    lisp.free_all_realized_faces(Value::NIL);
    Ok(alist)
}

fn f_internal_set_alternative_font_registry_alist(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let alist = set_alternative_alist(lisp, args[0], |lisp, v| {
        crate::casefiddle::f_downcase(lisp, &[v])
    })?;
    lisp.faces.alternative_font_registry_alist = alist;
    lisp.free_all_realized_faces(Value::NIL);
    Ok(alist)
}

pub(crate) fn f_clear_face_cache(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.clear_face_cache(!arg(args, 0).is_nil());
    lisp.faces.face_change = true;
    lisp.windows.windows_or_buffers_changed = 53;
    Ok(Value::NIL)
}

fn f_tty_suppress_bold_inverse_default_colors(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let suppress = args[0];
    lisp.faces.tty_suppress_bold_inverse_default_colors_p = !suppress.is_nil();
    lisp.faces.face_change = true;
    Ok(suppress)
}

fn f_bitmap_spec_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let object = args[0];
    let mut pixmap_p = false;
    if object.is_string() {
        pixmap_p = true;
    } else if object.is_cons() {
        let width = lisp.xcar(object);
        let rest = lisp.xcdr(object);
        let height = lisp.car_safe(rest);
        let rest = lisp.cdr_safe(rest);
        let data = lisp.car_safe(rest);
        let ranged = |v: Value| {
            v.as_fixnum()
                .is_some_and(|n| (1..=i32::MAX as i64).contains(&n))
        };
        if data.is_string() && ranged(width) && ranged(height) {
            let bytes_per_row = (width.as_fixnum().unwrap_or(0) + 7) / 8;
            let nbytes = lisp.string_bytes(data).len() as i64;
            if height.as_fixnum().unwrap_or(0) <= nbytes / bytes_per_row {
                pixmap_p = true;
            }
        }
    }
    Ok(lisp.bool(pixmap_p))
}

fn f_color_values_from_color_spec(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let spec = lisp.check_string(args[0])?;
    match parse_color_spec(lisp.string_bytes(spec)) {
        Some((r, g, b)) => Ok(lisp.list(&[
            Value::fixnum(i64::from(r)),
            Value::fixnum(i64::from(g)),
            Value::fixnum(i64::from(b)),
        ])),
        None => Ok(Value::NIL),
    }
}

fn f_color_gray_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (color, frame) = (args[0], arg(args, 1));
    lisp.check_string(color)?;
    let f = lisp.decode_any_frame(frame)?;
    let name = lisp.string_bytes(color).to_vec();
    let r = lisp.face_color_gray_p(f, &name)?;
    Ok(lisp.bool(r))
}

fn f_color_supported_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (color, frame, background_p) = (args[0], arg(args, 1), arg(args, 2));
    lisp.check_string(color)?;
    let f = lisp.decode_any_frame(frame)?;
    let name = lisp.string_bytes(color).to_vec();
    let r = lisp.face_color_supported_p(f, &name, !background_p.is_nil())?;
    Ok(lisp.bool(r))
}

fn scan_int(line: &[u8], pos: &mut usize) -> Option<i32> {
    while line.get(*pos).is_some_and(|c| c.is_ascii_whitespace()) {
        *pos += 1;
    }
    let start = *pos;
    if matches!(line.get(*pos), Some(b'+') | Some(b'-')) {
        *pos += 1;
    }
    let digits = *pos;
    while line.get(*pos).is_some_and(u8::is_ascii_digit) {
        *pos += 1;
    }
    if *pos == digits {
        return None;
    }
    let text = std::str::from_utf8(&line[start..*pos]).ok()?;
    Some(text.parse::<i64>().map_or(-1, |n| n as i32))
}

fn f_x_load_color_file(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let filename = lisp.check_string(args[0])?;
    let abspath = lisp.expand_file_name(filename, Value::NIL)?;
    let path = std::path::PathBuf::from(lisp.string_to_rust(abspath));
    let mut cmap = Value::NIL;
    let Ok(contents) = std::fs::read(&path) else {
        return Ok(cmap);
    };
    let mut lines = Vec::new();
    let mut rest = &contents[..];
    while !rest.is_empty() {
        let limit = rest.len().min(511);
        let end = rest[..limit]
            .iter()
            .position(|&c| c == b'\n')
            .map_or(limit, |p| p + 1);
        lines.push(&rest[..end]);
        rest = &rest[end..];
    }
    for line in lines {
        let line = crate::fontset::c_bytes(line);
        let mut pos = 0;
        let (Some(red), Some(green), Some(blue)) = (
            scan_int(line, &mut pos),
            scan_int(line, &mut pos),
            scan_int(line, &mut pos),
        ) else {
            continue;
        };
        while line.get(pos).is_some_and(|c| c.is_ascii_whitespace()) {
            pos += 1;
        }
        let color = (red << 16) | (green << 8) | blue;
        let mut name = &line[pos..];
        if name.last() == Some(&b'\n') {
            name = &name[..name.len() - 1];
        }
        let base = lisp.stack.len();
        lisp.stack.push(cmap);
        let s = lisp.make_string(name);
        let cell = lisp.cons(s, Value::fixnum(i64::from(color)));
        cmap = lisp.cons(cell, cmap);
        lisp.stack.truncate(base);
    }
    Ok(cmap)
}
