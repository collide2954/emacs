#![cfg(windows)]

#[test]
fn display_monitors_describe_the_screens() {
    let monitors = emacs_w32::display_monitors();
    assert!(!monitors.is_empty());
    assert_eq!(monitors.iter().filter(|m| m.primary).count(), 1);
    for m in &monitors {
        assert_ne!(m.handle, 0);
        assert!(m.monitor.right > m.monitor.left, "{m:?}");
        assert!(m.monitor.bottom > m.monitor.top, "{m:?}");
        assert!(
            m.work.left >= m.monitor.left && m.work.right <= m.monitor.right,
            "{m:?}"
        );
        assert!(
            m.work.top >= m.monitor.top && m.work.bottom <= m.monitor.bottom,
            "{m:?}"
        );
        assert!(m.dpi > 0, "{m:?}");
        assert!(m.device.starts_with("\\\\.\\DISPLAY"), "{m:?}");
        assert!(m.width_mm >= 0 && m.height_mm >= 0, "{m:?}");
    }
    let primary = monitors.iter().find(|m| m.primary).unwrap();
    assert_eq!((primary.monitor.left, primary.monitor.top), (0, 0));
    let handles: Vec<isize> = monitors.iter().map(|m| m.handle).collect();
    assert!(handles.contains(&emacs_w32::monitor_from_point(0, 0)));
}
