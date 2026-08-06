use freya::{prelude::*, radio::Radio};

use crate::{
    launch_config::{Data, DataChannel},
    utils::transit::parsers::stops::{get_stops, get_stops_in_radius},
};

pub fn use_stops(radio: &Radio<Data, DataChannel>) {
    let stops = radio.slice_mut(DataChannel::StopsUpdate, |s| &mut s.transit_data.stops);
    use_hook(|| {
        let mut stops = stops.clone();
        spawn(async move {
            if !stops.read().is_empty() {
                return;
            }

            let new_stops = get_stops().await.unwrap();

            *stops.write() = new_stops;
        });
    });

    let mut stops_radius = radio.slice_mut(DataChannel::StopsRadiusUpdate, |s| {
        &mut s.transit_data.stops_radius
    });
    let location = radio.slice(DataChannel::LocationUpdate, |s| &s.location);
    use_side_effect(move || {
        if stops.read().is_empty() || location.read().is_none() {
            return;
        }
        let cur_stops = stops.read().cloned();
        let current_location = location.read().unwrap();

        let new_stops_radius =
            get_stops_in_radius(cur_stops, current_location.0, current_location.1, 150.0);

        stops_radius.set_if_modified(new_stops_radius);
    });
}
