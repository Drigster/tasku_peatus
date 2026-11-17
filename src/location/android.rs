use super::{LocationBackend, LocationEvents};
use crate::utils::jni_utils::{
    check_and_request_permissions, start_location_enabled_updates, start_location_updates,
};

/// Android's `LocationManager`, through the Kotlin helpers in `android/app`.
pub struct AndroidLocation;

impl LocationBackend for AndroidLocation {
    async fn run(events: LocationEvents) {
        let watcher = events.clone();
        if let Err(e) = start_location_enabled_updates(move |enabled| watcher.enabled(enabled)) {
            log::error!("Failed to start location enabled watcher: {e}");
        }

        match check_and_request_permissions().await {
            Ok(true) => {
                let updates = events.clone();
                match start_location_updates(move |(lat, lng, accuracy)| {
                    updates.position(lat, lng, accuracy)
                }) {
                    Ok(_) => log::info!("Location updates started"),
                    Err(e) => log::error!("Error starting location updates: {e}"),
                }
            }
            Ok(false) => {
                log::warn!("Location permission denied");
                events.enabled(false);
            }
            Err(e) => log::error!("Error checking permissions: {e}"),
        }
    }
}
