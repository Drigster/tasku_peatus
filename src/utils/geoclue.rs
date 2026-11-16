//! Location through GeoClue 1, as shipped on Sailfish OS (session bus).

use smol::stream::{self, Stream, StreamExt};
use zbus::{Connection, OwnedObjectPath, proxy};

/// Accept any provider. Requiring "detailed" (6) limits GeoClue to GPS, which
/// can take minutes for a first fix; with "none" it starts from network
/// positioning and switches to GPS once it has a fix.
const ACCURACY_NONE: i32 = 0;
const MIN_UPDATE_SECONDS: i32 = 2;
const ALL_RESOURCES: i32 = 1023;
const FIELDS_LAT_LON: i32 = 0b011;

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

/// Sets up a GeoClue client and returns its fixes as `(latitude, longitude, accuracy)`.
///
/// Returns as soon as the client is running, so the caller can mark location as
/// available before the first fix. The current position, if known, is yielded first.
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

    let current = match position.get_position().await {
        Ok((fields, _, latitude, longitude, _, accuracy)) => {
            to_fix(fields, latitude, longitude, accuracy)
        }
        Err(e) => {
            log::debug!("No current GeoClue position: {e}");
            None
        }
    };

    let changes = changes.filter_map(|signal| {
        let args = signal.args().ok()?;
        to_fix(args.fields, args.latitude, args.longitude, args.accuracy)
    });

    Ok(stream::iter(current).chain(changes))
}
