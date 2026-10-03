use clap::Parser;
use color_eyre::eyre::Result;
use llm::{
    backends::ollama::Ollama,
    builder::{LLMBackend, LLMBuilder},
    chat::{ChatMessage, ChatRole},
    memory::{MemoryProvider, SharedMemory, SlidingWindowMemory},
    models::ModelsProvider,
};
use r18::tr;
use spark::{
    data::{Block, Blocks, FindById, FindMutById},
    show_data_as_table,
};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[structopt(version)]
struct Options {
    // Path to global prompt
    #[arg(short, long, default_value = "prompt.txt")]
    prompt_path: PathBuf,

    // Base URL of OpenAI-compatible API
    #[arg(short, long, default_value = "http://127.0.0.1:11434")]
    base_url: String,

    // API Key for AI
    #[arg(short, long, default_value = "")]
    api_key: String,

    // Backend for AI
    #[arg(short, long, default_value = "ollama")]
    backend: LLMBackend,

    // Model to use
    #[arg(short, long, default_value = "deepseek-r1")]
    model: String,

    // Timeout of AI answer
    #[arg(long, default_value = "100")]
    timeout: u64,

    // A max amount of words in answer
    #[arg(long, default_value = "16384")]
    max_tokens: u32,

    // Temperature of LLM
    #[arg(short, long, default_value = "1.0")]
    temperature: f32,

    // Story path
    #[arg(short, long, default_value = "save.spark")]
    input_path: PathBuf,

    // Save path
    #[arg(short, long, default_value = "save.spark")]
    save_path: PathBuf,
}

r18::init!("locales");

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    r18::auto_detect!();

    let mut current_block_index: usize = 0;
    let options = Options::parse();
    let prompt = std::fs::read_to_string(&options.prompt_path)?;

    let model = match options.backend {
        LLMBackend::Ollama => {
            let model_provider = Ollama::new(
                &options.base_url,
                Some(options.api_key.clone()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );

            let mut models = model_provider.list_models(None).await?.get_models();
            models.sort();

            let table = show_data_as_table(
                &models
                    .iter()
                    .enumerate()
                    .map(|(i, model)| vec![(i + 1).to_string(), model.to_owned()])
                    .collect::<Vec<Vec<String>>>(),
                tr!("model_table"),
            );

            termimad::print_text(&table);

            let mut model_name = String::new();
            if let Ok(_) = std::io::stdin().read_line(&mut model_name)
                && let Ok(model_id) = model_name.trim().parse::<usize>()
                && let Some(model) = models.get(model_id - 1)
            {
                model.to_owned()
            } else {
                eprintln!("{}", tr!("model_not_found"));
                options.model.clone()
            }
        }
        _ => options.model.to_owned(),
    };

    let mut blocks = {
        let content = std::fs::read(&options.input_path).unwrap_or_default();
        if let Ok(data) = serde_json::from_slice::<Vec<Block>>(&content) {
            data
        } else {
            rkyv::from_bytes::<Blocks, rkyv::rancor::Error>(&content).unwrap_or_default()
        }
    };
    blocks.sort_by_key(|block| block.id);
    let mut messages: Vec<ChatMessage> = blocks
        .iter()
        .take(2)
        .map(|block| block.to_owned().into())
        .collect();
    let memory = SlidingWindowMemory::new(10);
    let shared_memory = SharedMemory::new_reactive(memory);
    let llm = LLMBuilder::new()
        .backend(options.backend)
        .base_url(&options.base_url)
        .api_key(&options.api_key)
        .model(&model)
        .max_tokens(options.max_tokens)
        .temperature(options.temperature)
        .timeout_seconds(options.timeout)
        .memory(shared_memory.clone())
        .system(prompt)
        .build()?;

    let mut memory = shared_memory.clone();
    for message in messages.iter() {
        memory.remember(message).await?;
        println!(
            "{}",
            std::iter::repeat_n("-", termimad::terminal_size().0 as usize).collect::<String>(),
        );
        termimad::print_text(&format!(
            "<***{}***>\n{}\n",
            match message.role {
                ChatRole::User => tr!("user"),
                ChatRole::Assistant => tr!("ai"),
            },
            message.content
        ));
    }
    println!(
        "{}",
        std::iter::repeat_n('-', termimad::terminal_size().0 as usize).collect::<String>(),
    );
    if !messages.is_empty() {
        current_block_index = messages.len() - 1;
    }

    let mut message: Vec<String> = vec![];
    loop {
        let mut is_ready = false;
        let mut line = String::new();
        let current_block = blocks.find_by_id(current_block_index);
        if message.is_empty() {
            if let Some(current_block) = current_block
                && !current_block.branches.is_empty()
            {
                termimad::print_text(&get_branches_as_table(current_block, &blocks, tr!("branch_table")));
            }

            if messages.is_empty() {
                termimad::print_text(tr!("action_begin"));
            } else {
                termimad::print_text(tr!("action_needed"));
            }
        }
        std::io::stdin().read_line(&mut line)?;

        let simple_line = line.trim().to_lowercase();
        if simple_line.contains("/exit") {
            break;
        } else if simple_line.contains("/cancel") {
            if !message.is_empty() {
                let _ = message.pop();
            }

            if !message.is_empty() {
                termimad::print_text(&format!(
                    "{}\n<***{}***>\n{}",
                    std::iter::repeat_n('-', termimad::terminal_size().0 as usize)
                        .collect::<String>(),
                    tr!("user"),
                    message.join("\n")
                ));
            }
        } else {
            if simple_line.contains("/next") {
                is_ready = true;
            }
            if simple_line.contains("/save") {
                save_story(&blocks, &options.save_path)?;
            }
            message.push(line.trim_end().to_owned());
        }
        for line in &mut message {
            *line = line.replace("/next", "");
            *line = line.replace("/save", "");
        }

        if !(is_ready) {
            continue;
        }

        println!(
            "{}",
            std::iter::repeat_n('-', termimad::terminal_size().0 as usize).collect::<String>(),
        );

        let content = message.join("\n");
        messages.push(ChatMessage::user().content(&content).build());

        let response = if let Some(block) = current_block
            && let Some(user_block) = blocks.iter().find(|user_block| {
                block.branches.contains(&user_block.id) && user_block.content == content
            })
            && let Some(Some(block)) = user_block
                .branches
                .iter()
                .next()
                .map(|block| blocks.find_by_id(*block))
        {
            current_block_index = block.id;
            block.content.clone()
        } else {
            println!("{}", tr!("generating"));

            let mut user_block: Block = messages.last().unwrap().to_owned().into();
            user_block.id = blocks.len();
            user_block.branches.insert(blocks.len() + 1);

            if let Some(last_block) = blocks.find_mut_by_id(current_block_index) {
                last_block.branches.insert(user_block.id);
            }
            blocks.push(user_block);

            let mut content = llm
                .chat(&{
                    let mut messages = messages.clone();

                    if blocks.len() > 3 {
                        messages.last_mut().unwrap().content += &format!(
                            "\n\nAll ways of this story in JSON(don't print it to user):\n\t{}",
                            serde_json::to_string_pretty(&blocks)?
                                .replace(r##""role": "0""##, r##""role": "user""##)
                                .replace(r##""role": "1""##, r##""role": "assistant""##)
                        );
                    }

                    vec![messages.last().unwrap().to_owned()]
                })
                .await?
                .text()
                .unwrap();

            if let Some((_thinking, writing)) = content.split_once("</think>") {
                content = writing.to_owned();
            }
            content = spark::parse_macro(&content).to_string();
            let mut ai_block: Block = Block::ai(&content);
            ai_block.id = blocks.len();
            current_block_index = ai_block.id;
            blocks.push(ai_block);

            content
        };
        messages.push(ChatMessage::assistant().content(&response).build());
        shared_memory
            .clone()
            .remember(messages.last().unwrap())
            .await?;

        termimad::print_inline(&format!(
            "<***{}***>\n{}\n{}",
            tr!("ai"),
            response,
            std::iter::repeat_n('-', termimad::terminal_size().0 as usize).collect::<String>(),
        ));
        message.clear();
    }

    save_story(&blocks, &options.save_path)
}

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
