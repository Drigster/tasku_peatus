use freya::{
    animation::{AnimNum, Ease, OnCreation, use_animation},
    icons::lucide,
    prelude::*,
    radio::use_radio,
};

use crate::{
    DataChannel,
    components::{DepartureComponent, departure},
};

#[derive(Clone, PartialEq)]
pub struct StopComponent {
    pub siri_id: String,
}

impl StopComponent {
    pub fn new(siri_id: String) -> Self {
        Self { siri_id }
    }
}

impl Component for StopComponent {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let siri_id = self.siri_id.clone();

        let radio = use_radio(DataChannel::StopsUpdate);
        let stops = radio.slice_current(|s| &s.stops);
        let binding = stops.read();
        let stop_data = binding.iter().find(|e| e.siri_id == siri_id).unwrap();
        let stops_distances = radio
            .slice(DataChannel::StopsDistancesUpdate(siri_id.clone()), |s| {
                &s.stops_distances
            });
        let departures = radio.slice(DataChannel::DeparturesUpdate, |s| &s.departures);
        let binding = departures.read();
        let departures = binding.get(&siri_id);

        let mut open = use_state(|| true);

        let mut animation = use_animation(|conf| {
            conf.on_creation(OnCreation::Finish);
            (
                AnimNum::new(0., 100.).ease(Ease::InOut).time(200),
                AnimNum::new(0., 90.).ease(Ease::InOut).time(200),
            )
        });

        use_side_effect({
            move || {
                if open() {
                    if animation.peek().0.value() != 100.0 && !*animation.is_running().read() {
                        animation.start();
                    }
                } else if animation.peek().0.value() != 0.0 && !*animation.is_running().read() {
                    animation.reverse();
                }
            }
        });

        let height = animation.read().0.value();
        let rotation = animation.read().1.value();

        rect()
            .width(Size::Fill)
            .child(
                rect()
                    .width(Size::Fill)
                    .height(Size::px(35.0))
                    .margin((0.0, 4.0, 4.0, 4.0))
                    .spacing(2.0)
                    .corner_radius(6.0)
                    .background(theme.read().colors.primary)
                    .direction(Direction::Horizontal)
                    .content(Content::Flex)
                    .shadow(
                        Shadow::new()
                            .x(3.0)
                            .y(3.0)
                            .blur(6.0)
                            .color(Color::BLACK.with_a(102)),
                    )
                    .child(
                        rect()
                            .direction(Direction::Horizontal)
                            .on_press(move |_| {
                                open.set(!open());
                            })
                            .child(
                                rect()
                                    .width(Size::px(35.0))
                                    .height(Size::px(35.0))
                                    .center()
                                    .child(
                                        svg(lucide::chevron_right())
                                            .rotate(rotation)
                                            .width(Size::Fill)
                                            .height(Size::Fill),
                                    ),
                            )
                            .child(
                                rect()
                                    .height(Size::Fill)
                                    .main_align(Alignment::Center)
                                    .child(
                                        label()
                                            .color(theme.read().colors.text_primary)
                                            .font_size(20.0)
                                            .text(format!(
                                                "{} - {}m",
                                                stop_data.name,
                                                stops_distances
                                                    .read()
                                                    .get(&self.siri_id)
                                                    .copied()
                                                    .unwrap_or(0)
                                            )),
                                    ),
                            ),
                    )
                    .child(rect().width(Size::flex(1.0)))
                    .child(
                        rect()
                            .width(Size::px(35.0))
                            .height(Size::px(35.0))
                            .padding(4.0)
                            .center()
                            .child(svg(lucide::star()).width(Size::Fill).height(Size::Fill)),
                    ),
            )
            .child(
                rect()
                    .padding((4.0, 0.0, 0.0, 0.0))
                    .width(Size::Fill)
                    .visible_height(VisibleSize::inner_percent(height))
                    .spacing(4.0)
                    .padding((0.0, 4.0, 4.0, 4.0))
                    .overflow(Overflow::Clip)
                    .children(if let Some(departures) = departures {
                        departures
                            .iter()
                            .cloned()
                            .map(|departure| DepartureComponent::new(departure).into())
                            .collect()
                    } else {
                        vec![
                            rect()
                                .width(Size::Fill)
                                .height(Size::px(70.0))
                                .spacing(4.0)
                                .corner_radius(6.0)
                                .background(theme.read().colors.primary)
                                .direction(Direction::Horizontal)
                                .content(Content::Flex)
                                .shadow(
                                    Shadow::new()
                                        .x(3.0)
                                        .y(3.0)
                                        .blur(6.0)
                                        .color(Color::BLACK.with_a(102)),
                                )
                                .child(
                                    rect()
                                        .width(Size::Fill)
                                        .height(Size::Fill)
                                        .center()
                                        .child(label().text("No departures")),
                                )
                                .into(),
                        ]
                    }),
            )
    }
}
