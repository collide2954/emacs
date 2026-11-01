use emacs_gui::pgtkterm::pgtk_parse_color;

fn rgb(name: &str) -> Option<(u16, u16, u16, i64)> {
    pgtk_parse_color(name.as_bytes()).map(|c| (c.red, c.green, c.blue, c.pixel))
}

#[test]
fn named_colors_follow_pango() {
    assert_eq!(rgb("red"), Some((65535, 0, 0, 0xFF0000)));
    let lgy = Some((64250, 64250, 53970, 0xFAFAD2));
    assert_eq!(rgb("LightGoldenrodYellow"), lgy);
    assert_eq!(rgb("light goldenrod yellow"), lgy);
    assert_eq!(rgb("LIGHTgoldenrodyellow"), lgy);
    assert_eq!(rgb("Gray50"), Some((32639, 32639, 32639, 0x7F7F7F)));
    assert_eq!(rgb("grey50"), Some((32639, 32639, 32639, 0x7F7F7F)));
    assert_eq!(rgb("no such color"), None);
    assert_eq!(rgb(""), None);
}

#[test]
fn hex_colors_follow_pango() {
    assert_eq!(rgb("#123"), Some((4369, 8738, 13107, 0x112233)));
    assert_eq!(rgb("#ff8000"), Some((65535, 32896, 0, 0xFF8000)));
    assert_eq!(rgb("#fff000111"), Some((65535, 0, 4369, 0xFF0011)));
    assert_eq!(rgb("#ffff00000000"), Some((65535, 0, 0, 0xFF0000)));
    assert_eq!(rgb("#ff0000ff"), Some((65535, 0, 0, 0xFF0000)));
    assert_eq!(rgb("#12345"), None);
    assert_eq!(rgb("#gg0000"), None);
}

#[test]
fn css_rgb_follows_gdk() {
    assert_eq!(rgb("rgb(255,0,0)"), Some((65535, 0, 0, 0xFF0000)));
    assert_eq!(
        rgb("rgb( 50% , 0 , 100% )"),
        Some((32767, 0, 65535, 0x7F00FF))
    );
    assert_eq!(rgb("rgba(0,0,255,0.5)"), Some((0, 0, 65535, 0x0000FF)));
    assert_eq!(rgb("rgb(300,0,0)"), Some((65535, 0, 0, 0xFF0000)));
    assert_eq!(rgb("rgb(255,0)"), None);
    assert_eq!(rgb("rgb(255,0,0) x"), None);
    assert_eq!(rgb("rgb 255,0,0"), None);
}
