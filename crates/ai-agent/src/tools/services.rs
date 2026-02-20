use async_trait::async_trait;
use std::time::Duration;
use tokio::process::Command;

use crate::error::ToolError;
use crate::tools::{PermissionLevel, SystemTool};

pub struct ManageServiceTool;

#[async_trait]
impl SystemTool for ManageServiceTool {
    fn name(&self) -> &str {
        "manage_service"
    }
    fn description(&self) -> &str {
        "Manage systemd services: start, stop, restart, enable, disable, or check status."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "stop", "restart", "enable", "disable", "status"],
                    "description": "Action to perform on the service"
                },
                "service": {
                    "type": "string",
                    "description": "Service name (e.g. 'sshd', 'docker', 'bluetooth')"
                }
            },
            "required": ["action", "service"]
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::RequiresConfirmation
    }
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'action' field".into()))?;

        let service = input
            .get("service")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'service' field".into()))?;

        // Validate action
        if !["start", "stop", "restart", "enable", "disable", "status"].contains(&action) {
            return Err(ToolError::InvalidInput(format!(
                "Invalid action: {action}"
            )));
        }

        // Validate service name
        if !service
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '@')
        {
            return Err(ToolError::InvalidInput(format!(
                "Invalid service name: {service}"
            )));
        }

        let (cmd, args): (&str, Vec<&str>) = if action == "status" {
            ("systemctl", vec!["status", service])
        } else {
            ("sudo", vec!["systemctl", action, service])
        };

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            Command::new(cmd).args(&args).output(),
        )
        .await
        .map_err(|_| ToolError::Timeout { secs: 30 })?
        .map_err(ToolError::Io)?;

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        // systemctl status returns non-zero for inactive services — that's OK
        if action == "status" || result.status.success() {
            Ok(if stderr.is_empty() {
                stdout
            } else {
                format!("{stdout}\n[stderr]: {stderr}")
            })
        } else {
            Err(ToolError::ExecutionFailed {
                command: format!("{cmd} {}", args.join(" ")),
                exit_code: result.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }
}
