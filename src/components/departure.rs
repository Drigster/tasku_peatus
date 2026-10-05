use freya::{prelude::*, radio::use_radio};

use crate::{launch_config::DataChannel, utils::transit::parsers::routes::RouteType};

/// Depot runs terminate at the depot instead of serving the line, so they are
/// called out rather than blending into the times list.
const DEPOT_TIME_COLOR: Color = Color::from_rgb(0x9c, 0x16, 0x30);

pub(crate) fn format_time(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60 % 24, minutes % 60)
}

#[derive(Clone, PartialEq)]
pub struct ScheduledTime {
    pub(crate) text: String,
    /// True when only the static timetable lists this departure and the live
    /// feed has not confirmed it. Shown underlined.
    pub(crate) schedule_only: bool,
    /// True when this departure runs to the depot (destination key "dp").
    /// Highlighted so it is not mistaken for a normal service.
    pub(crate) to_depot: bool,
}

#[derive(Clone, PartialEq)]
pub struct DepartureTimes {
    pub(crate) destination_name: String,
    /// Seconds until departure as reported by the live feed, or `None` when the
    /// feed says nothing about this route. A countdown is only meaningful in
    /// the first case.
    pub(crate) until: Option<u32>,
    pub(crate) extra_data: Option<String>,
    /// The next few upcoming departures, pre-formatted where the schedule is
    /// filtered so they are not rebuilt on every clock tick.
    pub(crate) scheduled_times: Vec<ScheduledTime>,
    /// First upcoming scheduled departure, minutes from midnight. Used to order
    /// rows deterministically when their live countdowns tie.
    pub(crate) next_scheduled: Option<u32>,
}

#[derive(Clone, PartialEq)]
pub struct DepartureComponent {
    pub route_type: RouteType,
    pub departure_times: DepartureTimes,
}

impl DepartureComponent {
    pub fn new(route_type: RouteType, departure_times: DepartureTimes) -> Self {
        Self {
            route_type,
            departure_times,
        }
    }
}

impl Component for DepartureComponent {
    /// Rows are sorted by time-until-departure, which reorders them constantly.
    /// Keying on the route keeps each row's scope attached to its route instead
    /// of to a list position. One row per `RouteType` per stop, so this is
    /// unique among siblings.
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.route_type)
    }

    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let radio = use_radio(DataChannel::TickUpdate);

        // Derived from the shared clock rather than a per-row timer. Reading
        // `departures_fetched_at` needs no subscription of its own: when it
        // changes, so does `until`, and the parent re-renders this row.
        // Only the live feed can support a countdown. Without it the row falls
        // back to showing when the next timetabled departure is due.
        let countdown = self.departure_times.until.map(|until| {
            let data = radio.read();
            let elapsed = (data.now - data.departures_fetched_at).max(0);
            until.saturating_sub(elapsed.min(u32::MAX as i64) as u32)
        });

        let (transport_icon, transport_color) = self.route_type.get_transport_icon_and_color();

        let (primary, text_primary) = {
            let theme = theme.read();
            (theme.colors.primary, theme.colors.text_primary)
        };

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
                    .background(primary)
                    .direction(Direction::Horizontal)
                    .content(Content::Flex)
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
                                                    .color(text_primary)
                                                    .font_size(13.0)
                                                    .font_weight(FontWeight::BLACK)
                                                    .text(self.route_type.get_route().to_string()),
                                            ),
                                    )
                                    .child(
                                        label()
                                            .font_size(20.0)
                                            .font_weight(FontWeight::BOLD)
                                            .max_lines(1)
                                            .text(self.departure_times.destination_name.clone()),
                                    ),
                            )
                            .child({
                                // One span per time so the unconfirmed ones can
                                // be underlined individually; the separators stay
                                // undecorated so the rule does not run through
                                // the commas. Spans inherit the paragraph's
                                // colour and size and only override decoration.
                                let mut spans: Vec<Span<'static>> = Vec::new();
                                for scheduled in &self.departure_times.scheduled_times {
                                    if !spans.is_empty() {
                                        spans.push(Span::new(", "));
                                    }

                                    // The two markers are independent: a depot
                                    // run can also be unconfirmed.
                                    let mut span = Span::new(scheduled.text.clone());
                                    if scheduled.to_depot {
                                        span = span
                                            .color(DEPOT_TIME_COLOR)
                                            .font_weight(FontWeight::BOLD);
                                    }
                                    if scheduled.schedule_only {
                                        span = span.text_decoration(TextDecoration::Underline);
                                    }
                                    spans.push(span);
                                }

                                paragraph()
                                    .color(text_primary)
                                    .font_size(15.0)
                                    .max_lines(1)
                                    .spans_iter(spans.into_iter())
                            }),
                    )
                    .child({
                        rect()
                            .height(Size::px(70.0))
                            .width(Size::px(70.0))
                            .center()
                            .children({
                                match countdown {
                                    // Integer math; `departure_time` is seconds.
                                    Some(departure_time) => {
                                        let (value, unit) = if departure_time <= 30 {
                                            (None, "now")
                                        } else if departure_time < 60 {
                                            (Some(departure_time), "seconds")
                                        } else if departure_time < 60 * 60 {
                                            (Some(departure_time / 60), "minutes")
                                        } else {
                                            (Some(departure_time / 3600), "hours")
                                        };

                                        vec![
                                            label()
                                                .color(text_primary)
                                                .font_size(25.0)
                                                .font_weight(FontWeight::BOLD)
                                                .text(match value {
                                                    Some(value) => value.to_string(),
                                                    None => "now".to_string(),
                                                })
                                                .into_element(),
                                            label()
                                                .color(text_primary)
                                                .font_size(14.0)
                                                .text(match value {
                                                    Some(_) => unit.to_string(),
                                                    None => departure_time.to_string(),
                                                })
                                                .into_element(),
                                        ]
                                    }
                                    // A clock time is five characters wide,
                                    // where a countdown is one or two, so it
                                    // needs a smaller size to fit the fixed
                                    // 70px column without wrapping.
                                    None => vec![
                                        label()
                                            .color(text_primary)
                                            .font_size(20.0)
                                            .max_lines(1)
                                            .text(
                                                self.departure_times
                                                    .next_scheduled
                                                    .map(format_time)
                                                    .unwrap_or_default(),
                                            )
                                            .into_element(),
                                    ],
                                }
                            })
                    }),
            )
            .child(rect().width(Size::Fill))
    }
}
