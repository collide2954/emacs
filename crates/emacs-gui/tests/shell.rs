use std::sync::mpsc;

use emacs_core::keyboard::CTRL_MODIFIER;
use emacs_core::termhooks::EventKind;
use emacs_gui::protocol::{Command, Notice, ToolBar, ToolBarImage, ToolBarPosition, ToolItem};
use emacs_gui::shell::{Message, Shell};
use iced::keyboard::key::{Code, Physical};
use iced::keyboard::{self, Key, Location, Modifiers};
use iced::{Point, Size, mouse, window};

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
    assert!(!shell.has_image(id));

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
            width: 60,
            height: 40,
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

#[test]
fn cursor_motion_becomes_a_motion_notice() {
    let mut shell = Shell::boot();
    let (tx, rx) = mpsc::channel();
    let _ = shell.update(Message::Connected(tx));
    let _ = shell.update(Message::Lisp(Command::OpenFrame {
        frame: 4,
        title: "m".into(),
        width: 100,
        height: 50,
    }));
    let id = shell.window_of(4).expect("window for frame");
    let _ = shell.update(Message::Mouse(
        id,
        mouse::Event::CursorMoved {
            position: Point::new(12.7, 30.2),
        },
    ));
    let notices: Vec<Notice> = rx.try_iter().collect();
    assert!(
        matches!(
            notices.as_slice(),
            [Notice::Motion {
                frame: 4,
                x: 12,
                y: 30,
                ..
            }]
        ),
        "{notices:?}"
    );
    let _ = shell.update(Message::Mouse(
        id,
        mouse::Event::ButtonPressed(mouse::Button::Left),
    ));
    match rx.try_recv().expect("button") {
        Notice::Input(e) => assert_eq!(
            (e.kind, e.code, e.x, e.y, e.frame),
            (EventKind::MouseClick, 0, 12, 30, 4)
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn leaving_a_window_and_pointer_shapes() {
    use iced::mouse::{self, Interaction};
    let mut shell = Shell::boot();
    let (tx, rx) = mpsc::channel();
    let _ = shell.update(Message::Connected(tx));
    let _ = shell.update(Message::Lisp(Command::OpenFrame {
        frame: 8,
        title: "l".into(),
        width: 10,
        height: 10,
    }));
    let id = shell.window_of(8).expect("window for frame");
    assert_eq!(shell.interaction(id), Interaction::Idle);
    let _ = shell.update(Message::Mouse(id, mouse::Event::CursorLeft));
    assert_eq!(
        rx.try_iter().collect::<Vec<_>>(),
        [Notice::Leave { frame: 8 }]
    );
    let _ = shell.update(Message::Lisp(Command::SetCursor {
        frame: 8,
        cursor: Interaction::Pointer,
    }));
    assert_eq!(shell.interaction(id), Interaction::Pointer);
    let _ = shell.update(Message::Lisp(Command::SetCursor {
        frame: 99,
        cursor: Interaction::Text,
    }));
    assert_eq!(shell.interaction(id), Interaction::Pointer);
}

#[test]
fn a_realized_window_reports_its_raw_id() {
    let mut shell = Shell::boot();
    let (tx, rx) = mpsc::channel();
    let _ = shell.update(Message::Connected(tx));
    let _ = shell.update(Message::Lisp(Command::OpenFrame {
        frame: 7,
        title: "r".into(),
        width: 10,
        height: 10,
    }));
    let id = shell.window_of(7).expect("window for frame");
    let _ = shell.update(Message::Realized(id, 0x0004_0A2C));
    assert_eq!(
        rx.try_iter().collect::<Vec<_>>(),
        [Notice::Realize {
            frame: 7,
            window: 0x0004_0A2C,
        }]
    );
}

#[test]
fn a_presented_image_is_shown_once_allocated() {
    use iced::widget::image::Handle;
    let mut shell = Shell::boot();
    let _ = shell.update(Message::Lisp(Command::OpenFrame {
        frame: 6,
        title: "p".into(),
        width: 8,
        height: 4,
    }));
    let id = shell.window_of(6).expect("window for frame");
    let pixels = |width: u32, height: u32| vec![0; (width * height * 4) as usize];
    let present = |width, height| {
        Message::Lisp(Command::Present {
            frame: 6,
            width,
            height,
            pixels: pixels(width, height),
        })
    };
    let allocated = |seq, width, height| Message::Allocated {
        id,
        seq,
        handle: Handle::from_rgba(width, height, pixels(width, height)),
        width,
        height,
        allocation: None,
    };
    let _ = shell.update(present(4, 2));
    assert_eq!(shell.image_size(id), None);
    let _ = shell.update(allocated(1, 4, 2));
    assert_eq!(shell.image_size(id), Some((4, 2)));
    let _ = shell.update(present(8, 2));
    let _ = shell.update(present(8, 4));
    assert_eq!(shell.image_size(id), Some((4, 2)));
    let _ = shell.update(allocated(3, 8, 4));
    assert_eq!(shell.image_size(id), Some((8, 4)));
    let _ = shell.update(allocated(2, 8, 2));
    assert_eq!(shell.image_size(id), Some((8, 4)));
}

#[test]
fn popup_windows_take_no_events_and_follow_commands() {
    use iced::{Point, mouse};
    let mut shell = Shell::boot();
    let (tx, rx) = mpsc::channel();
    let _ = shell.update(Message::Connected(tx));
    let _ = shell.update(Message::Lisp(Command::OpenPopup {
        frame: 12,
        x: 40,
        y: 30,
        width: 20,
        height: 10,
    }));
    let id = shell.window_of(12).expect("popup window");
    assert!(shell.is_popup(id));
    assert!(shell.visible(id));
    assert_eq!(shell.position(id), Some((40, 30)));
    let _ = shell.update(Message::Window(
        id,
        window::Event::Opened {
            position: None,
            size: Size::new(20.0, 10.0),
        },
    ));
    let _ = shell.update(Message::Realized(id, 0));
    let _ = shell.update(Message::Window(id, window::Event::Focused));
    let _ = shell.update(Message::Window(
        id,
        window::Event::Resized(Size::new(30.0, 12.0)),
    ));
    let _ = shell.update(Message::Mouse(
        id,
        mouse::Event::CursorMoved {
            position: Point::new(1.0, 1.0),
        },
    ));
    let _ = shell.update(Message::Mouse(
        id,
        mouse::Event::ButtonPressed(mouse::Button::Left),
    ));
    let _ = shell.update(Message::Mouse(id, mouse::Event::CursorLeft));
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
    let _ = shell.update(Message::Window(id, window::Event::CloseRequested));
    assert_eq!(rx.try_iter().collect::<Vec<_>>(), []);
    let _ = shell.update(Message::Lisp(Command::MoveFrame {
        frame: 12,
        x: 7,
        y: 9,
    }));
    assert_eq!(shell.position(id), Some((7, 9)));
    let _ = shell.update(Message::Lisp(Command::SetVisible {
        frame: 12,
        on: false,
    }));
    assert!(!shell.visible(id));
    let _ = shell.update(Message::Lisp(Command::SetVisible {
        frame: 12,
        on: true,
    }));
    assert!(shell.visible(id));
    assert_eq!(rx.try_iter().collect::<Vec<_>>(), []);
    let _ = shell.view(id);
    let _ = shell.update(Message::Lisp(Command::CloseFrame { frame: 12 }));
    assert_eq!(shell.window_of(12), None);
}

#[test]
fn frames_hide_show_and_carry_tooltips() {
    let mut shell = Shell::boot();
    let (tx, rx) = mpsc::channel();
    let _ = shell.update(Message::Connected(tx));
    let _ = shell.update(Message::Lisp(Command::OpenFrame {
        frame: 2,
        title: "v".into(),
        width: 10,
        height: 10,
    }));
    let id = shell.window_of(2).expect("window for frame");
    assert!(!shell.is_popup(id));
    assert!(shell.visible(id));
    assert_eq!(shell.tooltip(id), None);
    let _ = shell.update(Message::Lisp(Command::SetVisible {
        frame: 2,
        on: false,
    }));
    assert!(!shell.visible(id));
    assert_eq!(rx.try_iter().collect::<Vec<_>>(), []);
    let _ = shell.update(Message::Lisp(Command::SetVisible { frame: 2, on: true }));
    assert!(shell.visible(id));
    assert_eq!(
        rx.try_iter().collect::<Vec<_>>(),
        [Notice::Map { frame: 2 }]
    );
    let _ = shell.update(Message::Lisp(Command::SetTooltip {
        frame: 2,
        text: Some("tip text".into()),
    }));
    assert_eq!(shell.tooltip(id).as_deref(), Some("tip text"));
    let _ = shell.view(id);
    let _ = shell.update(Message::Lisp(Command::SetTooltip {
        frame: 2,
        text: None,
    }));
    assert_eq!(shell.tooltip(id), None);
    let _ = shell.update(Message::Lisp(Command::SetTooltip {
        frame: 77,
        text: Some("x".into()),
    }));
    let _ = shell.update(Message::Lisp(Command::SetVisible {
        frame: 77,
        on: true,
    }));
    let _ = shell.update(Message::Lisp(Command::MoveFrame {
        frame: 77,
        x: 1,
        y: 1,
    }));
    assert_eq!(rx.try_iter().collect::<Vec<_>>(), []);
}

#[test]
fn an_opened_window_is_configured_and_then_mapped() {
    let mut shell = Shell::boot();
    let (tx, rx) = mpsc::channel();
    let _ = shell.update(Message::Connected(tx));
    let _ = shell.update(Message::Lisp(Command::OpenFrame {
        frame: 3,
        title: "t".into(),
        width: 100,
        height: 50,
    }));
    let id = shell.window_of(3).expect("window for frame");
    let _ = shell.update(Message::Window(
        id,
        window::Event::Opened {
            position: None,
            size: Size::new(100.0, 50.0),
        },
    ));
    let notices: Vec<Notice> = rx.try_iter().collect();
    assert_eq!(
        notices,
        [
            Notice::Configure {
                frame: 3,
                width: 100,
                height: 50,
                scale: 1.0,
            },
            Notice::Map { frame: 3 },
        ]
    );
    let _ = shell.update(Message::Lisp(Command::SetMaximized { frame: 3, on: true }));
    let _ = shell.update(Message::Lisp(Command::SetFullscreen {
        frame: 3,
        on: false,
    }));
    let _ = shell.update(Message::Lisp(Command::SetMaximized { frame: 9, on: true }));
}

fn shell_tool_bar(position: ToolBarPosition) -> ToolBar {
    ToolBar {
        items: vec![
            ToolItem::Button {
                index: 0,
                image: Some(ToolBarImage {
                    width: 1,
                    height: 1,
                    rgba: vec![255, 0, 0, 255],
                }),
                label: None,
                enabled: true,
            },
            ToolItem::Separator,
            ToolItem::Button {
                index: 2,
                image: None,
                label: Some("Off".into()),
                enabled: false,
            },
        ],
        horiz: false,
        text_image: false,
        hmargin: 0,
        vmargin: 0,
        position: Some(position),
    }
}

fn shell_with_tool_bar(position: ToolBarPosition) -> (Shell, mpsc::Receiver<Notice>, window::Id) {
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
    let _ = shell.update(Message::Lisp(Command::SetToolBar {
        frame: 5,
        bar: shell_tool_bar(position),
    }));
    let _ = shell.update(Message::Window(
        id,
        window::Event::Resized(Size::new(100.0, 80.0)),
    ));
    let _ = rx.try_iter().count();
    (shell, rx, id)
}

#[test]
fn tool_bar_buttons_send_tool_bar_and_help_notices() {
    let (mut shell, rx, id) = shell_with_tool_bar(ToolBarPosition::Top);
    assert_eq!(
        shell.tool_bar(id),
        Some(&shell_tool_bar(ToolBarPosition::Top))
    );
    let _ = shell.update(Message::Key(
        id,
        keyboard::Event::ModifiersChanged(Modifiers::CTRL),
    ));
    let _ = shell.update(Message::ToolBarClicked(id, 2));
    match rx.try_recv() {
        Ok(Notice::Input(e)) => assert_eq!(
            (e.kind, e.code, e.frame, e.modifiers),
            (EventKind::ToolBar, 2, 5, CTRL_MODIFIER)
        ),
        other => panic!("{other:?}"),
    }
    let _ = shell.update(Message::ToolBarHover(id, Some(2)));
    assert_eq!(
        rx.try_recv(),
        Ok(Notice::ToolBarHelp {
            frame: 5,
            index: Some(2)
        })
    );
    let _ = shell.update(Message::ToolBarHover(id, None));
    assert_eq!(
        rx.try_recv(),
        Ok(Notice::ToolBarHelp {
            frame: 5,
            index: None
        })
    );
    let _ = shell.update(Message::Lisp(Command::RemoveToolBar { frame: 5 }));
    assert_eq!(shell.tool_bar(id), None);
}

#[test]
fn the_frame_area_excludes_the_tool_bar() {
    let (mut shell, rx, id) = shell_with_tool_bar(ToolBarPosition::Top);
    let _ = shell.update(Message::ToolBarAllocated(id, Size::new(100.0, 30.0)));
    let notices: Vec<Notice> = rx.try_iter().collect();
    assert_eq!(
        notices,
        [
            Notice::ToolBarSize {
                frame: 5,
                width: 100,
                height: 30
            },
            Notice::Configure {
                frame: 5,
                width: 100,
                height: 50,
                scale: 1.0
            }
        ]
    );
    let _ = shell.update(Message::Mouse(
        id,
        mouse::Event::CursorMoved {
            position: Point::new(10.0, 40.0),
        },
    ));
    assert!(matches!(
        rx.try_recv(),
        Ok(Notice::Motion {
            frame: 5,
            x: 10,
            y: 10,
            ..
        })
    ));
    let _ = shell.update(Message::Mouse(
        id,
        mouse::Event::CursorMoved {
            position: Point::new(10.0, 20.0),
        },
    ));
    assert_eq!(rx.try_recv(), Ok(Notice::Leave { frame: 5 }));
    let _ = shell.update(Message::Mouse(
        id,
        mouse::Event::ButtonPressed(mouse::Button::Left),
    ));
    assert!(rx.try_recv().is_err());

    let (mut shell, rx, id) = shell_with_tool_bar(ToolBarPosition::Left);
    let _ = shell.update(Message::ToolBarAllocated(id, Size::new(20.0, 80.0)));
    let notices: Vec<Notice> = rx.try_iter().collect();
    assert_eq!(
        notices.last(),
        Some(&Notice::Configure {
            frame: 5,
            width: 80,
            height: 80,
            scale: 1.0
        })
    );
    let _ = shell.update(Message::Mouse(
        id,
        mouse::Event::CursorMoved {
            position: Point::new(30.0, 5.0),
        },
    ));
    assert!(matches!(
        rx.try_recv(),
        Ok(Notice::Motion {
            frame: 5,
            x: 10,
            y: 5,
            ..
        })
    ));
}
