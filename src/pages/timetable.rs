use freya::{prelude::*, radio::use_radio};

use crate::{
    components::{Loader, StopComponent},
    launch_config::{AppState, DataChannel},
};

#[derive(PartialEq)]
pub struct Timetable {}
impl Component for Timetable {
    fn render(&self) -> impl IntoElement {
        let radio = use_radio(DataChannel::StopsRadiusUpdate);
        // `state` is published on its own channel, and `use_radio` only
        // subscribes to the channel it is given, so take an explicit slice.
        // Without it the loading message never repaints on its own.
        let state = radio.slice(DataChannel::StateUpdate, |s| &s.state);

        let is_empty = radio.read().transit_data.stops_radius.is_empty();

        rect()
            .width(Size::Fill)
            .height(Size::Fill)
            .child(if is_empty {
                let state = state.read().clone();
                rect()
                    .expanded()
                    .center()
                    .child(Loader)
                    .maybe_child(state.map(|state| {
                        rect()
                            .padding((8.0, 0.0, 0.0, 0.0))
                            .color(Color::WHITE)
                            .child(match state {
                                AppState::LocationDisabled => label().text("Location is disabled"),
                                AppState::WitingForLocation => label().text("Waiting for location"),
                                AppState::LocationError(e) => label().text(format!("Error: {e}")),
                            })
                            .into_element()
                    }))
                    .into_element()
            } else {
                // Clone only the stop id each component needs, rather than the
                // whole `Vec<StopRadius>` (which would also copy every siri_id).
                let stops: Vec<StopComponent> = radio
                    .read()
                    .transit_data
                    .stops_radius
                    .iter()
                    .map(|stop_radius| {
                        StopComponent::new(stop_radius.stop_id.clone(), stop_radius.distance)
                    })
                    .collect();

                ScrollView::new()
                    .width(Size::Fill)
                    .height(Size::Fill)
                    .child(rect().padding((4.0, 0.0, 0.0, 0.0)).children(stops))
                    .into_element()
            })
    }
}
