use freya::{
    prelude::{Interactive::No, *},
    radio::Radio,
};

use crate::launch_config::{AppState, Data, DataChannel};

pub fn use_location(radio: &Radio<Data, DataChannel>) {
    let mut is_location_enabled = radio.slice_mut(DataChannel::LocationEnabledUpdate, |s| {
        &mut s.is_location_enabled
    });
    let mut location = radio.slice_mut(DataChannel::LocationUpdate, |s| &mut s.location);
    #[allow(unused)]
    let state_tx = radio.read().state_tx.clone().unwrap();
    let mut state = radio.slice_mut(DataChannel::StateUpdate, |s| &mut s.state);

    let mut is_loading = use_state(|| true);

    let is_location_enabled_clone = is_location_enabled.clone();
    let location_clone = location.clone();
    use_side_effect(move || {
        if is_loading() == false {
            if *is_location_enabled_clone.read() == false {
                *state.write() = Some(AppState::LocationDisabled);
            } else if location_clone.read().is_none() {
                *state.write() = Some(AppState::WitingForLocation);
            } else {
                *state.write() = None;
            }
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
                let _ = state_tx_clone.unbounded_send(ChannelSend::LocationEnabledUpdate(enabled));
            }) {
                Ok(callback_ptr) => {
                    println!("[Print] Location enabled watcher started: {callback_ptr}");
                }
                Err(e) => {
                    println!("[Print] Failed to start location enabled watcher: {e}");
                }
            }
        });
    }

    use_hook(|| {
        spawn(async move {
            #[cfg(target_os = "android")]
            {
                use crate::launch_config::ChannelSend;
                use crate::utils::jni_utils::{
                    check_and_request_permissions, get_last_known_location, start_location_updates,
                };

                match check_and_request_permissions().await {
                    Ok(true) => {
                        match get_last_known_location() {
                            Ok(last_known_location) => {
                                println!("[Print] Last Known Location: {:?}", last_known_location);
                                *location.write() =
                                    Some((last_known_location.0, last_known_location.1));
                            }
                            Err(e) => {
                                println!("[Print] Error getting location: {e}");
                            }
                        }
                        match start_location_updates(move |(lat, lng, accuracy)| {
                            println!(
                                "[Print] Location changed: lat={lat}, lng={lng}, accuracy={accuracy}"
                            );
                            let _ =
                                state_tx.unbounded_send(ChannelSend::LocationUpdate((lat, lng)));
                        }) {
                            Ok(callback_ptr) => {
                                println!("[Print] Location updates started: {callback_ptr}");
                            }
                            Err(e) => {
                                println!("[Print] Error starting location updates: {e}");
                            }
                        }
                    }
                    Ok(false) => {
                        println!("[Print] Permissions: false");
                        *is_location_enabled.write() = false;
                    }
                    Err(e) => {
                        println!("[Print] Error checking permissions: {e}");
                    }
                }
            }
            #[cfg(feature = "geoclue")]
            {
                use crate::launch_config::ChannelSend;
                use crate::utils::geoclue::start_location_updates;

                match start_location_updates(move |(lat, lng, accuracy)| {
                    println!("[Print] Location changed: lat={lat}, lng={lng}, accuracy={accuracy}");
                    let _ = state_tx.unbounded_send(ChannelSend::LocationUpdate((lat, lng)));
                })
                .await
                {
                    Ok(callback_ptr) => {
                        *is_location_enabled.write() = true;
                        println!("[Print] Location updates started: {callback_ptr:?}");
                    }
                    Err(e) => {
                        println!("[Print] Error starting location updates: {e}");
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
