use std::{collections::HashMap, time::Duration};

use freya::{prelude::*, radio::use_radio};

use crate::{
    Data, DataChannel,
    components::StopComponent,
    utils::{departures_parser::get_departures, stops_parser},
};

use crate::ChannelSend;

#[derive(PartialEq)]
pub struct Loading {}
impl Component for Loading {
    fn render(&self) -> impl IntoElement {
        let mut error: State<Option<String>> = use_state(|| None);
        let mut is_loading = use_state(|| true);
        let mut radio = use_radio(DataChannel::StopsUpdate);
        let state_tx = radio.read().state_tx.clone().unwrap();

        // let stops_radius = radio.slice(DataChannel::StopsRadiusUpdate, |s| &s.stops_radius);
        // use_hook(|| {
        //     let stops_radius = stops_radius.clone();
        //     spawn(async move {
        //         loop {
        //             if stops_radius.read().is_empty() {
        //                 smol::Timer::after(Duration::from_millis(10)).await;
        //                 continue;
        //             }
        //             println!("[Print] departures update");

        //             let (departures, departures_next_update) = match get_departures(
        //                 stops_radius
        //                     .read()
        //                     .iter()
        //                     .filter_map(|e| {
        //                         if e.siri_id.is_empty() {
        //                             None
        //                         } else {
        //                             Some(e.siri_id.clone())
        //                         }
        //                     })
        //                     .collect(),
        //             )
        //             .await
        //             {
        //                 Ok(stops_departures) => stops_departures,
        //                 Err(e) => {
        //                     log::error!("Error getting departures: {e}");
        //                     (HashMap::new(), 30)
        //                 }
        //             };
        //             radio
        //                 .write_channel(DataChannel::DeparturesUpdate)
        //                 .departures = departures;
        //             radio
        //                 .write_channel(DataChannel::DeparturesUpdate)
        //                 .departures_next_update = departures_next_update;

        //             is_loading.set_if_modified(false);

        //             smol::Timer::after(Duration::from_secs(departures_next_update.into())).await;
        //         }
        //     })
        // });

        // use_hook(|| {
        //     spawn(async move {
        //         #[cfg(target_os = "android")]
        //         {
        //             use crate::utils::jni_utils::{
        //                 check_and_request_permissions, get_last_known_location,
        //                 request_fresh_location, start_location_updates,
        //             };
        //             if check_and_request_permissions().await.unwrap() {
        //                 match start_location_updates(move |(lat, lng, accuracy)| {
        //                     println!(
        //                         "[Print] Location changed: lat={lat}, lng={lng}, accuracy={accuracy}"
        //                     );
        //                     let _ =
        //                         state_tx.unbounded_send(ChannelSend::LocationUpdate((lat, lng)));
        //                 }) {
        //                     Ok(callback_ptr) => {
        //                         println!("[Print] Location updates started: {callback_ptr}");
        //                     }
        //                     Err(e) => {
        //                         log::error!("Error starting location updates: {e}");
        //                     }
        //                 }

        //                 // let location = get_last_known_location();
        //                 // match location {
        //                 //     Ok(location) => {
        //                 //         println!("[Print] Location: {:?}", location);
        //                 //         radio.write_channel(DataChannel::LocationUpdate).location =
        //                 //             Some(location)
        //                 //     }
        //                 //     Err(e) => {
        //                 //         log::error!("Error getting location: {e}");
        //                 //     }
        //                 // }
        //             } else {
        //                 error.set(Some("No permissions".to_string()));
        //                 return;
        //             }
        //         }
        //         #[cfg(not(target_os = "android"))]
        //         {
        //             let closure = move || {
        //                 let _ = state_tx.unbounded_send(ChannelSend::LocationUpdate((0.0, 0.0)));
        //             };

        //             let _ = closure();

        //             radio.write_channel(DataChannel::LocationUpdate).location =
        //                 Some((59.436184, 24.751787));
        //         }

        //         let location = radio.read().location;

        //         if location.is_none() {
        //             error.set(Some("[Print] No location".to_string()));
        //             return;
        //         }
        //     });
        // });

        // let location = radio.slice(DataChannel::LocationUpdate, |s| &s.location);
        // use_side_effect(move || {
        //     println!("[Print] Updating stops");
        //     let location = location.read();
        //     if location.is_none() {
        //         return;
        //     }
        //     let location = location.unwrap();
        //     spawn(async move {
        //         println!("[Print] Updating stops 2");
        //         let stops = stops_parser::get_stops().await;
        //         match stops {
        //             Ok(stops) => {
        //                 radio.write().stops = stops.clone();

        //                 let stops_radius: Vec<stops_parser::Stop> =
        //                     stops_parser::get_stops_in_radius(stops, location.0, location.1, 150.0);

        //                 println!("[Print] stops_radius: {:?}", stops_radius.len());
        //                 radio
        //                     .write_channel(DataChannel::StopsRadiusUpdate)
        //                     .stops_radius = stops_radius;
        //             }
        //             Err(e) => {
        //                 log::error!("Error updating stops: {e}");
        //             }
        //         }
        //     });
        // });

        if is_loading() {
            rect()
                .width(Size::Fill)
                .height(Size::Fill)
                .main_align(Alignment::Center)
                .cross_align(Alignment::Center)
                .child(if let Some(error) = (*error.read()).clone() {
                    label()
                        .font_size(20.0)
                        .color(Color::WHITE)
                        .text(error)
                        .into_element()
                } else {
                    CircularLoader::new().into_element()
                })
        } else {
            rect().width(Size::Fill).height(Size::Fill).child(
                ScrollView::new()
                    .width(Size::Fill)
                    .height(Size::Fill)
                    .child(rect().padding((4.0, 0.0, 0.0, 0.0)).children(
                        [], // stops_radius
                            //     .read()
                            //     .clone()
                            //     .into_iter()
                            //     .map(|stop| StopComponent::new(stop).into()),
                    )),
            )
        }
    }
}
