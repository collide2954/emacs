use emacs_core::termhooks::InputEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBarImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolItem {
    Separator,
    Button {
        index: usize,
        image: Option<ToolBarImage>,
        label: Option<String>,
        enabled: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolBarPosition {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBar {
    pub items: Vec<ToolItem>,
    pub horiz: bool,
    pub text_image: bool,
    pub hmargin: u32,
    pub vmargin: u32,
    pub position: Option<ToolBarPosition>,
}

#[derive(Debug, Clone)]
pub enum Command {
    OpenFrame {
        frame: u64,
        title: String,
        width: u32,
        height: u32,
    },
    OpenPopup {
        frame: u64,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    SetTitle {
        frame: u64,
        title: String,
    },
    MoveFrame {
        frame: u64,
        x: i32,
        y: i32,
    },
    SetVisible {
        frame: u64,
        on: bool,
    },
    SetTooltip {
        frame: u64,
        text: Option<String>,
    },
    ResizeFrame {
        frame: u64,
        width: u32,
        height: u32,
    },
    Present {
        frame: u64,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
    SetFullscreen {
        frame: u64,
        on: bool,
    },
    SetMaximized {
        frame: u64,
        on: bool,
    },
    CloseFrame {
        frame: u64,
    },
    SetCursor {
        frame: u64,
        cursor: iced::mouse::Interaction,
    },
    SetToolBar {
        frame: u64,
        bar: ToolBar,
    },
    RemoveToolBar {
        frame: u64,
    },
    Exit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Notice {
    Input(InputEvent),
    Configure {
        frame: u64,
        width: u32,
        height: u32,
        scale: f32,
    },
    Map {
        frame: u64,
    },
    Realize {
        frame: u64,
        window: u64,
    },
    Leave {
        frame: u64,
    },
    Motion {
        frame: u64,
        x: i64,
        y: i64,
        timestamp: u64,
    },
    ToolBarHelp {
        frame: u64,
        index: Option<usize>,
    },
    ToolBarSize {
        frame: u64,
        width: u32,
        height: u32,
    },
}
