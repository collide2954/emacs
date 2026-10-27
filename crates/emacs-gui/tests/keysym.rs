use emacs_core::keyboard::{CTRL_MODIFIER, META_MODIFIER, SHIFT_MODIFIER, SUPER_MODIFIER};
use emacs_core::termhooks::{EventKind, InputEvent};
use emacs_gui::keysym::{KeyPress, key_press_event, keysym_name};
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Location, Modifiers};

fn press(
    key: Key,
    modified: Key,
    location: Location,
    modifiers: Modifiers,
    text: Option<&str>,
) -> Option<InputEvent> {
    key_press_event(
        &KeyPress {
            key: &key,
            modified_key: &modified,
            location,
            modifiers,
            text,
        },
        7,
        3,
    )
}

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

fn named(n: Named) -> Key {
    Key::Named(n)
}

fn kind_code_mods(e: Option<InputEvent>) -> (EventKind, u32, i64) {
    let e = e.expect("event");
    assert_eq!(e.timestamp, 7);
    assert_eq!(e.frame, 3);
    (e.kind, e.code, e.modifiers)
}

#[test]
fn ascii_keys() {
    let std = Location::Standard;
    assert_eq!(
        kind_code_mods(press(ch("a"), ch("a"), std, Modifiers::empty(), Some("a"))),
        (EventKind::AsciiKeystroke, 97, 0)
    );
    assert_eq!(
        kind_code_mods(press(ch("a"), ch("a"), std, Modifiers::CTRL, None)),
        (EventKind::AsciiKeystroke, 97, CTRL_MODIFIER)
    );
    assert_eq!(
        kind_code_mods(press(ch("a"), ch("A"), std, Modifiers::SHIFT, Some("A"))),
        (EventKind::AsciiKeystroke, 65, SHIFT_MODIFIER)
    );
    assert_eq!(
        kind_code_mods(press(ch("x"), ch("x"), std, Modifiers::ALT, None)),
        (EventKind::AsciiKeystroke, 120, META_MODIFIER)
    );
    assert_eq!(
        kind_code_mods(press(ch("x"), ch("x"), std, Modifiers::LOGO, None)),
        (EventKind::AsciiKeystroke, 120, SUPER_MODIFIER)
    );
    assert_eq!(
        kind_code_mods(press(
            named(Named::Space),
            named(Named::Space),
            std,
            Modifiers::empty(),
            Some(" ")
        )),
        (EventKind::AsciiKeystroke, 32, 0)
    );
}

#[test]
fn multibyte_keys() {
    let std = Location::Standard;
    assert_eq!(
        kind_code_mods(press(ch("é"), ch("é"), std, Modifiers::empty(), Some("é"))),
        (EventKind::MultibyteCharKeystroke, 0xE9, 0)
    );
    let e = press(
        Key::Unidentified,
        Key::Unidentified,
        std,
        Modifiers::empty(),
        Some("ab"),
    )
    .expect("event");
    assert_eq!(e.kind, EventKind::MultibyteCharKeystroke);
    assert_eq!(e.arg.as_deref(), Some("ab"));
}

#[test]
fn function_keys() {
    let std = Location::Standard;
    let cases = [
        (Named::ArrowLeft, 0xFF51),
        (Named::F5, 0xFFC2),
        (Named::F35, 0xFFE0),
        (Named::Tab, 0xFF09),
        (Named::Enter, 0xFF0D),
        (Named::Escape, 0xFF1B),
        (Named::Backspace, 0xFF08),
        (Named::Delete, 0xFFFF),
        (Named::Home, 0xFF50),
        (Named::PageDown, 0xFF56),
        (Named::ContextMenu, 0xFF67),
        (Named::AudioVolumeMute, 0x1008_FF12),
    ];
    for (key, keysym) in cases {
        assert_eq!(
            kind_code_mods(press(named(key), named(key), std, Modifiers::empty(), None)),
            (EventKind::NonAsciiKeystroke, keysym, 0),
            "{key:?}"
        );
    }
    assert_eq!(
        kind_code_mods(press(
            named(Named::Tab),
            named(Named::Tab),
            std,
            Modifiers::SHIFT,
            None
        )),
        (EventKind::NonAsciiKeystroke, 0xFE20, SHIFT_MODIFIER)
    );
    assert_eq!(keysym_name(0x1008_FF12), Some("XF86AudioMute"));
}

#[test]
fn keypad_keys() {
    let pad = Location::Numpad;
    assert_eq!(
        kind_code_mods(press(ch("1"), ch("1"), pad, Modifiers::empty(), Some("1"))),
        (EventKind::NonAsciiKeystroke, 0xFFB1, 0)
    );
    assert_eq!(
        kind_code_mods(press(ch("+"), ch("+"), pad, Modifiers::empty(), Some("+"))),
        (EventKind::NonAsciiKeystroke, 0xFFAB, 0)
    );
    assert_eq!(
        kind_code_mods(press(
            named(Named::Enter),
            named(Named::Enter),
            pad,
            Modifiers::empty(),
            None
        )),
        (EventKind::NonAsciiKeystroke, 0xFF8D, 0)
    );
    assert_eq!(
        kind_code_mods(press(
            named(Named::ArrowUp),
            named(Named::ArrowUp),
            pad,
            Modifiers::empty(),
            None
        )),
        (EventKind::NonAsciiKeystroke, 0xFF97, 0)
    );
}

#[test]
fn modifier_keys_produce_no_event() {
    for key in [
        Named::Shift,
        Named::Control,
        Named::Alt,
        Named::Super,
        Named::CapsLock,
        Named::NumLock,
        Named::AltGraph,
    ] {
        assert_eq!(
            press(
                named(key),
                named(key),
                Location::Left,
                Modifiers::empty(),
                None
            ),
            None,
            "{key:?}"
        );
    }
}
