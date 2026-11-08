use crate::doprnt::Arg;
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Fullscreen {
    #[default]
    None,
    Width,
    Height,
    Both,
    Maximized,
}

pub const X_NEGATIVE: i64 = 0x0010;
pub const Y_NEGATIVE: i64 = 0x0020;
pub const US_POSITION: i64 = 1 << 0;
pub const US_SIZE: i64 = 1 << 1;
pub const P_POSITION: i64 = 1 << 2;
pub const P_SIZE: i64 = 1 << 3;
pub const NORTH_WEST_GRAVITY: i64 = 1;
pub const NORTH_EAST_GRAVITY: i64 = 3;
pub const SOUTH_WEST_GRAVITY: i64 = 7;
pub const SOUTH_EAST_GRAVITY: i64 = 9;
const DEFAULT_TAB_BAR_BUTTON_RELIEF: i64 = 1;
const DEFAULT_TAB_BAR_IMAGE_HEIGHT: i64 = 18;
const DEFAULT_TOOL_BAR_BUTTON_RELIEF: i64 = 1;
const DEFAULT_TOOL_BAR_IMAGE_HEIGHT: i64 = 24;

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
    pub resized_p: bool,
    pub after_make_frame: bool,
    pub new_size_p: bool,
    pub select_mini_window_flag: bool,
    pub no_special_glyphs: bool,
    pub no_accept_focus: bool,
    pub undecorated: bool,
    pub override_redirect: bool,
    pub tooltip: bool,
    pub face_change: bool,
    pub default_face_done_p: bool,
    pub pointer_invisible: bool,
    pub mouse_moved: bool,
    pub borders_respect_alpha_background: bool,
    pub native_window: bool,
    pub was_invisible: bool,
    pub horizontal_scroll_bars: bool,
    pub inhibit_clear_image_cache: bool,
    pub glyphs_initialized_p: bool,
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
    pub extra_line_spacing_above: i64,
    pub fringe_cols: i64,
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
    pub want_fullscreen: Fullscreen,
    pub size_hint_flags: i64,
    pub win_gravity: i64,
    pub gamma: f64,
    pub alpha_background: f64,
    pub face_cache: Option<Box<crate::xfaces::FaceCache>>,
    pub font_driver_list: Vec<crate::font::FontDriverList>,
    pub fonts_changed: bool,
    pub font: Value,
    pub fontset: i64,
    pub baseline_offset: i64,
    pub n_tab_bar_rows: i64,
    pub n_tool_bar_rows: i64,
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
            want_fullscreen: Fullscreen::None,
            size_hint_flags: 0,
            win_gravity: 0,
            gamma: 0.0,
            alpha_background: 0.0,
            face_cache: None,
            font_driver_list: Vec::new(),
            fonts_changed: false,
            font: Value::NIL,
            fontset: -1,
            baseline_offset: 0,
            n_tab_bar_rows: 0,
            n_tool_bar_rows: 0,
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
            self.font,
        ]);
        if let Some(cache) = &self.face_cache {
            cache.trace(out);
        }
        out.extend(self.font_driver_list.iter().map(|list| list.driver.type_));
    }

    pub fn window_p(&self) -> bool {
        match self.output_method {
            OutputMethod::Initial => false,
            OutputMethod::Pgtk => true,
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
    pub default_tab_bar_height: i64,
    pub default_tool_bar_height: i64,
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
            default_tab_bar_height: 0,
            default_tool_bar_height: 0,
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
    Subr::fixed("delete-frame", 0, 2, f_delete_frame).interactive(""),
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
    Subr::fixed("make-frame-visible", 0, 1, f_make_frame_visible).interactive(""),
    Subr::fixed("make-frame-invisible", 0, 2, f_make_frame_invisible).interactive(""),
    Subr::fixed("iconify-frame", 0, 1, f_iconify_frame).interactive(""),
    Subr::fixed("raise-frame", 0, 1, f_raise_frame).interactive(""),
    Subr::fixed("lower-frame", 0, 1, f_lower_frame).interactive(""),
    Subr::fixed("x-get-resource", 2, 4, f_x_get_resource),
];

const EMACS_CLASS: &[u8] = b"Emacs";
const MAX_ALLOCA: usize = 16 * 1024;

fn x_resource_char_p(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'_'
}

impl Lisp {
    pub fn validate_x_resource_name(&mut self) -> LispResult<()> {
        if !self.is_string(self.plain_value(sym::X_RESOURCE_CLASS)) {
            let class = self.make_string(EMACS_CLASS);
            self.set_plain_value(sym::X_RESOURCE_CLASS, class);
        }
        let name = self.plain_value(sym::X_RESOURCE_NAME);
        let (len, good_count, bad_count) = if self.is_string(name) {
            let bytes = self.string_bytes(name);
            let good = bytes.iter().filter(|&&c| x_resource_char_p(c)).count();
            (bytes.len(), good, bytes.len() - good)
        } else {
            (0, 0, 5)
        };
        if bad_count == 0 {
            return Ok(());
        }
        if good_count < 2 || MAX_ALLOCA - ".customization\0".len() < len {
            let emacs = self.make_string(b"emacs");
            self.set_plain_value(sym::X_RESOURCE_NAME, emacs);
            return Ok(());
        }
        let new = crate::fns::f_copy_sequence(self, &[name])?;
        self.set_plain_value(sym::X_RESOURCE_NAME, new);
        for c in self.heap.string_mut(new).data.iter_mut() {
            if !x_resource_char_p(*c) {
                *c = b'_';
            }
        }
        Ok(())
    }

    pub fn decode_window_system_frame(&mut self, frame: Value) -> LispResult<Value> {
        let f = self.decode_live_frame(frame)?;
        self.check_window_system(Some(f))?;
        Ok(f)
    }

    pub fn gui_display_get_resource(
        &mut self,
        terminal: Value,
        attribute: Value,
        class: Value,
        component: Value,
        subclass: Value,
    ) -> LispResult {
        self.check_string(attribute)?;
        self.check_string(class)?;
        if !component.is_nil() {
            self.check_string(component)?;
        }
        if !subclass.is_nil() {
            self.check_string(subclass)?;
        }
        if component.is_nil() != subclass.is_nil() {
            return Err(
                self.error("x-get-resource: must specify both COMPONENT and SUBCLASS or neither")
            );
        }
        self.validate_x_resource_name()?;
        let mut name_key = self
            .string_bytes(self.plain_value(sym::X_RESOURCE_NAME))
            .to_vec();
        let mut class_key = self
            .string_bytes(self.plain_value(sym::X_RESOURCE_CLASS))
            .to_vec();
        class_key.push(b'.');
        class_key.extend_from_slice(self.string_bytes(class));
        if !component.is_nil() {
            class_key.push(b'.');
            class_key.extend_from_slice(self.string_bytes(subclass));
            name_key.push(b'.');
            name_key.extend_from_slice(self.string_bytes(component));
        }
        name_key.push(b'.');
        name_key.extend_from_slice(self.string_bytes(attribute));
        let value = match self.terminal(terminal).hooks.get_string_resource {
            Some(hook) => hook(self, &name_key, &class_key)?,
            None => None,
        };
        Ok(match value {
            Some(value) if !value.is_empty() => self.make_string(&value),
            _ => Value::NIL,
        })
    }
}

fn f_x_get_resource(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.check_window_system(None)?;
    let terminal = match lisp.window_system {
        Some(window_system) => (window_system.check_x_display_info)(lisp, Value::NIL)?,
        None => return Err(lisp.error("Window system is not in use or not initialized")),
    };
    lisp.gui_display_get_resource(terminal, args[0], args[1], arg(args, 2), arg(args, 3))
}

impl Lisp {
    pub fn frame_window_p(&self, f: Value) -> bool {
        match self.frame(f).output_method {
            OutputMethod::Initial => false,
            OutputMethod::Pgtk => true,
        }
    }

    fn is_tty_child_frame(&self, f: Value) -> bool {
        match self.frame(f).output_method {
            OutputMethod::Initial | OutputMethod::Pgtk => false,
        }
    }

    fn other_frames(&self, f: Value, invisible: bool, force: bool) -> bool {
        self.frames.list.iter().any(|&f1| {
            if f1 == f {
                return false;
            }
            let fl = &self.frame(f1).flags;
            !fl.tooltip
                && self.frame_parent_frame(f1).is_nil()
                && (invisible || self.get_frame_param(f1, sym::DELETE_BEFORE).is_nil())
                && (fl.visible
                    || fl.iconified
                    || (!invisible
                        && (force || (self.frame_window_p(f1) && !self.frame_window_p(f)))))
        })
    }

    pub fn delete_frame(&mut self, frame: Value, force: Value) -> LispResult {
        let base = self.stack.len();
        let r = self.delete_frame_1(frame, force);
        self.stack.truncate(base);
        r
    }

    fn rooted_frame_list(&mut self) -> Vec<Value> {
        let frames = self.frames.list.clone();
        self.stack.extend_from_slice(&frames);
        frames
    }

    fn delete_frame_hook_call(
        &mut self,
        hook: Value,
        frame: Value,
        force: Value,
    ) -> LispResult<()> {
        if force == sym::NOELISP {
            let call = self.list(&[sym::RUN_HOOK_WITH_ARGS, hook, frame]);
            self.pending_funcalls = self.cons(call, self.pending_funcalls);
        } else {
            self.safe_calln(sym::RUN_HOOK_WITH_ARGS, &[hook, frame])?;
        }
        Ok(())
    }

    fn delete_frame_1(&mut self, frame: Value, force: Value) -> LispResult {
        let f = self.decode_any_frame(frame)?;
        let nochild = self.frame_parent_frame(f).is_nil();
        let mut minibuffer_child_frame = Value::NIL;
        if !self.frame_live_p(f) {
            return Ok(Value::NIL);
        } else if force != sym::NOELISP && !self.other_frames(f, false, !force.is_nil()) {
            return Err(if force.is_nil() {
                self.error("Attempt to delete the sole visible or iconified frame")
            } else {
                self.error("Attempt to delete the only frame")
            });
        }
        let frame = f;
        self.stack.push(frame);
        if self.is_tty_frame(f) && force.is_nil() {
            unreachable!("tty frames need frame_subsumes_p");
        }
        for frame1 in self.rooted_frame_list() {
            if !self.frames.list.contains(&frame1)
                || frame1 == frame
                || self.frame(frame1).flags.tooltip
            {
                continue;
            } else if self.frame_parent_frame(frame1) == f {
                if self.frame_has_minibuf_p(frame1)
                    && !self.frame_has_minibuf_p(f)
                    && self.frame(f).minibuffer_window == self.frame(frame1).minibuffer_window
                {
                    let cell = self.cons(sym::PARENT_FRAME, Value::NIL);
                    let alist = self.cons(cell, Value::NIL);
                    f_modify_frame_parameters(self, &[frame1, alist])?;
                    minibuffer_child_frame = frame1;
                    self.stack.push(minibuffer_child_frame);
                } else {
                    self.delete_frame(frame1, Value::NIL)?;
                }
            } else if nochild && self.get_frame_param(frame1, sym::DELETE_BEFORE) == frame {
                self.delete_frame(frame1, Value::NIL)?;
            }
        }
        if self.frame_has_minibuf_p(f) {
            for frame1 in self.rooted_frame_list() {
                if !self.frames.list.contains(&frame1) || frame1 == frame {
                    continue;
                }
                let fminiw = self.frame(frame1).minibuffer_window;
                if self.is_window(fminiw) && frame == self.window(fminiw).frame {
                    if force == sym::NOELISP {
                        self.delete_frame(frame1, sym::NOELISP)?;
                    } else {
                        return Err(self.error("Attempt to delete a surrogate minibuffer frame"));
                    }
                }
            }
        }
        let is_tooltip_frame = self.frame(f).flags.tooltip;
        if !self.plain_value(sym::RUN_HOOKS_ENABLED).is_nil() && !is_tooltip_frame {
            self.delete_frame_hook_call(sym::DELETE_FRAME_FUNCTIONS, frame, force)?;
        }
        if !self.frame_live_p(f) {
            return Ok(Value::NIL);
        } else if force != sym::NOELISP && !self.other_frames(f, false, !force.is_nil()) {
            return Err(if force.is_nil() {
                self.error("Attempt to delete the sole visible or iconified frame")
            } else {
                self.error("Attempt to delete the only frame")
            });
        }
        let mut sf = self.frames.selected_frame;
        if f == sf {
            if self.is_tty_child_frame(f) {
                unreachable!("tty child frames need mru_rooted_frame");
            }
            let mut frame1 = Value::NIL;
            if self.find_symbol_value(sym::DELETE_FRAME_CHOOSE_SELECTED) == sym::MRU
                && force != sym::NOELISP
            {
                frame1 = self.safe_calln(sym::GET_MRU_FRAME, &[sym::VISIBLE, Value::NIL, frame])?;
                if self.is_frame(frame1) {
                    let f1 = self.frame(frame1);
                    if f1.flags.tooltip
                        || self.frame(f).terminal != f1.terminal
                        || !f1.flags.visible
                        || frame1 == frame
                    {
                        frame1 = Value::NIL;
                    }
                } else {
                    frame1 = Value::NIL;
                }
            }
            if frame1.is_nil() {
                let terminal = self.frame(f).terminal;
                frame1 = self
                    .frames
                    .list
                    .iter()
                    .copied()
                    .find(|&f1| {
                        let fr = self.frame(f1);
                        f1 != frame
                            && !fr.flags.tooltip
                            && fr.terminal == terminal
                            && fr.flags.visible
                    })
                    .unwrap_or(Value::NIL);
                if frame1.is_nil() || frame1 == frame {
                    frame1 = self
                        .frames
                        .list
                        .iter()
                        .copied()
                        .find(|&f1| {
                            f1 != frame && self.frame_live_p(f1) && !self.frame(f1).flags.tooltip
                        })
                        .unwrap_or(Value::NIL);
                    if !frame1.is_nil() && self.is_tty_frame(frame1) {
                        unreachable!("tty frames need their terminal's top frame");
                    }
                }
            }
            if self.is_frame(frame1) && frame1 != f {
                self.do_switch_frame(frame1, true, Value::NIL)?;
            } else if force == sym::NOELISP {
                crate::emacs::f_kill_emacs(self, &[Value::fixnum(70), Value::NIL])?;
            } else {
                panic!("emacs_abort: no frame to select instead of the deleted one");
            }
            sf = self.frames.selected_frame;
        } else {
            let selected = self.frames.selected_frame;
            self.move_minibuffers_onto_frame(f, selected, true)?;
        }
        if self.frame(f).minibuffer_window == self.xdisp.echo_area_window {
            self.xdisp.echo_area_window = self.frame(sf).minibuffer_window;
        }
        self.font_update_drivers(f, Value::NIL)?;
        let root = self.frame(f).root_window;
        self.delete_all_child_windows(root)?;
        self.frame_mut(f).root_window = Value::NIL;
        self.frames.list.retain(|&f1| f1 != frame);
        let fr = self.frame_mut(f);
        fr.flags.visible = false;
        fr.menu_bar_vector = Value::NIL;
        fr.buffer_list = Value::NIL;
        fr.buried_buffer_list = Value::NIL;
        self.free_font_driver_list(f);
        let terminal = self.frame(f).terminal;
        if let Some(hook) = self.terminal(terminal).hooks.delete_frame {
            hook(self, f);
        }
        self.frame_mut(f).terminal = Value::NIL;
        let t = self.terminal_mut(terminal);
        t.reference_count -= 1;
        if t.reference_count == 0 && t.kind == OutputMethod::Pgtk {
            t.reference_count = 1;
        }
        let kb = if self.terminal(terminal).reference_count == 0 {
            if force != sym::NOELISP {
                let force = if force.is_nil() { sym::T } else { force };
                crate::terminal::f_delete_terminal(self, &[terminal, force])?;
            }
            None
        } else {
            self.terminal(terminal).kboard
        };
        if f == self.frames.last_nonminibuf_frame {
            self.frames.last_nonminibuf_frame = self
                .frames
                .list
                .iter()
                .copied()
                .find(|&f1| !self.frame_minibuf_only_p(f1))
                .unwrap_or(Value::NIL);
        }
        if let Some(kb) = kb
            && !self
                .frames
                .list
                .iter()
                .any(|&f1| self.frame_kboard(f1) == Some(kb))
        {
            self.not_single_kboard_state(kb);
        }
        let slot = crate::keyboard::Kvar::DefaultMinibufferFrame as usize;
        if let Some(kb) = kb
            && frame == self.kboards[kb].vars[slot]
        {
            let mut frame_with_minibuf = Value::NIL;
            let mut frame_on_same_kboard = Value::NIL;
            for &f1 in &self.frames.list {
                if !self.frame(f1).flags.tooltip && self.frame_kboard(f1) == Some(kb) {
                    frame_on_same_kboard = f1;
                    if self.frame_has_minibuf_p(f1) {
                        frame_with_minibuf = f1;
                        if self.frame_minibuf_only_p(f1) {
                            break;
                        }
                    }
                }
            }
            if !frame_on_same_kboard.is_nil() {
                if frame_with_minibuf.is_nil() {
                    panic!("emacs_abort: frames on a kboard without a minibuffer frame");
                }
                self.kboards[kb].vars[slot] = frame_with_minibuf;
            } else {
                self.kboards[kb].vars[slot] = Value::NIL;
            }
        }
        if !is_tooltip_frame {
            self.windows.update_mode_lines = 15;
        }
        if !self.plain_value(sym::RUN_HOOKS_ENABLED).is_nil() && !is_tooltip_frame {
            self.delete_frame_hook_call(sym::AFTER_DELETE_FRAME_FUNCTIONS, frame, force)?;
        }
        if !minibuffer_child_frame.is_nil() && self.frame_live_p(minibuffer_child_frame) {
            let f1 = minibuffer_child_frame;
            let window1 = self.frame(f1).root_window;
            for frame2 in self.rooted_frame_list() {
                if frame2 == minibuffer_child_frame || self.frame(frame2).flags.tooltip {
                    continue;
                } else if self.frame(frame2).minibuffer_window == window1 {
                    let fl = &self.frame(f1).flags;
                    if !fl.visible && !fl.iconified {
                        f_make_frame_visible(self, &[minibuffer_child_frame])?;
                    }
                    return Ok(Value::NIL);
                }
            }
            if force == sym::NOELISP || self.other_frames(f1, false, !force.is_nil()) {
                self.delete_frame(minibuffer_child_frame, sym::NOELISP)?;
            }
        }
        Ok(Value::NIL)
    }

    fn make_frame_visible_1(&mut self, window: Value) -> LispResult<()> {
        let mut window = window;
        while !window.is_nil() {
            let contents = self.window(window).contents;
            if self.is_window(contents) {
                self.make_frame_visible_1(contents)?;
            } else {
                let now = crate::timefns::f_current_time(self, &[])?;
                self.buffer_local_set(contents, sym::BUFFER_DISPLAY_TIME, now);
            }
            window = self.window(window).next;
        }
        Ok(())
    }
}

pub(crate) fn f_make_frame_visible(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    if lisp.frame_window_p(f)
        && let Some(hook) = lisp.frame_terminal_hooks(f).frame_visible_invisible
    {
        hook(lisp, f, true);
    }
    if lisp.is_tty_child_frame(f) {
        unreachable!("tty child frames need tty_raise_lower_frame");
    }
    let root = lisp.frame(f).root_window;
    lisp.make_frame_visible_1(root)?;
    Ok(f)
}

pub(crate) fn f_make_frame_invisible(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    if arg(args, 1).is_nil() && !lisp.other_frames(f, true, false) {
        return Err(lisp.error("Attempt to make invisible the sole visible or iconified frame"));
    }
    if lisp.frame_window_p(f)
        && let Some(hook) = lisp.frame_terminal_hooks(f).frame_visible_invisible
    {
        hook(lisp, f, false);
    }
    if lisp.is_tty_child_frame(f) {
        unreachable!("tty child frames need mru_rooted_frame");
    }
    lisp.windows.windows_or_buffers_changed = 16;
    Ok(Value::NIL)
}

pub(crate) fn f_iconify_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let frame = arg(args, 0);
    let f = lisp.decode_live_frame(frame)?;
    if !lisp.frame_parent_frame(f).is_nil() {
        let iconify_child_frame = lisp.find_symbol_value(sym::ICONIFY_CHILD_FRAME);
        if iconify_child_frame.is_nil() {
            return Ok(Value::NIL);
        } else if lisp.frame_window_p(f) && iconify_child_frame == sym::ICONIFY_TOP_LEVEL {
            unreachable!();
        } else if iconify_child_frame == sym::MAKE_INVISIBLE {
            f_make_frame_invisible(lisp, &[frame, Value::NIL])?;
            return Ok(Value::NIL);
        }
    }
    if lisp.frame_window_p(f)
        && let Some(hook) = lisp.frame_terminal_hooks(f).iconify_frame
    {
        hook(lisp, f);
    }
    Ok(Value::NIL)
}

pub(crate) fn f_raise_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    f_make_frame_visible(lisp, &[f])?;
    if let Some(hook) = lisp.frame_terminal_hooks(f).frame_raise_lower {
        hook(lisp, f, true);
    }
    Ok(Value::NIL)
}

fn f_lower_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    if let Some(hook) = lisp.frame_terminal_hooks(f).frame_raise_lower {
        hook(lisp, f, false);
    }
    Ok(Value::NIL)
}

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

    pub fn frame_terminal_hooks(&self, f: Value) -> crate::terminal::TerminalHooks {
        self.terminal(self.frame(f).terminal).hooks
    }

    pub fn frame_make_pointer_invisible(&mut self, f: Value) {
        if !self.find_symbol_value(sym::MAKE_POINTER_INVISIBLE).is_nil()
            && self.frame_live_p(f)
            && !self.frame(f).flags.pointer_invisible
            && let Some(hook) = self.frame_terminal_hooks(f).toggle_invisible_pointer
        {
            self.frame_mut(f).flags.mouse_moved = false;
            hook(self, f, true);
            self.frame_mut(f).flags.pointer_invisible = true;
        }
    }

    pub fn frame_make_pointer_visible(&mut self, f: Value) {
        if self.frame_live_p(f)
            && self.frame(f).flags.pointer_invisible
            && self.frame(f).flags.mouse_moved
            && let Some(hook) = self.frame_terminal_hooks(f).toggle_invisible_pointer
        {
            hook(self, f, false);
            self.frame_mut(f).flags.pointer_invisible = false;
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

    pub(crate) fn frame_tool_bar_top_height(&self, f: Value) -> i64 {
        let fr = self.frame(f);
        if fr.tool_bar_position == sym::TOP {
            fr.geometry.tool_bar_height
        } else {
            0
        }
    }

    pub(crate) fn frame_tool_bar_top_lines(&self, f: Value) -> i64 {
        let fr = self.frame(f);
        if fr.tool_bar_position == sym::TOP {
            fr.geometry.tool_bar_lines
        } else {
            0
        }
    }

    pub fn frame_top_margin(&self, f: Value) -> i64 {
        let g = &self.frame(f).geometry;
        g.menu_bar_lines + g.tab_bar_lines + self.frame_tool_bar_top_lines(f)
    }

    pub fn frame_top_margin_height(&self, f: Value) -> i64 {
        let g = &self.frame(f).geometry;
        g.menu_bar_height + g.tab_bar_height + self.frame_tool_bar_top_height(f)
    }

    fn frame_margin_height(&self, f: Value) -> i64 {
        let g = &self.frame(f).geometry;
        g.menu_bar_height + g.tab_bar_height + g.tool_bar_height
    }

    fn frame_text_to_pixel_width(&self, f: Value, width: i64) -> i64 {
        width
            + self.frame_scroll_bar_area_width(f)
            + self.frame_total_fringe_width(f)
            + 2 * self.frame(f).geometry.internal_border_width
    }

    fn frame_text_to_pixel_height(&self, f: Value, height: i64) -> i64 {
        height
            + self.frame_margin_height(f)
            + self.frame_scroll_bar_area_height(f)
            + 2 * self.frame(f).geometry.internal_border_width
    }

    fn frame_pixel_to_text_width(&self, f: Value, width: i64) -> i64 {
        width
            - self.frame_scroll_bar_area_width(f)
            - self.frame_total_fringe_width(f)
            - 2 * self.frame(f).geometry.internal_border_width
    }

    fn frame_pixel_to_text_height(&self, f: Value, height: i64) -> i64 {
        height
            - self.frame_margin_height(f)
            - self.frame_scroll_bar_area_height(f)
            - 2 * self.frame(f).geometry.internal_border_width
    }

    fn frame_inner_width(&self, f: Value) -> i64 {
        let g = &self.frame(f).geometry;
        g.pixel_width - 2 * g.internal_border_width
    }

    fn frame_inner_height(&self, f: Value) -> i64 {
        let g = &self.frame(f).geometry;
        g.pixel_height - self.frame_margin_height(f) - 2 * g.internal_border_width
    }

    fn is_tty_frame(&self, f: Value) -> bool {
        match self.frame(f).output_method {
            OutputMethod::Initial | OutputMethod::Pgtk => false,
        }
    }

    fn frame_inhibit_resize(&self, f: Value, horizontal: bool, parameter: Value) -> bool {
        let fullscreen = self.get_frame_param(f, sym::FULLSCREEN);
        let implied = self.plain_value(sym::FRAME_INHIBIT_IMPLIED_RESIZE);
        implied == sym::FORCE
            || (self.frame(f).flags.after_make_frame
                && (implied == sym::T
                    || (implied.is_cons() && !self.memq(parameter, implied).is_nil())
                    || (horizontal && !fullscreen.is_nil() && fullscreen != sym::FULLHEIGHT)
                    || (!horizontal && !fullscreen.is_nil() && fullscreen != sym::FULLWIDTH)
                    || self.is_tty_frame(f)))
    }

    fn frame_windows_min_size(
        &mut self,
        frame: Value,
        horizontal: Value,
        ignore: Value,
        pixelwise: Value,
    ) -> LispResult<i64> {
        let par_size = if horizontal.is_nil() {
            self.get_frame_param(frame, sym::MIN_HEIGHT)
        } else {
            self.get_frame_param(frame, sym::MIN_WIDTH)
        };
        let mut retval = if let Some(min_size) = par_size
            .as_fixnum()
            .filter(|n| (i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(n))
        {
            let min_size = min_size.max(1);
            let g = &self.frame(frame).geometry;
            if pixelwise.is_nil() {
                min_size
            } else if horizontal.is_nil() {
                min_size * g.line_height
            } else {
                min_size * g.column_width
            }
        } else {
            let r = self.funcall(
                sym::FRAME_WINDOWS_MIN_SIZE,
                &[frame, horizontal, ignore, pixelwise],
            )?;
            self.check_fixnum(r)?
        };
        if self.is_tty_frame(frame) && horizontal.is_nil() {
            let g = &self.frame(frame).geometry;
            let mut min_height = g.menu_bar_lines
                + g.tab_bar_lines
                + i64::from(self.frame(frame).flags.wants_modeline)
                + i64::from(self.frame_has_minibuf_p(frame));
            if min_height == 0 {
                min_height = 1;
            }
            retval = retval.max(min_height);
        }
        Ok(retval)
    }

    #[allow(clippy::too_many_arguments)]
    fn frame_size_history_adjust(
        &mut self,
        f: Value,
        inhibit: i64,
        parameter: Value,
        sizes: [i64; 16],
        min_inner: (i64, i64),
        inhibit_horizontal: bool,
        inhibit_vertical: bool,
    ) {
        let history = self.plain_value(sym::FRAME_SIZE_HISTORY);
        if !history.is_cons() {
            return;
        }
        let Some(n) = self.xcar(history).as_fixnum().filter(|&n| 0 < n) else {
            return;
        };
        let head = self.list(&[f, Value::fixnum(5), Value::fixnum(inhibit), parameter]);
        let mut groups = Vec::with_capacity(5);
        for chunk in sizes.chunks(4) {
            let items: Vec<Value> = chunk.iter().map(|&n| Value::fixnum(n)).collect();
            groups.push(self.list(&items));
        }
        let ih = self.bool(inhibit_horizontal);
        let iv = self.bool(inhibit_vertical);
        let mins = self.list(&[
            Value::fixnum(min_inner.0),
            Value::fixnum(min_inner.1),
            ih,
            iv,
        ]);
        groups.push(mins);
        let body = self.list(&groups);
        let entry = self.cons(head, body);
        let rest = self.xcdr(history);
        let rest = self.cons(entry, rest);
        let history = self.cons(Value::fixnum(n - 1), rest);
        self.set_plain_value(sym::FRAME_SIZE_HISTORY, history);
    }

    pub fn adjust_frame_size(
        &mut self,
        f: Value,
        new_text_width: i64,
        new_text_height: i64,
        inhibit: i64,
        parameter: Value,
    ) -> LispResult<()> {
        let unit_width = self.frame(f).geometry.column_width;
        let unit_height = self.frame(f).geometry.line_height;
        let old_native_width = self.frame(f).geometry.pixel_width;
        let old_native_height = self.frame(f).geometry.pixel_height;
        let r = self.frame(f).root_window;
        let old_inner_width = self.window(r).pixel_width;
        let old_inner_height = self.window(r).pixel_height
            + if self.frame_has_minibuf_p(f) && !self.frame_minibuf_only_p(f) {
                let mw = self.frame(f).minibuffer_window;
                self.window(mw).pixel_height
            } else {
                0
            };
        let old_text_cols = self.frame(f).geometry.text_cols;
        let old_text_lines = self.frame(f).geometry.text_lines;
        let old_text_width = self.frame(f).geometry.text_width;
        let old_text_height = self.frame(f).geometry.text_height;
        let size_and_position = parameter == sym::SIZE_AND_POSITION;
        let ignore = if inhibit == 5 { sym::SAFE } else { Value::NIL };
        let min_inner_width = self.frame_windows_min_size(f, sym::T, ignore, sym::T)?;
        let min_inner_height = self.frame_windows_min_size(f, Value::NIL, ignore, sym::T)?;
        let mut new_text_width = new_text_width;
        let mut new_text_height = new_text_height;
        let (inhibit_horizontal, inhibit_vertical) = if (2..=4).contains(&inhibit) {
            if new_text_width == -1 {
                new_text_width = old_text_width;
            }
            if new_text_height == -1 {
                new_text_height = old_text_height;
            }
            (
                self.frame_inner_width(f) >= min_inner_width
                    && (inhibit == 4 || self.frame_inhibit_resize(f, true, parameter)),
                self.frame_inner_height(f) >= min_inner_height
                    && (inhibit == 4 || self.frame_inhibit_resize(f, false, parameter)),
            )
        } else {
            (inhibit == 5, inhibit == 5)
        };
        let border = self.frame(f).geometry.internal_border_width;
        let new_native_width = if inhibit_horizontal && inhibit < 5 {
            old_native_width
        } else {
            self.frame_text_to_pixel_width(f, new_text_width)
                .max(min_inner_width + 2 * border)
        };
        let new_inner_width = new_native_width - 2 * border;
        let new_text_width = self.frame_pixel_to_text_width(f, new_native_width);
        let new_text_cols = new_text_width / unit_width;
        let new_native_height = if inhibit_vertical && inhibit < 5 {
            old_native_height
        } else {
            self.frame_text_to_pixel_height(f, new_text_height)
                .max(min_inner_height + self.frame_margin_height(f) + 2 * border)
        };
        let new_inner_height = new_native_height - self.frame_margin_height(f) - 2 * border;
        let new_text_height = self.frame_pixel_to_text_height(f, new_native_height);
        let new_text_lines = new_text_height / unit_height;
        if self.frame(f).window_p()
            && self.frame(f).flags.can_set_window_size
            && (size_and_position
                || (!inhibit_horizontal
                    && (new_native_width != old_native_width || inhibit == 0 || inhibit == 2))
                || (!inhibit_vertical
                    && (new_native_height != old_native_height || inhibit == 0 || inhibit == 2)))
        {
            let (mut new_native_width, mut new_native_height) =
                (new_native_width, new_native_height);
            if inhibit == 2
                && (self.frame(f).geometry.new_width >= 0 || self.frame(f).geometry.new_height >= 0)
            {
                if self.frame(f).geometry.new_width >= 0 {
                    new_native_width = self.frame(f).geometry.new_width;
                }
                if self.frame(f).geometry.new_height >= 0 {
                    new_native_height = self.frame(f).geometry.new_height;
                }
            }
            self.frame_size_history_adjust(
                f,
                inhibit,
                parameter,
                [
                    old_text_width,
                    old_text_height,
                    new_text_width,
                    new_text_height,
                    old_text_cols,
                    old_text_lines,
                    new_text_cols,
                    new_text_lines,
                    old_native_width,
                    old_native_height,
                    new_native_width,
                    new_native_height,
                    old_inner_width,
                    old_inner_height,
                    new_inner_width,
                    new_inner_height,
                ],
                (min_inner_width, min_inner_height),
                inhibit_horizontal,
                inhibit_vertical,
            );
            if inhibit == 0 || inhibit == 1 {
                let fr = self.frame_mut(f);
                fr.geometry.new_width = new_native_width;
                fr.geometry.new_height = new_native_height;
                fr.flags.new_size_p = false;
            }
            if size_and_position {
                unreachable!("set_window_size_and_position_hook needs child frames");
            } else if let Some(set_window_size) = self.frame_terminal_hooks(f).set_window_size {
                set_window_size(self, f, false, new_native_width, new_native_height)?;
            }
            self.frame_mut(f).flags.resized_p = true;
            return Ok(());
        }
        self.frame_size_history_adjust(
            f,
            inhibit,
            parameter,
            [
                old_text_width,
                old_text_height,
                new_text_width,
                new_text_height,
                old_text_cols,
                old_text_lines,
                new_text_cols,
                new_text_lines,
                old_native_width,
                old_native_height,
                new_native_width,
                new_native_height,
                old_inner_width,
                old_inner_height,
                new_inner_width,
                new_inner_height,
            ],
            (min_inner_width, min_inner_height),
            inhibit_horizontal,
            inhibit_vertical,
        );
        if self.window(r).pixel_top == self.frame_top_margin_height(f)
            && new_text_width == old_text_width
            && new_text_height == old_text_height
            && new_inner_width == old_inner_width
            && new_inner_height == old_inner_height
            && new_native_width == old_native_width
            && new_native_height == old_native_height
            && new_text_cols == old_text_cols
            && new_text_lines == old_text_lines
        {
            return Ok(());
        }
        if new_inner_width != old_inner_width {
            self.resize_frame_windows(f, new_inner_width, true)?;
            let tab_bar_window = self.frame(f).tab_bar_window;
            if self.is_window(tab_bar_window) {
                let w = self.window_mut(tab_bar_window);
                w.pixel_width = new_inner_width;
                w.total_cols = new_inner_width / unit_width;
            }
            let tool_bar_window = self.frame(f).tool_bar_window;
            if self.is_window(tool_bar_window) {
                let w = self.window_mut(tool_bar_window);
                w.pixel_width = new_inner_width;
                w.total_cols = new_inner_width / unit_width;
            }
        } else if new_text_cols != old_text_cols {
            self.funcall(sym::WINDOW__PIXEL_TO_TOTAL, &[f, sym::T])?;
        }
        if new_inner_height != old_inner_height
            || self.window(r).pixel_top != self.frame_top_margin_height(f)
        {
            self.resize_frame_windows(f, new_inner_height, false)?;
        } else if new_text_lines != old_text_lines {
            self.funcall(sym::WINDOW__PIXEL_TO_TOTAL, &[f, Value::NIL])?;
        }
        let g = &mut self.frame_mut(f).geometry;
        g.text_cols = new_text_cols;
        g.text_lines = new_text_lines;
        g.text_width = new_text_width;
        g.text_height = new_text_height;
        g.pixel_width = new_native_width;
        g.pixel_height = new_native_height;
        g.total_cols = g.pixel_width / g.column_width;
        g.total_lines = g.pixel_height / g.line_height;
        self.set_frame_garbaged(f);
        self.frame_mut(f).flags.resized_p = true;
        let frames = self.frames.list.clone();
        if frames
            .iter()
            .any(|&frame1| self.frame_parent_frame(frame1) == f)
        {
            unreachable!("keep_ratio needs child frames");
        }
        Ok(())
    }

    pub fn frame_set_id_from_params(&mut self, f: Value, params: Value) -> LispResult<i64> {
        let cell = self.assq(sym::FRAME_ID, params);
        let param_id = self.cdr(cell)?;
        let id = match param_id.as_fixnum() {
            Some(id) if id > 0 && id <= i64::from(i32::MAX) => id,
            _ => 0,
        };
        self.frame_set_id(f, id)
    }

    pub fn gui_figure_window_size(
        &mut self,
        f: Value,
        parms: Value,
        tabbar_p: bool,
        toolbar_p: bool,
    ) -> LispResult<i64> {
        let mut window_prompting = 0;
        let dpyinfo = self.frame_display_info(f);
        let mut cache = FrameFloatCache::default();
        let mut text_width = 80 * self.frame(f).geometry.column_width;
        let mut text_height = 36 * self.frame(f).geometry.line_height;
        self.frame_mut(f).geometry.top_pos = 0;
        self.frame_mut(f).geometry.left_pos = 0;
        if tabbar_p && self.frame(f).geometry.tab_bar_lines != 0 {
            let height = if self.frames.default_tab_bar_height != 0 {
                self.frames.default_tab_bar_height
            } else {
                let relief = self
                    .find_symbol_value(sym::TAB_BAR_BUTTON_RELIEF)
                    .as_fixnum()
                    .unwrap_or(-1);
                let relief = if relief < 0 {
                    DEFAULT_TAB_BAR_BUTTON_RELIEF
                } else {
                    relief.min(1_000_000)
                };
                let margin = self.bar_button_margin(sym::TAB_BAR_BUTTON_MARGIN);
                DEFAULT_TAB_BAR_IMAGE_HEIGHT + 2 * margin + 2 * relief
            };
            self.frame_mut(f).geometry.tab_bar_height = height;
        }
        if toolbar_p && self.frame(f).geometry.tool_bar_lines != 0 {
            let height = if self.frames.default_tool_bar_height != 0 {
                self.frames.default_tool_bar_height
            } else {
                let relief = self
                    .find_symbol_value(sym::TOOL_BAR_BUTTON_RELIEF)
                    .as_fixnum()
                    .unwrap_or(-1);
                let relief = if relief < 0 {
                    DEFAULT_TOOL_BAR_BUTTON_RELIEF
                } else {
                    relief.min(1_000_000)
                };
                let margin = self.bar_button_margin(sym::TOOL_BAR_BUTTON_MARGIN);
                DEFAULT_TOOL_BAR_IMAGE_HEIGHT + 2 * margin + 2 * relief
            };
            self.frame_mut(f).geometry.tool_bar_height = height;
        }
        self.frame_mut(f).geometry.new_width = -1;
        self.frame_mut(f).geometry.new_height = -1;
        let height = self.gui_display_get_arg(
            dpyinfo,
            parms,
            sym::HEIGHT,
            None,
            None,
            ResourceType::Number,
        )?;
        let width =
            self.gui_display_get_arg(dpyinfo, parms, sym::WIDTH, None, None, ResourceType::Number)?;
        if width != sym::UNBOUND || height != sym::UNBOUND {
            if width != sym::UNBOUND {
                text_width = self.figure_size_arg(f, width, true, &mut cache, text_width)?;
            }
            if height != sym::UNBOUND {
                text_height = self.figure_size_arg(f, height, false, &mut cache, text_height)?;
            }
            let user_size = self.gui_display_get_arg(
                dpyinfo,
                parms,
                sym::USER_SIZE,
                None,
                None,
                ResourceType::Number,
            )?;
            if !user_size.is_nil() && user_size != sym::UNBOUND {
                window_prompting |= US_SIZE;
            } else {
                window_prompting |= P_SIZE;
            }
        }
        self.adjust_frame_size(f, text_width, text_height, 5, sym::GUI_FIGURE_WINDOW_SIZE)?;
        let top =
            self.gui_display_get_arg(dpyinfo, parms, sym::TOP, None, None, ResourceType::Number)?;
        let left =
            self.gui_display_get_arg(dpyinfo, parms, sym::LEFT, None, None, ResourceType::Number)?;
        let user_position = self.gui_display_get_arg(
            dpyinfo,
            parms,
            sym::USER_POSITION,
            None,
            None,
            ResourceType::Number,
        )?;
        if top != sym::UNBOUND || left != sym::UNBOUND {
            let (pos, negative) = self.figure_position_arg(f, top, false, &mut cache)?;
            self.frame_mut(f).geometry.top_pos = pos;
            if negative {
                window_prompting |= Y_NEGATIVE;
            }
            let (pos, negative) = self.figure_position_arg(f, left, true, &mut cache)?;
            self.frame_mut(f).geometry.left_pos = pos;
            if negative {
                window_prompting |= X_NEGATIVE;
            }
            if !user_position.is_nil() && user_position != sym::UNBOUND {
                window_prompting |= US_POSITION;
            } else {
                window_prompting |= P_POSITION;
            }
        }
        let gravity = match (
            window_prompting & X_NEGATIVE != 0,
            window_prompting & Y_NEGATIVE != 0,
        ) {
            (true, true) => SOUTH_EAST_GRAVITY,
            (true, false) => NORTH_EAST_GRAVITY,
            (false, true) => SOUTH_WEST_GRAVITY,
            (false, false) => NORTH_WEST_GRAVITY,
        };
        self.frame_mut(f).win_gravity = gravity;
        self.frame_mut(f).size_hint_flags = window_prompting;
        Ok(window_prompting)
    }

    fn bar_button_margin(&self, variable: Value) -> i64 {
        let margin = self.find_symbol_value(variable);
        let ranged = |v: Value| {
            v.as_fixnum()
                .filter(|&n| (1..=i64::from(i32::MAX)).contains(&n))
        };
        if let Some(n) = ranged(margin) {
            n
        } else if margin.is_cons() {
            ranged(self.xcdr(margin)).unwrap_or(0)
        } else {
            0
        }
    }

    fn figure_size_arg(
        &mut self,
        f: Value,
        arg: Value,
        width_p: bool,
        cache: &mut FrameFloatCache,
        current: i64,
    ) -> LispResult<i64> {
        if arg.is_cons() && self.xcar(arg) == sym::TEXT_PIXELS {
            let n = self.check_fixnum(self.xcdr(arg))?;
            if !(0..=i64::from(i32::MAX)).contains(&n) {
                let data = self.list(&[self.xcdr(arg)]);
                return Err(self.signal(sym::ARGS_OUT_OF_RANGE, data));
            }
            return Ok(n);
        }
        if arg.is_float() {
            let d = self.heap.float_value(arg);
            if !(0.0..=1.0).contains(&d) {
                let data = self.list(&[arg]);
                return Err(self.signal(sym::ARGS_OUT_OF_RANGE, data));
            }
            let what = if width_p {
                FrameFloatType::Width
            } else {
                FrameFloatType::Height
            };
            let new = self.frame_float(f, arg, what, cache, -1)?;
            return Ok(if new > -1 { new } else { current });
        }
        let n = self.check_fixnum(arg)?;
        if !(0..=i64::from(i32::MAX)).contains(&n) {
            let data = self.list(&[arg]);
            return Err(self.signal(sym::ARGS_OUT_OF_RANGE, data));
        }
        let unit = if width_p {
            self.frame(f).geometry.column_width
        } else {
            self.frame(f).geometry.line_height
        };
        Ok(n * unit)
    }

    fn figure_position_arg(
        &mut self,
        f: Value,
        arg: Value,
        left_p: bool,
        cache: &mut FrameFloatCache,
    ) -> LispResult<(i64, bool)> {
        let int_range = |v: Value, lo: i64| {
            v.as_fixnum()
                .filter(|&n| (lo..=i64::from(i32::MAX)).contains(&n))
        };
        if arg == sym::MINUS {
            return Ok((0, true));
        }
        if arg.is_cons()
            && self.xcar(arg) == sym::MINUS
            && self.xcdr(arg).is_cons()
            && let Some(n) = int_range(self.xcar(self.xcdr(arg)), -i64::from(i32::MAX))
        {
            return Ok((-n, true));
        }
        if arg.is_cons()
            && self.xcar(arg) == sym::PLUS
            && self.xcdr(arg).is_cons()
            && let Some(n) = int_range(self.xcar(self.xcdr(arg)), i64::from(i32::MIN))
        {
            return Ok((n, false));
        }
        if arg.is_float() {
            let what = if left_p {
                FrameFloatType::Left
            } else {
                FrameFloatType::Top
            };
            return Ok((self.frame_float(f, arg, what, cache, 0)?, false));
        }
        if arg == sym::UNBOUND {
            return Ok((0, false));
        }
        let n = self.check_integer_range(arg, i64::from(i32::MIN), i64::from(i32::MAX))?;
        Ok((n, n < 0))
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

    pub fn make_frame_without_minibuffer(
        &mut self,
        mut mini_window: Value,
        kb: usize,
        display: Value,
    ) -> LispResult<Value> {
        if !mini_window.is_nil() {
            self.check_live_window(mini_window)?;
        }
        if !mini_window.is_nil() {
            let frame = self.window(mini_window).frame;
            if self.frame_kboard(frame) != Some(kb) {
                return Err(self.error("Frame and minibuffer must be on the same terminal"));
            }
        }
        let f = self.make_frame(false)?;
        let base = self.stack.len();
        self.stack.push(f);
        let r = self.make_frame_without_minibuffer_1(f, &mut mini_window, kb, display);
        self.stack.truncate(base);
        r?;
        Ok(f)
    }

    fn make_frame_without_minibuffer_1(
        &mut self,
        f: Value,
        mini_window: &mut Value,
        kb: usize,
        display: Value,
    ) -> LispResult<()> {
        let slot = crate::keyboard::Kvar::DefaultMinibufferFrame as usize;
        if mini_window.is_nil() {
            let default = self.kboards[kb].vars[slot];
            if !self.is_frame(default) || !self.frame_live_p(default) {
                let initial_frame = self.funcall(sym::MAKE_INITIAL_MINIBUFFER_FRAME, &[display])?;
                self.kboards[kb].vars[slot] = initial_frame;
            }
            let default = self.kboards[kb].vars[slot];
            *mini_window = self.frame(default).minibuffer_window;
        }
        let mini_window = *mini_window;
        self.frame_mut(f).minibuffer_window = mini_window;
        self.store_frame_param(f, sym::MINIBUFFER, mini_window)?;
        let contents = self.window(mini_window).contents;
        let list = self.minibuf.minibuffer_list;
        if self.memq(contents, list).is_nil() {
            let buffer = if list.is_nil() {
                self.get_minibuffer(0)?
            } else {
                self.car(list)?
            };
            self.set_window_buffer(mini_window, buffer, false, false)?;
        }
        Ok(())
    }

    pub fn make_minibuffer_frame(&mut self) -> LispResult<Value> {
        let f = self.make_frame(false)?;
        let base = self.stack.len();
        self.stack.push(f);
        let r = self.make_minibuffer_frame_1(f);
        self.stack.truncate(base);
        r?;
        Ok(f)
    }

    fn make_minibuffer_frame_1(&mut self, f: Value) -> LispResult<()> {
        {
            let flags = &mut self.frame_mut(f).flags;
            flags.auto_raise = false;
            flags.auto_lower = false;
            flags.no_split = true;
            flags.wants_modeline = false;
        }
        let mini_window = self.frame(f).root_window;
        self.frame_mut(f).minibuffer_window = mini_window;
        self.store_frame_param(f, sym::MINIBUFFER, sym::ONLY)?;
        {
            let w = self.window_mut(mini_window);
            w.mini = true;
            w.next = Value::NIL;
            w.prev = Value::NIL;
            w.frame = f;
        }
        let list = self.minibuf.minibuffer_list;
        let buffer = if list.is_nil() {
            self.get_minibuffer(0)?
        } else {
            self.car(list)?
        };
        self.set_window_buffer(mini_window, buffer, false, false)
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
                let text = self.string_bytes(name).to_vec();
                return Err(self.verror(
                    "Frame of the form F<num> named `%s' already exists",
                    &[Arg::Str(&text)],
                ));
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
                let name = self.string_bytes(self.symbol_name(prop)).to_vec();
                if !self.frame_live_p(val)
                    || (self.frame_live_p(f) && self.frame(f).terminal != self.frame(val).terminal)
                {
                    return Err(self.verror("Invalid `%s' frame parameter", &[Arg::Str(&name)]));
                }
                let mut frame1 = val;
                while self.frame_live_p(frame1) {
                    if frame1 == f {
                        return Err(self.verror(
                            "Circular specification of `%s' frame parameter",
                            &[Arg::Str(&name)],
                        ));
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
            return Ok(unspec);
        };
        self.tty_color_name(f, pixel)
    }

    pub fn do_switch_frame(
        &mut self,
        frame: Value,
        for_deletion: bool,
        norecord: Value,
    ) -> LispResult {
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
        self.move_minibuffers_onto_frame(sf, frame, for_deletion)?;
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
        OutputMethod::Pgtk => sym::PGTK,
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

pub(crate) fn f_window_system(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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
    lisp.do_switch_frame(frame, false, arg(args, 1))
}

fn f_handle_switch_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prefix = lisp.find_symbol_value(sym::CURRENT_PREFIX_ARG);
    lisp.set_kvar(crate::keyboard::Kvar::PrefixArg, prefix);
    lisp.run_hook(sym::MOUSE_LEAVE_BUFFER_HOOK)?;
    lisp.do_switch_frame(args[0], false, Value::NIL)
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

pub(crate) fn f_delete_frame(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let force = if arg(args, 1).is_nil() {
        Value::NIL
    } else {
        sym::T
    };
    lisp.delete_frame(arg(args, 0), force)
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

pub(crate) fn f_redirect_frame_focus(lisp: &mut Lisp, args: &[Value]) -> LispResult {
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
    let parameter = lisp.check_symbol(args[1])?;
    if !lisp.frame_live_p(f) {
        return Ok(Value::NIL);
    }
    let value = lisp.frame_get_param_value(f, parameter)?;
    Ok(if value == sym::UNBOUND {
        lisp.get_frame_param(f, parameter)
    } else {
        value
    })
}

pub(crate) fn f_modify_frame_parameters(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(args[0])?;
    if lisp.frame_window_p(f) {
        lisp.gui_set_frame_parameters(f, args[1])?;
        return Ok(Value::NIL);
    }
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResourceType {
    Number,
    BooleanNumber,
    Float,
    Boolean,
    String,
    Symbol,
}

#[derive(Clone, Copy)]
enum FrameFloatType {
    Width,
    Height,
    Left,
    Top,
}

#[derive(Default)]
struct FrameFloatCache {
    parent_done: i32,
    outer_done: i32,
    parent_width: i64,
    parent_height: i64,
    parent_left: i64,
    parent_top: i64,
    outer_minus_text_width: i64,
    outer_minus_text_height: i64,
}

pub fn c_atoi(s: &[u8]) -> i64 {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || (b'\t'..=b'\r').contains(&s[i])) {
        i += 1;
    }
    let negative = s.get(i) == Some(&b'-');
    if matches!(s.get(i), Some(b'-' | b'+')) {
        i += 1;
    }
    let mut n: i64 = 0;
    while let Some(&c @ b'0'..=b'9') = s.get(i) {
        n = n.saturating_mul(10).saturating_add(i64::from(c - b'0'));
        i += 1;
    }
    let n = if negative { n.saturating_neg() } else { n };
    i64::from(n as i32)
}

pub fn c_atof(s: &[u8]) -> f64 {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || (b'\t'..=b'\r').contains(&s[i])) {
        i += 1;
    }
    let start = i;
    if matches!(s.get(i), Some(b'-' | b'+')) {
        i += 1;
    }
    let mut digits = 0;
    while s.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
        digits += 1;
    }
    if s.get(i) == Some(&b'.') {
        i += 1;
        while s.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    let mantissa_end = i;
    if matches!(s.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        if matches!(s.get(j), Some(b'-' | b'+')) {
            j += 1;
        }
        if s.get(j).is_some_and(u8::is_ascii_digit) {
            while s.get(j).is_some_and(u8::is_ascii_digit) {
                j += 1;
            }
            i = j;
        }
    }
    let end = if i > mantissa_end { i } else { mantissa_end };
    std::str::from_utf8(&s[start..end])
        .ok()
        .and_then(|t| t.parse::<f64>().ok())
        .unwrap_or(0.0)
}

impl Lisp {
    pub(crate) fn xfloatint(&self, v: Value) -> f64 {
        if v.is_float() {
            self.heap.float_value(v)
        } else {
            v.as_fixnum().unwrap_or(0) as f64
        }
    }

    pub fn gui_display_get_arg(
        &mut self,
        dpyinfo: Option<Value>,
        alist: Value,
        param: Value,
        attribute: Option<&str>,
        class: Option<&str>,
        kind: ResourceType,
    ) -> LispResult {
        let mut tem = self.assq(param, alist);
        if !tem.is_nil() {
            self.heap.set_car(tem, Value::NIL);
            let mut tail = alist;
            while tail.is_cons() {
                let elt = self.xcar(tail);
                if elt.is_cons() && self.eq(self.xcar(elt), param) {
                    self.heap.set_car(elt, Value::NIL);
                }
                tail = self.xcdr(tail);
            }
        } else {
            let defaults = self.plain_value(sym::DEFAULT_FRAME_ALIST);
            tem = self.assq(param, defaults);
        }
        if tem.is_nil() {
            let (Some(attribute), Some(dpyinfo)) = (attribute, dpyinfo) else {
                return Ok(sym::UNBOUND);
            };
            let at = self.make_string(attribute.as_bytes());
            self.stack.push(at);
            let cl = self.make_string(class.unwrap_or("").as_bytes());
            self.stack.push(cl);
            let r = self.gui_display_get_resource(dpyinfo, at, cl, Value::NIL, Value::NIL);
            self.stack.truncate(self.stack.len() - 2);
            let tem = r?;
            if tem.is_nil() {
                return Ok(sym::UNBOUND);
            }
            let text = self.string_bytes(tem).to_vec();
            return Ok(match kind {
                ResourceType::Number => Value::fixnum(c_atoi(&text)),
                ResourceType::BooleanNumber => {
                    if text == b"on" || text == b"true" {
                        Value::fixnum(1)
                    } else {
                        Value::fixnum(c_atoi(&text))
                    }
                }
                ResourceType::Float => self.float(c_atof(&text)),
                ResourceType::Boolean => {
                    let lower = crate::casefiddle::f_downcase(self, &[tem])?;
                    let lower = self.string_bytes(lower);
                    self.bool(lower == b"on" || lower == b"true")
                }
                ResourceType::String => tem,
                ResourceType::Symbol => {
                    let lower = crate::casefiddle::f_downcase(self, &[tem])?;
                    let lower = self.string_bytes(lower).to_vec();
                    if lower == b"on" || lower == b"true" {
                        sym::T
                    } else if lower == b"off" || lower == b"false" {
                        Value::NIL
                    } else {
                        crate::lread::f_intern(self, &[tem, Value::NIL])?
                    }
                }
            });
        }
        self.cdr(tem)
    }

    pub fn frame_display_info(&self, f: Value) -> Option<Value> {
        self.frame(f).window_p().then(|| self.frame(f).terminal)
    }

    pub fn frame_res(&self, f: Value) -> f64 {
        self.display_info(self.frame(f).terminal).resy
    }

    pub fn gui_frame_get_arg(
        &mut self,
        f: Value,
        alist: Value,
        param: Value,
        attribute: Option<&str>,
        class: Option<&str>,
        kind: ResourceType,
    ) -> LispResult {
        let dpyinfo = self.frame_display_info(f);
        self.gui_display_get_arg(dpyinfo, alist, param, attribute, class, kind)
    }

    pub fn gui_frame_get_and_record_arg(
        &mut self,
        f: Value,
        alist: Value,
        param: Value,
        attribute: Option<&str>,
        class: Option<&str>,
        kind: ResourceType,
    ) -> LispResult {
        let value = self.gui_frame_get_arg(f, alist, param, attribute, class, kind)?;
        if !value.is_nil() && value != sym::UNBOUND {
            self.store_frame_param(f, param, value)?;
        }
        Ok(value)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn gui_default_parameter(
        &mut self,
        f: Value,
        alist: Value,
        prop: Value,
        deflt: Value,
        xprop: Option<&str>,
        xclass: Option<&str>,
        kind: ResourceType,
    ) -> LispResult {
        let mut tem = self.gui_frame_get_arg(f, alist, prop, xprop, xclass, kind)?;
        let was_unbound = tem == sym::UNBOUND;
        if was_unbound {
            tem = deflt;
        }
        let base = self.stack.len();
        self.stack.push(tem);
        let pair = self.cons(prop, tem);
        let arg = self.list(&[pair]);
        self.stack.push(arg);
        let r = self.gui_set_frame_parameters_1(f, arg, was_unbound);
        self.stack.truncate(base);
        r?;
        Ok(tem)
    }

    fn handle_frame_param(
        &mut self,
        f: Value,
        prop: Value,
        val: Value,
        old_value: Value,
    ) -> LispResult<()> {
        let index = self.get(prop, sym::X_FRAME_PARAMETER);
        let Some(index) = index
            .as_fixnum()
            .and_then(|i| usize::try_from(i).ok())
            .filter(|&i| i < FRAME_PARMS.len())
        else {
            return Ok(());
        };
        if let Some(handlers) = self.frame_terminal_hooks(f).frame_parm_handlers
            && let Some(Some(handler)) = handlers.get(index)
        {
            handler(self, f, val, old_value)?;
        }
        Ok(())
    }

    fn frame_float(
        &mut self,
        f: Value,
        val: Value,
        what: FrameFloatType,
        cache: &mut FrameFloatCache,
        default_value: i64,
    ) -> LispResult<i64> {
        let d_val = self.xfloatint(val);
        if !(0.0..=1.0).contains(&d_val) {
            return Ok(default_value);
        }
        let p = self.frame_parent_frame(f);
        if cache.parent_done == 1 {
        } else if !p.is_nil() {
            cache.parent_width = self.frame(p).geometry.pixel_width;
            cache.parent_height = self.frame(p).geometry.pixel_height;
            cache.parent_done = 1;
        } else if cache.parent_done == 0 {
            return Ok(default_value);
        } else if cache.parent_done == -1 {
            let monitor_attributes = self.funcall(sym::FRAME_MONITOR_ATTRIBUTES, &[f])?;
            if monitor_attributes.is_nil() {
                cache.parent_done = 0;
                return Ok(default_value);
            }
            let entry = self.assq(sym::WORKAREA, monitor_attributes);
            let workarea = self.cdr(entry)?;
            if workarea.is_nil() {
                cache.parent_done = 0;
                return Ok(default_value);
            }
            let fixnum = |lisp: &Lisp, n: usize| lisp.nth(n, workarea).as_fixnum().unwrap_or(0);
            cache.parent_left = fixnum(self, 0);
            cache.parent_top = fixnum(self, 1);
            cache.parent_width = fixnum(self, 2);
            cache.parent_height = fixnum(self, 3);
            cache.parent_done = 1;
        }
        let g = self.frame(f).geometry.clone();
        if cache.outer_done == 1 {
        } else if self.frame(f).flags.undecorated {
            cache.outer_minus_text_width = g.pixel_width - g.text_width;
            cache.outer_minus_text_height = g.pixel_height - g.text_height;
            cache.outer_done = 1;
        } else if cache.outer_done == 0 {
            return Ok(default_value);
        } else if cache.outer_done == -1 {
            let outer_edges = self.funcall(sym::FRAME_EDGES, &[f, sym::OUTER_EDGES])?;
            if !outer_edges.is_nil() {
                let fixnum =
                    |lisp: &Lisp, n: usize| lisp.nth(n, outer_edges).as_fixnum().unwrap_or(0);
                cache.outer_minus_text_width = fixnum(self, 2) - fixnum(self, 0) - g.text_width;
                cache.outer_minus_text_height = fixnum(self, 3) - fixnum(self, 1) - g.text_height;
            } else {
                cache.outer_minus_text_width = g.pixel_width - g.text_width;
                cache.outer_minus_text_height = g.pixel_height - g.text_height;
            }
            cache.outer_done = 1;
        }
        let parent_width = cache.parent_width as u32 as f64;
        let parent_height = cache.parent_height as u32 as f64;
        Ok(match what {
            FrameFloatType::Width => {
                (parent_width * d_val - cache.outer_minus_text_width as u32 as f64) as i32 as i64
            }
            FrameFloatType::Height => {
                (parent_height * d_val - cache.outer_minus_text_height as u32 as f64) as i32 as i64
            }
            FrameFloatType::Left => {
                let rest_width = (cache.parent_width as u32)
                    .wrapping_sub(g.text_width as u32)
                    .wrapping_sub(cache.outer_minus_text_width as u32)
                    as i32;
                if !p.is_nil() {
                    if rest_width <= 0 {
                        0
                    } else {
                        (d_val * f64::from(rest_width)) as i64
                    }
                } else if rest_width <= 0 {
                    cache.parent_left
                } else {
                    (cache.parent_left as f64 + d_val * f64::from(rest_width)) as i64
                }
            }
            FrameFloatType::Top => {
                let rest_height = (cache.parent_height as u32)
                    .wrapping_sub(g.text_height as u32)
                    .wrapping_sub(cache.outer_minus_text_height as u32)
                    as i32;
                if !p.is_nil() {
                    if rest_height <= 0 {
                        0
                    } else {
                        (d_val * f64::from(rest_height)) as i64
                    }
                } else if rest_height <= 0 {
                    cache.parent_top
                } else {
                    (cache.parent_top as f64 + d_val * f64::from(rest_height)) as i64
                }
            }
        })
    }

    pub fn gui_set_frame_parameters(&mut self, f: Value, alist: Value) -> LispResult<()> {
        self.gui_set_frame_parameters_1(f, alist, false)
    }

    pub fn gui_set_frame_parameters_1(
        &mut self,
        f: Value,
        alist: Value,
        default_parameter: bool,
    ) -> LispResult<()> {
        let mut size = 0;
        let mut tail = alist;
        while tail.is_cons() {
            size += 1;
            tail = self.xcdr(tail);
        }
        self.check_list_end(tail, alist)?;
        let base = self.stack.len();
        self.stack.resize(base + 2 * size, Value::NIL);
        let (mut i, mut j) = (0, size);
        let mut tail = alist;
        while tail.is_cons() {
            let elt = self.xcar(tail);
            let prop = self.car(elt)?;
            let val = self.cdr(elt)?;
            if prop == sym::FOREGROUND_COLOR || prop == sym::BACKGROUND_COLOR || prop == sym::FONT {
                j -= 1;
                self.stack[base + j] = prop;
                self.stack[base + size + j] = val;
            } else {
                self.stack[base + i] = prop;
                self.stack[base + size + i] = val;
                i += 1;
            }
            tail = self.xcdr(tail);
        }
        let r = self.gui_set_frame_parameters_2(f, base, size, default_parameter);
        self.stack.truncate(base);
        r
    }

    fn gui_set_frame_parameters_2(
        &mut self,
        f: Value,
        base: usize,
        size: usize,
        default_parameter: bool,
    ) -> LispResult<()> {
        let (mut width, mut height) = (-1, -1);
        let (mut width_change, mut height_change) = (false, false);
        let mut fullscreen = Value::NIL;
        let mut fullscreen_change = false;
        let mut cache = FrameFloatCache {
            parent_done: -1,
            outer_done: -1,
            ..FrameFloatCache::default()
        };
        let (mut left, mut top) = (sym::UNBOUND, sym::UNBOUND);
        let (mut icon_left, mut icon_top) = (sym::UNBOUND, sym::UNBOUND);
        for i in (0..size).rev() {
            let prop = self.stack[base + i];
            let val = self.stack[base + size + i];
            if prop == sym::WIDTH {
                width_change = true;
                let column_width = self.frame(f).geometry.column_width;
                if let Some(n) = val
                    .as_fixnum()
                    .filter(|&n| (0..=i64::from(i32::MAX)).contains(&n))
                {
                    width = n * column_width;
                } else if val.is_cons()
                    && self.xcar(val) == sym::TEXT_PIXELS
                    && let Some(n) = self
                        .xcdr(val)
                        .as_fixnum()
                        .filter(|&n| (0..=i64::from(i32::MAX)).contains(&n))
                {
                    width = n;
                } else if val.is_float() {
                    width = self.frame_float(f, val, FrameFloatType::Width, &mut cache, -1)?;
                } else {
                    width_change = false;
                }
            } else if prop == sym::HEIGHT {
                height_change = true;
                let line_height = self.frame(f).geometry.line_height;
                if let Some(n) = val
                    .as_fixnum()
                    .filter(|&n| (0..=i64::from(i32::MAX)).contains(&n))
                {
                    height = n * line_height;
                } else if val.is_cons()
                    && self.xcar(val) == sym::TEXT_PIXELS
                    && let Some(n) = self
                        .xcdr(val)
                        .as_fixnum()
                        .filter(|&n| (0..=i64::from(i32::MAX)).contains(&n))
                {
                    height = n;
                } else if val.is_float() {
                    height = self.frame_float(f, val, FrameFloatType::Height, &mut cache, -1)?;
                } else {
                    height_change = false;
                }
            } else if prop == sym::TOP {
                top = val;
            } else if prop == sym::LEFT {
                left = val;
            } else if prop == sym::ICON_TOP {
                icon_top = val;
            } else if prop == sym::ICON_LEFT {
                icon_left = val;
            } else if prop == sym::FULLSCREEN {
                fullscreen = val;
                fullscreen_change = true;
            } else {
                let old_value = self.get_frame_param(f, prop);
                self.stack.push(old_value);
                let r = self
                    .store_frame_param(f, prop, val)
                    .and_then(|()| self.handle_frame_param(f, prop, val, old_value));
                self.stack.pop();
                r?;
                if !default_parameter && prop == sym::FONT {
                    self.store_frame_param(f, sym::FONT_PARAMETER, val)?;
                }
            }
        }
        let mut left_no_change = false;
        let mut top_no_change = false;
        let g = self.frame(f).geometry.clone();
        if left == sym::UNBOUND {
            left_no_change = true;
            left = if g.left_pos < 0 {
                self.list(&[sym::PLUS, Value::fixnum(g.left_pos)])
            } else {
                Value::fixnum(g.left_pos)
            };
        }
        self.stack.push(left);
        if top == sym::UNBOUND {
            top_no_change = true;
            top = if g.top_pos < 0 {
                self.list(&[sym::PLUS, Value::fixnum(g.top_pos)])
            } else {
                Value::fixnum(g.top_pos)
            };
        }
        self.stack.push(top);
        let int_p = |v: Value| v.as_fixnum().is_some_and(|n| i32::try_from(n).is_ok());
        if !int_p(icon_left) {
            let param_alist = self.frame(f).param_alist;
            let entry = self.assq(sym::ICON_LEFT, param_alist);
            icon_left = self.cdr(entry)?;
            if icon_left.is_nil() {
                icon_left = Value::fixnum(0);
            }
        }
        if !int_p(icon_top) {
            let param_alist = self.frame(f).param_alist;
            let entry = self.assq(sym::ICON_TOP, param_alist);
            icon_top = self.cdr(entry)?;
            if icon_top.is_nil() {
                icon_top = Value::fixnum(0);
            }
        }
        let _ = (icon_left, icon_top);
        if width_change || height_change {
            let parameter = if width_change {
                if height_change {
                    sym::SIZE
                } else {
                    height = self.frame(f).geometry.text_height;
                    sym::WIDTH
                }
            } else {
                width = self.frame(f).geometry.text_width;
                sym::HEIGHT
            };
            self.adjust_frame_size(f, width, height, 1, parameter)?;
        }
        let (left_pos, top_pos) = (
            self.frame(f).geometry.left_pos,
            self.frame(f).geometry.top_pos,
        );
        if (!left.is_nil() || !top.is_nil())
            && !(left_no_change && top_no_change)
            && !(left.as_fixnum() == Some(left_pos) && top.as_fixnum() == Some(top_pos))
        {
            let mut leftpos = 0;
            let mut toppos = 0;
            let mut flags = self.frame(f).size_hint_flags & !(X_NEGATIVE | Y_NEGATIVE);
            for (value, is_left) in [(left, true), (top, false)] {
                let negative_flag = if is_left { X_NEGATIVE } else { Y_NEGATIVE };
                let mut pos = 0;
                if value == sym::MINUS {
                    flags |= negative_flag;
                } else if int_p(value) {
                    pos = value.as_fixnum().unwrap_or(0);
                    if pos < 0 {
                        flags |= negative_flag;
                    }
                } else if value.is_cons()
                    && self.xcar(value) == sym::MINUS
                    && self.xcdr(value).is_cons()
                    && let Some(n) = self
                        .xcar(self.xcdr(value))
                        .as_fixnum()
                        .filter(|&n| (-i64::from(i32::MAX)..=i64::from(i32::MAX)).contains(&n))
                {
                    pos = -n;
                    flags |= negative_flag;
                } else if value.is_cons()
                    && self.xcar(value) == sym::PLUS
                    && self.xcdr(value).is_cons()
                    && int_p(self.xcar(self.xcdr(value)))
                {
                    pos = self.xcar(self.xcdr(value)).as_fixnum().unwrap_or(0);
                } else if value.is_float() {
                    let what = if is_left {
                        FrameFloatType::Left
                    } else {
                        FrameFloatType::Top
                    };
                    pos = self.frame_float(f, value, what, &mut cache, 0)?;
                }
                if is_left {
                    leftpos = pos;
                } else {
                    toppos = pos;
                }
            }
            let fr = self.frame_mut(f);
            fr.size_hint_flags = flags;
            fr.geometry.top_pos = toppos;
            fr.geometry.left_pos = leftpos;
            fr.win_gravity = NORTH_WEST_GRAVITY;
            if let Some(hook) = self.frame_terminal_hooks(f).set_frame_offset {
                hook(self, f, leftpos, toppos, -1)?;
            }
        }
        if fullscreen_change {
            let old_value = self.get_frame_param(f, sym::FULLSCREEN);
            self.store_frame_param(f, sym::FULLSCREEN, fullscreen)?;
            if !self.eq(fullscreen, old_value) {
                self.gui_set_fullscreen(f, fullscreen, old_value)?;
            }
        }
        Ok(())
    }

    pub fn gui_set_fullscreen(
        &mut self,
        f: Value,
        new_value: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        let want = if new_value.is_nil() {
            Some(Fullscreen::None)
        } else if new_value == sym::FULLBOTH || new_value == sym::FULLSCREEN {
            Some(Fullscreen::Both)
        } else if new_value == sym::FULLWIDTH {
            Some(Fullscreen::Width)
        } else if new_value == sym::FULLHEIGHT {
            Some(Fullscreen::Height)
        } else if new_value == sym::MAXIMIZED {
            Some(Fullscreen::Maximized)
        } else {
            None
        };
        if let Some(want) = want {
            self.frame_mut(f).want_fullscreen = want;
        }
        if let Some(hook) = self.frame_terminal_hooks(f).fullscreen {
            hook(self, f)?;
        }
        Ok(())
    }

    pub fn gui_set_line_spacing(
        &mut self,
        f: Value,
        new_value: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        let line_height = self.frame(f).geometry.line_height;
        let int_value = |v: Value| {
            v.as_fixnum()
                .filter(|&n| (0..=i64::from(i32::MAX)).contains(&n))
        };
        let (spacing, above) = if new_value.is_nil() {
            (0, 0)
        } else if let Some(n) = int_value(new_value) {
            (n, 0)
        } else if new_value.is_float() {
            let new_spacing = (self.xfloatint(new_value) * line_height as f64) as i32 as i64;
            if new_spacing >= 0 {
                (new_spacing, 0)
            } else {
                return Err(self.signal_error("Invalid line-spacing", new_value));
            }
        } else if new_value.is_cons() {
            let above = self.xcar(new_value);
            let below = self.xcdr(new_value);
            if let (Some(a), Some(b)) = (int_value(above), int_value(below)) {
                (a + b, a)
            } else if above.is_float() && below.is_float() {
                let (a, b) = (self.xfloatint(above), self.xfloatint(below));
                let new_spacing = ((a + b) * line_height as f64) as i32 as i64;
                let spacing_above = (a * line_height as f64) as i32 as i64;
                if new_spacing >= 0 && spacing_above >= 0 {
                    (new_spacing, spacing_above)
                } else {
                    return Err(self.signal_error("Invalid line-spacing", new_value));
                }
            } else {
                return Err(self.signal_error("Invalid line-spacing", new_value));
            }
        } else {
            return Err(self.signal_error("Invalid line-spacing", new_value));
        };
        let g = &mut self.frame_mut(f).geometry;
        g.extra_line_spacing = spacing;
        g.extra_line_spacing_above = above;
        if self.frame(f).flags.visible {
            self.redraw_frame(f);
        }
        Ok(())
    }

    pub fn gui_set_screen_gamma(
        &mut self,
        f: Value,
        new_value: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        if new_value.is_nil() {
            self.frame_mut(f).gamma = 0.0;
        } else if self.is_number(new_value) && self.xfloatint(new_value) > 0.0 {
            let v = self.xfloatint(new_value);
            self.frame_mut(f).gamma = 1.0 / (0.4545 * v);
        } else {
            return Err(self.signal_error("Invalid screen-gamma", new_value));
        }
        let param_alist = self.frame(f).param_alist;
        let bgcolor = self.assq(sym::BACKGROUND_COLOR, param_alist);
        if bgcolor.is_cons() {
            let bgcolor = self.xcdr(bgcolor);
            if self.is_string(bgcolor) {
                self.handle_frame_param(f, sym::BACKGROUND_COLOR, bgcolor, Value::NIL)?;
            }
        }
        self.clear_face_cache(true);
        self.fset_redisplay(f);
        Ok(())
    }

    pub fn gui_set_visibility(
        &mut self,
        f: Value,
        value: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        if value.is_nil() {
            f_make_frame_invisible(self, &[f, sym::T])?;
        } else if value == sym::ICON {
            f_iconify_frame(self, &[f])?;
        } else {
            f_make_frame_visible(self, &[f])?;
        }
        Ok(())
    }

    pub fn gui_set_autoraise(&mut self, f: Value, arg: Value, _old_value: Value) -> LispResult<()> {
        self.frame_mut(f).flags.auto_raise = !arg.is_nil();
        Ok(())
    }

    pub fn gui_set_autolower(&mut self, f: Value, arg: Value, _old_value: Value) -> LispResult<()> {
        self.frame_mut(f).flags.auto_lower = !arg.is_nil();
        Ok(())
    }

    pub fn gui_set_unsplittable(
        &mut self,
        f: Value,
        arg: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        self.frame_mut(f).flags.no_split = !arg.is_nil();
        Ok(())
    }

    pub fn gui_set_border_width(
        &mut self,
        f: Value,
        arg: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        let border_width =
            self.check_integer_range(arg, i64::from(i32::MIN), i64::from(i32::MAX))?;
        if border_width == self.frame(f).geometry.border_width {
            return Ok(());
        }
        if self.frame(f).flags.native_window {
            return Err(self.error("Cannot change the border width of a frame"));
        }
        self.frame_mut(f).geometry.border_width = border_width;
        Ok(())
    }

    fn gui_set_fringe(&mut self, f: Value, new_value: Value, left: bool) -> LispResult<()> {
        let g = self.frame(f).geometry.clone();
        let unit = g.column_width;
        let old_width = if left {
            g.left_fringe_width
        } else {
            g.right_fringe_width
        };
        let new_width = match new_value
            .as_fixnum()
            .filter(|&n| (-i64::from(i32::MAX)..=i64::from(i32::MAX)).contains(&n))
        {
            Some(n) => n.abs(),
            None => 8,
        };
        if new_width != old_width {
            let other = if left {
                g.right_fringe_width
            } else {
                g.left_fringe_width
            };
            let geometry = &mut self.frame_mut(f).geometry;
            if left {
                geometry.left_fringe_width = new_width;
            } else {
                geometry.right_fringe_width = new_width;
            }
            geometry.fringe_cols = (new_width + other + unit - 1) / unit;
            if self.frame(f).flags.native_window {
                let parameter = if left {
                    sym::LEFT_FRINGE
                } else {
                    sym::RIGHT_FRINGE
                };
                self.adjust_frame_size(f, -1, -1, 3, parameter)?;
            }
            self.set_frame_garbaged(f);
        }
        Ok(())
    }

    pub fn gui_set_left_fringe(
        &mut self,
        f: Value,
        new_value: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        self.gui_set_fringe(f, new_value, true)
    }

    pub fn gui_set_right_fringe(
        &mut self,
        f: Value,
        new_value: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        self.gui_set_fringe(f, new_value, false)
    }

    pub fn gui_set_vertical_scroll_bars(
        &mut self,
        f: Value,
        arg: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        let current = self.frame(f).vertical_scroll_bar_type;
        let has = current != VerticalScrollBarType::None;
        if (arg == sym::LEFT && current == VerticalScrollBarType::Right)
            || (arg == sym::RIGHT && current == VerticalScrollBarType::Left)
            || (arg.is_nil() && has)
            || (!arg.is_nil() && !has)
        {
            let default = self.plain_value(sym::DEFAULT_FRAME_SCROLL_BARS);
            self.frame_mut(f).vertical_scroll_bar_type = if arg.is_nil() {
                VerticalScrollBarType::None
            } else if arg == sym::LEFT {
                VerticalScrollBarType::Left
            } else if arg == sym::RIGHT {
                VerticalScrollBarType::Right
            } else if default == sym::LEFT {
                VerticalScrollBarType::Left
            } else if default == sym::RIGHT {
                VerticalScrollBarType::Right
            } else {
                VerticalScrollBarType::None
            };
            if self.frame(f).flags.native_window {
                self.adjust_frame_size(f, -1, -1, 3, sym::VERTICAL_SCROLL_BARS)?;
            }
            self.set_frame_garbaged(f);
        }
        Ok(())
    }

    pub fn gui_set_horizontal_scroll_bars(
        &mut self,
        _f: Value,
        _arg: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        Ok(())
    }

    pub fn gui_set_scroll_bar_height(
        &mut self,
        _f: Value,
        _arg: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        Ok(())
    }

    pub fn gui_set_alpha(&mut self, f: Value, arg: Value, _old_value: Value) -> LispResult<()> {
        let mut newval = [1.0; 2];
        let mut arg = arg;
        for slot in &mut newval {
            let item = if arg.is_cons() {
                let item = self.xcar(arg);
                arg = self.xcdr(arg);
                item
            } else {
                arg
            };
            *slot = if item.is_nil() {
                -1.0
            } else if item.is_float() {
                let alpha = self.xfloatint(item);
                if !(0.0..=1.0).contains(&alpha) {
                    let (lo, hi) = (self.float(0.0), self.float(1.0));
                    return Err(self.args_out_of_range(lo, hi));
                }
                alpha
            } else if let Some(ialpha) = item.as_fixnum() {
                if !(0..=100).contains(&ialpha) {
                    return Err(self.args_out_of_range(Value::fixnum(0), Value::fixnum(100)));
                }
                ialpha as f64 / 100.0
            } else {
                return Err(self.wrong_type(sym::NUMBERP, item));
            };
        }
        self.frame_mut(f).alpha = newval;
        if let Some(hook) = self.frame_terminal_hooks(f).set_frame_alpha {
            hook(self, f)?;
        }
        Ok(())
    }

    pub fn gui_set_alpha_background(
        &mut self,
        f: Value,
        arg: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        let alpha = if arg.is_nil() {
            1.0
        } else if arg.is_float() {
            let alpha = self.xfloatint(arg);
            if !(0.0..=1.0).contains(&alpha) {
                let (lo, hi) = (self.float(0.0), self.float(1.0));
                return Err(self.args_out_of_range(lo, hi));
            }
            alpha
        } else if let Some(ialpha) = arg.as_fixnum() {
            if !(0..=100).contains(&ialpha) {
                return Err(self.args_out_of_range(Value::fixnum(0), Value::fixnum(100)));
            }
            ialpha as f64 / 100.0
        } else {
            return Err(self.wrong_type(sym::NUMBERP, arg));
        };
        self.frame_mut(f).alpha_background = alpha;
        self.recompute_basic_faces(f)?;
        self.set_frame_garbaged(f);
        Ok(())
    }

    pub fn gui_set_borders_respect_alpha_background(
        &mut self,
        f: Value,
        arg: Value,
        _old_value: Value,
    ) -> LispResult<()> {
        self.frame_mut(f).flags.borders_respect_alpha_background = !arg.is_nil();
        Ok(())
    }

    pub fn gui_set_no_special_glyphs(
        &mut self,
        f: Value,
        new_value: Value,
        old_value: Value,
    ) -> LispResult<()> {
        if !self.eq(new_value, old_value) {
            self.frame_mut(f).flags.no_special_glyphs = !new_value.is_nil();
        }
        Ok(())
    }

    pub fn gui_set_font(&mut self, f: Value, arg: Value, oldval: Value) -> LispResult<()> {
        let base = self.stack.len();
        self.stack.extend([arg, oldval]);
        let r = self.gui_set_font_1(f, arg, oldval);
        self.stack.truncate(base);
        r
    }

    fn gui_set_font_1(&mut self, f: Value, arg: Value, oldval: Value) -> LispResult<()> {
        use crate::font::FONT_NAME_INDEX;
        let mut arg = arg;
        self.store_frame_param(f, sym::FONT, oldval)?;
        let font_object;
        let mut fontset;
        if arg.is_string() {
            fontset = self.fs_query_fontset(arg, 0)?;
            if fontset < 0 {
                font_object = self.font_open_by_name(f, arg)?;
                if font_object.is_nil() {
                    let name = self.string_bytes(arg).to_vec();
                    return Err(self.verror("Font `%s' is not defined", &[Arg::Str(&name)]));
                }
                arg = self.font_aref(font_object, FONT_NAME_INDEX);
            } else if fontset > 0 {
                let ascii = self.fontset_ascii(fontset as usize);
                font_object = self.font_open_by_name(f, ascii)?;
                if font_object.is_nil() {
                    let name = self.string_bytes(arg).to_vec();
                    return Err(self.verror("Font `%s' is not defined", &[Arg::Str(&name)]));
                }
                arg = self.font_aref(font_object, FONT_NAME_INDEX);
            } else {
                return Err(self.error("The default fontset can't be used for a frame font"));
            }
        } else if arg.is_cons() && self.xcar(arg).is_string() && self.font_object_p(self.xcdr(arg))
        {
            let name = self.xcar(arg);
            fontset = self.fs_query_fontset(name, 0)?;
            if fontset < 0 {
                let name = self.string_bytes(name).to_vec();
                return Err(self.verror("Unknown fontset: %s", &[Arg::Str(&name)]));
            }
            font_object = self.xcdr(arg);
            arg = self.font_aref(font_object, FONT_NAME_INDEX);
        } else if self.font_object_p(arg) {
            font_object = arg;
            arg = self.font_aref(font_object, FONT_NAME_INDEX);
            fontset = self.frame(f).fontset;
            if fontset >= 0 {
                let ascii_font = self.fontset_ascii(fontset as usize);
                let spec = self.font_spec_from_name(ascii_font)?;
                if spec.is_nil() || !self.font_match_p(spec, font_object)? {
                    fontset = -1;
                }
            }
        } else {
            return Err(self.signal_error("Invalid font", arg));
        }
        self.stack.extend([font_object, arg]);
        if self.equal(font_object, oldval)? {
            return Ok(());
        }
        if let Some(set_new_font) = self.frame_terminal_hooks(f).set_new_font {
            set_new_font(self, f, font_object, fontset)?;
        }
        self.store_frame_param(f, sym::FONT, arg)?;
        self.frame_mut(f).n_tab_bar_rows = 0;
        self.frame_mut(f).n_tool_bar_rows = 0;
        self.set_frame_garbaged(f);
        self.frame_mut(f).fonts_changed = true;
        Ok(())
    }

    pub fn gui_set_font_backend(
        &mut self,
        f: Value,
        new_value: Value,
        old_value: Value,
    ) -> LispResult<()> {
        let base = self.stack.len();
        self.stack.extend([new_value, old_value]);
        let r = self.gui_set_font_backend_1(f, new_value, old_value);
        self.stack.truncate(base);
        r
    }

    fn gui_set_font_backend_1(
        &mut self,
        f: Value,
        new_value: Value,
        old_value: Value,
    ) -> LispResult<()> {
        let mut new_value = new_value;
        if !new_value.is_nil() && !new_value.is_cons() {
            self.check_string(new_value)?;
            let bytes = crate::fontset::c_bytes(self.string_bytes(new_value)).to_vec();
            let mut names = Vec::new();
            let mut p0 = 0;
            while p0 < bytes.len() {
                let mut p1 = p0;
                while p1 < bytes.len() && !c_isspace(bytes[p1]) && bytes[p1] != b',' {
                    p1 += 1;
                }
                if p0 < p1 {
                    let name = self.make_string(&bytes[p0..p1]);
                    let symbol = crate::lread::f_intern(self, &[name, Value::NIL])?;
                    names.push(symbol);
                }
                if p1 < bytes.len() {
                    p1 += 1;
                    while p1 < bytes.len() && c_isspace(bytes[p1]) {
                        p1 += 1;
                    }
                }
                p0 = p1;
            }
            new_value = self.list(&names);
            self.stack.push(new_value);
        }
        if !old_value.is_nil() && self.equal(old_value, new_value)? {
            return Ok(());
        }
        let has_font = !self.frame(f).font.is_nil();
        if has_font {
            self.free_all_realized_faces(f);
        }
        let requested = if new_value.is_nil() {
            sym::T
        } else {
            new_value
        };
        let new_value = self.font_update_drivers(f, requested)?;
        if new_value.is_nil() {
            if old_value.is_nil() {
                return Err(self.error("No font backend available"));
            }
            self.font_update_drivers(f, old_value)?;
            return Err(self.error("None of specified font backends are available"));
        }
        self.stack.push(new_value);
        self.store_frame_param(f, sym::FONT_BACKEND, new_value)?;
        if has_font {
            if let Some(default_font_parameter) =
                self.frame_terminal_hooks(f).default_font_parameter
            {
                default_font_parameter(self, f, Value::NIL)?;
            }
            self.faces.face_change = true;
            self.windows.windows_or_buffers_changed = 18;
        }
        Ok(())
    }
}

fn c_isspace(c: u8) -> bool {
    c == b' ' || (b'\t'..=b'\r').contains(&c)
}
