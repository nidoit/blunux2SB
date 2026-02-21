use std::collections::HashMap;
use std::io::{self, BufRead, Write};

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

pub struct Agent {
    provider: Box<dyn Provider>,
    tools: ToolRegistry,
    memory: Memory,
    safety: SafetyChecker,
    conversation: Vec<Message>,
    /// Per-user conversation history for daemon mode (keyed by phone number).
    user_conversations: HashMap<String, Vec<Message>>,
    lang: Language,
    /// When true, skip interactive confirmation prompts (daemon / WhatsApp mode).
    auto_confirm: bool,
}

impl Agent {
    pub fn new(config: &AgentConfig) -> Result<Self, AgentError> {
        let provider = build_provider(config).map_err(AgentError::Config)?;
        let tools = ToolRegistry::default_tools();
        let memory = Memory::new(config.config_dir.clone());
        let safety = SafetyChecker::new();

        Ok(Self {
            provider,
            tools,
            memory,
            safety,
            conversation: Vec::new(),
            user_conversations: HashMap::new(),
            lang: config.language.clone(),
            auto_confirm: false,
        })
    }

    /// Create an agent configured for daemon / WhatsApp mode (auto-confirms all prompts).
    pub fn new_daemon(config: &AgentConfig) -> Result<Self, AgentError> {
        let mut agent = Self::new(config)?;
        agent.auto_confirm = true;
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
        // Restore per-user conversation
        let mut conv = self
            .user_conversations
            .remove(phone)
            .unwrap_or_default();

        // Swap in the user's conversation
        std::mem::swap(&mut self.conversation, &mut conv);

        let result = self.chat(user_message).await;

        // Swap back and store
        std::mem::swap(&mut self.conversation, &mut conv);
        self.user_conversations.insert(phone.to_string(), conv);

        result
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

        Ok(format!(
            "You are Blunux AI Agent, a Linux system management assistant for Blunux (Arch-based).\n\
             You help users manage their system using natural language.\n\
             {lang_instruction}\n\
             \n\
             Available tools: {tool_list}\n\
             Safe mode: enabled\n\
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
        &self,
        result: &CompletionResult,
    ) -> Result<Vec<ContentBlock>, AgentError> {
        let mut tool_results = Vec::new();

        for (id, name, input) in result.tool_uses() {
            let tool_result = self.execute_tool(id, name, input.clone()).await?;
            tool_results.push(tool_result);
        }

        Ok(tool_results)
    }

    async fn execute_tool(
        &self,
        tool_use_id: &str,
        name: &str,
        input: serde_json::Value,
    ) -> Result<ContentBlock, AgentError> {
        let tool = match self.tools.get(name) {
            Some(t) => t,
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
        match tool.permission_level() {
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
                } else {
                    // Non-run_command tool requiring confirmation
                    let description = strings::tool_executing(&self.lang, name);
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

    fn prompt_confirmation(&self) -> bool {
        if self.auto_confirm {
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
