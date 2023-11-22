use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

const ROLE_SYSTEM: &str = "system";
const ROLE_ASSISTANT: &str = "assistant";
const ROLE_USER: &str = "user";

static MODEL: &str = "gpt-4-1106-preview";
static MAX_TOKENS: i32 = 1024 * 4;
static STOP_SIGN: &str = "stop";
static PROMPT: &str = r#"#1 You are a professional Coding AI assistant can
answer technicial questions based on context given. Analysis questions step by
step and being very clear & precise on the problems and solutions.
You need to make sure all the code MUST wrapped inside
```(code-language)
(code)
```
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
}

#[derive(Debug, Deserialize)]
struct EventData {
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
}

pub enum MessageAction {
    SendBody(Arc<String>),
    Stop,
    NoAction,
}

pub struct OpenAI<'a> {
    api_key: &'a str,
    model: &'a str,
    stream_enabled: bool,
    default_timeout: Duration,
    client: reqwest::Client,
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
                .expect("Failed to create Client"),
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
        let ctx = context
            .as_deref()
            .map(|ctx_message| {
                if !ctx_message.is_empty() {
                    Message::new(ROLE_ASSISTANT, ctx_message)
                } else {
                    Message::new(ROLE_SYSTEM, PROMPT)
                }
            })
            .unwrap_or_else(|| Message::new(ROLE_SYSTEM, PROMPT));

        let messages = MessagesWrapper {
            stream: self.stream_enabled,
            temperature: 0.0,
            max_tokens: MAX_TOKENS,
            model: self.model,
            top_p: 0.1,
            frequency_penalty: 0,
            presence_penalty: 0,
            messages: vec![
                ctx,
                Message {
                    role: ROLE_USER,
                    content: msg.as_str(),
                },
            ],
            user: uuid.as_str(),
        };

        json!(&messages)
    }

    pub fn create_request(
        &self,
        uuid: Arc<String>,
        msg: Arc<String>,
        context: Option<String>,
    ) -> reqwest::RequestBuilder {
        let json_payload = self.payload(uuid, msg, context);
        self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(self.default_timeout)
            .json(&json_payload)
    }

    pub fn process(&self, message: &str) -> Result<MessageAction, serde_json::Error> {
        let event_data: EventData = serde_json::from_str(message)?;
        let choice = &event_data.choices[0];
        match &choice.finish_reason {
            Some(reason) if reason.as_str() == STOP_SIGN => Ok(MessageAction::Stop),
            None => match &choice.delta.content {
                Some(body) => {
                    let body = Arc::new(body.to_string());
                    Ok(MessageAction::SendBody(body))
                }
                None => Ok(MessageAction::NoAction),
            },
            _ => Ok(MessageAction::NoAction),
        }
    }
}
