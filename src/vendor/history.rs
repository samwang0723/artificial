use super::request::*;
use super::request::{Message, MessagesWrapper};

pub fn inject_histories<'a>(messages: &mut MessagesWrapper<'a>, context: &'a str) {
    let histories: Vec<Message> = context
        .split("[[stop]]")
        .filter(|&s| !s.is_empty()) // Filter out empty strings
        .map(|s| {
            let (role, content) = reload_memory(s);
            Message::new(role, content)
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
