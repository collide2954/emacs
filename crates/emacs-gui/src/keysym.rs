use emacs_core::keyboard::{CTRL_MODIFIER, META_MODIFIER, SHIFT_MODIFIER, SUPER_MODIFIER};
use emacs_core::termhooks::{EventKind, InputEvent};
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Location, Modifiers};

pub const VOID_SYMBOL: u32 = 0x00FF_FFFF;

const BACKSPACE: u32 = 0xFF08;
const ESCAPE: u32 = 0xFF1B;
const DELETE: u32 = 0xFFFF;
const ISO_LEFT_TAB: u32 = 0xFE20;
const ISO_ENTER: u32 = 0xFE34;
const ISO_LOCK: u32 = 0xFE01;
const ISO_LAST_GROUP_LOCK: u32 = 0xFE0F;
const SHIFT_L: u32 = 0xFFE1;
const HYPER_R: u32 = 0xFFEE;
const MODE_SWITCH: u32 = 0xFF7E;
const NUM_LOCK: u32 = 0xFF7F;
const VENDOR_BIT: u32 = 1 << 28;

const NAMED: &[(Named, u32)] = &[
    (Named::Backspace, BACKSPACE),
    (Named::Tab, 0xFF09),
    (Named::Clear, 0xFF0B),
    (Named::Enter, 0xFF0D),
    (Named::Pause, 0xFF13),
    (Named::ScrollLock, 0xFF14),
    (Named::Escape, ESCAPE),
    (Named::Delete, DELETE),
    (Named::Compose, 0xFF20),
    (Named::KanjiMode, 0xFF21),
    (Named::NonConvert, 0xFF22),
    (Named::Convert, 0xFF23),
    (Named::Romaji, 0xFF24),
    (Named::Hiragana, 0xFF25),
    (Named::Katakana, 0xFF26),
    (Named::HiraganaKatakana, 0xFF27),
    (Named::Zenkaku, 0xFF28),
    (Named::Hankaku, 0xFF29),
    (Named::ZenkakuHankaku, 0xFF2A),
    (Named::KanaMode, 0xFF2D),
    (Named::Eisu, 0xFF30),
    (Named::HangulMode, 0xFF31),
    (Named::HanjaMode, 0xFF34),
    (Named::CodeInput, 0xFF37),
    (Named::SingleCandidate, 0xFF3C),
    (Named::AllCandidates, 0xFF3D),
    (Named::PreviousCandidate, 0xFF3E),
    (Named::Home, 0xFF50),
    (Named::ArrowLeft, 0xFF51),
    (Named::ArrowUp, 0xFF52),
    (Named::ArrowRight, 0xFF53),
    (Named::ArrowDown, 0xFF54),
    (Named::PageUp, 0xFF55),
    (Named::PageDown, 0xFF56),
    (Named::End, 0xFF57),
    (Named::Select, 0xFF60),
    (Named::PrintScreen, 0xFF61),
    (Named::Print, 0xFF61),
    (Named::Execute, 0xFF62),
    (Named::Insert, 0xFF63),
    (Named::Undo, 0xFF65),
    (Named::Redo, 0xFF66),
    (Named::ContextMenu, 0xFF67),
    (Named::Find, 0xFF68),
    (Named::Cancel, 0xFF69),
    (Named::Help, 0xFF6A),
    (Named::ModeChange, MODE_SWITCH),
    (Named::NumLock, NUM_LOCK),
    (Named::Shift, SHIFT_L),
    (Named::Control, 0xFFE3),
    (Named::CapsLock, 0xFFE5),
    (Named::Meta, 0xFFE7),
    (Named::Alt, 0xFFE9),
    (Named::Super, 0xFFEB),
    (Named::Hyper, 0xFFED),
    (Named::AltGraph, 0xFE03),
    (Named::Space, 0x20),
];

const VENDOR: &[(Named, u32, &str)] = &[
    (Named::BrightnessUp, 0x1008_FF02, "XF86MonBrightnessUp"),
    (Named::BrightnessDown, 0x1008_FF03, "XF86MonBrightnessDown"),
    (Named::Standby, 0x1008_FF10, "XF86Standby"),
    (Named::AudioVolumeDown, 0x1008_FF11, "XF86AudioLowerVolume"),
    (Named::AudioVolumeMute, 0x1008_FF12, "XF86AudioMute"),
    (Named::AudioVolumeUp, 0x1008_FF13, "XF86AudioRaiseVolume"),
    (Named::MediaPlay, 0x1008_FF14, "XF86AudioPlay"),
    (Named::MediaPlayPause, 0x1008_FF14, "XF86AudioPlay"),
    (Named::MediaStop, 0x1008_FF15, "XF86AudioStop"),
    (Named::MediaTrackPrevious, 0x1008_FF16, "XF86AudioPrev"),
    (Named::MediaTrackNext, 0x1008_FF17, "XF86AudioNext"),
    (Named::BrowserHome, 0x1008_FF18, "XF86HomePage"),
    (Named::LaunchMail, 0x1008_FF19, "XF86Mail"),
    (Named::BrowserSearch, 0x1008_FF1B, "XF86Search"),
    (Named::MediaRecord, 0x1008_FF1C, "XF86AudioRecord"),
    (Named::LaunchApplication2, 0x1008_FF1D, "XF86Calculator"),
    (Named::LaunchCalendar, 0x1008_FF20, "XF86Calendar"),
    (Named::BrowserBack, 0x1008_FF26, "XF86Back"),
    (Named::BrowserForward, 0x1008_FF27, "XF86Forward"),
    (Named::BrowserStop, 0x1008_FF28, "XF86Stop"),
    (Named::BrowserRefresh, 0x1008_FF29, "XF86Refresh"),
    (Named::PowerOff, 0x1008_FF2A, "XF86PowerOff"),
    (Named::Power, 0x1008_FF2A, "XF86PowerOff"),
    (Named::WakeUp, 0x1008_FF2B, "XF86WakeUp"),
    (Named::Eject, 0x1008_FF2C, "XF86Eject"),
    (Named::LaunchScreenSaver, 0x1008_FF2D, "XF86ScreenSaver"),
    (Named::LaunchWebBrowser, 0x1008_FF2E, "XF86WWW"),
    (Named::BrowserFavorites, 0x1008_FF30, "XF86Favorites"),
    (Named::MediaPause, 0x1008_FF31, "XF86AudioPause"),
    (Named::LaunchMediaPlayer, 0x1008_FF32, "XF86AudioMedia"),
    (Named::LaunchApplication1, 0x1008_FF33, "XF86MyComputer"),
    (Named::MediaRewind, 0x1008_FF3E, "XF86AudioRewind"),
    (Named::Close, 0x1008_FF56, "XF86Close"),
    (Named::Copy, 0x1008_FF57, "XF86Copy"),
    (Named::Cut, 0x1008_FF58, "XF86Cut"),
    (Named::LogOff, 0x1008_FF61, "XF86LogOff"),
    (Named::New, 0x1008_FF68, "XF86New"),
    (Named::Open, 0x1008_FF6B, "XF86Open"),
    (Named::Paste, 0x1008_FF6D, "XF86Paste"),
    (Named::Save, 0x1008_FF77, "XF86Save"),
    (Named::ZoomIn, 0x1008_FF8B, "XF86ZoomIn"),
    (Named::ZoomOut, 0x1008_FF8C, "XF86ZoomOut"),
    (Named::MediaFastForward, 0x1008_FF97, "XF86AudioForward"),
    (Named::Hibernate, 0x1008_FFA8, "XF86Hibernate"),
    (Named::MicrophoneVolumeMute, 0x1008_FFB2, "XF86AudioMicMute"),
];

pub struct KeyPress<'a> {
    pub key: &'a Key,
    pub modified_key: &'a Key,
    pub location: Location,
    pub modifiers: Modifiers,
    pub text: Option<&'a str>,
}

pub fn keysym_name(keysym: u32) -> Option<&'static str> {
    VENDOR
        .iter()
        .find(|(_, k, _)| *k == keysym)
        .map(|(_, _, name)| *name)
}

pub fn emacs_modifiers(modifiers: Modifiers) -> i64 {
    let mut m = 0;
    if modifiers.shift() {
        m |= SHIFT_MODIFIER;
    }
    if modifiers.control() {
        m |= CTRL_MODIFIER;
    }
    if modifiers.alt() {
        m |= META_MODIFIER;
    }
    if modifiers.logo() {
        m |= SUPER_MODIFIER;
    }
    m
}

fn keypad_char(c: char) -> Option<u32> {
    Some(match c {
        '0'..='9' => 0xFFB0 + (c as u32 - '0' as u32),
        '*' => 0xFFAA,
        '+' => 0xFFAB,
        ',' => 0xFFAC,
        '-' => 0xFFAD,
        '.' => 0xFFAE,
        '/' => 0xFFAF,
        '=' => 0xFFBD,
        _ => return None,
    })
}

fn keypad_named(named: Named) -> Option<u32> {
    Some(match named {
        Named::Space => 0xFF80,
        Named::Tab => 0xFF89,
        Named::Enter => 0xFF8D,
        Named::Home => 0xFF95,
        Named::ArrowLeft => 0xFF96,
        Named::ArrowUp => 0xFF97,
        Named::ArrowRight => 0xFF98,
        Named::ArrowDown => 0xFF99,
        Named::PageUp => 0xFF9A,
        Named::PageDown => 0xFF9B,
        Named::End => 0xFF9C,
        Named::Clear => 0xFF9D,
        Named::Insert => 0xFF9E,
        Named::Delete => 0xFF9F,
        _ => return None,
    })
}

fn function_key(named: Named) -> Option<u32> {
    let n = match named {
        Named::F1 => 1,
        Named::F2 => 2,
        Named::F3 => 3,
        Named::F4 => 4,
        Named::F5 => 5,
        Named::F6 => 6,
        Named::F7 => 7,
        Named::F8 => 8,
        Named::F9 => 9,
        Named::F10 => 10,
        Named::F11 => 11,
        Named::F12 => 12,
        Named::F13 => 13,
        Named::F14 => 14,
        Named::F15 => 15,
        Named::F16 => 16,
        Named::F17 => 17,
        Named::F18 => 18,
        Named::F19 => 19,
        Named::F20 => 20,
        Named::F21 => 21,
        Named::F22 => 22,
        Named::F23 => 23,
        Named::F24 => 24,
        Named::F25 => 25,
        Named::F26 => 26,
        Named::F27 => 27,
        Named::F28 => 28,
        Named::F29 => 29,
        Named::F30 => 30,
        Named::F31 => 31,
        Named::F32 => 32,
        Named::F33 => 33,
        Named::F34 => 34,
        Named::F35 => 35,
        _ => return None,
    };
    Some(0xFFBE + n - 1)
}

fn char_keysym(c: char) -> u32 {
    let code = c as u32;
    if (0x20..0x7F).contains(&code) {
        code
    } else {
        0x0100_0000 | code
    }
}

pub fn keysym(press: &KeyPress) -> u32 {
    let numpad = press.location == Location::Numpad;
    match press.modified_key {
        Key::Named(named) => {
            if numpad && let Some(k) = keypad_named(*named) {
                return k;
            }
            if *named == Named::Tab && press.modifiers.shift() {
                return ISO_LEFT_TAB;
            }
            if let Some(k) = function_key(*named) {
                return k;
            }
            if let Some((_, k)) = NAMED.iter().find(|(n, _)| n == named) {
                return *k;
            }
            VENDOR
                .iter()
                .find(|(n, _, _)| n == named)
                .map_or(VOID_SYMBOL, |(_, k, _)| *k)
        }
        Key::Character(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => {
                    if numpad && let Some(k) = keypad_char(c) {
                        return k;
                    }
                    char_keysym(c)
                }
                _ => VOID_SYMBOL,
            }
        }
        Key::Unidentified => VOID_SYMBOL,
    }
}

fn is_modifier_key(keysym: u32) -> bool {
    (SHIFT_L..=HYPER_R).contains(&keysym)
        || (ISO_LOCK..=ISO_LAST_GROUP_LOCK).contains(&keysym)
        || keysym == 0xFE03
        || keysym == MODE_SWITCH
        || keysym == NUM_LOCK
}

fn is_function_like(keysym: u32) -> bool {
    (BACKSPACE..=ESCAPE).contains(&keysym)
        || keysym == DELETE
        || (ISO_LEFT_TAB..=ISO_ENTER).contains(&keysym)
        || (0xFF50..0xFF60).contains(&keysym)
        || (0xFF60..0xFF80).contains(&keysym)
        || (0xFF80..0xFFBE).contains(&keysym)
        || (0xFFBE..0xFFE1).contains(&keysym)
        || keysym & VENDOR_BIT != 0
        || (0xFE50..=0xFE60).contains(&keysym)
        || (0xFF20..0xFF40).contains(&keysym)
}

pub fn key_press_event(press: &KeyPress, timestamp: u64, frame: u64) -> Option<InputEvent> {
    let keysym = keysym(press);
    if keysym != VOID_SYMBOL && is_modifier_key(keysym) {
        return None;
    }
    if matches!(
        press.modified_key,
        Key::Named(Named::Fn | Named::FnLock | Named::Symbol | Named::SymbolLock)
    ) {
        return None;
    }
    let mut event = InputEvent {
        modifiers: emacs_modifiers(press.modifiers),
        timestamp,
        frame,
        ..InputEvent::default()
    };
    if (32..128).contains(&keysym) {
        event.kind = EventKind::AsciiKeystroke;
        event.code = keysym;
        return Some(event);
    }
    if (0x0100_0000..=0x0110_FFFF).contains(&keysym) {
        event.kind = if keysym < 0x0100_0080 {
            EventKind::AsciiKeystroke
        } else {
            EventKind::MultibyteCharKeystroke
        };
        event.code = keysym & 0x00FF_FFFF;
        return Some(event);
    }
    let nbytes = press.text.map_or(0, str::len);
    if is_function_like(keysym) || (keysym != VOID_SYMBOL && nbytes == 0) {
        event.kind = EventKind::NonAsciiKeystroke;
        event.code = keysym;
        return Some(event);
    }
    event.kind = EventKind::MultibyteCharKeystroke;
    event.arg = Some(press.text.unwrap_or("").to_string());
    Some(event)
}
