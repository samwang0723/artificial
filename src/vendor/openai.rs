use anyhow::{anyhow, Result};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use super::plugins::{tool, tool::Tool};

const ROLE_SYSTEM: &str = "system";
const ROLE_ASSISTANT: &str = "assistant";
const ROLE_USER: &str = "user";

static MODEL: &str = "gpt-4-1106-preview";
static MAX_TOKENS: i32 = 1024 * 4;
static PROMPT: &str = r#"#1 You are playing two roles:
a. professional Coding AI assistant can answer technicial questions based on context given.
Analysis questions step by step and being very clear & precise on the problems and solutions.
You need to make sure all the code MUST wrapped inside
```(code-language)
(code)
```
b. pro stock analyzer can parse the data from tools and provide suggestions. The strategy is to
compare the data with concentration(1-60)/foreign/foreign10/trust/trust10/dealer, the more positive
means the stronger the stock may perform. when response, don't show the strategy.
#2 Did the answer meet the assignment?
#3 Review your answer and find problems within
#4 Based on the problems you found, improve your answer
"#;
static API_KEY: Lazy<String> = Lazy::new(|| std::env::var("OPENAI_API_KEY").unwrap());

#[derive(Debug, Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct MessagesWrapper<'a> {
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

#[derive(Debug, Deserialize)]
struct EventData {
    id: String,
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ToolCall {
    function: FunctionCall,
}

#[derive(Serialize, Deserialize, Debug)]
struct FunctionCall {
    name: Option<String>,
    arguments: String,
}

pub enum MessageAction {
    SendBody(Arc<String>),
    SendFunc(Arc<String>),
    Stop,
    NoAction,
}

pub struct OpenAI<'a> {
    api_key: &'a str,
    model: &'a str,
    stream_enabled: bool,
    default_timeout: Duration,
    client: reqwest::Client,
    function_calls: DashMap<String, String>,
}

impl<'a> Message<'a> {
    fn new(role: &'a str, content: &'a str) -> Self {
        Message { role, content }
    }
}

impl<'a> Default for OpenAI<'a> {
    fn default() -> Self {
        OpenAI {
            api_key: API_KEY.as_str(),
            model: MODEL,
            stream_enabled: true,
            default_timeout: Duration::from_secs(60 * 10),
            client: reqwest::Client::builder()
                .build()
                .expect("Failed to create Client for OpenAI"),
            function_calls: DashMap::new(),
        }
    }
}

impl OpenAI<'_> {
    fn payload(
        &self,
        uuid: Arc<String>,
        msg: Arc<String>,
        context: Option<String>,
    ) -> serde_json::Value {
        let mut messages = MessagesWrapper {
            stream: self.stream_enabled,
            temperature: 0.0,
            max_tokens: MAX_TOKENS,
            model: self.model,
            top_p: 0.1,
            frequency_penalty: 0,
            presence_penalty: 0,
            messages: Vec::new(),
            user: uuid.as_str(),
            tools: None,
            tool_choice: None,
        };

        messages.messages.push(Message::new(ROLE_SYSTEM, PROMPT));
        let context = context.unwrap_or_default();
        let histories: Vec<Message> = context
            .split("[[stop]]")
            .filter(|&s| !s.is_empty()) // Filter out empty strings
            .map(|s| {
                let (role, content) = history_rebuild(s);
                Message { role, content }
            })
            .collect();
        for history in histories {
            messages.messages.push(history);
        }
        messages.messages.push(Message {
            role: ROLE_USER,
            content: msg.as_str(),
        });

        // allow additional plugins
        let use_plugin = std::env::var("USE_PLUGIN").unwrap_or_else(|_| "false".to_string());
        if bool::from_str(&use_plugin).unwrap() {
            messages.tools = Some(tool::payload().tools);
            messages.tool_choice = Some("auto");
        }

        json!(&messages)
    }

    pub fn create_request(
        &self,
        uuid: Arc<String>,
        msg: Arc<String>,
        context: Option<String>,
    ) -> reqwest::RequestBuilder {
        let json_payload = self.payload(uuid, msg, context);
        println!("payload: {:?}", json_payload);
        self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(self.default_timeout)
            .json(&json_payload)
    }

    pub async fn process(&self, message: &str) -> Result<MessageAction, anyhow::Error> {
        let event_data: EventData = serde_json::from_str(message)?;
        let choice = &event_data.choices[0];
        let id = &event_data.id;
        match &choice.finish_reason {
            Some(reason) => match reason.as_str() {
                "tool_calls" => match self.dispatch(id).await {
                    Ok(body) => {
                        let body = Arc::new(body);
                        Ok(MessageAction::SendFunc(body))
                    }
                    Err(err) => Err(anyhow!("unable to dispatch to plugin: {:?}", err)),
                },
                _ => Ok(MessageAction::Stop),
            },
            None => match (&choice.delta.content, &choice.delta.tool_calls) {
                (Some(body), _) => {
                    let body = Arc::new(body.to_string());
                    Ok(MessageAction::SendBody(body))
                }
                (_, Some(tool_calls)) => {
                    // Handle the case where tool_calls exists
                    // Assuming you have a way to process tool_calls and create a MessageAction
                    self.process_tool_calls(id, tool_calls);
                    Ok(MessageAction::NoAction)
                }
                (None, None) => Ok(MessageAction::NoAction),
            },
        }
    }

    async fn dispatch(&self, id: &String) -> Result<String, anyhow::Error> {
        match self.function_calls.get(id) {
            Some(val) => {
                let cmd = val.to_string();
                // self.function_calls.remove(id); //TODO: remove the cmd cache, this will cause deadlock
                match tool::dispatch(cmd).await {
                    Ok(res) => Ok(res),
                    Err(err) => Err(anyhow!("unable to dispatch to plugin: {:?}", err)),
                }
            }
            None => Err(anyhow!("unable to find function call for id: {:?}", id)),
        }
    }

    fn process_tool_calls(&self, id: &String, tool_calls: &[ToolCall]) {
        if tool_calls.is_empty() {
            return;
        }
        let call_ref = &tool_calls[0];
        let function_name = call_ref.function.name.clone().unwrap_or_default();
        let arguments = &call_ref.function.arguments;

        // Insert or update the entry in the DashMap
        let _ = self
            .function_calls
            .entry(id.to_string())
            .and_modify(|e| {
                // If the entry exists, append the arguments
                e.push_str(arguments);
            })
            .or_insert_with(|| {
                // If the entry does not exist, create it with the function name and arguments
                format!("{},{}", function_name, arguments)
            });
    }
}

fn history_rebuild(s: &str) -> (&str, &str) {
    if let Some(stripped) = s.strip_prefix("user:") {
        (ROLE_USER, stripped)
    } else if let Some(stripped) = s.strip_prefix("tool:") {
        ("tool", stripped)
    } else {
        (ROLE_ASSISTANT, s)
    }
}
