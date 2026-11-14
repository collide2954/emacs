use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::time::Duration;

use iced::futures::channel::mpsc::UnboundedSender;

use crate::monitor::{GdkMonitor, MonitorSource, SystemMonitors};
use crate::protocol::{Command, Notice};
use crate::surface::Surface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disconnected;

pub struct Display {
    commands: UnboundedSender<Command>,
    notices: Receiver<Notice>,
    pending: Vec<Notice>,
    surfaces: HashMap<u64, Surface>,
    shown: HashSet<u64>,
    monitor_source: Box<dyn MonitorSource + Send>,
}

impl Display {
    pub fn new(commands: UnboundedSender<Command>, notices: Receiver<Notice>) -> Display {
        Display {
            commands,
            notices,
            pending: Vec::new(),
            surfaces: HashMap::new(),
            shown: HashSet::new(),
            monitor_source: Box::new(SystemMonitors),
        }
    }

    pub fn set_monitor_source(&mut self, source: Box<dyn MonitorSource + Send>) {
        self.monitor_source = source;
    }

    pub fn monitors(&self) -> Vec<GdkMonitor> {
        self.monitor_source.monitors()
    }

    pub fn monitor_at_window(&self, window: u64) -> isize {
        self.monitor_source.monitor_at_window(window)
    }

    pub fn wait(&mut self, timeout: Duration) -> Result<bool, Disconnected> {
        if !self.pending.is_empty() {
            return Ok(true);
        }
        match self.notices.recv_timeout(timeout) {
            Ok(notice) => {
                self.pending.push(notice);
                Ok(true)
            }
            Err(RecvTimeoutError::Timeout) => Ok(false),
            Err(RecvTimeoutError::Disconnected) => Err(Disconnected),
        }
    }

    fn send(&self, command: Command) {
        let _ = self.commands.unbounded_send(command);
    }

    pub fn open_frame(
        &mut self,
        frame: u64,
        title: &str,
        width: u32,
        height: u32,
        background: u32,
    ) {
        self.create_frame(frame, width, height, background);
        self.show_frame(frame, title);
    }

    pub fn create_frame(&mut self, frame: u64, width: u32, height: u32, background: u32) {
        self.surfaces
            .insert(frame, Surface::new(width, height, background));
    }

    pub fn show_frame(&mut self, frame: u64, title: &str) {
        if self.shown.contains(&frame) {
            return;
        }
        let Some(surface) = self.surfaces.get(&frame) else {
            return;
        };
        let (width, height) = (surface.width(), surface.height());
        self.shown.insert(frame);
        self.send(Command::OpenFrame {
            frame,
            title: title.to_string(),
            width,
            height,
        });
    }

    pub fn resize_frame(&mut self, frame: u64, width: u32, height: u32) {
        let Some(surface) = self.surfaces.get_mut(&frame) else {
            return;
        };
        if (surface.width(), surface.height()) != (width, height) {
            surface.resize(width, height);
        }
        if self.shown.contains(&frame) {
            self.send(Command::ResizeFrame {
                frame,
                width,
                height,
            });
        }
    }

    pub fn surface(&self, frame: u64) -> Option<&Surface> {
        self.surfaces.get(&frame)
    }

    pub fn surface_mut(&mut self, frame: u64) -> Option<&mut Surface> {
        self.surfaces.get_mut(&frame)
    }

    pub fn set_title(&self, frame: u64, title: &str) {
        self.send(Command::SetTitle {
            frame,
            title: title.to_string(),
        });
    }

    pub fn flush(&self, frame: u64) {
        if let Some(surface) = self.surfaces.get(&frame) {
            self.send(Command::Present {
                frame,
                width: surface.width(),
                height: surface.height(),
                pixels: surface.rgba(),
            });
        }
    }

    pub fn set_fullscreen(&self, frame: u64, on: bool) {
        self.send(Command::SetFullscreen { frame, on });
    }

    pub fn set_maximized(&self, frame: u64, on: bool) {
        self.send(Command::SetMaximized { frame, on });
    }

    pub fn set_cursor(&self, frame: u64, cursor: iced::mouse::Interaction) {
        self.send(Command::SetCursor { frame, cursor });
    }

    pub fn close_frame(&mut self, frame: u64) {
        self.surfaces.remove(&frame);
        self.shown.remove(&frame);
        self.send(Command::CloseFrame { frame });
    }

    pub fn exit(&self) {
        self.send(Command::Exit);
    }

    pub fn read_notices(&mut self, timeout: Option<Duration>) -> Result<Vec<Notice>, Disconnected> {
        let mut out = std::mem::take(&mut self.pending);
        if out.is_empty()
            && let Some(timeout) = timeout
        {
            match self.notices.recv_timeout(timeout) {
                Ok(notice) => out.push(notice),
                Err(RecvTimeoutError::Timeout) => return Ok(out),
                Err(RecvTimeoutError::Disconnected) => return Err(Disconnected),
            }
        }
        loop {
            match self.notices.try_recv() {
                Ok(notice) => out.push(notice),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if out.is_empty() {
                        return Err(Disconnected);
                    }
                    break;
                }
            }
        }
        Ok(out)
    }

    pub fn resize_surface(&mut self, frame: u64, width: u32, height: u32) -> bool {
        match self.surfaces.get_mut(&frame) {
            Some(surface) if (surface.width(), surface.height()) != (width, height) => {
                surface.resize(width, height);
                true
            }
            _ => false,
        }
    }
}
