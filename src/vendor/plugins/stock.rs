use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Asia::Taipei;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use super::tool::*;

static PROMPT: &str = r#"[Play as professional investor role][DO NOT respond with code advice] with json stock data provided:
DO NOT share the analysis stratey, just sharing the result.
1. Top 3 categories on the day (count by category in json)
2. Top 5 stocks that most concentrate (sum concentration1,concentration5,concentration10,concentration20,concentration60,foreign,foreign10,trust,trust10 as rank, sort by rank & quoteChange desc).
3. List the candlestick chart link with pure text, replace {stockID} with the top 5 selection with exact this url:
https://stock.wearn.com/finance_chart.asp?stockid={stockID}&timekind=0&timeblock=120&sma1=8&sma2=21&sma3=55&volume=1
"#;

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    success: bool,
    #[serde(rename = "errorMessage")]
    error_message: Option<String>,
    #[serde(rename = "accessToken")]
    access_token: String,
}

#[derive(Debug, Serialize)]
struct SelectionRequest<'a> {
    date: &'a str,
    strict: bool,
}

pub struct Stock {
    access_token: String,
    token_expires: DateTime<Tz>,
    default_timeout: std::time::Duration,
    client: reqwest::Client,
}

impl Default for Stock {
    fn default() -> Self {
        Stock {
            access_token: String::from(""),
            token_expires: Utc::now().with_timezone(&Taipei),
            default_timeout: std::time::Duration::from_secs(5),
            client: reqwest::Client::builder()
                .build()
                .expect("Failed to create Client for Stock"),
        }
    }
}

impl Stock {
    async fn authn(&mut self) -> Result<()> {
        let email = &std::env::var("PLUGIN_STOCK_USER").unwrap();
        let password = &std::env::var("PLUGIN_STOCK_PASSWD").unwrap();
        let json_payload = json!(LoginRequest { email, password });

        let response = self
            .client
            .post("https://api.jarvis-stock.tw/v1/login")
            .header("Content-Type", "application/json")
            .timeout(self.default_timeout)
            .json(&json_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            // If the response status is not successful, return an error early.
            return Err(anyhow!("Failed to authenticate: {}", response.status()));
        }

        // Attempt to deserialize the response body into `LoginResponse`.
        let login_response: LoginResponse = response.json().await?;
        if !login_response.success {
            // Return an error if authentication was not successful.
            return Err(anyhow!(
                "Failed to authenticate: {}",
                login_response
                    .error_message
                    .unwrap_or("Unknown".to_string()),
            ));
        }

        // If authentication was successful, set the access token.
        self.access_token = login_response.access_token;

        // Convert the current UTC date and time to Taiwan timezone
        let taiwan_now = Utc::now().with_timezone(&Taipei);
        self.token_expires = taiwan_now + chrono::Duration::days(1); // expire 1 day

        Ok(())
    }

    pub async fn selection(&mut self, date: &str) -> Result<String> {
        if self.access_token.is_empty() || Utc::now().with_timezone(&Taipei) > self.token_expires {
            self.authn().await?;
        }

        let json_payload = json!(SelectionRequest {
            date,
            strict: false,
        });

        let response = self
            .client
            .post("https://api.jarvis-stock.tw/v1/selections")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .timeout(self.default_timeout)
            .json(&json_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            // If the response status is not successful, return an error early.
            return Err(anyhow!(
                "Failed fetch stock_selection: {}",
                response.status()
            ));
        }

        let res = response.text().await?;
        Ok(format!("{}\n{}", PROMPT, res))
    }
}

pub fn plugin() -> Tool<'static> {
    Tool {
        tool_type: "function",
        function: Function {
            name: "get_stock_selection",
            description: "Get the stock selection in a given date",
            parameters: Parameters {
                parameters_type: "object",
                properties: {
                    let mut props = HashMap::new();
                    props.insert(
                        "date",
                        Property {
                            property_type: "string",
                            description: Some("The date format, e.g. 20231101"),
                            enum_values: None,
                        },
                    );
                    props
                },
                required: vec!["date"],
            },
        },
    }
}
