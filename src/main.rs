use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};

mod api_client;
use api_client::*;

async fn receive(mut rx: tokio::sync::mpsc::UnboundedReceiver<String>) {
    while let Some(message) = rx.recv().await {
        print!("{}", message);
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let api_client = api_client::OpenAI::default();
    // Create a channel for sending messages to SSE clients
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Spawn a task to handle OpenAI responses
    let request_handle = tokio::spawn(async move {
        let request = api_client.create_request("Hello, world!".to_string());
        let mut es = EventSource::new(request).expect("Failed to create EventSource");
        let tx_clone = tx.clone();
        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => println!("Connection Open!"),
                Ok(Event::Message(message)) => {
                    match api_client.process(&message.data) {
                        Ok(MessageAction::SendBody(body)) => {
                            // Send the body to SSE clients
                            let _ = &tx_clone.send(body);
                        }
                        Ok(MessageAction::Stop) => es.close(),
                        Ok(MessageAction::NoAction) => (),
                        Err(err) => println!("Error parsing message: {}", err),
                    }
                }
                Err(err) => {
                    println!("Error: {}", err);
                    es.close();
                }
            }
        }
        drop(tx_clone);
    });

    let receive_handle = tokio::spawn(receive(rx));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(request_handle, receive_handle);
}
