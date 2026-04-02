use chrono::{DateTime, Utc};
use freya::{
    prelude::*,
    radio::{RadioChannel, RadioStation},
};
use smol::stream::StreamExt;
use std::collections::HashMap;

mod app;
mod components;
mod layouts;
mod pages;
mod utils;

use app::MyApp;

use crate::utils::{departures_parser::Departure, stops_parser::Stop};

pub static APP_DIR_NAME: &str = "TaskuPeatus";

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
fn main() {
    env_logger::init();

    let mut radio_station = RadioStation::create_global(Data::default());

    let (state_tx, mut state_rx) = futures_channel::mpsc::unbounded::<ChannelSend>();

    radio_station.write_channel(DataChannel::NoUpdate).state_tx = Some(state_tx.clone());

    launch(
        LaunchConfig::new()
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
            .with_window(WindowConfig::new_app(MyApp { radio_station }).with_size(420.0, 900.0)),
    )
}

#[cfg(target_os = "android")]
use freya::prelude::{LaunchConfig, WindowConfig, launch};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(droid_app: AndroidApp) {
    use freya_winit::renderer::NativeEvent;
    use winit::{event_loop::EventLoop, platform::android::EventLoopBuilderExtAndroid};

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug),
    );

    let mut event_loop_builder = EventLoop::<NativeEvent>::with_user_event();
    event_loop_builder.with_android_app(droid_app);

    let mut radio_station = RadioStation::create_global(Data::default());

    let (state_tx, mut state_rx) = futures_channel::mpsc::unbounded::<ChannelSend>();

    radio_station.write_channel(DataChannel::NoUpdate).state_tx = Some(state_tx.clone());

    launch(
        LaunchConfig::new()
            .with_future(move |proxy| async move {
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
            .with_window(WindowConfig::new_app(MyApp { radio_station }).with_size(500.0, 450.0))
            .with_event_loop_builder(event_loop_builder),
    )
}

#[derive(Debug, Clone)]
pub enum ErrorState {
    NoPermissions,
    LocationWatcherError(String),
    NoLocation(String),
    StopsUpdateError(String),
}

#[derive(Default, Clone)]
pub struct Data {
    stops: Vec<Stop>,
    stops_radius: Vec<Stop>,
    stops_distances: HashMap<String, u64>,
    departures: HashMap<String, Vec<Departure>>,
    departures_next_update: DateTime<Utc>,
    location: Option<(f64, f64)>,

    is_location_enabled: bool,
    error_state: Option<ErrorState>,

    state_tx: Option<futures_channel::mpsc::UnboundedSender<ChannelSend>>,
}

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
}

impl RadioChannel<Data> for DataChannel {}

pub enum ChannelSend {
    LocationUpdate((f64, f64)),
    LocationEnabledUpdate(bool),
}
