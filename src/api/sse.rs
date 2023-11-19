use serde::Serialize;

/// Message variants.
#[derive(Serialize, Debug, Clone)]
pub enum Message {
    Connected(String),
    Reply(String),
}

#[macro_export]
macro_rules! sse {
    ($sse:expr) => {
        self::routes::sse_route::sse()
            .and(with_sse($sse))
            .and_then(self::handlers::sse_handler::connect)
    };
}
