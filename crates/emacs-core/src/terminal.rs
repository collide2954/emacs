use crate::coding::CodingSystem;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::Vectorlike;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutputMethod {
    Initial,
    Pgtk,
}

pub type StringResourceHook = fn(&mut Lisp, &[u8], &[u8]) -> LispResult<Option<Vec<u8>>>;
pub type FrameParmHandler = fn(&mut Lisp, Value, Value, Value) -> LispResult<()>;
pub type FrameHook = fn(&mut Lisp, Value) -> LispResult<()>;
pub type FrameOffsetHook = fn(&mut Lisp, Value, i64, i64, i64) -> LispResult<()>;
pub type DefinedColorHook =
    fn(&mut Lisp, Value, &[u8]) -> LispResult<(bool, crate::xfaces::EmacsColor)>;
pub type SetNewFontHook = fn(&mut Lisp, Value, Value, i64) -> LispResult<Value>;
pub type DefaultFontParameterHook = fn(&mut Lisp, Value, Value) -> LispResult<()>;
pub type WindowSizeHook = fn(&mut Lisp, Value, bool, i64, i64) -> LispResult<()>;

#[derive(Clone, Copy, Default, Debug)]
pub struct TerminalHooks {
    pub ring_bell: Option<fn(&mut Lisp, Value)>,
    pub toggle_invisible_pointer: Option<fn(&mut Lisp, Value, bool)>,
    pub frame_raise_lower: Option<fn(&mut Lisp, Value, bool)>,
    pub iconify_frame: Option<fn(&mut Lisp, Value)>,
    pub get_string_resource: Option<StringResourceHook>,
    pub defined_color: Option<DefinedColorHook>,
    pub frame_parm_handlers: Option<&'static [Option<FrameParmHandler>]>,
    pub fullscreen: Option<FrameHook>,
    pub set_frame_offset: Option<FrameOffsetHook>,
    pub set_frame_alpha: Option<FrameHook>,
    pub set_scroll_bar_default_width: Option<FrameHook>,
    pub set_scroll_bar_default_height: Option<FrameHook>,
    pub set_new_font: Option<SetNewFontHook>,
    pub default_font_parameter: Option<DefaultFontParameterHook>,
    pub set_window_size: Option<WindowSizeHook>,
    pub frame_visible_invisible: Option<fn(&mut Lisp, Value, bool)>,
    pub delete_frame: Option<fn(&mut Lisp, Value)>,
    pub delete_terminal: Option<fn(&mut Lisp, Value) -> LispResult<()>>,
    pub read_socket: Option<ReadSocketHook>,
    pub update_begin: Option<fn(&mut Lisp, Value)>,
    pub update_end: Option<fn(&mut Lisp, Value)>,
}

pub type ReadSocketHook = fn(&mut Lisp, Value, &mut crate::termhooks::KbdEvent) -> LispResult<i32>;

#[derive(Clone, Copy, Debug)]
pub struct WindowSystem {
    pub mark: fn(&mut Vec<Value>),
    pub check_x_display_info: fn(&mut Lisp, Value) -> LispResult<Value>,
    pub get_keysym_name: fn(u32) -> Option<&'static str>,
    pub wait_for_input: fn(&mut Lisp, std::time::Duration) -> bool,
}

#[derive(Clone, Debug)]
pub struct DisplayInfo {
    pub name_list_element: Value,
    pub resx: f64,
    pub resy: f64,
    pub color_p: bool,
    pub n_planes: i32,
    pub root_window: i64,
    pub n_fonts: i32,
    pub smallest_font_height: i32,
    pub smallest_char_width: i32,
    pub reference_count: i32,
    pub highlight_frame: Value,
    pub mouse_highlight: crate::dispextern::MouseHlInfo,
}

impl DisplayInfo {
    pub fn new(name_list_element: Value) -> DisplayInfo {
        DisplayInfo {
            name_list_element,
            resx: 0.0,
            resy: 0.0,
            color_p: false,
            n_planes: 0,
            root_window: 0,
            n_fonts: 0,
            smallest_font_height: 0,
            smallest_char_width: 0,
            reference_count: 0,
            highlight_frame: Value::NIL,
            mouse_highlight: crate::dispextern::MouseHlInfo::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Terminal {
    pub param_alist: Value,
    pub charset_list: Value,
    pub selection_alist: Value,
    pub glyph_code_table: Value,
    pub id: i64,
    pub reference_count: i64,
    pub kind: OutputMethod,
    pub name: Option<String>,
    pub kboard: Option<usize>,
    pub terminal_coding: Box<CodingSystem>,
    pub keyboard_coding: Box<CodingSystem>,
    pub hooks: TerminalHooks,
    pub rif: Option<&'static crate::dispextern::RedisplayInterface>,
    pub display_info: Option<Box<DisplayInfo>>,
}

impl Terminal {
    pub fn trace(&self, out: &mut Vec<Value>) {
        out.push(self.param_alist);
        out.push(self.charset_list);
        out.push(self.selection_alist);
        out.push(self.glyph_code_table);
        for c in [&self.terminal_coding, &self.keyboard_coding] {
            out.push(c.name);
            out.push(c.eol_type);
        }
        if let Some(info) = &self.display_info {
            out.push(info.name_list_element);
            out.push(info.highlight_frame);
            info.mouse_highlight.trace(out);
        }
    }
}

pub struct Terminals {
    pub list: Vec<Value>,
    pub next_id: i64,
    pub initial: Value,
}

impl Default for Terminals {
    fn default() -> Self {
        Terminals {
            list: Vec::new(),
            next_id: 0,
            initial: Value::NIL,
        }
    }
}

impl Terminals {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&self.list);
        out.push(self.initial);
    }
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("delete-terminal", 0, 2, f_delete_terminal),
    Subr::fixed("frame-terminal", 0, 1, f_frame_terminal),
    Subr::fixed("terminal-live-p", 1, 1, f_terminal_live_p),
    Subr::fixed("frame-initial-p", 0, 1, f_frame_initial_p),
    Subr::fixed("terminal-list", 0, 0, f_terminal_list),
    Subr::fixed("terminal-name", 0, 1, f_terminal_name),
    Subr::fixed("terminal-parameters", 0, 1, f_terminal_parameters),
    Subr::fixed("terminal-parameter", 2, 2, f_terminal_parameter),
    Subr::fixed("set-terminal-parameter", 3, 3, f_set_terminal_parameter),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::RING_BELL_FUNCTION, Value::NIL);
    lisp.defvar(sym::DELETE_TERMINAL_FUNCTIONS, Value::NIL);
    let feature = lisp.intern("multi-tty");
    crate::fns::f_provide(lisp, &[feature, Value::NIL]).expect("provide multi-tty");
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

impl Lisp {
    pub fn is_terminal(&self, v: Value) -> bool {
        v.is_vectorlike() && matches!(self.heap.vectorlike_ref(v), Vectorlike::Terminal(_))
    }

    pub fn terminal(&self, v: Value) -> &Terminal {
        match self.heap.vectorlike_ref(v) {
            Vectorlike::Terminal(t) => t,
            _ => unreachable!("not a terminal"),
        }
    }

    pub fn terminal_mut(&mut self, v: Value) -> &mut Terminal {
        match self.heap.vectorlike_mut(v) {
            Vectorlike::Terminal(t) => t,
            _ => unreachable!("not a terminal"),
        }
    }

    pub fn decode_terminal(&self, terminal: Value) -> Option<Value> {
        let terminal = if terminal.is_nil() {
            self.frames.selected_frame
        } else {
            terminal
        };
        let t = if self.is_terminal(terminal) {
            terminal
        } else if self.is_frame(terminal) {
            self.frame(terminal).terminal
        } else {
            return None;
        };
        (self.is_terminal(t) && self.terminal(t).name.is_some()).then_some(t)
    }

    pub fn decode_live_terminal(&mut self, terminal: Value) -> LispResult<Value> {
        match self.decode_terminal(terminal) {
            Some(t) => Ok(t),
            None => Err(self.wrong_type(sym::TERMINAL_LIVE_P, terminal)),
        }
    }

    fn default_coding(&mut self, variable: Value, fallback: Value) -> LispResult<CodingSystem> {
        let mut coding = self.find_symbol_value(variable);
        if coding.is_nil() || coding == sym::UNBOUND || !self.coding_system_p(coding)? {
            coding = fallback;
        }
        self.setup_coding_system(coding)
    }

    pub fn create_terminal(&mut self, kind: OutputMethod) -> LispResult<Value> {
        let keyboard_coding =
            self.default_coding(sym::DEFAULT_KEYBOARD_CODING_SYSTEM, sym::NO_CONVERSION)?;
        let terminal_coding =
            self.default_coding(sym::DEFAULT_TERMINAL_CODING_SYSTEM, sym::UNDECIDED)?;
        let id = self.terminals.next_id;
        self.terminals.next_id += 1;
        let terminal = self
            .heap
            .vectorlike(Vectorlike::Terminal(Box::new(Terminal {
                param_alist: Value::NIL,
                charset_list: Value::NIL,
                selection_alist: Value::NIL,
                glyph_code_table: Value::NIL,
                id,
                reference_count: 0,
                kind,
                name: None,
                kboard: None,
                terminal_coding: Box::new(terminal_coding),
                keyboard_coding: Box::new(keyboard_coding),
                hooks: TerminalHooks::default(),
                rif: None,
                display_info: None,
            })));
        self.terminals.list.insert(0, terminal);
        Ok(terminal)
    }

    pub fn init_initial_terminal(&mut self) -> LispResult<Value> {
        let terminal = self.create_terminal(OutputMethod::Initial)?;
        let kboard = self.initial_kboard;
        let t = self.terminal_mut(terminal);
        t.name = Some("initial_terminal".to_string());
        t.kboard = Some(kboard);
        t.hooks.delete_terminal = Some(delete_initial_terminal);
        t.hooks.delete_frame = Some(initial_free_frame_resources);
        self.terminals.initial = terminal;
        Ok(terminal)
    }

    pub fn delete_terminal(&mut self, terminal: Value) -> LispResult<()> {
        if self.terminal(terminal).name.is_none() {
            return Ok(());
        }
        self.terminal_mut(terminal).name = None;
        let base = self.stack.len();
        let frames = self.frames.list.clone();
        self.stack.extend_from_slice(&frames);
        for frame in frames {
            if self.frame_live_p(frame)
                && self.frame(frame).terminal == terminal
                && let Err(e) = self.delete_frame(frame, sym::NOELISP)
            {
                self.stack.truncate(base);
                return Err(e);
            }
        }
        self.stack.truncate(base);
        self.delete_terminal_internal(terminal);
        Ok(())
    }

    pub fn delete_terminal_internal(&mut self, terminal: Value) {
        let Some(i) = self.terminals.list.iter().position(|&t| t == terminal) else {
            panic!("emacs_abort: terminal is not on the terminal list");
        };
        self.terminals.list.remove(i);
        if let Some(kb) = self.terminal(terminal).kboard {
            self.kboards[kb].reference_count -= 1;
            if self.kboards[kb].reference_count == 0 {
                self.delete_kboard(kb);
                self.terminal_mut(terminal).kboard = None;
            }
        }
    }

    fn terminal_active_p(&self, terminal: Value) -> bool {
        match self.terminal(terminal).kind {
            OutputMethod::Initial | OutputMethod::Pgtk => true,
        }
    }

    fn store_terminal_param(&mut self, terminal: Value, parameter: Value, value: Value) -> Value {
        let alist = self.terminal(terminal).param_alist;
        let old = self.assq(parameter, alist);
        if old.is_nil() {
            let cell = self.cons(parameter, value);
            let alist = self.cons(cell, alist);
            self.terminal_mut(terminal).param_alist = alist;
            Value::NIL
        } else {
            let result = self.xcdr(old);
            self.heap.set_cdr(old, value);
            result
        }
    }

    pub fn display_info(&self, t: Value) -> &DisplayInfo {
        self.terminal(t)
            .display_info
            .as_deref()
            .expect("terminal without display info")
    }

    pub fn display_info_mut(&mut self, t: Value) -> &mut DisplayInfo {
        self.terminal_mut(t)
            .display_info
            .as_deref_mut()
            .expect("terminal without display info")
    }

    pub fn terminal_glyph_code(&self, t: Value, _ch: u32) -> Value {
        match self.terminal(t).kind {
            OutputMethod::Initial | OutputMethod::Pgtk => Value::NIL,
        }
    }

    pub fn terminal_type_symbol(&self, terminal: Value) -> Value {
        match self.terminal(terminal).kind {
            OutputMethod::Initial => sym::T,
            OutputMethod::Pgtk => sym::PGTK,
        }
    }
}

fn initial_free_frame_resources(lisp: &mut Lisp, f: Value) {
    lisp.free_frame_faces(f);
}

fn delete_initial_terminal(lisp: &mut Lisp, terminal: Value) -> LispResult<()> {
    if terminal != lisp.terminals.initial {
        panic!("emacs_abort: not the initial terminal");
    }
    lisp.delete_terminal(terminal)?;
    lisp.terminals.initial = Value::NIL;
    Ok(())
}

pub(crate) fn f_delete_terminal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (terminal, force) = (arg(args, 0), arg(args, 1));
    let Some(t) = lisp.decode_terminal(terminal) else {
        return Ok(Value::NIL);
    };
    if force.is_nil()
        && !lisp
            .terminals
            .list
            .iter()
            .any(|&p| p != t && lisp.terminal_active_p(p))
    {
        return Err(lisp.error("Attempt to delete the sole active display terminal"));
    }
    if !lisp.plain_value(sym::RUN_HOOKS_ENABLED).is_nil() {
        if force == sym::NOELISP {
            let call = lisp.list(&[
                sym::RUN_HOOK_WITH_ARGS,
                sym::DELETE_TERMINAL_FUNCTIONS,
                terminal,
            ]);
            lisp.pending_funcalls = lisp.cons(call, lisp.pending_funcalls);
        } else {
            lisp.safe_calln(
                sym::RUN_HOOK_WITH_ARGS,
                &[sym::DELETE_TERMINAL_FUNCTIONS, terminal],
            )?;
        }
    }
    match lisp.terminal(t).hooks.delete_terminal {
        Some(hook) => hook(lisp, t)?,
        None => lisp.delete_terminal(t)?,
    }
    Ok(Value::NIL)
}

fn f_frame_terminal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let f = lisp.decode_live_frame(arg(args, 0))?;
    let t = lisp.frame(f).terminal;
    Ok(if lisp.is_terminal(t) { t } else { Value::NIL })
}

fn f_terminal_live_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(match lisp.decode_terminal(args[0]) {
        Some(t) => lisp.terminal_type_symbol(t),
        None => Value::NIL,
    })
}

fn f_frame_initial_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let frame = if arg(args, 0).is_nil() {
        lisp.frames.selected_frame
    } else {
        arg(args, 0)
    };
    if lisp.is_frame(frame) {
        let initial =
            lisp.frame_live_p(frame) && lisp.frame(frame).output_method == OutputMethod::Initial;
        return Ok(lisp.bool(initial));
    }
    let initial = lisp
        .decode_terminal(frame)
        .is_some_and(|t| lisp.terminal(t).kind == OutputMethod::Initial);
    Ok(lisp.bool(initial))
}

fn f_terminal_list(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let mut terminals = Value::NIL;
    let list = lisp.terminals.list.clone();
    for t in list {
        terminals = lisp.cons(t, terminals);
    }
    Ok(terminals)
}

fn f_terminal_name(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let t = lisp.decode_live_terminal(arg(args, 0))?;
    Ok(match lisp.terminal(t).name.clone() {
        Some(name) => lisp.string(&name),
        None => Value::NIL,
    })
}

fn f_terminal_parameters(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let t = lisp.decode_live_terminal(arg(args, 0))?;
    let alist = lisp.terminal(t).param_alist;
    crate::fns::f_copy_alist(lisp, &[alist])
}

pub(crate) fn f_terminal_parameter(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let parameter = lisp.check_symbol(args[1])?;
    let t = lisp.decode_live_terminal(args[0])?;
    let alist = lisp.terminal(t).param_alist;
    let cell = lisp.assq(parameter, alist);
    Ok(lisp.cdr_safe(cell))
}

fn f_set_terminal_parameter(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let t = lisp.decode_live_terminal(args[0])?;
    Ok(lisp.store_terminal_param(t, args[1], args[2]))
}
