pub const MAX_CHAR: u32 = 0x3F_FFFF;
pub const MAX_UNICODE_CHAR: u32 = 0x10_FFFF;
pub const MAX_1_BYTE_CHAR: u32 = 0x7F;
pub const MAX_2_BYTE_CHAR: u32 = 0x7FF;
pub const MAX_3_BYTE_CHAR: u32 = 0xFFFF;
pub const MAX_4_BYTE_CHAR: u32 = 0x1F_FFFF;
pub const MAX_5_BYTE_CHAR: u32 = 0x3F_FF7F;
pub const MAX_MULTIBYTE_LENGTH: usize = 5;
pub const NO_BREAK_SPACE: u32 = 0xA0;

pub const CHAR_ALT: i64 = 0x0040_0000;
pub const CHAR_SUPER: i64 = 0x0080_0000;
pub const CHAR_HYPER: i64 = 0x0100_0000;
pub const CHAR_SHIFT: i64 = 0x0200_0000;
pub const CHAR_CTL: i64 = 0x0400_0000;
pub const CHAR_META: i64 = 0x0800_0000;
pub const CHAR_MODIFIER_MASK: i64 =
    CHAR_ALT | CHAR_SUPER | CHAR_HYPER | CHAR_SHIFT | CHAR_CTL | CHAR_META;

pub const fn char_byte8_p(c: u32) -> bool {
    c > MAX_5_BYTE_CHAR
}

pub const fn byte8_to_char(byte: u8) -> u32 {
    byte as u32 + 0x3F_FF00
}

pub const fn char_to_byte8(c: u32) -> u8 {
    if char_byte8_p(c) {
        (c - 0x3F_FF00) as u8
    } else {
        (c & 0xFF) as u8
    }
}

pub const fn ascii_char_p(c: u32) -> bool {
    c < 0x80
}

pub const fn single_byte_char_p(c: u32) -> bool {
    c < 0x100
}

pub const fn char_bytes(c: u32) -> usize {
    if c <= MAX_1_BYTE_CHAR {
        1
    } else if c <= MAX_2_BYTE_CHAR {
        2
    } else if c <= MAX_3_BYTE_CHAR {
        3
    } else if c <= MAX_4_BYTE_CHAR {
        4
    } else if c <= MAX_5_BYTE_CHAR {
        5
    } else {
        2
    }
}

pub fn push_char(out: &mut Vec<u8>, c: u32) {
    if c <= MAX_1_BYTE_CHAR {
        out.push(c as u8);
    } else if c <= MAX_2_BYTE_CHAR {
        out.push(0xC0 | (c >> 6) as u8);
        out.push(0x80 | (c & 0x3F) as u8);
    } else if c <= MAX_3_BYTE_CHAR {
        out.push(0xE0 | (c >> 12) as u8);
        out.push(0x80 | ((c >> 6) & 0x3F) as u8);
        out.push(0x80 | (c & 0x3F) as u8);
    } else if c <= MAX_4_BYTE_CHAR {
        out.push(0xF0 | (c >> 18) as u8);
        out.push(0x80 | ((c >> 12) & 0x3F) as u8);
        out.push(0x80 | ((c >> 6) & 0x3F) as u8);
        out.push(0x80 | (c & 0x3F) as u8);
    } else if c <= MAX_5_BYTE_CHAR {
        out.push(0xF8);
        out.push(0x80 | ((c >> 18) & 0x0F) as u8);
        out.push(0x80 | ((c >> 12) & 0x3F) as u8);
        out.push(0x80 | ((c >> 6) & 0x3F) as u8);
        out.push(0x80 | (c & 0x3F) as u8);
    } else {
        let byte = char_to_byte8(c);
        out.push(0xC0 | ((byte >> 6) & 1));
        out.push(0x80 | (byte & 0x3F));
    }
}

pub fn char_string(c: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(char_bytes(c));
    push_char(&mut out, c);
    out
}

pub fn multibyte_length(bytes: &[u8]) -> Option<usize> {
    let first = *bytes.first()?;
    let cont = |i: usize| bytes.get(i).is_some_and(|b| b & 0xC0 == 0x80);
    if first < 0x80 {
        Some(1)
    } else if first < 0xC0 {
        None
    } else if first < 0xE0 {
        cont(1).then_some(2)
    } else if first < 0xF0 {
        (cont(1) && cont(2) && (first != 0xE0 || bytes[1] >= 0xA0)).then_some(3)
    } else if first < 0xF8 {
        (cont(1) && cont(2) && cont(3) && (first != 0xF0 || bytes[1] >= 0x90)).then_some(4)
    } else if first == 0xF8 {
        (cont(1) && cont(2) && cont(3) && cont(4) && (0x88..0x90).contains(&bytes[1])).then_some(5)
    } else {
        None
    }
}

pub fn string_char(bytes: &[u8]) -> (u32, usize) {
    let first = bytes[0];
    if first < 0x80 {
        return (first as u32, 1);
    }
    match multibyte_length(bytes) {
        Some(2) if first < 0xC2 => {
            let byte = ((first & 1) << 6) | (bytes[1] & 0x3F);
            (byte8_to_char(byte | 0x80), 2)
        }
        Some(2) => (((first as u32 & 0x1F) << 6) | (bytes[1] as u32 & 0x3F), 2),
        Some(3) => (
            ((first as u32 & 0x0F) << 12)
                | ((bytes[1] as u32 & 0x3F) << 6)
                | (bytes[2] as u32 & 0x3F),
            3,
        ),
        Some(4) => (
            ((first as u32 & 0x07) << 18)
                | ((bytes[1] as u32 & 0x3F) << 12)
                | ((bytes[2] as u32 & 0x3F) << 6)
                | (bytes[3] as u32 & 0x3F),
            4,
        ),
        Some(5) => (
            ((bytes[1] as u32 & 0x0F) << 18)
                | ((bytes[2] as u32 & 0x3F) << 12)
                | ((bytes[3] as u32 & 0x3F) << 6)
                | (bytes[4] as u32 & 0x3F),
            5,
        ),
        _ => (byte8_to_char(first), 1),
    }
}

pub fn chars_in_multibyte(bytes: &[u8]) -> usize {
    let mut i = 0;
    let mut n = 0;
    while i < bytes.len() {
        i += multibyte_length(&bytes[i..]).unwrap_or(1);
        n += 1;
    }
    n
}

pub fn multibyte_chars(bytes: &[u8]) -> impl Iterator<Item = u32> + '_ {
    let mut i = 0;
    std::iter::from_fn(move || {
        if i >= bytes.len() {
            return None;
        }
        let (c, len) = string_char(&bytes[i..]);
        i += len;
        Some(c)
    })
}

pub fn char_offset_to_byte(bytes: &[u8], chars: usize) -> usize {
    let mut i = 0;
    for _ in 0..chars {
        if i >= bytes.len() {
            break;
        }
        i += multibyte_length(&bytes[i..]).unwrap_or(1);
    }
    i
}

pub fn str_to_multibyte(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for &b in bytes {
        if b < 0x80 {
            out.push(b);
        } else {
            push_char(&mut out, byte8_to_char(b));
        }
    }
    out
}

pub fn str_as_unibyte(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let (c, len) = string_char(&bytes[i..]);
        if char_byte8_p(c) {
            out.push(char_to_byte8(c));
        } else {
            out.extend_from_slice(&bytes[i..i + len]);
        }
        i += len;
    }
    out
}

pub fn char_hexdigit(c: i64) -> i64 {
    match c {
        0x30..=0x39 => c - 0x30,
        0x41..=0x46 => c - 0x41 + 10,
        0x61..=0x66 => c - 0x61 + 10,
        _ => -1,
    }
}

pub fn encode_utf8_str(s: &str) -> (Vec<u8>, usize, bool) {
    let chars = s.chars().count();
    (s.as_bytes().to_vec(), chars, chars != s.len())
}

pub fn decode_to_string(bytes: &[u8], multibyte: bool) -> String {
    let mut out = String::with_capacity(bytes.len());
    if multibyte {
        for c in multibyte_chars(bytes) {
            out.push(char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER));
        }
    } else {
        for &b in bytes {
            out.push(char::from(b));
        }
    }
    out
}
