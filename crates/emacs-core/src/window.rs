pub mod resize;
pub mod scroll;
pub mod split;

use crate::dispextern::{GlyphMatrix, GlyphRowArea, WindowPart};
use crate::dispnew::matrix::WhichMatrix;
use crate::doprnt::Arg;
use crate::error::LispResult;
use crate::frame::VerticalScrollBarType;
use crate::lisp::Lisp;
use crate::object::{Vectorlike, Weakness};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowBodyUnit {
    CanonicalChars,
    Pixels,
    RemappedChars,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarginUnit {
    InLines,
    InPixels,
}

fn window_body_unit_from_symbol(unit: Value) -> WindowBodyUnit {
    if unit == sym::REMAP {
        WindowBodyUnit::RemappedChars
    } else if unit.is_nil() {
        WindowBodyUnit::CanonicalChars
    } else {
        WindowBodyUnit::Pixels
    }
}

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
    pub last_cursor_off_p: bool,
    pub phys_cursor_on_p: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CursorPos {
    pub x: i64,
    pub y: i64,
    pub hpos: i64,
    pub vpos: i64,
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
    pub cursor: CursorPos,
    pub output_cursor: CursorPos,
    pub phys_cursor: CursorPos,
    pub phys_cursor_type: crate::dispextern::TextCursorKind,
    pub phys_cursor_ascent: i64,
    pub phys_cursor_height: i64,
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
    pub window_end_bytepos: i64,
    pub window_end_vpos: i64,
    pub last_point: i64,
    pub last_mark: i64,
    pub vscroll: i64,
    pub last_modified: i64,
    pub last_overlay_modified: i64,
    pub mini: bool,
    pub horizontal: bool,
    pub flags: WindowFlags,
    pub current_matrix: Option<Box<GlyphMatrix>>,
    pub desired_matrix: Option<Box<GlyphMatrix>>,
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
        for matrix in [&self.current_matrix, &self.desired_matrix]
            .into_iter()
            .flatten()
        {
            matrix.trace(out);
        }
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
    pub window_change_record_frames: bool,
    pub window_scroll_pixel_based_preserve_x: i32,
    pub window_scroll_pixel_based_preserve_y: i32,
    pub window_scroll_preserve_hpos: i64,
    pub window_scroll_preserve_vpos: i64,
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
            window_change_record_frames: false,
            window_scroll_pixel_based_preserve_x: -1,
            window_scroll_pixel_based_preserve_y: -1,
            window_scroll_preserve_hpos: -1,
            window_scroll_preserve_vpos: -1,
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

const USE_HORIZONTAL_SCROLL_BARS: bool = false;

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
    Subr::fixed("window-body-width", 0, 2, f_window_body_width),
    Subr::fixed("window-body-height", 0, 2, f_window_body_height),
    Subr::fixed("pos-visible-in-window-p", 0, 3, f_pos_visible_in_window_p),
    Subr::fixed("window-line-height", 0, 2, f_window_line_height),
    Subr::fixed("window-vscroll", 0, 2, f_window_vscroll),
    Subr::fixed("coordinates-in-window-p", 2, 2, f_coordinates_in_window_p),
    Subr::fixed("window-at", 2, 3, f_window_at),
    Subr::fixed("set-window-vscroll", 2, 4, f_set_window_vscroll),
    Subr::fixed("window-new-total", 0, 1, f_window_new_total),
    Subr::fixed("window-normal-size", 0, 2, f_window_normal_size),
    Subr::fixed("window-new-normal", 0, 1, f_window_new_normal),
    Subr::fixed("window-new-pixel", 0, 1, f_window_new_pixel),
    Subr::fixed("window-pixel-left", 0, 1, f_window_pixel_left),
    Subr::fixed("window-pixel-top", 0, 1, f_window_pixel_top),
    Subr::fixed("window-left-column", 0, 1, f_window_left_column),
    Subr::fixed("window-top-line", 0, 1, f_window_top_line),
    Subr::fixed("window-mode-line-height", 0, 1, f_window_mode_line_height),
    Subr::fixed(
        "window-header-line-height",
        0,
        1,
        f_window_header_line_height,
    ),
    Subr::fixed("window-tab-line-height", 0, 1, f_window_tab_line_height),
    Subr::fixed(
        "window-right-divider-width",
        0,
        1,
        f_window_right_divider_width,
    ),
    Subr::fixed(
        "window-bottom-divider-width",
        0,
        1,
        f_window_bottom_divider_width,
    ),
    Subr::fixed("window-scroll-bar-width", 0, 1, f_window_scroll_bar_width),
    Subr::fixed("window-scroll-bar-height", 0, 1, f_window_scroll_bar_height),
    Subr::fixed("window-hscroll", 0, 1, f_window_hscroll),
    Subr::fixed("window-point", 0, 1, f_window_point),
    Subr::fixed("window-old-point", 0, 1, f_window_old_point),
    Subr::fixed("window-start", 0, 1, f_window_start),
    Subr::fixed("set-window-point", 2, 2, f_set_window_point),
    Subr::fixed("set-window-start", 2, 3, f_set_window_start),
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
    Subr::fixed("set-window-margins", 2, 3, f_set_window_margins),
    Subr::fixed("window-margins", 0, 1, f_window_margins),
    Subr::fixed("set-window-fringes", 2, 5, f_set_window_fringes),
    Subr::fixed("set-window-scroll-bars", 1, 6, f_set_window_scroll_bars),
    Subr::fixed("window-scroll-bars", 0, 1, f_window_scroll_bars),
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
            let name = lisp.string_bytes(name).to_vec();
            return Err(lisp.verror("Window is dedicated to `%s'", &[Arg::Str(&name)]));
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

    pub(crate) fn allocate_window() -> Window {
        Window {
            frame: Value::NIL,
            next: Value::NIL,
            prev: Value::NIL,
            parent: Value::NIL,
            normal_lines: Value::NIL,
            normal_cols: Value::NIL,
            new_total: Value::NIL,
            new_normal: Value::NIL,
            new_pixel: Value::NIL,
            contents: Value::NIL,
            prev_buffers: Value::NIL,
            next_buffers: Value::NIL,
            old_buffer: Value::NIL,
            start: Value::NIL,
            pointm: Value::NIL,
            old_pointm: Value::NIL,
            temslot: Value::NIL,
            vertical_scroll_bar: Value::NIL,
            vertical_scroll_bar_type: Value::NIL,
            horizontal_scroll_bar: Value::NIL,
            horizontal_scroll_bar_type: Value::NIL,
            display_table: Value::NIL,
            dedicated: Value::NIL,
            combination_limit: Value::NIL,
            window_parameters: Value::NIL,
            cursor_type: Value::NIL,
            mode_line_help_echo: Value::NIL,
            use_time: 0,
            sequence_number: 0,
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
            column_number_displayed: 0,
            nrows_scale_factor: 0,
            ncols_scale_factor: 0,
            cursor: CursorPos::default(),
            output_cursor: CursorPos::default(),
            phys_cursor: CursorPos::default(),
            phys_cursor_type: crate::dispextern::TextCursorKind::FilledBox,
            phys_cursor_ascent: 0,
            phys_cursor_height: 0,
            last_cursor_vpos: 0,
            phys_cursor_width: 0,
            left_fringe_width: 0,
            right_fringe_width: 0,
            left_margin_cols: 0,
            right_margin_cols: 0,
            scroll_bar_width: 0,
            scroll_bar_height: 0,
            mode_line_height: 0,
            header_line_height: 0,
            tab_line_height: 0,
            window_end_pos: 0,
            window_end_bytepos: 0,
            last_point: 0,
            last_mark: 0,
            window_end_vpos: 0,
            vscroll: 0,
            last_modified: 0,
            last_overlay_modified: 0,
            mini: false,
            horizontal: false,
            flags: WindowFlags::default(),
            current_matrix: None,
            desired_matrix: None,
        }
    }

    pub fn make_window(&mut self) -> Value {
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
        let mut w = Lisp::allocate_window();
        w.normal_lines = normal_lines;
        w.normal_cols = normal_cols;
        w.new_total = Value::fixnum(0);
        w.new_normal = Value::fixnum(0);
        w.new_pixel = Value::fixnum(0);
        w.start = start;
        w.pointm = pointm;
        w.old_pointm = old_pointm;
        w.vertical_scroll_bar_type = sym::T;
        w.horizontal_scroll_bar_type = sym::T;
        w.cursor_type = sym::T;
        w.nrows_scale_factor = 1;
        w.ncols_scale_factor = 1;
        w.left_fringe_width = -1;
        w.right_fringe_width = -1;
        w.mode_line_height = -1;
        w.tab_line_height = -1;
        w.header_line_height = -1;
        w.phys_cursor_type = crate::dispextern::TextCursorKind::NoCursor;
        w.phys_cursor_width = -1;
        self.windows.sequence_number += 1;
        w.sequence_number = self.windows.sequence_number;
        w.scroll_bar_width = -1;
        w.scroll_bar_height = -1;
        w.column_number_displayed = -1;
        self.windows.window_list = Value::NIL;
        self.heap.vectorlike(Vectorlike::Window(Box::new(w)))
    }

    pub(crate) fn make_parent_window(&mut self, frame: Value) -> Value {
        let mut p = Lisp::allocate_window();
        self.windows.sequence_number += 1;
        p.sequence_number = self.windows.sequence_number;
        p.frame = frame;
        self.heap.vectorlike(Vectorlike::Window(Box::new(p)))
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

    pub(crate) fn window_frame_column_width(&self, w: Value) -> i64 {
        self.frame(self.window(w).frame).geometry.column_width
    }

    fn window_frame_window_p(&self, w: Value) -> bool {
        self.frame(self.window(w).frame).window_p()
    }

    pub(crate) fn window_left_fringe_width(&self, w: Value) -> i64 {
        let win = self.window(w);
        if win.left_fringe_width >= 0 {
            win.left_fringe_width
        } else {
            self.frame(win.frame).geometry.left_fringe_width
        }
    }

    pub(crate) fn window_right_fringe_width(&self, w: Value) -> i64 {
        let win = self.window(w);
        if win.right_fringe_width >= 0 {
            win.right_fringe_width
        } else {
            self.frame(win.frame).geometry.right_fringe_width
        }
    }

    pub(crate) fn window_fringes_width(&self, w: Value) -> i64 {
        if !self.window_frame_window_p(w) {
            return 0;
        }
        self.window_left_fringe_width(w) + self.window_right_fringe_width(w)
    }

    pub(crate) fn window_margins_width(&self, w: Value) -> i64 {
        let win = self.window(w);
        (win.left_margin_cols + win.right_margin_cols) * self.window_frame_column_width(w)
    }

    fn window_scroll_bar_cols(&self, w: Value) -> i64 {
        if !self.window_has_vertical_scroll_bar(w) {
            return 0;
        }
        let win = self.window(w);
        if win.scroll_bar_width >= 0 {
            let unit = self.window_frame_column_width(w);
            (win.scroll_bar_width + unit - 1) / unit
        } else {
            self.frame(win.frame).geometry.config_scroll_bar_cols
        }
    }

    fn window_scroll_bar_lines(&self, w: Value) -> i64 {
        if !self.window_has_horizontal_scroll_bar(w) {
            return 0;
        }
        let win = self.window(w);
        if win.scroll_bar_height >= 0 {
            let unit = self.window_frame_line_height(w);
            (win.scroll_bar_height + unit - 1) / unit
        } else {
            self.frame(win.frame).geometry.config_scroll_bar_lines
        }
    }

    fn window_vertical_scroll_bar_type(&self, w: Value) -> VerticalScrollBarType {
        let win = self.window(w);
        if win.flags.pseudo_window_p {
            VerticalScrollBarType::None
        } else if win.vertical_scroll_bar_type == sym::T {
            self.frame(win.frame).vertical_scroll_bar_type
        } else if win.vertical_scroll_bar_type == sym::LEFT {
            VerticalScrollBarType::Left
        } else if win.vertical_scroll_bar_type == sym::RIGHT {
            VerticalScrollBarType::Right
        } else {
            VerticalScrollBarType::None
        }
    }

    pub(crate) fn window_has_vertical_scroll_bar_on_right(&self, w: Value) -> bool {
        self.window_vertical_scroll_bar_type(w) == VerticalScrollBarType::Right
    }

    pub(crate) fn frame_has_vertical_scroll_bars(&self, f: Value) -> bool {
        self.frame(f).vertical_scroll_bar_type != VerticalScrollBarType::None
    }

    pub(crate) fn window_has_vertical_scroll_bar(&self, w: Value) -> bool {
        self.window_vertical_scroll_bar_type(w) != VerticalScrollBarType::None
    }

    fn window_config_scroll_bar_width(&self, w: Value) -> i64 {
        let win = self.window(w);
        if win.scroll_bar_width >= 0 {
            win.scroll_bar_width
        } else {
            self.frame(win.frame).geometry.config_scroll_bar_width
        }
    }

    pub(crate) fn window_right_scroll_bar_area_width(&self, w: Value) -> i64 {
        if self.window_has_vertical_scroll_bar_on_right(w) {
            self.window_config_scroll_bar_width(w)
        } else {
            0
        }
    }

    pub fn window_bottom_edge_y(&self, w: Value) -> i64 {
        let border = if self.window_bar_window_p(w) {
            0
        } else {
            self.frame_internal_border_width(self.window(w).frame)
        };
        border + self.window(w).pixel_top + self.window(w).pixel_height
    }

    pub fn window_box_right_edge_x(&self, w: Value) -> i64 {
        self.frame_internal_border_width(self.window(w).frame)
            + self.window(w).pixel_left
            + self.window(w).pixel_width
            - self.window_right_divider_width(w)
            - self.window_right_scroll_bar_area_width(w)
    }

    pub fn coordinates_in_window(&mut self, w: Value, x: i64, y: i64) -> WindowPart {
        let f = self.window(w).frame;
        let ux = self.frame(f).geometry.column_width;
        let mut left_x = self.window_left_edge_x(w);
        let mut right_x = self.window_right_edge_x(w);
        let top_y = self.window_top_edge_y(w);
        let bottom_y = self.window_bottom_edge_y(w);
        let grabbable_width = ux;
        if y < top_y || y >= bottom_y || x < left_x || x >= right_x {
            return WindowPart::Nothing;
        }
        let bottom_divider = self.window_bottom_divider_width(w);
        let right_divider = self.window_right_divider_width(w);
        if bottom_divider > 0 && y >= bottom_y - bottom_divider && y <= bottom_y {
            return WindowPart::BottomDivider;
        } else if !self.window_rightmost_p(w)
            && right_divider > 0
            && x >= right_x - right_divider
            && x <= right_x
        {
            return WindowPart::RightDivider;
        }
        let mode_line_height = self.current_mode_line_height(w);
        if self.window_has_horizontal_scroll_bar(w)
            && y >= bottom_y
                - self.window_scroll_bar_area_height(w)
                - mode_line_height
                - bottom_divider
            && y <= bottom_y - mode_line_height - bottom_divider
        {
            return WindowPart::HorizontalScrollBar;
        }
        let mut part = WindowPart::Nothing;
        if self.window_wants_mode_line(w)
            && y >= bottom_y - mode_line_height - bottom_divider
            && y <= bottom_y - bottom_divider
        {
            part = WindowPart::ModeLine;
        } else if self.window_wants_tab_line(w) && y < top_y + self.current_tab_line_height(w) {
            part = WindowPart::TabLine;
        } else if self.window_wants_header_line(w) {
            let tab = if self.window_wants_tab_line(w) {
                self.current_tab_line_height(w)
            } else {
                0
            };
            if y < top_y + self.current_header_line_height(w) + tab {
                part = WindowPart::HeaderLine;
            }
        }
        if part != WindowPart::Nothing {
            let on_left = self.window_has_vertical_scroll_bar_on_left(w);
            if right_divider == 0
                && ((on_left
                    && self.window(w).pixel_left != 0
                    && (x - left_x).abs() < grabbable_width)
                    || (!on_left
                        && !self.window_rightmost_p(w)
                        && (x - right_x).abs() < grabbable_width))
            {
                return WindowPart::VerticalBorder;
            }
            return part;
        }
        if self.window(w).flags.pseudo_window_p {
            left_x = 0;
            right_x = self.window(w).pixel_width - 1;
        } else {
            left_x = self.window_box_left_edge_x(w);
            right_x = self.window_box_right_edge_x(w) - 1;
        }
        if x < left_x || x > right_x {
            return WindowPart::VerticalScrollBar;
        }
        let lmargin_width = self.window_box_width(w, GlyphRowArea::LeftMargin);
        let rmargin_width = self.window_box_width(w, GlyphRowArea::RightMargin);
        let text_left = self.window_box_left(w, GlyphRowArea::Text);
        let text_right = text_left + self.window_box_width(w, GlyphRowArea::Text);
        if self.frame_window_p(f) {
            if !self.window(w).flags.pseudo_window_p
                && right_divider == 0
                && !self.window_has_vertical_scroll_bar(w)
                && !self.window_rightmost_p(w)
                && (x - right_x).abs() < grabbable_width
            {
                return WindowPart::VerticalBorder;
            }
        } else if !self.window(w).flags.pseudo_window_p
            && right_divider == 0
            && !self.window_rightmost_p(w)
            && x > right_x - ux
        {
            return WindowPart::VerticalBorder;
        }
        let outside = self.window_has_fringes_outside_margins(w);
        if x < text_left {
            let in_margin = if outside {
                x >= left_x + self.window_left_fringe_width(w)
            } else {
                x < left_x + lmargin_width
            };
            return if lmargin_width > 0 && in_margin {
                WindowPart::LeftMargin
            } else {
                WindowPart::LeftFringe
            };
        }
        if x >= text_right {
            let in_margin = if outside {
                x < right_x - self.window_right_fringe_width(w)
            } else {
                x >= right_x - rmargin_width
            };
            return if rmargin_width > 0 && in_margin {
                WindowPart::RightMargin
            } else {
                WindowPart::RightFringe
            };
        }
        WindowPart::Text
    }

    fn foreach_window_containing(
        &mut self,
        w: Value,
        x: i64,
        y: i64,
    ) -> Option<(Value, WindowPart)> {
        let mut w = w;
        while !w.is_nil() {
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                if let Some(found) = self.foreach_window_containing(contents, x, y) {
                    return Some(found);
                }
            } else {
                let part = self.coordinates_in_window(w, x, y);
                if part != WindowPart::Nothing {
                    return Some((w, part));
                }
            }
            w = self.window(w).next;
        }
        None
    }

    pub fn window_from_coordinates(
        &mut self,
        f: Value,
        x: i64,
        y: i64,
        part: &mut WindowPart,
        tab_bar_p: bool,
    ) -> Value {
        let root = self.frame(f).root_window;
        let mut window = Value::NIL;
        if self.is_window(root)
            && let Some((w, found)) = self.foreach_window_containing(root, x, y)
        {
            *part = found;
            window = w;
        }
        let tab_bar_window = self.frame(f).tab_bar_window;
        if window.is_nil()
            && tab_bar_p
            && self.is_window(tab_bar_window)
            && self.window(tab_bar_window).total_lines > 0
            && self.coordinates_in_window(tab_bar_window, x, y) != WindowPart::Nothing
        {
            *part = WindowPart::Text;
            window = tab_bar_window;
        }
        window
    }

    pub(crate) fn window_scroll_bar_area_width(&self, w: Value) -> i64 {
        if self.window_has_vertical_scroll_bar(w) {
            self.window_config_scroll_bar_width(w)
        } else {
            0
        }
    }

    pub(crate) fn window_has_horizontal_scroll_bar(&self, w: Value) -> bool {
        if !USE_HORIZONTAL_SCROLL_BARS {
            return false;
        }
        let win = self.window(w);
        if win.flags.pseudo_window_p || (win.mini && win.horizontal_scroll_bar_type != sym::BOTTOM)
        {
            false
        } else if win.horizontal_scroll_bar_type == sym::T {
            self.frame(win.frame).flags.horizontal_scroll_bars
        } else {
            win.horizontal_scroll_bar_type == sym::BOTTOM
        }
    }

    fn window_config_scroll_bar_height(&self, w: Value) -> i64 {
        let win = self.window(w);
        if win.scroll_bar_height >= 0 {
            win.scroll_bar_height
        } else {
            self.frame(win.frame).geometry.config_scroll_bar_height
        }
    }

    pub(crate) fn window_scroll_bar_area_height(&self, w: Value) -> i64 {
        if self.window_has_horizontal_scroll_bar(w) {
            self.window_config_scroll_bar_height(w)
        } else {
            0
        }
    }

    fn window_frame_line_height(&self, w: Value) -> i64 {
        self.frame(self.window(w).frame).geometry.line_height
    }

    pub(crate) fn window_rightmost_p(&self, w: Value) -> bool {
        let win = self.window(w);
        let root = self.window(self.frame(win.frame).root_window);
        win.pixel_left + win.pixel_width == root.pixel_left + root.pixel_width
    }

    fn window_bottommost_p(&self, w: Value) -> bool {
        let win = self.window(w);
        let root = self.window(self.frame(win.frame).root_window);
        win.pixel_top + win.pixel_height == root.pixel_top + root.pixel_height
    }

    pub(crate) fn window_right_divider_width(&self, w: Value) -> i64 {
        if self.window_rightmost_p(w) {
            0
        } else {
            self.frame(self.window(w).frame)
                .geometry
                .right_divider_width
        }
    }

    pub(crate) fn window_bottom_divider_width(&self, w: Value) -> i64 {
        let win = self.window(w);
        let root = self.frame(win.frame).root_window;
        if (self.window_bottommost_p(w) && self.window(root).next.is_nil())
            || win.prev == root
            || win.flags.pseudo_window_p
        {
            0
        } else {
            self.frame(win.frame).geometry.bottom_divider_width
        }
    }

    fn window_wants_line(&self, w: Value, format_symbol: Value, lines_below: i64) -> bool {
        let format = self.window_parameter(w, format_symbol);
        let win = self.window(w);
        self.is_buffer(win.contents)
            && !win.mini
            && !win.flags.pseudo_window_p
            && format != sym::NONE
            && (!format.is_nil()
                || !self
                    .buffer_local_value(format_symbol, win.contents)
                    .is_nil())
            && win.pixel_height > (lines_below + 1) * self.window_frame_line_height(w)
    }

    pub fn window_wants_mode_line(&self, w: Value) -> bool {
        self.window_wants_line(w, sym::MODE_LINE_FORMAT, 0)
    }

    pub fn window_wants_header_line(&self, w: Value) -> bool {
        let below = i64::from(self.window_wants_mode_line(w));
        self.window_wants_line(w, sym::HEADER_LINE_FORMAT, below)
    }

    pub fn window_wants_tab_line(&self, w: Value) -> bool {
        let below =
            i64::from(self.window_wants_mode_line(w)) + i64::from(self.window_wants_header_line(w));
        self.window_wants_line(w, sym::TAB_LINE_FORMAT, below)
    }

    fn matrix_line_height(
        &self,
        w: Value,
        row: fn(&crate::dispextern::GlyphMatrix) -> usize,
    ) -> i64 {
        self.window(w)
            .current_matrix
            .as_deref()
            .filter(|m| !m.rows.is_empty())
            .map_or(0, |m| i64::from(m.rows[row(m)].height))
    }

    pub(crate) fn current_mode_line_height(&mut self, w: Value) -> i64 {
        let height = self.window(w).mode_line_height;
        if height >= 0 {
            return height;
        }
        let matrix_height = self.matrix_line_height(w, |m| m.mode_line_row());
        if matrix_height != 0 {
            self.window_mut(w).mode_line_height = matrix_height;
            return matrix_height;
        }
        let face_id = self.current_line_active_face_id(
            w,
            crate::xfaces::MODE_LINE_ACTIVE_FACE_ID,
            crate::xfaces::MODE_LINE_INACTIVE_FACE_ID,
        );
        let height = self.estimate_mode_line_height(self.window(w).frame, face_id);
        self.window_mut(w).mode_line_height = height;
        height
    }

    pub(crate) fn current_header_line_height(&mut self, w: Value) -> i64 {
        let height = self.window(w).header_line_height;
        if height >= 0 {
            return height;
        }
        let matrix_height = self.matrix_line_height(w, |m| m.header_line_row());
        if matrix_height != 0 {
            self.window_mut(w).header_line_height = matrix_height;
            return matrix_height;
        }
        let face_id = self.current_line_active_face_id(
            w,
            crate::xfaces::HEADER_LINE_ACTIVE_FACE_ID,
            crate::xfaces::HEADER_LINE_INACTIVE_FACE_ID,
        );
        let height = self.estimate_mode_line_height(self.window(w).frame, face_id);
        self.window_mut(w).header_line_height = height;
        height
    }

    pub(crate) fn current_tab_line_height(&mut self, w: Value) -> i64 {
        let height = self.window(w).tab_line_height;
        if height >= 0 {
            return height;
        }
        let matrix_height = self.matrix_line_height(w, |_| 0);
        if matrix_height != 0 {
            self.window_mut(w).tab_line_height = matrix_height;
            return matrix_height;
        }
        let face_id = self.current_line_active_face_id(
            w,
            crate::xfaces::TAB_LINE_ACTIVE_FACE_ID,
            crate::xfaces::TAB_LINE_INACTIVE_FACE_ID,
        );
        let height = self.estimate_mode_line_height(self.window(w).frame, face_id);
        self.window_mut(w).tab_line_height = height;
        height
    }

    pub fn window_display_table(&self, w: Value) -> Value {
        let win = self.window(w);
        if self.disp_table_p(win.display_table) {
            return win.display_table;
        }
        if self.is_buffer(win.contents) {
            let table = self.buffer_local_value(sym::BUFFER_DISPLAY_TABLE, win.contents);
            if self.disp_table_p(table) {
                return table;
            }
            let standard = self.find_symbol_value(sym::STANDARD_DISPLAY_TABLE);
            if self.disp_table_p(standard) {
                return standard;
            }
        }
        Value::NIL
    }

    pub fn window_internal_height(&self, w: Value) -> i64 {
        let mut ht = self.window(w).total_lines;
        if self.window_wants_mode_line(w) {
            ht -= 1;
        }
        if self.window_wants_header_line(w) {
            ht -= 1;
        }
        if self.window_wants_tab_line(w) {
            ht -= 1;
        }
        ht
    }

    pub fn window_scroll_margin(&mut self, w: Value, unit: MarginUnit) -> i32 {
        let scroll_margin = self.int_var(sym::SCROLL_MARGIN);
        if scroll_margin <= 0 {
            return 0;
        }
        let frame_line_height = self.default_line_pixel_height(w);
        let window_lines = (self.window_box_height(w) / i64::from(frame_line_height)) as i32;
        let mut ratio = 0.25;
        let maximum = self.find_symbol_value(sym::MAXIMUM_SCROLL_MARGIN);
        if maximum.is_float() {
            ratio = self.heap.float_value(maximum).clamp(0.0, 0.5);
        }
        let max_margin = ((window_lines - 1) / 2).min((f64::from(window_lines) * ratio) as i32);
        let margin = if scroll_margin < 0 {
            0
        } else if scroll_margin <= i64::from(max_margin) {
            scroll_margin as i32
        } else {
            max_margin
        };
        if unit == MarginUnit::InPixels {
            margin * frame_line_height
        } else {
            margin
        }
    }

    pub(crate) fn window_left_margin_width(&self, w: Value) -> i64 {
        self.window(w).left_margin_cols * self.window_frame_column_width(w)
    }

    pub(crate) fn window_right_margin_width(&self, w: Value) -> i64 {
        self.window(w).right_margin_cols * self.window_frame_column_width(w)
    }

    pub(crate) fn window_left_scroll_bar_area_width(&self, w: Value) -> i64 {
        if self.window_vertical_scroll_bar_type(w) == VerticalScrollBarType::Left {
            self.window_config_scroll_bar_width(w)
        } else {
            0
        }
    }

    pub(crate) fn window_has_vertical_scroll_bar_on_left(&self, w: Value) -> bool {
        self.window_vertical_scroll_bar_type(w) == VerticalScrollBarType::Left
    }

    pub(crate) fn window_has_fringes_outside_margins(&self, w: Value) -> bool {
        self.window(w).flags.fringes_outside_margins
    }

    pub(crate) fn window_full_width_p(&self, w: Value) -> bool {
        let root = self.frame(self.window(w).frame).root_window;
        self.window(w).pixel_width == self.window(root).pixel_width
    }

    pub fn window_body_height(&mut self, w: Value, pixelwise: WindowBodyUnit) -> LispResult<i64> {
        let height = self.window(w).pixel_height
            - self.window_tab_line_height(w)
            - self.window_header_line_height(w)
            - if self.window_has_horizontal_scroll_bar(w) {
                self.window_scroll_bar_area_height(w)
            } else {
                0
            }
            - self.window_mode_line_height(w)
            - self.window_bottom_divider_width(w);
        let mut pixelwise = pixelwise;
        let mut denom = 1;
        let f = self.window(w).frame;
        if pixelwise == WindowBodyUnit::RemappedChars {
            if !self.find_symbol_value(sym::FACE_REMAPPING_ALIST).is_nil() {
                let face_id = self.lookup_named_face(None, f, sym::DEFAULT, true)?;
                if let Some(face) = self.face_from_id(f, face_id as usize)
                    && !face.font.is_nil()
                    && self.font_object(face.font).height != 0
                {
                    denom = i64::from(self.font_object(face.font).height);
                }
            } else {
                pixelwise = WindowBodyUnit::CanonicalChars;
            }
        }
        if pixelwise == WindowBodyUnit::CanonicalChars {
            denom = self.frame(f).geometry.line_height;
        }
        Ok((height / denom).max(0))
    }

    pub fn window_body_width(&mut self, w: Value, pixelwise: WindowBodyUnit) -> LispResult<i64> {
        let f = self.window(w).frame;
        let window_p = self.frame_window_p(f);
        let width = self.window(w).pixel_width
            - self.window_right_divider_width(w)
            - if self.window_has_vertical_scroll_bar(w) {
                self.window_scroll_bar_area_width(w)
            } else {
                i64::from(
                    !window_p
                        && !self.window_rightmost_p(w)
                        && self.window_right_divider_width(w) == 0,
                )
            }
            - self.window_margins_width(w)
            - if window_p {
                self.window_fringes_width(w)
            } else {
                0
            };
        let mut pixelwise = pixelwise;
        let mut denom = 1;
        if pixelwise == WindowBodyUnit::RemappedChars {
            if !self.find_symbol_value(sym::FACE_REMAPPING_ALIST).is_nil() {
                let face_id = self.lookup_named_face(None, f, sym::DEFAULT, true)?;
                if let Some(face) = self.face_from_id(f, face_id as usize)
                    && !face.font.is_nil()
                {
                    let font = self.font_object(face.font);
                    if font.average_width != 0 {
                        denom = i64::from(font.average_width);
                    } else if font.space_width != 0 {
                        denom = i64::from(font.space_width);
                    }
                }
            } else {
                pixelwise = WindowBodyUnit::CanonicalChars;
            }
        }
        if pixelwise == WindowBodyUnit::CanonicalChars {
            denom = self.frame(f).geometry.column_width;
        }
        Ok((width / denom).max(0))
    }

    pub fn window_mode_line_height(&mut self, w: Value) -> i64 {
        if self.window_wants_mode_line(w) {
            self.current_mode_line_height(w)
        } else {
            0
        }
    }

    pub fn window_header_line_height(&mut self, w: Value) -> i64 {
        if self.window_wants_header_line(w) {
            self.current_header_line_height(w)
        } else {
            0
        }
    }

    pub fn window_tab_line_height(&mut self, w: Value) -> i64 {
        if self.window_wants_tab_line(w) {
            self.current_tab_line_height(w)
        } else {
            0
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

    fn set_window_fringes(
        &mut self,
        w: Value,
        left_width: Value,
        right_width: Value,
        outside_margins: Value,
        persistent: Value,
    ) -> LispResult<bool> {
        if !self.window_frame_window_p(w) {
            return Ok(false);
        }
        let f = self.window(w).frame;
        let old_left = self.window_left_fringe_width(w);
        let old_right = self.window_right_fringe_width(w);
        let new_left = self.extract_dimension(left_width)?;
        let new_right = self.extract_dimension(right_width)?;
        let outside = !outside_margins.is_nil();
        let mut changed = false;
        let mut failed = false;
        let g = &self.frame(f).geometry;
        let room = self.window(w).pixel_width
            - self.window_margins_width(w)
            - self.window_scroll_bar_area_width(w)
            - self.window_right_divider_width(w)
            - if new_left == -1 {
                g.left_fringe_width
            } else {
                new_left
            }
            - if new_right == -1 {
                g.right_fringe_width
            } else {
                new_right
            };
        if room >= self.min_safe_window_pixel_width(w) {
            let win = self.window_mut(w);
            win.left_fringe_width = new_left;
            win.right_fringe_width = new_right;
            changed = new_left != old_left || new_right != old_right;
        } else {
            failed = true;
        }
        let win = self.window_mut(w);
        if outside != win.flags.fringes_outside_margins {
            win.flags.fringes_outside_margins = outside;
            changed = true;
        }
        if !failed {
            win.flags.fringes_persistent = !persistent.is_nil();
        }
        if changed {
            self.windows.windows_or_buffers_changed = 35;
        }
        Ok(changed)
    }

    fn set_window_scroll_bars(
        &mut self,
        w: Value,
        width: Value,
        vertical_type: Value,
        height: Value,
        horizontal_type: Value,
        persistent: Value,
    ) -> LispResult<bool> {
        if !self.window_frame_window_p(w) {
            return Ok(false);
        }
        let f = self.window(w).frame;
        let new_width = self.extract_dimension(width)?;
        let mut vertical_type = vertical_type;
        let mut changed = false;
        let mut failed = false;
        if new_width == 0 {
            vertical_type = Value::NIL;
        } else if !(vertical_type.is_nil()
            || vertical_type == sym::LEFT
            || vertical_type == sym::RIGHT
            || vertical_type == sym::T)
        {
            return Err(self.error("Invalid type of vertical scroll bar"));
        }
        let room = self.window(w).pixel_width
            - self.window_margins_width(w)
            - self.window_fringes_width(w)
            - self.window_right_divider_width(w)
            - if new_width == -1 {
                self.frame_scroll_bar_area_width(f)
            } else {
                new_width
            };
        if room >= self.min_safe_window_pixel_width(w) {
            changed = vertical_type != self.window(w).vertical_scroll_bar_type
                || new_width != self.window_scroll_bar_area_width(w);
            let win = self.window_mut(w);
            win.vertical_scroll_bar_type = vertical_type;
            win.scroll_bar_width = new_width;
        } else {
            failed = true;
        }
        if USE_HORIZONTAL_SCROLL_BARS {
            let new_height = self.extract_dimension(height)?;
            let mut horizontal_type = horizontal_type;
            if (self.window(w).mini && horizontal_type != sym::BOTTOM) || new_height == 0 {
                horizontal_type = Value::NIL;
            }
            if !(horizontal_type.is_nil()
                || horizontal_type == sym::BOTTOM
                || horizontal_type == sym::T)
            {
                return Err(self.error("Invalid type of horizontal scroll bar"));
            }
            let room = self.window(w).pixel_height
                - self.window_tab_line_height(w)
                - self.window_header_line_height(w)
                - self.window_mode_line_height(w)
                - if new_height == -1 {
                    self.frame_scroll_bar_area_height(f)
                } else {
                    new_height
                };
            if room >= self.window_frame_line_height(w) {
                changed = changed
                    || horizontal_type != self.window(w).horizontal_scroll_bar_type
                    || new_height != self.window_scroll_bar_area_height(w);
                let win = self.window_mut(w);
                win.horizontal_scroll_bar_type = horizontal_type;
                win.scroll_bar_height = new_height;
            } else {
                failed = true;
            }
        } else {
            self.window_mut(w).horizontal_scroll_bar_type = Value::NIL;
        }
        if !failed {
            self.window_mut(w).flags.scroll_bars_persistent = !persistent.is_nil();
        }
        if changed {
            self.wset_redisplay(w);
        }
        Ok(changed)
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
        self.buffer_mut(buffer).display_error_modiff = 0;
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
            let local = |l: &Lisp, var| l.buffer_local_value(var, buffer);
            if !self.window(window).flags.fringes_persistent {
                let (left, right, outside) = (
                    local(self, sym::LEFT_FRINGE_WIDTH),
                    local(self, sym::RIGHT_FRINGE_WIDTH),
                    local(self, sym::FRINGES_OUTSIDE_MARGINS),
                );
                self.set_window_fringes(window, left, right, outside, Value::NIL)?;
            }
            if !self.window(window).flags.scroll_bars_persistent {
                let (width, vertical, height, horizontal) = (
                    local(self, sym::SCROLL_BAR_WIDTH),
                    local(self, sym::VERTICAL_SCROLL_BAR),
                    local(self, sym::SCROLL_BAR_HEIGHT),
                    local(self, sym::HORIZONTAL_SCROLL_BAR),
                );
                self.set_window_scroll_bars(
                    window,
                    width,
                    vertical,
                    height,
                    horizontal,
                    Value::NIL,
                )?;
            }
            let (left, right) = (
                local(self, sym::LEFT_MARGIN_WIDTH),
                local(self, sym::RIGHT_MARGIN_WIDTH),
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

pub(crate) fn f_previous_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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

pub(crate) fn f_frame_first_window(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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

impl Lisp {
    pub fn window_outdated(&self, w: Value) -> bool {
        let win = self.window(w);
        let text = &self.buffer(win.contents).text;
        win.last_modified < text.modiff || win.last_overlay_modified < text.overlay_modiff
    }
}

pub(crate) fn f_pos_visible_in_window_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (pos, window, partially) = (arg(args, 0), arg(args, 1), arg(args, 2));
    let w = lisp.decode_live_window(window)?;
    let contents = lisp.window(w).contents;
    let top = lisp.marker(lisp.window(w).start).charpos as i64;
    let posint = if pos == sym::T {
        -1
    } else if !pos.is_nil() {
        lisp.fix_position(pos)?
    } else if w == lisp.windows.selected_window {
        lisp.cur().pt as i64
    } else {
        lisp.marker(lisp.window(w).pointm).charpos as i64
    };
    let (begv, zv) = {
        let b = lisp.buffer(contents);
        (b.begv as i64, b.zv as i64)
    };
    let mut in_window = Value::NIL;
    let mut visible = None;
    if (pos == sym::T || (posint >= top && posint <= zv)) && top >= begv && top <= zv {
        visible = lisp.pos_visible_p(w, posint)?;
    }
    let mut fully_p = true;
    if let Some(v) = visible {
        fully_p = v.rtop == 0 && v.rbot == 0;
        if !partially.is_nil() || fully_p {
            in_window = sym::T;
        }
    }
    if let Some(v) = visible
        && !in_window.is_nil()
        && !partially.is_nil()
    {
        let mut part = Value::NIL;
        if !fully_p {
            let items = [v.rtop, v.rbot, v.rowh, v.vpos].map(|n| Value::fixnum(i64::from(n)));
            part = lisp.list(&items);
        }
        let y = lisp.cons(Value::fixnum(i64::from(v.y)), part);
        in_window = lisp.cons(Value::fixnum(i64::from(v.x)), y);
    }
    Ok(in_window)
}

fn f_window_line_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (line, window) = (arg(args, 0), arg(args, 1));
    let w = lisp.decode_live_window(window)?;
    if lisp.noninteractive || lisp.window(w).flags.pseudo_window_p {
        return Ok(Value::NIL);
    }
    let contents = lisp.window(w).contents;
    if !lisp.is_buffer(contents) {
        return Err(lisp.wrong_type(sym::BUFFERP, contents));
    }
    let b = lisp.buffer(contents);
    if !lisp.window(w).flags.window_end_valid
        || lisp.windows.windows_or_buffers_changed != 0
        || b.clip_changed
        || b.prevent_redisplay_optimizations_p
        || lisp.window_outdated(w)
    {
        return Ok(Value::NIL);
    }
    let list4i = |lisp: &mut Lisp, items: [i64; 4]| lisp.list(&items.map(Value::fixnum));
    let current = WhichMatrix::Current;
    let row_at = |lisp: &Lisp, i: usize| {
        let r = &lisp.window_matrix(w, current).expect("current matrix").rows[i];
        (r.enabled_p, i64::from(r.y), i64::from(r.height))
    };
    let (mut row, mut i, max_y);
    if line.is_nil() {
        let vpos = lisp.window(w).cursor.vpos;
        let nrows = i64::from(
            lisp.window_matrix(w, current)
                .expect("current matrix")
                .nrows,
        );
        if vpos < 0 || vpos >= nrows || !row_at(lisp, vpos as usize).0 {
            return Ok(Value::NIL);
        }
        row = vpos as usize;
        i = vpos;
        max_y = lisp.window_text_bottom_y(w);
    } else {
        if line == sym::TAB_LINE {
            if !lisp.window_wants_tab_line(w) {
                return Ok(Value::NIL);
            }
            let (enabled, _, height) = row_at(lisp, 0);
            return Ok(if enabled {
                list4i(lisp, [height, 0, 0, 0])
            } else {
                Value::NIL
            });
        }
        if line == sym::HEADER_LINE {
            if !lisp.window_wants_header_line(w) {
                return Ok(Value::NIL);
            }
            let header = lisp
                .window_matrix(w, current)
                .expect("current matrix")
                .header_line_row();
            let (enabled, _, height) = row_at(lisp, header);
            return Ok(if enabled {
                list4i(lisp, [height, 0, 0, 0])
            } else {
                Value::NIL
            });
        }
        if line == sym::MODE_LINE {
            let mode = lisp
                .window_matrix(w, current)
                .expect("current matrix")
                .mode_line_row();
            let (enabled, _, height) = row_at(lisp, mode);
            if !enabled {
                return Ok(Value::NIL);
            }
            let y = lisp.window_tab_line_height(w)
                + lisp.window_header_line_height(w)
                + lisp.window_text_bottom_y(w);
            return Ok(list4i(lisp, [height, 0, y, 0]));
        }
        let mut n = lisp.check_fixnum(line)?;
        row = lisp
            .window_matrix(w, current)
            .expect("current matrix")
            .first_text_row();
        let end_row = lisp.matrix_bottom_text_row(w, current);
        max_y = lisp.window_text_bottom_y(w);
        i = 0;
        while (n < 0 || i < n) && row <= end_row && {
            let (enabled, y, height) = row_at(lisp, row);
            enabled && y + height < max_y
        } {
            row += 1;
            i += 1;
        }
        if row > end_row || !row_at(lisp, row).0 {
            return Ok(Value::NIL);
        }
        n += 1;
        if n < 0 {
            if -n > i {
                return Ok(Value::NIL);
            }
            row = (row as i64 + n) as usize;
            i += n;
        }
    }
    let (_, y, height) = row_at(lisp, row);
    let crop = 0.max(y + height - max_y);
    Ok(list4i(lisp, [height + 0.min(y) - crop, i, y, crop]))
}

fn canon_from_pixel(lisp: &mut Lisp, pixels: i64, unit: i64) -> Value {
    if pixels % unit != 0 {
        lisp.float(pixels as f64 / unit as f64)
    } else {
        Value::fixnum(pixels / unit)
    }
}

fn check_number(lisp: &mut Lisp, v: Value) -> LispResult<()> {
    if v.is_fixnum() || v.is_float() || lisp.is_bignum(v) {
        Ok(())
    } else {
        Err(lisp.wrong_type(sym::NUMBERP, v))
    }
}

fn f_coordinates_in_window_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (coordinates, window) = (args[0], arg(args, 1));
    let w = lisp.decode_live_window(window)?;
    let f = lisp.window(w).frame;
    if !coordinates.is_cons() {
        return Err(lisp.wrong_type(sym::CONSP, coordinates));
    }
    let lx = lisp.xcar(coordinates);
    let ly = lisp.xcdr(coordinates);
    check_number(lisp, lx)?;
    check_number(lisp, ly)?;
    let g = lisp.frame(f).geometry.clone();
    let mut x =
        i64::from((lisp.xfloatint(lx) * g.column_width as f64) as i32) + g.internal_border_width;
    let mut y =
        i64::from((lisp.xfloatint(ly) * g.line_height as f64) as i32) + g.internal_border_width;
    Ok(match lisp.coordinates_in_window(w, x, y) {
        WindowPart::Nothing => Value::NIL,
        WindowPart::Text => {
            x -= lisp.window_box_left(w, GlyphRowArea::Text);
            y -= lisp.window_top_edge_y(w);
            let cx = canon_from_pixel(lisp, x, g.column_width);
            let cy = canon_from_pixel(lisp, y, g.line_height);
            lisp.cons(cx, cy)
        }
        WindowPart::ModeLine => sym::MODE_LINE,
        WindowPart::VerticalBorder => sym::VERTICAL_LINE,
        WindowPart::HeaderLine => sym::HEADER_LINE,
        WindowPart::TabLine => sym::TAB_LINE,
        WindowPart::LeftFringe => sym::LEFT_FRINGE,
        WindowPart::RightFringe => sym::RIGHT_FRINGE,
        WindowPart::LeftMargin => sym::LEFT_MARGIN,
        WindowPart::RightMargin => sym::RIGHT_MARGIN,
        WindowPart::VerticalScrollBar | WindowPart::HorizontalScrollBar => Value::NIL,
        WindowPart::RightDivider => sym::RIGHT_DIVIDER,
        WindowPart::BottomDivider => sym::BOTTOM_DIVIDER,
    })
}

fn f_window_at(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (x, y, frame) = (args[0], args[1], arg(args, 2));
    let f = lisp.decode_live_frame(frame)?;
    lisp.check_integer(x)?;
    lisp.check_integer(y)?;
    let g = lisp.frame(f).geometry.clone();
    let (xi, yi) = (x.as_fixnum().unwrap_or(0), y.as_fixnum().unwrap_or(0));
    if xi < 0 || xi > g.pixel_width || yi < 0 || yi > g.pixel_height {
        return Ok(Value::NIL);
    }
    let px =
        i64::from((lisp.xfloatint(x) * g.column_width as f64) as i32) + g.internal_border_width;
    let py = i64::from((lisp.xfloatint(y) * g.line_height as f64) as i32) + g.internal_border_width;
    let mut part = WindowPart::Nothing;
    Ok(lisp.window_from_coordinates(f, px, py, &mut part, false))
}

fn f_window_vscroll(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (window, pixels_p) = (arg(args, 0), arg(args, 1));
    let w = lisp.decode_live_window(window)?;
    let f = lisp.window(w).frame;
    if !lisp.frame_window_p(f) {
        return Ok(Value::fixnum(0));
    }
    let y = -lisp.window(w).vscroll;
    if !pixels_p.is_nil() {
        return Ok(Value::fixnum(y));
    }
    let lh = lisp.frame(f).geometry.line_height;
    Ok(if y % lh != 0 {
        lisp.float(y as f64 / lh as f64)
    } else {
        Value::fixnum(y / lh)
    })
}

fn f_set_window_vscroll(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (window, vscroll, pixels_p, preserve_vscroll_p) =
        (arg(args, 0), args[1], arg(args, 2), arg(args, 3));
    let w = lisp.decode_live_window(window)?;
    let f = lisp.window(w).frame;
    if !(vscroll.is_fixnum() || vscroll.is_float() || lisp.is_bignum(vscroll)) {
        return Err(lisp.wrong_type(sym::NUMBERP, vscroll));
    }
    if lisp.frame_window_p(f) {
        let old_dy = lisp.window(w).vscroll;
        let amount = lisp.xfloatint(vscroll);
        let scaled = if pixels_p.is_nil() {
            lisp.frame(f).geometry.line_height as f64 * amount
        } else {
            amount
        };
        let vscroll = (-scaled as i64).min(0);
        lisp.window_mut(w).vscroll = vscroll;
        if vscroll != old_dy {
            if vscroll < 0 && vscroll < old_dy {
                lisp.adjust_frame_glyphs(f);
            }
            let contents = lisp.window(w).contents;
            lisp.buffer_mut(contents).prevent_redisplay_optimizations_p = true;
            lisp.wset_redisplay(w);
        }
        lisp.window_mut(w).flags.preserve_vscroll_p = !preserve_vscroll_p.is_nil();
    }
    f_window_vscroll(lisp, &[window, pixels_p])
}

fn f_window_body_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    let unit = window_body_unit_from_symbol(arg(args, 1));
    Ok(Value::fixnum(lisp.window_body_width(w, unit)?))
}

fn f_window_body_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    let unit = window_body_unit_from_symbol(arg(args, 1));
    Ok(Value::fixnum(lisp.window_body_height(w, unit)?))
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

fn f_window_mode_line_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window_mode_line_height(w)))
}

fn f_window_header_line_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window_header_line_height(w)))
}

fn f_window_tab_line_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window_tab_line_height(w)))
}

fn f_window_right_divider_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window_right_divider_width(w)))
}

fn f_window_bottom_divider_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window_bottom_divider_width(w)))
}

fn f_window_scroll_bar_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window_scroll_bar_area_width(w)))
}

fn f_window_scroll_bar_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window_scroll_bar_area_height(w)))
}

fn f_window_hscroll(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window(w).hscroll))
}

pub(crate) fn f_window_point(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    Ok(Value::fixnum(lisp.window_point(w)))
}

pub(crate) fn f_window_old_point(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    let m = lisp.window(w).old_pointm;
    Ok(lisp.marker_position_value(m).unwrap_or(Value::NIL))
}

pub(crate) fn f_window_start(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    let m = lisp.window(w).start;
    Ok(lisp.marker_position_value(m).unwrap_or(Value::NIL))
}

pub(crate) fn f_set_window_point(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(args[0])?;
    let mut pos = args[1];
    if w == lisp.windows.selected_window {
        let contents = lisp.window(w).contents;
        if contents == lisp.current_buffer {
            crate::editfns::f_goto_char(lisp, &[pos])?;
        } else {
            let old_buffer = lisp.current_buffer;
            if lisp.is_marker(pos) {
                pos = Value::fixnum(lisp.marker_position_checked(pos)? as i64);
            } else if !pos.is_fixnum() {
                return Err(lisp.wrong_type(sym::INTEGER_OR_MARKER_P, pos));
            }
            lisp.set_buffer_internal(contents);
            let r = crate::editfns::f_goto_char(lisp, &[pos]);
            lisp.set_buffer_internal(old_buffer);
            r?;
        }
    } else {
        let (pointm, contents) = {
            let win = lisp.window(w);
            (win.pointm, win.contents)
        };
        lisp.set_marker(pointm, pos, contents, true)?;
        lisp.wset_redisplay(w);
    }
    Ok(pos)
}

pub(crate) fn f_set_window_start(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(args[0])?;
    let (start, contents) = {
        let win = lisp.window(w);
        (win.start, win.contents)
    };
    lisp.set_marker(start, args[1], contents, true)?;
    let win = lisp.window_mut(w);
    win.flags.start_at_line_beg = false;
    if arg(args, 2).is_nil() {
        win.flags.force_start = true;
    }
    lisp.wset_update_mode_line(w);
    lisp.window_mut(w).flags.window_end_valid = false;
    lisp.wset_redisplay(w);
    Ok(args[1])
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

fn f_set_window_margins(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(args[0])?;
    if lisp.set_window_margins(w, args[1], arg(args, 2))? {
        lisp.apply_window_adjustment(w);
        Ok(sym::T)
    } else {
        Ok(Value::NIL)
    }
}

fn f_set_window_fringes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(args[0])?;
    if lisp.set_window_fringes(w, args[1], arg(args, 2), arg(args, 3), arg(args, 4))? {
        lisp.apply_window_adjustment(w);
        Ok(sym::T)
    } else {
        Ok(Value::NIL)
    }
}

fn f_set_window_scroll_bars(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(args[0])?;
    if lisp.set_window_scroll_bars(
        w,
        arg(args, 1),
        arg(args, 2),
        arg(args, 3),
        arg(args, 4),
        arg(args, 5),
    )? {
        lisp.apply_window_adjustment(w);
        Ok(sym::T)
    } else {
        Ok(Value::NIL)
    }
}

fn f_window_scroll_bars(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let w = lisp.decode_live_window(arg(args, 0))?;
    let win = lisp.window(w);
    let width = if win.scroll_bar_width >= 0 {
        Value::fixnum(win.scroll_bar_width)
    } else {
        Value::NIL
    };
    let height = if win.scroll_bar_height >= 0 {
        Value::fixnum(win.scroll_bar_height)
    } else {
        Value::NIL
    };
    let (vertical, horizontal) = (win.vertical_scroll_bar_type, win.horizontal_scroll_bar_type);
    let persistent = lisp.bool(win.flags.scroll_bars_persistent);
    let cols = Value::fixnum(lisp.window_scroll_bar_cols(w));
    let lines = Value::fixnum(lisp.window_scroll_bar_lines(w));
    Ok(lisp.list(&[width, cols, vertical, height, lines, horizontal, persistent]))
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

    pub(crate) fn buffer_local_value(&self, symbol: Value, buffer: Value) -> Value {
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
                self.do_switch_frame(target, false, Value::NIL)?;
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

pub(crate) fn restore_focus_frame(lisp: &mut Lisp, frame_and_focus_frame: Value) -> LispResult<()> {
    let frame = lisp.car(frame_and_focus_frame)?;
    let focus_frame = lisp.cdr(frame_and_focus_frame)?;
    if lisp.frame_live_p(frame) && (focus_frame.is_nil() || lisp.frame_live_p(focus_frame)) {
        let frame_focus_frame = lisp.frame(frame).focus_frame;
        crate::frame::f_redirect_frame_focus(lisp, &[frame, focus_frame])?;
        if lisp.plain_value(sym::FOCUS_FOLLOWS_MOUSE) == sym::AUTO_RAISE
            && focus_frame.is_nil()
            && lisp.frames.selected_frame == frame_focus_frame
        {
            lisp.funcall(sym::SELECT_FRAME_SET_INPUT_FOCUS, &[frame, Value::NIL])?;
        }
    }
    Ok(())
}

fn f_window_fringes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let window = lisp.decode_live_window(arg(args, 0))?;
    let left = lisp.window_left_fringe_width(window);
    let right = lisp.window_right_fringe_width(window);
    let w = lisp.window(window);
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

pub(crate) fn f_set_window_configuration(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (configuration, dont_set_frame, dont_set_miniwindow) =
        (args[0], arg(args, 1), arg(args, 2));
    let data = lisp.check_window_configuration(configuration)?;
    let base = lisp.stack.len();
    lisp.stack.push(configuration);
    let r = lisp.set_window_configuration(&data, dont_set_frame, dont_set_miniwindow);
    lisp.stack.truncate(base);
    r
}

pub(crate) fn f_current_window_configuration(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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

impl Lisp {
    pub(crate) fn window_sub_list(&mut self, window: Value, windows: Value) -> Value {
        let mut windows = windows;
        let mut window = window;
        while !window.is_nil() {
            let contents = self.window(window).contents;
            if self.is_window(contents) {
                windows = self.window_sub_list(contents, windows);
            } else {
                windows = self.cons(window, windows);
            }
            window = self.window(window).next;
        }
        windows
    }

    fn window_change_record_windows(
        &mut self,
        window: Value,
        stamp: i64,
        number: i64,
    ) -> LispResult<i64> {
        let mut number = number;
        let mut w = window;
        while !w.is_nil() {
            let contents = self.window(w).contents;
            if self.is_window(contents) {
                number = self.window_change_record_windows(contents, stamp, number)?;
            } else {
                number += 1;
                let body_width = self.window_body_width(w, WindowBodyUnit::Pixels)?;
                let body_height = self.window_body_height(w, WindowBodyUnit::Pixels)?;
                let win = self.window_mut(w);
                win.change_stamp = stamp;
                win.old_buffer = win.contents;
                win.old_pixel_width = win.pixel_width;
                win.old_pixel_height = win.pixel_height;
                win.old_body_pixel_width = body_width;
                win.old_body_pixel_height = body_height;
            }
            w = self.window(w).next;
        }
        Ok(number)
    }

    fn window_change_record(&mut self) -> LispResult<()> {
        if self.windows.window_change_record_frames {
            for f in self.frames.list.clone() {
                let selected = self.frame(f).selected_window;
                let fr = self.frame_mut(f);
                fr.old_selected_window = selected;
                fr.change_stamp += 1;
                if fr.change_stamp == 0 {
                    fr.change_stamp = 1;
                }
                let (root, stamp) = (fr.root_window, fr.change_stamp);
                let n = self.window_change_record_windows(root, stamp, 0)?;
                let fr = self.frame_mut(f);
                fr.number_of_windows = n;
                fr.flags.window_change = false;
                fr.flags.window_state_change = false;
            }
        }
        self.windows.old_selected_window = self.windows.selected_window;
        self.frames.old_selected_frame = self.frames.selected_frame;
        Ok(())
    }

    fn run_window_change_functions_locally(
        &mut self,
        symbol: Value,
        buffer: Value,
        window: Value,
    ) -> LispResult<()> {
        if self.buffer_local_get(buffer, symbol).is_some() {
            let mut funs = self.buffer_local_value(symbol, buffer);
            crate::buffer::f_set_buffer(self, &[buffer])?;
            let base = self.stack.len();
            self.stack.push(funs);
            while funs.is_cons() {
                let fun = self.xcar(funs);
                if fun != sym::T {
                    self.windows.window_change_record_frames = true;
                    self.safe_calln(fun, &[window])?;
                }
                funs = self.xcdr(funs);
            }
            self.stack.truncate(base);
        }
        Ok(())
    }

    fn run_window_change_functions_globally(
        &mut self,
        symbol: Value,
        frame: Value,
    ) -> LispResult<()> {
        let mut funs = crate::data::f_default_value(self, &[symbol])?;
        let base = self.stack.len();
        self.stack.push(funs);
        while funs.is_cons() {
            let fun = self.xcar(funs);
            if fun != sym::T {
                self.windows.window_change_record_frames = true;
                self.safe_calln(fun, &[frame])?;
            }
            funs = self.xcdr(funs);
        }
        self.stack.truncate(base);
        Ok(())
    }

    fn run_funs(&mut self, funs: Value) -> LispResult<()> {
        let base = self.stack.len();
        self.stack.push(funs);
        let mut funs = funs;
        while funs.is_cons() {
            let fun = self.xcar(funs);
            if fun != sym::T {
                self.funcall(fun, &[])?;
            }
            funs = self.xcdr(funs);
        }
        self.stack.truncate(base);
        Ok(())
    }

    pub fn run_window_configuration_change_hook(&mut self, f: Value) -> LispResult<()> {
        let count = self.specpdl_index();
        let global_wcch =
            crate::data::f_default_value(self, &[sym::WINDOW_CONFIGURATION_CHANGE_HOOK])?;
        let base = self.stack.len();
        self.stack.push(global_wcch);
        let frame = f;
        if self.find_symbol_value(sym::RUN_HOOKS).is_nil()
            || !self.frame(f).flags.can_set_window_size
            || !self.frame(f).flags.after_make_frame
        {
            self.stack.truncate(base);
            return Ok(());
        }
        let selected_buffer = self.window(self.windows.selected_window).contents;
        if self.current_buffer != selected_buffer {
            self.record_unwind_current_buffer();
            crate::buffer::f_set_buffer(self, &[selected_buffer])?;
        }
        if self.frames.selected_frame != f {
            let sf = self.frames.selected_frame;
            self.record_unwind(select_frame_norecord, sf);
            select_frame_norecord(self, frame)?;
        }
        let windows = f_window_list(self, &[frame, sym::LAMBDA, Value::NIL])?;
        self.stack.push(windows);
        let mut tail = windows;
        while tail.is_cons() {
            let window = self.xcar(tail);
            let buffer = self.window(window).contents;
            let local = crate::data::f_local_variable_p(
                self,
                &[sym::WINDOW_CONFIGURATION_CHANGE_HOOK, buffer],
            )?;
            if !local.is_nil() {
                let inner_count = self.specpdl_index();
                let sw = self.windows.selected_window;
                self.record_unwind(select_window_norecord, sw);
                select_window_norecord(self, window)?;
                let funs = crate::buffer::f_buffer_local_value(
                    self,
                    &[sym::WINDOW_CONFIGURATION_CHANGE_HOOK, buffer],
                )?;
                self.run_funs(funs)?;
                self.unbind_to(inner_count)?;
            }
            tail = self.xcdr(tail);
        }
        self.run_funs(global_wcch)?;
        self.stack.truncate(base);
        self.unbind_to(count)
    }

    pub fn run_window_change_functions(&mut self) -> LispResult<()> {
        let selected_frame_change = self.frames.selected_frame != self.frames.old_selected_frame;
        let mut run_window_state_change_hook = false;
        let count = self.specpdl_index();
        self.windows.window_change_record_frames = false;
        self.record_unwind(window_change_record_unwind, Value::NIL);
        self.specbind(sym::INHIBIT_REDISPLAY, sym::T)?;
        for frame in self.frames.list.clone() {
            let f = frame;
            if !self.frame_live_p(f) {
                continue;
            }
            let root = self.frame(f).root_window;
            let frame_window_change = self.frame(f).flags.window_change;
            let mut frame_buffer_change = false;
            let mut frame_size_change = false;
            let old_selected_frame = self.frames.old_selected_frame;
            let frame_selected_change = selected_frame_change
                && (frame == old_selected_frame || frame == self.frames.selected_frame);
            let frame_selected_window_change =
                self.frame(f).old_selected_window != self.frame(f).selected_window;
            let frame_window_state_change = self.frame(f).flags.window_state_change;
            let fl = &self.frame(f).flags;
            if !fl.can_set_window_size
                || !fl.after_make_frame
                || fl.tooltip
                || !(frame_window_change
                    || frame_selected_change
                    || frame_selected_window_change
                    || frame_window_state_change)
            {
                continue;
            }
            let list = self.window_sub_list(root, Value::NIL);
            let windows = crate::fns::f_nreverse(self, &[list])?;
            let base = self.stack.len();
            self.stack.push(windows);
            let mut number_of_windows = 0;
            let mut tail = windows;
            while tail.is_cons() {
                let window = self.xcar(tail);
                tail = self.xcdr(tail);
                let count1 = self.specpdl_index();
                number_of_windows += 1;
                if !self.window_live_p(window) {
                    continue;
                }
                let buffer = self.window(window).contents;
                let (old_buffer, change_stamp) = {
                    let win = self.window(window);
                    (win.old_buffer, win.change_stamp)
                };
                let window_buffer_change = frame_window_change
                    && (buffer != old_buffer || change_stamp != self.frame(f).change_stamp);
                let window_size_change = frame_window_change
                    && (window_buffer_change || {
                        let (pw, ph, opw, oph, obw, obh) = {
                            let win = self.window(window);
                            (
                                win.pixel_width,
                                win.pixel_height,
                                win.old_pixel_width,
                                win.old_pixel_height,
                                win.old_body_pixel_width,
                                win.old_body_pixel_height,
                            )
                        };
                        pw != opw
                            || ph != oph
                            || self.window_body_width(window, WindowBodyUnit::Pixels)? != obw
                            || self.window_body_height(window, WindowBodyUnit::Pixels)? != obh
                    });
                frame_buffer_change = frame_buffer_change || window_buffer_change;
                frame_size_change = frame_size_change || window_size_change;
                self.record_unwind_current_buffer();
                if window_buffer_change {
                    if self.is_buffer(old_buffer) && self.buffer_live_p(old_buffer) {
                        self.run_window_change_functions_locally(
                            sym::WINDOW_BUFFER_CHANGE_FUNCTIONS,
                            old_buffer,
                            window,
                        )?;
                    }
                    self.run_window_change_functions_locally(
                        sym::WINDOW_BUFFER_CHANGE_FUNCTIONS,
                        buffer,
                        window,
                    )?;
                }
                if window_size_change && self.window_live_p(window) {
                    self.run_window_change_functions_locally(
                        sym::WINDOW_SIZE_CHANGE_FUNCTIONS,
                        buffer,
                        window,
                    )?;
                }
                let selection_change = (frame_selected_change
                    && (window == self.windows.old_selected_window
                        || window == self.windows.selected_window))
                    || (frame_selected_window_change
                        && (window == self.frame(f).old_selected_window
                            || window == self.frame(f).selected_window));
                if selection_change && self.window_live_p(window) {
                    self.run_window_change_functions_locally(
                        sym::WINDOW_SELECTION_CHANGE_FUNCTIONS,
                        buffer,
                        window,
                    )?;
                }
                let selection_change = (frame_selected_change
                    && (window == self.windows.old_selected_window
                        || window == self.windows.selected_window))
                    || (frame_selected_window_change
                        && (window == self.frame(f).old_selected_window
                            || window == self.frame(f).selected_window));
                if (window_buffer_change || window_size_change || selection_change)
                    && self.window_live_p(window)
                {
                    self.run_window_change_functions_locally(
                        sym::WINDOW_STATE_CHANGE_FUNCTIONS,
                        buffer,
                        window,
                    )?;
                }
                self.unbind_to(count1)?;
            }
            self.stack.truncate(base);
            let window_deleted = number_of_windows < self.frame(f).number_of_windows;
            if (frame_buffer_change || window_deleted) && self.frame_live_p(f) {
                self.run_window_change_functions_globally(
                    sym::WINDOW_BUFFER_CHANGE_FUNCTIONS,
                    frame,
                )?;
            }
            if frame_size_change && self.frame_live_p(f) {
                self.run_window_change_functions_globally(
                    sym::WINDOW_SIZE_CHANGE_FUNCTIONS,
                    frame,
                )?;
            }
            if (frame_selected_change || frame_selected_window_change) && self.frame_live_p(f) {
                self.run_window_change_functions_globally(
                    sym::WINDOW_SELECTION_CHANGE_FUNCTIONS,
                    frame,
                )?;
            }
            if (frame_selected_change
                || frame_selected_window_change
                || frame_buffer_change
                || window_deleted
                || frame_size_change
                || frame_window_state_change)
                && self.frame_live_p(f)
            {
                self.run_window_change_functions_globally(
                    sym::WINDOW_STATE_CHANGE_FUNCTIONS,
                    frame,
                )?;
                run_window_state_change_hook = true;
                self.windows.window_change_record_frames = true;
            }
            if (frame_size_change || window_deleted) && self.frame_live_p(f) {
                self.run_window_configuration_change_hook(f)?;
            }
        }
        if run_window_state_change_hook
            && !self
                .find_symbol_value(sym::WINDOW_STATE_CHANGE_HOOK)
                .is_nil()
        {
            self.safe_run_hooks(sym::WINDOW_STATE_CHANGE_HOOK)?;
        }
        self.unbind_to(count)
    }
}

fn window_change_record_unwind(lisp: &mut Lisp, _: Value) -> LispResult<()> {
    lisp.window_change_record()
}

fn select_window_norecord(lisp: &mut Lisp, window: Value) -> LispResult<()> {
    if lisp.window_live_p(window) {
        f_select_window(lisp, &[window, sym::T])?;
    }
    Ok(())
}

fn select_frame_norecord(lisp: &mut Lisp, frame: Value) -> LispResult<()> {
    if lisp.frame_live_p(frame) {
        crate::frame::f_select_frame(lisp, &[frame, sym::T])?;
    }
    Ok(())
}
