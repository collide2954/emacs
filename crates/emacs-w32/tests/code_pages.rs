#![cfg(windows)]

#[test]
fn code_pages() {
    let ansi = emacs_w32::ansi_code_page();
    assert!(ansi > 0);
    assert!(emacs_w32::is_valid_code_page(ansi));
    assert!(!emacs_w32::is_valid_code_page(0));
    let console = emacs_w32::console_cp();
    assert!(console == 0 || emacs_w32::is_valid_code_page(console));
    let output = emacs_w32::console_output_cp();
    assert!(output == 0 || emacs_w32::is_valid_code_page(output));
    assert!(emacs_w32::multibyte_code_page() >= 0);
}
