#![cfg(windows)]

use windows_sys::Win32::Globalization::{GetACP, IsValidCodePage};
use windows_sys::Win32::System::Console::{
    GetConsoleCP, GetConsoleOutputCP, SetConsoleCP, SetConsoleOutputCP,
};

unsafe extern "C" {
    fn _getmbcp() -> i32;
}

pub fn ansi_code_page() -> u32 {
    unsafe { GetACP() }
}

pub fn multibyte_code_page() -> i32 {
    unsafe { _getmbcp() }
}

pub fn is_valid_code_page(code_page: u32) -> bool {
    unsafe { IsValidCodePage(code_page) != 0 }
}

pub fn console_cp() -> u32 {
    unsafe { GetConsoleCP() }
}

pub fn console_output_cp() -> u32 {
    unsafe { GetConsoleOutputCP() }
}

pub fn set_console_cp(code_page: u32) -> bool {
    unsafe { SetConsoleCP(code_page) != 0 }
}

pub fn set_console_output_cp(code_page: u32) -> bool {
    unsafe { SetConsoleOutputCP(code_page) != 0 }
}
