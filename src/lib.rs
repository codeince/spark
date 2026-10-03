use std::borrow::Cow;

use regex::{Captures, Regex};
pub mod data;

pub fn show_data_as_table(rows: &[Vec<impl ToString>], header: &str) -> String {
    let mut result: Vec<String> = header
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                None
            } else {
                Some(line.to_owned())
            }
        })
        .collect();

    let mut rows: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "|{}",
                row.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
                    .join("|")
            )
        })
        .collect();
    rows.sort();
    result.push(format!("{}\n|-", rows.join("\n")));

    result.join("\n")
}

pub fn parse_macro(input: &str) -> Cow<'_, str> {
    let pattern = Regex::new(
        &[
            r"@(?P<name>\w+)",
            r"\((?P<arguments>[^)]*)\)",
            r"(\[(?P<success>[^\]]*)\])?(",
            r"\{(?P<failure>[^}]*)\})?",
        ]
        .join(r"\s*"),
    )
    .unwrap();

    pattern.replace_all(input, |caps: &Captures| {
        if let Some(name) = caps.name("name")
            && let Some(arguments) = caps.name("arguments")
        {
            let arguments: Vec<&str> = arguments.as_str().split(',').map(str::trim).collect();

            if name.as_str().trim() == "dnd" {
                let arguments: Vec<i64> = arguments.iter().filter_map(|v| v.parse().ok()).collect();

                if arguments.len() == 3 {
                    let value = arguments[0];
                    let minimum_value = arguments[1];
                    let maximum_value = arguments[2];

                    return if rand::random_range(minimum_value..=maximum_value) >= value {
                        caps.name("success")
                            .map(|success| success.as_str().trim())
                            .unwrap_or_default()
                            .to_owned()
                    } else {
                        caps.name("failure")
                            .map(|failure| failure.as_str().trim())
                            .unwrap_or_default()
                            .to_owned()
                    };
                }
            }
        }

        caps.get_match().as_str().to_owned()
    })
}
