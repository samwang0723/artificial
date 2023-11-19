use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::sse::Event;

use crate::api::sse::Message;
use crate::emitter::sse_emitter::Sse;

pub async fn connect(sse: Sse) -> Result<impl warp::Reply, warp::Rejection> {
    let stream = stream(sse).await;
    Ok(warp::sse::reply(warp::sse::keep_alive().stream(stream)))
}

async fn stream(sse: Sse) -> impl Stream<Item = Result<Event, warp::Error>> + Send + 'static {
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);
    let uuid = Uuid::new_v4().to_string(); // generate uuid for client for tx channel

    tx.send(Message::Connected(uuid.clone())).unwrap();

    let mut sse = sse.lock().await;
    sse.insert(uuid, tx);

    //TODO: response with json_data
    rx.map(|msg| match msg {
        Message::Connected(uuid) => Ok(Event::default().event("system").data(uuid)),
        Message::Reply(text) => Ok(Event::default().event("user").data(text)),
    })
}
