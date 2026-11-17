# Tasku Peatus

Upcoming public transport departures from the stops around you in Tallinn, built with
[Freya](https://freyaui.dev). Runs on desktop Linux, Android and Sailfish OS.

## How it works

1. **Timetables.** `stops.txt` and `routes.txt` from
   [transport.tallinn.ee](https://transport.tallinn.ee/data/) are downloaded once and
   cached; they are fetched again only when the server's copy is newer.
2. **Location.** A platform backend reports whether location is available and each new
   fix. Stops within 150 m that at least one route serves are shown, nearest first.
3. **Departures.** Each stop shows one row per route, combining the timetable with live
   times from the SIRI departures feed, which is polled about once a minute.

## Platforms

| Platform | Location | How to build |
|---|---|---|
| Desktop | Fixed position in central Tallinn | `cargo run` |
| Android | `LocationManager`, through Kotlin helpers | [`android/`](android/README.md) |
| Sailfish OS | GeoClue 1 | [`sailfish/`](sailfish/README.md) |

The `geoclue` feature also works on desktop Linux, but only where GeoClue 1 runs on the
session bus. Most distributions ship GeoClue 2 instead.

## Development

```sh
cargo run                            # desktop, with the fixed position
RUST_LOG=tasku_peatus=debug cargo run
cargo clippy --all-targets           # add --features geoclue to check that backend too
cargo test
cargo fmt
```

Logging goes through the `log` crate: `env_logger` on desktop and Sailfish (set
`RUST_LOG`), logcat under the `tasku_peatus` tag on Android.

### Features

| Feature | Effect |
|---|---|
| `geoclue` | Location from GeoClue 1 instead of the fixed position |
| `scaled` | Fixed 2.375 scale factor, for phone displays without proper scaling |
| `devtools` | Freya's developer tools |
| `hotreload` | Freya hot reload, used with `dx serve --hot-patch` |

## Layout

| Path | Contents |
|---|---|
| `src/location/` | `LocationBackend` trait and the Android, GeoClue and desktop backends |
| `src/utils/transit/` | Parsers and caching for stops, routes and live departures |
| `src/hooks/` | Hooks that connect location, stops and departures to app state |
| `src/components/`, `src/pages/`, `src/layouts/` | UI |
| `android/` | Gradle project, Kotlin helpers, build and deploy scripts |
| `sailfish/` | Cross-compilation sysroot, build and deploy scripts |
