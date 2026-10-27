use emacs_gui::surface::{Rect, Surface};

const RED: u32 = 0xFF0000;
const BG: u32 = 0x112233;

fn px(s: &Surface, x: u32, y: u32) -> [u8; 4] {
    s.pixel(x, y).expect("pixel in bounds")
}

#[test]
fn new_surface_is_background() {
    let s = Surface::new(4, 3, BG);
    assert_eq!((s.width(), s.height()), (4, 3));
    assert_eq!(px(&s, 0, 0), [0x11, 0x22, 0x33, 255]);
    assert_eq!(px(&s, 3, 2), [0x11, 0x22, 0x33, 255]);
    assert_eq!(s.rgba().len(), 4 * 3 * 4);
    assert_eq!(s.pixel(4, 0), None);
}

#[test]
fn fill_rectangle_stays_in_bounds() {
    let mut s = Surface::new(4, 3, BG);
    s.fill_rectangle(RED, Rect::new(1, 1, 2, 1), false);
    assert_eq!(px(&s, 1, 1), [255, 0, 0, 255]);
    assert_eq!(px(&s, 2, 1), [255, 0, 0, 255]);
    assert_eq!(px(&s, 0, 1), [0x11, 0x22, 0x33, 255]);
    assert_eq!(px(&s, 3, 1), [0x11, 0x22, 0x33, 255]);
    assert_eq!(px(&s, 1, 0), [0x11, 0x22, 0x33, 255]);
    s.fill_rectangle(RED, Rect::new(-2, -2, 3, 3), false);
    assert_eq!(px(&s, 0, 0), [255, 0, 0, 255]);
    assert_eq!(px(&s, 1, 0), [0x11, 0x22, 0x33, 255]);
}

#[test]
fn clear_area_uses_background_and_alpha() {
    let mut s = Surface::new(2, 2, BG);
    s.fill_rectangle(RED, Rect::new(0, 0, 2, 2), false);
    s.set_alpha_background(0.5);
    s.clear_area(Rect::new(0, 0, 1, 1));
    let [r, g, b, a] = px(&s, 0, 0);
    assert!((127..=128).contains(&a), "alpha {a}");
    assert!(r.abs_diff(0x11) <= 2 && g.abs_diff(0x22) <= 2 && b.abs_diff(0x33) <= 2);
    assert_eq!(px(&s, 1, 1), [255, 0, 0, 255]);
}

#[test]
fn copy_bits_moves_pixels() {
    let mut s = Surface::new(4, 4, BG);
    s.fill_rectangle(RED, Rect::new(0, 0, 1, 1), false);
    s.copy_bits(Rect::new(0, 0, 2, 2), Rect::new(1, 1, 2, 2));
    assert_eq!(px(&s, 1, 1), [255, 0, 0, 255]);
    assert_eq!(px(&s, 2, 2), [0x11, 0x22, 0x33, 255]);
    assert_eq!(px(&s, 0, 0), [255, 0, 0, 255]);
    s.fill_rectangle(0x00FF00, Rect::new(0, 3, 4, 1), false);
    s.copy_bits(Rect::new(0, 1, 4, 3), Rect::new(0, 0, 4, 3));
    assert_eq!(px(&s, 1, 0), [255, 0, 0, 255]);
    assert_eq!(px(&s, 0, 2), [0, 255, 0, 255]);
    assert_eq!(px(&s, 0, 3), [0, 255, 0, 255]);
}

#[test]
fn clip_limits_drawing() {
    let mut s = Surface::new(4, 4, BG);
    s.set_clip_rectangles(&[Rect::new(0, 0, 2, 2), Rect::new(3, 3, 1, 1)]);
    s.fill_rectangle(RED, Rect::new(0, 0, 4, 4), false);
    assert_eq!(px(&s, 1, 1), [255, 0, 0, 255]);
    assert_eq!(px(&s, 3, 3), [255, 0, 0, 255]);
    assert_eq!(px(&s, 2, 2), [0x11, 0x22, 0x33, 255]);
    s.reset_clip();
    s.fill_rectangle(RED, Rect::new(0, 0, 4, 4), false);
    assert_eq!(px(&s, 2, 2), [255, 0, 0, 255]);
}

#[test]
fn draw_mask_blends_coverage() {
    let mut s = Surface::new(3, 1, 0x000000);
    s.draw_mask(Rect::new(0, 0, 2, 1), &[255, 128], 0xFFFFFF);
    assert_eq!(px(&s, 0, 0), [255, 255, 255, 255]);
    let [r, g, b, a] = px(&s, 1, 0);
    assert_eq!(a, 255);
    assert!(r.abs_diff(128) <= 1 && g == r && b == r, "{r}");
    assert_eq!(px(&s, 2, 0), [0, 0, 0, 255]);
}

#[test]
fn blits_crop_at_edges() {
    let mut s = Surface::new(2, 2, 0x000000);
    s.draw_mask(Rect::new(-1, -1, 2, 2), &[255, 255, 255, 128], 0xFFFFFF);
    let [r, _, _, a] = px(&s, 0, 0);
    assert!(r.abs_diff(128) <= 1 && a == 255, "{r}");
    assert_eq!(px(&s, 1, 0), [0, 0, 0, 255]);
    assert_eq!(px(&s, 1, 1), [0, 0, 0, 255]);
    s.copy_bits(Rect::new(0, 0, 2, 2), Rect::new(1, -1, 2, 2));
    assert_eq!(px(&s, 1, 0), px(&s, 0, 1));
}

#[test]
fn draw_rgba_composites_over() {
    let mut s = Surface::new(2, 1, 0x000000);
    s.draw_rgba(Rect::new(0, 0, 2, 1), &[255, 0, 0, 255, 0, 0, 255, 0]);
    assert_eq!(px(&s, 0, 0), [255, 0, 0, 255]);
    assert_eq!(px(&s, 1, 0), [0, 0, 0, 255]);
}

#[test]
fn resize_clears_to_background() {
    let mut s = Surface::new(2, 2, BG);
    s.fill_rectangle(RED, Rect::new(0, 0, 2, 2), false);
    s.resize(5, 3);
    assert_eq!((s.width(), s.height()), (5, 3));
    assert_eq!(px(&s, 4, 2), [0x11, 0x22, 0x33, 255]);
    assert_eq!(px(&s, 0, 0), [0x11, 0x22, 0x33, 255]);
}
