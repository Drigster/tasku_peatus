use std::collections::HashMap;

use freya::{prelude::*, radio::use_radio};

use crate::{DataChannel, components::StopComponent, utils::departures_parser::get_departures};

#[derive(PartialEq)]
pub struct Timetable {}
impl Component for Timetable {
    fn render(&self) -> impl IntoElement {
        let mut radio = use_radio(DataChannel::StopsRadiusUpdate);

        let stops_radius = radio.read().stops_radius.clone();

        // spawn({
        //     let stops_radius = stops_radius.clone();
        //     async move {
        //         let stops_departures = match get_departures(
        //             stops_radius
        //                 .into_iter()
        //                 .filter_map(|e| {
        //                     if e.siri_id.is_empty() {
        //                         None
        //                     } else {
        //                         Some(e.siri_id.clone())
        //                     }
        //                 })
        //                 .collect(),
        //         )
        //         .await
        //         {
        //             Ok(stops_departures) => stops_departures,
        //             Err(e) => {
        //                 log::error!("Error getting departures: {e}");
        //                 (HashMap::new(), 30)
        //             }
        //         };

        //         radio
        //             .write_channel(DataChannel::DeparturesUpdate)
        //             .departures = stops_departures.0;
        //         radio
        //             .write_channel(DataChannel::DeparturesUpdate)
        //             .departures_next_update = stops_departures.1;
        //     }
        // });

        rect().width(Size::Fill).height(Size::Fill).child(
            ScrollView::new()
                .width(Size::Fill)
                .height(Size::Fill)
                .child(
                    rect().padding((4.0, 0.0, 0.0, 0.0)).children(
                        stops_radius
                            .into_iter()
                            .map(|stop| StopComponent::new(stop.siri_id).into()),
                    ),
                ),
        )
    }
}
