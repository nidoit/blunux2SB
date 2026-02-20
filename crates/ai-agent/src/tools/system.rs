use async_trait::async_trait;
use std::time::Duration;
use tokio::process::Command;

use crate::error::ToolError;
use crate::tools::{PermissionLevel, SystemTool};

async fn run_cmd(cmd: &str, args: &[&str], timeout_secs: u64) -> Result<String, ToolError> {
    let result = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        Command::new(cmd).args(args).output(),
    )
    .await
    .map_err(|_| ToolError::Timeout {
        secs: timeout_secs,
    })?
    .map_err(ToolError::Io)?;

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

    if result.status.success() {
        Ok(stdout)
    } else {
        // Still return stdout if it has content, append stderr
        if !stdout.is_empty() {
            Ok(format!("{stdout}\n[stderr]: {stderr}"))
        } else {
            Err(ToolError::ExecutionFailed {
                command: cmd.to_string(),
                exit_code: result.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }
}

// ── check_disk ───────────────────────────────────────────────────────────────

pub struct CheckDiskTool;

#[async_trait]
impl SystemTool for CheckDiskTool {
    fn name(&self) -> &str {
        "check_disk"
    }
    fn description(&self) -> &str {
        "Check disk usage on all mounted filesystems. Returns human-readable output from df -h."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Safe
    }
    async fn execute(&self, _input: serde_json::Value) -> Result<String, ToolError> {
        run_cmd("df", &["-h"], 60).await
    }
}

// ── check_memory ─────────────────────────────────────────────────────────────

pub struct CheckMemoryTool;

#[async_trait]
impl SystemTool for CheckMemoryTool {
    fn name(&self) -> &str {
        "check_memory"
    }
    fn description(&self) -> &str {
        "Check RAM and swap usage. Returns human-readable output from free -h."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Safe
    }
    async fn execute(&self, _input: serde_json::Value) -> Result<String, ToolError> {
        run_cmd("free", &["-h"], 60).await
    }
}

// ── check_processes ──────────────────────────────────────────────────────────

pub struct CheckProcessesTool;

#[async_trait]
impl SystemTool for CheckProcessesTool {
    fn name(&self) -> &str {
        "check_processes"
    }
    fn description(&self) -> &str {
        "List running processes sorted by memory usage. Returns output from ps aux."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "sort_by": {
                    "type": "string",
                    "enum": ["memory", "cpu"],
                    "description": "Sort by memory or CPU usage (default: memory)"
                }
            },
            "required": []
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Safe
    }
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError> {
        let sort = input
            .get("sort_by")
            .and_then(|v| v.as_str())
            .unwrap_or("memory");
        let sort_flag = if sort == "cpu" { "-%cpu" } else { "-%mem" };
        run_cmd("ps", &["aux", "--sort", sort_flag], 60).await
    }
}

// ── read_logs ────────────────────────────────────────────────────────────────

pub struct ReadLogsTool;

#[async_trait]
impl SystemTool for ReadLogsTool {
    fn name(&self) -> &str {
        "read_logs"
    }
    fn description(&self) -> &str {
        "Read system logs using journalctl. Supports filtering by time, priority, and unit."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "since": {
                    "type": "string",
                    "description": "Start time (e.g. 'today', '1 hour ago', '2026-02-20')"
                },
                "priority": {
                    "type": "string",
                    "enum": ["emerg", "alert", "crit", "err", "warning", "notice", "info", "debug"],
                    "description": "Minimum priority level"
                },
                "unit": {
                    "type": "string",
                    "description": "Systemd unit name to filter (e.g. 'sshd', 'docker')"
                },
                "lines": {
                    "type": "integer",
                    "description": "Number of lines to show (default: 50)"
                }
            },
            "required": []
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Safe
    }
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError> {
        let mut args: Vec<String> = vec!["--no-pager".into()];

        if let Some(since) = input.get("since").and_then(|v| v.as_str()) {
            args.push("--since".into());
            args.push(since.into());
        } else {
            args.push("--since".into());
            args.push("today".into());
        }

        if let Some(priority) = input.get("priority").and_then(|v| v.as_str()) {
            args.push("-p".into());
            args.push(priority.into());
        }

        if let Some(unit) = input.get("unit").and_then(|v| v.as_str()) {
            args.push("-u".into());
            args.push(unit.into());
        }

        let lines = input
            .get("lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(50);
        args.push("-n".into());
        args.push(lines.to_string());

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        run_cmd("journalctl", &arg_refs, 60).await
    }
}

// ── check_network ────────────────────────────────────────────────────────────

pub struct CheckNetworkTool;

#[async_trait]
impl SystemTool for CheckNetworkTool {
    fn name(&self) -> &str {
        "check_network"
    }
    fn description(&self) -> &str {
        "Check network status and list available WiFi networks."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["wifi_list", "status"],
                    "description": "Action: 'wifi_list' to scan WiFi, 'status' for connection status (default: status)"
                }
            },
            "required": []
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Safe
    }
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("status");
        match action {
            "wifi_list" => run_cmd("nmcli", &["device", "wifi", "list"], 60).await,
            _ => run_cmd("nmcli", &["general", "status"], 60).await,
        }
    }
}

// ── run_command (generic fallback) ───────────────────────────────────────────

pub struct RunCommandTool;

#[async_trait]
impl SystemTool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }
    fn description(&self) -> &str {
        "Run an arbitrary shell command. Use this when no specific tool matches the task. The command will be checked for safety before execution."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            },
            "required": ["command"]
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::RequiresConfirmation
    }
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'command' field".into()))?;

        let result = tokio::time::timeout(
            Duration::from_secs(60),
            Command::new("sh").arg("-c").arg(command).output(),
        )
        .await
        .map_err(|_| ToolError::Timeout { secs: 60 })?
        .map_err(ToolError::Io)?;

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        if result.status.success() {
            Ok(if stderr.is_empty() {
                stdout
            } else {
                format!("{stdout}\n[stderr]: {stderr}")
            })
        } else {
            Err(ToolError::ExecutionFailed {
                command: command.to_string(),
                exit_code: result.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }
}
