use freya::{
    prelude::*,
    radio::{RadioChannel, RadioStation},
};
use smol::stream::StreamExt;

use crate::{app::MyApp, utils::transit::TransitData};

pub static APP_DIR_NAME: &str = "tasku_peatus";

#[allow(dead_code)]
pub fn build_launch_config() -> freya::prelude::LaunchConfig {
    let mut radio_station = RadioStation::create_global(Data::default());

    let (state_tx, mut state_rx) = futures_channel::mpsc::unbounded::<ChannelSend>();

    radio_station.write_channel(DataChannel::NoUpdate).state_tx = Some(state_tx.clone());

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
        .with_window(
            WindowConfig::new_app(MyApp { radio_station })
                .with_size(420.0, 900.0)
                .with_custom_scale_factor(if cfg!(feature = "scaled") { 2.375 } else { 1.0 })
                .with_decorations(false),
        )
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppState {
    LocationDisabled,
    WitingForLocation,
    LocationError(String),
}

#[derive(Default, Clone)]
pub struct Data {
    pub transit_data: TransitData,

    pub is_location_enabled: bool,
    pub location: Option<(f64, f64)>,

    pub state: Option<AppState>,

    pub state_tx: Option<futures_channel::mpsc::UnboundedSender<ChannelSend>>,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
#[allow(dead_code)]
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
    StateUpdate,
}

impl RadioChannel<Data> for DataChannel {}

#[allow(dead_code)]
pub enum ChannelSend {
    LocationUpdate((f64, f64)),
    LocationEnabledUpdate(bool),
}
