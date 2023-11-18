use emitter::sse_emitter::create_sse;
use emitter::sse_emitter::with_sse;
use warp::Filter;

use crate::handlers::openai_handler::initialize;

mod api;
mod emitter;
mod handlers;
mod routes;
mod vendor;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let sse = create_sse();
    let log = warp::log("any");
    initialize().await;

    // GET / -> index html
    let index = warp::path::end().map(|| {
        warp::http::Response::builder()
            .header("content-type", "text/html; charset=utf-8")
            .body(INDEX_HTML)
    });

    let api = index.or(send!(sse.clone())).or(sse!(sse));
    let api = api.with(log);
    warp::serve(api).run(([127, 0, 0, 1], 3000)).await;
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
        var uri = 'http://' + location.host + '/api/v1/sse';
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
            console.log('user: ' + msg.data);
            message(msg.data);
        });
        sse.addEventListener("system", function(msg) {
            console.log('system: ' + msg.data);
        });

        send.onclick = function() {
            var msg = text.value;
            var xhr = new XMLHttpRequest();
            xhr.open("POST", 'http://' + location.host + '/api/v1/send', true);
            xhr.setRequestHeader('Content-Type', 'application/json; charset=UTF-8');
            var data = {
              message: msg,
            };
            var jsonStr = JSON.stringify(data);
            xhr.send(jsonStr);
            text.value = '';
            message_self('<You>: ' + msg);
            activeDiv = null;
            currentMsg = "";
        };
        </script>
    </body>
</html>
"#;
