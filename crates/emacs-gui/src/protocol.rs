use emacs_core::termhooks::InputEvent;

#[derive(Debug, Clone)]
pub enum Command {
    OpenFrame {
        frame: u64,
        title: String,
        width: u32,
        height: u32,
    },
    SetTitle {
        frame: u64,
        title: String,
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
