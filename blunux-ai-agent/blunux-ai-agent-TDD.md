# TDD: Blunux AI Agent — Technical Design Document

- **Project:** Blunux AI Agent
- **Version:** 1.0.0 (Phase 1 design — all phases implemented)
- **Date:** 2026-02-21
- **Author:** Jaewoo Joung (정재우)
- **Parent PRD:** blunux-ai-agent-PRD.md v1.0.0
- **License:** MIT

---

## 1. Overview

This document defines the Rust technical design for the `crates/ai-agent` crate. It covers module structure, trait interfaces, data types, error hierarchy, CLI commands, and test plan. All five development phases are now complete: the WhatsApp bridge, daemon mode, and automation scheduler (Phases 2–4) are fully implemented; AUR packaging and ISO integration (Phase 5) are shipped.

**Binary name:** `blunux-ai`
**Crate name:** `ai-agent`

---

## 2. Module Structure

```
crates/ai-agent/
├── Cargo.toml
└── src/
    ├── main.rs          # CLI entry point (clap subcommands)
    ├── agent.rs         # Agent orchestrator (core loop)
    ├── error.rs         # Unified error types
    ├── config.rs        # AgentConfig — loads [ai_agent] from config.toml
    ├── providers/
    │   ├── mod.rs       # Provider trait + ProviderType enum
    │   ├── claude.rs    # ClaudeApiProvider (Mode A: HTTP) + ClaudeOAuthProvider (Mode B: subprocess)
    │   └── deepseek.rs  # DeepSeekProvider (HTTP API)
    ├── tools/
    │   ├── mod.rs       # SystemTool trait + ToolRegistry
    │   ├── packages.rs  # PacmanTool, YayTool — package management
    │   ├── services.rs  # SystemctlTool — service management
    │   ├── system.rs    # DfTool, PsTool, FreeTool, JournalctlTool, NmcliTool
    │   └── safety.rs    # SafetyChecker — command permission layer
    ├── memory.rs        # Memory — markdown-based cross-session context
    ├── setup.rs         # SetupWizard — blunux-ai setup
    └── ipc.rs           # IPC types (Phase 2 stub — types only, no runtime)
```

---

## 3. Cargo.toml

```toml
[package]
name = "ai-agent"
version = "0.1.0"
edition = "2021"
description = "Blunux AI Agent — natural language Linux system management"
license = "MIT"

[[bin]]
name = "blunux-ai"
path = "src/main.rs"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP client (Claude API, DeepSeek API)
reqwest = { version = "0.12", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CLI
clap = { version = "4", features = ["derive"] }

# Error handling
anyhow = "1"
thiserror = "1"

# Async traits
async-trait = "0.1"

# Setup wizard TUI
dialoguer = "0.11"
crossterm = "0.27"
indicatif = "0.17"

# Config / date
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }

# User dirs
dirs = "5"

# Internal
blunux-config = { path = "../blunux-config" }
```

---

## 4. Data Structures

### 4.1 Conversation Message

```rust
// src/agent.rs (or providers/mod.rs)

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
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
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
}
```

### 4.2 Provider Response

```rust
// src/providers/mod.rs

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
    /// Extract all text blocks joined with newlines.
    pub fn text(&self) -> String {
        self.content.iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract all tool_use blocks.
    pub fn tool_uses(&self) -> Vec<&ContentBlock> {
        self.content.iter()
            .filter(|b| matches!(b, ContentBlock::ToolUse { .. }))
            .collect()
    }

    pub fn has_tool_use(&self) -> bool {
        self.content.iter().any(|b| matches!(b, ContentBlock::ToolUse { .. }))
    }
}
```

### 4.3 Tool Definition

```rust
// src/tools/mod.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
```

### 4.4 Permission Level

```rust
// src/tools/safety.rs

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    /// Read-only, non-destructive — auto-execute.
    Safe,
    /// Modifies system state — prompt user before executing.
    RequiresConfirmation,
    /// Destructive / dangerous — never execute.
    Blocked,
}
```

### 4.5 Agent Config

```rust
// src/config.rs

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub provider: ProviderType,
    pub claude_mode: ClaudeMode,
    pub model: ModelId,
    pub whatsapp_enabled: bool,
    pub language: Language,
    pub safe_mode: bool,
    pub config_dir: PathBuf,   // ~/.config/blunux-ai/
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderType {
    Claude,
    DeepSeek,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClaudeMode {
    Api,    // Mode A: direct HTTP to api.anthropic.com
    OAuth,  // Mode B: spawn `claude -p <msg> --output-format json`
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelId {
    ClaudeSonnet46,             // claude-sonnet-4-6
    ClaudeOpus46,               // claude-opus-4-6
    DeepSeekChat,               // deepseek-chat
    DeepSeekCoder,              // deepseek-coder
}

impl ModelId {
    pub fn api_name(&self) -> &'static str {
        match self {
            Self::ClaudeSonnet46  => "claude-sonnet-4-6",
            Self::ClaudeOpus46    => "claude-opus-4-6",
            Self::DeepSeekChat    => "deepseek-chat",
            Self::DeepSeekCoder   => "deepseek-coder",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    Korean,
    English,
}

impl Language {
    /// Detect from config.toml locale.language list.
    /// Returns Korean if any entry starts with "ko", otherwise English.
    pub fn from_locale(languages: &[String]) -> Self {
        if languages.iter().any(|l| l.starts_with("ko")) {
            Self::Korean
        } else {
            Self::English
        }
    }
}
```

### 4.6 System Info (for SYSTEM.md memory)

```rust
// src/memory.rs

#[derive(Debug, Default)]
pub struct SystemInfo {
    pub distro: String,
    pub kernel: String,
    pub desktop_env: String,
    pub shell: String,
    pub cpu: String,
    pub memory_total_gb: f64,
    pub memory_used_gb: f64,
    pub disk_total_gb: f64,
    pub disk_used_gb: f64,
    pub hostname: String,
    pub username: String,
}
```

---

## 5. Error Types

```rust
// src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Safety block: {reason}")]
    SafetyBlock { reason: String },

    #[error("User cancelled")]
    UserCancelled,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Provider ──────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limit exceeded — retry after {retry_after_secs}s")]
    RateLimit { retry_after_secs: u64 },

    #[error("Authentication failed — check credentials")]
    AuthenticationFailed,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("OAuth subprocess exited {exit_code}: {stderr}")]
    SubprocessError { exit_code: i32, stderr: String },

    #[error("Response parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("Empty response from provider")]
    EmptyResponse,
}

// ── Tool ──────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Command `{command}` failed (exit {exit_code}): {stderr}")]
    ExecutionFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },

    #[error("Command timed out after {secs}s")]
    Timeout { secs: u64 },

    #[error("Invalid tool input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Memory ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Failed to read memory file {path}: {source}")]
    Read { path: String, source: std::io::Error },

    #[error("Failed to write memory file {path}: {source}")]
    Write { path: String, source: std::io::Error },
}

// ── Config ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config.toml not found at {path}")]
    NotFound { path: String },

    #[error("TOML parse error: {0}")]
    Parse(String),

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },
}
```

---

## 6. Provider Trait & Implementations

### 6.1 Provider Trait

```rust
// src/providers/mod.rs

use async_trait::async_trait;
use crate::agent::{ContentBlock, Message};
use crate::error::ProviderError;
use crate::tools::ToolDefinition;

pub use self::claude::{ClaudeApiProvider, ClaudeOAuthProvider};
pub use self::deepseek::DeepSeekProvider;

#[async_trait]
pub trait Provider: Send + Sync {
    /// Human-readable provider name (used in logs and status output).
    fn name(&self) -> &str;

    /// Send a conversation turn and return the model's completion.
    async fn complete(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
        max_tokens: u32,
    ) -> Result<CompletionResult, ProviderError>;
}

/// Instantiate the correct provider from AgentConfig.
pub fn build_provider(config: &AgentConfig) -> Result<Box<dyn Provider>, ConfigError> {
    match (&config.provider, &config.claude_mode) {
        (ProviderType::Claude, ClaudeMode::Api) => {
            let api_key = load_credential(&config.config_dir.join("credentials/claude"))?;
            Ok(Box::new(ClaudeApiProvider::new(api_key, config.model.clone())))
        }
        (ProviderType::Claude, ClaudeMode::OAuth) => {
            Ok(Box::new(ClaudeOAuthProvider::new(config.model.clone())))
        }
        (ProviderType::DeepSeek, _) => {
            let api_key = load_credential(&config.config_dir.join("credentials/deepseek"))?;
            Ok(Box::new(DeepSeekProvider::new(api_key, config.model.clone())))
        }
    }
}
```

### 6.2 Claude API Provider (Mode A)

```rust
// src/providers/claude.rs

pub struct ClaudeApiProvider {
    client: reqwest::Client,
    api_key: String,
    model: ModelId,
}

impl ClaudeApiProvider {
    pub fn new(api_key: String, model: ModelId) -> Self;
}

// Request/response shapes (internal to this module):

#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
    stop_reason: String,
    usage: ClaudeUsage,
}

// HTTP headers: x-api-key, anthropic-version: 2023-06-01, content-type
// Endpoint: https://api.anthropic.com/v1/messages
// Timeout: 120s
```

### 6.3 Claude OAuth Provider (Mode B)

```rust
// src/providers/claude.rs

pub struct ClaudeOAuthProvider {
    model: ModelId,
}

impl ClaudeOAuthProvider {
    pub fn new(model: ModelId) -> Self;
}

// Implementation:
// Spawns: claude -p "<message>" --output-format json --model <model>
// Stdin is closed; stdout is parsed as JSON.
// Stderr is collected for error reporting.
// Timeout: 120s
// Note: Multi-turn conversation is flattened to a single prompt string
//       (user/assistant turns separated by newlines with role labels).
```

### 6.4 DeepSeek Provider

```rust
// src/providers/deepseek.rs

pub struct DeepSeekProvider {
    client: reqwest::Client,
    api_key: String,
    model: ModelId,
}

impl DeepSeekProvider {
    pub fn new(api_key: String, model: ModelId) -> Self;
}

// OpenAI-compatible API:
// Endpoint: https://api.deepseek.com/v1/chat/completions
// Auth: Authorization: Bearer <api_key>
// Request format: OpenAI ChatCompletion format with tools
// Timeout: 120s
```

---

## 7. Tool Trait & Implementations

### 7.1 SystemTool Trait

```rust
// src/tools/mod.rs

#[async_trait]
pub trait SystemTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;

    /// JSON Schema for the tool's input parameters.
    fn input_schema(&self) -> serde_json::Value;

    /// Permission level determines if user confirmation is needed.
    fn permission_level(&self) -> PermissionLevel;

    /// Execute the tool with the given input.
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError>;

    /// Build ToolDefinition for passing to the provider API.
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: self.input_schema(),
        }
    }
}

/// Registry holds all available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn SystemTool>>,
}

impl ToolRegistry {
    pub fn default_tools() -> Self;
    pub fn get(&self, name: &str) -> Option<&dyn SystemTool>;
    pub fn definitions(&self) -> Vec<ToolDefinition>;
}
```

### 7.2 Tool Inventory (Phase 1)

| Tool name | Permission | Command(s) | Notes |
|---|---|---|---|
| `check_disk` | Safe | `df -h` | Disk usage |
| `check_memory` | Safe | `free -h` | RAM/swap usage |
| `check_processes` | Safe | `ps aux --sort=-%mem` | Process list |
| `read_logs` | Safe | `journalctl --since ... -p ...` | System logs |
| `check_network` | Safe | `nmcli device wifi list` | WiFi scan |
| `list_packages` | Safe | `pacman -Q` | Installed packages |
| `install_package` | RequiresConfirmation | `yay -S --noconfirm <pkg>` | Install |
| `remove_package` | RequiresConfirmation | `yay -Rns --noconfirm <pkg>` | Remove |
| `update_system` | RequiresConfirmation | `sudo pacman -Syu --noconfirm` | Full update |
| `manage_service` | RequiresConfirmation | `systemctl {enable,disable,start,stop} <svc>` | Service control |
| `run_command` | RequiresConfirmation | Any shell command | Generic fallback |

All tools must:
- Capture stdout + stderr separately
- Return structured output as a readable string
- Timeout after 60 seconds (300s for `update_system`)

### 7.3 Safety Checker

```rust
// src/tools/safety.rs

pub struct SafetyChecker {
    blocked_patterns: Vec<regex::Regex>,
}

impl SafetyChecker {
    pub fn new() -> Self;

    /// Classify a command string into its permission level.
    pub fn check(&self, command: &str) -> SafetyResult;
}

pub enum SafetyResult {
    Safe,
    RequiresConfirmation { reason: String },
    Blocked { reason: String },
}

// Blocked patterns (never execute):
// - rm -rf / (or any path starting with /)
// - dd if=... (raw disk write)
// - chmod 777 / (root permission change)
// - Fork bomb: :(){ :|:& };:
// - mkfs.* /dev/sd? (format disk)
// - > /dev/sda (raw write to block device)
// - Anything piped to /dev/sda, /dev/nvme, etc.
//
// RequiresConfirmation triggers:
// - pacman -R, yay -R (package removal)
// - pacman -Syu, yay -Syu (system update)
// - systemctl enable/disable/start/stop
// - Any sudo command not in the safe list
// - curl/wget | sh (pipe install)
```

---

## 8. Agent Orchestrator

```rust
// src/agent.rs

pub struct Agent {
    config: AgentConfig,
    provider: Box<dyn Provider>,
    tools: ToolRegistry,
    memory: Memory,
    safety: SafetyChecker,
    conversation: Vec<Message>,
    lang: Language,
}

impl Agent {
    /// Create a new Agent from AgentConfig.
    pub fn new(config: AgentConfig) -> Result<Self, AgentError>;

    /// Process a single user message and return the assistant's reply.
    /// Handles tool-use loops internally.
    pub async fn chat(&mut self, user_message: &str) -> Result<String, AgentError>;

    /// Run an interactive REPL loop (blunux-ai chat).
    pub async fn run_interactive(&mut self) -> Result<(), AgentError>;

    /// Reset conversation history (keep memory).
    pub fn reset_conversation(&mut self);

    // ── Private ──────────────────────────────────────────────────────────

    /// Build the system prompt from memory files + config.
    fn build_system_prompt(&self) -> Result<String, MemoryError>;

    /// One full provider round-trip including tool-use loop.
    async fn complete_with_tools(&mut self) -> Result<CompletionResult, AgentError>;

    /// Execute a single tool_use ContentBlock.
    async fn execute_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<String, AgentError>;

    /// Ask user for yes/no confirmation (CLI mode).
    fn prompt_confirmation(&self, action_description: &str) -> bool;

    /// Print a message in the current language.
    fn print(&self, key: UiKey);

    /// Format a message string in the current language.
    fn format(&self, key: UiKey) -> &'static str;
}
```

### 8.1 Tool-Use Loop Sequence

```
User → Agent.chat(msg)
  │
  ├─ append Message::user(msg) to conversation
  ├─ build system_prompt from memory
  │
  └─ loop:
       provider.complete(system, conversation, tools) → CompletionResult
         │
         ├─ StopReason::EndTurn → extract text → return to user
         │
         └─ StopReason::ToolUse →
              for each ToolUse block:
                safety.check(command) →
                  Blocked          → append error ToolResult, continue
                  RequiresConfirm  → prompt_confirmation()
                    No             → append "cancelled" ToolResult
                    Yes            → tools.get(name).execute(input)
                  Safe             → tools.get(name).execute(input)
                append ToolResult to conversation
              continue loop (max 10 iterations)
```

### 8.2 UI String Keys (i18n)

```rust
// src/agent.rs (or ui.rs)

pub enum UiKey {
    Welcome,
    Prompt,
    Thinking,
    ConfirmAction,
    YesNo,
    Cancelled,
    Error,
    Goodbye,
    ConfirmInstall { package: String },
    ConfirmService { action: String, service: String },
    ConfirmUpdate,
    ConfirmCommand { command: String },
}

impl Agent {
    fn ui(&self, key: &UiKey) -> String {
        match self.lang {
            Language::Korean => strings::ko(key),
            Language::English => strings::en(key),
        }
    }
}

// src/strings.rs — all UI strings in both languages
```

---

## 9. Memory Module

```rust
// src/memory.rs

pub struct Memory {
    base_dir: PathBuf,  // ~/.config/blunux-ai/
}

impl Memory {
    pub fn new(base_dir: PathBuf) -> Self;

    /// Load SYSTEM.md — auto-detected hardware/OS info.
    pub fn load_system(&self) -> Result<String, MemoryError>;

    /// Load USER.md — learned user preferences.
    pub fn load_user(&self) -> Result<String, MemoryError>;

    /// Load MEMORY.md — long-term facts.
    pub fn load_long_term(&self) -> Result<String, MemoryError>;

    /// Load today's daily log (daily/YYYY-MM-DD.md).
    pub fn load_today(&self) -> Result<String, MemoryError>;

    /// Append a line to today's daily log.
    pub fn append_today(&self, content: &str) -> Result<(), MemoryError>;

    /// Overwrite USER.md with new content.
    pub fn update_user(&self, content: &str) -> Result<(), MemoryError>;

    /// Concatenate all memory files into a single system prompt section.
    pub fn build_context(&self) -> Result<String, MemoryError>;

    /// Auto-detect system info and write SYSTEM.md.
    pub fn refresh_system_info(&self) -> Result<(), MemoryError>;

    /// Detect current system info without writing.
    pub fn detect_system_info(&self) -> SystemInfo;
}
```

### 9.1 File Layout

```
~/.config/blunux-ai/
├── config.toml              # agent config (provider, model, etc.)
├── credentials/             # chmod 600
│   ├── claude               # ANTHROPIC_API_KEY value
│   └── deepseek             # DEEPSEEK_API_KEY value
├── automations.toml         # Phase 4
├── memory/
│   ├── SYSTEM.md            # auto-refreshed on each startup
│   ├── USER.md              # updated by agent as it learns
│   ├── MEMORY.md            # long-term persistent facts
│   └── daily/
│       └── YYYY-MM-DD.md    # per-day conversation summary
├── logs/
│   └── commands.log         # append-only executed commands log
└── whatsapp/
    └── session/             # Phase 2: whatsapp-web.js session data
```

### 9.2 System Prompt Construction

```
[SYSTEM PROMPT]

You are Blunux AI Agent, a Linux system management assistant for Blunux (Arch-based).
You help users manage their system using natural language.
Available tools: [tool list]
Safe mode: [on/off]

## System Information
[SYSTEM.md content]

## User Preferences
[USER.md content]

## Long-term Memory
[MEMORY.md content]

## Today's Session ({date})
[daily/YYYY-MM-DD.md content]
```

---

## 10. Setup Wizard

```rust
// src/setup.rs

pub struct SetupWizard {
    lang: Language,
    config_dir: PathBuf,
    blunux_config_path: PathBuf,
}

impl SetupWizard {
    pub fn new(lang: Language, config_dir: PathBuf, blunux_config_path: PathBuf) -> Self;

    /// Run the full interactive setup. Returns AgentConfig on success.
    pub fn run(&self) -> Result<AgentConfig, AgentError>;
}
```

### 10.1 Wizard Flow

```
Step 1: Welcome banner (language-aware)

Step 2: Provider selection
  → [ Claude (Anthropic) | DeepSeek ]

Step 3a (if Claude): Connection mode
  → [ OAuth — Claude Pro/Max subscription (no API key) |
      API Key — Direct HTTP (pay per token) ]

Step 3b (if Claude OAuth): Check claude CLI installed
  → if missing: "Installing Claude Code..." → npm install -g @anthropic-ai/claude-code
  → "Please run: claude login" (open browser)

Step 3b (if Claude API): Enter API key
  → Masked input → validate by calling API → save to credentials/claude (chmod 600)

Step 3c (if DeepSeek): Enter API key
  → Masked input → validate → save to credentials/deepseek (chmod 600)

Step 4: Model selection
  Claude: [ claude-sonnet-4-6 (Recommended) | claude-opus-4-6 (More capable, slower) ]
  DeepSeek: [ deepseek-chat (Recommended) | deepseek-coder (Code focus) ]

Step 5: Write ~/.config/blunux-ai/config.toml

Step 6: Initialize memory
  → Create directory structure
  → Detect and write SYSTEM.md
  → Create empty USER.md and MEMORY.md

Step 7: WhatsApp setup (Phase 2 — show "Coming soon / 추후 지원 예정" in Phase 1)

Step 8: Done
  → Show: blunux-ai chat
  → Test message example
```

---

## 11. CLI Commands

```rust
// src/main.rs

#[derive(Parser)]
#[command(name = "blunux-ai", about = "Blunux AI Agent")]
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
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Show current memory contents
    Show,
    /// Clear all memory files
    Clear,
    /// Refresh SYSTEM.md with current system info
    Refresh,
}
```

### 11.1 Command Behaviors

| Command | Behavior |
|---|---|
| `blunux-ai` (no args) | Same as `blunux-ai chat` |
| `blunux-ai chat` | Interactive REPL. Ctrl+C exits. |
| `blunux-ai setup` | Setup wizard. Overwrites config if re-run. |
| `blunux-ai status` | Print provider, model, memory size, service status. |
| `blunux-ai memory show` | Print all memory file contents. |
| `blunux-ai memory clear` | Prompt confirmation, then delete daily/ and MEMORY.md. |
| `blunux-ai memory refresh` | Re-detect system info and update SYSTEM.md. |

---

## 12. Config Integration with blunux-config

### 12.1 AiAgent struct (blunux-config crate)

```rust
// crates/blunux-config/src/lib.rs — additions

#[derive(Debug, Deserialize, Serialize)]
pub struct BlunuxConfig {
    // ... existing fields ...
    #[serde(default)]
    pub ai_agent: Option<AiAgent>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AiAgent {
    pub enabled: bool,
    pub provider: String,        // "claude" | "deepseek"
    pub claude_mode: String,     // "oauth" | "api"
    pub whatsapp_enabled: bool,
    pub language: String,        // "ko" | "en" (overrides locale.language if set)
    pub safe_mode: bool,
}

impl Default for AiAgent {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "claude".into(),
            claude_mode: "oauth".into(),
            whatsapp_enabled: true,
            language: "auto".into(),  // "auto" = detect from locale.language
            safe_mode: true,
        }
    }
}
```

### 12.2 config.toml Extension

```toml
[ai_agent]
enabled = true
provider = "claude"       # "claude" | "deepseek"
claude_mode = "oauth"     # "oauth" | "api"
whatsapp_enabled = true
language = "auto"         # "auto" | "ko" | "en"
safe_mode = true
```

### 12.3 build.jl Extension

```julia
# In build_rust():
ai = get(get(cfg, "ai_agent", Dict()), "enabled", false)
if ai
    println("  ai_agent.enabled = true → including ai-agent crate")
    push!(binaries_to_copy, "blunux-ai")
else
    println("  ai_agent.enabled = false → skipping ai-agent")
end
```

### 12.4 Cargo.toml Extension (workspace)

```toml
[workspace]
members = [
    "crates/blunux-config",
    "crates/toml2cal",
    "crates/wizard",
    "crates/setup",
    "crates/ai-agent",   # NEW
]
```

---

## 13. IPC Types (Phase 2 Stub)

These types are defined in Phase 1 but the runtime (Unix socket listener) is not implemented until Phase 2.

```rust
// src/ipc.rs

use serde::{Deserialize, Serialize};

/// Socket path: /run/user/{uid}/blunux-ai.sock
pub fn socket_path() -> std::path::PathBuf {
    let uid = unsafe { libc::getuid() };
    std::path::PathBuf::from(format!("/run/user/{uid}/blunux-ai.sock"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    #[serde(rename = "type")]
    pub msg_type: IpcMessageType,

    /// Sender phone number (Bridge → Agent: message)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,

    /// Message body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,

    /// Recipient phone number (Agent → Bridge: response)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,

    /// Quick-reply action buttons (Agent → Bridge)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<String>>,

    /// Selected action (Bridge → Agent: confirmation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IpcMessageType {
    Message,   // Bridge → Agent: incoming WhatsApp message
    Response,  // Agent → Bridge: reply to send
    Action,    // Bridge → Agent: user selected a quick-reply action
}
```

---

## 14. Command Log Format

All executed commands are appended to `~/.config/blunux-ai/logs/commands.log`:

```
[2026-02-20T09:15:32Z] SAFE        df -h
[2026-02-20T09:16:01Z] CONFIRMED   yay -S google-chrome
[2026-02-20T09:17:45Z] BLOCKED     rm -rf /home/blu
[2026-02-20T09:18:12Z] CANCELLED   sudo pacman -Syu
```

Format: `[ISO8601] {SAFE|CONFIRMED|BLOCKED|CANCELLED|FAILED} {command}`

---

## 15. Test Plan

### 15.1 Unit Tests

| Module | Test | Approach |
|---|---|---|
| `error.rs` | Error display strings | `assert!(format!("{err}").contains(...))` |
| `config.rs` | `Language::from_locale` | Test with `["ko_KR"]`, `["en_US"]`, `[]` |
| `config.rs` | `ModelId::api_name` | All variants |
| `tools/safety.rs` | Blocked patterns | `rm -rf /`, fork bomb, `dd if=...` |
| `tools/safety.rs` | RequiresConfirmation | pacman -R, systemctl enable |
| `tools/safety.rs` | Safe commands | `df -h`, `free -h`, `ps aux` |
| `memory.rs` | File read/write round-trip | Temp dir fixture |
| `memory.rs` | `build_context` with empty files | No panic |
| `agent.rs` | `Message::user` / `Message::assistant_text` | Field checks |
| `agent.rs` | `CompletionResult::text` | Multiple content blocks |
| `agent.rs` | `CompletionResult::has_tool_use` | true/false cases |
| `ipc.rs` | `IpcMessage` serde round-trip | JSON serialize → deserialize |

### 15.2 Integration Tests

| Test | Description |
|---|---|
| `test_claude_api_provider` | Real HTTP call with `ANTHROPIC_API_KEY` env var (skip if unset) |
| `test_deepseek_provider` | Real HTTP call with `DEEPSEEK_API_KEY` env var (skip if unset) |
| `test_tool_disk_check` | Run `check_disk` tool, verify output contains `/` |
| `test_tool_memory_check` | Run `check_memory` tool, verify `MemTotal` present |
| `test_memory_lifecycle` | Init memory dir → write → read → append today |
| `test_setup_config_write` | Wizard writes valid `config.toml` to temp dir |

### 15.3 Test Conventions

- Tests in each module under `#[cfg(test)]` block
- Integration tests in `crates/ai-agent/tests/`
- Network tests gated by env vars: skip with `#[cfg_attr(not(env = "INTEGRATION"), ignore)]`
- Use `tempfile` crate for file system tests

---

## 16. Phase 1 Deliverables Checklist

- [ ] `crates/ai-agent/Cargo.toml`
- [ ] `src/error.rs` — all error types
- [ ] `src/config.rs` — AgentConfig, Language, ModelId, ProviderType
- [ ] `src/providers/mod.rs` — Provider trait + `build_provider`
- [ ] `src/providers/claude.rs` — ClaudeApiProvider + ClaudeOAuthProvider
- [ ] `src/providers/deepseek.rs` — DeepSeekProvider
- [ ] `src/tools/mod.rs` — SystemTool trait + ToolRegistry
- [ ] `src/tools/safety.rs` — SafetyChecker with all blocked patterns
- [ ] `src/tools/system.rs` — df, free, ps, journalctl, nmcli tools
- [ ] `src/tools/packages.rs` — pacman/yay install, remove, update tools
- [ ] `src/tools/services.rs` — systemctl tool
- [ ] `src/memory.rs` — Memory struct + SystemInfo detection
- [ ] `src/agent.rs` — Agent with tool-use loop + i18n UI strings
- [ ] `src/setup.rs` — SetupWizard
- [ ] `src/main.rs` — CLI with chat, setup, status, memory subcommands
- [ ] `src/ipc.rs` — IPC type stubs
- [ ] `crates/blunux-config/src/lib.rs` — AiAgent struct added
- [ ] `config.toml` — `[ai_agent]` section added
- [ ] `Cargo.toml` — ai-agent workspace member added
- [ ] `build.jl` — conditional ai-agent build
- [ ] All unit tests passing (`cargo test`)
