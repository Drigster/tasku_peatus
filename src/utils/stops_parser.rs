use chrono::{DateTime, Utc};
use geo::{Distance, Haversine, Point};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::PathBuf,
};

use crate::utils::{preferences::get_cache_dir, text_utils::parse_csv_line};

static STOPS_URL: &str = "https://transport.tallinn.ee/data/stops.txt";

pub async fn get_stops() -> Result<Vec<Stop>, Box<dyn std::error::Error>> {
    let last_modified = DateTime::<Utc>::MAX_UTC;
    if last_modified < get_last_modified_version() || !get_stops_file_path().exists() {
        blocking::unblock(|| {
            let mut result = ureq::get(STOPS_URL).call()?;
            let body = result.body_mut().read_to_string()?;

            fs::write(get_stops_file_path(), &body)?;

            Ok(parse_stops(body))
        })
        .await
        .map_err(|e: ureq::Error| -> Box<dyn std::error::Error> { Box::new(e) })
    } else {
        let stops = blocking::unblock(|| {
            let data = fs::read_to_string(get_stops_file_path())?;
            Ok::<Vec<Stop>, io::Error>(parse_stops(data))
        })
        .await?;

        Ok(stops)
    }
}

pub fn get_stops_in_radius(
    stops: Vec<Stop>,
    center_lat: f64,
    center_lon: f64,
    radius_meters: f64,
) -> (HashMap<String, Stop>, HashMap<String, u64>) {
    let mut stops_distances: HashMap<String, u64> = HashMap::new();
    let mut stops_radius: HashMap<String, Stop> = HashMap::new();

    for stop in stops.into_iter() {
        let lat_delta = meters_to_degrees_lat(radius_meters + 5.0);
        let lon_delta = meters_to_degrees_lon(radius_meters + 5.0, center_lat);

        if (stop.lat - center_lat).abs() <= lat_delta && (stop.lon - center_lon).abs() <= lon_delta
        //&& stop.transports.is_empty() == false
        {
            let distance = Haversine.distance(
                Point::new(center_lon, center_lat),
                Point::new(stop.lon, stop.lat),
            );

            if distance > radius_meters {
                continue;
            }

            stops_distances.insert(stop.siri_id.clone(), distance as u64);
            stops_radius.insert(stop.siri_id.clone(), stop);
        }
    }

    (stops_radius, stops_distances)
}

pub fn parse_stops(data: String) -> Vec<Stop> {
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

    let mut stops = Vec::new();
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
        let is_favorite = false;
        let transports = HashMap::new();

        stops.push(Stop {
            stop_id,
            siri_id,
            lat,
            lon,
            stops: vec![],
            name,
            is_favorite,
            transports,
        });
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
            None => DateTime::<Utc>::MIN_UTC,
        },
        Err(e) => {
            log::error!("Error getting last modified version: {e}");
            DateTime::<Utc>::MIN_UTC
        }
    }
}

pub fn get_stops_file_path() -> PathBuf {
    let cache_dir = get_cache_dir().unwrap();
    cache_dir.join("stops.txt")
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Stop {
    #[serde(rename = "ID")]
    pub stop_id: String,
    #[serde(rename = "SiriID")]
    pub siri_id: String,
    #[serde(rename = "Lat")]
    pub lat: f64,
    #[serde(rename = "Lng")]
    pub lon: f64,
    #[serde(rename = "Stops")]
    pub stops: Vec<String>,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(skip)]
    pub is_favorite: bool,
    #[serde(skip)]
    pub transports: HashMap<String, HashSet<String>>,
}

fn meters_to_degrees_lat(meters: f64) -> f64 {
    meters / 111_320.0
}

/// Convert meters to degrees of longitude at a given latitude
fn meters_to_degrees_lon(meters: f64, latitude_deg: f64) -> f64 {
    let lat_rad = latitude_deg.to_radians();
    meters / (111_320.0 * lat_rad.cos())
}
