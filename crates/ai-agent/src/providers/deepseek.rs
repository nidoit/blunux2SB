use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::ModelId;
use crate::error::ProviderError;
use crate::providers::{
    CompletionResult, ContentBlock, Message, Provider, Role, StopReason, ToolDefinition, Usage,
};

pub struct DeepSeekProvider {
    client: reqwest::Client,
    api_key: String,
    model: ModelId,
}

impl DeepSeekProvider {
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

// ── OpenAI-compatible request/response types ─────────────────────────────────

#[derive(Serialize)]
struct OpenAIRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAIMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIChoiceMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIChoiceMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Deserialize)]
struct OpenAIToolCall {
    id: String,
    function: OpenAIFunctionCall,
}

#[derive(Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAIError {
    error: OpenAIErrorBody,
}

#[derive(Deserialize)]
struct OpenAIErrorBody {
    message: String,
}

fn convert_messages(system_prompt: &str, messages: &[Message]) -> Vec<OpenAIMessage> {
    let mut out = vec![OpenAIMessage {
        role: "system".into(),
        content: system_prompt.into(),
    }];

    for msg in messages {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        // Flatten content blocks into a single string
        let text: String = msg
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                ContentBlock::ToolResult { content, .. } => Some(content.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            out.push(OpenAIMessage {
                role: role.into(),
                content: text,
            });
        }
    }
    out
}

fn convert_tools(tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
    tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                }
            })
        })
        .collect()
}

#[async_trait]
impl Provider for DeepSeekProvider {
    fn name(&self) -> &str {
        "DeepSeek"
    }

    async fn complete(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
        max_tokens: u32,
    ) -> Result<CompletionResult, ProviderError> {
        let body = OpenAIRequest {
            model: self.model.api_name(),
            messages: convert_messages(system_prompt, messages),
            max_tokens,
            tools: convert_tools(tools),
        };

        let resp = self
            .client
            .post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
            let message = serde_json::from_str::<OpenAIError>(&text)
                .map(|e| e.error.message)
                .unwrap_or(text);
            return Err(ProviderError::ApiError { status, message });
        }

        let api_resp: OpenAIResponse =
            resp.json().await.map_err(|e| ProviderError::Parse(e.to_string()))?;

        let choice = api_resp
            .choices
            .into_iter()
            .next()
            .ok_or(ProviderError::EmptyResponse)?;

        let mut content = Vec::new();

        if let Some(text) = choice.message.content {
            if !text.is_empty() {
                content.push(ContentBlock::Text { text });
            }
        }

        let mut has_tools = false;
        if let Some(tool_calls) = choice.message.tool_calls {
            for tc in tool_calls {
                has_tools = true;
                let input: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));
                content.push(ContentBlock::ToolUse {
                    id: tc.id,
                    name: tc.function.name,
                    input,
                });
            }
        }

        let stop_reason = if has_tools {
            StopReason::ToolUse
        } else {
            match choice.finish_reason.as_deref() {
                Some("length") => StopReason::MaxTokens,
                _ => StopReason::EndTurn,
            }
        };

        let usage = api_resp.usage.map_or(Usage::default(), |u| Usage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        });

        Ok(CompletionResult {
            content,
            stop_reason,
            usage,
        })
    }
}
