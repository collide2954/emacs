use emacs_core::error::LispResult;
use emacs_core::lisp::Lisp;
use emacs_core::subr::Subr;
use emacs_core::symbols as sym;
use emacs_core::value::Value;

static SUBRS: &[Subr] = &[Subr::fixed(
    "tool-bar-get-system-style",
    0,
    0,
    f_tool_bar_get_system_style,
)];

pub fn f_tool_bar_get_system_style(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let style = lisp.find_symbol_value(sym::TOOL_BAR_STYLE);
    if [
        sym::IMAGE,
        sym::TEXT,
        sym::BOTH,
        sym::BOTH_HORIZ,
        sym::TEXT_IMAGE_HORIZ,
    ]
    .contains(&style)
    {
        return Ok(style);
    }
    Ok(sym::IMAGE)
}

pub fn syms_of_xsettings(lisp: &mut Lisp) {
    for subr in SUBRS {
        lisp.defsubr(subr);
    }
}
