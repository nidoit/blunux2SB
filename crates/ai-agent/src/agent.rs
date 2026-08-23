use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::time::{Duration, Instant};

use crate::config::{AgentConfig, Language};
use crate::error::AgentError;
use crate::memory::Memory;
use crate::providers::{
    build_provider, CompletionResult, ContentBlock, Message, Provider, StopReason,
};
use crate::strings;
use crate::tools::{PermissionLevel, SafetyChecker, SafetyResult, ToolRegistry};

const MAX_TOOL_LOOP_ITERATIONS: usize = 10;
const MAX_TOKENS: u32 = 4096;
/// How long a WhatsApp confirmation request stays valid.
const CONFIRM_TIMEOUT: Duration = Duration::from_secs(300);

/// How confirmation prompts are resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ConfirmMode {
    /// CLI chat: ask on stdin.
    Interactive,
    /// Daemon with `daemon_auto_confirm = true`: execute without asking.
    AutoApprove,
    /// Daemon default: don't execute — ask the user to reply YES over WhatsApp.
    Deferred,
}

/// A privileged tool call waiting for the user's WhatsApp confirmation.
struct PendingAction {
    tool_name: String,
    input: serde_json::Value,
    description: String,
}

struct PendingSet {
    actions: Vec<PendingAction>,
    created: Instant,
}

pub struct Agent {
    provider: Box<dyn Provider>,
    tools: ToolRegistry,
    memory: Memory,
    safety: SafetyChecker,
    conversation: Vec<Message>,
    /// Per-user conversation history for daemon mode (keyed by phone number).
    user_conversations: HashMap<String, Vec<Message>>,
    lang: Language,
    safe_mode: bool,
    confirm_mode: ConfirmMode,
    /// Deferred confirmations per user (daemon mode).
    pending_confirmations: HashMap<String, PendingSet>,
    /// User whose message is currently being processed (daemon mode).
    current_user: Option<String>,
    /// Set inside the tool loop when an action was deferred this turn.
    deferred_question: Option<String>,
    /// Per-user inactivity cutoff before conversation history is reset.
    session_timeout: Duration,
    last_activity: HashMap<String, Instant>,
}

impl Agent {
    pub fn new(config: &AgentConfig) -> Result<Self, AgentError> {
        let provider = build_provider(config).map_err(AgentError::Config)?;
        let tools = ToolRegistry::default_tools();
        let memory = Memory::new(config.config_dir.clone());
        let safety = SafetyChecker::new(config.safe_mode);

        Ok(Self {
            provider,
            tools,
            memory,
            safety,
            conversation: Vec::new(),
            user_conversations: HashMap::new(),
            lang: config.language.clone(),
            safe_mode: config.safe_mode,
            confirm_mode: ConfirmMode::Interactive,
            pending_confirmations: HashMap::new(),
            current_user: None,
            deferred_question: None,
            session_timeout: Duration::from_secs(u64::from(config.whatsapp.session_timeout)),
            last_activity: HashMap::new(),
        })
    }

    /// Create an agent configured for daemon / WhatsApp mode.
    ///
    /// Privileged actions are NOT auto-approved: the user gets a confirmation
    /// request over WhatsApp and must reply YES within 5 minutes. Setting
    /// `daemon_auto_confirm = true` in config.toml restores auto-approval.
    pub fn new_daemon(config: &AgentConfig) -> Result<Self, AgentError> {
        let mut agent = Self::new(config)?;
        agent.confirm_mode = if config.daemon_auto_confirm {
            ConfirmMode::AutoApprove
        } else {
            ConfirmMode::Deferred
        };
        Ok(agent)
    }

    pub async fn chat(&mut self, user_message: &str) -> Result<String, AgentError> {
        // Add user message
        self.conversation.push(Message::user(user_message));

        // Log to daily memory
        let _ = self.memory.append_today(user_message);

        // Build system prompt
        let system_prompt = self.build_system_prompt()?;
        let tool_defs = self.tools.definitions();

        // Tool-use loop
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > MAX_TOOL_LOOP_ITERATIONS {
                break;
            }

            let result = self
                .provider
                .complete(&system_prompt, &self.conversation, &tool_defs, MAX_TOKENS)
                .await
                .map_err(AgentError::Provider)?;

            // Add assistant response to conversation
            self.conversation.push(Message {
                role: crate::providers::Role::Assistant,
                content: result.content.clone(),
            });

            match result.stop_reason {
                StopReason::EndTurn | StopReason::MaxTokens => {
                    let text = result.text();
                    let _ = self.memory.append_today(&format!("AI: {text}"));
                    return Ok(text);
                }
                StopReason::ToolUse => {
                    let tool_results = self.process_tool_calls(&result).await?;
                    if !tool_results.is_empty() {
                        self.conversation.push(Message::tool_results(tool_results));
                    }
                    // Deferred mode: an action needs the user's WhatsApp
                    // confirmation. Stop here and return the question instead
                    // of letting the model continue.
                    if let Some(question) = self.deferred_question.take() {
                        self.conversation.push(Message::assistant_text(question.clone()));
                        let _ = self.memory.append_today(&format!("AI: {question}"));
                        return Ok(question);
                    }
                    // Continue loop for next completion
                }
            }
        }

        // If we exhausted iterations, return whatever text we have
        let last_text = self
            .conversation
            .last()
            .map(|m| {
                m.content
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        Ok(last_text)
    }

    pub async fn run_interactive(&mut self) -> Result<(), AgentError> {
        // Refresh system info on startup
        let _ = self.memory.refresh_system_info();

        // Welcome
        println!(
            "\n  {} v{}",
            strings::welcome(&self.lang),
            env!("CARGO_PKG_VERSION")
        );
        println!(
            "   {} ({}) | {}",
            self.provider.name(),
            "AI Agent",
            strings::exit_hint(&self.lang)
        );
        println!();

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            // Prompt
            print!("{}: ", strings::prompt(&self.lang));
            stdout.flush().map_err(AgentError::Io)?;

            // Read line
            let mut input = String::new();
            match stdin.lock().read_line(&mut input) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(_) => break,
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            // Thinking indicator
            print!("\n  {} ", strings::thinking(&self.lang));
            stdout.flush().map_err(AgentError::Io)?;

            match self.chat(input).await {
                Ok(response) => {
                    // Clear thinking line and print response
                    print!("\r");
                    println!("\nAI: {response}\n");
                }
                Err(AgentError::UserCancelled) => {
                    println!("\n  {}\n", strings::cancelled(&self.lang));
                }
                Err(e) => {
                    println!(
                        "\n  {}: {e}\n",
                        strings::error_prefix(&self.lang)
                    );
                }
            }
        }

        println!("\n  {}", strings::goodbye(&self.lang));
        Ok(())
    }

    pub fn reset_conversation(&mut self) {
        self.conversation.clear();
    }

    /// Clear the stored conversation history for a specific user (daemon mode).
    pub fn reset_user_conversation(&mut self, phone: &str) {
        self.user_conversations.remove(phone);
    }

    /// Process a message on behalf of a specific user (daemon / WhatsApp mode).
    /// Each phone number maintains its own conversation history.
    pub async fn chat_as_user(
        &mut self,
        phone: &str,
        user_message: &str,
    ) -> Result<String, AgentError> {
        // Session timeout: reset stale conversations (and any stale pending
        // confirmation with them) after `session_timeout` of inactivity.
        let now = Instant::now();
        if let Some(last) = self.last_activity.get(phone) {
            if now.duration_since(*last) > self.session_timeout {
                self.user_conversations.remove(phone);
                self.pending_confirmations.remove(phone);
            }
        }
        self.last_activity.insert(phone.to_string(), now);

        // A confirmation is pending for this user: interpret the reply.
        if let Some(set) = self.pending_confirmations.remove(phone) {
            let reply = user_message.trim().to_lowercase();
            if set.created.elapsed() > CONFIRM_TIMEOUT {
                if is_affirmative(&reply) {
                    // Too late — require a fresh request instead of running
                    // a possibly forgotten action.
                    return Ok(strings::wa_confirm_expired(&self.lang).to_string());
                }
                // Unrelated/negative message: silently drop the stale request.
            } else if is_affirmative(&reply) {
                return self.execute_pending(phone, set).await;
            } else if is_negative(&reply) {
                self.record_exchange(phone, user_message, strings::cancelled(&self.lang));
                return Ok(strings::cancelled(&self.lang).to_string());
            }
            // Any other message implicitly cancels and is handled normally.
        }

        // Restore per-user conversation
        let mut conv = self
            .user_conversations
            .remove(phone)
            .unwrap_or_default();

        // Swap in the user's conversation
        std::mem::swap(&mut self.conversation, &mut conv);
        self.current_user = Some(phone.to_string());

        let result = self.chat(user_message).await;

        // Swap back and store
        self.current_user = None;
        std::mem::swap(&mut self.conversation, &mut conv);
        self.user_conversations.insert(phone.to_string(), conv);

        result
    }

    /// Execute a confirmed pending action set and report the results.
    async fn execute_pending(
        &mut self,
        phone: &str,
        set: PendingSet,
    ) -> Result<String, AgentError> {
        let mut outputs = Vec::new();
        for action in set.actions {
            let tool = match self.tools.get(&action.tool_name) {
                Some(t) => t,
                None => continue,
            };
            let _ = self.memory.log_command("CONFIRMED", &action.description);
            match tool.execute(action.input).await {
                Ok(out) => outputs.push(format!("✅ {}\n{}", action.description, out)),
                Err(e) => {
                    let _ = self.memory.log_command("FAILED", &action.description);
                    outputs.push(format!("❌ {}: {e}", action.description));
                }
            }
        }
        let reply = if outputs.is_empty() {
            strings::cancelled(&self.lang).to_string()
        } else {
            outputs.join("\n\n")
        };
        self.record_exchange(phone, strings::wa_confirm_approved(&self.lang), &reply);
        Ok(reply)
    }

    /// Append a user/assistant exchange to a user's stored history so the
    /// model keeps context about what actually happened.
    fn record_exchange(&mut self, phone: &str, user_text: &str, assistant_text: &str) {
        let conv = self.user_conversations.entry(phone.to_string()).or_default();
        conv.push(Message::user(user_text));
        conv.push(Message::assistant_text(assistant_text));
    }

    /// Run a scheduled automation action without a user phone number.
    /// The action string is treated as a system-initiated instruction to the AI;
    /// the reply is returned as the notification body.
    pub async fn run_automation(&mut self, action: &str) -> Result<String, AgentError> {
        // Use a temporary isolated conversation so automations don't pollute
        // any active user conversation history.
        let saved = std::mem::take(&mut self.conversation);
        let result = self.chat(action).await;
        self.conversation = saved;
        result
    }

    fn build_system_prompt(&self) -> Result<String, AgentError> {
        let memory_ctx = self.memory.build_context().map_err(AgentError::Memory)?;

        let lang_instruction = match self.lang {
            Language::Korean => "사용자에게 한국어로 답변하세요.",
            Language::English => "Respond in English.",
        };

        let tool_names: Vec<String> = self.tools.definitions().iter().map(|t| t.name.clone()).collect();
        let safe_mode_str = if self.safe_mode { "enabled" } else { "disabled" };

        Ok(format!(
            "You are Blunux AI Agent, a Linux system management assistant for Blunux (Arch-based).\n\
             You help users manage their system using natural language.\n\
             {lang_instruction}\n\
             \n\
             Available tools: {tool_list}\n\
             Safe mode: {safe_mode_str}\n\
             \n\
             Rules:\n\
             - Use the provided tools to execute system commands\n\
             - Explain what you're doing before executing commands\n\
             - For package names, use the exact Arch Linux / AUR package name\n\
             - Never run destructive commands without user confirmation\n\
             - Report results clearly and concisely\n\
             \n\
             {memory_ctx}",
            tool_list = tool_names.join(", "),
        ))
    }

    async fn process_tool_calls(
        &mut self,
        result: &CompletionResult,
    ) -> Result<Vec<ContentBlock>, AgentError> {
        let mut tool_results = Vec::new();

        for (id, name, input) in result.tool_uses() {
            let id = id.to_string();
            let name = name.to_string();
            let input = input.clone();
            let tool_result = self.execute_tool(&id, &name, input).await?;
            tool_results.push(tool_result);
        }

        Ok(tool_results)
    }

    async fn execute_tool(
        &mut self,
        tool_use_id: &str,
        name: &str,
        input: serde_json::Value,
    ) -> Result<ContentBlock, AgentError> {
        let permission = match self.tools.get(name) {
            Some(t) => t.permission_level(),
            None => {
                return Ok(ContentBlock::ToolResult {
                    tool_use_id: tool_use_id.to_string(),
                    content: format!("Unknown tool: {name}"),
                    is_error: true,
                });
            }
        };

        // For run_command, extract the command string and check safety
        let command_str = if name == "run_command" {
            input.get("command").and_then(|v| v.as_str()).map(|s| s.to_string())
        } else {
            None
        };

        // Check permission level
        match permission {
            PermissionLevel::Safe => {
                // Auto-execute
            }
            PermissionLevel::RequiresConfirmation => {
                // Check safety for run_command specifically
                if let Some(ref cmd) = command_str {
                    match self.safety.check(cmd) {
                        SafetyResult::Blocked { reason } => {
                            let _ = self.memory.log_command("BLOCKED", cmd);
                            return Ok(ContentBlock::ToolResult {
                                tool_use_id: tool_use_id.to_string(),
                                content: format!(
                                    "{}: {reason}",
                                    strings::blocked(&self.lang)
                                ),
                                is_error: true,
                            });
                        }
                        SafetyResult::RequiresConfirmation { reason } => {
                            let description =
                                strings::confirm_command(&self.lang, cmd);
                            if self.confirm_mode == ConfirmMode::Deferred {
                                return Ok(self.defer_action(
                                    tool_use_id,
                                    name,
                                    input,
                                    &description,
                                    &reason,
                                ));
                            }
                            println!("\n  {description}");
                            println!("  ({reason})");
                            if !self.prompt_confirmation() {
                                let _ = self.memory.log_command("CANCELLED", cmd);
                                return Ok(ContentBlock::ToolResult {
                                    tool_use_id: tool_use_id.to_string(),
                                    content: strings::cancelled(&self.lang).to_string(),
                                    is_error: false,
                                });
                            }
                        }
                        SafetyResult::Safe => {}
                    }
                } else if self.safe_mode {
                    // Non-run_command tool requiring confirmation
                    let description = strings::tool_executing(&self.lang, name);
                    if self.confirm_mode == ConfirmMode::Deferred {
                        return Ok(self.defer_action(
                            tool_use_id,
                            name,
                            input,
                            &description,
                            "",
                        ));
                    }
                    println!("\n  {description}");
                    if !self.prompt_confirmation() {
                        let _ = self.memory.log_command("CANCELLED", name);
                        return Ok(ContentBlock::ToolResult {
                            tool_use_id: tool_use_id.to_string(),
                            content: strings::cancelled(&self.lang).to_string(),
                            is_error: false,
                        });
                    }
                }
            }
            PermissionLevel::Blocked => {
                let _ = self.memory.log_command("BLOCKED", name);
                return Ok(ContentBlock::ToolResult {
                    tool_use_id: tool_use_id.to_string(),
                    content: strings::blocked(&self.lang).to_string(),
                    is_error: true,
                });
            }
        }

        // Execute the tool
        let log_cmd = command_str.as_deref().unwrap_or(name);
        let tool = match self.tools.get(name) {
            Some(t) => t,
            None => unreachable!("tool existence checked above"),
        };
        match tool.execute(input).await {
            Ok(output) => {
                let status = if tool.permission_level() == PermissionLevel::Safe {
                    "SAFE"
                } else {
                    "CONFIRMED"
                };
                let _ = self.memory.log_command(status, log_cmd);
                Ok(ContentBlock::ToolResult {
                    tool_use_id: tool_use_id.to_string(),
                    content: output,
                    is_error: false,
                })
            }
            Err(e) => {
                let _ = self.memory.log_command("FAILED", log_cmd);
                Ok(ContentBlock::ToolResult {
                    tool_use_id: tool_use_id.to_string(),
                    content: format!("Error: {e}"),
                    is_error: true,
                })
            }
        }
    }

    /// Queue a privileged action for WhatsApp confirmation instead of
    /// executing it, and prepare the confirmation question for the user.
    /// Returns the tool_result block recording that nothing was executed.
    fn defer_action(
        &mut self,
        tool_use_id: &str,
        name: &str,
        input: serde_json::Value,
        description: &str,
        reason: &str,
    ) -> ContentBlock {
        // Automation runs have no user to ask — the action is simply skipped
        // and reported as not executed.
        if let Some(phone) = self.current_user.clone() {
            let desc = if reason.is_empty() {
                description.to_string()
            } else {
                format!("{description} — {reason}")
            };
            let _ = self.memory.log_command("PENDING", &desc);

            let entry = self
                .pending_confirmations
                .entry(phone)
                .or_insert_with(|| PendingSet {
                    actions: Vec::new(),
                    created: Instant::now(),
                });
            entry.created = Instant::now();
            entry.actions.push(PendingAction {
                tool_name: name.to_string(),
                input,
                description: desc,
            });

            let items = entry
                .actions
                .iter()
                .map(|a| format!("• {}", a.description))
                .collect::<Vec<_>>()
                .join("\n");
            self.deferred_question = Some(strings::wa_confirm_request(&self.lang, &items));
        }

        ContentBlock::ToolResult {
            tool_use_id: tool_use_id.to_string(),
            content: "NOT EXECUTED — this action requires explicit user confirmation. \
                      The user has been asked to confirm. Do not retry or work around it."
                .to_string(),
            is_error: false,
        }
    }

    fn prompt_confirmation(&self) -> bool {
        if self.confirm_mode == ConfirmMode::AutoApprove {
            return true;
        }

        print!("  {}", strings::confirm_action(&self.lang));
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            return false;
        }

        let input = input.trim().to_lowercase();
        input == "y" || input == "yes"
    }
}

/// Does a (trimmed, lowercased) WhatsApp reply approve the pending action?
fn is_affirmative(reply: &str) -> bool {
    matches!(
        reply,
        "yes" | "y" | "ok" | "okay" | "네" | "예" | "응" | "확인" | "승인" | "실행" | "좋아" | "ㅇㅋ"
    )
}

/// Does a (trimmed, lowercased) WhatsApp reply reject the pending action?
fn is_negative(reply: &str) -> bool {
    matches!(
        reply,
        "no" | "n" | "cancel" | "stop" | "아니" | "아니요" | "아니오" | "취소" | "중지"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affirmative_replies() {
        for r in ["yes", "y", "네", "예", "확인", "승인", "ok"] {
            assert!(is_affirmative(r), "{r:?} should approve");
        }
        for r in ["no", "아니요", "install firefox", ""] {
            assert!(!is_affirmative(r), "{r:?} must not approve");
        }
    }

    #[test]
    fn test_negative_replies() {
        for r in ["no", "n", "아니요", "취소", "cancel"] {
            assert!(is_negative(r), "{r:?} should cancel");
        }
        assert!(!is_negative("yes"));
        assert!(!is_negative("what?"));
    }
}
