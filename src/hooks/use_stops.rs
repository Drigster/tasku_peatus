use freya::{prelude::*, radio::Radio};

use crate::{
    launch_config::{Data, DataChannel},
    utils::transit::parsers::{
        routes::{DEPOT_DESTINATION_KEY, get_routes},
        stops::{get_stops, get_stops_in_radius},
    },
};

pub fn use_stops(radio: &Radio<Data, DataChannel>) {
    let stops = radio.slice_mut(DataChannel::StopsUpdate, |s| &mut s.transit_data.stops);
    let mut depot_destinations = radio.slice_mut(DataChannel::StopsUpdate, |s| {
        &mut s.transit_data.depot_destinations
    });
    use_hook(|| {
        let mut stops = stops.clone();
        spawn(async move {
            if !stops.read().is_empty() {
                return;
            }

            // Independent downloads, so run them concurrently rather than
            // serializing two network round trips at startup.
            let (new_stops, routes) = smol::future::zip(get_stops(), get_routes()).await;
            let mut new_stops = new_stops.unwrap();
            let routes = routes.unwrap();

            for (stop_id, route) in routes {
                if let Some(stop) = new_stops.get_mut(&stop_id) {
                    stop.routes = route;
                }
            }

            // Collected once here rather than rescanned per render: the set is
            // a property of the network, not of any one stop.
            *depot_destinations.write() = new_stops
                .values()
                .flat_map(|stop| &stop.routes)
                .filter(|route| route.destination_key == DEPOT_DESTINATION_KEY)
                .map(|route| route.destination_name.clone())
                .collect();

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
        let current_location = location.read().unwrap();

        // Borrow the stop database rather than cloning it: it holds ~3.7k stops
        // with every route and timetable entry attached, and this effect runs on
        // every published location update.
        let new_stops_radius =
            get_stops_in_radius(&stops.read(), current_location.0, current_location.1, 150.0);

        stops_radius.set_if_modified(new_stops_radius);
    });
}
