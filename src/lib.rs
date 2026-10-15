pub mod data;
use crate::data::{Block, Blocks, FindById};
use regex::Captures;
use std::{borrow::Cow, path::Path};

pub fn save_story(blocks: &Blocks, output_path: &Path) -> color_eyre::Result<()> {
    if !blocks.is_empty() {
        let content = if output_path.ends_with(".json") {
            let data = serde_json::to_string(blocks)?.to_owned();
            data.as_bytes().into()
        } else {
            rkyv::to_bytes::<rkyv::rancor::Error>(blocks)?.to_vec()
        };
        std::fs::write(output_path, content)?;
    }

    Ok(())
}

pub fn get_branches_as_table(block: &Block, blocks: &[Block], header: &str) -> String {
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

fn get_with_fallback(caps: &Captures, fallback: &str) -> Option<String> {
    caps.name(fallback)
        .or(caps.name(&format!("{fallback}2")))
        .map(|v| v.as_str().to_owned())
}

fn replace_macros(caps: &Captures) -> String {
    let mut success_text = get_with_fallback(caps, "success")
        .map(|v| v.trim().to_owned())
        .unwrap_or_default();
    let failure_text = get_with_fallback(caps, "failure")
        .map(|v| v.trim().to_owned())
        .unwrap_or_default();

    if let Some(name) = caps.name("name").map(|name| name.as_str().trim())
        && let Some(arguments) = caps
            .name("arguments")
            .map(|arguments| arguments.as_str().trim())
    {
        let arguments: Vec<&str> = arguments.split(',').map(str::trim).collect();

        let macro_result = match name {
            "dnd" => Some({
                let arguments: Vec<i64> = arguments
                    .iter()
                    .filter_map(|v| v.trim().parse().ok())
                    .collect();

                if arguments.len() == 3 {
                    let value = arguments[0];
                    let minimum_value = arguments[1];
                    let maximum_value = arguments[2];

                    rand::random_range(minimum_value..=maximum_value) >= value
                } else {
                    false
                }
            }),
            "randi" => Some({
                let arguments: Vec<i64> = arguments
                    .iter()
                    .filter_map(|v| v.trim().parse().ok())
                    .collect();

                if arguments.len() == 2 {
                    let minimum_value = arguments[0];
                    let maximum_value = arguments[1];
                    let value = rand::random_range(minimum_value..=maximum_value);
                    success_text = success_text.replace("{num}", &value.to_string());

                    true
                } else {
                    false
                }
            }),
            "randf" => Some({
                let arguments: Vec<f64> = arguments
                    .iter()
                    .filter_map(|v| v.trim().parse().ok())
                    .collect();

                if arguments.len() == 2 {
                    let minimum_value = arguments[0];
                    let maximum_value = arguments[1];
                    let value = rand::random_range(minimum_value..=maximum_value);
                    success_text = success_text.replace("{num}", &value.to_string());

                    true
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
}

pub fn parse_macro(input: &str) -> Cow<'_, str> {
    regex::regex!(r"@(?P<name>[^()]*)\((?P<arguments>[^()]*)\)\s*(?:\[(?P<success>[^\]]*)\]\s*(?:\{(?P<failure>[^}]*)\})?|\{(?P<failure2>[^}]*)\}\s*(?:\[(?P<success2>[^\]]*)\])?)?")
        .replace_all(input, replace_macros)
}
