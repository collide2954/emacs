use tiny_skia::{
    BlendMode, Color, FillRule, LineCap, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint,
    PremultipliedColorU8, Stroke, StrokeDash, Transform,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn intersect(self, other: Rect) -> Option<Rect> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.width).min(other.x + other.width);
        let y1 = (self.y + self.height).min(other.y + other.height);
        (x0 < x1 && y0 < y1).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
    }

    fn to_skia(self) -> Option<tiny_skia::Rect> {
        if self.width <= 0 || self.height <= 0 {
            return None;
        }
        tiny_skia::Rect::from_xywh(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        )
    }
}

pub struct Surface {
    pixmap: Pixmap,
    background: u32,
    alpha_background: f32,
    clip: Option<Mask>,
    clip_stack: Vec<Option<Mask>>,
}

fn rgb(pixel: u32) -> (u8, u8, u8) {
    ((pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8)
}

fn color(pixel: u32, alpha: f32) -> Color {
    let (r, g, b) = rgb(pixel);
    Color::from_rgba8(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn premultiply(value: u8, alpha: u8) -> u8 {
    ((value as u16 * alpha as u16 + 127) / 255) as u8
}

fn premultiplied(r: u8, g: u8, b: u8, a: u8) -> PremultipliedColorU8 {
    PremultipliedColorU8::from_rgba(premultiply(r, a), premultiply(g, a), premultiply(b, a), a)
        .unwrap_or(PremultipliedColorU8::TRANSPARENT)
}

fn new_pixmap(width: u32, height: u32) -> Pixmap {
    Pixmap::new(width.max(1), height.max(1)).expect("surface size within limits")
}

impl Surface {
    pub fn new(width: u32, height: u32, background: u32) -> Surface {
        let mut surface = Surface {
            pixmap: new_pixmap(width, height),
            background,
            alpha_background: 1.0,
            clip: None,
            clip_stack: Vec::new(),
        };
        surface.pixmap.fill(color(background, 1.0));
        surface
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    pub fn background(&self) -> u32 {
        self.background
    }

    pub fn set_background(&mut self, pixel: u32) {
        self.background = pixel;
    }

    pub fn alpha_background(&self) -> f32 {
        self.alpha_background
    }

    pub fn set_alpha_background(&mut self, alpha: f32) {
        self.alpha_background = alpha.clamp(0.0, 1.0);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.pixmap = new_pixmap(width, height);
        self.pixmap
            .fill(color(self.background, self.alpha_background));
        self.clip = None;
        self.clip_stack.clear();
    }

    fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width() as i32, self.height() as i32)
    }

    fn clipped(&self, x: i32, y: i32) -> bool {
        self.clip
            .as_ref()
            .is_some_and(|m| m.data()[y as usize * self.width() as usize + x as usize] == 0)
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width() || y >= self.height() {
            return None;
        }
        let c = self.pixmap.pixel(x, y)?.demultiply();
        Some([c.red(), c.green(), c.blue(), c.alpha()])
    }

    pub fn encode_png(&self) -> Option<Vec<u8>> {
        self.pixmap.encode_png().ok()
    }

    pub fn rgba(&self) -> Vec<u8> {
        self.pixmap
            .pixels()
            .iter()
            .flat_map(|p| {
                let c = p.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            })
            .collect()
    }

    pub fn fill_rectangle(&mut self, pixel: u32, rect: Rect, respect_alpha_background: bool) {
        let Some(area) = rect.intersect(self.bounds()).and_then(Rect::to_skia) else {
            return;
        };
        let (alpha, blend_mode) = if respect_alpha_background {
            (self.alpha_background, BlendMode::Source)
        } else {
            (1.0, BlendMode::SourceOver)
        };
        let mut paint = Paint {
            anti_alias: false,
            blend_mode,
            ..Paint::default()
        };
        paint.set_color(color(pixel, alpha));
        self.pixmap
            .fill_rect(area, &paint, Transform::identity(), self.clip.as_ref());
    }

    pub fn clear_area(&mut self, rect: Rect) {
        self.fill_rectangle(self.background, rect, true);
    }

    pub fn copy_bits(&mut self, src: Rect, dst: Rect) {
        if dst.width <= 0 || dst.height <= 0 {
            return;
        }
        let width = self.width() as i32;
        let height = self.height() as i32;
        let mut temp = vec![PremultipliedColorU8::TRANSPARENT; (dst.width * dst.height) as usize];
        let pixels = self.pixmap.pixels();
        for row in 0..dst.height {
            let sy = src.y + row;
            if !(0..height).contains(&sy) {
                continue;
            }
            for col in 0..dst.width {
                let sx = src.x + col;
                if (0..width).contains(&sx) {
                    temp[(row * dst.width + col) as usize] = pixels[(sy * width + sx) as usize];
                }
            }
        }
        let Some(area) = dst.intersect(self.bounds()) else {
            return;
        };
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if self.clipped(x, y) {
                    continue;
                }
                let value = temp[((y - dst.y) * dst.width + (x - dst.x)) as usize];
                self.pixmap.pixels_mut()[(y * width + x) as usize] = value;
            }
        }
    }

    pub fn set_clip_rectangles(&mut self, rects: &[Rect]) {
        let Some(mut mask) = Mask::new(self.width(), self.height()) else {
            return;
        };
        let mut builder = PathBuilder::new();
        for rect in rects {
            if let Some(area) = rect.to_skia() {
                builder.push_rect(area);
            }
        }
        if let Some(path) = builder.finish() {
            mask.fill_path(&path, FillRule::Winding, false, Transform::identity());
        }
        self.clip = Some(mask);
    }

    pub fn reset_clip(&mut self) {
        self.clip = None;
    }

    pub fn save(&mut self) {
        self.clip_stack.push(self.clip.clone());
    }

    pub fn restore(&mut self) {
        if let Some(clip) = self.clip_stack.pop() {
            self.clip = clip;
        }
    }

    pub fn clip_path(&mut self, path: Option<&Path>) {
        let (width, height) = (self.width(), self.height());
        match (&mut self.clip, path) {
            (Some(mask), Some(path)) => {
                mask.intersect_path(path, FillRule::Winding, false, Transform::identity())
            }
            (Some(mask), None) => mask.clear(),
            (None, path) => {
                let Some(mut mask) = Mask::new(width, height) else {
                    return;
                };
                if let Some(path) = path {
                    mask.fill_path(path, FillRule::Winding, false, Transform::identity());
                }
                self.clip = Some(mask);
            }
        }
    }

    pub fn clip_rectangles(&mut self, rects: &[Rect]) {
        let mut builder = PathBuilder::new();
        for rect in rects {
            if let Some(area) = rect.to_skia() {
                builder.push_rect(area);
            }
        }
        let path = builder.finish();
        self.clip_path(path.as_ref());
    }

    fn paint(pixel: u32, anti_alias: bool) -> Paint<'static> {
        let mut paint = Paint {
            anti_alias,
            blend_mode: BlendMode::SourceOver,
            ..Paint::default()
        };
        paint.set_color(color(pixel, 1.0));
        paint
    }

    pub fn stroke_rectangle(&mut self, pixel: u32, rect: Rect, respect_alpha_background: bool) {
        let Some(area) = tiny_skia::Rect::from_xywh(
            rect.x as f32 + 0.5,
            rect.y as f32 + 0.5,
            rect.width.max(0) as f32,
            rect.height.max(0) as f32,
        ) else {
            return;
        };
        let path = PathBuilder::from_rect(area);
        let mut paint = Self::paint(pixel, false);
        if respect_alpha_background {
            paint.set_color(color(pixel, self.alpha_background));
            paint.blend_mode = BlendMode::Source;
        }
        let stroke = Stroke {
            width: 1.0,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            Transform::identity(),
            self.clip.as_ref(),
        );
    }

    pub fn fill_polygon(&mut self, pixel: u32, points: &[(f32, f32)]) {
        let mut builder = PathBuilder::new();
        let Some((&(x0, y0), rest)) = points.split_first() else {
            return;
        };
        builder.move_to(x0, y0);
        for &(x, y) in rest {
            builder.line_to(x, y);
        }
        builder.close();
        if let Some(path) = builder.finish() {
            self.pixmap.fill_path(
                &path,
                &Self::paint(pixel, true),
                FillRule::Winding,
                Transform::identity(),
                self.clip.as_ref(),
            );
        }
    }

    pub fn stroke_polyline(
        &mut self,
        pixel: u32,
        points: &[(f32, f32)],
        width: f32,
        dash: Option<(f32, f32)>,
    ) {
        let mut builder = PathBuilder::new();
        let Some((&(x0, y0), rest)) = points.split_first() else {
            return;
        };
        builder.move_to(x0, y0);
        for &(x, y) in rest {
            builder.line_to(x, y);
        }
        let Some(path) = builder.finish() else {
            return;
        };
        let stroke = Stroke {
            width,
            line_cap: LineCap::Butt,
            dash: dash
                .and_then(|(segment, offset)| StrokeDash::new(vec![segment, segment], offset)),
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            &path,
            &Self::paint(pixel, true),
            &stroke,
            Transform::identity(),
            self.clip.as_ref(),
        );
    }

    pub fn stroke_line(
        &mut self,
        pixel: u32,
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        dash: Option<(f32, f32)>,
    ) {
        self.stroke_polyline(pixel, &[from, to], width, dash);
    }

    fn blit(&mut self, rect: Rect, pixels: Vec<PremultipliedColorU8>) {
        let Some(area) = rect.intersect(self.bounds()) else {
            return;
        };
        let Some(mut image) = Pixmap::new(area.width as u32, area.height as u32) else {
            return;
        };
        let stride = area.width as usize;
        let slots = image.pixels_mut();
        for row in 0..area.height {
            for col in 0..area.width {
                let sx = area.x + col - rect.x;
                let sy = area.y + row - rect.y;
                if let Some(&value) = pixels.get((sy * rect.width + sx) as usize) {
                    slots[row as usize * stride + col as usize] = value;
                }
            }
        }
        self.pixmap.draw_pixmap(
            area.x,
            area.y,
            image.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            self.clip.as_ref(),
        );
    }

    pub fn draw_mask(&mut self, rect: Rect, coverage: &[u8], pixel: u32) {
        let (r, g, b) = rgb(pixel);
        let pixels = coverage
            .iter()
            .map(|&a| premultiplied(r, g, b, a))
            .collect();
        self.blit(rect, pixels);
    }

    pub fn draw_rgba(&mut self, rect: Rect, rgba: &[u8]) {
        let pixels = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[r, g, b, a]| premultiplied(r, g, b, a))
            .collect();
        self.blit(rect, pixels);
    }
}
