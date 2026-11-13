#![cfg(windows)]

fn whoami_user_rid() -> u32 {
    let out = std::process::Command::new("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .expect("whoami");
    let text = String::from_utf8_lossy(&out.stdout);
    let sid = text
        .trim()
        .rsplit(',')
        .next()
        .expect("sid")
        .trim_matches('"');
    sid.rsplit('-')
        .next()
        .expect("rid")
        .parse()
        .expect("numeric rid")
}

#[test]
fn the_process_token_names_the_user_and_group() {
    let id = emacs_w32::process_token_identity().expect("token identity");
    let user = emacs_w32::user_name().expect("GetUserNameA");
    assert!(!id.user_name.is_empty());
    assert!(
        id.user_name.eq_ignore_ascii_case(&user),
        "{id:?} vs {user:?}"
    );
    assert_eq!(id.user_rid, whoami_user_rid());
    let (group_rid, group_name) = id.primary_group.expect("primary group");
    assert!(group_rid > 0);
    assert!(group_name.is_none_or(|name| !name.is_empty()));
}
