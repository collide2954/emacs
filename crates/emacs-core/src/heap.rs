use crate::object::{Cons, LispString, Symbol, Trace, Vectorlike, Weakness};
use crate::value::{GENERATION_MASK, Tag, Value};

struct Slot<T> {
    generation: u32,
    marked: bool,
    object: Option<T>,
}

pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    pinned: u32,
    live: usize,
}

impl<T: Trace> Arena<T> {
    fn new() -> Self {
        Arena {
            slots: Vec::new(),
            free: Vec::new(),
            pinned: 0,
            live: 0,
        }
    }

    fn alloc(&mut self, object: T) -> (u32, u32) {
        self.live += 1;
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.object = Some(object);
            return (index, slot.generation);
        }
        let index = u32::try_from(self.slots.len()).expect("arena exhausted");
        self.slots.push(Slot {
            generation: 0,
            marked: false,
            object: Some(object),
        });
        (index, 0)
    }

    fn get(&self, value: Value) -> &T {
        let slot = &self.slots[value.index() as usize];
        match &slot.object {
            Some(object) if slot.generation == value.generation() => object,
            _ => panic!("stale {:?} handle", value.tag()),
        }
    }

    fn get_mut(&mut self, value: Value) -> &mut T {
        let slot = &mut self.slots[value.index() as usize];
        match &mut slot.object {
            Some(object) if slot.generation == value.generation() => object,
            _ => panic!("stale {:?} handle", value.tag()),
        }
    }

    fn marked(&self, value: Value) -> Option<bool> {
        let slot = self.slots.get(value.index() as usize)?;
        if slot.object.is_none() || slot.generation != value.generation() {
            return None;
        }
        Some(slot.marked)
    }

    fn contains(&self, value: Value) -> bool {
        self.slots
            .get(value.index() as usize)
            .is_some_and(|slot| slot.object.is_some() && slot.generation == value.generation())
    }

    fn mark(&mut self, value: Value, out: &mut Vec<Value>) {
        let slot = &mut self.slots[value.index() as usize];
        if slot.marked {
            return;
        }
        slot.marked = true;
        match &slot.object {
            Some(object) if slot.generation == value.generation() => object.trace(out),
            _ => panic!("stale {:?} handle reached during marking", value.tag()),
        }
    }

    fn mark_pinned(&mut self, out: &mut Vec<Value>) {
        for slot in &mut self.slots[..self.pinned as usize] {
            if !slot.marked {
                slot.marked = true;
                if let Some(object) = &slot.object {
                    object.trace(out);
                }
            }
        }
    }

    fn sweep(&mut self) -> usize {
        let mut freed = 0;
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.marked {
                slot.marked = false;
            } else if slot.object.is_some() {
                slot.object = None;
                slot.generation = (slot.generation + 1) & GENERATION_MASK;
                self.free.push(index as u32);
                freed += 1;
            }
        }
        self.live -= freed;
        freed
    }

    pub fn live(&self) -> usize {
        self.live
    }

    pub fn pin_all(&mut self) {
        self.pinned = self.slots.len() as u32;
    }
}

pub struct Heap {
    conses: Arena<Cons>,
    floats: Arena<f64>,
    strings: Arena<LispString>,
    symbols: Arena<Symbol>,
    vectors: Arena<Vectorlike>,
    pub bytes_since_gc: usize,
    pub collections: u64,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    pub fn new() -> Self {
        Heap {
            conses: Arena::new(),
            floats: Arena::new(),
            strings: Arena::new(),
            symbols: Arena::new(),
            vectors: Arena::new(),
            bytes_since_gc: 0,
            collections: 0,
        }
    }

    pub fn cons(&mut self, car: Value, cdr: Value) -> Value {
        self.bytes_since_gc += 16;
        let (index, generation) = self.conses.alloc(Cons { car, cdr });
        Value::heap(Tag::Cons, index, generation)
    }

    pub fn car(&self, cell: Value) -> Value {
        self.conses.get(cell).car
    }

    pub fn cdr(&self, cell: Value) -> Value {
        self.conses.get(cell).cdr
    }

    pub fn set_car(&mut self, cell: Value, value: Value) {
        self.conses.get_mut(cell).car = value;
    }

    pub fn set_cdr(&mut self, cell: Value, value: Value) {
        self.conses.get_mut(cell).cdr = value;
    }

    pub fn float(&mut self, value: f64) -> Value {
        self.bytes_since_gc += 8;
        let (index, generation) = self.floats.alloc(value);
        Value::heap(Tag::Float, index, generation)
    }

    pub fn float_value(&self, value: Value) -> f64 {
        *self.floats.get(value)
    }

    pub fn string(&mut self, string: LispString) -> Value {
        self.bytes_since_gc += 32 + string.data.len();
        let (index, generation) = self.strings.alloc(string);
        Value::heap(Tag::String, index, generation)
    }

    pub fn string_ref(&self, value: Value) -> &LispString {
        self.strings.get(value)
    }

    pub fn string_mut(&mut self, value: Value) -> &mut LispString {
        self.strings.get_mut(value)
    }

    pub fn symbol(&mut self, symbol: Symbol) -> Value {
        self.bytes_since_gc += 48;
        let (index, generation) = self.symbols.alloc(symbol);
        Value::heap(Tag::Symbol, index, generation)
    }

    pub fn symbol_ref(&self, value: Value) -> &Symbol {
        self.symbols.get(value)
    }

    pub fn symbol_mut(&mut self, value: Value) -> &mut Symbol {
        self.symbols.get_mut(value)
    }

    pub fn pin_symbols(&mut self) {
        self.symbols.pin_all();
    }

    pub fn vectorlike(&mut self, object: Vectorlike) -> Value {
        self.bytes_since_gc += 16 + object.approximate_size();
        let (index, generation) = self.vectors.alloc(object);
        Value::heap(Tag::Vectorlike, index, generation)
    }

    pub fn vectorlike_ref(&self, value: Value) -> &Vectorlike {
        self.vectors.get(value)
    }

    pub fn vectorlike_mut(&mut self, value: Value) -> &mut Vectorlike {
        self.vectors.get_mut(value)
    }

    pub fn is_live(&self, value: Value) -> bool {
        match value.tag() {
            Tag::Fixnum => true,
            Tag::Cons => self.conses.contains(value),
            Tag::Float => self.floats.contains(value),
            Tag::String => self.strings.contains(value),
            Tag::Symbol => self.symbols.contains(value),
            Tag::Vectorlike => self.vectors.contains(value),
        }
    }

    pub fn live_conses(&self) -> usize {
        self.conses.live()
    }

    fn drain(&mut self, work: &mut Vec<Value>) {
        while let Some(value) = work.pop() {
            match value.tag() {
                Tag::Fixnum => {}
                Tag::Symbol if value.is_nil() => {}
                Tag::Cons => self.conses.mark(value, work),
                Tag::Float => self.floats.mark(value, work),
                Tag::String => self.strings.mark(value, work),
                Tag::Symbol => self.symbols.mark(value, work),
                Tag::Vectorlike => self.vectors.mark(value, work),
            }
        }
    }

    fn is_marked(&self, value: Value) -> bool {
        let marked = |slots_marked: Option<bool>| slots_marked.unwrap_or(true);
        match value.tag() {
            Tag::Fixnum => true,
            Tag::Symbol if value.is_nil() => true,
            Tag::Cons => marked(self.conses.marked(value)),
            Tag::Float => marked(self.floats.marked(value)),
            Tag::String => marked(self.strings.marked(value)),
            Tag::Symbol => marked(self.symbols.marked(value)),
            Tag::Vectorlike => marked(self.vectors.marked(value)),
        }
    }

    fn weak_pass(&mut self, work: &mut Vec<Value>) {
        loop {
            let mut pending = Vec::new();
            for index in 0..self.vectors.slots.len() {
                let slot = &self.vectors.slots[index];
                if !slot.marked {
                    continue;
                }
                if let Some(Vectorlike::HashTable(h)) = &slot.object {
                    if h.weakness == Weakness::None {
                        continue;
                    }
                    for (k, v) in h.entries.iter().flatten() {
                        let (km, vm) = (self.is_marked(*k), self.is_marked(*v));
                        let keep = match h.weakness {
                            Weakness::Key => km,
                            Weakness::Value => vm,
                            Weakness::KeyOrValue => km || vm,
                            Weakness::KeyAndValue => km && vm,
                            Weakness::None => true,
                        };
                        if keep {
                            if !km {
                                pending.push(*k);
                            }
                            if !vm {
                                pending.push(*v);
                            }
                        }
                    }
                }
            }
            if pending.is_empty() {
                break;
            }
            work.extend(pending);
            self.drain(work);
        }
        for index in 0..self.vectors.slots.len() {
            if !self.vectors.slots[index].marked {
                continue;
            }
            let removals: Vec<usize> = match &self.vectors.slots[index].object {
                Some(Vectorlike::HashTable(h)) if h.weakness != Weakness::None => h
                    .entries
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, e)| {
                        let (k, v) = (*e)?;
                        let (km, vm) = (self.is_marked(k), self.is_marked(v));
                        let keep = match h.weakness {
                            Weakness::Key => km,
                            Weakness::Value => vm,
                            Weakness::KeyOrValue => km || vm,
                            Weakness::KeyAndValue => km && vm,
                            Weakness::None => true,
                        };
                        (!keep).then_some(slot)
                    })
                    .collect(),
                Some(Vectorlike::Buffer(b)) => {
                    let dead: Vec<usize> = b
                        .text
                        .markers
                        .iter()
                        .enumerate()
                        .filter(|(_, m)| !self.is_marked(**m))
                        .map(|(i, _)| i)
                        .collect();
                    if !dead.is_empty()
                        && let Some(Vectorlike::Buffer(b)) = &mut self.vectors.slots[index].object
                    {
                        let mut i = 0;
                        b.text.markers.retain(|_| {
                            let keep = !dead.contains(&i);
                            i += 1;
                            keep
                        });
                    }
                    Vec::new()
                }
                _ => Vec::new(),
            };
            if !removals.is_empty()
                && let Some(Vectorlike::HashTable(h)) = &mut self.vectors.slots[index].object
            {
                for slot in removals {
                    h.remove_slot(slot);
                }
            }
        }
    }

    pub fn collect(&mut self, roots: &[Value]) -> usize {
        let mut work: Vec<Value> = roots.to_vec();
        self.symbols.mark_pinned(&mut work);
        self.drain(&mut work);
        self.weak_pass(&mut work);
        let freed = self.conses.sweep()
            + self.floats.sweep()
            + self.strings.sweep()
            + self.symbols.sweep()
            + self.vectors.sweep();
        self.bytes_since_gc = 0;
        self.collections += 1;
        freed
    }
}
