use futures::StreamExt;
use once_cell::sync::Lazy;
use reqwest_eventsource::{Event, EventSource};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::{self};
use warp::http::StatusCode;
use warp::reply::with_status;

use crate::api::openai::OpenAiRequest;
use crate::api::openai::OpenAiRequestIntermediate;
use crate::api::sse::Message;
use crate::emitter::memory_emitter::{get_memory, record, Memory};
use crate::emitter::sse_emitter::{publish, Sse};
use crate::vendor::openai::{MessageAction, OpenAI};

type OpenAIChan = Lazy<Mutex<Option<mpsc::UnboundedSender<(Sse, Memory, OpenAiRequest)>>>>;

static API_CLIENT: Lazy<OpenAI> = Lazy::new(OpenAI::default);
static OPENAI_CHANNEL: OpenAIChan = Lazy::new(|| Mutex::new(None));
static STOP_SIGN: Lazy<Arc<String>> = Lazy::new(|| Arc::new(String::from("[[stop]]")));

pub async fn send(
    request: OpenAiRequestIntermediate,
    sse: Sse,
    mem: Memory,
) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(openai_tx) = OPENAI_CHANNEL.lock().unwrap().as_ref() {
        let request: OpenAiRequest = request.into();
        let _ = openai_tx.send((sse, mem, request));
    }

    Ok(with_status(warp::reply(), StatusCode::OK))
}

pub async fn setup_openai_chan() {
    let (openai_tx, openai_rx) = mpsc::unbounded_channel();
    *OPENAI_CHANNEL.lock().unwrap() = Some(openai_tx);
    let _openai_task = tokio::spawn(async move {
        listening_openai(openai_rx).await;
    });
}

async fn listening_openai(mut rx: mpsc::UnboundedReceiver<(Sse, Memory, OpenAiRequest)>) {
    while let Some((sse, mem, request)) = rx.recv().await {
        tokio::spawn(async move {
            let _ = request_to_openai(sse, mem, request).await;
        });
    }
}

async fn request_to_openai(
    sse: Sse,
    mem: Memory,
    request: OpenAiRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let memory = get_memory(mem.clone(), request.uuid.clone()).await;
    if !request.message.starts_with("tool:") {
        let history = Arc::new(format!("user:{}[[stop]]", request.message));
        record(mem.clone(), request.uuid.clone(), history).await;
    };

    let openai_request =
        API_CLIENT.create_request(request.uuid.clone(), request.message, Some(memory));
    let mut es = EventSource::new(openai_request).expect("Failed to create EventSource");
    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => println!("Connection Open!"),
            Ok(Event::Message(message)) => match API_CLIENT.process(&message.data).await {
                Ok(MessageAction::SendBody(body)) => {
                    let b_clone = body.clone();
                    publish(sse.clone(), request.uuid.clone(), Message::Reply(b_clone)).await;
                    record(mem.clone(), request.uuid.clone(), body).await;
                }
                Ok(MessageAction::SendFunc(body)) => {
                    es.close();
                    let forward = OpenAiRequestIntermediate {
                        uuid: request.uuid.clone().to_string(),
                        message: format!("tool:{}", body),
                    };
                    let _ = send(forward, sse.clone(), mem.clone()).await;
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
