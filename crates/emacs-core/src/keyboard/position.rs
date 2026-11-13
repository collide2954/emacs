use crate::dispextern::{GlyphRowArea, WindowPart};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("posn-at-x-y", 2, 4, f_posn_at_x_y),
    Subr::fixed("posn-at-point", 0, 2, f_posn_at_point),
    Subr::fixed("posn-point", 1, 1, f_posn_point),
];

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn f_posn_point(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let position = args[0];
    let posn = crate::fns::f_nth(lisp, &[Value::fixnum(5), position])?;
    if !posn.is_nil() {
        return Ok(posn);
    }
    let posn = lisp.xcar(lisp.xcdr(position));
    if posn.is_cons() {
        return Ok(lisp.xcar(posn));
    }
    if posn.is_fixnum() {
        return Ok(posn);
    }
    Ok(Value::NIL)
}

impl Lisp {
    pub fn make_lispy_position(&mut self, f: Option<Value>, x: i64, y: i64, t: i64) -> LispResult {
        self.with_gc_deferred(|lisp| lisp.make_lispy_position_1(f, x, y, t))
    }

    fn make_lispy_position_1(&mut self, f: Option<Value>, mx: i64, my: i64, t: i64) -> LispResult {
        let mut part = WindowPart::Nothing;
        let mut posn = Value::NIL;
        let mut extra_info = Value::NIL;
        let (mut xret, mut yret) = (0, 0);
        let mut window_or_frame = match f {
            Some(f) => self.window_from_coordinates(f, mx, my, &mut part, true),
            None => Value::NIL,
        };
        let track_mouse = self.find_symbol_value(sym::TRACK_MOUSE);
        if let Some(f) = f {
            let tab_bar_window = self.frame(f).tab_bar_window;
            if self.is_window(tab_bar_window) && window_or_frame == tab_bar_window {
                if track_mouse.is_nil() || track_mouse == sym::T {
                    posn = sym::TAB_BAR;
                }
                window_or_frame = Value::NIL;
            }
            let g = &self.frame(f).geometry;
            let (menu_bar_lines, tab_bar_lines) = (g.menu_bar_lines, g.tab_bar_lines);
            if !self.frame_window_p(f)
                && tab_bar_lines > 0
                && my >= menu_bar_lines
                && my < menu_bar_lines + tab_bar_lines
            {
                posn = sym::TAB_BAR;
                window_or_frame = Value::NIL;
            }
        }
        if self.is_window(window_or_frame) {
            let w = window_or_frame;
            let mut string_info = Value::NIL;
            let mut textpos: i64 = 0;
            let (mut col, mut row) = (-1, -1);
            let (mut dx, mut dy) = (-1, -1);
            let (mut width, mut height) = (-1, -1);
            let mut object = Value::NIL;
            let wx = mx - self.window_left_edge_x(w);
            let wy = my - self.window_top_edge_y(w);
            let top_lines = |lisp: &mut Lisp| {
                lisp.window_tab_line_height(w) + lisp.window_header_line_height(w)
            };
            match part {
                WindowPart::Text => {
                    xret = mx - self.window_box_left(w, GlyphRowArea::Text);
                    yret = wy - top_lines(self);
                }
                WindowPart::ModeLine | WindowPart::TabLine | WindowPart::HeaderLine => {
                    posn = match part {
                        WindowPart::ModeLine => sym::MODE_LINE,
                        WindowPart::TabLine => sym::TAB_LINE,
                        _ => sym::HEADER_LINE,
                    };
                    let hit = self.mode_line_string(w, part, wx, wy);
                    (col, row, dx, dy, width, height) =
                        (hit.x, hit.y, hit.dx, hit.dy, hit.width, hit.height);
                    if let Some(o) = hit.object {
                        object = o;
                    }
                    if hit.string.is_string() {
                        string_info = self.cons(hit.string, Value::fixnum(hit.charpos));
                    }
                    textpos = -1;
                    xret = wx;
                    yret = wy;
                }
                WindowPart::LeftMargin | WindowPart::RightMargin => {
                    posn = if part == WindowPart::LeftMargin {
                        sym::LEFT_MARGIN
                    } else {
                        sym::RIGHT_MARGIN
                    };
                    let col0 = wx - self.window_left_scroll_bar_area_width(w);
                    let hit = self.marginal_area_string(w, part, col0, wy);
                    (col, row, dx, dy, width, height) =
                        (hit.x, hit.y, hit.dx, hit.dy, hit.width, hit.height);
                    if let Some(o) = hit.object {
                        object = o;
                    }
                    if hit.string.is_string() {
                        string_info = self.cons(hit.string, Value::fixnum(hit.charpos));
                    }
                    xret = wx;
                    yret = wy - top_lines(self);
                }
                WindowPart::LeftFringe => {
                    posn = sym::LEFT_FRINGE;
                    col = 0;
                    xret = wx;
                    dx = wx
                        - if self.window_has_fringes_outside_margins(w) {
                            0
                        } else {
                            self.window_box_width(w, GlyphRowArea::LeftMargin)
                        };
                    yret = wy - top_lines(self);
                    dy = yret;
                }
                WindowPart::RightFringe => {
                    posn = sym::RIGHT_FRINGE;
                    col = 0;
                    xret = wx;
                    dx = wx
                        - self.window_box_width(w, GlyphRowArea::LeftMargin)
                        - self.window_box_width(w, GlyphRowArea::Text)
                        - if self.window_has_fringes_outside_margins(w) {
                            self.window_box_width(w, GlyphRowArea::RightMargin)
                        } else {
                            0
                        };
                    yret = wy - top_lines(self);
                    dy = yret;
                }
                WindowPart::VerticalBorder => {
                    posn = sym::VERTICAL_LINE;
                    width = 1;
                    dx = 0;
                    xret = wx;
                    yret = wy;
                    dy = wy;
                }
                WindowPart::VerticalScrollBar => {
                    posn = sym::VERTICAL_SCROLL_BAR;
                    width = self.window_scroll_bar_area_width(w);
                    (dx, xret, dy, yret) = (wx, wx, wy, wy);
                }
                WindowPart::HorizontalScrollBar => {
                    posn = sym::HORIZONTAL_SCROLL_BAR;
                    width = self.window_scroll_bar_area_height(w);
                    (dx, xret, dy, yret) = (wx, wx, wy, wy);
                }
                WindowPart::RightDivider => {
                    posn = sym::RIGHT_DIVIDER;
                    width = self.window_right_divider_width(w);
                    (dx, xret, dy, yret) = (wx, wx, wy, wy);
                }
                WindowPart::BottomDivider => {
                    posn = sym::BOTTOM_DIVIDER;
                    width = self.window_bottom_divider_width(w);
                    (dx, xret, dy, yret) = (wx, wx, wy, wy);
                }
                WindowPart::Nothing => {}
            }
            if textpos == 0 {
                let x2 = match part {
                    WindowPart::Text => xret,
                    WindowPart::RightFringe | WindowPart::RightMargin => {
                        mx - self.window_box_left(w, GlyphRowArea::Text)
                    }
                    WindowPart::VerticalScrollBar
                        if self.window_has_vertical_scroll_bar_on_right(w) =>
                    {
                        mx - self.window_box_left(w, GlyphRowArea::Text)
                    }
                    _ => 0,
                };
                let found = self.buffer_posn_from_coords(w, x2, wy)?;
                textpos = found.pos.pos.charpos;
                if col < 0 {
                    col = found.x;
                }
                if row < 0 {
                    row = found.y;
                }
                if dx < 0 {
                    dx = found.dx;
                }
                if dy < 0 {
                    dy = found.dy;
                }
                if width < 0 {
                    width = found.width;
                }
                if height < 0 {
                    height = found.height;
                }
                if posn.is_nil() {
                    posn = Value::fixnum(textpos);
                    if found.string.is_string() {
                        string_info =
                            self.cons(found.string, Value::fixnum(found.pos.string_pos.charpos));
                    }
                }
                if object.is_nil() {
                    object = found.object.unwrap_or(Value::NIL);
                }
            }
            if object.is_cons() && self.xcar(object) == sym::IMAGE {
                unreachable!("image hot spots need image.c's find_hot_spot");
            }
            let d = self.cons(Value::fixnum(dx), Value::fixnum(dy));
            let size = self.cons(Value::fixnum(width), Value::fixnum(height));
            extra_info = self.list(&[object, d, size]);
            let col_row = self.cons(Value::fixnum(col), Value::fixnum(row));
            extra_info = self.cons(col_row, extra_info);
            let pos = if textpos < 0 {
                Value::NIL
            } else {
                Value::fixnum(textpos)
            };
            extra_info = self.cons(pos, extra_info);
            extra_info = self.cons(string_info, extra_info);
        } else if let Some(f) = f {
            window_or_frame = f;
            xret = mx;
            yret = my;
            if self.frame_live_p(f)
                && posn.is_nil()
                && self.frame(f).geometry.internal_border_width > 0
                && !self.get_frame_param(f, sym::DRAG_INTERNAL_BORDER).is_nil()
            {
                posn = self.frame_internal_border_part(f, xret, yret);
            }
        } else {
            if track_mouse == sym::DRAG_SOURCE {
                xret = mx;
                yret = my;
            }
            window_or_frame = Value::NIL;
        }
        let rest = self.cons(Value::fixnum(t), extra_info);
        let xy = self.cons(Value::fixnum(xret), Value::fixnum(yret));
        let rest = self.cons(xy, rest);
        let rest = self.cons(posn, rest);
        Ok(self.cons(window_or_frame, rest))
    }
}

fn check_coordinate(lisp: &mut Lisp, v: Value) -> LispResult<i64> {
    let n = lisp.check_fixnum(v)?;
    if n != -1 && n < 0 {
        return Err(lisp.wrong_type(sym::WHOLENUMP, v));
    }
    Ok(n)
}

pub(crate) fn f_posn_at_x_y(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (x, y, frame_or_window, whole) = (args[0], args[1], arg(args, 2), arg(args, 3));
    let mut xi = check_coordinate(lisp, x)?;
    let mut yi = check_coordinate(lisp, y)?;
    let mut frame_or_window = if frame_or_window.is_nil() {
        lisp.windows.selected_window
    } else {
        frame_or_window
    };
    if lisp.is_window(frame_or_window) {
        let w = lisp.decode_live_window(frame_or_window)?;
        xi += lisp.window_left_edge_x(w)
            + if whole.is_nil() {
                lisp.window_box_left_offset(w, GlyphRowArea::Text)
            } else {
                0
            };
        yi = lisp.window_to_frame_pixel_y(w, yi);
        frame_or_window = lisp.window(w).frame;
    }
    let f = lisp.check_live_frame(frame_or_window)?;
    lisp.make_lispy_position(Some(f), xi, yi, 0)
}

fn f_posn_at_point(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (pos, window) = (arg(args, 0), arg(args, 1));
    let window = if window.is_nil() {
        lisp.windows.selected_window
    } else {
        window
    };
    let tem = crate::window::f_pos_visible_in_window_p(lisp, &[pos, window, sym::T])?;
    if tem.is_nil() {
        return Ok(tem);
    }
    let x = lisp.xcar(tem);
    let mut y = lisp.xcar(lisp.xcdr(tem));
    let aux_info = lisp.xcdr(lisp.xcdr(tem));
    let y_coord = y.as_fixnum().unwrap_or(0);
    if x.as_fixnum().unwrap_or(0) < -1 {
        return Ok(Value::NIL);
    }
    if !aux_info.is_nil() && y_coord < 0 {
        let rtop = lisp.xcar(aux_info).as_fixnum().unwrap_or(0);
        y = Value::fixnum(y_coord + rtop);
    }
    f_posn_at_x_y(lisp, &[x, y, window, Value::NIL])
}
