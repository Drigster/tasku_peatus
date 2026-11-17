use smol::stream::StreamExt;

use super::{LocationBackend, LocationEvents};
use crate::utils::geoclue::{location_enabled_changes, start_location_updates};

/// GeoClue 1, as shipped on Sailfish OS.
pub struct GeoClueLocation;

impl LocationBackend for GeoClueLocation {
    async fn run(events: LocationEvents) {
        smol::future::zip(watch_enabled(events.clone()), watch_positions(events)).await;
    }
}

async fn watch_enabled(events: LocationEvents) {
    let mut changes = std::pin::pin!(location_enabled_changes());
    while let Some(enabled) = changes.next().await {
        events.enabled(enabled);
    }
}

async fn watch_positions(events: LocationEvents) {
    let updates = match start_location_updates().await {
        Ok(updates) => updates,
        Err(e) => {
            log::error!("Error starting location updates: {e}");
            return;
        }
    };
    log::info!("Location updates started");

    let mut updates = std::pin::pin!(updates);
    while let Some((lat, lng, accuracy)) = updates.next().await {
        events.position(lat, lng, accuracy);
    }
    log::warn!("GeoClue position stream ended");
}
