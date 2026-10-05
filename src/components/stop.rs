use chrono::{Datelike, Local, Timelike};
use freya::{
    animation::{AnimNum, Ease, OnCreation, use_animation},
    icons::lucide,
    prelude::*,
    radio::use_radio,
};

use crate::{
    components::{DepartureComponent, DepartureTimes, ScheduledTime, format_time},
    launch_config::DataChannel,
    utils::transit::parsers::routes::{DEPOT_DESTINATION_KEY, RouteType},
};

/// Days between chrono's "days from CE" epoch and the epoch the transit feed
/// uses for its `valid_from` / `valid_to` day numbers.
const FEED_DAY_EPOCH_OFFSET: i32 = 719_163;

/// How many upcoming scheduled departures a row shows.
const DISPLAYED_SCHEDULED_TIMES: usize = 5;

/// One upcoming departure taken from the static timetable, before the live feed
/// is merged in.
struct Upcoming<'a> {
    minutes: u32,
    destination_name: &'a str,
    to_depot: bool,
}

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
    /// The list is re-sorted by distance on every location update. Keying on the
    /// stop id keeps each scope — and its expanded/collapsed state — attached to
    /// its stop instead of to a list position.
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.stop_id)
    }

    fn render(&self) -> impl IntoElement {
        // Every hook runs before any early return. Freya hooks are positional
        // per scope and panic if the sequence differs between runs, so a render
        // that bailed out early used to be a latent crash.
        let theme = use_theme();
        let radio = use_radio(DataChannel::StopsUpdate);
        let departures_slice = radio.slice(DataChannel::DeparturesUpdate, |s| {
            &s.transit_data.departures
        });
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

        let (primary, text_primary) = {
            let theme = theme.read();
            (theme.colors.primary, theme.colors.text_primary)
        };

        let data = radio.read();
        let stop_data = match data.transit_data.stops.get(&self.stop_id) {
            Some(stop_data) => stop_data,
            None => return rect().child(label().text("No stop data")),
        };

        // Resolve the clock once. This used to be `Local::now()` three times per
        // timetable entry — hundreds to thousands of local-timezone lookups per
        // render — plus two `NaiveDate` constructions per entry to compare
        // validity windows. The feed already stores those windows as day
        // numbers, so converting today into the same integer domain lets the
        // comparison be plain integer arithmetic.
        let now = Local::now();
        let now_minutes = now.num_seconds_from_midnight() / 60;
        let weekday = now.weekday().num_days_from_monday() as u8 + 1;
        let today = (now.num_days_from_ce() - FEED_DAY_EPOCH_OFFSET) as u32;

        let departures = departures_slice.read();
        let stop_departures = departures.get(&stop_data.siri_id);
        let depot_destinations = &data.transit_data.depot_destinations;

        // A stop serves one row per route, but the feed defines a route once per
        // variant and `convert_route` dedups on (route_type, destination_key) —
        // so the same line can appear several times here, sometimes even with
        // the same destination name. Group by route and merge the variants.
        let mut grouped: Vec<(RouteType, Vec<Upcoming>)> = Vec::new();

        for route in &stop_data.routes {
            let mut upcoming: Vec<Upcoming> = Vec::new();
            let live = stop_departures.and_then(|live| live.get(&route.route_type));

            // A late departure has not left yet, but its timetabled minute is
            // already in the past. Dropping it would leave the countdown — which
            // is still ticking down to that very departure — with no scheduled
            // time beside it, so the feed's earliest pending entry lowers the
            // cutoff when it has fallen behind.
            let earliest_live = live.and_then(|departures| {
                departures
                    .iter()
                    .map(|departure| u32::from(departure.scheduled_time))
                    .min()
            });
            let cutoff = match earliest_live {
                Some(earliest) if earliest <= now_minutes => earliest.saturating_sub(1),
                _ => now_minutes,
            };

            for (route_weekday, times) in route.weekdays_times.iter() {
                if !route_weekday.is_nth_day(weekday) {
                    continue;
                }

                // `convert_route` leaves each bucket sorted by time, so skip the
                // past in O(log n) and keep only what is actually displayed
                // rather than collecting every remaining departure of the day.
                let start = times.partition_point(|time| time.times <= cutoff);

                upcoming.extend(
                    times[start..]
                        .iter()
                        .filter(|time| {
                            // Past minutes survive only while the feed still
                            // lists them, so a merely-missed departure stays out.
                            (time.times > now_minutes
                                || live.is_some_and(|departures| {
                                    departures.iter().any(|departure| {
                                        u32::from(departure.scheduled_time) == time.times
                                    })
                                }))
                                && time.valid_from <= today
                                && (time.valid_to == 0 || time.valid_to >= today)
                        })
                        .map(|time| Upcoming {
                            minutes: time.times,
                            destination_name: route.destination_name.as_str(),
                            to_depot: route.destination_key == DEPOT_DESTINATION_KEY,
                        })
                        .take(DISPLAYED_SCHEDULED_TIMES),
                );
            }

            match grouped
                .iter_mut()
                .find(|(known, _)| known == &route.route_type)
            {
                Some((_, merged)) => merged.extend(upcoming),
                None => grouped.push((route.route_type.clone(), upcoming)),
            }
        }

        let mut rows: Vec<(RouteType, DepartureTimes)> = Vec::with_capacity(grouped.len());

        for (route_type, mut upcoming) in grouped {
            let live = stop_departures.and_then(|live| live.get(&route_type));

            // Whatever the feed returns is still to come, so it has to appear
            // even when its scheduled minute has already passed and even when
            // the timetable has no entry for it (an extra run, or one filtered
            // out by weekday or validity). The timetable pass above already
            // covered the entries it does know about, which is where the
            // destination and depot flag come from; anything left is added here
            // from the feed itself.
            if let Some(departures) = live {
                for departure in departures {
                    let minutes = u32::from(departure.scheduled_time);
                    if upcoming.iter().any(|known| known.minutes == minutes) {
                        continue;
                    }

                    upcoming.push(Upcoming {
                        minutes,
                        destination_name: departure.destination_name.as_str(),
                        to_depot: depot_destinations.contains(&departure.destination_name),
                    });
                }
            }

            // Each variant contributed its own earliest few, and more than one
            // weekday bucket can match, so order the merged list before cutting
            // it down. Taking the earliest overall is still correct because the
            // global earliest must come from some variant's earliest.
            upcoming.sort_by_key(|upcoming| upcoming.minutes);
            upcoming.truncate(DISPLAYED_SCHEDULED_TIMES);

            // The destination shown is the one the *next* departure is headed
            // to, so it follows the merged timeline rather than being fixed to
            // whichever variant happened to come first in the data.
            let next = upcoming.first();

            // A departure the live feed reports is confirmed; one it does not
            // mention is schedule-only and gets underlined.
            let scheduled_times: Vec<ScheduledTime> = upcoming
                .iter()
                .map(|upcoming| ScheduledTime {
                    text: format_time(upcoming.minutes),
                    schedule_only: !live.is_some_and(|departures| {
                        departures.iter().any(|departure| {
                            u32::from(departure.scheduled_time) == upcoming.minutes
                        })
                    }),
                    to_depot: upcoming.to_depot,
                })
                .collect();

            let destination_name = next
                .map(|next| next.destination_name.to_string())
                .unwrap_or_default();
            let next_scheduled = next.map(|next| next.minutes);

            rows.push((
                route_type,
                DepartureTimes {
                    destination_name,
                    until: None,
                    extra_data: None,
                    next_scheduled,
                    scheduled_times,
                },
            ));
        }

        if let Some(stop_departures) = stop_departures {
            for (route_type, live_departures) in stop_departures {
                let first = match live_departures.first() {
                    Some(first) => first,
                    None => continue,
                };

                if let Some((_, row)) = rows.iter_mut().find(|(known, _)| known == route_type) {
                    // Live data describes the next departure, so it also decides
                    // which destination the row shows.
                    row.destination_name = first.destination_name.clone();
                    row.until = Some(first.until);
                    row.extra_data = first.extra_data.clone();
                } else {
                    // Same rule: everything the feed returns is kept, whether
                    // or not its scheduled minute has already passed. This route
                    // has no timetable entry at this stop, so the depot flag can
                    // only come from the destination name the feed reports.
                    let mut live_times: Vec<(u32, bool)> = live_departures
                        .iter()
                        .map(|departure| {
                            (
                                u32::from(departure.scheduled_time),
                                depot_destinations.contains(&departure.destination_name),
                            )
                        })
                        .collect();
                    live_times.sort_unstable();
                    live_times.dedup_by_key(|(minutes, _)| *minutes);
                    live_times.truncate(DISPLAYED_SCHEDULED_TIMES);

                    rows.push((
                        route_type.clone(),
                        DepartureTimes {
                            destination_name: first.destination_name.clone(),
                            until: Some(first.until),
                            extra_data: first.extra_data.clone(),
                            next_scheduled: live_times.first().map(|(minutes, _)| *minutes),
                            scheduled_times: live_times
                                .into_iter()
                                .map(|(minutes, to_depot)| ScheduledTime {
                                    text: format_time(minutes),
                                    schedule_only: false,
                                    to_depot,
                                })
                                .collect(),
                        },
                    ));
                }
            }
        }

        rows.retain(|(_, times)| times.next_scheduled.is_some() || times.until.is_some());

        // Order by how soon each row departs. A live row knows that directly; a
        // schedule-only row derives it from its next timetabled minute, so the
        // two kinds interleave properly instead of every schedule-only row
        // (which used to share `until == 0`) sinking to the top.
        let seconds_away = |times: &DepartureTimes| match times.until {
            Some(until) => until,
            None => times
                .next_scheduled
                .map(|minute| minute.saturating_sub(now_minutes) * 60)
                .unwrap_or(u32::MAX),
        };

        // Ties are broken explicitly: the previous ordering fell back to
        // `HashMap` iteration order, which is reseeded per map and so
        // reshuffled the visible list on every render.
        rows.sort_by(|a, b| {
            seconds_away(&a.1)
                .cmp(&seconds_away(&b.1))
                .then_with(|| a.1.next_scheduled.cmp(&b.1.next_scheduled))
                .then_with(|| a.0.get_route().cmp(b.0.get_route()))
        });

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
                            .background(primary)
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
                                                    .color(text_primary)
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
                    .children(if !rows.is_empty() {
                        // `rows` is owned and dropped right after, so move the
                        // route type and times into each child rather than
                        // cloning both per row per render.
                        rows.into_iter()
                            .map(|(route_type, departure_times)| {
                                DepartureComponent::new(route_type, departure_times).into_element()
                            })
                            .collect()
                    } else {
                        vec![
                            rect()
                                .width(Size::Fill)
                                .height(Size::px(70.0))
                                .spacing(4.0)
                                .corner_radius(6.0)
                                .background(primary)
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
