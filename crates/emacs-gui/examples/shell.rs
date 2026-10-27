use std::time::{Duration, Instant};

use emacs_core::termhooks::EventKind;
use emacs_gui::display::Display;
use emacs_gui::protocol::Notice;
use emacs_gui::surface::Rect;

const FRAME: u64 = 1;

fn paint(display: &mut Display) {
    let Some(surface) = display.surface_mut(FRAME) else {
        return;
    };
    let (width, height) = (surface.width() as i32, surface.height() as i32);
    surface.clear_area(Rect::new(0, 0, width, height));
    surface.fill_rectangle(0x3366CC, Rect::new(0, height - 20, width, 20), false);
    for i in 0..8 {
        surface.fill_rectangle(
            0xCC3333 + i as u32 * 0x1F00,
            Rect::new(10 + i * 20, 10, 12, 16),
            false,
        );
    }
    let coverage: Vec<u8> = (0..64 * 16).map(|i| ((i % 64) * 4) as u8).collect();
    surface.draw_mask(Rect::new(10, 40, 64, 16), &coverage, 0x000000);
    display.flush(FRAME);
}

fn lisp_main(mut display: Display) {
    let limit = std::env::args()
        .skip_while(|a| a != "--exit-after-ms")
        .nth(1)
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_millis);
    let start = Instant::now();
    display.open_frame(FRAME, "emacs-gui shell", 640, 400, 0xFFFFFF);
    paint(&mut display);
    loop {
        let Ok(notices) = display.read_notices(Some(Duration::from_millis(50))) else {
            return;
        };
        for notice in notices {
            println!("{notice:?}");
            match notice {
                Notice::Configure { .. } => paint(&mut display),
                Notice::Input(event) if event.kind == EventKind::DeleteWindow => {
                    display.exit();
                    return;
                }
                _ => {}
            }
        }
        if limit.is_some_and(|l| start.elapsed() >= l) {
            display.exit();
            return;
        }
    }
}

fn main() -> iced::Result {
    emacs_gui::run(lisp_main)
}
