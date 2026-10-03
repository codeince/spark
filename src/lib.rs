pub mod data;
use crate::data::{Block, Blocks, FindById};
use regex::{Captures, Regex};
use std::{borrow::Cow, path::Path};

pub fn save_story(blocks: &Blocks, save_path: &Path) -> color_eyre::Result<()> {
    if !blocks.is_empty() {
        let content = if save_path.ends_with(".json") {
            let data = serde_json::to_string(blocks)?.to_owned();
            data.as_bytes().into()
        } else {
            rkyv::to_bytes::<rkyv::rancor::Error>(blocks)?.to_vec()
        };
        std::fs::write(save_path, content)?;
    }

    Ok(())
}

pub fn get_branches_as_table(block: &Block, blocks: &Blocks, header: &str) -> String {
    show_data_as_table(
        &block
            .branches
            .iter()
            .filter_map(|branch| {
                blocks
                    .find_by_id(*branch)
                    .map(|branch| branch.content.to_string())
            })
            .map(|value| vec![value])
            .collect::<Vec<Vec<String>>>(),
        header,
    )
}

pub fn show_data_as_table(rows: &[Vec<impl ToString>], header: &str) -> String {
    let mut result: Vec<String> = header
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            (!line.is_empty()).then_some(line.to_owned())
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
