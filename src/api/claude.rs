use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ClaudeRequestIntermediate {
    pub uuid: String,
    pub message: String,
    pub image: Option<String>,
}

#[derive(Debug)]
pub struct ClaudeRequest {
    pub uuid: Arc<String>,
    pub message: Arc<String>,
    pub image: Option<Arc<String>>,
}

impl From<ClaudeRequestIntermediate> for ClaudeRequest {
    fn from(intermediate: ClaudeRequestIntermediate) -> Self {
        ClaudeRequest {
            uuid: Arc::new(intermediate.uuid),
            message: Arc::new(intermediate.message),
            image: intermediate.image.map(Arc::new),
        }
    }
}

#[macro_export]
macro_rules! send_claude {
    ($sse:expr, $mem:expr) => {
        self::routes::claude_route::send()
            .and(with_sse($sse))
            .and(with_memory($mem))
            .and_then(self::handlers::claude_handler::send)
    };
}
