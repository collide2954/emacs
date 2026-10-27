use cosmic_text::fontdb::{Stretch, Style};
use emacs_gui::font::{
    Background, DrawGlyphs, FontBackend, FontEntity, FontQuery, SPACING_MONO, slant_from_style,
    weight_from_opentype, width_from_stretch,
};
use emacs_gui::surface::Surface;

fn monospace(backend: &FontBackend) -> FontEntity {
    backend
        .match_font(&FontQuery {
            family: Some("monospace".into()),
            ..FontQuery::default()
        })
        .expect("a monospace font is installed")
}

#[test]
fn numeric_style_mapping() {
    let weights = [
        (100, 0),
        (200, 40),
        (300, 50),
        (350, 55),
        (380, 75),
        (400, 80),
        (450, 90),
        (500, 100),
        (600, 180),
        (700, 200),
        (800, 205),
        (900, 210),
        (1000, 215),
        (1200, 215),
    ];
    for (ot, fc) in weights {
        assert_eq!(weight_from_opentype(ot), fc, "weight {ot}");
    }
    assert_eq!(slant_from_style(Style::Normal), 100);
    assert_eq!(slant_from_style(Style::Italic), 200);
    assert_eq!(slant_from_style(Style::Oblique), 210);
    let widths = [
        (Stretch::UltraCondensed, 50),
        (Stretch::ExtraCondensed, 63),
        (Stretch::Condensed, 75),
        (Stretch::SemiCondensed, 87),
        (Stretch::Normal, 100),
        (Stretch::SemiExpanded, 113),
        (Stretch::Expanded, 125),
        (Stretch::ExtraExpanded, 150),
        (Stretch::UltraExpanded, 200),
    ];
    for (stretch, fc) in widths {
        assert_eq!(width_from_stretch(stretch), fc, "{stretch:?}");
    }
}

#[test]
fn listing_and_matching() {
    let backend = FontBackend::new();
    let entity = monospace(&backend);
    assert_eq!(entity.spacing, SPACING_MONO);
    assert_eq!((entity.slant, entity.weight), (100, 80));
    let listed = backend.list(&FontQuery {
        family: Some(entity.family.to_lowercase()),
        ..FontQuery::default()
    });
    assert!(listed.iter().any(|e| e.id == entity.id));
    assert!(
        listed
            .iter()
            .all(|e| e.family.eq_ignore_ascii_case(&entity.family))
    );
    let bold = backend.list(&FontQuery {
        family: Some(entity.family.clone()),
        weight: Some(200),
        ..FontQuery::default()
    });
    assert!(bold.iter().all(|e| e.weight == 200));
    assert!(
        backend
            .list(&FontQuery {
                family: Some("no such family anywhere".into()),
                ..FontQuery::default()
            })
            .is_empty()
    );
    let italic = backend
        .match_font(&FontQuery {
            family: Some(entity.family.clone()),
            slant: Some(200),
            ..FontQuery::default()
        })
        .expect("match");
    assert!(italic.family.eq_ignore_ascii_case(&entity.family));
}

#[test]
fn open_font_metrics() {
    let mut backend = FontBackend::new();
    let entity = monospace(&backend);
    let font = backend.open(&entity, 16).expect("open");
    assert_eq!(font.pixel_size, 16);
    assert!(font.ascent > 0 && font.descent > 0);
    assert_eq!(font.height, font.ascent + font.descent);
    assert!(font.space_width > 0);
    assert_eq!(font.min_width, font.max_width);
    assert_eq!(font.average_width, font.space_width);
    assert!(font.underline_position >= 0);
}

#[test]
fn glyphs_and_extents() {
    let mut backend = FontBackend::new();
    let entity = monospace(&backend);
    let mut font = backend.open(&entity, 16).expect("open");
    let a = backend.encode_char(&font, 'A' as u32).expect("A");
    let b = backend.encode_char(&font, 'B' as u32).expect("B");
    assert_ne!(a, b);
    assert_eq!(backend.encode_char(&font, 0x10_FFFD), None);
    assert_eq!(backend.encode_char(&font, 0x3F_FF80), None);
    let ma = backend.glyph_extents(&mut font, a);
    assert_eq!(ma.width, font.space_width);
    assert!(ma.ascent > 0 && ma.rbearing > ma.lbearing);
    let both = backend.text_extents(&mut font, &[a, b]);
    assert_eq!(both.width, 2 * ma.width);
    assert!(both.lbearing <= ma.lbearing);
    assert!(both.rbearing > ma.width);
    assert!(both.ascent >= ma.ascent);
}

#[test]
fn rasterize_and_draw() {
    let mut backend = FontBackend::new();
    let entity = monospace(&backend);
    let mut font = backend.open(&entity, 24).expect("open");
    let a = backend.encode_char(&font, 'A' as u32).expect("A");
    let image = backend.rasterize(&mut font, a).expect("A has an image");
    assert!(image.width > 0 && image.height > 0 && !image.color);
    assert!(image.data.iter().any(|&v| v > 128));
    let space = backend.encode_char(&font, ' ' as u32).expect("space");
    assert!(backend.rasterize(&mut font, space).is_none());

    let mut surface = Surface::new(64, 40, 0xFFFFFF);
    let drawn = backend.draw(
        &mut surface,
        &mut font,
        &[a, a],
        &DrawGlyphs {
            x: 4,
            y: 30,
            foreground: 0x000000,
            background: None,
            padding: false,
        },
    );
    assert_eq!(drawn, 2);
    let dark = (0..64)
        .flat_map(|x| (0..40).map(move |y| (x, y)))
        .filter(|&(x, y)| surface.pixel(x, y).expect("pixel")[0] < 100)
        .count();
    assert!(dark > 10, "{dark}");

    let mut surface = Surface::new(64, 40, 0xFFFFFF);
    backend.draw(
        &mut surface,
        &mut font,
        &[space],
        &DrawGlyphs {
            x: 4,
            y: 30,
            foreground: 0x000000,
            background: Some(Background {
                color: 0x0000FF,
                width: 20,
                respect_alpha: false,
            }),
            padding: false,
        },
    );
    assert_eq!(surface.pixel(5, 29), Some([0, 0, 255, 255]));
    assert_eq!(surface.pixel(30, 29), Some([255, 255, 255, 255]));
}
