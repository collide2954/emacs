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
