use futures::SinkExt;
use wayland_client::{
    delegate_noop,
    protocol::{
        wl_output::{self, WlOutput},
        wl_registry,
    },
    Connection, Dispatch, Proxy,
};

use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1;

#[derive(Debug, Default)]
struct SubscribeState {
    events: Vec<WaylandEvents>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for SubscribeState {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == wl_output::WlOutput::interface().name {
                    let output = proxy.bind::<wl_output::WlOutput, _, _>(name, version, qh, ());
                    state.events.push(WaylandEvents::OutputInsert(output));
                }
            }
            wl_registry::Event::GlobalRemove { .. } => {}
            _ => unreachable!(),
        }
    }
}
delegate_noop!(SubscribeState: ignore WlOutput); // output is need to place layer_shell, although here
delegate_noop!(SubscribeState: ignore ZxdgOutputManagerV1);

#[derive(Debug, Clone)]
pub enum WaylandEvents {
    OutputInsert(wl_output::WlOutput),
}

pub fn listen() -> iced::Subscription<WaylandEvents> {
    iced::Subscription::run(|| {
        iced::stream::channel(100, |mut output| async move {
            let connection = Connection::connect_to_env().unwrap();
            let mut state = SubscribeState::default();

            let mut event_queue = connection.new_event_queue::<SubscribeState>();
            let qhandle = event_queue.handle();
            let display = connection.display();

            display.get_registry(&qhandle, ());
            loop {
                event_queue.blocking_dispatch(&mut state).unwrap();
                let mut current_events = vec![];
                std::mem::swap(&mut current_events, &mut state.events);
                for event in current_events {
                    output.send(event).await.ok();
                }
                async_io::Timer::after(std::time::Duration::from_millis(10)).await;
            }
        })
    })
}
