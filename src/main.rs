use futures::StreamExt;
use once_cell::sync::Lazy;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::json;
use std::time::Duration;

static MODEL: &str = "gpt-4-1106-preview";
static MAX_TOKENS: i32 = 1024 * 4;
static PROMPT: &str = r#"#1 You are a professional Rust lang AI assistant can 
answer technicial questions based on context given. You need to make sure all 
the code MUST wrapped inside
```(code-language)
(code)
```"#;
static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .build()
        .expect("Failed to create Client")
});

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct MessagesWrapper {
    stream: bool,
    temperature: f32,
    max_tokens: i32,
    model: String,
    top_p: f32,
    frequency_penalty: usize,
    presence_penalty: usize,
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct EventData {
    id: String,
    object: String,
    created: u64,
    model: String,
    system_fingerprint: String,
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    index: u64,
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

fn build_request() -> reqwest::RequestBuilder {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap();
    let messages = MessagesWrapper {
        stream: true,
        temperature: 0.0,
        max_tokens: MAX_TOKENS,
        model: MODEL.to_string(),
        top_p: 0.1,
        frequency_penalty: 0,
        presence_penalty: 0,
        messages: vec![
            Message {
                role: "system".to_string(),
                content: PROMPT.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "How can I use generic in rust?".to_string(),
            },
        ],
    };
    let json_payload = json!(&messages);

    CLIENT
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(60 * 10))
        .json(&json_payload)
}

async fn receive(mut rx: tokio::sync::mpsc::UnboundedReceiver<String>) {
    while let Some(message) = rx.recv().await {
        print!("{}", message);
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Create a channel for sending messages to SSE clients
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Spawn a task to handle OpenAI responses
    let fetch_handle = tokio::spawn(async move {
        let mut es = EventSource::new(build_request()).expect("Failed to create EventSource");
        let tx_clone = tx.clone();
        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => println!("Connection Open!"),
                Ok(Event::Message(message)) => {
                    // Parse the inner JSON string in the `data` field into the EventData struct
                    let event_data: EventData = serde_json::from_str(&message.data).unwrap();
                    let choice = &event_data.choices[0];
                    match choice.finish_reason.as_ref() {
                        Some(reason) => {
                            if *reason == "stop".to_string() {
                                es.close();
                                drop(tx_clone);
                                break;
                            }
                        }
                        None => match &choice.delta.content {
                            Some(body) => {
                                // Send the body to SSE clients
                                let _ = tx_clone.send(body.clone());
                            }
                            None => (),
                        },
                    }
                }
                Err(err) => {
                    println!("Error: {}", err);
                    es.close();
                }
            }
        }
    });

    let receive_handle = tokio::spawn(receive(rx));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(fetch_handle, receive_handle);
}
