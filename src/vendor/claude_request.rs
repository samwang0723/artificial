use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

// pub const ROLE_ASSISTANT: &str = "assistant";
pub const ROLE_USER: &str = "user";

static MODEL: &str = "claude-3-opus-20240229";
static MAX_TOKENS: i32 = 1024 * 4;

#[derive(Debug, Serialize)]
pub struct MessagesWrapper<'a> {
    stream: bool,
    max_tokens: i32,
    model: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Debug, Deserialize)]
pub struct EventData {
    pub event: String,
    pub data: Data,
}

#[derive(Debug, Deserialize)]
pub struct Data {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub text: Option<String>,
    pub partial_json: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

impl<'a> Message<'a> {
    pub fn new(role: &'a str, content: &'a str) -> Self {
        Message { role, content }
    }
}

pub fn get_payload(msg: Arc<String>) -> serde_json::Value {
    let mut messages = MessagesWrapper {
        stream: true,
        max_tokens: MAX_TOKENS,
        model: MODEL,
        messages: Vec::new(),
    };

    // Append user new message
    messages
        .messages
        .push(Message::new(ROLE_USER, msg.as_str()));

    json!(&messages)
}
