use anyhow::{anyhow, Result};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;

use super::*;

static API_KEY: Lazy<String> = Lazy::new(|| std::env::var("OPENAI_API_KEY").unwrap());

pub enum MessageAction {
    SendBody(Arc<String>),
    SendFunc(Arc<String>),
    Stop,
    NoAction,
}

pub struct OpenAI<'a> {
    api_key: &'a str,
    default_timeout: Duration,
    client: reqwest::Client,
    function_calls: DashMap<String, String>,
}

impl<'a> Default for OpenAI<'a> {
    fn default() -> Self {
        OpenAI {
            api_key: API_KEY.as_str(),
            default_timeout: Duration::from_secs(60 * 10),
            client: reqwest::Client::builder()
                .build()
                .expect("Failed to create Client for OpenAI"),
            function_calls: DashMap::new(),
        }
    }
}

impl OpenAI<'_> {
    pub fn create_request(
        &self,
        uuid: Arc<String>,
        msg: Arc<String>,
        image: Option<Arc<String>>,
        context: Option<String>,
    ) -> reqwest::RequestBuilder {
        let json_payload = requests::openai::get_payload(uuid, msg, image, context);

        self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(self.default_timeout)
            .json(&json_payload)
    }

    pub async fn process(&self, message: &str) -> Result<MessageAction, anyhow::Error> {
        let event_data: requests::openai::EventData = serde_json::from_str(message)?;
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
                    plugins::tool::append_fragment(&self.function_calls, id, tool_calls);
                    Ok(MessageAction::NoAction)
                }
                (None, None) => Ok(MessageAction::NoAction),
            },
        }
    }

    async fn dispatch(&self, id: &String) -> Result<String, anyhow::Error> {
        if let Some(cmd) = self.function_calls.get(id).map(|val| val.clone()) {
            // The reference is dropped here after cloning the value.
            self.function_calls.remove(id); // Now it's safe to remove the item.
            match plugins::tool::dispatch(cmd).await {
                Ok(res) => Ok(res),
                Err(err) => Err(anyhow!("unable to dispatch to plugin: {:?}", err)),
            }
        } else {
            Err(anyhow!("unable to find function call for id: {:?}", id))
        }
    }
}
