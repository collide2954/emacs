use crate::dispextern::TextCursorKind;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::terminal::OutputMethod;
use crate::value::Value;
use crate::window::CursorPos;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("split-window-internal", 4, 5, f_split_window_internal),
    Subr::fixed("delete-window-internal", 1, 1, f_delete_window_internal),
    Subr::fixed(
        "delete-other-windows-internal",
        0,
        2,
        f_delete_other_windows_internal,
    )
    .interactive(""),
    Subr::fixed(
        "run-window-scroll-functions",
        0,
        1,
        f_run_window_scroll_functions,
    ),
];

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn xfixnum(v: Value) -> i64 {
    v.as_fixnum().unwrap_or(0)
}

impl Lisp {
    fn window_internal_p(&self, w: Value) -> bool {
        self.is_window(self.window(w).contents)
    }

    fn window_horizontal_combination_p(&self, w: Value) -> bool {
        self.window_internal_p(w) && self.window(w).horizontal
    }

    fn window_vertical_combination_p(&self, w: Value) -> bool {
        self.window_internal_p(w) && !self.window(w).horizontal
    }

    fn frame_initial_p(&self, f: Value) -> bool {
        self.frame(f).output_method == OutputMethod::Initial
    }

    pub(crate) fn replace_window(&mut self, old: Value, new: Value, setflag: bool) {
        let frame = self.window(old).frame;
        if old == self.frame(frame).root_window {
            self.frame_mut(frame).root_window = new;
        }
        if setflag {
            let o = self.window(old).clone_geometry();
            let one_cols = self.float(1.0);
            let one_lines = self.float(1.0);
            let n = self.window_mut(new);
            n.pixel_left = o.pixel_left;
            n.pixel_top = o.pixel_top;
            n.pixel_width = o.pixel_width;
            n.pixel_height = o.pixel_height;
            n.left_col = o.left_col;
            n.top_line = o.top_line;
            n.total_cols = o.total_cols;
            n.total_lines = o.total_lines;
            n.normal_cols = o.normal_cols;
            n.normal_lines = o.normal_lines;
            n.desired_matrix = None;
            n.current_matrix = None;
            n.vscroll = 0;
            n.cursor = CursorPos::default();
            n.phys_cursor = CursorPos::default();
            n.last_cursor_vpos = 0;
            n.phys_cursor_type = TextCursorKind::NoCursor;
            n.phys_cursor_width = -1;
            n.flags.must_be_updated_p = false;
            n.flags.pseudo_window_p = false;
            n.window_end_vpos = 0;
            n.window_end_pos = 0;
            n.flags.window_end_valid = false;
            let o = self.window_mut(old);
            o.normal_cols = one_cols;
            o.normal_lines = one_lines;
        }
        let tem = self.window(old).next;
        self.window_mut(new).next = tem;
        if !tem.is_nil() {
            self.window_mut(tem).prev = new;
        }
        let tem = self.window(old).prev;
        self.window_mut(new).prev = tem;
        if !tem.is_nil() {
            self.window_mut(tem).next = new;
        }
        let tem = self.window(old).parent;
        self.window_mut(new).parent = tem;
        if !tem.is_nil() && self.window(tem).contents == old {
            let horizontal = self.window(tem).horizontal;
            self.wset_combination(tem, horizontal, new);
        }
    }

    fn recombine_windows(&mut self, window: Value) {
        let parent = self.window(window).parent;
        if parent.is_nil() || !self.window(window).combination_limit.is_nil() {
            return;
        }
        if !(self.window_internal_p(parent)
            && self.window_internal_p(window)
            && self.window(parent).horizontal == self.window(window).horizontal)
        {
            return;
        }
        let horflag = self.window_horizontal_combination_p(window);
        let mut child = self.window(window).contents;
        let prev = self.window(window).prev;
        if prev.is_nil() {
            self.wset_combination(parent, horflag, child);
        } else {
            self.window_mut(child).prev = prev;
            self.window_mut(prev).next = child;
        }
        let (parent_width, parent_height) = {
            let p = self.window(parent);
            (p.pixel_width, p.pixel_height)
        };
        loop {
            self.window_mut(child).parent = parent;
            let normal = if horflag {
                self.window(child).pixel_width as f64 / parent_width as f64
            } else {
                self.window(child).pixel_height as f64 / parent_height as f64
            };
            let normal = self.float(normal);
            if horflag {
                self.window_mut(child).normal_cols = normal;
            } else {
                self.window_mut(child).normal_lines = normal;
            }
            let next = self.window(child).next;
            if next.is_nil() {
                let wnext = self.window(window).next;
                if !wnext.is_nil() {
                    self.window_mut(child).next = wnext;
                    self.window_mut(wnext).prev = child;
                }
                break;
            }
            child = next;
        }
        self.wset_combination(window, false, Value::NIL);
    }

    pub(crate) fn resize_root_window(
        &mut self,
        window: Value,
        delta: i64,
        horizontal: Value,
        ignore: Value,
        pixelwise: Value,
    ) -> LispResult {
        self.funcall(
            sym::WINDOW__RESIZE_ROOT_WINDOW,
            &[window, Value::fixnum(delta), horizontal, ignore, pixelwise],
        )
    }

    pub(crate) fn window_pixel_to_total(&mut self, frame: Value, horizontal: Value) -> LispResult {
        self.funcall(sym::WINDOW__PIXEL_TO_TOTAL, &[frame, horizontal])
    }
}

struct Geometry {
    pixel_left: i64,
    pixel_top: i64,
    pixel_width: i64,
    pixel_height: i64,
    left_col: i64,
    top_line: i64,
    total_cols: i64,
    total_lines: i64,
    normal_cols: Value,
    normal_lines: Value,
}

impl crate::window::Window {
    fn clone_geometry(&self) -> Geometry {
        Geometry {
            pixel_left: self.pixel_left,
            pixel_top: self.pixel_top,
            pixel_width: self.pixel_width,
            pixel_height: self.pixel_height,
            left_col: self.left_col,
            top_line: self.top_line,
            total_cols: self.total_cols,
            total_lines: self.total_lines,
            normal_cols: self.normal_cols,
            normal_lines: self.normal_lines,
        }
    }
}

fn f_run_window_scroll_functions(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let window = arg(args, 0);
    let w = lisp.decode_live_window(window)?;
    let count = lisp.specpdl.len();
    lisp.record_unwind_current_buffer();
    let contents = lisp.window(w).contents;
    let result = (|| -> LispResult<()> {
        crate::buffer::f_set_buffer(lisp, &[contents])?;
        if !lisp
            .find_symbol_value(sym::WINDOW_SCROLL_FUNCTIONS)
            .is_nil()
        {
            let start = lisp.window(w).start;
            let pos = lisp.marker_position_value(start).unwrap_or(Value::NIL);
            lisp.run_hook_with_args(&[sym::WINDOW_SCROLL_FUNCTIONS, window, pos], |l, a| {
                l.funcall(a[0], &a[1..]).map(|_| Value::NIL)
            })?;
        }
        Ok(())
    })();
    let unbound = lisp.unbind_to(count);
    result?;
    unbound?;
    Ok(Value::NIL)
}

fn f_split_window_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (pixel_size, side, normal_size) = (args[1], args[2], args[3]);
    let mut refer = arg(args, 4);
    let old = lisp.decode_valid_window(args[0])?;
    let frame = lisp.window(old).frame;
    let horflag = side == sym::T || side == sym::LEFT || side == sym::RIGHT;
    let mut parent = Value::NIL;
    let mut dead = false;
    let size = lisp.check_fixnum(pixel_size)?;
    let unit = if horflag {
        lisp.frame(frame).geometry.column_width
    } else {
        lisp.frame(frame).geometry.line_height
    };
    let total_size = size / unit;
    let o_parent = lisp.window(old).parent;
    let combination_limit = lisp.find_symbol_value(sym::WINDOW_COMBINATION_LIMIT) == sym::T
        || o_parent.is_nil()
        || if horflag {
            lisp.window_vertical_combination_p(o_parent)
        } else {
            lisp.window_horizontal_combination_p(o_parent)
        };
    let r;
    if refer.is_nil() {
        refer = if lisp.window_live_p(old) {
            old
        } else {
            lisp.frame(frame).selected_window
        };
        r = refer;
    } else if refer.is_cons() {
        parent = lisp.xcdr(refer);
        refer = lisp.xcar(refer);
        r = lisp.decode_any_window(refer)?;
        let (contents, old_buffer, r_frame) = {
            let rw = lisp.window(r);
            (rw.contents, rw.old_buffer, rw.frame)
        };
        if !contents.is_nil() || !lisp.is_buffer(old_buffer) {
            return Err(lisp.error("REFER's car must specify a deleted, former live window"));
        } else if !lisp.buffer_live_p(old_buffer) {
            return Err(lisp.error("The buffer formerly shown by REFER's car has been killed"));
        } else if r_frame != frame {
            return Err(lisp.error("REFER's car must specify a window on same frame as WINDOW"));
        }
        if parent == sym::T {
            parent = lisp.window(r).parent;
        }
        let p = lisp.decode_any_window(parent)?;
        let (contents, old_buffer, p_frame) = {
            let pw = lisp.window(p);
            (pw.contents, pw.old_buffer, pw.frame)
        };
        if !contents.is_nil() || lisp.is_buffer(old_buffer) {
            return Err(lisp.error("REFER's cdr must specify a deleted, former parent window"));
        } else if p_frame != frame {
            return Err(lisp.error("REFER's cdr must specify window on same frame as WINDOW"));
        }
        dead = true;
    } else {
        r = lisp.decode_any_window(refer)?;
        let (contents, old_buffer, r_frame) = {
            let rw = lisp.window(r);
            (rw.contents, rw.old_buffer, rw.frame)
        };
        if contents.is_nil() {
            if !lisp.is_buffer(old_buffer) {
                return Err(lisp
                    .error("REFER must specify a former live window (must have shown a buffer)"));
            } else if !lisp.buffer_live_p(old_buffer) {
                return Err(lisp.error("The buffer formerly shown by REFER has been killed"));
            } else if r_frame != frame {
                return Err(lisp.error("REFER must specify a window on same frame as WINDOW"));
            }
            dead = true;
        } else if !parent.is_nil() {
            return Err(lisp.error("If REFER is a cons, its car must not specify a live window"));
        } else if !lisp.window_live_p(refer) {
            return Err(lisp.error("REFER is not a live window (does not show a buffer)"));
        }
    }
    if lisp.window(old).mini {
        return Err(lisp.error("Attempt to split minibuffer window"));
    } else if total_size < if horflag { 2 } else { 1 } {
        return Err(lisp.error("Size of new window too small (after split)"));
    } else if !combination_limit
        && !lisp
            .find_symbol_value(sym::WINDOW_COMBINATION_RESIZE)
            .is_nil()
    {
        let op = o_parent;
        let op_size = if horflag {
            lisp.window(op).pixel_width
        } else {
            lisp.window(op).pixel_height
        };
        lisp.window_mut(op).new_pixel = Value::fixnum(op_size - size);
        if !lisp.window_resize_check(op, horflag) {
            return Err(lisp.error("Window sizes don't fit"));
        }
        lisp.window_mut(op).new_pixel = Value::fixnum(op_size);
    } else if !lisp.window_resize_check(old, horflag) {
        return Err(lisp.error("Resizing old window failed"));
    } else {
        let o = lisp.window(old);
        let o_size = if horflag {
            o.pixel_width
        } else {
            o.pixel_height
        };
        if size + xfixnum(o.new_pixel) != o_size {
            return Err(lisp.error("Sum of sizes of old and new window don't fit"));
        }
    }
    let p = if combination_limit {
        let new_normal = if horflag {
            lisp.window(old).normal_cols
        } else {
            lisp.window(old).normal_lines
        };
        if parent.is_nil() {
            parent = lisp.make_parent_window(frame);
        }
        let p = parent;
        lisp.replace_window(old, parent, true);
        {
            let o = lisp.window_mut(old);
            o.next = Value::NIL;
            o.prev = Value::NIL;
            o.parent = parent;
        }
        lisp.wset_combination(p, horflag, old);
        if lisp.find_symbol_value(sym::WINDOW_COMBINATION_LIMIT) == sym::T {
            lisp.window_mut(p).combination_limit = sym::T;
        }
        let (o_pixel, o_total) = {
            let o = lisp.window(old);
            if horflag {
                (o.pixel_width, o.total_cols)
            } else {
                (o.pixel_height, o.total_lines)
            }
        };
        let pw = lisp.window_mut(p);
        pw.new_pixel = Value::fixnum(o_pixel);
        pw.new_total = Value::fixnum(o_total);
        pw.new_normal = new_normal;
        p
    } else {
        o_parent
    };
    lisp.fset_redisplay(frame);
    let new = if dead { refer } else { lisp.make_window() };
    let o_parent = lisp.window(old).parent;
    {
        let n = lisp.window_mut(new);
        n.frame = frame;
        n.parent = o_parent;
    }
    if side == sym::ABOVE || side == sym::LEFT {
        let o_prev = lisp.window(old).prev;
        lisp.window_mut(new).prev = o_prev;
        if o_prev.is_nil() {
            lisp.wset_combination(p, horflag, new);
        } else {
            lisp.window_mut(o_prev).next = new;
        }
        lisp.window_mut(new).next = old;
        lisp.window_mut(old).prev = new;
    } else {
        let o_next = lisp.window(old).next;
        lisp.window_mut(new).next = o_next;
        if !o_next.is_nil() {
            lisp.window_mut(o_next).prev = new;
        }
        lisp.window_mut(new).prev = old;
        lisp.window_mut(old).next = new;
    }
    {
        let n = lisp.window_mut(new);
        n.flags.window_end_valid = false;
        n.last_cursor_vpos = 0;
    }
    if !dead {
        let rw = lisp.window(r);
        let (lm, rm, lf, rf, outside, sbw, sbh, vtype, htype) = (
            rw.left_margin_cols,
            rw.right_margin_cols,
            rw.left_fringe_width,
            rw.right_fringe_width,
            rw.flags.fringes_outside_margins,
            rw.scroll_bar_width,
            rw.scroll_bar_height,
            rw.vertical_scroll_bar_type,
            rw.horizontal_scroll_bar_type,
        );
        let n = lisp.window_mut(new);
        n.left_margin_cols = lm;
        n.right_margin_cols = rm;
        n.left_fringe_width = lf;
        n.right_fringe_width = rf;
        n.flags.fringes_outside_margins = outside;
        n.scroll_bar_width = sbw;
        n.scroll_bar_height = sbh;
        n.vertical_scroll_bar_type = vtype;
        n.horizontal_scroll_bar_type = htype;
    }
    let o = lisp.window(old).clone_geometry();
    {
        let n = lisp.window_mut(new);
        if horflag {
            n.pixel_top = o.pixel_top;
            n.top_line = o.top_line;
            n.pixel_height = o.pixel_height;
            n.total_lines = o.total_lines;
        } else {
            n.pixel_left = o.pixel_left;
            n.left_col = o.left_col;
            n.pixel_width = o.pixel_width;
            n.total_cols = o.total_cols;
        }
        n.new_pixel = pixel_size;
    }
    let sum: i64 = lisp
        .window_children(p)
        .into_iter()
        .filter(|&c| c != new)
        .map(|c| xfixnum(lisp.window(c).new_total))
        .sum();
    let p_total = if horflag {
        lisp.window(p).total_cols
    } else {
        lisp.window(p).total_lines
    };
    {
        let n = lisp.window_mut(new);
        n.new_total = Value::fixnum(p_total - sum);
        n.new_normal = normal_size;
    }
    if dead {
        let old_buffer = lisp.window(new).old_buffer;
        lisp.wset_buffer(new, old_buffer);
        let contents = lisp.window(new).contents;
        let markers = {
            let n = lisp.window(new);
            [n.start, n.pointm, n.old_pointm]
        };
        for marker in markers {
            let charpos = lisp.marker(marker).charpos as i64;
            lisp.set_marker(marker, Value::fixnum(charpos), contents, true)?;
        }
        lisp.windows.window_list = Value::NIL;
        let seq = lisp.window(new).sequence_number;
        let table = lisp.plain_value(sym::WINDOW_DEAD_WINDOWS_TABLE);
        crate::fns::f_remhash(lisp, &[Value::fixnum(seq), table])?;
    }
    lisp.window_resize_apply(p, horflag);
    lisp.adjust_frame_glyphs(frame);
    let r_contents = lisp.window(r).contents;
    lisp.set_window_buffer(new, r_contents, true, true)?;
    lisp.frame_mut(frame).flags.window_change = true;
    Ok(new)
}

fn f_delete_window_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let window = lisp.decode_any_window(args[0])?;
    if lisp.window(window).contents.is_nil() {
        return Ok(Value::NIL);
    }
    let parent = lisp.window(window).parent;
    if parent.is_nil() {
        return Err(lisp.error("Attempt to delete minibuffer or sole ordinary window"));
    } else if lisp.window(window).prev.is_nil() && lisp.window(window).next.is_nil() {
        return Err(lisp.error("Attempt to delete sole window of parent"));
    }
    let p = parent;
    let horflag = lisp.window_horizontal_combination_p(p);
    let frame = lisp.window(window).frame;
    let root = lisp.frame(frame).root_window;
    let mut before_sibling = false;
    let sibling;
    if lisp.window(window).prev.is_nil() {
        before_sibling = true;
        sibling = lisp.window(window).next;
        lisp.window_mut(sibling).prev = Value::NIL;
        lisp.wset_combination(p, horflag, sibling);
    } else {
        sibling = lisp.window(window).prev;
        let w_next = lisp.window(window).next;
        lisp.window_mut(sibling).next = w_next;
        if !w_next.is_nil() {
            lisp.window_mut(w_next).prev = sibling;
        }
    }
    let root_size = if horflag {
        lisp.window(root).pixel_width
    } else {
        lisp.window(root).pixel_height
    };
    if lisp.window_resize_check(root, horflag) && xfixnum(lisp.window(root).new_pixel) == root_size
    {
        lisp.window_resize_apply(p, horflag);
        if !lisp.frame_initial_p(frame) && lisp.mouse_hl_info(frame).mouse_face_window == window {
            lisp.mouse_hl_info_mut(frame).mouse_face_window = Value::NIL;
        }
        lisp.fset_redisplay(frame);
        lisp.windows.window_list = Value::NIL;
        lisp.window_mut(window).next = Value::NIL;
        lisp.free_window_matrices(window);
        let contents = lisp.window(window).contents;
        if lisp.is_window(contents) {
            lisp.delete_all_child_windows(contents)?;
            lisp.wset_combination(window, false, Value::NIL);
        } else {
            lisp.window_mut(window).old_buffer = contents;
            lisp.unshow_buffer(window)?;
            let (pointm, old_pointm, start) = {
                let w = lisp.window(window);
                (w.pointm, w.old_pointm, w.start)
            };
            lisp.unchain_marker(pointm);
            lisp.unchain_marker(old_pointm);
            lisp.unchain_marker(start);
            lisp.wset_buffer(window, Value::NIL);
            let seq = lisp.window(window).sequence_number;
            let table = lisp.plain_value(sym::WINDOW_DEAD_WINDOWS_TABLE);
            lisp.puthash(Value::fixnum(seq), window, table)?;
        }
        if lisp.window(sibling).prev.is_nil() && lisp.window(sibling).next.is_nil() {
            lisp.replace_window(parent, sibling, false);
            let (normal_cols, normal_lines) = {
                let pw = lisp.window(p);
                (pw.normal_cols, pw.normal_lines)
            };
            let s = lisp.window_mut(sibling);
            s.normal_cols = normal_cols;
            s.normal_lines = normal_lines;
            lisp.wset_combination(p, false, Value::NIL);
            lisp.recombine_windows(sibling);
        }
        lisp.adjust_frame_glyphs(frame);
        let frame_selected = lisp.frame(frame).selected_window;
        if !lisp.window_live_p(frame_selected) {
            let new_selected_window = crate::window::f_frame_first_window(lisp, &[frame])?;
            if frame_selected == lisp.windows.selected_window {
                crate::window::f_select_window(lisp, &[new_selected_window, sym::T])?;
            } else {
                lisp.frame_mut(frame).selected_window = new_selected_window;
            }
        }
        lisp.frame_mut(frame).flags.window_change = true;
    } else {
        if before_sibling {
            lisp.window_mut(sibling).prev = window;
            lisp.wset_combination(p, horflag, window);
        } else {
            lisp.window_mut(sibling).next = window;
            let w_next = lisp.window(window).next;
            if !w_next.is_nil() {
                lisp.window_mut(w_next).prev = window;
            }
        }
        return Err(lisp.error("Deletion failed"));
    }
    Ok(Value::NIL)
}

fn f_delete_other_windows_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut window = lisp.decode_valid_window(arg(args, 0))?;
    let mut root = arg(args, 1);
    let frame = lisp.window(window).frame;
    if root.is_nil() {
        root = lisp.frame(frame).root_window;
    } else {
        root = lisp.decode_valid_window(root)?;
        let mut pwindow = lisp.window(window).parent;
        while !pwindow.is_nil() {
            if pwindow == root {
                break;
            }
            pwindow = lisp.window(pwindow).parent;
        }
        if pwindow != root {
            return Err(lisp.error("Specified root is not an ancestor of specified window"));
        }
    }
    let r = root;
    if window == root {
        return Ok(Value::NIL);
    } else if lisp.window(window).mini {
        return Err(lisp.error("Can't expand minibuffer to full frame"));
    }
    let mut startpos = 0;
    let mut startbyte = 0;
    let mut top = 0;
    let mut swindow = Value::NIL;
    if lisp.is_buffer(lisp.window(window).contents) {
        let start = lisp.window(window).start;
        startpos = lisp.marker_position_checked(start)?;
        startbyte = lisp.marker_byte_position_checked(start)?;
        top = lisp.window(window).top_line - lisp.frame_top_margin(frame);
        if window != lisp.frame(frame).selected_window {
            if lisp.frames.selected_frame == frame {
                crate::window::f_select_window(lisp, &[window, Value::NIL])?;
            } else {
                lisp.frame_mut(frame).selected_window = window;
            }
        }
    } else {
        swindow = lisp.frame(frame).selected_window;
        loop {
            let mut pwindow = swindow;
            while !pwindow.is_nil() && window != pwindow {
                pwindow = lisp.window(pwindow).parent;
            }
            if window == pwindow {
                break;
            }
            swindow = crate::window::f_previous_window(lisp, &[swindow, sym::LAMBDA, Value::NIL])?;
        }
        if swindow != lisp.frame(frame).selected_window {
            if lisp.frames.selected_frame == frame {
                crate::window::f_select_window(lisp, &[swindow, Value::NIL])?;
            } else {
                lisp.frame_mut(frame).selected_window = swindow;
            }
        }
    }
    if !lisp.frame_initial_p(frame) && lisp.mouse_hl_info(frame).mouse_face_window == window {
        lisp.mouse_hl_info_mut(frame).reset();
    }
    lisp.free_window_matrices(r);
    lisp.fset_redisplay(frame);
    lisp.windows.window_list = Value::NIL;
    let mut resize_failed = false;
    if !lisp.is_buffer(lisp.window(window).contents) {
        let base = lisp.stack.len();
        lisp.stack.extend_from_slice(&[window, swindow, r]);
        let result = (|| -> LispResult<()> {
            let (r_pixel_top, r_top_line, r_pixel_height) = {
                let rw = lisp.window(r);
                (rw.pixel_top, rw.top_line, rw.pixel_height)
            };
            let delta = r_pixel_height - lisp.window(window).pixel_height;
            {
                let w = lisp.window_mut(window);
                w.pixel_top = r_pixel_top;
                w.top_line = r_top_line;
            }
            lisp.resize_root_window(window, delta, Value::NIL, Value::NIL, sym::T)?;
            if lisp.window_resize_check(window, false) {
                lisp.window_resize_apply(window, false);
            } else {
                lisp.resize_root_window(window, delta, Value::NIL, sym::T, sym::T)?;
                if lisp.window_resize_check(window, false) {
                    lisp.window_resize_apply(window, false);
                } else {
                    resize_failed = true;
                }
            }
            if !resize_failed {
                let (r_left_col, r_pixel_left, r_pixel_width) = {
                    let rw = lisp.window(r);
                    (rw.left_col, rw.pixel_left, rw.pixel_width)
                };
                {
                    let w = lisp.window_mut(window);
                    w.left_col = r_left_col;
                    w.pixel_left = r_pixel_left;
                }
                let delta = r_pixel_width - lisp.window(window).pixel_width;
                lisp.resize_root_window(window, delta, sym::T, Value::NIL, sym::T)?;
                if lisp.window_resize_check(window, true) {
                    lisp.window_resize_apply(window, true);
                } else {
                    lisp.resize_root_window(window, delta, sym::T, sym::T, sym::T)?;
                    if lisp.window_resize_check(window, true) {
                        lisp.window_resize_apply(window, true);
                    } else {
                        resize_failed = true;
                    }
                }
            }
            Ok(())
        })();
        lisp.stack.truncate(base);
        result?;
        if resize_failed {
            window = swindow;
        }
    }
    let w_prev = lisp.window(window).prev;
    if !w_prev.is_nil() {
        let sibling = w_prev;
        let w_next = lisp.window(window).next;
        lisp.window_mut(sibling).next = w_next;
        if !w_next.is_nil() {
            lisp.window_mut(w_next).prev = sibling;
        }
    } else {
        let sibling = lisp.window(window).next;
        lisp.window_mut(sibling).prev = Value::NIL;
        let w_parent = lisp.window(window).parent;
        let horizontal = lisp.window(w_parent).horizontal;
        lisp.wset_combination(w_parent, horizontal, sibling);
    }
    let r_contents = lisp.window(r).contents;
    if lisp.is_window(r_contents) {
        lisp.delete_all_child_windows(r_contents)?;
        lisp.wset_combination(r, false, Value::NIL);
    }
    lisp.replace_window(root, window, true);
    lisp.window_pixel_to_total(frame, Value::NIL)?;
    lisp.window_pixel_to_total(frame, sym::T)?;
    let contents = lisp.window(window).contents;
    if lisp.is_buffer(contents) && !resize_failed {
        let new_top = lisp.window(window).top_line - lisp.frame_top_margin(frame);
        let (begv, zv) = {
            let b = lisp.buffer(contents);
            (b.begv, b.zv)
        };
        if new_top != top && startpos >= begv && startpos <= zv {
            let obuf = lisp.current_buffer;
            crate::buffer::f_set_buffer(lisp, &[contents])?;
            let pos = lisp.vmotion(startpos as i64, startbyte as i64, -top, window)?;
            let start = lisp.window(window).start;
            lisp.set_marker_position(start, pos.bufpos as usize, contents);
            let at_line_beg = pos.bytepos as usize == lisp.cur().begv_byte
                || lisp.cur().text.byte_at(pos.bytepos as usize - 1) == b'\n';
            let w = lisp.window_mut(window);
            w.flags.window_end_valid = false;
            w.flags.start_at_line_beg = at_line_beg;
            w.flags.optional_new_start = true;
            w.vscroll = 0;
            w.flags.preserve_vscroll_p = false;
            lisp.set_buffer_internal(obuf);
        }
    }
    lisp.adjust_frame_glyphs(frame);
    lisp.frame_mut(frame).flags.window_change = true;
    Ok(Value::NIL)
}
