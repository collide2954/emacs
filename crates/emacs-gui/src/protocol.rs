use emacs_core::termhooks::InputEvent;

#[derive(Debug)]
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
    Present {
        frame: u64,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
    CloseFrame {
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
}
