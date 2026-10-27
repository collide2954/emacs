use std::sync::mpsc;
use std::time::Duration;

use emacs_core::termhooks::{EventKind, InputEvent};
use emacs_gui::display::Display;
use emacs_gui::protocol::{Command, Notice};
use emacs_gui::surface::Rect;
use iced::futures::channel::mpsc as fmpsc;

fn setup() -> (
    Display,
    fmpsc::UnboundedReceiver<Command>,
    mpsc::Sender<Notice>,
) {
    let (commands, command_rx) = fmpsc::unbounded();
    let (notice_tx, notices) = mpsc::channel();
    (Display::new(commands, notices), command_rx, notice_tx)
}

#[test]
fn open_and_present_frame() {
    let (mut display, mut commands, _notices) = setup();
    display.open_frame(1, "emacs", 8, 4, 0xFFFFFF);
    match commands.try_recv() {
        Ok(Command::OpenFrame {
            frame: 1,
            title,
            width: 8,
            height: 4,
        }) => assert_eq!(title, "emacs"),
        other => panic!("{other:?}"),
    }
    display
        .surface_mut(1)
        .expect("surface")
        .fill_rectangle(0xFF0000, Rect::new(0, 0, 1, 1), false);
    display.flush(1);
    match commands.try_recv() {
        Ok(Command::Present {
            frame: 1,
            width: 8,
            height: 4,
            pixels,
        }) => {
            assert_eq!(pixels.len(), 8 * 4 * 4);
            assert_eq!(&pixels[0..8], &[255, 0, 0, 255, 255, 255, 255, 255]);
        }
        other => panic!("{other:?}"),
    }
    display.set_title(1, "other");
    assert!(matches!(
        commands.try_recv(),
        Ok(Command::SetTitle { frame: 1, .. })
    ));
}

#[test]
fn notices_resize_surfaces_and_pass_input() {
    let (mut display, _commands, notices) = setup();
    display.open_frame(2, "f", 4, 4, 0);
    notices
        .send(Notice::Configure {
            frame: 2,
            width: 10,
            height: 6,
            scale: 1.5,
        })
        .expect("send");
    let input = InputEvent {
        kind: EventKind::AsciiKeystroke,
        code: 97,
        frame: 2,
        ..InputEvent::default()
    };
    notices.send(Notice::Input(input.clone())).expect("send");
    let read = display
        .read_notices(Some(Duration::from_millis(200)))
        .expect("connected");
    assert_eq!(read.len(), 2);
    assert_eq!(read[1], Notice::Input(input));
    let surface = display.surface(2).expect("surface");
    assert_eq!((surface.width(), surface.height()), (10, 6));
    assert_eq!(display.read_notices(None).expect("connected"), Vec::new());
}

#[test]
fn closing_and_disconnection() {
    let (mut display, mut commands, notices) = setup();
    display.open_frame(3, "f", 2, 2, 0);
    let _ = commands.try_recv();
    display.close_frame(3);
    assert!(display.surface(3).is_none());
    assert!(matches!(
        commands.try_recv(),
        Ok(Command::CloseFrame { frame: 3 })
    ));
    display.exit();
    assert!(matches!(commands.try_recv(), Ok(Command::Exit)));
    drop(notices);
    assert!(
        display
            .read_notices(Some(Duration::from_millis(10)))
            .is_err()
    );
}
