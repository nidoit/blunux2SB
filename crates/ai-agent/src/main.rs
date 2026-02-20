use std::io::Write as _;

mod agent;
mod config;
mod daemon;
mod error;
mod ipc;
mod memory;
mod providers;
mod setup;
mod strings;
mod tools;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use config::{AgentConfig, Language};
use memory::Memory;

#[derive(Parser)]
#[command(name = "blunux-ai", version, about = "Blunux AI Agent — natural language Linux system management")]
struct Cli {
    /// Path to blunux config.toml (for language detection)
    #[arg(long, default_value = "/usr/share/blunux/config.toml")]
    blunux_config: PathBuf,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start interactive AI chat
    Chat,
    /// First-time setup wizard
    Setup,
    /// Show agent status and configuration
    Status,
    /// Memory management
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// Run as background daemon (Unix socket, for WhatsApp bridge)
    Daemon,
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Show current memory contents
    Show,
    /// Clear daily logs and long-term memory
    Clear,
    /// Refresh SYSTEM.md with current system info
    Refresh,
}

fn detect_language(blunux_config_path: &PathBuf) -> Language {
    // Try loading blunux config for locale detection
    if let Ok(cfg) = blunux_config::BlunuxConfig::load(blunux_config_path) {
        return Language::from_locale(&cfg.locale.language);
    }
    // Fallback: check the main repo config.toml
    if let Ok(cfg) = blunux_config::BlunuxConfig::load(std::path::Path::new("config.toml")) {
        return Language::from_locale(&cfg.locale.language);
    }
    // Default: Korean for Blunux
    Language::Korean
}

fn run_status(config_dir: &PathBuf, lang: &Language) -> anyhow::Result<()> {
    match AgentConfig::load(config_dir) {
        Ok(cfg) => {
            let provider_name = match (&cfg.provider, &cfg.claude_mode) {
                (config::ProviderType::Claude, config::ClaudeMode::Api) => "Claude (API Mode)",
                (config::ProviderType::Claude, config::ClaudeMode::OAuth) => "Claude (OAuth Mode)",
                (config::ProviderType::DeepSeek, _) => "DeepSeek",
            };
            let lang_name = match cfg.language {
                Language::Korean => "한국어",
                Language::English => "English",
            };
            let safe_str = if cfg.safe_mode {
                match lang {
                    Language::Korean => "활성화",
                    Language::English => "Enabled",
                }
            } else {
                match lang {
                    Language::Korean => "비활성화",
                    Language::English => "Disabled",
                }
            };

            println!("\n  Blunux AI Agent Status\n");
            println!("  Provider:    {provider_name}");
            println!("  Model:       {}", cfg.model.display_name());
            println!("  Language:    {lang_name}");
            println!("  Safe Mode:   {safe_str}");
            println!("  Config:      {}\n", config_dir.display());

            // Memory stats
            let mem = Memory::new(config_dir.clone());
            let system = mem.load_system().unwrap_or_default();
            let user = mem.load_user().unwrap_or_default();
            let long_term = mem.load_long_term().unwrap_or_default();

            println!("  Memory:");
            println!(
                "    SYSTEM.md:  {} bytes",
                if system.is_empty() {
                    "empty".into()
                } else {
                    format!("{}", system.len())
                }
            );
            println!(
                "    USER.md:    {} bytes",
                if user.is_empty() {
                    "empty".into()
                } else {
                    format!("{}", user.len())
                }
            );
            println!(
                "    MEMORY.md:  {} bytes",
                if long_term.is_empty() {
                    "empty".into()
                } else {
                    format!("{}", long_term.len())
                }
            );

            let whatsapp_status = match lang {
                Language::Korean => "비활성화 (Phase 2 예정)",
                Language::English => "Disabled (Phase 2 — coming soon)",
            };
            println!("\n  WhatsApp:    {whatsapp_status}\n");
        }
        Err(_) => {
            let msg = match lang {
                Language::Korean => "설정 파일을 찾을 수 없습니다. 'blunux-ai setup'을 실행하세요.",
                Language::English => "Config not found. Please run 'blunux-ai setup'.",
            };
            println!("\n  {msg}\n");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let lang = detect_language(&cli.blunux_config);
    let config_dir = AgentConfig::default_config_dir();

    match cli.command {
        None | Some(Command::Chat) => {
            // Load config, start interactive chat
            match AgentConfig::load(&config_dir) {
                Ok(cfg) => {
                    let mut agent = agent::Agent::new(&cfg)?;
                    agent.run_interactive().await?;
                }
                Err(_) => {
                    let msg = match lang {
                        Language::Korean => {
                            "설정이 필요합니다. 'blunux-ai setup'을 먼저 실행하세요."
                        }
                        Language::English => "Setup required. Please run 'blunux-ai setup' first.",
                    };
                    println!("\n  {msg}\n");
                }
            }
        }
        Some(Command::Setup) => {
            let wizard = setup::SetupWizard::new(lang, config_dir);
            wizard.run()?;
        }
        Some(Command::Status) => {
            run_status(&config_dir, &lang)?;
        }
        Some(Command::Daemon) => {
            match AgentConfig::load(&config_dir) {
                Ok(cfg) => {
                    daemon::run_daemon(&cfg).await?;
                }
                Err(_) => {
                    let msg = match lang {
                        Language::Korean => {
                            "설정이 필요합니다. 'blunux-ai setup'을 먼저 실행하세요."
                        }
                        Language::English => "Setup required. Please run 'blunux-ai setup' first.",
                    };
                    println!("\n  {msg}\n");
                }
            }
        }
        Some(Command::Memory { action }) => {
            let mem = Memory::new(config_dir);
            match action {
                MemoryAction::Show => {
                    let output = mem.show_all().map_err(|e| anyhow::anyhow!("{e}"))?;
                    println!("{output}");
                }
                MemoryAction::Clear => {
                    let confirm_msg = match lang {
                        Language::Korean => {
                            "메모리를 초기화하시겠습니까? (일일 로그와 MEMORY.md가 삭제됩니다)"
                        }
                        Language::English => {
                            "Clear memory? (daily logs and MEMORY.md will be deleted)"
                        }
                    };
                    println!("  {confirm_msg}");
                    print!("  (y/n): ");
                    std::io::stdout().flush()?;

                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().to_lowercase().starts_with('y') {
                        mem.clear().map_err(|e| anyhow::anyhow!("{e}"))?;
                        let done = match lang {
                            Language::Korean => "메모리가 초기화되었습니다.",
                            Language::English => "Memory cleared.",
                        };
                        println!("  {done}");
                    } else {
                        println!("  {}", strings::cancelled(&lang));
                    }
                }
                MemoryAction::Refresh => {
                    mem.refresh_system_info()
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    let done = match lang {
                        Language::Korean => "SYSTEM.md가 업데이트되었습니다.",
                        Language::English => "SYSTEM.md refreshed.",
                    };
                    println!("  {done}");
                }
            }
        }
    }

    Ok(())
}
