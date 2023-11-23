use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value;
use std::collections::HashMap;

use super::{stock, stock::Stock};

// Define the top-level structure that holds an array of tools
#[derive(Serialize, Deserialize, Debug)]
pub struct Tools<'a> {
    #[serde(borrow)]
    pub tools: Vec<Tool<'a>>,
}

// Define the Tool struct with a type field and a function field
#[derive(Serialize, Deserialize, Debug)]
pub struct Tool<'a> {
    #[serde(rename = "type")]
    pub tool_type: &'a str,
    pub function: Function<'a>,
}

// Define the Function struct with name, description, and parameters fields
#[derive(Serialize, Deserialize, Debug)]
pub struct Function<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub parameters: Parameters<'a>,
}

// Define the Parameters struct with type, properties, and required fields
#[derive(Serialize, Deserialize, Debug)]
pub struct Parameters<'a> {
    #[serde(rename = "type")]
    pub parameters_type: &'a str,
    pub properties: HashMap<&'a str, Property<'a>>,
    pub required: Vec<&'a str>,
}

// Define the Property struct with type, description, and possibly an enum field
#[derive(Serialize, Deserialize, Debug)]
pub struct Property<'a> {
    #[serde(rename = "type")]
    pub property_type: &'a str,
    pub description: Option<&'a str>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<&'a str>>,
}

pub fn payload() -> Tools<'static> {
    Tools {
        tools: vec![stock::plugin()],
    }
}

pub async fn dispatch(cmd: String) -> Result<String, anyhow::Error> {
    let parts: Vec<&str> = cmd.splitn(2, ',').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid input format"));
    }
    let function_name = parts[0];
    match function_name {
        "get_stock_selection" => {
            let args = parts[1];
            let json_value: Value = serde_json::from_str(args).expect("Invalid JSON format");
            if let Some(date) = json_value["date"].as_str() {
                let mut stock = Stock::default();
                let selection = stock.selection(date).await?;
                Ok(selection)
            } else {
                Err(anyhow!("Missing date"))
            }
        }
        _ => Err(anyhow!("Invalid function name")),
    }
}
