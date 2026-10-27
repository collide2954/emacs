use std::time::{SystemTime, UNIX_EPOCH};

use jiff::tz::{AmbiguousOffset, TimeZone};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::error::{Flow, LispResult};
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

const TIMESPEC_HZ: i64 = 1_000_000_000;
const TRILLION: i64 = 1_000_000_000_000;
const LO_TIME_BITS: u32 = 16;
const TM_YEAR_BASE: i64 = 1900;

#[derive(Clone, Debug)]
pub enum Zone {
    Utc,
    Fixed { offset: i64, abbr: String },
    Tz(TimeZone),
}

#[derive(Clone, Debug)]
pub struct Tm {
    pub sec: i64,
    pub min: i64,
    pub hour: i64,
    pub mday: i64,
    pub mon: i64,
    pub year: i64,
    pub wday: i64,
    pub yday: i64,
    pub isdst: i64,
    pub gmtoff: i64,
    pub zone: String,
}

impl Tm {
    fn tm_year(&self) -> i64 {
        self.year - TM_YEAR_BASE
    }
}

pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn zone_offset_info(zone: &Zone, t: i64) -> Option<(i64, i64, String)> {
    match zone {
        Zone::Utc => Some((0, 0, "UTC".to_string())),
        Zone::Fixed { offset, abbr } => Some((*offset, 0, abbr.clone())),
        Zone::Tz(tz) => {
            let ts = jiff::Timestamp::from_second(t).ok()?;
            let info = tz.to_offset_info(ts);
            Some((
                info.offset().seconds() as i64,
                i64::from(info.dst().is_dst()),
                info.abbreviation().to_string(),
            ))
        }
    }
}

pub fn localtime(zone: &Zone, t: i64) -> Option<Tm> {
    let (offset, isdst, abbr) = zone_offset_info(zone, t)?;
    let local = t.checked_add(offset)?;
    let days = local.div_euclid(86400);
    let secs = local.rem_euclid(86400);
    let (year, month, day) = civil_from_days(days);
    Some(Tm {
        sec: secs % 60,
        min: secs / 60 % 60,
        hour: secs / 3600,
        mday: day,
        mon: month - 1,
        year,
        wday: (days + 4).rem_euclid(7),
        yday: days - days_from_civil(year, 1, 1),
        isdst,
        gmtoff: offset,
        zone: abbr,
    })
}

pub fn mktime(zone: &Zone, fields: [i64; 6], isdst: i64) -> Option<i64> {
    let [sec, min, hour, mday, mon, year] = fields;
    let y = year.checked_add(mon.div_euclid(12))?;
    let m = mon.rem_euclid(12) + 1;
    let days = days_from_civil(y, m, 1).checked_add(mday - 1)?;
    let local = days
        .checked_mul(86400)?
        .checked_add(hour.checked_mul(3600)?)?
        .checked_add(min.checked_mul(60)?)?
        .checked_add(sec)?;
    match zone {
        Zone::Utc => Some(local),
        Zone::Fixed { offset, .. } => local.checked_sub(*offset),
        Zone::Tz(tz) => {
            let ld = local.div_euclid(86400);
            let ls = local.rem_euclid(86400);
            let (cy, cm, cd) = civil_from_days(ld);
            let dt = jiff::civil::DateTime::new(
                i16::try_from(cy).ok()?,
                cm as i8,
                cd as i8,
                (ls / 3600) as i8,
                (ls / 60 % 60) as i8,
                (ls % 60) as i8,
                0,
            )
            .ok()?;
            let off = match tz.to_ambiguous_timestamp(dt).offset() {
                AmbiguousOffset::Unambiguous { offset } => offset,
                AmbiguousOffset::Gap { before, after } => {
                    if isdst == 1 {
                        after
                    } else {
                        before
                    }
                }
                AmbiguousOffset::Fold { before, after } => {
                    if isdst == 0 {
                        after
                    } else {
                        before
                    }
                }
            };
            local.checked_sub(off.seconds() as i64)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pad {
    Zero,
    AlwaysZero,
    Sign,
    Space,
    No,
}

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn isleap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn iso_week_days(yday: i64, wday: i64) -> i64 {
    let big_enough_multiple_of_7 = (366 / 7 + 2) * 7;
    yday - (yday - wday + 4 + big_enough_multiple_of_7) % 7 + 4 - 1
}

struct Fmt<'a> {
    out: &'a mut Vec<u8>,
    pad: Pad,
    width: i64,
    to_lowcase: bool,
    to_uppcase: bool,
}

impl Fmt<'_> {
    fn width_add(&mut self, width: i64, bytes: &[u8]) {
        let w = if self.pad == Pad::No || width < 0 {
            0
        } else {
            width as usize
        };
        if bytes.len() < w {
            let fill = if self.pad == Pad::AlwaysZero || self.pad == Pad::Sign {
                b'0'
            } else {
                b' '
            };
            self.out.extend(std::iter::repeat_n(fill, w - bytes.len()));
        }
        if self.to_lowcase {
            self.out.extend(bytes.iter().map(u8::to_ascii_lowercase));
        } else if self.to_uppcase {
            self.out.extend(bytes.iter().map(u8::to_ascii_uppercase));
        } else {
            self.out.extend_from_slice(bytes);
        }
    }

    fn cpy(&mut self, bytes: &[u8]) {
        let width = self.width;
        self.width_add(width, bytes);
    }

    fn add1(&mut self, b: u8) {
        let width = self.width;
        let lower = self.to_lowcase;
        let upper = self.to_uppcase;
        self.to_lowcase = false;
        self.to_uppcase = false;
        self.width_add(width, &[b]);
        self.to_lowcase = lower;
        self.to_uppcase = upper;
    }
}

#[allow(clippy::too_many_arguments)]
fn number_out(
    fmt: &mut Fmt,
    digits: i64,
    negative: bool,
    magnitude: u64,
    always_sign: bool,
    mut colon_mask: u32,
) {
    let mut buf: Vec<u8> = Vec::new();
    let mut u = magnitude;
    loop {
        if colon_mask & 1 != 0 {
            buf.push(b':');
        }
        colon_mask >>= 1;
        buf.push(b'0' + (u % 10) as u8);
        u /= 10;
        if u == 0 && colon_mask == 0 {
            break;
        }
    }
    buf.reverse();
    sign_and_padding(fmt, digits, negative, always_sign, &buf);
}

fn sign_and_padding(fmt: &mut Fmt, digits: i64, negative: bool, always_sign: bool, buf: &[u8]) {
    if fmt.pad == Pad::Zero {
        fmt.pad = Pad::AlwaysZero;
    }
    if fmt.width < 0 {
        fmt.width = digits;
    }
    let sign = if negative {
        Some(b'-')
    } else if always_sign {
        Some(b'+')
    } else {
        None
    };
    let shortage = fmt.width - i64::from(sign.is_some()) - buf.len() as i64;
    let padding = if fmt.pad == Pad::No || shortage <= 0 {
        0
    } else {
        shortage
    };
    if let Some(s) = sign {
        if fmt.pad == Pad::Space {
            fmt.out.extend(std::iter::repeat_n(b' ', padding as usize));
            fmt.width -= padding;
        }
        fmt.width_add(0, &[s]);
        fmt.width -= 1;
    }
    fmt.cpy(buf);
}

#[allow(clippy::too_many_arguments)]
fn strftime_internal(
    out: &mut Vec<u8>,
    format: &[u8],
    tp: &Tm,
    upcase: bool,
    yr_spec: Pad,
    width_init: i64,
    ns: i64,
    zone: &Zone,
) -> Result<(), ()> {
    let at = |i: usize| format.get(i).copied().unwrap_or(0);
    let mut hour12 = tp.hour;
    if hour12 > 12 {
        hour12 -= 12;
    } else if hour12 == 0 {
        hour12 = 12;
    }
    let tm_year = tp.tm_year();
    let mut width = width_init;
    let mut f = 0usize;
    while f < format.len() {
        if format[f] != b'%' {
            let mut fm = Fmt {
                out: &mut *out,
                pad: Pad::Zero,
                width,
                to_lowcase: false,
                to_uppcase: upcase,
            };
            fm.add1(format[f]);
            f += 1;
            width = -1;
            continue;
        }
        let percent = f;
        let mut pad = Pad::Zero;
        let mut to_uppcase = upcase;
        let to_lowcase = false;
        let mut change_case = false;
        loop {
            f += 1;
            match at(f) {
                b'_' => pad = Pad::Space,
                b'-' => pad = Pad::No,
                b'+' => pad = Pad::Sign,
                b'0' => pad = Pad::AlwaysZero,
                b'^' => to_uppcase = true,
                b'#' => change_case = true,
                _ => break,
            }
        }
        if at(f).is_ascii_digit() {
            width = 0;
            while at(f).is_ascii_digit() {
                width = width
                    .checked_mul(10)
                    .and_then(|w| w.checked_add((at(f) - b'0') as i64))
                    .ok_or(())?;
                f += 1;
            }
        }
        let modifier = match at(f) {
            b'E' | b'O' => {
                let m = at(f);
                f += 1;
                m
            }
            _ => 0,
        };
        let format_char = at(f);
        let mut fm = Fmt {
            out: &mut *out,
            pad,
            width,
            to_lowcase,
            to_uppcase,
        };
        enum Sub<'s> {
            None,
            Fmt(&'s [u8], i64),
        }
        let mut sub = Sub::None;
        let mut bad = false;
        macro_rules! number {
            ($d:expr, $v:expr) => {{
                let v: i64 = $v;
                number_out(&mut fm, $d, v < 0, v.unsigned_abs(), false, 0);
            }};
        }
        macro_rules! number_spacepad {
            ($d:expr, $v:expr) => {{
                if fm.pad == Pad::Zero {
                    fm.pad = Pad::Space;
                }
                number!($d, $v);
            }};
        }
        macro_rules! signed_number {
            ($d:expr, $neg:expr, $v:expr) => {{
                let v: i64 = $v;
                number_out(&mut fm, $d, $neg, v.unsigned_abs(), false, 0);
            }};
        }
        macro_rules! yearish {
            ($d:expr, $neg:expr, $v:expr) => {{
                let v: i64 = $v;
                if fm.pad == Pad::Zero {
                    fm.pad = yr_spec;
                }
                let u = v.unsigned_abs();
                let always = fm.pad == Pad::Sign
                    && ((if $d == 2 { 99 } else { 9999 }) < u || ($d as i64) < fm.width);
                number_out(&mut fm, $d, $neg, u, always, 0);
            }};
        }
        match format_char {
            b'%' => {
                if f - 1 != percent {
                    f -= 1;
                    bad = true;
                } else {
                    fm.add1(b'%');
                }
            }
            b'a' | b'A' => {
                if modifier != 0 {
                    bad = true;
                } else {
                    if change_case {
                        fm.to_uppcase = true;
                        fm.to_lowcase = false;
                    }
                    let name = WEEKDAYS
                        .get(tp.wday as usize)
                        .map_or("?", |n| if format_char == b'a' { &n[..3] } else { n });
                    fm.cpy(name.as_bytes());
                }
            }
            b'b' | b'h' | b'B' => {
                if change_case {
                    fm.to_uppcase = true;
                    fm.to_lowcase = false;
                }
                if modifier == b'E' {
                    bad = true;
                } else {
                    let name = MONTHS
                        .get(tp.mon as usize)
                        .map_or("?", |n| if format_char == b'B' { n } else { &n[..3] });
                    fm.cpy(name.as_bytes());
                }
            }
            b'c' => {
                if modifier == b'O' {
                    bad = true;
                } else {
                    sub = Sub::Fmt(b"%a %b %e %H:%M:%S %Y", -1);
                }
            }
            b'C' => {
                let negative_year = tm_year < -TM_YEAR_BASE;
                let zero_thru_1899 = !negative_year && tm_year < 0;
                let century = (tm_year - 99 * i64::from(zero_thru_1899)) / 100 + TM_YEAR_BASE / 100;
                yearish!(2, negative_year, century);
            }
            b'x' => {
                if modifier == b'O' {
                    bad = true;
                } else {
                    sub = Sub::Fmt(b"%m/%d/%y", -1);
                }
            }
            b'D' => {
                if modifier != 0 {
                    bad = true;
                } else {
                    sub = Sub::Fmt(b"%m/%d/%y", -1);
                }
            }
            b'd' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number!(2, tp.mday);
                }
            }
            b'e' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number_spacepad!(2, tp.mday);
                }
            }
            b'F' => {
                if modifier != 0 {
                    bad = true;
                } else {
                    let subwidth = if fm.pad == Pad::Zero && fm.width < 0 {
                        fm.pad = Pad::Sign;
                        4
                    } else {
                        (fm.width - 6).max(0)
                    };
                    sub = Sub::Fmt(b"%Y-%m-%d", subwidth);
                }
            }
            b'H' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number!(2, tp.hour);
                }
            }
            b'I' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number!(2, hour12);
                }
            }
            b'k' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number_spacepad!(2, tp.hour);
                }
            }
            b'l' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number_spacepad!(2, hour12);
                }
            }
            b'j' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    signed_number!(3, tp.yday < -1, tp.yday + 1);
                }
            }
            b'M' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number!(2, tp.min);
                }
            }
            b'm' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    signed_number!(2, tp.mon < -1, tp.mon + 1);
                }
            }
            b'N' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    let mut n = ns;
                    let ns_digits = 9i64;
                    if fm.width <= 0 {
                        fm.width = ns_digits;
                    }
                    let mut ndigs = ns_digits;
                    while fm.width < ndigs || (1 < ndigs && n % 10 == 0) {
                        ndigs -= 1;
                        n /= 10;
                    }
                    let mut buf = vec![b'0'; ndigs as usize];
                    for j in (0..ndigs as usize).rev() {
                        buf[j] = b'0' + (n % 10) as u8;
                        n /= 10;
                    }
                    if fm.pad == Pad::Zero {
                        fm.pad = Pad::AlwaysZero;
                    }
                    fm.width_add(0, &buf);
                    let rest = fm.width - ndigs;
                    fm.width_add(rest, &[]);
                }
            }
            b'n' => fm.add1(b'\n'),
            b'P' | b'p' => {
                if format_char == b'P' {
                    fm.to_lowcase = true;
                }
                if change_case {
                    fm.to_uppcase = false;
                    fm.to_lowcase = true;
                }
                fm.cpy(if tp.hour >= 12 { b"PM" } else { b"AM" });
            }
            b'q' => signed_number!(1, false, ((tp.mon * 11) >> 5) + 1),
            b'R' => sub = Sub::Fmt(b"%H:%M", -1),
            b'r' => sub = Sub::Fmt(b"%I:%M:%S %p", -1),
            b'S' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number!(2, tp.sec);
                }
            }
            b's' => {
                let fields = [tp.sec, tp.min, tp.hour, tp.mday, tp.mon, tp.year];
                let t = mktime(zone, fields, tp.isdst).ok_or(())?;
                let buf = t.unsigned_abs().to_string().into_bytes();
                sign_and_padding(&mut fm, 1, t < 0, false, &buf);
            }
            b'X' => {
                if modifier == b'O' {
                    bad = true;
                } else {
                    sub = Sub::Fmt(b"%H:%M:%S", -1);
                }
            }
            b'T' => sub = Sub::Fmt(b"%H:%M:%S", -1),
            b't' => fm.add1(b'\t'),
            b'u' => number!(1, (tp.wday - 1 + 7) % 7 + 1),
            b'U' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number!(2, (tp.yday - tp.wday + 7) / 7);
                }
            }
            b'V' | b'g' | b'G' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    let year = tm_year
                        + if tm_year < 0 {
                            TM_YEAR_BASE % 400
                        } else {
                            TM_YEAR_BASE % 400 - 400
                        };
                    let mut year_adjust = 0;
                    let mut days = iso_week_days(tp.yday, tp.wday);
                    if days < 0 {
                        year_adjust = -1;
                        days =
                            iso_week_days(tp.yday + (365 + i64::from(isleap(year - 1))), tp.wday);
                    } else {
                        let d = iso_week_days(tp.yday - (365 + i64::from(isleap(year))), tp.wday);
                        if 0 <= d {
                            year_adjust = 1;
                            days = d;
                        }
                    }
                    match format_char {
                        b'g' => {
                            let yy = (tm_year % 100 + year_adjust) % 100;
                            let v = if 0 <= yy {
                                yy
                            } else if tm_year < -TM_YEAR_BASE - year_adjust {
                                -yy
                            } else {
                                yy + 100
                            };
                            yearish!(2, false, v);
                        }
                        b'G' => {
                            yearish!(
                                4,
                                tm_year < -TM_YEAR_BASE - year_adjust,
                                tm_year + TM_YEAR_BASE + year_adjust
                            );
                        }
                        _ => number!(2, days / 7 + 1),
                    }
                }
            }
            b'W' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number!(2, (tp.yday - (tp.wday - 1 + 7) % 7 + 7) / 7);
                }
            }
            b'w' => {
                if modifier == b'E' {
                    bad = true;
                } else {
                    number!(1, tp.wday);
                }
            }
            b'Y' => yearish!(4, tm_year < -TM_YEAR_BASE, tm_year + TM_YEAR_BASE),
            b'y' => {
                let mut yy = tm_year % 100;
                if yy < 0 {
                    yy = if tm_year < -TM_YEAR_BASE {
                        -yy
                    } else {
                        yy + 100
                    };
                }
                yearish!(2, false, yy);
            }
            b'Z' => {
                if change_case {
                    fm.to_uppcase = false;
                    fm.to_lowcase = true;
                }
                fm.cpy(tp.zone.as_bytes());
            }
            b':' | b'z' => {
                let mut colons = 0usize;
                if format_char == b':' {
                    colons = 1;
                    while at(f + colons) == b':' {
                        colons += 1;
                    }
                    if at(f + colons) != b'z' {
                        bad = true;
                    } else {
                        f += colons;
                    }
                }
                if !bad && tp.isdst >= 0 {
                    let diff = tp.gmtoff;
                    let mut negative = diff < 0;
                    if diff == 0 {
                        negative = tp.zone.starts_with('-');
                    }
                    let hour_diff = diff / 60 / 60;
                    let min_diff = diff / 60 % 60;
                    let sec_diff = diff % 60;
                    let mut tz_out = |digits: i64, mask: u32, v: i64| {
                        number_out(&mut fm, digits, negative, v.unsigned_abs(), true, mask);
                    };
                    match colons {
                        0 => tz_out(5, 0, hour_diff * 100 + min_diff),
                        1 => tz_out(6, 0o4, hour_diff * 100 + min_diff),
                        2 => tz_out(9, 0o24, hour_diff * 10000 + min_diff * 100 + sec_diff),
                        3 => {
                            if sec_diff != 0 {
                                tz_out(9, 0o24, hour_diff * 10000 + min_diff * 100 + sec_diff);
                            } else if min_diff != 0 {
                                tz_out(6, 0o4, hour_diff * 100 + min_diff);
                            } else {
                                tz_out(3, 0, hour_diff);
                            }
                        }
                        _ => bad = true,
                    }
                }
            }
            0 => {
                f -= 1;
                bad = true;
            }
            _ => bad = true,
        }
        if bad {
            let text = format[percent..(f + 1).min(format.len())].to_vec();
            fm.cpy(&text);
        }
        if let Sub::Fmt(subfmt, subwidth) = sub {
            let subpad = if format_char == b'F' || format_char == b'Y' {
                fm.pad
            } else {
                yr_spec
            };
            let mut tmp = Vec::new();
            strftime_internal(
                &mut tmp,
                subfmt,
                tp,
                fm.to_uppcase,
                subpad,
                subwidth,
                ns,
                zone,
            )?;
            fm.to_lowcase = false;
            fm.to_uppcase = false;
            let w = fm.width;
            fm.width_add(w, &tmp);
        }
        f += 1;
        width = -1;
    }
    Ok(())
}

pub fn nstrftime(format: &[u8], tm: &Tm, zone: &Zone, ns: i64) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    strftime_internal(&mut out, format, tm, false, Pad::Zero, -1, ns, zone).ok()?;
    Some(out)
}

pub const SUBRS: &[Subr] = &[
    Subr::fixed("time-add", 2, 2, f_time_add),
    Subr::fixed("time-subtract", 2, 2, f_time_subtract),
    Subr::fixed("time-less-p", 2, 2, f_time_less_p),
    Subr::fixed("time-equal-p", 2, 2, f_time_equal_p),
    Subr::fixed("float-time", 0, 1, f_float_time),
    Subr::fixed("format-time-string", 1, 3, f_format_time_string),
    Subr::fixed("decode-time", 0, 3, f_decode_time),
    Subr::many("encode-time", 1, f_encode_time),
    Subr::fixed("time-convert", 1, 2, f_time_convert),
    Subr::fixed("current-time", 0, 0, f_current_time),
    Subr::fixed("current-cpu-time", 0, 0, f_current_cpu_time),
    Subr::fixed("current-time-string", 0, 2, f_current_time_string),
    Subr::fixed("current-time-zone", 0, 2, f_current_time_zone),
    Subr::fixed("set-time-zone-rule", 1, 1, f_set_time_zone_rule),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_bool(sym::CURRENT_TIME_LIST, false);
    lisp.local_tz = None;
    if let Ok(tz) = std::env::var("TZ") {
        let zone = lisp.string(&tz);
        if let Ok(z) = lisp.tzlookup(zone) {
            lisp.local_tz = Some(z);
        }
    }
}

struct TicksHz {
    ticks: BigInt,
    hz: BigInt,
}

fn floor_div(a: &BigInt, b: &BigInt) -> BigInt {
    a.div_floor(b)
}

fn double_integer_scale(d: f64) -> i64 {
    let bits = d.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i64;
    if biased == 0 {
        1074
    } else {
        52 - (biased - 1023)
    }
}

fn float_to_ticks_hz(d: f64) -> TicksHz {
    if d == 0.0 {
        return TicksHz {
            ticks: BigInt::zero(),
            hz: BigInt::one(),
        };
    }
    let bits = d.to_bits();
    let negative = bits >> 63 != 0;
    let biased = ((bits >> 52) & 0x7ff) as i64;
    let frac = bits & ((1u64 << 52) - 1);
    let (mantissa, exp) = if biased == 0 {
        (frac, -1074)
    } else {
        (frac | (1u64 << 52), biased - 1075)
    };
    let scale = double_integer_scale(d).max(0);
    let mut ticks = BigInt::from(mantissa);
    let shift = exp + scale;
    if shift >= 0 {
        ticks <<= shift as usize;
    } else {
        ticks >>= (-shift) as usize;
    }
    if negative {
        ticks = -ticks;
    }
    TicksHz {
        ticks,
        hz: BigInt::one() << scale as usize,
    }
}

fn frac_to_double(n: &BigInt, d: &BigInt) -> f64 {
    if let (Some(ni), Some(di)) = (n.to_i64(), d.to_i64())
        && di != 0
        && ni % di == 0
    {
        return (ni / di) as f64;
    }
    let ndig = n.bits().max(1) as i64;
    let ddig = d.bits().max(1) as i64;
    let mut scale = ddig - ndig + 53;
    let (nn, dd) = if scale < 0 {
        (n.clone(), d << ((-scale) as usize))
    } else {
        scale = scale.min(1074);
        (n << scale as usize, d.clone())
    };
    let (mut q, r) = nn.div_rem(&dd);
    let incr: i64 = if q.bits() <= 53 {
        let r2 = r.abs() << 1usize;
        let cmp = r2.cmp(&dd.abs());
        i64::from(
            cmp == std::cmp::Ordering::Greater
                || (cmp == std::cmp::Ordering::Equal && (&q % 2u32) != BigInt::zero()),
        )
    } else {
        let lo_2digits = (q.abs() % 4u32).to_i64().unwrap_or(0);
        let lo_digit = lo_2digits % 2;
        if lo_digit > 1 || (lo_digit == 1 && ((lo_2digits / 2) & 1 != 0 || !r.is_zero())) {
            2
        } else {
            0
        }
    };
    if incr != 0 {
        if nn.is_negative() {
            q -= incr;
        } else {
            q += incr;
        }
    }
    let qf = q.to_f64().unwrap_or(f64::INFINITY);
    qf * 2f64.powi(-(scale as i32))
}

impl Lisp {
    fn int_value(&self, v: Value) -> Option<BigInt> {
        if let Some(n) = v.as_fixnum() {
            return Some(BigInt::from(n));
        }
        self.bignum_ref(v).cloned()
    }

    fn current_time_list(&self) -> bool {
        !self.plain_value(sym::CURRENT_TIME_LIST).is_nil()
    }

    fn time_spec_invalid(&mut self) -> Flow {
        self.error("Invalid time specification")
    }

    fn time_overflow(&mut self) -> Flow {
        self.error("Specified time is not representable")
    }

    fn invalid_hz(&mut self, hz: Value) -> Flow {
        let msg = self.string("Invalid time frequency");
        self.signal_list(sym::ERROR, &[msg, hz])
    }

    fn now_ticks_hz(&self) -> TicksHz {
        let ticks = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => BigInt::from(d.as_secs()) * TIMESPEC_HZ + d.subsec_nanos(),
            Err(e) => {
                let d = e.duration();
                -(BigInt::from(d.as_secs()) * TIMESPEC_HZ + d.subsec_nanos())
            }
        };
        TicksHz {
            ticks,
            hz: BigInt::from(TIMESPEC_HZ),
        }
    }

    fn decode_time_components(
        &mut self,
        high: Value,
        low: Value,
        usec: Value,
        psec: Value,
        hz: i64,
    ) -> LispResult<TicksHz> {
        let (Some(mut us), Some(mut ps)) = (usec.as_fixnum(), psec.as_fixnum()) else {
            return Err(self.time_spec_invalid());
        };
        us += ps.div_euclid(1_000_000);
        let s_from_us_ps = us.div_euclid(1_000_000);
        ps = ps.rem_euclid(1_000_000);
        us = us.rem_euclid(1_000_000);
        let (Some(h), Some(l)) = (self.int_value(high), self.int_value(low)) else {
            return Err(self.time_spec_invalid());
        };
        let s = (h << LO_TIME_BITS as usize) + l + s_from_us_ps;
        let ticks = match hz {
            TRILLION => s * TRILLION + us * 1_000_000 + ps,
            1_000_000 => s * 1_000_000 + us,
            _ => s,
        };
        Ok(TicksHz {
            ticks,
            hz: BigInt::from(hz),
        })
    }

    fn decode_lisp_time(&mut self, t: Value, secs_only: bool) -> LispResult<TicksHz> {
        if t.is_nil() {
            return Ok(self.now_ticks_hz());
        }
        if t.is_cons() {
            let high = self.xcar(t);
            let low = self.xcdr(t);
            if low.is_cons() {
                let mut hz = 1i64;
                let mut usec = Value::fixnum(0);
                let mut psec = Value::fixnum(0);
                let mut low_tail = self.xcdr(low);
                let low = self.xcar(low);
                if !secs_only {
                    if low_tail.is_cons() {
                        usec = self.xcar(low_tail);
                        low_tail = self.xcdr(low_tail);
                        if low_tail.is_cons() {
                            psec = self.xcar(low_tail);
                            hz = TRILLION;
                        } else {
                            hz = 1_000_000;
                        }
                    } else if !low_tail.is_nil() {
                        usec = low_tail;
                        hz = 1_000_000;
                    }
                }
                return self.decode_time_components(high, low, usec, psec, hz);
            }
            let ticks = self.int_value(high);
            let hz = self.int_value(low).filter(|h| h.is_positive());
            return match (ticks, hz) {
                (Some(ticks), Some(hz)) => Ok(TicksHz { ticks, hz }),
                _ => Err(self.time_spec_invalid()),
            };
        }
        if let Some(n) = self.int_value(t) {
            return Ok(TicksHz {
                ticks: n,
                hz: BigInt::one(),
            });
        }
        if t.is_float() {
            let d = self.heap.float_value(t);
            if !d.is_finite() {
                return Err(if d.is_nan() {
                    self.time_spec_invalid()
                } else {
                    self.time_overflow()
                });
            }
            return Ok(float_to_ticks_hz(d));
        }
        Err(self.time_spec_invalid())
    }

    fn ticks_hz_to_timespec(&mut self, th: &TicksHz) -> LispResult<(i64, i64)> {
        let total = floor_div(&(&th.ticks * TIMESPEC_HZ), &th.hz);
        let (sec, ns) = total.div_mod_floor(&BigInt::from(TIMESPEC_HZ));
        match (sec.to_i64(), ns.to_i64()) {
            (Some(s), Some(n)) => Ok((s, n)),
            _ => Err(self.time_overflow()),
        }
    }

    fn lisp_time_argument(&mut self, t: Value) -> LispResult<(i64, i64)> {
        let th = self.decode_lisp_time(t, false)?;
        self.ticks_hz_to_timespec(&th)
    }

    fn lisp_seconds_argument(&mut self, t: Value) -> LispResult<i64> {
        let th = self.decode_lisp_time(t, true)?;
        Ok(self.ticks_hz_to_timespec(&th)?.0)
    }

    fn make_int(&mut self, n: BigInt) -> Value {
        self.bignum(n)
    }

    fn ticks_hz_list4(&mut self, ticks: &BigInt, hz: &BigInt) -> Value {
        let fps = floor_div(&(ticks * TRILLION), hz);
        let (q, rem) = fps.div_mod_floor(&BigInt::from(TRILLION));
        let rem = rem.to_i64().unwrap_or(0);
        let us = rem / 1_000_000;
        let ps = rem % 1_000_000;
        let lo = (&q & BigInt::from(0xFFFF)).to_i64().unwrap_or(0);
        let hi = q >> LO_TIME_BITS as usize;
        let hi = self.make_int(hi);
        self.list(&[hi, Value::fixnum(lo), Value::fixnum(us), Value::fixnum(ps)])
    }

    fn make_lisp_time(&mut self, sec: i64, ns: i64) -> Value {
        if self.current_time_list() {
            let hi = self.integer(sec >> LO_TIME_BITS);
            self.list(&[
                hi,
                Value::fixnum(sec & 0xFFFF),
                Value::fixnum(ns / 1000),
                Value::fixnum(ns % 1000 * 1000),
            ])
        } else {
            let ticks = BigInt::from(sec) * TIMESPEC_HZ + ns;
            let ticks = self.make_int(ticks);
            self.cons(ticks, Value::fixnum(TIMESPEC_HZ))
        }
    }

    fn ticks_hz_hz_ticks(&mut self, t: &TicksHz, hz: Value) -> LispResult {
        let Some(target) = self.int_value(hz).filter(|h| h.is_positive()) else {
            return Err(self.invalid_hz(hz));
        };
        if target == t.hz {
            return Ok(self.make_int(t.ticks.clone()));
        }
        let v = floor_div(&(&t.ticks * &target), &t.hz);
        Ok(self.make_int(v))
    }

    fn time_arith(&mut self, a: Value, b: Value, subtract: bool) -> LispResult {
        let ta = self.decode_lisp_time(a, false)?;
        let tb = self.decode_lisp_time(b, false)?;
        let (ticks, hz) = if ta.hz == tb.hz {
            let t = if subtract {
                &ta.ticks - &tb.ticks
            } else {
                &ta.ticks + &tb.ticks
            };
            (t, ta.hz)
        } else {
            let hzmin = ta.hz.clone().min(tb.hz.clone());
            let g = ta.hz.gcd(&tb.hz);
            let fa = &ta.hz / &g;
            let fb = &tb.hz / &g;
            let mut ihz = &fa * &tb.hz;
            let mut iticks = if subtract {
                &fb * &ta.ticks - &fa * &tb.ticks
            } else {
                &fb * &ta.ticks + &fa * &tb.ticks
            };
            let ig = iticks.gcd(&ihz);
            if ig > BigInt::one() {
                iticks /= &ig;
                ihz /= &ig;
                if ihz < hzmin {
                    let rescale = (&hzmin + &ihz - 1u32) / &ihz;
                    iticks *= &rescale;
                    ihz *= &rescale;
                }
            }
            (iticks, ihz)
        };
        if hz.is_one() {
            return Ok(self.make_int(ticks));
        }
        let pair_form = |lisp: &Lisp, v: Value| v.is_cons() && !lisp.xcdr(v).is_cons();
        let trillion_factor = hz.to_i64().is_some_and(|h| TRILLION % h == 0);
        if !self.current_time_list() || pair_form(self, a) || pair_form(self, b) || !trillion_factor
        {
            let t = self.make_int(ticks);
            self.stack.push(t);
            let h = self.make_int(hz);
            self.stack.pop();
            Ok(self.cons(t, h))
        } else {
            Ok(self.ticks_hz_list4(&ticks, &hz))
        }
    }

    fn time_cmp(&mut self, a: Value, b: Value) -> LispResult<std::cmp::Ordering> {
        if a == b {
            return Ok(std::cmp::Ordering::Equal);
        }
        let (mut x, mut y) = (a, b);
        if a.is_cons() && b.is_cons() && self.xcdr(a) == self.xcdr(b) {
            x = self.xcar(a);
            y = self.xcar(b);
        }
        if let (Some(xi), Some(yi)) = (x.as_fixnum(), y.as_fixnum()) {
            return Ok(xi.cmp(&yi));
        }
        let ta = self.decode_lisp_time(a, false)?;
        let tb = self.decode_lisp_time(b, false)?;
        Ok((&ta.ticks * &tb.hz).cmp(&(&tb.ticks * &ta.hz)))
    }

    fn system_zone(&mut self) -> Zone {
        if self.system_tz.is_none() {
            self.system_tz = Some(TimeZone::system());
        }
        Zone::Tz(self.system_tz.clone().unwrap_or(TimeZone::UTC))
    }

    pub fn tzlookup(&mut self, zone: Value) -> LispResult<Zone> {
        if zone.is_nil() {
            return Ok(match &self.local_tz {
                Some(z) => z.clone(),
                None => self.system_zone(),
            });
        }
        if zone == Value::fixnum(0) || zone == sym::T {
            return Ok(Zone::Utc);
        }
        if zone == sym::WALL {
            return Ok(self.system_zone());
        }
        if zone.is_string() {
            let name = self.string_to_rust(zone);
            if name.is_empty() {
                return Ok(Zone::Utc);
            }
            if let Ok(tz) = TimeZone::get(&name) {
                return Ok(Zone::Tz(tz));
            }
            if let Ok(tz) = TimeZone::posix(&name) {
                return Ok(Zone::Tz(tz));
            }
            return Err(self.invalid_zone(zone));
        }
        let plain = zone.is_fixnum();
        let pair = zone.is_cons() && self.xcar(zone).is_fixnum() && self.xcdr(zone).is_cons();
        if !plain && !pair {
            return Err(self.invalid_zone(zone));
        }
        let (offset, abbr_value) = if plain {
            (zone.as_fixnum().unwrap_or(0), Value::NIL)
        } else {
            let a = self.xcar(self.xcdr(zone));
            (self.xcar(zone).as_fixnum().unwrap_or(0), a)
        };
        let abszone = offset.unsigned_abs() as i64;
        let hour = abszone / 3600;
        let hour_remainder = abszone % 3600;
        let min = hour_remainder / 60;
        let sec = hour_remainder % 60;
        let abbr = if plain {
            let mut prec = 2;
            let mut numzone = hour;
            if hour_remainder != 0 {
                prec += 2;
                numzone = 100 * numzone + min;
                if sec != 0 {
                    prec += 2;
                    numzone = 100 * numzone + sec;
                }
            }
            let sign = if offset < 0 { '-' } else { '+' };
            format!("{sign}{numzone:0prec$}")
        } else if abbr_value.is_string() {
            self.string_to_rust(abbr_value)
        } else {
            return Err(self.invalid_zone(zone));
        };
        Ok(Zone::Fixed { offset, abbr })
    }

    fn invalid_zone(&mut self, zone: Value) -> Flow {
        let msg = self.string("Invalid time zone specification");
        self.signal_list(sym::ERROR, &[msg, zone])
    }

    fn localtime_checked(&mut self, zone: &Zone, t: i64) -> LispResult<Tm> {
        match localtime(zone, t) {
            Some(tm) => Ok(tm),
            None => Err(self.time_overflow()),
        }
    }

    fn format_time_string_bytes(
        &mut self,
        format: &[u8],
        sec: i64,
        ns: i64,
        zone: Value,
    ) -> LispResult<(Vec<u8>, Tm)> {
        let tz = self.tzlookup(zone)?;
        let tm = self.localtime_checked(&tz, sec)?;
        match nstrftime(format, &tm, &tz, ns) {
            Some(bytes) => Ok((bytes, tm)),
            None => Err(self.time_overflow()),
        }
    }
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::NIL)
}

fn f_time_add(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.time_arith(args[0], args[1], false)
}

fn f_time_subtract(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args[0] == args[1] {
        return Ok(lisp.make_lisp_time(0, 0));
    }
    lisp.time_arith(args[0], args[1], true)
}

fn f_time_less_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let ord = lisp.time_cmp(args[0], args[1])?;
    Ok(lisp.bool(ord == std::cmp::Ordering::Less))
}

fn f_time_equal_p(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (a, b) = (args[0], args[1]);
    if a.is_nil() != b.is_nil() {
        return Ok(Value::NIL);
    }
    let ord = lisp.time_cmp(a, b)?;
    Ok(lisp.bool(ord == std::cmp::Ordering::Equal))
}

fn f_float_time(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let t = arg(args, 0);
    if t.is_float() {
        return Ok(t);
    }
    let th = lisp.decode_lisp_time(t, false)?;
    Ok(lisp.float(frac_to_double(&th.ticks, &th.hz)))
}

fn f_format_time_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (sec, ns) = lisp.lisp_time_argument(arg(args, 1))?;
    let format = lisp.check_string(args[0])?;
    let bytes = lisp.string_bytes(format).to_vec();
    let multibyte = lisp.string_multibyte(format);
    let (out, _) = lisp.format_time_string_bytes(&bytes, sec, ns, arg(args, 2))?;
    Ok(lisp.make_string_from_bytes(out, multibyte))
}

fn f_decode_time(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (specified, zone, form) = (arg(args, 0), arg(args, 1), arg(args, 2));
    let (time_spec, th) = if form == sym::T {
        let th = lisp.decode_lisp_time(specified, false)?;
        let (s, _) = lisp.ticks_hz_to_timespec(&th)?;
        (s, Some(th))
    } else {
        (lisp.lisp_seconds_argument(specified)?, None)
    };
    let tz = lisp.tzlookup(zone)?;
    let tm = lisp.localtime_checked(&tz, time_spec)?;
    let year = lisp.integer(tm.year);
    let sec = match th {
        Some(th) if !th.hz.is_one() => {
            let ticks = &th.hz * tm.sec + th.ticks.mod_floor(&th.hz);
            let t = lisp.make_int(ticks);
            lisp.stack.push(t);
            let h = lisp.make_int(th.hz);
            lisp.stack.pop();
            lisp.cons(t, h)
        }
        _ => Value::fixnum(tm.sec),
    };
    let dst = if tm.isdst < 0 {
        Value::fixnum(-1)
    } else if tm.isdst == 0 {
        Value::NIL
    } else {
        sym::T
    };
    Ok(lisp.list(&[
        sec,
        Value::fixnum(tm.min),
        Value::fixnum(tm.hour),
        Value::fixnum(tm.mday),
        Value::fixnum(tm.mon + 1),
        year,
        Value::fixnum(tm.wday),
        dst,
        Value::fixnum(tm.gmtoff),
    ]))
}

fn check_tm_member(lisp: &mut Lisp, obj: Value, offset: i64) -> LispResult<i64> {
    let n = lisp.check_fixnum(obj)?;
    let i = n - offset;
    if i32::try_from(i).is_err() {
        return Err(lisp.time_overflow());
    }
    Ok(i)
}

fn f_encode_time(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut zone = Value::NIL;
    let mut isdst = -1i64;
    let (secarg, minarg, hourarg, mdayarg, monarg, yeararg);
    if args.len() == 1 {
        let mut a = args[0];
        let mut tail = a;
        for _ in 0..6 {
            lisp.check_cons(tail)?;
            tail = lisp.xcdr(tail);
        }
        let mut next = |lisp: &mut Lisp| {
            let v = lisp.xcar(a);
            a = lisp.xcdr(a);
            v
        };
        secarg = next(lisp);
        minarg = next(lisp);
        hourarg = next(lisp);
        mdayarg = next(lisp);
        monarg = next(lisp);
        yeararg = next(lisp);
        if !a.is_nil() {
            lisp.check_cons(a)?;
            a = lisp.xcdr(a);
            lisp.check_cons(a)?;
            let dstflag = lisp.xcar(a);
            a = lisp.xcdr(a);
            lisp.check_cons(a)?;
            zone = lisp.xcar(a);
            if dstflag.is_symbol() && !zone.is_fixnum() && !zone.is_cons() {
                isdst = i64::from(!dstflag.is_nil());
            }
        }
    } else if args.len() < 6 {
        return Err(lisp.signal_list(
            sym::WRONG_NUMBER_OF_ARGUMENTS,
            &[sym::ENCODE_TIME, Value::fixnum(args.len() as i64)],
        ));
    } else {
        if args.len() > 6 {
            zone = args[args.len() - 1];
        }
        secarg = args[0];
        minarg = args[1];
        hourarg = args[2];
        mdayarg = args[3];
        monarg = args[4];
        yeararg = args[5];
    }
    let th = lisp.decode_lisp_time(secarg, false)?;
    let (sec, subsecticks) = th.ticks.div_mod_floor(&th.hz);
    let sec_value = lisp.make_int(sec);
    let fields = [
        check_tm_member(lisp, sec_value, 0)?,
        check_tm_member(lisp, minarg, 0)?,
        check_tm_member(lisp, hourarg, 0)?,
        check_tm_member(lisp, mdayarg, 0)?,
        check_tm_member(lisp, monarg, 1)?,
        check_tm_member(lisp, yeararg, TM_YEAR_BASE)? + TM_YEAR_BASE,
    ];
    let tz = lisp.tzlookup(zone)?;
    let Some(value) = mktime(&tz, fields, isdst) else {
        return Err(lisp.time_overflow());
    };
    if th.hz.is_one() {
        if lisp.current_time_list() {
            let hi = lisp.integer(value >> LO_TIME_BITS);
            return Ok(lisp.list(&[hi, Value::fixnum(value & 0xFFFF)]));
        }
        return Ok(lisp.integer(value));
    }
    let ticks = BigInt::from(value) * &th.hz + subsecticks;
    let t = lisp.make_int(ticks);
    lisp.stack.push(t);
    let h = lisp.make_int(th.hz);
    lisp.stack.pop();
    Ok(lisp.cons(t, h))
}

fn f_time_convert(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let time = args[0];
    let t = lisp.decode_lisp_time(time, false)?;
    let mut form = arg(args, 1);
    if form.is_nil() {
        form = if lisp.current_time_list() {
            sym::LIST
        } else {
            sym::T
        };
    }
    if form == sym::LIST {
        return Ok(lisp.ticks_hz_list4(&t.ticks, &t.hz));
    }
    if form == sym::INTEGER {
        if lisp.is_integer(time) {
            return Ok(time);
        }
        return Ok(lisp.make_int(t.ticks.div_floor(&t.hz)));
    }
    if form == sym::T {
        form = lisp.make_int(t.hz.clone());
    }
    if time.is_cons() && lisp.eql(form, lisp.xcdr(time)) {
        return Ok(time);
    }
    lisp.stack.push(form);
    let ticks = lisp.ticks_hz_hz_ticks(&t, form);
    lisp.stack.pop();
    let ticks = ticks?;
    Ok(lisp.cons(ticks, form))
}

pub(crate) fn f_current_time(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let now = lisp.now_ticks_hz();
    let (s, ns) = lisp.ticks_hz_to_timespec(&now)?;
    Ok(lisp.make_lisp_time(s, ns))
}

fn f_current_cpu_time(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let ms = lisp.start_instant.elapsed().as_millis() as i64;
    Ok(lisp.cons(Value::fixnum(ms), Value::fixnum(1000)))
}

fn f_current_time_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let value = lisp.lisp_seconds_argument(arg(args, 0))?;
    let tz = lisp.tzlookup(arg(args, 1))?;
    let tm = lisp.localtime_checked(&tz, value)?;
    let text = format!(
        "{} {}{:3} {:02}:{:02}:{:02} {}",
        &WEEKDAYS[tm.wday as usize][..3],
        &MONTHS[tm.mon as usize][..3],
        tm.mday,
        tm.hour,
        tm.min,
        tm.sec,
        tm.year
    );
    Ok(lisp.unibyte_string(text.as_bytes()))
}

fn f_current_time_zone(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let sec = lisp.lisp_seconds_argument(arg(args, 0))?;
    let (name, tm) = lisp.format_time_string_bytes(b"%Z", sec, 0, arg(args, 1))?;
    let offset = tm.gmtoff;
    let name = if name.is_empty() {
        let hour = offset / 3600;
        let min_sec = offset % 3600;
        let mut s = format!("{}{:02}", if offset < 0 { '-' } else { '+' }, hour.abs());
        if min_sec != 0 {
            let amin_sec = min_sec.abs();
            s.push_str(&format!("{:02}", amin_sec / 60));
            if amin_sec % 60 != 0 {
                s.push_str(&format!("{:02}", amin_sec % 60));
            }
        }
        lisp.string(&s)
    } else {
        lisp.make_string_from_bytes(name, false)
    };
    Ok(lisp.list(&[Value::fixnum(offset), name]))
}

fn f_set_time_zone_rule(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let tz = if args[0].is_nil() { sym::WALL } else { args[0] };
    let zone = lisp.tzlookup(tz)?;
    lisp.local_tz = Some(zone);
    Ok(Value::NIL)
}
