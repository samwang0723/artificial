use serde::Serialize;
use std::sync::Arc;

/// Message variants.
#[derive(Debug, Clone)]
pub enum Message {
    Connected(Arc<String>),
    Reply(Arc<String>),
}

#[derive(Debug, Serialize)]
pub struct MessageEvent<'a> {
    pub message: &'a str,
}

#[macro_export]
macro_rules! sse {
    ($sse:expr) => {
        self::routes::sse_route::sse()
            .and(with_sse($sse))
            .and_then(self::handlers::sse_handler::connect)
    };
}
