use futures::{Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::sse::Event;

use crate::api::sse::{Message, MessageEvent};
use crate::emitter::sse_emitter::Sse;

pub async fn connect(sse: Sse) -> Result<impl warp::Reply, warp::Rejection> {
    let stream = stream(sse).await;
    Ok(warp::sse::reply(warp::sse::keep_alive().stream(stream)))
}

async fn stream(sse: Sse) -> impl Stream<Item = Result<Event, warp::Error>> + Send + 'static {
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);
    let uuid = Arc::new(Uuid::new_v4().to_string()); // generate uuid for client for tx channel

    tx.send(Message::Connected(uuid.clone())).unwrap();

    let mut sse = sse.lock().await;
    sse.insert(uuid.clone(), tx);

    rx.map(|msg| match msg {
        Message::Connected(uuid) => Ok(Event::default().event("system").data(uuid.as_str())),
        Message::Reply(text) => {
            let copy = text.clone();
            let event = MessageEvent {
                message: copy.as_str(),
            };
            Ok(Event::default().event("user").json_data(event).unwrap())
        }
    })
}
