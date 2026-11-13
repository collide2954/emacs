use crate::dispextern::{EmacsRectangle, GlyphRowArea, GlyphType, WindowPart};
use crate::dispnew::matrix::WhichMatrix;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::window::WindowBodyUnit;

enum MouseGlyph {
    Edge {
        gx: i64,
        gy: i64,
        width: i64,
        height: i64,
    },
    Virtual {
        width: i64,
        height: i64,
    },
    Image,
}

impl Lisp {
    pub fn remember_mouse_glyph(&mut self, f: Value, gx: i64, gy: i64) -> EmacsRectangle {
        let rect = |x: i64, y: i64, width: i64, height: i64| EmacsRectangle {
            x: x as i32,
            y: y as i32,
            width: width as i32,
            height: height as i32,
        };
        if self.var_true(sym::MOUSE_FINE_GRAINED_TRACKING) {
            return rect(gx, gy, 1, 1);
        }
        let original_gx = gx;
        let glyph = if self.var_true(sym::WINDOW_RESIZE_PIXELWISE) {
            MouseGlyph::Virtual {
                width: 1,
                height: 1,
            }
        } else {
            let mut part = WindowPart::Nothing;
            let window = if self.frame(f).flags.glyphs_initialized_p {
                self.window_from_coordinates(f, gx, gy, &mut part, false)
            } else {
                Value::NIL
            };
            if window.is_nil() {
                let info = self.display_info(self.frame(f).terminal);
                MouseGlyph::Virtual {
                    width: i64::from(info.smallest_char_width),
                    height: i64::from(info.smallest_font_height),
                }
            } else {
                match self.window_mouse_glyph(window, part, gx, gy) {
                    MouseGlyph::Edge {
                        gx,
                        gy,
                        width,
                        height,
                    } => MouseGlyph::Edge {
                        gx: gx + self.window_left_edge_x(window),
                        gy: gy + self.window_top_edge_y(window),
                        width,
                        height,
                    },
                    other => other,
                }
            }
        };
        let (mut gx, gy, mut width, height) = match glyph {
            MouseGlyph::Edge {
                gx,
                gy,
                width,
                height,
            } => (gx, gy, width, height),
            MouseGlyph::Virtual { width, height } => {
                let mut gx = gx;
                let mut gy = gy;
                if gx < 0 {
                    gx -= width - 1;
                }
                if gy < 0 {
                    gy -= height - 1;
                }
                ((gx / width) * width, (gy / height) * height, width, height)
            }
            MouseGlyph::Image => return rect(0, 0, 0, 0),
        };
        if self.var_true(sym::MOUSE_PREFER_CLOSEST_GLYPH) {
            let half_width = width / 2;
            width = half_width;
            let bisection = gx + half_width;
            if original_gx > bisection {
                gx = bisection;
            }
        }
        rect(gx, gy, width, height)
    }

    fn window_mouse_glyph(&mut self, w: Value, part: WindowPart, gx: i64, gy: i64) -> MouseGlyph {
        let mut width = self.window_frame_column_width(w);
        let mut height = self.window_frame_line_height(w);
        let mut x = self.window_relative_x_coord(w, part, gx);
        let mut y = gy - self.window_top_edge_y(w);
        let first = self
            .window_matrix(w, WhichMatrix::Current)
            .expect("glyph matrix")
            .first_text_row();
        let end_row = self.matrix_bottom_text_row(w, WhichMatrix::Current);
        let row_at = |lisp: &Lisp, y: i64| -> Option<usize> {
            let m = lisp
                .window_matrix(w, WhichMatrix::Current)
                .expect("glyph matrix");
            (first..=end_row)
                .map_while(|r| {
                    m.rows
                        .get(r)
                        .filter(|row| row.enabled_p)
                        .map(|row| (r, row))
                })
                .find(|(_, row)| i64::from(row.y + row.height) > y)
                .map(|(r, _)| r)
        };
        let row_y = |lisp: &Lisp, r: usize| -> (i64, i64) {
            let row = &lisp
                .window_matrix(w, WhichMatrix::Current)
                .expect("glyph matrix")
                .rows[r];
            (i64::from(row.y), i64::from(row.height))
        };
        let mut part = part;
        let text = if self.window(w).flags.pseudo_window_p {
            part = WindowPart::ModeLine;
            Some((GlyphRowArea::Text, row_at(self, y)))
        } else {
            match part {
                WindowPart::LeftMargin => Some((GlyphRowArea::LeftMargin, row_at(self, y))),
                WindowPart::RightMargin => Some((GlyphRowArea::RightMargin, row_at(self, y))),
                WindowPart::TabLine | WindowPart::HeaderLine | WindowPart::ModeLine => {
                    let m = self
                        .window_matrix(w, WhichMatrix::Current)
                        .expect("glyph matrix");
                    let r = match part {
                        WindowPart::TabLine => 0,
                        WindowPart::HeaderLine => m.header_line_row(),
                        _ => m.mode_line_row(),
                    };
                    Some((GlyphRowArea::Text, Some(r)))
                }
                WindowPart::Text => Some((GlyphRowArea::Text, row_at(self, y))),
                _ => None,
            }
        };
        let on_line = matches!(
            part,
            WindowPart::ModeLine | WindowPart::HeaderLine | WindowPart::TabLine
        );
        if let Some((area, row)) = text {
            let mut gy = row.map_or(0, |r| row_y(self, r).0);
            let mut gx;
            if let Some(r) = row.filter(|_| gy <= y) {
                let (start, row_height, hit) = {
                    let row = &self
                        .window_matrix(w, WhichMatrix::Current)
                        .expect("glyph matrix")
                        .rows[r];
                    let mut gx = i64::from(row.x);
                    let mut hit = None;
                    for g in row.area_glyphs(area.index()) {
                        if gx + i64::from(g.pixel_width) > x {
                            hit = Some((g.kind, i64::from(g.pixel_width)));
                            break;
                        }
                        gx += i64::from(g.pixel_width);
                    }
                    (gx, i64::from(row.height), hit)
                };
                gx = start;
                height = row_height;
                match hit {
                    Some((GlyphType::Image, _)) => return MouseGlyph::Image,
                    Some((_, pixel_width)) => width = pixel_width,
                    None => {
                        x -= gx;
                        gx += (x / width) * width;
                    }
                }
                if !on_line {
                    gx += self.window_box_left_offset(w, area);
                    height = height.min((self.window_box_height_no_mode_line(w) - gy).max(0));
                }
            } else {
                gx = (x / width) * width;
                y -= gy;
                gy += (y / height) * height;
                if !on_line {
                    height = height.min((self.window_box_height_no_mode_line(w) - gy).max(0));
                }
            }
            return MouseGlyph::Edge {
                gx,
                gy,
                width,
                height,
            };
        }
        let outside = self.window_has_fringes_outside_margins(w);
        let pixel_width = self.window(w).pixel_width;
        let gx = match part {
            WindowPart::LeftFringe => {
                width = self.window_left_fringe_width(w);
                if outside {
                    self.window_left_scroll_bar_area_width(w)
                } else {
                    self.window_box_right_offset(w, GlyphRowArea::LeftMargin)
                }
            }
            WindowPart::RightFringe => {
                let gx = if outside {
                    self.window_box_right_offset(w, GlyphRowArea::RightMargin)
                } else {
                    self.window_box_right_offset(w, GlyphRowArea::Text)
                };
                width = if self.window_right_divider_width(w) == 0
                    && !self.window_has_vertical_scroll_bar(w)
                    && !self.window_rightmost_p(w)
                {
                    if gx < pixel_width - width {
                        self.window_right_fringe_width(w) - width
                    } else {
                        pixel_width - gx
                    }
                } else {
                    self.window_right_fringe_width(w)
                };
                gx
            }
            WindowPart::VerticalBorder => pixel_width - width,
            WindowPart::VerticalScrollBar => {
                width = self.window_scroll_bar_area_width(w);
                if self.window_has_vertical_scroll_bar_on_left(w) {
                    0
                } else {
                    self.window_box_right_offset(w, GlyphRowArea::RightMargin)
                        + if outside {
                            self.window_right_fringe_width(w)
                        } else {
                            0
                        }
                }
            }
            WindowPart::RightDivider => {
                let divider = self.window_right_divider_width(w);
                return MouseGlyph::Edge {
                    gx: pixel_width - divider,
                    gy: 0,
                    width: divider,
                    height: self.window(w).pixel_height - self.window_bottom_divider_width(w),
                };
            }
            WindowPart::BottomDivider => {
                let divider = self.window_bottom_divider_width(w);
                return MouseGlyph::Edge {
                    gx: 0,
                    gy: self.window(w).pixel_height - divider,
                    width: pixel_width,
                    height: divider,
                };
            }
            _ => return MouseGlyph::Virtual { width, height },
        };
        let row = row_at(self, y);
        let mut gy = row.map_or(0, |r| row_y(self, r).0);
        if let Some(r) = row.filter(|_| gy <= y) {
            height = row_y(self, r).1;
        } else {
            y -= gy;
            gy += (y / height) * height;
        }
        MouseGlyph::Edge {
            gx,
            gy,
            width,
            height,
        }
    }

    pub fn frame_pixel_x_to_col(&self, f: Value, x: i64) -> i64 {
        let g = &self.frame(f).geometry;
        (x - g.internal_border_width) / g.column_width
    }

    pub fn frame_pixel_y_to_line(&self, f: Value, y: i64) -> i64 {
        let top_margin = self.frame_top_margin_height(f);
        let g = &self.frame(f).geometry;
        let border = g.internal_border_width;
        (if y < top_margin {
            y
        } else if y < top_margin + border {
            y - (top_margin + border + g.line_height - 1)
        } else {
            y - border
        }) / g.line_height
    }

    pub fn pixel_to_glyph_coords(
        &self,
        f: Value,
        pix_x: i64,
        pix_y: i64,
        noclip: bool,
    ) -> (i64, i64) {
        let (mut pix_x, mut pix_y) = (pix_x, pix_y);
        if self.frame(f).window_p() {
            let g = &self.frame(f).geometry;
            if pix_x < 0 {
                pix_x -= g.column_width - 1;
            }
            if pix_y < 0 {
                pix_y -= g.line_height - 1;
            }
            pix_x = self.frame_pixel_x_to_col(f, pix_x);
            pix_y = self.frame_pixel_y_to_line(f, pix_y);
            if !noclip {
                let g = &self.frame(f).geometry;
                pix_x = pix_x.clamp(0, g.total_cols.max(0));
                pix_y = pix_y.clamp(0, g.total_lines.max(0));
            }
        }
        (pix_x, pix_y)
    }

    pub fn window_text_bottom_y(&mut self, w: Value) -> i64 {
        let mut height = self.window(w).pixel_height;
        height -= self.window_bottom_divider_width(w);
        if self.window_wants_mode_line(w) {
            height -= self.current_mode_line_height(w);
        }
        height -= self.window_scroll_bar_area_height(w);
        height
    }

    pub fn window_box_width(&self, w: Value, area: GlyphRowArea) -> i64 {
        let mut width = self.window(w).pixel_width;
        if !self.window(w).flags.pseudo_window_p {
            width -= self.window_scroll_bar_area_width(w);
            width -= self.window_right_divider_width(w);
            match area {
                GlyphRowArea::Text => {
                    width -= self.window_margins_width(w) + self.window_fringes_width(w);
                }
                GlyphRowArea::LeftMargin => width = self.window_left_margin_width(w),
                GlyphRowArea::RightMargin => width = self.window_right_margin_width(w),
                GlyphRowArea::Any => {}
            }
        }
        width.max(0)
    }

    pub fn window_box_left_offset(&self, w: Value, area: GlyphRowArea) -> i64 {
        if self.window(w).flags.pseudo_window_p {
            return 0;
        }
        let mut x = self.window_left_scroll_bar_area_width(w);
        let outside = self.window_has_fringes_outside_margins(w);
        match area {
            GlyphRowArea::Text => {
                x += self.window_left_fringe_width(w)
                    + self.window_box_width(w, GlyphRowArea::LeftMargin);
            }
            GlyphRowArea::RightMargin => {
                x += self.window_left_fringe_width(w)
                    + self.window_box_width(w, GlyphRowArea::LeftMargin)
                    + self.window_box_width(w, GlyphRowArea::Text)
                    + if outside {
                        0
                    } else {
                        self.window_right_fringe_width(w)
                    };
            }
            GlyphRowArea::LeftMargin => {
                if outside {
                    x += self.window_left_fringe_width(w);
                }
            }
            GlyphRowArea::Any => {}
        }
        x.min(self.window(w).pixel_width)
    }

    pub fn window_box_right_offset(&self, w: Value, area: GlyphRowArea) -> i64 {
        (self.window_box_left_offset(w, area) + self.window_box_width(w, area))
            .min(self.window(w).pixel_width)
    }

    pub fn window_hscroll_limited(&self, w: Value) -> i64 {
        let mut window_hscroll = self.window(w).hscroll;
        let window_text_width = self.window_box_width(w, GlyphRowArea::Text);
        let colwidth = self.frame(self.window(w).frame).geometry.column_width;
        let limit = (i64::from(i32::MAX) - window_text_width) / colwidth - 1;
        if window_hscroll > limit {
            window_hscroll = limit;
        }
        window_hscroll
    }

    pub fn hscrolling_current_line_p(&self, w: Value) -> bool {
        let win = self.window(w);
        !win.flags.suspend_auto_hscroll
            && self.buffer_local_value(sym::AUTO_HSCROLL_MODE, win.contents) == sym::CURRENT_LINE
    }

    fn get_narrowed_width(&mut self, w: Value) -> LispResult<i64> {
        let f = self.window(w).frame;
        let fact = if self.frame_window_p(f) { 3 } else { 2 };
        let width = self.window_body_width(w, WindowBodyUnit::CanonicalChars)?
            - if self.window_right_fringe_width(w) == 0 || self.window_left_fringe_width(w) == 0 {
                1
            } else {
                0
            };
        Ok(fact * width.max(1))
    }

    fn get_narrowed_len(&mut self, w: Value) -> LispResult<i64> {
        let height = self.window_body_height(w, WindowBodyUnit::CanonicalChars)?;
        Ok(self.get_narrowed_width(w)? * height.max(1))
    }

    pub fn get_medium_narrowing_begv(&mut self, w: Value, pos: i64) -> LispResult<i64> {
        let len = self.get_narrowed_len(w)?;
        Ok(((pos / len - 1) * len).max(self.cur().begv as i64))
    }

    pub fn get_medium_narrowing_zv(&mut self, w: Value, pos: i64) -> LispResult<i64> {
        let len = self.get_narrowed_len(w)?;
        Ok(((pos / len + 1) * len).min(self.cur().zv as i64))
    }

    fn get_nearby_bol_pos(&self, pos: i64) -> i64 {
        let begv = self.cur().begv as i64;
        let mut pos = pos;
        let mut bol = begv - 1;
        let mut dist = 500;
        while dist <= 500000 {
            let start = (pos - dist).max(begv);
            let mut cur = start;
            while cur < pos {
                let (next, count) = self.find_newline(cur as usize, pos as usize, 1);
                if count >= 1 {
                    bol = next as i64;
                    cur = next as i64;
                } else {
                    break;
                }
            }
            if bol >= begv || start == begv {
                break;
            }
            pos = (pos - dist).max(begv);
            dist *= 10;
        }
        bol
    }

    pub fn get_small_narrowing_begv(&mut self, w: Value, pos: i64) -> LispResult<i64> {
        let len = self.get_narrowed_width(w)?;
        let begv = self.cur().begv as i64;
        let bol_pos = self.get_nearby_bol_pos(pos).max(begv);
        Ok((bol_pos + ((pos - bol_pos) / len - 1) * len).max(begv))
    }
}
