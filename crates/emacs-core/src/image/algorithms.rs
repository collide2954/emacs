use super::{
    CLEAR_IMAGE_COLORS, CLEAR_IMAGE_MASK, CLEAR_IMAGE_PIXMAP, Image, PIX_MASK_DRAW,
    PIX_MASK_RETAIN, PixContainer, blue_from_ulong, four_corners_best, green_from_ulong,
    image_clear_image_1, lookup_rgb_color, red_from_ulong,
};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const EMBOSS_MATRIX: [i32; 9] = [2, -1, 0, -1, 0, 1, 0, 1, -2];
const LAPLACE_MATRIX: [i32; 9] = [1, 0, 0, 0, 0, 0, 0, 0, -1];

#[derive(Clone, Copy, Default)]
struct Color {
    red: i32,
    green: i32,
    blue: i32,
}

fn color_intensity(r: i32, g: i32, b: i32) -> i32 {
    (2 * r + 3 * g + b) / 6
}

fn image_to_emacs_colors(img: &Image) -> Vec<Color> {
    let pixmap = img.pixmap.as_ref().expect("image pixmap");
    let mut colors = Vec::with_capacity((img.width.max(0) * img.height.max(0)) as usize);
    for y in 0..img.height {
        for x in 0..img.width {
            let pixel = pixmap.get_pixel(x, y);
            colors.push(Color {
                red: (red_from_ulong(pixel) * 0x101) as i32,
                green: (green_from_ulong(pixel) * 0x101) as i32,
                blue: (blue_from_ulong(pixel) * 0x101) as i32,
            });
        }
    }
    colors
}

fn image_from_emacs_colors(img: &mut Image, colors: &[Color]) {
    image_clear_image_1(img, CLEAR_IMAGE_PIXMAP | CLEAR_IMAGE_COLORS);
    let mut ximage = PixContainer::new(img.width, img.height, 0);
    let mut p = colors.iter();
    for y in 0..img.height {
        for x in 0..img.width {
            let c = p.next().copied().unwrap_or_default();
            ximage.put_pixel(x, y, lookup_rgb_color(c.red, c.green, c.blue));
        }
    }
    img.pixmap = Some(ximage);
}

fn image_detect_edges(img: &mut Image, matrix: &[i32; 9], color_adjust: i32) {
    let colors = image_to_emacs_colors(img);
    let sum: i32 = matrix.iter().map(|m| m.abs()).sum();
    let (w, h) = (img.width.max(0) as usize, img.height.max(0) as usize);
    let mut new = vec![Color::default(); w * h];
    let half = Color {
        red: 0xffff / 2,
        green: 0xffff / 2,
        blue: 0xffff / 2,
    };
    for y in 0..h {
        new[y * w] = half;
        new[y * w + w - 1] = half;
    }
    for x in 1..w.saturating_sub(1) {
        new[x] = half;
        new[(h - 1) * w + x] = half;
    }
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let (mut r, mut g, mut b, mut i) = (0i32, 0i32, 0i32, 0usize);
            for yy in y - 1..y + 2 {
                for xx in x - 1..x + 2 {
                    if matrix[i] != 0 {
                        let t = colors[yy * w + xx];
                        r += matrix[i] * t.red;
                        g += matrix[i] * t.green;
                        b += matrix[i] * t.blue;
                    }
                    i += 1;
                }
            }
            let r = (r / sum + color_adjust) & 0xffff;
            let g = (g / sum + color_adjust) & 0xffff;
            let b = (b / sum + color_adjust) & 0xffff;
            let v = color_intensity(r, g, b);
            new[y * w + x] = Color {
                red: v,
                green: v,
                blue: v,
            };
        }
    }
    image_from_emacs_colors(img, &new);
}

fn draw_cross(pixmap: &mut PixContainer, width: i32, height: i32, color: u64) {
    let mut line = |x0: i32, y0: i32, x1: i32, y1: i32| {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            if (0..pixmap.width).contains(&x) && (0..pixmap.height).contains(&y) {
                pixmap.put_pixel(x, y, color);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    };
    line(0, 0, width - 1, height - 1);
    line(0, height - 1, width - 1, 0);
}

impl Lisp {
    pub(super) fn image_emboss(&mut self, img: &mut Image) {
        image_detect_edges(img, &EMBOSS_MATRIX, 0xffff / 2);
    }

    pub(super) fn image_laplace(&mut self, img: &mut Image) {
        image_detect_edges(img, &LAPLACE_MATRIX, 45000);
    }

    pub(super) fn image_edge_detection(
        &mut self,
        img: &mut Image,
        matrix: Value,
        color_adjust: Value,
    ) {
        let mut i = 0;
        let mut trans = [0i32; 9];
        if matrix.is_cons() {
            let mut m = matrix;
            while i < 9 && m.is_cons() && self.is_number(self.xcar(m)) {
                trans[i] = self.xfloatint(self.xcar(m)) as i32;
                i += 1;
                m = self.xcdr(m);
            }
        } else if self.is_vector(matrix) && self.vector_items(matrix).len() >= 9 {
            let items = self.vector_items(matrix).to_vec();
            while i < 9 && self.is_number(items[i]) {
                trans[i] = self.xfloatint(items[i]) as i32;
                i += 1;
            }
        }
        let color_adjust = if color_adjust.is_nil() {
            Value::fixnum(0xffff / 2)
        } else {
            color_adjust
        };
        if i == 9 && self.is_number(color_adjust) {
            let adjust = self.xfloatint(color_adjust) as i32;
            image_detect_edges(img, &trans, adjust);
        }
    }

    pub(super) fn image_disable_image(&mut self, f: Value, img: &mut Image) -> LispResult<()> {
        let terminal = self.frame(f).terminal;
        let n_planes = self.display_info(terminal).n_planes;
        if n_planes >= 2 {
            let mut colors = image_to_emacs_colors(img);
            let (h, l) = (15000, 30000);
            for p in &mut colors {
                let i = color_intensity(p.red, p.green, p.blue);
                let i2 = (0xffff - h - l) * i / 0xffff + l;
                *p = Color {
                    red: i2,
                    green: i2,
                    blue: i2,
                };
            }
            image_from_emacs_colors(img, &colors);
        }
        if n_planes < 2 || !self.plain_value(sym::CROSS_DISABLED_IMAGES).is_nil() {
            let (width, height) = (img.width, img.height);
            if let Some(pixmap) = img.pixmap.as_mut() {
                draw_cross(pixmap, width, height, 0);
            }
            if let Some(mask) = img.mask.as_mut() {
                draw_cross(mask, width, height, PIX_MASK_DRAW);
            }
        }
        Ok(())
    }

    pub(super) fn image_build_heuristic_mask(
        &mut self,
        _f: Value,
        img: &mut Image,
        how: Value,
    ) -> LispResult<()> {
        if img.mask.is_some() {
            image_clear_image_1(img, CLEAR_IMAGE_MASK);
        }
        let mut mask_img = PixContainer::new(img.width, img.height, 1);
        let ximg = img.pixmap.as_ref().expect("image pixmap");
        let mut use_img_background = true;
        let mut bg = 0;
        if how.is_cons() {
            let mut rgb = [0i32; 3];
            let mut i = 0;
            let mut how = how;
            while i < 3 && how.is_cons() && self.xcar(how).as_fixnum().is_some_and(|n| n >= 0) {
                rgb[i] = (self.xcar(how).as_fixnum().unwrap_or(0) & 0xffff) as i32;
                how = self.xcdr(how);
                i += 1;
            }
            if i == 3 && how.is_nil() {
                bg = lookup_rgb_color(rgb[0], rgb[1], rgb[2]);
                use_img_background = false;
            }
        }
        if use_img_background {
            bg = four_corners_best(ximg, &img.corners);
        }
        for y in 0..img.height {
            for x in 0..img.width {
                let pixel = if ximg.get_pixel(x, y) != bg {
                    PIX_MASK_DRAW
                } else {
                    PIX_MASK_RETAIN
                };
                mask_img.put_pixel(x, y, pixel);
            }
        }
        img.mask = Some(mask_img);
        Lisp::image_background_transparent(img);
        Ok(())
    }
}
