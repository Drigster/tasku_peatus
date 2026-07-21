use blocking::unblock;
use chrono::Utc;
use freya::prelude::{Bytes, Color};
use revision::revisioned;
use std::{collections::HashMap, vec};

use crate::utils::text_utils::parse_csv_line;
//                                                        destination_name
pub type Departures = HashMap<String, HashMap<(RouteType, String), Departure>>;

pub async fn get_departures(
    stops: Vec<String>,
) -> Result<(Departures, u32), Box<dyn std::error::Error>> {
    if stops.is_empty() {
        return Err("No stops".into());
    }

    let departures = unblock(move || -> Result<(Departures, u32), String> {
        let mut departures: Departures = HashMap::new();

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

                let route = parts.get(route_index).unwrap().to_string();
                let departure_type = RouteType::from((row_type.clone(), route));
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
                if let Some(departure) = current_departures
                    .iter_mut()
                    .find(|e| *e.0 == (departure_type.clone(), direction.clone()))
                {
                    departure.1.expected_times.push(expected_time);
                    departure.1.scheduled_times.push(scheduled_time);
                } else {
                    let departure = Departure {
                        expected_times: vec![expected_time],
                        scheduled_times: vec![scheduled_time],
                        destination_name: direction.clone(),
                        until,
                        extra_data,
                    };

                    current_departures.insert((departure_type, direction), departure);
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
                departures.insert(stop, HashMap::new());
            }
        }

        Ok((departures, next_update))
    })
    .await
    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    Ok(departures)
}

#[revisioned(revision = 2)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    pub fn get_route(&self) -> String {
        return match self {
            RouteType::Metro(route) => route.clone(),
            RouteType::Bus(route) => route.clone(),
            RouteType::NightBus(route) => route.clone(),
            RouteType::Trol(route) => route.clone(),
            RouteType::Tram(route) => route.clone(),
            RouteType::RegionalBus(route) => route.clone(),
            RouteType::SuburbanBus(route) => route.clone(),
            RouteType::CommercialBus(route) => route.clone(),
            RouteType::IntercityBus(route) => route.clone(),
            RouteType::InternationalBus(route) => route.clone(),
            RouteType::SeasonalBus(route) => route.clone(),
            RouteType::ExpressBus(route) => route.clone(),
            RouteType::MiniBus(route) => route.clone(),
            RouteType::Train(route) => route.clone(),
            RouteType::Plane(route) => route.clone(),
            RouteType::Festival(route) => route.clone(),
            RouteType::EventBus(route) => route.clone(),
            RouteType::Ferry(route) => route.clone(),
            RouteType::Aquabus(route) => route.clone(),
            RouteType::Unknown(route) => route.clone(),
        };
    }
    pub fn get_transport_icon_and_color(&self) -> (Bytes, Color) {
        match self {
            // Copied from https://transport.tallinn.ee CSS
            RouteType::Metro(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/subway-variant.svg")),
                Color::from_hex("0xff6A00").unwrap(),
            ),
            RouteType::Bus(..) | RouteType::NightBus(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
                Color::from_hex("0x00e1b4").unwrap(),
            ),
            RouteType::Trol(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
                Color::from_hex("0x0064d7").unwrap(),
            ),
            RouteType::Tram(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/tram.svg")),
                Color::from_hex("0xff601e").unwrap(),
            ),
            RouteType::RegionalBus(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
                Color::from_hex("0x9c1630").unwrap(),
            ),
            RouteType::SuburbanBus(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
                Color::from_hex("0x004a7f").unwrap(),
            ),
            RouteType::CommercialBus(..)
            | RouteType::IntercityBus(..)
            | RouteType::InternationalBus(..)
            | RouteType::SeasonalBus(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
                Color::from_hex("0x800080").unwrap(),
            ),
            RouteType::ExpressBus(..) | RouteType::MiniBus(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
                Color::from_hex("0x008000").unwrap(),
            ),
            RouteType::Train(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/train.svg")),
                Color::from_hex("0x009900").unwrap(),
            ),
            RouteType::Plane(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/airplane.svg")),
                Color::from_hex("0x404040").unwrap(),
            ),
            RouteType::Festival(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
                Color::from_hex("0xffa500").unwrap(),
            ),
            RouteType::EventBus(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
                Color::from_hex("0xff6a00").unwrap(),
            ),
            RouteType::Ferry(..) | RouteType::Aquabus(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/ferry.svg")),
                Color::from_hex("0x0064d7").unwrap(),
            ),
            RouteType::Unknown(..) => (
                Bytes::from_static(include_bytes!("../assets/MDI/help.svg")),
                Color::from_hex("0x000000").unwrap(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Departure {
    pub expected_times: Vec<u64>,
    pub scheduled_times: Vec<u64>,
    pub destination_name: String,
    pub until: u32,
    pub extra_data: String,
}
