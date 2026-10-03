use std::collections::HashSet;
use llm::chat::{ChatMessage, ChatRole};
use rkyv::Archive;

pub type Blocks = Vec<Block>;
pub type Branches = HashSet<usize>;

#[derive(
    Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    serde::Deserialize,
    serde::Serialize,
    Debug,
    Clone,
    Default,
)]
pub struct Block {
    pub id: usize,
    pub role: u8,
    pub content: String,
    pub branches: Branches,
}

impl Block {
    pub fn user(content: &str) -> Self {
        Self {
            role: ChatRole::User as u8,
            content: content.to_owned(),
            ..Default::default()
        }
    }

    pub fn ai(content: &str) -> Self {
        Self {
            role: ChatRole::Assistant as u8,
            content: content.to_owned(),
            ..Default::default()
        }
    }
}

impl From<ChatMessage> for Block {
    fn from(value: ChatMessage) -> Self {
        match value.role {
            ChatRole::User => Self::user(&value.content),
            ChatRole::Assistant => Self::ai(&value.content),
        }
    }
}

impl From<Block> for ChatMessage {
    fn from(value: Block) -> Self {
        match value.role {
            0 => Self::user(),
            1 => Self::assistant(),
            _ => panic!("Role isn't real!"),
        }
        .content(value.content)
        .build()
    }
}

pub trait FindById<T> {
    fn find_by_id(&self, id: impl Into<usize>) -> Option<&T>;
}

impl FindById<Block> for Blocks {
    fn find_by_id(&self, id: impl Into<usize>) -> Option<&Block> {
        let id = id.into();
        self.iter().find(|block| block.id == id)
    }
}

pub trait FindMutById<T> {
    fn find_mut_by_id(&mut self, id: impl Into<usize>) -> Option<&mut T>;
}

impl FindMutById<Block> for Blocks {
    fn find_mut_by_id(&mut self, id: impl Into<usize>) -> Option<&mut Block> {
        let id = id.into();
        self.iter_mut().find(|block| block.id == id)
    }
}
