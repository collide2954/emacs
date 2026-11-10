use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;
use crate::window::WindowBodyUnit;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("set-window-new-pixel", 2, 3, f_set_window_new_pixel),
    Subr::fixed("set-window-new-total", 2, 3, f_set_window_new_total),
    Subr::fixed("set-window-new-normal", 1, 2, f_set_window_new_normal),
    Subr::fixed("window-resize-apply", 0, 2, f_window_resize_apply),
    Subr::fixed(
        "window-resize-apply-total",
        0,
        2,
        f_window_resize_apply_total,
    ),
    Subr::fixed(
        "resize-mini-window-internal",
        1,
        1,
        f_resize_mini_window_internal,
    ),
];

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn xfixnum(v: Value) -> i64 {
    v.as_fixnum().unwrap_or(0)
}

impl Lisp {
    fn window_children(&self, w: Value) -> Vec<Value> {
        let mut out = Vec::new();
        let mut c = self.window(w).contents;
        while !c.is_nil() {
            out.push(c);
            c = self.window(c).next;
        }
        out
    }

    pub(crate) fn window_resize_check(&self, w: Value, horflag: bool) -> bool {
        let win = self.window(w);
        let new_pixel = xfixnum(win.new_pixel);
        if self.is_window(win.contents) {
            let children = self.window_children(w);
            if win.horizontal != horflag {
                children.into_iter().all(|c| {
                    xfixnum(self.window(c).new_pixel) == new_pixel
                        && self.window_resize_check(c, horflag)
                })
            } else {
                let mut remaining_pixels = new_pixel;
                for c in children {
                    if !self.window_resize_check(c, horflag) {
                        return false;
                    }
                    remaining_pixels -= xfixnum(self.window(c).new_pixel);
                    if remaining_pixels < 0 {
                        return false;
                    }
                }
                remaining_pixels == 0
            }
        } else {
            let geometry = &self.frame(win.frame).geometry;
            new_pixel
                >= if horflag {
                    2 * geometry.column_width
                } else {
                    geometry.line_height
                }
        }
    }

    pub(crate) fn window_resize_apply(&mut self, w: Value, horflag: bool) {
        let f = self.window(w).frame;
        let unit = if horflag {
            self.frame(f).geometry.column_width
        } else {
            self.frame(f).geometry.line_height
        };
        let new_normal = self.window(w).new_normal;
        let number = self.is_number(new_normal);
        let win = self.window_mut(w);
        let mut edge = if horflag {
            win.pixel_width = xfixnum(win.new_pixel);
            win.total_cols = win.pixel_width / unit;
            if number {
                win.normal_cols = new_normal;
            }
            win.pixel_left
        } else {
            win.pixel_height = xfixnum(win.new_pixel);
            win.total_lines = win.pixel_height / unit;
            if number {
                win.normal_lines = new_normal;
            }
            win.pixel_top
        };
        let (contents, horizontal) = (win.contents, win.horizontal);
        if self.is_window(contents) {
            for c in self.window_children(w) {
                let child = self.window_mut(c);
                if horflag {
                    child.pixel_left = edge;
                    child.left_col = edge / unit;
                } else {
                    child.pixel_top = edge;
                    child.top_line = edge / unit;
                }
                self.window_resize_apply(c, horflag);
                if horizontal == horflag {
                    let child = self.window(c);
                    edge += if horflag {
                        child.pixel_width
                    } else {
                        child.pixel_height
                    };
                }
            }
        } else {
            self.window_mut(w).flags.window_end_valid = false;
        }
        if !self.window(w).flags.pseudo_window_p {
            self.frame_mut(f).flags.window_change = true;
        }
    }

    fn window_resize_apply_total(&mut self, w: Value, horflag: bool) {
        let win = self.window_mut(w);
        let mut edge = if horflag {
            win.total_cols = xfixnum(win.new_total);
            win.left_col
        } else {
            win.total_lines = xfixnum(win.new_total);
            win.top_line
        };
        let (contents, horizontal) = (win.contents, win.horizontal);
        if self.is_window(contents) {
            for c in self.window_children(w) {
                if horflag {
                    self.window_mut(c).left_col = edge;
                } else {
                    self.window_mut(c).top_line = edge;
                }
                self.window_resize_apply_total(c, horflag);
                if horizontal == horflag {
                    let child = self.window(c);
                    edge += if horflag {
                        child.total_cols
                    } else {
                        child.total_lines
                    };
                }
            }
        }
    }

    fn resize_mini_window_apply(&mut self, w: Value, delta: i64) {
        let f = self.window(w).frame;
        let r = self.frame(f).root_window;
        let line_height = self.frame(f).geometry.line_height;
        let win = self.window_mut(w);
        win.pixel_height += delta;
        win.total_lines = win.pixel_height / line_height;
        self.window_resize_apply(r, false);
        let root = self.window(r);
        let (pixel_top, top_line) = (
            root.pixel_top + root.pixel_height,
            root.top_line + root.total_lines,
        );
        let win = self.window_mut(w);
        win.pixel_top = pixel_top;
        win.top_line = top_line;
        if self.frame(f).flags.redisplay {
            self.set_frame_garbaged(f);
        }
        self.fset_redisplay(f);
        self.adjust_frame_glyphs(f);
    }

    pub(crate) fn grow_mini_window(&mut self, w: Value, delta: i32, unit: i32) -> LispResult<()> {
        let f = self.window(w).frame;
        let old_height = self.window_body_height(w, WindowBodyUnit::Pixels)? as i32;
        let min_height = unit;
        let mut delta = delta;
        if old_height + delta < min_height {
            delta = if old_height > min_height {
                min_height - old_height
            } else {
                0
            };
        }
        if delta != 0 {
            let root = self.frame(f).root_window;
            let grow = self.funcall(
                sym::WINDOW__RESIZE_ROOT_WINDOW_VERTICALLY,
                &[root, Value::fixnum(-i64::from(delta)), sym::T],
            )?;
            if let Some(grow) = grow.as_fixnum()
                && grow != 0
                && self.window_resize_check(root, false)
            {
                self.resize_mini_window_apply(w, -grow);
            }
        }
        let body = self.window_body_height(w, WindowBodyUnit::Pixels)?;
        self.frame_mut(f).flags.frozen_window_starts = body > i64::from(unit);
        Ok(())
    }

    pub(crate) fn shrink_mini_window(&mut self, w: Value, unit: i32) -> LispResult<()> {
        let f = self.window(w).frame;
        let delta = self.window_body_height(w, WindowBodyUnit::Pixels)? as i32 - unit;
        if delta > 0 {
            let root = self.frame(f).root_window;
            let grow = self.funcall(
                sym::WINDOW__RESIZE_ROOT_WINDOW_VERTICALLY,
                &[root, Value::fixnum(i64::from(delta)), sym::T],
            )?;
            if let Some(grow) = grow.as_fixnum()
                && self.window_resize_check(root, false)
            {
                self.resize_mini_window_apply(w, -grow);
            }
        } else if delta < 0 {
            self.grow_mini_window(w, -delta, unit)?;
        }
        let body = self.window_body_height(w, WindowBodyUnit::Pixels)?;
        self.frame_mut(f).flags.frozen_window_starts = body > i64::from(unit);
        Ok(())
    }
}

fn f_set_window_new_pixel(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    let (size, add) = (arg(args, 1), arg(args, 2));
    let size_min = if add.is_nil() {
        0
    } else {
        -xfixnum(lisp.window(w).new_pixel)
    };
    let size_max = size_min + i64::from(i32::MAX);
    let checked_size = lisp.check_integer_range(size, size_min, size_max)?;
    let new_pixel = if add.is_nil() {
        size
    } else {
        Value::fixnum(xfixnum(lisp.window(w).new_pixel) + checked_size)
    };
    lisp.window_mut(w).new_pixel = new_pixel;
    Ok(new_pixel)
}

fn f_set_window_new_total(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    let (size, add) = (arg(args, 1), arg(args, 2));
    let n = lisp.check_fixnum(size)?;
    let new_total = if add.is_nil() {
        size
    } else {
        Value::fixnum(xfixnum(lisp.window(w).new_total) + n)
    };
    lisp.window_mut(w).new_total = new_total;
    Ok(new_total)
}

fn f_set_window_new_normal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    let size = arg(args, 1);
    lisp.window_mut(w).new_normal = size;
    Ok(size)
}

fn f_window_resize_apply(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    let r = lisp.frame(f).root_window;
    let horflag = !arg(args, 1).is_nil();
    let size = if horflag {
        lisp.window(r).pixel_width
    } else {
        lisp.window(r).pixel_height
    };
    if !lisp.window_resize_check(r, horflag) || xfixnum(lisp.window(r).new_pixel) != size {
        return Ok(Value::NIL);
    }
    lisp.window_resize_apply(r, horflag);
    lisp.fset_redisplay(f);
    lisp.adjust_frame_glyphs(f);
    Ok(sym::T)
}

fn f_window_resize_apply_total(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    let horizontal = arg(args, 1);
    let r = lisp.frame(f).root_window;
    let top_margin = lisp.frame_top_margin(f);
    let root = lisp.window_mut(r);
    root.left_col = 0;
    root.top_line = top_margin;
    lisp.window_resize_apply_total(r, !horizontal.is_nil());
    if lisp.frame_has_minibuf_p(f) && !lisp.frame_minibuf_only_p(f) {
        let m = lisp.frame(f).minibuffer_window;
        let root = lisp.window(r);
        let top_line = root.top_line + root.total_lines;
        let mini = lisp.window_mut(m);
        if horizontal.is_nil() {
            mini.top_line = top_line;
            mini.total_lines = xfixnum(mini.new_total);
        } else {
            mini.total_cols = xfixnum(mini.new_total);
        }
    }
    Ok(sym::T)
}

fn f_resize_mini_window_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let window = args[0];
    let w = lisp.check_live_window(window)?;
    let f = lisp.window(w).frame;
    if lisp.frame(f).minibuffer_window != window {
        return Err(lisp.error("Not a valid minibuffer window"));
    } else if lisp.frame_minibuf_only_p(f) {
        return Err(lisp.error("Cannot resize a minibuffer-only frame"));
    }
    let r = lisp.frame(f).root_window;
    let old_height = lisp.window(r).pixel_height + lisp.window(w).pixel_height;
    let new_pixel = xfixnum(lisp.window(w).new_pixel);
    let delta = new_pixel - lisp.window(w).pixel_height;
    if lisp.window_resize_check(r, false)
        && new_pixel > 0
        && old_height == xfixnum(lisp.window(r).new_pixel) + new_pixel
    {
        lisp.resize_mini_window_apply(w, delta);
        Ok(sym::T)
    } else {
        Err(lisp.error("Cannot resize mini window"))
    }
}
