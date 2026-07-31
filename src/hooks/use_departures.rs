use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use freya::{prelude::*, radio::use_radio};

use crate::{
    launch_config::DataChannel,
    utils::{departures_parser::get_departures, stops_parser},
};

pub fn use_departures() {
    let mut radio = use_radio(DataChannel::NoUpdate);
    let mut stops_radius = radio.slice_mut(DataChannel::StopsRadiusUpdate, |s| &mut s.stops_radius);
    let mut departures_next_update = radio.slice_mut(DataChannel::DeparturesUpdate, |s| {
        &mut s.departures_next_update
    });
    let mut departures = radio.slice_mut(DataChannel::DeparturesUpdate, |s| &mut s.departures);
    let location = radio.slice_mut(DataChannel::LocationUpdate, |s| &mut s.location);
    let stops = radio.slice_mut(DataChannel::StopsUpdate, |s| &mut s.stops);

    use_hook(|| {
        spawn(async move {
            let mut last_location = None;
            let mut next_update = Utc::now();
            loop {
                if next_update > Utc::now() {
                    smol::Timer::after((next_update - Utc::now()).to_std().unwrap()).await;
                    continue;
                }

                let location = location.read().cloned();
                if location.is_none() {
                    next_update += Duration::from_millis(10);
                    continue;
                }

                if stops.read().is_empty() {
                    next_update += Duration::from_millis(10);
                    continue;
                }
                next_update += Duration::from_secs(5);

                if let Some(current_location) = location
                    && location != last_location
                {
                    let cur_stops = stops.read().clone();
                    if cur_stops.is_empty() {
                        continue;
                    }
                    last_location = location;

                    let (new_stops_radius, new_stops_distances) = stops_parser::get_stops_in_radius(
                        cur_stops,
                        current_location.0,
                        current_location.1,
                        150.0,
                    );

                    {
                        stops_radius.set_if_modified(new_stops_radius);
                        for (id, distance) in new_stops_distances {
                            radio
                                .write_channel(DataChannel::StopsDistancesUpdate(id.clone()))
                                .stops_distances
                                .insert(id, distance);
                        }
                    }
                }

                let cur_departures_next_update = departures_next_update.read().cloned();

                if cur_departures_next_update <= Utc::now() {
                    let stops_departures = match get_departures(stops_radius.read().cloned()).await
                    {
                        Ok(stops_departures) => stops_departures,
                        Err(e) => {
                            log::error!("Error getting departures: {e}");
                            (HashMap::new(), 30)
                        }
                    };

                    *departures_next_update.write() =
                        Utc::now() + Duration::from_secs(stops_departures.1.into());
                    *departures.write() = stops_departures.0;
                }
            }
        });
    });
}
