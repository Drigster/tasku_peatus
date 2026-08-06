use freya::{
    animation::{AnimNum, Ease, OnCreation, use_animation},
    icons::lucide,
    prelude::*,
    radio::use_radio,
};

use crate::{components::DepartureComponent, launch_config::DataChannel};

#[derive(Clone, PartialEq)]
pub struct StopComponent {
    pub stop_id: String,
    pub distance: u64,
}

impl StopComponent {
    pub fn new(stop_id: String, distance: u64) -> Self {
        Self { stop_id, distance }
    }
}

impl Component for StopComponent {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let stop_id = self.stop_id.clone();

        let radio = use_radio(DataChannel::StopsUpdate);
        let stops = &radio.read().transit_data.stops;
        let stop_data = stops.get(&stop_id);
        let departures = radio.slice(DataChannel::DeparturesUpdate, |s| {
            &s.transit_data.departures
        });
        let departures = departures.read();

        if stop_data.is_none() {
            return rect();
        };
        let stop_data = stop_data.unwrap();
        if stop_data.routes.is_empty() {
            return rect();
        };

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
                    .corner_radius(6.0)
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
                            .spacing(2.0)
                            .corner_radius(6.0)
                            .background(theme.read().colors.primary)
                            .direction(Direction::Horizontal)
                            .content(Content::Flex)
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
                                            .padding(2.0)
                                            .center()
                                            .child(
                                                SvgViewer::new(lucide::chevron_right())
                                                    .rotation(rotation)
                                                    .width(Size::Fill)
                                                    .height(Size::Fill),
                                            ),
                                    )
                                    .child(
                                        rect()
                                            .height(Size::Fill)
                                            .main_align(Alignment::Center)
                                            .overflow(Overflow::Clip)
                                            .child(
                                                label()
                                                    .color(theme.read().colors.text_primary)
                                                    .font_size(20.0)
                                                    .max_lines(1)
                                                    .text(format!(
                                                        "{} - {}m",
                                                        stop_data.name, self.distance
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
                                    .child(
                                        SvgViewer::new(lucide::star())
                                            .width(Size::Fill)
                                            .height(Size::Fill),
                                    ),
                            ),
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
                    .children(
                        if !stop_data.routes.is_empty()
                            && let Some(departure) = departures.get(&stop_data.siri_id)
                        {
                            stop_data
                                .routes
                                .iter()
                                .cloned()
                                .filter_map(|route| {
                                    let dep = departure.get(&(
                                        route.route_type.clone(),
                                        route
                                            .route_name
                                            .split(" - ")
                                            .last()
                                            .unwrap_or(&route.route_name)
                                            .to_string(),
                                    ));
                                    match dep {
                                        Some(dep) => Some(
                                            DepartureComponent::new(route.clone(), dep.clone())
                                                .into_element(),
                                        ),
                                        None => None,
                                    }
                                })
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
                                    .into_element(),
                            ]
                        },
                    ),
            )
    }
}
