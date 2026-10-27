use crate::value::Value;

#[derive(Clone, Copy, Debug)]
pub struct Cons {
    pub car: Value,
    pub cdr: Value,
}

#[derive(Clone, Debug, Default)]
pub struct LispString {
    pub data: Vec<u8>,
    pub chars: usize,
    pub multibyte: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Interned {
    Uninterned,
    Interned,
    InitialObarray,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrappedWrite {
    Untrapped,
    NoWrite,
    Trapped,
}

#[derive(Clone, Debug)]
pub enum SymbolValue {
    Plain(Value),
    Alias(Value),
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: Value,
    pub value: SymbolValue,
    pub function: Value,
    pub plist: Value,
    pub interned: Interned,
    pub special: bool,
    pub trapped_write: TrappedWrite,
}

#[derive(Clone, Debug)]
pub enum Vectorlike {
    Vector(Vec<Value>),
    Record(Vec<Value>),
    Closure(Vec<Value>),
}

pub trait Trace {
    fn trace(&self, out: &mut Vec<Value>);
}

impl Trace for Cons {
    fn trace(&self, out: &mut Vec<Value>) {
        out.push(self.car);
        out.push(self.cdr);
    }
}

impl Trace for LispString {
    fn trace(&self, _out: &mut Vec<Value>) {}
}

impl Trace for f64 {
    fn trace(&self, _out: &mut Vec<Value>) {}
}

impl Trace for Symbol {
    fn trace(&self, out: &mut Vec<Value>) {
        out.push(self.name);
        out.push(self.function);
        out.push(self.plist);
        match &self.value {
            SymbolValue::Plain(v) | SymbolValue::Alias(v) => out.push(*v),
        }
    }
}

impl Trace for Vectorlike {
    fn trace(&self, out: &mut Vec<Value>) {
        match self {
            Vectorlike::Vector(items) | Vectorlike::Record(items) | Vectorlike::Closure(items) => {
                out.extend_from_slice(items)
            }
        }
    }
}
