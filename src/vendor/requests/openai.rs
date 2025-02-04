use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fmt, sync::Arc};

use crate::vendor::message::*;

static MODEL: &str = "o3-mini-2025-01-31";
static MAX_TOKENS: i32 = 1024 * 4;
static PROMPT: &str = r#"#1 You are playing two roles:
a. professional Coding AI assistant can answer technicial questions based on context given.
Analysis questions step by step and being very clear & precise on the problems and solutions.
You need to make sure all the code MUST wrapped inside
```(code-language)
(code)
```
b. pro stock analyzer can parse the data from tools and provide suggestions. The strategy is to
compare the data with concentration(1-60 days, present in percentage)/foreign/foreign10/trust/trust10/dealer, the more positive
means the stronger the stock may perform. when response, don't show the strategy.
#2 Did the answer meet the assignment?
#3 Review your answer and find problems within
#4 Based on the problems you found, improve your answer
"#;

#[derive(Debug, Deserialize)]
pub struct EventData {
    pub id: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCall {
    pub function: FunctionCall,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionCall {
    pub name: Option<String>,
    pub arguments: String,
}

pub fn get_payload(
    uuid: Arc<String>,
    msg: Arc<String>,
    image: Option<Arc<String>>,
    context: Option<String>,
) -> serde_json::Value {
    let mut messages = MessagesWrapper {
        stream: true,
        model: MODEL,
        messages: Vec::new(),
        user: Some(uuid.as_str()),
        max_tokens: None,
        temperature: None,
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        tools: None,
        tool_choice: None,
    };

    // Always start with a system prompt
    if MODEL != "o3-mini-2025-01-31" {
        messages.max_tokens = Some(MAX_TOKENS);
        messages
            .messages
            .push(Message::new(ROLE_SYSTEM, json!(PROMPT)));
    }

    // Restore context history from previous conversation
    let context = context.unwrap_or_default();
    messages.inject_histories(&context);

    // Construct the user message content
    let mut user_content = vec![json!({
        "type": "text",
        "text": msg.as_str()
    })];

    // If an image is provided, add it to the content
    if MODEL != "o3-mini-2025-01-31" {
        if let Some(image_url) = image {
            user_content.push(json!({
            "type": "image_url",
            "image_url": {
                // need to add prefix data:image/png;base64,{base64 encode url}
                "url": fmt::format(format_args!("data:image/png;base64,{}", general_purpose::STANDARD.encode(image_url.as_str())))
            }
        }));
        }
    }

    // Append user new message
    messages
        .messages
        .push(Message::new(ROLE_USER, json!(user_content)));

    // Allow additional tool plugins
    if MODEL != "o3-mini-2025-01-31" {
        messages.inject_tools();
    }

    json!(&messages)
}
