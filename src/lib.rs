#[cfg(target_os = "android")]
use chrono::{DateTime, Utc};
#[cfg(target_os = "android")]
use freya::{
    prelude::*,
    radio::{RadioChannel, RadioStation},
};
#[cfg(target_os = "android")]
use smol::stream::StreamExt;
#[cfg(target_os = "android")]
use std::collections::HashMap;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
mod app;
#[cfg(target_os = "android")]
mod components;
#[cfg(target_os = "android")]
mod layouts;
#[cfg(target_os = "android")]
mod pages;
#[cfg(target_os = "android")]
mod utils;
#[cfg(target_os = "android")]
use app::MyApp;

#[cfg(target_os = "android")]
use crate::utils::{
    departures_parser::Departure, routes_parser::DepartureTimes, stops_parser::Stop,
};

#[cfg(target_os = "android")]
pub static APP_DIR_NAME: &str = "TaskuPeatus";

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(droid_app: AndroidApp) {
    use freya::android::AndroidPlugin;
    use freya_winit::renderer::NativeEvent;
    use winit::{event_loop::EventLoop, platform::android::EventLoopBuilderExtAndroid};

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug),
    );

    let event_loop = EventLoop::<NativeEvent>::with_user_event()
        .with_android_app(droid_app.clone())
        .build()
        .expect("Failed to build event loop");

    let mut radio_station = RadioStation::create_global(Data::default());

    let (state_tx, mut state_rx) = futures_channel::mpsc::unbounded::<ChannelSend>();

    radio_station.write_channel(DataChannel::NoUpdate).state_tx = Some(state_tx.clone());

    launch(
        LaunchConfig::new()
            .with_plugin(AndroidPlugin::new(droid_app))
            .with_future(move |_| async move {
                while let Some(channel_data) = state_rx.next().await {
                    match channel_data {
                        ChannelSend::LocationUpdate(location) => {
                            radio_station
                                .write_channel(DataChannel::LocationUpdate)
                                .location = Some(location);
                        }
                        ChannelSend::LocationEnabledUpdate(enabled) => {
                            radio_station
                                .write_channel(DataChannel::LocationEnabledUpdate)
                                .is_location_enabled = enabled;
                        }
                    }
                }
            })
            .with_window(WindowConfig::new_app(MyApp { radio_station }))
            .with_event_loop(event_loop),
    )
}

#[cfg(target_os = "android")]
#[derive(Debug, Clone)]
pub enum ErrorState {
    NoPermissions,
    LocationWatcherError(String),
    NoLocation(String),
    StopsUpdateError(String),
}

#[cfg(target_os = "android")]
#[derive(Default, Clone)]
pub struct Data {
    stops: Vec<Stop>,
    stops_radius: HashMap<String, Stop>,
    stops_distances: HashMap<String, u64>,
    departures: HashMap<String, Vec<Departure>>,
    departures_next_update: DateTime<Utc>,
    routes: HashMap<String, HashMap<String, DepartureTimes>>,

    location: Option<(f64, f64)>,

    is_location_enabled: bool,
    error_state: Option<ErrorState>,

    state_tx: Option<futures_channel::mpsc::UnboundedSender<ChannelSend>>,
}

#[cfg(target_os = "android")]
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum DataChannel {
    NoUpdate,
    StopsUpdate,
    StopsRadiusUpdate,
    StopsDistancesUpdate(String),
    DeparturesUpdate,
    DepartureUpdate(String),
    LocationUpdate,
    LocationEnabledUpdate,
    ErrorStateUpdate,
    RoutesUpdate,
}

#[cfg(target_os = "android")]
impl RadioChannel<Data> for DataChannel {}

#[cfg(target_os = "android")]
pub enum ChannelSend {
    LocationUpdate((f64, f64)),
    LocationEnabledUpdate(bool),
}
