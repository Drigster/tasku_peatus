use std::{cmp, time::Duration};

use chrono::TimeDelta;
use freya::{icons::lucide, prelude::*};

use crate::utils::departures_parser::Departure;

#[derive(Clone, PartialEq)]
pub struct DepartureComponent {
    pub departure: Departure,
}

impl DepartureComponent {
    pub fn new(departure: Departure) -> Self {
        Self { departure }
    }
}

impl Component for DepartureComponent {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();

        let mut departure_time = use_state(|| self.departure.until);

        use_side_effect_with_deps(&self.departure.until, move |value| {
            departure_time.set(*value);
        });

        use_hook(|| {
            spawn({
                let mut departure_time = departure_time.clone();
                async move {
                    loop {
                        smol::Timer::after(Duration::from_secs(1)).await;
                        if *departure_time.read() == 0 {
                            continue;
                        }

                        *departure_time.write() -= 1;
                    }
                }
            });
        });

        rect()
            .width(Size::Fill)
            .child(
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
                            .height(Size::px(70.0))
                            .width(Size::px(70.0))
                            .padding(6.0)
                            .center()
                            .child(
                                svg(lucide::bus_front())
                                    .width(Size::Fill)
                                    .height(Size::Fill),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.0))
                            .height(Size::Fill)
                            .main_align(Alignment::Center)
                            .child(
                                rect()
                                    .spacing(4.0)
                                    .direction(Direction::Horizontal)
                                    .cross_align(Alignment::Center)
                                    .child(
                                        rect()
                                            .width(Size::px(35.0))
                                            .height(Size::px(20.0))
                                            .center()
                                            .background(Color::from_hex("#00E1B4").unwrap())
                                            .corner_radius(8.0)
                                            .child(
                                                label()
                                                    .color(theme.read().colors.text_primary)
                                                    .font_size(13.0)
                                                    .font_weight(FontWeight::BLACK)
                                                    .text(self.departure.route.clone()),
                                            ),
                                    )
                                    .child(
                                        label()
                                            .font_size(20.0)
                                            .font_weight(FontWeight::BOLD)
                                            .text(self.departure.direction.to_string()),
                                    ),
                            )
                            .child(
                                label()
                                    .color(theme.read().colors.text_primary)
                                    .font_size(15.0)
                                    .text({
                                        self.departure
                                            .scheduled_times
                                            .iter()
                                            .map(|time| {
                                                let time = TimeDelta::seconds(*time as i64);
                                                format!(
                                                    "{}:{:02}",
                                                    time.num_hours(),
                                                    time.num_minutes() % 60
                                                )
                                            })
                                            .collect::<Vec<String>>()
                                            [0..cmp::min(5, self.departure.scheduled_times.len())]
                                            .join(", ")
                                    }),
                            ),
                    )
                    .child({
                        let departure_time = *departure_time.read() as f64;
                        if departure_time <= 30.0 {
                            rect()
                                .height(Size::px(70.0))
                                .width(Size::px(70.0))
                                .center()
                                .child(
                                    label()
                                        .color(theme.read().colors.text_primary)
                                        .font_size(25.0)
                                        .font_weight(FontWeight::BOLD)
                                        .text("now"),
                                )
                                .child(
                                    label()
                                        .color(theme.read().colors.text_primary)
                                        .font_size(14.0)
                                        .text(departure_time.to_string()),
                                )
                        } else if departure_time < 60.0 {
                            rect()
                                .height(Size::px(70.0))
                                .width(Size::px(70.0))
                                .center()
                                .child(
                                    label()
                                        .color(theme.read().colors.text_primary)
                                        .font_size(25.0)
                                        .font_weight(FontWeight::BOLD)
                                        .text(format!("{}", departure_time)),
                                )
                                .child(
                                    label()
                                        .color(theme.read().colors.text_primary)
                                        .font_size(14.0)
                                        .text("seconds"),
                                )
                        } else if departure_time < 60.0 * 60.0 {
                            rect()
                                .height(Size::px(70.0))
                                .width(Size::px(70.0))
                                .center()
                                .child(
                                    label()
                                        .color(theme.read().colors.text_primary)
                                        .font_size(25.0)
                                        .font_weight(FontWeight::BOLD)
                                        .text(format!("{}", (departure_time / 60.0).floor())),
                                )
                                .child(
                                    label()
                                        .color(theme.read().colors.text_primary)
                                        .font_size(14.0)
                                        .text("minutes"),
                                )
                        } else {
                            rect()
                                .height(Size::px(70.0))
                                .width(Size::px(70.0))
                                .center()
                                .child(
                                    label()
                                        .color(theme.read().colors.text_primary)
                                        .font_size(25.0)
                                        .font_weight(FontWeight::BOLD)
                                        .text(format!(
                                            "{}",
                                            (departure_time / 60.0 * 60.0).floor()
                                        )),
                                )
                                .child(
                                    label()
                                        .color(theme.read().colors.text_primary)
                                        .font_size(14.0)
                                        .text("hours"),
                                )
                        }
                    }),
            )
            .child(rect().width(Size::Fill))
    }
}
