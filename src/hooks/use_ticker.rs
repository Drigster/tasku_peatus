use std::time::Duration;

use chrono::Utc;
use freya::{prelude::*, radio::Radio};

use crate::launch_config::{Data, DataChannel};

/// Single app-wide one-second clock.
///
/// Every departure row used to own its own `smol::Timer` and decrement a local
/// counter, so N visible rows meant N timers firing at N unsynchronized phases,
/// each dirtying its scope in a separate frame and forcing its own layout and
/// paint pass. Publishing one absolute timestamp instead keeps all countdowns in
/// a single frame per second, and because consumers derive their remaining time
/// from the wall clock rather than accumulating decrements, the display cannot
/// drift if a tick is late or dropped.
pub fn use_ticker(radio: &Radio<Data, DataChannel>) {
    let mut now = radio.slice_mut(DataChannel::TickUpdate, |s| &mut s.now);

    use_hook(|| {
        now.set_if_modified(Utc::now().timestamp());

        spawn(async move {
            loop {
                smol::Timer::after(Duration::from_secs(1)).await;
                now.set_if_modified(Utc::now().timestamp());
            }
        });
    });
}
