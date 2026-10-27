use std::sync::mpsc;

use emacs_core::termhooks::EventKind;
use emacs_gui::protocol::{Command, Notice};
use emacs_gui::shell::{Message, Shell};
use iced::keyboard::key::{Code, Physical};
use iced::keyboard::{self, Key, Location, Modifiers};
use iced::{Size, window};

fn input_kind(notice: Notice) -> (EventKind, u32, u64) {
    match notice {
        Notice::Input(e) => (e.kind, e.code, e.frame),
        other => panic!("{other:?}"),
    }
}

#[test]
fn shell_tracks_frames_and_forwards_events() {
    let mut shell = Shell::boot();
    let (tx, rx) = mpsc::channel();
    let _ = shell.update(Message::Connected(tx));
    let _ = shell.update(Message::Lisp(Command::OpenFrame {
        frame: 5,
        title: "t".into(),
        width: 100,
        height: 50,
    }));
    let id = shell.window_of(5).expect("window for frame");
    assert_eq!(shell.title(id), "t");
    let _ = shell.update(Message::Lisp(Command::SetTitle {
        frame: 5,
        title: "u".into(),
    }));
    assert_eq!(shell.title(id), "u");
    assert!(!shell.has_image(id));
    let _ = shell.update(Message::Lisp(Command::Present {
        frame: 5,
        width: 100,
        height: 50,
        pixels: vec![0; 100 * 50 * 4],
    }));
    assert!(shell.has_image(id));

    let _ = shell.update(Message::Window(id, window::Event::Rescaled(2.0)));
    let _ = shell.update(Message::Window(
        id,
        window::Event::Resized(Size::new(60.0, 40.0)),
    ));
    let configures: Vec<Notice> = rx.try_iter().collect();
    assert_eq!(
        configures.last(),
        Some(&Notice::Configure {
            frame: 5,
            width: 120,
            height: 80,
            scale: 2.0,
        })
    );

    let _ = shell.update(Message::Key(
        id,
        keyboard::Event::KeyPressed {
            key: Key::Character("q".into()),
            modified_key: Key::Character("q".into()),
            physical_key: Physical::Code(Code::KeyQ),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: Some("q".into()),
            repeat: false,
        },
    ));
    assert_eq!(
        input_kind(rx.try_recv().expect("key")),
        (EventKind::AsciiKeystroke, 113, 5)
    );

    let _ = shell.update(Message::Window(id, window::Event::Focused));
    assert_eq!(
        input_kind(rx.try_recv().expect("focus")).0,
        EventKind::FocusIn
    );
    let _ = shell.update(Message::Window(id, window::Event::Unfocused));
    assert_eq!(
        input_kind(rx.try_recv().expect("unfocus")).0,
        EventKind::FocusOut
    );
    let _ = shell.update(Message::Window(id, window::Event::CloseRequested));
    assert_eq!(
        input_kind(rx.try_recv().expect("close")),
        (EventKind::DeleteWindow, 0, 5)
    );

    let _ = shell.update(Message::Lisp(Command::CloseFrame { frame: 5 }));
    assert_eq!(shell.window_of(5), None);
}
