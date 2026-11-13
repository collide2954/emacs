use emacs_core::keyboard::{DOWN_MODIFIER, UP_MODIFIER};
use emacs_core::termhooks::{EventKind, InputEvent};
use iced::Point;
use iced::keyboard::Modifiers;
use iced::mouse::{Button, ScrollDelta};

use crate::keysym::emacs_modifiers;

pub fn button_number(button: Button) -> u32 {
    match button {
        Button::Left => 1,
        Button::Middle => 2,
        Button::Right => 3,
        Button::Back => 8,
        Button::Forward => 9,
        Button::Other(n) => u32::from(n),
    }
}

pub fn button_event(
    button: Button,
    pressed: bool,
    position: Point,
    modifiers: Modifiers,
    timestamp: u64,
    frame: u64,
) -> InputEvent {
    InputEvent {
        kind: EventKind::MouseClick,
        code: button_number(button).saturating_sub(1),
        timestamp,
        modifiers: emacs_modifiers(modifiers) | if pressed { DOWN_MODIFIER } else { UP_MODIFIER },
        x: position.x as i64,
        y: position.y as i64,
        frame,
        ..InputEvent::default()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WheelAccumulator {
    x: f32,
    y: f32,
}

impl WheelAccumulator {
    pub fn events(
        &mut self,
        delta: ScrollDelta,
        position: Point,
        modifiers: Modifiers,
        timestamp: u64,
        frame: u64,
    ) -> Vec<InputEvent> {
        let ScrollDelta::Lines { x, y } = delta else {
            return Vec::new();
        };
        self.x += x;
        self.y += y;
        let event = |kind, direction| InputEvent {
            kind,
            timestamp,
            modifiers: emacs_modifiers(modifiers) | direction,
            x: position.x as i64,
            y: position.y as i64,
            frame,
            ..InputEvent::default()
        };
        let mut events = Vec::new();
        while self.y >= 1.0 {
            self.y -= 1.0;
            events.push(event(EventKind::Wheel, UP_MODIFIER));
        }
        while self.y <= -1.0 {
            self.y += 1.0;
            events.push(event(EventKind::Wheel, DOWN_MODIFIER));
        }
        while self.x >= 1.0 {
            self.x -= 1.0;
            events.push(event(EventKind::HorizWheel, UP_MODIFIER));
        }
        while self.x <= -1.0 {
            self.x += 1.0;
            events.push(event(EventKind::HorizWheel, DOWN_MODIFIER));
        }
        events
    }
}
