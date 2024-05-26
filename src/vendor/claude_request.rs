use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use super::history::inject_claude_histories;

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
    content: Value,
}

impl<'a> Message<'a> {
    pub fn new(role: &'a str, content: Value) -> Self {
        Message { role, content }
    }
}

impl<'a> MessagesWrapper<'a> {
    pub fn set_histories(&mut self, memories: Vec<Message<'a>>) {
        self.messages.extend(memories);
    }
}

pub fn get_payload(msg: Arc<String>, context: Option<String>) -> serde_json::Value {
    let mut messages = MessagesWrapper {
        stream: true,
        max_tokens: MAX_TOKENS,
        model: MODEL,
        messages: Vec::new(),
    };

    // Restore context history from previous conversation
    let context = context.unwrap_or_default();
    inject_claude_histories(&mut messages, &context);

    // Construct the user message content
    let user_content = vec![json!({
        "type": "text",
        "text": msg.as_str()
    })];

    // Append user new message
    messages
        .messages
        .push(Message::new(ROLE_USER, json!(user_content)));

    json!(&messages)
}
