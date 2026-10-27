use std::collections::HashMap;

use num_bigint::BigInt;

use crate::subr::Subr;
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
    pub props: Vec<PropRun>,
}

#[derive(Clone, Copy, Debug)]
pub struct PropRun {
    pub start: usize,
    pub end: usize,
    pub plist: Value,
}

impl LispString {
    pub fn unibyte(data: Vec<u8>) -> Self {
        let chars = data.len();
        LispString {
            data,
            chars,
            multibyte: false,
            props: Vec::new(),
        }
    }

    pub fn multibyte(data: Vec<u8>, chars: usize) -> Self {
        LispString {
            data,
            chars,
            multibyte: true,
            props: Vec::new(),
        }
    }
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

#[derive(Clone, Copy, Debug)]
pub struct PerBuffer {
    pub always_local: bool,
    pub permanent: bool,
    pub predicate: Value,
}

#[derive(Clone, Debug)]
pub struct Localized {
    pub local_if_set: bool,
    pub default: Value,
    pub per_buffer: Option<PerBuffer>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Forward {
    Int(i64),
    Bool(bool),
    Kboard(usize),
}

#[derive(Clone, Debug)]
pub enum SymbolValue {
    Plain(Value),
    Alias(Value),
    Localized(Box<Localized>),
    Forwarded(Forward),
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

#[derive(Clone, Debug, Default)]
pub struct Obarray {
    pub symbols: HashMap<Vec<u8>, Value>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weakness {
    None,
    Key,
    Value,
    KeyOrValue,
    KeyAndValue,
}

#[derive(Clone, Debug)]
pub struct HashTable {
    pub test: Value,
    pub weakness: Weakness,
    pub entries: Vec<Option<(Value, Value)>>,
    pub hashes: Vec<u64>,
    pub free: Vec<usize>,
    pub index: HashMap<u64, Vec<usize>>,
    pub count: usize,
    pub mutable: bool,
}

impl HashTable {
    pub fn new(test: Value, weakness: Weakness) -> Self {
        HashTable {
            test,
            weakness,
            entries: Vec::new(),
            hashes: Vec::new(),
            free: Vec::new(),
            index: HashMap::new(),
            count: 0,
            mutable: true,
        }
    }

    pub fn candidates(&self, hash: u64) -> &[usize] {
        self.index.get(&hash).map_or(&[], Vec::as_slice)
    }

    pub fn insert_new(&mut self, hash: u64, key: Value, value: Value) {
        let slot = match self.free.pop() {
            Some(slot) => {
                self.entries[slot] = Some((key, value));
                self.hashes[slot] = hash;
                slot
            }
            None => {
                self.entries.push(Some((key, value)));
                self.hashes.push(hash);
                self.entries.len() - 1
            }
        };
        self.index.entry(hash).or_default().push(slot);
        self.count += 1;
    }

    pub fn remove_slot(&mut self, slot: usize) {
        let hash = self.hashes[slot];
        if let Some(chain) = self.index.get_mut(&hash) {
            chain.retain(|&s| s != slot);
            if chain.is_empty() {
                self.index.remove(&hash);
            }
        }
        self.entries[slot] = None;
        self.free.push(slot);
        self.count -= 1;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.hashes.clear();
        self.free.clear();
        self.index.clear();
        self.count = 0;
    }

    pub fn live_entries(&self) -> Vec<(Value, Value)> {
        self.entries.iter().flatten().copied().collect()
    }
}

#[derive(Clone, Debug)]
pub struct BoolVector {
    pub bits: Vec<bool>,
}

#[derive(Clone, Debug)]
pub struct CharTable {
    pub purpose: Value,
    pub default: Value,
    pub parent: Value,
    pub ascii: Value,
    pub contents: Vec<Value>,
    pub extras: Vec<Value>,
}

#[derive(Clone, Debug)]
pub struct SubCharTable {
    pub depth: u8,
    pub min_char: u32,
    pub contents: Vec<Value>,
}

#[derive(Clone, Debug)]
pub enum Vectorlike {
    Vector(Vec<Value>),
    Record(Vec<Value>),
    Closure(Vec<Value>),
    Subr(&'static Subr),
    Bignum(BigInt),
    Obarray(Obarray),
    HashTable(Box<HashTable>),
    BoolVector(BoolVector),
    CharTable(Box<CharTable>),
    SubCharTable(Box<SubCharTable>),
    SymbolWithPos { symbol: Value, position: Value },
    Finalizer { function: Value },
    Buffer(Box<crate::buffer::Buffer>),
    Marker(crate::buffer::Marker),
    Overlay(Box<crate::buffer::Overlay>),
    Terminal(Box<crate::terminal::Terminal>),
    Frame(Box<crate::frame::Frame>),
    Window(Box<crate::window::Window>),
}

impl Vectorlike {
    pub fn approximate_size(&self) -> usize {
        match self {
            Vectorlike::Vector(v) | Vectorlike::Record(v) | Vectorlike::Closure(v) => 8 * v.len(),
            Vectorlike::HashTable(h) => 24 * h.entries.len(),
            Vectorlike::BoolVector(b) => b.bits.len() / 8,
            Vectorlike::CharTable(t) => 8 * (t.contents.len() + t.extras.len()),
            Vectorlike::SubCharTable(t) => 8 * t.contents.len(),
            Vectorlike::Obarray(o) => 32 * o.symbols.len(),
            _ => 16,
        }
    }
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
    fn trace(&self, out: &mut Vec<Value>) {
        out.extend(self.props.iter().map(|run| run.plist));
    }
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
            SymbolValue::Localized(l) => {
                out.push(l.default);
                if let Some(p) = l.per_buffer {
                    out.push(p.predicate);
                }
            }
            SymbolValue::Forwarded(_) => {}
        }
    }
}

impl Trace for Vectorlike {
    fn trace(&self, out: &mut Vec<Value>) {
        match self {
            Vectorlike::Vector(items) | Vectorlike::Record(items) | Vectorlike::Closure(items) => {
                out.extend_from_slice(items)
            }
            Vectorlike::Subr(_) | Vectorlike::Bignum(_) | Vectorlike::BoolVector(_) => {}
            Vectorlike::Obarray(o) => out.extend(o.symbols.values().copied()),
            Vectorlike::HashTable(h) => {
                out.push(h.test);
                if h.weakness == Weakness::None {
                    for (k, v) in h.entries.iter().flatten() {
                        out.push(*k);
                        out.push(*v);
                    }
                }
            }
            Vectorlike::CharTable(t) => {
                out.push(t.purpose);
                out.push(t.default);
                out.push(t.parent);
                out.push(t.ascii);
                out.extend_from_slice(&t.contents);
                out.extend_from_slice(&t.extras);
            }
            Vectorlike::SubCharTable(t) => out.extend_from_slice(&t.contents),
            Vectorlike::SymbolWithPos { symbol, position } => {
                out.push(*symbol);
                out.push(*position);
            }
            Vectorlike::Finalizer { function } => out.push(*function),
            Vectorlike::Buffer(b) => b.trace(out),
            Vectorlike::Marker(m) => out.push(m.buffer),
            Vectorlike::Overlay(o) => {
                out.push(o.buffer);
                out.push(o.plist);
            }
            Vectorlike::Terminal(t) => t.trace(out),
            Vectorlike::Frame(f) => f.trace(out),
            Vectorlike::Window(w) => w.trace(out),
        }
    }
}
