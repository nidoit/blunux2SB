pub mod claude;
pub mod deepseek;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::{AgentConfig, ClaudeMode, ProviderType};
use crate::error::{ConfigError, ProviderError};
use crate::tools::ToolDefinition;

pub use claude::{ClaudeApiProvider, ClaudeOAuthProvider};
pub use deepseek::DeepSeekProvider;

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text { text: text.into() }],
        }
    }

    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::Text { text: text.into() }],
        }
    }

    pub fn tool_results(results: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::User,
            content: results,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: Usage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
}

#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl CompletionResult {
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn tool_uses(&self) -> Vec<(&str, &str, &serde_json::Value)> {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.as_str(), name.as_str(), input))
                }
                _ => None,
            })
            .collect()
    }

    pub fn has_tool_use(&self) -> bool {
        self.content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
    }
}

// ── Provider trait ───────────────────────────────────────────────────────────

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;

    async fn complete(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
        max_tokens: u32,
    ) -> Result<CompletionResult, ProviderError>;
}

// ── Factory ──────────────────────────────────────────────────────────────────

pub fn build_provider(config: &AgentConfig) -> Result<Box<dyn Provider>, ConfigError> {
    match (&config.provider, &config.claude_mode) {
        (ProviderType::Claude, ClaudeMode::Api) => {
            let key_path = config.config_dir.join("credentials/claude");
            let api_key = crate::config::load_credential(&key_path)?;
            Ok(Box::new(ClaudeApiProvider::new(
                api_key,
                config.model.clone(),
            )))
        }
        (ProviderType::Claude, ClaudeMode::OAuth) => {
            Ok(Box::new(ClaudeOAuthProvider::new(config.model.clone())))
        }
        (ProviderType::DeepSeek, _) => {
            let key_path = config.config_dir.join("credentials/deepseek");
            let api_key = crate::config::load_credential(&key_path)?;
            Ok(Box::new(DeepSeekProvider::new(
                api_key,
                config.model.clone(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_user() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content.len(), 1);
    }

    #[test]
    fn test_completion_result_text() {
        let result = CompletionResult {
            content: vec![
                ContentBlock::Text {
                    text: "Hello".into(),
                },
                ContentBlock::Text {
                    text: "World".into(),
                },
            ],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        };
        assert_eq!(result.text(), "Hello\nWorld");
    }

    #[test]
    fn test_completion_result_has_tool_use() {
        let with_tool = CompletionResult {
            content: vec![ContentBlock::ToolUse {
                id: "1".into(),
                name: "test".into(),
                input: serde_json::json!({}),
            }],
            stop_reason: StopReason::ToolUse,
            usage: Usage::default(),
        };
        assert!(with_tool.has_tool_use());

        let without_tool = CompletionResult {
            content: vec![ContentBlock::Text {
                text: "hi".into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        };
        assert!(!without_tool.has_tool_use());
    }
}
