use chrono::{DateTime, Utc};
use freya::prelude::{Bytes, Color};
use revision::{from_slice, revisioned, to_vec};
use serde::Serialize;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::utils::{preferences::app_cache_dir, text_utils::parse_csv_line};

static ROUTES_URL: &str = "https://transport.tallinn.ee/data/routes.txt";

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct RawRoute {
    route_num: String,
    authority: String,
    city: String,
    transport: String,
    operator: String,
    validity_periods: Vec<u64>,
    special_dates: Vec<u64>,
    route_tag: String,
    route_type: String,
    commercial: String,
    route_name: String,
    weekdays: String,
    streets: String,
    route_stops: Vec<String>,
    route_stops_platforms: String,
    times: Option<ExplodedTimes>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
struct ExplodedTimes {
    weekdays: Vec<String>,
    valid_from: Vec<i32>,
    valid_to: Vec<i32>,
    low_ground: Vec<bool>,
    times: Vec<Vec<i32>>,
}

#[revisioned(revision = 2)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum RouteType {
    Metro(String),
    Bus(String),
    NightBus(String),
    Trol(String),
    Tram(String),
    RegionalBus(String),
    SuburbanBus(String),
    CommercialBus(String),
    IntercityBus(String),
    InternationalBus(String),
    SeasonalBus(String),
    ExpressBus(String),
    MiniBus(String),
    Train(String),
    Plane(String),
    Festival(String),
    EventBus(String),
    Ferry(String),
    Aquabus(String),
    Unknown(String),
}

impl From<(String, String)> for RouteType {
    fn from(s: (String, String)) -> Self {
        match s.0.as_str() {
            "metro" => RouteType::Metro(s.1),
            "bus" => RouteType::Bus(s.1),
            "nightbus" => RouteType::NightBus(s.1),
            "trol" => RouteType::Trol(s.1),
            "tram" => RouteType::Tram(s.1),
            "regionalbus" => RouteType::RegionalBus(s.1),
            "suburbanbus" => RouteType::SuburbanBus(s.1),
            "commercialbus" => RouteType::CommercialBus(s.1),
            "intercitybus" => RouteType::IntercityBus(s.1),
            "internationalbus" => RouteType::InternationalBus(s.1),
            "seasonalbus" => RouteType::SeasonalBus(s.1),
            "expressbus" => RouteType::ExpressBus(s.1),
            "minibus" => RouteType::MiniBus(s.1),
            "train" => RouteType::Train(s.1),
            "plane" => RouteType::Plane(s.1),
            "festival" => RouteType::Festival(s.1),
            "eventbus" => RouteType::EventBus(s.1),
            "ferry" => RouteType::Ferry(s.1),
            "aquabus" => RouteType::Aquabus(s.1),
            _ => RouteType::Unknown(s.1),
        }
    }
}

impl RouteType {
    pub fn get_route(&self) -> &str {
        match self {
            RouteType::Metro(route)
            | RouteType::Bus(route)
            | RouteType::NightBus(route)
            | RouteType::Trol(route)
            | RouteType::Tram(route)
            | RouteType::RegionalBus(route)
            | RouteType::SuburbanBus(route)
            | RouteType::CommercialBus(route)
            | RouteType::IntercityBus(route)
            | RouteType::InternationalBus(route)
            | RouteType::SeasonalBus(route)
            | RouteType::ExpressBus(route)
            | RouteType::MiniBus(route)
            | RouteType::Train(route)
            | RouteType::Plane(route)
            | RouteType::Festival(route)
            | RouteType::EventBus(route)
            | RouteType::Ferry(route)
            | RouteType::Aquabus(route)
            | RouteType::Unknown(route) => route,
        }
    }
    pub fn get_transport_icon_and_color(&self) -> (Bytes, Color) {
        match self {
            // Copied from https://transport.tallinn.ee CSS
            RouteType::Metro(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/subway-variant.svg")),
                Color::from_rgb(0xff, 0x6a, 0x00),
            ),
            RouteType::Bus(..) | RouteType::NightBus(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/bus.svg")),
                Color::from_rgb(0x00, 0xe1, 0xb4),
            ),
            RouteType::Trol(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/bus.svg")),
                Color::from_rgb(0x00, 0x64, 0xd7),
            ),
            RouteType::Tram(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/tram.svg")),
                Color::from_rgb(0xff, 0x60, 0x1e),
            ),
            RouteType::RegionalBus(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/bus.svg")),
                Color::from_rgb(0x9c, 0x16, 0x30),
            ),
            RouteType::SuburbanBus(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/bus.svg")),
                Color::from_rgb(0x00, 0x4a, 0x7f),
            ),
            RouteType::CommercialBus(..)
            | RouteType::IntercityBus(..)
            | RouteType::InternationalBus(..)
            | RouteType::SeasonalBus(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/bus.svg")),
                Color::from_rgb(0x80, 0x00, 0x80),
            ),
            RouteType::ExpressBus(..) | RouteType::MiniBus(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/bus.svg")),
                Color::from_rgb(0x00, 0x80, 0x00),
            ),
            RouteType::Train(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/train.svg")),
                Color::from_rgb(0x00, 0x99, 0x00),
            ),
            RouteType::Plane(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/airplane.svg")),
                Color::from_rgb(0x40, 0x40, 0x40),
            ),
            RouteType::Festival(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/bus.svg")),
                Color::from_rgb(0xff, 0xa5, 0x00),
            ),
            RouteType::EventBus(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/bus.svg")),
                Color::from_rgb(0xff, 0x6a, 0x00),
            ),
            RouteType::Ferry(..) | RouteType::Aquabus(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/ferry.svg")),
                Color::from_rgb(0x00, 0x64, 0xd7),
            ),
            RouteType::Unknown(..) => (
                Bytes::from_static(include_bytes!("../../../assets/MDI/help.svg")),
                Color::from_rgb(0x00, 0x00, 0x00),
            ),
        }
    }
}

/// `destination_key` of a run that ends at the depot instead of serving the
/// line. Derived from the trailing segment of the raw route type, e.g. "b-dp".
pub const DEPOT_DESTINATION_KEY: &str = "dp";

//                        StopId
pub type Routes = HashMap<String, Vec<Route>>;

#[revisioned(revision = 4)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Route {
    pub route_type: RouteType,
    pub weekdays_times: HashMap<Weekdays, Vec<WeekdaysTime>>,
    pub route_name: String,
    pub route_key: String,
    pub destination_key: String,
    pub destination_name: String,
    pub is_night: bool,
}

#[revisioned(revision = 1)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Weekdays {
    Workdays,
    Weekends,
    Saturday,
    Sunday,
    All,
    Other(u8),
}

impl Weekdays {
    pub fn is_nth_day(&self, n: u8) -> bool {
        if n == 0 || n > 7 {
            return false;
        }

        match self {
            Weekdays::Workdays => n < 6,
            Weekdays::Weekends => n >= 6,
            Weekdays::Saturday => n == 6,
            Weekdays::Sunday => n == 7,
            Weekdays::All => true,
            Weekdays::Other(day) => *day == n,
        }
    }
}

#[revisioned(revision = 1)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WeekdaysTime {
    pub times: u32,
    pub valid_from: u32,
    pub valid_to: u32,
}

pub async fn get_routes() -> Result<Routes, Box<dyn std::error::Error>> {
    let current_last_modified = match fs::read_to_string(get_routes_last_modified_file_path()) {
        Ok(contents) => match contents.trim().parse::<i64>() {
            Ok(timestamp) => {
                DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap_or(DateTime::<Utc>::MIN_UTC)
            }
            Err(_) => DateTime::<Utc>::MIN_UTC,
        },
        Err(_) => DateTime::<Utc>::MIN_UTC,
    };
    // `get_last_modified_version` is a synchronous `ureq::head()`; running it
    // directly in the async body stalled the UI task at startup.
    let target_last_modified = blocking::unblock(get_last_modified_version).await;
    println!("Current routes last modified: {current_last_modified:?}");
    println!("Target routes last modified: {target_last_modified:?}");
    if get_routes_file_path().exists() && current_last_modified >= target_last_modified {
        let routes = blocking::unblock(|| {
            let bytes = match fs::read(get_routes_file_path()) {
                Ok(bytes) => bytes,
                Err(err) => return Err(err.to_string()),
            };

            let data: Routes = match from_slice(&bytes) {
                Ok(data) => data,
                Err(err) => return Err(err.to_string()),
            };
            Ok::<Routes, String>(data)
        })
        .await;

        match routes {
            Ok(routes) => return Ok(routes),
            Err(err) => println!("Error reading routes: {err}"),
        }
    }

    let routes = blocking::unblock(|| {
        let mut result = ureq::get(ROUTES_URL).call()?;
        let body = result.body_mut().read_to_string()?;

        Ok(parse_routes(body))
    })
    .await
    .map_err(|e: ureq::Error| -> Box<dyn std::error::Error> { Box::new(e) })?;

    fs::write(
        get_routes_last_modified_file_path(),
        Utc::now().timestamp().to_string(),
    )?;

    let routes = convert_route(routes);

    let bytes = to_vec(&routes).unwrap();
    fs::write(get_routes_file_path(), &bytes)?;

    Ok(routes)
}

fn parse_routes(data: String) -> Vec<RawRoute> {
    let mut lines = data.lines();

    let header: Vec<&str> = lines
        .next()
        .unwrap()
        .trim_start_matches('\u{feff}')
        .split(';')
        .collect();

    let route_num_index = header.iter().position(|x| *x == "RouteNum").unwrap();
    let authority_index = header.iter().position(|x| *x == "Authority").unwrap();
    let city_index = header.iter().position(|x| *x == "City").unwrap();
    let transport_index = header.iter().position(|x| *x == "Transport").unwrap();
    let operator_index = header.iter().position(|x| *x == "Operator").unwrap();
    let validity_periods_index = header.iter().position(|x| *x == "ValidityPeriods").unwrap();
    let special_dates_index = header.iter().position(|x| *x == "SpecialDates").unwrap();
    let route_tag_index = header.iter().position(|x| *x == "RouteTag").unwrap();
    let route_type_index = header.iter().position(|x| *x == "RouteType").unwrap();
    let commercial_index = header.iter().position(|x| *x == "Commercial").unwrap();
    let route_name_index = header.iter().position(|x| *x == "RouteName").unwrap();
    let weekdays_index = header.iter().position(|x| *x == "Weekdays").unwrap();
    let streets_index = header.iter().position(|x| *x == "Streets").unwrap();
    let route_stops_index = header.iter().position(|x| *x == "RouteStops").unwrap();
    let route_stops_platforms_index = header
        .iter()
        .position(|x| *x == "RouteStopsPlatforms")
        .unwrap();

    let header_len = header.len();
    let mut previous_parts = vec![String::new(); header_len];

    let mut routes: Vec<RawRoute> = Vec::new();
    for (i, line) in lines.enumerate() {
        if i == 0 {
            continue;
        }

        if line.starts_with("#") {
            continue;
        }

        // Cheap discriminator first: the times rows carry no delimiter at all,
        // and splitting them is the most expensive parse in the file.
        if !line.contains(';') {
            let times = explode_times(line);
            let routes_len = routes.len();
            routes[routes_len - 1].times = Some(times);
            continue;
        }

        let mut parts = parse_csv_line(line, ';');

        if parts.len() < header_len {
            parts.resize(header_len, "".to_string());
        }

        for (j, part) in parts.iter_mut().enumerate() {
            if part.trim().is_empty() {
                *part = previous_parts[j].clone();
            } else {
                previous_parts[j] = part.clone();
            }
        }

        if parts[authority_index] == "SpecialDates" {
            continue;
        }

        let route_num = parts[route_num_index].clone();
        let authority = parts[authority_index].clone();
        let city = parts[city_index].clone();
        let transport = parts[transport_index].clone();
        let operator = parts[operator_index].clone();
        let validity_periods = parts[validity_periods_index]
            .split(",")
            .filter_map(|e| match e.parse::<u64>() {
                Ok(value) => Some(value),
                Err(_) => None,
            })
            .collect();
        let special_dates = parts[special_dates_index]
            .split(",")
            .filter_map(|e| match e.parse::<u64>() {
                Ok(value) => Some(value),
                Err(_) => None,
            })
            .collect();
        let route_tag = parts[route_tag_index].clone();
        let route_type = parts[route_type_index].clone();
        let commercial = parts[commercial_index].clone();
        let route_name = parts[route_name_index].clone();
        let weekdays = parts[weekdays_index].clone();
        let streets = parts[streets_index].clone();
        let route_stops = parts[route_stops_index]
            .split(",")
            .map(|e| e.to_string())
            .collect();
        let route_stops_platforms = parts[route_stops_platforms_index].clone();

        let route = RawRoute {
            route_num,
            authority,
            city,
            transport,
            operator,
            validity_periods,
            special_dates,
            route_tag,
            route_type,
            commercial,
            route_name,
            weekdays,
            streets,
            route_stops,
            route_stops_platforms,
            times: None,
        };

        routes.push(route);
    }

    routes
}

pub fn get_last_modified_version() -> DateTime<Utc> {
    let response = ureq::head(ROUTES_URL).call();
    match response {
        Ok(response) => match response.headers().get("Last-Modified") {
            Some(last_modified) => DateTime::parse_from_rfc2822(last_modified.to_str().unwrap())
                .unwrap()
                .to_utc(),
            None => DateTime::<Utc>::MAX_UTC,
        },
        Err(e) => {
            log::error!("Error getting last modified version: {e}");
            DateTime::<Utc>::MAX_UTC
        }
    }
}

pub fn get_routes_file_path() -> PathBuf {
    app_cache_dir().join("routes.dat")
}

pub fn get_routes_last_modified_file_path() -> PathBuf {
    app_cache_dir().join("routes_last_modified.dat")
}

/// Per-trip data that is identical for every stop along a route.
///
/// Only `ExplodedTimes::times[stop][trip]` varies with the stop, so the weekday
/// classification and validity window are resolved once per trip instead of
/// once per (stop, trip) pair.
struct Trip<'a> {
    weekday: Option<Weekdays>,
    raw_weekdays: &'a str,
    valid_from: u32,
    valid_to: u32,
}

fn convert_route(routes: Vec<RawRoute>) -> Routes {
    let mut converted_routes: Routes = HashMap::new();

    for route in routes {
        // All route-level, so compute once rather than once per stop served.
        let route_type = RouteType::from((route.transport.clone(), route.route_num.clone()));
        let destination_key = route
            .route_type
            .split("-")
            .last()
            .unwrap_or(&route.route_type)
            .to_string();
        let destination_name = route
            .route_name
            .split(" - ")
            .last()
            .unwrap_or(&route.route_name)
            .to_string();
        let is_night = route.route_name.starts_with("ÖÖ");

        let trips: Vec<Trip> = match route.times {
            Some(ref times) => times
                .weekdays
                .iter()
                .enumerate()
                .map(|(j, weekdays)| {
                    let weekday = match weekdays.as_str() {
                        "1234567" => Some(Weekdays::All),
                        "12345" => Some(Weekdays::Workdays),
                        "67" => Some(Weekdays::Weekends),
                        "6" => Some(Weekdays::Saturday),
                        "7" => Some(Weekdays::Sunday),
                        _ => {
                            log::trace!("Unknown weekday: {weekdays}");
                            None
                        }
                    };

                    Trip {
                        weekday,
                        raw_weekdays: weekdays.as_str(),
                        valid_from: times.valid_from[j] as u32,
                        valid_to: times.valid_to[j] as u32,
                    }
                })
                .collect(),
            None => Vec::new(),
        };

        let make_route = |weekdays_times| Route {
            route_type: route_type.clone(),
            weekdays_times,
            destination_name: destination_name.clone(),
            destination_key: destination_key.clone(),
            route_name: route.route_name.clone(),
            route_key: route.route_type.clone(),
            is_night,
        };

        for (i, route_stop) in route.route_stops.iter().enumerate() {
            let mut weekdays_times: HashMap<Weekdays, Vec<WeekdaysTime>> = HashMap::new();

            if let Some(ref times) = route.times {
                for (j, trip) in trips.iter().enumerate() {
                    let dep_times = times.times[i][j];
                    if dep_times < 0 {
                        continue;
                    }

                    let entry = WeekdaysTime {
                        times: dep_times as u32,
                        valid_from: trip.valid_from,
                        valid_to: trip.valid_to,
                    };

                    match trip.weekday {
                        Some(ref weekday) => {
                            weekdays_times
                                .entry(weekday.clone())
                                .or_default()
                                .push(entry);
                        }
                        None => {
                            for day in trip.raw_weekdays.chars() {
                                let Some(day) = day.to_digit(10) else {
                                    continue;
                                };
                                weekdays_times
                                    .entry(Weekdays::Other(day as u8))
                                    .or_default()
                                    .push(entry.clone());
                            }
                        }
                    }
                }

                if let Some(saturday) = weekdays_times.get(&Weekdays::Saturday)
                    && let Some(sunday) = weekdays_times.get(&Weekdays::Sunday)
                    && saturday == sunday
                {
                    let saturday = weekdays_times.remove(&Weekdays::Saturday).unwrap();
                    weekdays_times.remove(&Weekdays::Sunday);
                    weekdays_times.insert(Weekdays::Weekends, saturday);
                }
            }

            if let Some(stop_routes) = converted_routes.get_mut(route_stop) {
                let existing = stop_routes.iter_mut().find(|existing| {
                    existing.destination_key == destination_key && existing.route_type == route_type
                });

                match existing {
                    Some(existing) => {
                        for (key, existing_times) in existing.weekdays_times.iter_mut() {
                            if let Some(incoming) = weekdays_times.get(key) {
                                // Sorting is deferred to one pass at the end
                                // rather than re-sorting the whole accumulated
                                // vector after every merge.
                                existing_times.extend_from_slice(incoming);
                            }
                        }
                    }
                    None => stop_routes.push(make_route(weekdays_times)),
                }
            } else {
                converted_routes.insert(route_stop.clone(), vec![make_route(weekdays_times)]);
            }
        }
    }

    // `StopComponent` relies on each bucket being ordered by time so it can skip
    // past departures with a binary search.
    for stop_routes in converted_routes.values_mut() {
        for route in stop_routes.iter_mut() {
            for times in route.weekdays_times.values_mut() {
                times.sort_by_key(|time| time.times);
            }
        }
    }

    converted_routes
}

fn parse_i32_lossy(token: &str, malformed_tokens: &mut Vec<String>) -> i32 {
    match token.trim().parse::<i32>() {
        Ok(value) => value,
        Err(_) => {
            malformed_tokens.push(token.trim().to_string());
            0
        }
    }
}

fn parse_usize_lossy(token: &str, malformed_tokens: &mut Vec<String>) -> usize {
    match token.trim().parse::<usize>() {
        Ok(value) => value,
        Err(_) => {
            malformed_tokens.push(token.trim().to_string());
            0
        }
    }
}

fn decode_rle_i32(
    tokens: &[&str],
    cursor: &mut usize,
    width: usize,
    malformed_tokens: &mut Vec<String>,
) -> Vec<i32> {
    let mut out = Vec::with_capacity(width);

    while *cursor < tokens.len() && out.len() < width {
        let value = parse_i32_lossy(tokens[*cursor], malformed_tokens);
        *cursor += 1;

        if *cursor >= tokens.len() {
            out.push(value);
            break;
        }

        let count_token = tokens[*cursor].trim();
        *cursor += 1;

        let count = if count_token.is_empty() {
            width - out.len()
        } else {
            parse_usize_lossy(count_token, malformed_tokens)
        };

        let count = count.min(width - out.len());
        out.extend(std::iter::repeat_n(value, count));

        if count_token.is_empty() {
            break;
        }
    }

    if out.len() < width {
        out.resize(width, 0);
    }

    out
}

fn decode_rle_string(
    tokens: &[&str],
    cursor: &mut usize,
    width: usize,
    malformed_tokens: &mut Vec<String>,
) -> Vec<String> {
    let mut out = Vec::with_capacity(width);

    while *cursor < tokens.len() && out.len() < width {
        let value = tokens[*cursor].trim().to_owned();
        *cursor += 1;

        if *cursor >= tokens.len() {
            out.push(value);
            break;
        }

        let count_token = tokens[*cursor].trim();
        *cursor += 1;

        let count = if count_token.is_empty() {
            width - out.len()
        } else {
            parse_usize_lossy(count_token, malformed_tokens)
        };

        let count = count.min(width - out.len());
        out.extend(std::iter::repeat_n(value.clone(), count));

        if count_token.is_empty() {
            break;
        }
    }

    if out.len() < width {
        out.resize(width, String::new());
    }

    out
}

fn explode_times(encoded_times: &str) -> ExplodedTimes {
    let tokens: Vec<&str> = encoded_times.split(',').collect();
    if tokens.is_empty() {
        return ExplodedTimes::default();
    }

    let mut malformed_tokens: Vec<String> = Vec::new();

    // Stage 1: decode start times for all trips and low-floor flags.
    let mut cursor = 0usize;
    let mut start_times = Vec::new();
    let mut low_ground = Vec::new();
    let mut previous_time = 0i32;

    while cursor < tokens.len() {
        let token = tokens[cursor].trim();
        if token.is_empty() {
            cursor += 1;
            break;
        }

        let bytes = token.as_bytes();
        let is_low_ground = bytes.first() == Some(&b'+')
            || (bytes.first() == Some(&b'-') && bytes.get(1) == Some(&b'0'));

        previous_time += parse_i32_lossy(token, &mut malformed_tokens);
        start_times.push(previous_time);
        low_ground.push(is_low_ground);
        cursor += 1;
    }

    let width = start_times.len();
    if width == 0 {
        return ExplodedTimes::default();
    }

    // Stage 2-4: decode validity ranges and weekdays using run-length encoded pairs.
    let valid_from = decode_rle_i32(&tokens, &mut cursor, width, &mut malformed_tokens);
    let valid_to = decode_rle_i32(&tokens, &mut cursor, width, &mut malformed_tokens);
    let weekdays = decode_rle_string(&tokens, &mut cursor, width, &mut malformed_tokens);

    // Stage 5: decode row-by-row travel-time deltas to produce absolute minutes for each stop.
    let mut flat_minutes = start_times;
    let mut column_index = width;
    let mut columns_left = width;
    let mut driving_delta = 5i32;

    while cursor + 1 < tokens.len() {
        let delta_token = tokens[cursor].trim();
        cursor += 1;

        if delta_token.is_empty() {
            continue;
        }

        driving_delta += parse_i32_lossy(delta_token, &mut malformed_tokens) - 5;

        let count_token = tokens[cursor].trim();
        cursor += 1;

        let mut count = if count_token.is_empty() {
            columns_left
        } else {
            parse_usize_lossy(count_token, &mut malformed_tokens)
        };

        count = count.min(columns_left);
        columns_left -= count;

        for _ in 0..count {
            let previous_index = column_index.saturating_sub(width);
            if previous_index >= flat_minutes.len() {
                break;
            }

            let next_value = flat_minutes[previous_index] + driving_delta;
            flat_minutes.push(next_value);
            column_index += 1;
        }

        if columns_left == 0 {
            columns_left = width;
            driving_delta = 5;
        }
    }

    // Convert flat minutes into stop rows of raw minute values.
    let mut timetable = Vec::with_capacity(flat_minutes.len().div_ceil(width));

    for row in flat_minutes.chunks(width) {
        let mut stop_times = Vec::with_capacity(row.len());
        for minute in row {
            stop_times.push(*minute);
        }
        timetable.push(stop_times);
    }

    if !malformed_tokens.is_empty() {
        eprintln!(
            "explode_times: malformed token(s): {}",
            malformed_tokens.join(", ")
        );
    }

    ExplodedTimes {
        weekdays,
        valid_from,
        valid_to,
        low_ground,
        times: timetable,
    }
}
