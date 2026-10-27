use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cosmic_text::{Font, FontSystem, fontdb};
use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;

use crate::surface::{Rect, Surface};

pub const SPACING_PROPORTIONAL: i32 = 0;
pub const SPACING_DUAL: i32 = 90;
pub const SPACING_MONO: i32 = 100;
pub const SPACING_CHARCELL: i32 = 110;

const WEIGHT_REGULAR: i32 = 80;
const SLANT_ROMAN: i32 = 100;
const WIDTH_NORMAL: i32 = 100;

const WEIGHT_MAP: [(f64, f64); 13] = [
    (0.0, 0.0),
    (100.0, 0.0),
    (200.0, 40.0),
    (300.0, 50.0),
    (350.0, 55.0),
    (380.0, 75.0),
    (400.0, 80.0),
    (500.0, 100.0),
    (600.0, 180.0),
    (700.0, 200.0),
    (800.0, 205.0),
    (900.0, 210.0),
    (1000.0, 215.0),
];

pub fn weight_from_opentype(ot_weight: u16) -> i32 {
    let w = (ot_weight as f64).min(WEIGHT_MAP[WEIGHT_MAP.len() - 1].0);
    let mut i = 1;
    while w > WEIGHT_MAP[i].0 {
        i += 1;
    }
    let (x1, y1) = WEIGHT_MAP[i - 1];
    let (x2, y2) = WEIGHT_MAP[i];
    let value = if w == x2 {
        y2
    } else {
        y1 + (w - x1) * (y2 - y1) / (x2 - x1)
    };
    (value + 0.5) as i32
}

pub fn slant_from_style(style: fontdb::Style) -> i32 {
    100 + match style {
        fontdb::Style::Normal => 0,
        fontdb::Style::Italic => 100,
        fontdb::Style::Oblique => 110,
    }
}

pub fn width_from_stretch(stretch: fontdb::Stretch) -> i32 {
    match stretch {
        fontdb::Stretch::UltraCondensed => 50,
        fontdb::Stretch::ExtraCondensed => 63,
        fontdb::Stretch::Condensed => 75,
        fontdb::Stretch::SemiCondensed => 87,
        fontdb::Stretch::Normal => 100,
        fontdb::Stretch::SemiExpanded => 113,
        fontdb::Stretch::Expanded => 125,
        fontdb::Stretch::ExtraExpanded => 150,
        fontdb::Stretch::UltraExpanded => 200,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontEntity {
    pub id: fontdb::ID,
    pub family: String,
    pub weight: i32,
    pub slant: i32,
    pub width: i32,
    pub spacing: i32,
    pub file: Option<PathBuf>,
    pub index: u32,
    ot_weight: fontdb::Weight,
}

#[derive(Clone, Debug, Default)]
pub struct FontQuery {
    pub family: Option<String>,
    pub weight: Option<i32>,
    pub slant: Option<i32>,
    pub width: Option<i32>,
    pub spacing: Option<i32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlyphMetrics {
    pub lbearing: i32,
    pub rbearing: i32,
    pub width: i32,
    pub ascent: i32,
    pub descent: i32,
}

#[derive(Clone, Debug)]
pub struct GlyphImage {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub color: bool,
    pub data: Vec<u8>,
}

pub struct OpenFont {
    pub entity: FontEntity,
    pub pixel_size: u32,
    pub ascent: i32,
    pub descent: i32,
    pub height: i32,
    pub min_width: i32,
    pub max_width: i32,
    pub average_width: i32,
    pub space_width: i32,
    pub underline_position: i32,
    pub underline_thickness: i32,
    font: Arc<Font>,
    metrics: HashMap<u32, GlyphMetrics>,
    images: HashMap<u32, Option<GlyphImage>>,
}

#[derive(Clone, Copy, Debug)]
pub struct Background {
    pub color: u32,
    pub width: i32,
    pub respect_alpha: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct DrawGlyphs {
    pub x: i32,
    pub y: i32,
    pub foreground: u32,
    pub background: Option<Background>,
    pub padding: bool,
}

pub struct FontBackend {
    system: FontSystem,
    scale: ScaleContext,
}

impl Default for FontBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn entity_of(face: &fontdb::FaceInfo) -> FontEntity {
    let file = match &face.source {
        fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) => Some(path.clone()),
        fontdb::Source::Binary(_) => None,
    };
    FontEntity {
        id: face.id,
        family: face
            .families
            .first()
            .map(|(name, _)| name.clone())
            .unwrap_or_default(),
        weight: weight_from_opentype(face.weight.0),
        slant: slant_from_style(face.style),
        width: width_from_stretch(face.stretch),
        spacing: if face.monospaced {
            SPACING_MONO
        } else {
            SPACING_PROPORTIONAL
        },
        file,
        index: face.index,
        ot_weight: face.weight,
    }
}

const SERIF: &[&str] = &[
    "Noto Serif",
    "DejaVu Serif",
    "Times New Roman",
    "Thorndale AMT",
    "Luxi Serif",
    "Nimbus Roman No9 L",
    "Nimbus Roman",
    "Times",
];

const SANS_SERIF: &[&str] = &[
    "Noto Sans",
    "DejaVu Sans",
    "Verdana",
    "Arial",
    "Albany AMT",
    "Luxi Sans",
    "Nimbus Sans L",
    "Nimbus Sans",
    "Helvetica",
    "Lucida Sans Unicode",
    "BPG Glaho International",
    "Tahoma",
];

const MONOSPACE: &[&str] = &[
    "Noto Sans Mono",
    "DejaVu Sans Mono",
    "Inconsolata",
    "Andale Mono",
    "Courier New",
    "Cumberland AMT",
    "Luxi Mono",
    "Nimbus Mono L",
    "Nimbus Mono",
    "Nimbus Mono PS",
    "Courier",
];

fn generic_family(family: &str) -> Option<&'static [&'static str]> {
    match family.to_ascii_lowercase().as_str() {
        "monospace" | "mono" => Some(MONOSPACE),
        "sans-serif" | "sans" => Some(SANS_SERIF),
        "serif" => Some(SERIF),
        _ => None,
    }
}

fn matches(entity: &FontEntity, query: &FontQuery) -> bool {
    query
        .family
        .as_ref()
        .is_none_or(|f| entity.family.eq_ignore_ascii_case(f))
        && query.weight.is_none_or(|w| entity.weight == w)
        && query.slant.is_none_or(|s| entity.slant == s)
        && query.width.is_none_or(|w| entity.width == w)
        && query.spacing.is_none_or(|s| entity.spacing == s)
}

impl FontBackend {
    pub fn new() -> FontBackend {
        FontBackend {
            system: FontSystem::new(),
            scale: ScaleContext::new(),
        }
    }

    pub fn entities(&self) -> Vec<FontEntity> {
        self.system.db().faces().map(entity_of).collect()
    }

    pub fn list(&self, query: &FontQuery) -> Vec<FontEntity> {
        self.system
            .db()
            .faces()
            .map(entity_of)
            .filter(|e| matches(e, query))
            .collect()
    }

    fn first_installed(all: &[FontEntity], families: &[&str]) -> Vec<FontEntity> {
        families
            .iter()
            .map(|f| {
                all.iter()
                    .filter(|e| e.family.eq_ignore_ascii_case(f))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .find(|faces| !faces.is_empty())
            .unwrap_or_default()
    }

    pub fn match_font(&self, query: &FontQuery) -> Option<FontEntity> {
        let all = self.entities();
        let generic = query.family.as_deref().and_then(generic_family);
        let mut candidates = match (&query.family, generic) {
            (_, Some(prefer)) => Self::first_installed(&all, prefer),
            (Some(family), None) => Self::first_installed(&all, &[family.as_str()]),
            (None, None) => Vec::new(),
        };
        if candidates.is_empty() && generic == Some(MONOSPACE) {
            candidates = all
                .iter()
                .filter(|e| e.spacing == SPACING_MONO)
                .cloned()
                .collect();
        }
        if candidates.is_empty() {
            candidates = Self::first_installed(&all, SANS_SERIF);
        }
        if candidates.is_empty() {
            candidates = all;
        }
        let weight = query.weight.unwrap_or(WEIGHT_REGULAR);
        let slant = query.slant.unwrap_or(SLANT_ROMAN);
        let width = query.width.unwrap_or(WIDTH_NORMAL);
        candidates.into_iter().min_by_key(|e| {
            (
                (e.slant - slant).abs(),
                (e.weight - weight).abs(),
                (e.width - width).abs(),
                query.spacing.map_or(0, |s| i32::from(e.spacing != s)),
            )
        })
    }

    pub fn open(&mut self, entity: &FontEntity, pixel_size: u32) -> Option<OpenFont> {
        let font = self.system.get_font(entity.id, entity.ot_weight)?;
        let swash = font.as_swash();
        let size = pixel_size as f32;
        let design = swash.metrics(&[]);
        let scaled = design.scale(size);
        let charmap = swash.charmap();
        let ascii: Vec<(u8, u32)> = (32u8..127)
            .map(|c| (c, charmap.map(c as char) as u32))
            .collect();
        let mut open = OpenFont {
            entity: entity.clone(),
            pixel_size,
            ascent: 0,
            descent: 0,
            height: 0,
            min_width: 0,
            max_width: 0,
            average_width: 0,
            space_width: 0,
            underline_position: -1,
            underline_thickness: 0,
            font: font.clone(),
            metrics: HashMap::new(),
            images: HashMap::new(),
        };
        let mut total = 0;
        let mut n = 0;
        for (c, glyph) in ascii {
            let this_width = self.glyph_extents(&mut open, glyph).width;
            if this_width > 0 {
                if open.min_width == 0 || open.min_width > this_width {
                    open.min_width = this_width;
                }
                if this_width > open.max_width {
                    open.max_width = this_width;
                }
                if c == 32 {
                    open.space_width = this_width;
                }
                total += this_width;
                n += 1;
            }
        }
        if n > 0 {
            open.average_width = total / n;
        }
        open.ascent = scaled.ascent.ceil() as i32;
        open.descent = scaled.descent.ceil() as i32;
        open.height = open.ascent + open.descent;
        let upem = design.units_per_em as f32;
        if upem > 0.0 {
            open.underline_position = (-design.underline_offset * size / upem) as i32;
            open.underline_thickness = (design.stroke_size * size / upem) as i32;
            if open.underline_thickness > 2 {
                open.underline_position -= open.underline_thickness / 2;
            }
        }
        Some(open)
    }

    pub fn has_char(&self, font: &OpenFont, c: u32) -> bool {
        self.encode_char(font, c).is_some()
    }

    pub fn encode_char(&self, font: &OpenFont, c: u32) -> Option<u32> {
        let ch = char::from_u32(c)?;
        match font.font.as_swash().charmap().map(ch) {
            0 => None,
            glyph => Some(glyph as u32),
        }
    }

    pub fn glyph_extents(&mut self, font: &mut OpenFont, glyph: u32) -> GlyphMetrics {
        if let Some(m) = font.metrics.get(&glyph) {
            return *m;
        }
        let size = font.pixel_size as f32;
        let swash = font.font.as_swash();
        let advance = swash
            .glyph_metrics(&[])
            .scale(size)
            .advance_width(glyph as u16);
        let mut scaler = self.scale.builder(swash).size(size).hint(false).build();
        let mut metrics = GlyphMetrics {
            width: advance.round() as i32,
            ..GlyphMetrics::default()
        };
        if let Some(outline) = scaler.scale_outline(glyph as u16)
            && !outline.points().is_empty()
        {
            let bounds = outline.bounds();
            metrics.lbearing = bounds.min.x.floor() as i32;
            metrics.rbearing = bounds.max.x.ceil() as i32;
            metrics.ascent = (bounds.max.y - 1.0 / 256.0).ceil() as i32;
            metrics.descent = (-bounds.min.y).ceil() as i32;
        }
        font.metrics.insert(glyph, metrics);
        metrics
    }

    pub fn text_extents(&mut self, font: &mut OpenFont, glyphs: &[u32]) -> GlyphMetrics {
        let Some((&first, rest)) = glyphs.split_first() else {
            return GlyphMetrics::default();
        };
        let mut metrics = self.glyph_extents(font, first);
        let mut width = metrics.width;
        for &glyph in rest {
            let m = self.glyph_extents(font, glyph);
            metrics.lbearing = metrics.lbearing.min(width + m.lbearing);
            metrics.rbearing = metrics.rbearing.max(width + m.rbearing);
            metrics.ascent = metrics.ascent.max(m.ascent);
            metrics.descent = metrics.descent.max(m.descent);
            width += m.width;
        }
        metrics.width = width;
        metrics
    }

    pub fn rasterize(&mut self, font: &mut OpenFont, glyph: u32) -> Option<GlyphImage> {
        if let Some(image) = font.images.get(&glyph) {
            return image.clone();
        }
        let swash = font.font.as_swash();
        let mut scaler = self
            .scale
            .builder(swash)
            .size(font.pixel_size as f32)
            .hint(false)
            .build();
        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .render(&mut scaler, glyph as u16)
        .filter(|i| i.placement.width > 0 && i.placement.height > 0)
        .map(|i| GlyphImage {
            left: i.placement.left,
            top: i.placement.top,
            width: i.placement.width,
            height: i.placement.height,
            color: matches!(i.content, Content::Color),
            data: i.data,
        });
        font.images.insert(glyph, image.clone());
        image
    }

    pub fn draw(
        &mut self,
        surface: &mut Surface,
        font: &mut OpenFont,
        glyphs: &[u32],
        how: &DrawGlyphs,
    ) -> usize {
        if let Some(bg) = how.background {
            surface.fill_rectangle(
                bg.color,
                Rect::new(how.x, how.y - font.ascent, bg.width, font.height),
                bg.respect_alpha,
            );
        }
        let mut x = how.x;
        for &glyph in glyphs {
            if let Some(image) = self.rasterize(font, glyph) {
                let rect = Rect::new(
                    x + image.left,
                    how.y - image.top,
                    image.width as i32,
                    image.height as i32,
                );
                if image.color {
                    surface.draw_rgba(rect, &image.data);
                } else {
                    surface.draw_mask(rect, &image.data, how.foreground);
                }
            }
            x += if how.padding {
                1
            } else {
                self.glyph_extents(font, glyph).width
            };
        }
        glyphs.len()
    }
}
