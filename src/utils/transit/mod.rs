use std::collections::HashMap;

use crate::utils::transit::parsers::{
    routes::RouteType,
    stops::{Stop, StopRadius},
};

pub mod parsers;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TransitData {
    pub stops: HashMap<String, Stop>, // stop_id -> Stop
    pub stops_radius: Vec<StopRadius>,
    // pub routes: Vec<Route>,

    // stop_id -> list of (route_type, route_num) serving this stop
    pub lines_by_stop: HashMap<String, Vec<RouteType>>,

    // (stop_id, route_num, destination) -> scheduled times (secs from midnight)
    pub schedule: HashMap<ScheduleKey, HashMap<u64, Vec<u64>>>,

    pub departures: parsers::departures::Departures,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScheduleKey {
    pub stop_id: String,
    pub route_num: String,
    pub destination: String,
}
