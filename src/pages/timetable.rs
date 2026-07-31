use freya::{prelude::*, radio::use_radio};

use crate::{
    components::{StopComponent, loader},
    launch_config::{AppState, DataChannel},
};

#[derive(PartialEq)]
pub struct Timetable {}
impl Component for Timetable {
    fn render(&self) -> impl IntoElement {
        let radio = use_radio(DataChannel::StopsRadiusUpdate);
        let stops_radius = radio.read().stops_radius.clone();
        let state = radio.read().state.clone();

        rect()
            .width(Size::Fill)
            .height(Size::Fill)
            .child(if stops_radius.is_empty() {
                rect()
                    .expanded()
                    .center()
                    .child(loader())
                    .maybe_child(match state {
                        Some(state) => Some(
                            rect()
                                .padding((8.0, 0.0, 0.0, 0.0))
                                .color(Color::WHITE)
                                .child(match state {
                                    AppState::LocationDisabled => {
                                        label().text("Location is disabled")
                                    }
                                    AppState::WitingForLocation => {
                                        label().text("Waiting for location")
                                    }
                                    AppState::LocationError(e) => {
                                        label().text(format!("Error: {e}"))
                                    }
                                })
                                .into_element(),
                        ),
                        None => None,
                    })
                    .into_element()
            } else {
                ScrollView::new()
                    .width(Size::Fill)
                    .height(Size::Fill)
                    .child(
                        rect().padding((4.0, 0.0, 0.0, 0.0)).children(
                            stops_radius
                                .into_iter()
                                .map(|siri_id| StopComponent::new(siri_id)),
                        ),
                    )
                    .into_element()
            })
        // .child(match state {
        //     Some(state) => rect()
        //         .expanded()
        //         .center()
        //         .child(match state {
        //             State::LocationDisabled => label().text("Location is disabled"),
        //             State::WitingForLocation => label().text("Waiting for location"),
        //             State::LocationError(e) => label().text(format!("Error: {e}")),
        //         })
        //         .into_element(),
        //     None => ScrollView::new()
        //         .width(Size::Fill)
        //         .height(Size::Fill)
        //         .child(
        //             rect().padding((4.0, 0.0, 0.0, 0.0)).children(
        //                 stops_radius
        //                     .into_iter()
        //                     .map(|siri_id| StopComponent::new(siri_id)),
        //             ),
        //         )
        //         .into_element(),
        // })
    }
}
