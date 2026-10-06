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
    let mut fetched_at = radio.slice_mut(DataChannel::DeparturesUpdate, |s| {
        &mut s.departures_fetched_at
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
                    // Stops are still downloading/parsing. Back off rather than
                    // re-checking every 10ms, which pegged the UI executor at
                    // ~100 wake-ups per second for the whole startup window.
                    next_update += Duration::from_millis(500);
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
                // Written before the departures themselves; no await separates
                // them, so no render can observe one without the other.
                *fetched_at.write() = Utc::now().timestamp();
                *departures.write() = stops_departures.0;

                // println!("Departures: {:?}", stops_departures.0);

                // let _ = fs::write(
                //     "departures.json",
                //     serde_json::to_string(&stops_departures.0).unwrap(),
                // );
            }
        });
    });
}
