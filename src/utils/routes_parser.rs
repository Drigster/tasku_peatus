use chrono::{DateTime, Utc};
use std::{collections::HashMap, fs, io, path::PathBuf};

use crate::utils::{preferences::get_cache_dir, text_utils::parse_csv_line};

static ROUTES_URL: &str = "https://transport.tallinn.ee/data/routes.txt";

pub async fn get_routes()
-> Result<HashMap<String, HashMap<String, DepartureTimes>>, Box<dyn std::error::Error>> {
    let last_modified = DateTime::<Utc>::MAX_UTC;
    if last_modified < get_last_modified_version() || !get_routes_file_path().exists() {
        let routes = blocking::unblock(|| {
            let mut result = ureq::get(ROUTES_URL).call()?;
            let body = result.body_mut().read_to_string()?;

            fs::write(get_routes_file_path(), &body)?;

            Ok(parse_routes(body))
        })
        .await
        .map_err(|e: ureq::Error| -> Box<dyn std::error::Error> { Box::new(e) })?;
        Ok(convert_route(routes))
    } else {
        let routes = blocking::unblock(|| {
            let data = fs::read_to_string(get_routes_file_path())?;
            Ok::<Vec<Route>, io::Error>(parse_routes(data))
        })
        .await?;

        Ok(convert_route(routes))
    }
}

fn parse_routes(data: String) -> Vec<Route> {
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

    let mut routes: Vec<Route> = Vec::new();
    for (i, line) in lines.enumerate() {
        if i == 0 {
            continue;
        }

        if line.starts_with("#") {
            continue;
        }

        let mut parts = parse_csv_line(line, ';');

        if parts.len() < header_len {
            parts.resize(header_len, "".to_string());
        }

        if !line.contains(&";".to_string()) {
            let times = explode_times(line);
            let routes_len = routes.len();
            routes[routes_len - 1].times = Some(times);
            continue;
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
            .clone()
            .split(",")
            .filter_map(|e| match e.parse::<u64>() {
                Ok(value) => Some(value),
                Err(_) => None,
            })
            .collect();
        let special_dates = parts[special_dates_index]
            .clone()
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
            .clone()
            .split(",")
            .map(|e| e.to_string())
            .collect();
        let route_stops_platforms = parts[route_stops_platforms_index].clone();

        let route = Route {
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
                .into(),
            None => DateTime::<Utc>::MIN_UTC,
        },
        Err(e) => {
            log::error!("Error getting last modified version: {e}");
            DateTime::<Utc>::MIN_UTC
        }
    }
}

pub fn get_routes_file_path() -> PathBuf {
    let cache_dir = get_cache_dir().unwrap();
    cache_dir.join("routes.txt")
}

fn convert_route(routes: Vec<Route>) -> HashMap<String, HashMap<String, DepartureTimes>> {
    //                                StopId          RouteNum        WeekDays
    let mut converted_routes: HashMap<String, HashMap<String, DepartureTimes>> = HashMap::new();

    for route in routes {
        for (i, route_stop) in route.route_stops.into_iter().enumerate() {
            let stop_times = match route.times {
                Some(ref times) => times
                    .times
                    .get(i)
                    .expect("Expected stops and times to be same length")
                    .clone(),
                None => Vec::new(),
            };

            let new_route = DepartureTimes {
                route_num: route.route_num.clone(),
                transport: route.transport.clone(),
                times: stop_times,
            };
            let stop_routes = converted_routes.get_mut(&route_stop);
            if let Some(stop_routes) = stop_routes {
                stop_routes.insert(route.route_num.clone(), new_route);
            } else {
                converted_routes.insert(
                    route_stop,
                    HashMap::from([(route.route_num.clone(), new_route)]),
                );
            }
        }
    }

    converted_routes
}

#[allow(dead_code)]
struct Route {
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

#[derive(Debug, Clone, PartialEq)]
pub struct DepartureTimes {
    pub route_num: String,
    pub transport: String,
    pub times: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct ExplodedTimes {
    weekdays: Vec<String>,
    valid_from: Vec<i32>,
    valid_to: Vec<i32>,
    low_ground: Vec<bool>,
    times: Vec<Vec<u32>>,
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
    // Negative values cannot be represented as u32, so skip and report once.
    let mut skipped_negative_count = 0usize;
    let mut timetable = Vec::with_capacity(flat_minutes.len().div_ceil(width));

    for row in flat_minutes.chunks(width) {
        let mut stop_times = Vec::with_capacity(row.len());
        for minute in row {
            if *minute < 0 {
                skipped_negative_count += 1;
                continue;
            }

            stop_times.push(*minute as u32);
        }
        timetable.push(stop_times);
    }

    if skipped_negative_count > 0 {
        eprintln!("explode_times: skipped {skipped_negative_count} negative time value(s)");
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
