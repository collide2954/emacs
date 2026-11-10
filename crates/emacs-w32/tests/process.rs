#![cfg(windows)]

use std::ffi::OsStr;

fn cmd_exe() -> std::ffi::OsString {
    emacs_w32::search_path(None, OsStr::new("cmd"), Some(OsStr::new(".exe"))).expect("cmd.exe")
}

#[test]
fn search_and_short_paths() {
    let cmd = cmd_exe();
    let lower = cmd.to_string_lossy().to_lowercase();
    assert!(lower.ends_with("\\system32\\cmd.exe"), "{lower}");
    let system32 = std::path::Path::new(&cmd)
        .parent()
        .expect("system32")
        .as_os_str()
        .to_owned();
    assert_eq!(
        emacs_w32::search_path(Some(&system32), OsStr::new("cmd.exe"), None).as_deref(),
        Some(cmd.as_os_str())
    );
    assert_eq!(
        emacs_w32::search_path(Some(&system32), OsStr::new("cmd"), Some(OsStr::new(".exe")))
            .as_deref(),
        Some(cmd.as_os_str())
    );
    assert_eq!(
        emacs_w32::search_path(Some(&system32), OsStr::new("cmd"), None),
        None
    );
    assert_eq!(
        emacs_w32::search_path(
            None,
            OsStr::new("emacs-w32-no-such-program"),
            Some(OsStr::new(".exe"))
        ),
        None
    );
    let short = emacs_w32::short_path_name(&cmd).expect("short name");
    assert_eq!(
        short.to_string_lossy().to_lowercase(),
        emacs_w32::short_path_name(&short)
            .expect("short name of a short name")
            .to_string_lossy()
            .to_lowercase()
    );
    assert_eq!(
        emacs_w32::short_path_name(OsStr::new("C:\\no\\such\\file\\anywhere")),
        None
    );
}

#[test]
fn command_line_and_environment() {
    let exe = std::env::current_exe().expect("current exe");
    let stem = exe
        .file_stem()
        .expect("stem")
        .to_string_lossy()
        .to_lowercase();
    let line = emacs_w32::command_line().to_string_lossy().to_lowercase();
    assert!(line.contains(&stem), "{line}");
    let size = emacs_w32::environment_size();
    let vars: usize = std::env::vars_os()
        .map(|(k, v)| k.len() + v.len() + 2)
        .sum();
    assert!(size >= vars / 2 && size > 2, "{size} {vars}");
}

#[test]
fn processes() {
    let cmd = cmd_exe();
    let dir = std::env::temp_dir().join(format!("emacs-w32-process-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create directory");
    std::fs::write(dir.join("marker.txt"), "x").expect("create marker");
    let dir = dir.into_os_string();
    let child = emacs_w32::create_process(Some(&cmd), OsStr::new("cmd /c exit 7"), &dir)
        .expect("cmd starts");
    assert_eq!(child.wait(), 7);
    let child = emacs_w32::create_process(None, OsStr::new("cmd /c exit 3"), &dir)
        .expect("cmd found from the command line");
    assert_eq!(child.wait(), 3);
    let child = emacs_w32::create_process(
        Some(&cmd),
        OsStr::new("cmd /c if exist marker.txt exit 4"),
        &dir,
    )
    .expect("cmd starts in the directory");
    assert_eq!(child.wait(), 4);
    assert!(
        emacs_w32::create_process(
            Some(OsStr::new("C:\\no\\such\\program.exe")),
            OsStr::new("program"),
            &dir
        )
        .is_none()
    );
    let ping = emacs_w32::search_path(None, OsStr::new("ping"), Some(OsStr::new(".exe")))
        .expect("ping.exe");
    let child = emacs_w32::create_process(Some(&ping), OsStr::new("ping -n 30 127.0.0.1"), &dir)
        .expect("ping starts");
    assert!(!emacs_w32::wait_for_handle(child.handle(), 100));
    assert!(emacs_w32::terminate_handle(child.handle(), 9));
    assert_eq!(child.wait(), 9);
    assert!(emacs_w32::wait_for_handle(child.handle(), 0));
}

#[test]
fn console_ctrl_handler() {
    fn handler(_event: u32) -> bool {
        true
    }
    assert!(emacs_w32::set_console_ctrl_handler(handler));
    assert_eq!(emacs_w32::CTRL_C_EVENT, 0);
    assert_eq!(emacs_w32::CTRL_BREAK_EVENT, 1);
    assert_eq!(emacs_w32::STATUS_CONTROL_C_EXIT, 0xC000_013A);
}
