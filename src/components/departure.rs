use std::time::Duration;

use chrono::TimeDelta;
use freya::prelude::*;

use crate::utils::{departures_parser::Departure, routes_parser::StopRoute};

#[derive(Clone, PartialEq)]
pub struct DepartureComponent {
    pub departure: Departure,
    pub route: StopRoute,
}

impl DepartureComponent {
    pub fn new(route: StopRoute, departure: Departure) -> Self {
        Self { route, departure }
    }
}

impl Component for DepartureComponent {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();

        let mut departure_time = use_state(|| self.departure.until);
        // let radio = use_radio(DataChannel::RoutesUpdate);
        // let route_times = match radio.read().routes.get(&self.stop_id) {
        //     Some(route_times) => {
        //         match route_times.get(&(
        //             self.departure.route_type.clone(),
        //             self.departure.route.clone(),
        //         )) {
        //             Some(route_times) => route_times.clone(),
        //             None => HashMap::new(),
        //         }
        //     }
        //     None => HashMap::new(),
        // };
        // println!(
        //     "{:?} {:?} {:?}",
        //     self.departure.direction, self.departure.route, route_times
        // );

        // let now = Local::now();
        // let today_route_times = route_times
        //     .get(&(now.weekday().num_days_from_monday() as u8 + 1))
        //     .unwrap_or(&vec![])
        //     .clone();
        // let filtered_route_times = today_route_times
        //     .into_iter()
        //     .filter(|e| (e * 60) >= now.num_seconds_from_midnight() as i32)
        //     .take(5)
        //     .collect::<Vec<i32>>();

        let (transport_icon, transport_color) =
            self.route.route_type.get_transport_icon_and_color();

        use_side_effect_with_deps(&self.departure.until, move |value| {
            departure_time.set(*value);
        });

        use_hook(|| {
            spawn({
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
                    .height(Size::px(70.0))
                    .corner_radius(6.0)
                    .background(theme.read().colors.primary)
                    .direction(Direction::Horizontal)
                    .content(Content::Flex)
                    // .shadow(
                    //     Shadow::new()
                    //         .x(3.0)
                    //         .y(3.0)
                    //         .blur(6.0)
                    //         .color(Color::BLACK.with_a(102)),
                    // )
                    .child(
                        rect()
                            .height(Size::px(70.0))
                            .width(Size::px(70.0))
                            .padding(10.0)
                            .center()
                            .child(
                                SvgViewer::new(transport_icon)
                                    .width(Size::Fill)
                                    .height(Size::Fill),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.0))
                            .height(Size::Fill)
                            .spacing(4.0)
                            .overflow(Overflow::Clip)
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
                                            .background(transport_color)
                                            .corner_radius(8.0)
                                            .child(
                                                label()
                                                    .color(theme.read().colors.text_primary)
                                                    .font_size(13.0)
                                                    .font_weight(FontWeight::BLACK)
                                                    .text(self.route.route_type.get_route()),
                                            ),
                                    )
                                    .child(
                                        label()
                                            .font_size(20.0)
                                            .font_weight(FontWeight::BOLD)
                                            .max_lines(1)
                                            .text(
                                                self.route
                                                    .route_name
                                                    .split(" - ")
                                                    .last()
                                                    .unwrap_or(&self.route.route_name)
                                                    .to_string(),
                                            ),
                                    ),
                            )
                            .child(
                                label()
                                    .color(theme.read().colors.text_primary)
                                    .font_size(15.0)
                                    .max_lines(1)
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
                                            .take(5)
                                            .collect::<Vec<String>>()
                                            .join(", ")
                                    }),
                            ), // .child(
                               //     label()
                               //         .color(theme.read().colors.text_primary)
                               //         .font_size(15.0)
                               //         .text({
                               //             filtered_route_times
                               //                 .iter()
                               //                 .map(|time| {
                               //                     let time = TimeDelta::minutes(*time as i64);
                               //                     format!(
                               //                         "{}:{:02}",
                               //                         time.num_hours(),
                               //                         time.num_minutes() % 60
                               //                     )
                               //                 })
                               //                 .collect::<Vec<String>>()
                               //                 .join(", ")
                               //         }),
                               // ),
                    )
                    .child({
                        rect()
                            .height(Size::px(70.0))
                            .width(Size::px(70.0))
                            .center()
                            .children({
                                let departure_time = *departure_time.read() as f64;
                                if departure_time <= 30.0 {
                                    [
                                        label()
                                            .color(theme.read().colors.text_primary)
                                            .font_size(25.0)
                                            .font_weight(FontWeight::BOLD)
                                            .text("now")
                                            .into_element(),
                                        label()
                                            .color(theme.read().colors.text_primary)
                                            .font_size(14.0)
                                            .text(departure_time.to_string())
                                            .into_element(),
                                    ]
                                } else if departure_time < 60.0 {
                                    [
                                        label()
                                            .color(theme.read().colors.text_primary)
                                            .font_size(25.0)
                                            .font_weight(FontWeight::BOLD)
                                            .text(format!("{}", departure_time))
                                            .into_element(),
                                        label()
                                            .color(theme.read().colors.text_primary)
                                            .font_size(14.0)
                                            .text("seconds")
                                            .into_element(),
                                    ]
                                } else if departure_time < 60.0 * 60.0 {
                                    [
                                        label()
                                            .color(theme.read().colors.text_primary)
                                            .font_size(25.0)
                                            .font_weight(FontWeight::BOLD)
                                            .text(format!("{}", (departure_time / 60.0).floor()))
                                            .into_element(),
                                        label()
                                            .color(theme.read().colors.text_primary)
                                            .font_size(14.0)
                                            .text("minutes")
                                            .into_element(),
                                    ]
                                } else {
                                    [
                                        label()
                                            .color(theme.read().colors.text_primary)
                                            .font_size(25.0)
                                            .font_weight(FontWeight::BOLD)
                                            .text(format!(
                                                "{}",
                                                (departure_time / 60.0 * 60.0).floor()
                                            ))
                                            .into_element(),
                                        label()
                                            .color(theme.read().colors.text_primary)
                                            .font_size(14.0)
                                            .text("hours")
                                            .into_element(),
                                    ]
                                }
                            })
                    }),
            )
            .child(rect().width(Size::Fill))
    }
}
