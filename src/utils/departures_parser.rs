use std::{collections::HashMap, vec};

use crate::utils::text_utils::parse_csv_line;
use blocking::unblock;
use chrono::Utc;
use revision::revisioned;

pub async fn get_departures(
    stops: Vec<String>,
) -> Result<(HashMap<String, Vec<Departure>>, u32), Box<dyn std::error::Error>> {
    if stops.is_empty() {
        return Err("No stops".into());
    }

    let departures = unblock(
        move || -> Result<(HashMap<String, Vec<Departure>>, u32), String> {
            let mut departures = HashMap::<String, Vec<Departure>>::new();

            let mut next_update: u32 = u32::MAX;

            for chunk in stops.chunks(5) {
                println!(
                    "https://transport.tallinn.ee/siri-stop-departures.php?stopid={}&time={}",
                    chunk.join(","),
                    Utc::now().timestamp_millis()
                );
                let mut response = ureq::get(
                    format!(
                        "https://transport.tallinn.ee/siri-stop-departures.php?stopid={}&time={}",
                        chunk.join(","),
                        Utc::now().timestamp_millis()
                    )
                    .as_str(),
                )
                .call()
                .map_err(|e| e.to_string())?;

                let data = response
                    .body_mut()
                    .read_to_string()
                    .map_err(|e| e.to_string())?;
                let data = data.trim_start_matches('\u{feff}');

                if data.is_empty() || data.starts_with("ERROR") {
                    return Err("API ERROR".to_string());
                }

                let mut lines = data.lines();

                let header: Vec<&str> = lines
                    .next()
                    .unwrap()
                    .trim_start_matches('\u{feff}')
                    .split(',')
                    .collect();

                let type_index = header.iter().position(|x| *x == "Transport").unwrap();
                let route_index = header.iter().position(|x| *x == "RouteNum").unwrap();
                let expected_time_index = header
                    .iter()
                    .position(|x| *x == "ExpectedTimeInSeconds")
                    .unwrap();
                let scheduled_time_index = header
                    .iter()
                    .position(|x| *x == "ScheduleTimeInSeconds")
                    .unwrap();
                let dirsection_index = 4;
                let until_index = 5;
                let extra_data_index = 6;

                let mut current_stop = Option::<String>::None;
                for line in lines {
                    if line.starts_with("#") {
                        continue;
                    }
                    let parts = parse_csv_line(line, ',');

                    let row_type = parts.get(type_index);
                    if row_type.is_none() {
                        continue;
                    }
                    let row_type = row_type.unwrap();

                    if row_type == "stop" && parts.len() >= 2 {
                        current_stop = chunk.iter().find(|e| *e == parts.get(1).unwrap()).cloned();
                        continue;
                    } else if current_stop.is_none() {
                        continue;
                    }

                    let departure_type = RouteType::from(row_type);
                    let route = parts.get(route_index).unwrap().to_string();
                    let expected_time = parts
                        .get(expected_time_index)
                        .unwrap()
                        .parse::<u64>()
                        .unwrap();
                    let scheduled_time = parts
                        .get(scheduled_time_index)
                        .unwrap()
                        .parse::<u64>()
                        .unwrap();
                    let direction = parts.get(dirsection_index).unwrap().to_string();
                    let until = parts.get(until_index).unwrap().parse::<u32>().unwrap();
                    let extra_data = parts
                        .get(extra_data_index)
                        .unwrap_or(&"".to_string())
                        .to_string();

                    if next_update > until {
                        next_update = until;
                    }

                    let current_departures =
                        departures.entry(current_stop.clone().unwrap()).or_default();
                    if let Some(departure) = current_departures.iter_mut().find(|e| {
                        e.route_type == departure_type
                            && e.route == route
                            && e.direction == direction
                    }) {
                        departure.expected_times.push(expected_time);
                        departure.scheduled_times.push(scheduled_time);
                    } else {
                        let departure = Departure {
                            route_type: departure_type,
                            route: route.clone(),
                            expected_times: vec![expected_time],
                            scheduled_times: vec![scheduled_time],
                            direction: direction.clone(),
                            until,
                            extra_data,
                        };

                        current_departures.push(departure);
                    }
                }
            }

            if next_update > 300 {
                next_update = 60;
            } else if next_update > 60 {
                next_update = 30;
            } else {
                next_update = 15;
            }

            for stop in stops {
                if !departures.contains_key(&stop) {
                    departures.insert(stop, Vec::new());
                }
            }

            Ok((departures, next_update))
        },
    )
    .await
    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    Ok(departures)
}

#[revisioned(revision = 1)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RouteType {
    Metro,
    Bus,
    NightBus,
    Trol,
    Tram,
    RegionalBus,
    SuburbanBus,
    CommercialBus,
    IntercityBus,
    InternationalBus,
    SeasonalBus,
    ExpressBus,
    MiniBus,
    Train,
    Plane,
    Festival,
    EventBus,
    Ferry,
    Aquabus,
}

impl From<&String> for RouteType {
    fn from(s: &String) -> Self {
        match s.as_str() {
            "metro" => RouteType::Metro,
            "bus" => RouteType::Bus,
            "nightbus" => RouteType::NightBus,
            "trol" => RouteType::Trol,
            "tram" => RouteType::Tram,
            "regionalbus" => RouteType::RegionalBus,
            "suburbanbus" => RouteType::SuburbanBus,
            "commercialbus" => RouteType::CommercialBus,
            "intercitybus" => RouteType::IntercityBus,
            "internationalbus" => RouteType::InternationalBus,
            "seasonalbus" => RouteType::SeasonalBus,
            "expressbus" => RouteType::ExpressBus,
            "minibus" => RouteType::MiniBus,
            "train" => RouteType::Train,
            "plane" => RouteType::Plane,
            "festival" => RouteType::Festival,
            "eventbus" => RouteType::EventBus,
            "ferry" => RouteType::Ferry,
            "aquabus" => RouteType::Aquabus,
            _ => RouteType::Bus,
        }
    }
}

impl Into<&str> for RouteType {
    fn into(self) -> &'static str {
        match self {
            RouteType::Metro => "metro",
            RouteType::Bus => "bus",
            RouteType::NightBus => "nightbus",
            RouteType::Trol => "trol",
            RouteType::Tram => "tram",
            RouteType::RegionalBus => "regionalbus",
            RouteType::SuburbanBus => "suburbanbus",
            RouteType::CommercialBus => "commercialbus",
            RouteType::IntercityBus => "intercitybus",
            RouteType::InternationalBus => "internationalbus",
            RouteType::SeasonalBus => "seasonalbus",
            RouteType::ExpressBus => "expressbus",
            RouteType::MiniBus => "minibus",
            RouteType::Train => "train",
            RouteType::Plane => "plane",
            RouteType::Festival => "festival",
            RouteType::EventBus => "eventbus",
            RouteType::Ferry => "ferry",
            RouteType::Aquabus => "aquabus",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Departure {
    pub route_type: RouteType,
    pub route: String,
    pub expected_times: Vec<u64>,
    pub scheduled_times: Vec<u64>,
    pub direction: String,
    pub until: u32,
    pub extra_data: String,
}
