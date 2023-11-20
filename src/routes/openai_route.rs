use warp::filters::BoxedFilter;
use warp::{path, Filter};

use crate::api::openai::OpenAiRequestIntermediate;

fn path_prefix() -> BoxedFilter<()> {
    path!("api" / "v1" / "send" / ..).boxed()
}

pub fn send() -> BoxedFilter<(OpenAiRequestIntermediate,)> {
    let body = warp::body::content_length_limit(4096).and(warp::body::json());

    warp::post()
        .and(path_prefix())
        .and(warp::path::end())
        .and(body)
        .boxed()
}
