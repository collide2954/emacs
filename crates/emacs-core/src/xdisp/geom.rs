use crate::dispextern::GlyphRowArea;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::window::WindowBodyUnit;

impl Lisp {
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
