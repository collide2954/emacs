use emacs_core::value::{MOST_NEGATIVE_FIXNUM, MOST_POSITIVE_FIXNUM, Tag, Value};

#[test]
fn nil_is_the_zero_word() {
    assert_eq!(Value::NIL.bits(), 0);
    assert!(Value::NIL.is_nil());
    assert_eq!(Value::NIL.tag(), Tag::Symbol);
}

#[test]
fn fixnum_range_matches_64_bit_emacs() {
    assert_eq!(MOST_POSITIVE_FIXNUM, (1 << 61) - 1);
    assert_eq!(MOST_NEGATIVE_FIXNUM, -(1 << 61));
}

#[test]
fn fixnums_round_trip() {
    for n in [
        0,
        1,
        -1,
        42,
        -1000,
        MOST_POSITIVE_FIXNUM,
        MOST_NEGATIVE_FIXNUM,
    ] {
        let v = Value::fixnum(n);
        assert_eq!(v.tag(), Tag::Fixnum);
        assert_eq!(v.as_fixnum(), Some(n));
    }
}

#[test]
fn fixnum_overflow_is_rejected() {
    assert!(Value::try_fixnum(MOST_POSITIVE_FIXNUM + 1).is_none());
    assert!(Value::try_fixnum(MOST_NEGATIVE_FIXNUM - 1).is_none());
}

#[test]
fn heap_handles_keep_index_generation_and_tag() {
    let v = Value::heap(Tag::Cons, 7, 3);
    assert_eq!(v.tag(), Tag::Cons);
    assert_eq!(v.index(), 7);
    assert_eq!(v.generation(), 3);
    assert_eq!(v.as_fixnum(), None);
}

#[test]
fn every_tag_survives_encoding() {
    for tag in [
        Tag::Symbol,
        Tag::String,
        Tag::Vectorlike,
        Tag::Cons,
        Tag::Float,
    ] {
        assert_eq!(Value::heap(tag, 1, 1).tag(), tag);
    }
}
