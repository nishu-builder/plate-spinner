use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn get_api_key() -> Option<String> {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        return Some(key);
    }

    let config_path = dirs::home_dir()?.join(".plate-spinner").join("config");
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            for line in content.lines() {
                if let Some(key) = line.strip_prefix("ANTHROPIC_API_KEY=") {
                    return Some(key.trim().to_string());
                }
            }
        }
    }
    None
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

pub fn summarize_session(transcript_path: &str) -> Option<String> {
    let api_key = get_api_key()?;
    let path = Path::new(transcript_path);
    if !path.exists() {
        return None;
    }

    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let msg = entry.get("message").unwrap_or(&serde_json::Value::Null);
            let content = msg.get("content");

            match entry_type {
                "user" => {
                    if let Some(text) = content.and_then(|c| c.as_str()) {
                        if !text.is_empty() {
                            let truncated: String = text.chars().take(200).collect();
                            messages.push(format!("User: {}", truncated));
                        }
                    }
                }
                "assistant" => {
                    if let Some(arr) = content.and_then(|c| c.as_array()) {
                        for block in arr.iter().take(3) {
                            let block_type = block.get("type").and_then(|v| v.as_str());
                            match block_type {
                                Some("text") => {
                                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                        let truncated: String = text.chars().take(200).collect();
                                        if !truncated.is_empty() {
                                            messages.push(format!("Assistant: {}", truncated));
                                        }
                                    }
                                }
                                Some("tool_use") => {
                                    let name = block
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown");
                                    messages.push(format!("Tool: {}", name));
                                }
                                _ => {}
                            }
                        }
                    } else if let Some(text) = content.and_then(|c| c.as_str()) {
                        if !text.is_empty() {
                            let truncated: String = text.chars().take(200).collect();
                            messages.push(format!("Assistant: {}", truncated));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if messages.is_empty() {
        return None;
    }

    let context: String = messages
        .iter()
        .rev()
        .take(15)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    let client = reqwest::blocking::Client::new();
    let request = ApiRequest {
        model: "claude-3-5-haiku-latest".to_string(),
        max_tokens: 30,
        messages: vec![Message {
            role: "user".to_string(),
            content: format!(
                "What is this conversation about? Reply with ONLY a 3-8 word phrase, nothing else.\n\n{}",
                context
            ),
        }],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .ok()?;

    let api_response: ApiResponse = response.json().ok()?;
    api_response
        .content
        .first()
        .and_then(|block| block.text.clone())
        .map(|s| s.trim().to_string())
}
