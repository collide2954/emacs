use std::rc::Rc;

use crate::bidi::{BidiDir, BidiIt, IT_STACK_SIZE};
use crate::composite::CompositionIt;
use crate::value::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DrawGlyphsFace {
    NormalText,
    InverseVideo,
    Cursor,
    MouseFace,
    ImageRaised,
    ImageSunken,
}

#[derive(Clone, Debug)]
pub struct GlyphString {
    pub f: Value,
    pub face_id: usize,
    pub font: Value,
    pub char2b: Vec<u32>,
    pub nchars: usize,
    pub width: i32,
    pub padding_p: bool,
    pub hl: DrawGlyphsFace,
    pub foreground: u64,
    pub background: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TextPos {
    pub charpos: i64,
    pub bytepos: i64,
}

impl TextPos {
    pub fn new(charpos: i64, bytepos: i64) -> TextPos {
        TextPos { charpos, bytepos }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DisplayPos {
    pub pos: TextPos,
    pub overlay_string_index: i64,
    pub string_pos: TextPos,
    pub dpvec_index: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GlyphRowArea {
    Any,
    LeftMargin,
    #[default]
    Text,
    RightMargin,
}

pub const LEFT_MARGIN_AREA: usize = 0;
pub const TEXT_AREA: usize = 1;
pub const RIGHT_MARGIN_AREA: usize = 2;
pub const LAST_AREA: usize = 3;

impl GlyphRowArea {
    pub fn index(self) -> usize {
        match self {
            GlyphRowArea::LeftMargin => LEFT_MARGIN_AREA,
            GlyphRowArea::Text => TEXT_AREA,
            GlyphRowArea::RightMargin => RIGHT_MARGIN_AREA,
            GlyphRowArea::Any => unreachable!("ANY_AREA has no glyph index"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GlyphType {
    #[default]
    Char,
    Composite,
    Glyphless,
    Image,
    Stretch,
    Xwidget,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GlyphSlice {
    #[default]
    None,
    Img {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
    Cmp {
        from: i32,
        to: i32,
    },
    Glyphless {
        upper_xoff: i16,
        upper_yoff: i16,
        lower_xoff: i16,
        lower_yoff: i16,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlyphU {
    Ch(u32),
    Cmp {
        automatic: bool,
        id: u32,
    },
    ImgId(i32),
    Stretch {
        height: u16,
        ascent: u16,
    },
    Glyphless {
        method: u8,
        for_no_font: bool,
        len: u8,
        ch: u32,
    },
}

impl Default for GlyphU {
    fn default() -> GlyphU {
        GlyphU::Ch(0)
    }
}

impl GlyphU {
    pub fn val(self) -> u32 {
        match self {
            GlyphU::Ch(c) => c,
            GlyphU::Cmp { automatic, id } => u32::from(automatic) | (id << 1),
            GlyphU::ImgId(id) => id as u32,
            GlyphU::Stretch { height, ascent } => u32::from(height) | (u32::from(ascent) << 16),
            GlyphU::Glyphless {
                method,
                for_no_font,
                len,
                ch,
            } => {
                u32::from(method & 3)
                    | (u32::from(for_no_font) << 2)
                    | (u32::from(len & 15) << 3)
                    | ((ch & 0x1FF_FFFF) << 7)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Glyph {
    pub charpos: i64,
    pub object: Value,
    pub frame: Value,
    pub pixel_width: i16,
    pub ascent: i16,
    pub descent: i16,
    pub voffset: i16,
    pub kind: GlyphType,
    pub multibyte_p: bool,
    pub left_box_line_p: bool,
    pub right_box_line_p: bool,
    pub overlaps_vertically_p: bool,
    pub padding_p: bool,
    pub glyph_not_available_p: bool,
    pub avoid_cursor_p: bool,
    pub resolved_level: u8,
    pub bidi_type: u8,
    pub face_id: u32,
    pub font_type: u8,
    pub slice: GlyphSlice,
    pub u: GlyphU,
}

impl Glyph {
    pub fn slice_equal_p(&self, other: &Glyph) -> bool {
        match self.kind {
            GlyphType::Image => self.slice == other.slice,
            GlyphType::Composite => match (self.slice, other.slice) {
                (GlyphSlice::Cmp { from: a, .. }, GlyphSlice::Cmp { from: b, .. }) => a == b,
                (a, b) => a == b,
            },
            _ => true,
        }
    }

    pub fn equal_p(&self, other: &Glyph) -> bool {
        self.kind == other.kind
            && self.u.val() == other.u.val()
            && self.slice_equal_p(other)
            && self.face_id == other.face_id
            && self.frame == other.frame
            && self.padding_p == other.padding_p
            && self.left_box_line_p == other.left_box_line_p
            && self.right_box_line_p == other.right_box_line_p
            && self.voffset == other.voffset
            && self.pixel_width == other.pixel_width
    }
}

#[derive(Clone, Debug, Default)]
pub struct GlyphRow {
    pub glyphs: Vec<Glyph>,
    pub area: [usize; LAST_AREA + 1],
    pub used: [i16; LAST_AREA + 1],
    pub hash: u32,
    pub x: i32,
    pub y: i32,
    pub pixel_width: i32,
    pub ascent: i32,
    pub height: i32,
    pub phys_ascent: i32,
    pub phys_height: i32,
    pub visible_height: i32,
    pub extra_line_spacing: i32,
    pub extra_line_spacing_above: i32,
    pub start: DisplayPos,
    pub end: DisplayPos,
    pub minpos: TextPos,
    pub maxpos: TextPos,
    pub overlay_arrow_bitmap: i32,
    pub left_user_fringe_bitmap: u32,
    pub right_user_fringe_bitmap: u32,
    pub left_fringe_bitmap: u32,
    pub right_fringe_bitmap: u32,
    pub left_user_fringe_face_id: u32,
    pub right_user_fringe_face_id: u32,
    pub left_fringe_face_id: u32,
    pub right_fringe_face_id: u32,
    pub left_fringe_offset: i32,
    pub right_fringe_offset: i32,
    pub fringe_bitmap_periodic_p: bool,
    pub redraw_fringe_bitmaps_p: bool,
    pub enabled_p: bool,
    pub truncated_on_left_p: bool,
    pub truncated_on_right_p: bool,
    pub continued_p: bool,
    pub displays_text_p: bool,
    pub ends_at_zv_p: bool,
    pub fill_line_p: bool,
    pub indicate_empty_line_p: bool,
    pub contains_overlapping_glyphs_p: bool,
    pub full_width_p: bool,
    pub mode_line_p: bool,
    pub tab_line_p: bool,
    pub overlapped_p: bool,
    pub ends_in_middle_of_char_p: bool,
    pub starts_in_middle_of_char_p: bool,
    pub overlapping_p: bool,
    pub mouse_face_p: bool,
    pub ends_in_newline_from_string_p: bool,
    pub exact_window_width_line_p: bool,
    pub cursor_in_fringe_p: bool,
    pub ends_in_ellipsis_p: bool,
    pub indicate_bob_p: bool,
    pub indicate_top_line_p: bool,
    pub indicate_eob_p: bool,
    pub indicate_bottom_line_p: bool,
    pub reversed_p: bool,
    pub stipple_p: bool,
    pub continuation_lines_width: i32,
    pub clip: Option<(i32, i32, i32, i32)>,
}

impl GlyphRow {
    pub fn area_glyphs(&self, area: usize) -> &[Glyph] {
        let start = self.area[area];
        &self.glyphs[start..start + self.used[area] as usize]
    }

    pub fn area_capacity(&self, area: usize) -> usize {
        self.area[area + 1].saturating_sub(self.area[area])
    }

    pub fn bottom_y(&self) -> i32 {
        self.y + self.height
    }

    pub fn start_charpos(&self) -> i64 {
        self.minpos.charpos
    }

    pub fn end_charpos(&self) -> i64 {
        self.maxpos.charpos
    }

    pub fn continuation_line_p(&self) -> bool {
        self.continuation_lines_width > 0
    }
}

#[derive(Clone, Debug, Default)]
pub struct GlyphMatrix {
    pub rows: Vec<GlyphRow>,
    pub nrows: i32,
    pub matrix_x: i32,
    pub matrix_y: i32,
    pub matrix_w: i32,
    pub matrix_h: i32,
    pub window_pixel_left: i32,
    pub window_pixel_top: i32,
    pub window_height: i32,
    pub window_width: i32,
    pub window_vscroll: i32,
    pub left_margin_glyphs: i32,
    pub right_margin_glyphs: i32,
    pub no_scrolling_p: bool,
    pub tab_line_p: bool,
    pub header_line_p: bool,
    pub buffer: Value,
    pub begv: i64,
    pub zv: i64,
}

impl GlyphMatrix {
    pub fn mode_line_row(&self) -> usize {
        (self.nrows - 1) as usize
    }

    pub fn header_line_row(&self) -> usize {
        usize::from(self.tab_line_p)
    }

    pub fn first_text_row(&self) -> usize {
        if self.rows[0].mode_line_p {
            if self.rows[1].mode_line_p { 2 } else { 1 }
        } else {
            0
        }
    }

    pub fn trace(&self, out: &mut Vec<Value>) {
        out.push(self.buffer);
        for row in self.rows.iter().take(self.nrows.max(0) as usize) {
            if row.enabled_p {
                for area in LEFT_MARGIN_AREA..LAST_AREA {
                    out.extend(row.area_glyphs(area).iter().map(|g| g.object));
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlyphRowRef {
    Window {
        window: Value,
        desired: bool,
        vpos: usize,
    },
    Scratch,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DisplayElementType {
    #[default]
    Character,
    Composition,
    Glyphless,
    Image,
    Stretch,
    Eob,
    Truncation,
    Continuation,
    Xwidget,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ItMethod {
    #[default]
    FromBuffer,
    FromDisplayVector,
    FromString,
    FromCString,
    FromImage,
    FromStretch,
    FromXwidget,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LineWrap {
    #[default]
    Truncate,
    WordWrap,
    WindowWrap,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GlyphlessMethod {
    #[default]
    ThinSpace,
    EmptyBox,
    Acronym,
    HexCode,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropHandled {
    Normally,
    RecomputeProps,
    OverlayStringConsumed,
    Return,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveItResult {
    Undefined,
    PosMatchOrZv,
    XReached,
    LineContinued,
    LineTruncated,
    NewlineOrCr,
}

pub const MOVE_TO_X: i32 = 0x01;
pub const MOVE_TO_Y: i32 = 0x02;
pub const MOVE_TO_VPOS: i32 = 0x04;
pub const MOVE_TO_POS: i32 = 0x08;

pub const OVERLAY_STRING_CHUNK_SIZE: usize = 16;
pub const TEXT_PROP_DISTANCE_LIMIT: i64 = 100;

#[derive(Clone, Copy, Debug)]
pub struct ItSlice {
    pub x: Value,
    pub y: Value,
    pub width: Value,
    pub height: Value,
}

impl Default for ItSlice {
    fn default() -> Self {
        ItSlice {
            x: Value::NIL,
            y: Value::NIL,
            width: Value::NIL,
            height: Value::NIL,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DpVec {
    #[default]
    None,
    Lisp(Value),
    CtlChars,
}

#[derive(Clone, Debug)]
pub struct IteratorStackEntry {
    pub string: Value,
    pub string_nchars: i64,
    pub end_charpos: i64,
    pub stop_charpos: i64,
    pub prev_stop: i64,
    pub base_level_stop: i64,
    pub cmp_it: CompositionIt,
    pub face_id: i32,
    pub u_object: Value,
    pub u_slice: ItSlice,
    pub u_image_id: i64,
    pub position: TextPos,
    pub current: DisplayPos,
    pub from_overlay: Value,
    pub area: GlyphRowArea,
    pub method: ItMethod,
    pub paragraph_embedding: BidiDir,
    pub multibyte_p: bool,
    pub string_from_display_prop_p: bool,
    pub string_from_prefix_prop_p: bool,
    pub display_ellipsis_p: bool,
    pub avoid_cursor_p: bool,
    pub bidi_p: bool,
    pub from_disp_prop_p: bool,
    pub line_wrap: LineWrap,
    pub voffset: i16,
    pub space_width: Value,
    pub font_height: Value,
}

impl Default for IteratorStackEntry {
    fn default() -> Self {
        IteratorStackEntry {
            string: Value::NIL,
            string_nchars: 0,
            end_charpos: 0,
            stop_charpos: 0,
            prev_stop: 0,
            base_level_stop: 0,
            cmp_it: CompositionIt::default(),
            face_id: 0,
            u_object: Value::NIL,
            u_slice: ItSlice::default(),
            u_image_id: 0,
            position: TextPos::default(),
            current: DisplayPos::default(),
            from_overlay: Value::NIL,
            area: GlyphRowArea::LeftMargin,
            method: ItMethod::FromBuffer,
            paragraph_embedding: BidiDir::NeutralDir,
            multibyte_p: false,
            string_from_display_prop_p: false,
            string_from_prefix_prop_p: false,
            display_ellipsis_p: false,
            avoid_cursor_p: false,
            bidi_p: false,
            from_disp_prop_p: false,
            line_wrap: LineWrap::Truncate,
            voffset: 0,
            space_width: Value::NIL,
            font_height: Value::NIL,
        }
    }
}

#[derive(Clone, Debug)]
pub struct It {
    pub window: Value,
    pub f: Value,
    pub method: ItMethod,
    pub stop_charpos: i64,
    pub prev_stop: i64,
    pub base_level_stop: i64,
    pub end_charpos: i64,
    pub medium_narrowing_begv: i64,
    pub medium_narrowing_zv: i64,
    pub large_narrowing_begv: i64,
    pub large_narrowing_zv: i64,
    pub s: Option<Rc<[u8]>>,
    pub string_nchars: i64,
    pub multibyte_p: bool,
    pub tab_line_p: bool,
    pub header_line_p: bool,
    pub string_from_display_prop_p: bool,
    pub string_from_prefix_prop_p: bool,
    pub from_disp_prop_p: bool,
    pub ellipsis_p: bool,
    pub avoid_cursor_p: bool,
    pub dp: Value,
    pub dpvec: DpVec,
    pub dpend: i64,
    pub dpvec_char_len: i32,
    pub dpvec_face_id: i32,
    pub saved_face_id: i32,
    pub ctl_chars: [Value; 16],
    pub start: DisplayPos,
    pub current: DisplayPos,
    pub n_overlay_strings: i64,
    pub overlay_strings_charpos: i64,
    pub overlay_strings: [Value; OVERLAY_STRING_CHUNK_SIZE],
    pub string_overlays: [Value; OVERLAY_STRING_CHUNK_SIZE],
    pub string: Value,
    pub from_overlay: Value,
    pub stack: [IteratorStackEntry; IT_STACK_SIZE],
    pub sp: i32,
    pub selective: i64,
    pub what: DisplayElementType,
    pub face_id: i32,
    pub selective_display_ellipsis_p: bool,
    pub ctl_arrow_p: bool,
    pub face_box_p: bool,
    pub start_of_box_run_p: bool,
    pub end_of_box_run_p: bool,
    pub overlay_strings_at_end_processed_p: bool,
    pub ignore_overlay_strings_at_pos_p: bool,
    pub glyph_not_available_p: bool,
    pub starts_in_middle_of_char_p: bool,
    pub face_before_selective_p: bool,
    pub constrain_row_ascent_descent_p: bool,
    pub line_number_produced_p: bool,
    pub align_visually_p: bool,
    pub line_wrap: LineWrap,
    pub base_face_id: i32,
    pub c: i32,
    pub len: i32,
    pub cmp_it: CompositionIt,
    pub char_to_display: i32,
    pub glyphless_method: GlyphlessMethod,
    pub image_id: i64,
    pub slice: ItSlice,
    pub space_width: Value,
    pub voffset: i16,
    pub tab_width: i16,
    pub font_height: Value,
    pub object: Value,
    pub position: TextPos,
    pub truncation_pixel_width: i16,
    pub continuation_pixel_width: i16,
    pub first_visible_x: i32,
    pub last_visible_x: i32,
    pub last_visible_y: i32,
    pub extra_line_spacing: i32,
    pub extra_line_spacing_above: i32,
    pub max_extra_line_spacing: i32,
    pub override_ascent: i32,
    pub override_descent: i32,
    pub override_boff: i32,
    pub glyph_row: Option<GlyphRowRef>,
    pub area: GlyphRowArea,
    pub nglyphs: i32,
    pub pixel_width: i32,
    pub ascent: i32,
    pub descent: i32,
    pub max_ascent: i32,
    pub max_descent: i32,
    pub phys_ascent: i32,
    pub phys_descent: i32,
    pub max_phys_ascent: i32,
    pub max_phys_descent: i32,
    pub current_x: i32,
    pub wrap_prefix_width: i32,
    pub continuation_lines_width: i32,
    pub eol_pos: TextPos,
    pub current_y: i32,
    pub first_vpos: i32,
    pub vpos: i32,
    pub hpos: i32,
    pub lnum: i64,
    pub lnum_bytepos: i64,
    pub lnum_width: i32,
    pub lnum_pixel_width: i32,
    pub pt_lnum: i64,
    pub stretch_adjust: i32,
    pub left_user_fringe_bitmap: u32,
    pub right_user_fringe_bitmap: u32,
    pub left_user_fringe_face_id: u32,
    pub right_user_fringe_face_id: u32,
    pub bidi_p: bool,
    pub bidi_it: BidiIt,
    pub paragraph_embedding: BidiDir,
    pub min_width_property: Value,
    pub min_width_start: i32,
}

impl Default for It {
    fn default() -> Self {
        It {
            window: Value::NIL,
            f: Value::NIL,
            method: ItMethod::FromBuffer,
            stop_charpos: 0,
            prev_stop: 0,
            base_level_stop: 0,
            end_charpos: 0,
            medium_narrowing_begv: 0,
            medium_narrowing_zv: 0,
            large_narrowing_begv: 0,
            large_narrowing_zv: 0,
            s: None,
            string_nchars: 0,
            multibyte_p: false,
            tab_line_p: false,
            header_line_p: false,
            string_from_display_prop_p: false,
            string_from_prefix_prop_p: false,
            from_disp_prop_p: false,
            ellipsis_p: false,
            avoid_cursor_p: false,
            dp: Value::NIL,
            dpvec: DpVec::None,
            dpend: 0,
            dpvec_char_len: 0,
            dpvec_face_id: 0,
            saved_face_id: 0,
            ctl_chars: [Value::NIL; 16],
            start: DisplayPos::default(),
            current: DisplayPos::default(),
            n_overlay_strings: 0,
            overlay_strings_charpos: 0,
            overlay_strings: [Value::NIL; OVERLAY_STRING_CHUNK_SIZE],
            string_overlays: [Value::NIL; OVERLAY_STRING_CHUNK_SIZE],
            string: Value::NIL,
            from_overlay: Value::NIL,
            stack: std::array::from_fn(|_| IteratorStackEntry::default()),
            sp: 0,
            selective: 0,
            what: DisplayElementType::Character,
            face_id: 0,
            selective_display_ellipsis_p: false,
            ctl_arrow_p: false,
            face_box_p: false,
            start_of_box_run_p: false,
            end_of_box_run_p: false,
            overlay_strings_at_end_processed_p: false,
            ignore_overlay_strings_at_pos_p: false,
            glyph_not_available_p: false,
            starts_in_middle_of_char_p: false,
            face_before_selective_p: false,
            constrain_row_ascent_descent_p: false,
            line_number_produced_p: false,
            align_visually_p: false,
            line_wrap: LineWrap::Truncate,
            base_face_id: 0,
            c: 0,
            len: 0,
            cmp_it: CompositionIt::default(),
            char_to_display: 0,
            glyphless_method: GlyphlessMethod::ThinSpace,
            image_id: 0,
            slice: ItSlice::default(),
            space_width: Value::NIL,
            voffset: 0,
            tab_width: 0,
            font_height: Value::NIL,
            object: Value::NIL,
            position: TextPos::default(),
            truncation_pixel_width: 0,
            continuation_pixel_width: 0,
            first_visible_x: 0,
            last_visible_x: 0,
            last_visible_y: 0,
            extra_line_spacing: 0,
            extra_line_spacing_above: 0,
            max_extra_line_spacing: 0,
            override_ascent: 0,
            override_descent: 0,
            override_boff: 0,
            glyph_row: None,
            area: GlyphRowArea::LeftMargin,
            nglyphs: 0,
            pixel_width: 0,
            ascent: 0,
            descent: 0,
            max_ascent: 0,
            max_descent: 0,
            phys_ascent: 0,
            phys_descent: 0,
            max_phys_ascent: 0,
            max_phys_descent: 0,
            current_x: 0,
            wrap_prefix_width: 0,
            continuation_lines_width: 0,
            eol_pos: TextPos::default(),
            current_y: 0,
            first_vpos: 0,
            vpos: 0,
            hpos: 0,
            lnum: 0,
            lnum_bytepos: 0,
            lnum_width: 0,
            lnum_pixel_width: 0,
            pt_lnum: 0,
            stretch_adjust: 0,
            left_user_fringe_bitmap: 0,
            right_user_fringe_bitmap: 0,
            left_user_fringe_face_id: 0,
            right_user_fringe_face_id: 0,
            bidi_p: false,
            bidi_it: BidiIt::default(),
            paragraph_embedding: BidiDir::NeutralDir,
            min_width_property: Value::NIL,
            min_width_start: 0,
        }
    }
}

impl It {
    pub fn charpos(&self) -> i64 {
        self.current.pos.charpos
    }

    pub fn bytepos(&self) -> i64 {
        self.current.pos.bytepos
    }

    pub fn string_charpos(&self) -> i64 {
        self.current.string_pos.charpos
    }

    pub fn string_bytepos(&self) -> i64 {
        self.current.string_pos.bytepos
    }

    pub fn at_end_p(&self) -> bool {
        self.what == DisplayElementType::Eob
    }

    pub fn at_end_of_line_p(&self) -> bool {
        self.what == DisplayElementType::Character
            && (self.c == 10 || (self.c == 13 && self.selective != 0))
    }

    pub fn trace(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.window,
            self.f,
            self.dp,
            self.string,
            self.from_overlay,
            self.space_width,
            self.font_height,
            self.object,
            self.min_width_property,
            self.slice.x,
            self.slice.y,
            self.slice.width,
            self.slice.height,
        ]);
        if let DpVec::Lisp(v) = self.dpvec {
            out.push(v);
        }
        out.extend_from_slice(&self.ctl_chars);
        out.extend_from_slice(&self.overlay_strings);
        out.extend_from_slice(&self.string_overlays);
        for entry in &self.stack {
            out.extend_from_slice(&[
                entry.string,
                entry.u_object,
                entry.from_overlay,
                entry.space_width,
                entry.font_height,
                entry.u_slice.x,
                entry.u_slice.y,
                entry.u_slice.width,
                entry.u_slice.height,
            ]);
        }
        self.bidi_it.trace(out);
    }
}
