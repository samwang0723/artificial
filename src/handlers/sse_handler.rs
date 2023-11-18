use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
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

    tx.send(Message::Connected).unwrap();

    let mut sse = sse.lock().await;
    sse.insert(tx);

    //TODO: response with json_data
    rx.map(|msg| match msg {
        Message::Connected => Ok(Event::default()
            .event("system")
            .json_data(format!("{:?}", msg))
            .unwrap()),
        Message::Reply(text) => Ok(Event::default().event("user").data(text)),
    })
}
