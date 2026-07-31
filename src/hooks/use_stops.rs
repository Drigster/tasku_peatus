use freya::{prelude::*, radio::use_radio};

use crate::{launch_config::DataChannel, utils::stops_parser};

pub fn use_stops() {
    let radio = use_radio(DataChannel::NoUpdate);
    let stops = radio.slice_mut(DataChannel::StopsUpdate, |s| &mut s.stops);
    use_hook(|| {
        let mut stops = stops.clone();
        spawn(async move {
            if !stops.read().is_empty() {
                return;
            }

            let new_stops = stops_parser::get_stops().await.unwrap();
            *stops.write() = new_stops;
        });
    });
}
