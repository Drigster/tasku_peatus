use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use freya::{prelude::*, radio::Radio};

use crate::{
    launch_config::{Data, DataChannel},
    utils::transit::parsers::departures::get_departures,
};

pub fn use_departures(radio: &Radio<Data, DataChannel>) {
    let stops_radius = radio.slice(DataChannel::StopsRadiusUpdate, |s| {
        &s.transit_data.stops_radius
    });
    let mut departures = radio.slice_mut(DataChannel::DeparturesUpdate, |s| {
        &mut s.transit_data.departures
    });

    use_hook(|| {
        spawn(async move {
            let mut next_update = Utc::now();
            loop {
                if next_update > Utc::now() {
                    smol::Timer::after((next_update - Utc::now()).to_std().unwrap()).await;
                    continue;
                }

                if stops_radius.read().is_empty() {
                    next_update += Duration::from_millis(10);
                    continue;
                }

                let siri_ids = stops_radius
                    .read()
                    .iter()
                    .map(|e| e.siri_id.clone())
                    .collect();

                let stops_departures = match get_departures(siri_ids).await {
                    Ok(stops_departures) => stops_departures,
                    Err(e) => {
                        log::error!("Error getting departures: {e}");
                        (HashMap::new(), 30)
                    }
                };

                next_update = Utc::now() + Duration::from_secs(stops_departures.1.into());
                *departures.write() = stops_departures.0;
            }
        });
    });
}
