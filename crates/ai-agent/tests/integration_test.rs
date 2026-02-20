//! Phase 1 TDD Integration Test Suite
//!
//! Tests from TDD §15.2 — Integration Tests.
//!
//! ## Running all tests
//! ```
//! cargo test --manifest-path crates/ai-agent/Cargo.toml
//! ```
//!
//! ## Running network tests (requires API keys)
//! ```
//! ANTHROPIC_API_KEY=<key> cargo test --manifest-path crates/ai-agent/Cargo.toml -- --ignored
//! DEEPSEEK_API_KEY=<key>  cargo test --manifest-path crates/ai-agent/Cargo.toml -- --ignored
//! ```

use tempfile::tempdir;

use ai_agent::config::{AgentConfig, ClaudeMode, Language, ModelId, ProviderType, WhatsAppConfig};
use ai_agent::memory::Memory;
use ai_agent::providers::{ClaudeApiProvider, DeepSeekProvider, Message, Provider, StopReason};
use ai_agent::tools::ToolRegistry;

// ── Tool tests ────────────────────────────────────────────────────────────────

/// TDD §15.2: test_tool_disk_check
/// Run the `check_disk` tool and verify it returns `df -h` output containing
/// the root filesystem.
#[tokio::test]
async fn test_tool_disk_check() {
    let registry = ToolRegistry::default_tools();
    let tool = registry.get("check_disk").expect("check_disk tool not registered");

    let output = tool
        .execute(serde_json::json!({}))
        .await
        .expect("check_disk execute failed");

    assert!(!output.is_empty(), "check_disk returned empty output");
    // df -h always includes the root mount point "/"
    assert!(
        output.contains('/'),
        "check_disk output should contain '/' mount point\ngot: {output}"
    );
    // Verify human-readable size columns (G, M, T, or %)
    assert!(
        output.contains('%') || output.contains('G') || output.contains('M'),
        "check_disk output should contain size information\ngot: {output}"
    );
}

/// TDD §15.2: test_tool_memory_check
/// Run the `check_memory` tool and verify `free -h` output contains `Mem:`.
#[tokio::test]
async fn test_tool_memory_check() {
    let registry = ToolRegistry::default_tools();
    let tool = registry
        .get("check_memory")
        .expect("check_memory tool not registered");

    let output = tool
        .execute(serde_json::json!({}))
        .await
        .expect("check_memory execute failed");

    assert!(!output.is_empty(), "check_memory returned empty output");
    // free -h always prints a "Mem:" row
    assert!(
        output.contains("Mem:"),
        "check_memory output should contain 'Mem:' header\ngot: {output}"
    );
}

/// Extra: verify the tool registry contains all 11 expected tools.
#[test]
fn test_tool_registry_has_all_tools() {
    let registry = ToolRegistry::default_tools();
    let expected = [
        "check_disk",
        "check_memory",
        "check_processes",
        "read_logs",
        "check_network",
        "list_packages",
        "install_package",
        "remove_package",
        "update_system",
        "manage_service",
        "run_command",
    ];
    for name in &expected {
        assert!(
            registry.get(name).is_some(),
            "tool '{name}' missing from registry"
        );
    }
    // Verify definitions() returns the same count
    assert_eq!(registry.definitions().len(), expected.len());
}

// ── Memory lifecycle ──────────────────────────────────────────────────────────

/// TDD §15.2: test_memory_lifecycle
/// Init memory dir → write → read → append today
#[test]
fn test_memory_lifecycle() {
    let tmp = tempdir().expect("failed to create temp dir");
    let mem = Memory::new(tmp.path().to_path_buf());

    // 1. init_dirs — creates all expected subdirectories
    mem.init_dirs().expect("init_dirs failed");
    assert!(tmp.path().join("memory").is_dir(), "memory/ dir missing");
    assert!(tmp.path().join("memory/daily").is_dir(), "memory/daily/ dir missing");
    assert!(tmp.path().join("logs").is_dir(), "logs/ dir missing");
    assert!(tmp.path().join("credentials").is_dir(), "credentials/ dir missing");

    // 2. Write user preferences to USER.md
    mem.update_user("## Preferences\n- Prefers dark mode\n- Language: Korean\n")
        .expect("update_user failed");

    // 3. Read back and verify content
    let user_content = mem.load_user().expect("load_user failed");
    assert!(
        user_content.contains("dark mode"),
        "USER.md should contain written preferences"
    );
    assert!(
        user_content.contains("Korean"),
        "USER.md should contain Korean preference"
    );

    // 4. Append two entries to today's daily log
    mem.append_today("[09:00] User asked: disk usage?")
        .expect("append_today #1 failed");
    mem.append_today("[09:01] Agent ran: df -h — output: / 40G 20G")
        .expect("append_today #2 failed");

    // 5. Verify both entries are present
    let today = mem.load_today().expect("load_today failed");
    assert!(
        today.contains("disk usage"),
        "today log should contain first entry"
    );
    assert!(
        today.contains("df -h"),
        "today log should contain second entry"
    );

    // 6. build_context returns non-empty string and doesn't panic
    let ctx = mem.build_context().expect("build_context failed");
    assert!(!ctx.is_empty(), "build_context should return non-empty output");
    // Context should contain the user preferences we wrote
    assert!(
        ctx.contains("dark mode"),
        "build_context should include USER.md content"
    );
}

// ── Config save/load round-trip ───────────────────────────────────────────────

/// TDD §15.2: test_setup_config_write
/// Construct an AgentConfig, save it, verify the file on disk, and reload
/// to confirm a perfect round-trip.
#[test]
fn test_setup_config_write() {
    let tmp = tempdir().expect("failed to create temp dir");

    let original = AgentConfig {
        provider: ProviderType::DeepSeek,
        claude_mode: ClaudeMode::OAuth,
        model: ModelId::DeepSeekChat,
        whatsapp_enabled: true,
        language: Language::English,
        safe_mode: false,
        config_dir: tmp.path().to_path_buf(),
        whatsapp: WhatsAppConfig {
            allowed_numbers: vec![
                "+821012345678".to_string(),
                "+821087654321".to_string(),
            ],
            max_messages_per_minute: 10,
        },
    };

    // Write config.toml
    original.save().expect("save failed");

    let config_path = tmp.path().join("config.toml");
    assert!(config_path.exists(), "config.toml should exist after save");

    // Spot-check raw content
    let raw = std::fs::read_to_string(&config_path).unwrap();
    assert!(raw.contains("deepseek"), "raw config should contain provider name");
    assert!(raw.contains("safe_mode = false"), "raw config should reflect safe_mode = false");
    assert!(
        raw.contains("+821012345678"),
        "raw config should contain first allowed number"
    );
    assert!(
        raw.contains("+821087654321"),
        "raw config should contain second allowed number"
    );
    assert!(
        raw.contains("max_messages_per_minute = 10"),
        "raw config should contain rate limit"
    );

    // Full round-trip: load and verify all fields
    let loaded = AgentConfig::load(tmp.path()).expect("load failed");
    assert_eq!(loaded.provider, ProviderType::DeepSeek);
    assert_eq!(loaded.model, ModelId::DeepSeekChat);
    assert_eq!(loaded.language, Language::English);
    assert!(!loaded.safe_mode, "safe_mode should be false after round-trip");
    assert!(loaded.whatsapp_enabled, "whatsapp_enabled should be true after round-trip");
    assert_eq!(
        loaded.whatsapp.allowed_numbers.len(),
        2,
        "should have 2 allowed numbers"
    );
    assert_eq!(
        loaded.whatsapp.max_messages_per_minute,
        10,
        "rate limit should round-trip correctly"
    );
}

// ── Network tests (ignored — require external API keys) ───────────────────────

/// TDD §15.2: test_claude_api_provider
///
/// Run with:
///   ANTHROPIC_API_KEY=<key> cargo test test_claude_api_provider -- --ignored
#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY environment variable"]
async fn test_claude_api_provider() {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set to run this test");
    assert!(!api_key.is_empty(), "ANTHROPIC_API_KEY must not be empty");

    let provider = ClaudeApiProvider::new(api_key, ModelId::ClaudeSonnet46);
    let messages = vec![Message::user(
        "Reply with exactly the single word: pong. No punctuation.",
    )];

    let result = provider
        .complete("You are a test assistant. Follow instructions exactly.", &messages, &[], 32)
        .await
        .expect("Claude API call failed");

    assert_eq!(
        result.stop_reason,
        StopReason::EndTurn,
        "stop_reason should be EndTurn for a simple text reply"
    );
    let text = result.text().to_lowercase();
    assert!(!text.is_empty(), "response should not be empty");
    assert!(
        text.contains("pong"),
        "expected 'pong' in response, got: {text}"
    );
}

/// TDD §15.2: test_deepseek_provider
///
/// Run with:
///   DEEPSEEK_API_KEY=<key> cargo test test_deepseek_provider -- --ignored
#[tokio::test]
#[ignore = "requires DEEPSEEK_API_KEY environment variable"]
async fn test_deepseek_provider() {
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .expect("DEEPSEEK_API_KEY must be set to run this test");
    assert!(!api_key.is_empty(), "DEEPSEEK_API_KEY must not be empty");

    let provider = DeepSeekProvider::new(api_key, ModelId::DeepSeekChat);
    let messages = vec![Message::user(
        "Reply with exactly the single word: pong. No punctuation.",
    )];

    let result = provider
        .complete("You are a test assistant. Follow instructions exactly.", &messages, &[], 32)
        .await
        .expect("DeepSeek API call failed");

    assert_eq!(
        result.stop_reason,
        StopReason::EndTurn,
        "stop_reason should be EndTurn for a simple text reply"
    );
    let text = result.text().to_lowercase();
    assert!(!text.is_empty(), "response should not be empty");
    assert!(
        text.contains("pong"),
        "expected 'pong' in response, got: {text}"
    );
}
