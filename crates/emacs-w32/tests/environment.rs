#![cfg(windows)]

#[test]
fn locale() {
    let name = emacs_w32::abbrev_lang_name(emacs_w32::LOCALE_USER_DEFAULT).expect("language name");
    assert_eq!(name.len(), 3);
    assert!(name.iter().all(u8::is_ascii_uppercase));
    assert_ne!(emacs_w32::user_default_ui_language(), 0);
}

#[test]
fn files_and_folders() {
    let appdata = emacs_w32::appdata_folder().expect("APPDATA folder");
    assert!(emacs_w32::is_directory(&appdata));
    assert!(emacs_w32::file_exists(&appdata));
    assert!(!emacs_w32::file_exists(b"C:/no/such/file/anywhere"));
    let module = emacs_w32::module_file_name().expect("module file name");
    assert!(emacs_w32::file_exists(&module));
    assert!(!emacs_w32::is_directory(&module));
}

#[test]
fn registry_and_environment() {
    assert_eq!(
        emacs_w32::get_resource(b"SOFTWARE\\GNU\\Emacs\\NoSuchKey", b"HOME"),
        None
    );
    let (root, kind) = emacs_w32::get_resource(
        b"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
        b"SystemRoot",
    )
    .expect("SystemRoot");
    assert_eq!(kind, emacs_w32::REG_SZ);
    let root: Vec<u8> = root.into_iter().take_while(|&b| b != 0).collect();
    let expanded = emacs_w32::expand_environment_strings(b"%SystemRoot%");
    assert!(expanded.eq_ignore_ascii_case(&root));
    assert!(emacs_w32::putenv(b"EMACS_W32_TEST", b"%SystemRoot%\\x"));
    assert_eq!(
        emacs_w32::getenv(b"EMACS_W32_TEST").as_deref(),
        Some(&b"%SystemRoot%\\x"[..])
    );
    assert_eq!(
        std::env::var("EMACS_W32_TEST").as_deref(),
        Ok("%SystemRoot%\\x")
    );
    assert_eq!(
        emacs_w32::expand_environment_strings(b"<%EMACS_W32_TEST%>"),
        b"<%SystemRoot%\\x>"
    );
    assert_eq!(emacs_w32::getenv(b"EMACS_W32_NO_SUCH_VARIABLE"), None);
}
