use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::vendor::message::*;

static MODEL: &str = "gpt-4o";
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
        temperature: Some(0.0),
        max_tokens: MAX_TOKENS,
        model: MODEL,
        top_p: Some(0.1),
        frequency_penalty: Some(0),
        presence_penalty: Some(0),
        messages: Vec::new(),
        user: Some(uuid.as_str()),
        tools: None,
        tool_choice: None,
    };

    // Always start with a system prompt
    messages
        .messages
        .push(Message::new(ROLE_SYSTEM, json!(PROMPT)));

    // Restore context history from previous conversation
    let context = context.unwrap_or_default();
    messages.inject_histories(&context);

    // Construct the user message content
    let mut user_content = vec![json!({
        "type": "text",
        "text": msg.as_str()
    })];

    // If an image is provided, add it to the content
    if let Some(image_url) = image {
        user_content.push(json!({
            "type": "image_url",
            "image_url": {
                "url": image_url.as_str()
            }
        }));
    }

    // Append user new message
    messages
        .messages
        .push(Message::new(ROLE_USER, json!(user_content)));

    // Allow additional tool plugins
    messages.inject_tools();

    json!(&messages)
}
