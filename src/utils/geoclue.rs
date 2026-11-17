//! Location through GeoClue 1, as shipped on Sailfish OS (session bus).

use std::time::Duration;

use smol::stream::{self, Stream, StreamExt};
use zbus::{Connection, OwnedObjectPath, proxy};

/// Written by Sailfish's location settings and the top menu toggle.
const LOCATION_CONFIG: &str = "/etc/location/location.conf";
/// The settings app replaces the file rather than editing it, so it is polled
/// instead of watched.
const ENABLED_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Accept any provider. Requiring "detailed" (6) limits GeoClue to GPS, which
/// can take minutes for a first fix; with "none" it starts from network
/// positioning and switches to GPS once it has a fix.
const ACCURACY_NONE: i32 = 0;
const MIN_UPDATE_SECONDS: i32 = 2;
const ALL_RESOURCES: i32 = 1023;
const FIELDS_LAT_LON: i32 = 0b011;
/// Providers keep their last fix while they look for a new one. Older ones are
/// too likely to be from somewhere else.
const MAX_CACHED_AGE_SECONDS: i64 = 15 * 60;
const PROVIDERS: [&str; 2] = ["Hybris", "Mlsdb"];

#[proxy(
    interface = "org.freedesktop.Geoclue.Master",
    default_service = "org.freedesktop.Geoclue.Master",
    default_path = "/org/freedesktop/Geoclue/Master"
)]
trait Master {
    fn create(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.Geoclue.MasterClient",
    default_service = "org.freedesktop.Geoclue.Master"
)]
trait MasterClient {
    fn set_requirements(
        &self,
        accuracy: i32,
        time: i32,
        require_updates: bool,
        allowed_resources: i32,
    ) -> zbus::Result<()>;
    fn position_start(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.Geoclue.Position",
    default_service = "org.freedesktop.Geoclue.Master"
)]
trait Position {
    /// Returns (fields, timestamp, latitude, longitude, altitude, (level, horizontal, vertical)).
    #[allow(clippy::type_complexity)]
    fn get_position(&self) -> zbus::Result<(i32, i32, f64, f64, f64, (i32, f64, f64))>;

    #[zbus(signal)]
    fn position_changed(
        &self,
        fields: i32,
        timestamp: i32,
        latitude: f64,
        longitude: f64,
        altitude: f64,
        accuracy: (i32, f64, f64),
    ) -> zbus::Result<()>;
}

fn to_fix(
    fields: i32,
    latitude: f64,
    longitude: f64,
    accuracy: (i32, f64, f64),
) -> Option<(f64, f64, f32)> {
    (fields & FIELDS_LAT_LON == FIELDS_LAT_LON).then_some((latitude, longitude, accuracy.1 as f32))
}

/// Asks a position source for its current fix, accepting it only if it's recent.
async fn recent_fix(position: &PositionProxy<'_>, now: i64) -> Option<(f64, f64, f32)> {
    let (fields, timestamp, latitude, longitude, _, accuracy) = position
        .get_position()
        .await
        .inspect_err(|e| log::debug!("No position from {}: {e}", position.inner().destination()))
        .ok()?;
    let source = position.inner().destination();
    let Some(fix) = to_fix(fields, latitude, longitude, accuracy) else {
        log::debug!("No fix from {source}");
        return None;
    };
    let age = now - i64::from(timestamp);
    if age > MAX_CACHED_AGE_SECONDS {
        log::debug!("Ignoring {age}s old position from {source}");
        return None;
    }
    Some(fix)
}

async fn provider_position<'a>(conn: &Connection, name: &str) -> zbus::Result<PositionProxy<'a>> {
    PositionProxy::builder(conn)
        .destination(format!("org.freedesktop.Geoclue.Providers.{name}"))
        .path(format!("/org/freedesktop/Geoclue/Providers/{name}"))
        .build()
        .await
}

/// The most accurate recent fix from the master client or any provider.
///
/// The master only uses a provider once it reports itself available, which for
/// the cell-id provider can take minutes. Providers still answer with their last
/// fix while acquiring, so asking them directly gives a position right away.
async fn current_fix(conn: &Connection, master: &PositionProxy<'_>) -> Option<(f64, f64, f32)> {
    let now = chrono::Utc::now().timestamp();
    let mut best = recent_fix(master, now).await;
    for name in PROVIDERS {
        let Ok(provider) = provider_position(conn, name).await else {
            continue;
        };
        if let Some(fix) = recent_fix(&provider, now).await
            && best.is_none_or(|best| fix.2 < best.2)
        {
            best = Some(fix);
        }
    }
    best
}

/// Sets up a GeoClue client and returns its fixes as `(latitude, longitude, accuracy)`.
///
/// Returns as soon as the client is running, so the caller can mark location as
/// available before the first fix. A recent cached position, if any, is yielded first.
pub async fn start_location_updates()
-> Result<impl Stream<Item = (f64, f64, f32)>, Box<dyn std::error::Error>> {
    let conn = Connection::session().await?;

    let client_path = MasterProxy::new(&conn).await?.create().await?;
    let client = MasterClientProxy::new(&conn, client_path.clone()).await?;
    client
        .set_requirements(ACCURACY_NONE, MIN_UPDATE_SECONDS, true, ALL_RESOURCES)
        .await?;

    let position = PositionProxy::new(&conn, client_path).await?;
    let changes = position.receive_position_changed().await?;
    client.position_start().await?;

    let current = current_fix(&conn, &position).await;

    let changes = changes.filter_map(|signal| {
        let args = signal.args().ok()?;
        to_fix(args.fields, args.latitude, args.longitude, args.accuracy)
    });

    Ok(stream::iter(current).chain(changes))
}

/// Reads `enabled` from the `[location]` section. A missing file or key counts
/// as enabled, so non-Sailfish systems don't show location as disabled.
fn read_location_enabled() -> bool {
    std::fs::read_to_string(LOCATION_CONFIG).map_or(true, |config| parse_location_enabled(&config))
}

fn parse_location_enabled(config: &str) -> bool {
    let mut in_location = false;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_location = line == "[location]";
        } else if in_location && let Some(value) = line.strip_prefix("enabled=") {
            return value == "true";
        }
    }
    true
}

/// Yields whether location is switched on, immediately and then on every change.
///
/// GeoClue gives no dependable signal for this: the master reports no provider
/// at every startup, and while location is off it just stops sending positions.
pub fn location_enabled_changes() -> impl Stream<Item = bool> {
    let mut last = None;
    stream::once(())
        .chain(smol::Timer::interval(ENABLED_POLL_INTERVAL).map(|_| ()))
        .filter_map(move |()| {
            let enabled = read_location_enabled();
            (last != Some(enabled)).then(|| {
                last = Some(enabled);
                enabled
            })
        })
}

#[cfg(test)]
mod tests {
    use super::parse_location_enabled;

    #[test]
    fn reads_enabled_from_location_section() {
        assert!(!parse_location_enabled(
            "[location]\nagreement_accepted=false\nenabled=false\ngps\\enabled=true\n"
        ));
        assert!(parse_location_enabled("[location]\nenabled=true\n"));
        assert!(parse_location_enabled("[other]\nenabled=false\n"));
        assert!(parse_location_enabled(""));
    }
}
