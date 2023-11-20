use futures::StreamExt;
use once_cell::sync::Lazy;
use reqwest_eventsource::{Event, EventSource};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::{self};
use warp::http::StatusCode;
use warp::reply::with_status;

use crate::api::openai::OpenAiRequest;
use crate::api::sse::Message;
use crate::emitter::sse_emitter::{publish, Sse};
use crate::vendor::openai::{MessageAction, OpenAI};

extern crate lazy_static;

type OpenAIChan = Lazy<Mutex<Option<mpsc::UnboundedSender<(Sse, Arc<String>, Arc<String>)>>>>;

static API_CLIENT: Lazy<OpenAI> = Lazy::new(OpenAI::default);
static OPENAI_CHANNEL: OpenAIChan = Lazy::new(|| Mutex::new(None));
lazy_static! {
    static ref STOP_SIGN: Arc<String> = Arc::new(String::from("[[stop]]"));
}

pub async fn send(request: OpenAiRequest, sse: Sse) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(openai_tx) = OPENAI_CHANNEL.lock().unwrap().as_ref() {
        let uuid = Arc::new(request.uuid);
        let message = Arc::new(request.message);

        let _ = openai_tx.send((sse, uuid, message));
    }

    Ok(with_status(warp::reply(), StatusCode::OK))
}

pub async fn initialize() {
    let (openai_tx, openai_rx) = mpsc::unbounded_channel();
    *OPENAI_CHANNEL.lock().unwrap() = Some(openai_tx);
    let _openai_task = tokio::spawn(async move {
        openai_trigger(openai_rx).await;
    });
}

async fn openai_trigger(mut rx: mpsc::UnboundedReceiver<(Sse, Arc<String>, Arc<String>)>) {
    while let Some((sse, uuid, message)) = rx.recv().await {
        tokio::spawn(async move {
            let _ = openai_send(sse, uuid, message.clone()).await;
        });
    }
}

async fn openai_send(
    sse: Sse,
    uuid: Arc<String>,
    msg: Arc<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = API_CLIENT.create_request(msg.as_str().to_owned());
    let mut es = EventSource::new(request).expect("Failed to create EventSource");
    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => println!("Connection Open!"),
            Ok(Event::Message(message)) => match API_CLIENT.process(&message.data) {
                Ok(MessageAction::SendBody(body)) => {
                    publish(sse.clone(), uuid.clone(), Message::Reply(body)).await;
                }
                Ok(MessageAction::Stop) => {
                    publish(sse.clone(), uuid.clone(), Message::Reply(STOP_SIGN.clone())).await;
                    es.close()
                }
                Ok(MessageAction::NoAction) => (),
                Err(err) => println!("Error parsing message: {}", err),
            },
            Err(err) => {
                println!("Error: {}", err);
                publish(sse.clone(), uuid.clone(), Message::Reply(STOP_SIGN.clone())).await;
                es.close();
                break;
            }
        }
    }
    Ok(())
}
