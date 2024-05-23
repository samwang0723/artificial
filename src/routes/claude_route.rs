use warp::filters::BoxedFilter;
use warp::{path, Filter};

use crate::api::claude::ClaudeRequestIntermediate;

fn path_prefix() -> BoxedFilter<()> {
    path!("api" / "v1" / "send_claude" / ..).boxed()
}

pub fn send() -> BoxedFilter<(ClaudeRequestIntermediate,)> {
    let body = warp::body::content_length_limit(8192).and(warp::body::json());

    warp::post()
        .and(path_prefix())
        .and(warp::path::end())
        .and(body)
        .boxed()
}
