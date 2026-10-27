use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};
use std::time::Instant;

use emacs_core::termhooks::{EventKind, InputEvent};
use iced::futures::channel::mpsc as fmpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use iced::keyboard;
use iced::widget::image::{FilterMethod, Handle};
use iced::widget::{image, space};
use iced::{ContentFit, Element, Event, Length, Size, Subscription, Task, event, window};

use crate::display::Display;
use crate::keysym::{KeyPress, key_press_event};
use crate::protocol::{Command, Notice};

const LISP_STACK_SIZE: usize = 1 << 30;

static LISP_MAIN: OnceLock<fn(Display)> = OnceLock::new();

#[derive(Debug)]
pub enum Message {
    Connected(Sender<Notice>),
    Lisp(Command),
    Window(window::Id, window::Event),
    Key(window::Id, keyboard::Event),
}

struct View {
    frame: u64,
    title: String,
    image: Option<(Handle, u32, u32)>,
    scale: f32,
    size: Size,
}

pub struct Shell {
    to_lisp: Option<Sender<Notice>>,
    views: BTreeMap<window::Id, View>,
    frames: HashMap<u64, window::Id>,
    epoch: Instant,
}

impl Shell {
    pub fn boot() -> Shell {
        Shell {
            to_lisp: None,
            views: BTreeMap::new(),
            frames: HashMap::new(),
            epoch: Instant::now(),
        }
    }

    pub fn window_of(&self, frame: u64) -> Option<window::Id> {
        self.frames.get(&frame).copied()
    }

    pub fn has_image(&self, id: window::Id) -> bool {
        self.views.get(&id).is_some_and(|v| v.image.is_some())
    }

    pub fn title(&self, id: window::Id) -> String {
        self.views
            .get(&id)
            .map(|v| v.title.clone())
            .unwrap_or_default()
    }

    fn timestamp(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    fn notify(&self, notice: Notice) {
        if let Some(tx) = &self.to_lisp {
            let _ = tx.send(notice);
        }
    }

    fn send_input(&self, frame: u64, kind: EventKind) {
        self.notify(Notice::Input(InputEvent {
            kind,
            frame,
            timestamp: self.timestamp(),
            ..InputEvent::default()
        }));
    }

    fn configure(&self, id: window::Id) {
        if let Some(view) = self.views.get(&id) {
            self.notify(Notice::Configure {
                frame: view.frame,
                width: (view.size.width * view.scale).round() as u32,
                height: (view.size.height * view.scale).round() as u32,
                scale: view.scale,
            });
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Connected(tx) => {
                self.to_lisp = Some(tx);
                Task::none()
            }
            Message::Lisp(command) => self.command(command),
            Message::Window(id, event) => {
                self.window_event(id, event);
                Task::none()
            }
            Message::Key(id, event) => {
                self.key_event(id, event);
                Task::none()
            }
        }
    }

    fn command(&mut self, command: Command) -> Task<Message> {
        match command {
            Command::OpenFrame {
                frame,
                title,
                width,
                height,
            } => {
                let size = Size::new(width as f32, height as f32);
                let (id, opened) = window::open(window::Settings {
                    size,
                    exit_on_close_request: false,
                    ..window::Settings::default()
                });
                self.views.insert(
                    id,
                    View {
                        frame,
                        title,
                        image: None,
                        scale: 1.0,
                        size,
                    },
                );
                self.frames.insert(frame, id);
                opened.then(|id| {
                    window::scale_factor(id)
                        .map(move |scale| Message::Window(id, window::Event::Rescaled(scale)))
                })
            }
            Command::SetTitle { frame, title } => {
                if let Some(view) = self.window_of(frame).and_then(|id| self.views.get_mut(&id)) {
                    view.title = title;
                }
                Task::none()
            }
            Command::Present {
                frame,
                width,
                height,
                pixels,
            } => {
                if let Some(view) = self.window_of(frame).and_then(|id| self.views.get_mut(&id)) {
                    view.image = Some((Handle::from_rgba(width, height, pixels), width, height));
                }
                Task::none()
            }
            Command::CloseFrame { frame } => match self.frames.remove(&frame) {
                Some(id) => {
                    self.views.remove(&id);
                    window::close(id)
                }
                None => Task::none(),
            },
            Command::Exit => iced::exit(),
        }
    }

    fn window_event(&mut self, id: window::Id, event: window::Event) {
        let Some(frame) = self.views.get(&id).map(|v| v.frame) else {
            return;
        };
        match event {
            window::Event::Opened { size, .. } | window::Event::Resized(size) => {
                if let Some(view) = self.views.get_mut(&id) {
                    view.size = size;
                }
                self.configure(id);
            }
            window::Event::Rescaled(scale) => {
                if let Some(view) = self.views.get_mut(&id) {
                    view.scale = scale;
                }
                self.configure(id);
            }
            window::Event::CloseRequested => self.send_input(frame, EventKind::DeleteWindow),
            window::Event::Focused => self.send_input(frame, EventKind::FocusIn),
            window::Event::Unfocused => self.send_input(frame, EventKind::FocusOut),
            window::Event::Closed => {
                self.views.remove(&id);
                self.frames.remove(&frame);
            }
            _ => {}
        }
    }

    fn key_event(&mut self, id: window::Id, event: keyboard::Event) {
        let Some(frame) = self.views.get(&id).map(|v| v.frame) else {
            return;
        };
        if let keyboard::Event::KeyPressed {
            key,
            modified_key,
            location,
            modifiers,
            text,
            ..
        } = event
        {
            let press = KeyPress {
                key: &key,
                modified_key: &modified_key,
                location,
                modifiers,
                text: text.as_deref(),
            };
            if let Some(input) = key_press_event(&press, self.timestamp(), frame) {
                self.notify(Notice::Input(input));
            }
        }
    }

    pub fn view(&self, id: window::Id) -> Element<'_, Message> {
        match self.views.get(&id) {
            Some(View {
                image: Some((handle, width, height)),
                scale,
                ..
            }) => image(handle.clone())
                .width(Length::Fixed(*width as f32 / scale))
                .height(Length::Fixed(*height as f32 / scale))
                .content_fit(ContentFit::Fill)
                .filter_method(FilterMethod::Nearest)
                .into(),
            _ => space().into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            Subscription::run(bridge),
            window::events().map(|(id, event)| Message::Window(id, event)),
            event::listen_with(|event, _status, id| match event {
                Event::Keyboard(key) => Some(Message::Key(id, key)),
                _ => None,
            }),
        ])
    }
}

fn bridge() -> impl Stream<Item = Message> {
    iced::stream::channel(64, async |mut output: fmpsc::Sender<Message>| {
        let (commands, mut command_rx) = fmpsc::unbounded();
        let (notice_tx, notices) = mpsc::channel();
        let _ = output.send(Message::Connected(notice_tx)).await;
        if let Some(&main) = LISP_MAIN.get() {
            let _ = std::thread::Builder::new()
                .name("lisp".into())
                .stack_size(LISP_STACK_SIZE)
                .spawn(move || main(Display::new(commands, notices)));
        }
        while let Some(command) = command_rx.next().await {
            if output.send(Message::Lisp(command)).await.is_err() {
                break;
            }
        }
    })
}

pub fn run(main: fn(Display)) -> iced::Result {
    let _ = LISP_MAIN.set(main);
    iced::daemon(Shell::boot, Shell::update, Shell::view)
        .title(Shell::title)
        .subscription(Shell::subscription)
        .run()
}
