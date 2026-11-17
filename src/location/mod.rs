//! Platform location backends behind one interface.
//!
//! Each backend reports through [`LocationEvents`]; `use_location` starts
//! whichever [`Backend`] the build targets and turns the events into app state.

#[cfg(target_os = "android")]
mod android;
#[cfg(all(feature = "geoclue", not(target_os = "android")))]
mod geoclue;
#[cfg(all(not(feature = "geoclue"), not(target_os = "android")))]
mod mock;

use futures_channel::mpsc::UnboundedSender;

use crate::launch_config::ChannelSend;

#[cfg(target_os = "android")]
pub type Backend = android::AndroidLocation;
#[cfg(all(feature = "geoclue", not(target_os = "android")))]
pub type Backend = geoclue::GeoClueLocation;
#[cfg(all(not(feature = "geoclue"), not(target_os = "android")))]
pub type Backend = mock::MockLocation;

pub trait LocationBackend {
    /// Starts the backend and reports through `events` until the app exits.
    ///
    /// Report whether location is available as soon as it's known, so the app
    /// can tell "disabled" apart from "waiting for a fix".
    async fn run(events: LocationEvents);
}

/// Where backends send their results. Cheap to clone into callbacks and tasks.
#[derive(Clone)]
pub struct LocationEvents {
    state_tx: UnboundedSender<ChannelSend>,
}

impl LocationEvents {
    pub fn new(state_tx: UnboundedSender<ChannelSend>) -> Self {
        Self { state_tx }
    }

    /// Location services were switched on or off, or permission was refused.
    pub fn enabled(&self, enabled: bool) {
        log::info!("Location enabled changed: {enabled}");
        let _ = self
            .state_tx
            .unbounded_send(ChannelSend::LocationEnabledUpdate(enabled));
    }

    /// A new fix, with horizontal accuracy in metres.
    pub fn position(&self, lat: f64, lng: f64, accuracy: f32) {
        log::debug!("Location changed: lat={lat}, lng={lng}, accuracy={accuracy}");
        let _ = self
            .state_tx
            .unbounded_send(ChannelSend::LocationUpdate((lat, lng)));
    }
}
