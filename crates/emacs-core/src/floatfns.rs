use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::data::{Num, num_bigint_from_f64};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("acos", 1, 1, f_acos),
    Subr::fixed("asin", 1, 1, f_asin),
    Subr::fixed("atan", 1, 2, f_atan),
    Subr::fixed("cos", 1, 1, f_cos),
    Subr::fixed("sin", 1, 1, f_sin),
    Subr::fixed("tan", 1, 1, f_tan),
    Subr::fixed("isnan", 1, 1, f_isnan),
    Subr::fixed("copysign", 2, 2, f_copysign),
    Subr::fixed("frexp", 1, 1, f_frexp),
    Subr::fixed("ldexp", 2, 2, f_ldexp),
    Subr::fixed("exp", 1, 1, f_exp),
    Subr::fixed("expt", 2, 2, f_expt),
    Subr::fixed("log", 1, 2, f_log),
    Subr::fixed("sqrt", 1, 1, f_sqrt),
    Subr::fixed("abs", 1, 1, f_abs),
    Subr::fixed("float", 1, 1, f_float),
    Subr::fixed("logb", 1, 1, f_logb),
    Subr::fixed("ceiling", 1, 2, f_ceiling),
    Subr::fixed("floor", 1, 2, f_floor),
    Subr::fixed("round", 1, 2, f_round),
    Subr::fixed("truncate", 1, 2, f_truncate),
    Subr::fixed("fceiling", 1, 1, f_fceiling),
    Subr::fixed("ffloor", 1, 1, f_ffloor),
    Subr::fixed("fround", 1, 1, f_fround),
    Subr::fixed("ftruncate", 1, 1, f_ftruncate),
];

pub fn init_vars(_lisp: &mut Lisp) {}

pub(crate) fn float_arg(lisp: &mut Lisp, v: Value) -> LispResult<f64> {
    if v.is_float() {
        return Ok(lisp.heap.float_value(v));
    }
    if v.is_fixnum() || lisp.is_bignum(v) {
        return Ok(lisp.number(v)?.to_f64());
    }
    Err(lisp.wrong_type(sym::NUMBERP, v))
}

fn unary(lisp: &mut Lisp, v: Value, f: fn(f64) -> f64) -> LispResult {
    let x = float_arg(lisp, v)?;
    Ok(lisp.float(f(x)))
}

fn f_acos(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    unary(lisp, args[0], f64::acos)
}

fn f_asin(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    unary(lisp, args[0], f64::asin)
}

fn f_atan(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let y = float_arg(lisp, args[0])?;
    if args[1].is_nil() {
        return Ok(lisp.float(y.atan()));
    }
    let x = float_arg(lisp, args[1])?;
    Ok(lisp.float(y.atan2(x)))
}

fn f_cos(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    unary(lisp, args[0], f64::cos)
}

fn f_sin(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    unary(lisp, args[0], f64::sin)
}

fn f_tan(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    unary(lisp, args[0], f64::tan)
}

fn f_isnan(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if !args[0].is_float() {
        return Err(lisp.wrong_type(sym::FLOATP, args[0]));
    }
    let x = lisp.heap.float_value(args[0]);
    Ok(lisp.bool(x.is_nan()))
}

fn f_copysign(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    for &a in &args[..2] {
        if !a.is_float() {
            return Err(lisp.wrong_type(sym::FLOATP, a));
        }
    }
    let x = lisp.heap.float_value(args[0]);
    let y = lisp.heap.float_value(args[1]);
    Ok(lisp.float(x.copysign(y)))
}

fn f_frexp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let x = float_arg(lisp, args[0])?;
    let (m, e) = if x == 0.0 || x.is_nan() || x.is_infinite() {
        (x, 0)
    } else {
        let bits = x.to_bits();
        let exp = ((bits >> 52) & 0x7FF) as i64;
        if exp == 0 {
            let scaled = x * 2f64.powi(54);
            let bits = scaled.to_bits();
            let exp = ((bits >> 52) & 0x7FF) as i64 - 1022 - 54;
            let m = f64::from_bits((bits & !(0x7FF << 52)) | (1022 << 52));
            (m, exp)
        } else {
            let m = f64::from_bits((bits & !(0x7FF << 52)) | (1022 << 52));
            (m, exp - 1022)
        }
    };
    let mv = lisp.float(m);
    Ok(lisp.cons(mv, Value::fixnum(e)))
}

fn f_ldexp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let m = float_arg(lisp, args[0])?;
    let e = lisp.check_fixnum(args[1])?;
    let e = e.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    Ok(lisp.float(m * 2f64.powi(e)))
}

fn f_exp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    unary(lisp, args[0], f64::exp)
}

fn f_expt(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (x, y) = (args[0], args[1]);
    if lisp.is_integer(x) && lisp.is_integer(y) {
        let ny = lisp.check_integer(y)?;
        let nonneg = match &ny {
            Num::Fix(n) => *n >= 0,
            Num::Big(b) => !b.is_negative(),
            Num::Float(_) => false,
        };
        if nonneg {
            let base = lisp.check_integer(x)?.to_bigint().unwrap_or_default();
            let exp = ny.to_bigint().unwrap_or_default();
            if base == BigInt::from(1) {
                return Ok(x);
            }
            if base.is_zero() {
                return Ok(if exp.is_zero() { Value::fixnum(1) } else { x });
            }
            if base == BigInt::from(-1) {
                return Ok(if exp.is_odd() { x } else { Value::fixnum(1) });
            }
            let Some(e) = exp.to_u32() else {
                return Err(lisp.signal(sym::OVERFLOW_ERROR, Value::NIL));
            };
            let width = lisp.int_var(sym::INTEGER_WIDTH).max(0) as u64;
            if base.bits() * e as u64 > width + 1 {
                return Err(lisp.signal(sym::OVERFLOW_ERROR, Value::NIL));
            }
            return Ok(lisp.bignum(num_traits::pow(base, e as usize)));
        }
    }
    let fx = float_arg(lisp, x)?;
    let fy = float_arg(lisp, y)?;
    Ok(lisp.float(fx.powf(fy)))
}

fn f_log(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let d = float_arg(lisp, args[0])?;
    if args[1].is_nil() {
        return Ok(lisp.float(d.ln()));
    }
    let b = float_arg(lisp, args[1])?;
    let r = if b == 10.0 {
        d.log10()
    } else if b == 2.0 {
        d.log2()
    } else {
        d.ln() / b.ln()
    };
    Ok(lisp.float(r))
}

fn f_sqrt(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    unary(lisp, args[0], f64::sqrt)
}

fn f_abs(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let n = lisp.number(args[0])?;
    match n {
        Num::Fix(i) => Ok(match i.checked_abs() {
            Some(a) => Value::fixnum(a),
            None => lisp.bignum(BigInt::from(i).abs()),
        }),
        Num::Big(b) => Ok(lisp.bignum(b.abs())),
        Num::Float(f) => Ok(lisp.float(f.abs())),
    }
}

fn f_float(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    if args[0].is_float() {
        return Ok(args[0]);
    }
    let x = float_arg(lisp, args[0])?;
    Ok(lisp.float(x))
}

fn f_logb(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let v = args[0];
    if v.is_float() {
        let f = lisp.heap.float_value(v);
        if f == 0.0 {
            return Ok(lisp.float(f64::NEG_INFINITY));
        }
        if !f.is_finite() {
            return Ok(lisp.float(f.abs()));
        }
        return Ok(Value::fixnum(f.abs().log2().floor() as i64));
    }
    let n = lisp.check_integer(v)?;
    let b = n.to_bigint().unwrap_or_default();
    if b.is_zero() {
        return Ok(lisp.float(f64::NEG_INFINITY));
    }
    Ok(Value::fixnum(b.abs().bits() as i64 - 1))
}

#[derive(Clone, Copy)]
enum Rounding {
    Ceiling,
    Floor,
    Round,
    Truncate,
}

fn round_float(f: f64, mode: Rounding) -> f64 {
    match mode {
        Rounding::Ceiling => f.ceil(),
        Rounding::Floor => f.floor(),
        Rounding::Truncate => f.trunc(),
        Rounding::Round => {
            let r = f.round();
            if (f - f.trunc()).abs() == 0.5 && r % 2.0 != 0.0 {
                r - f.signum()
            } else {
                r
            }
        }
    }
}

fn round_bigints(n: &BigInt, d: &BigInt, mode: Rounding) -> BigInt {
    match mode {
        Rounding::Floor => n.div_floor(d),
        Rounding::Ceiling => -((-n).div_floor(d)),
        Rounding::Truncate => n / d,
        Rounding::Round => {
            let (q, r) = n.div_mod_floor(d);
            let twice: BigInt = &r * 2;
            let dabs = d.abs();
            let r2 = if d.is_negative() { -twice } else { twice };
            match r2.cmp(&dabs) {
                std::cmp::Ordering::Less => q,
                std::cmp::Ordering::Greater => q + 1,
                std::cmp::Ordering::Equal => {
                    if q.is_odd() {
                        q + 1
                    } else {
                        q
                    }
                }
            }
        }
    }
}

fn rounding_driver(lisp: &mut Lisp, args: &[Value], mode: Rounding, name: &str) -> LispResult {
    let (arg, divisor) = (args[0], args[1]);
    let a = lisp.number(arg)?;
    if divisor.is_nil() {
        return match a {
            Num::Fix(_) | Num::Big(_) => Ok(arg),
            Num::Float(f) => float_to_integer(lisp, round_float(f, mode), name, arg),
        };
    }
    let d = lisp.number(divisor)?;
    match (&a, &d) {
        (Num::Float(_), _) | (_, Num::Float(_)) => {
            let fd = d.to_f64();
            if fd == 0.0 && !matches!(a, Num::Float(_)) && !matches!(d, Num::Float(_)) {
                return Err(lisp.signal(sym::ARITH_ERROR, Value::NIL));
            }
            let fa = a.to_f64();
            let q = round_float(fa / fd, mode);
            float_to_integer(lisp, q, name, arg)
        }
        _ => {
            let n = a.to_bigint().unwrap_or_default();
            let dd = d.to_bigint().unwrap_or_default();
            if dd.is_zero() {
                return Err(lisp.signal(sym::ARITH_ERROR, Value::NIL));
            }
            Ok(lisp.bignum(round_bigints(&n, &dd, mode)))
        }
    }
}

fn float_to_integer(lisp: &mut Lisp, f: f64, name: &str, arg: Value) -> LispResult {
    if !f.is_finite() {
        let s = lisp.intern(name);
        return Err(lisp.signal_list(sym::OVERFLOW_ERROR, &[s, arg]));
    }
    if f.abs() < 4.0e18 {
        return Ok(lisp.integer(f as i64));
    }
    Ok(lisp.bignum(num_bigint_from_f64(f)))
}

fn f_ceiling(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    rounding_driver(lisp, args, Rounding::Ceiling, "ceiling")
}

fn f_floor(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    rounding_driver(lisp, args, Rounding::Floor, "floor")
}

fn f_round(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    rounding_driver(lisp, args, Rounding::Round, "round")
}

fn f_truncate(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    rounding_driver(lisp, args, Rounding::Truncate, "truncate")
}

fn fround_driver(lisp: &mut Lisp, v: Value, mode: Rounding) -> LispResult {
    if !v.is_float() {
        return Err(lisp.wrong_type(sym::FLOATP, v));
    }
    let f = lisp.heap.float_value(v);
    Ok(lisp.float(round_float(f, mode)))
}

fn f_fceiling(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    fround_driver(lisp, args[0], Rounding::Ceiling)
}

fn f_ffloor(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    fround_driver(lisp, args[0], Rounding::Floor)
}

fn f_fround(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    fround_driver(lisp, args[0], Rounding::Round)
}

fn f_ftruncate(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    fround_driver(lisp, args[0], Rounding::Truncate)
}
