#![cfg(windows)]

use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{ERROR_SUCCESS, MAX_PATH, S_OK};
use windows_sys::Win32::Globalization::{
    GetACP, GetLocaleInfoA, GetUserDefaultUILanguage, IsValidCodePage, LOCALE_SABBREVLANGNAME,
    LOCALE_USE_CP_ACP,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, GetFileAttributesA, INVALID_FILE_ATTRIBUTES,
};
use windows_sys::Win32::System::Console::{
    GetConsoleCP, GetConsoleOutputCP, SetConsoleCP, SetConsoleOutputCP,
};
use windows_sys::Win32::System::Environment::{
    ExpandEnvironmentStringsA, GetEnvironmentVariableA, SetEnvironmentVariableA,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameA;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegOpenKeyExA,
    RegQueryValueExA,
};
use windows_sys::Win32::UI::Shell::{CSIDL_APPDATA, SHGetFolderPathA};

pub use windows_sys::Win32::System::Registry::{REG_EXPAND_SZ, REG_SZ};

pub const LOCALE_USER_DEFAULT: u32 = 0x0400;

unsafe extern "C" {
    fn _getmbcp() -> i32;
}

fn c_string(bytes: &[u8]) -> Vec<u8> {
    let mut s: Vec<u8> = bytes.iter().copied().take_while(|&b| b != 0).collect();
    s.push(0);
    s
}

fn until_nul(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().take_while(|&b| b != 0).collect()
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

pub fn abbrev_lang_name(lcid: u32) -> Option<Vec<u8>> {
    let mut buf = [0u8; 32];
    let n = unsafe {
        GetLocaleInfoA(
            lcid,
            LOCALE_SABBREVLANGNAME | LOCALE_USE_CP_ACP,
            buf.as_mut_ptr(),
            buf.len() as i32,
        )
    };
    (n != 0).then(|| until_nul(&buf))
}

pub fn user_default_ui_language() -> u16 {
    unsafe { GetUserDefaultUILanguage() }
}

pub fn appdata_folder() -> Option<Vec<u8>> {
    let mut buf = [0u8; MAX_PATH as usize];
    let result = unsafe {
        SHGetFolderPathA(
            null_mut(),
            CSIDL_APPDATA as i32,
            null_mut(),
            0,
            buf.as_mut_ptr(),
        )
    };
    (result == S_OK).then(|| until_nul(&buf))
}

pub fn module_file_name() -> Option<Vec<u8>> {
    let mut buf = [0u8; MAX_PATH as usize];
    let n = unsafe { GetModuleFileNameA(null_mut(), buf.as_mut_ptr(), MAX_PATH) };
    (n != 0).then(|| until_nul(&buf))
}

fn file_attributes(path: &[u8]) -> Option<u32> {
    let path = c_string(path);
    let attributes = unsafe { GetFileAttributesA(path.as_ptr()) };
    (attributes != INVALID_FILE_ATTRIBUTES).then_some(attributes)
}

pub fn file_exists(path: &[u8]) -> bool {
    file_attributes(path).is_some()
}

pub fn is_directory(path: &[u8]) -> bool {
    file_attributes(path).is_some_and(|a| a & FILE_ATTRIBUTE_DIRECTORY != 0)
}

fn query_value(root: HKEY, key: &[u8], name: &[u8]) -> Option<(Vec<u8>, u32)> {
    let key = c_string(key);
    let name = c_string(name);
    let mut hkey: HKEY = null_mut();
    if unsafe { RegOpenKeyExA(root, key.as_ptr(), 0, KEY_READ, &mut hkey) } != ERROR_SUCCESS {
        return None;
    }
    let mut size = 0u32;
    let mut kind = 0u32;
    let mut value = None;
    if unsafe {
        RegQueryValueExA(
            hkey,
            name.as_ptr(),
            null(),
            null_mut(),
            null_mut(),
            &mut size,
        )
    } == ERROR_SUCCESS
    {
        let mut data = vec![0u8; size as usize];
        if unsafe {
            RegQueryValueExA(
                hkey,
                name.as_ptr(),
                null(),
                &mut kind,
                data.as_mut_ptr(),
                &mut size,
            )
        } == ERROR_SUCCESS
        {
            data.truncate(size as usize);
            value = Some((data, kind));
        }
    }
    unsafe { RegCloseKey(hkey) };
    value
}

pub fn get_resource(key: &[u8], name: &[u8]) -> Option<(Vec<u8>, u32)> {
    query_value(HKEY_CURRENT_USER, key, name).or_else(|| query_value(HKEY_LOCAL_MACHINE, key, name))
}

pub fn expand_environment_strings(src: &[u8]) -> Vec<u8> {
    let src = c_string(src);
    let needed = unsafe { ExpandEnvironmentStringsA(src.as_ptr(), null_mut(), 0) };
    if needed == 0 {
        return until_nul(&src);
    }
    let mut buf = vec![0u8; needed as usize];
    let n = unsafe { ExpandEnvironmentStringsA(src.as_ptr(), buf.as_mut_ptr(), needed) };
    if n == 0 || n > needed {
        return until_nul(&src);
    }
    until_nul(&buf)
}

pub fn getenv(name: &[u8]) -> Option<Vec<u8>> {
    let name = c_string(name);
    let needed = unsafe { GetEnvironmentVariableA(name.as_ptr(), null_mut(), 0) };
    if needed == 0 {
        return None;
    }
    let mut buf = vec![0u8; needed as usize];
    let n = unsafe { GetEnvironmentVariableA(name.as_ptr(), buf.as_mut_ptr(), needed) };
    (n < needed).then(|| until_nul(&buf))
}

pub fn putenv(name: &[u8], value: &[u8]) -> bool {
    let name = c_string(name);
    let value = c_string(value);
    unsafe { SetEnvironmentVariableA(name.as_ptr(), value.as_ptr()) != 0 }
}
