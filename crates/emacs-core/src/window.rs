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
    pub cursor_off_p: bool,
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
    pub last_modified: i64,
    pub last_overlay_modified: i64,
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

#[derive(Clone, Debug)]
pub struct SaveWindowData {
    pub selected_frame: Value,
    pub current_window: Value,
    pub f_current_buffer: Value,
    pub minibuf_scroll_window: Value,
    pub minibuf_selected_window: Value,
    pub root_window: Value,
    pub focus_frame: Value,
    pub saved_windows: Value,
    pub change_stamp: i64,
    pub frame_cols: i64,
    pub frame_lines: i64,
    pub frame_menu_bar_lines: i64,
    pub frame_tab_bar_lines: i64,
    pub frame_tool_bar_lines: i64,
    pub frame_text_width: i64,
    pub frame_text_height: i64,
    pub frame_menu_bar_height: i64,
    pub frame_tab_bar_height: i64,
    pub frame_tool_bar_height: i64,
}

impl SaveWindowData {
    pub fn trace(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.selected_frame,
            self.current_window,
            self.f_current_buffer,
            self.minibuf_scroll_window,
            self.minibuf_selected_window,
            self.root_window,
            self.focus_frame,
            self.saved_windows,
        ]);
    }
}

const SW_WINDOW: usize = 0;
const SW_BUFFER: usize = 1;
const SW_OLD_BUFFER: usize = 2;
const SW_START: usize = 3;
const SW_POINTM: usize = 4;
const SW_OLD_POINTM: usize = 5;
const SW_PIXEL_LEFT: usize = 6;
const SW_PIXEL_TOP: usize = 7;
const SW_PIXEL_HEIGHT: usize = 8;
const SW_PIXEL_WIDTH: usize = 9;
const SW_LEFT_COL: usize = 10;
const SW_TOP_LINE: usize = 11;
const SW_TOTAL_COLS: usize = 12;
const SW_TOTAL_LINES: usize = 13;
const SW_NORMAL_COLS: usize = 14;
const SW_NORMAL_LINES: usize = 15;
const SW_HSCROLL: usize = 16;
const SW_MIN_HSCROLL: usize = 17;
const SW_HSCROLL_WHOLE: usize = 18;
const SW_SUSPEND_AUTO_HSCROLL: usize = 19;
const SW_VSCROLL: usize = 20;
const SW_PARENT: usize = 21;
const SW_PREV: usize = 22;
const SW_START_AT_LINE_BEG: usize = 23;
const SW_DISPLAY_TABLE: usize = 24;
const SW_LEFT_MARGIN_COLS: usize = 25;
const SW_RIGHT_MARGIN_COLS: usize = 26;
const SW_LEFT_FRINGE_WIDTH: usize = 27;
const SW_RIGHT_FRINGE_WIDTH: usize = 28;
const SW_FRINGES_OUTSIDE_MARGINS: usize = 29;
const SW_FRINGES_PERSISTENT: usize = 30;
const SW_SCROLL_BAR_WIDTH: usize = 31;
const SW_VERTICAL_SCROLL_BAR_TYPE: usize = 32;
const SW_SCROLL_BAR_HEIGHT: usize = 33;
const SW_HORIZONTAL_SCROLL_BAR_TYPE: usize = 34;
const SW_SCROLL_BARS_PERSISTENT: usize = 35;
const SW_DEDICATED: usize = 36;
const SW_COMBINATION_LIMIT: usize = 37;
const SW_WINDOW_PARAMETERS: usize = 38;
const SAVED_WINDOW_SIZE: usize = 39;

struct RestoreContext<'a> {
    data: &'a SaveWindowData,
    saved: &'a [Value],
    frame: Value,
    new_current_buffer: Value,
    dont_set_miniwindow: Value,
    kept_index: usize,
    dead_index: usize,
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
    Subr::fixed("next-window", 0, 3, f_next_window),
    Subr::fixed("previous-window", 0, 3, f_previous_window),
    Subr::fixed("window-list", 0, 3, f_window_list),
    Subr::fixed("window-list-1", 0, 3, f_window_list_1),
    Subr::fixed("get-buffer-window", 0, 2, f_get_buffer_window),
    Subr::fixed("set-window-buffer", 2, 3, f_set_window_buffer),
    Subr::fixed("window-fringes", 0, 1, f_window_fringes),
    Subr::fixed("window-configuration-p", 1, 1, f_window_configuration_p),
    Subr::fixed(
        "window-configuration-frame",
        1,
        1,
        f_window_configuration_frame,
    ),
    Subr::fixed("set-window-configuration", 1, 3, f_set_window_configuration),
    Subr::fixed(
        "current-window-configuration",
        0,
        1,
        f_current_window_configuration,
    ),
    Subr::fixed(
        "window-configuration-equal-p",
        2,
        2,
        f_window_configuration_equal_p,
    ),
];

fn f_set_window_buffer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (window, buffer_or_name, keep_margins) = (args[0], args[1], args[2]);
    let window = lisp.decode_live_window(window)?;
    let buffer = crate::buffer::f_get_buffer(lisp, &[buffer_or_name])?;
    lisp.check_buffer(buffer)?;
    if !lisp.buffer_live_p(buffer) {
        return Err(lisp.error("Attempt to display deleted buffer"));
    }
    let tem = lisp.window(window).contents;
    if tem.is_nil() {
        return Err(lisp.error("Window is deleted"));
    }
    if tem != buffer {
        if lisp.window(window).dedicated == sym::T {
            let name = lisp.buffer(tem).name;
            let name = lisp.string_to_rust(name);
            return Err(lisp.error(&format!("Window is dedicated to `{name}'")));
        }
        lisp.window_mut(window).dedicated = Value::NIL;
        lisp.funcall(sym::RECORD_WINDOW_BUFFER, &[window])?;
    }
    lisp.unshow_buffer(window)?;
    lisp.set_window_buffer(window, buffer, true, !keep_margins.is_nil())?;
    Ok(Value::NIL)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WindowLoop {
    GetBufferWindow,
    ReplaceBufferInWindowsSafely,
}

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

    fn collect_windows(&self, w: Value, out: &mut Vec<Value>) {
        let mut w = w;
        loop {
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                self.collect_windows(contents, out);
            } else {
                out.push(w);
            }
            let next = self.window(w).next;
            if next.is_nil() {
                break;
            }
            w = next;
        }
    }

    pub fn foreach_window_list(&self, f: Value) -> Vec<Value> {
        let mut out = Vec::new();
        let root = self.frame(f).root_window;
        if self.is_window(root) {
            self.collect_windows(root, &mut out);
        }
        out
    }

    pub fn window_list(&mut self) -> LispResult {
        if !self.windows.window_list.is_cons() {
            let count = self.specpdl.len();
            self.windows.window_list = Value::NIL;
            self.specbind(sym::INHIBIT_QUIT, sym::T)?;
            let mut all = Vec::new();
            for frame in self.frame_list_vec() {
                all.extend(self.foreach_window_list(frame));
            }
            self.windows.window_list = self.list(&all);
            self.unbind_to(count)?;
        }
        Ok(self.windows.window_list)
    }

    fn candidate_window_p(
        &self,
        window: Value,
        owindow: Value,
        minibuf: Value,
        all_frames: Value,
    ) -> bool {
        let w = self.window(window);
        let f = self.frame(w.frame);
        let same_terminal =
            || self.frame(w.frame).terminal == self.frame(self.frames.selected_frame).terminal;
        let excluded_mini = w.mini
            && (minibuf == sym::LAMBDA || (self.window_live_p(minibuf) && minibuf != window));
        if !self.is_buffer(w.contents) || excluded_mini {
            false
        } else if all_frames == sym::T {
            true
        } else if all_frames.is_nil() {
            w.frame == self.window(owindow).frame
        } else if all_frames == sym::VISIBLE {
            f.flags.visible && same_terminal()
        } else if all_frames == Value::fixnum(0) {
            (f.flags.visible || f.flags.iconified) && same_terminal()
        } else if self.is_window(all_frames) {
            let aw = self.window(all_frames);
            (aw.frame == w.frame
                || (f.minibuffer_window == all_frames && aw.frame == f.focus_frame))
                && (minibuf == sym::T || !self.is_minibuffer(0, aw.contents))
        } else if self.is_frame(all_frames) {
            all_frames == w.frame
        } else {
            true
        }
    }

    fn decode_next_window_args(
        &mut self,
        window: Value,
        minibuf: Value,
        all_frames: Value,
    ) -> LispResult<(Value, Value, Value)> {
        let window = self.decode_live_window(window)?;
        let miniwin = self.frame(self.window(window).frame).minibuffer_window;
        let mut minibuf = minibuf;
        if self.window_live_p(miniwin) && minibuf.is_nil() {
            let contents = self.window(miniwin).contents;
            minibuf = if self.this_minibuffer_depth(contents) != 0 {
                miniwin
            } else {
                sym::LAMBDA
            };
        } else if minibuf != sym::T {
            minibuf = sym::LAMBDA;
        }
        let mut all_frames = all_frames;
        if all_frames.is_nil() {
            all_frames = if self.window_live_p(miniwin) && minibuf != sym::LAMBDA {
                miniwin
            } else {
                Value::NIL
            };
        } else if all_frames == sym::VISIBLE
            || all_frames == Value::fixnum(0)
            || self.is_frame(all_frames)
        {
        } else if all_frames != sym::T {
            all_frames = Value::NIL;
        }
        Ok((window, minibuf, all_frames))
    }

    fn next_window(
        &mut self,
        window: Value,
        minibuf: Value,
        all_frames: Value,
        next_p: bool,
    ) -> LispResult {
        let count = self.specpdl.len();
        let (mut window, minibuf, all_frames) =
            self.decode_next_window_args(window, minibuf, all_frames)?;
        if self.is_frame(all_frames) && all_frames != self.window(window).frame {
            return f_frame_first_window(self, &[all_frames]);
        }
        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
        let all = self.window_list()?;
        if next_p {
            let mut list = self.memq(window, all);
            if list.is_cons() {
                list = self.xcdr(list);
                while list.is_cons() {
                    if self.candidate_window_p(self.xcar(list), window, minibuf, all_frames) {
                        break;
                    }
                    list = self.xcdr(list);
                }
            }
            if !list.is_cons() {
                list = self.windows.window_list;
                while list.is_cons() && self.xcar(list) != window {
                    if self.candidate_window_p(self.xcar(list), window, minibuf, all_frames) {
                        break;
                    }
                    list = self.xcdr(list);
                }
            }
            if list.is_cons() {
                window = self.xcar(list);
            }
        } else {
            let mut candidate = Value::NIL;
            let mut list = all;
            while list.is_cons() {
                let w = self.xcar(list);
                if w == window {
                    if self.is_window(candidate) {
                        break;
                    }
                } else if self.candidate_window_p(w, window, minibuf, all_frames) {
                    candidate = w;
                }
                list = self.xcdr(list);
            }
            if self.is_window(candidate) {
                window = candidate;
            }
        }
        self.unbind_to(count)?;
        Ok(window)
    }

    pub fn window_list_1(
        &mut self,
        window: Value,
        minibuf: Value,
        all_frames: Value,
    ) -> LispResult {
        let count = self.specpdl.len();
        let (window, minibuf, all_frames) =
            self.decode_next_window_args(window, minibuf, all_frames)?;
        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
        let all = self.window_list()?;
        let mut items = Vec::new();
        let mut tail = all;
        while tail.is_cons() {
            let w = self.xcar(tail);
            if self.candidate_window_p(w, window, minibuf, all_frames) {
                items.push(w);
            }
            tail = self.xcdr(tail);
        }
        if let Some(pos) = items.iter().position(|&w| w == window) {
            items.rotate_left(pos);
        }
        let list = self.list(&items);
        self.unbind_to(count)?;
        Ok(list)
    }

    pub fn window_loop(
        &mut self,
        kind: WindowLoop,
        obj: Value,
        mini: bool,
        frames: Value,
    ) -> LispResult {
        let f = if self.is_frame(frames) {
            Some(frames)
        } else if frames.is_nil() {
            Some(self.frames.selected_frame)
        } else {
            None
        };
        let frame_arg = if f.is_some() {
            sym::LAMBDA
        } else if frames == Value::fixnum(0) || frames == sym::VISIBLE {
            frames
        } else {
            sym::T
        };
        let window = if self.is_window(obj) {
            obj
        } else {
            let frame = f.unwrap_or(self.frames.selected_frame);
            self.frame(frame).selected_window
        };
        let windows =
            self.window_list_1(window, if mini { sym::T } else { Value::NIL }, frame_arg)?;
        let base = self.stack.len();
        self.stack.push(windows);
        let r = self.window_loop_1(kind, obj, mini, windows);
        self.stack.truncate(base);
        r
    }

    fn window_loop_1(
        &mut self,
        kind: WindowLoop,
        obj: Value,
        mini: bool,
        windows: Value,
    ) -> LispResult {
        let mut best_window = Value::NIL;
        let mut frame_best_window_flag = false;
        let mut windows = windows;
        while windows.is_cons() {
            let window = self.xcar(windows);
            windows = self.xcdr(windows);
            let is_mini = self.window(window).mini;
            if !(!is_mini
                || kind == WindowLoop::ReplaceBufferInWindowsSafely
                || (mini && self.minibuf.minibuf_level > 0))
            {
                continue;
            }
            match kind {
                WindowLoop::GetBufferWindow => {
                    let w = self.window(window);
                    if w.contents == obj && (!w.mini || window == self.windows.minibuf_window) {
                        if window == self.windows.selected_window {
                            return Ok(window);
                        } else if w.frame == self.frames.selected_frame && !frame_best_window_flag {
                            best_window = window;
                            frame_best_window_flag = true;
                        } else if best_window.is_nil() {
                            best_window = window;
                        }
                    }
                }
                WindowLoop::ReplaceBufferInWindowsSafely => {
                    if self.window(window).contents == obj {
                        self.window_mut(window).dedicated = Value::NIL;
                        let contents = self.window(window).contents;
                        let other = self.other_buffer_safely(contents);
                        self.set_window_buffer(window, other, false, false)?;
                        let contents = self.window(window).contents;
                        if window == self.windows.selected_window && contents == self.current_buffer
                        {
                            crate::buffer::f_set_buffer(self, &[contents])?;
                        }
                    }
                }
            }
        }
        Ok(best_window)
    }

    pub fn replace_buffer_in_windows(&mut self, buffer: Value) -> LispResult<()> {
        if !self
            .heap
            .symbol_ref(sym::REPLACE_BUFFER_IN_WINDOWS)
            .function
            .is_nil()
        {
            self.funcall(sym::REPLACE_BUFFER_IN_WINDOWS, &[buffer])?;
        }
        Ok(())
    }

    pub fn replace_buffer_in_windows_safely(&mut self, buffer: Value) -> LispResult<()> {
        if self.buffer(buffer).window_count > 0 {
            for frame in self.frame_list_vec() {
                self.window_loop(
                    WindowLoop::ReplaceBufferInWindowsSafely,
                    buffer,
                    true,
                    frame,
                )?;
            }
        }
        Ok(())
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
            last_modified: 0,
            last_overlay_modified: 0,
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
        self.wset_update_mode_line(window);
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

    fn unshow_buffer(&mut self, w: Value) -> LispResult<()> {
        let buf = self.window(w).contents;
        let start = self.window(w).start;
        let last_window_start = self.marker_position_checked(start)?;
        self.buffer_mut(buf).last_window_start = last_window_start;
        let selected = self.windows.selected_window;
        let lsw = self.buffer(buf).last_selected_window;
        let lsw_is_window = self.is_window(lsw);
        if buf != self.window(selected).contents
            && !(lsw_is_window && w != lsw && buf == self.window(lsw).contents)
        {
            let pointm = self.window(w).pointm;
            let charpos = self.marker_position_checked(pointm)?;
            let bytepos = self.marker_byte_position_checked(pointm)?;
            let b = self.buffer_mut(buf);
            b.pt = charpos.clamp(b.begv, b.zv);
            b.pt_byte = bytepos.clamp(b.begv_byte, b.zv_byte);
        }
        if lsw_is_window && w == lsw {
            self.buffer_mut(buf).last_selected_window = Value::NIL;
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

fn f_next_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.next_window(arg(args, 0), arg(args, 1), arg(args, 2), true)
}

fn f_previous_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.next_window(arg(args, 0), arg(args, 1), arg(args, 2), false)
}

fn f_window_list(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (mut frame, minibuf, mut window) = (arg(args, 0), arg(args, 1), arg(args, 2));
    if window.is_nil() {
        window = if lisp.is_frame(frame) {
            lisp.frame(frame).selected_window
        } else {
            lisp.windows.selected_window
        };
    }
    if !lisp.is_window(window) {
        return Err(lisp.wrong_type(sym::WINDOWP, window));
    }
    if frame.is_nil() {
        frame = lisp.frames.selected_frame;
    }
    if frame != lisp.window(window).frame {
        return Err(lisp.error("Window is on a different frame"));
    }
    lisp.window_list_1(window, minibuf, frame)
}

fn f_window_list_1(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.window_list_1(arg(args, 0), arg(args, 1), arg(args, 2))
}

pub(crate) fn f_get_buffer_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (buffer_or_name, all_frames) = (arg(args, 0), arg(args, 1));
    let buffer = if buffer_or_name.is_nil() {
        lisp.current_buffer
    } else {
        crate::buffer::f_get_buffer(lisp, &[buffer_or_name])?
    };
    if lisp.is_buffer(buffer) {
        lisp.window_loop(WindowLoop::GetBufferWindow, buffer, true, all_frames)
    } else {
        Ok(Value::NIL)
    }
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

impl Lisp {
    pub fn is_window_configuration(&self, v: Value) -> bool {
        v.is_vectorlike()
            && matches!(
                self.heap.vectorlike_ref(v),
                Vectorlike::WindowConfiguration(_)
            )
    }

    fn check_window_configuration(&mut self, v: Value) -> LispResult<SaveWindowData> {
        if !self.is_window_configuration(v) {
            return Err(self.wrong_type(sym::WINDOW_CONFIGURATION_P, v));
        }
        match self.heap.vectorlike_ref(v) {
            Vectorlike::WindowConfiguration(data) => Ok(data.as_ref().clone()),
            _ => unreachable!("not a window configuration"),
        }
    }

    fn sw(&self, saved_window: Value, slot: usize) -> Value {
        self.vector_items(saved_window)[slot]
    }

    fn sw_set(&mut self, saved_window: Value, slot: usize, value: Value) {
        if let Vectorlike::Vector(items) = self.heap.vectorlike_mut(saved_window) {
            items[slot] = value;
        }
    }

    fn wset_combination(&mut self, w: Value, horflag: bool, val: Value) {
        let win = self.window_mut(w);
        win.contents = val;
        if !val.is_nil() {
            win.horizontal = horflag;
        }
    }

    fn count_windows(&self, window: Value) -> usize {
        let w = self.window(window);
        let mut count = 1;
        if !w.next.is_nil() {
            count += self.count_windows(w.next);
        }
        if self.is_window(w.contents) {
            count += self.count_windows(w.contents);
        }
        count
    }

    pub(crate) fn delete_all_child_windows(&mut self, window: Value) -> LispResult<()> {
        let next = self.window(window).next;
        if !next.is_nil() {
            self.delete_all_child_windows(next)?;
        }
        let contents = self.window(window).contents;
        if self.is_window(contents) {
            self.delete_all_child_windows(contents)?;
            self.wset_combination(window, false, Value::NIL);
        } else if self.is_buffer(contents) {
            self.window_mut(window).old_buffer = contents;
            self.unshow_buffer(window)?;
            let (pointm, old_pointm, start) = {
                let w = self.window(window);
                (w.pointm, w.old_pointm, w.start)
            };
            self.unchain_marker(pointm);
            self.unchain_marker(old_pointm);
            self.unchain_marker(start);
            self.window_mut(window).combination_limit = contents;
            self.wset_buffer(window, Value::NIL);
            let seq = self.window(window).sequence_number;
            let table = self.plain_value(sym::WINDOW_DEAD_WINDOWS_TABLE);
            self.puthash(Value::fixnum(seq), window, table)?;
        }
        self.windows.window_list = Value::NIL;
        Ok(())
    }

    pub(crate) fn window_discard_buffer_from_dead_windows(
        &mut self,
        buffer: Value,
    ) -> LispResult<()> {
        let table = self.plain_value(sym::WINDOW_DEAD_WINDOWS_TABLE);
        let windows: Vec<Value> = self
            .hash_table(table)
            .entries
            .iter()
            .flatten()
            .map(|&(_, v)| v)
            .collect();
        let base = self.stack.len();
        self.stack.extend_from_slice(&windows);
        let mut r = Ok(());
        for window in windows {
            r = self.window_discard_buffer_from_window_all(buffer, window);
            if r.is_err() {
                break;
            }
        }
        self.stack.truncate(base);
        r
    }

    fn window_discard_buffer_from_window_all(
        &mut self,
        buffer: Value,
        window: Value,
    ) -> LispResult<()> {
        self.window_discard_buffer_from_window(buffer, window);
        let quit_restore = self.window_parameter(window, sym::QUIT_RESTORE);
        let quit_restore_prev = self.window_parameter(window, sym::QUIT_RESTORE_PREV);
        if self.quit_restore_mentions(buffer, quit_restore_prev)? {
            f_set_window_parameter(self, &[window, sym::QUIT_RESTORE_PREV, Value::NIL])?;
        }
        if self.quit_restore_mentions(buffer, quit_restore)? {
            let prev = self.window_parameter(window, sym::QUIT_RESTORE_PREV);
            f_set_window_parameter(self, &[window, sym::QUIT_RESTORE, prev])?;
            f_set_window_parameter(self, &[window, sym::QUIT_RESTORE_PREV, Value::NIL])?;
        }
        Ok(())
    }

    fn quit_restore_mentions(&mut self, buffer: Value, quit_restore: Value) -> LispResult<bool> {
        let third = crate::fns::f_nth(self, &[Value::fixnum(3), quit_restore])?;
        if third == buffer {
            return Ok(true);
        }
        let rest = self.cdr(quit_restore)?;
        let quint = self.car(rest)?;
        Ok(quint.is_cons() && self.car(quint)? == buffer)
    }

    fn buffer_local_value(&self, symbol: Value, buffer: Value) -> Value {
        self.buffer_local_get(buffer, symbol)
            .unwrap_or_else(|| self.default_value_unchecked(symbol))
    }

    fn save_window_save(
        &mut self,
        mut window: Value,
        vector: Value,
        mut i: usize,
    ) -> LispResult<usize> {
        while !window.is_nil() {
            let p = self.vector_items(vector)[i];
            self.window_mut(window).temslot = Value::fixnum(i as i64);
            i += 1;
            let w = self.window(window).clone();
            let leaf = self.is_buffer(w.contents);
            let fields = [
                (SW_WINDOW, window),
                (SW_BUFFER, if leaf { w.contents } else { Value::NIL }),
                (SW_OLD_BUFFER, w.old_buffer),
                (SW_PIXEL_LEFT, Value::fixnum(w.pixel_left)),
                (SW_PIXEL_TOP, Value::fixnum(w.pixel_top)),
                (SW_PIXEL_WIDTH, Value::fixnum(w.pixel_width)),
                (SW_PIXEL_HEIGHT, Value::fixnum(w.pixel_height)),
                (SW_LEFT_COL, Value::fixnum(w.left_col)),
                (SW_TOP_LINE, Value::fixnum(w.top_line)),
                (SW_TOTAL_COLS, Value::fixnum(w.total_cols)),
                (SW_TOTAL_LINES, Value::fixnum(w.total_lines)),
                (SW_NORMAL_COLS, w.normal_cols),
                (SW_NORMAL_LINES, w.normal_lines),
                (SW_HSCROLL, Value::fixnum(w.hscroll)),
                (
                    SW_SUSPEND_AUTO_HSCROLL,
                    self.bool(w.flags.suspend_auto_hscroll),
                ),
                (SW_MIN_HSCROLL, Value::fixnum(w.min_hscroll)),
                (SW_HSCROLL_WHOLE, Value::fixnum(w.hscroll_whole)),
                (SW_VSCROLL, Value::fixnum(-w.vscroll)),
                (SW_DISPLAY_TABLE, w.display_table),
                (SW_LEFT_MARGIN_COLS, Value::fixnum(w.left_margin_cols)),
                (SW_RIGHT_MARGIN_COLS, Value::fixnum(w.right_margin_cols)),
                (SW_LEFT_FRINGE_WIDTH, Value::fixnum(w.left_fringe_width)),
                (SW_RIGHT_FRINGE_WIDTH, Value::fixnum(w.right_fringe_width)),
                (
                    SW_FRINGES_OUTSIDE_MARGINS,
                    self.bool(w.flags.fringes_outside_margins),
                ),
                (SW_FRINGES_PERSISTENT, self.bool(w.flags.fringes_persistent)),
                (SW_SCROLL_BAR_WIDTH, Value::fixnum(w.scroll_bar_width)),
                (SW_SCROLL_BAR_HEIGHT, Value::fixnum(w.scroll_bar_height)),
                (
                    SW_SCROLL_BARS_PERSISTENT,
                    self.bool(w.flags.scroll_bars_persistent),
                ),
                (SW_VERTICAL_SCROLL_BAR_TYPE, w.vertical_scroll_bar_type),
                (SW_HORIZONTAL_SCROLL_BAR_TYPE, w.horizontal_scroll_bar_type),
                (SW_DEDICATED, w.dedicated),
                (SW_COMBINATION_LIMIT, w.combination_limit),
                (SW_WINDOW_PARAMETERS, Value::NIL),
            ];
            for (slot, value) in fields {
                self.sw_set(p, slot, value);
            }
            let persistent = self.plain_value(sym::WINDOW_PERSISTENT_PARAMETERS);
            if !persistent.is_nil() {
                let mut hare = persistent;
                let mut tortoise = persistent;
                while hare.is_cons() {
                    hare = self.xcdr(hare);
                    if !hare.is_cons() {
                        break;
                    }
                    hare = self.xcdr(hare);
                    tortoise = self.xcdr(tortoise);
                    if hare == tortoise {
                        self.set_plain_value(sym::WINDOW_PERSISTENT_PARAMETERS, Value::NIL);
                        break;
                    }
                }
                let mut tem = self.plain_value(sym::WINDOW_PERSISTENT_PARAMETERS);
                while tem.is_cons() {
                    let pers = self.xcar(tem);
                    if pers.is_cons() && !self.xcdr(pers).is_nil() {
                        let key = self.xcar(pers);
                        let par = self.assq(key, self.window(window).window_parameters);
                        let entry = if par.is_nil() {
                            self.cons(key, Value::NIL)
                        } else {
                            let (k, v) = (self.xcar(par), self.xcdr(par));
                            self.cons(k, v)
                        };
                        let params = self.sw(p, SW_WINDOW_PARAMETERS);
                        let params = self.cons(entry, params);
                        self.sw_set(p, SW_WINDOW_PARAMETERS, params);
                    }
                    tem = self.xcdr(tem);
                }
            }
            if leaf {
                let insertion_type = !self
                    .buffer_local_value(sym::WINDOW_POINT_INSERTION_TYPE, w.contents)
                    .is_nil();
                let pointm = if window == self.windows.selected_window {
                    let pt = self.buffer(w.contents).pt;
                    self.make_marker_at(pt, w.contents, false)
                } else {
                    crate::marker::f_copy_marker(self, &[w.pointm, Value::NIL])?
                };
                self.sw_set(p, SW_POINTM, pointm);
                let old_pointm = crate::marker::f_copy_marker(self, &[w.old_pointm, Value::NIL])?;
                self.sw_set(p, SW_OLD_POINTM, old_pointm);
                self.marker_mut(pointm).insertion_type = insertion_type;
                self.marker_mut(old_pointm).insertion_type = insertion_type;
                let start = crate::marker::f_copy_marker(self, &[w.start, Value::NIL])?;
                self.sw_set(p, SW_START, start);
                let beg = self.bool(w.flags.start_at_line_beg);
                self.sw_set(p, SW_START_AT_LINE_BEG, beg);
            } else {
                for slot in [SW_POINTM, SW_OLD_POINTM, SW_START, SW_START_AT_LINE_BEG] {
                    self.sw_set(p, slot, Value::NIL);
                }
            }
            let parent = if w.parent.is_nil() {
                Value::NIL
            } else {
                self.window(w.parent).temslot
            };
            self.sw_set(p, SW_PARENT, parent);
            let prev = if w.prev.is_nil() {
                Value::NIL
            } else {
                self.window(w.prev).temslot
            };
            self.sw_set(p, SW_PREV, prev);
            if self.is_window(w.contents) {
                i = self.save_window_save(w.contents, vector, i)?;
            }
            window = w.next;
        }
        Ok(i)
    }

    fn push_restore_list(&mut self, index: usize, item: Value) {
        let list = self.stack[index];
        self.stack[index] = self.cons(item, list);
    }

    fn restore_saved_window(&mut self, cx: &RestoreContext, p: Value) -> LispResult<()> {
        let (data, saved, frame) = (cx.data, cx.saved, cx.frame);
        let (new_current_buffer, dont_set_miniwindow) =
            (cx.new_current_buffer, cx.dont_set_miniwindow);
        let window = self.sw(p, SW_WINDOW);
        self.window_mut(window).next = Value::NIL;
        let parent = self.sw(p, SW_PARENT);
        let parent = match parent.as_fixnum() {
            Some(n) => self.sw(saved[n as usize], SW_WINDOW),
            None => Value::NIL,
        };
        self.window_mut(window).parent = parent;
        match self.sw(p, SW_PREV).as_fixnum() {
            Some(n) => {
                let prev = self.sw(saved[n as usize], SW_WINDOW);
                self.window_mut(window).prev = prev;
                self.window_mut(prev).next = window;
            }
            None => {
                self.window_mut(window).prev = Value::NIL;
                if !parent.is_nil() {
                    let horflag = self.sw(p, SW_TOTAL_COLS).as_fixnum()
                        != Some(self.window(parent).total_cols);
                    self.wset_combination(parent, horflag, window);
                }
            }
        }
        let squirreled = self.window(window).combination_limit;
        if self.is_buffer(squirreled) {
            self.wset_buffer(window, squirreled);
        }
        if data.change_stamp == self.frame(frame).change_stamp {
            let old_buffer = self.sw(p, SW_OLD_BUFFER);
            self.window_mut(window).old_buffer = old_buffer;
        }
        let int = |lisp: &Lisp, slot: usize| lisp.sw(p, slot).as_fixnum().unwrap_or(0);
        let flag = |lisp: &Lisp, slot: usize| !lisp.sw(p, slot).is_nil();
        let ints = [
            int(self, SW_PIXEL_LEFT),
            int(self, SW_PIXEL_TOP),
            int(self, SW_PIXEL_WIDTH),
            int(self, SW_PIXEL_HEIGHT),
            int(self, SW_LEFT_COL),
            int(self, SW_TOP_LINE),
            int(self, SW_TOTAL_COLS),
            int(self, SW_TOTAL_LINES),
            int(self, SW_HSCROLL),
            int(self, SW_MIN_HSCROLL),
            int(self, SW_HSCROLL_WHOLE),
            int(self, SW_VSCROLL),
            int(self, SW_LEFT_MARGIN_COLS),
            int(self, SW_RIGHT_MARGIN_COLS),
            int(self, SW_LEFT_FRINGE_WIDTH),
            int(self, SW_RIGHT_FRINGE_WIDTH),
            int(self, SW_SCROLL_BAR_WIDTH),
            int(self, SW_SCROLL_BAR_HEIGHT),
        ];
        let flags = [
            flag(self, SW_SUSPEND_AUTO_HSCROLL),
            flag(self, SW_FRINGES_OUTSIDE_MARGINS),
            flag(self, SW_FRINGES_PERSISTENT),
            flag(self, SW_SCROLL_BARS_PERSISTENT),
        ];
        let values = [
            self.sw(p, SW_NORMAL_COLS),
            self.sw(p, SW_NORMAL_LINES),
            self.sw(p, SW_DISPLAY_TABLE),
            self.sw(p, SW_VERTICAL_SCROLL_BAR_TYPE),
            self.sw(p, SW_HORIZONTAL_SCROLL_BAR_TYPE),
            self.sw(p, SW_DEDICATED),
            self.sw(p, SW_COMBINATION_LIMIT),
        ];
        {
            let w = self.window_mut(window);
            w.pixel_left = ints[0];
            w.pixel_top = ints[1];
            w.pixel_width = ints[2];
            w.pixel_height = ints[3];
            w.left_col = ints[4];
            w.top_line = ints[5];
            w.total_cols = ints[6];
            w.total_lines = ints[7];
            w.normal_cols = values[0];
            w.normal_lines = values[1];
            w.hscroll = ints[8];
            w.flags.suspend_auto_hscroll = flags[0];
            w.min_hscroll = ints[9];
            w.hscroll_whole = ints[10];
            w.vscroll = -ints[11];
            w.display_table = values[2];
            w.left_margin_cols = ints[12];
            w.right_margin_cols = ints[13];
            w.left_fringe_width = ints[14];
            w.right_fringe_width = ints[15];
            w.flags.fringes_outside_margins = flags[1];
            w.flags.fringes_persistent = flags[2];
            w.scroll_bar_width = ints[16];
            w.scroll_bar_height = ints[17];
            w.flags.scroll_bars_persistent = flags[3];
            w.vertical_scroll_bar_type = values[3];
            w.horizontal_scroll_bar_type = values[4];
            w.dedicated = values[5];
            w.combination_limit = values[6];
        }
        let mut tem = self.sw(p, SW_WINDOW_PARAMETERS);
        while tem.is_cons() {
            let pers = self.xcar(tem);
            if pers.is_cons() {
                let (key, value) = (self.xcar(pers), self.xcdr(pers));
                if value.is_nil() {
                    let par = self.assq(key, self.window(window).window_parameters);
                    if par.is_cons() && !self.xcdr(par).is_nil() {
                        self.heap.set_cdr(par, Value::NIL);
                    }
                } else {
                    f_set_window_parameter(self, &[window, key, value])?;
                }
            }
            tem = self.xcdr(tem);
        }
        let seq = self.window(window).sequence_number;
        let table = self.plain_value(sym::WINDOW_DEAD_WINDOWS_TABLE);
        crate::fns::f_remhash(self, &[Value::fixnum(seq), table])?;
        let pbuf = self.sw(p, SW_BUFFER);
        let mini = self.window(window).mini;
        let restore_killed = self.plain_value(sym::WINDOW_RESTORE_KILLED_BUFFER_WINDOWS);
        let old_contents = self.window(window).contents;
        let old_contents_live = self.is_buffer(old_contents) && self.buffer_live_p(old_contents);
        if (dont_set_miniwindow.is_nil() || !mini)
            && self.is_buffer(pbuf)
            && self.buffer_live_p(pbuf)
        {
            if self.window(window).contents != pbuf {
                self.wset_buffer(window, pbuf);
                self.window_discard_buffer_from_window(pbuf, window);
            }
            self.window_mut(window).flags.start_at_line_beg =
                !self.sw(p, SW_START_AT_LINE_BEG).is_nil();
            let (start, pointm, old_pointm) = {
                let w = self.window(window);
                (w.start, w.pointm, w.old_pointm)
            };
            let contents = self.window(window).contents;
            let (s, pm, opm) = (
                self.sw(p, SW_START),
                self.sw(p, SW_POINTM),
                self.sw(p, SW_OLD_POINTM),
            );
            self.set_marker(start, s, contents, true)?;
            self.set_marker(pointm, pm, contents, true)?;
            self.set_marker(old_pointm, opm, contents, true)?;
            if pbuf != new_current_buffer && pbuf == self.current_buffer {
                crate::editfns::f_goto_char(self, &[pointm])?;
            }
        } else if old_contents_live {
            let contents = old_contents;
            let (start, pointm, old_pointm) = {
                let w = self.window(window);
                (w.start, w.pointm, w.old_pointm)
            };
            let pt = Value::fixnum(self.buffer(contents).pt as i64);
            if self.marker(start).buffer.is_nil() {
                self.set_marker(start, Value::fixnum(0), contents, true)?;
            }
            if self.marker(pointm).buffer.is_nil() {
                self.set_marker(pointm, pt, contents, true)?;
            }
            if self.marker(old_pointm).buffer.is_nil() {
                self.set_marker(old_pointm, pt, contents, true)?;
            }
            self.window_mut(window).flags.start_at_line_beg = true;
            if self.functionp(restore_killed) && !mini {
                let entry = self.kept_window(p, window, pbuf, sym::T);
                self.push_restore_list(cx.kept_index, entry);
            }
        } else if !self.window(window).start.is_nil() {
            let current = self.current_buffer;
            let other = self.other_buffer_safely(current);
            self.wset_buffer(window, other);
            self.window_discard_buffer_from_window(other, window);
            let (start, pointm, old_pointm) = {
                let w = self.window(window);
                (w.start, w.pointm, w.old_pointm)
            };
            for marker in [start, pointm, old_pointm] {
                self.set_marker(marker, Value::fixnum(0), other, true)?;
            }
            self.window_mut(window).flags.start_at_line_beg = true;
            if !mini {
                if self.functionp(restore_killed) {
                    let entry = self.kept_window(p, window, pbuf, Value::NIL);
                    self.push_restore_list(cx.kept_index, entry);
                } else if restore_killed == sym::DELETE
                    || (!self.sw(p, SW_DEDICATED).is_nil()
                        && (restore_killed.is_nil() || restore_killed == sym::DEDICATED))
                {
                    self.push_restore_list(cx.dead_index, window);
                }
                self.window_mut(window).dedicated = Value::NIL;
            }
        }
        Ok(())
    }

    fn kept_window(&mut self, p: Value, window: Value, pbuf: Value, live: Value) -> Value {
        let last = |lisp: &Lisp, slot: usize| {
            let m = lisp.sw(p, slot);
            Value::fixnum(lisp.marker(m).charpos as i64)
        };
        let (start, pointm) = (last(self, SW_START), last(self, SW_POINTM));
        let dedicated = self.sw(p, SW_DEDICATED);
        self.list(&[window, pbuf, start, pointm, dedicated, live])
    }

    fn set_window_configuration(
        &mut self,
        data: &SaveWindowData,
        dont_set_frame: Value,
        dont_set_miniwindow: Value,
    ) -> LispResult {
        let saved: Vec<Value> = self.vector_items(data.saved_windows).to_vec();
        let kept_index = self.stack.len();
        self.stack.push(Value::NIL);
        let dead_index = self.stack.len();
        self.stack.push(Value::NIL);
        let r = self.set_window_configuration_1(
            data,
            &saved,
            dont_set_frame,
            dont_set_miniwindow,
            (kept_index, dead_index),
        );
        self.stack.truncate(kept_index);
        r
    }

    fn set_window_configuration_1(
        &mut self,
        data: &SaveWindowData,
        saved: &[Value],
        dont_set_frame: Value,
        dont_set_miniwindow: Value,
        (kept_index, dead_index): (usize, usize),
    ) -> LispResult {
        let old_frame = self.frames.selected_frame;
        let mut old_point: i64 = -1;
        let mut new_current_buffer = data.f_current_buffer;
        if !self.buffer_live_p(new_current_buffer) {
            new_current_buffer = Value::NIL;
        } else {
            let cw = data.current_window;
            let cw_contents = self.window(cw).contents;
            let sw = self.windows.selected_window;
            let use_cw_point = if new_current_buffer == self.current_buffer {
                cw_contents == new_current_buffer
                    && self.is_window(sw)
                    && self.window(sw).contents == new_current_buffer
                    && sw != cw
            } else {
                cw_contents == new_current_buffer && sw != cw
            };
            old_point = if use_cw_point {
                let pointm = self.window(cw).pointm;
                self.marker_position_checked(pointm)? as i64
            } else {
                self.buffer(new_current_buffer).pt as i64
            };
        }
        let frame = self.window(self.sw(saved[0], SW_WINDOW)).frame;
        if self.frame_live_p(frame) {
            for &p in saved {
                let window = self.sw(p, SW_WINDOW);
                let contents = self.window(window).contents;
                let pbuf = self.sw(p, SW_BUFFER);
                if self.is_buffer(contents)
                    && contents != pbuf
                    && self.is_buffer(pbuf)
                    && self.buffer_live_p(pbuf)
                    && crate::minibuf::f_minibufferp(self, &[pbuf, Value::NIL])?.is_nil()
                {
                    self.funcall(sym::RECORD_WINDOW_BUFFER, &[window])?;
                }
            }
            self.frame_mut(frame).flags.can_set_window_size = false;
            let sw = self.windows.selected_window;
            let contents = self.window(sw).contents;
            if !contents.is_nil() {
                let pointm = self.window(sw).pointm;
                let pt = self.buffer(contents).pt;
                self.set_marker_position(pointm, pt, contents);
            }
            self.fset_redisplay(frame);
            let root = self.frame(frame).root_window;
            self.delete_all_child_windows(root)?;
            let cx = RestoreContext {
                data,
                saved,
                frame,
                new_current_buffer,
                dont_set_miniwindow,
                kept_index,
                dead_index,
            };
            for &p in saved {
                self.restore_saved_window(&cx, p)?;
            }
            self.frame_mut(frame).root_window = data.root_window;
            let cw = data.current_window;
            if self.window(cw).contents == new_current_buffer {
                let (pointm, contents) = {
                    let w = self.window(cw);
                    (w.pointm, w.contents)
                };
                self.set_marker(pointm, Value::fixnum(old_point), contents, true)?;
            }
            self.select_window(cw, sym::T, true)?;
            let sw = self.windows.selected_window;
            let contents = self.window(sw).contents;
            self.buffer_mut(contents).last_selected_window = sw;
            self.windows.window_list = Value::NIL;
            let focus_frame = data.focus_frame;
            if focus_frame.is_nil()
                || (self.is_frame(focus_frame) && self.frame_live_p(focus_frame))
            {
                let frame_focus_frame = self.frame(frame).focus_frame;
                crate::frame::f_redirect_frame_focus(self, &[frame, focus_frame])?;
                if self.plain_value(sym::FOCUS_FOLLOWS_MOUSE) == sym::AUTO_RAISE
                    && focus_frame.is_nil()
                    && self.frames.selected_frame == frame_focus_frame
                {
                    self.funcall(sym::SELECT_FRAME_SET_INPUT_FOCUS, &[frame, Value::NIL])?;
                }
            }
            self.frame_mut(frame).flags.can_set_window_size = true;
            self.adjust_frame_size(frame, -1, -1, 4, sym::SET_WINDOW_CONFIGURATION)?;
            while self.stack[dead_index].is_cons() {
                let window = self.xcar(self.stack[dead_index]);
                if self.window_live_p(window) && window != self.frame(frame).root_window {
                    self.delete_deletable_window(window)?;
                }
                self.stack[dead_index] = self.xcdr(self.stack[dead_index]);
            }
            if self.window_live_p(cw) {
                self.select_window(cw, Value::NIL, false)?;
            }
            if self.frame_live_p(data.selected_frame) {
                let target = if dont_set_frame.is_nil() {
                    data.selected_frame
                } else {
                    old_frame
                };
                self.do_switch_frame(target, Value::NIL)?;
            }
        }
        self.frame_mut(frame).flags.window_change = true;
        if !new_current_buffer.is_nil() {
            crate::buffer::f_set_buffer(self, &[new_current_buffer])?;
            let sw = self.windows.selected_window;
            if self.window(sw).contents != new_current_buffer {
                crate::editfns::f_goto_char(self, &[Value::fixnum(old_point)])?;
            }
        }
        self.set_plain_value(sym::MINIBUFFER_SCROLL_WINDOW, data.minibuf_scroll_window);
        self.windows.minibuf_selected_window = data.minibuf_selected_window;
        let restore_killed = self.plain_value(sym::WINDOW_RESTORE_KILLED_BUFFER_WINDOWS);
        if self.var_true(sym::RUN_HOOKS_ENABLED) && self.functionp(restore_killed) {
            let kept_windows = self.stack[kept_index];
            self.safe_calln(restore_killed, &[frame, kept_windows, sym::CONFIGURATION])?;
        }
        Ok(self.bool(self.frame_live_p(frame)))
    }

    fn delete_deletable_window(&mut self, window: Value) -> LispResult<()> {
        if !self.funcall(sym::WINDOW_DELETABLE_P, &[window])?.is_nil() {
            self.funcall(sym::DELETE_WINDOW, &[window])?;
        }
        Ok(())
    }

    fn minibuffer_window_body_height(&self, m: Value) -> i64 {
        let w = self.window(m);
        let f = w.frame;
        let scroll_bar = if !w.flags.pseudo_window_p && w.horizontal_scroll_bar_type == sym::BOTTOM
        {
            if w.scroll_bar_height >= 0 {
                w.scroll_bar_height
            } else {
                self.frame(f).geometry.config_scroll_bar_height
            }
        } else {
            0
        };
        let divider = if w.prev == self.frame(f).root_window || w.flags.pseudo_window_p {
            0
        } else {
            self.frame(f).geometry.bottom_divider_width
        };
        (w.pixel_height - scroll_bar - divider).max(0)
    }

    pub(crate) fn resize_frame_windows(
        &mut self,
        f: Value,
        size: i64,
        horflag: bool,
    ) -> LispResult<()> {
        let root = self.frame(f).root_window;
        let old_pixel_size = if horflag {
            self.window(root).pixel_width
        } else {
            self.window(root).pixel_height
        };
        let unit = if horflag {
            self.frame(f).geometry.column_width
        } else {
            self.frame(f).geometry.line_height
        };
        let mini = self.frame(f).minibuffer_window;
        let has_mini = self.frame_has_minibuf_p(f) && !self.frame_minibuf_only_p(f);
        let mini_height = if has_mini {
            unit + self.window(mini).pixel_height - self.minibuffer_window_body_height(mini)
        } else {
            0
        };
        let new_pixel_size = (if horflag { size } else { size - mini_height }).max(unit);
        let new_size = new_pixel_size / unit;
        let top_margin = self.frame_top_margin(f);
        let top_margin_height = self.frame_top_margin_height(f);
        if new_pixel_size == old_pixel_size
            && (horflag || self.window(root).pixel_top == top_margin_height)
        {
        } else if self.is_buffer(self.window(root).contents) {
            let r = self.window_mut(root);
            if horflag {
                r.total_cols = new_size;
                r.pixel_width = new_pixel_size;
            } else {
                r.top_line = top_margin;
                r.pixel_top = top_margin_height;
                r.total_lines = new_size;
                r.pixel_height = new_pixel_size;
            }
            let pseudo = r.flags.pseudo_window_p;
            self.frame_mut(f).flags.window_change = !pseudo && new_pixel_size != old_pixel_size;
        } else {
            unreachable!("resizing a split root window needs window_resize_apply");
        }
        if has_mini {
            let (top_line, total_lines, pixel_top, pixel_height) = {
                let r = self.window(root);
                (r.top_line, r.total_lines, r.pixel_top, r.pixel_height)
            };
            let m = self.window_mut(mini);
            if horflag {
                m.total_cols = new_size;
                m.pixel_width = new_pixel_size;
            } else {
                m.total_lines = mini_height / unit;
                m.pixel_height = mini_height;
                m.top_line = top_line + total_lines;
                m.pixel_top = pixel_top + pixel_height;
            }
        }
        self.fset_redisplay(f);
        Ok(())
    }

    fn compare_window_configurations(&mut self, c1: Value, c2: Value) -> LispResult<bool> {
        let d1 = self.check_window_configuration(c1)?;
        let d2 = self.check_window_configuration(c2)?;
        let sws1 = self.vector_items(d1.saved_windows).to_vec();
        let sws2 = self.vector_items(d2.saved_windows).to_vec();
        if d1.frame_cols != d2.frame_cols
            || d1.frame_lines != d2.frame_lines
            || d1.frame_menu_bar_lines != d2.frame_menu_bar_lines
            || d1.selected_frame != d2.selected_frame
            || d1.f_current_buffer != d2.f_current_buffer
            || d1.focus_frame != d2.focus_frame
            || sws1.len() != sws2.len()
        {
            return Ok(false);
        }
        for (&sw1, &sw2) in sws1.iter().zip(&sws2) {
            if (d1.current_window == self.sw(sw1, SW_WINDOW))
                != (d2.current_window == self.sw(sw2, SW_WINDOW))
            {
                return Ok(false);
            }
            for slot in [
                SW_BUFFER,
                SW_PIXEL_LEFT,
                SW_PIXEL_TOP,
                SW_PIXEL_HEIGHT,
                SW_PIXEL_WIDTH,
                SW_LEFT_COL,
                SW_TOP_LINE,
                SW_TOTAL_COLS,
                SW_TOTAL_LINES,
                SW_DISPLAY_TABLE,
                SW_PARENT,
                SW_PREV,
                SW_LEFT_MARGIN_COLS,
                SW_RIGHT_MARGIN_COLS,
                SW_LEFT_FRINGE_WIDTH,
                SW_RIGHT_FRINGE_WIDTH,
                SW_FRINGES_OUTSIDE_MARGINS,
                SW_FRINGES_PERSISTENT,
                SW_SCROLL_BAR_WIDTH,
                SW_SCROLL_BAR_HEIGHT,
                SW_VERTICAL_SCROLL_BAR_TYPE,
                SW_HORIZONTAL_SCROLL_BAR_TYPE,
                SW_SCROLL_BARS_PERSISTENT,
            ] {
                if self.sw(sw1, slot) != self.sw(sw2, slot) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

pub(crate) fn restore_window_configuration(
    lisp: &mut Lisp,
    configuration: Value,
) -> LispResult<()> {
    if configuration.is_cons() {
        let config = lisp.xcar(configuration);
        let rest = lisp.xcdr(configuration);
        let dont_set_frame = lisp.car_safe(rest);
        let rest = lisp.cdr_safe(rest);
        let dont_set_miniwindow = lisp.car_safe(rest);
        f_set_window_configuration(lisp, &[config, dont_set_frame, dont_set_miniwindow])?;
    } else {
        f_set_window_configuration(lisp, &[configuration, Value::NIL, Value::NIL])?;
    }
    Ok(())
}

fn f_window_fringes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let window = lisp.decode_live_window(arg(args, 0))?;
    let w = lisp.window(window);
    let g = &lisp.frame(w.frame).geometry;
    let left = if w.left_fringe_width >= 0 {
        w.left_fringe_width
    } else {
        g.left_fringe_width
    };
    let right = if w.right_fringe_width >= 0 {
        w.right_fringe_width
    } else {
        g.right_fringe_width
    };
    let outside = lisp.bool(w.flags.fringes_outside_margins);
    let persistent = lisp.bool(w.flags.fringes_persistent);
    Ok(lisp.list(&[
        Value::fixnum(left),
        Value::fixnum(right),
        outside,
        persistent,
    ]))
}

fn f_window_configuration_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.bool(lisp.is_window_configuration(args[0])))
}

fn f_window_configuration_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let data = lisp.check_window_configuration(args[0])?;
    let first = lisp.vector_items(data.saved_windows)[0];
    let window = lisp.sw(first, SW_WINDOW);
    Ok(lisp.window(window).frame)
}

fn f_set_window_configuration(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (configuration, dont_set_frame, dont_set_miniwindow) =
        (args[0], arg(args, 1), arg(args, 2));
    let data = lisp.check_window_configuration(configuration)?;
    let base = lisp.stack.len();
    lisp.stack.push(configuration);
    let r = lisp.set_window_configuration(&data, dont_set_frame, dont_set_miniwindow);
    lisp.stack.truncate(base);
    r
}

fn f_current_window_configuration(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    let root = lisp.frame(f).root_window;
    let n_windows = lisp.count_windows(root);
    let minibuf_active = lisp.minibuf.minibuf_level > 0;
    let fr = lisp.frame(f);
    let g = &fr.geometry;
    let mut data = SaveWindowData {
        selected_frame: lisp.frames.selected_frame,
        current_window: fr.selected_window,
        f_current_buffer: lisp.current_buffer,
        minibuf_scroll_window: if minibuf_active {
            lisp.plain_value(sym::MINIBUFFER_SCROLL_WINDOW)
        } else {
            Value::NIL
        },
        minibuf_selected_window: if minibuf_active {
            lisp.windows.minibuf_selected_window
        } else {
            Value::NIL
        },
        root_window: root,
        focus_frame: fr.focus_frame,
        saved_windows: Value::NIL,
        change_stamp: fr.change_stamp,
        frame_cols: g.text_cols,
        frame_lines: g.text_lines,
        frame_menu_bar_lines: g.menu_bar_lines,
        frame_tab_bar_lines: g.tab_bar_lines,
        frame_tool_bar_lines: g.tool_bar_lines,
        frame_text_width: g.text_width,
        frame_text_height: g.text_height,
        frame_menu_bar_height: g.menu_bar_height,
        frame_tab_bar_height: g.tab_bar_height,
        frame_tool_bar_height: g.tool_bar_height,
    };
    let saved: Vec<Value> = (0..n_windows)
        .map(|_| lisp.vector(vec![Value::NIL; SAVED_WINDOW_SIZE]))
        .collect();
    let tem = lisp.vector(saved);
    data.saved_windows = tem;
    let base = lisp.stack.len();
    lisp.stack.push(tem);
    let r = lisp.save_window_save(root, tem, 0);
    lisp.stack.truncate(base);
    r?;
    Ok(lisp
        .heap
        .vectorlike(Vectorlike::WindowConfiguration(Box::new(data))))
}

fn f_window_configuration_equal_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let equal = lisp.compare_window_configurations(args[0], args[1])?;
    Ok(lisp.bool(equal))
}
