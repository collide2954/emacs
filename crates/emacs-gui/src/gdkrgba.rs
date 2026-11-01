use std::collections::HashMap;
use std::sync::OnceLock;

const RGB_TXT: &str = include_str!("../../../etc/rgb.txt");

fn normalize(name: &[u8]) -> Vec<u8> {
    name.iter()
        .filter(|&&c| c != b' ')
        .map(u8::to_ascii_lowercase)
        .collect()
}

fn color_table() -> &'static HashMap<Vec<u8>, (u16, u16, u16)> {
    static TABLE: OnceLock<HashMap<Vec<u8>, (u16, u16, u16)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = HashMap::new();
        for line in RGB_TXT.lines() {
            if line.starts_with('#') || line.starts_with('!') {
                continue;
            }
            let mut fields = line.split_whitespace();
            let rgb: Vec<u32> = fields
                .by_ref()
                .take(3)
                .filter_map(|f| f.parse().ok())
                .collect();
            let name: Vec<&str> = fields.collect();
            if let [r, g, b] = rgb[..]
                && !name.is_empty()
            {
                table
                    .entry(normalize(name.join(" ").as_bytes()))
                    .or_insert((
                        (r * 65535 / 255) as u16,
                        (g * 65535 / 255) as u16,
                        (b * 65535 / 255) as u16,
                    ));
            }
        }
        table
    })
}

fn hex(spec: &[u8]) -> Option<u32> {
    spec.iter()
        .try_fold(0u32, |c, &d| (d as char).to_digit(16).map(|v| (c << 4) | v))
}

fn pango_color_parse_with_alpha(spec: &[u8]) -> Option<(u16, u16, u16)> {
    if let Some(spec) = spec.strip_prefix(b"#") {
        let (len, has_alpha) = match spec.len() {
            3 | 6 | 9 | 12 => (spec.len() / 3, false),
            4 | 8 | 16 => (spec.len() / 4, true),
            _ => return None,
        };
        let mut r = hex(&spec[..len])?;
        let mut g = hex(&spec[len..2 * len])?;
        let mut b = hex(&spec[2 * len..3 * len])?;
        if has_alpha {
            hex(&spec[3 * len..4 * len])?;
        }
        let mut bits = len as u32 * 4;
        r <<= 16 - bits;
        g <<= 16 - bits;
        b <<= 16 - bits;
        while bits < 16 {
            r |= r >> bits;
            g |= g >> bits;
            b |= b >> bits;
            bits *= 2;
        }
        Some((r as u16, g as u16, b as u16))
    } else {
        color_table().get(&normalize(spec)).copied()
    }
}

fn skip_whitespace(s: &[u8], mut i: usize) -> usize {
    while s.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}

fn strtod(s: &[u8], start: usize) -> Option<(f64, usize)> {
    let mut i = skip_whitespace(s, start);
    let begin = i;
    if matches!(s.get(i), Some(b'-' | b'+')) {
        i += 1;
    }
    let mut digits = 0;
    while s.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
        digits += 1;
    }
    if s.get(i) == Some(&b'.') {
        i += 1;
        while s.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if matches!(s.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        if matches!(s.get(j), Some(b'-' | b'+')) {
            j += 1;
        }
        if s.get(j).is_some_and(u8::is_ascii_digit) {
            while s.get(j).is_some_and(u8::is_ascii_digit) {
                j += 1;
            }
            i = j;
        }
    }
    let value = std::str::from_utf8(&s[begin..i])
        .ok()?
        .parse::<f64>()
        .ok()?;
    Some((value, i))
}

fn parse_rgb_value(s: &[u8], start: usize) -> Option<(f64, usize)> {
    let (number, end) = strtod(s, start)?;
    if !number.is_finite() {
        return None;
    }
    let p = skip_whitespace(s, end);
    if s.get(p) == Some(&b'%') {
        Some(((number / 100.0).clamp(0.0, 1.0), p + 1))
    } else {
        Some(((number / 255.0).clamp(0.0, 1.0), end))
    }
}

pub fn gdk_rgba_parse(spec: &[u8]) -> Option<[f64; 4]> {
    let (has_alpha, mut i) = if spec.starts_with(b"rgba") {
        (true, 4)
    } else if spec.starts_with(b"rgb") {
        (false, 3)
    } else {
        let (r, g, b) = pango_color_parse_with_alpha(spec)?;
        return Some([
            f64::from(r) / 65535.0,
            f64::from(g) / 65535.0,
            f64::from(b) / 65535.0,
            1.0,
        ]);
    };
    i = skip_whitespace(spec, i);
    if spec.get(i) != Some(&b'(') {
        return None;
    }
    i += 1;
    let mut channels = [0.0; 3];
    for (n, channel) in channels.iter_mut().enumerate() {
        i = skip_whitespace(spec, i);
        let (value, end) = parse_rgb_value(spec, i)?;
        *channel = value;
        i = skip_whitespace(spec, end);
        if n < 2 {
            if spec.get(i) != Some(&b',') {
                return None;
            }
            i += 1;
        }
    }
    let mut alpha = 1.0;
    if has_alpha {
        if spec.get(i) != Some(&b',') {
            return None;
        }
        i = skip_whitespace(spec, i + 1);
        let (value, end) = strtod(spec, i).unwrap_or((0.0, i));
        if value.is_infinite() {
            return None;
        }
        alpha = value;
        i = skip_whitespace(spec, end);
    }
    if spec.get(i) != Some(&b')') {
        return None;
    }
    i = skip_whitespace(spec, i + 1);
    if i != spec.len() {
        return None;
    }
    Some([channels[0], channels[1], channels[2], alpha.clamp(0.0, 1.0)])
}
