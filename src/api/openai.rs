use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenAiRequest {
    pub message: String,
}

impl OpenAiRequest {
    pub fn new(message: String) -> Self {
        OpenAiRequest { message }
    }
}

impl From<OpenAiRequest> for String {
    fn from(ur: OpenAiRequest) -> Self {
        ur.message
    }
}

#[macro_export]
macro_rules! send {
    ($sse:expr) => {
        self::routes::openai_route::send()
            .and(with_sse($sse))
            .and_then(self::handlers::openai_handler::send)
    };
}
