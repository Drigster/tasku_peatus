use freya::{prelude::*, radio::Radio};

use crate::{
    launch_config::{AppState, Data, DataChannel},
    location::{Backend, LocationBackend, LocationEvents},
};

pub fn use_location(radio: &Radio<Data, DataChannel>) {
    let is_location_enabled = radio.slice(DataChannel::LocationEnabledUpdate, |s| {
        &s.is_location_enabled
    });
    let location = radio.slice(DataChannel::LocationUpdate, |s| &s.location);
    let state_tx = radio.read().state_tx.clone().unwrap();
    let mut state = radio.slice_mut(DataChannel::StateUpdate, |s| &mut s.state);

    let mut is_loading = use_state(|| true);

    use_side_effect(move || {
        if !is_loading() {
            // Guarded write: this effect reruns on every location update, and an
            // unconditional write dirties every `StateUpdate` subscriber.
            let new_state = if !*is_location_enabled.read() {
                Some(AppState::LocationDisabled)
            } else if location.read().is_none() {
                Some(AppState::WitingForLocation)
            } else {
                None
            };

            state.set_if_modified(new_state);
        }
    });

    use_hook(move || {
        spawn(Backend::run(LocationEvents::new(state_tx)));
        is_loading.set(false);
    });
}
