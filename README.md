# Tasku Peatus

Shows upcoming public transport departures from the stops around you in Tallinn, built
with [Freya](https://freyaui.dev). Timetables come from
[transport.tallinn.ee](https://transport.tallinn.ee), with live departure times layered
on top.

## Platforms

| Platform | Location source | Build |
|---|---|---|
| Desktop | Fixed position in central Tallinn | `cargo run` |
| Desktop with GeoClue 1 | GeoClue 1 on the session bus | `cargo run --features geoclue` |
| Android | `LocationManager` | [`android/`](android/README.md) |
| Sailfish OS | GeoClue 1 | [`sailfish/`](sailfish/README.md) |

Logs go through the `log` crate; on desktop, set `RUST_LOG=tasku_peatus=debug` to see them.

## Features

| Feature | Effect |
|---|---|
| `geoclue` | Location from GeoClue 1 instead of the fixed desktop position |
| `scaled` | Fixed 2.375 scale factor, for running on a phone display |
| `devtools` | Freya developer tools |
| `hotreload` | Freya hot reload |
