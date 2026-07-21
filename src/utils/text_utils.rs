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
