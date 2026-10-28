use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::{Vectorlike, Weakness};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::terminal::OutputMethod;
use crate::value::Value;

pub const FACE_TTY_DEFAULT_FG_COLOR: i64 = -2;
pub const FACE_TTY_DEFAULT_BG_COLOR: i64 = -3;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum VerticalScrollBarType {
    #[default]
    None,
    Left,
    Right,
}

#[derive(Clone, Debug, Default)]
pub struct FrameFlags {
    pub visible: bool,
    pub iconified: bool,
    pub garbaged: bool,
    pub wants_modeline: bool,
    pub auto_raise: bool,
    pub auto_lower: bool,
    pub no_split: bool,
    pub explicit_name: bool,
    pub window_change: bool,
    pub window_state_change: bool,
    pub redisplay: bool,
    pub can_set_window_size: bool,
    pub after_make_frame: bool,
    pub new_size_p: bool,
    pub select_mini_window_flag: bool,
    pub no_special_glyphs: bool,
    pub no_accept_focus: bool,
    pub undecorated: bool,
    pub tooltip: bool,
    pub face_change: bool,
    pub default_face_done_p: bool,
    pub pointer_invisible: bool,
    pub was_invisible: bool,
    pub horizontal_scroll_bars: bool,
}

#[derive(Clone, Debug, Default)]
pub struct FrameGeometry {
    pub text_cols: i64,
    pub text_lines: i64,
    pub text_width: i64,
    pub text_height: i64,
    pub total_cols: i64,
    pub total_lines: i64,
    pub pixel_width: i64,
    pub pixel_height: i64,
    pub new_width: i64,
    pub new_height: i64,
    pub left_pos: i64,
    pub top_pos: i64,
    pub border_width: i64,
    pub child_frame_border_width: i64,
    pub internal_border_width: i64,
    pub right_divider_width: i64,
    pub bottom_divider_width: i64,
    pub left_fringe_width: i64,
    pub right_fringe_width: i64,
    pub menu_bar_lines: i64,
    pub menu_bar_height: i64,
    pub tab_bar_lines: i64,
    pub tab_bar_height: i64,
    pub tool_bar_lines: i64,
    pub tool_bar_height: i64,
    pub column_width: i64,
    pub line_height: i64,
    pub config_scroll_bar_width: i64,
    pub config_scroll_bar_cols: i64,
    pub config_scroll_bar_height: i64,
    pub config_scroll_bar_lines: i64,
    pub extra_line_spacing: i64,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub name: Value,
    pub icon_name: Value,
    pub title: Value,
    pub parent_frame: Value,
    pub last_mouse_device: Value,
    pub focus_frame: Value,
    pub root_window: Value,
    pub selected_window: Value,
    pub old_selected_window: Value,
    pub minibuffer_window: Value,
    pub param_alist: Value,
    pub scroll_bars: Value,
    pub condemned_scroll_bars: Value,
    pub menu_bar_items: Value,
    pub face_hash_table: Value,
    pub menu_bar_vector: Value,
    pub buffer_predicate: Value,
    pub buffer_list: Value,
    pub buried_buffer_list: Value,
    pub menu_bar_window: Value,
    pub tab_bar_window: Value,
    pub desired_tab_bar_string: Value,
    pub current_tab_bar_string: Value,
    pub tool_bar_window: Value,
    pub desired_tool_bar_string: Value,
    pub current_tool_bar_string: Value,
    pub tool_bar_position: Value,
    pub font_data: Value,
    pub tab_bar_items: Value,
    pub tool_bar_items: Value,
    pub terminal: Value,
    pub id: i64,
    pub output_method: OutputMethod,
    pub vertical_scroll_bar_type: VerticalScrollBarType,
    pub flags: FrameFlags,
    pub geometry: FrameGeometry,
    pub change_stamp: i64,
    pub number_of_windows: i64,
    pub last_tab_bar_item: i64,
    pub last_tool_bar_item: i64,
    pub background_pixel: i64,
    pub foreground_pixel: i64,
    pub alpha: [f64; 2],
    pub alpha_background: f64,
    pub face_cache: Option<Box<crate::xfaces::FaceCache>>,
}

impl Frame {
    fn allocate() -> Frame {
        Frame {
            name: Value::NIL,
            icon_name: Value::NIL,
            title: Value::NIL,
            parent_frame: Value::NIL,
            last_mouse_device: Value::NIL,
            focus_frame: Value::NIL,
            root_window: Value::NIL,
            selected_window: Value::NIL,
            old_selected_window: Value::NIL,
            minibuffer_window: Value::NIL,
            param_alist: Value::NIL,
            scroll_bars: Value::NIL,
            condemned_scroll_bars: Value::NIL,
            menu_bar_items: Value::NIL,
            face_hash_table: Value::NIL,
            menu_bar_vector: Value::NIL,
            buffer_predicate: Value::NIL,
            buffer_list: Value::NIL,
            buried_buffer_list: Value::NIL,
            menu_bar_window: Value::NIL,
            tab_bar_window: Value::NIL,
            desired_tab_bar_string: Value::NIL,
            current_tab_bar_string: Value::NIL,
            tool_bar_window: Value::NIL,
            desired_tool_bar_string: Value::NIL,
            current_tool_bar_string: Value::NIL,
            tool_bar_position: Value::NIL,
            font_data: Value::NIL,
            tab_bar_items: Value::NIL,
            tool_bar_items: Value::NIL,
            terminal: Value::NIL,
            id: 0,
            output_method: OutputMethod::Initial,
            vertical_scroll_bar_type: VerticalScrollBarType::None,
            flags: FrameFlags::default(),
            geometry: FrameGeometry::default(),
            change_stamp: 0,
            number_of_windows: 0,
            last_tab_bar_item: 0,
            last_tool_bar_item: 0,
            background_pixel: 0,
            foreground_pixel: 0,
            alpha: [0.0; 2],
            alpha_background: 0.0,
            face_cache: None,
        }
    }

    pub fn trace(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.name,
            self.icon_name,
            self.title,
            self.parent_frame,
            self.last_mouse_device,
            self.focus_frame,
            self.root_window,
            self.selected_window,
            self.old_selected_window,
            self.minibuffer_window,
            self.param_alist,
            self.scroll_bars,
            self.condemned_scroll_bars,
            self.menu_bar_items,
            self.face_hash_table,
            self.menu_bar_vector,
            self.buffer_predicate,
            self.buffer_list,
            self.buried_buffer_list,
            self.menu_bar_window,
            self.tab_bar_window,
            self.desired_tab_bar_string,
            self.current_tab_bar_string,
            self.tool_bar_window,
            self.desired_tool_bar_string,
            self.current_tool_bar_string,
            self.tool_bar_position,
            self.font_data,
            self.tab_bar_items,
            self.tool_bar_items,
            self.terminal,
        ]);
        if let Some(cache) = &self.face_cache {
            cache.trace(out);
        }
    }

    pub fn window_p(&self) -> bool {
        match self.output_method {
            OutputMethod::Initial => false,
        }
    }

    pub fn termcap_p(&self) -> bool {
        false
    }
}

pub struct Frames {
    pub list: Vec<Value>,
    pub selected_frame: Value,
    pub old_selected_frame: Value,
    pub last_nonminibuf_frame: Value,
    pub next_id: i64,
    pub tty_frame_count: i64,
    pub delayed_size_change: bool,
    pub frame_garbaged: bool,
}

impl Default for Frames {
    fn default() -> Self {
        Frames {
            list: Vec::new(),
            selected_frame: Value::NIL,
            old_selected_frame: Value::NIL,
            last_nonminibuf_frame: Value::NIL,
            next_id: 1,
            tty_frame_count: 0,
            delayed_size_change: false,
            frame_garbaged: false,
        }
    }
}

impl Frames {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&self.list);
        out.push(self.selected_frame);
        out.push(self.old_selected_frame);
        out.push(self.last_nonminibuf_frame);
    }
}

const FRAME_PARMS: &[Value] = &[
    sym::AUTO_RAISE,
    sym::AUTO_LOWER,
    sym::BACKGROUND_COLOR,
    sym::BORDER_COLOR,
    sym::BORDER_WIDTH,
    sym::CURSOR_COLOR,
    sym::CURSOR_TYPE,
    sym::FONT,
    sym::FOREGROUND_COLOR,
    sym::ICON_NAME,
    sym::ICON_TYPE,
    sym::CHILD_FRAME_BORDER_WIDTH,
    sym::INTERNAL_BORDER_WIDTH,
    sym::RIGHT_DIVIDER_WIDTH,
    sym::BOTTOM_DIVIDER_WIDTH,
    sym::MENU_BAR_LINES,
    sym::MOUSE_COLOR,
    sym::NAME,
    sym::SCROLL_BAR_WIDTH,
    sym::SCROLL_BAR_HEIGHT,
    sym::TITLE,
    sym::UNSPLITTABLE,
    sym::VERTICAL_SCROLL_BARS,
    sym::HORIZONTAL_SCROLL_BARS,
    sym::VISIBILITY,
    sym::TAB_BAR_LINES,
    sym::TOOL_BAR_LINES,
    sym::SCROLL_BAR_FOREGROUND,
    sym::SCROLL_BAR_BACKGROUND,
    sym::SCREEN_GAMMA,
    sym::LINE_SPACING,
    sym::LEFT_FRINGE,
    sym::RIGHT_FRINGE,
    sym::WAIT_FOR_WM,
    sym::FULLSCREEN,
    sym::FONT_BACKEND,
    sym::ALPHA,
    sym::STICKY,
    sym::TOOL_BAR_POSITION,
    sym::INHIBIT_DOUBLE_BUFFERING,
    sym::UNDECORATED,
    sym::PARENT_FRAME,
    sym::SKIP_TASKBAR,
    sym::NO_FOCUS_ON_MAP,
    sym::NO_ACCEPT_FOCUS,
    sym::Z_GROUP,
    sym::OVERRIDE_REDIRECT,
    sym::NO_SPECIAL_GLYPHS,
    sym::ALPHA_BACKGROUND,
    sym::BORDERS_RESPECT_ALPHA_BACKGROUND,
    sym::USE_FRAME_SYNCHRONIZATION,
    sym::DISPLAY,
    sym::LEFT,
    sym::TOP,
    sym::WIDTH,
    sym::HEIGHT,
    sym::PARENT_ID,
    sym::WINDOW_ID,
    sym::EXPLICIT_NAME,
    sym::MODELINE,
    sym::BUFFER_LIST,
    sym::BURIED_BUFFER_LIST,
];

pub const SUBRS: &[Subr] = &[
    Subr::fixed("frame-id", 0, 1, f_frame_id),
    Subr::fixed("framep", 1, 1, f_framep),
    Subr::fixed("frame-live-p", 1, 1, f_frame_live_p),
    Subr::fixed("window-system", 0, 1, f_window_system),
    Subr::fixed("frame-windows-min-size", 4, 4, f_frame_windows_min_size),
    Subr::fixed("select-frame", 1, 2, f_select_frame),
    Subr::fixed("handle-switch-frame", 1, 1, f_handle_switch_frame),
    Subr::fixed("selected-frame", 0, 0, f_selected_frame),
    Subr::fixed("old-selected-frame", 0, 0, f_old_selected_frame),
    Subr::fixed("frame-list", 0, 0, f_frame_list),
    Subr::fixed("frame-parent", 0, 1, f_frame_parent),
    Subr::fixed("frame-ancestor-p", 2, 2, f_frame_ancestor_p),
    Subr::fixed("frame-root-frame", 0, 1, f_frame_root_frame),
    Subr::fixed("next-frame", 0, 2, f_next_frame),
    Subr::fixed("previous-frame", 0, 2, f_previous_frame),
    Subr::fixed("last-nonminibuffer-frame", 0, 0, f_last_nonminibuffer_frame),
    Subr::fixed("frame-visible-p", 0, 1, f_frame_visible_p),
    Subr::fixed("visible-frame-list", 0, 0, f_visible_frame_list),
    Subr::fixed("redirect-frame-focus", 1, 2, f_redirect_frame_focus),
    Subr::fixed("frame-focus", 0, 1, f_frame_focus),
    Subr::fixed("frame-after-make-frame", 2, 2, f_frame_after_make_frame),
    Subr::fixed("frame-parameters", 0, 1, f_frame_parameters),
    Subr::fixed("frame-parameter", 2, 2, f_frame_parameter),
    Subr::fixed("modify-frame-parameters", 2, 2, f_modify_frame_parameters),
    Subr::fixed("frame-char-height", 0, 1, f_frame_char_height),
    Subr::fixed("frame-char-width", 0, 1, f_frame_char_width),
    Subr::fixed("frame-native-width", 0, 1, f_frame_native_width),
    Subr::fixed("frame-native-height", 0, 1, f_frame_native_height),
    Subr::fixed("tool-bar-pixel-width", 0, 1, f_tool_bar_pixel_width),
    Subr::fixed("frame-text-cols", 0, 1, f_frame_text_cols),
    Subr::fixed("frame-text-lines", 0, 1, f_frame_text_lines),
    Subr::fixed("frame-total-cols", 0, 1, f_frame_total_cols),
    Subr::fixed("frame-total-lines", 0, 1, f_frame_total_lines),
    Subr::fixed("frame-text-width", 0, 1, f_frame_text_width),
    Subr::fixed("frame-text-height", 0, 1, f_frame_text_height),
    Subr::fixed("frame-scroll-bar-width", 0, 1, f_scroll_bar_width),
    Subr::fixed("frame-scroll-bar-height", 0, 1, f_scroll_bar_height),
    Subr::fixed("frame-fringe-width", 0, 1, f_fringe_width),
    Subr::fixed(
        "frame-child-frame-border-width",
        0,
        1,
        f_frame_child_frame_border_width,
    ),
    Subr::fixed(
        "frame-internal-border-width",
        0,
        1,
        f_frame_internal_border_width,
    ),
    Subr::fixed("frame-right-divider-width", 0, 1, f_right_divider_width),
    Subr::fixed("frame-bottom-divider-width", 0, 1, f_bottom_divider_width),
    Subr::fixed("frame-pointer-visible-p", 0, 1, f_frame_pointer_visible_p),
    Subr::fixed("frame--set-was-invisible", 2, 2, f_frame_set_was_invisible),
    Subr::fixed(
        "frame-window-state-change",
        0,
        1,
        f_frame_window_state_change,
    ),
    Subr::fixed(
        "set-frame-window-state-change",
        0,
        2,
        f_set_frame_window_state_change,
    ),
    Subr::fixed("frame-scale-factor", 0, 1, f_frame_scale_factor),
];

pub fn init_vars(lisp: &mut Lisp) {
    for (i, &parameter) in FRAME_PARMS.iter().enumerate() {
        lisp.put(parameter, sym::X_FRAME_PARAMETER, Value::fixnum(i as i64));
    }
    lisp.defvar(sym::X_RESOURCE_NAME, Value::NIL);
    let class = lisp.string("Emacs");
    lisp.defvar(sym::X_RESOURCE_CLASS, class);
    lisp.defvar(sym::FRAME_ALPHA_LOWER_LIMIT, Value::fixnum(20));
    lisp.defvar(sym::DEFAULT_FRAME_ALIST, Value::NIL);
    lisp.defvar(sym::DEFAULT_FRAME_SCROLL_BARS, sym::LEFT);
    lisp.defvar_bool(sym::SCROLL_BAR_ADJUST_THUMB_PORTION, true);
    lisp.defvar(sym::TERMINAL_FRAME_2, Value::NIL);
    lisp.defvar(sym::MOUSE_POSITION_FUNCTION, Value::NIL);
    lisp.defvar(sym::MOUSE_HIGHLIGHT, sym::T);
    lisp.defvar(sym::MAKE_POINTER_INVISIBLE, sym::T);
    lisp.defvar(sym::MOVE_FRAME_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::DELETE_FRAME_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::AFTER_DELETE_FRAME_FUNCTIONS, Value::NIL);
    lisp.defvar(sym::MENU_BAR_MODE, sym::T);
    lisp.defvar(sym::TAB_BAR_MODE, Value::NIL);
    lisp.defvar(sym::TOOL_BAR_MODE, sym::T);
    lisp.defvar_kboard(
        sym::DEFAULT_MINIBUFFER_FRAME,
        crate::keyboard::Kvar::DefaultMinibufferFrame,
    );
    lisp.defvar(sym::RESIZE_MINI_FRAMES, Value::NIL);
    lisp.defvar(sym::FOCUS_FOLLOWS_MOUSE, Value::NIL);
    lisp.defvar_bool(sym::FRAME_RESIZE_PIXELWISE, false);
    let implied = lisp.list(&[sym::TAB_BAR_LINES, sym::TOOL_BAR_LINES]);
    lisp.defvar(sym::FRAME_INHIBIT_IMPLIED_RESIZE, implied);
    lisp.defvar(sym::FRAME_SIZE_HISTORY, Value::NIL);
    lisp.defvar_bool(sym::TOOLTIP_REUSE_HIDDEN_FRAME, false);
    lisp.defvar_bool(sym::USE_SYSTEM_TOOLTIPS, true);
    lisp.defvar(sym::ICONIFY_CHILD_FRAME, sym::ICONIFY_TOP_LEVEL);
    lisp.defvar(sym::EXPOSE_HIDDEN_BUFFER, Value::NIL);
    lisp.make_variable_buffer_local_builtin(sym::EXPOSE_HIDDEN_BUFFER);
    let internal = lisp.list(&[
        sym::UNDELETED,
        sym::CLONED_FROM,
        sym::FRAME_ID,
        sym::NAME,
        sym::PARENT_ID,
        sym::WINDOW_ID,
    ]);
    lisp.defvar(sym::FRAME_INTERNAL_PARAMETERS, internal);
    lisp.defvar(sym::ALTER_FULLSCREEN_FRAMES, Value::NIL);
    lisp.defvar(sym::DELETE_FRAME_CHOOSE_SELECTED, sym::MRU);
    crate::fns::f_provide(lisp, &[sym::MOVE_TOOLBAR, Value::NIL]).expect("provide move-toolbar");
    lisp.defvar(sym::TOOLKIT_THEME, Value::NIL);
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

impl Lisp {
    pub fn is_frame(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Frame(_))
    }

    pub fn frame(&self, v: Value) -> &Frame {
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Frame(f) => f,
            _ => unreachable!("not a frame"),
        }
    }

    pub fn frame_mut(&mut self, v: Value) -> &mut Frame {
        match self.heap.vectorlike_mut(v) {
            Vectorlike::Frame(f) => f,
            _ => unreachable!("not a frame"),
        }
    }

    pub fn frame_live_p(&self, v: Value) -> bool {
        self.is_frame(v) && !self.frame(v).terminal.is_nil()
    }

    pub fn check_frame(&mut self, v: Value) -> LispResult<Value> {
        if self.is_frame(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::FRAMEP, v))
        }
    }

    pub fn check_live_frame(&mut self, v: Value) -> LispResult<Value> {
        if self.frame_live_p(v) {
            Ok(v)
        } else {
            Err(self.wrong_type(sym::FRAME_LIVE_P, v))
        }
    }

    pub fn decode_live_frame(&mut self, frame: Value) -> LispResult<Value> {
        let frame = if frame.is_nil() {
            self.frames.selected_frame
        } else {
            frame
        };
        self.check_live_frame(frame)
    }

    pub fn decode_any_frame(&mut self, frame: Value) -> LispResult<Value> {
        let frame = if frame.is_nil() {
            self.frames.selected_frame
        } else {
            frame
        };
        self.check_frame(frame)
    }

    pub fn window_system_available(&self, f: Option<Value>) -> bool {
        match f {
            Some(f) => self.frame(f).window_p(),
            None => self
                .terminals
                .list
                .iter()
                .any(|&t| self.terminal(t).kind != OutputMethod::Initial),
        }
    }

    pub fn check_window_system(&mut self, f: Option<Value>) -> LispResult<()> {
        if self.window_system_available(f) {
            return Ok(());
        }
        Err(self.error(if f.is_some() {
            "Window system frame should be used"
        } else {
            "Window system is not in use or not initialized"
        }))
    }

    pub fn frame_list_vec(&self) -> Vec<Value> {
        self.frames.list.clone()
    }

    pub fn frame_minibuf_only_p(&self, f: Value) -> bool {
        let fr = self.frame(f);
        fr.root_window == fr.minibuffer_window
    }

    pub fn frame_has_minibuf_p(&self, f: Value) -> bool {
        let mw = self.frame(f).minibuffer_window;
        self.is_window(mw) && self.window(mw).frame == f
    }

    pub fn frame_kboard(&self, f: Value) -> Option<usize> {
        let t = self.frame(f).terminal;
        if self.is_terminal(t) {
            self.terminal(t).kboard
        } else {
            None
        }
    }

    pub fn frame_make_pointer_invisible(&mut self, f: Value) {
        if !self.find_symbol_value(sym::MAKE_POINTER_INVISIBLE).is_nil()
            && self.frame_live_p(f)
            && !self.frame(f).flags.pointer_invisible
        {
            match self.terminal(self.frame(f).terminal).kind {
                OutputMethod::Initial => {}
            }
        }
    }

    pub fn get_frame_param(&self, f: Value, prop: Value) -> Value {
        let cell = self.assq(prop, self.frame(f).param_alist);
        self.cdr_safe(cell)
    }

    pub fn frame_parent_frame(&self, f: Value) -> Value {
        let p = self.frame(f).parent_frame;
        if self.is_frame(p) { p } else { Value::NIL }
    }

    pub fn frame_ancestor_p(&self, af: Value, df: Value) -> bool {
        let mut pf = self.frame_parent_frame(df);
        while !pf.is_nil() {
            if pf == af {
                return true;
            }
            pf = self.frame_parent_frame(pf);
        }
        false
    }

    pub fn root_frame(&self, mut f: Value) -> Value {
        loop {
            let p = self.frame_parent_frame(f);
            if p.is_nil() {
                return f;
            }
            f = p;
        }
    }

    pub fn frame_scroll_bar_area_width(&self, f: Value) -> i64 {
        let fr = self.frame(f);
        if fr.vertical_scroll_bar_type != VerticalScrollBarType::None {
            fr.geometry.config_scroll_bar_width
        } else {
            0
        }
    }

    pub fn frame_scroll_bar_area_height(&self, f: Value) -> i64 {
        let fr = self.frame(f);
        if fr.flags.horizontal_scroll_bars {
            fr.geometry.config_scroll_bar_height
        } else {
            0
        }
    }

    pub fn frame_total_fringe_width(&self, f: Value) -> i64 {
        let g = &self.frame(f).geometry;
        g.left_fringe_width + g.right_fringe_width
    }

    pub fn frame_set_id(&mut self, f: Value, id: i64) -> LispResult<i64> {
        if id >= self.frames.next_id {
            return Err(self.error("Specified frame ID unassigned"));
        }
        if id > 0 {
            let in_use = self.frames.list.iter().any(|&fr| self.frame(fr).id == id);
            if in_use {
                return Err(self.error("Specified frame ID already in use"));
            }
        }
        if id == 0 {
            if self.frame(f).id != 0 {
                return Ok(0);
            }
            let next = self.frames.next_id;
            self.frames.next_id += 1;
            self.frame_mut(f).id = next;
        } else {
            self.frame_mut(f).id = id;
        }
        Ok(self.frame(f).id)
    }

    pub fn make_frame(&mut self, mini_p: bool) -> LispResult<Value> {
        let mut fr = Frame::allocate();
        fr.tool_bar_position = sym::TOP;
        fr.flags.wants_modeline = true;
        fr.flags.redisplay = true;
        fr.flags.garbaged = true;
        fr.geometry.column_width = 1;
        fr.geometry.line_height = 1;
        fr.geometry.new_width = -1;
        fr.geometry.new_height = -1;
        fr.geometry.child_frame_border_width = -1;
        fr.last_tab_bar_item = -1;
        fr.last_tool_bar_item = -1;
        fr.change_stamp = 1;
        let frame = self.heap.vectorlike(Vectorlike::Frame(Box::new(fr)));
        let base = self.stack.len();
        self.stack.push(frame);
        let r = self.make_frame_1(frame, mini_p);
        self.stack.truncate(base);
        r?;
        Ok(frame)
    }

    fn make_frame_1(&mut self, frame: Value, mini_p: bool) -> LispResult<()> {
        let root_window = self.make_window();
        self.stack.push(root_window);
        let mini_window = if mini_p {
            let mw = self.make_window();
            self.stack.push(mw);
            self.window_mut(root_window).next = mw;
            let w = self.window_mut(mw);
            w.prev = root_window;
            w.mini = true;
            w.frame = frame;
            self.frame_mut(frame).minibuffer_window = mw;
            self.store_frame_param(frame, sym::MINIBUFFER, sym::T)?;
            mw
        } else {
            self.window_mut(root_window).next = Value::NIL;
            self.frame_mut(frame).minibuffer_window = Value::NIL;
            Value::NIL
        };
        self.window_mut(root_window).frame = frame;
        let (column_width, line_height) = {
            let g = &self.frame(frame).geometry;
            (g.column_width, g.line_height)
        };
        {
            let g = &mut self.frame_mut(frame).geometry;
            g.text_cols = 80;
            g.total_cols = 80;
            g.text_width = 80 * column_width;
            g.pixel_width = 80 * column_width;
            g.text_lines = 25;
            g.total_lines = 25;
            g.text_height = 25 * line_height;
            g.pixel_height = 25 * line_height;
        }
        let rw_lines = 25 - i64::from(mini_p);
        {
            let rw = self.window_mut(root_window);
            rw.total_cols = 80;
            rw.pixel_width = 80 * column_width;
            rw.total_lines = rw_lines;
            rw.pixel_height = rw_lines * line_height;
        }
        let table = self.make_hash_table_with(sym::EQ, Weakness::None);
        self.frame_mut(frame).face_hash_table = table;
        if mini_p {
            let (total_cols, pixel_width) = {
                let rw = self.window(root_window);
                (rw.total_cols, rw.pixel_width)
            };
            let mw = self.window_mut(mini_window);
            mw.top_line = rw_lines;
            mw.pixel_top = rw_lines * line_height;
            mw.total_cols = total_cols;
            mw.pixel_width = pixel_width;
            mw.total_lines = 1;
            mw.pixel_height = line_height;
        }
        let mut buf = self.current_buffer;
        let expose = self.find_symbol_value(sym::EXPOSE_HIDDEN_BUFFER);
        if self.buffer_hidden_p(buf) && expose.is_nil() {
            buf = self.other_buffer_safely(buf);
        }
        self.set_window_buffer(root_window, buf, false, false)?;
        let list = self.list(&[buf]);
        self.frame_mut(frame).buffer_list = list;
        if mini_p {
            let minibuf = if self.minibuf.minibuffer_list.is_nil() {
                self.get_minibuffer(0)?
            } else {
                self.car_safe(self.minibuf.minibuffer_list)
            };
            self.set_window_buffer(mini_window, minibuf, false, false)?;
        }
        let f = self.frame_mut(frame);
        f.root_window = root_window;
        f.selected_window = root_window;
        self.windows.window_select_count += 1;
        let count = self.windows.window_select_count;
        self.window_mut(root_window).use_time = count;
        Ok(())
    }

    fn frame_name_fnn_p(name: &[u8]) -> bool {
        name.len() > 1
            && name[0] == b'F'
            && name[1].is_ascii_digit()
            && name[2..].iter().all(|b| b.is_ascii_digit())
    }

    fn frame_next_f_name(&mut self) -> Value {
        loop {
            self.frames.tty_frame_count += 1;
            let name = format!("F{}", self.frames.tty_frame_count);
            let taken = self.frames.list.iter().any(|&fr| {
                let n = self.frame(fr).name;
                !n.is_nil() && self.is_string(n) && self.string_bytes(n) == name.as_bytes()
            });
            if !taken {
                return self.string(&name);
            }
        }
    }

    fn frame_get(&mut self, name: Value) -> LispResult {
        for fr in self.frame_list_vec() {
            let n = self.frame(fr).name;
            if self.is_string(n) && self.string_equal(n, name)? {
                return Ok(fr);
            }
        }
        Ok(Value::NIL)
    }

    fn string_equal(&mut self, a: Value, b: Value) -> LispResult<bool> {
        Ok(!crate::fns::f_string_equal(self, &[a, b])?.is_nil())
    }

    fn set_term_frame_name(&mut self, f: Value, name: Value) -> LispResult<()> {
        self.frame_mut(f).flags.explicit_name = !name.is_nil();
        let name = if name.is_nil() {
            let current = self.frame(f).name;
            if self.is_string(current) && Self::frame_name_fnn_p(self.string_bytes(current)) {
                return Ok(());
            }
            self.frame_next_f_name()
        } else {
            self.check_string(name)?;
            let current = self.frame(f).name;
            if self.string_equal(name, current)? {
                return Ok(());
            }
            if Self::frame_name_fnn_p(self.string_bytes(name)) && !self.frame_get(name)?.is_nil() {
                let text = self.string_to_rust(name);
                return Err(self.error(&format!(
                    "Frame of the form F<num> named `{text}' already exists"
                )));
            }
            name
        };
        self.frame_mut(f).name = name;
        self.windows.update_mode_lines = 16;
        Ok(())
    }

    fn change_frame_size_delayed(&mut self, f: Value, new_width: i64, new_height: i64) {
        let g = &mut self.frame_mut(f).geometry;
        g.new_width = new_width;
        g.new_height = new_height;
        self.frame_mut(f).flags.new_size_p = true;
        self.frames.delayed_size_change = true;
    }

    fn set_menu_bar_lines(&mut self, f: Value, value: Value) {
        let olines = self.frame(f).geometry.menu_bar_lines;
        let nlines = value
            .as_fixnum()
            .filter(|n| i32::try_from(*n).is_ok())
            .unwrap_or(0);
        if !self.frame_minibuf_only_p(f) && nlines != olines {
            self.windows.windows_or_buffers_changed = 14;
            let g = &mut self.frame_mut(f).geometry;
            g.menu_bar_lines = nlines;
            g.menu_bar_height = nlines;
            let (w, h) = (g.pixel_width, g.pixel_height);
            self.change_frame_size_delayed(f, w, h);
        }
    }

    fn set_tab_bar_lines(&mut self, f: Value, value: Value) {
        let olines = self.frame(f).geometry.tab_bar_lines;
        let nlines = value
            .as_fixnum()
            .filter(|n| i32::try_from(*n).is_ok())
            .unwrap_or(0);
        if !self.frame_minibuf_only_p(f) && nlines != olines {
            self.windows.windows_or_buffers_changed = 14;
            let g = &mut self.frame_mut(f).geometry;
            g.tab_bar_lines = nlines;
            g.tab_bar_height = nlines;
            let (w, h) = (g.pixel_width, g.pixel_height);
            self.change_frame_size_delayed(f, w, h);
        }
    }

    pub fn make_initial_frame(&mut self) -> LispResult<Value> {
        let terminal = self.init_initial_terminal()?;
        let frame = self.make_frame(true)?;
        self.frame_set_id(frame, 0)?;
        self.frames.list.insert(0, frame);
        self.frames.tty_frame_count = 1;
        let name = self.string("F1");
        let kind = self.terminal(terminal).kind;
        let f = self.frame_mut(frame);
        f.name = name;
        f.flags.visible = true;
        f.output_method = kind;
        f.terminal = terminal;
        f.foreground_pixel = FACE_TTY_DEFAULT_FG_COLOR;
        f.background_pixel = FACE_TTY_DEFAULT_BG_COLOR;
        f.vertical_scroll_bar_type = VerticalScrollBarType::None;
        f.flags.horizontal_scroll_bars = false;
        self.terminal_mut(terminal).reference_count += 1;
        self.set_menu_bar_lines(frame, Value::fixnum(1));
        self.set_tab_bar_lines(frame, Value::fixnum(0));
        if !self.noninteractive() {
            self.init_frame_faces(frame)?;
        }
        self.frames.last_nonminibuf_frame = frame;
        let f = self.frame_mut(frame);
        f.flags.can_set_window_size = true;
        f.flags.after_make_frame = true;
        Ok(frame)
    }

    pub fn store_in_alist(&mut self, alist: Value, prop: Value, val: Value) -> Value {
        let tem = self.assq(prop, alist);
        if tem.is_nil() {
            let cell = self.cons(prop, val);
            self.cons(cell, alist)
        } else {
            self.heap.set_cdr(tem, val);
            alist
        }
    }

    pub fn store_frame_param(&mut self, f: Value, prop: Value, mut val: Value) -> LispResult<()> {
        if prop == sym::MINIBUFFER {
            if self.is_window(val) {
                if !self.window_live_p(val) || !self.window(val).mini {
                    return Err(self.error(
                        "The `minibuffer' parameter does not specify a valid minibuffer window",
                    ));
                }
                let vframe = self.window(val).frame;
                if self.frame_live_p(f) && self.frame(vframe).terminal != self.frame(f).terminal {
                    return Err(self.error(
                        "Minibuffer window must be on same terminal as frame that uses it",
                    ));
                }
                if self.frame_minibuf_only_p(f) {
                    if val == self.frame(f).minibuffer_window {
                        val = sym::ONLY;
                    } else {
                        return Err(self.error(
                            "Can't change the minibuffer window of a minibuffer-only frame",
                        ));
                    }
                } else if self.frame_has_minibuf_p(f) {
                    if val == self.frame(f).minibuffer_window {
                        val = sym::T;
                    } else {
                        return Err(self.error(
                            "Can't change the minibuffer window of a frame with its own minibuffer",
                        ));
                    }
                } else {
                    self.frame_mut(f).minibuffer_window = val;
                }
            } else {
                let old_val = self.get_frame_param(f, sym::MINIBUFFER);
                if !old_val.is_nil() {
                    if self.is_window(old_val) && val.is_nil() {
                        val = old_val;
                    } else if old_val != val {
                        return Err(
                            self.error("Can't change the `minibuffer' parameter of this frame")
                        );
                    }
                }
            }
        } else if prop == sym::PARENT_FRAME || prop == sym::DELETE_BEFORE {
            let oldval = self.get_frame_param(f, prop);
            if oldval != val && !val.is_nil() {
                let name = self.symbol_name_string(prop);
                if !self.frame_live_p(val)
                    || (self.frame_live_p(f) && self.frame(f).terminal != self.frame(val).terminal)
                {
                    return Err(self.error(&format!("Invalid `{name}' frame parameter")));
                }
                let mut frame1 = val;
                while self.frame_live_p(frame1) {
                    if frame1 == f {
                        return Err(self.error(&format!(
                            "Circular specification of `{name}' frame parameter"
                        )));
                    }
                    frame1 = self.get_frame_param(frame1, prop);
                }
            }
        } else if prop == sym::BUFFER_LIST || prop == sym::BURIED_BUFFER_LIST {
            let mut items = Vec::new();
            let mut tail = val;
            while tail.is_cons() {
                let b = self.xcar(tail);
                if self.is_buffer(b) && self.buffer_live_p(b) {
                    items.push(b);
                }
                tail = self.xcdr(tail);
            }
            let list = self.list(&items);
            if prop == sym::BUFFER_LIST {
                self.frame_mut(f).buffer_list = list;
            } else {
                self.frame_mut(f).buried_buffer_list = list;
            }
            return Ok(());
        } else if (prop == sym::SCROLL_BAR_WIDTH || prop == sym::SCROLL_BAR_HEIGHT)
            && !val.is_nil()
            && !val
                .as_fixnum()
                .is_some_and(|n| (1..=i32::MAX as i64).contains(&n))
        {
            val = self.get_frame_param(f, prop);
        }
        let alist = self.frame(f).param_alist;
        let old = self.assq(prop, alist);
        if old.is_nil() {
            let cell = self.cons(prop, val);
            let alist = self.cons(cell, alist);
            self.frame_mut(f).param_alist = alist;
        } else {
            self.heap.set_cdr(old, val);
        }
        if prop == sym::BUFFER_PREDICATE {
            self.frame_mut(f).buffer_predicate = val;
        }
        if !self.frame(f).window_p() {
            if prop == sym::MENU_BAR_LINES {
                self.set_menu_bar_lines(f, val);
            } else if prop == sym::TAB_BAR_LINES {
                self.set_tab_bar_lines(f, val);
            } else if prop == sym::NAME {
                self.set_term_frame_name(f, val)?;
            }
        }
        Ok(())
    }

    pub fn frame_visible_p(&self, f: Value) -> Value {
        let fl = &self.frame(f).flags;
        if fl.visible {
            sym::T
        } else if fl.iconified {
            sym::ICON
        } else {
            Value::NIL
        }
    }

    fn frame_get_param_value(&mut self, f: Value, parameter: Value) -> LispResult {
        let index = self.get(parameter, sym::X_FRAME_PARAMETER);
        let Some(index) = index
            .as_fixnum()
            .filter(|&i| i >= 0 && (i as usize) < FRAME_PARMS.len())
        else {
            return Ok(sym::UNBOUND);
        };
        let window_p = self.frame(f).window_p();
        let fr = self.frame(f);
        let g = &fr.geometry;
        let p = FRAME_PARMS[index as usize];
        Ok(match p {
            _ if p == sym::NAME => fr.name,
            _ if p == sym::WIDTH => Value::fixnum(g.text_cols),
            _ if p == sym::HEIGHT => Value::fixnum(g.text_lines),
            _ if p == sym::MODELINE => self.bool(fr.flags.wants_modeline),
            _ if p == sym::UNSPLITTABLE => self.bool(fr.flags.no_split),
            _ if p == sym::BUFFER_LIST => fr.buffer_list,
            _ if p == sym::BURIED_BUFFER_LIST => fr.buried_buffer_list,
            _ if p == sym::VISIBILITY => self.frame_visible_p(f),
            _ if p == sym::LINE_SPACING && g.extra_line_spacing == 0 => Value::fixnum(0),
            _ if !window_p && p == sym::BACKGROUND_COLOR => {
                let elt = self.assq(sym::BACKGROUND_COLOR, fr.param_alist);
                let pixel = fr.background_pixel;
                if elt.is_cons() && self.is_string(self.xcdr(elt)) {
                    let unspec = self.xcdr(elt);
                    self.frame_unspecified_color(f, unspec)?
                } else {
                    self.tty_color_name(f, pixel)?
                }
            }
            _ if !window_p && p == sym::FOREGROUND_COLOR => {
                let elt = self.assq(sym::FOREGROUND_COLOR, fr.param_alist);
                let pixel = fr.foreground_pixel;
                if elt.is_cons() && self.is_string(self.xcdr(elt)) {
                    let unspec = self.xcdr(elt);
                    self.frame_unspecified_color(f, unspec)?
                } else {
                    self.tty_color_name(f, pixel)?
                }
            }
            _ if !window_p && p == sym::FONT => self.string("tty"),
            _ if !window_p && p == sym::MENU_BAR_LINES => Value::fixnum(g.menu_bar_lines),
            _ if !window_p && p == sym::TAB_BAR_LINES => Value::fixnum(g.tab_bar_lines),
            _ if !window_p && p == sym::NO_ACCEPT_FOCUS => self.bool(fr.flags.no_accept_focus),
            _ => sym::UNBOUND,
        })
    }

    fn frame_unspecified_color(&mut self, f: Value, unspec: Value) -> LispResult {
        let text = self.string_bytes(unspec).to_vec();
        let pixel = if b"unspecified-bg".starts_with(&text) {
            self.frame(f).background_pixel
        } else if b"unspecified-fg".starts_with(&text) {
            self.frame(f).foreground_pixel
        } else {
            return Ok(Value::NIL);
        };
        self.tty_color_name(f, pixel)
    }

    pub fn do_switch_frame(&mut self, frame: Value, norecord: Value) -> LispResult {
        let frame = if frame.is_cons()
            && self.xcar(frame) == sym::SWITCH_FRAME
            && self.xcdr(frame).is_cons()
        {
            self.xcar(self.xcdr(frame))
        } else {
            frame
        };
        self.check_frame(frame)?;
        let sf = self.frames.selected_frame;
        if !self.frame_live_p(frame) || self.frame(frame).flags.tooltip {
            return Ok(Value::NIL);
        }
        if frame == sf {
            return Ok(frame);
        }
        let sf_mini = {
            let sw = self.frame(sf).selected_window;
            self.window(sw).mini
        };
        self.frame_mut(sf).flags.select_mini_window_flag = sf_mini;
        let (fsw, fmw) = {
            let fr = self.frame(frame);
            (fr.selected_window, fr.minibuffer_window)
        };
        if fsw == fmw {
            let contents = self.window(fmw).contents;
            if !self.minibufferp(contents, true) {
                let w = self.funcall(sym::GET_MRU_WINDOW, &[frame])?;
                if self.window_live_p(w) {
                    crate::window::f_set_frame_selected_window(self, &[frame, w, Value::NIL])?;
                }
            }
        }
        self.frames.selected_frame = frame;
        let select_mini = self.frame(frame).flags.select_mini_window_flag;
        if select_mini {
            let mw = self.frame(frame).minibuffer_window;
            let contents = self.window(mw).contents;
            if self.minibufferp(contents, true) {
                self.frame_mut(frame).selected_window = mw;
            }
        }
        self.frame_mut(frame).flags.select_mini_window_flag = false;
        if !self.frame_minibuf_only_p(frame) {
            self.frames.last_nonminibuf_frame = frame;
        }
        let sw = self.frame(frame).selected_window;
        crate::window::f_select_window(self, &[sw, norecord])?;
        Ok(frame)
    }
}

fn f_frame_id(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    let id = lisp.frame(f).id;
    Ok(if id == 0 {
        Value::NIL
    } else {
        Value::fixnum(id)
    })
}

fn framep_value(lisp: &Lisp, object: Value) -> Value {
    if !lisp.is_frame(object) {
        return Value::NIL;
    }
    match lisp.frame(object).output_method {
        OutputMethod::Initial => sym::T,
    }
}

fn f_framep(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(framep_value(lisp, args[0]))
}

fn f_frame_live_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(if lisp.frame_live_p(args[0]) {
        framep_value(lisp, args[0])
    } else {
        Value::NIL
    })
}

fn f_window_system(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let frame = if arg(args, 0).is_nil() {
        lisp.frames.selected_frame
    } else {
        arg(args, 0)
    };
    let kind = framep_value(lisp, frame);
    if kind.is_nil() {
        return Err(lisp.wrong_type(sym::FRAMEP, frame));
    }
    Ok(if kind == sym::T { Value::NIL } else { kind })
}

fn f_frame_windows_min_size(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(0))
}

pub(crate) fn f_select_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let frame = lisp.check_live_frame(args[0])?;
    if lisp.frame(frame).flags.tooltip {
        return Err(lisp.error("Cannot select a tooltip frame"));
    }
    lisp.do_switch_frame(frame, arg(args, 1))
}

fn f_handle_switch_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prefix = lisp.find_symbol_value(sym::CURRENT_PREFIX_ARG);
    lisp.set_kvar(crate::keyboard::Kvar::PrefixArg, prefix);
    lisp.run_hook(sym::MOUSE_LEAVE_BUFFER_HOOK)?;
    lisp.do_switch_frame(args[0], Value::NIL)
}

fn f_selected_frame(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.frames.selected_frame)
}

fn f_old_selected_frame(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.frames.old_selected_frame)
}

fn f_frame_list(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let frames: Vec<Value> = lisp
        .frame_list_vec()
        .into_iter()
        .filter(|&f| !lisp.frame(f).flags.tooltip)
        .collect();
    Ok(lisp.list(&frames))
}

fn f_frame_parent(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    Ok(lisp.frame_parent_frame(f))
}

fn f_frame_ancestor_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let af = lisp.decode_live_frame(args[0])?;
    let df = lisp.decode_live_frame(args[1])?;
    let r = lisp.frame_ancestor_p(af, df);
    Ok(lisp.bool(r))
}

fn f_frame_root_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    Ok(lisp.root_frame(f))
}

fn candidate_frame(lisp: &Lisp, candidate: Value, frame: Value, minibuf: Value) -> Value {
    let (c, f) = (lisp.frame(candidate), lisp.frame(frame));
    let same = (!c.termcap_p()
        && !f.termcap_p()
        && lisp.frame_kboard(candidate) == lisp.frame_kboard(frame))
        || (c.termcap_p() && f.termcap_p() && c.terminal == f.terminal);
    if !same
        || !lisp
            .get_frame_param(candidate, sym::NO_OTHER_FRAME)
            .is_nil()
    {
        return Value::NIL;
    }
    let ok = if minibuf.is_nil() {
        !lisp.frame_minibuf_only_p(candidate)
    } else if minibuf == sym::VISIBLE {
        c.flags.visible
    } else if lisp.is_window(minibuf) {
        let wf = lisp.window(minibuf).frame;
        c.minibuffer_window == minibuf || wf == candidate || wf == c.focus_frame
    } else if minibuf.as_fixnum() == Some(0) {
        c.flags.visible || c.flags.iconified
    } else {
        true
    };
    if ok { candidate } else { Value::NIL }
}

fn next_frame(lisp: &Lisp, frame: Value, minibuf: Value) -> Value {
    let mut next = Value::NIL;
    let mut passed = false;
    for f in lisp.frame_list_vec() {
        if f == frame {
            passed = true;
        } else {
            let c = candidate_frame(lisp, f, frame, minibuf);
            if !c.is_nil() {
                if passed {
                    return c;
                } else if next.is_nil() {
                    next = c;
                }
            }
        }
    }
    if next.is_nil() { frame } else { next }
}

fn prev_frame(lisp: &Lisp, frame: Value, minibuf: Value) -> Value {
    let mut prev = Value::NIL;
    for f in lisp.frame_list_vec() {
        if f == frame && !prev.is_nil() {
            return prev;
        }
        let c = candidate_frame(lisp, f, frame, minibuf);
        if !c.is_nil() {
            prev = c;
        }
    }
    if prev.is_nil() { frame } else { prev }
}

fn f_next_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let frame = if arg(args, 0).is_nil() {
        lisp.frames.selected_frame
    } else {
        arg(args, 0)
    };
    lisp.check_live_frame(frame)?;
    Ok(next_frame(lisp, frame, arg(args, 1)))
}

fn f_previous_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let frame = if arg(args, 0).is_nil() {
        lisp.frames.selected_frame
    } else {
        arg(args, 0)
    };
    lisp.check_live_frame(frame)?;
    Ok(prev_frame(lisp, frame, arg(args, 1)))
}

fn f_last_nonminibuffer_frame(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let f = lisp.frames.last_nonminibuf_frame;
    Ok(if lisp.frame_live_p(f) { f } else { Value::NIL })
}

fn f_frame_visible_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    Ok(lisp.frame_visible_p(f))
}

fn f_visible_frame_list(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let mut value = Value::NIL;
    for f in lisp.frame_list_vec() {
        if lisp.frame(f).flags.visible {
            value = lisp.cons(f, value);
        }
    }
    Ok(value)
}

fn f_redirect_frame_focus(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(args[0])?;
    let focus = arg(args, 1);
    if !focus.is_nil() {
        lisp.check_live_frame(focus)?;
    }
    lisp.frame_mut(f).focus_frame = focus;
    Ok(Value::NIL)
}

fn f_frame_focus(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    Ok(lisp.frame(f).focus_frame)
}

fn f_frame_after_make_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(args[0])?;
    lisp.frame_mut(f).flags.after_make_frame = !args[1].is_nil();
    Ok(args[1])
}

fn f_frame_parameters(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    let alist = lisp.frame(f).param_alist;
    let mut alist = crate::fns::f_copy_alist(lisp, &[alist])?;
    let base = lisp.stack.len();
    lisp.stack.push(alist);
    if lisp.frame_live_p(f) {
        for &parameter in FRAME_PARMS {
            let value = match lisp.frame_get_param_value(f, parameter) {
                Ok(v) => v,
                Err(e) => {
                    lisp.stack.truncate(base);
                    return Err(e);
                }
            };
            if value != sym::UNBOUND {
                alist = lisp.store_in_alist(alist, parameter, value);
                lisp.stack[base] = alist;
            }
        }
    }
    lisp.stack.truncate(base);
    Ok(alist)
}

fn f_frame_parameter(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(args[0])?;
    lisp.check_symbol(args[1])?;
    if !lisp.frame_live_p(f) {
        return Ok(Value::NIL);
    }
    let value = lisp.frame_get_param_value(f, args[1])?;
    Ok(if value == sym::UNBOUND {
        lisp.get_frame_param(f, args[1])
    } else {
        value
    })
}

pub(crate) fn f_modify_frame_parameters(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(args[0])?;
    let mut parms = Vec::new();
    let mut tail = args[1];
    while tail.is_cons() {
        let elt = lisp.xcar(tail);
        let prop = lisp.car(elt)?;
        let val = lisp.cdr(elt)?;
        parms.push((prop, val));
        tail = lisp.xcdr(tail);
    }
    for &(prop, val) in parms.iter().rev() {
        lisp.store_frame_param(f, prop, val)?;
        if prop == sym::FOREGROUND_COLOR || prop == sym::BACKGROUND_COLOR {
            lisp.update_face_from_frame_parameter(f, prop, val)?;
        }
    }
    Ok(Value::NIL)
}

fn f_frame_char_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    let fr = lisp.frame(f);
    Ok(Value::fixnum(if fr.window_p() {
        fr.geometry.line_height
    } else {
        1
    }))
}

fn f_frame_char_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    let fr = lisp.frame(f);
    Ok(Value::fixnum(if fr.window_p() {
        fr.geometry.column_width
    } else {
        1
    }))
}

fn f_frame_native_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    let fr = lisp.frame(f);
    Ok(Value::fixnum(if fr.window_p() {
        fr.geometry.pixel_width
    } else {
        fr.geometry.total_cols
    }))
}

fn f_frame_native_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    let fr = lisp.frame(f);
    Ok(Value::fixnum(if fr.window_p() {
        fr.geometry.pixel_height
    } else {
        fr.geometry.total_lines
    }))
}

fn f_tool_bar_pixel_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.decode_any_frame(arg(args, 0))?;
    Ok(Value::fixnum(0))
}

fn geometry_getter(lisp: &mut Lisp, args: &[Value], get: fn(&FrameGeometry) -> i64) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    Ok(Value::fixnum(get(&lisp.frame(f).geometry)))
}

fn f_frame_text_cols(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    geometry_getter(lisp, args, |g| g.text_cols)
}

fn f_frame_text_lines(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    geometry_getter(lisp, args, |g| g.text_lines)
}

fn f_frame_total_cols(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    geometry_getter(lisp, args, |g| g.total_cols)
}

fn f_frame_total_lines(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    geometry_getter(lisp, args, |g| g.total_lines)
}

fn f_frame_text_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    geometry_getter(lisp, args, |g| g.text_width)
}

fn f_frame_text_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    geometry_getter(lisp, args, |g| g.text_height)
}

fn f_scroll_bar_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    Ok(Value::fixnum(lisp.frame_scroll_bar_area_width(f)))
}

fn f_scroll_bar_height(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    Ok(Value::fixnum(lisp.frame_scroll_bar_area_height(f)))
}

fn f_fringe_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    Ok(Value::fixnum(lisp.frame_total_fringe_width(f)))
}

fn f_frame_child_frame_border_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    let g = &lisp.frame(f).geometry;
    Ok(Value::fixnum(if g.child_frame_border_width < 0 {
        g.internal_border_width
    } else {
        g.child_frame_border_width
    }))
}

fn f_frame_internal_border_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    geometry_getter(lisp, args, |g| g.internal_border_width)
}

fn f_right_divider_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    geometry_getter(lisp, args, |g| g.right_divider_width)
}

fn f_bottom_divider_width(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    geometry_getter(lisp, args, |g| g.bottom_divider_width)
}

fn f_frame_pointer_visible_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_any_frame(arg(args, 0))?;
    let invisible = lisp.frame(f).flags.pointer_invisible;
    Ok(lisp.bool(!invisible))
}

fn f_frame_set_was_invisible(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(args[0])?;
    lisp.frame_mut(f).flags.was_invisible = !args[1].is_nil();
    Ok(lisp.bool(!args[1].is_nil()))
}

fn f_frame_window_state_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    let v = lisp.frame(f).flags.window_state_change;
    Ok(lisp.bool(v))
}

fn f_set_frame_window_state_change(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    let v = !arg(args, 1).is_nil();
    lisp.frame_mut(f).flags.window_state_change = v;
    Ok(lisp.bool(v))
}

fn f_frame_scale_factor(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.decode_live_frame(arg(args, 0))?;
    Ok(lisp.float(1.0))
}
