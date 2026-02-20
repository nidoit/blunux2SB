use std::path::PathBuf;

use dialoguer::{Input, Password, Select};

use crate::config::{AgentConfig, ClaudeMode, Language, ModelId, ProviderType, WhatsAppConfig};
use crate::error::AgentError;
use crate::memory::Memory;
use crate::strings;

pub struct SetupWizard {
    lang: Language,
    config_dir: PathBuf,
}

impl SetupWizard {
    pub fn new(lang: Language, config_dir: PathBuf) -> Self {
        Self { lang, config_dir }
    }

    pub fn run(&self) -> Result<AgentConfig, AgentError> {
        // Banner
        println!("\n{}", "=".repeat(44));
        println!("    {}", strings::setup_welcome(&self.lang));
        println!("{}\n", "=".repeat(44));

        // Step 1: Provider
        let provider = self.select_provider()?;

        // Step 2: Claude mode (only if Claude)
        let claude_mode = if provider == ProviderType::Claude {
            self.select_claude_mode()?
        } else {
            ClaudeMode::Api // irrelevant for DeepSeek
        };

        // Step 3: API key (if needed)
        match (&provider, &claude_mode) {
            (ProviderType::Claude, ClaudeMode::OAuth) => {
                self.setup_claude_oauth()?;
            }
            (ProviderType::Claude, ClaudeMode::Api) => {
                self.setup_api_key("claude")?;
            }
            (ProviderType::DeepSeek, _) => {
                self.setup_api_key("deepseek")?;
            }
        }

        // Step 4: Model
        let model = self.select_model(&provider)?;

        // Step 5: WhatsApp (Phase 2 notice)
        println!("\n  {}", strings::setup_whatsapp_coming_soon(&self.lang));

        // Step 6: Build and save config
        let config = AgentConfig {
            provider,
            claude_mode,
            model,
            whatsapp_enabled: false,
            language: self.lang.clone(),
            safe_mode: true,
            config_dir: self.config_dir.clone(),
            whatsapp: WhatsAppConfig {
                allowed_numbers: vec![],
                max_messages_per_minute: 5,
            },
        };
        config.save().map_err(AgentError::Config)?;

        // Step 7: Initialize memory
        let memory = Memory::new(self.config_dir.clone());
        memory.init_dirs().map_err(AgentError::Memory)?;
        memory.refresh_system_info().map_err(AgentError::Memory)?;

        // Create empty USER.md and MEMORY.md if they don't exist
        let user_path = self.config_dir.join("memory/USER.md");
        if !user_path.exists() {
            std::fs::write(&user_path, "").map_err(AgentError::Io)?;
        }
        let mem_path = self.config_dir.join("memory/MEMORY.md");
        if !mem_path.exists() {
            std::fs::write(&mem_path, "").map_err(AgentError::Io)?;
        }

        // Done
        println!("\n  {}\n", strings::setup_done(&self.lang));

        Ok(config)
    }

    fn select_provider(&self) -> Result<ProviderType, AgentError> {
        let items = vec![
            "Claude (Anthropic) — Recommended",
            "DeepSeek — Alternative",
        ];
        let selection = Select::new()
            .with_prompt(strings::setup_provider_prompt(&self.lang))
            .items(&items)
            .default(0)
            .interact()
            .map_err(|_| AgentError::UserCancelled)?;

        Ok(match selection {
            0 => ProviderType::Claude,
            _ => ProviderType::DeepSeek,
        })
    }

    fn select_claude_mode(&self) -> Result<ClaudeMode, AgentError> {
        let items = vec![
            "OAuth — Claude Pro/Max subscription (no API key needed)",
            "API Key — Direct HTTP (pay per token)",
        ];
        let selection = Select::new()
            .with_prompt(strings::setup_claude_mode_prompt(&self.lang))
            .items(&items)
            .default(0)
            .interact()
            .map_err(|_| AgentError::UserCancelled)?;

        Ok(match selection {
            0 => ClaudeMode::OAuth,
            _ => ClaudeMode::Api,
        })
    }

    fn select_model(&self, provider: &ProviderType) -> Result<ModelId, AgentError> {
        let (items, models) = match provider {
            ProviderType::Claude => (
                vec![
                    "claude-sonnet-4-6 — Fast & balanced (Recommended)",
                    "claude-opus-4-6 — More capable, slower",
                ],
                vec![ModelId::ClaudeSonnet46, ModelId::ClaudeOpus46],
            ),
            ProviderType::DeepSeek => (
                vec![
                    "deepseek-chat — General purpose (Recommended)",
                    "deepseek-coder — Code-focused",
                ],
                vec![ModelId::DeepSeekChat, ModelId::DeepSeekCoder],
            ),
        };

        let selection = Select::new()
            .with_prompt(strings::setup_model_prompt(&self.lang))
            .items(&items)
            .default(0)
            .interact()
            .map_err(|_| AgentError::UserCancelled)?;

        Ok(models[selection].clone())
    }

    fn setup_claude_oauth(&self) -> Result<(), AgentError> {
        // Check if claude CLI is installed
        let claude_check = std::process::Command::new("which")
            .arg("claude")
            .output();

        match claude_check {
            Ok(output) if output.status.success() => {
                println!("  Claude CLI found.");
            }
            _ => {
                println!("  Claude CLI not found. Please install it:");
                println!("    npm install -g @anthropic-ai/claude-code");
                println!("  Then run: claude login");
                println!();

                // Try to install
                let msg = match self.lang {
                    Language::Korean => "Claude CLI를 지금 설치하시겠습니까?",
                    Language::English => "Install Claude CLI now?",
                };
                let install: String = Input::new()
                    .with_prompt(format!("{msg} (y/n)"))
                    .default("y".into())
                    .interact_text()
                    .map_err(|_| AgentError::UserCancelled)?;

                if install.starts_with('y') || install.starts_with('Y') {
                    println!("  Installing Claude CLI...");
                    let result = std::process::Command::new("npm")
                        .args(["install", "-g", "@anthropic-ai/claude-code"])
                        .status();

                    match result {
                        Ok(s) if s.success() => println!("  Claude CLI installed."),
                        _ => {
                            println!("  Failed to install. Please install manually.");
                        }
                    }
                }
            }
        }

        println!("  Please ensure you are logged in: claude login");
        Ok(())
    }

    fn setup_api_key(&self, provider_name: &str) -> Result<(), AgentError> {
        let key: String = Password::new()
            .with_prompt(strings::setup_api_key_prompt(&self.lang))
            .interact()
            .map_err(|_| AgentError::UserCancelled)?;

        if key.trim().is_empty() {
            return Err(AgentError::Config(crate::error::ConfigError::MissingField {
                field: "API key".into(),
            }));
        }

        // Save credential
        let cred_dir = self.config_dir.join("credentials");
        std::fs::create_dir_all(&cred_dir).map_err(AgentError::Io)?;
        let cred_path = cred_dir.join(provider_name);
        std::fs::write(&cred_path, key.trim()).map_err(AgentError::Io)?;

        // Set file permissions to 600
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cred_path, std::fs::Permissions::from_mode(0o600))
                .map_err(AgentError::Io)?;
        }

        let msg = match self.lang {
            Language::Korean => "API 키가 저장되었습니다.",
            Language::English => "API key saved.",
        };
        println!("  {msg}");
        Ok(())
    }
}
