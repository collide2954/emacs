use std::sync::OnceLock;

static BOOT_TIME: OnceLock<Option<(i64, i64)>> = OnceLock::new();

pub fn get_boot_time() -> Option<(i64, i64)> {
    *BOOT_TIME.get_or_init(get_boot_time_uncached)
}

#[cfg(any(target_os = "linux", target_os = "openbsd", windows))]
fn stat_mtime(path: &str) -> Option<(i64, i64)> {
    std::fs::metadata(path)
        .ok()
        .map(|m| crate::fileio::stat_mtime(&m))
}

#[cfg(any(target_os = "linux", target_os = "android", windows))]
fn subtract_uptime(now: (i64, i64), uptime: (i64, i64)) -> (i64, i64) {
    let (mut sec, mut nsec) = now;
    if nsec < uptime.1 {
        nsec += 1_000_000_000;
        sec -= 1;
    }
    (sec - uptime.0, nsec - uptime.1)
}

#[cfg(any(target_os = "linux", target_os = "android", windows))]
fn realtime_now() -> Option<(i64, i64)> {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    Some((d.as_secs() as i64, d.subsec_nanos() as i64))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_linux_uptime() -> Option<(i64, i64)> {
    if let Some(uptime) = emacs_unix::clock_boottime() {
        return Some(uptime);
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(buf) = std::fs::read("/proc/uptime") {
            let buf = &buf[..buf.len().min(32)];
            if !buf.is_empty() {
                let digits = buf.iter().take_while(|b| b.is_ascii_digit()).count();
                if digits > 0 {
                    let s = buf[..digits]
                        .iter()
                        .fold(0i64, |s, &b| 10 * s + i64::from(b - b'0'));
                    let mut ns = 0i64;
                    let mut p = digits;
                    if buf.get(p) == Some(&b'.') {
                        p += 1;
                        for _ in 0..9 {
                            ns = 10 * ns
                                + match buf.get(p) {
                                    Some(&b) if b.is_ascii_digit() => {
                                        p += 1;
                                        i64::from(b - b'0')
                                    }
                                    _ => 0,
                                };
                        }
                    }
                    return Some((s, ns));
                }
            }
        }
        if let Some(uptime) = emacs_unix::sysinfo_uptime() {
            return Some((uptime, 0));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn get_linux_boot_time_fallback() -> Option<(i64, i64)> {
    [
        "/var/lib/systemd/random-seed",
        "/var/lib/urandom/random-seed",
        "/var/lib/random-seed",
        "/var/run/utmp",
    ]
    .iter()
    .filter_map(|f| stat_mtime(f))
    .find(|t| t.0 >= 1122334455)
}

#[cfg(target_os = "linux")]
fn get_linux_boot_time_final_fallback() -> Option<(i64, i64)> {
    let uptime = get_linux_uptime()?;
    Some(subtract_uptime(realtime_now()?, uptime))
}

#[cfg(target_os = "linux")]
fn utmp_boot_time() -> (i64, i64) {
    let mut found = (0, 0);
    let mut runlevel = (0, 0);
    for ut in emacs_unix::utmpx_entries() {
        let ts = (ut.tv_sec, ut.tv_usec * 1000);
        if ut.boot_time {
            found = ts;
        }
        if ut.user == b"runlevel" && ut.line == b"~" {
            runlevel = ts;
        }
    }
    if found.0 <= 60 && runlevel.0 != 0 {
        found = runlevel;
    }
    if found.0 == 0
        && let Some(t) = get_linux_boot_time_fallback()
    {
        found = t;
    }
    if found.0 == 0
        && let Some(t) = get_linux_boot_time_final_fallback()
    {
        found = t;
    }
    found
}

#[cfg(target_os = "android")]
fn utmp_boot_time() -> (i64, i64) {
    match get_linux_uptime().zip(realtime_now()) {
        Some((uptime, now)) => subtract_uptime(now, uptime),
        None => (0, 0),
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "netbsd"))]
fn utmp_boot_time() -> (i64, i64) {
    let mut found = (0, 0);
    for ut in emacs_unix::utmpx_entries() {
        if ut.boot_time {
            found = (ut.tv_sec, ut.tv_usec * 1000);
        }
    }
    found
}

#[cfg(target_os = "openbsd")]
fn utmp_boot_time() -> (i64, i64) {
    ["/var/db/host.random", "/var/run/utmp"]
        .iter()
        .find_map(|f| stat_mtime(f))
        .unwrap_or((0, 0))
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn utmp_boot_time() -> (i64, i64) {
    (0, 0)
}

#[cfg(windows)]
fn get_windows_boot_time() -> Option<(i64, i64)> {
    stat_mtime("C:\\pagefile.sys")
}

#[cfg(windows)]
fn get_windows_boot_time_fallback() -> Option<(i64, i64)> {
    let uptime_ms = emacs_w32::tick_count64() as i64;
    let now = realtime_now()?;
    let now = (now.0, now.1 / 1000 * 1000);
    Some(subtract_uptime(
        now,
        (uptime_ms / 1000, (uptime_ms % 1000) * 1_000_000),
    ))
}

fn get_boot_time_uncached() -> Option<(i64, i64)> {
    #[allow(unused_mut)]
    let mut found = utmp_boot_time();
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    if found.0 == 0
        && let Some(t) = emacs_unix::sysctl_boottime()
    {
        found = t;
    }
    #[cfg(windows)]
    if found.0 == 0 {
        if let Some(t) = get_windows_boot_time() {
            found = t;
        }
        if found.0 == 0
            && let Some(t) = get_windows_boot_time_fallback()
        {
            found = t;
        }
    }
    (found.0 != 0).then_some(found)
}
