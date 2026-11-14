use emacs_core::termhooks::InputEvent;

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
}
