use warp::filters::BoxedFilter;
use warp::path;
use warp::Filter;

fn path_prefix() -> BoxedFilter<()> {
    path!("api" / "v1" / "sse" / ..).boxed()
}

pub fn sse() -> BoxedFilter<()> {
    warp::get()
        .and(path_prefix())
        .and(warp::path::end())
        .boxed()
}
