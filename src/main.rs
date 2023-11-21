use emitter::memory_emitter::create_memory;
use emitter::memory_emitter::with_memory;
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
    let mem = create_memory();
    let log = warp::log("any");
    initialize().await;

    // Set up CORS
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST"])
        .allow_headers(vec!["Content-Type", "Authorization"]);

    // Define the directory to serve static files from.
    let static_files_dir = "static/";
    let static_files = warp::fs::dir(static_files_dir);

    let api = static_files
        .or(send!(sse.clone(), mem.clone()))
        .or(sse!(sse));
    let api = api.with(cors).with(log);

    warp::serve(api).run(([127, 0, 0, 1], 3000)).await;
}
