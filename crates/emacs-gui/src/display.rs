use std::collections::HashMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::time::Duration;

use iced::futures::channel::mpsc::UnboundedSender;

use crate::protocol::{Command, Notice};
use crate::surface::Surface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disconnected;

pub struct Display {
    commands: UnboundedSender<Command>,
    notices: Receiver<Notice>,
    surfaces: HashMap<u64, Surface>,
}

impl Display {
    pub fn new(commands: UnboundedSender<Command>, notices: Receiver<Notice>) -> Display {
        Display {
            commands,
            notices,
            surfaces: HashMap::new(),
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
        self.surfaces
            .insert(frame, Surface::new(width, height, background));
        self.send(Command::OpenFrame {
            frame,
            title: title.to_string(),
            width,
            height,
        });
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

    pub fn close_frame(&mut self, frame: u64) {
        self.surfaces.remove(&frame);
        self.send(Command::CloseFrame { frame });
    }

    pub fn exit(&self) {
        self.send(Command::Exit);
    }

    pub fn read_notices(&mut self, timeout: Option<Duration>) -> Result<Vec<Notice>, Disconnected> {
        let mut out = Vec::new();
        if let Some(timeout) = timeout {
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
        for notice in &out {
            if let Notice::Configure {
                frame,
                width,
                height,
                ..
            } = *notice
                && let Some(surface) = self.surfaces.get_mut(&frame)
                && (surface.width(), surface.height()) != (width, height)
            {
                surface.resize(width, height);
            }
        }
        Ok(out)
    }
}
