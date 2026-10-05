use std::collections::{HashMap, HashSet};

use crate::utils::transit::parsers::stops::{Stop, StopRadius};

pub mod parsers;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TransitData {
    pub stops: HashMap<String, Stop>, // stop_id -> Stop
    pub stops_radius: Vec<StopRadius>,

    pub departures: parsers::departures::Departures,

    /// Destination names that belong to depot runs. The live feed reports a
    /// destination name but no destination key, so this is how a live-only
    /// departure is recognised as a depot run.
    pub depot_destinations: HashSet<String>,
}
