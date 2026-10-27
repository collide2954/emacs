use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("upcase", 1, 1, f_upcase),
    Subr::fixed("downcase", 1, 1, f_downcase),
    Subr::fixed("capitalize", 1, 1, f_capitalize),
    Subr::fixed("upcase-initials", 1, 1, f_upcase_initials),
];

pub fn upcase_char(c: u32) -> u32 {
    match char::from_u32(c) {
        Some(ch) => {
            let mut up = ch.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => u as u32,
                _ => c,
            }
        }
        None => c,
    }
}

pub fn downcase_char(c: u32) -> u32 {
    match char::from_u32(c) {
        Some(ch) => {
            let mut low = ch.to_lowercase();
            match (low.next(), low.next()) {
                (Some(l), None) => l as u32,
                _ => c,
            }
        }
        None => c,
    }
}

fn is_word_constituent(c: u32) -> bool {
    char::from_u32(c).is_some_and(char::is_alphanumeric)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaseOp {
    Up,
    Down,
    Capitalize,
    UpInitials,
}

impl Lisp {
    fn casify_object(&mut self, op: CaseOp, obj: Value) -> LispResult {
        if let Some(n) = obj.as_fixnum() {
            let flags = n & crate::character::CHAR_MODIFIER_MASK;
            let c = (n & !crate::character::CHAR_MODIFIER_MASK) as u32;
            if c > crate::character::MAX_CHAR {
                return Ok(obj);
            }
            let r = match op {
                CaseOp::Down => downcase_char(c),
                _ => upcase_char(c),
            };
            let r = if c < 0x80 || r < 0x80 || (c >= 0x80 && r >= 0x80) {
                r
            } else {
                c
            };
            return Ok(Value::fixnum(r as i64 | flags));
        }
        if !obj.is_string() {
            return Err(self.wrong_type(sym::CHAR_OR_STRING_P, obj));
        }
        let chars = self.string_char_vec(obj);
        let multibyte = self.string_multibyte(obj);
        let mut out = Vec::with_capacity(chars.len());
        let mut in_word = false;
        for &c in &chars {
            let c = if !multibyte && c >= 0x80 {
                out.push(c);
                in_word = false;
                continue;
            } else {
                c
            };
            let mapped: Vec<u32> = match op {
                CaseOp::Up => special_upcase(c),
                CaseOp::Down => vec![downcase_char(c)],
                CaseOp::Capitalize => {
                    if in_word {
                        vec![downcase_char(c)]
                    } else {
                        titlecase(c)
                    }
                }
                CaseOp::UpInitials => {
                    if in_word {
                        vec![c]
                    } else {
                        titlecase(c)
                    }
                }
            };
            out.extend(mapped);
            in_word = is_word_constituent(c);
        }
        let multibyte = multibyte || out.iter().any(|&c| c >= 0x80);
        Ok(self.string_from_chars(&out, multibyte))
    }
}

fn special_upcase(c: u32) -> Vec<u32> {
    match char::from_u32(c) {
        Some(ch) => ch.to_uppercase().map(|u| u as u32).collect(),
        None => vec![c],
    }
}

fn titlecase(c: u32) -> Vec<u32> {
    match c {
        0x01C4..=0x01C6 => vec![0x01C5],
        0x01C7..=0x01C9 => vec![0x01C8],
        0x01CA..=0x01CC => vec![0x01CB],
        0x01F1..=0x01F3 => vec![0x01F2],
        0x00DF => vec![0x0053, 0x0073],
        _ => special_upcase(c),
    }
}

fn f_upcase(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_object(CaseOp::Up, args[0])
}

fn f_downcase(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_object(CaseOp::Down, args[0])
}

fn f_capitalize(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_object(CaseOp::Capitalize, args[0])
}

fn f_upcase_initials(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.casify_object(CaseOp::UpInitials, args[0])
}
