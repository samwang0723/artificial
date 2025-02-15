use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use reqwest;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::vendor::message::*;

static MODEL: &str = "claude-3-5-sonnet-latest";
static MAX_TOKENS: i32 = 1024 * 4;

#[derive(Debug, Deserialize)]
pub struct Data {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub delta: Option<Delta>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Delta {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub text: Option<String>,
    pub partial_json: Option<String>,
}

pub fn get_payload(
    msg: Arc<String>,
    image: Option<Arc<String>>,
    context: Option<String>,
) -> serde_json::Value {
    let mut messages = MessagesWrapper {
        stream: true,
        max_tokens: Some(MAX_TOKENS),
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
    let mut user_content = vec![json!({
        "type": "text",
        "text": msg.as_str()
    })];

    // If an image is provided, add it to the content
    if let Some(image_url) = image {
        if let Ok(base64_image) = download_and_encode_image(image_url.as_str()) {
            user_content.push(json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": get_media_type(image_url.as_str()),
                    "data": format!("{}", base64_image),
                }
            }));
        }
    }

    // Append user new messages
    messages
        .messages
        .push(Message::new(ROLE_USER, json!(user_content)));

    json!(&messages)
}

/// Downloads an image from a URL and converts it to base64
pub fn download_and_encode_image(url: &str) -> Result<Arc<String>> {
    // Download the image synchronously
    let response = reqwest::blocking::get(url)?;
    let image_bytes = response.bytes()?;

    // Convert to base64
    let base64_image = general_purpose::STANDARD.encode(&image_bytes);

    Ok(Arc::new(base64_image))
}

/// Determines the media type from the image URL
fn get_media_type(url: &str) -> &'static str {
    let lowercase_url = url.to_lowercase();
    if lowercase_url.ends_with(".png") {
        "image/png"
    } else if lowercase_url.ends_with(".jpg") || lowercase_url.ends_with(".jpeg") {
        "image/jpeg"
    } else if lowercase_url.ends_with(".gif") {
        "image/gif"
    } else if lowercase_url.ends_with(".webp") {
        "image/webp"
    } else {
        // Default to png if unknown
        "image/png"
    }
}
