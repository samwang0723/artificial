use dashmap::DashMap;
use std::env::var;
use std::str::FromStr;

use super::plugins::tool;
use super::request::{MessagesWrapper, ToolCall};

pub fn inject_tools(messages: &mut MessagesWrapper) {
    let use_plugin = var("USE_PLUGIN").unwrap_or_else(|_| "false".to_string());
    if bool::from_str(&use_plugin).unwrap() {
        messages.set_tool_choice("auto");
        messages.set_tools(tool::payload().tools);
    }
}

pub fn append_fragment(calls: &DashMap<String, String>, id: &String, tool_calls: &[ToolCall]) {
    if tool_calls.is_empty() {
        return;
    }
    let call_ref = &tool_calls[0];
    let function_name = call_ref.function.name.clone().unwrap_or_default();
    let arguments = &call_ref.function.arguments;

    // Insert or update the entry in the DashMap
    let _ = calls
        .entry(id.to_string())
        .and_modify(|e| {
            // If the entry exists, append the arguments
            e.push_str(arguments);
        })
        .or_insert_with(|| {
            // If the entry does not exist, create it with the function name and arguments
            format!("{},{}", function_name, arguments)
        });
}
