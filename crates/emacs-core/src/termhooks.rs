use crate::symbols as sym;
use crate::value::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EventKind {
    #[default]
    NoEvent,
    AsciiKeystroke,
    MultibyteCharKeystroke,
    NonAsciiKeystroke,
    Timer,
    MouseClick,
    Wheel,
    HorizWheel,
    ScrollBarClick,
    HorizontalScrollBarClick,
    SelectionRequest,
    SelectionClear,
    DeleteWindow,
    MenuBar,
    Iconify,
    Deiconify,
    MenuBarActivate,
    DragNDrop,
    UserSignal,
    Help,
    TabBar,
    ToolBar,
    FocusIn,
    FocusOut,
    MoveFrame,
    SelectWindow,
    SaveSession,
    Thread,
    ConfigChanged,
    FileNotify,
    Sleep,
    PreeditText,
    TouchEnd,
    TouchscreenUpdate,
    TouchscreenBegin,
    TouchscreenEnd,
    Pinch,
    MonitorsChanged,
    ToolkitThemeChanged,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScrollBarPart {
    #[default]
    Nowhere,
    AboveHandle,
    Handle,
    BelowHandle,
    UpArrow,
    DownArrow,
    ToTop,
    ToBottom,
    EndScroll,
    MoveRatio,
    BeforeHandle,
    HorizontalHandle,
    AfterHandle,
    LeftArrow,
    RightArrow,
    ToLeftmost,
    ToRightmost,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KbdEvent {
    pub kind: EventKind,
    pub part: ScrollBarPart,
    pub code: u32,
    pub modifiers: i64,
    pub x: Value,
    pub y: Value,
    pub timestamp: u64,
    pub frame_or_window: Value,
    pub arg: Value,
    pub device: Value,
}

impl Default for KbdEvent {
    fn default() -> Self {
        KbdEvent {
            kind: EventKind::NoEvent,
            part: ScrollBarPart::Nowhere,
            code: 0,
            modifiers: 0,
            x: Value::NIL,
            y: Value::NIL,
            timestamp: 0,
            frame_or_window: Value::NIL,
            arg: Value::NIL,
            device: sym::T,
        }
    }
}

impl KbdEvent {
    pub fn trace(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[self.x, self.y, self.frame_or_window, self.arg, self.device]);
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct InputEvent {
    pub kind: EventKind,
    pub part: ScrollBarPart,
    pub code: u32,
    pub modifiers: i64,
    pub x: i64,
    pub y: i64,
    pub timestamp: u64,
    pub frame: u64,
    pub arg: Option<String>,
}
