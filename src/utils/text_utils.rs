use regex::Regex;
use std::sync::LazyLock;

fn csv_pattern(delimiter: char) -> String {
    format!("(?:\"([^\"]*)\"|([^\"{}]*))(?:{}|$)", delimiter, delimiter)
}

// The transit feeds only ever use these two delimiters, and `parse_csv_line` is
// called once per line of multi-MB files (and once per departure row on every
// refresh), so the pattern must not be recompiled per call.
static SEMICOLON_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(&csv_pattern(';')).unwrap());
static COMMA_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(&csv_pattern(',')).unwrap());

pub fn parse_csv_line(line: &str, delimiter: char) -> Vec<String> {
    match delimiter {
        ';' => split_with(&SEMICOLON_REGEX, line),
        ',' => split_with(&COMMA_REGEX, line),
        _ => split_with(&Regex::new(&csv_pattern(delimiter)).unwrap(), line),
    }
}

fn split_with(regex: &Regex, line: &str) -> Vec<String> {
    regex
        .captures_iter(line)
        .map(|c| c.get(2).unwrap().as_str().to_string())
        .collect()
}
