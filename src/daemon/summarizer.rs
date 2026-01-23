use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn get_api_key() -> Option<String> {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        return Some(key);
    }

    if let Some(auth) = crate::config::load_auth_config() {
        return Some(auth.anthropic_api_key);
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

pub struct SummaryResult {
    pub goal: Option<String>,
    pub summary: String,
}

fn call_api(api_key: &str, prompt: &str, max_tokens: u32) -> Option<String> {
    let client = reqwest::blocking::Client::new();
    let request = ApiRequest {
        model: "claude-3-5-haiku-latest".to_string(),
        max_tokens,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
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

fn extract_messages(transcript_path: &str) -> Option<Vec<String>> {
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
                        let truncated: String = text.chars().take(200).collect();
                        // Skip very short messages (likely just confirmations)
                        if truncated.len() >= 10 {
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
                                        if truncated.len() >= 10 {
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
                        let truncated: String = text.chars().take(200).collect();
                        if truncated.len() >= 10 {
                            messages.push(format!("Assistant: {}", truncated));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

pub fn summarize_session(
    transcript_path: &str,
    cached_goal: Option<&str>,
) -> Option<SummaryResult> {
    let api_key = get_api_key()?;
    let messages = extract_messages(transcript_path)?;

    // Short sessions (< 5 messages): simple summary, no goal caching
    if messages.len() < 5 {
        let context = messages.join("\n");
        let prompt = format!(
            "What is this conversation about? Reply with ONLY a short phrase (3-8 words).\n\n{}",
            context
        );
        let summary = call_api(&api_key, &prompt, 30)?;
        return Some(SummaryResult {
            goal: None,
            summary,
        });
    }

    // Build context: first 5 messages + last 10 messages
    let context = if messages.len() <= 15 {
        messages.join("\n")
    } else {
        let first: Vec<_> = messages.iter().take(5).cloned().collect();
        let last: Vec<_> = messages.iter().rev().take(10).rev().cloned().collect();
        format!("{}\n...\n{}", first.join("\n"), last.join("\n"))
    };

    // If we have a cached goal, only ask for current status
    if let Some(goal) = cached_goal {
        let last_context: String = messages
            .iter()
            .rev()
            .take(5)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!(
            "Conversation excerpt:\n---\n{}\n---\n\n\
             The overall task is \"{}\". Based on the last Assistant message, what is the current activity?\n\
             Reply with ONLY a brief phrase (3-8 words).",
            last_context, goal
        );
        let status = call_api(&api_key, &prompt, 40)?;
        return Some(SummaryResult {
            goal: None, // Don't update goal
            summary: format!("{}: {}", goal, status),
        });
    }

    // First time: extract both goal and status
    let prompt = format!(
        "Conversation excerpt:\n---\n{}\n---\n\n\
         Summarize as: Goal: current activity\n\
         - Goal = overall task (2-4 words)\n\
         - Current activity = from the LAST assistant message\n\
         Example: Auth system: Running login tests\n\
         Reply with ONLY that one line.",
        context
    );

    let summary = call_api(&api_key, &prompt, 60)?;

    // Extract goal from the summary (everything before the first colon)
    // If no colon, use the whole summary as the goal to prevent re-extraction loops
    let goal = summary
        .split_once(':')
        .map(|(g, _)| g.trim().to_string())
        .unwrap_or_else(|| summary.clone());

    Some(SummaryResult {
        goal: Some(goal),
        summary,
    })
}
