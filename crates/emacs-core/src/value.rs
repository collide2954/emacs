#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Value(u64);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tag {
    Symbol,
    Fixnum,
    String,
    Vectorlike,
    Cons,
    Float,
}

const TAG_BITS: u32 = 3;
const TAG_MASK: u64 = 0b111;
const FIXNUM_MASK: u64 = 0b11;
const FIXNUM_TAG: u64 = 0b10;
const FIXNUM_SHIFT: u32 = 2;
const SYMBOL_TAG: u64 = 0b000;
const CONS_TAG: u64 = 0b011;
const STRING_TAG: u64 = 0b100;
const VECTORLIKE_TAG: u64 = 0b101;
const FLOAT_TAG: u64 = 0b111;
const GENERATION_BITS: u32 = 29;
pub const GENERATION_MASK: u32 = (1 << GENERATION_BITS) - 1;
const INDEX_SHIFT: u32 = 32;

pub const MOST_POSITIVE_FIXNUM: i64 = (1 << 61) - 1;
pub const MOST_NEGATIVE_FIXNUM: i64 = -(1 << 61);

impl Value {
    pub const NIL: Value = Value(0);

    pub const fn from_bits(bits: u64) -> Value {
        Value(bits)
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn is_nil(self) -> bool {
        self.0 == 0
    }

    pub const fn tag(self) -> Tag {
        if self.0 & FIXNUM_MASK == FIXNUM_TAG {
            return Tag::Fixnum;
        }
        match self.0 & TAG_MASK {
            SYMBOL_TAG => Tag::Symbol,
            CONS_TAG => Tag::Cons,
            STRING_TAG => Tag::String,
            VECTORLIKE_TAG => Tag::Vectorlike,
            FLOAT_TAG => Tag::Float,
            _ => panic!("invalid value tag"),
        }
    }

    pub const fn fixnum(n: i64) -> Value {
        assert!(n >= MOST_NEGATIVE_FIXNUM && n <= MOST_POSITIVE_FIXNUM);
        Value(((n << FIXNUM_SHIFT) as u64) | FIXNUM_TAG)
    }

    pub const fn try_fixnum(n: i64) -> Option<Value> {
        if n >= MOST_NEGATIVE_FIXNUM && n <= MOST_POSITIVE_FIXNUM {
            Some(Value::fixnum(n))
        } else {
            None
        }
    }

    pub const fn as_fixnum(self) -> Option<i64> {
        if self.0 & FIXNUM_MASK == FIXNUM_TAG {
            Some((self.0 as i64) >> FIXNUM_SHIFT)
        } else {
            None
        }
    }

    pub const fn is_fixnum(self) -> bool {
        self.0 & FIXNUM_MASK == FIXNUM_TAG
    }

    pub const fn heap(tag: Tag, index: u32, generation: u32) -> Value {
        let tag_bits = match tag {
            Tag::Symbol => SYMBOL_TAG,
            Tag::Cons => CONS_TAG,
            Tag::String => STRING_TAG,
            Tag::Vectorlike => VECTORLIKE_TAG,
            Tag::Float => FLOAT_TAG,
            Tag::Fixnum => panic!("fixnums are immediate"),
        };
        Value(
            ((index as u64) << INDEX_SHIFT)
                | (((generation & GENERATION_MASK) as u64) << TAG_BITS)
                | tag_bits,
        )
    }

    pub const fn index(self) -> u32 {
        (self.0 >> INDEX_SHIFT) as u32
    }

    pub const fn generation(self) -> u32 {
        ((self.0 >> TAG_BITS) as u32) & GENERATION_MASK
    }

    pub const fn is_symbol(self) -> bool {
        self.0 & TAG_MASK == SYMBOL_TAG
    }

    pub const fn is_cons(self) -> bool {
        self.0 & TAG_MASK == CONS_TAG
    }

    pub const fn is_string(self) -> bool {
        self.0 & TAG_MASK == STRING_TAG
    }

    pub const fn is_vectorlike(self) -> bool {
        self.0 & TAG_MASK == VECTORLIKE_TAG
    }

    pub const fn is_float(self) -> bool {
        self.0 & TAG_MASK == FLOAT_TAG
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_fixnum() {
            Some(n) => write!(f, "Fixnum({n})"),
            None => write!(
                f,
                "{:?}({}#{})",
                self.tag(),
                self.index(),
                self.generation()
            ),
        }
    }
}
