use blocking::unblock;
use chrono::Utc;
use revision::revisioned;
use serde::Serialize;
use std::collections::HashMap;

use crate::utils::{text_utils::parse_csv_line, transit::parsers::routes::RouteType};

#[revisioned(revision = 1)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Departure {
    pub expected_time: u32,
    pub scheduled_time: u16,
    pub destination_name: String,
    pub until: u32,
    pub extra_data: Option<String>,
}

//                            siri_id
pub type Departures = HashMap<String, HashMap<RouteType, Vec<Departure>>>;

/// Fetches one chunk of stops. Separate so the chunks can be in flight at the
/// same time instead of one blocking round trip after another.
async fn fetch_chunk(chunk: Vec<String>) -> Result<(Vec<String>, String), String> {
    unblock(move || {
        let url = format!(
            "https://transport.tallinn.ee/siri-stop-departures.php?stopid={}&time={}",
            chunk.join(","),
            Utc::now().timestamp_millis()
        );
        log::debug!("Fetching departures: {url}");

        let mut response = ureq::get(&url).call().map_err(|e| e.to_string())?;
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())?;

        Ok((chunk, body))
    })
    .await
}

pub async fn get_departures(
    siri_ids: Vec<String>,
) -> Result<(Departures, u32), Box<dyn std::error::Error>> {
    if siri_ids.is_empty() {
        return Err("No stops".into());
    }

    // `unblock` schedules onto the thread pool as soon as the task is created,
    // so starting every chunk before awaiting any of them puts the requests in
    // flight together. With ~20 stops in radius this turns four serialized
    // round trips per refresh into one.
    let fetches: Vec<_> = siri_ids
        .chunks(5)
        .map(|chunk| fetch_chunk(chunk.to_vec()))
        .collect();

    let mut responses = Vec::with_capacity(fetches.len());
    for fetch in fetches {
        responses.push(fetch.await?);
    }

    let departures = unblock(move || -> Result<(Departures, u32), String> {
        let mut departures: Departures = HashMap::new();

        let mut next_update: u32 = u32::MAX;

        for (chunk, data) in &responses {
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

            let mut current_stop_siri_id = Option::<String>::None;
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
                    current_stop_siri_id =
                        chunk.iter().find(|e| *e == parts.get(1).unwrap()).cloned();
                    continue;
                } else if current_stop_siri_id.is_none() {
                    continue;
                }

                let route = parts.get(route_index).unwrap().to_string();
                let departure_type = RouteType::from((row_type.clone(), route));
                let expected_time = parts
                    .get(expected_time_index)
                    .unwrap()
                    .parse::<u32>()
                    .unwrap();
                let scheduled_time = parts
                    .get(scheduled_time_index)
                    .unwrap()
                    .parse::<u32>()
                    .unwrap();
                // scheduled time is in seconds by default, convert to minutes to match routes
                let scheduled_time = (scheduled_time / 60) as u16;

                let direction = parts.get(dirsection_index).unwrap().to_string();
                let until = parts.get(until_index).unwrap().parse::<u32>().unwrap();
                let extra_data = parts
                    .get(extra_data_index)
                    .map(String::as_str)
                    .unwrap_or("")
                    .to_string();

                if next_update > until {
                    next_update = until;
                }

                // Allocate the key only on first insert, rather than cloning
                // the siri id for every departure row.
                let siri_id = current_stop_siri_id.as_deref().unwrap();
                if !departures.contains_key(siri_id) {
                    departures.insert(siri_id.to_string(), HashMap::new());
                }

                departures
                    .get_mut(siri_id)
                    .unwrap()
                    .entry(departure_type)
                    .or_default()
                    .push(Departure {
                        expected_time,
                        scheduled_time,
                        destination_name: direction,
                        until,
                        extra_data: Some(extra_data),
                    });
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
    })
    .await
    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    Ok(departures)
}
