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
fn notices_pass_through_and_surfaces_resize_on_request() {
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
    assert_eq!((surface.width(), surface.height()), (4, 4));
    assert_eq!(display.read_notices(None).expect("connected"), Vec::new());
    assert!(display.resize_surface(2, 10, 6));
    let surface = display.surface(2).expect("surface");
    assert_eq!((surface.width(), surface.height()), (10, 6));
    assert!(!display.resize_surface(2, 10, 6));
    assert!(!display.resize_surface(9, 10, 6));
}

struct Pointer;

impl emacs_gui::monitor::MonitorSource for Pointer {
    fn monitors(&self) -> Vec<emacs_gui::monitor::GdkMonitor> {
        Vec::new()
    }

    fn monitor_at_window(&self, _window: u64) -> isize {
        0
    }

    fn pointer_position(&self) -> Option<(i32, i32)> {
        Some((310, 145))
    }
}

#[test]
fn popups_open_where_they_were_moved() {
    let (mut display, mut commands, _notices) = setup();
    display.create_popup(4, 20, 10, 0xFFFFE0);
    display.move_frame(4, 100, 200);
    display.resize_frame(4, 30, 12);
    assert!(commands.try_recv().is_err());
    let surface = display.surface(4).expect("surface");
    assert_eq!((surface.width(), surface.height()), (30, 12));
    display.hide_frame(4);
    assert!(commands.try_recv().is_err());
    display.show_frame(4, "tooltip");
    assert!(matches!(
        commands.try_recv(),
        Ok(Command::OpenPopup {
            frame: 4,
            x: 100,
            y: 200,
            width: 30,
            height: 12,
        })
    ));
    display.move_frame(4, 5, 6);
    assert!(matches!(
        commands.try_recv(),
        Ok(Command::MoveFrame {
            frame: 4,
            x: 5,
            y: 6
        })
    ));
    display.hide_frame(4);
    assert!(matches!(
        commands.try_recv(),
        Ok(Command::SetVisible {
            frame: 4,
            on: false
        })
    ));
    display.show_frame(4, "tooltip");
    assert!(matches!(
        commands.try_recv(),
        Ok(Command::SetVisible { frame: 4, on: true })
    ));
    display.open_frame(1, "f", 8, 8, 0);
    assert!(matches!(
        commands.try_recv(),
        Ok(Command::OpenFrame { frame: 1, .. })
    ));
    display.set_tooltip(1, Some("hi"));
    match commands.try_recv() {
        Ok(Command::SetTooltip { frame: 1, text }) => assert_eq!(text.as_deref(), Some("hi")),
        other => panic!("{other:?}"),
    }
    display.set_tooltip(1, None);
    assert!(matches!(
        commands.try_recv(),
        Ok(Command::SetTooltip {
            frame: 1,
            text: None
        })
    ));
    display.set_monitor_source(Box::new(Pointer));
    assert_eq!(display.pointer_position(), Some((310, 145)));
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
