use serde::Serialize;
use serde_json::{json, Value};
use std::env::var;
use std::str::FromStr;

use super::*;

pub const ROLE_SYSTEM: &str = "system";
pub const ROLE_ASSISTANT: &str = "assistant";
pub const ROLE_USER: &str = "user";

const USE_PLUGIN: &str = "USE_PLUGIN";
const STOP_SIGNAL: &str = "[[stop]]";

pub trait HistoryHandler<'a> {
    fn inject_histories(&mut self, context: &'a str);
    fn reload_memory(&mut self, s: &'a str) -> (&'a str, &'a str);
}

pub trait ToolHandler {
    fn inject_tools(&mut self);
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

#[derive(Debug, Serialize)]
pub struct MessagesWrapper<'a> {
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    pub model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<usize>,
    pub messages: Vec<Message<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<plugins::tool::Tool<'a>>>,
}

impl<'a> MessagesWrapper<'a> {
    pub fn set_tool_choice(&mut self, choice: &'a str) {
        self.tool_choice = Some(choice);
    }

    pub fn set_tools(&mut self, tools: Vec<plugins::tool::Tool<'a>>) {
        self.tools = Some(tools);
    }

    pub fn set_histories(&mut self, memories: Vec<Message<'a>>) {
        self.messages.extend(memories);
    }
}

impl<'a> HistoryHandler<'a> for MessagesWrapper<'a> {
    fn inject_histories(&mut self, context: &'a str) {
        let histories: Vec<Message> = context
            .split(STOP_SIGNAL)
            .filter(|&s| !s.is_empty()) // Filter out empty strings
            .map(|s| {
                let (role, content) = self.reload_memory(s);
                Message::new(role, json!(content))
            })
            .collect();
        self.set_histories(histories);
    }

    fn reload_memory(&mut self, s: &'a str) -> (&'a str, &'a str) {
        if let Some(stripped) = s.strip_prefix("user:") {
            (ROLE_USER, stripped)
        } else if let Some(stripped) = s.strip_prefix("tool:") {
            ("tool", stripped)
        } else {
            (ROLE_ASSISTANT, s)
        }
    }
}

impl<'a> ToolHandler for MessagesWrapper<'a> {
    fn inject_tools(&mut self) {
        let use_plugin = var(USE_PLUGIN).unwrap_or_else(|_| "false".to_string());
        if bool::from_str(&use_plugin).unwrap() {
            self.set_tool_choice("auto");
            self.set_tools(plugins::tool::payload().tools);
        }
    }
}
