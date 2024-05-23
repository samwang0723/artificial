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
use crate::emitter::memory_emitter::{get_memory, record, Memory};
use crate::emitter::sse_emitter::{publish, Sse};
use crate::vendor::claude::{Claude, MessageAction};

type ClaudeChan = Lazy<Mutex<Option<mpsc::UnboundedSender<(Sse, Memory, ClaudeRequest)>>>>;

static API_CLIENT: Lazy<Claude> = Lazy::new(Claude::default);
static CLAUDE_CHANNEL: ClaudeChan = Lazy::new(|| Mutex::new(None));
static STOP_SIGN: Lazy<Arc<String>> = Lazy::new(|| Arc::new(String::from("[[stop]]")));

pub async fn send(
    request: ClaudeRequestIntermediate,
    sse: Sse,
    mem: Memory,
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

async fn listening_claude(mut rx: mpsc::UnboundedReceiver<(Sse, Memory, ClaudeRequest)>) {
    while let Some((sse, mem, request)) = rx.recv().await {
        tokio::spawn(async move {
            let _ = request_to_claude(sse, mem, request).await;
        });
    }
}

async fn request_to_claude(
    sse: Sse,
    mem: Memory,
    request: ClaudeRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let _memory = get_memory(mem.clone(), request.uuid.clone()).await;

    let claude_request = API_CLIENT.create_request(request.message);
    let mut es = EventSource::new(claude_request).expect("Failed to create EventSource");
    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => println!("Connection Open!"),
            Ok(Event::Message(message)) => match API_CLIENT.process(&message.data).await {
                Ok(MessageAction::SendBody(body)) => {
                    let b_clone = body.clone();
                    publish(sse.clone(), request.uuid.clone(), Message::Reply(b_clone)).await;
                    record(mem.clone(), request.uuid.clone(), body).await;
                }
                Ok(MessageAction::Stop) => {
                    publish(
                        sse.clone(),
                        request.uuid.clone(),
                        Message::Reply(STOP_SIGN.clone()),
                    )
                    .await;
                    record(mem.clone(), request.uuid.clone(), STOP_SIGN.clone()).await;
                    es.close()
                }
                Ok(MessageAction::NoAction) => (),
                Err(err) => println!("Error parsing message: {}", err),
            },
            Err(err) => {
                println!("Error: {}", err);
                publish(
                    sse.clone(),
                    request.uuid.clone(),
                    Message::Reply(STOP_SIGN.clone()),
                )
                .await;
                record(mem.clone(), request.uuid.clone(), STOP_SIGN.clone()).await;
                es.close();
                return Err(Box::new(err));
            }
        }
    }
    Ok(())
}
