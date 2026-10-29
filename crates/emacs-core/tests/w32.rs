#![cfg(windows)]

mod common;

use common::check;

#[test]
fn code_pages() {
    let ansi = emacs_w32::ansi_code_page().to_string();
    let multibyte = emacs_w32::multibyte_code_page().to_string();
    let console = emacs_w32::console_cp().to_string();
    let output = emacs_w32::console_output_cp().to_string();
    check(&[
        ("w32-ansi-code-page", &ansi),
        ("w32-multibyte-code-page", &multibyte),
        ("(w32-get-console-codepage)", &console),
        ("(w32-get-console-output-codepage)", &output),
        ("(w32-set-console-codepage 0)", "nil"),
        ("(w32-set-console-output-codepage 0)", "nil"),
        (
            "(w32-set-console-codepage 'x)",
            "error: (wrong-type-argument fixnump x)",
        ),
        (
            "(w32-set-console-output-codepage 1.5)",
            "error: (wrong-type-argument fixnump 1.5)",
        ),
    ]);
}
