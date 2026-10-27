use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::value::Value;

pub type NormalFn = fn(&mut Lisp, &[Value]) -> LispResult;
pub type SpecialFn = fn(&mut Lisp, Value) -> LispResult;

#[derive(Clone, Copy)]
pub enum SubrKind {
    Fixed(u16, NormalFn),
    Many(NormalFn),
    Special(SpecialFn),
}

pub struct Subr {
    pub name: &'static str,
    pub min_args: u16,
    pub kind: SubrKind,
    pub interactive: Option<&'static str>,
}

impl Subr {
    pub const fn fixed(name: &'static str, min_args: u16, max_args: u16, f: NormalFn) -> Subr {
        Subr {
            name,
            min_args,
            kind: SubrKind::Fixed(max_args, f),
            interactive: None,
        }
    }

    pub const fn many(name: &'static str, min_args: u16, f: NormalFn) -> Subr {
        Subr {
            name,
            min_args,
            kind: SubrKind::Many(f),
            interactive: None,
        }
    }

    pub const fn special(name: &'static str, min_args: u16, f: SpecialFn) -> Subr {
        Subr {
            name,
            min_args,
            kind: SubrKind::Special(f),
            interactive: None,
        }
    }

    pub const fn interactive(self, spec: &'static str) -> Subr {
        Subr {
            interactive: Some(spec),
            ..self
        }
    }

    pub fn max_args(&self) -> Option<u16> {
        match self.kind {
            SubrKind::Fixed(max, _) => Some(max),
            SubrKind::Many(_) | SubrKind::Special(_) => None,
        }
    }

    pub fn is_special_form(&self) -> bool {
        matches!(self.kind, SubrKind::Special(_))
    }
}

impl std::fmt::Debug for Subr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#<subr {}>", self.name)
    }
}
