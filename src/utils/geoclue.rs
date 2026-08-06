use smol::stream::StreamExt;
use zbus::{Connection, proxy, zvariant};

const MASTER_SERVICE: &str = "org.freedesktop.Geoclue.Master";

#[proxy(
    interface = "org.freedesktop.Geoclue.MasterClient",
    default_service = "org.freedesktop.Geoclue.Master"
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
}

#[proxy(
    interface = "org.freedesktop.Geoclue.Position",
    default_service = "org.freedesktop.Geoclue.Master"
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

pub async fn get_location() -> Result<(f64, f64), Box<dyn std::error::Error>> {
    println!("Get location");
    // Session bus, run as defaultuser
    let conn = Connection::session().await?;
    println!("Connection: {:?}", conn);

    // Activate the master (this creates/keeps client0 alive)
    let client_body = conn
        .call_method(
            Some(MASTER_SERVICE),
            "/org/freedesktop/Geoclue/Master",
            Some("org.freedesktop.Geoclue.Master"),
            "Create",
            &(),
        )
        .await?
        .body();

    let client_path: zvariant::OwnedObjectPath = client_body.deserialize()?;

    let client = MasterClientProxy::new(&conn, client_path.clone()).await?;
    println!("Client: {:?}", client);
    let requirments = client.set_requirements(6, 0, true, 1023).await;

    println!("Requirements: {requirments:?}");

    let pos_start = client.position_start().await;

    println!("Position start: {pos_start:?}");

    let pos = PositionProxy::new(&conn, client_path).await?;
    let position = pos.get_position().await?;

    println!("Position: {:?}", position);

    Ok((0.0, 0.0))
}

pub async fn start_location_updates<F>(callback: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn((f64, f64, f32)) + Send + Sync + 'static,
{
    // Session bus, run as defaultuser
    let conn = Connection::session().await?;

    // Activate the master (this creates/keeps client0 alive)
    let client_body = conn
        .call_method(
            Some(MASTER_SERVICE),
            "/org/freedesktop/Geoclue/Master",
            Some("org.freedesktop.Geoclue.Master"),
            "Create",
            &(),
        )
        .await?
        .body();

    let client_path: zvariant::OwnedObjectPath = client_body.deserialize()?;

    let client = MasterClientProxy::new(&conn, client_path.clone()).await?;
    println!("Client: {:?}", client);

    let requirments = client.set_requirements(6, 2, true, 1023).await;
    println!("Requirements: {requirments:?}");

    let pos = PositionProxy::new(&conn, client_path).await?;

    let pos_start = client.position_start().await;
    println!("Position start: {pos_start:?}");

    let mut updates = pos.receive_position_changed().await?;
    while let Some(signal) = updates.next().await {
        let args = signal.args()?;
        if args.fields & 0b011 == 0b011 {
            callback((args.latitude, args.longitude, args.accuracy.1 as f32));
        }
    }
    Ok(())
}
