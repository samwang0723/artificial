use super::request::*;
use super::request::{Message, MessagesWrapper};
use serde_json::json;

pub fn inject_claude_histories<'a>(
    messages: &mut super::claude_request::MessagesWrapper<'a>,
    context: &'a str,
) {
    let histories: Vec<super::claude_request::Message> = context
        .split("[[stop]]")
        .filter(|&s| !s.is_empty()) // Filter out empty strings
        .map(|s| {
            let (role, content) = reload_memory(s);
            super::claude_request::Message::new(role, json!(content))
        })
        .collect();
    messages.set_histories(histories);
}

pub fn inject_histories<'a>(messages: &mut MessagesWrapper<'a>, context: &'a str) {
    let histories: Vec<Message> = context
        .split("[[stop]]")
        .filter(|&s| !s.is_empty()) // Filter out empty strings
        .map(|s| {
            let (role, content) = reload_memory(s);
            Message::new(role, json!(content))
        })
        .collect();
    messages.set_histories(histories);
}

fn reload_memory(s: &str) -> (&str, &str) {
    if let Some(stripped) = s.strip_prefix("user:") {
        (ROLE_USER, stripped)
    } else if let Some(stripped) = s.strip_prefix("tool:") {
        ("tool", stripped)
    } else {
        (ROLE_ASSISTANT, s)
    }
}
