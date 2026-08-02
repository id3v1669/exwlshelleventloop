# iced_wayland_subscriber

used to subscribe the wayland event

## Example

```rust
use std::collections::HashMap;

use iced::widget::{button, column, container, row, text, text_input};
use iced::window::Id;
use iced::{Alignment, Element, Event, Length, Task as Command, event};
use iced_layershell::actions::{IcedNewPopupSettings, IcedXdgWindowSettings};
use iced_runtime::window::Action as WindowAction;
use iced_runtime::{Action, task};

use iced_layershell::daemon;
use iced_layershell::reexport::{
    Anchor, KeyboardInteractivity, Layer, LayerSize, NewLayerShellSettings, OutputOption,
    PixelSize, PopupGravity,
};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use iced_wayland_subscriber::{OutputInfo, output::OutputEvent};
use wayland_client::Connection;

pub fn main() -> Result<(), iced_layershell::Error> {
    tracing_subscriber::fmt().init();
    let connection = Connection::connect_to_env().unwrap();
    let connection2 = connection.clone();
    daemon(
        move || Counter::new("hello", connection.clone()),
        Counter::namespace,
        Counter::update,
        Counter::view,
    )
    .title(Counter::title)
    .subscription(Counter::subscription)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            size: LayerSize::fill_width(400),
            exclusive_zone: 400,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            start_mode: StartMode::AllScreens,
            ..Default::default()
        },
        with_connection: Some(connection2.into()),
        ..Default::default()
    })
    .run()
}

#[derive(Debug)]
struct Counter {
    value: i32,
    text: String,
    ids: HashMap<iced::window::Id, WindowInfo>,
    connection: Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowInfo {
    Left,
    NormalWindow,
    PopUp,
    TopBar,
}

#[derive(Debug, Clone, Copy)]
enum WindowDirection {
    Top(Id),
    Left(Id),
    Right(Id),
    Bottom(Id),
}

#[derive(Debug, Clone)]
enum WayEvent {
    OutputInsert(OutputInfo),
    Stop(String),
}

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    NewWindowLeft,
    NewNormalWindow,
    Close(Id),
    WindowClosed(Id),
    TextInput(String),
    Direction(WindowDirection),
    IcedEvent(Event),
    Wayland(WayEvent),
}

impl Counter {
    fn window_id(&self, info: &WindowInfo) -> Option<&iced::window::Id> {
        for (k, v) in self.ids.iter() {
            if info == v {
                return Some(k);
            }
        }
        None
    }
}

impl Counter {
    fn new(text: &str, connection: Connection) -> Self {
        Self {
            value: 0,
            text: text.to_string(),
            ids: HashMap::new(),
            connection,
        }
    }

    fn title(&self, id: iced::window::Id) -> Option<String> {
        if let Some(WindowInfo::NormalWindow) = self.id_info(id) {
            return Some("hello, it is a normal window".to_owned());
        }
        None
    }

    fn id_info(&self, id: iced::window::Id) -> Option<WindowInfo> {
        self.ids.get(&id).cloned()
    }

    fn remove_id(&mut self, id: iced::window::Id) {
        self.ids.remove(&id);
    }

    fn namespace() -> String {
        String::from("Counter - Iced")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch(vec![
            event::listen().map(Message::IcedEvent),
            iced::window::close_events().map(Message::WindowClosed),
            iced_wayland_subscriber::output::listen(self.connection.clone()).filter_map(
                |event| match event {
                    OutputEvent::Insert(output) => {
                        Some(Message::Wayland(WayEvent::OutputInsert(output)))
                    }
                    OutputEvent::Stop(error) => {
                        Some(Message::Wayland(WayEvent::Stop(error.to_string())))
                    }
                    // Changed/Removed need nothing here: the compositor closes
                    // a destroyed output's layer surfaces, which arrives as
                    // WindowClosed.
                    _ => None,
                },
            ),
        ])
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        use iced::Event;
        use iced::keyboard;
        use iced::keyboard::key::Named;
        match message {
            Message::WindowClosed(id) => {
                self.remove_id(id);
                Command::none()
            }
            Message::IcedEvent(event) => {
                match event {
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(Named::Escape),
                        ..
                    }) => {
                        if let Some(id) = self.window_id(&WindowInfo::Left) {
                            return iced_runtime::task::effect(Action::Window(
                                WindowAction::Close(*id),
                            ));
                        }
                    }
                    Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right)) => {
                        if let Some(parent) = self.window_id(&WindowInfo::Left).copied() {
                            let id = iced::window::Id::unique();
                            self.ids.insert(id, WindowInfo::PopUp);
                            return Command::done(Message::NewPopUp {
                                settings: IcedNewPopupSettings::new(
                                    parent,
                                    PixelSize::px(100, 100),
                                    (0, 0),
                                    PixelSize::px(1, 1),
                                )
                                .gravity(PopupGravity::TopRight),
                                id,
                            });
                        }
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::Wayland(WayEvent::Stop(error)) => {
                eprintln!("output subscription stopped: {error}");
                Command::none()
            }
            Message::Wayland(WayEvent::OutputInsert(output)) => {
                let id = iced::window::Id::unique();
                self.ids.insert(id, WindowInfo::TopBar);
                Command::done(Message::NewLayerShell {
                    settings: NewLayerShellSettings {
                        anchor: Anchor::Left | Anchor::Right | Anchor::Top,
                        layer: Layer::Top,
                        exclusive_zone: Some(30),
                        size: LayerSize::fill_width(30),
                        output_option: OutputOption::GlobalName(output.id),
                        ..Default::default()
                    },
                    id,
                })
            }
            Message::IncrementPressed => {
                self.value += 1;
                Command::none()
            }
            Message::DecrementPressed => {
                self.value -= 1;
                Command::none()
            }
            Message::TextInput(text) => {
                self.text = text;
                Command::none()
            }
            Message::Direction(direction) => match direction {
                WindowDirection::Left(id) => Command::done(Message::LayoutChange {
                    id,
                    anchor: Anchor::Left,
                    size: LayerSize::fill_height(400),
                }),
                WindowDirection::Right(id) => Command::done(Message::LayoutChange {
                    id,
                    anchor: Anchor::Right,
                    size: LayerSize::fill_height(400),
                }),
                WindowDirection::Bottom(id) => Command::done(Message::LayoutChange {
                    id,
                    anchor: Anchor::Bottom,
                    size: LayerSize::fill_width(400),
                }),
                WindowDirection::Top(id) => Command::done(Message::LayoutChange {
                    id,
                    anchor: Anchor::Top,
                    size: LayerSize::fill_width(400),
                }),
            },
            Message::NewWindowLeft => {
                let id = iced::window::Id::unique();
                self.ids.insert(id, WindowInfo::Left);
                Command::done(Message::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: LayerSize::px(100, 100),
                        exclusive_zone: None,
                        anchor: Anchor::Left | Anchor::Bottom,
                        layer: Layer::Top,
                        margin: None,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        output_option: OutputOption::LastOutput,
                        ..Default::default()
                    },
                    id,
                })
            }
            Message::NewNormalWindow => {
                let (id, task) = Message::base_window_open(IcedXdgWindowSettings::default());
                self.ids.insert(id, WindowInfo::NormalWindow);
                task
            }
            Message::Close(id) => task::effect(Action::Window(WindowAction::Close(id))),
            _ => unreachable!(),
        }
    }

    fn view(&self, id: iced::window::Id) -> Element<'_, Message> {
        if let Some(WindowInfo::Left) = self.id_info(id) {
            return button("close left").on_press(Message::Close(id)).into();
        }
        if let Some(WindowInfo::NormalWindow) = self.id_info(id) {
            return container(
                column![
                    text_input("hello", &self.text)
                        .on_input(Message::TextInput)
                        .padding(10),
                    button("close the normal window").on_press(Message::Close(id)),
                ]
                .align_x(Alignment::Center)
                .padding(20),
            )
            .center_y(Length::Fill)
            .center_x(Length::Fill)
            .height(Length::Fill)
            .into();
        }
        if let Some(WindowInfo::TopBar) = self.id_info(id) {
            return text("hello here is topbar").into();
        }
        if let Some(WindowInfo::PopUp) = self.id_info(id) {
            return container(button("close PopUp").on_press(Message::Close(id)))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Color::from_rgba(0., 0.5, 0.7, 0.6).into()),
                    ..Default::default()
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
        let center = column![
            button("Increment").on_press(Message::IncrementPressed),
            button("Decrement").on_press(Message::DecrementPressed),
            text(self.value).size(50),
            button("newwindowLeft").on_press(Message::NewWindowLeft),
            button("new normal window").on_press(Message::NewNormalWindow),
        ]
        .align_x(Alignment::Center)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);
        row![
            button("left")
                .on_press(Message::Direction(WindowDirection::Left(id)))
                .height(Length::Fill),
            column![
                button("top")
                    .on_press(Message::Direction(WindowDirection::Top(id)))
                    .width(Length::Fill),
                center,
                text_input("hello", &self.text)
                    .on_input(Message::TextInput)
                    .padding(10),
                button("bottom")
                    .on_press(Message::Direction(WindowDirection::Bottom(id)))
                    .width(Length::Fill),
            ]
            .width(Length::Fill),
            button("right")
                .on_press(Message::Direction(WindowDirection::Right(id)))
                .height(Length::Fill),
        ]
        .padding(20)
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
```

## Workspaces

The subscriber also tracks `ext_workspace_manager_v1` when the compositor
implements it. A snapshot of the whole workspace tree arrives as
`WorkspaceEvent::Updated` once per compositor-side change. The
protocol's `done` event is an atomic barrier, so the snapshot is never torn
mid-update. If the protocol is absent you get a single `WorkspaceEvent::Unsupported`
instead, and output events continue as normal. Both `Unsupported` and `Finished`
end the subscription: neither is followed by anything, so the stream has nothing 
left to deliver.

```rust
match event {
    WorkspaceEvent::Updated(snapshot) => {
        for group in &snapshot.groups {
            // Which monitors this group covers.
            let outputs = &group.outputs;
            // The workspace to highlight for those monitors.
            let active = snapshot.active_in(&group.id);
            let _ = (outputs, active);
        }
        self.workspaces = Some(snapshot);
    }
    WorkspaceEvent::Unsupported => {
        // Hide workspace UI.
    }
    _ => {}
}
```

To switch workspace, call the request methods on a stored snapshot. Requests go
straight onto the connection from wherever you call them, so `update()` is fine:

```rust
snapshot.activate(&workspace_id)?;
```

Each method checks the capability the compositor advertised and returns
`RequestError::Unsupported` rather than doing nothing, because the compositor
itself silently ignores requests it does not support.
Also available: `deactivate`, `remove`, `assign` (which moves a
workspace to another group, not a window) and `create_workspace`.

See `iced_examples/workspace_bar` for a working bar.

## Feature flags

Because `ext_workspace_manager_v1` is not that widely supported,
workspace subscription is behind the non-default `workspace` feature:

```toml
iced_wayland_subscriber = { version = "…", features = ["workspace"] }
```
