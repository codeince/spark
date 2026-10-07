pub mod data;
use crate::data::{Block, Blocks, FindById};
use regex::Captures;
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
    regex::regex!(r"@\s*(?P<name>\w+)\s*\((?P<arguments>[^)]*)\)\s*(\[(?P<success>[^\]]*)\])?(\s*\{(?P<failure>[^}]*)\})?")
        .replace_all(input, |caps: &Captures| {
        let success_text = caps
            .name("success")
            .map(|success| success.as_str().trim())
            .unwrap_or_default()
            .to_owned();
        let failure_text = caps
            .name("failure")
            .map(|failure| failure.as_str().trim())
            .unwrap_or_default()
            .to_owned();

        if let Some(name) = caps.name("name").map(|name| name.as_str().trim())
            && let Some(arguments) = caps
                .name("arguments")
                .map(|arguments| arguments.as_str().trim())
        {
            let arguments: Vec<&str> = arguments.split(',').map(str::trim).collect();

            let macro_result = match name {
                "dnd" => Some({
                    let arguments: Vec<i64> =
                        arguments.iter().filter_map(|v| v.parse().ok()).collect();
                        
                    if arguments.len() == 3 {
                        let value = arguments[0];
                        let minimum_value = arguments[1];
                        let maximum_value = arguments[2];

                        rand::random_range(minimum_value..=maximum_value) >= value
                    } else {
                        false
                    }
                }),
                _ => None,
            };

            if let Some(is_success) = macro_result {
                return if is_success {
                    success_text
                } else {
                    failure_text
                };
            }
        }

        caps.get_match().as_str().to_owned()
    })
}
