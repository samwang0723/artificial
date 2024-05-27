use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::vendor::message::*;

static MODEL: &str = "claude-3-opus-20240229";
static MAX_TOKENS: i32 = 1024 * 4;

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

pub fn get_payload(msg: Arc<String>, context: Option<String>) -> serde_json::Value {
    let mut messages = MessagesWrapper {
        stream: true,
        max_tokens: MAX_TOKENS,
        model: MODEL,
        messages: Vec::new(),
        temperature: None,
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        user: None,
        tool_choice: None,
        tools: None,
    };

    // Restore context history from previous conversation
    let context = context.unwrap_or_default();
    messages.inject_histories(&context);

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
