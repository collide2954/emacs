use emacs_core::dispextern::EmacsRectangle;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdkMonitor {
    pub handle: isize,
    pub geometry: EmacsRectangle,
    pub workarea: EmacsRectangle,
    pub width_mm: i32,
    pub height_mm: i32,
    pub model: Option<String>,
    pub scale_factor: i32,
    pub primary: bool,
}

pub trait MonitorSource {
    fn monitors(&self) -> Vec<GdkMonitor>;
    fn monitor_at_window(&self, window: u64) -> isize;
    fn pointer_position(&self) -> Option<(i32, i32)> {
        None
    }
}

pub struct SystemMonitors;

impl MonitorSource for SystemMonitors {
    fn monitors(&self) -> Vec<GdkMonitor> {
        system_monitors()
    }

    fn monitor_at_window(&self, window: u64) -> isize {
        system_monitor_at_window(window)
    }

    fn pointer_position(&self) -> Option<(i32, i32)> {
        system_pointer_position()
    }
}

#[cfg(windows)]
fn system_pointer_position() -> Option<(i32, i32)> {
    let (x, y) = emacs_w32::cursor_position()?;
    let dpi = f64::from(emacs_w32::monitor_dpi(emacs_w32::monitor_from_point(x, y)).max(1));
    let scale = |v: i32| (f64::from(v) * 96.0 / dpi).round() as i32;
    Some((scale(x), scale(y)))
}

#[cfg(not(windows))]
fn system_pointer_position() -> Option<(i32, i32)> {
    None
}

#[cfg(windows)]
fn system_monitors() -> Vec<GdkMonitor> {
    emacs_w32::display_monitors()
        .into_iter()
        .map(|m| {
            let dpi = f64::from(m.dpi.max(1));
            let logical = |r: emacs_w32::Rect| {
                let scale = |v: i32| (f64::from(v) * 96.0 / dpi).round() as i32;
                let (left, top) = (scale(r.left), scale(r.top));
                EmacsRectangle {
                    x: left,
                    y: top,
                    width: scale(r.right) - left,
                    height: scale(r.bottom) - top,
                }
            };
            GdkMonitor {
                handle: m.handle,
                geometry: logical(m.monitor),
                workarea: logical(m.work),
                width_mm: m.width_mm,
                height_mm: m.height_mm,
                model: m.model,
                scale_factor: 1,
                primary: m.primary,
            }
        })
        .collect()
}

#[cfg(not(windows))]
fn system_monitors() -> Vec<GdkMonitor> {
    Vec::new()
}

#[cfg(windows)]
fn system_monitor_at_window(window: u64) -> isize {
    emacs_w32::monitor_from_window(window)
}

#[cfg(not(windows))]
fn system_monitor_at_window(_window: u64) -> isize {
    0
}
