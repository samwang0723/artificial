use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use super::history::inject_histories;
use super::plugins::tool::Tool;
use super::tool::inject_tools;

pub const ROLE_SYSTEM: &str = "system";
pub const ROLE_ASSISTANT: &str = "assistant";
pub const ROLE_USER: &str = "user";

static MODEL: &str = "gpt-4-turbo-preview";
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

#[derive(Debug, Serialize)]
pub struct MessagesWrapper<'a> {
    stream: bool,
    temperature: f32,
    max_tokens: i32,
    model: &'a str,
    top_p: f32,
    frequency_penalty: usize,
    presence_penalty: usize,
    messages: Vec<Message<'a>>,
    user: &'a str,
    tool_choice: Option<&'a str>,
    tools: Option<Vec<Tool<'a>>>,
}

impl<'a> MessagesWrapper<'a> {
    pub fn set_tool_choice(&mut self, choice: &'a str) {
        self.tool_choice = Some(choice);
    }

    pub fn set_tools(&mut self, tools: Vec<Tool<'a>>) {
        self.tools = Some(tools);
    }

    pub fn set_histories(&mut self, memories: Vec<Message<'a>>) {
        self.messages.extend(memories);
    }
}

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

pub fn get_payload(
    uuid: Arc<String>,
    msg: Arc<String>,
    context: Option<String>,
) -> serde_json::Value {
    let mut messages = MessagesWrapper {
        stream: true,
        temperature: 0.0,
        max_tokens: MAX_TOKENS,
        model: MODEL,
        top_p: 0.1,
        frequency_penalty: 0,
        presence_penalty: 0,
        messages: Vec::new(),
        user: uuid.as_str(),
        tools: None,
        tool_choice: None,
    };

    // Always start with a system prompt
    messages.messages.push(Message::new(ROLE_SYSTEM, PROMPT));

    // Restore context history from previous conversation
    let context = context.unwrap_or_default();
    inject_histories(&mut messages, &context);

    // Append user new message
    messages.messages.push(Message {
        role: ROLE_USER,
        content: msg.as_str(),
    });

    // allow additional tool plugins
    inject_tools(&mut messages);

    json!(&messages)
}
