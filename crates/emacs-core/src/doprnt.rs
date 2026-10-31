use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;

const SIZE_BOUND_EXTRA: i64 = f64::MAX_10_EXP as i64 + 50;

#[derive(Clone, Copy, Debug)]
pub enum Arg<'a> {
    Str(&'a [u8]),
    Int(i64),
    Uint(u64),
    Float(f64),
}

impl Arg<'_> {
    fn int(self) -> i64 {
        match self {
            Arg::Int(v) => v,
            Arg::Uint(v) => v as i64,
            _ => unreachable!("doprnt: integer argument expected"),
        }
    }

    fn uint(self) -> u64 {
        match self {
            Arg::Uint(v) => v,
            Arg::Int(v) => v as u64,
            _ => unreachable!("doprnt: unsigned argument expected"),
        }
    }

    fn float(self) -> f64 {
        match self {
            Arg::Float(v) => v,
            _ => unreachable!("doprnt: floating-point argument expected"),
        }
    }
}

struct Spec {
    minusflag: bool,
    plusflag: bool,
    zeroflag: bool,
    spaceflag: bool,
    wid: usize,
    prec: i64,
}

impl Spec {
    fn pad(&self, sign: &str, digits: &str, zero_pad: bool) -> Vec<u8> {
        let len = sign.len() + digits.len();
        let fill = self.wid.saturating_sub(len);
        let text = if fill == 0 {
            format!("{sign}{digits}")
        } else if self.minusflag {
            format!("{sign}{digits}{}", " ".repeat(fill))
        } else if self.zeroflag && zero_pad {
            format!("{sign}{}{digits}", "0".repeat(fill))
        } else {
            format!("{}{sign}{digits}", " ".repeat(fill))
        };
        text.into_bytes()
    }

    fn integer(&self, negative: bool, magnitude: u64, conversion: u8) -> Vec<u8> {
        let mut digits = match conversion {
            b'o' => format!("{magnitude:o}"),
            b'x' => format!("{magnitude:x}"),
            b'X' => format!("{magnitude:X}"),
            _ => magnitude.to_string(),
        };
        if self.prec == 0 && magnitude == 0 {
            digits.clear();
        }
        let prec = usize::try_from(self.prec).unwrap_or(0);
        if digits.len() < prec {
            digits = format!("{}{digits}", "0".repeat(prec - digits.len()));
        }
        let sign = if conversion != b'd' {
            ""
        } else if negative {
            "-"
        } else if self.plusflag {
            "+"
        } else if self.spaceflag {
            " "
        } else {
            ""
        };
        self.pad(sign, &digits, self.prec < 0)
    }

    fn float(&self, d: f64, conversion: u8) -> Vec<u8> {
        let precision = usize::try_from(self.prec).ok();
        let text = crate::editfns::format_float(
            d,
            conversion as char,
            precision,
            false,
            self.plusflag,
            self.spaceflag,
        );
        let split = usize::from(text.starts_with(['-', '+', ' ']));
        let (sign, digits) = text.split_at(split);
        self.pad(sign, digits, d.is_finite())
    }
}

impl Lisp {
    fn parse_format_integer(&mut self, fmt: &[u8], mut i: usize) -> LispResult<(i64, usize)> {
        let mut n: i32 = 0;
        let mut overflow = false;
        while let Some(&c @ b'0'..=b'9') = fmt.get(i) {
            let (m, o1) = n.overflowing_mul(10);
            let (a, o2) = m.overflowing_add(i32::from(c - b'0'));
            overflow |= o1 | o2;
            n = a;
            i += 1;
        }
        if overflow || (isize::MAX as i64) - SIZE_BOUND_EXTRA < i64::from(n) {
            return Err(self.error("Format width or precision too large"));
        }
        Ok((i64::from(n), i))
    }

    pub fn strwidth(&self, string: &[u8]) -> usize {
        let dp = self.buffer_display_table();
        let mut width = 0;
        let mut i_byte = 0;
        while i_byte < string.len() {
            let (c, bytes) = character::string_char(&string[i_byte..]);
            width += self.char_width(c, dp);
            i_byte += bytes;
        }
        width
    }

    pub fn doprnt(&mut self, format: &str, args: &[Arg]) -> LispResult<Vec<u8>> {
        let end = format.find('\0').unwrap_or(format.len());
        let format = &format[..end];
        let fmt = format.as_bytes();
        let mut buffer = Vec::new();
        let mut args = args.iter().copied();
        let quoting_style = self.text_quoting_style();
        let mut i = 0;
        while i < fmt.len() {
            let fmt0 = i;
            let fmtchar = fmt[i];
            i += 1;
            if fmtchar == b'%' {
                let mut spec = Spec {
                    minusflag: false,
                    plusflag: false,
                    zeroflag: false,
                    spaceflag: false,
                    wid: 0,
                    prec: -1,
                };
                loop {
                    match fmt.get(i) {
                        Some(b'-') => spec.minusflag = true,
                        Some(b'+') => spec.plusflag = true,
                        Some(b' ') => spec.spaceflag = true,
                        Some(b'0') => spec.zeroflag = true,
                        _ => break,
                    }
                    i += 1;
                }
                if let Some(b'1'..=b'9') = fmt.get(i) {
                    let (wid, next) = self.parse_format_integer(fmt, i)?;
                    spec.wid = wid as usize;
                    i = next;
                }
                if fmt.get(i) == Some(&b'.') {
                    let (prec, next) = self.parse_format_integer(fmt, i + 1)?;
                    spec.prec = prec;
                    i = next;
                }
                if fmt[i..].starts_with(b"ll") {
                    i += 2;
                } else if let Some(b'l' | b't') = fmt.get(i) {
                    i += 1;
                }
                let conversion = fmt.get(i).copied();
                i += 1;
                if conversion != Some(b'%') {
                    let field = if spec.minusflag {
                        -(spec.wid as i64)
                    } else {
                        spec.wid as i64
                    };
                    let (string, mut minlen) = match conversion {
                        Some(b'd') => {
                            let v = args.next().map_or(0, Arg::int);
                            (spec.integer(v < 0, v.unsigned_abs(), b'd'), 0)
                        }
                        Some(c @ (b'o' | b'u' | b'x' | b'X')) => {
                            let v = args.next().map_or(0, Arg::uint);
                            (spec.integer(false, v, c), 0)
                        }
                        Some(c @ (b'f' | b'e' | b'g')) => {
                            let d = args.next().map_or(0.0, Arg::float);
                            (spec.float(d, c), 0)
                        }
                        Some(b'S' | b's') => {
                            let s = match args.next() {
                                Some(Arg::Str(s)) => s,
                                Some(_) => unreachable!("doprnt: string argument expected"),
                                None => b"",
                            };
                            let tem = s.iter().position(|&b| b == 0).unwrap_or(s.len());
                            (s[..tem].to_vec(), field)
                        }
                        Some(b'c') => {
                            let chr = args.next().map_or(0, Arg::int);
                            (character::char_string(chr as u32), field)
                        }
                        _ => {
                            let rest = fmt[fmt0..].to_vec();
                            return Err(
                                self.verror("Invalid format operation %s", &[Arg::Str(&rest)])
                            );
                        }
                    };
                    let width = if minlen == 0 {
                        0
                    } else {
                        self.strwidth(&string) as i64
                    };
                    while minlen > width {
                        buffer.push(b' ');
                        minlen -= 1;
                    }
                    buffer.extend_from_slice(&string);
                    while minlen < -width {
                        buffer.push(b' ');
                        minlen += 1;
                    }
                    continue;
                }
            }
            if quoting_style == sym::CURVE && fmtchar == b'`' {
                buffer.extend_from_slice("\u{2018}".as_bytes());
            } else if quoting_style == sym::CURVE && fmtchar == b'\'' {
                buffer.extend_from_slice("\u{2019}".as_bytes());
            } else if fmtchar < 0x80 {
                if quoting_style == sym::STRAIGHT && fmtchar == b'`' {
                    buffer.push(b'\'');
                } else {
                    buffer.push(fmtchar);
                }
            } else {
                let srclen = format[fmt0..].chars().next().map_or(1, char::len_utf8);
                buffer.extend_from_slice(&fmt[fmt0..fmt0 + srclen]);
                i = fmt0 + srclen;
            }
        }
        Ok(buffer)
    }
}
