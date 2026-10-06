use chrono::{DateTime, Utc};
use geo::{Distance, Haversine, Point};
use revision::{from_slice, revisioned, to_vec};
use serde::Serialize;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::utils::{
    preferences::app_cache_dir, text_utils::parse_csv_line, transit::parsers::routes::Route,
};

static STOPS_URL: &str = "https://transport.tallinn.ee/data/stops.txt";

#[revisioned(revision = 2)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stop {
    pub stop_id: String,
    pub siri_id: String,
    pub lat: f64,
    pub lon: f64,
    pub stops: Vec<String>,
    pub name: String,
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StopRadius {
    pub stop_id: String,
    pub siri_id: String,
    pub distance: u64,
}

pub async fn get_stops() -> Result<HashMap<String, Stop>, Box<dyn std::error::Error>> {
    let current_last_modified = match fs::read_to_string(get_stops_last_modified_file_path()) {
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
    println!("Current stops last modified: {current_last_modified:?}");
    println!("Target stops last modified: {target_last_modified:?}");
    if get_stops_file_path().exists() && current_last_modified >= target_last_modified {
        let stops = blocking::unblock(|| {
            let bytes = match fs::read(get_stops_file_path()) {
                Ok(bytes) => bytes,
                Err(err) => return Err(err.to_string()),
            };

            let data = match from_slice(&bytes) {
                Ok(data) => data,
                Err(err) => return Err(err.to_string()),
            };
            Ok::<HashMap<String, Stop>, String>(data)
        })
        .await;

        match stops {
            Ok(stops) => return Ok(stops),
            Err(err) => println!("Error reading stops: {err}"),
        }
    }
    let stops = blocking::unblock(|| {
        let mut result = ureq::get(STOPS_URL).call().expect("Error getting stops");
        let body = result
            .body_mut()
            .read_to_string()
            .expect("Error reading body");

        parse_stops(body)
    })
    .await;

    fs::write(
        get_stops_last_modified_file_path(),
        Utc::now().timestamp().to_string(),
    )?;

    blocking::unblock(|| {
        let bytes = to_vec(&stops).unwrap();
        fs::write(get_stops_file_path(), &bytes)?;

        Ok(stops)
    })
    .await
    .map_err(|e: ureq::Error| -> Box<dyn std::error::Error> { Box::new(e) })
}

pub fn get_stops_in_radius(
    stops: &HashMap<String, Stop>,
    center_lat: f64,
    center_lon: f64,
    radius_meters: f64,
) -> Vec<StopRadius> {
    let mut stops_radius: Vec<StopRadius> = Vec::new();

    // Loop-invariant, and `meters_to_degrees_lon` costs a cos() per call.
    let lat_delta = meters_to_degrees_lat(radius_meters + 5.0);
    let lon_delta = meters_to_degrees_lon(radius_meters + 5.0, center_lat);
    let center = Point::new(center_lon, center_lat);

    for stop in stops.values() {
        if stop.stop_id.trim().is_empty() {
            continue;
        };

        if (stop.lat - center_lat).abs() <= lat_delta && (stop.lon - center_lon).abs() <= lon_delta
        //&& stop.transports.is_empty() == false
        {
            let distance = Haversine.distance(center, Point::new(stop.lon, stop.lat));

            if distance > radius_meters {
                continue;
            }

            stops_radius.push(StopRadius {
                stop_id: stop.stop_id.clone(),
                siri_id: stop.siri_id.clone(),
                distance: distance as u64,
            });
        }
    }

    stops_radius.sort_by(|a, b| a.distance.cmp(&b.distance));

    stops_radius
}

pub fn parse_stops(data: String) -> HashMap<String, Stop> {
    let mut lines = data.lines();

    let header: Vec<&str> = lines
        .next()
        .unwrap()
        .trim_start_matches('\u{feff}')
        .split(';')
        .collect();

    let id_index = header.iter().position(|x| *x == "ID").unwrap();
    let siri_index = header.iter().position(|x| *x == "SiriID").unwrap();
    let lat_index = header.iter().position(|x| *x == "Lat").unwrap();
    let lon_index = header.iter().position(|x| *x == "Lng").unwrap();
    let name_index = header.iter().position(|x| *x == "Name").unwrap();

    let header_len = header.len();
    let mut previous_parts = vec![String::new(); header_len];

    let mut stops = HashMap::new();
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

        for (j, part) in parts.iter_mut().enumerate() {
            if part.trim().is_empty() {
                *part = previous_parts[j].clone();
            } else {
                previous_parts[j] = part.clone();
            }
        }

        let siri_id = parts[siri_index].clone();
        let stop_id = parts[id_index].clone();
        let lat = match parts[lat_index].parse::<u64>() {
            Ok(lat) => lat as f64 / 100_000.0,
            Err(_) => {
                println!(
                    "[Print] Error parsing: {} lat: {}",
                    siri_id, parts[lat_index]
                );
                continue;
            }
        };
        let lon = match parts[lon_index].parse::<u64>() {
            Ok(lon) => lon as f64 / 100_000.0,
            Err(_) => {
                println!(
                    "[Print] Error parsing: {} lon: {}",
                    siri_id, parts[lon_index]
                );
                continue;
            }
        };
        let name = parts[name_index].clone();

        stops.insert(
            stop_id.clone(),
            Stop {
                stop_id,
                siri_id,
                lat,
                lon,
                stops: vec![],
                name,
                routes: vec![],
            },
        );
    }

    stops
}

pub fn get_last_modified_version() -> DateTime<Utc> {
    let response = ureq::head(STOPS_URL).call();
    match response {
        Ok(response) => match response.headers().get("Last-Modified") {
            Some(last_modified) => DateTime::parse_from_rfc2822(last_modified.to_str().unwrap())
                .unwrap()
                .into(),
            None => DateTime::<Utc>::MAX_UTC,
        },
        Err(e) => {
            log::error!("Error getting last modified version: {e}");
            DateTime::<Utc>::MAX_UTC
        }
    }
}

pub fn get_stops_file_path() -> PathBuf {
    app_cache_dir().join("stops.dat")
}

pub fn get_stops_last_modified_file_path() -> PathBuf {
    app_cache_dir().join("stops_last_modified.dat")
}

fn meters_to_degrees_lat(meters: f64) -> f64 {
    meters / 111_320.0
}

/// Convert meters to degrees of longitude at a given latitude
fn meters_to_degrees_lon(meters: f64, latitude_deg: f64) -> f64 {
    let lat_rad = latitude_deg.to_radians();
    meters / (111_320.0 * lat_rad.cos())
}
