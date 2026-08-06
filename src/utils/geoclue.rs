use smol::stream::StreamExt;
use zbus::{Connection, proxy, zvariant};

const MASTER_SERVICE: &str = "org.freedesktop.GeoClue.Master";

#[proxy(
    interface = "org.freedesktop.Geoclue.Master",
    default_service = "org.freedesktop.Geoclue.Master",
    default_path = "/org/freedesktop/Geoclue/Master"
)]
trait Master {
    fn create(&self) -> Result<zbus::zvariant::OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.GeoClue.MasterClient",
    default_service = "org.freedesktop.GeoClue.Master"
)]
trait MasterClient {
    fn set_requirements(
        &self,
        accuracy: i32, // 6 = detailed
        time: i32,
        require_updates: bool,
        allowed_resources: i32, // 1023 = all
    ) -> zbus::Result<()>;
    fn position_start(&self) -> zbus::Result<()>;

    fn get_position_provider(&self) -> zbus::Result<(String, String, String, String)>;

    #[zbus(signal)]
    fn position_provider_changed(
        &self,
        name: String,
        description: String,
        service: String,
        path: String,
    ) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.GeoClue.Position",
    default_service = "org.freedesktop.GeoClue.Master"
)]
trait Position {
    fn get_position(
        &self,
    ) -> zbus::Result<(
        i32,             // fields bitmask: 1=lat, 2=lon, 4=alt
        i32,             // timestamp (epoch)
        f64,             // latitude
        f64,             // longitude
        f64,             // altitude
        (i32, f64, f64), // accuracy: (level, horizontal, vertical)
    )>;

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

#[derive(Debug, Clone)]
pub enum LocationEvent {
    Enabled,
    Disabled,
    Position {
        latitude: f64,
        longitude: f64,
        altitude: f64,
        horizontal_accuracy: f64,
    },
}

pub async fn start_location_watcher<F>(mut on_event: F) -> Result<()>
where
    F: FnMut(LocationEvent),
{
    let conn = Connection::session().await?;

    let master = MasterProxy::new(&conn).await?;
    let client_path = master.create().await?;

    let client = MasterClientProxy::builder(&conn)
        .path(client_path.clone())?
        .build()
        .await?;

    client.set_requirements(4, 0, true, 1023).await?;

    // Initial on/off state
    let mut enabled = match client.get_position_provider().await {
        Ok((_, _, service, _)) => !service.is_empty(),
        Err(_) => false,
    };
    on_event(if enabled {
        LocationEvent::Enabled
    } else {
        LocationEvent::Disabled
    });

    // Position signals arrive on the same client object
    let position = PositionProxy::builder(&conn)
        .path(client_path)?
        .build()
        .await?;

    let provider_stream = client
        .receive_position_provider_changed()
        .await
        .into_iter()
        .filter_map(|s| async move { s.args().ok().map(|a| (!a.service.is_empty()).into()) });

    let position_stream = position
        .receive_position_changed()
        .await
        .into_iter()
        .filter_map(|s| async move {
            let a = s.args().ok()?;
            let fields = *a.fields;
            // bits 1 (latitude) and 2 (longitude) must be set for a valid fix
            if fields & 0b011 != 0 {
                Some(LocationEvent::Position {
                    latitude: *a.latitude,
                    longitude: *a.longitude,
                    altitude: *a.altitude,
                    horizontal_accuracy: a.accuracy.1,
                })
            } else {
                None
            }
        });

    let mut events = futures_util::stream::select(provider_stream, position_stream);

    while let Some(event) = events.next().await {
        match event {
            LocationEvent::Enabled => {
                if !enabled {
                    enabled = true;
                    on_event(LocationEvent::Enabled);
                }
            }
            LocationEvent::Disabled => {
                if enabled {
                    enabled = false;
                    on_event(LocationEvent::Disabled);
                }
            }
            pos @ LocationEvent::Position { .. } => {
                on_event(pos);
            }
        }
    }

    Ok(())
}
