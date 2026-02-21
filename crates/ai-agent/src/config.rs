use std::path::{Path, PathBuf};

use crate::error::ConfigError;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub provider: ProviderType,
    pub claude_mode: ClaudeMode,
    pub model: ModelId,
    pub whatsapp_enabled: bool,
    pub language: Language,
    pub safe_mode: bool,
    pub config_dir: PathBuf,
    pub whatsapp: WhatsAppConfig,
}

#[derive(Debug, Clone)]
pub struct WhatsAppConfig {
    /// Phone numbers allowed to send commands. Format: "+821012345678"
    pub allowed_numbers: Vec<String>,
    /// Max messages per minute per user (rate limiting).
    pub max_messages_per_minute: u32,
    /// If true, messages must start with "/ai " to be treated as commands.
    /// Useful for group chats where the agent should not respond to everything.
    pub require_prefix: bool,
    /// Seconds of inactivity before a user's conversation is reset.
    /// Default: 3600 (1 hour).
    pub session_timeout: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderType {
    Claude,
    DeepSeek,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClaudeMode {
    Api,
    OAuth,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelId {
    ClaudeSonnet46,
    ClaudeOpus46,
    DeepSeekChat,
    DeepSeekCoder,
}

impl ModelId {
    pub fn api_name(&self) -> &'static str {
        match self {
            Self::ClaudeSonnet46 => "claude-sonnet-4-6",
            Self::ClaudeOpus46 => "claude-opus-4-6",
            Self::DeepSeekChat => "deepseek-chat",
            Self::DeepSeekCoder => "deepseek-coder",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ClaudeSonnet46 => "Claude Sonnet 4.6",
            Self::ClaudeOpus46 => "Claude Opus 4.6",
            Self::DeepSeekChat => "DeepSeek Chat",
            Self::DeepSeekCoder => "DeepSeek Coder",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    Korean,
    English,
}

impl Language {
    pub fn from_locale(languages: &[String]) -> Self {
        if languages.iter().any(|l| l.starts_with("ko")) {
            Self::Korean
        } else {
            Self::English
        }
    }
}

impl AgentConfig {
    pub fn default_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("blunux-ai")
    }

    /// Load AgentConfig from the agent's own config.toml inside config_dir.
    pub fn load(config_dir: &Path) -> Result<Self, ConfigError> {
        let config_path = config_dir.join("config.toml");
        if !config_path.exists() {
            return Err(ConfigError::NotFound {
                path: config_path.display().to_string(),
            });
        }
        let content = std::fs::read_to_string(&config_path).map_err(ConfigError::Io)?;
        let table: toml::Table =
            toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;

        let agent = table
            .get("agent")
            .ok_or_else(|| ConfigError::MissingField {
                field: "agent".into(),
            })?;

        let provider_str = agent
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("claude");
        let provider = match provider_str {
            "claude" => ProviderType::Claude,
            "deepseek" => ProviderType::DeepSeek,
            other => {
                return Err(ConfigError::InvalidValue {
                    field: "provider".into(),
                    value: other.into(),
                })
            }
        };

        let claude_mode_str = agent
            .get("claude_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("oauth");
        let claude_mode = match claude_mode_str {
            "oauth" => ClaudeMode::OAuth,
            "api" => ClaudeMode::Api,
            other => {
                return Err(ConfigError::InvalidValue {
                    field: "claude_mode".into(),
                    value: other.into(),
                })
            }
        };

        let model_str = agent
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("claude-sonnet-4-6");
        let model = match model_str {
            "claude-sonnet-4-6" => ModelId::ClaudeSonnet46,
            "claude-opus-4-6" => ModelId::ClaudeOpus46,
            "deepseek-chat" => ModelId::DeepSeekChat,
            "deepseek-coder" => ModelId::DeepSeekCoder,
            other => {
                return Err(ConfigError::InvalidValue {
                    field: "model".into(),
                    value: other.into(),
                })
            }
        };

        let language_str = agent
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("auto");
        let language = match language_str {
            "ko" => Language::Korean,
            "en" => Language::English,
            _ => Language::Korean, // default for Blunux
        };

        let safe_mode = agent
            .get("safe_mode")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let whatsapp_enabled = agent
            .get("whatsapp_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // [whatsapp] section — optional, defaults to empty
        let wa_section = table.get("whatsapp");
        let allowed_numbers: Vec<String> = wa_section
            .and_then(|s| s.get("allowed_numbers"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let max_messages_per_minute = wa_section
            .and_then(|s| s.get("max_messages_per_minute"))
            .and_then(|v| v.as_integer())
            .map(|v| v as u32)
            .unwrap_or(5);
        let require_prefix = wa_section
            .and_then(|s| s.get("require_prefix"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let session_timeout = wa_section
            .and_then(|s| s.get("session_timeout"))
            .and_then(|v| v.as_integer())
            .map(|v| v as u32)
            .unwrap_or(3600);

        Ok(Self {
            provider,
            claude_mode,
            model,
            whatsapp_enabled,
            language,
            safe_mode,
            config_dir: config_dir.to_path_buf(),
            whatsapp: WhatsAppConfig {
                allowed_numbers,
                max_messages_per_minute,
                require_prefix,
                session_timeout,
            },
        })
    }

    /// Save the current config to config_dir/config.toml.
    pub fn save(&self) -> Result<(), ConfigError> {
        std::fs::create_dir_all(&self.config_dir).map_err(ConfigError::Io)?;
        let provider_str = match self.provider {
            ProviderType::Claude => "claude",
            ProviderType::DeepSeek => "deepseek",
        };
        let claude_mode_str = match self.claude_mode {
            ClaudeMode::Api => "api",
            ClaudeMode::OAuth => "oauth",
        };
        let language_str = match self.language {
            Language::Korean => "ko",
            Language::English => "en",
        };
        let allowed_numbers_toml = self
            .whatsapp
            .allowed_numbers
            .iter()
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let content = format!(
            r#"[agent]
provider = "{provider_str}"
claude_mode = "{claude_mode_str}"
model = "{model}"
language = "{language_str}"
safe_mode = {safe_mode}
whatsapp_enabled = {whatsapp}

[whatsapp]
allowed_numbers = [{allowed_numbers_toml}]
max_messages_per_minute = {max_mpm}
require_prefix = {require_prefix}
session_timeout = {session_timeout}
"#,
            model = self.model.api_name(),
            safe_mode = self.safe_mode,
            whatsapp = self.whatsapp_enabled,
            max_mpm = self.whatsapp.max_messages_per_minute,
            require_prefix = self.whatsapp.require_prefix,
            session_timeout = self.whatsapp.session_timeout,
        );
        let path = self.config_dir.join("config.toml");
        std::fs::write(&path, content).map_err(ConfigError::Io)?;
        Ok(())
    }
}

/// Load an API key from a credential file (single line, trimmed).
pub fn load_credential(path: &Path) -> Result<String, ConfigError> {
    if !path.exists() {
        return Err(ConfigError::NotFound {
            path: path.display().to_string(),
        });
    }
    let content = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
    let key = content.trim().to_string();
    if key.is_empty() {
        return Err(ConfigError::MissingField {
            field: format!("credential at {}", path.display()),
        });
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_locale_korean() {
        let langs = vec!["ko_KR".to_string()];
        assert_eq!(Language::from_locale(&langs), Language::Korean);
    }

    #[test]
    fn test_language_from_locale_english() {
        let langs = vec!["en_US".to_string()];
        assert_eq!(Language::from_locale(&langs), Language::English);
    }

    #[test]
    fn test_language_from_locale_empty() {
        let langs: Vec<String> = vec![];
        assert_eq!(Language::from_locale(&langs), Language::English);
    }

    #[test]
    fn test_model_id_api_name() {
        assert_eq!(ModelId::ClaudeSonnet46.api_name(), "claude-sonnet-4-6");
        assert_eq!(ModelId::ClaudeOpus46.api_name(), "claude-opus-4-6");
        assert_eq!(ModelId::DeepSeekChat.api_name(), "deepseek-chat");
        assert_eq!(ModelId::DeepSeekCoder.api_name(), "deepseek-coder");
    }

    #[test]
    fn test_config_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = AgentConfig {
            provider: ProviderType::Claude,
            claude_mode: ClaudeMode::OAuth,
            model: ModelId::ClaudeSonnet46,
            whatsapp_enabled: false,
            language: Language::Korean,
            safe_mode: true,
            config_dir: tmp.path().to_path_buf(),
            whatsapp: WhatsAppConfig {
                allowed_numbers: vec![],
                max_messages_per_minute: 5,
                require_prefix: false,
                session_timeout: 3600,
            },
        };
        cfg.save().unwrap();
        let loaded = AgentConfig::load(tmp.path()).unwrap();
        assert_eq!(loaded.provider, ProviderType::Claude);
        assert_eq!(loaded.claude_mode, ClaudeMode::OAuth);
        assert_eq!(loaded.model, ModelId::ClaudeSonnet46);
        assert_eq!(loaded.language, Language::Korean);
        assert!(loaded.safe_mode);
    }
}
