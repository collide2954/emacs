use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

#[cfg(windows)]
use crate::error::LispResult;
#[cfg(windows)]
use crate::subr::Subr;

#[cfg(windows)]
pub const SUBRS: &[Subr] = &[
    Subr::fixed("w32-get-console-codepage", 0, 0, f_w32_get_console_codepage),
    Subr::fixed("w32-set-console-codepage", 1, 1, f_w32_set_console_codepage),
    Subr::fixed(
        "w32-get-console-output-codepage",
        0,
        0,
        f_w32_get_console_output_codepage,
    ),
    Subr::fixed(
        "w32-set-console-output-codepage",
        1,
        1,
        f_w32_set_console_output_codepage,
    ),
];

#[cfg(not(windows))]
pub const SUBRS: &[crate::subr::Subr] = &[];

#[cfg(windows)]
fn f_w32_get_console_codepage(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(i64::from(emacs_w32::console_cp())))
}

#[cfg(windows)]
fn f_w32_set_console_codepage(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let cp = lisp.check_fixnum(args[0])? as u32;
    if !emacs_w32::is_valid_code_page(cp) || !emacs_w32::set_console_cp(cp) {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(i64::from(emacs_w32::console_cp())))
}

#[cfg(windows)]
fn f_w32_get_console_output_codepage(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::fixnum(i64::from(emacs_w32::console_output_cp())))
}

#[cfg(windows)]
fn f_w32_set_console_output_codepage(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let cp = lisp.check_fixnum(args[0])? as u32;
    if !emacs_w32::is_valid_code_page(cp) || !emacs_w32::set_console_output_cp(cp) {
        return Ok(Value::NIL);
    }
    Ok(Value::fixnum(i64::from(emacs_w32::console_output_cp())))
}

pub fn init_vars(lisp: &mut Lisp) {
    if !cfg!(windows) {
        return;
    }
    for name in ["high", "low", "cygwin", "msys", "w32-native"] {
        lisp.intern(name);
    }
    let local = lisp.intern("local");
    for (name, value) in [
        ("w32-quote-process-args", sym::T),
        ("w32-start-process-show-window", Value::NIL),
        ("w32-start-process-share-console", Value::NIL),
        ("w32-start-process-inherit-error-mode", sym::T),
        ("w32-downcase-file-names", Value::NIL),
        ("w32-get-true-file-attributes", local),
        ("w32-collate-ignore-punctuation", Value::NIL),
    ] {
        let symbol = lisp.intern(name);
        lisp.defvar(symbol, value);
    }
    for name in ["w32-pipe-read-delay", "w32-pipe-buffer-size"] {
        let symbol = lisp.intern(name);
        lisp.defvar_int(symbol, 0);
    }
}
