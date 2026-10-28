use crate::character;
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::{LispString, Vectorlike};
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("make-string", 2, 3, f_make_string),
    Subr::fixed("make-bool-vector", 2, 2, f_make_bool_vector),
    Subr::many("bool-vector", 0, f_bool_vector),
    Subr::fixed("cons", 2, 2, f_cons),
    Subr::many("list", 0, f_list),
    Subr::fixed("make-list", 2, 2, f_make_list),
    Subr::fixed("make-record", 3, 3, f_make_record),
    Subr::many("record", 1, f_record),
    Subr::fixed("make-vector", 2, 2, f_make_vector),
    Subr::many("vector", 0, f_vector),
    Subr::many("make-byte-code", 4, f_make_byte_code),
    Subr::many("make-closure", 1, f_make_closure),
    Subr::fixed("make-symbol", 1, 1, f_make_symbol),
    Subr::fixed("make-finalizer", 1, 1, f_make_finalizer),
    Subr::fixed("garbage-collect", 0, 0, f_garbage_collect),
    Subr::fixed("garbage-collect-heapsize", 0, 0, f_garbage_collect_heapsize),
    Subr::fixed("garbage-collect-maybe", 1, 1, f_garbage_collect_maybe),
    Subr::fixed("memory-info", 0, 0, f_memory_info),
    Subr::fixed("memory-use-counts", 0, 0, f_memory_use_counts),
    Subr::fixed("malloc-info", 0, 0, f_ignore),
    Subr::fixed("malloc-trim", 0, 1, f_ignore),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_int(sym::GC_CONS_THRESHOLD, 800_000);
    let pct = lisp.float(0.1);
    lisp.defvar(sym::GC_CONS_PERCENTAGE, pct);
    lisp.defvar_int(sym::PURE_BYTES_USED, 0);
    lisp.defvar_int(sym::CONS_CELLS_CONSED, 0);
    lisp.defvar_int(sym::FLOATS_CONSED, 0);
    lisp.defvar_int(sym::VECTOR_CELLS_CONSED, 0);
    lisp.defvar_int(sym::SYMBOLS_CONSED, 0);
    lisp.defvar_int(sym::STRING_CHARS_CONSED, 0);
    lisp.defvar_int(sym::INTERVALS_CONSED, 0);
    lisp.defvar_int(sym::STRINGS_CONSED, 0);
    lisp.defvar(sym::PURIFY_FLAG, Value::NIL);
    lisp.defvar_bool(sym::GARBAGE_COLLECTION_MESSAGES, false);
    lisp.defvar(sym::POST_GC_HOOK, Value::NIL);
    lisp.defvar(sym::MEMORY_SIGNAL_DATA, Value::NIL);
    lisp.defvar(sym::MEMORY_FULL, Value::NIL);
    lisp.defvar_int(sym::GCS_DONE, 0);
    let elapsed = lisp.float(0.0);
    lisp.defvar(sym::GC_ELAPSED, elapsed);
    lisp.defvar_int(sym::INTEGER_WIDTH, 65536);
}

fn f_ignore(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

pub(crate) fn f_make_string(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = lisp.check_fixnat(args[0])? as usize;
    let c = lisp.check_character(args[1])?;
    let multibyte = !args[2].is_nil();
    if c < 0x80 && !multibyte {
        return Ok(lisp.heap.string(LispString::unibyte(vec![c as u8; n])));
    }
    if c < 0x80 {
        return Ok(lisp.heap.string(LispString::multibyte(vec![c as u8; n], n)));
    }
    let one = character::char_string(c);
    let data = one.repeat(n);
    Ok(lisp.heap.string(LispString::multibyte(data, n)))
}

fn f_make_bool_vector(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = lisp.check_fixnat(args[0])? as usize;
    Ok(lisp.make_bool_vector(vec![!args[1].is_nil(); n]))
}

fn f_bool_vector(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let bits = args.iter().map(|v| !v.is_nil()).collect();
    Ok(lisp.make_bool_vector(bits))
}

fn f_cons(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.cons(args[0], args[1]))
}

fn f_list(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.list(args))
}

fn f_make_list(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = lisp.check_fixnat(args[0])? as usize;
    let mut v = Value::NIL;
    for _ in 0..n {
        v = lisp.cons(args[1], v);
    }
    Ok(v)
}

fn f_make_record(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (kind, slots, init) = (args[0], args[1], args[2]);
    let n = lisp.check_fixnat(slots)? as usize;
    let mut items = vec![init; n + 1];
    items[0] = kind;
    Ok(lisp.heap.vectorlike(Vectorlike::Record(items)))
}

fn f_record(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.heap.vectorlike(Vectorlike::Record(args.to_vec())))
}

fn f_make_vector(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = lisp.check_fixnat(args[0])? as usize;
    Ok(lisp.vector(vec![args[1]; n]))
}

fn f_vector(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp.vector(args.to_vec()))
}

fn f_make_byte_code(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (arglist, code, constants, depth) = (args[0], args[1], args[2], args[3]);
    let valid = (arglist.is_fixnum() || arglist.is_cons() || arglist.is_nil())
        && ((code.is_string()
            && !lisp.string_multibyte(code)
            && lisp.is_vector(constants)
            && depth.as_fixnum().is_some_and(|d| d >= 0))
            || (code.is_cons() && (constants.is_cons() || constants.is_nil())));
    if !valid {
        return Err(lisp.error("Invalid byte-code object"));
    }
    Ok(lisp.heap.vectorlike(Vectorlike::Closure(args.to_vec())))
}

fn f_make_closure(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prototype = args[0];
    if !lisp.is_closure(prototype) {
        return Err(lisp.wrong_type(sym::BYTE_CODE_FUNCTION_P, prototype));
    }
    let mut slots = lisp.vector_items(prototype).to_vec();
    let constants = slots[2];
    if !lisp.is_vector(constants) {
        return Err(lisp.wrong_type(sym::VECTORP, constants));
    }
    let mut new_constants = lisp.vector_items(constants).to_vec();
    let n = args.len() - 1;
    if n > new_constants.len() {
        return Err(lisp.args_out_of_range(constants, Value::fixnum(n as i64)));
    }
    new_constants[..n].copy_from_slice(&args[1..]);
    slots[2] = lisp.vector(new_constants);
    Ok(lisp.heap.vectorlike(Vectorlike::Closure(slots)))
}

fn f_make_symbol(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let name = lisp.check_string(args[0])?;
    Ok(lisp.make_symbol(name))
}

fn f_make_finalizer(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(lisp
        .heap
        .vectorlike(Vectorlike::Finalizer { function: args[0] }))
}

fn gc_report(lisp: &mut Lisp) -> Value {
    let conses = lisp.heap.live_conses() as i64;
    let fields = [
        ("conses", 16, conses),
        ("symbols", 48, 0),
        ("strings", 32, 0),
        ("string-bytes", 1, 0),
        ("vectors", 16, 0),
        ("vector-slots", 8, 0),
        ("floats", 8, 0),
        ("intervals", 56, 0),
        ("buffers", 992, 0),
    ];
    let mut entries = Vec::new();
    for (name, size, used) in fields {
        let s = lisp.intern(name);
        entries.push(lisp.list(&[
            s,
            Value::fixnum(size),
            Value::fixnum(used),
            Value::fixnum(0),
        ]));
    }
    lisp.list(&entries)
}

fn f_garbage_collect(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    if lisp.gc_inhibited > 0 {
        return Ok(Value::NIL);
    }
    lisp.garbage_collect();
    let done = lisp.int_var(sym::GCS_DONE) + 1;
    lisp.set_forwarded_int(sym::GCS_DONE, done);
    let hook = sym::POST_GC_HOOK;
    lisp.run_hook(hook)?;
    Ok(gc_report(lisp))
}

fn f_garbage_collect_heapsize(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(gc_report(lisp))
}

fn f_garbage_collect_maybe(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let factor = lisp.check_fixnat(args[0])?;
    if factor > 0 {
        lisp.maybe_gc();
    }
    Ok(Value::NIL)
}

fn f_memory_info(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}

fn f_memory_use_counts(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let conses = Value::fixnum(lisp.heap.live_conses() as i64);
    let zero = Value::fixnum(0);
    Ok(lisp.list(&[conses, zero, zero, zero, zero, zero, zero]))
}
