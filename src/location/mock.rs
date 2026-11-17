use super::{LocationBackend, LocationEvents};

/// A fixed position in central Tallinn, for desktop builds without `geoclue`.
pub struct MockLocation;

impl LocationBackend for MockLocation {
    async fn run(events: LocationEvents) {
        events.enabled(true);
        events.position(59.436552, 24.753048, 0.0);
    }
}
