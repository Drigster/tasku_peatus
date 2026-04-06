use freya::prelude::{Bytes, Color};
use regex::Regex;

pub fn parse_csv_line(line: &str, delimiter: char) -> Vec<String> {
    let regex =
        Regex::new(format!("(?:\"([^\"]*)\"|([^\"{}]*))(?:{}|$)", delimiter, delimiter).as_str())
            .unwrap();
    regex
        .captures_iter(line)
        .map(|c| c.get(2).unwrap().as_str().to_string())
        .collect()
}

pub fn get_transport_icon_and_color(transport_type: &str) -> (Bytes, Color) {
    match transport_type {
        // Copied from https://transport.tallinn.ee CSS
        "metro" => (
            Bytes::from_static(include_bytes!("../assets/MDI/subway-variant.svg")),
            Color::from_hex("0xff6A00").unwrap(),
        ),
        "bus" | "nightbus" => (
            Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
            Color::from_hex("0x00e1b4").unwrap(),
        ),
        "trol" => (
            Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
            Color::from_hex("0x0064d7").unwrap(),
        ),
        "tram" => (
            Bytes::from_static(include_bytes!("../assets/MDI/tram.svg")),
            Color::from_hex("0xff601e").unwrap(),
        ),
        "regionalbus" => (
            Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
            Color::from_hex("0x9c1630").unwrap(),
        ),
        "suburbanbus" => (
            Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
            Color::from_hex("0x004a7f").unwrap(),
        ),
        "commercialbus" | "intercitybus" | "internationalbus" | "seasonalbus" => (
            Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
            Color::from_hex("0x800080").unwrap(),
        ),
        "expressbus" | "minibus" => (
            Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
            Color::from_hex("0x008000").unwrap(),
        ),
        "train" => (
            Bytes::from_static(include_bytes!("../assets/MDI/train.svg")),
            Color::from_hex("0x009900").unwrap(),
        ),
        "plane" => (
            Bytes::from_static(include_bytes!("../assets/MDI/airplane.svg")),
            Color::from_hex("0x404040").unwrap(),
        ),
        "festal" => (
            Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
            Color::from_hex("0xffa500").unwrap(),
        ),
        "eventbus" => (
            Bytes::from_static(include_bytes!("../assets/MDI/bus.svg")),
            Color::from_hex("0xff6a00").unwrap(),
        ),
        "ferry" | "aquabus" => (
            Bytes::from_static(include_bytes!("../assets/MDI/ferry.svg")),
            Color::from_hex("0x0064d7").unwrap(),
        ),
        _ => (
            Bytes::from_static(include_bytes!("../assets/MDI/help.svg")),
            Color::from_hex("0x000000").unwrap(),
        ),
    }
}
