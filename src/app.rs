use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use freya::{
    prelude::*,
    radio::{RadioStation, use_radio, use_share_radio},
    router::{Routable, RouterConfig},
};
use freya_router::prelude::Router;

use crate::ChannelSend;
use crate::{
    Data, DataChannel,
    utils::{
        departures_parser::get_departures,
        routes_parser::{self},
        stops_parser::{self, Stop},
    },
};
use crate::{layouts::AppLayout, pages::Timetable};

fn custom_theme() -> Theme {
    let mut theme = dark_theme();
    theme.name = "custom";
    theme.colors = ColorsSheet {
        primary: Color::from_rgb(227, 227, 227),
        secondary: Color::from_rgb(49, 161, 218),
        text_primary: Color::BLACK,
        text_secondary: Color::WHITE,
        ..DARK_COLORS
    };
    theme
}

pub struct MyApp {
    pub radio_station: RadioStation<Data, DataChannel>,
}
impl App for MyApp {
    fn render(&self) -> impl IntoElement {
        use_share_radio(move || self.radio_station);

        use_init_theme(|| custom_theme());

        let mut radio = use_radio(DataChannel::NoUpdate);
        let location = radio.slice(DataChannel::LocationUpdate, |s| &s.location);
        let stops = radio.slice_mut(DataChannel::StopsUpdate, |s| &mut s.stops);
        let mut stops_radius =
            radio.slice_mut(DataChannel::StopsRadiusUpdate, |s| &mut s.stops_radius);
        let mut routes = radio.slice_mut(DataChannel::RoutesUpdate, |s| &mut s.routes);
        let mut departures_next_update = radio.slice_mut(DataChannel::DeparturesUpdate, |s| {
            &mut s.departures_next_update
        });
        let mut departures = radio.slice_mut(DataChannel::DeparturesUpdate, |s| &mut s.departures);

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

        use_hook(|| {
            spawn(async move {
                if !routes.read().is_empty() {
                    return;
                }

                let new_routes = routes_parser::get_routes().await.unwrap();
                *routes.write() = new_routes;
            });
        });

        #[cfg(target_os = "android")]
        let state_tx = radio.read().state_tx.clone().unwrap();
        #[cfg(target_os = "android")]
        {
            let state_tx_clone = state_tx.clone();
            use_hook(move || {
                use crate::ChannelSend;
                use crate::utils::jni_utils::start_location_enabled_updates;

                match start_location_enabled_updates(move |enabled| {
                    let _ =
                        state_tx_clone.unbounded_send(ChannelSend::LocationEnabledUpdate(enabled));
                }) {
                    Ok(callback_ptr) => {
                        println!("[Print] Location enabled watcher started: {callback_ptr}");
                    }
                    Err(e) => {
                        println!("[Print] Failed to start location enabled watcher: {e}");
                    }
                }
            });
        }

        use_hook(|| {
            spawn(async move {
                #[cfg(target_os = "android")]
                {
                    use crate::utils::jni_utils::{
                        check_and_request_permissions, get_last_known_location,
                        start_location_updates,
                    };

                    match check_and_request_permissions().await {
                        Ok(true) => {
                            match get_last_known_location() {
                                Ok(last_known_location) => {
                                    println!("[Print] Location: {:?}", last_known_location);
                                    let _ = state_tx.unbounded_send(ChannelSend::LocationUpdate((
                                        last_known_location.0,
                                        last_known_location.1,
                                    )));
                                }
                                Err(e) => {
                                    println!("[Print] Error getting location: {e}");
                                }
                            }
                            match start_location_updates(move |(lat, lng, accuracy)| {
                                println!(
                                    "[Print] Location changed: lat={lat}, lng={lng}, accuracy={accuracy}"
                                );
                                let _ = state_tx
                                    .unbounded_send(ChannelSend::LocationUpdate((lat, lng)));
                            }) {
                                Ok(callback_ptr) => {
                                    println!("[Print] Location updates started: {callback_ptr}");
                                }
                                Err(e) => {
                                    println!("[Print] Error starting location updates: {e}");
                                }
                            }
                        }
                        Ok(false) => {
                            println!("[Print] Permissions: false");
                        }
                        Err(e) => {
                            println!("[Print] Error checking permissions: {e}");
                        }
                    }
                }
                #[cfg(not(target_os = "android"))]
                {
                    radio
                        .write_channel(DataChannel::LocationEnabledUpdate)
                        .is_location_enabled = true;
                    radio.write_channel(DataChannel::LocationUpdate).location =
                        Some((59.436552, 24.753048));
                }
            });
        });

        use_hook(|| {
            spawn(async move {
                let mut last_location = None;
                let mut next_update = Utc::now();
                loop {
                    println!("[Print] loop");
                    if next_update > Utc::now() {
                        println!("[Print] Loop: sleeping for {:?}", next_update - Utc::now());
                        smol::Timer::after((next_update - Utc::now()).to_std().unwrap()).await;
                        continue;
                    }

                    let location = location.read().cloned();
                    if location.is_none() {
                        next_update += Duration::from_millis(100);
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

                        let (new_stops_radius, new_stops_distances): (
                            HashMap<String, Stop>,
                            HashMap<String, u64>,
                        ) = stops_parser::get_stops_in_radius(
                            cur_stops,
                            current_location.0,
                            current_location.1,
                            150.0,
                        );

                        {
                            *stops_radius.write() = new_stops_radius;
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
                        let stops_radius = stops_radius
                            .read()
                            .cloned()
                            .keys()
                            .cloned()
                            .filter(|e| !e.trim().is_empty())
                            .collect();
                        let stops_departures = match get_departures(stops_radius).await {
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

        Router::new(|| RouterConfig::<Route>::default().with_initial_path(Route::Timetable))
    }
}

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
        #[route("/")]
        Timetable,
}
