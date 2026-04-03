use freya::{prelude::*, radio::use_radio};

use crate::{DataChannel, components::StopComponent};

#[derive(PartialEq)]
pub struct Timetable {}
impl Component for Timetable {
    fn render(&self) -> impl IntoElement {
        let radio = use_radio(DataChannel::StopsRadiusUpdate);
        let stops_radius = radio.read().stops_radius.clone();

        rect().width(Size::Fill).height(Size::Fill).child(
            ScrollView::new()
                .width(Size::Fill)
                .height(Size::Fill)
                .child(
                    rect().padding((4.0, 0.0, 0.0, 0.0)).children(
                        stops_radius
                            .keys()
                            .cloned()
                            .map(|siri_id| StopComponent::new(siri_id).into()),
                    ),
                ),
        )
    }
}
