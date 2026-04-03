use std::{collections::HashMap, vec};

use crate::utils::text_utils::parse_csv_line;
use blocking::unblock;
use chrono::Utc;

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

                    let departure_type = DepartureType::from(row_type);
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
                        e.departure_type == departure_type
                            && e.route == route
                            && e.direction == direction
                    }) {
                        departure.scheduled_times.push(scheduled_time);
                    } else {
                        let departure = Departure {
                            departure_type,
                            route: route.clone(),
                            expected_time,
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

            Ok((departures, next_update))
        },
    )
    .await
    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    Ok(departures)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepartureType {
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

impl From<&String> for DepartureType {
    fn from(s: &String) -> Self {
        match s.as_str() {
            "metro" => DepartureType::Metro,
            "bus" => DepartureType::Bus,
            "nightbus" => DepartureType::NightBus,
            "trol" => DepartureType::Trol,
            "tram" => DepartureType::Tram,
            "regionalbus" => DepartureType::RegionalBus,
            "suburbanbus" => DepartureType::SuburbanBus,
            "commercialbus" => DepartureType::CommercialBus,
            "intercitybus" => DepartureType::IntercityBus,
            "internationalbus" => DepartureType::InternationalBus,
            "seasonalbus" => DepartureType::SeasonalBus,
            "expressbus" => DepartureType::ExpressBus,
            "minibus" => DepartureType::MiniBus,
            "train" => DepartureType::Train,
            "plane" => DepartureType::Plane,
            "festival" => DepartureType::Festival,
            "eventbus" => DepartureType::EventBus,
            "ferry" => DepartureType::Ferry,
            "aquabus" => DepartureType::Aquabus,
            _ => DepartureType::Bus,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Departure {
    pub departure_type: DepartureType,
    pub route: String,
    pub expected_time: u64,
    pub scheduled_times: Vec<u64>,
    pub direction: String,
    pub until: u32,
    pub extra_data: String,
}
