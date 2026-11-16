use freya::{prelude::*, radio::Radio};

use crate::launch_config::{AppState, Data, DataChannel};

pub fn use_location(radio: &Radio<Data, DataChannel>) {
    #[cfg_attr(feature = "geoclue", allow(unused_mut))]
    let mut is_location_enabled = radio.slice_mut(DataChannel::LocationEnabledUpdate, |s| {
        &mut s.is_location_enabled
    });
    #[cfg_attr(any(feature = "geoclue", target_os = "android"), allow(unused_mut))]
    let mut location = radio.slice_mut(DataChannel::LocationUpdate, |s| &mut s.location);
    #[allow(unused)]
    let state_tx = radio.read().state_tx.clone().unwrap();
    let mut state = radio.slice_mut(DataChannel::StateUpdate, |s| &mut s.state);

    let mut is_loading = use_state(|| true);

    let is_location_enabled_clone = is_location_enabled.clone();
    let location_clone = location.clone();
    use_side_effect(move || {
        if !is_loading() {
            // Guarded write: this effect reruns on every location update, and an
            // unconditional write dirties every `StateUpdate` subscriber.
            let new_state = if !*is_location_enabled_clone.read() {
                Some(AppState::LocationDisabled)
            } else if location_clone.read().is_none() {
                Some(AppState::WitingForLocation)
            } else {
                None
            };

            state.set_if_modified(new_state);
        }
    });

    #[cfg(target_os = "android")]
    let state_tx_clone = state_tx.clone();
    #[cfg(target_os = "android")]
    {
        use_hook(move || {
            use crate::launch_config::ChannelSend;
            use crate::utils::jni_utils::start_location_enabled_updates;

            match start_location_enabled_updates(move |enabled| {
                log::info!("Location enabled changed: {enabled}");
                let _ = state_tx_clone.unbounded_send(ChannelSend::LocationEnabledUpdate(enabled));
            }) {
                Ok(callback_ptr) => {
                    log::info!("Location enabled watcher started: {callback_ptr}");
                }
                Err(e) => {
                    log::error!("Failed to start location enabled watcher: {e}");
                }
            }
        });
    }

    #[cfg(feature = "geoclue")]
    {
        let state_tx = state_tx.clone();
        use_hook(move || {
            spawn(async move {
                use crate::launch_config::ChannelSend;
                use crate::utils::geoclue::location_enabled_changes;
                use smol::stream::StreamExt;

                let mut changes = std::pin::pin!(location_enabled_changes());
                while let Some(enabled) = changes.next().await {
                    log::info!("Location enabled changed: {enabled}");
                    let _ = state_tx.unbounded_send(ChannelSend::LocationEnabledUpdate(enabled));
                }
            });
        });
    }

    use_hook(|| {
        spawn(async move {
            #[cfg(target_os = "android")]
            {
                use crate::launch_config::ChannelSend;
                use crate::utils::jni_utils::{
                    check_and_request_permissions, start_location_updates,
                };

                match check_and_request_permissions().await {
                    Ok(true) => {
                        match start_location_updates(move |(lat, lng, accuracy)| {
                            log::debug!(
                                "Location changed: lat={lat}, lng={lng}, accuracy={accuracy}"
                            );
                            let _ =
                                state_tx.unbounded_send(ChannelSend::LocationUpdate((lat, lng)));
                        }) {
                            Ok(callback_ptr) => {
                                log::info!("Location updates started: {callback_ptr}");
                            }
                            Err(e) => {
                                log::error!("Error starting location updates: {e}");
                            }
                        }
                    }
                    Ok(false) => {
                        log::warn!("Permissions: false");
                        *is_location_enabled.write() = false;
                    }
                    Err(e) => {
                        log::error!("Error checking permissions: {e}");
                    }
                }
            }
            #[cfg(feature = "geoclue")]
            {
                use crate::launch_config::ChannelSend;
                use crate::utils::geoclue::start_location_updates;
                use smol::stream::StreamExt;

                match start_location_updates().await {
                    Ok(updates) => {
                        log::info!("Location updates started");

                        let mut updates = std::pin::pin!(updates);
                        while let Some((lat, lng, accuracy)) = updates.next().await {
                            log::debug!(
                                "Location changed: lat={lat}, lng={lng}, accuracy={accuracy}"
                            );
                            let _ =
                                state_tx.unbounded_send(ChannelSend::LocationUpdate((lat, lng)));
                        }
                        log::warn!("GeoClue position stream ended");
                    }
                    Err(e) => {
                        log::error!("Error starting location updates: {e}");
                    }
                }
            }
            #[cfg(all(not(feature = "geoclue"), not(target_os = "android")))]
            {
                *is_location_enabled.write() = true;
                *location.write() = Some((59.436552, 24.753048));
            }
        });
        *is_loading.write() = false;
    });
}
