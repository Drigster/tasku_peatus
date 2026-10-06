use freya::{
    prelude::*,
    radio::{RadioChannel, RadioStation},
};
use geo::{Distance, Haversine, Point};
use smol::stream::StreamExt;

use crate::{app::MyApp, utils::transit::TransitData};

pub static APP_DIR_NAME: &str = "tasku_peatus";

/// Minimum movement before a new fix is published. Every published location
/// update recomputes the in-radius stop list, and Android delivers fixes as
/// often as once per second, so near-identical fixes are dropped.
const LOCATION_UPDATE_MIN_METERS: f64 = 10.0;

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
                        let has_moved = {
                            let data = radio_station.read();
                            match data.location {
                                None => true,
                                Some(previous) => {
                                    Haversine.distance(
                                        Point::new(previous.1, previous.0),
                                        Point::new(location.1, location.0),
                                    ) >= LOCATION_UPDATE_MIN_METERS
                                }
                            }
                        };

                        if has_moved {
                            radio_station
                                .write_channel(DataChannel::LocationUpdate)
                                .location = Some(location);
                        }
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

#[derive(Debug, Clone, PartialEq)]
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

    /// Epoch seconds, republished once per second by `use_ticker`. Countdowns
    /// derive their remaining time from this rather than each keeping a timer.
    pub now: i64,
    /// Epoch seconds at which `transit_data.departures` was fetched, so a row
    /// can tell how much of its `until` has already elapsed.
    pub departures_fetched_at: i64,

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
    TickUpdate,
}

impl RadioChannel<Data> for DataChannel {}

#[allow(dead_code)]
pub enum ChannelSend {
    LocationUpdate((f64, f64)),
    LocationEnabledUpdate(bool),
}
