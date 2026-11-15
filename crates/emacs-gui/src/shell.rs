use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};
use std::time::Instant;

use emacs_core::termhooks::{EventKind, InputEvent};
use iced::futures::channel::mpsc as fmpsc;
use iced::futures::{SinkExt, Stream, StreamExt};
use iced::keyboard;
use iced::mouse;
use iced::widget::image::{FilterMethod, Handle};
use iced::widget::{
    button, column, container, image, mouse_area, row, rule, sensor, space, text, tooltip,
};
use iced::{
    Alignment, ContentFit, Element, Event, Length, Point, Size, Subscription, Task, event, window,
};

use crate::display::Display;
use crate::keysym::{KeyPress, emacs_modifiers, key_press_event};
use crate::mouse::{WheelAccumulator, button_event};
use crate::protocol::{Command, Notice, ToolBar, ToolBarPosition, ToolItem};

const TOOL_BAR_BUTTON_PADDING: u16 = 4;
const TOOL_BAR_LABEL_SIZE: f32 = 12.0;

const LISP_STACK_SIZE: usize = 1 << 30;

static LISP_MAIN: OnceLock<fn(Display)> = OnceLock::new();

#[derive(Debug, Clone)]
pub enum Message {
    Connected(Sender<Notice>),
    Lisp(Command),
    Window(window::Id, window::Event),
    Key(window::Id, keyboard::Event),
    Mouse(window::Id, mouse::Event),
    Realized(window::Id, u64),
    Allocated {
        id: window::Id,
        seq: u64,
        handle: Handle,
        width: u32,
        height: u32,
        allocation: Option<image::Allocation>,
    },
    ToolBarClicked(window::Id, usize),
    ToolBarHover(window::Id, Option<usize>),
    ToolBarAllocated(window::Id, Size),
}

struct View {
    frame: u64,
    title: String,
    image: Option<(Handle, u32, u32)>,
    allocation: Option<image::Allocation>,
    presented: u64,
    shown: u64,
    scale: f32,
    size: Size,
    cursor: Point,
    wheel: WheelAccumulator,
    interaction: mouse::Interaction,
    popup: bool,
    visible: bool,
    fullscreen: bool,
    position: Option<(i32, i32)>,
    raw: u64,
    tooltip: Option<String>,
    tool_bar: Option<ToolBar>,
    tool_bar_images: Vec<Option<Handle>>,
    tool_bar_size: Size,
    pointer: Point,
    pressed: u32,
}

impl View {
    fn tool_bar_position(&self) -> Option<ToolBarPosition> {
        self.tool_bar.as_ref().and_then(|bar| bar.position)
    }

    fn canvas_origin(&self) -> Point {
        match self.tool_bar_position() {
            Some(ToolBarPosition::Top) => Point::new(0.0, self.tool_bar_size.height),
            Some(ToolBarPosition::Left) => Point::new(self.tool_bar_size.width, 0.0),
            _ => Point::ORIGIN,
        }
    }

    fn canvas_size(&self) -> Size {
        match self.tool_bar_position() {
            Some(ToolBarPosition::Top | ToolBarPosition::Bottom) => Size::new(
                self.size.width,
                (self.size.height - self.tool_bar_size.height).max(0.0),
            ),
            Some(ToolBarPosition::Left | ToolBarPosition::Right) => Size::new(
                (self.size.width - self.tool_bar_size.width).max(0.0),
                self.size.height,
            ),
            None => self.size,
        }
    }

    fn in_canvas(&self, pointer: Point) -> bool {
        let origin = self.canvas_origin();
        let size = self.canvas_size();
        pointer.x >= origin.x
            && pointer.y >= origin.y
            && pointer.x < origin.x + size.width
            && pointer.y < origin.y + size.height
    }

    fn set_tool_bar(&mut self, bar: Option<ToolBar>) {
        self.tool_bar_images = bar
            .iter()
            .flat_map(|bar| &bar.items)
            .map(|item| match item {
                ToolItem::Button {
                    image: Some(image), ..
                } => Some(Handle::from_rgba(
                    image.width,
                    image.height,
                    image.rgba.clone(),
                )),
                _ => None,
            })
            .collect();
        if bar.as_ref().and_then(|bar| bar.position) != self.tool_bar_position() {
            self.tool_bar_size = Size::ZERO;
        }
        self.tool_bar = bar;
    }

    fn new(frame: u64, title: String, size: Size, popup: bool) -> View {
        View {
            frame,
            title,
            image: None,
            allocation: None,
            presented: 0,
            shown: 0,
            scale: 1.0,
            size,
            cursor: Point::ORIGIN,
            wheel: WheelAccumulator::default(),
            interaction: mouse::Interaction::Idle,
            popup,
            visible: true,
            fullscreen: false,
            position: None,
            raw: 0,
            tooltip: None,
            tool_bar: None,
            tool_bar_images: Vec::new(),
            tool_bar_size: Size::ZERO,
            pointer: Point::ORIGIN,
            pressed: 0,
        }
    }
}

#[cfg(windows)]
fn popup_platform() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific {
        skip_taskbar: true,
        drag_and_drop: false,
        ..window::settings::PlatformSpecific::default()
    }
}

#[cfg(target_os = "linux")]
fn popup_platform() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific {
        override_redirect: true,
        ..window::settings::PlatformSpecific::default()
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
fn popup_platform() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific::default()
}

#[cfg(windows)]
fn prepare_popup(raw: u64, visible: bool) {
    emacs_w32::make_popup_window(raw);
    if visible {
        emacs_w32::show_window_no_activate(raw);
    }
}

#[cfg(not(windows))]
fn prepare_popup(_raw: u64, _visible: bool) {}

#[cfg(windows)]
fn popup_visibility(_id: window::Id, raw: u64, on: bool) -> Task<Message> {
    if on {
        emacs_w32::show_window_no_activate(raw);
    } else {
        emacs_w32::hide_window(raw);
    }
    Task::none()
}

#[cfg(not(windows))]
fn popup_visibility(id: window::Id, _raw: u64, on: bool) -> Task<Message> {
    window::set_mode(
        id,
        if on {
            window::Mode::Windowed
        } else {
            window::Mode::Hidden
        },
    )
}

pub struct Shell {
    to_lisp: Option<Sender<Notice>>,
    views: BTreeMap<window::Id, View>,
    frames: HashMap<u64, window::Id>,
    epoch: Instant,
    modifiers: keyboard::Modifiers,
}

impl Shell {
    pub fn boot() -> Shell {
        Shell {
            to_lisp: None,
            views: BTreeMap::new(),
            frames: HashMap::new(),
            epoch: Instant::now(),
            modifiers: keyboard::Modifiers::empty(),
        }
    }

    pub fn window_of(&self, frame: u64) -> Option<window::Id> {
        self.frames.get(&frame).copied()
    }

    pub fn has_image(&self, id: window::Id) -> bool {
        self.views.get(&id).is_some_and(|v| v.image.is_some())
    }

    pub fn interaction(&self, id: window::Id) -> mouse::Interaction {
        self.views
            .get(&id)
            .map_or(mouse::Interaction::Idle, |v| v.interaction)
    }

    pub fn is_popup(&self, id: window::Id) -> bool {
        self.views.get(&id).is_some_and(|v| v.popup)
    }

    pub fn visible(&self, id: window::Id) -> bool {
        self.views.get(&id).is_some_and(|v| v.visible)
    }

    pub fn position(&self, id: window::Id) -> Option<(i32, i32)> {
        self.views.get(&id).and_then(|v| v.position)
    }

    pub fn tooltip(&self, id: window::Id) -> Option<String> {
        self.views.get(&id).and_then(|v| v.tooltip.clone())
    }

    pub fn tool_bar(&self, id: window::Id) -> Option<&ToolBar> {
        self.views.get(&id).and_then(|v| v.tool_bar.as_ref())
    }

    pub fn image_size(&self, id: window::Id) -> Option<(u32, u32)> {
        self.views
            .get(&id)
            .and_then(|v| v.image.as_ref())
            .map(|(_, width, height)| (*width, *height))
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
            let size = view.canvas_size();
            self.notify(Notice::Configure {
                frame: view.frame,
                width: size.width.round() as u32,
                height: size.height.round() as u32,
                scale: view.scale,
            });
        }
    }

    fn replace_tool_bar(&mut self, frame: u64, bar: Option<ToolBar>) {
        let Some(id) = self.window_of(frame) else {
            return;
        };
        let Some(view) = self.views.get_mut(&id) else {
            return;
        };
        let before = view.canvas_size();
        view.set_tool_bar(bar);
        if view.canvas_size() != before {
            self.configure(id);
        }
    }

    fn tool_bar_message(&mut self, message: Message) {
        match message {
            Message::ToolBarClicked(id, index) => {
                if let Some(view) = self.views.get(&id) {
                    self.notify(Notice::Input(InputEvent {
                        kind: EventKind::ToolBar,
                        code: index as u32,
                        modifiers: emacs_modifiers(self.modifiers),
                        frame: view.frame,
                        timestamp: self.timestamp(),
                        ..InputEvent::default()
                    }));
                }
            }
            Message::ToolBarHover(id, index) => {
                if let Some(view) = self.views.get(&id) {
                    self.notify(Notice::ToolBarHelp {
                        frame: view.frame,
                        index,
                    });
                }
            }
            Message::ToolBarAllocated(id, size) => {
                let Some(view) = self.views.get_mut(&id) else {
                    return;
                };
                view.tool_bar_size = size;
                let frame = view.frame;
                self.notify(Notice::ToolBarSize {
                    frame,
                    width: size.width.round() as u32,
                    height: size.height.round() as u32,
                });
                self.configure(id);
            }
            _ => {}
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
            Message::Mouse(id, event) => {
                self.mouse_event(id, event);
                Task::none()
            }
            Message::Realized(id, window) => {
                if let Some(view) = self.views.get_mut(&id) {
                    view.raw = window;
                    if view.popup {
                        prepare_popup(window, view.visible);
                    } else {
                        let frame = view.frame;
                        self.notify(Notice::Realize { frame, window });
                    }
                }
                Task::none()
            }
            Message::Allocated {
                id,
                seq,
                handle,
                width,
                height,
                allocation,
            } => {
                if let Some(view) = self.views.get_mut(&id)
                    && seq > view.shown
                {
                    view.image = Some((handle, width, height));
                    view.allocation = allocation;
                    view.shown = seq;
                }
                Task::none()
            }
            Message::ToolBarClicked(..)
            | Message::ToolBarHover(..)
            | Message::ToolBarAllocated(..) => {
                self.tool_bar_message(message);
                Task::none()
            }
        }
    }

    fn mouse_event(&mut self, id: window::Id, event: mouse::Event) {
        let timestamp = self.timestamp();
        let modifiers = self.modifiers;
        let Some(view) = self.views.get_mut(&id) else {
            return;
        };
        if view.popup {
            return;
        }
        let frame = view.frame;
        let inside = view.in_canvas(view.pointer);
        let inputs = match event {
            mouse::Event::CursorMoved { position } => {
                let was_inside = inside;
                view.pointer = position;
                let origin = view.canvas_origin();
                view.cursor = Point::new(position.x - origin.x, position.y - origin.y);
                let (x, y) = (view.cursor.x as i64, view.cursor.y as i64);
                if view.in_canvas(position) || view.pressed != 0 {
                    self.notify(Notice::Motion {
                        frame,
                        x,
                        y,
                        timestamp,
                    });
                } else if was_inside {
                    self.notify(Notice::Leave { frame });
                }
                return;
            }
            mouse::Event::CursorLeft => {
                self.notify(Notice::Leave { frame });
                return;
            }
            mouse::Event::ButtonPressed(button) if inside || view.pressed != 0 => {
                view.pressed += 1;
                vec![button_event(
                    button,
                    true,
                    view.cursor,
                    modifiers,
                    timestamp,
                    frame,
                )]
            }
            mouse::Event::ButtonReleased(button) if view.pressed != 0 => {
                view.pressed -= 1;
                vec![button_event(
                    button,
                    false,
                    view.cursor,
                    modifiers,
                    timestamp,
                    frame,
                )]
            }
            mouse::Event::ButtonReleased(button) if inside => vec![button_event(
                button,
                false,
                view.cursor,
                modifiers,
                timestamp,
                frame,
            )],
            mouse::Event::WheelScrolled { delta } if inside => {
                view.wheel
                    .events(delta, view.cursor, modifiers, timestamp, frame)
            }
            _ => Vec::new(),
        };
        for input in inputs {
            self.notify(Notice::Input(input));
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
                self.views.insert(id, View::new(frame, title, size, false));
                self.frames.insert(frame, id);
                opened.then(|id| {
                    Task::batch([
                        window::scale_factor(id)
                            .map(move |scale| Message::Window(id, window::Event::Rescaled(scale))),
                        window::raw_id::<Message>(id).map(move |raw| Message::Realized(id, raw)),
                    ])
                })
            }
            Command::OpenPopup {
                frame,
                x,
                y,
                width,
                height,
            } => {
                let size = Size::new(width as f32, height as f32);
                let (id, opened) = window::open(window::Settings {
                    size,
                    position: window::Position::Specific(Point::new(x as f32, y as f32)),
                    visible: !cfg!(windows),
                    resizable: false,
                    minimizable: false,
                    closeable: false,
                    decorations: false,
                    level: window::Level::AlwaysOnTop,
                    exit_on_close_request: false,
                    platform_specific: popup_platform(),
                    ..window::Settings::default()
                });
                let mut view = View::new(frame, String::new(), size, true);
                view.position = Some((x, y));
                self.views.insert(id, view);
                self.frames.insert(frame, id);
                opened.then(|id| {
                    window::raw_id::<Message>(id).map(move |raw| Message::Realized(id, raw))
                })
            }
            Command::SetTitle { frame, title } => {
                if let Some(view) = self.window_of(frame).and_then(|id| self.views.get_mut(&id)) {
                    view.title = title;
                }
                Task::none()
            }
            Command::MoveFrame { frame, x, y } => {
                let Some(id) = self.window_of(frame) else {
                    return Task::none();
                };
                if let Some(view) = self.views.get_mut(&id) {
                    view.position = Some((x, y));
                }
                window::move_to(id, Point::new(x as f32, y as f32))
            }
            Command::SetVisible { frame, on } => {
                let Some(id) = self.window_of(frame) else {
                    return Task::none();
                };
                let Some(view) = self.views.get_mut(&id) else {
                    return Task::none();
                };
                view.visible = on;
                if view.popup {
                    return popup_visibility(id, view.raw, on);
                }
                if !on {
                    return window::set_mode(id, window::Mode::Hidden);
                }
                let fullscreen = view.fullscreen;
                self.notify(Notice::Map { frame });
                let show = window::set_mode(id, window::Mode::Windowed);
                if fullscreen {
                    show.chain(window::set_mode(id, window::Mode::Fullscreen))
                } else {
                    show
                }
            }
            Command::SetTooltip { frame, text } => {
                if let Some(view) = self.window_of(frame).and_then(|id| self.views.get_mut(&id)) {
                    view.tooltip = text;
                }
                Task::none()
            }
            Command::ResizeFrame {
                frame,
                width,
                height,
            } => match self.window_of(frame) {
                Some(id) => window::resize(id, Size::new(width as f32, height as f32)),
                None => Task::none(),
            },
            Command::Present {
                frame,
                width,
                height,
                pixels,
            } => {
                let Some(id) = self.window_of(frame) else {
                    return Task::none();
                };
                let Some(view) = self.views.get_mut(&id) else {
                    return Task::none();
                };
                view.presented += 1;
                let seq = view.presented;
                let handle = Handle::from_rgba(width, height, pixels);
                image::allocate(handle.clone()).map(move |result| Message::Allocated {
                    id,
                    seq,
                    handle: handle.clone(),
                    width,
                    height,
                    allocation: result.ok(),
                })
            }
            Command::SetFullscreen { frame, on } => match self.window_of(frame) {
                Some(id) => {
                    if let Some(view) = self.views.get_mut(&id) {
                        view.fullscreen = on;
                    }
                    window::set_mode(
                        id,
                        if on {
                            window::Mode::Fullscreen
                        } else {
                            window::Mode::Windowed
                        },
                    )
                }
                None => Task::none(),
            },
            Command::SetMaximized { frame, on } => match self.window_of(frame) {
                Some(id) => window::maximize(id, on),
                None => Task::none(),
            },
            Command::SetCursor { frame, cursor } => {
                if let Some(view) = self.window_of(frame).and_then(|id| self.views.get_mut(&id)) {
                    view.interaction = cursor;
                }
                Task::none()
            }
            Command::SetToolBar { frame, bar } => {
                self.replace_tool_bar(frame, Some(bar));
                Task::none()
            }
            Command::RemoveToolBar { frame } => {
                self.replace_tool_bar(frame, None);
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
        let Some((frame, popup)) = self.views.get(&id).map(|v| (v.frame, v.popup)) else {
            return;
        };
        if popup {
            if event == window::Event::Closed {
                self.views.remove(&id);
                self.frames.remove(&frame);
            }
            return;
        }
        match event {
            window::Event::Opened { size, .. } => {
                if let Some(view) = self.views.get_mut(&id) {
                    view.size = size;
                }
                self.configure(id);
                self.notify(Notice::Map { frame });
            }
            window::Event::Resized(size) => {
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
        if let keyboard::Event::ModifiersChanged(modifiers) = event {
            self.modifiers = modifiers;
        }
        let Some(frame) = self.views.get(&id).filter(|v| !v.popup).map(|v| v.frame) else {
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
        let Some(view) = self.views.get(&id) else {
            return space().into();
        };
        let canvas = frame_canvas(view);
        let (Some(position), Some(bar)) = (view.tool_bar_position(), view.tool_bar.as_ref()) else {
            return canvas;
        };
        let vertical = matches!(position, ToolBarPosition::Left | ToolBarPosition::Right);
        let strip: Element<'_, Message> =
            sensor(tool_bar_strip(id, bar, &view.tool_bar_images, vertical))
                .on_show(move |size| Message::ToolBarAllocated(id, size))
                .on_resize(move |size| Message::ToolBarAllocated(id, size))
                .into();
        match position {
            ToolBarPosition::Top => column([strip, canvas]).into(),
            ToolBarPosition::Bottom => column([canvas, strip]).into(),
            ToolBarPosition::Left => row([strip, canvas]).into(),
            ToolBarPosition::Right => row([canvas, strip]).into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            Subscription::run(bridge),
            window::events().map(|(id, event)| Message::Window(id, event)),
            event::listen_with(|event, _status, id| match event {
                Event::Keyboard(key) => Some(Message::Key(id, key)),
                Event::Mouse(mouse) => Some(Message::Mouse(id, mouse)),
                _ => None,
            }),
        ])
    }
}

fn frame_canvas(view: &View) -> Element<'_, Message> {
    let Some((handle, width, height)) = &view.image else {
        return space().into();
    };
    let content = mouse_area(
        image(handle.clone())
            .width(Length::Fixed(*width as f32))
            .height(Length::Fixed(*height as f32))
            .content_fit(ContentFit::Fill)
            .filter_method(if view.scale.fract() == 0.0 {
                FilterMethod::Nearest
            } else {
                FilterMethod::Linear
            }),
    )
    .interaction(view.interaction);
    match &view.tooltip {
        Some(tip) => tooltip(
            content,
            container(text(tip.as_str()))
                .padding(4)
                .style(container::bordered_box),
            tooltip::Position::FollowCursor,
        )
        .into(),
        None => content.into(),
    }
}

fn tool_bar_strip<'a>(
    id: window::Id,
    bar: &'a ToolBar,
    images: &'a [Option<Handle>],
    vertical: bool,
) -> Element<'a, Message> {
    let items = bar
        .items
        .iter()
        .zip(images)
        .map(|(item, handle)| match item {
            ToolItem::Separator => {
                if vertical {
                    rule::horizontal(1).into()
                } else {
                    rule::vertical(1).into()
                }
            }
            ToolItem::Button {
                index,
                image,
                label,
                enabled,
            } => tool_bar_button(
                id,
                bar,
                *index,
                image.as_ref().zip(handle.as_ref()),
                label.as_deref(),
                *enabled,
            ),
        });
    if vertical {
        container(column(items).align_x(Alignment::Center))
            .height(Length::Fill)
            .into()
    } else {
        container(row(items).align_y(Alignment::Center))
            .width(Length::Fill)
            .into()
    }
}

fn tool_bar_button<'a>(
    id: window::Id,
    bar: &ToolBar,
    index: usize,
    picture: Option<(&crate::protocol::ToolBarImage, &Handle)>,
    label: Option<&'a str>,
    enabled: bool,
) -> Element<'a, Message> {
    let picture: Option<Element<'a, Message>> = picture.map(|(picture, handle)| {
        container(
            image(handle.clone())
                .width(Length::Fixed(picture.width as f32))
                .height(Length::Fixed(picture.height as f32))
                .opacity(if enabled { 1.0_f32 } else { 0.5_f32 }),
        )
        .padding(iced::Padding::from([
            bar.vmargin as f32,
            bar.hmargin as f32,
        ]))
        .into()
    });
    let caption: Option<Element<'a, Message>> = label.map(|label| {
        let caption = text(label).size(TOOL_BAR_LABEL_SIZE);
        if enabled {
            caption.into()
        } else {
            caption.style(text::secondary).into()
        }
    });
    let parts: Vec<Element<'a, Message>> = if bar.text_image {
        [caption, picture]
    } else {
        [picture, caption]
    }
    .into_iter()
    .flatten()
    .collect();
    let content: Element<'a, Message> = if bar.horiz {
        row(parts).spacing(2).align_y(Alignment::Center).into()
    } else {
        column(parts).align_x(Alignment::Center).into()
    };
    let pressable = button(content)
        .style(button::text)
        .padding(TOOL_BAR_BUTTON_PADDING)
        .on_press_maybe(enabled.then_some(Message::ToolBarClicked(id, index)));
    mouse_area(pressable)
        .on_enter(Message::ToolBarHover(id, Some(index)))
        .on_exit(Message::ToolBarHover(id, None))
        .into()
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
