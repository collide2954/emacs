use emacs_core::heap::Heap;
use emacs_core::value::Value;

fn list(heap: &mut Heap, items: &[i64]) -> Value {
    items
        .iter()
        .rev()
        .fold(Value::NIL, |tail, &n| heap.cons(Value::fixnum(n), tail))
}

#[test]
fn cons_cells_hold_car_and_cdr() {
    let mut heap = Heap::new();
    let cell = heap.cons(Value::fixnum(1), Value::fixnum(2));
    assert!(cell.is_cons());
    assert_eq!(heap.car(cell), Value::fixnum(1));
    assert_eq!(heap.cdr(cell), Value::fixnum(2));
    heap.set_car(cell, Value::fixnum(3));
    heap.set_cdr(cell, Value::NIL);
    assert_eq!(heap.car(cell), Value::fixnum(3));
    assert_eq!(heap.cdr(cell), Value::NIL);
}

#[test]
fn floats_are_boxed_and_distinct() {
    let mut heap = Heap::new();
    let a = heap.float(1.5);
    let b = heap.float(1.5);
    assert!(a.is_float());
    assert_ne!(a, b);
    assert_eq!(heap.float_value(a), 1.5);
}

#[test]
fn collection_keeps_rooted_structure() {
    let mut heap = Heap::new();
    let kept = list(&mut heap, &[1, 2, 3]);
    let _garbage = list(&mut heap, &[4, 5]);
    let freed = heap.collect(&[kept]);
    assert_eq!(freed, 2);
    assert_eq!(heap.live_conses(), 3);
    assert_eq!(heap.car(heap.cdr(kept)), Value::fixnum(2));
}

#[test]
fn long_lists_are_marked_without_recursion() {
    let mut heap = Heap::new();
    let items: Vec<i64> = (0..1_000_000).collect();
    let long = list(&mut heap, &items);
    assert_eq!(heap.collect(&[long]), 0);
}

#[test]
fn cyclic_structure_is_collected() {
    let mut heap = Heap::new();
    let a = heap.cons(Value::NIL, Value::NIL);
    let b = heap.cons(a, a);
    heap.set_cdr(a, b);
    assert_eq!(heap.collect(&[]), 2);
}

#[test]
#[should_panic(expected = "stale")]
fn stale_handles_are_detected() {
    let mut heap = Heap::new();
    let dead = heap.cons(Value::NIL, Value::NIL);
    heap.collect(&[]);
    let _reused = heap.cons(Value::fixnum(9), Value::NIL);
    heap.car(dead);
}

#[test]
fn freed_slots_are_reused_with_a_new_generation() {
    let mut heap = Heap::new();
    let dead = heap.cons(Value::NIL, Value::NIL);
    heap.collect(&[]);
    let fresh = heap.cons(Value::NIL, Value::NIL);
    assert_eq!(fresh.index(), dead.index());
    assert_ne!(fresh.generation(), dead.generation());
    assert_ne!(fresh, dead);
}
