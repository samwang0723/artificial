use futures::{stream::Stream, StreamExt};
use once_cell::sync::Lazy;
use reqwest_eventsource::{Event, EventSource};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::Filter;

mod api_client;
use api_client::*;

type OpenAIChan = Lazy<Mutex<Option<UnboundedSender<(usize, String)>>>>;
/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `Message`
type Users = Arc<Mutex<HashMap<usize, UnboundedSender<Message>>>>;

static API_CLIENT: Lazy<OpenAI> = Lazy::new(OpenAI::default);
static OPENAI_CHANNEL: OpenAIChan = Lazy::new(|| Mutex::new(None));
/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Message variants.
#[derive(Debug)]
enum Message {
    UserId(usize),
    Reply(String),
}

#[derive(Debug)]
struct NotUtf8;
impl warp::reject::Reject for NotUtf8 {}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Keep track of all connected users, key is usize, value
    // is an event stream sender.
    let users = Arc::new(Mutex::new(HashMap::new()));
    let users_clone = Arc::clone(&users);
    // Turn our "state" into a new Filter...
    let users_filter = warp::any().map(move || users.clone());
    // POST /chat -> send message
    let chat_send = warp::path("chat")
        .and(warp::post())
        .and(warp::path::param::<usize>())
        .and(warp::body::content_length_limit(500))
        .and(
            warp::body::bytes().and_then(|body: bytes::Bytes| async move {
                std::str::from_utf8(&body)
                    .map(String::from)
                    .map_err(|_e| warp::reject::custom(NotUtf8))
            }),
        )
        .and(users_filter.clone())
        .map(|my_id, msg, users| {
            user_message(my_id, msg, &users);
            warp::reply()
        });

    // GET /chat -> messages stream
    let chat_recv = warp::path("chat")
        .and(warp::get())
        .and(users_filter)
        .map(|users| {
            // reply using server-sent events
            let stream = connected(users);
            warp::sse::reply(warp::sse::keep_alive().stream(stream))
        });

    // GET / -> index html
    let index = warp::path::end().map(|| {
        warp::http::Response::builder()
            .header("content-type", "text/html; charset=utf-8")
            .body(INDEX_HTML)
    });

    // Set up the channel for sending messages to OpenAI
    let (openai_tx, openai_rx) = mpsc::unbounded_channel();
    *OPENAI_CHANNEL.lock().unwrap() = Some(openai_tx);
    let _openai_task = tokio::spawn(async move {
        openai_receive(openai_rx, users_clone).await;
    });

    let routes = index.or(chat_recv).or(chat_send);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn openai_receive(mut rx: mpsc::UnboundedReceiver<(usize, String)>, users: Users) {
    while let Some((user_id, message)) = rx.recv().await {
        let sender = {
            let users_lock = users.lock().unwrap();
            // Clone the sender associated with the user_id, if it exists
            users_lock.get(&user_id).cloned()
        };

        if let Some(tx) = sender {
            if let Err(e) = openai_send(tx.clone(), message).await {
                println!("Failed to send message: {}", e);
            }
        } else {
            println!("User ID not found: {}", user_id);
        }
    }
}

async fn openai_send(
    tx: UnboundedSender<Message>,
    msg: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = API_CLIENT.create_request(msg);
    let mut es = EventSource::new(request).expect("Failed to create EventSource");
    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => println!("Connection Open!"),
            Ok(Event::Message(message)) => match API_CLIENT.process(&message.data) {
                Ok(MessageAction::SendBody(body)) => {
                    tx.send(Message::Reply(body.clone()))?;
                }
                Ok(MessageAction::Stop) => es.close(),
                Ok(MessageAction::NoAction) => (),
                Err(err) => println!("Error parsing message: {}", err),
            },
            Err(err) => {
                println!("Error: {}", err);
                es.close();
                break;
            }
        }
    }
    Ok(())
}

fn user_message(my_id: usize, msg: String, users: &Users) {
    users.lock().unwrap().retain(|uid, _tx| {
        if my_id == *uid {
            // Send the message to OpenAI along with the user ID
            if let Some(openai_tx) = OPENAI_CHANNEL.lock().unwrap().as_ref() {
                let _ = openai_tx.send((my_id, msg.clone()));
            }
        }
        true
    });
}

fn connected(
    users: Users,
) -> impl Stream<Item = Result<warp::sse::Event, warp::Error>> + Send + 'static {
    // Use a counter to assign a new unique ID for this user.
    let user_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("chat user connected: {}", user_id);

    // Create a channel for sending messages to SSE clients
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    tx.send(Message::UserId(user_id)).unwrap();
    // Save the sender in our list of connected users.
    users.lock().unwrap().insert(user_id, tx);

    rx.map(move |msg| match msg {
        Message::UserId(my_id) => Ok(warp::sse::Event::default()
            .event("user")
            .data(my_id.to_string())),
        Message::Reply(reply) => Ok(warp::sse::Event::default().data(reply)),
    })
}

static INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html>
    <head>
        <title>Warp Chat</title>
    </head>
    <body>
        <h1>warp chat</h1>
        <div id="chat">
            <p><em>Connecting...</em></p>
        </div>
        <input type="text" id="text" />
        <button type="button" id="send">Send</button>
        <style>
        .message-body {
            text-align: left;
            width: 70%;
            flex-direction: row;
            flex-wrap: wrap;
            word-wrap: break-word;
            margin: 10px;
        }
        </style>
        <script type="text/javascript">
        let activeDiv = null;
        let currentMsg = "";
        var uri = 'http://' + location.host + '/chat';
        var sse = new EventSource(uri);
        function message(data) {
            if (activeDiv) {
                currentMsg += ' ' + data;
                activeDiv.innerHTML = currentMsg;
            } else {
                var line = document.createElement('span');
                line.classList.add("message-body");
                currentMsg = '&lt;ChatGPT&gt;: ' + data;
                line.innerHTML = currentMsg;
                chat.appendChild(line);
                activeDiv = line;
                var separator = document.createElement('p');
                chat.appendChild(separator);
            }
        }
        function message_self(data) {
            var line = document.createElement('p');
            line.innerText = data;
            chat.appendChild(line);
        }

        sse.onopen = function() {
            chat.innerHTML = "<p><em>Connected!</em></p>";
        }
        var user_id;
        sse.addEventListener("user", function(msg) {
            user_id = msg.data;
        });
        sse.onmessage = function(msg) {
            console.log(msg);
            message(msg.data);
        };
        send.onclick = function() {
            var msg = text.value;
            var xhr = new XMLHttpRequest();
            xhr.open("POST", uri + '/' + user_id, true);
            xhr.send(msg);
            text.value = '';
            message_self('<You>: ' + msg);
            activeDiv = null;
            currentMsg = "";
        };
        </script>
    </body>
</html>
"#;
