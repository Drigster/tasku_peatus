use std::collections::HashMap;

use chrono::{Datelike, Local, NaiveDate, Timelike};
use freya::{
    animation::{AnimNum, Ease, OnCreation, use_animation},
    icons::lucide,
    prelude::*,
    radio::use_radio,
};

use crate::{
    components::{DepartureComponent, DepartureTimes},
    launch_config::DataChannel,
    utils::transit::parsers::routes::RouteType,
};

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

        if stop_data.is_none() {
            return rect().child(label().text("No stop data"));
        };
        let stop_data = stop_data.unwrap();
        let current_stop_departures_map = radio.slice(DataChannel::DeparturesUpdate, |s| {
            &s.transit_data.departures
        });
        let binding = current_stop_departures_map.read();
        let current_stop_departures_map = binding.get(&stop_data.siri_id);

        let mut formated_departure_times: HashMap<RouteType, DepartureTimes> = stop_data
            .routes
            .iter()
            .map(|route| {
                let mut scheduled_times = vec![];
                for (weekday, times) in route.weekdays_times.iter() {
                    if !weekday.is_nth_day(Local::now().weekday().num_days_from_monday() as u8 + 1)
                    {
                        continue;
                    }

                    let times: Vec<u32> = times
                        .iter()
                        .filter_map(|time| {
                            if time.times <= (Local::now().num_seconds_from_midnight() / 60) as u32
                            {
                                return None;
                            }
                            if NaiveDate::from_num_days_from_ce_opt(
                                time.valid_from as i32 + 719_163,
                            )
                            .expect("date out of range")
                                > Local::now().date_naive()
                            {
                                return None;
                            }
                            if time.valid_to > 0
                                && NaiveDate::from_num_days_from_ce_opt(
                                    time.valid_to as i32 + 719_163,
                                )
                                .expect("date out of range")
                                    < Local::now().date_naive()
                            {
                                return None;
                            }

                            return Some(time.times);
                        })
                        .collect();

                    scheduled_times.append(&mut times.clone());
                }
                let departure_times = DepartureTimes {
                    destination_name: route.destination_name.clone(),
                    until: 0,
                    extra_data: None,
                    scheduled_times,
                };

                return (route.route_type.clone(), departure_times);
            })
            .collect();

        if let Some(current_stop_departures_map) = current_stop_departures_map {
            for (current_stop_route_type, current_stop_departures) in current_stop_departures_map {
                if current_stop_departures.is_empty() {
                    continue;
                }
                let formated_departure_time =
                    formated_departure_times.get_mut(current_stop_route_type);
                let first = current_stop_departures.first().unwrap();
                if let Some(formated_departure_time) = formated_departure_time {
                    if formated_departure_time.destination_name != first.destination_name {
                        formated_departure_time.destination_name = first.destination_name.clone();
                    }
                    formated_departure_time.until = first.until;
                    formated_departure_time.extra_data = first.extra_data.clone();
                } else {
                    formated_departure_times.insert(
                        current_stop_route_type.clone(),
                        DepartureTimes {
                            destination_name: first.destination_name.clone(),
                            until: first.until,
                            extra_data: first.extra_data.clone(),
                            scheduled_times: current_stop_departures
                                .iter()
                                .filter_map(|departure| {
                                    if departure.scheduled_time
                                        > (Local::now().num_seconds_from_midnight() / 60) as u16
                                    {
                                        Some(departure.scheduled_time as u32)
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                        },
                    );
                }
            }
        }

        let mut sorted_formated_departure_times: Vec<(RouteType, DepartureTimes)> =
            formated_departure_times
                .into_iter()
                .filter(|e| e.1.scheduled_times.len() > 0 || e.1.until > 0)
                .collect();
        sorted_formated_departure_times.sort_by(|a, b| a.1.until.cmp(&b.1.until));

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
                    .children(if !sorted_formated_departure_times.is_empty() {
                        sorted_formated_departure_times
                            .iter()
                            .map(|formated_departure| {
                                DepartureComponent::new(
                                    formated_departure.0.clone(),
                                    formated_departure.1.clone(),
                                )
                                .into_element()
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
                    }),
            )
    }
}
