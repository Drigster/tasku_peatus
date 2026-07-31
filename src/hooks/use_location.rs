use freya::{prelude::*, radio::use_radio};

use crate::launch_config::DataChannel;

pub fn use_location() {
    let radio = use_radio(DataChannel::NoUpdate);
    let mut is_location_enabled = radio.slice_mut(DataChannel::LocationEnabledUpdate, |s| {
        &mut s.is_location_enabled
    });
    let mut location = radio.slice_mut(DataChannel::LocationUpdate, |s| &mut s.location);
    #[allow(unused)]
    let state_tx = radio.read().state_tx.clone().unwrap();

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

                *is_location_enabled.write() = true;
                match start_location_updates(move |(lat, lng, accuracy)| {
                    println!("[Print] Location changed: lat={lat}, lng={lng}, accuracy={accuracy}");
                    let _ = state_tx.unbounded_send(ChannelSend::LocationUpdate((lat, lng)));
                })
                .await
                {
                    Ok(callback_ptr) => {
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
    });
}
