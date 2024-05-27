use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;

use super::requests::*;

static API_KEY: Lazy<String> = Lazy::new(|| std::env::var("CLAUDE_API_KEY").unwrap());

pub enum MessageAction {
    SendBody(Arc<String>),
    Stop,
    NoAction,
}

pub struct Claude<'a> {
    api_key: &'a str,
    default_timeout: Duration,
    client: reqwest::Client,
}

impl<'a> Default for Claude<'a> {
    fn default() -> Self {
        Claude {
            api_key: API_KEY.as_str(),
            default_timeout: Duration::from_secs(60 * 10),
            client: reqwest::Client::builder()
                .build()
                .expect("Failed to create Client for Claude"),
        }
    }
}

impl Claude<'_> {
    pub fn create_request(
        &self,
        msg: Arc<String>,
        context: Option<String>,
    ) -> reqwest::RequestBuilder {
        let json_payload = claude::get_payload(msg, context);

        self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", self.api_key.to_string())
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .timeout(self.default_timeout)
            .json(&json_payload)
    }

    pub async fn process(&self, message: &str) -> Result<MessageAction, anyhow::Error> {
        let data: claude::Data = serde_json::from_str(message)?;

        if let Some(reason) = &data.type_ {
            match reason.as_str() {
                "content_block_delta" => {
                    if let Some(delta) = &data.delta {
                        if let Some(body) = &delta.text {
                            let body = Arc::new(body.to_string());
                            return Ok(MessageAction::SendBody(body));
                        }
                    }
                    Ok(MessageAction::NoAction)
                }
                "message_stop" => Ok(MessageAction::Stop),
                _ => Ok(MessageAction::NoAction),
            }
        } else {
            Ok(MessageAction::NoAction)
        }
    }
}
