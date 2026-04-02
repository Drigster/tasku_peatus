#[cfg(target_os = "android")]
use std::sync::atomic::Ordering;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use chrono::Utc;
use freya::{
    prelude::*,
    radio::{RadioStation, use_radio, use_share_radio},
    router::{Routable, RouterConfig},
};
use freya_router::prelude::Router;

use crate::{
    ChannelSend, Data, DataChannel, ErrorState,
    utils::{departures_parser::get_departures, stops_parser},
};
use crate::{
    layouts::AppLayout,
    pages::{Loading, Timetable},
};

pub const CUSTOM_THEME: Theme = Theme {
    name: "custom",
    colors: ColorsSheet {
        primary: Color::from_rgb(227, 227, 227),
        secondary: Color::from_rgb(49, 161, 218),
        text_primary: Color::BLACK,
        text_secondary: Color::WHITE,
        ..LIGHT_THEME.colors
    },
    ..LIGHT_THEME
};

pub struct MyApp {
    pub radio_station: RadioStation<Data, DataChannel>,
}
impl App for MyApp {
    fn render(&self) -> impl IntoElement {
        use_share_radio(move || self.radio_station);

        use_init_theme(|| CUSTOM_THEME);

        let mut radio = use_radio(DataChannel::NoUpdate);
        let state_tx = radio.read().state_tx.clone().unwrap();
        let location = radio.slice(DataChannel::LocationUpdate, |s| &s.location);
        let mut error_state =
            radio.slice_mut(DataChannel::ErrorStateUpdate, |s| &mut s.error_state);
        let mut stops = radio.slice_mut(DataChannel::StopsUpdate, |s| &mut s.stops);
        let mut stops_radius =
            radio.slice_mut(DataChannel::StopsRadiusUpdate, |s| &mut s.stops_radius);
        let mut departures_next_update = radio.slice_mut(DataChannel::DeparturesUpdate, |s| {
            &mut s.departures_next_update
        });
        let mut departures = radio.slice_mut(DataChannel::DeparturesUpdate, |s| &mut s.departures);

        let state_tx_clone = state_tx.clone();
        #[cfg(target_os = "android")]
        use_hook(move || {
            use crate::utils::jni_utils::start_location_enabled_updates;

            match start_location_enabled_updates(move |enabled| {
                println!("[Print] Location enabled changed: {enabled}");
                let _ = state_tx_clone.unbounded_send(ChannelSend::LocationEnabledUpdate(enabled));
            }) {
                Ok(callback_ptr) => {
                    println!("[Print] Location enabled watcher started: {callback_ptr}");
                }
                Err(e) => {
                    println!("[Print] Failed to start location enabled watcher: {e}");
                }
            }
        });

        #[cfg(target_os = "android")]
        use_hook(|| {
            spawn(async move {
                use crate::utils::jni_utils::{
                    check_and_request_permissions, get_last_known_location, start_location_updates,
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
                            let _ =
                                state_tx.unbounded_send(ChannelSend::LocationUpdate((lat, lng)));
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
            });
        });

        use_hook(|| {
            spawn(async move {
                let mut last_location = location.read().cloned();
                let mut next_update = Utc::now();
                loop {
                    if next_update > Utc::now() {
                        println!("[Print] Waiting for {:?}", next_update - Utc::now());
                        smol::Timer::after((next_update - Utc::now()).to_std().unwrap()).await;
                        continue;
                    }

                    println!("[Print] last_location: {:?}", last_location);
                    println!("[Print] Loop");
                    let location = location.read().cloned();
                    if location.is_none() {
                        next_update = next_update + Duration::from_millis(100);
                        continue;
                    }
                    next_update = next_update + Duration::from_secs(5);

                    if location.is_some() && location != last_location {
                        println!("[Print] Location changed: {:?}", location);
                        last_location = location;
                        let location = location.unwrap();
                        println!("[Print] Updating stops");
                        let mut cur_stops = stops.read().clone();
                        if cur_stops.is_empty() {
                            let new_stops = stops_parser::get_stops().await.unwrap();
                            *stops.write() = new_stops.clone();
                            cur_stops = new_stops;
                        }

                        let (new_stops_radius, new_stops_distances): (
                            Vec<stops_parser::Stop>,
                            HashMap<String, u64>,
                        ) = stops_parser::get_stops_in_radius(
                            cur_stops, location.0, location.1, 150.0,
                        );

                        println!("[Print] stops_radius: {:?}", new_stops_radius.len());
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
                        println!("[Print] Updating departures");
                        let stops_departures = match get_departures(
                            stops_radius
                                .read()
                                .cloned()
                                .iter()
                                .filter_map(|e| {
                                    if e.siri_id.is_empty() {
                                        None
                                    } else {
                                        Some(e.siri_id.clone())
                                    }
                                })
                                .collect(),
                        )
                        .await
                        {
                            Ok(stops_departures) => stops_departures,
                            Err(e) => {
                                log::error!("Error getting departures: {e}");
                                (HashMap::new(), 30)
                            }
                        };

                        println!("[Print] departures: {:?}", stops_departures);

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
        Loading,
        #[route("/timetable")]
        Timetable,
}
