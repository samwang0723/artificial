use futures::StreamExt;
use once_cell::sync::Lazy;
use reqwest_eventsource::{Event, EventSource};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::{self};
use warp::http::StatusCode;
use warp::reply::with_status;

use crate::api::claude::ClaudeRequest;
use crate::api::claude::ClaudeRequestIntermediate;
use crate::api::sse::Message;
use crate::emitter::*;
use crate::vendor::claude::{Claude, MessageAction};

type ClaudeChan = Lazy<
    Mutex<Option<mpsc::UnboundedSender<(sse_emitter::Sse, memory_emitter::Memory, ClaudeRequest)>>>,
>;

static API_CLIENT: Lazy<Claude> = Lazy::new(Claude::default);
static CLAUDE_CHANNEL: ClaudeChan = Lazy::new(|| Mutex::new(None));
static STOP_SIGN: Lazy<Arc<String>> = Lazy::new(|| Arc::new(String::from("[[stop]]")));

pub async fn send(
    request: ClaudeRequestIntermediate,
    sse: sse_emitter::Sse,
    mem: memory_emitter::Memory,
) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(claude_tx) = CLAUDE_CHANNEL.lock().unwrap().as_ref() {
        let request: ClaudeRequest = request.into();
        let _ = claude_tx.send((sse, mem, request));
    }

    Ok(with_status(warp::reply(), StatusCode::OK))
}

pub async fn setup_claude_chan() {
    let (claude_tx, claude_rx) = mpsc::unbounded_channel();
    *CLAUDE_CHANNEL.lock().unwrap() = Some(claude_tx);
    let _claude_task = tokio::spawn(async move {
        listening_claude(claude_rx).await;
    });
}

async fn listening_claude(
    mut rx: mpsc::UnboundedReceiver<(sse_emitter::Sse, memory_emitter::Memory, ClaudeRequest)>,
) {
    while let Some((sse, mem, request)) = rx.recv().await {
        tokio::spawn(async move {
            let _ = request_to_claude(sse, mem, request).await;
        });
    }
}

async fn request_to_claude(
    sse: sse_emitter::Sse,
    mem: memory_emitter::Memory,
    request: ClaudeRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let memory = memory_emitter::get_memory(mem.clone(), request.uuid.clone()).await;

    // Record lastest user input message
    let latest_input = Arc::new(format!("user:{}[[stop]]", request.message));
    memory_emitter::record(mem.clone(), request.uuid.clone(), latest_input).await;

    let claude_request = API_CLIENT.create_request(request.message, Some(memory));
    let mut es = EventSource::new(claude_request).expect("Failed to create EventSource");
    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => println!("Connection Open!"),
            Ok(Event::Message(message)) => match API_CLIENT.process(&message.data).await {
                Ok(MessageAction::SendBody(body)) => {
                    let b_clone = body.clone();
                    sse_emitter::publish(
                        sse.clone(),
                        request.uuid.clone(),
                        Message::Reply(b_clone),
                    )
                    .await;
                    memory_emitter::record(mem.clone(), request.uuid.clone(), body).await;
                }
                Ok(MessageAction::Stop) => {
                    sse_emitter::publish(
                        sse.clone(),
                        request.uuid.clone(),
                        Message::Reply(STOP_SIGN.clone()),
                    )
                    .await;
                    memory_emitter::record(mem.clone(), request.uuid.clone(), STOP_SIGN.clone())
                        .await;
                    es.close()
                }
                Ok(MessageAction::NoAction) => (),
                Err(err) => println!("Error parsing message: {}", err),
            },
            Err(err) => {
                println!("Error: {}", err);
                sse_emitter::publish(
                    sse.clone(),
                    request.uuid.clone(),
                    Message::Reply(STOP_SIGN.clone()),
                )
                .await;
                memory_emitter::record(mem.clone(), request.uuid.clone(), STOP_SIGN.clone()).await;
                es.close();
                return Err(Box::new(err));
            }
        }
    }
    Ok(())
}
