use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct OpenAiRequestIntermediate {
    pub uuid: String,
    pub message: String,
    pub image: Option<String>,
}

#[derive(Debug)]
pub struct OpenAiRequest {
    pub uuid: Arc<String>,
    pub message: Arc<String>,
    pub image: Option<Arc<String>>,
}

impl From<OpenAiRequestIntermediate> for OpenAiRequest {
    fn from(intermediate: OpenAiRequestIntermediate) -> Self {
        OpenAiRequest {
            uuid: Arc::new(intermediate.uuid),
            message: Arc::new(intermediate.message),
            image: intermediate.image.map(Arc::new),
        }
    }
}

#[macro_export]
macro_rules! send {
    ($sse:expr, $mem:expr) => {
        self::routes::openai_route::send()
            .and(with_sse($sse))
            .and(with_memory($mem))
            .and_then(self::handlers::openai_handler::send)
    };
}
