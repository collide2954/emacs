use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::{Vectorlike, Weakness};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

#[derive(Clone, Debug, Default)]
pub struct WindowFlags {
    pub update_mode_line: bool,
    pub last_had_star: bool,
    pub start_at_line_beg: bool,
    pub force_start: bool,
    pub optional_new_start: bool,
    pub must_be_updated_p: bool,
    pub pseudo_window_p: bool,
    pub fringes_outside_margins: bool,
    pub fringes_persistent: bool,
    pub scroll_bars_persistent: bool,
    pub window_end_valid: bool,
    pub redisplay: bool,
    pub suspend_auto_hscroll: bool,
    pub preserve_vscroll_p: bool,
}

#[derive(Clone, Debug)]
pub struct Window {
    pub frame: Value,
    pub next: Value,
    pub prev: Value,
    pub parent: Value,
    pub normal_lines: Value,
    pub normal_cols: Value,
    pub new_total: Value,
    pub new_normal: Value,
    pub new_pixel: Value,
    pub contents: Value,
    pub prev_buffers: Value,
    pub next_buffers: Value,
    pub old_buffer: Value,
    pub start: Value,
    pub pointm: Value,
    pub old_pointm: Value,
    pub temslot: Value,
    pub vertical_scroll_bar: Value,
    pub vertical_scroll_bar_type: Value,
    pub horizontal_scroll_bar: Value,
    pub horizontal_scroll_bar_type: Value,
    pub display_table: Value,
    pub dedicated: Value,
    pub combination_limit: Value,
    pub window_parameters: Value,
    pub cursor_type: Value,
    pub mode_line_help_echo: Value,
    pub use_time: i64,
    pub sequence_number: i64,
    pub change_stamp: i64,
    pub pixel_left: i64,
    pub pixel_top: i64,
    pub left_col: i64,
    pub top_line: i64,
    pub pixel_width: i64,
    pub pixel_height: i64,
    pub old_pixel_width: i64,
    pub old_pixel_height: i64,
    pub old_body_pixel_width: i64,
    pub old_body_pixel_height: i64,
    pub total_cols: i64,
    pub total_lines: i64,
    pub hscroll: i64,
    pub min_hscroll: i64,
    pub hscroll_whole: i64,
    pub base_line_number: i64,
    pub base_line_pos: i64,
    pub column_number_displayed: i64,
    pub nrows_scale_factor: i64,
    pub ncols_scale_factor: i64,
    pub last_cursor_vpos: i64,
    pub phys_cursor_width: i64,
    pub left_fringe_width: i64,
    pub right_fringe_width: i64,
    pub left_margin_cols: i64,
    pub right_margin_cols: i64,
    pub scroll_bar_width: i64,
    pub scroll_bar_height: i64,
    pub mode_line_height: i64,
    pub header_line_height: i64,
    pub tab_line_height: i64,
    pub window_end_pos: i64,
    pub window_end_vpos: i64,
    pub vscroll: i64,
    pub mini: bool,
    pub horizontal: bool,
    pub flags: WindowFlags,
}

impl Window {
    pub fn trace(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.frame,
            self.next,
            self.prev,
            self.parent,
            self.normal_lines,
            self.normal_cols,
            self.new_total,
            self.new_normal,
            self.new_pixel,
            self.contents,
            self.prev_buffers,
            self.next_buffers,
            self.old_buffer,
            self.start,
            self.pointm,
            self.old_pointm,
            self.temslot,
            self.vertical_scroll_bar,
            self.vertical_scroll_bar_type,
            self.horizontal_scroll_bar,
            self.horizontal_scroll_bar_type,
            self.display_table,
            self.dedicated,
            self.combination_limit,
            self.window_parameters,
            self.cursor_type,
            self.mode_line_help_echo,
        ]);
    }
}

pub struct Windows {
    pub selected_window: Value,
    pub old_selected_window: Value,
    pub minibuf_window: Value,
    pub minibuf_selected_window: Value,
    pub window_list: Value,
    pub last_mouse_window: Value,
    pub sequence_number: i64,
    pub window_select_count: i64,
    pub windows_or_buffers_changed: i64,
    pub update_mode_lines: i64,
}

impl Default for Windows {
    fn default() -> Self {
        Windows {
            selected_window: Value::NIL,
            old_selected_window: Value::NIL,
            minibuf_window: Value::NIL,
            minibuf_selected_window: Value::NIL,
            window_list: Value::NIL,
            last_mouse_window: Value::NIL,
            sequence_number: 0,
            window_select_count: 0,
            windows_or_buffers_changed: 0,
            update_mode_lines: 0,
        }
    }
}

impl Windows {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.selected_window,
            self.old_selected_window,
            self.minibuf_window,
            self.minibuf_selected_window,
            self.window_list,
            self.last_mouse_window,
        ]);
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("windowp", 1, 1, f_windowp),
    Subr::fixed("window-valid-p", 1, 1, f_window_valid_p),
    Subr::fixed("window-live-p", 1, 1, f_window_live_p),
    Subr::fixed("window-frame", 0, 1, f_window_frame),
    Subr::fixed("frame-root-window", 0, 1, f_frame_root_window),
    Subr::fixed("minibuffer-window", 0, 1, f_minibuffer_window),
    Subr::fixed("window-minibuffer-p", 0, 1, f_window_minibuffer_p),
    Subr::fixed("frame-first-window", 0, 1, f_frame_first_window),
    Subr::fixed("frame-selected-window", 0, 1, f_frame_selected_window),
    Subr::fixed(
        "frame-old-selected-window",
        0,
        1,
        f_frame_old_selected_window,
    ),
    Subr::fixed(
        "set-frame-selected-window",
        2,
        3,
        f_set_frame_selected_window,
    ),
    Subr::fixed("selected-window", 0, 0, f_selected_window),
    Subr::fixed("old-selected-window", 0, 0, f_old_selected_window),
    Subr::fixed(
        "minibuffer-selected-window",
        0,
        0,
        f_minibuffer_selected_window,
    ),
    Subr::fixed("select-window", 1, 2, f_select_window),
    Subr::fixed("window-buffer", 0, 1, f_window_buffer),
    Subr::fixed("window-old-buffer", 0, 1, f_window_old_buffer),
    Subr::fixed("window-parent", 0, 1, f_window_parent),
    Subr::fixed("window-top-child", 0, 1, f_window_top_child),
    Subr::fixed("window-left-child", 0, 1, f_window_left_child),
    Subr::fixed("window-next-sibling", 0, 1, f_window_next_sibling),
    Subr::fixed("window-prev-sibling", 0, 1, f_window_prev_sibling),
    Subr::fixed("window-combination-limit", 1, 1, f_window_combination_limit),
    Subr::fixed(
        "set-window-combination-limit",
        2,
        2,
        f_set_window_combination_limit,
    ),
    Subr::fixed("window-use-time", 0, 1, f_window_use_time),
    Subr::fixed("window-bump-use-time", 0, 1, f_window_bump_use_time),
    Subr::fixed("window-pixel-width", 0, 1, f_window_pixel_width),
    Subr::fixed("window-pixel-height", 0, 1, f_window_pixel_height),
    Subr::fixed("window-old-pixel-width", 0, 1, f_window_old_pixel_width),
    Subr::fixed("window-old-pixel-height", 0, 1, f_window_old_pixel_height),
    Subr::fixed("window-total-height", 0, 2, f_window_total_height),
    Subr::fixed("window-total-width", 0, 2, f_window_total_width),
    Subr::fixed("window-new-total", 0, 1, f_window_new_total),
    Subr::fixed("window-normal-size", 0, 2, f_window_normal_size),
    Subr::fixed("window-new-normal", 0, 1, f_window_new_normal),
    Subr::fixed("window-new-pixel", 0, 1, f_window_new_pixel),
    Subr::fixed("window-pixel-left", 0, 1, f_window_pixel_left),
    Subr::fixed("window-pixel-top", 0, 1, f_window_pixel_top),
    Subr::fixed("window-left-column", 0, 1, f_window_left_column),
    Subr::fixed("window-top-line", 0, 1, f_window_top_line),
    Subr::fixed("window-hscroll", 0, 1, f_window_hscroll),
    Subr::fixed("window-point", 0, 1, f_window_point),
    Subr::fixed("window-old-point", 0, 1, f_window_old_point),
    Subr::fixed("window-start", 0, 1, f_window_start),
    Subr::fixed("window-dedicated-p", 0, 1, f_window_dedicated_p),
    Subr::fixed("set-window-dedicated-p", 2, 2, f_set_window_dedicated_p),
    Subr::fixed("window-prev-buffers", 0, 1, f_window_prev_buffers),
    Subr::fixed("set-window-prev-buffers", 2, 2, f_set_window_prev_buffers),
    Subr::fixed("window-next-buffers", 0, 1, f_window_next_buffers),
    Subr::fixed("set-window-next-buffers", 2, 2, f_set_window_next_buffers),
    Subr::fixed("window-parameters", 0, 1, f_window_parameters),
    Subr::fixed("window-parameter", 2, 2, f_window_parameter),
    Subr::fixed("set-window-parameter", 3, 3, f_set_window_parameter),
    Subr::fixed("window-display-table", 0, 1, f_window_display_table),
    Subr::fixed("set-window-display-table", 2, 2, f_set_window_display_table),
    Subr::fixed("window-margins", 0, 1, f_window_margins),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.put(sym::SCROLL_UP, sym::SCROLL_COMMAND, sym::T);
    lisp.put(sym::SCROLL_DOWN, sym::SCROLL_COMMAND, sym::T);
    lisp.defvar(sym::TEMP_BUFFER_SHOW_FUNCTION, Value::NIL);
    lisp.defvar(sym::MINIBUFFER_SCROLL_WINDOW, Value::NIL);
    lisp.defvar_bool(sym::MODE_LINE_IN_NON_SELECTED_WINDOWS, true);
    lisp.defvar(sym::OTHER_WINDOW_SCROLL_BUFFER, Value::NIL);
    lisp.defvar(sym::OTHER_WINDOW_SCROLL_DEFAULT, Value::NIL);
    lisp.defvar_bool(sym::AUTO_WINDOW_VSCROLL, true);
    lisp.defvar_int(sym::NEXT_SCREEN_CONTEXT_LINES, 2);
    for symbol in [
        sym::SCROLL_PRESERVE_SCREEN_POSITION,
        sym::WINDOW_POINT_INSERTION_TYPE,
        sym::WINDOW_BUFFER_CHANGE_FUNCTIONS,
        sym::WINDOW_SIZE_CHANGE_FUNCTIONS,
        sym::WINDOW_SELECTION_CHANGE_FUNCTIONS,
        sym::WINDOW_STATE_CHANGE_FUNCTIONS,
        sym::WINDOW_STATE_CHANGE_HOOK,
        sym::WINDOW_CONFIGURATION_CHANGE_HOOK,
        sym::WINDOW_RESTORE_KILLED_BUFFER_WINDOWS,
    ] {
        lisp.defvar(symbol, Value::NIL);
    }
    lisp.defvar(sym::RECENTER_REDISPLAY, sym::TTY);
    lisp.defvar(sym::WINDOW_COMBINATION_RESIZE, Value::NIL);
    lisp.defvar(sym::WINDOW_COMBINATION_LIMIT, sym::WINDOW_SIZE);
    let clone = lisp.cons(sym::CLONE_OF, sym::T);
    let persistent = lisp.list(&[clone]);
    lisp.defvar(sym::WINDOW_PERSISTENT_PARAMETERS, persistent);
    lisp.defvar_bool(sym::WINDOW_RESIZE_PIXELWISE, false);
    lisp.defvar_bool(sym::FAST_BUT_IMPRECISE_SCROLLING, false);
    let dead = lisp.make_hash_table_with(sym::EQL, Weakness::Value);
    lisp.defvar(sym::WINDOW_DEAD_WINDOWS_TABLE, dead);
    lisp.defvar_bool(sym::WINDOW_AUTO_REDRAW_ON_PARAMETER_CHANGE, true);
}

pub fn init_window_once(lisp: &mut Lisp) {
    let frame = lisp.make_initial_frame().expect("initial frame");
    lisp.frames.selected_frame = frame;
    lisp.frames.old_selected_frame = frame;
    lisp.set_plain_value(sym::TERMINAL_FRAME_2, frame);
    let f = lisp.frame(frame);
    let (mw, sw, osw) = (
        f.minibuffer_window,
        f.selected_window,
        f.old_selected_window,
    );
    lisp.windows.minibuf_window = mw;
    lisp.windows.selected_window = sw;
    lisp.windows.old_selected_window = osw;
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn window_discard_buffer_from_alist(
    lisp: &mut Lisp,
    buffer: Value,
    list: Value,
    by_car: bool,
) -> Value {
    let mut head = list;
    let mut prev = Value::NIL;
    let mut tail = list;
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        let key = if by_car { lisp.car_safe(elt) } else { elt };
        let next = lisp.xcdr(tail);
        if key == buffer {
            if prev.is_nil() {
                head = next;
            } else {
                lisp.heap.set_cdr(prev, next);
            }
        } else {
            prev = tail;
        }
        tail = next;
    }
    head
}

impl Lisp {
    pub fn is_window(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Window(_))
    }

    pub fn window(&self, v: Value) -> &Window {
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Window(w) => w,
            _ => unreachable!("not a window"),
        }
    }

    pub fn window_mut(&mut self, v: Value) -> &mut Window {
        match self.heap.vectorlike_mut(v) {
            Vectorlike::Window(w) => w,
            _ => unreachable!("not a window"),
        }
    }

    pub fn window_live_p(&self, v: Value) -> bool {
        self.is_window(v) && self.is_buffer(self.window(v).contents)
    }

    pub fn window_valid_p(&self, v: Value) -> bool {
        self.is_window(v) && !self.window(v).contents.is_nil()
    }

    pub fn check_live_window(&mut self, v: Value) -> LispResult<Value> {
        if self.window_live_p(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::WINDOW_LIVE_P, v))
        }
    }

    pub fn check_valid_window(&mut self, v: Value) -> LispResult<Value> {
        if self.window_valid_p(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::WINDOW_VALID_P, v))
        }
    }

    pub fn decode_live_window(&mut self, window: Value) -> LispResult<Value> {
        if window.is_nil() {
            return Ok(self.windows.selected_window);
        }
        self.check_live_window(window)
    }

    pub fn decode_any_window(&mut self, window: Value) -> LispResult<Value> {
        if window.is_nil() {
            return Ok(self.windows.selected_window);
        }
        if self.is_window(window) {
            Ok(window)
        } else {
            Err(self.wrong_type(sym::WINDOWP, window))
        }
    }

    pub fn decode_valid_window(&mut self, window: Value) -> LispResult<Value> {
        if window.is_nil() {
            return Ok(self.windows.selected_window);
        }
        self.check_valid_window(window)
    }

    pub fn make_window(&mut self) -> Value {
        self.windows.sequence_number += 1;
        let sequence_number = self.windows.sequence_number;
        let normal_lines = self.float(1.0);
        self.stack.push(normal_lines);
        let normal_cols = self.float(1.0);
        self.stack.push(normal_cols);
        let start = self.make_marker_object();
        self.stack.push(start);
        let pointm = self.make_marker_object();
        self.stack.push(pointm);
        let old_pointm = self.make_marker_object();
        self.stack.truncate(self.stack.len() - 4);
        let w = Window {
            frame: Value::NIL,
            next: Value::NIL,
            prev: Value::NIL,
            parent: Value::NIL,
            normal_lines,
            normal_cols,
            new_total: Value::fixnum(0),
            new_normal: Value::fixnum(0),
            new_pixel: Value::fixnum(0),
            contents: Value::NIL,
            prev_buffers: Value::NIL,
            next_buffers: Value::NIL,
            old_buffer: Value::NIL,
            start,
            pointm,
            old_pointm,
            temslot: Value::NIL,
            vertical_scroll_bar: Value::NIL,
            vertical_scroll_bar_type: sym::T,
            horizontal_scroll_bar: Value::NIL,
            horizontal_scroll_bar_type: sym::T,
            display_table: Value::NIL,
            dedicated: Value::NIL,
            combination_limit: Value::NIL,
            window_parameters: Value::NIL,
            cursor_type: sym::T,
            mode_line_help_echo: Value::NIL,
            use_time: 0,
            sequence_number,
            change_stamp: 0,
            pixel_left: 0,
            pixel_top: 0,
            left_col: 0,
            top_line: 0,
            pixel_width: 0,
            pixel_height: 0,
            old_pixel_width: 0,
            old_pixel_height: 0,
            old_body_pixel_width: 0,
            old_body_pixel_height: 0,
            total_cols: 0,
            total_lines: 0,
            hscroll: 0,
            min_hscroll: 0,
            hscroll_whole: 0,
            base_line_number: 0,
            base_line_pos: 0,
            column_number_displayed: -1,
            nrows_scale_factor: 1,
            ncols_scale_factor: 1,
            last_cursor_vpos: 0,
            phys_cursor_width: -1,
            left_fringe_width: -1,
            right_fringe_width: -1,
            left_margin_cols: 0,
            right_margin_cols: 0,
            scroll_bar_width: -1,
            scroll_bar_height: -1,
            mode_line_height: -1,
            header_line_height: -1,
            tab_line_height: -1,
            window_end_pos: 0,
            window_end_vpos: 0,
            vscroll: 0,
            mini: false,
            horizontal: false,
            flags: WindowFlags::default(),
        };
        self.windows.window_list = Value::NIL;
        self.heap.vectorlike(Vectorlike::Window(Box::new(w)))
    }

    fn adjust_window_count(&mut self, w: Value, arg: i64) {
        let contents = self.window(w).contents;
        if self.is_buffer(contents) {
            let base = self.buffer(contents).base_buffer;
            let b = if self.is_buffer(base) { base } else { contents };
            let buf = self.buffer_mut(b);
            buf.window_count = (buf.window_count as i64 + arg).max(0) as usize;
            let win = self.window_mut(w);
            win.flags.window_end_valid = false;
            win.base_line_pos = 0;
        }
    }

    pub fn wset_buffer(&mut self, w: Value, val: Value) {
        self.adjust_window_count(w, -1);
        self.window_mut(w).contents = val;
        self.adjust_window_count(w, 1);
    }

    fn window_frame_column_width(&self, w: Value) -> i64 {
        self.frame(self.window(w).frame).geometry.column_width
    }

    fn window_frame_window_p(&self, w: Value) -> bool {
        self.frame(self.window(w).frame).window_p()
    }

    fn window_fringes_width(&self, w: Value) -> i64 {
        if !self.window_frame_window_p(w) {
            return 0;
        }
        let win = self.window(w);
        let g = &self.frame(win.frame).geometry;
        let left = if win.left_fringe_width >= 0 {
            win.left_fringe_width
        } else {
            g.left_fringe_width
        };
        let right = if win.right_fringe_width >= 0 {
            win.right_fringe_width
        } else {
            g.right_fringe_width
        };
        left + right
    }

    fn window_scroll_bar_area_width(&self, w: Value) -> i64 {
        let win = self.window(w);
        if !self.window_frame_window_p(w) || win.vertical_scroll_bar_type.is_nil() {
            return 0;
        }
        if win.scroll_bar_width >= 0 {
            win.scroll_bar_width
        } else {
            self.frame_scroll_bar_area_width(win.frame)
        }
    }

    fn min_safe_window_pixel_width(&self, w: Value) -> i64 {
        2 * self.window_frame_column_width(w)
    }

    fn set_window_margins(
        &mut self,
        w: Value,
        left_width: Value,
        right_width: Value,
    ) -> LispResult<bool> {
        let unit = self.window_frame_column_width(w);
        let left = if left_width.is_nil() {
            0
        } else {
            self.extract_dimension(left_width)?
        };
        let right = if right_width.is_nil() {
            0
        } else {
            self.extract_dimension(right_width)?
        };
        let win = self.window(w);
        if win.left_margin_cols != left || win.right_margin_cols != right {
            let room = win.pixel_width
                - self.window_fringes_width(w)
                - self.window_scroll_bar_area_width(w)
                - (left + right) * unit;
            if room >= self.min_safe_window_pixel_width(w) {
                let win = self.window_mut(w);
                win.left_margin_cols = left;
                win.right_margin_cols = right;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn extract_dimension(&mut self, dimension: Value) -> LispResult<i64> {
        if dimension.is_nil() {
            return Ok(-1);
        }
        match dimension.as_fixnum() {
            Some(n) if (0..=i32::MAX as i64).contains(&n) => Ok(n),
            _ => Err(self.args_out_of_range3(
                dimension,
                Value::fixnum(0),
                Value::fixnum(i32::MAX as i64),
            )),
        }
    }

    fn apply_window_adjustment(&mut self, w: Value) {
        let win = self.window_mut(w);
        win.flags.window_end_valid = false;
        self.wset_redisplay(w);
    }

    pub fn set_window_buffer(
        &mut self,
        window: Value,
        buffer: Value,
        run_hooks_p: bool,
        keep_margins_p: bool,
    ) -> LispResult<()> {
        let count = self.specpdl.len();
        let base = self.stack.len();
        self.stack.push(window);
        self.stack.push(buffer);
        let r = self.set_window_buffer_1(window, buffer, run_hooks_p, keep_margins_p);
        let u = self.unbind_to(count);
        self.stack.truncate(base);
        r?;
        u
    }

    fn set_window_buffer_1(
        &mut self,
        window: Value,
        buffer: Value,
        run_hooks_p: bool,
        keep_margins_p: bool,
    ) -> LispResult<()> {
        let samebuf = buffer == self.window(window).contents;
        self.wset_buffer(window, buffer);
        if window == self.windows.selected_window {
            self.buffer_mut(buffer).last_selected_window = window;
        }
        let count = self
            .buffer_local_get(buffer, sym::BUFFER_DISPLAY_COUNT)
            .unwrap_or(Value::NIL);
        if count.is_fixnum() || self.is_bignum(count) {
            let next = crate::data::f_add1(self, &[count])?;
            self.buffer_local_set(buffer, sym::BUFFER_DISPLAY_COUNT, next);
        }
        let now = crate::timefns::f_current_time(self, &[])?;
        self.buffer_local_set(buffer, sym::BUFFER_DISPLAY_TIME, now);
        {
            let w = self.window_mut(window);
            w.window_end_pos = 0;
            w.window_end_vpos = 0;
            w.last_cursor_vpos = 0;
        }
        self.window_discard_buffer_from_window(buffer, window);
        if !(keep_margins_p && samebuf) {
            let (pt, last_window_start) = {
                let b = self.buffer(buffer);
                (b.pt, b.last_window_start)
            };
            {
                let w = self.window_mut(window);
                w.hscroll = 0;
                w.min_hscroll = 0;
                w.hscroll_whole = 0;
                w.flags.suspend_auto_hscroll = false;
                w.vscroll = 0;
            }
            let (pointm, old_pointm, start) = {
                let w = self.window(window);
                (w.pointm, w.old_pointm, w.start)
            };
            self.set_marker(pointm, Value::fixnum(pt as i64), buffer, false)?;
            self.set_marker(old_pointm, Value::fixnum(pt as i64), buffer, false)?;
            self.set_marker(start, Value::fixnum(last_window_start as i64), buffer, true)?;
            let w = self.window_mut(window);
            w.flags.start_at_line_beg = false;
            w.flags.force_start = false;
            w.base_line_number = 0;
        }
        self.wset_redisplay(window);
        self.window_mut(window).flags.update_mode_line = true;
        self.record_unwind_current_buffer();
        crate::buffer::f_set_buffer(self, &[buffer])?;
        let insertion_type = !self
            .find_symbol_value(sym::WINDOW_POINT_INSERTION_TYPE)
            .is_nil();
        let (pointm, old_pointm) = {
            let w = self.window(window);
            (w.pointm, w.old_pointm)
        };
        self.marker_mut(pointm).insertion_type = insertion_type;
        self.marker_mut(old_pointm).insertion_type = insertion_type;
        if !keep_margins_p {
            let (left, right) = (
                self.buffer_local_get(buffer, sym::LEFT_MARGIN_WIDTH)
                    .unwrap_or(Value::NIL),
                self.buffer_local_get(buffer, sym::RIGHT_MARGIN_WIDTH)
                    .unwrap_or(Value::NIL),
            );
            self.set_window_margins(window, left, right)?;
            self.apply_window_adjustment(window);
        }
        let scroll_functions = self.find_symbol_value(sym::WINDOW_SCROLL_FUNCTIONS);
        if run_hooks_p && !scroll_functions.is_nil() {
            let start = self.window(window).start;
            let pos = self.marker_position_value(start);
            let pos = pos.unwrap_or(Value::NIL);
            self.run_hook_with_args(&[sym::WINDOW_SCROLL_FUNCTIONS, window, pos], |l, a| {
                l.funcall(a[0], &a[1..]).map(|_| Value::NIL)
            })?;
        }
        if !samebuf && !self.window(window).mini && !self.window(window).flags.pseudo_window_p {
            let f = self.window(window).frame;
            self.frame_mut(f).flags.window_change = true;
        }
        Ok(())
    }

    fn window_discard_buffer_from_window(&mut self, buffer: Value, window: Value) {
        let prev = self.window(window).prev_buffers;
        let prev = window_discard_buffer_from_alist(self, buffer, prev, true);
        self.window_mut(window).prev_buffers = prev;
        let next = self.window(window).next_buffers;
        let next = window_discard_buffer_from_alist(self, buffer, next, false);
        self.window_mut(window).next_buffers = next;
    }

    pub fn window_point(&self, w: Value) -> i64 {
        let win = self.window(w);
        if w == self.windows.selected_window {
            self.buffer(win.contents).pt as i64
        } else {
            self.marker(win.pointm).charpos as i64
        }
    }

    fn select_window_1(&mut self, window: Value, inhibit_point_swap: bool) -> LispResult<()> {
        if !inhibit_point_swap {
            let ow = self.windows.selected_window;
            let contents = self.window(ow).contents;
            if self.is_buffer(contents) {
                let pt = self.buffer(contents).pt;
                let pointm = self.window(ow).pointm;
                self.set_marker(pointm, Value::fixnum(pt as i64), contents, false)?;
            }
        }
        self.windows.selected_window = window;
        let pointm = self.window(window).pointm;
        let pos = self.marker_position_checked(pointm)?;
        self.goto_char_internal(pos);
        Ok(())
    }

    pub fn select_window(
        &mut self,
        window: Value,
        norecord: Value,
        inhibit_point_swap: bool,
    ) -> LispResult {
        self.check_live_window(window)?;
        let frame = self.window(window).frame;
        if self.frame(frame).flags.tooltip {
            return Err(self.error("Cannot select a tooltip window"));
        }
        self.frame_mut(frame).flags.select_mini_window_flag = false;
        let contents = self.window(window).contents;
        crate::buffer::f_set_buffer(self, &[contents])?;
        if !(window == self.windows.selected_window && !inhibit_point_swap) {
            if norecord.is_nil() || norecord == sym::MARK_FOR_REDISPLAY {
                let sw = self.windows.selected_window;
                self.wset_redisplay(sw);
                self.wset_redisplay(window);
            }
            let sf = self.frames.selected_frame;
            if frame != sf {
                self.frame_mut(frame).selected_window = window;
                crate::frame::f_select_frame(self, &[frame, norecord])?;
                return Ok(window);
            }
            self.frame_mut(sf).selected_window = window;
            self.select_window_1(window, inhibit_point_swap)?;
            let contents = self.window(window).contents;
            self.buffer_mut(contents).last_selected_window = window;
        }
        if norecord.is_nil() {
            self.windows.window_select_count += 1;
            let count = self.windows.window_select_count;
            self.window_mut(window).use_time = count;
            let contents = self.window(window).contents;
            self.record_buffer(contents)?;
        }
        Ok(window)
    }

    pub fn window_parameter(&self, w: Value, parameter: Value) -> Value {
        let cell = self.assq(parameter, self.window(w).window_parameters);
        self.cdr_safe(cell)
    }
}

fn f_windowp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.is_window(args[0])))
}

fn f_window_valid_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.window_valid_p(args[0])))
}

fn f_window_live_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.window_live_p(args[0])))
}

fn f_window_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    Ok(lisp.window(w).frame)
}

fn frame_or_window_frame(lisp: &mut Lisp, frame_or_window: Value) -> LispResult<Value> {
    if frame_or_window.is_nil() {
        Ok(lisp.frames.selected_frame)
    } else if lisp.window_valid_p(frame_or_window) {
        Ok(lisp.window(frame_or_window).frame)
    } else {
        lisp.check_live_frame(frame_or_window)
    }
}

fn f_frame_root_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = frame_or_window_frame(lisp, arg(args, 0))?;
    Ok(lisp.frame(f).root_window)
}

fn f_minibuffer_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    Ok(lisp.frame(f).minibuffer_window)
}

fn f_window_minibuffer_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    Ok(lisp.bool(lisp.window(w).mini))
}

fn f_frame_first_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = frame_or_window_frame(lisp, arg(args, 0))?;
    let mut window = lisp.frame(f).root_window;
    while lisp.is_window(lisp.window(window).contents) {
        window = lisp.window(window).contents;
    }
    Ok(window)
}

fn f_frame_selected_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = frame_or_window_frame(lisp, arg(args, 0))?;
    Ok(lisp.frame(f).selected_window)
}

fn f_frame_old_selected_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let frame = if arg(args, 0).is_nil() {
        lisp.frames.selected_frame
    } else {
        arg(args, 0)
    };
    lisp.check_live_frame(frame)?;
    Ok(lisp.frame(frame).old_selected_window)
}

pub(crate) fn f_set_frame_selected_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let frame = if args[0].is_nil() {
        lisp.frames.selected_frame
    } else {
        args[0]
    };
    lisp.check_live_frame(frame)?;
    let window = lisp.check_live_window(args[1])?;
    if frame != lisp.window(window).frame {
        return Err(lisp.error("In `set-frame-selected-window', WINDOW is not on FRAME"));
    }
    if frame == lisp.frames.selected_frame {
        lisp.select_window(window, arg(args, 2), false)
    } else {
        lisp.frame_mut(frame).selected_window = window;
        Ok(window)
    }
}

fn f_selected_window(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.windows.selected_window)
}

fn f_old_selected_window(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.windows.old_selected_window)
}

fn f_minibuffer_selected_window(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let sw = lisp.windows.selected_window;
    let msw = lisp.windows.minibuf_selected_window;
    Ok(
        if lisp.minibuf.minibuf_level > 0 && lisp.window(sw).mini && lisp.window_live_p(msw) {
            msw
        } else {
            Value::NIL
        },
    )
}

pub(crate) fn f_select_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.select_window(args[0], arg(args, 1), false)
}

fn f_window_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_any_window(arg(args, 0))?;
    let contents = lisp.window(w).contents;
    Ok(if lisp.is_buffer(contents) {
        contents
    } else {
        Value::NIL
    })
}

fn f_window_old_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_any_window(arg(args, 0))?;
    let win = lisp.window(w);
    Ok(if win.old_buffer.is_nil() {
        Value::NIL
    } else if win.change_stamp != lisp.frame(win.frame).change_stamp {
        sym::T
    } else {
        win.old_buffer
    })
}

fn valid_field(lisp: &mut Lisp, args: &[Value], get: fn(&Window) -> Value) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    Ok(get(lisp.window(w)))
}

fn valid_int(lisp: &mut Lisp, args: &[Value], get: fn(&Window) -> i64) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    Ok(Value::fixnum(get(lisp.window(w))))
}

fn f_window_parent(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_field(lisp, args, |w| w.parent)
}

fn f_window_top_child(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    let win = lisp.window(w);
    Ok(if lisp.is_window(win.contents) && !win.horizontal {
        win.contents
    } else {
        Value::NIL
    })
}

fn f_window_left_child(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    let win = lisp.window(w);
    Ok(if lisp.is_window(win.contents) && win.horizontal {
        win.contents
    } else {
        Value::NIL
    })
}

fn f_window_next_sibling(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_field(lisp, args, |w| w.next)
}

fn f_window_prev_sibling(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_field(lisp, args, |w| w.prev)
}

fn f_window_combination_limit(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.check_valid_window(args[0])?;
    if lisp.is_buffer(lisp.window(w).contents) {
        return Err(lisp.error("Combination limit is meaningful for internal windows only"));
    }
    Ok(lisp.window(w).combination_limit)
}

fn f_set_window_combination_limit(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.check_valid_window(args[0])?;
    if lisp.is_buffer(lisp.window(w).contents) {
        return Err(lisp.error("Combination limit is meaningful for internal windows only"));
    }
    lisp.window_mut(w).combination_limit = args[1];
    Ok(args[1])
}

fn f_window_use_time(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window(w).use_time))
}

fn f_window_bump_use_time(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    let sw = lisp.windows.selected_window;
    let count = lisp.windows.window_select_count;
    if w != sw && lisp.window(sw).use_time == count {
        lisp.window_mut(w).use_time = count;
        lisp.windows.window_select_count += 1;
        let next = lisp.windows.window_select_count;
        lisp.window_mut(sw).use_time = next;
        Ok(Value::fixnum(count))
    } else {
        Ok(Value::NIL)
    }
}

fn f_window_pixel_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_int(lisp, args, |w| w.pixel_width)
}

fn f_window_pixel_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_int(lisp, args, |w| w.pixel_height)
}

fn f_window_old_pixel_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_int(lisp, args, |w| w.old_pixel_width)
}

fn f_window_old_pixel_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_int(lisp, args, |w| w.old_pixel_height)
}

fn rounded_size(lisp: &mut Lisp, args: &[Value], horizontal: bool) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    let round = arg(args, 1);
    let win = lisp.window(w);
    let (total, pixels) = if horizontal {
        (win.total_cols, win.pixel_width)
    } else {
        (win.total_lines, win.pixel_height)
    };
    if round != sym::FLOOR && round != sym::CEILING {
        return Ok(Value::fixnum(total));
    }
    let g = &lisp.frame(win.frame).geometry;
    let unit = if horizontal {
        g.column_width
    } else {
        g.line_height
    };
    Ok(Value::fixnum(if round == sym::CEILING {
        (pixels + unit - 1) / unit
    } else {
        pixels / unit
    }))
}

fn f_window_total_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    rounded_size(lisp, args, false)
}

fn f_window_total_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    rounded_size(lisp, args, true)
}

fn f_window_new_total(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_field(lisp, args, |w| w.new_total)
}

fn f_window_normal_size(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    let win = lisp.window(w);
    Ok(if arg(args, 1).is_nil() {
        win.normal_lines
    } else {
        win.normal_cols
    })
}

fn f_window_new_normal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_field(lisp, args, |w| w.new_normal)
}

fn f_window_new_pixel(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_field(lisp, args, |w| w.new_pixel)
}

fn f_window_pixel_left(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_int(lisp, args, |w| w.pixel_left)
}

fn f_window_pixel_top(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_int(lisp, args, |w| w.pixel_top)
}

fn f_window_left_column(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_int(lisp, args, |w| w.left_col)
}

fn f_window_top_line(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    valid_int(lisp, args, |w| w.top_line)
}

fn f_window_hscroll(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window(w).hscroll))
}

fn f_window_point(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window_point(w)))
}

fn f_window_old_point(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    let m = lisp.window(w).old_pointm;
    Ok(lisp.marker_position_value(m).unwrap_or(Value::NIL))
}

fn f_window_start(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    let m = lisp.window(w).start;
    Ok(lisp.marker_position_value(m).unwrap_or(Value::NIL))
}

fn f_window_dedicated_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(lisp.window(w).dedicated)
}

fn f_set_window_dedicated_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(args[0])?;
    lisp.window_mut(w).dedicated = args[1];
    Ok(args[1])
}

fn f_window_prev_buffers(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(lisp.window(w).prev_buffers)
}

fn f_set_window_prev_buffers(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(args[0])?;
    lisp.window_mut(w).prev_buffers = args[1];
    Ok(args[1])
}

fn f_window_next_buffers(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(lisp.window(w).next_buffers)
}

fn f_set_window_next_buffers(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(args[0])?;
    lisp.window_mut(w).next_buffers = args[1];
    Ok(args[1])
}

fn f_window_parameters(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_valid_window(arg(args, 0))?;
    let params = lisp.window(w).window_parameters;
    crate::fns::f_copy_alist(lisp, &[params])
}

fn f_window_parameter(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_any_window(args[0])?;
    Ok(lisp.window_parameter(w, args[1]))
}

fn f_set_window_parameter(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (window, parameter, value) = (args[0], args[1], args[2]);
    let w = lisp.decode_any_window(window)?;
    let params = lisp.window(w).window_parameters;
    let old = lisp.assq(parameter, params);
    let f = lisp.window(w).frame;
    if parameter.is_symbol()
        && lisp.window_live_p(window)
        && lisp.get(parameter, sym::K_FILTERED) == sym::T
        && lisp.frame(f).window_p()
        && lisp.cdr_safe(old) != value
        && lisp.var_true(sym::WINDOW_AUTO_REDRAW_ON_PARAMETER_CHANGE)
    {
        lisp.frame_mut(f).flags.face_change = true;
        lisp.fset_redisplay(f);
    }
    if old.is_nil() {
        let cell = lisp.cons(parameter, value);
        let params = lisp.cons(cell, params);
        lisp.window_mut(w).window_parameters = params;
    } else {
        lisp.heap.set_cdr(old, value);
    }
    Ok(value)
}

fn f_window_display_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(lisp.window(w).display_table)
}

fn f_set_window_display_table(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(args[0])?;
    lisp.window_mut(w).display_table = args[1];
    Ok(args[1])
}

fn f_window_margins(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    let win = lisp.window(w);
    let left = if win.left_margin_cols != 0 {
        Value::fixnum(win.left_margin_cols)
    } else {
        Value::NIL
    };
    let right = if win.right_margin_cols != 0 {
        Value::fixnum(win.right_margin_cols)
    } else {
        Value::NIL
    };
    Ok(lisp.cons(left, right))
}
