use emacs_core::keyboard::{CTRL_MODIFIER, DOWN_MODIFIER, SHIFT_MODIFIER, UP_MODIFIER};
use emacs_core::termhooks::{EventKind, InputEvent};
use emacs_gui::mouse::{WheelAccumulator, button_event};
use iced::Point;
use iced::keyboard::Modifiers;
use iced::mouse::{Button, ScrollDelta};

fn kinds(events: &[InputEvent]) -> Vec<(EventKind, i64)> {
    events.iter().map(|e| (e.kind, e.modifiers)).collect()
}

#[test]
fn buttons_become_mouse_clicks() {
    assert_eq!(
        button_event(
            Button::Left,
            true,
            Point::new(12.7, 30.2),
            Modifiers::CTRL,
            99,
            4
        ),
        InputEvent {
            kind: EventKind::MouseClick,
            code: 0,
            modifiers: CTRL_MODIFIER | DOWN_MODIFIER,
            x: 12,
            y: 30,
            timestamp: 99,
            frame: 4,
            ..InputEvent::default()
        }
    );
    for (button, code) in [
        (Button::Middle, 1),
        (Button::Right, 2),
        (Button::Back, 7),
        (Button::Forward, 8),
        (Button::Other(10), 9),
    ] {
        let event = button_event(
            button,
            false,
            Point::new(1.0, 2.0),
            Modifiers::empty(),
            5,
            1,
        );
        assert_eq!(
            (event.code, event.modifiers),
            (code, UP_MODIFIER),
            "{button:?}"
        );
    }
}

#[test]
fn wheel_lines_become_wheel_events() {
    let mut wheel = WheelAccumulator::default();
    let at = Point::new(5.0, 6.0);
    let none = Modifiers::empty();
    let lines = |x, y| ScrollDelta::Lines { x, y };
    let events = wheel.events(lines(0.0, 1.0), at, none, 10, 3);
    assert_eq!(
        events,
        [InputEvent {
            kind: EventKind::Wheel,
            modifiers: UP_MODIFIER,
            x: 5,
            y: 6,
            timestamp: 10,
            frame: 3,
            ..InputEvent::default()
        }]
    );
    assert_eq!(
        kinds(&wheel.events(lines(0.0, -2.0), at, none, 11, 3)),
        [
            (EventKind::Wheel, DOWN_MODIFIER),
            (EventKind::Wheel, DOWN_MODIFIER)
        ]
    );
    assert!(wheel.events(lines(0.0, 0.5), at, none, 12, 3).is_empty());
    assert_eq!(
        kinds(&wheel.events(lines(0.0, 0.5), at, none, 13, 3)),
        [(EventKind::Wheel, UP_MODIFIER)]
    );
    assert_eq!(
        kinds(&wheel.events(lines(1.0, 0.0), at, none, 14, 3)),
        [(EventKind::HorizWheel, UP_MODIFIER)]
    );
    assert_eq!(
        kinds(&wheel.events(lines(-1.0, 0.0), at, none, 15, 3)),
        [(EventKind::HorizWheel, DOWN_MODIFIER)]
    );
    assert_eq!(
        kinds(&wheel.events(lines(0.0, 1.0), at, Modifiers::SHIFT, 16, 3)),
        [(EventKind::Wheel, SHIFT_MODIFIER | UP_MODIFIER)]
    );
}
