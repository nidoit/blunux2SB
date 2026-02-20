use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::ModelId;
use crate::error::ProviderError;
use crate::providers::{
    CompletionResult, ContentBlock, Message, Provider, Role, StopReason, ToolDefinition, Usage,
};

// ── Claude API Provider (Mode A: direct HTTP) ───────────────────────────────

pub struct ClaudeApiProvider {
    client: reqwest::Client,
    api_key: String,
    model: ModelId,
}

impl ClaudeApiProvider {
    pub fn new(api_key: String, model: ModelId) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            api_key,
            model,
        }
    }
}

#[derive(Serialize)]
struct ClaudeApiRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<ClaudeApiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct ClaudeApiMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Deserialize)]
struct ClaudeApiResponse {
    content: Vec<ClaudeApiContentBlock>,
    stop_reason: String,
    usage: ClaudeApiUsage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ClaudeApiContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Deserialize)]
struct ClaudeApiUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct ClaudeApiError {
    error: ClaudeApiErrorBody,
}

#[derive(Deserialize)]
struct ClaudeApiErrorBody {
    message: String,
}

fn convert_messages(messages: &[Message]) -> Vec<ClaudeApiMessage> {
    messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };
            let content: Vec<serde_json::Value> = msg
                .content
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => {
                        serde_json::json!({"type": "text", "text": text})
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        serde_json::json!({"type": "tool_use", "id": id, "name": name, "input": input})
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } => {
                        serde_json::json!({"type": "tool_result", "tool_use_id": tool_use_id, "content": content, "is_error": is_error})
                    }
                })
                .collect();
            ClaudeApiMessage {
                role: role.to_string(),
                content: serde_json::Value::Array(content),
            }
        })
        .collect()
}

fn convert_tools(tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
    tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect()
}

#[async_trait]
impl Provider for ClaudeApiProvider {
    fn name(&self) -> &str {
        "Claude API"
    }

    async fn complete(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
        max_tokens: u32,
    ) -> Result<CompletionResult, ProviderError> {
        let body = ClaudeApiRequest {
            model: self.model.api_name(),
            max_tokens,
            system: system_prompt,
            messages: convert_messages(messages),
            tools: convert_tools(tools),
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status().as_u16();
        if status == 401 {
            return Err(ProviderError::AuthenticationFailed);
        }
        if status == 429 {
            return Err(ProviderError::RateLimit {
                retry_after_secs: 60,
            });
        }
        if status >= 400 {
            let text = resp.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ClaudeApiError>(&text)
                .map(|e| e.error.message)
                .unwrap_or(text);
            return Err(ProviderError::ApiError { status, message });
        }

        let api_resp: ClaudeApiResponse =
            resp.json().await.map_err(|e| ProviderError::Parse(e.to_string()))?;

        let content = api_resp
            .content
            .into_iter()
            .map(|b| match b {
                ClaudeApiContentBlock::Text { text } => ContentBlock::Text { text },
                ClaudeApiContentBlock::ToolUse { id, name, input } => {
                    ContentBlock::ToolUse { id, name, input }
                }
            })
            .collect();

        let stop_reason = match api_resp.stop_reason.as_str() {
            "tool_use" => StopReason::ToolUse,
            "max_tokens" => StopReason::MaxTokens,
            _ => StopReason::EndTurn,
        };

        Ok(CompletionResult {
            content,
            stop_reason,
            usage: Usage {
                input_tokens: api_resp.usage.input_tokens,
                output_tokens: api_resp.usage.output_tokens,
            },
        })
    }
}

// ── Claude OAuth Provider (Mode B: subprocess) ──────────────────────────────

pub struct ClaudeOAuthProvider {
    model: ModelId,
}

impl ClaudeOAuthProvider {
    pub fn new(model: ModelId) -> Self {
        Self { model }
    }
}

/// Flatten multi-turn conversation into a single prompt string for the CLI.
fn flatten_conversation(system: &str, messages: &[Message]) -> String {
    let mut prompt = String::new();
    prompt.push_str("[System]\n");
    prompt.push_str(system);
    prompt.push_str("\n\n");
    for msg in messages {
        let role_label = match msg.role {
            Role::User => "[User]",
            Role::Assistant => "[Assistant]",
        };
        prompt.push_str(role_label);
        prompt.push('\n');
        for block in &msg.content {
            match block {
                ContentBlock::Text { text } => {
                    prompt.push_str(text);
                    prompt.push('\n');
                }
                ContentBlock::ToolUse { name, input, .. } => {
                    prompt.push_str(&format!("[Tool call: {name}({input})]\n"));
                }
                ContentBlock::ToolResult { content, .. } => {
                    prompt.push_str(&format!("[Tool result: {content}]\n"));
                }
            }
        }
        prompt.push('\n');
    }
    prompt
}

#[async_trait]
impl Provider for ClaudeOAuthProvider {
    fn name(&self) -> &str {
        "Claude OAuth"
    }

    async fn complete(
        &self,
        system_prompt: &str,
        messages: &[Message],
        _tools: &[ToolDefinition],
        _max_tokens: u32,
    ) -> Result<CompletionResult, ProviderError> {
        let prompt = flatten_conversation(system_prompt, messages);

        let output = tokio::process::Command::new("claude")
            .arg("-p")
            .arg(&prompt)
            .arg("--output-format")
            .arg("text")
            .arg("--model")
            .arg(self.model.api_name())
            .output()
            .await
            .map_err(|e| ProviderError::SubprocessError {
                exit_code: -1,
                stderr: format!("Failed to spawn claude CLI: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ProviderError::SubprocessError {
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Err(ProviderError::EmptyResponse);
        }

        // OAuth mode does not support tool use — always EndTurn with text.
        Ok(CompletionResult {
            content: vec![ContentBlock::Text { text }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        })
    }
}
